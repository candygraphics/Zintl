//! WinUI 3 renderer for Zintl UI trees.
//!
//! The backend reads the same [`RenderNode`](zintl_ui_backend_core::RenderNode)
//! descriptions as the AppKit backend and maps them to WinUI controls.

#[cfg(target_os = "windows")]
mod backend;

#[cfg(target_os = "windows")]
pub use backend::{AppError, WinUI3Backend, run_composer};
