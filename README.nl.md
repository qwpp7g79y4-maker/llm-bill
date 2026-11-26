# llm-bill

[![crates.io](https://img.shields.io/crates/v/llm-bill.svg)](https://crates.io/crates/llm-bill) [![downloads](https://img.shields.io/crates/d/llm-bill.svg)](https://crates.io/crates/llm-bill) [![CI](https://github.com/qwpp7g79y4-maker/llm-bill/actions/workflows/ci.yml/badge.svg)](https://github.com/qwpp7g79y4-maker/llm-bill/actions/workflows/ci.yml) [![PEPAI checked](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fqwpp7g79y4-maker%2Fllm-bill%2Fmain%2F.github%2Fbadge.json)](QUALITY.md) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://github.com/qwpp7g79y4-maker/llm-bill/blob/main/Cargo.toml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/qwpp7g79y4-maker/llm-bill#license)

Vat LLM-tokengebruik en -kosten samen per dag, aanbieder, model en servicetier,
uit opgeslagen API-antwoorden. Offline: het leest bestanden, doet geen
netwerkaanroepen en heeft geen API-sleutel nodig. Versie 0.1.0 (unreleased).

De tests en de gemeten getallen staan in `QUALITY.md`.

```
llm-bill [OPTIONS] <FILES>...
```

`--help` eindigt met exit 0 en print dit (Engels, zo is de binary):

```
Summarise LLM token usage and cost per provider, model and day from saved API responses or the normalised JSONL. Reads files only, no network

Usage: llm-bill [OPTIONS] <FILES>...

Arguments:
  <FILES>...  Input files; `-` reads standard input

Options:
      --json          Print JSON instead of the text table
      --require-cost  Exit 2 when a row has tokens but no known cost
  -h, --help          Print help
  -V, --version       Print version
```

`--version` print `llm-bill 0.1.0` en eindigt met exit 0. `-` leest stdin.
Elk bestand moet geldige UTF-8 zijn en hoogstens 268435456 bytes (256 MiB).

Exitcode 0 is gelukt. Exitcode 2 is foute invoer, fout gebruik (onbekende optie,
of geen bestanden) of een mislukte schrijfactie naar stdout. Invoer- en
schrijffouten gaan naar stderr en beginnen met `error:`. Gebruiksfouten komen
van de argumentparser en beginnen met `error:`. Een falende stderr wordt
genegeerd, dus geen paniek en geen exit 101 bij een volle of gesloten stdout of stderr.

## Installatie

Installeer met `cargo install --locked --path .` vanuit een clone van deze repository (Rust 1.85 of nieuwer).

## Aan de slag in 5 minuten

1. Clone de repository: `git clone https://github.com/qwpp7g79y4-maker/llm-bill` en `cd llm-bill`.
2. Installeer: `cargo install --locked --path .`
3. Draai het kleine voorbeeld: `llm-bill examples/one-row.json` (exit 0).
4. Print het tekstrapport van een synthetische fixture: `llm-bill tests/fixtures/f01-basic.synthetic.jsonl` (de uitvoer staat onder Voorbeeld).
5. Dezelfde samenvatting als JSON: `llm-bill --json tests/fixtures/f01-basic.synthetic.jsonl`.

## Voorbeeld

Tekstrapport van `tests/fixtures/f01-basic.synthetic.jsonl` (exit 0, stderr leeg):

```
$ llm-bill tests/fixtures/f01-basic.synthetic.jsonl
day         provider   model            tier      input  cache_read  cache_write  output     cost
2026-10-01  anthropic  example-model-a  standard   1500         200            0     500  unknown
total                                              1500         200            0     500  unknown
```

Dezelfde vorm met een bedrag dat de bron wél geeft (`tests/fixtures/f04-amount.synthetic.jsonl`):

```
$ llm-bill tests/fixtures/f04-amount.synthetic.jsonl
day         provider   model            tier      input  cache_read  cache_write  output   cost
2026-10-02  anthropic  example-model-a  standard      0           0            0       0  $1.23
total                                                 0           0            0       0  $1.23
```

Kolommen: day, provider, model, tier, input, cache_read, cache_write, output
en cost. Tekstkolommen links uitgelijnd, getalkolommen rechts. De laatste rij
is `total`. `requests` staat niet in de teksttabel; alleen in `--json`.
Meerdere bestanden worden opgeteld.

De kostencel is `$1.23` bij een volledig bedrag, `$1.23+` als maar een deel
van de tokenrecords in die rij een bedrag heeft, `unknown` bij tokens zonder
bekend bedrag, en `-` bij geen tokens en geen bedrag (er mist niets;
`f05-zero.synthetic.jsonl` print `-`).

`--json` print een pretty-printed document met `version` `"0.1"`, `warnings`,
`rows` en `totals`. Rijsleutels, alfabetisch, zijn `cache_read_tokens`,
`cache_write_tokens`, `cost_known`, `cost_usd`, `day`, `input_tokens`, `model`,
`output_tokens`, `provider`, `requests`, `service_tier`. `cost_usd` is `null`
of een decimale string. Het document eindigt op `}` en zet daar geen newline
achter. Waarschuwingen in het tekstrapport zijn regels `warning: ...` op
stdout, vóór de tabel. Bij `--json` staan die waarschuwingen in de array
`warnings` en ook op stderr.

Foute aanroep, allemaal exit 2, stdout leeg, stderr zonder paniek:

```
$ llm-bill --nope tests/fixtures/f01-basic.synthetic.jsonl
error: unexpected argument '--nope' found

  tip: to pass '--nope' as a value, use '-- --nope'

Usage: llm-bill [OPTIONS] <FILES>...

For more information, try '--help'.
```

```
$ llm-bill /nonexistent/llm-bill-test.jsonl
error: /nonexistent/llm-bill-test.jsonl: No such file or directory (os error 2)
```

```
$ printf '%s' '{"day":' | llm-bill -
error: line 1: invalid JSON: EOF while parsing a value at line 1 column 7
```

Stuurtekens (C0, DEL, C1 zoals U+0085) en bidi-tekens in data worden geëscaped als
`\u{..}` in tekst en als `\uXXXX` in `--json`.

## Invoerformaten

Het formaat wordt automatisch herkend.

1. **Genormaliseerd JSONL** (v0.1), één regel per (dag, aanbieder, model, tier):

   ```
   {"day":"2026-10-01","provider":"anthropic","model":"voorbeeld","service_tier":"standard",
    "input_tokens":1500,"cache_read_tokens":200,"cache_write_tokens":0,"output_tokens":500,
    "requests":3,"cost_usd":null}
   ```

   `day` is een UTC-datum. `provider` is `anthropic`, `openai` of `other`.
   `service_tier` (standaard `standard`) is `standard`, `batch`, `priority`, `flex`
   of `other`. `cache_read_tokens` en `cache_write_tokens` zijn standaard 0,
   `requests` en `cost_usd` mogen null zijn. `cost_usd` is een decimale **string**
   in dollars (een JSON-getal wordt afgewezen). Onbekende velden worden afgewezen.

2. **Anthropic Usage Admin API** (`GET /v1/organizations/usage_report/messages`):
   niet-gecachete input, cache-lezen, cache-schrijven (5m + 1h opgeteld), output, model, servicetier.
   <https://platform.claude.com/docs/en/api/beta/organization/usage_report/retrieve_messages>,
   <https://platform.claude.com/docs/en/manage-claude/usage-cost-api>

3. **Anthropic Cost Admin API** (`GET /v1/organizations/cost_report`): `amount` is
   een decimale string in centen en wordt omgerekend naar dollars (`"123.45"` is
   $1,2345). Alleen `cost_type` `tokens` wordt gelezen; andere kosttypen worden met een waarschuwing overgeslagen.
   <https://platform.claude.com/docs/en/api/beta/organization/cost_report/retrieve>

4. **OpenAI Usage API** (`GET /v1/organization/usage/completions`): `input_tokens`,
   `output_tokens`, `input_cached_tokens`, `input_cache_write_tokens`,
   `num_model_requests`, `model`, `service_tier`, `batch`. Buckets gebruiken Unix-seconden (UTC).
   <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage>

5. **OpenAI Costs API** (`GET /v1/organization/costs`): `amount.value` met valuta `usd`.
   De eenheid van `amount.value` is in de bron niet bevestigd; dollars zijn
   aangenomen en een waarschuwing meldt dat. Het antwoord heeft geen model, dus
   `line_item` staat in de modelkolom. De bron noemt geen eenheid voor `amount.value`
   (gecontroleerd in review llmbill-eind-0001), dus het blijft een aanname. Bron: <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage>.

Regels:

- Een veld dat de bron weglaat (niet op die dimensie
  gegroepeerd) staat als `unknown`. Tijdstippen die niet in UTC zijn worden afgewezen.
  OpenAI `amount.value` wordt als dollars gelezen. Die eenheid is een aanname, en
  een waarschuwing meldt dat.
- OpenAI `input_tokens` bevat de gecachete en cache-write tokens (de bron zegt
  "including cached and cache-write tokens"). De kolom `input` toont daarom
  `input_tokens - input_cached_tokens - input_cache_write_tokens`, de ongecachete input,
  zoals Anthropic's `uncached_input_tokens`. Een resultaat waarin de gecachete en
  cache-write tokens groter zijn dan `input_tokens` wordt geweigerd. Bron: de OpenAI Usage API-URL hierboven.
- Dubbele gegevens worden niet dubbel geteld: een invoer die identiek is aan een eerdere
  (hetzelfde bestand twee keer) wordt overgeslagen, net als een API-resultaat dat gelijk
  is aan een al geziene in een bucket met dezelfde start (herhaalde buckets, overlappende
  pagina's). Beide geven een waarschuwing. Genormaliseerde JSONL-regels worden nooit
  ontdubbeld: verschillende invoer met dezelfde sleutel wordt opgeteld.
- `has_more=true` in een antwoord geeft een waarschuwing: het bestand is één
  pagina van meer en de totalen zijn onvolledig.
- Kosten staan er alleen als de bron ze geeft. Zonder bedrag zegt de kostenkolom
  `unknown`; `--require-cost` maakt daar exitcode 2 van. Er is nog geen prijsbestand.
  Een rij waarvan maar een deel van de records met tokens een bedrag heeft, toont `$X+`
  (en `cost_known: false`), ook bij `--require-cost`. Een record met bedrag maar zonder
  tokens (de Anthropic Cost API naast de Usage API) dekt de usage-records van zijn rij.
- OpenAI-usage `batch: true` kiest tier `batch` en wint van `service_tier`.
  `batch: false` of null houdt `service_tier`. De API-waarde `default` is
  `standard`. Een servicetier-string die de API stuurt en die niet `standard`,
  `default`, `batch`, `priority` of `flex` is, wordt `other`. Een tier die de
  API weglaat wordt `unknown`. Genormaliseerd JSONL weigert een onbekende
  `service_tier` of `provider` met exit 2.
- Meer dan 12 decimalen, of een exponent (`1.4e-6`), wordt afgerond op 12
  decimalen. De run gaat door en bewaart één waarschuwing,
  `amount "…" rounded to 12 decimal places`. `1.4e-6` verschijnt als `$0.0000014`.
  Dit geldt voor decimale strings en voor OpenAI `amount.value`. Een
  JSON-getal in genormaliseerd `cost_usd` wordt nog steeds afgewezen.

## Nog niet

- Dashboard-CSV van OpenAI en Anthropic: de kolommen zijn niet gedocumenteerd.
  Ondersteuning volgt (later) zodra er een export in `tests/fixtures/` staat.
- Google en AWS Bedrock, een prijsbestand om kosten uit tokens te schatten, de
  5m/1h-cachesplitsing, thinking-tokens als apart veld, Priority Tier, web search en code execution.
- Ophalen bij de API's zelf (vraagt een admin-sleutel). Sla het antwoord op in een bestand.

Fixtures in `tests/fixtures/` zijn synthetisch. Geen echte klantdata.

## Onderhoud

Na een release staat de tool in sluimermodus (dormant): een nieuwe versie komt er alleen bij een gemelde bug, een verzoek of een beveiligingsprobleem. Beveiliging gaat altijd voor.

## Licentie

MIT OR Apache-2.0.

De licenties van de externe crates in het programma staan in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).
