//! UCUM's atoms plus declared ones: resolution, reduction, conversion and
//! kind checks.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::defs::{Definitions, QuantityKind};
use crate::dimension::UCUM_BASE_UNITS;
use crate::expr::{Term, is_recip, parse, peel};
use crate::table::Special;
use crate::ucum_table::{ATOMS, PREFIXES};
use crate::{Dimension, Error, Rational};

/// How a brace annotation standing alone, which no definition declares, is
/// read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Annotations {
    /// It is an error ([`Error::UndeclaredAnnotation`]). The default: a
    /// misspelt `{trays}` should not quietly mean 1.
    #[default]
    Strict,
    /// It is the number 1, as UCUM says.
    Ucum,
}

/// A unit reduced to its dimension and its exact relation to base units.
///
/// One of the unit is `factor` base units, plus `offset` for a temperature
/// with an offset such as `Cel`. Every other unit has a zero offset, and
/// is called linear. Base units are UCUM's (so mass is in grams) and the
/// declared ones.
///
/// The algebra ([`checked_mul`](Reduced::checked_mul),
/// [`checked_div`](Reduced::checked_div),
/// [`checked_pow`](Reduced::checked_pow)) combines linear units exactly, as
/// a formula carries units through. It returns `None` for a unit with an
/// offset, whose products have no meaning, on exponent overflow, and when a
/// factor's numerator or denominator would exceed [`MAX_FACTOR_BITS`].
///
/// ```
/// let reg = pubunit::Registry::ucum();
/// let kwh = reg.reduce("kW.h")?;
/// assert_eq!(kwh.dimension().to_string(), "m2.s-2.g");
/// assert_eq!(kwh.factor().to_string(), "3600000000");
///
/// let per_hour = reg.reduce("/h")?;
/// let kw = kwh.checked_mul(&per_hour).ok_or("overflow")?;
/// assert_eq!(kw, reg.reduce("kW")?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduced {
    dimension: Dimension,
    factor: Rational,
    offset: Rational,
}

impl Reduced {
    /// A linear unit: one of it is `factor` units of `dimension`'s bases.
    ///
    /// ```
    /// use pubunit::{Dimension, Rational, Reduced};
    /// let km = Reduced::new(Dimension::base("m"), Rational::from_integer(1000));
    /// assert!(km.is_linear());
    /// ```
    #[must_use]
    pub fn new(dimension: Dimension, factor: Rational) -> Reduced {
        Reduced {
            dimension,
            factor,
            offset: Rational::zero(),
        }
    }

    /// The dimension.
    ///
    /// ```
    /// let r = pubunit::Registry::ucum().reduce("N")?;
    /// assert_eq!(r.dimension().to_string(), "m.s-2.g");
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// How many base units one of the unit is.
    ///
    /// ```
    /// let r = pubunit::Registry::ucum().reduce("[in_i]")?;
    /// assert_eq!(r.factor().to_string(), "0.0254");
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn factor(&self) -> &Rational {
        &self.factor
    }

    /// The offset added after scaling: zero except for a temperature with an
    /// offset.
    ///
    /// ```
    /// let r = pubunit::Registry::ucum().reduce("Cel")?;
    /// assert_eq!(r.offset().to_string(), "273.15");
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn offset(&self) -> &Rational {
        &self.offset
    }

    /// Whether the offset is zero.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// assert!(reg.reduce("K")?.is_linear());
    /// assert!(!reg.reduce("[degF]")?.is_linear());
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn is_linear(&self) -> bool {
        self.offset.is_zero()
    }

    /// The product of two linear units. `None` if either has an offset, an
    /// exponent overflows, or the factor would be too large.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// let nm = reg.reduce("N")?.checked_mul(&reg.reduce("m")?);
    /// assert_eq!(nm, Some(reg.reduce("J")?));
    /// assert_eq!(reg.reduce("Cel")?.checked_mul(&reg.reduce("m")?), None);
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn checked_mul(&self, other: &Reduced) -> Option<Reduced> {
        if !self.is_linear() || !other.is_linear() {
            return None;
        }
        fits(self.factor.bits() + other.factor.bits())?;
        Some(Reduced::new(
            self.dimension.checked_mul(&other.dimension)?,
            &self.factor * &other.factor,
        ))
    }

