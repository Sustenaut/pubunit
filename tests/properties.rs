//! Invariants of the arithmetic and the grammar, over many random inputs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use proptest::prelude::*;
use pubunit::{Dimension, Rational, Reduced, Registry};

/// Units with exact factors, of several dimensions.
const UNITS: [(&str, &str); 12] = [
    ("m", "length"),
    ("[in_i]", "length"),
    ("[mi_i]", "length"),
    ("km", "length"),
    ("kW.h", "energy"),
    ("J", "energy"),
    ("cal", "energy"),
    ("[lb_av]", "mass"),
    ("kg", "mass"),
    ("t", "mass"),
    ("Cel", "temperature"),
    ("[degF]", "temperature"),
];

fn rational() -> impl Strategy<Value = Rational> {
    (any::<i32>(), 1..=i32::MAX).prop_map(|(n, d)| Rational::new(n.into(), d.into()).unwrap())
}

fn dimension() -> impl Strategy<Value = Dimension> {
    proptest::collection::btree_map(
        prop_oneof![
            Just("m"),
            Just("s"),
            Just("g"),
            Just("K"),
            Just("{tray}"),
            Just("[EUR]")
        ]
        .prop_map(str::to_owned),
        -6..=6i32,
        0..5,
    )
    .prop_map(Dimension::from)
}

proptest! {
    // covers: Rational::checked_div, Rational::recip
    #[test]
    fn division_undoes_multiplication(a in rational(), b in rational()) {
        prop_assume!(!b.is_zero());
        prop_assert_eq!((&a * &b).checked_div(&b), Some(a.clone()));
        prop_assert_eq!(b.recip().map(|r| &b * &r), Some(Rational::one()));
    }

    // covers: Rational::to_decimal
    #[test]
    fn a_decimal_reads_back_as_the_same_number(a in rational()) {
        if let Some(d) = a.to_decimal() {
            prop_assert_eq!(d.parse::<Rational>().unwrap(), a.clone());
        }
        prop_assert_eq!(a.to_string().parse::<Rational>().unwrap(), a);
    }

    // covers: Rational::round
    #[test]
    fn rounding_is_within_half_a_place(a in rational(), places in 0i32..8) {
        let rounded: Rational = a.round(places.unsigned_abs()).parse().unwrap();
        let half = Rational::new(1, 2).unwrap()
            .checked_div(&Rational::from_integer(10).checked_pow(places).unwrap()).unwrap();
        let diff = &rounded - &a;
        let diff = if diff.is_negative() { -&diff } else { diff };
        prop_assert!(diff <= half);
    }

    // covers: Dimension::checked_mul, Dimension::checked_div, Dimension::checked_pow
    #[test]
    fn dimension_algebra_is_a_group(a in dimension(), b in dimension(), n in -4..=4i32) {
        let ab = a.checked_mul(&b).unwrap();
        prop_assert_eq!(ab.checked_div(&b).unwrap(), a.clone());
        prop_assert_eq!(ab, b.checked_mul(&a).unwrap());
        prop_assert!(a.checked_div(&a).unwrap().is_dimensionless());
        let squared = a.checked_mul(&a).unwrap();
        prop_assert_eq!(a.checked_pow(2).unwrap(), squared);
        prop_assert_eq!(a.checked_pow(n).unwrap().checked_pow(-1).unwrap(), a.checked_pow(-n).unwrap());
    }

    // covers: Dimension::to_ucum
    #[test]
    fn a_dimension_writes_a_unit_of_itself(d in dimension()) {
        // to_ucum is a UCUM expression that reduces back to the same
        // dimension, with factor 1.
        let reg = Registry::with_definitions(&pubunit::Definitions::from_json(
            r#"{"dimensions": [{"symbol": "{tray}", "name": "tray"}, {"symbol": "[EUR]", "name": "euro"}]}"#,
        ).unwrap()).unwrap();
        let r = reg.reduce(&d.to_ucum()).unwrap();
        prop_assert_eq!(r.dimension(), &d);
        prop_assert_eq!(r.factor(), &Rational::one());
    }

    // covers: Registry::convert
    #[test]
    fn converting_there_and_back_gives_the_value(v in rational(), i in 0..UNITS.len(), j in 0..UNITS.len()) {
        let reg = Registry::ucum();
        let (a, ka) = UNITS[i];
        let (b, kb) = UNITS[j];
        let there = reg.convert(&v, a, b);
        if ka == kb {
            let back = reg.convert(&there.unwrap(), b, a).unwrap();
            prop_assert_eq!(back, v);
        } else {
            prop_assert!(there.is_err());
        }
    }

    // covers: Registry::convert
    #[test]
    fn conversion_composes(v in rational(), i in 0..3usize, j in 0..3usize, k in 0..3usize) {
        // Through any intermediate unit, the result is the same, exactly.
        let reg = Registry::ucum();
        let lengths = ["[in_i]", "[ft_i]", "km"];
        let direct = reg.convert(&v, lengths[i], lengths[k]).unwrap();
        let via = reg.convert(&reg.convert(&v, lengths[i], lengths[j]).unwrap(), lengths[j], lengths[k]).unwrap();
        prop_assert_eq!(direct, via);
    }

    // covers: Reduced::checked_mul, Reduced::checked_div
    #[test]
    fn reduction_is_a_homomorphism(i in 0..10usize, j in 0..10usize) {
        // Reducing a product is the product of the reductions.
        let reg = Registry::ucum();
        let (a, _) = UNITS[i];
        let (b, _) = UNITS[j];
        let ra = reg.reduce(a).unwrap();
        let rb = reg.reduce(b).unwrap();
        prop_assert_eq!(reg.reduce(&format!("({a}).({b})")).unwrap(), ra.checked_mul(&rb).unwrap());
        prop_assert_eq!(reg.reduce(&format!("({a})/({b})")).unwrap(), ra.checked_div(&rb).unwrap());
        let one: Reduced = ra.checked_div(&ra).unwrap();
        prop_assert!(one.dimension().is_dimensionless());
    }

    // covers: parse
    #[test]
    fn a_term_reads_back_as_itself(text in "[a-zA-Z\\[\\]{}()./0-9+*_-]{1,24}") {
        if let Ok(t) = pubunit::parse(&text) {
            let normal = t.to_string();
            prop_assert_eq!(pubunit::parse(&normal).map(|t| t.to_string()).ok(), Some(normal));
        }
    }
}
