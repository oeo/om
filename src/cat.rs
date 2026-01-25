use crate::cli::CatArgs;
use crate::git;
use crate::ignore::IgnorePatterns;
use crate::scorer::{score_files, ScoredFile};
use crate::session::Session;
use std::fs;
use std::path::{Path, PathBuf};

static BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "mp3", "mp4", "wav", "avi", "mov", "mkv",
    "webm", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "pdf", "doc", "docx", "xls", "xlsx",
    "ppt", "pptx", "exe", "dll", "so", "dylib", "a", "o", "ttf", "otf", "woff", "woff2", "eot",
    "db", "sqlite", "sqlite3",
];

pub fn run(args: CatArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(args.path.unwrap_or_else(|| ".".to_string()));
    let root = git::repo_root(&path)?;

    let session_name = args.session.or_else(|| std::env::var("OM_SESSION").ok());
    let mut session = session_name.map(|name| Session::load(&name)).transpose()?;

    if args.files.is_empty() {
        cat_by_level(&root, args.level, args.no_headers, &mut session)?;
    } else {
        cat_files(&root, &args.files, args.no_headers, &mut session)?;
    }

    if let Some(ref sess) = session {
        sess.save()?;
    }

    Ok(())
}

fn cat_by_level(
    root: &Path,
    level: Option<i32>,
    no_headers: bool,
    session: &mut Option<Session>,
) -> Result<(), Box<dyn std::error::Error>> {
    let min_score = level.unwrap_or(5);

    let files = git::ls_files(root)?;
    let ignore = IgnorePatterns::load(root);

    let file_strs: Vec<String> = files
        .into_iter()
        .filter_map(|p| p.to_str().map(String::from))
        .filter(|p| !ignore.is_ignored(p))
        .collect();

    let mut scored = score_files(file_strs);
    scored.retain(|f| f.score >= min_score);
    scored.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    output_files(root, &scored, no_headers, session)
}

fn cat_files(
    root: &Path,
    files: &[String],
    no_headers: bool,
    session: &mut Option<Session>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scored: Vec<ScoredFile> = files
        .iter()
        .map(|f| ScoredFile {
            path: f.clone(),
            score: 10,
            reason: "explicit".to_string(),
        })
        .collect();

    output_files(root, &scored, no_headers, session)
}

fn output_files(
    root: &Path,
    files: &[ScoredFile],
    no_headers: bool,
    session: &mut Option<Session>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut total_files = 0;
    let mut skipped_binary = 0;
    let mut skipped_session = 0;
    let mut total_lines = 0;
    let mut output_files = Vec::new();

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
        output_files.push((f.path.clone(), content));
    }

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

    for (path, content) in output_files {
        let content_str = String::from_utf8_lossy(&content);
        let line_count = content_str.lines().count();
        total_lines += line_count;

        println!("\n# File: {}", path);
        println!("{}", content_str);

        if let Some(ref mut sess) = session {
            let hash = Session::compute_hash(&content);
            sess.mark_read(&path, &hash);
        }
    }

    if !no_headers && total_files > 0 {
        println!("\n# Total lines: {}", total_lines);
    }

    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if BINARY_EXTENSIONS.contains(&ext) {
            return false;
        }
    }

    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > 100_000 {
            return false;
        }
    }

    true
}
