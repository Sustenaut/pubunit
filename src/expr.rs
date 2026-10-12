//! The UCUM expression grammar, parsed into a [`Term`] tree.

use core::fmt;
use core::str::FromStr;

use crate::error::Error;

/// The longest expression [`parse`] reads, in bytes.
pub const MAX_LEN: usize = 1024;
/// The deepest nesting of parentheses [`parse`] reads.
pub const MAX_DEPTH: usize = 32;
/// The largest exponent magnitude on one symbol (`m64`, `s-64`).
pub const MAX_EXPONENT: i32 = 64;

/// A parsed UCUM unit expression.
///
/// The tree follows UCUM's grammar (§§ 7–10 of the specification): `.` and
/// `/` are left-associative and of equal precedence, so `kg/m.s` is
/// `(kg/m).s`. Symbols are kept as written: whether `kW` is kilo-watt is
/// decided when a [`Registry`](crate::Registry) resolves it, because that
/// depends on which atoms exist.
///
/// [`Display`](fmt::Display) writes the expression back in a normal form,
/// with only the parentheses the grammar needs, so two spellings of one
/// expression print the same.
///
/// ```
/// use pubunit::Term;
///
/// let t = pubunit::parse("kW.h/{tray}")?;
/// let Term::Div(num, den) = &t else { panic!("a quotient") };
/// assert_eq!(num.to_string(), "kW.h");
/// assert_eq!(**den, Term::Annotation("tray".to_owned()));
/// assert_eq!(pubunit::parse("((kW).h)/{tray}")?.to_string(), "kW.h/{tray}");
/// # Ok::<(), pubunit::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Term {
    /// A positive integer factor, such as the `100` in `/100{cells}`, with
    /// the annotation that follows it, if any.
    Factor {
        /// The integer.
        value: u64,
        /// The text between the braces after it.
        annotation: Option<String>,
    },
    /// A unit symbol with its exponent, such as `m2` or `[ft_i]`, with the
    /// annotation that follows it, if any (`kg{CO2e}`).
    Symbol {
        /// The symbol as written, prefix included: `kW`, `[EUR]`.
        symbol: String,
        /// The exponent; 1 when none is written.
        exponent: i32,
        /// The text between the braces after it.
        annotation: Option<String>,
    },
    /// An annotation standing alone, such as `{tray}`: its text, without
    /// the braces.
    Annotation(String),
    /// `a.b`
    Mul(Box<Term>, Box<Term>),
    /// `a/b`
    Div(Box<Term>, Box<Term>),
    /// A leading `/`: `/s` is per second.
    Recip(Box<Term>),
    /// A parenthesized term, kept so the tree mirrors the text.
    Group(Box<Term>),
}

