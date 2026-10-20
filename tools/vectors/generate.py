#!/usr/bin/env python3
"""Generate the test vectors in testdata/ from independent sources.

Nothing here calls pubunit or reads UCUM's table. Expected values are
computed with Python's fractions.Fraction, exactly, from:

- the exact definitions of customary units: the international yard and
  pound (1959), the US gallon (231 cubic inches), the thermochemical
  calorie (4.184 J), standard gravity (9.80665 m/s2), the standard
  atmosphere (101 325 Pa), and the temperature scales, as NIST SP 811
  (2008), Appendix B, lists them;
- the SI Brochure, 9th edition (2019), Table 4: SI derived units expressed
  in SI base units, translated to UCUM's base units (gram, and coulomb for
  charge) here, by its definitions kg = 1000 g and A = C/s.

The vectors are committed; this script documents and reproduces them.
Run from the repository root: tools/vectors/generate.py
"""

import json
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

NIST = "NIST SP 811 (2008), Appendix B, exact factors"
SI = "SI Brochure, 9th edition (2019), Table 4"


def write(name, data):
    path = ROOT / "testdata" / name
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {path.relative_to(ROOT)}")


def text(q):
    return str(q.numerator) if q.denominator == 1 else f"{q.numerator}/{q.denominator}"


# --- exact definitions, in metres, kilograms, seconds, kelvin -------------

INCH = F(254, 10_000)                   # m (0.0254), international yard and pound, 1959
FOOT = 12 * INCH
YARD = 3 * FOOT
MILE = 5280 * FOOT
NAUTICAL_MILE = F(1852)
POUND = F("0.45359237")                 # kg
OUNCE = POUND / 16
GALLON_US = 231 * INCH**3               # m3
FLUID_OUNCE_US = GALLON_US / 128
LITRE = F(1, 1000)
HOUR, DAY = F(3600), F(86400)
CALORIE = F("4.184")                    # J, thermochemical
GRAVITY = F("9.80665")                  # m/s2
ATMOSPHERE = F(101325)                  # Pa
BAR = F(100000)
SQUARE_INCH = INCH * INCH               # m2
PSI = POUND * GRAVITY / SQUARE_INCH     # lbf/in2, in Pa


def conversions():
    # (name, value, from, to, result): result is value in `to`.
    cases = [
        ("inch to centimetre", 1, "[in_i]", "cm", INCH * 100),
        ("foot to metre", 1, "[ft_i]", "m", FOOT),
        ("yard to foot", 1, "[yd_i]", "[ft_i]", YARD / FOOT),
        ("foot to yard", 1, "[ft_i]", "[yd_i]", FOOT / YARD),
        ("mile to kilometre", 1, "[mi_i]", "km", MILE / 1000),
        ("nautical mile to mile", 1, "[nmi_i]", "[mi_i]", NAUTICAL_MILE / MILE),
        ("pound to kilogram", 1, "[lb_av]", "kg", POUND),
        ("ounce to gram", 1, "[oz_av]", "g", OUNCE * 1000),
        ("kilogram to pound", 1, "kg", "[lb_av]", 1 / POUND),
        ("US gallon to litre", 1, "[gal_us]", "L", GALLON_US / LITRE),
        ("US fluid ounce to millilitre", 1, "[foz_us]", "mL", FLUID_OUNCE_US / LITRE * 1000),
        ("cubic inch to millilitre", 1, "[in_i]3", "mL", INCH**3 / LITRE * 1000),
        ("day to hour", 1, "d", "h", DAY / HOUR),
        ("kilowatt hour to megajoule", 1, "kW.h", "MJ", 1000 * HOUR / 10**6),
        ("calorie to joule", 1, "cal", "J", CALORIE),
        ("kilocalorie to kilowatt hour", 1, "kcal", "kW.h", 1000 * CALORIE / (1000 * HOUR)),
        ("standard atmosphere to kilopascal", 1, "atm", "kPa", ATMOSPHERE / 1000),
        ("bar to standard atmosphere", 1, "bar", "atm", BAR / ATMOSPHERE),
        ("pound-force per square inch to pascal", 1, "[psi]", "Pa", PSI),
        ("pound-force to newton", 1, "[lbf_av]", "N", POUND * GRAVITY),
        ("standard gravity to metre per second squared", 1, "[g]", "m/s2", GRAVITY),
        ("mile per hour to metre per second", 1, "[mi_i]/h", "m/s", MILE / HOUR),
        ("kilometre per hour to metre per second", 1, "km/h", "m/s", F(1000) / HOUR),
        ("gram per US gallon to milligram per litre", 1, "g/[gal_us]", "mg/L", 1000 * LITRE / GALLON_US),
        ("pound per square foot to kilogram per square metre", 1, "[lb_av]/[ft_i]2", "kg/m2", POUND / FOOT**2),
        ("watt hour per square foot to watt hour per square metre", 1, "W.h/[ft_i]2", "W.h/m2", 1 / FOOT**2),
    ]
    temperatures = [
        ("body temperature, Fahrenheit to Celsius", F("98.6"), "[degF]", "Cel", (F("98.6") - 32) * F(5, 9)),
        ("minus forty, Celsius to Fahrenheit", -40, "Cel", "[degF]", F(-40)),
        ("absolute zero in Celsius", 0, "K", "Cel", F("-273.15")),
        ("freezing point in kelvin", 32, "[degF]", "K", F("273.15")),
        ("grow room, Celsius to Fahrenheit", F("21.5"), "Cel", "[degF]", F("21.5") * F(9, 5) + 32),
    ]
    return {
        "description": "Values converted exactly between units with exact definitions.",
        "source": NIST + "; the temperature scales by their definitions "
                  "(K = °C + 273.15, °F = °C × 9/5 + 32).",
        "cases": [
            {"name": n, "value": text(F(v)), "from": a, "to": b, "result": text(F(r))}
            for n, v, a, b, r in cases + temperatures
        ],
    }


