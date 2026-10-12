//! Any text read as definitions gives a registry or an error, never a
//! panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pubunit::{Definitions, Registry};

fuzz_target!(|text: &str| {
    if let Ok(defs) = Definitions::from_json(text) {
        if let Ok(reg) = Registry::with_definitions(&defs) {
            for kind in reg.kinds() {
                for unit in kind.units.iter().flatten() {
                    let _ = reg.check_kind(unit, &kind.name);
                }
            }
        }
    }
});
