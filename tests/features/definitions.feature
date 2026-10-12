Feature: Definitions that cannot be used are refused
  Definitions are plain JSON. A field the format does not define, a symbol
  that clashes with UCUM or an earlier declaration, and a kind whose units
  do not have its dimension are refused when the definitions are read, not
  when a unit happens to use them.

  Scenario Outline: A bad definition is refused when read
    Given the definitions
      """
      <json>
      """
    When I run pubunit reduce "m"
    Then it fails
    And stderr mentions "<reason>"

    Examples:
      | json                                                                                              | reason                     |
      | { "dimension": [] }                                                                               | unknown field              |
      | { "dimensions": [ { "symbol": "[ft_i]", "name": "foot" } ] }                                      | already defined            |
      | { "dimensions": [ { "symbol": "tray", "name": "tray" } ] }                                        | one brace annotation       |
      | { "dimensions": [ { "symbol": "{a}{b}", "name": "two" } ] }                                       | one brace annotation       |
      | { "dimensions": [ { "symbol": "{tray}", "name": "tray" }, { "symbol": "{tray}", "name": "again" } ] } | already defined        |
      | { "dimensions": [ { "symbol": "{tray}", "name": "" } ] }                                          | needs a name               |
      | { "units": [ { "symbol": "{dozen}", "name": "dozen", "value": "0", "unit": "1" } ] }              | not greater than zero      |
      | { "units": [ { "symbol": "{x}", "name": "x", "value": "1", "unit": "Cel" } ] }                    | has an offset              |
      | { "units": [ { "symbol": "{x}", "name": "x", "value": "1", "unit": "{y}" } ] }                    | no definition declares it  |
      | { "kinds": [ { "name": "length", "dimension": { "m": 1 }, "units": ["s"] } ] }                    | is s, not m                |
      | { "kinds": [ { "name": "length", "dimension": { "m": 1 }, "units": [] } ] }                       | `units` is empty           |
      | { "kinds": [ { "name": "pallets", "dimension": { "{pallet}": 1 } } ] }                            | not a base dimension       |
