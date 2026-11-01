//! Handbook: real `--help` against `man/llm-bill.1`, and the documented example.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Cargo sets `CARGO_BIN_EXE_<name>` with the binary name unchanged, so this
/// package gets `CARGO_BIN_EXE_llm-bill`. Prefer `CARGO_BIN_EXE_llm_bill` when
/// that name is set as well.
fn binary() -> &'static str {
    if let Some(path) = option_env!("CARGO_BIN_EXE_llm_bill") {
        path
    } else {
        env!("CARGO_BIN_EXE_llm-bill")
    }
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(args)
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "0")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("spawn llm-bill")
}

/// Long options are whitespace tokens that start with `--`, including `--help`
/// and `--version` when the binary prints them.
fn long_options(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for raw in text.split_whitespace() {
        let token = raw.trim_matches(|c: char| {
            matches!(
                c,
                ',' | '.' | ';' | ':' | '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '<' | '>'
            )
        });
        let Some(rest) = token.strip_prefix("--") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if name.chars().any(|c| c.is_ascii_alphanumeric()) {
            found.insert(format!("--{name}"));
        }
    }
    found
}

/// Subcommands named under a `Commands:` heading in the top-level help.
fn subcommand_names(help: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_section = false;
    for line in help.lines() {
        if !in_section {
            if line == "Commands:" {
                in_section = true;
            }
            continue;
        }
        if line.trim().is_empty() {
            break;
        }
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !name.starts_with('-') {
            names.push(name.to_string());
        }
    }
    names
}

fn help_stdout(args: &[&str]) -> String {
    let out = run(args);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "args={args:?} stderr={stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    String::from_utf8(out.stdout).expect("help is utf-8")
}

#[test]
fn help_exits_zero_and_man_page_has_the_same_long_options() {
    let top = help_stdout(&["--help"]);
    let mut opts = long_options(&top);
    for name in subcommand_names(&top) {
        let sub = help_stdout(&[&name, "--help"]);
        opts.extend(long_options(&sub));
    }
    assert!(!opts.is_empty(), "{top}");

    let man_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("man/llm-bill.1");
    let man = std::fs::read_to_string(&man_path)
        .unwrap_or_else(|err| panic!("{}: {err}", man_path.display()));
    let man_opts = long_options(&man);
    assert_eq!(opts, man_opts, "help={opts:?}\nman={man_opts:?}");

    for section in [
        "NAME",
        "SYNOPSIS",
        "DESCRIPTION",
        "OPTIONS",
        "EXIT STATUS",
        "EXAMPLES",
    ] {
        assert!(
            man.contains(&format!(".SH {section}")),
            "man page missing {section}"
        );
    }
}

#[test]
fn unknown_option_exits_nonzero_without_panic() {
    let out = run(&["--not-a-handbook-option"]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
}

/// Command and exit code are the ones written in `examples/README.md`.
#[test]
fn documented_example_exits_as_written() {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/README.md");
    let readme = std::fs::read_to_string(&readme_path)
        .unwrap_or_else(|err| panic!("{}: {err}", readme_path.display()));

    let mut command = None;
    let mut fenced = false;
    for line in readme.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced && command.is_none() && trimmed.starts_with("llm-bill ") {
            command = Some(trimmed.to_string());
        }
    }
    let command = command.expect("examples/README.md has no llm-bill command in a fence");
    let args: Vec<String> = command
        .split_whitespace()
        .skip(1)
        .map(str::to_string)
        .collect();
    assert!(!args.is_empty(), "{command}");

    let exit = readme
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("Expected exit code:")
                .map(str::trim)
                .map(|n| n.parse::<i32>().expect("exit code"))
        })
        .expect("examples/README.md has no Expected exit code");

    for arg in &args {
        if !arg.starts_with('-') {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(arg);
            assert!(path.is_file(), "{}", path.display());
        }
    }

    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run(&argv);
    assert_eq!(
        out.status.code(),
        Some(exit),
        "command={command}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
}
