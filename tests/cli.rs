//! Integration tests. Every fixture under tests/fixtures is SYNTHETIC.

use std::io::Write;
use std::process::{Command, Stdio};

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn run_with(args: &[&str], stdin: Option<&[u8]>) -> Out {
    let mut child = Command::new(env!("CARGO_BIN_EXE_llm-bill"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    if let Some(data) = stdin {
        pipe.write_all(data).unwrap();
    }
    drop(pipe);
    let o = child.wait_with_output().unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// Run on a fixture with `--json` and return the parsed document.
fn json_of(name: &str) -> serde_json::Value {
    let o = run_with(&["--json", &fixture(name)], None);
    assert_eq!(o.code, 0, "{}", o.stderr);
    serde_json::from_str(&o.stdout).unwrap()
}

fn row(doc: &serde_json::Value, i: usize, key: &str) -> String {
    doc["rows"][i][key]
        .to_string()
        .trim_matches('"')
        .to_string()
}

#[test]
fn f01_basis() {
    let d = json_of("f01-basis.synthetic.jsonl");
    assert_eq!(d["rows"].as_array().unwrap().len(), 1);
    assert_eq!(row(&d, 0, "model"), "voorbeeld-model-a");
    assert_eq!(row(&d, 0, "input_tokens"), "1500");
    assert_eq!(row(&d, 0, "cache_read_tokens"), "200");
    assert_eq!(row(&d, 0, "output_tokens"), "500");
    assert_eq!(row(&d, 0, "requests"), "3");
    assert!(d["rows"][0]["cost_usd"].is_null());
    assert_eq!(d["rows"][0]["cost_known"], false);
}

#[test]
fn f02_batch() {
    let d = json_of("f02-batch.synthetic.jsonl");
    assert_eq!(row(&d, 0, "service_tier"), "batch");
    assert_eq!(row(&d, 0, "input_tokens"), "10000");
}

#[test]
fn f03_cache_heavy() {
    let d = json_of("f03-cache.synthetic.jsonl");
    assert_eq!(row(&d, 0, "provider"), "openai");
    assert_eq!(row(&d, 0, "cache_read_tokens"), "9000");
    assert_eq!(row(&d, 0, "cache_write_tokens"), "700");
}

#[test]
fn f04_source_gives_amount() {
    let d = json_of("f04-bedrag.synthetic.jsonl");
    assert_eq!(row(&d, 0, "cost_usd"), "1.23");
    assert_eq!(d["rows"][0]["cost_known"], true);
    assert_eq!(d["totals"]["cost_usd"], "1.23");
    assert_eq!(d["totals"]["cost_complete"], true);
}

#[test]
fn f05_zero_and_defaults() {
    let d = json_of("f05-nul.synthetic.jsonl");
    assert_eq!(row(&d, 0, "service_tier"), "standard");
    assert_eq!(row(&d, 0, "cache_read_tokens"), "0");
    assert_eq!(row(&d, 0, "cache_write_tokens"), "0");
    assert!(d["rows"][0]["requests"].is_null());
    // Zero tokens and no cost is complete, not "unknown".
    assert_eq!(d["rows"][0]["cost_known"], true);
}

#[test]
fn f06_error_cases_exit_2() {
    for (name, needle) in [
        ("f06a-negatief.synthetic.jsonl", "negatief"),
        ("f06b-geen-model.synthetic.jsonl", "model"),
        ("f06c-dag.synthetic.jsonl", "ongeldige dag"),
        ("f06d-float.synthetic.jsonl", "cost_usd"),
    ] {
        let o = run_with(&[&fixture(name)], None);
        assert_eq!(o.code, 2, "{name}");
        assert!(o.stderr.contains(needle), "{name}: {}", o.stderr);
        assert!(o.stdout.is_empty(), "{name}");
        assert!(!o.stderr.contains("panicked"), "{name}");
    }
}

#[test]
fn f06e_unknown_model_price() {
    let name = fixture("f06e-onbekend-model.synthetic.jsonl");
    let ok = run_with(&[&name], None);
    assert_eq!(ok.code, 0);
    assert!(ok.stdout.contains("onbekend"));
    let strict = run_with(&["--require-cost", &name], None);
    assert_eq!(strict.code, 2);
    assert!(strict.stderr.contains("voorbeeld-onbekend-model"));
}

#[test]
fn f07_anthropic_usage() {
    let d = json_of("f07-anthropic-usage.synthetic.json");
    assert_eq!(row(&d, 0, "day"), "2026-10-01");
    assert_eq!(row(&d, 0, "input_tokens"), "1500");
    assert_eq!(row(&d, 0, "cache_read_tokens"), "200");
    // 5m (30) and 1h (12) are summed in v0.1.
    assert_eq!(row(&d, 0, "cache_write_tokens"), "42");
    assert_eq!(row(&d, 0, "output_tokens"), "500");
}

#[test]
fn f08_anthropic_cost_cents_to_dollars() {
    let d = json_of("f08-anthropic-cost.synthetic.json");
    assert_eq!(row(&d, 0, "cost_usd"), "1.2345");
    assert_eq!(row(&d, 0, "provider"), "anthropic");
    let text = run_with(&[&fixture("f08-anthropic-cost.synthetic.json")], None);
    assert!(text.stdout.contains("$1.2345"));
}

#[test]
fn f09_openai_usage() {
    let d = json_of("f09-openai-usage.synthetic.json");
    // 1790812800 is 2026-10-01T00:00:00Z.
    assert_eq!(row(&d, 0, "day"), "2026-10-01");
    assert_eq!(row(&d, 0, "provider"), "openai");
    // OpenAI input_tokens (9300) includes the cached tokens (9000): 300 uncached.
    assert_eq!(row(&d, 0, "input_tokens"), "300");
    assert_eq!(row(&d, 0, "cache_read_tokens"), "9000");
    assert_eq!(row(&d, 0, "requests"), "10");
}

#[test]
fn f10_has_more_warns() {
    let o = run_with(&["--json", &fixture("f10-has-more.synthetic.json")], None);
    assert_eq!(o.code, 0);
    assert!(o.stderr.contains("has_more"));
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert!(d["warnings"][0].as_str().unwrap().contains("has_more"));
    let text = run_with(&[&fixture("f10-has-more.synthetic.json")], None);
    assert!(text.stdout.contains("waarschuwing: has_more"));
}

#[test]
fn f11_openai_costs_with_assumption_warning() {
    let o = run_with(
        &["--json", &fixture("f11-openai-costs.synthetic.json")],
        None,
    );
    assert_eq!(o.code, 0);
    assert!(o.stderr.contains("niet bevestigd"));
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["rows"][0]["cost_usd"], "0.06");
}

#[test]
fn combined_files_sum_per_day_model() {
    // Two different inputs for the same (day, provider, model, tier) are summed.
    let a = fixture("f01-basis.synthetic.jsonl");
    let second = std::fs::read_to_string(&a)
        .unwrap()
        .replace("\"input_tokens\":1500", "\"input_tokens\":100");
    let o = run_with(&["--json", &a, "-"], Some(second.as_bytes()));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["rows"].as_array().unwrap().len(), 1);
    assert_eq!(d["rows"][0]["input_tokens"], 1600);
    assert_eq!(d["totals"]["output_tokens"], 1000);
    assert_eq!(d["warnings"].as_array().unwrap().len(), 0);
}

#[test]
fn text_summary_has_columns_and_total() {
    let o = run_with(&[&fixture("f01-basis.synthetic.jsonl")], None);
    assert_eq!(o.code, 0);
    let first = o.stdout.lines().next().unwrap();
    for col in [
        "day", "provider", "model", "tier", "input", "output", "cost",
    ] {
        assert!(first.contains(col), "{col}");
    }
    assert!(o.stdout.lines().last().unwrap().starts_with("totaal"));
}

// ---- error handling ------------------------------------------------------

fn expect_fail(input: &[u8], needle: &str) {
    let o = run_with(&["-"], Some(input));
    assert_eq!(o.code, 2, "{}", String::from_utf8_lossy(input));
    assert!(o.stderr.contains(needle), "{}", o.stderr);
    assert!(!o.stderr.contains("panicked"));
    assert!(o.stdout.is_empty());
}

#[test]
fn broken_json() {
    expect_fail(b"{\"day\":\n", "ongeldige JSON");
    expect_fail(b"not json at all", "ongeldige JSON");
    expect_fail(b"{\"data\":[{\"starting_at\":", "ongeldige JSON");
}

#[test]
fn empty_input() {
    expect_fail(b"", "lege invoer");
    expect_fail(b"  \n\n", "lege invoer");
    expect_fail(b"{\"data\":[]}", "geen gegevens");
}

#[test]
fn missing_file() {
    let o = run_with(&["/nonexistent/llm-bill-test.jsonl"], None);
    assert_eq!(o.code, 2);
    assert!(o.stderr.starts_with("fout:"));
}

#[test]
fn negative_and_huge_numbers() {
    let rec = |tokens: &str| {
        format!(
            "{{\"day\":\"2026-10-01\",\"provider\":\"other\",\"model\":\"m\",\"input_tokens\":{tokens},\"output_tokens\":0}}\n"
        )
    };
    expect_fail(rec("-1").as_bytes(), "negatief");
    expect_fail(rec("18446744073709551616").as_bytes(), "bereik");
    expect_fail(rec("1e30").as_bytes(), "geheel getal");
    expect_fail(rec("1.5").as_bytes(), "geheel getal");
    expect_fail(rec("\"7\"").as_bytes(), "geheel getal");
    // Each value fits in u64 but the sum overflows.
    let two = format!("{}{}", rec("18446744073709551615"), rec("1"));
    expect_fail(two.as_bytes(), "overflow");
    // Money: negative, too large, too precise, exponent.
    for cost in [
        "-1.00",
        "99999999999999999999.00",
        "1.1234567890123",
        "1e3",
        ".5",
        "1.",
    ] {
        let line = format!(
            "{{\"day\":\"2026-10-01\",\"provider\":\"other\",\"model\":\"m\",\"input_tokens\":0,\"output_tokens\":0,\"cost_usd\":\"{cost}\"}}\n"
        );
        expect_fail(line.as_bytes(), "bedrag");
    }
}

#[test]
fn api_negative_and_huge_numbers() {
    let doc = |n: &str| {
        format!(
            "{{\"data\":[{{\"starting_at\":\"2026-10-01T00:00:00Z\",\"results\":[{{\"model\":\"m\",\"uncached_input_tokens\":{n},\"cache_read_input_tokens\":0,\"cache_creation\":{{\"ephemeral_5m_input_tokens\":0,\"ephemeral_1h_input_tokens\":0}},\"output_tokens\":0}}]}}]}}"
        )
    };
    expect_fail(doc("-3").as_bytes(), "negatief");
    expect_fail(doc("99999999999999999999999").as_bytes(), "bereik");
    expect_fail(
        b"{\"data\":[{\"starting_at\":\"2026-10-01T00:00:00+02:00\",\"results\":[]}]}",
        "UTC",
    );
    expect_fail(b"{\"data\":[{\"results\":[]}]}", "starting_at");
}

#[test]
fn unknown_fields_and_values_are_not_guessed() {
    let base = |extra: &str| {
        format!(
            "{{\"day\":\"2026-10-01\",\"provider\":\"other\",\"model\":\"m\",\"input_tokens\":1,\"output_tokens\":1{extra}}}\n"
        )
    };
    expect_fail(base(",\"thinking_tokens\":5").as_bytes(), "onbekend veld");
    expect_fail(
        base(",\"service_tier\":\"gold\"").as_bytes(),
        "service_tier",
    );
    expect_fail(
        b"{\"day\":\"2026-10-01\",\"provider\":\"acme\",\"model\":\"m\",\"input_tokens\":1,\"output_tokens\":1}",
        "provider",
    );
}

#[test]
fn control_characters_are_escaped() {
    // ESC, newline and a right-to-left override inside the model name.
    let line = "{\"day\":\"2026-10-01\",\"provider\":\"other\",\"model\":\"a\\u001b[31mb\\nc\\u202ed\",\"input_tokens\":1,\"output_tokens\":1}\n";
    let o = run_with(&["-"], Some(line.as_bytes()));
    assert_eq!(o.code, 0);
    assert!(!o.stdout.contains('\u{1b}'));
    assert!(!o.stdout.contains('\u{202e}'));
    assert!(o.stdout.contains("\\u{1b}"));
    assert!(o.stdout.contains("\\u{a}"));
    // Same in an error message.
    let bad = "{\"day\":\"2026-10-01\\u001b\",\"provider\":\"other\",\"model\":\"m\",\"input_tokens\":1,\"output_tokens\":1}\n";
    let e = run_with(&["-"], Some(bad.as_bytes()));
    assert_eq!(e.code, 2);
    assert!(!e.stderr.contains('\u{1b}'));
    // JSON output escapes them as well.
    let j = run_with(&["--json", "-"], Some(line.as_bytes()));
    assert!(!j.stdout.contains('\u{1b}'));
    assert!(j.stdout.contains("\\u001b"));
}

#[test]
fn leap_day_and_unix_days() {
    let ok = "{\"day\":\"2028-02-29\",\"provider\":\"other\",\"model\":\"m\",\"input_tokens\":1,\"output_tokens\":1}\n";
    assert_eq!(run_with(&["-"], Some(ok.as_bytes())).code, 0);
    let bad = ok.replace("2028", "2026");
    expect_fail(bad.as_bytes(), "ongeldige dag");
    // 1790899199 is the last second of 2026-10-01, 1790899200 the first of 10-02.
    for (secs, day) in [(1790899199u64, "2026-10-01"), (1790899200, "2026-10-02")] {
        let doc = format!(
            "{{\"data\":[{{\"start_time\":{secs},\"results\":[{{\"input_tokens\":1,\"output_tokens\":1,\"model\":\"m\"}}]}}]}}"
        );
        let o = run_with(&["-"], Some(doc.as_bytes()));
        assert_eq!(o.code, 0, "{}", o.stderr);
        assert!(o.stdout.contains(day), "{}", o.stdout);
    }
}

#[test]
fn b2_json_and_text_escape_c0_c1_and_bidi() {
    // ESC (C0), U+0085 (C1) and U+202E (bidi) inside the model name.
    let line = "{\"day\":\"2026-10-01\",\"provider\":\"other\",\"model\":\"a\\u001b[31m\\u0085\\u202eb\",\"input_tokens\":1,\"output_tokens\":1}\n";
    for args in [&["-"][..], &["--json", "-"][..]] {
        let o = run_with(args, Some(line.as_bytes()));
        assert_eq!(o.code, 0, "{}", o.stderr);
        for bad in ['\u{1b}', '\u{85}', '\u{202e}'] {
            assert!(!o.stdout.contains(bad), "{args:?}: raw {bad:?}");
        }
    }
    let j = run_with(&["--json", "-"], Some(line.as_bytes()));
    assert!(j.stdout.contains("\\u0085"), "{}", j.stdout);
    assert!(j.stdout.contains("\\u202e"), "{}", j.stdout);
    // The escaped JSON still parses back to the original model name.
    let d: serde_json::Value = serde_json::from_str(&j.stdout).unwrap();
    assert_eq!(d["rows"][0]["model"], "a\u{1b}[31m\u{85}\u{202e}b");
    let t = run_with(&["-"], Some(line.as_bytes()));
    assert!(t.stdout.contains("\\u{85}"), "{}", t.stdout);
}

fn line(day: &str, tokens: u64, cost: Option<&str>) -> String {
    let cost = cost.map_or(String::new(), |c| format!(",\"cost_usd\":\"{c}\""));
    format!(
        "{{\"day\":\"{day}\",\"provider\":\"other\",\"model\":\"m\",\"input_tokens\":{tokens},\"output_tokens\":0{cost}}}\n"
    )
}

#[test]
fn b3_partial_cost_row_is_incomplete() {
    // Same row: one record with tokens and cost, one with tokens and no cost.
    let input = format!(
        "{}{}",
        line("2026-10-01", 10, Some("5.00")),
        line("2026-10-01", 20, None)
    );
    let j = run_with(&["--json", "-"], Some(input.as_bytes()));
    assert_eq!(j.code, 0, "{}", j.stderr);
    let d: serde_json::Value = serde_json::from_str(&j.stdout).unwrap();
    assert_eq!(d["rows"][0]["cost_usd"], "5.00");
    assert_eq!(d["rows"][0]["cost_known"], false);
    assert_eq!(d["totals"]["cost_complete"], false);
    let t = run_with(&["-"], Some(input.as_bytes()));
    assert!(t.stdout.contains("$5.00+"), "{}", t.stdout);
    let strict = run_with(&["--require-cost", "-"], Some(input.as_bytes()));
    assert_eq!(strict.code, 2);
    assert!(strict.stdout.is_empty());
    assert!(
        strict.stderr.contains("--require-cost"),
        "{}",
        strict.stderr
    );
}

#[test]
fn b3_cost_only_record_still_covers_usage_only_record() {
    // Usage from one source and cost from another (no tokens) stay complete.
    let input = format!(
        "{}{}",
        line("2026-10-01", 20, None),
        line("2026-10-01", 0, Some("5.00"))
    );
    let o = run_with(&["--require-cost", "--json", "-"], Some(input.as_bytes()));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["rows"][0]["cost_known"], true);
}

