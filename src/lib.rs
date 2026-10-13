/*!
Exact, kind-aware checking and conversion of [UCUM] units, with base
dimensions and quantity kinds declared as data.

```
use pubunit::{Definitions, Registry};

let reg = Registry::with_definitions(&Definitions::from_json(r#"{
  "dimensions": [
    { "symbol": "{tray}", "name": "growing tray" },
    { "symbol": "{CO2e}", "name": "carbon dioxide equivalent" }
  ],
  "kinds": [
    { "name": "climate impact per tray", "dimension": { "g": 1, "{CO2e}": 1, "{tray}": -1 },
      "units": ["g{CO2e}/{tray}", "kg{CO2e}/{tray}"] }
  ]
}"#)?)?;

// Exact conversion within a kind: no floating point anywhere.
let v = reg.convert_within(&"0.42".parse()?, "kg{CO2e}/{tray}", "g{CO2e}/{tray}",
                           "climate impact per tray")?;
assert_eq!(v.to_string(), "420");

// A kilogram of CO2e is not a kilogram of basil.
assert!(reg.check_kind("kg/{tray}", "climate impact per tray").is_err());
# Ok::<(), pubunit::Error>(())
```

# Overview

| Item | What it is |
|---|---|
| [`parse`], [`Term`] | UCUM's grammar, parsed into a public tree |
| [`Registry`] | UCUM's atoms plus declared ones: [`validate`](Registry::validate), [`reduce`](Registry::reduce), [`convert`](Registry::convert), [`check_kind`](Registry::check_kind) |
| [`Reduced`] | a unit as a [`Dimension`] and an exact factor, with checked `*`, `/` and powers |
| [`Dimension`] | integer exponents over base units, UCUM's seven and declared ones |
| [`Rational`] | the exact numbers every factor and converted value is |
| [`Definitions`] | the serde format for declared [`BaseDimension`]s, [`UnitDefinition`]s and [`QuantityKind`]s, and QUDT's kinds ([`Definitions::qudt`]) |
| [`Error`] | why something failed |

The invariants it keeps:

- **No floating point.** Factors are read from UCUM's own decimal strings
  as rationals, and every result is exact. No floating-point type appears in
  the source, and a guard checks that.
- **Kinds are declared, never inferred.** Energy and torque share the
  dimension `m2.s-2.g`. Only a declaration says which a column measures.
- **Nothing ucum-units refuses is accepted.** Every expression a registry
  reads from text is also read by [ucum-units], with declared atoms stood in
  for by an annotation. A tree built in code and given to
  [`Registry::reduce_term`] is not.
- **Inputs are bounded.** An expression is at most [`MAX_LEN`] bytes,
  [`MAX_DEPTH`] levels deep, with exponents within ±[`MAX_EXPONENT`], and
  a factor's numerator and denominator within [`MAX_FACTOR_BITS`] bits.

The rest of this page:

* [The definitions format](#the-definitions-format): declaring dimensions,
  units and kinds as data.
* [QUDT's quantity kinds](#qudts-quantity-kinds): a starting set of kinds.
* [Examples](#example-a-currency): a short cookbook, failures included.
* [What this version does not do](#what-this-version-does-not-do).
* [Why this crate?](#why-this-crate) and [related crates](#related-crates).
* [Crate features](#crate-features), [minimum Rust
  version](#minimum-rust-version) and [versioning](#versioning).

Longer documents live in [`_documentation`]: how every item is
[tested](crate::_documentation::testing), how each dependency is
[vouched for](crate::_documentation::supply_chain), the project's
[security](crate::_documentation::security) policy, and the
[changelog](crate::_documentation::changelog). The `pubunit` command is
described in its README and by `pubunit --help`.

# The definitions format

[`Definitions`] has three optional lists, read with serde (JSON with
[`Definitions::from_json`]). A field the format does not define is refused.

```json
{
  "dimensions": [
    { "symbol": "{tray}", "name": "growing tray, 25 x 50 cm" },
    { "symbol": "[EUR]",  "name": "euro (ISO 4217 EUR)" },
    { "symbol": "{CO2e}", "name": "carbon dioxide equivalent" }
  ],
  "units": [
    { "symbol": "{flat}", "name": "flat of six trays", "value": "6", "unit": "{tray}" }
  ],
  "kinds": [
    { "name": "climate impact", "dimension": { "g": 1, "{CO2e}": 1 },
      "units": ["g{CO2e}", "kg{CO2e}", "t{CO2e}"] }
  ]
}
```

- **`dimensions`** ([`BaseDimension`]): new base dimensions, each named by
  the symbol of its unit. A symbol is one brace annotation (`{tray}`) or
  one bracketed atom UCUM does not define (`[EUR]`). A brace symbol counts
  wherever it appears. It may stand alone (`kW.h/{tray}`) or follow a unit:
  `kg{CO2e}` is a kilogram of CO2 equivalent, of dimension `g.{CO2e}`.
- **`units`** ([`UnitDefinition`]): one `symbol` is `value` times `unit`.
  The value is a decimal or a fraction, greater than zero. The unit may use
  UCUM, the declared dimensions, and the units declared before it, so there
  are no cycles.
- **`kinds`** ([`QuantityKind`]): a name, a dimension (base symbol to
  exponent; mass is in grams, UCUM's base unit), and optionally the units
  that may measure it, compared in normal form. Without `units`, any unit
  of the dimension may.

Each entry may carry an `id`, such as the content identifier of the publet
that defines it. pubunit keeps it and does not read it, and does not depend
on publet: an adapter in publet turns definition publets into this format.

A brace annotation standing alone that nothing declares, such as a misspelt
`{trays}`, is an error ([`Annotations::Strict`]). UCUM reads it as the
number 1, which [`Annotations::Ucum`] asks for. An undeclared annotation
after a unit (`g{basil}`) is a label, as in UCUM.

Declare a counted thing as a dimension, not as an area. If `{tray}` were
defined as 0.125 `m2`, then kW.h/{tray} would convert into kW.h/m2. A tray's
area is a fact about trays, stated in a publet, not a unit conversion.

# Example: a currency

UCUM has no money. Each currency is declared as a base dimension of its
own, so euros never convert to dollars: an exchange rate is a dated fact,
not a unit.

```
use pubunit::{Definitions, Error, Registry};

let reg = Registry::with_definitions(&Definitions::from_json(r#"{
  "dimensions": [
    { "symbol": "[EUR]", "name": "euro" },
    { "symbol": "[USD]", "name": "US dollar" }
  ]
}"#)?)?;
assert_eq!(reg.reduce("[EUR]/kg")?.dimension().to_string(), "g-1.[EUR]");
assert!(matches!(
    reg.convert(&"1".parse()?, "[EUR]", "[USD]"),
    Err(Error::Incommensurable { .. })
));
# Ok::<(), pubunit::Error>(())
```

# Example: a unit defined by another

A declared unit is an exact multiple of one that already exists.

```
use pubunit::{Definitions, Registry};

let reg = Registry::with_definitions(&Definitions::from_json(r#"{
  "dimensions": [{ "symbol": "{tray}", "name": "growing tray" }],
  "units": [{ "symbol": "{flat}", "name": "flat of six trays", "value": "6", "unit": "{tray}" }]
}"#)?)?;
let v = reg.convert(&"3".parse()?, "{flat}", "{tray}")?;
assert_eq!(v.to_string(), "18");
# Ok::<(), pubunit::Error>(())
```

# QUDT's quantity kinds

[`Definitions::qudt`] gives a starting set of kinds from [QUDT], generated
at build time from a pinned release ([`Definitions::qudt_source`]) and
checked against it, so the set is reproducible and a QUDT update arrives as
a reviewable change. pubunit reads no RDF. Each kind is named `qudt:` and
its QUDT local name, and carries its QUDT URI as `id`.

QUDT's dimension vectors are restated in UCUM's bases: current becomes
charge per second, and plane angle, which QUDT calls dimensionless, keeps
UCUM's `rad`. Kinds UCUM cannot express are left out, chiefly those
involving amount of substance, which UCUM counts as a number. The crate's
`data/qudt/REPORT.md` lists every kind and unit left out, and why.

```
use pubunit::{Definitions, Registry};

let mut defs = Definitions::from_json(r#"{
  "dimensions": [{ "symbol": "{tray}", "name": "growing tray" }]
}"#)?;
defs.extend(Definitions::qudt()?);
let reg = Registry::with_definitions(&defs)?;
reg.check_kind("kW.h", "qudt:Energy")?;
assert!(reg.check_kind("J", "qudt:Torque").is_err());
assert!(reg.check_kind("Hz", "qudt:Activity").is_err());
# Ok::<(), pubunit::Error>(())
```

# Example: what is refused

```
use pubunit::{Error, Registry};

let reg = Registry::ucum();
// UCUM reads {trays} as the number 1; pubunit refuses it unless declared.
assert!(matches!(reg.reduce("kW.h/{trays}"), Err(Error::UndeclaredAnnotation { .. })));
// A logarithmic unit has no linear factor.
assert!(matches!(reg.reduce("dB"), Err(Error::NotLinear { .. })));
// Celsius converts on its own, but a product with it means nothing.
assert_eq!(reg.convert(&"25".parse()?, "Cel", "K")?.to_string(), "298.15");
assert!(matches!(reg.reduce("Cel.m"), Err(Error::NotLinear { .. })));
// Results that are not finite decimals stay exact.
assert_eq!(reg.convert(&"1".parse()?, "[ft_i]", "[yd_i]")?.to_string(), "1/3");
# Ok::<(), pubunit::Error>(())
```

# Example: the parse tree and dimension algebra

The tree and the algebra are public, so that symbolic computation can
carry units through a formula.

```
use pubunit::{Registry, Term};

let reg = Registry::ucum();
let power = reg.reduce("W")?;
let time = reg.reduce("h")?;
let energy = power.checked_mul(&time).ok_or("overflow")?;
assert_eq!(energy, reg.reduce("W.h")?);

let tree = pubunit::parse("kg.m/s2")?;
assert!(matches!(tree, Term::Div(..)));
assert_eq!(reg.reduce_term(&tree)?, reg.reduce("N")?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

# What this version does not do

- **Special units.** Logarithmic units (`dB`, `[pH]`, `Np`) have no linear
  factor, and [`Registry::reduce`] and [`Registry::convert`] refuse them.
  Temperatures with an offset (`Cel`, `[degF]`, `[degRe]`) convert when
  they stand alone, and are refused inside a product. Arbitrary units
  (`[IU]`) are commensurable with nothing.
- **Case-insensitive UCUM** (`c/i`): only the case-sensitive form is read.
- **Prefixes on declared units:** `k[EUR]` is not a unit.
- **Amount of substance:** UCUM counts moles as a number, so QUDT's kinds
  that involve it are left out of [`Definitions::qudt`].
- **Checking derivations:** pubunit gives the dimension of a formula's
  result (`kW.h/{tray}.kg{CO2e}/(kW.h)` is `g.{CO2e}.{tray}-1`). Checking a
  method's declared input and output kinds against it is planned.

# Why this crate?

A unit library answers "do these units have the same dimension, and what
is the factor?". A publication that people check needs more:

- **Exact factors.** UCUM writes its values as decimals (`0.3048`,
  `6.67430e-11`). Read as floats, a conversion depends on rounding, and two
  implementations can disagree in the last digit. pubunit reads them as
  rationals, so 1 `[ft_i]` is 0.3048 `m` exactly and 1 `[ft_i]` is 1/3
  `[yd_i]` exactly.
- **Our own base dimensions.** Trays, batches, currencies and substance
  equivalents are dimensions, not UCUM's dimensionless annotations. Adding
  a kilogram of CO2 equivalent to a kilogram of basil must fail.
- **Quantity kinds.** Some different things share a dimension: energy and
  torque, Hz and Bq. A kind names what is measured, and which units may
  measure it.

# Related crates

Each was checked at the version given. ucum-units was read in full for the
supply-chain audit (an AI review, as `supply-chain/audits.toml` says) and is
run as the tests' oracle. octofhir-ucum was run on the inputs below. uom was
checked from its documentation only.

| Crate | Version | How pubunit relates to it |
|---|---|---|
| [ucum-units] | 0.1.0 | **Built on.** It reads every expression, so pubunit accepts nothing it refuses. Its essence file is the source of pubunit's atom table. Its README reports passing UCUM's conformance cases, and pubunit's own suite runs the validation and conversion cases. Its factors are floating point and its parse tree is private, so pubunit has its own exact layer and tree. |
| [octofhir-ucum] | 0.6.1 | Made for `FHIRPath`, with a heavier dependency tree (`regex`, `phf`, `rust_decimal`). Its factors are `rust_decimal` values, but they pass through floating point: run on 2026-10-12, it gave `[ft_i]` as 0.3048000000000000153654866608 and `[ft_i]`/`[yd_i]` as 0.3333333333333333535692443976, where pubunit gives 0.3048 and 1/3. |
| [uom] | 0.38.0 | Units checked at compile time, for Rust code. Publication units arrive at run time. |

`wise_units`, which an earlier plan named as a test oracle, is not on
crates.io, and its GitHub repositories return 404. The tests compare pubunit
with ucum-units' own floating-point factors instead, for every atom and
prefix. That oracle shares pubunit's source table, the essence file, so it
checks parsing and arithmetic, not UCUM's values. The values are checked
against NIST SP 811 and the SI Brochure (`testdata/`) and against UCUM's own
functional tests.

# Crate features

None.

# Minimum Rust version

1.89, the `rust-version` in `Cargo.toml`, which ucum-units needs. It may
rise in a release that bumps the second number of the version, never in one
that bumps only the third.

# Versioning

pubunit follows Cargo's convention: below 1.0, a breaking change bumps the
second number (`0.y.z`) and anything else the third. Each change is recorded
with [pubrel] as it lands, and the changelog is generated from the package
publet `pkg.pubunit`. The public API is checked against the last release by
cargo-semver-checks, through pubrel, so a break recorded as anything less
fails CI.

[UCUM]: https://ucum.org/ucum
[QUDT]: https://www.qudt.org
[ucum-units]: https://crates.io/crates/ucum-units
[octofhir-ucum]: https://crates.io/crates/octofhir-ucum
[uom]: https://crates.io/crates/uom
[pubrel]: https://github.com/schryer/pubrel
*/

