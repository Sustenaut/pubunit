//! The test vectors in `testdata/`, run against this crate.
//!
//! Each file's expected values come from somewhere other than this crate
//! (see its `source` field, and `tools/vectors/generate.py`): exact unit
//! definitions from NIST SP 811 and the SI Brochure, computed with exact
//! fractions. So passing them shows agreement with those definitions, not
//! only with UCUM's table as pubunit reads it. They are plain JSON, so that
//! another implementation can run the same cases.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use pubunit::{Dimension, Rational, Registry};
use serde_json::Value;

fn load(name: &str) -> Value {
    let path = format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn text<'a>(case: &'a Value, key: &str) -> &'a str {
    case[key].as_str().unwrap()
}

// covers: Registry::convert
#[test]
fn exact_definitions_convert_exactly() {
    let file = load("conversions.json");
    let reg = Registry::ucum();
    let cases = file["cases"].as_array().unwrap();
    assert!(cases.len() > 25);
    for case in cases {
        let value: Rational = text(case, "value").parse().unwrap();
        let want: Rational = text(case, "result").parse().unwrap();
        let got = reg.convert(&value, text(case, "from"), text(case, "to"));
        assert_eq!(got.as_ref(), Ok(&want), "{}", text(case, "name"));
    }
}

// covers: Registry::reduce
#[test]
fn si_derived_units_reduce_to_their_definitions() {
    let file = load("si-derived.json");
    let reg = Registry::ucum();
    for case in file["cases"].as_array().unwrap() {
        let unit = text(case, "unit");
        let dimension: BTreeMap<String, i32> =
            serde_json::from_value(case["dimension"].clone()).unwrap();
        let want_factor: Rational = text(case, "factor").parse().unwrap();
        let r = reg.reduce(unit).unwrap();
        assert_eq!(r.dimension(), &Dimension::from(dimension), "{unit}");
        assert_eq!(r.factor(), &want_factor, "{unit}");
    }
}
