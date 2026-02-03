use crate::cli::TreeArgs;
use crate::git;
use crate::ignore::IgnorePatterns;
use crate::output::{self, FileOutput, OutputFormat, TreeOutput};
use crate::scorer::{score_files, ScoredFile};
use colored::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn run(args: TreeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(args.path.unwrap_or_else(|| ".".to_string()));
    let root = git::repo_root(&path)?;

    let files = git::ls_files(&root)?;
    let ignore = IgnorePatterns::load(&root);

    let git_status = if args.dirty || args.staged || args.unstaged {
        Some(git::git_status(&root)?)
    } else {
        None
    };

    let filter_prefix = if args.git_root {
        None
    } else {
        let abs_path = std::fs::canonicalize(&path)?;
        let abs_root = std::fs::canonicalize(&root)?;
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

    let jobs = if args.jobs == 0 {
        num_cpus::get()
    } else {
        args.jobs
    };

    let mut scored: Vec<ScoredFile> = if jobs > 1 {
        file_strs
            .par_iter()
            .map(|f| crate::scorer::score_file(f))
            .collect()
    } else {
        score_files(file_strs)
    };

    scored.retain(|f| f.score >= args.min_score.unwrap_or(1));

    if let Some(max_depth) = args.depth {
        scored.retain(|f| {
            let depth = f.path.split('/').count() - 1;
            depth <= max_depth
        });
    }

    let format = if let Some(ref fmt) = args.format {
        fmt.parse::<OutputFormat>()?
    } else {
        OutputFormat::Text
    };

    match format {
        OutputFormat::Text => {
            if args.flat {
                print_flat(&scored, args.no_color, args.tokens, &root);
            } else {
                print_tree(&scored, args.no_color, args.tokens, &root);
            }
        }
        OutputFormat::Json => {
            let project_name = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string();

            let files: Vec<FileOutput> = if args.tokens {
                scored
                    .par_iter()
                    .map(|f| {
                        let full_path = root.join(&f.path);
                        let tokens = std::fs::read_to_string(&full_path).ok().map(|c| {
                            crate::tokens::count_tokens(&c, "cl100k_base").unwrap_or(c.len() / 4)
                        });
                        FileOutput {
                            path: f.path.clone(),
                            score: f.score,
                            tokens,
                            lines: 0,
                            content: None,
                        }
                    })
                    .collect()
            } else {
                scored
                    .iter()
                    .map(|f| FileOutput {
                        path: f.path.clone(),
                        score: f.score,
                        tokens: None,
                        lines: 0,
                        content: None,
                    })
                    .collect()
            };

            let output = TreeOutput {
                project: project_name,
                files,
            };

            output::json::output_tree(&output)?;
        }
        OutputFormat::Xml => {
            let project_name = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string();

            let files: Vec<FileOutput> = if args.tokens {
                scored
                    .par_iter()
                    .map(|f| {
                        let full_path = root.join(&f.path);
                        let tokens = std::fs::read_to_string(&full_path).ok().map(|c| {
                            crate::tokens::count_tokens(&c, "cl100k_base").unwrap_or(c.len() / 4)
                        });
                        FileOutput {
                            path: f.path.clone(),
                            score: f.score,
                            tokens,
                            lines: 0,
                            content: None,
                        }
                    })
                    .collect()
            } else {
                scored
                    .iter()
                    .map(|f| FileOutput {
                        path: f.path.clone(),
                        score: f.score,
                        tokens: None,
                        lines: 0,
                        content: None,
                    })
                    .collect()
            };

            let output = TreeOutput {
                project: project_name,
                files,
            };

            output::xml::output_tree(&output)?;
        }
    }

    Ok(())
}

fn print_flat(files: &[ScoredFile], no_color: bool, show_tokens: bool, root: &Path) {
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    for file in sorted {
        let score_str = format!("{:2}", file.score);
        let colored_score = if no_color {
            score_str
        } else {
            match file.score {
                8..=10 => score_str.green().bold().to_string(),
                5..=7 => score_str.yellow().to_string(),
                _ => score_str.dimmed().to_string(),
            }
        };

        let mut line = format!("{} {}", colored_score, file.path);

        if show_tokens {
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(full_path) {
                let tokens = crate::count_tokens(&content);
                line.push_str(&format!(" ({} tokens)", tokens));
            }
        }

        println!("{}", line);
    }
}

fn print_tree(files: &[ScoredFile], no_color: bool, show_tokens: bool, root: &Path) {
    let tree = build_tree(files);
    print_node(&tree, "", true, no_color, show_tokens, root);
}

struct TreeNode {
    name: String,
    path: String,
    score: Option<i32>,
    children: HashMap<String, TreeNode>,
}

impl TreeNode {
    fn new(name: String, path: String) -> Self {
        TreeNode {
            name,
            path,
            score: None,
            children: HashMap::new(),
        }
    }
}

fn build_tree(files: &[ScoredFile]) -> TreeNode {
    let mut root = TreeNode::new(".".to_string(), ".".to_string());

    for file in files {
        let parts: Vec<&str> = file.path.split('/').collect();
        let mut current = &mut root;
        let mut current_path = String::new();

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(part);

            let part_path = current_path.clone();
            current = current
                .children
                .entry(part.to_string())
                .or_insert_with(|| TreeNode::new(part.to_string(), part_path));

            if is_last {
                current.score = Some(file.score);
            }
        }
    }

    root
}