# --- SI derived units, in UCUM's base units -------------------------------

# Table 4, as (symbol, {SI base: exponent}).
DERIVED = [
    ("Hz", {"s": -1}),
    ("N", {"kg": 1, "m": 1, "s": -2}),
    ("Pa", {"kg": 1, "m": -1, "s": -2}),
    ("J", {"kg": 1, "m": 2, "s": -2}),
    ("W", {"kg": 1, "m": 2, "s": -3}),
    ("C", {"A": 1, "s": 1}),
    ("V", {"kg": 1, "m": 2, "s": -3, "A": -1}),
    ("F", {"kg": -1, "m": -2, "s": 4, "A": 2}),
    ("Ohm", {"kg": 1, "m": 2, "s": -3, "A": -2}),
    ("S", {"kg": -1, "m": -2, "s": 3, "A": 2}),
    ("Wb", {"kg": 1, "m": 2, "s": -2, "A": -1}),
    ("T", {"kg": 1, "s": -2, "A": -1}),
    ("H", {"kg": 1, "m": 2, "s": -2, "A": -2}),
    ("Gy", {"m": 2, "s": -2}),
    ("Sv", {"m": 2, "s": -2}),
    ("Bq", {"s": -1}),
]

# UCUM's bases are m, s, g, rad, K, C, cd: kg = 1000 g, and A = C/s.
TO_UCUM = {"m": ({"m": 1}, F(1)), "s": ({"s": 1}, F(1)), "kg": ({"g": 1}, F(1000)),
           "A": ({"C": 1, "s": -1}, F(1)), "K": ({"K": 1}, F(1))}


def si_derived():
    cases = []
    for symbol, si in DERIVED:
        dim, factor = {}, F(1)
        for base, e in si.items():
            ucum, f = TO_UCUM[base]
            factor *= f**e
            for b, x in ucum.items():
                dim[b] = dim.get(b, 0) + x * e
        dim = {b: x for b, x in sorted(dim.items()) if x}
        cases.append({"unit": symbol, "si": si, "dimension": dim, "factor": text(factor)})
    return {
        "description": "SI derived units: their dimension in UCUM's base units, and how many base units one is.",
        "source": SI + ", translated to UCUM's base units by kg = 1000 g and A = C/s.",
        "cases": cases,
    }


write("conversions.json", conversions())
write("si-derived.json", si_derived())
