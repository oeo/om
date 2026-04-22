use crate::cli::CatArgs;
use crate::git;
use crate::ignore::IgnorePatterns;
use crate::output::{self, CatOutput, FileOutput, OutputFormat};
use crate::scorer::{score_files, ScoredFile};
use crate::session::Session;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: CatArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(args.path.clone().unwrap_or_else(|| ".".to_string()));
    let root = git::repo_root(&path)?;

    let session_name = args
        .session
        .clone()
        .or_else(|| std::env::var("OM_SESSION").ok());
    let mut session = session_name.map(|name| Session::load(&name)).transpose()?;

    let format = if let Some(ref fmt) = args.format {
        fmt.parse::<OutputFormat>()?
    } else {
        OutputFormat::Text
    };

    if args.files.is_empty() {
        cat_by_level(&root, &args, &mut session, format)?;
    } else {
        cat_files(&root, &args.files, &args, &mut session, format)?;
    }

    if let Some(ref sess) = session {
        sess.save()?;
    }

    Ok(())
}

fn cat_by_level(
    root: &Path,
    args: &CatArgs,
    session: &mut Option<Session>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let min_score = args.level.unwrap_or(5);

    let path = PathBuf::from(args.path.clone().unwrap_or_else(|| ".".to_string()));

    let files = git::ls_files(root)?;
    let ignore = IgnorePatterns::load(root);

    let git_status = if args.dirty || args.staged || args.unstaged {
        Some(git::git_status(root)?)
    } else {
        None
    };

    let filter_prefix = if args.git_root {
        None
    } else {
        let abs_path = std::fs::canonicalize(&path)?;
        let abs_root = std::fs::canonicalize(root)?;
        abs_path.strip_prefix(&abs_root).ok().and_then(|p| {
            if p.as_os_str().is_empty() {
                None
            } else {
                p.to_str().map(|s| s.to_string())
            }
        })
    };

    let file_strs: Vec<String> = files
        .into_iter()
        .filter_map(|p| p.to_str().map(String::from))
        .filter(|p| !ignore.is_ignored(p))
        .filter(|p| {
            if let Some(prefix) = &filter_prefix {
                p.starts_with(prefix)
            } else {
                true
            }
        })
        .filter(|p| {
            if let Some(status) = &git_status {
                if args.staged && status.staged.contains(p) {
                    return true;
                }
                if args.unstaged && status.unstaged.contains(p) {
                    return true;
                }
                if args.dirty && status.dirty.contains(p) {
                    return true;
                }
                false
            } else {
                true
            }
        })
        .collect();

    let jobs = num_cpus::get();
    let mut scored: Vec<ScoredFile> = if jobs > 1 {
        use rayon::prelude::*;
        file_strs
            .par_iter()
            .map(|f| crate::scorer::score_file(f))
            .collect()
    } else {
        score_files(file_strs)
    };
    scored.retain(|f| f.score >= min_score);
    scored.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    output_files(root, &scored, args, session, format)
}

fn cat_files(
    root: &Path,
    files: &[String],
    args: &CatArgs,
    session: &mut Option<Session>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let canonical_root = fs::canonicalize(root)?;

    let scored: Vec<ScoredFile> = files
        .iter()
        .map(|f| {
            let abs_path = cwd.join(f);
            let canonical_path = fs::canonicalize(&abs_path).unwrap_or(abs_path);
            let rel_path = match canonical_path.strip_prefix(&canonical_root) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => f.clone(),
            };

            ScoredFile {
                path: rel_path,
                score: 10,
                reason: "explicit".to_string(),
            }
        })
        .collect();

    output_files(root, &scored, args, session, format)
}

// collected result from scanning files — pure, no I/O side effects beyond reading files
struct CollectedOutput {
    data: CatOutput,
    // raw bytes + decoded string for each file that passed all filters
    file_contents: Vec<(String, i32, Vec<u8>, String)>,
}

