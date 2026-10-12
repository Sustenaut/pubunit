# Fuzzing regressions

Inputs that once made a target fail. `make fuzz-smoke` replays them before
fuzzing anything new, so a bug they found cannot quietly return.

| Target | Input | What it found |
|---|---|---|
| `parse` | `symbol-ending-in-digit` | `1+3+3` parsed as the symbol `1+3` with exponent 3, which printed as `1+33` and read back as another term. A symbol ending in a digit is now refused: no UCUM atom ends in one outside brackets. |
| `parse` | `symbol-ending-in-sign` | `1-+3` parsed as the symbol `1-` with exponent 3, which printed as `1-3`. A symbol ending in `+` or `-` is now refused too. |
| `parse` | `nested-reciprocal` | `/(/@/s)` printed as `//@/s`, which does not parse. The same omission printed `(/s).m` as `/s.m`, which means 1/(s.m): a reciprocal now keeps its parentheses anywhere but first. ucum-units reads both as pubunit does. |

| `reduce` | `pi-to-the-64th-many-times` | Found by timing, not fuzzing: `[pi]64` written 120 times grew factors to half a million digits and took minutes. Factors are now bounded by `MAX_FACTOR_BITS`. Kept here so every fuzzing run replays it. |

Each also has a regression test in pubunit's own tests:
`a_symbol_ending_in_a_digit_or_sign_is_refused` and
`a_reciprocal_keeps_its_parentheses` in `src/expr.rs`, and
`a_huge_factor_is_refused_quickly` in `src/registry.rs`.
