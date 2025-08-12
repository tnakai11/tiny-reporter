use clap::{Parser, Subcommand};

/// A tiny reporter that periodically runs shell commands and records their output.
#[derive(Parser)]
#[command(name = "trep")]
#[command(author = "trep developers")]
#[command(version)]
#[command(about = "Periodically run commands and record their output to CSV or JSONL", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a command on a schedule and record its output
    Run(RunOpts),
}

/// Options for the run subcommand
#[derive(Parser, Debug)]
pub struct RunOpts {
    /// Name for this job (used for directory and file naming)
    #[arg(long = "as", short = 'n')]
    pub name: String,
    /// Interval at which to run the command (e.g. "1m", "10s"). If omitted, runs once.
    #[arg(long)]
    pub every: Option<String>,
    /// Output format: "csv" or "jsonl". Defaults to csv.
    #[arg(long, default_value = "csv")]
    pub format: String,
    /// Timeout for each command run (e.g. "5s"). Optional.
    #[arg(long)]
    pub timeout: Option<String>,
    /// Command to execute, use after `--` to separate from options
    #[arg(last = true, required = true)]
    pub cmd: Vec<String>,
}
