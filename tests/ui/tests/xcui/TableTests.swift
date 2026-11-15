import XCTest

@MainActor
final class TableTests: ZintlUITestCase {
  /// Verifies the table shows column headers and lines cells up under them.
  func testColumnsHaveHeadersAndAlignedCells() throws {
    try launch(scenario: "table")

    let table = app.tables["sections"]
    XCTAssertTrue(table.waitToExist(timeout: 2))
    // The ID names the table only, not the scroll view around it.
    XCTAssertFalse(app.scrollViews["sections"].exists)
    XCTAssertTrue(table.buttons["Name"].exists || table.staticTexts["Name"].exists)
    let first = app.staticTexts["length-1"]
    let second = app.staticTexts["length-2"]
    XCTAssertEqual(first.frame.minX, second.frame.minX, accuracy: 1)
    XCTAssertGreaterThan(first.frame.minX, app.staticTexts["name-1"].frame.maxX)
  }

  /// Verifies a declared column width is the rendered width, with no gap between columns.
  func testColumnWidthsMatchTheDeclaredWidths() throws {
    try launch(scenario: "table")

    let table = app.tables["sections"]
    XCTAssertTrue(table.waitToExist(timeout: 2))
    let name = table.buttons["Name"]
    let length = table.buttons["Length"]
    XCTAssertEqual(name.frame.width, 200, accuracy: 1)
    XCTAssertGreaterThanOrEqual(length.frame.width, 119)
    // Each column starts where the previous one ends, so the widths add up.
    XCTAssertEqual(length.frame.minX, name.frame.maxX, accuracy: 1)
    // Legacy scroll bars may take part of the width, so only the window is an upper bound.
    let window = app.windows["main-window"]
    XCTAssertLessThanOrEqual(length.frame.maxX, window.frame.maxX + 1)
  }

  /// Returns the horizontal range of drawn text in `element`, as fractions of its width.
  private func inkRange(_ element: XCUIElement) -> ClosedRange<CGFloat>? {
    guard let data = element.screenshot().image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: data),
      let background = bitmap.colorAt(x: 1, y: 1)?.usingColorSpace(.sRGB)
    else {
      return nil
    }
    var columns: [Int] = []
    for x in 0..<bitmap.pixelsWide {
      for y in 0..<bitmap.pixelsHigh {
        guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
        if abs(color.brightnessComponent - background.brightnessComponent) > 0.3 {
          columns.append(x)
          break
        }
      }
    }
    guard let first = columns.first, let last = columns.last else { return nil }
    let width = CGFloat(bitmap.pixelsWide)
    return CGFloat(first) / width...CGFloat(last) / width
  }

  /// Verifies a trailing column draws its text cells at the right edge, while others stay leading.
  func testColumnAlignmentAppliesToTextCells() throws {
    try launch(scenario: "table")

    let length = app.staticTexts["length-1"]
    XCTAssertTrue(length.waitToExist(timeout: 2))
    let trailing = try XCTUnwrap(inkRange(length))
    XCTAssertGreaterThan(trailing.lowerBound, 0.5, "\(trailing)")
    XCTAssertGreaterThan(trailing.upperBound, 0.8, "\(trailing)")
    let leading = try XCTUnwrap(inkRange(app.staticTexts["name-1"]))
    XCTAssertLessThan(leading.lowerBound, 0.2, "\(leading)")
  }

  /// Verifies clicking a row writes its ID to the bound Store.
  func testClickingARowSelectsIt() throws {
    try launch(scenario: "table")

    let selection = app.staticTexts["selection"]
    app.staticTexts["name-3"].click()
    XCTAssertTrue(waitForValue(selection, "Selected: section-3"))
  }

  /// Verifies a selection set by the app scrolls its row into view.
  func testProgrammaticSelectionScrollsToTheRow() throws {
    try launch(scenario: "table")

    let row = app.staticTexts["name-40"]
    XCTAssertFalse(row.exists && row.isHittable)
    app.buttons["select-40"].click()
    XCTAssertTrue(waitForValue(app.staticTexts["selection"], "Selected: section-40"))
    XCTAssertTrue(row.waitToBeHittable(timeout: 2))
  }
}