fn collect_output(
    root: &Path,
    files: &[ScoredFile],
    session: &mut Option<Session>,
    no_cache: bool,
    show_tokens: bool,
) -> Result<CollectedOutput, Box<dyn std::error::Error>> {
    let mut skipped_binary = 0;
    let mut skipped_unreadable = 0;
    let mut skipped_session = 0;
    let mut skipped_binary_paths: Vec<String> = Vec::new();
    let mut skipped_unreadable_paths: Vec<String> = Vec::new();
    let mut file_contents: Vec<(String, i32, Vec<u8>, String)> = Vec::new();

    for f in files {
        let full_path = root.join(&f.path);

        if !full_path.exists() {
            continue;
        }

        if !is_text_file(&full_path) {
            skipped_binary += 1;
            skipped_binary_paths.push(f.path.clone());
            continue;
        }

        let raw = match fs::read(&full_path) {
            Ok(c) => c,
            Err(_) => {
                skipped_unreadable += 1;
                skipped_unreadable_paths.push(f.path.clone());
                continue;
            }
        };

        // skip empty files — they contribute no signal
        if raw.is_empty() {
            continue;
        }

        // reject files with invalid UTF-8 rather than silently corrupting them
        let content_str = match String::from_utf8(raw.clone()) {
            Ok(s) => s,
            Err(_) => {
                skipped_unreadable += 1;
                skipped_unreadable_paths.push(f.path.clone());
                continue;
            }
        };

        if let Some(ref sess) = session {
            if !no_cache {
                let hash = Session::compute_hash(&raw);
                if sess.was_read(&f.path, &hash) {
                    skipped_session += 1;
                    continue;
                }
            }
        }

        file_contents.push((f.path.clone(), f.score, raw, content_str));
    }

    let project_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let session_name = session.as_ref().map(|s| s.name.clone());

    let mut total_lines = 0;
    let mut file_outputs: Vec<FileOutput> = Vec::new();

    for (path, score, raw, content_str) in &file_contents {
        let line_count = content_str.lines().count();
        total_lines += line_count;

        let hash = Session::compute_hash(raw);

        let tokens = if show_tokens {
            Some(
                crate::tokens::count_tokens(content_str, "cl100k_base")
                    .unwrap_or(content_str.len() / 4),
            )
        } else {
            None
        };

        file_outputs.push(FileOutput {
            path: path.clone(),
            score: *score,
            tokens,
            lines: line_count,
            content: Some(content_str.clone()),
        });

        if let Some(ref mut sess) = session {
            if !no_cache {
                sess.mark_read(path, &hash);
            }
        }
    }

    Ok(CollectedOutput {
        data: CatOutput {
            project: project_name,
            session: session_name,
            files_shown: file_contents.len(),
            skipped_binary,
            skipped_unreadable,
            skipped_session,
            total_lines,
            files: file_outputs,
            skipped_binary_paths,
            skipped_unreadable_paths,
        },
        file_contents,
    })
}

fn render_text(collected: &CollectedOutput, no_headers: bool, show_tokens: bool) -> String {
    let mut out = String::new();
    let data = &collected.data;

    if !no_headers {
        out.push_str(&format!("# Project: {}\n", data.project));

        if let Some(ref session) = data.session {
            out.push_str(&format!("# Session: {}\n", session));
        }

        out.push_str(&format!("# Files: {} shown\n", data.files_shown));

        if data.skipped_binary > 0 {
            out.push_str(&format!("# Skipped: {} binary\n", data.skipped_binary));
            for path in &data.skipped_binary_paths {
                out.push_str(&format!("#   - {}\n", path));
            }
        }

        if data.skipped_unreadable > 0 {
            out.push_str(&format!(
                "# Skipped: {} unreadable (read error or invalid UTF-8)\n",
                data.skipped_unreadable
            ));
            for path in &data.skipped_unreadable_paths {
                out.push_str(&format!("#   - {}\n", path));
            }
        }

        if data.skipped_session > 0 {
            out.push_str(&format!(
                "# Skipped: {} unchanged (session)\n",
                data.skipped_session
            ));
        }
    }

    for (path, _score, raw, content_str) in &collected.file_contents {
        let line_count = content_str.lines().count();
        let hash = Session::compute_hash(raw);
        let hash_prefix = &hash[..12];

        let mut header = format!("FILE: {}\nLINES: {}", path, line_count);
        if show_tokens {
            let tokens = crate::tokens::count_tokens(content_str, "cl100k_base")
                .unwrap_or(content_str.len() / 4);
            header.push_str(&format!("\nTOKENS: {}", tokens));
        }
        header.push_str(&format!("\nHASH: {}", hash_prefix));

        out.push_str(&format!("\n{}\n", "=".repeat(80)));
        out.push_str(&format!("{}\n", header));
        out.push_str(&format!("{}\n", "=".repeat(80)));
        out.push_str(&format!("{}\n", content_str));
    }

    if !no_headers && data.files_shown > 0 {
        out.push_str(&format!("\n# Total lines: {}\n", data.total_lines));
    }

    out
}

