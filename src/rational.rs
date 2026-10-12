//! Exact rational numbers, the only kind of number pubunit computes with.

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, Mul, Neg, Sub};
use core::str::FromStr;

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::error::Error;

/// The largest power of ten a decimal exponent may name (`1e1000`).
///
/// It bounds the size of a number read from text, so a short input cannot
/// ask for a huge allocation.
pub const MAX_DECIMAL_EXPONENT: u32 = 1000;

/// An exact rational number, always in lowest terms.
///
/// Conversion factors, offsets and converted values are `Rational`s, so a
/// result never depends on floating-point rounding. Read one from a decimal
/// string (`"0.125"`, `"6.67430e-11"`) or a fraction (`"5/9"`), and write it
/// back with [`Display`](fmt::Display): as a decimal when it has a finite
/// one, as a fraction otherwise.
///
/// ```
/// use pubunit::Rational;
///
/// let third: Rational = "1/3".parse()?;
/// assert_eq!(third.to_string(), "1/3");
/// assert_eq!(third.to_decimal(), None);
/// assert_eq!(third.round(4), "0.3333");
///
/// let g: Rational = "6.67430e-11".parse()?;
/// assert_eq!(g.to_string(), "0.000000000066743");
/// # Ok::<(), pubunit::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    // Invariants: den > 0, and gcd(num, den) == 1.
    num: BigInt,
    den: BigInt,
}

impl Rational {
    /// Zero.
    ///
    /// ```
    /// assert!(pubunit::Rational::zero().is_zero());
    /// ```
    #[must_use]
    pub fn zero() -> Rational {
        Rational {
            num: BigInt::zero(),
            den: BigInt::one(),
        }
    }

    /// One.
    ///
    /// ```
    /// assert_eq!(pubunit::Rational::one().to_string(), "1");
    /// ```
    #[must_use]
    pub fn one() -> Rational {
        Rational::from_integer(1)
    }

    /// The integer `n`.
    ///
    /// ```
    /// assert_eq!(pubunit::Rational::from_integer(-3).to_string(), "-3");
    /// ```
    #[must_use]
    pub fn from_integer(n: i64) -> Rational {
        Rational {
            num: BigInt::from(n),
            den: BigInt::one(),
        }
    }

    /// `num / den` in lowest terms, or `None` when `den` is zero.
    ///
    /// ```
    /// use pubunit::Rational;
    /// assert_eq!(Rational::new(6, -4).map(|r| r.to_string()), Some("-1.5".to_owned()));
    /// assert_eq!(Rational::new(1, 0), None);
    /// ```
    #[must_use]
    pub fn new(num: i64, den: i64) -> Option<Rational> {
        Rational::from_parts(BigInt::from(num), BigInt::from(den))
    }

    fn from_parts(num: BigInt, den: BigInt) -> Option<Rational> {
        if den.is_zero() {
            return None;
        }
        let g = num.gcd(&den);
        let (mut num, mut den) = (num / &g, den / &g);
        if den.is_negative() {
            num = -num;
            den = -den;
        }
        Some(Rational { num, den })
    }

    /// The size in bits of the larger of numerator and denominator.
    pub(crate) fn bits(&self) -> u64 {
        self.num.bits().max(self.den.bits())
    }

