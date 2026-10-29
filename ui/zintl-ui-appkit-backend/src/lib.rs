//! AppKit renderer for Zintl UI trees.

#![cfg(target_os = "macos")]

mod backend;
mod convert;

pub use backend::{AppError, AppKitBackend, run_composer};
// Backend modules address the shared descriptions through the crate root.
use zintl_ui_backend_core::*;
