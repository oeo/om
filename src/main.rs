mod cat;
mod cli;
mod git;
mod ignore;
mod init;
mod scorer;
mod session;
mod session_cmd;
mod tree;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Tree(args) => tree::run(args),
        Commands::Cat(args) => cat::run(args),
        Commands::Init(args) => init::run(args),
        Commands::Session(args) => session_cmd::run(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
