#!/usr/bin/env python3
"""Write data/qudt/kinds.json and data/qudt/REPORT.md from a pinned QUDT release.

pubunit's starting set of quantity kinds comes from QUDT, cited by release,
so it is reproducible and a QUDT update shows up as a reviewable diff. RDF
is read here, at build time, with Oxigraph; pubunit itself never reads RDF.

    tools/gen-qudt-kinds.py           # rewrite data/qudt/
    tools/gen-qudt-kinds.py --check   # fail if the committed copy differs

The release named in data/qudt/source.json is downloaded once into
target/qudt/ and checked against its SHA-256. Each kind's units are the
UCUM codes QUDT gives its applicable units, read by the pubunit command
built from this checkout (target/debug/pubunit, or $PUBUNIT_BIN_DIR), so
every unit kept is one pubunit can reduce. A kind is kept when those units
agree with QUDT's dimension vector; REPORT.md says why each other kind was
left out.
"""

import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import urllib.request
import zipfile
from collections import defaultdict

import pyoxigraph as ox

ROOT = pathlib.Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "qudt"
SOURCE = DATA / "source.json"
KINDS = DATA / "kinds.json"
REPORT = DATA / "REPORT.md"
CACHE = ROOT / "target" / "qudt"
FILES = [
    "vocab/quantitykinds/VOCAB_QUDT-QUANTITY-KINDS-ALL.ttl",
    "vocab/unit/VOCAB_QUDT-UNITS-ALL.ttl",
]
QK = "http://qudt.org/vocab/quantitykind/"
PREFIXES = """
PREFIX qudt: <http://qudt.org/schema/qudt/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
"""
# QUDT's base dimensions, in UCUM's base units. UCUM has charge where QUDT
# has current (A = C/s), counts moles as a number, and makes plane angle a
# base where QUDT calls it dimensionless; D marks a ratio and is no base.
QUDT_DIMS = re.compile(r"A(-?\d+)E(-?\d+)L(-?\d+)I(-?\d+)M(-?\d+)H(-?\d+)T(-?\d+)D(-?\d+)$")


def release() -> tuple[dict, pathlib.Path]:
    src = json.loads(SOURCE.read_text())
    path = CACHE / pathlib.Path(src["url"]).name
    if not path.exists():
        CACHE.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(src["url"]) as r:
            path.write_bytes(r.read())
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != src["sha256"]:
        sys.exit(f"{path}: sha256 {digest}, but {SOURCE.relative_to(ROOT)} pins {src['sha256']}")
    return src, path


def store(path: pathlib.Path) -> ox.Store:
    s = ox.Store()
    with zipfile.ZipFile(path) as z:
        for name in FILES:
            s.bulk_load(z.read(name), format=ox.RdfFormat.TURTLE)
    return s


def ucum_dimension(vector: str) -> dict[str, int] | str:
    """QUDT's vector in UCUM's bases, plane angle aside, or why it has none."""
    name = vector.rsplit("/", 1)[-1]
    if "dot" in name:
        return "has a fractional exponent, which a UCUM unit cannot have"
    m = QUDT_DIMS.match(name)
    if not m:
        return "QUDT gives no single dimension vector"
    a, e, l, i, mass, h, t, _d = map(int, m.groups())
    if a:
        return "involves amount of substance, which UCUM counts as a number"
    dims = {"m": l, "s": t - e, "g": mass, "K": h, "C": e, "cd": i}
    return {k: v for k, v in dims.items() if v}


TOKEN = re.compile(r"^(.*?)(-?\d+)?$")


def parse_dimension(text: str) -> dict[str, int]:
    if text in ("", "1"):
        return {}
    out = {}
    for part in text.split("."):
        sym, exp = TOKEN.match(part).groups()
        out[sym] = int(exp or 1)
    return out


def pubunit() -> str:
    bin_dir = os.environ.get("PUBUNIT_BIN_DIR", str(ROOT / "target" / "debug"))
    return str(pathlib.Path(bin_dir) / "pubunit")


