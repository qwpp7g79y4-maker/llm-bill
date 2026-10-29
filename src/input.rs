//! Readers: normalised JSONL, Anthropic usage/cost reports, OpenAI usage/costs.
//!
//! Anything the scan marks `onbekend` is an error or the literal "onbekend",
//! never a guess. Unknown timezones are rejected.

use crate::model::{err, esc, Day, Money, Provider, Record, Result, Tier};
use serde_json::{Map, Value};
use std::collections::HashSet;

type Obj = Map<String, Value>;

/// Placeholder for fields the source leaves out (not grouped on that dimension).
pub const UNKNOWN: &str = "onbekend";

#[derive(Debug, Default)]
pub struct Parsed {
    pub records: Vec<Record>,
    pub warnings: Vec<String>,
}

/// Parse one input text. Auto-detects an API response (`{"data":[...]}`) or JSONL.
pub fn parse(text: &str) -> Result<Parsed> {
    parse_with(text, &mut HashSet::new())
}

/// Like [`parse`], sharing `seen` between inputs: an API result that is identical to
/// one already seen in the same bucket (same start) is skipped with a warning, so
/// repeated buckets and overlapping pages are not counted twice.
pub fn parse_with(text: &str, seen: &mut HashSet<String>) -> Result<Parsed> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return err("lege invoer");
    }
    let mut out = Parsed::default();
    if trimmed.starts_with('{') {
        if let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(trimmed) {
            if doc.contains_key("data") {
                parse_api(&doc, seen, &mut out)?;
                return finish(out);
            }
        }
    }
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let n = i + 1;
        let v: Value = serde_json::from_str(line).map_err(|e| {
            crate::model::Error(format!(
                "regel {n}: ongeldige JSON: {}",
                esc(&e.to_string())
            ))
        })?;
        let rec =
            parse_normalised(&v).map_err(|e| crate::model::Error(format!("regel {n}: {e}")))?;
        out.records.push(rec);
    }
    finish(out)
}

fn finish(out: Parsed) -> Result<Parsed> {
    if out.records.is_empty() {
        return err("geen gegevens gevonden in de invoer");
    }
    Ok(out)
}

// ---- field helpers -------------------------------------------------------

fn as_obj<'a>(v: &'a Value, what: &str) -> Result<&'a Obj> {
    v.as_object()
        .map_or_else(|| err(format!("{what}: verwacht een object")), Ok)
}

fn get<'a>(o: &'a Obj, key: &str) -> Option<&'a Value> {
    o.get(key).filter(|v| !v.is_null())
}

fn uint_value(v: &Value, key: &str) -> Result<u64> {
    let Value::Number(n) = v else {
        return err(format!("{key}: verwacht een geheel getal"));
    };
    let text = n.to_string();
    if text.starts_with('-') {
        return err(format!(
            "{key}: negatief getal {} niet toegestaan",
            esc(&text)
        ));
    }
    text.parse::<u64>().map_err(|_| {
        crate::model::Error(format!(
            "{key}: {} is geen geheel getal binnen bereik (u64)",
            esc(&text)
        ))
    })
}

fn uint(o: &Obj, key: &str) -> Result<u64> {
    match get(o, key) {
        Some(v) => uint_value(v, key),
        None => err(format!("{key}: ontbreekt")),
    }
}

fn uint_or(o: &Obj, key: &str, default: u64) -> Result<u64> {
    get(o, key).map_or(Ok(default), |v| uint_value(v, key))
}

fn opt_str<'a>(o: &'a Obj, key: &str) -> Result<Option<&'a str>> {
    match get(o, key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => err(format!("{key}: verwacht een string")),
    }
}

fn req_str<'a>(o: &'a Obj, key: &str) -> Result<&'a str> {
    opt_str(o, key)?.map_or_else(|| err(format!("{key}: ontbreekt")), Ok)
}

fn model_name(s: Option<&str>) -> Result<String> {
    match s {
        None => Ok(UNKNOWN.to_string()),
        Some(m) if m.trim().is_empty() => err("model: leeg"),
        Some(m) => Ok(m.to_string()),
    }
}

fn tier_name(s: Option<&str>) -> Tier {
    s.map_or(Tier::Unknown, Tier::from_api)
}

// ---- normalised JSONL ----------------------------------------------------

const NORMALISED_KEYS: [&str; 10] = [
    "day",
    "provider",
    "model",
    "service_tier",
    "input_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "output_tokens",
    "requests",
    "cost_usd",
];