#[test]
fn b4_same_file_twice_is_counted_once_with_warning() {
    let a = fixture("f01-basis.synthetic.jsonl");
    let o = run_with(&["--json", &a, &a], None);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["rows"][0]["input_tokens"], 1500);
    assert_eq!(d["totals"]["output_tokens"], 500);
    assert!(d["warnings"][0].as_str().unwrap().contains("identiek"));
    assert!(o.stderr.contains("identiek"), "{}", o.stderr);
    let t = run_with(&[&a, &a], None);
    assert!(t.stdout.contains("waarschuwing: "), "{}", t.stdout);
}

#[test]
fn b4_duplicate_bucket_result_is_counted_once_with_warning() {
    let r = "{\"model\":\"m\",\"uncached_input_tokens\":10,\"cache_read_input_tokens\":0,\"cache_creation\":{\"ephemeral_5m_input_tokens\":0,\"ephemeral_1h_input_tokens\":0},\"output_tokens\":5}";
    let bucket = format!("{{\"starting_at\":\"2026-10-01T00:00:00Z\",\"results\":[{r}]}}");
    let doc = format!("{{\"data\":[{bucket},{bucket}]}}");
    let o = run_with(&["--json", "-"], Some(doc.as_bytes()));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["rows"][0]["input_tokens"], 10);
    assert_eq!(d["rows"][0]["output_tokens"], 5);
    assert!(d["warnings"][0].as_str().unwrap().contains("dubbel"));
    // Same result in a later bucket (other start) is real usage and is summed.
    let later = bucket.replace("2026-10-01", "2026-10-02");
    let doc = format!("{{\"data\":[{bucket},{later}]}}");
    let o = run_with(&["--json", "-"], Some(doc.as_bytes()));
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["totals"]["input_tokens"], 20);
    assert_eq!(d["warnings"].as_array().unwrap().len(), 0);
}

