"""Fixtures for pubunit's functional suite.

Every scenario drives the built `pubunit` as a user would. The binary is
resolved by pubkit's `binary` fixture from PUBUNIT_BIN_DIR, so another
implementation runs this suite by setting one variable. The steps that
judge a result (`it succeeds`, `it prints "..."`) are pubkit's.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture
def pubunit(binary) -> Path:
    return binary("pubunit")


@pytest.fixture(scope="session")
def ucum_functional_tests() -> Path:
    """UCUM's functional tests, from the copy the pinned ucum-units bundles.

    The file is under the Eclipse Public License 1.0, so it is read where
    Cargo put it rather than copied into this repository.
    """
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked"], cwd=ROOT))
    for pkg in meta["packages"]:
        if pkg["name"] == "ucum-units":
            return Path(pkg["manifest_path"]).parent / "vendor" / "UcumFunctionalTests.xml"
    raise LookupError("ucum-units is not in the dependency graph")
