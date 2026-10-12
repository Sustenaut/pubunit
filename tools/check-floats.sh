#!/usr/bin/env bash
# pubunit promises exact results (publet spec 4.1, R8): every factor and
# converted value is a rational, never a float. Clippy's
# float_arithmetic lint (see Cargo.toml) catches float *arithmetic*; this
# catches the *types*, including in struct fields and signatures where no
# arithmetic appears yet -- a `fn f(x: f64) -> f64 { x }` passes clippy and
# fails here.
#
# Comment lines are excluded. The documentation explaining why floats are
# forbidden necessarily names them, and a guard that fires on its own
# rationale is a guard people learn to bypass.
set -euo pipefail
cd "$(dirname "$0")/.."

hits=$(
  grep -rnE '\b(f32|f64)\b' src --include='*.rs' 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*)' \
    || true
)
if [ -n "$hits" ]; then
  echo "error: floating point is forbidden in src/" >&2
  echo "$hits" >&2
  exit 1
fi
echo "guard-floats: clean"
