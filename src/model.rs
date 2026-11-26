//! Normalised v0.1 record and its checked scalar types.

use std::cell::RefCell;
use std::fmt;

/// Error for bad input; the CLI maps it to exit code 2.
#[derive(Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error(msg.into()))
}

/// Characters that must never reach a terminal raw: C0, DEL, C1 (U+0080..U+009F)
/// and the bidirectional formatting characters.
pub fn is_unsafe(c: char) -> bool {
    let bidi =
        matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
    c.is_control() || bidi
}

/// Make user-supplied text safe to print: no raw control or bidi characters.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_unsafe(c) {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// A UTC calendar day, validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day {
    year: u16,
    month: u8,
    day: u8,
}

fn is_leap(y: u16) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

impl Day {
    pub fn new(year: u16, month: u8, day: u8) -> Option<Day> {
        let max = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap(year) => 29,
            2 => 28,
            _ => return None,
        };
        (year >= 1 && (1..=max).contains(&day)).then_some(Day { year, month, day })
    }

    /// Parse exactly `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Result<Day> {
        let b = s.as_bytes();
        let ok = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !ok {
            return err(format!("invalid day {:?}, expected YYYY-MM-DD", esc(s)));
        }
        let n = |r: std::ops::Range<usize>| s[r].parse::<u16>().unwrap_or(0);
        Day::new(n(0..4), n(5..7) as u8, n(8..10) as u8)
            .map_or_else(|| err(format!("invalid day {:?}", esc(s))), Ok)
    }

    /// UTC day of a Unix timestamp in seconds.
    pub fn from_unix(secs: i64) -> Result<Day> {
        let z = secs.div_euclid(86_400) + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        u16::try_from(y)
            .ok()
            .and_then(|y| Day::new(y, m as u8, d as u8))
            .map_or_else(|| err(format!("timestamp {secs} out of range")), Ok)
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Dollar amount as fixed-point integer, 12 decimal places. Never a float.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(i128);

const SCALE_DIGITS: usize = 12;

impl Money {
    pub const ZERO: Money = Money(0);

    /// Parse a non-negative decimal (`123`, `1.23`) or scientific form (`1.4e-6`).
    /// No sign. More than 12 decimal places is rounded; at most one warning is
    /// kept for the run. `shift` divides by 10^shift (2 converts cents to dollars).
    pub fn parse(s: &str, shift: usize) -> Result<Money> {
        let (mantissa, exp) = scientific(s)?;
        let Some((int, frac)) = plain_decimal(mantissa) else {
            return Err(invalid_money(s));
        };
        // Exact path stays silent. Exponents and extra digits round instead of aborting.
        if exp == 0 && frac.len() + shift <= SCALE_DIGITS {
            return parse_exact(s, int, frac, shift);
        }
        parse_rounded(s, int, frac, exp, shift)
    }

    pub fn checked_add(self, o: Money) -> Option<Money> {
        self.0.checked_add(o.0).map(Money)
    }
}

impl fmt::Display for Money {
    /// Decimal dollars, at least two places, trailing zeros trimmed beyond that.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = 10i128.pow(SCALE_DIGITS as u32);
        let frac = format!("{:012}", self.0 % unit);
        let frac = frac.trim_end_matches('0');
        write!(f, "{}.{:0<2}", self.0 / unit, frac)
    }
}

fn invalid_money(s: &str) -> Error {
    Error(format!(
        "invalid amount {:?}: expected a non-negative decimal number",
        esc(s)
    ))
}

fn too_precise(s: &str) -> Error {
    Error(format!("amount {:?} too large or too precise", esc(s)))
}

/// Mantissa and base-10 exponent. No exponent is `0`.
fn scientific(s: &str) -> Result<(&str, i32)> {
    let Some(i) = s.bytes().position(|c| c == b'e' || c == b'E') else {
        return Ok((s, 0));
    };
    if s.bytes().skip(i + 1).any(|c| c == b'e' || c == b'E') {
        return Err(invalid_money(s));
    }
    let mantissa = &s[..i];
    let mut exp = &s[i + 1..];
    if mantissa.is_empty() || exp.is_empty() {
        return Err(invalid_money(s));
    }
    let neg = if let Some(rest) = exp.strip_prefix('+') {
        exp = rest;
        false
    } else if let Some(rest) = exp.strip_prefix('-') {
        exp = rest;
        true
    } else {
        false
    };
    if exp.is_empty() || !exp.bytes().all(|c| c.is_ascii_digit()) {
        return Err(invalid_money(s));
    }
    // A huge positive power cannot fit; a huge negative one rounds to zero later.
    if exp.len() > 9 {
        return if neg {
            Ok((mantissa, -1_000_000_000))
        } else {
            Err(too_precise(s))
        };
    }
    let value: i32 = exp.parse().map_err(|_| too_precise(s))?;
    Ok((mantissa, if neg { -value } else { value }))
}

