# Quality

pep: 6.5

## Open points that count

| Point | Class | Deduction |
|---|---|---|
| Test fixture and model names are still Dutch (for example f04-bedrag and voorbeeld-onbekend-model), and tests/fixtures/README.md is in Dutch | S- | 3.5 |

Score: 10 − 3.5 = 6.5. Other open points in this file have no class and do not count. They are not repaired yet.

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
