@conformance
Feature: pubunit agrees with UCUM's functional tests
  UCUM's functional tests (UcumFunctionalTests.xml, by Grahame Grieve and
  contributors, under the Eclipse Public License 1.0) are read from the
  copy that the pinned ucum-units bundles. They are not copied into this
  repository. Annotations are read as UCUM reads them.

  Background:
    Given a scratch directory
    And the program under test is "pubunit"

  Scenario: Every validation case is judged as UCUM judges it
    When I run every validation case
    Then the result is empty

  Scenario: Every conversion case gives UCUM's outcome, at its precision
    When I run every conversion case
    Then the result is empty