fn output_files(
    root: &Path,
    files: &[ScoredFile],
    args: &CatArgs,
    session: &mut Option<Session>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let collected = collect_output(root, files, session, args.no_cache, args.tokens)?;

    match format {
        OutputFormat::Text => {
            print!("{}", render_text(&collected, args.no_headers, args.tokens));
        }
        OutputFormat::Json => output::json::output_cat(&collected.data)?,
        OutputFormat::Xml => output::xml::output_cat(&collected.data)?,
    }

    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    use mime_guess::mime;

    let mime = mime_guess::from_path(path).first();

    // if mime_guess has a confident opinion, trust it for known binary types
    if let Some(m) = mime {
        let is_binary = match m.type_() {
            mime::IMAGE | mime::VIDEO | mime::AUDIO => true,
            mime::APPLICATION => m.subtype() == mime::OCTET_STREAM,
            _ => false,
        };
        if is_binary {
            return false;
        }
    }
    // if mime_guess has no opinion (unknown/no extension), fall through to byte probe

    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > 200_000 {
            return false;
        }
    }

    // probe the first 8KB for null bytes — the definitive binary signal
    if let Ok(mut file) = fs::File::open(path) {
        use std::io::Read;
        let mut buf = [0u8; 8192];
        if let Ok(n) = file.read(&mut buf) {
            if buf[..n].contains(&0u8) {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    // helper: build a ScoredFile pointing at a real path on disk
    fn scored(path: &str) -> ScoredFile {
        ScoredFile {
            path: path.to_string(),
            score: 10,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn test_cat_files_explicit_list() {
        let files = vec!["foo.rs".to_string(), "bar.rs".to_string()];
        let scored_files: Vec<ScoredFile> = files
            .iter()
            .map(|f| ScoredFile {
                path: f.clone(),
                score: 10,
                reason: "explicit".to_string(),
            })
            .collect();

        assert_eq!(scored_files.len(), 2);
        assert_eq!(scored_files[0].path, "foo.rs");
        assert_eq!(scored_files[0].score, 10);
        assert_eq!(scored_files[1].path, "bar.rs");
        assert_eq!(scored_files[1].score, 10);
    }

    #[test]
    fn test_empty_file_is_silently_dropped() {
        let dir = tempdir().unwrap();
        let empty = dir.path().join("empty.txt");
        std::fs::File::create(&empty).unwrap();

        let files = vec![scored("empty.txt")];
        let mut session = None;
        let result = collect_output(dir.path(), &files, &mut session, true, false).unwrap();

        assert_eq!(result.data.files_shown, 0);
        assert_eq!(result.data.skipped_binary, 0);
        assert_eq!(result.data.skipped_unreadable, 0);
        assert!(result.file_contents.is_empty());
    }

    #[test]
    fn test_invalid_utf8_goes_to_skipped_unreadable() {
        let dir = tempdir().unwrap();
        let bad = dir.path().join("bad.txt");
        {
            let mut f = std::fs::File::create(&bad).unwrap();
            // valid UTF-8 prefix followed by invalid byte sequence
            f.write_all(b"hello \xFF\xFE world").unwrap();
        }

        let files = vec![scored("bad.txt")];
        let mut session = None;
        let result = collect_output(dir.path(), &files, &mut session, true, false).unwrap();

        assert_eq!(result.data.files_shown, 0);
        assert_eq!(result.data.skipped_unreadable, 1);
        assert_eq!(result.data.skipped_unreadable_paths, vec!["bad.txt"]);
        assert_eq!(result.data.skipped_binary, 0);
    }

    #[test]
    fn test_binary_file_goes_to_skipped_binary() {
        let dir = tempdir().unwrap();
        let bin = dir.path().join("app.wasm");
        {
            let mut f = std::fs::File::create(&bin).unwrap();
            // wasm magic bytes — mime_guess identifies wasm as application/wasm
            f.write_all(&[0x00, 0x61, 0x73, 0x6d]).unwrap();
        }

        let files = vec![scored("app.wasm")];
        let mut session = None;
        let result = collect_output(dir.path(), &files, &mut session, true, false).unwrap();

        assert_eq!(result.data.files_shown, 0);
        assert_eq!(result.data.skipped_binary, 1);
        assert_eq!(result.data.skipped_binary_paths, vec!["app.wasm"]);
        assert_eq!(result.data.skipped_unreadable, 0);
    }

    #[test]
    fn test_skipped_paths_appear_in_text_output() {
        let dir = tempdir().unwrap();

        let bin = dir.path().join("logo.png");
        {
            let mut f = std::fs::File::create(&bin).unwrap();
            f.write_all(&[0x89, 0x50, 0x4e, 0x47]).unwrap();
        }

        let bad = dir.path().join("corrupt.txt");
        {
            let mut f = std::fs::File::create(&bad).unwrap();
            f.write_all(b"ok \xFF\xFE bad").unwrap();
        }

        let files = vec![scored("logo.png"), scored("corrupt.txt")];
        let mut session = None;
        let collected = collect_output(dir.path(), &files, &mut session, true, false).unwrap();
        let text = render_text(&collected, false, false);

        assert!(
            text.contains("# Skipped: 1 binary"),
            "missing binary skip line"
        );
        assert!(text.contains("#   - logo.png"), "missing binary path");
        assert!(
            text.contains("# Skipped: 1 unreadable"),
            "missing unreadable skip line"
        );
        assert!(
            text.contains("#   - corrupt.txt"),
            "missing unreadable path"
        );
    }

    #[test]
    fn test_valid_text_file_is_shown() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("main.rs");
        {
            let mut f = std::fs::File::create(&src).unwrap();
            f.write_all(b"fn main() {}\n").unwrap();
        }

        let files = vec![scored("main.rs")];
        let mut session = None;
        let collected = collect_output(dir.path(), &files, &mut session, true, false).unwrap();

        assert_eq!(collected.data.files_shown, 1);
        assert_eq!(collected.data.skipped_binary, 0);
        assert_eq!(collected.data.skipped_unreadable, 0);
        assert_eq!(collected.data.files[0].lines, 1);

        let text = render_text(&collected, false, false);
        assert!(text.contains("FILE: main.rs"));
        assert!(text.contains("fn main() {}"));
    }

    #[test]
    fn test_skipped_paths_in_json_output() {
        let dir = tempdir().unwrap();

        let bin = dir.path().join("image.png");
        {
            let mut f = std::fs::File::create(&bin).unwrap();
            f.write_all(&[0x89, 0x50, 0x4e, 0x47]).unwrap();
        }

        let bad = dir.path().join("bad.rs");
        {
            let mut f = std::fs::File::create(&bad).unwrap();
            f.write_all(b"let x = \xFF;").unwrap();
        }

        let files = vec![scored("image.png"), scored("bad.rs")];
        let mut session = None;
        let collected = collect_output(dir.path(), &files, &mut session, true, false).unwrap();
        let json = serde_json::to_string(&collected.data).unwrap();

        assert!(json.contains("\"skipped_binary_paths\""));
        assert!(json.contains("image.png"));
        assert!(json.contains("\"skipped_unreadable_paths\""));
        assert!(json.contains("bad.rs"));
        assert_eq!(collected.data.skipped_binary_paths, vec!["image.png"]);
        assert_eq!(collected.data.skipped_unreadable_paths, vec!["bad.rs"]);
    }

    #[test]
    fn test_is_text_file() {
        assert!(is_text_file(Path::new("src/main.rs")));

        let dir = tempdir().unwrap();

        let png_path = dir.path().join("test.png");
        {
            let mut f = std::fs::File::create(&png_path).unwrap();
            f.write_all(&[0_u8; 1024]).unwrap();
        }
        assert!(!is_text_file(&png_path));

        let big_txt = dir.path().join("big.txt");
        {
            let mut f = std::fs::File::create(&big_txt).unwrap();
            let data = vec![b'a'; 300_000];
            f.write_all(&data).unwrap();
        }
        assert!(!is_text_file(&big_txt));

        let small_txt = dir.path().join("small.txt");
        {
            let mut f = std::fs::File::create(&small_txt).unwrap();
            f.write_all(b"hello").unwrap();
        }
        assert!(is_text_file(&small_txt));

        // extensionless text files (Makefile, LICENSE, Dockerfile, etc.)
        // must not be falsely skipped as binary
        let makefile = dir.path().join("Makefile");
        {
            let mut f = std::fs::File::create(&makefile).unwrap();
            f.write_all(b"all:\n\techo done\n").unwrap();
        }
        assert!(is_text_file(&makefile));

        // extensionless file with actual binary content (null bytes) must be skipped
        let bin_no_ext = dir.path().join("binary_no_ext");
        {
            let mut f = std::fs::File::create(&bin_no_ext).unwrap();
            f.write_all(&[0x7f, 0x45, 0x4c, 0x46, 0x00, 0x00]).unwrap();
        }
        assert!(!is_text_file(&bin_no_ext));
    }
}