/// Parse a UCUM expression (case-sensitive form) into a [`Term`].
///
/// This checks syntax only: it accepts `[XYZ]` without knowing whether
/// such an atom exists. [`Registry::validate`](crate::Registry::validate)
/// resolves the atoms, and also has ucum-units check the expression.
///
/// # Errors
///
/// Fails with [`Error::Syntax`] when the text is not a UCUM expression:
/// - it is empty, or holds a character that is not printable ASCII;
/// - a parenthesis, bracket or brace is not closed, or a `{` is nested
///   inside an annotation;
/// - an operator or an exponent stands where a unit belongs;
/// - a symbol outside brackets ends in a digit or a sign, which no UCUM
///   atom does, so `1+3+3` cannot be read two ways.
///
/// Fails with [`Error::TooLarge`] on input beyond [`MAX_LEN`] bytes or
/// [`MAX_DEPTH`] levels of parentheses, on an exponent beyond
/// ±[`MAX_EXPONENT`], however many digits it has, and on a factor beyond
/// `u64`.
///
/// ```
/// let t = pubunit::parse("mg/dL")?;
/// assert_eq!(t.to_string(), "mg/dL");
///
/// assert!(pubunit::parse("m//s").is_err());
/// assert!(pubunit::parse("kg.(m").is_err());
/// assert!(pubunit::parse("m 2").is_err());
/// # Ok::<(), pubunit::Error>(())
/// ```
pub fn parse(expr: &str) -> Result<Term, Error> {
    if expr.len() > MAX_LEN {
        return Err(Error::TooLarge {
            what: format!("an expression of {} bytes (at most {MAX_LEN})", expr.len()),
        });
    }
    let mut p = Parser {
        text: expr,
        bytes: expr.as_bytes(),
        pos: 0,
    };
    if expr.is_empty() {
        return Err(p.error("an empty expression"));
    }
    if let Some(i) = expr.bytes().position(|b| !(33..=126).contains(&b)) {
        p.pos = i;
        return Err(p.error("a character that is not printable ASCII"));
    }
    let term = p.main_term(0)?;
    if p.pos < p.bytes.len() {
        return Err(p.error("an unexpected character"));
    }
    Ok(term)
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn error(&self, message: &str) -> Error {
        Error::Syntax {
            expr: self.text.to_owned(),
            position: self.pos,
            message: message.to_owned(),
        }
    }

    /// `mainTerm := '/' term | term`
    fn main_term(&mut self, depth: usize) -> Result<Term, Error> {
        if self.peek() == Some(b'/') {
            self.pos += 1;
            Ok(Term::Recip(Box::new(self.term(depth)?)))
        } else {
            self.term(depth)
        }
    }

    /// `term := component (('.' | '/') component)*`, left-associative.
    fn term(&mut self, depth: usize) -> Result<Term, Error> {
        let mut left = self.component(depth)?;
        loop {
            match self.peek() {
                Some(b'.') => {
                    self.pos += 1;
                    let right = self.component(depth)?;
                    left = Term::Mul(Box::new(left), Box::new(right));
                }
                Some(b'/') => {
                    self.pos += 1;
                    let right = self.component(depth)?;
                    left = Term::Div(Box::new(left), Box::new(right));
                }
                _ => return Ok(left),
            }
        }
    }

    /// `component := '(' mainTerm ')' | annotation | factor annotation? |
    /// symbol exponent? annotation?`
    fn component(&mut self, depth: usize) -> Result<Term, Error> {
        match self.peek() {
            Some(b'(') => {
                if depth >= MAX_DEPTH {
                    return Err(Error::TooLarge {
                        what: format!("more than {MAX_DEPTH} levels of parentheses"),
                    });
                }
                self.pos += 1;
                let inner = self.main_term(depth + 1)?;
                if self.peek() != Some(b')') {
                    return Err(self.error("a missing `)`"));
                }
                self.pos += 1;
                Ok(Term::Group(Box::new(inner)))
            }
            Some(b'{') => Ok(Term::Annotation(self.annotation()?)),
            None | Some(b'.' | b'/' | b')' | b'}') => Err(self.error("a missing unit")),
            Some(_) => self.symbol(),
        }
    }

    /// The text of `{...}`, the cursor on the `{`.
    fn annotation(&mut self) -> Result<String, Error> {
        let start = self.pos + 1;
        let rest = self.bytes.get(start..).unwrap_or_default();
        let Some(len) = rest.iter().position(|&b| b == b'}') else {
            return Err(self.error("an annotation with no closing `}`"));
        };
        let text = self.text.get(start..start + len).unwrap_or_default();
        if text.contains('{') {
            self.pos = start + text.find('{').unwrap_or(0);
            return Err(self.error("a `{` inside an annotation"));
        }
        self.pos = start + len + 1;
        Ok(text.to_owned())
    }

    fn optional_annotation(&mut self) -> Result<Option<String>, Error> {
        if self.peek() == Some(b'{') {
            self.annotation().map(Some)
        } else {
            Ok(None)
        }
    }

    /// A run of symbol characters, with `[...]` read whole, then a trailing
    /// exponent split off.
    fn symbol(&mut self) -> Result<Term, Error> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            match b {
                b'.' | b'/' | b'(' | b')' | b'{' | b'}' => break,
                b'[' => {
                    let rest = self.bytes.get(self.pos..).unwrap_or_default();
                    let Some(len) = rest.iter().position(|&b| b == b']') else {
                        return Err(self.error("a `[` with no closing `]`"));
                    };
                    self.pos += len + 1;
                }
                _ => self.pos += 1,
            }
        }
        let run = self.text.get(start..self.pos).unwrap_or_default();
        if run.bytes().all(|b| b.is_ascii_digit()) {
            let value = run.parse::<u64>().map_err(|_| Error::TooLarge {
                what: format!("the factor {run} (at most {})", u64::MAX),
            })?;
            let annotation = self.optional_annotation()?;
            return Ok(Term::Factor { value, annotation });
        }
        let (symbol, exponent) = split_exponent(run);
        let exponent = match exponent {
            None => 1,
            Some(e) => {
                let too_large = || Error::TooLarge {
                    what: format!("the exponent {e} (at most ±{MAX_EXPONENT})"),
                };
                let n: i64 = e.parse().map_err(|_| too_large())?;
                if n.unsigned_abs() > u64::from(MAX_EXPONENT.unsigned_abs()) {
                    return Err(too_large());
                }
                i32::try_from(n).map_err(|_| too_large())?
            }
        };
        if symbol.is_empty() || symbol.starts_with(['+', '-']) {
            self.pos = start;
            return Err(self.error("an exponent with no unit"));
        }
        // No UCUM atom ends in a digit or a sign outside brackets. A symbol
        // that did would not read back as written: `1+3` with exponent 3
        // prints as `1+33`, and `1-` with exponent +3 as `1-3`.
        if symbol.ends_with(|c: char| c.is_ascii_digit() || c == '+' || c == '-') {
            self.pos = start;
            return Err(self.error("a unit symbol ending in a digit or a sign"));
        }
        let annotation = self.optional_annotation()?;
        Ok(Term::Symbol {
            symbol: symbol.to_owned(),
            exponent,
            annotation,
        })
    }
}

