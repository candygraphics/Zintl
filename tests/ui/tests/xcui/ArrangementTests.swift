import XCTest

@MainActor
final class ArrangementTests: ZintlUITestCase {
  /// Verifies ZStack places the overlay over the first child at the same origin.
  func testZStackOverlaysItsChildren() throws {
    try launch(scenario: "arrangements")

    let preview = app.staticTexts["preview"]
    let overlay = app.staticTexts["overlay"]
    XCTAssertTrue(overlay.waitForExistence(timeout: 2))
    XCTAssertEqual(overlay.frame.minX, preview.frame.minX, accuracy: 1)
    XCTAssertEqual(overlay.frame.minY, preview.frame.minY, accuracy: 1)
  }

  /// Verifies grid cells line up in columns as wide as their widest cell.
  func testGridAlignsColumns() throws {
    try launch(scenario: "arrangements")

    let name = app.staticTexts["cell-0"]
    let length = app.staticTexts["cell-1"]
    let sectionB = app.staticTexts["cell-4"]
    let lengthB = app.staticTexts["cell-5"]
    XCTAssertTrue(name.waitForExistence(timeout: 2))
    XCTAssertEqual(length.frame.minX, lengthB.frame.minX, accuracy: 1)
    XCTAssertEqual(name.frame.minX, sectionB.frame.minX, accuracy: 1)
    XCTAssertGreaterThan(length.frame.minX, sectionB.frame.maxX)
    XCTAssertGreaterThan(sectionB.frame.minY, name.frame.maxY)
  }

  /// Verifies the disclosure triangle shows and hides the group's content.
  func testDisclosureGroupFolds() throws {
    try launch(scenario: "arrangements")

    let triangle = app.disclosureTriangles["disclosure"]
    XCTAssertTrue(triangle.waitForExistence(timeout: 2))
    XCTAssertFalse(app.staticTexts["details"].exists)
    triangle.click()
    XCTAssertTrue(app.staticTexts["details"].waitForExistence(timeout: 2))
    triangle.click()
    XCTAssertTrue(app.staticTexts["details"].waitForNonExistence(timeout: 2))
  }

  /// Verifies scroll_to brings the target row into the visible area.
  func testScrollToRevealsTheTargetRow() throws {
    try launch(scenario: "arrangements")

    let row = app.staticTexts["row-30"]
    XCTAssertTrue(row.waitForExistence(timeout: 2))
    XCTAssertFalse(row.isHittable)
    app.buttons["jump"].click()
    let hittable = NSPredicate(format: "isHittable == true")
    let expectation = XCTNSPredicateExpectation(predicate: hittable, object: row)
    XCTAssertEqual(XCTWaiter.wait(for: [expectation], timeout: 2), .completed)
  }
}
