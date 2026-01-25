use crate::cli::TreeArgs;
use crate::git;
use crate::ignore::IgnorePatterns;
use crate::scorer::{score_files, ScoredFile};
use colored::*;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn run(args: TreeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(args.path.unwrap_or_else(|| ".".to_string()));
    let root = git::repo_root(&path)?;

    let files = git::ls_files(&root)?;
    let ignore = IgnorePatterns::load(&root);

    let file_strs: Vec<String> = files
        .into_iter()
        .filter_map(|p| p.to_str().map(String::from))
        .filter(|p| !ignore.is_ignored(p))
        .collect();

    let mut scored = score_files(file_strs);

    scored.retain(|f| f.score >= args.min_score.unwrap_or(1));

    if let Some(max_depth) = args.depth {
        scored.retain(|f| {
            let depth = f.path.split('/').count() - 1;
            depth <= max_depth
        });
    }

    if args.flat {
        print_flat(&scored, args.no_color);
    } else {
        print_tree(&scored, args.no_color);
    }

    Ok(())
}

fn print_flat(files: &[ScoredFile], no_color: bool) {
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

        println!("{} {}", colored_score, file.path);
    }
}

fn print_tree(files: &[ScoredFile], no_color: bool) {
    let tree = build_tree(files);
    print_node(&tree, "", true, no_color);
}

struct TreeNode {
    name: String,
    score: Option<i32>,
    children: HashMap<String, TreeNode>,
}

impl TreeNode {
    fn new(name: String) -> Self {
        TreeNode {
            name,
            score: None,
            children: HashMap::new(),
        }
    }
}

fn build_tree(files: &[ScoredFile]) -> TreeNode {
    let mut root = TreeNode::new(".".to_string());

    for file in files {
        let parts: Vec<&str> = file.path.split('/').collect();
        let mut current = &mut root;

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            current = current
                .children
                .entry(part.to_string())
                .or_insert_with(|| TreeNode::new(part.to_string()));

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

fn print_node(node: &TreeNode, prefix: &str, is_last: bool, no_color: bool) {
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
            format!("{} {}", colored_score, node.name)
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
        print_node(child, &new_prefix, is_last_child, no_color);
    }
}
