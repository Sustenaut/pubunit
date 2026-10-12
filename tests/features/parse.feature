Feature: The parse tree is public
  A later package for symbolic computation carries units through formulas,
  so pubunit exposes the tree it parses, not only yes-or-no checks. The
  tree follows UCUM's grammar: `.` and `/` are left-associative and of
  equal precedence.

  Scenario: A quotient parses into its parts
    When I run pubunit parse "kW.h/{tray}"
    Then it succeeds
    And it prints ""div""
    And it prints ""symbol": "kW""
    And it prints ""annotation": "tray""

  Scenario: Operators are left-associative
    When I run pubunit parse "kg/m.s"
    Then the first key is "mul"

  Scenario: An annotation after a unit belongs to it
    When I run pubunit parse "kg{CO2e}"
    Then it prints ""symbol": "kg""
    And it prints ""annotation": "CO2e""

  Scenario: Every symbol is checked to resolve
    When I run pubunit validate "mg/dL"
    Then it succeeds
    When I run pubunit validate "mg/dLL"
    Then it fails
    And stderr mentions "not a known unit"
