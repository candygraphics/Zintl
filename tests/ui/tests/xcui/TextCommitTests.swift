import XCTest

@MainActor
final class TextCommitTests: ZintlUITestCase {
  /// Verifies typing does not commit, and Return and Tab each commit the final text once.
  func testReturnAndTabCommitTheFinalText() throws {
    try launch(scenario: "text-commit")

    let field = app.textFields["name-field"]
    let commits = app.staticTexts["commits"]
    XCTAssertTrue(field.waitToExist(timeout: 2))
    field.click()
    field.typeText("Section")
    XCTAssertEqual(commits.value as? String, "Commits: ")

    field.typeKey(.return, modifierFlags: [])
    XCTAssertTrue(waitForValue(commits, "Commits: Section"))

    // Return selects the whole text, so move to its end before typing.
    field.typeKey(.rightArrow, modifierFlags: [])
    field.typeText(" A")
    field.typeKey(.tab, modifierFlags: [])
    XCTAssertTrue(waitForValue(commits, "Commits: Section|Section A"))
  }

  /// Verifies the number field shows formatted text, parses typed text on Return, and restores invalid text.
  func testNumberFieldFormatsAndParsesKilometrePosts() throws {
    try launch(scenario: "text-commit")

    let field = app.textFields.matching(NSPredicate(format: "value ENDSWITH ' m'")).firstMatch
    let position = app.staticTexts["position"]
    XCTAssertTrue(field.waitToExist(timeout: 2))
    XCTAssertEqual(field.value as? String, "1k000.00 m")

    field.click()
    field.typeKey("a", modifierFlags: .command)
    field.typeText("12k345.5\r")
    XCTAssertTrue(waitForValue(position, "Position: 12345.5"))
    XCTAssertTrue(waitForValue(field, "12k345.50 m"))

    field.typeKey("a", modifierFlags: .command)
    field.typeText("north\r")
    XCTAssertTrue(waitForValue(field, "12k345.50 m"))
    XCTAssertEqual(position.value as? String, "Position: 12345.5")
  }
}
