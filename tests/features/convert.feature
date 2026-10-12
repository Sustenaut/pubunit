Feature: Conversion is exact, and only between commensurable units
  A value converts between two units of the same dimension. The result is
  exact: a decimal when it has a finite one, a fraction otherwise. Values
  are read as decimals or fractions, never as floats.

  Scenario Outline: Values convert exactly
    When I run pubunit convert "<value>" "<from>" "<to>"
    Then it succeeds
    And the output is "<result>"

    Examples:
      | value | from     | to      | result    |
      | 1     | [mi_i]   | km      | 1.609344  |
      | 6.3   | [in_i]   | cm      | 16.002    |
      | 1     | [ft_i]   | [yd_i]  | 1/3       |
      | 2.5   | kW.h     | MJ      | 9         |
      | 1     | [gal_us] | L       | 3.785411784 |
      | 1/3   | h        | min     | 20        |
      | 98.6  | [degF]   | Cel     | 37        |
      | -40   | Cel      | [degF]  | -40       |
      | 0     | K        | Cel     | -273.15   |

  Scenario: A result can be rounded, halves to even
    When I run pubunit convert "1" "[ft_i]" "[yd_i]" --places 4
    Then the output is "0.3333"

  Scenario: Units of different dimensions do not convert
    When I run pubunit convert "1" "kg" "m"
    Then it fails
    And stderr mentions "different dimensions"

  Scenario: A value that is not a number is refused
    When I run pubunit convert "1,5" "kg" "g"
    Then it fails
    And stderr mentions "not a number"

  Scenario: Rounding is bounded
    When I run pubunit convert "1" "m" "m" --places 1001
    Then it fails
    And stderr mentions "at most 1000"

  Scenario: A huge factor is refused, not computed
    When I run pubunit reduce "[pi]64.[pi]64.[pi]64.[pi]64.[pi]64.[pi]64.[pi]64.[pi]64"
    Then it fails
    And stderr mentions "too large"
