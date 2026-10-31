// The AppKit test runs only on macOS; other platforms build an empty binary.
#[cfg(target_os = "macos")]
include!("macos/title_bar.rs");

#[cfg(not(target_os = "macos"))]
fn main() {}
