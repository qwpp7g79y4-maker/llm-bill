# llm-bill

Summarise LLM token usage and cost per day, provider, model and service tier from
saved API responses. Offline: it reads files, makes no network calls and needs no
API key. Version 0.1.0 (unreleased).

```
llm-bill [--json] [--require-cost] FILE...     # "-" reads stdin
```

Output is a text table (day, provider, model, tier, input, cache_read,
cache_write, output, cost) with a total row, or `--json`. Several files are
summed. Exit code 0 is success, 2 is bad input, bad usage or a failed
write to stdout (a message on stderr). A failing stderr is ignored, so there is no
panic and no exit 101 when stdout or stderr is full or closed. Control characters
(C0, DEL, C1 such as U+0085) and bidirectional characters in data are escaped as
`\u{..}` in text and as `\uXXXX` in `--json`.

## Input formats

The format is detected automatically.

1. **Normalised JSONL** (v0.1), one line per (day, provider, model, tier):

   ```
   {"day":"2026-10-01","provider":"anthropic","model":"example","service_tier":"standard",
    "input_tokens":1500,"cache_read_tokens":200,"cache_write_tokens":0,"output_tokens":500,
    "requests":3,"cost_usd":null}
   ```

   `day` is a UTC date. `provider` is `anthropic`, `openai` or `other`.
   `service_tier` (default `standard`) is `standard`, `batch`, `priority`, `flex` or
   `other`. `cache_read_tokens` and `cache_write_tokens` default to 0, `requests`
   and `cost_usd` may be null. `cost_usd` is a decimal **string** in dollars (a
   JSON number is rejected). Unknown fields are rejected.

2. **Anthropic Usage Admin API** (`GET /v1/organizations/usage_report/messages`):
   uncached input, cache read, cache creation (5m + 1h summed), output tokens, model, service tier.
   <https://platform.claude.com/docs/en/api/beta/organization/usage_report/retrieve_messages>,
   <https://platform.claude.com/docs/en/manage-claude/usage-cost-api>

3. **Anthropic Cost Admin API** (`GET /v1/organizations/cost_report`): `amount` is a
   decimal string in cents and is converted to dollars (`"123.45"` is $1.2345). Only
   `cost_type` `tokens` is read; other cost types are skipped with a warning.
   <https://platform.claude.com/docs/en/api/beta/organization/cost_report/retrieve>

4. **OpenAI Usage API** (`GET /v1/organization/usage/completions`): `input_tokens`,
   `output_tokens`, `input_cached_tokens`, `input_cache_write_tokens`,
   `num_model_requests`, `model`, `service_tier`, `batch`. Buckets use Unix seconds (UTC).
   <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage>

5. **OpenAI Costs API** (`GET /v1/organization/costs`): `amount.value` with currency `usd`.
   The unit of `amount.value` is not confirmed in the source; dollars are assumed
   and a warning says so. The response has no model, so `line_item` is shown as the model.
   The source states no unit for `amount.value` (checked in review llmbill-eind-0001),
   so it stays an assumption. Source: <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage>.

Rules:

- Nothing is guessed. A field the source leaves out (not grouped on that
  dimension) shows as `onbekend`. Timestamps that are not UTC are rejected.
- OpenAI `input_tokens` includes the cached and cache-write tokens (the source says
  "including cached and cache-write tokens"). The `input` column therefore shows
  `input_tokens - input_cached_tokens - input_cache_write_tokens`, the uncached input,
  like Anthropic's `uncached_input_tokens`. A result where the cached and cache-write
  tokens exceed `input_tokens` is rejected. Source: the OpenAI Usage API URL above.
- Duplicates are not counted twice: an input identical to an earlier one (the same
  file twice) is skipped, and so is an API result identical to one already seen in a
  bucket with the same start (repeated buckets, overlapping pages). Each gives a
  warning. Normalised JSONL lines are never de-duplicated: different inputs with
  the same key are summed.
- `has_more=true` in a response gives a warning: the file is one page of several
  and the totals are incomplete.
- Cost is only shown when the source gives it. Without it the cost column says
  `onbekend`; `--require-cost` turns that into exit code 2. There is no price file yet.
  A row where only some of the records with tokens carry a cost shows `$X+` (and
  `cost_known: false`), also under `--require-cost`. A record with cost but no tokens
  (the Anthropic Cost API next to the Usage API) covers the usage records of its row.

## Not (yet)

- Dashboard CSV exports of OpenAI and Anthropic: their columns are not
  documented. Support follows when an export is added to `tests/fixtures/`.
- Google and AWS Bedrock, a price file to estimate cost from tokens,
  the 5m/1h cache split, thinking tokens as a separate field, Priority Tier,
  web search and code execution costs.
- Fetching from the provider APIs (needs an admin key). Save the response to a file.

Fixtures in `tests/fixtures/` are synthetic. No real customer data.

## License

MIT OR Apache-2.0.
