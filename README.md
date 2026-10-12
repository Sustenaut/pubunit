# pubunit

Exact, kind-aware checking and conversion of [UCUM](https://ucum.org/ucum)
units, with base dimensions and quantity kinds declared as data.

A publet's data table states a unit for each column, such as `kW.h/{tray}`.
pubunit says what that unit means, and whether it fits what the column
measures. Trays, currencies and CO2 equivalents are dimensions of their own,
so a kilogram of CO2e is not a kilogram of basil. Quantity kinds are
declared, so a joule is not a newton-metre. Every factor and conversion is
an exact rational, never a float.

Licensed under Apache-2.0.

### Documentation

[docs.rs/pubunit](https://docs.rs/pubunit): the overview, the definitions
format, a cookbook of examples, what this version does not do, why this crate
exists, the crates considered instead, how every item is tested, and how each
dependency is vouched for. The `pubunit` command is described below and by
`pubunit --help`.

### Usage

```toml
[dependencies]
pubunit = "0.1"
```

### Example

```rust
use pubunit::{Definitions, Registry};

fn main() -> Result<(), pubunit::Error> {
    let reg = Registry::with_definitions(&Definitions::from_json(r#"{
      "dimensions": [{ "symbol": "{tray}", "name": "growing tray" }],
      "kinds": [{ "name": "electrical energy per tray",
                  "dimension": { "m": 2, "s": -2, "g": 1, "{tray}": -1 },
                  "units": ["W.h/{tray}", "kW.h/{tray}"] }]
    }"#)?)?;

    reg.check_kind("kW.h/{tray}", "electrical energy per tray")?;
    let wh = reg.convert(&"1.1".parse()?, "kW.h/{tray}", "W.h/{tray}")?;
    assert_eq!(wh.to_string(), "1100");

    // Energy per tray is not energy.
    assert!(reg.convert(&"1".parse()?, "kW.h/{tray}", "kW.h").is_err());
    Ok(())
}
```

### The command

```sh
cargo install --locked pubunit
```

| Command | What it does |
|---|---|
| `parse EXPR` | Prints the parse tree as JSON. |
| `validate EXPR` | Checks that every symbol resolves. |
| `reduce EXPR` | Prints `dimension: …` and `factor: …`, and `offset: …` for a temperature with an offset. |
| `convert VALUE FROM TO` | Prints the converted value: a decimal when it has a finite one, else a fraction. `--kind KIND` holds both units to one kind; `--places N` rounds, halves to even, to at most 1000 places. |
| `check UNIT KIND` | Checks that `UNIT` measures the declared kind `KIND`. |
| `kinds` | Lists the declared kinds: `name: dimension [units]`. |
| `ucum` | Prints the UCUM version the atoms come from. |

`--defs FILE` reads definitions (the format is in the crate documentation),
and `--annotations=strict|ucum` says how an undeclared `{annotation}` standing
alone is read. The exit status is 0 on success and 1 on any failure, with the
reason on stderr. The commands, options, exit status, the output of
`reduce`, `convert` and `parse`, and the definitions format are versioned
like the library API.

```sh
$ pubunit convert 1 '[ft_i]' '[yd_i]'
1/3
$ pubunit --defs examples/publication-0.json check J torque
pubunit: `torque` does not allow `J`; it allows N.m
```

### Minimum Rust version

Rust 1.89, which ucum-units needs, tested in CI. The minimum may rise in a
release that bumps the minor version, and never in a patch release.

### Security

Report a vulnerability privately through the repository's Security tab.
[`SECURITY.md`](SECURITY.md) describes what is checked, and
[`SUPPLY-CHAIN.md`](SUPPLY-CHAIN.md) how each dependency is vouched for.
UCUM's table is used under the UCUM licence: see [`NOTICE`](NOTICE).
