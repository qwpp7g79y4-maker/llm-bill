use clap::Parser;
use llm_bill::{model::esc, run, Options};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

/// Largest input accepted per file.
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Summarise LLM token usage and cost per provider, model and day from
/// saved API responses or the normalised JSONL. Reads files only, no network.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Input files; `-` reads standard input
    #[arg(required = true)]
    files: Vec<PathBuf>,
    /// Print JSON instead of the text table
    #[arg(long)]
    json: bool,
    /// Exit 2 when a row has tokens but no known cost
    #[arg(long)]
    require_cost: bool,
}

fn read(path: &PathBuf) -> Result<String, String> {
    let mut buf = Vec::new();
    let res = if path.as_os_str() == "-" {
        std::io::stdin().take(MAX_BYTES + 1).read_to_end(&mut buf)
    } else {
        std::fs::File::open(path).and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut buf))
    };
    let name = esc(&path.to_string_lossy());
    res.map_err(|e| format!("{name}: {}", esc(&e.to_string())))?;
    if buf.len() as u64 > MAX_BYTES {
        return Err(format!("{name}: bestand groter dan {MAX_BYTES} bytes"));
    }
    String::from_utf8(buf).map_err(|_| format!("{name}: geen geldige UTF-8"))
}

/// Print a line on stderr. A failing stderr is ignored: `eprintln!` would panic.
fn note(line: &str) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

fn main() -> ExitCode {
    let args = Args::parse();
    let result = args
        .files
        .iter()
        .map(read)
        .collect::<Result<Vec<_>, _>>()
        .and_then(|texts| {
            let opts = Options {
                json: args.json,
                require_cost: args.require_cost,
            };
            run(&texts, &opts).map_err(|e| e.to_string())
        });
    match result {
        Ok((out, warnings)) => {
            let mut stdout = std::io::stdout().lock();
            if let Err(e) = stdout
                .write_all(out.as_bytes())
                .and_then(|()| stdout.flush())
            {
                note(&format!(
                    "fout: schrijven naar stdout mislukt: {}",
                    esc(&e.to_string())
                ));
                return ExitCode::from(2);
            }
            if args.json {
                for w in &warnings {
                    note(&format!("waarschuwing: {}", esc(w)));
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            note(&format!("fout: {e}"));
            ExitCode::from(2)
        }
    }
}
