//! pubunit against ucum-units, an independent implementation of the same
//! table: every atom, and every prefix on every atom, must be accepted by
//! both or neither, and reduce to the same dimension and factor.
//!
//! ucum-units computes factors in `f64`, so they are compared within a
//! relative tolerance. That is the only floating point in this repository,
//! and it is here, in a test, as the oracle's own number type.
#![allow(
    clippy::float_arithmetic,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use pubunit::{Annotations, Error, Rational, Registry, UCUM_BASE_UNITS};

const PREFIXES: [&str; 24] = [
    "Y", "Z", "E", "P", "T", "G", "M", "k", "h", "da", "d", "c", "m", "u", "n", "p", "f", "a", "z",
    "y", "Ki", "Mi", "Gi", "Ti",
];

fn to_f64(r: &Rational) -> f64 {
    let s = r.to_string();
    match s.split_once('/') {
        Some((n, d)) => n.parse::<f64>().unwrap_or(f64::NAN) / d.parse::<f64>().unwrap_or(f64::NAN),
        None => s.parse().unwrap_or(f64::NAN),
    }
}

/// Compare one expression; return a description of any disagreement.
fn compare(reg: &Registry, expr: &str) -> Option<String> {
    let theirs = ucum::analyze(expr);
    let ours = reg.reduce(expr);
    match (theirs, ours) {
        (Ok(a), Ok(r)) => {
            let dims: Vec<i32> = UCUM_BASE_UNITS
                .iter()
                .map(|b| r.dimension().exponent(b))
                .collect();
            let theirs: Vec<i32> = a.dimension.0.iter().map(|&e| i32::from(e)).collect();
            if dims != theirs {
                return Some(format!("{expr}: dimension {dims:?} vs {theirs:?}"));
            }
            if a.is_special {
                // Only temperatures with an offset reduce in pubunit; the
                // factor ucum-units reports is that of the proper unit.
                if r.is_linear() {
                    return Some(format!("{expr}: special in ucum-units, linear here"));
                }
                return None;
            }
            let f = to_f64(r.factor());
            let rel = ((f - a.factor) / a.factor).abs();
            if rel > 1e-12 {
                return Some(format!(
                    "{expr}: factor {} ({f}) vs {}",
                    r.factor(),
                    a.factor
                ));
            }
            None
        }
        // Refusals pubunit makes on purpose: no linear factor.
        // Both refuse, or pubunit refuses an arbitrary unit, which has no
        // factor, or a special one, which has no linear factor.
        (Ok(a), Err(Error::NotLinear { .. })) if a.is_special => None,
        (_, Err(Error::Arbitrary { .. })) | (Err(_), Err(_)) => None,
        (Ok(_), Err(e)) => Some(format!("{expr}: refused here: {e}")),
        (Err(e), Ok(_)) => Some(format!("{expr}: refused by ucum-units: {e}")),
    }
}

// covers: Registry::reduce, Registry::ucum_atoms
#[test]
fn every_atom_agrees_with_ucum_units() {
    let reg = Registry::ucum().with_annotations(Annotations::Ucum);
    let bad: Vec<String> = Registry::ucum_atoms()
        .filter_map(|a| compare(&reg, a))
        .collect();
    // The comparison is not vacuous: most atoms reduce in both.
    let both = Registry::ucum_atoms()
        .filter(|a| reg.reduce(a).is_ok() && ucum::analyze(a).is_ok())
        .count();
    assert!(both > 250, "only {both} atoms reduce in both");
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

// covers: Registry::reduce, Registry::ucum_atoms
#[test]
fn every_prefixed_atom_agrees_with_ucum_units() {
    let reg = Registry::ucum().with_annotations(Annotations::Ucum);
    let mut checked = 0;
    let mut bad = Vec::new();
    for atom in Registry::ucum_atoms() {
        for p in PREFIXES {
            let expr = format!("{p}{atom}");
            checked += 1;
            bad.extend(compare(&reg, &expr));
        }
    }
    assert!(checked > 7000);
    let both = Registry::ucum_atoms()
        .flat_map(|a| PREFIXES.map(|p| format!("{p}{a}")))
        .filter(|e| reg.reduce(e).is_ok() && ucum::analyze(e).is_ok())
        .count();
    assert!(both > 2000, "only {both} prefixed atoms reduce in both");
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

// covers: Registry::reduce, Registry::ucum_atoms
#[test]
fn compound_expressions_agree_with_ucum_units() {
    let reg = Registry::ucum().with_annotations(Annotations::Ucum);
    let exprs = [
        "kg.m/s2",
        "kW.h",
        "mg/dL",
        "kg/(m.s)",
        "kg/m.s",
        "/min",
        "10*3/uL",
        "m3.kg-1.s-2",
        "[ft_i].[lbf_av]/s",
        "4.[pi].10*-7.N/A2",
        "mm[Hg]",
        "cal/(g.K)",
        "mol/L",
        "[in_i]2",
        "km/h",
        "[mi_i]/h",
        "N.m",
        "J",
        "Hz",
        "Bq",
        "%",
        "[ppm]",
        "g{dry}/m2",
        "/100{cells}",
        "[gal_us]/min",
        "/m.s",
        "(/s).m",
        "/(/s)",
        "m/(/s)",
        "(/s)/m",
        "eV",
        "u",
        "[ly]",
        "pc",
        "atm",
        "bar",
        "[psi]",
        "kat/L",
        "lm/m2",
        "lx",
    ];
    let bad: Vec<String> = exprs.iter().filter_map(|e| compare(&reg, e)).collect();
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