/// Split `m2`, `s-1`, `10*3` into symbol and exponent text.
fn split_exponent(run: &str) -> (&str, Option<&str>) {
    let digits = run.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return (run, None);
    }
    let mut cut = run.len() - digits;
    if cut > 0 && matches!(run.as_bytes().get(cut - 1), Some(b'+' | b'-')) {
        cut -= 1;
    }
    let (symbol, exp) = run.split_at(cut);
    (symbol, Some(exp.strip_prefix('+').unwrap_or(exp)))
}

impl FromStr for Term {
    type Err = Error;

    /// The same as [`parse`].
    fn from_str(expr: &str) -> Result<Term, Error> {
        parse(expr)
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_term(self, f, false)
    }
}

pub(crate) fn peel(mut t: &Term) -> &Term {
    while let Term::Group(inner) = t {
        t = inner;
    }
    t
}

/// Parentheses are needed around a compound right operand of `.` or `/`,
/// because both are left-associative, and around a reciprocal anywhere but
/// first, because a leading `/` takes the whole term after it: `/s.m` is
/// 1/(s.m), not (1/s).m.
fn write_term(t: &Term, f: &mut fmt::Formatter<'_>, paren: bool) -> fmt::Result {
    let t = peel(t);
    let compound = matches!(t, Term::Mul(..) | Term::Div(..) | Term::Recip(..));
    if paren && compound {
        f.write_str("(")?;
    }
    match t {
        Term::Factor { value, annotation } => {
            write!(f, "{value}")?;
            write_annotation(annotation.as_deref(), f)?;
        }
        Term::Symbol {
            symbol,
            exponent,
            annotation,
        } => {
            f.write_str(symbol)?;
            if *exponent != 1 {
                write!(f, "{exponent}")?;
            }
            write_annotation(annotation.as_deref(), f)?;
        }
        Term::Annotation(text) => write!(f, "{{{text}}}")?,
        Term::Mul(a, b) => {
            write_term(a, f, is_recip(a))?;
            f.write_str(".")?;
            write_term(b, f, true)?;
        }
        Term::Div(a, b) => {
            write_term(a, f, is_recip(a))?;
            f.write_str("/")?;
            write_term(b, f, true)?;
        }
        Term::Recip(a) => {
            f.write_str("/")?;
            write_term(a, f, is_recip(a))?;
        }
        // `peel` removed every group.
        Term::Group(_) => {}
    }
    if paren && compound {
        f.write_str(")")?;
    }
    Ok(())
}

/// Whether `t`, groups aside, is a reciprocal.
pub(crate) fn is_recip(t: &Term) -> bool {
    matches!(peel(t), Term::Recip(_))
}