/// Integer and fraction parts of a plain non-negative decimal. `None` if the shape is wrong.
fn plain_decimal(s: &str) -> Option<(&str, &str)> {
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let digits = |t: &str| t.bytes().all(|c| c.is_ascii_digit());
    if int.is_empty() || !digits(int) || !digits(frac) || (s.contains('.') && frac.is_empty()) {
        return None;
    }
    Some((int, frac))
}

fn parse_exact(s: &str, int: &str, frac: &str, shift: usize) -> Result<Money> {
    if int.len() > 18 || frac.len() + shift > SCALE_DIGITS {
        return Err(too_precise(s));
    }
    // value * 10^(12 - shift): int digits followed by frac padded to 12 - shift places.
    let places = SCALE_DIGITS - shift;
    let mut v: i128 = int.parse().unwrap_or(0);
    for i in 0..places {
        v = v * 10 + i128::from(frac.as_bytes().get(i).map_or(0, |c| c - b'0'));
    }
    Ok(Money(v))
}

// At most one rounding note on this thread. `run` clears it on entry and
// takes it once after the input loop.
thread_local! {
    static ROUNDED_WARNING: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn clear_rounded_warning() {
    ROUNDED_WARNING.with(|w| *w.borrow_mut() = None);
}

pub(crate) fn take_rounded_warning() -> Option<String> {
    ROUNDED_WARNING.with(|w| w.borrow_mut().take())
}

fn warn_rounded(s: &str) {
    ROUNDED_WARNING.with(|w| {
        let mut slot = w.borrow_mut();
        if slot.is_none() {
            *slot = Some(format!("amount {:?} rounded to 12 decimal places", esc(s)));
        }
    });
}

/// Round to `12 - shift` fractional digits of the input (12 dollar decimals) and warn.
fn parse_rounded(s: &str, int: &str, frac: &str, exp: i32, shift: usize) -> Result<Money> {
    if shift > SCALE_DIGITS {
        return Err(too_precise(s));
    }
    let places = (SCALE_DIGITS - shift) as i64;
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|c| c - b'0').collect();
    // value = significand * 10^power.
    let power = i64::from(exp) - frac.len() as i64;
    if let Some(lead) = digits.iter().position(|&d| d != 0) {
        digits.drain(..lead);
    } else {
        warn_rounded(s);
        return Ok(Money::ZERO);
    }
    // Same 18-digit ceiling as the exact path, counted on the value not the spelling.
    if digits.len() as i64 + power > 18 {
        return Err(too_precise(s));
    }
    let scale = power + places;
    let value = if scale >= 0 {
        if digits.len() as i64 + scale > 38 {
            return Err(too_precise(s));
        }
        let mut v: i128 = 0;
        for d in &digits {
            v = v
                .checked_mul(10)
                .and_then(|v| v.checked_add(i128::from(*d)))
                .ok_or_else(|| too_precise(s))?;
        }
        for _ in 0..scale {
            v = v.checked_mul(10).ok_or_else(|| too_precise(s))?;
        }
        v
    } else {
        let drop = -scale;
        if drop > digits.len() as i64 {
            // The first dropped digit is an implicit zero.
            0
        } else {
            let split = digits.len() - drop as usize;
            let (keep, rest) = digits.split_at(split);
            let mut v: i128 = 0;
            for d in keep {
                v = v
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(i128::from(*d)))
                    .ok_or_else(|| too_precise(s))?;
            }
            if rest.first().is_some_and(|d| *d >= 5) {
                v = v.checked_add(1).ok_or_else(|| too_precise(s))?;
            }
            v
        }
    };
    warn_rounded(s);
    Ok(Money(value))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Provider {
    Anthropic,
    Openai,
    Other,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::Openai => "openai",
            Provider::Other => "other",
        }
    }
}

/// Service tier. `Unknown` means the source did not say (not grouped by tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Standard,
    Batch,
    Priority,
    Flex,
    Other,
    Unknown,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Standard => "standard",
            Tier::Batch => "batch",
            Tier::Priority => "priority",
            Tier::Flex => "flex",
            Tier::Other => "other",
            Tier::Unknown => "unknown",
        }
    }

    /// Lenient mapping for API sources: unrecognised values become `Other`.
    pub fn from_api(s: &str) -> Tier {
        match s {
            "standard" | "default" => Tier::Standard,
            "batch" => Tier::Batch,
            "priority" => Tier::Priority,
            "flex" => Tier::Flex,
            _ => Tier::Other,
        }
    }
}

/// The normalised v0.1 record: one line per (day, provider, model, tier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
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
}

impl Record {
    pub fn has_tokens(&self) -> bool {
        [
            self.input_tokens,
            self.cache_read_tokens,
            self.cache_write_tokens,
            self.output_tokens,
        ]
        .iter()
        .any(|&t| t > 0)
    }
}
