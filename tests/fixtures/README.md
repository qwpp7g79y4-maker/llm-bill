# Fixtures (SYNTHETISCH)

Alle bestanden hier zijn **synthetisch**: verzonnen voorbeelden, afgeleid van de
gedocumenteerde veldnamen. Modelnamen (`voorbeeld-model-a`, `voorbeeld-model-b`),
aantallen en bedragen zijn illustratief en geen echte klantdata. API-fixtures
dragen `"_synthetic"` en de bestandsnamen eindigen op `.synthetic.*`.

Er zijn nog geen CSV-fixtures: de dashboard-exports van OpenAI en Anthropic
hebben geen gedocumenteerde kolommen. Een echte export wordt later toegevoegd.

| Fixture | Inhoud |
|---|---|
| f01 | genormaliseerd JSONL, basis |
| f02 | batch-tier |
| f03 | cache-zwaar (OpenAI) |
| f04 | bron geeft een bedrag |
| f05 | lege dag, nul-tokens en defaults |
| f06a-e | foutgevallen: negatieve tokens, model ontbreekt, ongeldige dag, bedrag als float, onbekend model (prijs ontbreekt) |
| f07 | Anthropic usage-API-respons |
| f08 | Anthropic cost-API-respons (centen naar dollars) |
| f09 | OpenAI usage/completions-respons |
| f10 | `has_more=true` met `next_page` |
| f11 | OpenAI costs-respons (eenheid aanname: dollars) |
