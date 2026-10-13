//! The definitions format: base dimensions, units and quantity kinds,
//! declared as data.

use serde::{Deserialize, Serialize};

use crate::{Dimension, Error};

/// Everything a [`Registry`](crate::Registry) adds to UCUM, as data.
///
/// The format is plain serde, so definitions can come from a JSON file, a
/// publet (through an adapter that lives in publet), or code. Unknown
/// fields are refused, so a misspelt field fails rather than being ignored.
///
/// Order matters within `units`: a unit may be defined in terms of UCUM,
/// the declared dimensions, and the units declared before it. So there
/// are never cycles.
///
/// ```
/// use pubunit::Definitions;
///
/// let defs = Definitions::from_json(r#"{
///   "dimensions": [
///     { "symbol": "{tray}", "name": "growing tray" },
///     { "symbol": "{CO2e}", "name": "carbon dioxide equivalent" }
///   ],
///   "units": [
///     { "symbol": "{flat}", "name": "flat of trays", "value": "6", "unit": "{tray}" }
///   ],
///   "kinds": [
///     { "name": "climate impact", "dimension": { "g": 1, "{CO2e}": 1 },
///       "units": ["g{CO2e}", "kg{CO2e}", "t{CO2e}"] }
///   ]
/// }"#)?;
/// assert_eq!(defs.dimensions.len(), 2);
/// assert_eq!(defs.kinds[0].dimension.to_string(), "g.{CO2e}");
/// # Ok::<(), pubunit::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definitions {
    /// New base dimensions, each with the unit that measures one of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<BaseDimension>,
    /// New units, defined as a multiple of an existing unit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<UnitDefinition>,
    /// Quantity kinds: what a column measures, beyond its dimension.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<QuantityKind>,
}

/// The QUDT release [`Definitions::qudt`] was generated from.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct QudtSource {
    /// The QUDT release, such as `3.5.2`.
    pub version: String,
    /// Where the release archive was downloaded from.
    pub url: String,
    /// The archive's SHA-256, in hex.
    pub sha256: String,
    /// QUDT's licence, and the attribution it asks for.
    pub license: String,
}

/// data/qudt/kinds.json, written by tools/gen-qudt-kinds.py.
const QUDT_KINDS: &str = include_str!("../data/qudt/kinds.json");
/// data/qudt/source.json: the release that file was generated from.
const QUDT_SOURCE: &str = include_str!("../data/qudt/source.json");

impl Definitions {
    /// QUDT's quantity kinds, from the release [`Definitions::qudt_source`]
    /// names: each kind QUDT gives a dimension that UCUM can express, with
    /// the UCUM codes of its applicable units that reduce to it.
    ///
    /// Each is named `qudt:` and its QUDT local name, labelled with QUDT's
    /// English label, and carries its QUDT URI as `id`. The set is generated
    /// at build time and checked against the pinned release; pubunit reads
    /// no RDF. The crate's `data/qudt/REPORT.md` lists the kinds left out
    /// and why: amount of substance, which UCUM counts as a number, is the
    /// commonest reason.
    ///
    /// ```
    /// use pubunit::{Definitions, Registry};
    ///
    /// let reg = Registry::with_definitions(&Definitions::qudt()?)?;
    /// reg.check_kind("kW.h", "qudt:Energy")?;
    /// reg.check_kind("N.m", "qudt:Torque")?;
    /// // Energy and torque share a dimension, but not their units.
    /// assert!(reg.check_kind("N.m", "qudt:Energy").is_err());
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Fails with [`Error::Json`] only if the embedded file is malformed,
    /// which the crate's tests rule out.
    pub fn qudt() -> Result<Definitions, Error> {
        Definitions::from_json(QUDT_KINDS)
    }

    /// The QUDT release [`Definitions::qudt`] comes from.
    ///
    /// ```
    /// let source = pubunit::Definitions::qudt_source()?;
    /// assert_eq!(source.version, "3.5.2");
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Fails with [`Error::Json`] only if the embedded file is malformed.
    pub fn qudt_source() -> Result<QudtSource, Error> {
        serde_json::from_str(QUDT_SOURCE).map_err(|e| Error::Json {
            message: e.to_string(),
        })
    }

