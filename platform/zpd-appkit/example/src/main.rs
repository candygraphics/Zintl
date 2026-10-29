// The AppKit example runs only on macOS; other platforms build an empty binary.
#[cfg(target_os = "macos")]
include!("macos.rs");

#[cfg(not(target_os = "macos"))]
fn main() {}
