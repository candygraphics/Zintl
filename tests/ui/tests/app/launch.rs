use std::collections::HashMap;
use std::env;
use std::process::Command;

/// The scenario and variables a UI test launched the app with.
///
/// A sandboxed test runner launches apps through `NSWorkspace`, which drops
/// arguments and the environment, so the runner also writes them to the ruler
/// pasteboard as `scenario=<name>` and `env:<NAME>=<value>` lines.
pub struct LaunchOptions {
    pub scenario: Option<String>,
    variables: HashMap<String, String>,
}

impl LaunchOptions {
    pub fn read() -> Self {
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

    pub fn variable(&self, name: &str) -> Option<String> {
        env::var(name)
            .ok()
            .or_else(|| self.variables.get(name).cloned())
    }
}
