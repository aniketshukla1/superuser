//! the arithmetic


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    /// Spread: the standard deviation.
    pub sd: f64,
    pub last: f64,
}

/// Over the finite values; `None` when there are none (a series of NaN is a series of nothing).
pub fn stats(values: &[f64]) -> Option<Stats> {
    let v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    let n = v.len();
    if n == 0 {
        return None;
    }
    let mean = v.iter().sum::<f64>() / n as f64;
    let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    Some(Stats {
        n,
        min: v.iter().copied().fold(f64::INFINITY, f64::min),
        max: v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        mean,
        sd,
        last: v[n - 1],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Clear,
    Possible,
    Steady,
    TooLittle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shift {
    pub before: Stats,
    pub after: Stats,
    /// After's average minus before's.
    pub change: f64,
    /// That as a percentage of before's; `None` when before averaged zero.
    pub pct: Option<f64>,
    /// The share of points after that are outside everything before, in the change's direction.
    pub beyond: f64,
    pub verdict: Verdict,
}

/// How `after` differs from `before`, and whether that's a shift or the metric's usual wobble:
///
/// - **Clear**: the average moved by 5% or more and by at least three times the spread it had
///   before, and at least half the points after are outside anything seen before — a step,
///   not a spike.
/// - **Possible**: moved by 5% or more and by at least the spread before, but not that far or
///   not sustained.
/// - **Steady**: less than that.
/// - **Too little**: fewer than three points on a side.
pub fn shift(before: &[f64], after: &[f64]) -> Option<Shift> {
    let (b, a) = (stats(before)?, stats(after)?);
    let change = a.mean - b.mean;
    let pct = (b.mean != 0.0).then(|| change / b.mean.abs() * 100.0);
    let outside = after
        .iter()
        .filter(|x| x.is_finite())
        .filter(|&&x| if change >= 0.0 { x > b.max } else { x < b.min })
        .count();
    let beyond = outside as f64 / a.n as f64;
    let big_enough = pct.map_or(change != 0.0, |p| p.abs() >= 5.0);
    let spreads = if b.sd > 0.0 { change.abs() / b.sd } else if change != 0.0 { f64::INFINITY } else { 0.0 };
    let verdict = if b.n < 3 || a.n < 3 {
        Verdict::TooLittle
    } else if big_enough && spreads >= 3.0 && beyond >= 0.5 {
        Verdict::Clear
    } else if big_enough && spreads >= 1.0 {
        Verdict::Possible
    } else {
        Verdict::Steady
    };
    Some(Shift { before: b, after: a, change, pct, beyond, verdict })
}

/// A number the way a person reads one: three significant figures, no exponent below a million.
pub fn fmt_num(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() { "NaN".into() } else if x > 0.0 { "+Inf".into() } else { "-Inf".into() };
    }
    if x == 0.0 {
        return "0".into();
    }
    let a = x.abs();
    if a >= 1e6 {
        return format!("{x:.3e}");
    }
    if a >= 100.0 {
        return format!("{x:.0}");
    }
    let decimals = (2 - a.log10().floor() as i32).clamp(0, 9) as usize;
    let s = format!("{x:.decimals$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

pub(super) fn fmt_move(s: &Shift) -> String {
    let dir = if s.change >= 0.0 { "up" } else { "down" };
    match s.pct {
        Some(p) => format!("{dir} {}%", fmt_num(p.abs())),
        None => format!("{dir} from zero"),
    }
}
