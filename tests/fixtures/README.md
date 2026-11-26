# Fixtures (SYNTHETIC)

Every file here is **synthetic**: made-up examples, derived from the documented
field names. Model names (`example-model-a`, `example-model-b`), counts and
amounts are illustrative and are not real customer data. API fixtures carry
`"_synthetic"` and the file names end in `.synthetic.*`.

There are no CSV fixtures yet: the dashboard exports of OpenAI and Anthropic
have no documented columns. A real export will be added later.

| Fixture | Content |
|---|---|
| f01 | normalized JSONL, basic |
| f02 | batch tier |
| f03 | cache-heavy (OpenAI) |
| f04 | the source gives an amount |
| f05 | empty day, zero tokens and defaults |
| f06a-e | error cases: negative tokens, missing model, invalid day, amount as a float, unknown model (no price) |
| f07 | Anthropic usage API response |
| f08 | Anthropic cost API response (cents to dollars) |
| f09 | OpenAI usage/completions response |
| f10 | `has_more=true` with `next_page` |
| f11 | OpenAI costs response (unit assumed: dollars) |
