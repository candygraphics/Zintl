import XCTest

/// Accessibility exposes a tooltip as a help tag without its text, so these
/// tests check that one appears next to the pointer, and only where help is set.
@MainActor
final class HelpTests: ZintlUITestCase {
  /// Rests the pointer on `point` and returns whether a help tag appears near it.
  private func helpTagAppears(at point: XCUICoordinate, timeout: TimeInterval = 5) -> Bool {
    point.hover()
    let tag = app.helpTags.firstMatch
    guard tag.waitToExist(timeout: timeout) else {
      return false
    }
    let pointer = point.screenPoint
    return abs(tag.frame.minX - pointer.x) < 80 && abs(tag.frame.minY - pointer.y) < 80
  }

  /// Returns the center of the element with the identifier `id`.
  private func center(_ id: String) -> XCUICoordinate {
    element(id).coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
  }

  /// Verifies resting the pointer on a control shows its tooltip.
  func testControlShowsItsTooltip() throws {
    try launch(scenario: "help")

    XCTAssertTrue(helpTagAppears(at: center("save")))
  }

  /// Verifies a control without help shows no tooltip.
  func testControlWithoutHelpShowsNone() throws {
    try launch(scenario: "help")

    XCTAssertFalse(helpTagAppears(at: center("reset"), timeout: 3))
  }

  /// Verifies a disabled control still shows its tooltip.
  func testDisabledControlShowsItsTooltip() throws {
    try launch(scenario: "help")

    XCTAssertFalse(app.buttons["publish"].isEnabled)
    XCTAssertTrue(helpTagAppears(at: center("publish")))
  }

  /// Verifies a container shows its tooltip over its own area.
  func testContainerShowsItsTooltip() throws {
    try launch(scenario: "help")

    // Rests on the container's padding above its label.
    let padding = element("depot").coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0))
      .withOffset(CGVector(dx: 0, dy: -8))
    XCTAssertTrue(helpTagAppears(at: padding))
  }
}