    /// Whether this is zero.
    ///
    /// ```
    /// assert!(!pubunit::Rational::one().is_zero());
    /// ```
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Whether this is an integer.
    ///
    /// ```
    /// use pubunit::Rational;
    /// assert!("12/4".parse::<Rational>()?.is_integer());
    /// assert!(!"0.5".parse::<Rational>()?.is_integer());
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.den.is_one()
    }

    /// Whether this is less than zero.
    ///
    /// ```
    /// assert!(pubunit::Rational::from_integer(-1).is_negative());
    /// ```
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.num.is_negative()
    }

    /// One divided by this, or `None` for zero.
    ///
    /// ```
    /// use pubunit::Rational;
    /// assert_eq!(Rational::from_integer(-4).recip().map(|r| r.to_string()), Some("-0.25".to_owned()));
    /// assert_eq!(Rational::zero().recip(), None);
    /// ```
    #[must_use]
    pub fn recip(&self) -> Option<Rational> {
        Rational::from_parts(self.den.clone(), self.num.clone())
    }

    /// This divided by `other`, or `None` when `other` is zero.
    ///
    /// ```
    /// use pubunit::Rational;
    /// let r = Rational::from_integer(1).checked_div(&Rational::from_integer(8));
    /// assert_eq!(r.map(|r| r.to_string()), Some("0.125".to_owned()));
    /// ```
    #[must_use]
    pub fn checked_div(&self, other: &Rational) -> Option<Rational> {
        Rational::from_parts(&self.num * &other.den, &self.den * &other.num)
    }

    /// This raised to the integer power `exp`, or `None` for zero to a
    /// negative power.
    ///
    /// ```
    /// use pubunit::Rational;
    /// let ten = Rational::from_integer(10);
    /// assert_eq!(ten.checked_pow(-3).map(|r| r.to_string()), Some("0.001".to_owned()));
    /// assert_eq!(Rational::zero().checked_pow(-1), None);
    /// ```
    #[must_use]
    pub fn checked_pow(&self, exp: i32) -> Option<Rational> {
        let n = exp.unsigned_abs();
        let r = Rational {
            num: num_traits::pow::Pow::pow(&self.num, n),
            den: num_traits::pow::Pow::pow(&self.den, n),
        };
        if exp < 0 { r.recip() } else { Some(r) }
    }

    /// The finite decimal expansion, or `None` when there is none (`1/3`).
    ///
    /// ```
    /// use pubunit::Rational;
    /// assert_eq!("5/8".parse::<Rational>()?.to_decimal().as_deref(), Some("0.625"));
    /// assert_eq!("5/9".parse::<Rational>()?.to_decimal(), None);
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn to_decimal(&self) -> Option<String> {
        // A finite decimal exists exactly when den = 2^a 5^b; it then has
        // max(a, b) digits after the point.
        let (mut d, mut twos, mut fives) = (self.den.clone(), 0u32, 0u32);
        let (two, five) = (BigInt::from(2), BigInt::from(5));
        while (&d % &two).is_zero() {
            d /= &two;
            twos += 1;
        }
        while (&d % &five).is_zero() {
            d /= &five;
            fives += 1;
        }
        if !d.is_one() {
            return None;
        }
        let places = twos.max(fives);
        let scaled = &self.num * num_traits::pow::Pow::pow(&BigInt::from(10), places) / &self.den;
        Some(place_point(&scaled, places))
    }

    /// The decimal rounded to `places` digits after the point, halves to
    /// even, with exactly that many digits. Time and memory grow with
    /// `places`, so bound it when it comes from outside.
    ///
    /// ```
    /// use pubunit::Rational;
    /// assert_eq!("2/3".parse::<Rational>()?.round(2), "0.67");
    /// assert_eq!("0.125".parse::<Rational>()?.round(2), "0.12");
    /// assert_eq!("0.375".parse::<Rational>()?.round(2), "0.38");
    /// assert_eq!("-1/2".parse::<Rational>()?.round(0), "-0");
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn round(&self, places: u32) -> String {
        let scaled = &self.num * num_traits::pow::Pow::pow(&BigInt::from(10), places);
        let (q, r) = scaled.abs().div_rem(&self.den);
        let twice = &r * BigInt::from(2);
        let q = match twice.cmp(&self.den) {
            Ordering::Greater => q + 1,
            Ordering::Equal if q.is_odd() => q + 1,
            _ => q,
        };
        let text = place_point(&q, places);
        if self.num.is_negative() {
            format!("-{text}")
        } else {
            text
        }
    }

    /// Parse a decimal (`-1.5`, `254e-2`) or a fraction of integers (`5/9`).
    fn parse(text: &str) -> Result<Rational, String> {
        if let Some((n, d)) = text.split_once('/') {
            let n: BigInt = integer(n)?;
            let d: BigInt = integer(d)?;
            return Rational::from_parts(n, d).ok_or_else(|| "a zero denominator".to_owned());
        }
        let (mantissa, exp) = match text.split_once(['e', 'E']) {
            Some((m, e)) => {
                let e: i64 = e
                    .strip_prefix('+')
                    .unwrap_or(e)
                    .parse()
                    .map_err(|_| format!("`{e}` is not an exponent"))?;
                (m, e)
            }
            None => (text, 0),
        };
        if exp.unsigned_abs() > u64::from(MAX_DECIMAL_EXPONENT) {
            return Err(format!("the exponent is beyond ±{MAX_DECIMAL_EXPONENT}"));
        }
        let (sign, digits) = match mantissa.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        };
        let (whole, frac) = digits.split_once('.').unwrap_or((digits, ""));
        let all = format!("{whole}{frac}");
        if all.is_empty() || !all.bytes().all(|b| b.is_ascii_digit()) {
            return Err("not a decimal number".to_owned());
        }
        let n = BigInt::parse_bytes(all.as_bytes(), 10)
            .ok_or_else(|| "not a decimal number".to_owned())?
            * sign;
        let frac_len = i64::try_from(frac.len()).map_err(|_| "too many digits".to_owned())?;
        let shift = exp - frac_len;
        let ten = Rational::from_integer(10);
        let scale = ten
            .checked_pow(i32::try_from(shift).map_err(|_| "too many digits".to_owned())?)
            .ok_or_else(|| "not a decimal number".to_owned())?;
        Ok(&Rational {
            num: n,
            den: BigInt::one(),
        } * &scale)
    }
}

