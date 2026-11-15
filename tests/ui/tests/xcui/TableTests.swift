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

  /// Verifies double-clicking a row and pressing Return on the selected row activate it.
  func testDoubleClickAndReturnActivateRows() throws {
    try launch(scenario: "table")

    let third = app.staticTexts["name-3"]
    XCTAssertTrue(third.waitToExist(timeout: 2))
    third.doubleClick()
    assertLabel("activated", "Activated: section-3")

    app.staticTexts["name-5"].click()
    assertLabel("selection", "Selected: section-5")
    app.typeKey(.return, modifierFlags: [])
    assertLabel("activated", "Activated: section-5")
  }

  /// Verifies Delete reports the selected row, and does nothing without a selection.
  func testDeleteKeyReportsTheSelectedRow() throws {
    try launch(scenario: "table")

    let fourth = app.staticTexts["name-4"]
    XCTAssertTrue(fourth.waitToExist(timeout: 2))
    fourth.click()
    assertLabel("selection", "Selected: section-4")
    app.typeKey(.delete, modifierFlags: [])
    assertLabel("deleted", "Deleted: section-4")

    // With the selection cleared by the app, Delete in the focused table reports nothing.
    app.buttons["clear-selection"].click()
    assertLabel("selection", "Selected: none")
    app.typeKey(.delete, modifierFlags: [])
    assertLabel("deleted", "Deleted: section-4")
  }

  /// Verifies Delete in a focused text field edits its text instead of reaching the table.
  func testDeleteInATextFieldStaysInTheField() throws {
    try launch(scenario: "table")

    app.staticTexts["name-4"].click()
    assertLabel("selection", "Selected: section-4")
    let note = app.textFields["note"]
    note.click()
    note.typeKey(.end, modifierFlags: [])
    note.typeKey(.delete, modifierFlags: [])
    XCTAssertTrue(waitForValue(note, "draf"))
    assertLabel("deleted", "Deleted: ")
  }

  /// Verifies right-clicking a row shows that row's menu, and rows without one show none.
  func testRowContextMenusRunTheirItems() throws {
    try launch(scenario: "table")

    let second = app.staticTexts["name-2"]
    XCTAssertTrue(second.waitToExist(timeout: 2))
    second.rightClick()
    let archive = app.menuItems["Archive Section 2"]
    XCTAssertTrue(archive.waitToExist(timeout: 2))
    // Only the clicked row's items are shown.
    XCTAssertFalse(app.menuItems["Archive Section 3"].exists)
    archive.click()
    assertLabel("log", "Log: Archive Section 2")
    // Right-clicking does not select the row.
    assertLabel("selection", "Selected: none")

    app.staticTexts["name-7"].rightClick()
    let items = app.menuItems.matching(NSPredicate(format: "title BEGINSWITH 'Archive'"))
    XCTAssertFalse(waitUntil(timeout: 1) { items.count > 0 })
  }

  /// Returns whether the table row for `section` is selected.
  private func isRowSelected(_ section: Int) -> Bool {
    app.tables["sections"].tableRows.element(boundBy: section - 1).isSelected
  }

  /// Verifies should_select can keep the current row selected for clicks and arrow keys.
  func testRejectedSelectionKeepsTheCurrentRow() throws {
    try launch(scenario: "table")

    app.staticTexts["name-5"].click()
    assertLabel("selection", "Selected: section-5")
    let lock = app.checkBoxes["lock-selection"]
    lock.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
    XCTAssertTrue(waitUntil { lock.value as? Int == 1 })

    app.staticTexts["name-3"].click()
    XCTAssertTrue(waitUntil { self.isRowSelected(5) }, "section 5 lost its selection")
    XCTAssertFalse(isRowSelected(3))
    assertLabel("selection", "Selected: section-5")

    app.typeKey(.downArrow, modifierFlags: [])
    XCTAssertTrue(waitUntil { self.isRowSelected(5) && !self.isRowSelected(6) })
    assertLabel("selection", "Selected: section-5")

    // Unlocked, clicking selects as before.
    lock.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
    app.staticTexts["name-3"].click()
    assertLabel("selection", "Selected: section-3")
  }

  /// Returns the ID of the name cell in the first table row.
  private func firstRowName() -> String {
    app.tables["sections"].tableRows.element(boundBy: 0).staticTexts.element(boundBy: 0).identifier
  }

  /// Verifies clicking the Name header reports the order, which the app's rows follow.
  func testClickingTheHeaderSortsTheRows() throws {
    try launch(scenario: "table")

    let header = app.tables["sections"].buttons["Name"]
    XCTAssertTrue(header.waitToExist(timeout: 2))
    header.click()
    assertLabel("sort", "Sort: name ascending")
    XCTAssertTrue(waitUntil { self.firstRowName() == "name-1" }, firstRowName())

    header.click()
    assertLabel("sort", "Sort: name descending")
    XCTAssertTrue(waitUntil { self.firstRowName() == "name-50" }, firstRowName())
  }

  /// Verifies an order set by the app reorders the rows.
  func testStoreChangeSortsTheRows() throws {
    try launch(scenario: "table")

    XCTAssertTrue(app.staticTexts["name-1"].waitToExist(timeout: 2))
    app.buttons["sort-descending"].click()
    assertLabel("sort", "Sort: name descending")
    XCTAssertTrue(waitUntil { self.firstRowName() == "name-50" }, firstRowName())
    // Clicking the header again continues from the app's order.
    app.tables["sections"].buttons["Name"].click()
    assertLabel("sort", "Sort: name ascending")
  }

  /// Asserts that every column header lies inside the window.
  private func assertHeadersInsideWindow(file: StaticString = #filePath, line: UInt = #line) {
    let window = app.windows["main-window"].frame
    for title in ["Name", "Length", "Col A", "Col B", "Col C", "Col D"] {
      let header = app.tables["sections"].buttons[title].frame
      XCTAssertGreaterThanOrEqual(header.minX, window.minX - 1, title, file: file, line: line)
      XCTAssertLessThanOrEqual(header.maxX, window.maxX + 1, title, file: file, line: line)
    }
  }

  /// Verifies added columns shrink to fit the table instead of running past its edge.
  func testAddedColumnsFitTheTableWidth() throws {
    try launch(scenario: "table")

    let more = app.checkBoxes["more-columns"]
    XCTAssertTrue(more.waitToExist(timeout: 2))
    more.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
    XCTAssertTrue(app.tables["sections"].buttons["Col D"].waitToExist(timeout: 2))
    assertHeadersInsideWindow()

    // A narrower window keeps every header visible.
    let window = app.windows["main-window"]
    let corner = window.coordinate(withNormalizedOffset: CGVector(dx: 0.99, dy: 0.99))
    let width = window.frame.width
    corner.click(forDuration: 0.2, thenDragTo: corner.withOffset(CGVector(dx: -80, dy: 0)))
    XCTAssertTrue(waitUntil { window.frame.width < width - 40 })
    assertHeadersInsideWindow()
  }

  /// Verifies an empty table shows its placeholder inside its frame, below visible headers.
  func testEmptyTableShowsItsPlaceholder() throws {
    try launch(scenario: "table")

    let clear = app.checkBoxes["clear-rows"]
    XCTAssertTrue(clear.waitToExist(timeout: 2))
    XCTAssertFalse(app.staticTexts["empty-placeholder"].exists)
    clear.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()

    let placeholder = app.staticTexts["empty-placeholder"]
    XCTAssertTrue(placeholder.waitToExist(timeout: 2))
    let table = app.tables["sections"]
    XCTAssertTrue(table.frame.contains(placeholder.frame), "\(placeholder.frame) \(table.frame)")
    XCTAssertTrue(table.buttons["Name"].exists)

    clear.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
    XCTAssertTrue(app.staticTexts["name-1"].waitToExist(timeout: 2))
    XCTAssertTrue(waitUntil { !placeholder.exists })
  }
}
