mod scenarios;

use std::collections::HashMap;
use std::env;
use std::process::Command;

use zintl_ui_desktop::{App, AppError};

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

    match scenario.as_str() {
        "arrangements" => App::new(scenarios::arrangements::MainView::new()).run()?,
        "canvas-action" => App::new(scenarios::canvas_action::MainView::new()).run()?,
        "controls" => App::new(scenarios::controls::MainView::new()).run()?,
        "document" => App::new(scenarios::document::MainView::new()).run()?,
        "store-sender" => App::new(scenarios::store_sender::MainView::new()).run()?,
        "table" => App::new(scenarios::table::MainView::new()).run()?,
        "truncation" => App::new(scenarios::truncation::MainView).run()?,
        "text-commit" => App::new(scenarios::text_commit::MainView::new()).run()?,
        "text-field" => App::new(scenarios::text_field::MainView::new()).run()?,
        "hstack" => App::new(scenarios::hstack::MainView).run()?,
        "full-width-stack" => App::new(scenarios::full_width_stack::MainView).run()?,
        "full-size-sidebar" => App::new(scenarios::full_size_sidebar::MainView::default()).run()?,
        "overflow" => App::new(scenarios::overflow::MainView).run()?,
        "perform-main" => App::new(scenarios::perform_main::MainView::new()).run()?,
        "scroll-status" => App::new(scenarios::scroll_status::MainView::default()).run()?,
        "window-close" => App::new(scenarios::window_close::MainView).run()?,
        "wgpu-first-frame" => App::new(scenarios::wgpu_first_frame::MainView::new()).run()?,
        unknown => {
            eprintln!("unknown UI test scenario: {unknown}");
            std::process::exit(2);
        }
    }

    if let Some(path) = options.variable("ZINTL_UI_TEST_EXIT_MARKER") {
        std::fs::write(path, "ok").expect("the UI test exit marker must be writable");
    }
    Ok(())
}

/// The scenario and variables a UI test launched the app with.
///
/// A sandboxed test runner launches apps through `NSWorkspace`, which drops
/// arguments and the environment, so the runner also writes them to the ruler
/// pasteboard as `scenario=<name>` and `env:<NAME>=<value>` lines.
struct LaunchOptions {
    scenario: Option<String>,
    variables: HashMap<String, String>,
}

impl LaunchOptions {
    fn read() -> Self {
        let mut options = Self {
            scenario: Self::scenario_argument(),
            variables: HashMap::new(),
        };
        if options.scenario.is_none() {
            options.read_pasteboard();
        }
        options
    }

    fn scenario_argument() -> Option<String> {
        let mut arguments = env::args().skip(1);
        while let Some(argument) = arguments.next() {
            if argument == "--scenario" {
                return arguments.next();
            }
        }
        None
    }

    fn read_pasteboard(&mut self) {
        let Ok(output) = Command::new("pbpaste").args(["-pboard", "ruler"]).output() else {
            return;
        };
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(scenario) = line.strip_prefix("scenario=") {
                self.scenario = Some(scenario.to_owned());
            } else if let Some((name, value)) = line
                .strip_prefix("env:")
                .and_then(|line| line.split_once('='))
            {
                self.variables.insert(name.to_owned(), value.to_owned());
            }
        }
    }

    fn variable(&self, name: &str) -> Option<String> {
        env::var(name)
            .ok()
            .or_else(|| self.variables.get(name).cloned())
    }
}
