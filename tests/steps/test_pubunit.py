"""Steps for every pubunit feature."""

from __future__ import annotations

import json
import shlex
import xml.etree.ElementTree as ET
from decimal import Decimal
from pathlib import Path

from pytest_bdd import given, parsers, scenarios, then, when

from conftest import ROOT

scenarios("../features/reduce.feature")
scenarios("../features/convert.feature")
scenarios("../features/dimensions.feature")
scenarios("../features/kinds.feature")
scenarios("../features/definitions.feature")
scenarios("../features/parse.feature")
scenarios("../features/cli.feature")
scenarios("../features/conformance.feature")
scenarios("../features/qudt.feature")


# --- givens ------------------------------------------------------------------


@given(parsers.parse('the definitions "{path}"'))
def definitions_file(ctx, path: str):
    ctx.defs = str(ROOT / path)


@given("the definitions")
def definitions_inline(ctx, tmp_path: Path, docstring: str):
    path = tmp_path / "definitions.json"
    path.write_text(docstring)
    ctx.defs = str(path)


# --- whens -------------------------------------------------------------------


@when(parsers.parse("I run pubunit {args}"), target_fixture="result")
def run_pubunit(ctx, run, pubunit, args: str):
    argv = shlex.split(args)
    defs = getattr(ctx, "defs", None)
    if defs and argv and not argv[0].startswith("--"):
        argv = ["--defs", defs, *argv]
    return run(pubunit, *argv)


@when("I run every validation case", target_fixture="disagreements")
def every_validation_case(run, pubunit, ucum_functional_tests: Path):
    root = ET.parse(ucum_functional_tests).getroot()
    cases = [c for c in root.find("validation") if c.tag == "case"]
    assert len(cases) > 500
    bad = []
    for c in cases:
        result = run(pubunit, "--annotations=ucum", "validate", c.get("unit"))
        if (result.code == 0) != (c.get("valid") == "true"):
            bad.append(f"{c.get('id')} {c.get('unit')!r}: valid={c.get('valid')}, "
                       f"pubunit: {result.stderr.strip() or 'valid'}")
    return bad


def _decimal(text: str) -> Decimal:
    if "/" in text:
        num, den = text.split("/")
        return Decimal(num) / Decimal(den)
    return Decimal(text)


@when("I run every conversion case", target_fixture="disagreements")
def every_conversion_case(run, pubunit, ucum_functional_tests: Path):
    root = ET.parse(ucum_functional_tests).getroot()
    cases = [c for c in root.find("conversion") if c.tag == "case"]
    assert len(cases) > 20
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


# --- thens -------------------------------------------------------------------


@then(parsers.parse('the output is "{text}"'))
def output_is(result, text: str):
    assert result.stdout.strip() == text, result.describe()


@then(parsers.parse('the first key is "{key}"'))
def first_key(result, key: str):
    assert list(json.loads(result.stdout)) == [key], result.describe()


@then("every case agrees")
def every_case_agrees(disagreements):
    assert not disagreements, "\n".join(disagreements)
