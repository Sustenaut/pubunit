Feature: A unit reduces to a dimension and an exact factor
  pubunit reads a UCUM expression and reduces it to a dimension, a product
  of base units, and the exact number of base units one of it is. Mass is
  in grams, as UCUM's base unit. Factors come from UCUM's own decimal
  strings, read as rationals, so they are exact.

  Scenario Outline: Standard UCUM units reduce exactly
    When I run pubunit reduce "<unit>"
    Then it succeeds
    And it prints "dimension: <dimension>"
    And it prints "factor: <factor>"

    Examples:
      | unit           | dimension  | factor           |
      | m              | m          | 1                |
      | kg             | g          | 1000             |
      | kW.h           | m2.s-2.g   | 3600000000       |
      | [in_i]         | m          | 0.0254           |
      | [ft_i]         | m          | 0.3048           |
      | [lb_av]        | g          | 453.59237        |
      | km/h           | m.s-1      | 5/18             |
      | mg/dL          | m-3.g      | 10               |
      | kg/m.s         | m-1.s.g    | 1000             |
      | kg/(m.s)       | m-1.s-1.g  | 1000             |
      | /min           | s-1        | 1/60             |
      | 10*3/uL        | m-3        | 1000000000000    |
      | %              | 1          | 0.01             |

  Scenario: A temperature with an offset reduces when it stands alone
    When I run pubunit reduce "Cel"
    Then it succeeds
    And it prints "dimension: K"
    And it prints "offset: 273.15"

  Scenario Outline: Units with no linear factor are refused
    When I run pubunit reduce "<unit>"
    Then it fails
    And stderr mentions "<reason>"

    Examples:
      | unit   | reason              |
      | dB     | special unit        |
      | [pH]   | special unit        |
      | Cel.m  | special unit        |
      | [IU]/L | arbitrary unit      |

  Scenario Outline: What is not UCUM is refused
    When I run pubunit reduce "<unit>"
    Then it fails
    And stderr mentions "<reason>"

    Examples:
      | unit     | reason                |
      | kmeter   | not a known unit      |
      | m//s     | is not UCUM           |
      | (m       | missing `)`           |
      | [EUR]    | not a known unit      |
      | m65      | too large             |
