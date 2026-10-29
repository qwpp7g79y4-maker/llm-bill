use clap::Parser;

/// Summarise LLM token usage and cost from saved API responses.
#[derive(Parser)]
#[command(version)]
struct Args {}

fn main() {
    let _ = Args::parse();
}
