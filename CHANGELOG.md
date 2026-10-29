# Changelog

## 0.1.0 (unreleased)

- Normalised v0.1 JSONL input (day, provider, model, service tier, input/cache-read/cache-write/output tokens, requests, cost in dollars).
- Readers for saved Anthropic Usage & Cost Admin API responses and OpenAI Usage (completions) and Costs API responses.
- Text summary per day, provider, model and service tier, and `--json`.
- `--require-cost`: exit 2 when a row has tokens but no known cost.
- Strict errors (exit 2, no panics) for broken JSON, empty input, negative or huge numbers, bad dates, non-UTC timestamps.
- Output escapes control and bidirectional characters.
- Synthetic fixtures F1-F11 with an integration test each.
- Not yet: dashboard CSV (columns undocumented), Google and Bedrock, price file, 5m/1h cache split.
