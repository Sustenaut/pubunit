"""Steps for every pubunit feature: pubunit runs through pubtest's `I run`,
as its program under test, and pubtest's steps judge what it did. What is
the suite's own is UCUM's functional tests, run case by case, and one
check of the JSON tree `parse` prints."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from decimal import Decimal
from pathlib import Path

from pytest_bdd import parsers, scenarios, then, when

scenarios("../features/reduce.feature")
scenarios("../features/convert.feature")
scenarios("../features/dimensions.feature")
scenarios("../features/kinds.feature")
scenarios("../features/definitions.feature")
scenarios("../features/parse.feature")
scenarios("../features/cli.feature")
scenarios("../features/conformance.feature")
scenarios("../features/qudt.feature")


# --- whens -------------------------------------------------------------------
# Each runs every case of UCUM's functional tests, which this repository may
# not copy (they are under the Eclipse Public License 1.0), so they cannot be
# a Scenario Outline's examples. The result is the cases pubunit disagrees on.


def validation_disagreements(run, pubunit: Path, tests: Path) -> list[str]:
    root = ET.parse(tests).getroot()
    cases = [c for c in root.find("validation") if c.tag == "case"]
    assert len(cases) > 500, f"only {len(cases)} validation cases"
    bad = []
    for c in cases:
        result = run(pubunit, "--annotations=ucum", "validate", c.get("unit"))
        if (result.code == 0) != (c.get("valid") == "true"):
            bad.append(f"{c.get('id')} {c.get('unit')!r}: valid={c.get('valid')}, "
                       f"pubunit: {result.stderr.strip() or 'valid'}")
    return bad


@when("I run every validation case")
def every_validation_case(ctx, run, binary, ucum_functional_tests: Path):
    ctx.attempt(lambda: validation_disagreements(run, binary("pubunit"), ucum_functional_tests))


def _decimal(text: str) -> Decimal:
    if "/" in text:
        num, den = text.split("/")
        return Decimal(num) / Decimal(den)
    return Decimal(text)


def conversion_disagreements(run, pubunit: Path, tests: Path) -> list[str]:
    root = ET.parse(tests).getroot()
    cases = [c for c in root.find("conversion") if c.tag == "case"]
    assert len(cases) > 20, f"only {len(cases)} conversion cases"
    bad = []
    for c in cases:
        result = run(pubunit, "--annotations=ucum", "convert",
                     c.get("value"), c.get("srcUnit"), c.get("dstUnit"))
        if result.code != 0:
            bad.append(f"{c.get('id')}: {result.stderr.strip()}")
            continue
        # The outcomes are rounded to the precision they are written at:
        # 6.3 [in_i] is "16.0" cm. Agree within half the last place.
        outcome = Decimal(c.get("outcome"))
        half = Decimal(1).scaleb(outcome.as_tuple().exponent) / 2
        got = _decimal(result.stdout.strip())
        if abs(got - outcome) > half:
            bad.append(f"{c.get('id')}: {c.get('value')} {c.get('srcUnit')} -> "
                       f"{c.get('dstUnit')}: {got}, not {outcome}")
    return bad


@when("I run every conversion case")
def every_conversion_case(ctx, run, binary, ucum_functional_tests: Path):
    ctx.attempt(lambda: conversion_disagreements(run, binary("pubunit"), ucum_functional_tests))


# --- thens -------------------------------------------------------------------


@then(parsers.parse('the JSON it prints has the one key "{key}"'))
def one_key(result, key: str):
    """pubtest reads a value at a key, but not which keys there are."""
    assert list(json.loads(result.stdout)) == [key], result.describe()
