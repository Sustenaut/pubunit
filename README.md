# pubunit

Exact, kind-aware checking and conversion of [UCUM](https://ucum.org/ucum)
units, with base dimensions and quantity kinds declared as data.

A publet's data table states a unit for each column, such as `kW.h/{tray}`.
pubunit says what that unit means and whether it fits what the column
measures. It reduces the unit to a dimension and an exact factor, and
converts between units without floating point. Its own base dimensions are
declared as data: trays, batches, currencies, CO2 equivalents. So are
quantity kinds such as "climate impact per tray". It checks a unit against
its kind, not only its dimension: a kilogram of CO2e is not a kilogram of
basil, and a joule is not a newton-metre.

**Documentation:** this README covers the command and the definitions
format. The [crate documentation](https://docs.rs/pubunit) covers the
library: every type and function, with examples.
[`CHANGELOG.md`](CHANGELOG.md) lists every release.
[`SECURITY.md`](SECURITY.md) says how to report a vulnerability and what is
checked. [`SUPPLY-CHAIN.md`](SUPPLY-CHAIN.md) lists every crate pubunit
builds, and how each is vouched for.

## Usage

As a library:

```rust
use pubunit::{Definitions, Registry};

fn main() -> Result<(), pubunit::Error> {
    let reg = Registry::with_definitions(&Definitions::from_json(r#"{
      "dimensions": [
        { "symbol": "{tray}", "name": "growing tray" },
        { "symbol": "{CO2e}", "name": "carbon dioxide equivalent" }
      ],
      "kinds": [
        { "name": "electrical energy per tray",
          "dimension": { "m": 2, "s": -2, "g": 1, "{tray}": -1 },
          "units": ["W.h/{tray}", "kW.h/{tray}"] }
      ]
    }"#)?)?;

    reg.check_kind("kW.h/{tray}", "electrical energy per tray")?;
    let wh = reg.convert(&"1.1".parse()?, "kW.h/{tray}", "W.h/{tray}")?;
    assert_eq!(wh.to_string(), "1100");

    // Energy per tray is not energy, and CO2e is not mass.
    assert!(reg.convert(&"1".parse()?, "kW.h/{tray}", "kW.h").is_err());
    assert!(reg.convert(&"1".parse()?, "kg{CO2e}", "kg").is_err());
    Ok(())
}
```

From a shell:

```sh
$ pubunit reduce kW.h
dimension: m2.s-2.g
factor: 3600000000
$ pubunit convert 1 '[ft_i]' '[yd_i]'
1/3
$ pubunit convert 98.6 '[degF]' Cel
37
$ pubunit --defs examples/publication-0.json check 'kg{CO2e}/(kW.h)' 'grid carbon intensity'
kg{CO2e}/(kW.h) measures grid carbon intensity
$ pubunit --defs examples/publication-0.json check J torque
pubunit: `torque` does not allow `J`; it allows N.m
```

## What it does

1. **Parse** a UCUM expression into a public tree (`Term`), following
   UCUM's grammar. [ucum-units](https://crates.io/crates/ucum-units) also
   reads every expression, and pubunit accepts nothing it refuses.
2. **Reduce** the tree to a dimension and an exact rational factor. UCUM's
   values are read from the essence file ucum-units bundles, as the decimal
   strings UCUM writes, so `[ft_i]` is exactly 0.3048 `m`.
3. **Extend** UCUM's seven base dimensions with ones declared as data:
   `{tray}`, `{batch}`, `[EUR]`, `{CO2e}`.
4. **Check the kind**: a unit measures a declared quantity kind when it has
   the kind's dimension and is one of the units the kind lists.
5. **Convert** exactly, between commensurable units, or within one kind.

Kinds are declared, never inferred from a dimension: energy and torque share
`m2.s-2.g`, and only a declaration says which a column measures.

The parse tree (`Term`) and the dimension algebra (`Dimension` and
`Reduced`, with checked products, quotients and powers) are public, so that
symbolic computation can carry units through formulas.

## The definitions format

Definitions are JSON (or any serde format), with three optional lists. A
field the format does not define is refused.

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

- **`dimensions`**: new base dimensions, each named by the symbol of its
  unit. A symbol is one brace annotation (`{tray}`) or one bracketed atom
  UCUM does not define (`[EUR]`). A brace symbol counts wherever it
  appears. It may stand alone (`kW.h/{tray}`) or follow a unit: `kg{CO2e}`
  is a kilogram of CO2 equivalent, of dimension `g.{CO2e}`.
- **`units`**: one `symbol` is `value` times `unit`. The value is a decimal
  or a fraction, greater than zero. The unit may use UCUM, the declared
  dimensions, and the units declared before it.
- **`kinds`**: a name, a dimension (base symbol to exponent; mass is in
  grams, UCUM's base unit), and optionally the units that may measure it.
  Without `units`, any unit of the dimension may.

Each entry may carry an `id`, such as the content identifier of the publet
that defines it. pubunit keeps it and does not read it. pubunit does not
depend on publet: an adapter in publet will turn definition publets into
this format.

A brace annotation standing alone that nothing declares, such as a misspelt
`{trays}`, is an error. UCUM reads it as the number 1; ask for that with
`Annotations::Ucum` or `--annotations=ucum`. An undeclared annotation after
a unit (`g{basil}`) is a label, as in UCUM.

Declare a counted thing as a dimension, not as an area. If `{tray}` were
defined as 0.125 `m2`, then kW.h/{tray} would convert into kW.h/m2. A tray's
area is a fact about trays, stated in a publet, not a unit conversion.

[`examples/publication-0.json`](examples/publication-0.json) declares the
dimensions and kinds of publication 0.

## The command

| Command | What it does |
|---|---|
| `parse EXPR` | Prints the parse tree as JSON. |
| `validate EXPR` | Checks that every symbol resolves. Special and arbitrary units pass. |
| `reduce EXPR` | Prints `dimension: …` and `factor: …`, and `offset: …` for a temperature with an offset. |
| `convert VALUE FROM TO` | Prints the converted value. `--kind KIND` holds both units to one kind. `--places N` rounds, halves to even. |
| `check UNIT KIND` | Checks that `UNIT` measures the declared kind `KIND`. |
| `kinds` | Lists the declared kinds: `name: dimension [units]`. |
| `ucum` | Prints the UCUM version the atoms come from. |

Options: `--defs FILE` reads definitions, and `--annotations=strict|ucum`
says how an undeclared annotation standing alone is read. `--help` and
`--version` print what they say.

Values are decimals (`0.125`, `6.6743e-11`) or fractions (`5/9`). A result
prints as a decimal when it has a finite one, else as a fraction (`1/3`).

The exit status is 0 on success and 1 on any failure, with the reason on
stderr. pubunit's versioned interface is its library API, the commands,
options and exit status, the output of `reduce`, `convert` and `parse`, and
the definitions format.

## What v0.1 does not do yet

- **Special units.** Logarithmic units (`dB`, `[pH]`, `Np`) have no linear
  factor and are refused by `reduce` and `convert`. Temperatures with an
  offset (`Cel`, `[degF]`, `[degRe]`) convert when they stand alone, and are
  refused inside a product. Arbitrary units (`[IU]`) are commensurable with
  nothing.
- **Case-insensitive UCUM** (`c/i`): only the case-sensitive form is read.
- **Prefixes on declared units:** `k[EUR]` is not a unit.
- **Kinds from QUDT:** kinds are declared by hand. A starting set generated
  from QUDT, at build time, is planned.
- **Checking derivations:** pubunit gives the dimension of a formula's
  result (`kW.h/{tray}.kg{CO2e}/(kW.h)` is `g.{CO2e}.{tray}-1`). Checking a
  method's declared input and output kinds against it is planned.

## Installation

```sh
cargo add pubunit                 # the library
cargo install --locked pubunit    # the command
```

## Minimum Rust version

pubunit builds on Rust 1.89 (`rust-version` in `Cargo.toml`), which
ucum-units needs. CI builds it there. The minimum may rise in a release that
bumps the second number of the version, never in one that bumps only the
third.

## Versioning

pubunit follows Cargo's convention: below 1.0, a breaking change bumps the
second number (`0.y.z`) and anything else the third. Each change is
recorded with [pubrel](https://github.com/schryer/pubrel) as it lands, and
the public API is checked against the last release by cargo-semver-checks.
A release is a version of the package publet `pkg.pubunit` in
[`corpus/`](corpus/), and the changelog is generated from it.

## Security

Report a vulnerability privately, through the repository's Security tab
("Report a vulnerability"). [`SECURITY.md`](SECURITY.md) says how, and
describes what is checked.

## Developing

```sh
make venv          # hash-locked Python environment for the functional suite
make check         # everything CI runs
```

Testing is split as in publet and pubrel:

- **Rust unit tests and doctests** cover the arithmetic, the grammar, and
  every public function's examples.
- **An oracle test** (`tests/oracle.rs`) compares every UCUM atom, and
  every prefix on every atom, with ucum-units' own floating-point factors.
- **A Gherkin suite** (`tests/features`) drives the built `pubunit` as a
  user would. It includes UCUM's own functional tests: all 529 validation
  cases and the 30 conversion cases. They are read from the copy ucum-units
  bundles, under the Eclipse Public License 1.0, and not copied into this
  repository.
- **Fuzzing** (`fuzz/`) covers what pubunit reads from outside: expressions
  and definitions. `make fuzz-smoke` replays every input that once found a
  bug, then fuzzes each target for a minute.

`src/ucum_table.rs` is generated from ucum-units' essence file by
`make ucum-table`, and `make ucum-table-check` fails when it is stale. The
shared testing setup is [pubkit](https://github.com/schryer/pubkit)'s.

## Licence

Apache-2.0: see [`LICENSE`](LICENSE). UCUM's table is used under the UCUM
licence: see [`NOTICE`](NOTICE).
