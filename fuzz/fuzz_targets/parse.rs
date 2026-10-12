//! Any text parses to a term or an error, never a panic; and a term's
//! normal form parses back to the same normal form.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    if let Ok(term) = pubunit::parse(text) {
        let normal = term.to_string();
        let again = pubunit::parse(&normal).map(|t| t.to_string());
        assert_eq!(again.as_deref(), Ok(normal.as_str()), "{text:?}");
    }
});