fn parse_normalised(v: &Value) -> Result<Record> {
    let o = as_obj(v, "record")?;
    if let Some(k) = o.keys().find(|k| !NORMALISED_KEYS.contains(&k.as_str())) {
        return err(format!("onbekend veld {:?}", esc(k)));
    }
    let provider = match req_str(o, "provider")? {
        "anthropic" => Provider::Anthropic,
        "openai" => Provider::Openai,
        "other" => Provider::Other,
        p => return err(format!("provider: onbekende waarde {:?}", esc(p))),
    };
    let tier = match opt_str(o, "service_tier")? {
        None | Some("standard") => Tier::Standard,
        Some("batch") => Tier::Batch,
        Some("priority") => Tier::Priority,
        Some("flex") => Tier::Flex,
        Some("other") => Tier::Other,
        Some(t) => return err(format!("service_tier: onbekende waarde {:?}", esc(t))),
    };
    let cost_usd = match get(o, "cost_usd") {
        None => None,
        Some(Value::String(s)) => Some(Money::parse(s, 0)?),
        Some(_) => return err("cost_usd: verwacht een decimale string, geen getal"),
    };
    let model = req_str(o, "model")?;
    Ok(Record {
        day: Day::parse(req_str(o, "day")?)?,
        provider,
        model: model_name(Some(model))?,
        tier,
        input_tokens: uint(o, "input_tokens")?,
        cache_read_tokens: uint_or(o, "cache_read_tokens", 0)?,
        cache_write_tokens: uint_or(o, "cache_write_tokens", 0)?,
        output_tokens: uint(o, "output_tokens")?,
        requests: get(o, "requests")
            .map(|v| uint_value(v, "requests"))
            .transpose()?,
        cost_usd,
    })
}

// ---- API responses -------------------------------------------------------

fn parse_api(doc: &Obj, seen: &mut HashSet<String>, out: &mut Parsed) -> Result<()> {
    let Some(Value::Array(buckets)) = doc.get("data") else {
        return err("data: verwacht een lijst met buckets");
    };
    if doc.get("has_more").and_then(Value::as_bool) == Some(true) {
        out.warnings.push(
            "has_more=true: de respons is niet compleet (volgende pagina ontbreekt); totalen zijn onvolledig"
                .to_string(),
        );
    }
    let mut skipped = 0usize;
    let mut duplicates = 0usize;
    for (i, b) in buckets.iter().enumerate() {
        let ctx = |e: crate::model::Error| crate::model::Error(format!("bucket {}: {e}", i + 1));
        let b = as_obj(b, "bucket").map_err(ctx)?;
        let (day, provider) = bucket_day(b).map_err(ctx)?;
        let Some(Value::Array(results)) = b.get("results") else {
            return err(format!("bucket {}: results: verwacht een lijst", i + 1));
        };
        let start = b
            .get("starting_at")
            .or_else(|| b.get("start_time"))
            .map_or_else(String::new, Value::to_string);
        for r in results {
            let r = as_obj(r, "result").map_err(ctx)?;
            // Map keys are ordered, so equal results serialise identically.
            if !seen.insert(format!("{start}|{}", Value::Object(r.clone()))) {
                duplicates += 1;
                continue;
            }
            let rec = match (provider, r.contains_key("amount")) {
                (Provider::Anthropic, false) => anthropic_usage(day, r).map(Some),
                (Provider::Anthropic, true) => anthropic_cost(day, r),
                (_, false) => openai_usage(day, r).map(Some),
                (_, true) => openai_cost(day, r, &mut out.warnings),
            };
            match rec.map_err(ctx)? {
                Some(rec) => out.records.push(rec),
                None => skipped += 1,
            }
        }
    }
    if duplicates > 0 {
        out.warnings.push(format!(
            "{duplicates} dubbel(e) resultaat/resultaten in dezelfde bucket overgeslagen (niet dubbel geteld)"
        ));
    }
    if skipped > 0 {
        out.warnings.push(format!(
            "{skipped} kostenregel(s) overgeslagen: alleen cost_type=tokens valt in v0.1"
        ));
    }
    Ok(())
}

/// Day and provider of a bucket. Anthropic buckets use RFC 3339, OpenAI Unix seconds.
fn bucket_day(b: &Obj) -> Result<(Day, Provider)> {
    if let Some(s) = opt_str(b, "starting_at")? {
        let utc = s.ends_with('Z') || s.ends_with("+00:00");
        if s.len() < 20 || s.as_bytes().get(10) != Some(&b'T') || !s.is_char_boundary(10) || !utc {
            return err(format!(
                "starting_at {:?}: verwacht RFC 3339 in UTC (Z); andere tijdzones worden niet geraden",
                esc(s)
            ));
        }
        return Ok((Day::parse(&s[..10])?, Provider::Anthropic));
    }
    match get(b, "start_time") {
        Some(Value::Number(n)) => {
            let secs = n.to_string().parse::<i64>().map_err(|_| {
                crate::model::Error(format!(
                    "start_time {}: verwacht Unix-seconden (geheel getal)",
                    esc(&n.to_string())
                ))
            })?;
            Ok((Day::from_unix(secs)?, Provider::Openai))
        }
        Some(_) => err("start_time: verwacht Unix-seconden (geheel getal)"),
        None => err("bucket heeft geen starting_at (Anthropic) of start_time (OpenAI)"),
    }
}