    /// The quotient of two linear units. `None` if either has an offset, an
    /// exponent overflows, or the factor would be too large.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// let v = reg.reduce("km")?.checked_div(&reg.reduce("h")?);
    /// assert_eq!(v.map(|v| v.factor().to_string()), Some("5/18".to_owned()));
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn checked_div(&self, other: &Reduced) -> Option<Reduced> {
        if !self.is_linear() || !other.is_linear() {
            return None;
        }
        fits(self.factor.bits() + other.factor.bits())?;
        Some(Reduced::new(
            self.dimension.checked_div(&other.dimension)?,
            self.factor.checked_div(&other.factor)?,
        ))
    }

    /// The `n`th power of a linear unit. `None` if it has an offset, an
    /// exponent overflows, or the factor would be too large.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// assert_eq!(reg.reduce("cm")?.checked_pow(3), Some(reg.reduce("mL")?));
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn checked_pow(&self, n: i32) -> Option<Reduced> {
        if !self.is_linear() {
            return None;
        }
        fits(
            self.factor
                .bits()
                .checked_mul(u64::from(n.unsigned_abs()))?,
        )?;
        Some(Reduced::new(
            self.dimension.checked_pow(n)?,
            self.factor.checked_pow(n)?,
        ))
    }

    /// A value in this unit, in base units: `factor × value + offset`.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// let k = reg.reduce("Cel")?.to_base(&"20".parse()?);
    /// assert_eq!(k.to_string(), "293.15");
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn to_base(&self, value: &Rational) -> Rational {
        &(&self.factor * value) + &self.offset
    }

    /// A value in base units, in this unit: `(base − offset) / factor`.
    /// `None` only for a zero factor, which [`Reduced::new`] allows and no
    /// registry produces.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// let f = reg.reduce("[degF]")?.from_base(&"273.15".parse()?);
    /// assert_eq!(f.map(|f| f.to_string()), Some("32".to_owned()));
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    #[must_use]
    pub fn from_base(&self, base: &Rational) -> Option<Rational> {
        (base - &self.offset).checked_div(&self.factor)
    }
}

/// Which UCUM a [`Registry`] reads: what a citation of it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct UcumSource {
    /// The UCUM version, such as `2.2`.
    pub ucum_version: &'static str,
    /// The revision date of its essence file, such as `2024-06-17`.
    pub revision_date: &'static str,
    /// The ucum-units release whose essence file it is.
    pub ucum_units_version: &'static str,
}

/// The most bits a reduced factor's numerator or denominator may have:
/// about 2,466 decimal digits.
///
/// Real units stay well inside it: `Ym64`, 10^1536 metres to the 64th, is
/// about 5,100 bits. It keeps a short expression from making exact
/// arithmetic slow: without it, `[pi]64` written 120 times, under 1 KB,
/// took minutes.
pub const MAX_FACTOR_BITS: u64 = 8192;

fn fits(bits: u64) -> Option<()> {
    (bits <= MAX_FACTOR_BITS).then_some(())
}

/// What an atom is, once resolved.
#[derive(Clone, Debug)]
enum Value {
    /// A linear unit, or a temperature with an offset.
    Reduced(Reduced),
    /// A special unit with no linear relation (logarithms, `[pH]`).
    Special,
    /// An arbitrary unit: commensurable with nothing.
    Arbitrary,
}

#[derive(Clone, Debug)]
struct Entry {
    metric: bool,
    declared: bool,
    value: Result<Value, Error>,
}

#[derive(Clone, Debug)]
struct Kind {
    def: QuantityKind,
    /// The allowed units in normal form.
    allowed: Option<Vec<String>>,
}

/// UCUM's atoms and prefixes, with any declared base dimensions, units and
/// quantity kinds.
///
/// [`Registry::ucum`] is standard UCUM, read from the essence file that the
/// pinned ucum-units bundles, with every value kept exact.
/// [`Registry::with_definitions`] adds declarations to it.
///
/// What it does with an expression:
///
/// | Method | Gives |
/// |---|---|
/// | [`validate`](Registry::validate) | whether every symbol resolves |
/// | [`reduce`](Registry::reduce) | its [`Reduced`] form: dimension and exact factor |
/// | [`convert`](Registry::convert) | a value converted exactly to a commensurable unit |
/// | [`check_kind`](Registry::check_kind) | whether a unit measures a declared kind |
/// | [`convert_within`](Registry::convert_within) | a conversion that keeps to one kind |
///
/// Each of those also has ucum-units read the expression (with declared
/// atoms stood in for), so nothing ucum-units refuses is accepted.
///
/// ```
/// use pubunit::{Definitions, Registry};
///
/// let reg = Registry::with_definitions(&Definitions::from_json(r#"{
///   "dimensions": [{ "symbol": "{tray}", "name": "growing tray" }]
/// }"#)?)?;
/// let r = reg.reduce("kW.h/{tray}")?;
/// assert_eq!(r.dimension().to_string(), "m2.s-2.g.{tray}-1");
/// assert!(reg.convert(&"1".parse()?, "kW.h/{tray}", "kW.h").is_err());
/// # Ok::<(), pubunit::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Registry {
    atoms: BTreeMap<String, Entry>,
    bases: BTreeSet<String>,
    kinds: BTreeMap<String, Kind>,
    annotations: Annotations,
}

