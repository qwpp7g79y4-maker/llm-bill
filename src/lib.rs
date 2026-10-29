//! llm-bill: summarise LLM usage and cost from saved API responses.
//! Offline only: it reads files, it never calls a provider.

pub mod input;
pub mod model;
pub mod report;

use model::{err, esc, Result};

pub struct Options {
    pub json: bool,
    /// Fail (exit 2) when a row has tokens but no cost from the source.
    pub require_cost: bool,
}

/// Run on already-read input texts and return the rendered output.
pub fn run(texts: &[String], opts: &Options) -> Result<(String, Vec<String>)> {
    let mut records = Vec::new();
    let mut warnings = Vec::new();
    for t in texts {
        let p = input::parse(t)?;
        records.extend(p.records);
        warnings.extend(p.warnings);
    }
    let rows = report::aggregate(&records)?;
    if opts.require_cost {
        if let Some(r) = rows.iter().find(|r| !r.cost_known) {
            return err(format!(
                "geen prijs of kosten bekend voor model {:?} op {} (--require-cost)",
                esc(&r.model),
                r.day
            ));
        }
    }
    let t = report::totals(&rows)?;
    let out = if opts.json {
        report::render_json(&rows, &t, &warnings)
    } else {
        report::render_text(&rows, &t, &warnings)
    };
    Ok((out, warnings))
}
