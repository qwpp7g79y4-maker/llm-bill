//! Aggregation per (day, provider, model, tier) and output rendering.

use crate::model::{err, esc, is_unsafe, Day, Money, Provider, Record, Result, Tier};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub day: Day,
    pub provider: Provider,
    pub model: String,
    pub tier: Tier,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub requests: Option<u64>,
    pub cost_usd: Option<Money>,
    /// False when the row has tokens but no cost from the source.
    pub cost_known: bool,
}

type Key = (Day, Provider, String, Tier);

fn add(a: u64, b: u64, what: &str) -> Result<u64> {
    a.checked_add(b)
        .map_or_else(|| err(format!("overflow while adding {what}")), Ok)
}

pub fn aggregate(records: &[Record]) -> Result<Vec<Row>> {
    let mut map: BTreeMap<Key, Row> = BTreeMap::new();
    // Per row: (has a record with tokens and cost, has a record with tokens and no cost).
    let mut mixed: BTreeMap<Key, (bool, bool)> = BTreeMap::new();
    for r in records {
        let key = (r.day, r.provider, r.model.clone(), r.tier);
        if r.has_tokens() {
            let m = mixed.entry(key.clone()).or_default();
            if r.cost_usd.is_some() {
                m.0 = true;
            } else {
                m.1 = true;
            }
        }
        let row = map.entry(key).or_insert_with(|| Row {
            day: r.day,
            provider: r.provider,
            model: r.model.clone(),
            tier: r.tier,
            input_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            output_tokens: 0,
            requests: None,
            cost_usd: None,
            cost_known: true,
        });
        row.input_tokens = add(row.input_tokens, r.input_tokens, "input_tokens")?;
        row.cache_read_tokens = add(
            row.cache_read_tokens,
            r.cache_read_tokens,
            "cache_read_tokens",
        )?;
        row.cache_write_tokens = add(
            row.cache_write_tokens,
            r.cache_write_tokens,
            "cache_write_tokens",
        )?;
        row.output_tokens = add(row.output_tokens, r.output_tokens, "output_tokens")?;
        if let Some(n) = r.requests {
            row.requests = Some(add(row.requests.unwrap_or(0), n, "requests")?);
        }
        if let Some(c) = r.cost_usd {
            row.cost_usd = Some(
                row.cost_usd
                    .unwrap_or(Money::ZERO)
                    .checked_add(c)
                    .map_or_else(|| err("overflow while adding amounts"), Ok)?,
            );
        }
    }
    let mut rows: Vec<Row> = Vec::with_capacity(map.len());
    for (key, mut row) in map {
        // Tokens recorded both with and without cost: the sum covers only part of them.
        let partial = mixed.get(&key).is_some_and(|m| m.0 && m.1);
        let has_tokens = [
            row.input_tokens,
            row.cache_read_tokens,
            row.cache_write_tokens,
            row.output_tokens,
        ]
        .iter()
        .any(|&t| t > 0);
        row.cost_known = (row.cost_usd.is_some() || !has_tokens) && !partial;
        rows.push(row);
    }
    Ok(rows)
}

pub struct Totals {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<Money>,
    pub cost_complete: bool,
}

pub fn totals(rows: &[Row]) -> Result<Totals> {
    let mut t = Totals {
        input_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        output_tokens: 0,
        cost_usd: None,
        cost_complete: true,
    };
    for r in rows {
        t.input_tokens = add(t.input_tokens, r.input_tokens, "input_tokens")?;
        t.cache_read_tokens = add(
            t.cache_read_tokens,
            r.cache_read_tokens,
            "cache_read_tokens",
        )?;
        t.cache_write_tokens = add(
            t.cache_write_tokens,
            r.cache_write_tokens,
            "cache_write_tokens",
        )?;
        t.output_tokens = add(t.output_tokens, r.output_tokens, "output_tokens")?;
        if let Some(c) = r.cost_usd {
            t.cost_usd = Some(
                t.cost_usd
                    .unwrap_or(Money::ZERO)
                    .checked_add(c)
                    .map_or_else(|| err("overflow while adding amounts"), Ok)?,
            );
        }
        t.cost_complete &= r.cost_known;
    }
    Ok(t)
}

fn cost_json(c: Option<Money>) -> Value {
    c.map_or(Value::Null, |m| Value::String(m.to_string()))
}

pub fn render_json(rows: &[Row], t: &Totals, warnings: &[String]) -> String {
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "day": r.day.to_string(),
                "provider": r.provider.as_str(),
                "model": r.model,
                "service_tier": r.tier.as_str(),
                "input_tokens": r.input_tokens,
                "cache_read_tokens": r.cache_read_tokens,
                "cache_write_tokens": r.cache_write_tokens,
                "output_tokens": r.output_tokens,
                "requests": r.requests,
                "cost_usd": cost_json(r.cost_usd),
                "cost_known": r.cost_known,
            })
        })
        .collect();
    let doc = json!({
        "version": "0.1",
        "warnings": warnings,
        "rows": rows,
        "totals": {
            "input_tokens": t.input_tokens,
            "cache_read_tokens": t.cache_read_tokens,
            "cache_write_tokens": t.cache_write_tokens,
            "output_tokens": t.output_tokens,
            "cost_usd": cost_json(t.cost_usd),
            "cost_complete": t.cost_complete,
        },
    });
    // serde_json escapes C0 controls but leaves DEL, C1 (U+0085) and bidi characters
    // raw. Outside strings the document is ASCII, so rewriting those as `\uXXXX`
    // keeps it valid and parses back to the same value. Pretty-print newlines stay.
    let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c >= '\u{7f}' && is_unsafe(c) {
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

fn cost_text(c: Option<Money>, known: bool) -> String {
    match (c, known) {
        (Some(m), true) => format!("${m}"),
        (Some(m), false) => format!("${m}+"),
        (None, true) => "-".to_string(),
        (None, false) => "unknown".to_string(),
    }
}

pub fn render_text(rows: &[Row], t: &Totals, warnings: &[String]) -> String {
    let head = [
        "day",
        "provider",
        "model",
        "tier",
        "input",
        "cache_read",
        "cache_write",
        "output",
        "cost",
    ];
    let mut table: Vec<Vec<String>> = vec![head.iter().map(|s| s.to_string()).collect()];
    for r in rows {
        table.push(vec![
            r.day.to_string(),
            r.provider.as_str().to_string(),
            esc(&r.model),
            r.tier.as_str().to_string(),
            r.input_tokens.to_string(),
            r.cache_read_tokens.to_string(),
            r.cache_write_tokens.to_string(),
            r.output_tokens.to_string(),
            cost_text(r.cost_usd, r.cost_known),
        ]);
    }
    table.push(vec![
        "total".to_string(),
        String::new(),
        String::new(),
        String::new(),
        t.input_tokens.to_string(),
        t.cache_read_tokens.to_string(),
        t.cache_write_tokens.to_string(),
        t.output_tokens.to_string(),
        cost_text(t.cost_usd, t.cost_complete),
    ]);
    let widths: Vec<usize> = (0..head.len())
        .map(|c| {
            table
                .iter()
                .map(|row| row[c].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for w in warnings {
        out.push_str(&format!("warning: {}\n", esc(w)));
    }
    for row in &table {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(c, s)| {
                let pad = " ".repeat(widths[c] - s.chars().count());
                // Text columns left-aligned, number columns right-aligned.
                if c < 4 {
                    format!("{s}{pad}")
                } else {
                    format!("{pad}{s}")
                }
            })
            .collect();
        out.push_str(cells.join("  ").trim_end());
        out.push('\n');
    }
    out
}
