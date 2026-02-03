use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "om")]
#[command(about = "LLM context tool that scores project files by importance", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Show project structure with scores")]
    Tree(TreeArgs),

    #[command(about = "Output file contents")]
    Cat(CatArgs),

    #[command(about = "Manage sessions")]
    Session(SessionArgs),
}

#[derive(Parser)]
pub struct TreeArgs {
    #[arg(help = "Project path (default: current directory)")]
    pub path: Option<String>,

    #[arg(
        short = 'j',
        long,
        default_value_t = 0,
        help = "Number of parallel jobs (0 = auto)"
    )]
    pub jobs: usize,

    #[arg(short = 's', long, help = "Minimum score (1-10)")]
    pub min_score: Option<i32>,

    #[arg(short, long, help = "Maximum depth")]
    pub depth: Option<usize>,

    #[arg(short, long, help = "Flat output instead of tree")]
    pub flat: bool,

    #[arg(long, help = "Disable colors")]
    pub no_color: bool,

    #[arg(long, help = "Show entire git repository (ignore path filtering)")]
    pub git_root: bool,

    #[arg(
        long,
        help = "Show only dirty files (modified, added, deleted, untracked)"
    )]
    pub dirty: bool,

    #[arg(long, help = "Show only staged files")]
    pub staged: bool,

    #[arg(long, help = "Show only unstaged files")]
    pub unstaged: bool,

    #[arg(long, help = "Output format: text, json, xml (default: text)")]
    pub format: Option<String>,

    #[arg(short, long, help = "Show token counts")]
    pub tokens: bool,
}

#[derive(Parser)]
pub struct CatArgs {
    #[arg(help = "Specific files to cat")]
    pub files: Vec<String>,

    #[arg(short = 'l', long, help = "Minimum score level (1-10, default: 5)")]
    pub level: Option<i32>,

    #[arg(short, long, help = "Project path (default: current directory)")]
    pub path: Option<String>,

    #[arg(long, help = "Disable headers")]
    pub no_headers: bool,

    #[arg(short = 'S', long, help = "Session name (overrides OM_SESSION env)")]
    pub session: Option<String>,

    #[arg(
        long,
        help = "Show only dirty files (modified, added, deleted, untracked)"
    )]
    pub dirty: bool,

    #[arg(long, help = "Show only staged files")]
    pub staged: bool,

    #[arg(long, help = "Show only unstaged files")]
    pub unstaged: bool,

    #[arg(long, help = "Output format: text, json, xml (default: text)")]
    pub format: Option<String>,

    #[arg(short, long, help = "Show token counts")]
    pub tokens: bool,
}

#[derive(Parser)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: Option<SessionCommand>,
}

#[derive(Subcommand)]
pub enum SessionCommand {
    #[command(about = "Clear session")]
    Clear {
        #[arg(help = "Session name")]
        name: String,
    },
}