/// The stand-in for a declared atom when ucum-units reads an expression.
const STAND_IN: &str = "{declared}";

/// The longest prefixes first, so `da` is tried before `d`.
fn prefixes() -> &'static [(&'static str, Rational)] {
    static P: OnceLock<Vec<(&'static str, Rational)>> = OnceLock::new();
    P.get_or_init(|| {
        let mut v: Vec<_> = PREFIXES
            .iter()
            .filter_map(|p| Some((p.code, p.value.parse().ok()?)))
            .collect();
        v.sort_by_key(|(code, _)| core::cmp::Reverse(code.len()));
        v
    })
}

fn ucum_atoms() -> &'static BTreeMap<String, Entry> {
    static ATOMS_RESOLVED: OnceLock<BTreeMap<String, Entry>> = OnceLock::new();
    ATOMS_RESOLVED.get_or_init(resolve_ucum)
}

/// Resolve every UCUM atom from its definition. Atoms are defined in terms
/// of each other in no particular order, so this resolves in passes until
/// no pass makes progress.
fn resolve_ucum() -> BTreeMap<String, Entry> {
    let mut reg = Registry {
        atoms: BTreeMap::new(),
        bases: BTreeSet::new(),
        kinds: BTreeMap::new(),
        annotations: Annotations::Ucum,
    };
    for base in UCUM_BASE_UNITS {
        reg.atoms.insert(
            base.to_owned(),
            Entry {
                metric: true,
                declared: false,
                value: Ok(Value::Reduced(Reduced::new(
                    Dimension::base(base),
                    Rational::one(),
                ))),
            },
        );
    }
    let mut pending: Vec<_> = ATOMS.iter().collect();
    loop {
        let before = pending.len();
        let mut failed = Vec::new();
        for atom in pending {
            let value = if atom.arbitrary {
                Ok(Value::Arbitrary)
            } else {
                atom_value(&reg, atom)
            };
            match value {
                Ok(v) => {
                    reg.atoms.insert(
                        atom.code.to_owned(),
                        Entry {
                            metric: atom.metric,
                            declared: false,
                            value: Ok(v),
                        },
                    );
                }
                Err(e) => failed.push((atom, e)),
            }
        }
        if failed.is_empty() {
            break;
        }
        if failed.len() == before {
            // No progress: record each failure, so using the atom reports it.
            for (atom, e) in failed {
                reg.atoms.insert(
                    atom.code.to_owned(),
                    Entry {
                        metric: atom.metric,
                        declared: false,
                        value: Err(e),
                    },
                );
            }
            break;
        }
        pending = failed.into_iter().map(|(atom, _)| atom).collect();
    }
    reg.atoms
}

fn atom_value(reg: &Registry, atom: &crate::table::Atom) -> Result<Value, Error> {
    let unit = reg.reduce_linear(&parse(atom.unit)?)?;
    let value: Rational = atom.value.parse()?;
    let factor = &value * unit.factor();
    let dimension = unit.dimension().clone();
    Ok(match atom.special {
        None => Value::Reduced(Reduced::new(dimension, factor)),
        Some(Special(name)) => match affine_offset(name) {
            Some(offset) => Value::Reduced(Reduced {
                dimension,
                factor,
                offset,
            }),
            None => Value::Special,
        },
    })
}

/// The offset, in kelvin, of UCUM's temperature functions.
fn affine_offset(function: &str) -> Option<Rational> {
    match function {
        // K = °C + 273.15 and K = °Ré × 5/4 + 273.15.
        "Cel" | "degRe" => Rational::new(27315, 100),
        // K = (°F + 459.67) × 5/9.
        "degF" => Rational::new(45967, 180),
        _ => None,
    }
}

impl Registry {
    /// Standard UCUM, with no declarations, reading undeclared annotations
    /// strictly.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// assert!(reg.validate("mg/dL").is_ok());
    /// assert!(reg.validate("[EUR]").is_err());
    /// ```
    #[must_use]
    pub fn ucum() -> Registry {
        Registry {
            atoms: ucum_atoms().clone(),
            bases: BTreeSet::new(),
            kinds: BTreeMap::new(),
            annotations: Annotations::Strict,
        }
    }

