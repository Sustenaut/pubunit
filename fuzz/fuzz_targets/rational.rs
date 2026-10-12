//! Any text read as a number gives a Rational or an error, never a panic,
//! and a Rational prints as text that reads back as itself. The command's
//! VALUE and a definition's `value` are read this way.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pubunit::Rational;

fuzz_target!(|text: &str| {
    if let Ok(r) = text.parse::<Rational>() {
        let back: Rational = r.to_string().parse().expect("printed form reads back");
        assert_eq!(back, r, "{text:?}");
        let _ = r.round(6);
    }
});