fn write_annotation(a: Option<&str>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match a {
        Some(text) => write!(f, "{{{text}}}"),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(s: &str, exponent: i32) -> Term {
        Term::Symbol {
            symbol: s.to_owned(),
            exponent,
            annotation: None,
        }
    }

    // covers: parse
    #[test]
    fn exponents_split_from_symbols() {
        assert_eq!(parse("m2").ok(), Some(sym("m", 2)));
        assert_eq!(parse("s-1").ok(), Some(sym("s", -1)));
        assert_eq!(parse("m+3").ok(), Some(sym("m", 3)));
        assert_eq!(parse("10*3").ok(), Some(sym("10*", 3)));
        assert_eq!(parse("10*-3").ok(), Some(sym("10*", -3)));
        assert_eq!(parse("[ft_i]2").ok(), Some(sym("[ft_i]", 2)));
        assert_eq!(parse("B[10.nV]").ok(), Some(sym("B[10.nV]", 1)));
    }

    // covers: parse
    #[test]
    fn left_associative() {
        let t = parse("kg/m.s").ok();
        let want = Term::Mul(
            Box::new(Term::Div(Box::new(sym("kg", 1)), Box::new(sym("m", 1)))),
            Box::new(sym("s", 1)),
        );
        assert_eq!(t, Some(want));
        assert_eq!(
            parse("kg/(m.s)").map(|t| t.to_string()).ok().as_deref(),
            Some("kg/(m.s)")
        );
    }

    // covers: parse
    #[test]
    fn factors_and_annotations() {
        assert_eq!(
            parse("/100{cells}").ok(),
            Some(Term::Recip(Box::new(Term::Factor {
                value: 100,
                annotation: Some("cells".to_owned())
            })))
        );
        assert_eq!(
            parse("kg{CO2e}").ok(),
            Some(Term::Symbol {
                symbol: "kg".to_owned(),
                exponent: 1,
                annotation: Some("CO2e".to_owned())
            })
        );
    }

    // covers: parse
    #[test]
    fn limits_hold() {
        assert!(matches!(parse("m65"), Err(Error::TooLarge { .. })));
        assert!(matches!(
            parse("m99999999999999999999"),
            Err(Error::TooLarge { .. })
        ));
        assert!(parse("m64").is_ok());
        let deep = format!("{}m{}", "(".repeat(40), ")".repeat(40));
        assert!(matches!(parse(&deep), Err(Error::TooLarge { .. })));
        assert!(matches!(
            parse(&"m.".repeat(600)),
            Err(Error::TooLarge { .. })
        ));
        assert!(matches!(
            parse("99999999999999999999"),
            Err(Error::TooLarge { .. })
        ));
    }

    // covers: parse
    // Found by fuzzing (fuzz/regressions/parse/symbol-ending-in-digit and
    // symbol-ending-in-sign): the symbol `1+3` with exponent 3 printed as
    // `1+33`, and `1-` with exponent +3 as `1-3`, which read back as other
    // terms.
    #[test]
    fn a_symbol_ending_in_a_digit_or_sign_is_refused() {
        assert!(matches!(parse("1+3+3"), Err(Error::Syntax { .. })));
        assert!(matches!(parse("m2+3"), Err(Error::Syntax { .. })));
        assert!(matches!(parse("1-+3"), Err(Error::Syntax { .. })));
        assert!(matches!(parse("m-"), Err(Error::Syntax { .. })));
        assert!(parse("10*+3").is_ok());
    }

    // covers: Term, parse
    // Found by fuzzing (fuzz/regressions/parse/nested-reciprocal):
    // `/(/@/s)` printed as `//@/s`, which does not parse, and `(/s).m`
    // printed as `/s.m`, which means 1/(s.m).
    #[test]
    fn a_reciprocal_keeps_its_parentheses() {
        for (text, normal) in [
            ("/(/@/s)", "/(/@/s)"),
            ("(/s).m", "(/s).m"),
            ("(/s)/m", "(/s)/m"),
            ("m/(/s)", "m/(/s)"),
            ("/(m.s)", "/m.s"),
            ("/m.s", "/m.s"),
        ] {
            let t = parse(text).map(|t| t.to_string());
            assert_eq!(t.as_deref(), Ok(normal), "{text}");
            assert_eq!(parse(normal).map(|t| t.to_string()).as_deref(), Ok(normal));
        }
    }

    // covers: parse
    #[test]
    fn syntax_errors() {
        for bad in [
            "", "/", "m.", ".m", "m//s", "(m", "m)", "{a", "a}", "[ft_i", "-1", "m{a{b}}", "é",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} parsed");
        }
    }
}