def reduce_all(codes: set[str]) -> dict[str, dict[str, int] | str]:
    """Each code's dimension, or the reason pubunit refuses it."""
    exe, out = pubunit(), {}
    for code in sorted(codes):
        if "{" in code:
            out[code] = "an annotation, which pubunit reads strictly"
            continue
        r = subprocess.run([exe, "reduce", code], capture_output=True, text=True)
        if r.returncode:
            out[code] = r.stderr.strip().removeprefix("pubunit: ")
            continue
        line = next(x for x in r.stdout.splitlines() if x.startswith("dimension: "))
        out[code] = parse_dimension(line.removeprefix("dimension: "))
    return out


def query(s: ox.Store, q: str):
    return s.query(PREFIXES + q)


def collect(s: ox.Store):
    kinds = {}
    for r in query(s, """
        SELECT ?k ?label ?dv ?dep WHERE {
          ?k a qudt:QuantityKind .
          OPTIONAL { ?k rdfs:label ?label FILTER(langMatches(lang(?label), "en")) }
          OPTIONAL { ?k qudt:hasDimensionVector ?dv }
          OPTIONAL { ?k qudt:deprecated ?dep }
        }"""):
        k = r["k"].value
        if not k.startswith(QK):
            continue
        entry = kinds.setdefault(k, {"labels": set(), "dv": set(), "deprecated": False, "codes": set(), "broader": set()})
        if r["label"]:
            entry["labels"].add(r["label"].value)
        if r["dv"]:
            entry["dv"].add(r["dv"].value)
        if r["dep"] and r["dep"].value == "true":
            entry["deprecated"] = True
    for r in query(s, "SELECT ?k ?code WHERE { ?k a qudt:QuantityKind ; qudt:applicableUnit ?u . ?u qudt:ucumCode ?code }"):
        if r["k"].value in kinds:
            kinds[r["k"].value]["codes"].add(r["code"].value)
    for r in query(s, "SELECT ?k ?b WHERE { ?k a qudt:QuantityKind ; skos:broader ?b }"):
        if r["k"].value in kinds:
            kinds[r["k"].value]["broader"].add(r["b"].value)
    return kinds


def local(uri: str) -> str:
    return uri.removeprefix(QK)


def decide(kinds, reduced):
    """Each kind's dimension and units, or why it is left out."""
    kept, left_out, notes = {}, {}, {}

    def own(k):
        """Units of k that agree with its QUDT vector, with the dropped ones."""
        e = kinds[k]
        want = ucum_dimension(next(iter(e["dv"])))
        groups = defaultdict(list)
        for code in e["codes"]:
            d = reduced[code]
            if isinstance(d, dict):
                groups[tuple(sorted(d.items()))].append(code)
        # Plane angle is a base in UCUM only: compare without it.
        match = [g for g in groups if {b: x for b, x in g if b != "rad"} == want]
        if not match:
            return None
        match.sort(key=lambda g: (-len(groups[g]), g))
        if len(match) > 1 and len(groups[match[0]]) == len(groups[match[1]]):
            return "ambiguous"
        return dict(match[0]), sorted(groups[match[0]])

    for k in sorted(kinds):
        e = kinds[k]
        if e["deprecated"]:
            left_out[k] = "deprecated in QUDT"
            continue
        want = ucum_dimension(next(iter(e["dv"]))) if len(e["dv"]) == 1 else "QUDT gives no single dimension vector"
        if isinstance(want, str):
            left_out[k] = want
            continue
        found = own(k)
        source = None
        if found is None:
            # "Applicable units are those of" a broader kind: take the nearest
            # broader kind with the same vector whose units agree.
            seen, frontier = {k}, sorted(e["broader"])
            while frontier and found is None:
                nxt = []
                for b in frontier:
                    if b in seen or b not in kinds:
                        continue
                    seen.add(b)
                    if kinds[b]["dv"] == e["dv"] and not kinds[b]["deprecated"]:
                        f = own(b)
                        if isinstance(f, tuple):
                            found, source = f, b
                            break
                    nxt += sorted(kinds[b]["broader"])
                frontier = nxt
        if found == "ambiguous":
            left_out[k] = "its units split evenly between plane angle and none"
            continue
        if found is None:
            left_out[k] = "no applicable unit has a UCUM code pubunit reads with QUDT's dimension"
            continue
        dim, units = found
        if source:
            notes[k] = f"units from the broader kind {local(source)}"
        dropped = sorted(c for c in e["codes"] if c not in units) if not source else []
        if dropped:
            notes[k] = "dropped " + ", ".join(f"`{c}`" for c in dropped)
        kept[k] = (dim, units)
    return kept, left_out, notes


