//! Dimension vectors over UCUM's seven base units and any declared ones.

use core::fmt;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// UCUM's base units, in the order UCUM lists them: length, time, mass,
/// plane angle, temperature, electric charge, luminous intensity.
pub const UCUM_BASE_UNITS: [&str; 7] = ["m", "s", "g", "rad", "K", "C", "cd"];

/// A dimension: an integer exponent for each base unit.
///
/// A base is named by its unit's symbol: UCUM's seven (`m`, `s`, `g`, `rad`,
/// `K`, `C`, `cd`, see [`UCUM_BASE_UNITS`]) and any declared as data, such
/// as `{tray}` or `[EUR]`. [`Display`](fmt::Display) writes a dimension
/// compactly as base units with exponents: energy is `m2.s-2.g`, energy per
/// tray `m2.s-2.g.{tray}-1`. That is not always valid UCUM, which gives an
/// annotation no exponent; [`to_ucum`](Dimension::to_ucum) writes one that
/// is. Zero exponents are never stored, so two equal dimensions compare
/// equal however they were built.
///
/// The algebra is exact and checked: [`checked_mul`](Dimension::checked_mul),
/// [`checked_div`](Dimension::checked_div) and
/// [`checked_pow`](Dimension::checked_pow) return `None` rather than
/// overflow an exponent.
///
/// In the definitions format a dimension is a JSON object from base symbol
/// to exponent: `{"g": 1, "{CO2e}": 1}`.
///
/// ```
/// use pubunit::Dimension;
///
/// let length = Dimension::base("m");
/// let time = Dimension::base("s");
/// let speed = length.checked_div(&time).ok_or("overflow")?;
/// assert_eq!(speed.to_string(), "m.s-1");
/// assert_eq!(speed.exponent("s"), -1);
/// let area = length.checked_pow(2).ok_or("overflow")?;
/// assert_eq!(area.to_string(), "m2");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "BTreeMap<String, i32>", into = "BTreeMap<String, i32>")]
pub struct Dimension(BTreeMap<String, i32>);

impl Dimension {
    /// The dimension of a pure number: every exponent zero.
    ///
    /// ```
    /// assert!(pubunit::Dimension::dimensionless().is_dimensionless());
    /// ```
    #[must_use]
    pub fn dimensionless() -> Dimension {
        Dimension(BTreeMap::new())
    }

    /// One base, to the first power.
    ///
    /// ```
    /// assert_eq!(pubunit::Dimension::base("{tray}").to_string(), "{tray}");
    /// ```
    #[must_use]
    pub fn base(symbol: impl Into<String>) -> Dimension {
        Dimension(BTreeMap::from([(symbol.into(), 1)]))
    }

    /// The exponent of `base`, zero when it is absent.
    ///
    /// ```
    /// let d = pubunit::Dimension::base("g");
    /// assert_eq!(d.exponent("g"), 1);
    /// assert_eq!(d.exponent("m"), 0);
    /// ```
    #[must_use]
    pub fn exponent(&self, base: &str) -> i32 {
        self.0.get(base).copied().unwrap_or(0)
    }

    /// Whether every exponent is zero.
    ///
    /// ```
    /// assert!(!pubunit::Dimension::base("m").is_dimensionless());
    /// ```
    #[must_use]
    pub fn is_dimensionless(&self) -> bool {
        self.0.is_empty()
    }