/// Run with stdout and/or stderr replaced by /dev/full (every write fails with ENOSPC).
#[cfg(target_os = "linux")]
fn run_full(args: &[&str], full_stdout: bool, full_stderr: bool) -> Out {
    let full = || {
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap()
    };
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_llm-bill"));
    cmd.args(args).stdin(Stdio::null());
    cmd.stdout(if full_stdout {
        Stdio::from(full())
    } else {
        Stdio::piped()
    });
    cmd.stderr(if full_stderr {
        Stdio::from(full())
    } else {
        Stdio::piped()
    });
    let o = cmd.output().unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn b5_stdout_write_error_is_nonzero_without_panic() {
    if !std::path::Path::new("/dev/full").exists() {
        return;
    }
    let f = fixture("f01-basis.synthetic.jsonl");
    for args in [&[f.as_str()][..], &["--json", f.as_str()][..]] {
        let o = run_full(args, true, false);
        assert_eq!(o.code, 2, "{args:?}: {}", o.stderr);
        assert!(o.stderr.starts_with("fout:"), "{}", o.stderr);
        assert!(!o.stderr.contains("panicked"), "{}", o.stderr);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn b5_stderr_write_error_does_not_panic() {
    if !std::path::Path::new("/dev/full").exists() {
        return;
    }
    // Error path: exit 2, not 101.
    let o = run_full(&["/nonexistent/llm-bill-test.jsonl"], false, true);
    assert_eq!(o.code, 2);
    // Warning path (--json prints warnings on stderr): output still succeeds.
    let f = fixture("f10-has-more.synthetic.json");
    let o = run_full(&["--json", f.as_str()], false, true);
    assert_eq!(o.code, 0);
    assert!(o.stdout.contains("has_more"));
}

fn openai_doc(input: u64, cached: u64, written: u64) -> String {
    format!(
        "{{\"data\":[{{\"start_time\":1790812800,\"results\":[{{\"input_tokens\":{input},\"output_tokens\":1,\"input_cached_tokens\":{cached},\"input_cache_write_tokens\":{written},\"model\":\"m\"}}]}}]}}"
    )
}

#[test]
fn b6_openai_input_excludes_cached_and_cache_write() {
    let o = run_with(
        &["--json", "-"],
        Some(openai_doc(1000, 600, 100).as_bytes()),
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    assert_eq!(d["rows"][0]["input_tokens"], 300);
    assert_eq!(d["rows"][0]["cache_read_tokens"], 600);
    assert_eq!(d["rows"][0]["cache_write_tokens"], 100);
    // Without cache fields nothing is subtracted.
    let plain = "{\"data\":[{\"start_time\":1790812800,\"results\":[{\"input_tokens\":7,\"output_tokens\":1,\"model\":\"m\"}]}]}";
    let d: serde_json::Value =
        serde_json::from_str(&run_with(&["--json", "-"], Some(plain.as_bytes())).stdout).unwrap();
    assert_eq!(d["rows"][0]["input_tokens"], 7);
    // Cached plus written above input contradicts the source: refuse, do not wrap.
    let o = run_with(&["-"], Some(openai_doc(100, 90, 20).as_bytes()));
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("input_tokens"), "{}", o.stderr);
    assert!(o.stdout.is_empty());
}

#[test]
fn b6_openai_amount_unit_is_a_documented_assumption() {
    let o = run_with(
        &["--json", &fixture("f11-openai-costs.synthetic.json")],
        None,
    );
    let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
    let w = d["warnings"][0].as_str().unwrap();
    assert!(w.contains("aangenomen dollars"), "{w}");
    for readme in [
        include_str!("../README.md"),
        include_str!("../README.nl.md"),
    ] {
        assert!(readme.contains("cache-write"));
        assert!(readme.contains("organization/subresources/usage"));
    }
    assert!(include_str!("../src/input.rs").contains("organization/subresources/usage"));
}

#[test]
fn n1_openai_batch_true_maps_to_batch_tier() {
    let doc = |batch: &str| {
        format!(
            "{{\"data\":[{{\"start_time\":1790812800,\"results\":[{{\"input_tokens\":1,\"output_tokens\":1,\"model\":\"m\",\"batch\":{batch},\"service_tier\":\"default\"}}]}}]}}"
        )
    };
    let tier = |batch: &str| {
        let o = run_with(&["--json", "-"], Some(doc(batch).as_bytes()));
        assert_eq!(o.code, 0, "{}", o.stderr);
        let d: serde_json::Value = serde_json::from_str(&o.stdout).unwrap();
        d["rows"][0]["service_tier"].as_str().unwrap().to_string()
    };
    assert_eq!(tier("true"), "batch");
    assert_eq!(tier("false"), "standard");
    assert_eq!(tier("null"), "standard");
}
