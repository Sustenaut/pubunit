//! The `pubunit` command: the library's checks from a shell, and the
//! surface the Gherkin suite drives.
//!
//! Arguments are parsed by hand, as in pubrel: the commands are few, and a
//! parser crate would be most of the dependency tree.

use std::process::ExitCode;

use pubunit::{Annotations, Definitions, Error, Rational, Registry, Term};
use serde_json::{Value, json};

const USAGE: &str = "\
usage: pubunit [OPTIONS] <command> [ARGS]

Exact, kind-aware checking and conversion of UCUM units.

commands:
  parse EXPR                 print the parse tree of EXPR as JSON
  validate EXPR              check that every symbol in EXPR resolves
  reduce EXPR                print the dimension and exact factor of EXPR
  convert VALUE FROM TO      convert VALUE exactly from unit FROM to unit TO
  check UNIT KIND            check that UNIT measures the declared kind KIND
  kinds                      list the declared kinds
  ucum                       print the UCUM version the atoms come from

options:
  --defs FILE                read declared dimensions, units and kinds (JSON)
  --annotations=strict|ucum  an undeclared {annotation} standing alone is an
                             error (strict, the default) or 1 (ucum)
  --kind KIND                convert: both units must measure KIND
  --places N                 convert: round to N decimal places, halves to even
  -h, --help                 print this help
  --version                  print the version

Values are decimals (0.125, 6.6743e-11) or fractions (5/9). A converted
value prints as a decimal when it has a finite one, else as a fraction.
Exit status: 0 on success, 1 on any failure.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("pubunit: {msg}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Default)]
struct Options {
    defs: Option<String>,
    annotations: Annotations,
    kind: Option<String>,
    places: Option<u32>,
    positional: Vec<String>,
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut o = Options::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match a.as_str() {
            "--defs" => o.defs = Some(value("--defs")?),
            "--kind" => o.kind = Some(value("--kind")?),
            "--places" => {
                let v = value("--places")?;
                o.places = Some(
                    v.parse()
                        .map_err(|_| format!("--places {v}: not a count"))?,
                );
            }
            "--annotations=strict" => o.annotations = Annotations::Strict,
            "--annotations=ucum" => o.annotations = Annotations::Ucum,
            s if s.starts_with("--") => return Err(format!("unknown option {s}\n\n{USAGE}")),
            s => o.positional.push(s.to_owned()),
        }
    }
    Ok(o)
}

fn registry(o: &Options) -> Result<Registry, String> {
    let reg = match &o.defs {
        None => Registry::ucum(),
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
            let defs = Definitions::from_json(&text).map_err(|e| format!("{path}: {e}"))?;
            Registry::with_definitions(&defs).map_err(|e| format!("{path}: {e}"))?
        }
    };
    Ok(reg.with_annotations(o.annotations))
}

fn run(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        None => return Err(format!("no command\n\n{USAGE}")),
        Some("-h" | "--help" | "help") => return Ok(USAGE.trim_end().to_owned()),
        Some("--version") => return Ok(format!("pubunit {}", env!("CARGO_PKG_VERSION"))),
        _ => {}
    }
    let opts = options(args)?;
    let words: Vec<&str> = opts.positional.iter().map(String::as_str).collect();
    let msg = |err: Error| err.to_string();
    match words.as_slice() {
        ["parse", expr] => {
            let t = pubunit::parse(expr).map_err(msg)?;
            serde_json::to_string_pretty(&tree(&t)).map_err(|e| e.to_string())
        }
        ["validate", expr] => {
            registry(&opts)?.validate(expr).map_err(msg)?;
            Ok(format!("{expr} is valid"))
        }
        ["reduce", expr] => {
            let r = registry(&opts)?.reduce(expr).map_err(msg)?;
            let mut out = format!("dimension: {}\nfactor: {}", r.dimension(), r.factor());
            if !r.is_linear() {
                out = format!("{out}\noffset: {}", r.offset());
            }
            Ok(out)
        }
        ["convert", value, from, to] => {
            let reg = registry(&opts)?;
            let value: Rational = value.parse().map_err(msg)?;
            let converted = match &opts.kind {
                Some(kind) => reg.convert_within(&value, from, to, kind),
                None => reg.convert(&value, from, to),
            }
            .map_err(msg)?;
            Ok(match opts.places {
                Some(places) => converted.round(places),
                None => converted.to_string(),
            })
        }
        ["check", unit, kind] => {
            registry(&opts)?.check_kind(unit, kind).map_err(msg)?;
            Ok(format!("{unit} measures {kind}"))
        }
        ["kinds"] => {
            let reg = registry(&opts)?;
            let lines: Vec<String> = reg
                .kinds()
                .map(|k| match &k.units {
                    Some(u) => format!("{}: {} [{}]", k.name, k.dimension, u.join(", ")),
                    None => format!("{}: {}", k.name, k.dimension),
                })
                .collect();
            Ok(lines.join("\n"))
        }
        ["ucum"] => {
            let s = Registry::ucum_source();
            Ok(format!(
                "UCUM {} ({}), from ucum-units {}",
                s.ucum_version, s.revision_date, s.ucum_units_version
            ))
        }
        [] => Err(format!("no command\n\n{USAGE}")),
        [
            cmd @ ("parse" | "validate" | "reduce" | "convert" | "check" | "kinds" | "ucum"),
            ..,
        ] => {
            let usage = match *cmd {
                "parse" => "parse EXPR",
                "validate" => "validate EXPR",
                "reduce" => "reduce EXPR",
                "convert" => "convert VALUE FROM TO",
                "check" => "check UNIT KIND",
                other => other,
            };
            Err(format!("usage: pubunit {usage}"))
        }
        [other, ..] => Err(format!("unknown command {other}\n\n{USAGE}")),
    }
}

/// The parse tree as JSON: one object per node, named by its kind.
fn tree(t: &Term) -> Value {
    match t {
        Term::Factor { value, annotation } => json!({ "factor": value, "annotation": annotation }),
        Term::Symbol {
            symbol,
            exponent,
            annotation,
        } => json!({ "symbol": symbol, "exponent": exponent, "annotation": annotation }),
        Term::Annotation(text) => json!({ "annotation": text }),
        Term::Mul(a, b) => json!({ "mul": [tree(a), tree(b)] }),
        Term::Div(a, b) => json!({ "div": [tree(a), tree(b)] }),
        Term::Recip(a) => json!({ "recip": tree(a) }),
        Term::Group(a) => json!({ "group": tree(a) }),
        _ => json!({ "unknown": t.to_string() }),
    }
}
