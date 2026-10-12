//! Any text reduces, validates and converts to a result or an error, never
//! a panic, against UCUM and against declared dimensions. Two halves of the
//! input are taken as the units to convert between.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use pubunit::{Definitions, Rational, Registry};

fn registries() -> &'static [Registry] {
    static R: OnceLock<Vec<Registry>> = OnceLock::new();
    R.get_or_init(|| {
        let defs = Definitions::from_json(
            r#"{"dimensions": [{"symbol": "{tray}", "name": "tray"},
                               {"symbol": "[EUR]", "name": "euro"},
                               {"symbol": "{CO2e}", "name": "CO2e"}],
                "units": [{"symbol": "{flat}", "name": "flat", "value": "6", "unit": "{tray}"}]}"#,
        )
        .unwrap_or_default();
        vec![
            Registry::ucum(),
            Registry::ucum().with_annotations(pubunit::Annotations::Ucum),
            Registry::with_definitions(&defs).unwrap_or_else(|_| Registry::ucum()),
        ]
    })
}

fuzz_target!(|text: &str| {
    let mut mid = text.len() / 2;
    while !text.is_char_boundary(mid) {
        mid -= 1;
    }
    let (a, b) = text.split_at(mid);
    for reg in registries() {
        let _ = reg.validate(text);
        if let Ok(r) = reg.reduce(text) {
            let _ = r.to_base(&Rational::one());
        }
        let _ = reg.convert(&Rational::one(), a, b);
    }
});
