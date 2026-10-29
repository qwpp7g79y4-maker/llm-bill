//! Normalised v0.1 record and its checked scalar types.

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
            return err(format!("ongeldige dag {:?}, verwacht YYYY-MM-DD", esc(s)));
        }
        let n = |r: std::ops::Range<usize>| s[r].parse::<u16>().unwrap_or(0);
        Day::new(n(0..4), n(5..7) as u8, n(8..10) as u8)
            .map_or_else(|| err(format!("ongeldige dag {:?}", esc(s))), Ok)
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
            .map_or_else(|| err(format!("tijdstip {secs} buiten bereik")), Ok)
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

    /// Parse a plain non-negative decimal (`123`, `1.23`), no sign or exponent.
    /// `shift` divides by 10^shift (2 converts cents to dollars).
    pub fn parse(s: &str, shift: usize) -> Result<Money> {
        let (int, frac) = s.split_once('.').unwrap_or((s, ""));
        let digits = |t: &str| t.bytes().all(|c| c.is_ascii_digit());
        if int.is_empty() || !digits(int) || !digits(frac) || (s.contains('.') && frac.is_empty()) {
            return err(format!(
                "ongeldig bedrag {:?}: verwacht niet-negatief decimaal getal",
                esc(s)
            ));
        }
        if int.len() > 18 || frac.len() + shift > SCALE_DIGITS {
            return err(format!("bedrag {:?} te groot of te nauwkeurig", esc(s)));
        }
        // value * 10^(12 - shift): int digits followed by frac padded to 12 - shift places.
        let places = SCALE_DIGITS - shift;
        let mut v: i128 = int.parse().unwrap_or(0);
        for i in 0..places {
            v = v * 10 + i128::from(frac.as_bytes().get(i).map_or(0, |c| c - b'0'));
        }
        Ok(Money(v))
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
            Tier::Unknown => "onbekend",
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
