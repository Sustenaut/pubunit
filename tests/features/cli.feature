Feature: The command line
  pubunit's commands, options and exit status are its versioned interface.

  Scenario: --version prints the version
    When I run pubunit --version
    Then it succeeds
    And it prints "pubunit "

  Scenario: --help lists every command
    When I run pubunit --help
    Then it succeeds
    And it prints "parse EXPR"
    And it prints "convert VALUE FROM TO"
    And it prints "check UNIT KIND"

  Scenario: --help works after a command too
    When I run pubunit convert --help
    Then it succeeds
    And it prints "convert VALUE FROM TO"

  Scenario: An unknown command fails with the usage
    When I run pubunit frobnicate
    Then it fails
    And stderr mentions "unknown command frobnicate"

  Scenario: A command with the wrong arguments prints its usage
    When I run pubunit convert "1" "m"
    Then it fails
    And stderr mentions "usage: pubunit convert VALUE FROM TO"

  Scenario: The UCUM version is stated
    When I run pubunit ucum
    Then it prints "UCUM 2.2 (2024-06-17), from ucum-units 0.1.0"
