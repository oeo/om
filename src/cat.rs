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
    let scored: Vec<ScoredFile> = files
        .iter()
        .map(|f| ScoredFile {
            path: f.clone(),
            score: 10,
            reason: "explicit".to_string(),
        })
        .collect();

    output_files(root, &scored, args, session, format)
}

fn output_files(
    root: &Path,
    files: &[ScoredFile],
    args: &CatArgs,
    session: &mut Option<Session>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let no_headers = args.no_headers;
    let mut total_files = 0;
    let mut skipped_binary = 0;
    let mut skipped_session = 0;
    let mut total_lines = 0;
    let mut output_files_data = Vec::new();

    for f in files {
        let full_path = root.join(&f.path);

        if !full_path.exists() {
            continue;
        }

        if !is_text_file(&full_path) {
            skipped_binary += 1;
            continue;
        }

        let content = match fs::read(&full_path) {
            Ok(c) => c,
            Err(_) => {
                skipped_binary += 1;
                continue;
            }
        };

        if let Some(ref sess) = session {
            let hash = Session::compute_hash(&content);
            if sess.was_read(&f.path, &hash) {
                skipped_session += 1;
                continue;
            }
        }

        total_files += 1;
        output_files_data.push((f.path.clone(), f.score, content));
    }

    match format {
        OutputFormat::Text => {
            if !no_headers {
                let project_name = root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project");

                println!("# Project: {}", project_name);

                if let Some(ref sess) = session {
                    println!("# Session: {}", sess.name);
                }

                println!("# Files: {} shown", total_files);

                if skipped_binary > 0 {
                    println!("# Skipped: {} binary/unreadable", skipped_binary);
                }

                if skipped_session > 0 {
                    println!("# Skipped: {} unchanged (session)", skipped_session);
                }
            }

            for (path, _score, content) in &output_files_data {
                let content_str = String::from_utf8_lossy(content);
                let line_count = content_str.lines().count();
                total_lines += line_count;

                let hash = Session::compute_hash(content);
                let hash_prefix = &hash[..12];

                let mut header = format!("FILE: {}\nLINES: {}", path, line_count);
                if args.tokens {
                    let tokens = crate::tokens::count_tokens(&content_str, "cl100k_base")
                        .unwrap_or(content_str.len() / 4);
                    header.push_str(&format!("\nTOKENS: {}", tokens));
                }
                header.push_str(&format!("\nHASH: {}", hash_prefix));

                println!("\n{}", "=".repeat(80));
                println!("{}", header);
                println!("{}", "=".repeat(80));
                println!("{}", content_str);

                if let Some(ref mut sess) = session {
                    sess.mark_read(path, &hash);
                }
            }

            if !no_headers && total_files > 0 {
                println!("\n# Total lines: {}", total_lines);
            }
        }
        OutputFormat::Json | OutputFormat::Xml => {
            let mut file_outputs = Vec::new();

            for (path, score, content) in &output_files_data {
                let content_str = String::from_utf8_lossy(content);
                let line_count = content_str.lines().count();
                total_lines += line_count;

                let hash = Session::compute_hash(content);

                let tokens = if args.tokens {
                    Some(
                        crate::tokens::count_tokens(&content_str, "cl100k_base")
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
                    content: Some(content_str.to_string()),
                });

                if let Some(ref mut sess) = session {
                    sess.mark_read(path, &hash);
                }
            }

            let project_name = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string();

            let session_name = session.as_ref().map(|s| s.name.clone());

            let cat_output = CatOutput {
                project: project_name,
                session: session_name,
                files_shown: total_files,
                skipped_binary,
                skipped_session,
                total_lines,
                files: file_outputs,
            };

            match format {
                OutputFormat::Json => output::json::output_cat(&cat_output)?,
                OutputFormat::Xml => output::xml::output_cat(&cat_output)?,
                OutputFormat::Text => unreachable!(),
            }
        }
    }

    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    use mime_guess::mime;

    let likely_binary = match mime.type_() {
        mime::IMAGE | mime::VIDEO | mime::AUDIO => true,
        mime::APPLICATION => mime.subtype() == mime::OCTET_STREAM,
        _ => false,
    };

    if likely_binary {
        return false;
    }

    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > 200_000 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cat_files_explicit_list() {
        let files = vec!["foo.rs".to_string(), "bar.rs".to_string()];
        let scored: Vec<ScoredFile> = files
            .iter()
            .map(|f| ScoredFile {
                path: f.clone(),
                score: 10,
                reason: "explicit".to_string(),
            })
            .collect();

        assert_eq!(scored.len(), 2);
        assert_eq!(scored[0].path, "foo.rs");
        assert_eq!(scored[0].score, 10);
        assert_eq!(scored[1].path, "bar.rs");
        assert_eq!(scored[1].score, 10);
    }

    use std::io::Write;
    use tempfile::tempdir;

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
    }
}
