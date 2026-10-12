Feature: Base dimensions are declared as data
  UCUM reads {tray} as an annotation with no dimension, and has no money.
  pubunit lets definitions declare them as base dimensions: counted things
  such as {tray} and {batch}, currencies such as [EUR], and substance
  equivalents such as {CO2e}. A brace symbol counts wherever it appears,
  alone or after a unit: kg{CO2e} is a kilogram of CO2 equivalent.

  Background:
    Given the definitions "examples/publication-0.json"

  Scenario: Energy per tray is not energy
    When I run pubunit reduce "kW.h/{tray}"
    Then it prints "dimension: m2.s-2.g.{tray}-1"
    When I run pubunit convert "1" "kW.h/{tray}" "kW.h"
    Then it fails
    And stderr mentions "different dimensions"

  Scenario: A kilogram of CO2e is not a kilogram
    When I run pubunit reduce "kg{CO2e}"
    Then it prints "dimension: g.{CO2e}"
    When I run pubunit convert "1" "kg{CO2e}" "kg{basil}"
    Then it fails
    And stderr mentions "different dimensions"

  Scenario: An undeclared annotation after a unit is only a label
    When I run pubunit convert "1500" "g{basil}" "kg"
    Then the output is "1.5"

  Scenario: Units of a declared dimension convert among themselves
    When I run pubunit convert "2.5" "t{CO2e}" "kg{CO2e}"
    Then the output is "2500"

  Scenario: A currency never converts to another
    Given the definitions
      """
      { "dimensions": [ { "symbol": "[EUR]", "name": "euro" },
                        { "symbol": "[USD]", "name": "US dollar" } ] }
      """
    When I run pubunit convert "1" "[EUR]" "[USD]"
    Then it fails
    And stderr mentions "different dimensions"

  Scenario: The worked example's chain has the dimension of its result
    When I run pubunit reduce "kW.h/{tray}.kg{CO2e}/(kW.h)"
    Then it prints "dimension: g.{CO2e}.{tray}-1"
    And it prints "factor: 1000"

  Scenario: A misspelt annotation standing alone is refused
    When I run pubunit reduce "kW.h/{trays}"
    Then it fails
    And stderr mentions "no definition declares it"

  Scenario: UCUM's reading of annotations can be asked for
    When I run pubunit reduce "kW.h/{trays}" --annotations=ucum
    Then it succeeds
    And it prints "dimension: m2.s-2.g"

  Scenario: A unit can be declared as a multiple of another
    Given the definitions
      """
      { "dimensions": [ { "symbol": "{tray}", "name": "growing tray" } ],
        "units": [ { "symbol": "{flat}", "name": "flat of six trays", "value": "6", "unit": "{tray}" } ] }
      """
    When I run pubunit convert "3" "{flat}" "{tray}"
    Then the output is "18"
