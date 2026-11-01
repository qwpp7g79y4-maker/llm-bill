# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Package version in `Cargo.toml` is 0.1.0. Nothing has been tagged or published.
Dates below are the git author dates on `main`.

### Added

- Project skeleton: Cargo package, empty CLI and `.gitignore` (2026-10-29).
- Normalised v0.1 JSONL (day, provider, model, service tier, input, cache-read,
  cache-write, output tokens, requests, cost in dollars as a decimal string).
  Readers for saved Anthropic Usage and Cost Admin API responses and for saved
  OpenAI Usage (completions) and Costs API responses. Text table and `--json`.
  `--require-cost` exits 2 when a row has tokens but no known cost (2026-10-29).
- OpenAI usage field `batch: true` selects service tier `batch` (reader from the
  start; integration test N1 on 2026-10-29).
- Eleven synthetic fixtures (F1–F11) and an integration test for each (2026-10-29).
- English `README.md`, Dutch `README.nl.md`, this changelog, and MIT OR Apache-2.0
  (2026-10-29).
- Integration tests: `--help` exits 0; an unknown option, a missing file and
  broken input exit non-zero with an error on stderr and without a panic.
- Handbook: roff manual `man/llm-bill.1`, fictional `examples/one-row.json`
  with the command and exit code in `examples/README.md`, and
  `tests/handbook_help.rs`. `README.md` points at the manual (2026-11-01).

### Changed

- OpenAI `input_tokens` is reported as uncached input (minus cached and
  cache-write tokens), because the source says `input_tokens` includes them.
  A result where cached plus cache-write exceeds `input_tokens` is rejected.
  The OpenAI `amount.value` unit is still an assumption of dollars, with a
  warning and the source URL in the README (2026-10-29, B6).
- Amounts with more than 12 decimal places, and amounts written with an
  exponent, are rounded to 12 decimal places and the run continues with a
  warning, instead of aborting (2026-10-29, G-1).

### Fixed

- Control characters (C0, DEL, C1) and bidirectional characters are escaped in
  `--json` as well as in the text table (2026-10-29, B2).
- A row that has token records both with and without a cost is incomplete
  (`$X+`, `cost_known: false`) and fails `--require-cost`. A cost-only record
  still covers the usage records of its row (2026-10-29, B3).
- An input identical to an earlier input, and an API result identical to one
  already seen in a bucket with the same start, are skipped with a warning and
  are not counted twice (2026-10-29, B4).
- A failed write to stdout exits 2 with `fout: schrijven naar stdout mislukt`.
  A failed write to stderr does not panic (2026-10-29, B5).
- The rounding warning is stored once in the warnings list, including under
  `--json` (2026-10-30, G-2a).

### Not in this version

- Dashboard CSV (columns undocumented), Google and AWS Bedrock, a price file,
  the 5m/1h cache split as separate columns, and fetching from the provider APIs.