fn get_max_score(node: &TreeNode) -> i32 {
    let mut max = node.score.unwrap_or(0);

    for child in node.children.values() {
        max = max.max(get_max_score(child));
    }

    max
}

fn print_node(
    node: &TreeNode,
    prefix: &str,
    is_last: bool,
    no_color: bool,
    show_tokens: bool,
    root: &Path,
) {
    if node.name != "." {
        let connector = if is_last { "└── " } else { "├── " };

        let display_name = if node.children.is_empty() {
            let score_str = format!("{:2}", node.score.unwrap_or(0));
            let colored_score = if no_color {
                score_str
            } else {
                match node.score.unwrap_or(0) {
                    8..=10 => score_str.green().bold().to_string(),
                    5..=7 => score_str.yellow().to_string(),
                    _ => score_str.dimmed().to_string(),
                }
            };

            let mut name = format!("{} {}", colored_score, node.name);
            if show_tokens {
                let full_path = root.join(&node.path);
                if let Ok(content) = std::fs::read_to_string(full_path) {
                    let tokens = crate::count_tokens(&content);
                    name.push_str(&format!(" ({} tokens)", tokens));
                }
            }
            name
        } else if no_color {
            format!("{}/", node.name)
        } else {
            format!("{}", node.name.blue().bold())
        };

        println!("{}{}{}", prefix, connector, display_name);
    }

    let mut sorted_children: Vec<_> = node.children.values().collect();
    sorted_children.sort_by(|a, b| {
        get_max_score(b)
            .cmp(&get_max_score(a))
            .then(a.name.cmp(&b.name))
    });

    for (i, child) in sorted_children.iter().enumerate() {
        let is_last_child = i == sorted_children.len() - 1;
        let new_prefix = if node.name == "." {
            String::new()
        } else {
            format!("{}{}   ", prefix, if is_last { " " } else { "│" })
        };
        print_node(
            child,
            &new_prefix,
            is_last_child,
            no_color,
            show_tokens,
            root,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scorer::ScoredFile;

    #[test]
    fn test_build_tree() {
        let files = vec![
            ScoredFile {
                path: "src/main.rs".to_string(),
                score: 10,
                reason: "".to_string(),
            },
            ScoredFile {
                path: "src/lib.rs".to_string(),
                score: 9,
                reason: "".to_string(),
            },
            ScoredFile {
                path: "README.md".to_string(),
                score: 8,
                reason: "".to_string(),
            },
        ];

        let root = build_tree(&files);
        assert_eq!(root.name, ".");
        assert_eq!(root.children.len(), 2);
        assert!(root.children.contains_key("src"));
        assert!(root.children.contains_key("README.md"));

        let src = &root.children["src"];
        assert_eq!(src.children.len(), 2);
    }

    #[test]
    fn test_max_score_calculation() {
        let mut root = TreeNode::new(".".to_string(), ".".to_string());
        let mut src = TreeNode::new("src".to_string(), "src".to_string());
        let mut main = TreeNode::new("main.rs".to_string(), "src/main.rs".to_string());
        main.score = Some(10);
        src.children.insert("main.rs".to_string(), main);
        root.children.insert("src".to_string(), src);

        assert_eq!(get_max_score(&root), 10);
    }

    #[test]
    fn test_depth_filtering_logic() {
        let mut scored = vec![
            ScoredFile {
                path: "a.rs".to_string(),
                score: 10,
                reason: "".to_string(),
            },
            ScoredFile {
                path: "dir/b.rs".to_string(),
                score: 10,
                reason: "".to_string(),
            },
            ScoredFile {
                path: "dir/subdir/c.rs".to_string(),
                score: 10,
                reason: "".to_string(),
            },
        ];

        let max_depth = 1;
        scored.retain(|f| {
            let depth = f.path.split('/').count() - 1;
            depth <= max_depth
        });

        assert_eq!(scored.len(), 2);
        assert!(scored.iter().any(|f| f.path == "a.rs"));
        assert!(scored.iter().any(|f| f.path == "dir/b.rs"));
        assert!(!scored.iter().any(|f| f.path == "dir/subdir/c.rs"));
    }
}
