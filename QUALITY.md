# Quality

pep: 8.6

Command: `cargo test --locked --no-fail-fast`
Date: 2026-11-23

## Result

39 tests passed, 0 failed, 0 ignored (36 in `tests/cli.rs`, 3 in `tests/handbook_help.rs`;
unit tests and doc-tests: 0).

STAND: green

## Not run

- `cargo clippy` and `cargo fmt --check`
- Code coverage (no percentage measured)
- Release build and tests against real provider APIs (the tool is offline)
