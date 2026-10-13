//! The registry's public API: definitions, kinds and conversion, including
//! the ways each refuses its input.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use pubunit::{Annotations, Definitions, Dimension, Error, Rational, Reduced, Registry};

fn publication_0() -> Registry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/publication-0.json");
    let defs = Definitions::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    Registry::with_definitions(&defs).unwrap()
}

fn r(text: &str) -> Rational {
    text.parse().unwrap()
}

// covers: Definitions::from_json, Registry::with_definitions, Registry::declared_bases, Registry::kinds, Registry::kind
#[test]
fn the_example_definitions_load() {
    let reg = publication_0();
    assert_eq!(
        reg.declared_bases().collect::<Vec<_>>(),
        ["[EUR]", "{CO2e}", "{batch}", "{tray}"]
    );
    assert_eq!(reg.kinds().count(), 10);
    let kind = reg.kind("climate impact").unwrap();
    assert_eq!(kind.dimension.to_string(), "g.{CO2e}");
    assert!(reg.kind("mass of basil").is_none());
}

// covers: Registry::with_definitions
#[test]
fn bad_definitions_are_refused_with_their_name() {
    let cases = [
        (
            r#"{"dimensions": [{"symbol": "[ft_i]", "name": "foot"}]}"#,
            "[ft_i]",
        ),
        (
            r#"{"dimensions": [{"symbol": "tray", "name": "tray"}]}"#,
            "tray",
        ),
        (
            r#"{"units": [{"symbol": "{x}", "name": "x", "value": "-1", "unit": "m"}]}"#,
            "{x}",
        ),
        (
            r#"{"units": [{"symbol": "{x}", "name": "x", "value": "1", "unit": "dB"}]}"#,
            "{x}",
        ),
        (
            r#"{"kinds": [{"name": "length", "dimension": {"m": 1}, "units": ["s"]}]}"#,
            "length",
        ),
        (
            r#"{"kinds": [{"name": "a", "dimension": {"m": 1}}, {"name": "a", "dimension": {"s": 1}}]}"#,
            "a",
        ),
    ];
    for (json, name) in cases {
        let defs = Definitions::from_json(json).unwrap();
        match Registry::with_definitions(&defs) {
            Err(Error::Definition { name: n, .. }) => assert_eq!(n, name, "{json}"),
            other => panic!("{json}: {other:?}"),
        }
    }
    assert!(matches!(
        Definitions::from_json(r#"{"kinds": [{"name": "a", "dimension": {}, "unit": []}]}"#),
        Err(Error::Json { .. })
    ));
}

// covers: Registry::check_kind, Registry::convert_within
#[test]
fn kinds_tell_apart_what_dimensions_cannot() {
    let reg = publication_0();
    assert!(reg.check_kind("kW.h", "energy").is_ok());
    assert!(matches!(
        reg.check_kind("kW.h", "torque"),
        Err(Error::NotAllowed { .. })
    ));
    assert!(matches!(
        reg.check_kind("kg", "climate impact"),
        Err(Error::WrongDimension { .. })
    ));
    assert!(matches!(
        reg.check_kind("kg", "no such kind"),
        Err(Error::UnknownKind { .. })
    ));
    // The normal form is compared, not the spelling.
    assert!(
        reg.check_kind("((kW).h)/{tray}", "electrical energy per tray")
            .is_ok()
    );
    assert_eq!(
        reg.convert_within(
            &r("420"),
            "g{CO2e}/{tray}",
            "kg{CO2e}/{tray}",
            "climate impact per tray"
        ),
        Ok(r("0.42"))
    );
    assert!(reg.convert_within(&r("1"), "J", "N.m", "energy").is_err());
}

// covers: Registry::convert, Registry::reduce
#[test]
fn declared_dimensions_keep_quantities_apart() {
    let reg = publication_0();
    for (from, to) in [
        ("kW.h/{tray}", "kW.h"),
        ("kg{CO2e}", "kg"),
        ("[EUR]", "1"),
        ("{tray}", "{batch}"),
    ] {
        assert!(
            matches!(
                reg.convert(&r("1"), from, to),
                Err(Error::Incommensurable { .. })
            ),
            "{from} {to}"
        );
    }
    assert_eq!(reg.convert(&r("1500"), "g{basil}", "kg"), Ok(r("1.5")));
    let chain = reg.reduce("kW.h/{tray}.kg{CO2e}/(kW.h)").unwrap();
    assert_eq!(chain.dimension().to_string(), "g.{CO2e}.{tray}-1");
}

// covers: Registry::validate, Registry::with_annotations
#[test]
fn undeclared_annotations_follow_the_policy() {
    let strict = Registry::ucum();
    let ucum = Registry::ucum().with_annotations(Annotations::Ucum);
    assert!(matches!(
        strict.validate("{rbc}/uL"),
        Err(Error::UndeclaredAnnotation { .. })
    ));
    assert!(ucum.validate("{rbc}/uL").is_ok());
    // After a unit, an undeclared annotation is a label under either policy.
    assert!(strict.validate("mg{dry}").is_ok());
    // A syntax error is pubunit's own, found before ucum-units reads it.
    assert!(matches!(strict.validate("m.(s"), Err(Error::Syntax { .. })));
}

// covers: Reduced::new, Reduced::to_base, Reduced::from_base, Reduced::checked_pow
#[test]
fn reduced_units_carry_values_both_ways() {
    let km = Reduced::new(Dimension::base("m"), r("1000"));
    assert_eq!(km.to_base(&r("1.5")), r("1500"));
    assert_eq!(km.from_base(&r("1500")), Some(r("1.5")));
    assert_eq!(
        km.checked_pow(2).map(|a| a.factor().clone()),
        Some(r("1000000"))
    );
    let zero = Reduced::new(Dimension::base("m"), Rational::zero());
    assert_eq!(zero.from_base(&r("1")), None);
    let cel = Registry::ucum().reduce("Cel").unwrap();
    assert_eq!(cel.checked_pow(2), None);
    assert_eq!(cel.from_base(&cel.to_base(&r("21.5"))), Some(r("21.5")));
}

// covers: Registry::ucum_source, Registry::ucum_atoms, Registry::ucum
#[test]
fn the_ucum_source_is_stated_and_complete() {
    let s = Registry::ucum_source();
    assert_eq!((s.ucum_version, s.ucum_units_version), ("2.2", "0.1.0"));
    let reg = Registry::ucum();
    let atoms: Vec<_> = Registry::ucum_atoms().collect();
    assert!(atoms.len() > 300);
    // Every atom resolves; all but arbitrary and special ones reduce.
    for atom in atoms {
        assert!(
            Registry::ucum()
                .with_annotations(Annotations::Ucum)
                .validate(atom)
                .is_ok(),
            "{atom}"
        );
        match reg.reduce(atom) {
            Ok(_) | Err(Error::Arbitrary { .. } | Error::NotLinear { .. }) => {}
            Err(e) => panic!("{atom}: {e}"),
        }
    }
}

// covers: Definitions::qudt, Definitions::qudt_source, Definitions::extend
#[test]
fn the_qudt_kinds_load_beside_declared_ones() {
    let source = Definitions::qudt_source().unwrap();
    assert_eq!(source.version, "3.5.2");
    assert_eq!(source.sha256.len(), 64);
    let qudt = Definitions::qudt().unwrap();
    assert!(qudt.kinds.len() > 800);
    for k in &qudt.kinds {
        let local = k.name.strip_prefix("qudt:").unwrap();
        assert_eq!(
            k.id.as_deref(),
            Some(format!("http://qudt.org/vocab/quantitykind/{local}").as_str())
        );
        assert!(!k.units.as_ref().unwrap().is_empty(), "{}", k.name);
    }

    let mut defs = Definitions::from_json(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/publication-0.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let own = defs.kinds.len();
    defs.extend(qudt);
    let reg = Registry::with_definitions(&defs).unwrap();
    assert!(reg.kinds().count() > own + 800);
    assert_eq!(
        reg.kind("qudt:Energy").unwrap().label.as_deref(),
        Some("Energy")
    );
    reg.check_kind("kW.h", "qudt:Energy").unwrap();
    reg.check_kind("kW.h/{tray}", "electrical energy per tray")
        .unwrap();
    assert!(reg.check_kind("N.m", "qudt:Energy").is_err());
    assert!(reg.check_kind("Hz", "qudt:Activity").is_err());
    // QUDT calls plane angle dimensionless; UCUM, and so pubunit, does not.
    assert_eq!(
        reg.kind("qudt:AngularVelocity")
            .unwrap()
            .dimension
            .to_string(),
        "s-1.rad"
    );
}