mod defs;
mod dimension;
mod error;
mod expr;
mod rational;
mod registry;
mod table;
mod ucum_table;

pub use defs::{BaseDimension, Definitions, QuantityKind, QudtSource, UnitDefinition};
pub use dimension::{Dimension, UCUM_BASE_UNITS};
pub use error::Error;
pub use expr::{MAX_DEPTH, MAX_EXPONENT, MAX_LEN, Term, parse};
pub use rational::{MAX_DECIMAL_EXPONENT, Rational};
pub use registry::{Annotations, MAX_FACTOR_BITS, Reduced, Registry, UcumSource};

/// Longer documents: testing, the supply chain, security, and the changelog.
///
/// These modules hold no code. Each is a document, kept in the repository as
/// Markdown and rendered here so it is versioned with the crate.
// Prose documents, some of them generated: clippy's backtick rule for API
// docs does not fit them, and brackets in them are text, not links. The
// first release's changelog says `[EUR]`, a currency, which rustdoc
// otherwise reads as a broken link and fails the doc build.
#[allow(clippy::doc_markdown, rustdoc::broken_intra_doc_links)]
pub mod _documentation {
    /// How every public item is tested: the examples in its documentation, and
    /// the tests that say they cover it.
    #[doc = include_str!("../TESTING.md")]
    pub mod testing {}

    /// Every crate this one depends on, and how its code is vouched for.
    #[doc = include_str!("../SUPPLY-CHAIN.md")]
    pub mod supply_chain {}

    /// How to report a vulnerability, and what is checked before each release.
    #[doc = include_str!("../SECURITY.md")]
    pub mod security {}

    /// Every release, newest first, generated from the package publet.
    #[doc = include_str!("../CHANGELOG.md")]
    pub mod changelog {}
}

/// The README's examples, run as tests without being rendered twice.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