    /// Append `other`'s dimensions, units and kinds to these, in order.
    ///
    /// ```
    /// use pubunit::{Definitions, Registry};
    ///
    /// let mut defs = Definitions::from_json(r#"{
    ///   "dimensions": [{ "symbol": "{tray}", "name": "growing tray" }]
    /// }"#)?;
    /// defs.extend(Definitions::qudt()?);
    /// let reg = Registry::with_definitions(&defs)?;
    /// assert_eq!(reg.reduce("kW.h/{tray}")?.dimension().to_string(), "m2.s-2.g.{tray}-1");
    /// assert!(reg.kind("qudt:Energy").is_some());
    /// # Ok::<(), pubunit::Error>(())
    /// ```
    pub fn extend(&mut self, other: Definitions) {
        self.dimensions.extend(other.dimensions);
        self.units.extend(other.units);
        self.kinds.extend(other.kinds);
    }

    /// Read definitions from JSON.
    ///
    /// # Errors
    ///
    /// Fails with [`Error::Json`] when the text is not JSON of this format,
    /// including when it has a field the format does not define.
    ///
    /// ```
    /// assert!(pubunit::Definitions::from_json(r#"{"dimension": []}"#).is_err());
    /// ```
    pub fn from_json(json: &str) -> Result<Definitions, Error> {
        serde_json::from_str(json).map_err(|e| Error::Json {
            message: e.to_string(),
        })
    }
}

/// A base dimension declared as data, such as counted trays, a currency or
/// a substance equivalent.
///
/// UCUM treats `{tray}` as an annotation with no dimension, and has no
/// currencies. Declaring `{tray}` as a base makes it a dimension of its
/// own, so kW.h/{tray} and kW.h do not convert into each other.
///
/// The symbol is a brace annotation (`{tray}`, `{CO2e}`) or a bracketed
/// atom UCUM does not define (`[EUR]`). A brace symbol counts wherever it
/// appears: standing alone (`kW.h/{tray}`) or after a unit (`kg{CO2e}`, a
/// kilogram of CO2 equivalent, whose dimension is `g.{CO2e}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseDimension {
    /// The unit's symbol, which also names the dimension: `{tray}`.
    pub symbol: String,
    /// What one of it is, for people.
    pub name: String,
    /// An identifier for the definition's source, such as a publet's
    /// content identifier. pubunit keeps it and does not read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// A unit declared as an exact multiple of another: one `symbol` is `value`
/// times `unit`.
///
/// A unit of trays defined as an area (`"value": "0.125", "unit": "m2"`)
/// would make kW.h/{tray} convertible into kW.h/m2. Declare `{tray}` as a
/// [`BaseDimension`] to keep the two apart, and state its area as a fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDefinition {
    /// The unit's symbol: a brace annotation or an undefined bracketed atom.
    pub symbol: String,
    /// What one of it is, for people.
    pub name: String,
    /// The multiplier, as a decimal string (`"0.125"`) or a fraction
    /// (`"5/9"`). It must be greater than zero.
    pub value: String,
    /// A UCUM expression for the unit multiplied, which may use declared
    /// dimensions and the units declared before this one.
    pub unit: String,
    /// An identifier for the definition's source. Kept, not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// A quantity kind: a name, a dimension, and the units that measure it.
///
/// Kinds are declared, never inferred from a dimension, because a
/// dimension does not determine its kind: energy and torque are both
/// `m2.s-2.g`. A unit fits a kind when it has the kind's dimension and,
/// if `units` is given, is one of them, compared in normal form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityKind {
    /// The kind's name: `climate impact`, `electrical energy per tray`.
    /// QUDT's kinds are named `qudt:` and their QUDT local name, such as
    /// `qudt:Energy` (see [`Definitions::qudt`]).
    pub name: String,
    /// What people call it, when that differs from `name`: QUDT's English
    /// label, such as `Energy`. Kept, not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Its dimension.
    pub dimension: Dimension,
    /// The units that may measure it. Absent, any unit of the dimension
    /// may.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<Vec<String>>,
    /// An identifier for the definition's source. Kept, not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}