fn anthropic_usage(day: Day, r: &Obj) -> Result<Record> {
    let Some(cc) = get(r, "cache_creation") else {
        return err("cache_creation: ontbreekt");
    };
    let cc = as_obj(cc, "cache_creation")?;
    let write = uint(cc, "ephemeral_5m_input_tokens")?
        .checked_add(uint(cc, "ephemeral_1h_input_tokens")?)
        .map_or_else(|| err("cache_creation: som past niet in u64"), Ok)?;
    Ok(Record {
        day,
        provider: Provider::Anthropic,
        model: model_name(opt_str(r, "model")?)?,
        tier: tier_name(opt_str(r, "service_tier")?),
        input_tokens: uint(r, "uncached_input_tokens")?,
        cache_read_tokens: uint(r, "cache_read_input_tokens")?,
        cache_write_tokens: write,
        output_tokens: uint(r, "output_tokens")?,
        requests: None,
        cost_usd: None,
    })
}

/// Cost record. `amount` is a decimal string in cents (scan §1.2); converted to dollars.
fn anthropic_cost(day: Day, r: &Obj) -> Result<Option<Record>> {
    if opt_str(r, "cost_type")? != Some("tokens") {
        return Ok(None);
    }
    if req_str(r, "currency")? != "USD" {
        return err("currency: alleen USD wordt ondersteund");
    }
    let cost = Money::parse(req_str(r, "amount")?, 2)?;
    Ok(Some(cost_record(
        day,
        Provider::Anthropic,
        model_name(opt_str(r, "model")?)?,
        tier_name(opt_str(r, "service_tier")?),
        cost,
    )))
}

fn cost_record(day: Day, provider: Provider, model: String, tier: Tier, cost: Money) -> Record {
    Record {
        day,
        provider,
        model,
        tier,
        input_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        output_tokens: 0,
        requests: None,
        cost_usd: Some(cost),
    }
}

/// OpenAI usage result. Per the source, `input_tokens` is "the aggregated number of
/// input tokens used, including cached and cache-write tokens", so the uncached input
/// is `input_tokens - input_cached_tokens - input_cache_write_tokens`.
/// <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage>
fn openai_usage(day: Day, r: &Obj) -> Result<Record> {
    let tier = if r.get("batch").and_then(Value::as_bool) == Some(true) {
        Tier::Batch
    } else {
        tier_name(opt_str(r, "service_tier")?)
    };
    let input = uint(r, "input_tokens")?;
    let cached = uint_or(r, "input_cached_tokens", 0)?;
    let written = uint_or(r, "input_cache_write_tokens", 0)?;
    let uncached = cached
        .checked_add(written)
        .and_then(|c| input.checked_sub(c))
        .map_or_else(
            || {
                err(format!(
                    "input_tokens: {input} is kleiner dan input_cached_tokens {cached} + input_cache_write_tokens {written}; de bron zegt dat input_tokens die bevat"
                ))
            },
            Ok,
        )?;
    Ok(Record {
        day,
        provider: Provider::Openai,
        model: model_name(opt_str(r, "model")?)?,
        tier,
        input_tokens: uncached,
        cache_read_tokens: cached,
        cache_write_tokens: written,
        output_tokens: uint(r, "output_tokens")?,
        requests: get(r, "num_model_requests")
            .map(|v| uint_value(v, "num_model_requests"))
            .transpose()?,
        cost_usd: None,
    })
}

/// Costs result. The source gives `amount.value` as a number with a separate
/// `currency` (`usd`) but does not state the unit, so dollars are an assumption and a
/// warning says so. Source (usage and costs reference, checked in review llmbill-eind-0001):
/// <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage>
fn openai_cost(day: Day, r: &Obj, warnings: &mut Vec<String>) -> Result<Option<Record>> {
    let amount = as_obj(r.get("amount").unwrap_or(&Value::Null), "amount")?;
    if !req_str(amount, "currency")?.eq_ignore_ascii_case("usd") {
        return err("amount.currency: alleen usd wordt ondersteund");
    }
    let Some(Value::Number(n)) = get(amount, "value") else {
        return err("amount.value: verwacht een getal");
    };
    let cost = Money::parse(&n.to_string(), 0)?;
    let note =
        "OpenAI amount.value: eenheid niet bevestigd in de bron, aangenomen dollars".to_string();
    if !warnings.contains(&note) {
        warnings.push(note);
    }
    // The costs API has no model field; the line item is the closest label.
    Ok(Some(cost_record(
        day,
        Provider::Openai,
        model_name(opt_str(r, "line_item")?)?,
        Tier::Unknown,
        cost,
    )))
}
