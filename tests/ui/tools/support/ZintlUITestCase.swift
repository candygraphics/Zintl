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

  private var defaultAppPath: String {
    URL(fileURLWithPath: #filePath)
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .appendingPathComponent("target/ZintlUITestApp.app")
      .path
  }
}
