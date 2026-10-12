Feature: A unit is checked against a declared quantity kind
  A dimension does not say what is measured: energy and torque share
  m2.s-2.g. A quantity kind is declared with a name, a dimension and the
  units that may measure it, and never inferred from a dimension.

  Background:
    Given the definitions "examples/publication-0.json"

  Scenario Outline: A unit of the kind's dimension that the kind lists fits
    When I run pubunit check "<unit>" "<kind>"
    Then it succeeds
    And it prints "<unit> measures <kind>"

    Examples:
      | unit             | kind                       |
      | kW.h             | energy                     |
      | N.m              | torque                     |
      | kW.h/{tray}      | electrical energy per tray |
      | (kW.h)/{tray}    | electrical energy per tray |
      | kg{CO2e}/(kW.h)  | grid carbon intensity      |
      | [EUR]/{tray}     | price per tray             |
      | Cel              | air temperature            |

  Scenario Outline: A unit of the right dimension that the kind does not list is refused
    When I run pubunit check "<unit>" "<kind>"
    Then it fails
    And stderr mentions "does not allow"

    Examples:
      | unit | kind   |
      | J    | torque |
      | N.m  | energy |

  Scenario Outline: A unit of another dimension is refused
    When I run pubunit check "<unit>" "<kind>"
    Then it fails
    And stderr mentions "but `<kind>` is"

    Examples:
      | unit        | kind           |
      | kg          | climate impact |
      | kg{CO2e}    | harvest mass   |
      | kW.h        | electrical energy per tray |

  Scenario: An unknown kind is refused
    When I run pubunit check "kg" "mass of basil"
    Then it fails
    And stderr mentions "no quantity kind is named"

  Scenario: A conversion can be held to one kind
    When I run pubunit convert "420" "g{CO2e}/{tray}" "kg{CO2e}/{tray}" --kind "climate impact per tray"
    Then the output is "0.42"
    When I run pubunit convert "1" "J" "N.m" --kind "energy"
    Then it fails
    And stderr mentions "does not allow"

  Scenario: The declared kinds can be listed
    When I run pubunit kinds
    Then it prints "torque: m2.s-2.g [N.m]"
    And it prints "climate impact: g.{CO2e} [g{CO2e}, kg{CO2e}, t{CO2e}]"