    /// The non-zero exponents, in display order: UCUM's base units first in
    /// UCUM's order, then declared ones by symbol.
    ///
    /// ```
    /// let d: pubunit::Dimension = serde_json::from_str(r#"{"{tray}": -1, "g": 1, "m": 2}"#)?;
    /// let bases: Vec<_> = d.iter().collect();
    /// assert_eq!(bases, [("m", 2), ("g", 1), ("{tray}", -1)]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&str, i32)> {
        let mut v: Vec<(&str, i32)> = self.0.iter().map(|(k, &e)| (k.as_str(), e)).collect();
        v.sort_by_key(|&(k, _)| (rank(k), k));
        v.into_iter()
    }

    /// The product: exponents added. `None` if an exponent overflows.
    ///
    /// ```
    /// use pubunit::Dimension;
    /// let d = Dimension::base("m").checked_mul(&Dimension::base("m"));
    /// assert_eq!(d.map(|d| d.to_string()), Some("m2".to_owned()));
    /// ```
    #[must_use]
    pub fn checked_mul(&self, other: &Dimension) -> Option<Dimension> {
        self.combine(other, 1)
    }

    /// The quotient: exponents subtracted. `None` if an exponent overflows.
    ///
    /// ```
    /// use pubunit::Dimension;
    /// let d = Dimension::base("m").checked_div(&Dimension::base("m"));
    /// assert_eq!(d, Some(Dimension::dimensionless()));
    /// ```
    #[must_use]
    pub fn checked_div(&self, other: &Dimension) -> Option<Dimension> {
        self.combine(other, -1)
    }

    /// The `n`th power: exponents multiplied by `n`. `None` if one
    /// overflows.
    ///
    /// ```
    /// use pubunit::Dimension;
    /// let d = Dimension::base("s").checked_pow(-2);
    /// assert_eq!(d.map(|d| d.to_string()), Some("s-2".to_owned()));
    /// ```
    #[must_use]
    pub fn checked_pow(&self, n: i32) -> Option<Dimension> {
        let mut out = BTreeMap::new();
        for (k, &e) in &self.0 {
            let e = e.checked_mul(n)?;
            if e != 0 {
                out.insert(k.clone(), e);
            }
        }
        Some(Dimension(out))
    }

    fn combine(&self, other: &Dimension, sign: i32) -> Option<Dimension> {
        let mut out = self.0.clone();
        for (k, &e) in &other.0 {
            let cur = out.get(k).copied().unwrap_or(0);
            let next = cur.checked_add(e.checked_mul(sign)?)?;
            if next == 0 {
                out.remove(k);
            } else {
                out.insert(k.clone(), next);
            }
        }
        Some(Dimension(out))
    }

    /// A UCUM expression of exactly this dimension, with factor 1: base
    /// units with positive exponents, then `/` and those with negative ones.
    /// A brace base, which UCUM gives no exponent, is repeated.
    ///
    /// ```
    /// use pubunit::{Dimension, Registry};
    ///
    /// let d: Dimension = serde_json::from_str(r#"{"m": 2, "s": -2, "g": 1, "{tray}": -2}"#)?;
    /// assert_eq!(d.to_string(), "m2.s-2.g.{tray}-2");
    /// assert_eq!(d.to_ucum(), "m2.g/(s2.{tray}.{tray})");
    /// assert_eq!(Dimension::base("s").checked_pow(-1).map(|d| d.to_ucum()), Some("/s".to_owned()));
    /// assert_eq!(Dimension::dimensionless().to_ucum(), "1");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn to_ucum(&self) -> String {
        let part = |positive: bool| -> Vec<String> {
            self.iter()
                .filter(|&(_, e)| (e > 0) == positive)
                .map(|(base, e)| {
                    let n = e.unsigned_abs();
                    if base.starts_with('{') {
                        vec![base; n as usize].join(".")
                    } else if n == 1 {
                        base.to_owned()
                    } else {
                        format!("{base}{n}")
                    }
                })
                .collect()
        };
        let (num, den) = (part(true), part(false));
        let den = match den.len() {
            0 => String::new(),
            1 if !den.concat().contains('.') => format!("/{}", den.concat()),
            _ => format!("/({})", den.join(".")),
        };
        match (num.is_empty(), den.is_empty()) {
            (true, true) => "1".to_owned(),
            (true, false) => den,
            _ => format!("{}{den}", num.join(".")),
        }
    }

    /// The exponents on UCUM's seven base units, in UCUM's order.
    pub(crate) fn ucum_part(&self) -> [i32; 7] {
        UCUM_BASE_UNITS.map(|b| self.exponent(b))
    }
}

fn rank(base: &str) -> usize {
    UCUM_BASE_UNITS
        .iter()
        .position(|&b| b == base)
        .unwrap_or(UCUM_BASE_UNITS.len())
}

impl From<BTreeMap<String, i32>> for Dimension {
    /// Zero exponents are dropped.
    fn from(mut map: BTreeMap<String, i32>) -> Dimension {
        map.retain(|_, e| *e != 0);
        Dimension(map)
    }
}

impl From<Dimension> for BTreeMap<String, i32> {
    fn from(d: Dimension) -> BTreeMap<String, i32> {
        d.0
    }
}

impl fmt::Display for Dimension {
    /// Base units with exponents, such as `m2.s-2.g`; `1` when
    /// dimensionless. See [`Dimension::to_ucum`] for valid UCUM.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return f.write_str("1");
        }
        for (i, (base, e)) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            if e == 1 {
                f.write_str(base)?;
            } else {
                write!(f, "{base}{e}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: Dimension::checked_pow, Dimension::checked_mul
    #[test]
    fn overflow_is_refused_not_wrapped() {
        let d = Dimension::base("m").checked_pow(i32::MAX);
        assert!(d.is_some());
        assert_eq!(d.and_then(|d| d.checked_pow(2)), None);
        let big = Dimension::base("m")
            .checked_pow(i32::MAX)
            .unwrap_or_default();
        assert_eq!(big.checked_mul(&Dimension::base("m")), None);
    }

    // covers: Dimension
    #[test]
    fn displays_ucum_bases_first() {
        let d = Dimension::from(BTreeMap::from([
            ("[EUR]".to_owned(), 1),
            ("s".to_owned(), -1),
            ("m".to_owned(), 0),
            ("g".to_owned(), 1),
        ]));
        assert_eq!(d.to_string(), "s-1.g.[EUR]");
    }
}
