# Security

## Reporting a vulnerability

Report it privately through GitHub:
**[report a vulnerability](https://github.com/schryer/pubunit/security/advisories/new)**
(the repository's Security tab, "Report a vulnerability"). Please don't open
a public issue for it.

Say which version is affected, the expression or definitions that trigger
it, and what an attacker could do. You'll get a reply in the advisory.

## Supported versions

Releases are cut only from `main`, with pubrel. A fix is released as the
next version of pubunit. Earlier versions get no separate fixes.

## What pubunit protects against

pubunit reads unit expressions and definitions that come from publets,
which anyone can write. Reading any text must give a result or an error:
never a panic, a hang, or an allocation out of proportion to the input.

- An expression is at most 1024 bytes (`MAX_LEN`), 32 levels of
  parentheses deep (`MAX_DEPTH`), with exponents within ±64
  (`MAX_EXPONENT`). A dimension's exponents on UCUM's base units stay within
  ±127, as in ucum-units.
- A decimal's exponent is within ±1000 (`MAX_DECIMAL_EXPONENT`), so `1e999999`
  cannot ask for a huge number.
- Every result is computed exactly, with rationals, so it cannot depend on
  floating-point rounding. No floating-point type appears in `src/`.
- Declarations cannot redefine a UCUM atom or one another, and units are
  defined only in terms of those declared before them, so there are no
  cycles.

## What is checked in pubunit itself

**On every pull request and push to `main`, CI runs:**

- `cargo fmt --check`;
- `cargo clippy` with the `all` and `pedantic` groups, `unwrap_used`,
  `expect_used`, `panic`, `indexing_slicing` and the float lints, and
  warnings as errors;
- a guard that no `f32` or `f64` appears in `src/`;
- a check that `src/ucum_table.rs` matches the essence file of the pinned
  ucum-units;
- `cargo doc`, with warnings as errors, and `cargo publish --dry-run`;
- the unit tests, doctests and the oracle test against ucum-units, on Linux,
  macOS and Windows, and on a 32-bit (i686) and a big-endian (s390x) target
  under emulation;
- a build on the minimum Rust version;
- the Gherkin suite, which runs the built binary, with UCUM's own functional
  tests;
- fuzzing of the parser, reduction and conversion, and the definitions
  reader (`fuzz/`), after replaying every input that once found a bug
  (`fuzz/regressions/`);
- `cargo deny check`: fails on any crate in the dependency tree with a
  RustSec advisory (vulnerable, unmaintained, unsound, or yanked), on a
  licence outside the permissive list, and on any source but crates.io
  (`deny.toml`);
- `cargo vet`: every dependency is covered by an audit imported from
  Mozilla, Google, the Bytecode Alliance, Zcash, ISRG or Embark Studios, by
  a publisher those organisations trust, or by an exemption that records
  the evidence that does exist (`supply-chain/`);
- on a pull request, `pubrel check`, with cargo-semver-checks once a
  release exists to compare against.

**In the package:** [`SUPPLY-CHAIN.md`](SUPPLY-CHAIN.md) lists every crate
pubunit builds, for any target, whether it runs code at build time, and how
it is vouched for. It is generated, and CI fails if it is stale.

**ucum-units** is pinned to exactly 0.1.0. It is its only release, and no
imported audit covers it, so its exemption in `supply-chain/config.toml`
records what was read: its source forbids `unsafe`, and it has one runtime
dependency. A new version is adopted only by changing the pin, reviewed
like any other change, and `make ucum-table-check` then shows any change to
UCUM's table.

**In every release:** `pubrel prepare` runs `cargo deny`, `cargo vet`, the
supply-chain report's check and the UCUM table's check before cutting a
release, and records them as a signed `security` claim in pubunit's package
publet.

**Fuzzing found** three bugs before pubunit was published, all in printing
a parsed expression back as text. `1+3+3` and `1-+3` read as symbols ending
in a digit or a sign, which printed as other expressions; such symbols are
now refused, as no UCUM atom has one. A reciprocal lost its parentheses:
`(/s).m` printed as `/s.m`, which means 1/(s.m). Each has a regression test.

**In the code:** `unsafe` is forbidden (`unsafe_code = "forbid"` in
`Cargo.toml`). Release builds keep integer overflow checks
(`overflow-checks = true`).