fn integer(text: &str) -> Result<BigInt, String> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("`{text}` is not an integer"));
    }
    BigInt::parse_bytes(text.as_bytes(), 10).ok_or_else(|| format!("`{text}` is not an integer"))
}

/// Write `scaled / 10^places` as a decimal with `places` digits after the
/// point.
fn place_point(scaled: &BigInt, places: u32) -> String {
    let negative = scaled.sign() == Sign::Minus;
    let digits = scaled.abs().to_string();
    let places = places as usize;
    let padded = if digits.len() <= places {
        format!("{}{digits}", "0".repeat(places + 1 - digits.len()))
    } else {
        digits
    };
    let (int, frac) = padded.split_at(padded.len() - places);
    let sign = if negative { "-" } else { "" };
    if frac.is_empty() {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac}")
    }
}

impl FromStr for Rational {
    type Err = Error;

    /// Parse a decimal (`-1.5`, `0.125`, `254e-2`) or a fraction of two
    /// integers (`5/9`). Exponents beyond ±[`MAX_DECIMAL_EXPONENT`] are
    /// refused.
    fn from_str(text: &str) -> Result<Rational, Error> {
        Rational::parse(text).map_err(|message| Error::Number {
            text: text.to_owned(),
            message,
        })
    }
}

impl fmt::Display for Rational {
    /// The finite decimal when there is one, `num/den` otherwise.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_decimal() {
            Some(d) => f.write_str(&d),
            None => write!(f, "{}/{}", self.num, self.den),
        }
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Rational) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Rational) -> Ordering {
        (&self.num * &other.den).cmp(&(&other.num * &self.den))
    }
}

impl Mul for &Rational {
    type Output = Rational;
    fn mul(self, other: &Rational) -> Rational {
        let num = &self.num * &other.num;
        let den = &self.den * &other.den;
        // den > 0 for both, so their product is non-zero.
        Rational::from_parts(num, den).unwrap_or_else(Rational::zero)
    }
}

impl Add for &Rational {
    type Output = Rational;
    fn add(self, other: &Rational) -> Rational {
        let num = &self.num * &other.den + &other.num * &self.den;
        let den = &self.den * &other.den;
        Rational::from_parts(num, den).unwrap_or_else(Rational::zero)
    }
}

impl Sub for &Rational {
    type Output = Rational;
    fn sub(self, other: &Rational) -> Rational {
        self + &-other
    }
}

impl Neg for &Rational {
    type Output = Rational;
    fn neg(self) -> Rational {
        Rational {
            num: -&self.num,
            den: self.den.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> Rational {
        s.parse().unwrap_or_else(|_| Rational::zero())
    }

    // covers: Rational
    #[test]
    fn reads_ucum_value_forms() {
        assert_eq!(r("254e-2").to_string(), "2.54");
        assert_eq!(r("980665e-5").to_string(), "9.80665");
        assert_eq!(r("1e3").to_string(), "1000");
        assert_eq!(r("1e-24").to_string(), format!("0.{}1", "0".repeat(23)));
        assert_eq!(r("37e9").to_string(), "37000000000");
        assert_eq!(r("-0.5").to_string(), "-0.5");
        assert_eq!(r("+2").to_string(), "2");
        assert_eq!(r("10/4").to_string(), "2.5");
    }

    // covers: Rational
    #[test]
    fn refuses_what_is_not_a_number() {
        for bad in [
            "", ".", "e3", "1e", "1..2", "1/0", "0x10", "1e1001", "1/", "/2", "1.5/2",
        ] {
            assert!(bad.parse::<Rational>().is_err(), "{bad} parsed");
        }
    }

    // covers: Rational::checked_pow, Rational::recip
    #[test]
    fn powers_and_inverse() {
        assert_eq!(r("2/3").checked_pow(2), Some(r("4/9")));
        assert_eq!(r("2/3").checked_pow(-2), Some(r("9/4")));
        assert_eq!(r("2/3").checked_pow(0), Some(Rational::one()));
    }

    // covers: Rational
    #[test]
    fn orders_by_value() {
        assert!(r("1/3") < r("0.34"));
        assert!(r("-1") < r("0"));
        assert_eq!(r("2/4"), r("0.5"));
    }
}
