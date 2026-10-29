# llm-bill

Vat LLM-tokengebruik en -kosten samen per dag, aanbieder, model en servicetier,
uit opgeslagen API-antwoorden. Offline: het leest bestanden, doet geen
netwerkaanroepen en heeft geen API-sleutel nodig. Versie 0.1.0 (unreleased).

```
llm-bill [--json] [--require-cost] BESTAND...     # "-" leest stdin
```

De uitvoer is een tekstabel (day, provider, model, tier, input, cache_read,
cache_write, output, cost) met een totaalregel, of `--json`. Meerdere bestanden
worden opgeteld. Exitcode 0 is gelukt, 2 is foute invoer, fout gebruik of een
mislukte schrijfactie naar stdout (een melding op stderr). Een falende stderr wordt
genegeerd, dus geen paniek en geen exit 101 bij een volle of gesloten stdout of stderr.
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
   `line_item` staat in de modelkolom. Zelfde URL als hierboven.

Regels:

- Er wordt niets geraden. Een veld dat de bron weglaat (niet op die dimensie
  gegroepeerd) staat als `onbekend`. Tijdstippen die niet in UTC zijn worden afgewezen.
- OpenAI `input_tokens` staat zoals gemeld; of de gecachete tokens daarin zitten
  staat niet in de gebruikte bron, vergelijk dus zelf met `cache_read`.
- Dubbele gegevens worden niet dubbel geteld: een invoer die identiek is aan een eerdere
  (hetzelfde bestand twee keer) wordt overgeslagen, net als een API-resultaat dat gelijk
  is aan een al geziene in een bucket met dezelfde start (herhaalde buckets, overlappende
  pagina's). Beide geven een waarschuwing. Genormaliseerde JSONL-regels worden nooit
  ontdubbeld: verschillende invoer met dezelfde sleutel wordt opgeteld.
- `has_more=true` in een antwoord geeft een waarschuwing: het bestand is één
  pagina van meer en de totalen zijn onvolledig.
- Kosten staan er alleen als de bron ze geeft. Zonder bedrag zegt de kostenkolom
  `onbekend`; `--require-cost` maakt daar exitcode 2 van. Er is nog geen prijsbestand.
  Een rij waarvan maar een deel van de records met tokens een bedrag heeft, toont `$X+`
  (en `cost_known: false`), ook bij `--require-cost`. Een record met bedrag maar zonder
  tokens (de Anthropic Cost API naast de Usage API) dekt de usage-records van zijn rij.

## Nog niet

- Dashboard-CSV van OpenAI en Anthropic: de kolommen zijn niet gedocumenteerd.
  Ondersteuning volgt (later) zodra er een export in `tests/fixtures/` staat.
- Google en AWS Bedrock, een prijsbestand om kosten uit tokens te schatten, de
  5m/1h-cachesplitsing, thinking-tokens als apart veld, Priority Tier, web search en code execution.
- Ophalen bij de API's zelf (vraagt een admin-sleutel). Sla het antwoord op in een bestand.

Fixtures in `tests/fixtures/` zijn synthetisch. Geen echte klantdata.

## Licentie

MIT OR Apache-2.0.
