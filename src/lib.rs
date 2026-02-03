pub mod git;
pub mod ignore;
pub mod output;
pub mod scorer;
pub mod session;

pub use git::{ls_files, repo_root};
pub use ignore::IgnorePatterns;
pub use output::{CatOutput, FileOutput, OutputFormat, TreeOutput};
pub use scorer::{score_file, score_files, ScoredFile};
pub use session::Session;

use std::path::Path;

pub fn get_context(
    root: &Path,
    min_score: i32,
) -> Result<Vec<ScoredFile>, Box<dyn std::error::Error>> {
    let files = ls_files(root)?;
    let ignore = IgnorePatterns::load(root);

    let file_strs: Vec<String> = files
        .into_iter()
        .filter_map(|p| p.to_str().map(String::from))
        .filter(|p| !ignore.is_ignored(p))
        .collect();

    let mut scored = score_files(file_strs);
    scored.retain(|f| f.score >= min_score);
    scored.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    Ok(scored)
}
