import XCTest

@MainActor
class ZintlUITestCase: XCTestCase {
  private(set) var app: XCUIApplication!

  override func setUpWithError() throws {
    continueAfterFailure = false
  }

  func launch(
    scenario: String,
    environment: [String: String] = [:],
    file: StaticString = #filePath,
    line: UInt = #line
  ) throws {
    let path = ProcessInfo.processInfo.environment["ZINTL_UI_TEST_APP"] ?? defaultAppPath

    app = XCUIApplication(url: URL(fileURLWithPath: path))
    app.launchArguments = ["--scenario", scenario]
    app.launchEnvironment.merge(environment) { _, value in value }
    // A sandboxed runner drops launch arguments, so the app also reads them from here.
    let pasteboard = NSPasteboard(name: .ruler)
    let options = ["scenario=\(scenario)"] + environment.map { "env:\($0.key)=\($0.value)" }
    pasteboard.clearContents()
    pasteboard.setString(options.joined(separator: "\n"), forType: .string)
    addTeardownBlock { @MainActor in
      pasteboard.clearContents()
    }
    app.launch()
    let launchedApp = app!
    addTeardownBlock { @MainActor in
      launchedApp.terminate()
    }

    XCTAssertTrue(
      app.windows["main-window"].waitForExistence(timeout: 5),
      "The main window did not appear",
      file: file,
      line: line
    )
  }

  /// Waits until `element` reports `value`.
  func waitForValue(_ element: XCUIElement, _ value: String, timeout: TimeInterval = 2) -> Bool {
    let predicate = NSPredicate(format: "value == %@", value)
    let expectation = XCTNSPredicateExpectation(predicate: predicate, object: element)
    return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
  }

  /// Returns the element with the accessibility identifier `id`, whatever its type.
  func element(_ id: String) -> XCUIElement {
    app.descendants(matching: .any)[id]
  }

  /// Asserts that the label `id` shows `value`, waiting for the app to update it.
  ///
  /// Scenarios echo their Stores in labels, so this proves native input reached Rust.
  func assertLabel(
    _ id: String,
    _ value: String,
    timeout: TimeInterval = 2,
    file: StaticString = #filePath,
    line: UInt = #line
  ) {
    let label = app.staticTexts[id]
    XCTAssertTrue(
      waitForValue(label, value, timeout: timeout),
      "Label \(id) shows \(String(describing: label.value)) instead of \(value)",
      file: file,
      line: line
    )
  }

  /// Polls the UI until `condition` holds or `timeout` passes, and returns whether it held.
  func waitUntil(timeout: TimeInterval = 2, _ condition: () -> Bool) -> Bool {
    let deadline = Date().addingTimeInterval(timeout)
    while Date() < deadline {
      if condition() {
        return true
      }
      RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    return condition()
  }

  /// Presses `element` at the normalized point `start` and drags it by `offset` points.
  func drag(
    _ element: XCUIElement,
    from start: CGVector = CGVector(dx: 0.5, dy: 0.5),
    by offset: CGVector
  ) {
    let origin = element.coordinate(withNormalizedOffset: start)
    origin.press(forDuration: 0.2, thenDragTo: origin.withOffset(offset))
  }

  /// Counts the pixels in a screenshot of `element` whose sRGB color satisfies `matches`.
  func pixelCount(in element: XCUIElement, where matches: (NSColor) -> Bool) -> Int {
    guard let data = element.screenshot().image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: data)
    else {
      return 0
    }
    var count = 0
    for y in 0..<bitmap.pixelsHigh {
      for x in 0..<bitmap.pixelsWide {
        guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
        if matches(color) {
          count += 1
        }
      }
    }
    return count
  }

  /// Creates an empty directory for files the app writes; the test removes it afterwards.
  func makeTemporaryDirectory() throws -> URL {
    let directory = FileManager.default.temporaryDirectory
      .appendingPathComponent("zintl-ui-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    addTeardownBlock {
      try? FileManager.default.removeItem(at: directory)
    }
    return directory
  }

  private var defaultAppPath: String {
    URL(fileURLWithPath: #filePath)
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .appendingPathComponent("target/ZintlUITestApp.app")
      .path
  }
}