    /// Standard UCUM plus `defs`: their dimensions first, then their units
    /// in order, then their kinds.
    ///
    /// # Errors
    ///
    /// Fails with [`Error::Definition`] when a declaration cannot be used:
    /// - a symbol that is not `{...}` or `[...]`, or that UCUM or an earlier
    ///   declaration already defines;
    /// - an empty name, or a kind's name used twice;
    /// - a unit's value that is not a number greater than zero, or its unit
    ///   that does not reduce, or is not linear;
    /// - a kind whose dimension names an unknown base, whose `units` is
    ///   empty, or lists a unit that does not reduce to that dimension.
    ///
    /// ```
    /// use pubunit::{Definitions, Error, Registry};
    ///
    /// let clash = Definitions::from_json(r#"{"dimensions": [{"symbol": "[ft_i]", "name": "foot"}]}"#)?;
    /// assert!(matches!(Registry::with_definitions(&clash), Err(Error::Definition { .. })));
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn with_definitions(defs: &Definitions) -> Result<Registry, Error> {
        let mut reg = Registry::ucum();
        reg.declare(defs)?;
        Ok(reg)
    }

    /// Read undeclared annotations as `annotations` says.
    ///
    /// ```
    /// use pubunit::{Annotations, Registry};
    /// assert!(Registry::ucum().validate("{cells}/uL").is_err());
    /// assert!(Registry::ucum().with_annotations(Annotations::Ucum).validate("{cells}/uL").is_ok());
    /// ```
    #[must_use]
    pub fn with_annotations(mut self, annotations: Annotations) -> Registry {
        self.annotations = annotations;
        self
    }

    /// Where the UCUM atoms come from: the UCUM version, its essence
    /// file's revision date, and the ucum-units release that bundles it.
    ///
    /// ```
    /// let source = pubunit::Registry::ucum_source();
    /// assert_eq!(source.ucum_version, "2.2");
    /// assert_eq!(source.ucum_units_version, "0.1.0");
    /// ```
    #[must_use]
    pub fn ucum_source() -> UcumSource {
        UcumSource {
            ucum_version: crate::ucum_table::UCUM_VERSION,
            revision_date: crate::ucum_table::UCUM_REVISION_DATE,
            ucum_units_version: crate::ucum_table::UCUM_UNITS_VERSION,
        }
    }

