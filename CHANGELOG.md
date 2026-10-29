# Changelog

## 0.1.0 (unreleased)

- Normalised v0.1 JSONL input (day, provider, model, service tier, input/cache-read/cache-write/output tokens, requests, cost in dollars).
- Readers for saved Anthropic Usage & Cost Admin API responses and OpenAI Usage (completions) and Costs API responses.
- OpenAI `input_tokens` is reported as uncached input (minus cached and cache-write tokens), as the source says it includes them.
- Text summary per day, provider, model and service tier, and `--json`.
- `--require-cost`: exit 2 when a row has tokens but no known cost.
- Strict errors (exit 2) for broken JSON, empty input, negative or huge numbers, bad dates, non-UTC timestamps.
- Output escapes control (C0, DEL, C1) and bidirectional characters, also in `--json`.
- A failed write to stdout gives exit 2; a failing stderr is ignored (no panic).
- Synthetic fixtures F1-F11 with an integration test each.
- Not yet: dashboard CSV (columns undocumented), Google and Bedrock, price file, 5m/1h cache split.