def label(e) -> str:
    labels = sorted(e["labels"])
    return labels[0] if labels else ""


def render(src, kinds, kept, left_out, notes, reduced) -> tuple[str, str]:
    out = []
    for k, (dim, units) in kept.items():
        entry = {"name": f"qudt:{local(k)}"}
        if label(kinds[k]):
            entry["label"] = label(kinds[k])
        entry |= {"dimension": dim, "units": units, "id": k}
        out.append(entry)
    # One kind to a line, so a QUDT update reads as a diff of kinds.
    body = ",\n".join("  " + json.dumps(e, ensure_ascii=False) for e in out)
    text = f'{{\n "kinds": [\n{body}\n ]\n}}\n'

    refused = sorted({c for c, d in reduced.items() if not isinstance(d, dict)})
    reasons = defaultdict(list)
    for k, why in left_out.items():
        reasons[why].append(local(k))
    lines = [
        f"# QUDT {src['version']} quantity kinds",
        "",
        f"Generated by `tools/gen-qudt-kinds.py` from [{pathlib.Path(src['url']).name}]({src['url']})",
        f"(sha256 `{src['sha256']}`). Do not edit: run `make qudt-kinds`.",
        "",
        f"{len(kept)} of {len(kinds)} quantity kinds are in `kinds.json`, each named",
        "`qudt:<local name>` with its QUDT URI as `id`. A kind's units are the UCUM",
        "codes of its applicable units that pubunit reduces to QUDT's dimension",
        "vector, read in UCUM's bases (charge for current, plane angle aside).",
        "",
        "## Left out",
        "",
        "| Reason | Kinds |",
        "|---|---|",
    ]
    for why in sorted(reasons, key=lambda w: (-len(reasons[w]), w)):
        lines.append(f"| {why} ({len(reasons[why])}) | {', '.join(reasons[why])} |")
    lines += ["", "## Kept, with a note", "", "| Kind | Note |", "|---|---|"]
    for k in sorted(notes):
        if k in kept:
            lines.append(f"| {local(k)} | {notes[k]} |")
    lines += ["", "## UCUM codes pubunit does not reduce", "", "| Code | Why |", "|---|---|"]
    for c in refused:
        lines.append(f"| `{c}` | {reduced[c]} |".replace("\n", " "))
    return text, "\n".join(lines) + "\n"


def main() -> None:
    src, path = release()
    kinds = collect(store(path))
    reduced = reduce_all({c for e in kinds.values() for c in e["codes"]})
    kept, left_out, notes = decide(kinds, reduced)
    kinds_text, report_text = render(src, kinds, kept, left_out, notes, reduced)
    if "--check" in sys.argv[1:]:
        for f, t in ((KINDS, kinds_text), (REPORT, report_text)):
            if not f.exists() or f.read_text() != t:
                sys.exit(f"{f.relative_to(ROOT)} is stale: run `make qudt-kinds`")
        print(f"qudt-kinds: current (QUDT {src['version']})")
        return
    KINDS.write_text(kinds_text)
    REPORT.write_text(report_text)
    print(f"wrote {len(kept)} kinds to {KINDS.relative_to(ROOT)}, {len(left_out)} left out")


if __name__ == "__main__":
    main()