    /// Every UCUM atom's code, base units first, in table order.
    ///
    /// ```
    /// let atoms: Vec<_> = pubunit::Registry::ucum_atoms().collect();
    /// assert_eq!(atoms[0], "m");
    /// assert!(atoms.contains(&"[ft_i]"));
    /// ```
    pub fn ucum_atoms() -> impl Iterator<Item = &'static str> {
        UCUM_BASE_UNITS
            .into_iter()
            .chain(ATOMS.iter().map(|a| a.code))
    }

    /// The declared base dimensions' symbols.
    ///
    /// ```
    /// use pubunit::{Definitions, Registry};
    /// let defs = Definitions::from_json(r#"{"dimensions": [{"symbol": "[EUR]", "name": "euro"}]}"#)?;
    /// let reg = Registry::with_definitions(&defs)?;
    /// assert_eq!(reg.declared_bases().collect::<Vec<_>>(), ["[EUR]"]);
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn declared_bases(&self) -> impl Iterator<Item = &str> {
        self.bases.iter().map(String::as_str)
    }

    /// The declared quantity kinds, by name.
    ///
    /// ```
    /// assert_eq!(pubunit::Registry::ucum().kinds().count(), 0);
    /// ```
    pub fn kinds(&self) -> impl Iterator<Item = &QuantityKind> {
        self.kinds.values().map(|k| &k.def)
    }

    /// The declared kind named `name`.
    ///
    /// ```
    /// assert!(pubunit::Registry::ucum().kind("length").is_none());
    /// ```
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<&QuantityKind> {
        self.kinds.get(name).map(|k| &k.def)
    }

    /// Parse `expr`, have ucum-units read it, and resolve every symbol.
    /// This accepts special and arbitrary units, which
    /// [`reduce`](Registry::reduce) refuses.
    ///
    /// # Errors
    ///
    /// Fails on a syntax error ([`Error::Syntax`], [`Error::TooLarge`]),
    /// when ucum-units refuses the expression ([`Error::Ucum`]), on a symbol
    /// that does not resolve ([`Error::UnknownSymbol`]), and on an
    /// undeclared annotation standing alone under
    /// [`Annotations::Strict`] ([`Error::UndeclaredAnnotation`]).
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// assert!(reg.validate("[IU]/L").is_ok());
    /// assert!(reg.validate("kmeter").is_err());
    /// ```
    pub fn validate(&self, expr: &str) -> Result<(), Error> {
        self.read(expr).map(|_| ())
    }

    /// Reduce `expr` to its dimension and exact factor.
    ///
    /// # Errors
    ///
    /// Fails as [`validate`](Registry::validate) does, and also on a special
    /// unit ([`Error::NotLinear`]) unless it is a temperature with an
    /// offset standing alone, and on an arbitrary unit
    /// ([`Error::Arbitrary`]). [`Error::TooLarge`] reports an exponent that
    /// overflows.
    ///
    /// ```
    /// use pubunit::{Error, Registry};
    /// let reg = Registry::ucum();
    /// assert_eq!(reg.reduce("[ft_i]")?.factor().to_string(), "0.3048");
    /// assert!(matches!(reg.reduce("dB"), Err(Error::NotLinear { .. })));
    /// assert!(matches!(reg.reduce("Cel/s"), Err(Error::NotLinear { .. })));
    /// assert!(matches!(reg.reduce("[IU]"), Err(Error::Arbitrary { .. })));
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn reduce(&self, expr: &str) -> Result<Reduced, Error> {
        let term = self.read(expr)?;
        self.reduce_term(&term)
    }

    /// Reduce a parsed term, without having ucum-units read it: for a tree
    /// built in code, such as by symbolic computation.
    ///
    /// # Errors
    ///
    /// As [`reduce`](Registry::reduce), less the ucum-units check.
    ///
    /// ```
    /// use pubunit::Term;
    /// let t = Term::Mul(Box::new(pubunit::parse("N")?), Box::new(pubunit::parse("m")?));
    /// let reg = pubunit::Registry::ucum();
    /// assert_eq!(reg.reduce_term(&t)?, reg.reduce("J")?);
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn reduce_term(&self, term: &Term) -> Result<Reduced, Error> {
        // A temperature with an offset reduces only standing alone.
        if let Term::Symbol {
            symbol,
            exponent: 1,
            annotation,
        } = peel(term)
            && let Some(Entry {
                value: Ok(Value::Reduced(r)),
                ..
            }) = self.atoms.get(symbol)
            && !r.is_linear()
            && !annotation
                .as_ref()
                .is_some_and(|a| self.atoms.contains_key(&format!("{{{a}}}")))
        {
            return Ok(r.clone());
        }
        self.reduce_linear(term)
    }

    /// Convert `value` from one unit to another, exactly.
    ///
    /// # Errors
    ///
    /// Fails as [`reduce`](Registry::reduce) does on either unit, and with
    /// [`Error::Incommensurable`] when their dimensions differ.
    ///
    /// ```
    /// let reg = pubunit::Registry::ucum();
    /// let v = reg.convert(&"1".parse()?, "[mi_i]", "km")?;
    /// assert_eq!(v.to_string(), "1.609344");
    /// let t = reg.convert(&"98.6".parse()?, "[degF]", "Cel")?;
    /// assert_eq!(t.to_string(), "37");
    /// assert!(reg.convert(&"1".parse()?, "kg", "m").is_err());
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn convert(&self, value: &Rational, from: &str, to: &str) -> Result<Rational, Error> {
        let f = self.reduce(from)?;
        let t = self.reduce(to)?;
        if f.dimension != t.dimension {
            return Err(Error::Incommensurable {
                from: from.to_owned(),
                from_dimension: f.dimension,
                to: to.to_owned(),
                to_dimension: t.dimension,
            });
        }
        t.from_base(&f.to_base(value)).ok_or_else(|| Error::Number {
            text: to.to_owned(),
            message: "a unit with a zero factor".to_owned(),
        })
    }

    /// Check that `unit` measures the declared kind `kind`: it has the
    /// kind's dimension and, when the kind lists units, is one of them.
    /// Returns the unit reduced.
    ///
    /// # Errors
    ///
    /// Fails as [`reduce`](Registry::reduce) does, with
    /// [`Error::UnknownKind`] when no kind has that name, with
    /// [`Error::WrongDimension`] when the dimensions differ, and with
    /// [`Error::NotAllowed`] when the kind lists units and `unit` is not
    /// one of them.
    ///
    /// ```
    /// use pubunit::{Definitions, Error, Registry};
    ///
    /// let reg = Registry::with_definitions(&Definitions::from_json(r#"{
    ///   "kinds": [
    ///     { "name": "energy", "dimension": {"m": 2, "s": -2, "g": 1}, "units": ["J", "kJ", "kW.h"] },
    ///     { "name": "torque", "dimension": {"m": 2, "s": -2, "g": 1}, "units": ["N.m"] }
    ///   ]
    /// }"#)?)?;
    /// assert!(reg.check_kind("kW.h", "energy").is_ok());
    /// assert!(matches!(reg.check_kind("J", "torque"), Err(Error::NotAllowed { .. })));
    /// assert!(matches!(reg.check_kind("kg", "energy"), Err(Error::WrongDimension { .. })));
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn check_kind(&self, unit: &str, kind: &str) -> Result<Reduced, Error> {
        let k = self.kinds.get(kind).ok_or_else(|| Error::UnknownKind {
            kind: kind.to_owned(),
        })?;
        let term = self.read(unit)?;
        let r = self.reduce_term(&term)?;
        if r.dimension != k.def.dimension {
            return Err(Error::WrongDimension {
                unit: unit.to_owned(),
                kind: kind.to_owned(),
                expected: k.def.dimension.clone(),
                found: r.dimension,
            });
        }
        if let Some(allowed) = &k.allowed
            && !allowed.contains(&term.to_string())
        {
            return Err(Error::NotAllowed {
                unit: unit.to_owned(),
                kind: kind.to_owned(),
                allowed: allowed.clone(),
            });
        }
        Ok(r)
    }

    /// Convert `value` between two units that both measure `kind`.
    ///
    /// # Errors
    ///
    /// Fails as [`check_kind`](Registry::check_kind) does on either unit.
    ///
    /// ```
    /// use pubunit::{Definitions, Registry};
    ///
    /// let reg = Registry::with_definitions(&Definitions::from_json(r#"{
    ///   "dimensions": [{ "symbol": "{CO2e}", "name": "carbon dioxide equivalent" }],
    ///   "kinds": [{ "name": "climate impact", "dimension": {"g": 1, "{CO2e}": 1} }]
    /// }"#)?)?;
    /// let v = reg.convert_within(&"2.5".parse()?, "t{CO2e}", "kg{CO2e}", "climate impact")?;
    /// assert_eq!(v.to_string(), "2500");
    /// assert!(reg.convert_within(&"1".parse()?, "kg{CO2e}", "kg", "climate impact").is_err());
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn convert_within(
        &self,
        value: &Rational,
        from: &str,
        to: &str,
        kind: &str,
    ) -> Result<Rational, Error> {
        self.check_kind(from, kind)?;
        self.check_kind(to, kind)?;
        self.convert(value, from, to)
    }

    fn declare(&mut self, defs: &Definitions) -> Result<(), Error> {
        for d in &defs.dimensions {
            self.check_new_symbol(&d.symbol, &d.name)?;
            self.bases.insert(d.symbol.clone());
            self.insert_declared(
                &d.symbol,
                Reduced::new(Dimension::base(d.symbol.clone()), Rational::one()),
            );
        }
        for u in &defs.units {
            self.check_new_symbol(&u.symbol, &u.name)?;
            let bad = |message: String| Error::Definition {
                name: u.symbol.clone(),
                message,
            };
            let value: Rational = u.value.parse().map_err(|e: Error| bad(e.to_string()))?;
            if value <= Rational::zero() {
                return Err(bad(format!("the value {value} is not greater than zero")));
            }
            let unit = self
                .reduce(&u.unit)
                .map_err(|e| bad(format!("its unit: {e}")))?;
            if !unit.is_linear() {
                return Err(bad(format!("its unit `{}` has an offset", u.unit)));
            }
            if value.bits() + unit.factor().bits() > MAX_FACTOR_BITS {
                return Err(bad(format!("its factor is beyond {MAX_FACTOR_BITS} bits")));
            }
            let factor = &value * unit.factor();
            self.insert_declared(&u.symbol, Reduced::new(unit.dimension, factor));
        }
        for k in &defs.kinds {
            self.declare_kind(k)?;
        }
        Ok(())
    }

    fn declare_kind(&mut self, k: &QuantityKind) -> Result<(), Error> {
        let bad = |message: String| Error::Definition {
            name: k.name.clone(),
            message,
        };
        if k.name.trim().is_empty() {
            return Err(bad("a kind needs a name".to_owned()));
        }
        if self.kinds.contains_key(&k.name) {
            return Err(bad("a kind of that name is already declared".to_owned()));
        }
        for (base, _) in k.dimension.iter() {
            if !UCUM_BASE_UNITS.contains(&base) && !self.bases.contains(base) {
                return Err(bad(format!("`{base}` is not a base dimension")));
            }
        }
        let allowed = match &k.units {
            None => None,
            Some(units) if units.is_empty() => {
                return Err(bad(
                    "`units` is empty; leave it out to allow any unit of the dimension".to_owned(),
                ));
            }
            Some(units) => {
                let mut normal = Vec::new();
                for u in units {
                    let term = self.read(u).map_err(|e| bad(e.to_string()))?;
                    let r = self.reduce_term(&term).map_err(|e| bad(e.to_string()))?;
                    if r.dimension != k.dimension {
                        return Err(bad(format!(
                            "`{u}` is {}, not {}",
                            r.dimension, k.dimension
                        )));
                    }
                    normal.push(term.to_string());
                }
                Some(normal)
            }
        };
        self.kinds.insert(
            k.name.clone(),
            Kind {
                def: k.clone(),
                allowed,
            },
        );
        Ok(())
    }

    fn check_new_symbol(&self, symbol: &str, name: &str) -> Result<(), Error> {
        let bad = |message: &str| Error::Definition {
            name: symbol.to_owned(),
            message: message.to_owned(),
        };
        if name.trim().is_empty() {
            return Err(bad("a declaration needs a name"));
        }
        let round_trips = match parse(symbol) {
            Ok(Term::Annotation(text)) => format!("{{{text}}}") == symbol,
            Ok(Term::Symbol {
                symbol: s,
                exponent: 1,
                annotation: None,
            }) => {
                s == symbol && s.starts_with('[') && s.ends_with(']') && s.matches('[').count() == 1
            }
            _ => false,
        };
        if !round_trips {
            return Err(bad(
                "a declared symbol is one brace annotation (`{tray}`) or one bracketed atom (`[EUR]`)",
            ));
        }
        if self.atoms.contains_key(symbol) {
            return Err(bad("this symbol is already defined"));
        }
        Ok(())
    }

    fn insert_declared(&mut self, symbol: &str, r: Reduced) {
        self.atoms.insert(
            symbol.to_owned(),
            Entry {
                metric: false,
                declared: true,
                value: Ok(Value::Reduced(r)),
            },
        );
    }

    /// Parse, resolve every symbol, and have ucum-units read the
    /// expression with every declared atom replaced by an annotation.
    fn read(&self, expr: &str) -> Result<Term, Error> {
        let term = parse(expr)?;
        self.resolve_all(&term)?;
        let mut view = String::new();
        self.ucum_view(&term, &mut view, false);
        ucum::validate(&view).map_err(|e| Error::Ucum {
            expr: view.clone(),
            message: e.to_string(),
        })?;
        Ok(term)
    }

    fn ucum_view(&self, t: &Term, out: &mut String, paren: bool) {
        let t = peel(t);
        let compound = matches!(t, Term::Mul(..) | Term::Div(..) | Term::Recip(..));
        if paren && compound {
            out.push('(');
        }
        match t {
            Term::Symbol { symbol, .. } if self.atoms.get(symbol).is_some_and(|e| e.declared) => {
                out.push_str(STAND_IN);
            }
            Term::Mul(a, b) | Term::Div(a, b) => {
                self.ucum_view(a, out, is_recip(a));
                out.push(if matches!(t, Term::Mul(..)) { '.' } else { '/' });
                self.ucum_view(b, out, true);
            }
            Term::Recip(a) => {
                out.push('/');
                self.ucum_view(a, out, is_recip(a));
            }
            other => out.push_str(&other.to_string()),
        }
        if paren && compound {
            out.push(')');
        }
    }

    /// The atom a symbol names, with the prefix factor if it has one.
    fn lookup(&self, symbol: &str) -> Result<(Option<&Rational>, &Entry), Error> {
        if let Some(e) = self.atoms.get(symbol) {
            return Ok((None, e));
        }
        for (code, factor) in prefixes() {
            if let Some(rest) = symbol.strip_prefix(code)
                && let Some(e) = self.atoms.get(rest)
                && e.metric
            {
                return Ok((Some(factor), e));
            }
        }
        Err(Error::UnknownSymbol {
            symbol: symbol.to_owned(),
        })
    }

    /// A declared annotation's unit, if `text` is one.
    fn declared_annotation(&self, text: &str) -> Option<&Reduced> {
        match self.atoms.get(&format!("{{{text}}}")) {
            Some(Entry {
                declared: true,
                value: Ok(Value::Reduced(r)),
                ..
            }) => Some(r),
            _ => None,
        }
    }

    fn resolve_all(&self, t: &Term) -> Result<(), Error> {
        match t {
            Term::Factor { .. } => Ok(()),
            Term::Symbol { symbol, .. } => self.lookup(symbol).map(|_| ()),
            Term::Annotation(text) => self.annotation(text).map(|_| ()),
            Term::Mul(a, b) | Term::Div(a, b) => {
                self.resolve_all(a)?;
                self.resolve_all(b)
            }
            Term::Recip(a) | Term::Group(a) => self.resolve_all(a),
        }
    }

    fn annotation(&self, text: &str) -> Result<Reduced, Error> {
        if let Some(r) = self.declared_annotation(text) {
            return Ok(r.clone());
        }
        match self.annotations {
            Annotations::Ucum => Ok(Reduced::new(Dimension::dimensionless(), Rational::one())),
            Annotations::Strict => Err(Error::UndeclaredAnnotation {
                annotation: format!("{{{text}}}"),
            }),
        }
    }

    /// Multiply by a declared annotation's unit, if the annotation is one.
    fn annotate(&self, r: Reduced, annotation: Option<&String>) -> Result<Reduced, Error> {
        match annotation.and_then(|a| self.declared_annotation(a)) {
            Some(a) => r.checked_mul(a).ok_or_else(overflow),
            None => Ok(r),
        }
    }

    fn reduce_linear(&self, t: &Term) -> Result<Reduced, Error> {
        match t {
            Term::Factor { value, annotation } => {
                let n =
                    Rational::from_integer(i64::try_from(*value).map_err(|_| Error::TooLarge {
                        what: format!("the factor {value}"),
                    })?);
                self.annotate(
                    Reduced::new(Dimension::dimensionless(), n),
                    annotation.as_ref(),
                )
            }
            Term::Symbol {
                symbol,
                exponent,
                annotation,
            } => {
                let (prefix, entry) = self.lookup(symbol)?;
                let r = match &entry.value {
                    Err(e) => return Err(e.clone()),
                    Ok(Value::Arbitrary) => {
                        return Err(Error::Arbitrary {
                            symbol: symbol.clone(),
                        });
                    }
                    Ok(Value::Special) => {
                        return Err(Error::NotLinear {
                            symbol: symbol.clone(),
                        });
                    }
                    Ok(Value::Reduced(r)) if !r.is_linear() => {
                        return Err(Error::NotLinear {
                            symbol: symbol.clone(),
                        });
                    }
                    Ok(Value::Reduced(r)) => r,
                };
                let r = match prefix {
                    Some(p) => Reduced::new(r.dimension.clone(), p * &r.factor),
                    None => r.clone(),
                };
                let r = r.checked_pow(*exponent).ok_or_else(overflow)?;
                self.annotate(r, annotation.as_ref())
            }
            Term::Annotation(text) => self.annotation(text),
            Term::Mul(a, b) => self
                .reduce_linear(a)?
                .checked_mul(&self.reduce_linear(b)?)
                .ok_or_else(overflow),
            Term::Div(a, b) => self
                .reduce_linear(a)?
                .checked_div(&self.reduce_linear(b)?)
                .ok_or_else(overflow),
            Term::Recip(a) => Reduced::new(Dimension::dimensionless(), Rational::one())
                .checked_div(&self.reduce_linear(a)?)
                .ok_or_else(overflow),
            Term::Group(a) => self.reduce_linear(a),
        }
        .and_then(|r| {
            // ucum-units keeps UCUM's exponents in an i8; stay inside it, so
            // the two never disagree about a dimension.
            if r.dimension
                .ucum_part()
                .iter()
                .all(|e| i8::try_from(*e).is_ok())
            {
                Ok(r)
            } else {
                Err(overflow())
            }
        })
    }
}

fn overflow() -> Error {
    Error::TooLarge {
        what: format!("an exponent beyond ±127, or a factor beyond {MAX_FACTOR_BITS} bits"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: Registry::reduce
    // Found while timing reduction: `[pi]64` written 120 times grew factors
    // to half a million digits, normalised by a gcd at every step, and ran
    // for minutes. A factor is now bounded by MAX_FACTOR_BITS.
    #[test]
    fn a_huge_factor_is_refused_quickly() {
        let reg = Registry::ucum();
        let expr = vec!["[pi]64"; 120].join(".");
        let start = std::time::Instant::now();
        assert!(matches!(reg.reduce(&expr), Err(Error::TooLarge { .. })));
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        // Large but real factors still reduce.
        assert!(reg.reduce("Ym64").is_ok());
        assert!(reg.reduce("[pi]16").is_ok());
    }
}
