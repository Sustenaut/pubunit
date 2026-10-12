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

impl Definitions {
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
    pub name: String,
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
