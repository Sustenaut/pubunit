Feature: QUDT's quantity kinds can be declared
  pubunit carries QUDT's quantity kinds, generated from a pinned QUDT
  release at build time, so a column can name a standard kind instead of
  declaring its own. Each is named qudt: and its QUDT local name.

  Scenario: The QUDT release is stated
    When I run pubunit qudt
    Then it succeeds
    And it prints "QUDT 3.5.2"
    And it prints "CC BY 4.0"

  Scenario Outline: A unit QUDT applies to a kind fits it
    When I run pubunit check "<unit>" "<kind>" --qudt
    Then it succeeds
    And it prints "<unit> measures <kind>"

    Examples:
      | unit    | kind                  |
      | kW.h    | qudt:Energy           |
      | N.m     | qudt:Torque           |
      | Cel     | qudt:Temperature      |
      | rad.s-1 | qudt:AngularVelocity  |
      | A       | qudt:ElectricCurrent  |
      | L       | qudt:Volume           |
      | g.m-2   | qudt:MassPerArea      |

  Scenario Outline: Kinds that share a dimension keep their own units
    When I run pubunit check "<unit>" "<kind>" --qudt
    Then it fails
    And stderr mentions "does not allow"

    Examples:
      | unit | kind          |
      | N.m  | qudt:Energy   |
      | J    | qudt:Torque   |
      | Bq   | qudt:Frequency |
      | Hz   | qudt:Activity |

  Scenario: A unit of another dimension is refused
    When I run pubunit check "kg" "qudt:Energy" --qudt
    Then it fails
    And stderr mentions "but `qudt:Energy` is"

  Scenario: QUDT's kinds sit beside declared ones
    Given the definitions "examples/publication-0.json"
    When I run pubunit check "kW.h/{tray}" "electrical energy per tray" --qudt
    Then it succeeds
    When I run pubunit check "kW.h" "qudt:Energy" --qudt
    Then it succeeds

  Scenario: Without --qudt there are no QUDT kinds
    When I run pubunit check "kW.h" "qudt:Energy"
    Then it fails
    And stderr mentions "no quantity kind is named"
