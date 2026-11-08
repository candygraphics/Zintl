mod launch;
mod scenarios;

use zintl_ui_desktop::AppError;

use launch::LaunchOptions;
use scenarios::Scenarios;

fn main() -> Result<(), AppError> {
    let options = LaunchOptions::read();
    // Records panics, including those during process teardown that abort.
    if let Some(path) = options.variable("ZINTL_UI_TEST_PANIC_MARKER") {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = std::fs::write(&path, info.to_string());
            previous(info);
        }));
    }
    let scenario = options
        .scenario
        .clone()
        .unwrap_or_else(|| "text-field".to_owned());

    let Some(result) = Scenarios::run(&scenario, &options) else {
        eprintln!("unknown UI test scenario: {scenario}");
        std::process::exit(2);
    };
    result?;

    if let Some(path) = options.variable("ZINTL_UI_TEST_EXIT_MARKER") {
        std::fs::write(path, "ok").expect("the UI test exit marker must be writable");
    }
    Ok(())
}
