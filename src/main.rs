mod cat;
mod cli;
mod config;
mod git;
mod ignore;
mod output;
mod scorer;
mod session;
mod session_cmd;
mod tokens;
mod tree;

use clap::Parser;
use cli::{Cli, Commands};

pub fn count_tokens(text: &str) -> usize {
    tokens::count_tokens(text, "o200k_base")
        .unwrap_or_else(|_| num_integer::Integer::div_ceil(&text.len(), &4usize))
}

fn main() {
    let cli = Cli::parse();
    let config = config::load_config();

    let result = match cli.command {
        Commands::Tree(mut args) => {
            if args.min_score.is_none() {
                args.min_score = config.min_score;
            }
            if args.depth.is_none() {
                args.depth = config.depth;
            }
            if !args.flat {
                if let Some(flat) = config.flat {
                    args.flat = flat;
                }
            }
            if !args.no_color {
                if let Some(no_color) = config.no_color {
                    args.no_color = no_color;
                }
            }
            if !args.git_root {
                if let Some(git_root) = config.git_root {
                    args.git_root = git_root;
                }
            }

            if args.jobs > 0 {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(args.jobs)
                    .build_global()
                    .ok();
            }

            tree::run(args)
        }
        Commands::Cat(mut args) => {
            if args.level.is_none() {
                args.level = config.level;
            }
            if !args.no_headers {
                if let Some(no_headers) = config.no_headers {
                    args.no_headers = no_headers;
                }
            }
            if !args.no_cache {
                if let Some(no_cache) = config.no_cache {
                    args.no_cache = no_cache;
                }
            }
            cat::run(args)
        }
        Commands::Session(args) => session_cmd::run(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
