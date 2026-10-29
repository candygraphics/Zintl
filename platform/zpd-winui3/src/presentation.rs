//! Flyouts, content dialogs, and file pickers.

use std::cell::RefCell;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use super::{
    AsElement, Error, Result, Window, abort_on_panic, check, ffi, invoke_unit, release_callback,
};

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    Bottom = 0,
    Top = 1,
    Left = 2,
    Right = 3,
}

/// A light-dismiss flyout showing one content element.
pub struct Popover {
    raw: NonNull<ffi::Popover>,
    _main_thread: PhantomData<Rc<()>>,
}

impl Popover {
    /// Creates a flyout for `content`; `closed` runs whenever it hides.
    pub fn new<F>(content: &impl AsElement, closed: F) -> Result<Self>
    where
        F: FnMut() + 'static,
    {
        let callback = Rc::into_raw(Rc::new(RefCell::new(closed)));
        // SAFETY: Native code takes ownership of one Rc strong reference.
        let raw = unsafe {
            ffi::zpd_winui3_popover_create(
                content.as_element().as_ptr(),
                callback.cast(),
                invoke_unit::<F>,
                release_callback::<F>,
            )
        };
        Ok(Self {
            raw: NonNull::new(raw).ok_or(Error(-1))?,
            _main_thread: PhantomData,
        })
    }

    pub fn show(&self, anchor: &impl AsElement, placement: Placement) -> Result<()> {
        // SAFETY: Both handles are live on the UI thread.
        check(unsafe {
            ffi::zpd_winui3_popover_show(
                self.raw.as_ptr(),
                anchor.as_element().as_ptr(),
                placement as i32,
            )
        })
    }

    pub fn hide(&self) -> Result<()> {
        // SAFETY: The popover is live on the UI thread.
        check(unsafe { ffi::zpd_winui3_popover_hide(self.raw.as_ptr()) })
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        // SAFETY: The popover is live on the UI thread.
        unsafe { ffi::zpd_winui3_popover_is_open(self.raw.as_ptr()) }
    }
}

impl Drop for Popover {
    fn drop(&mut self) {
        // SAFETY: This owned wrapper is released exactly once on the UI thread.
        unsafe { ffi::zpd_winui3_popover_release(self.raw.as_ptr()) };
    }
}

/// Which [`DialogConfig`] button the user chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogResult {
    /// The dialog was hidden without a choice.
    None,
    Primary,
    Secondary,
    Close,
}

/// A `ContentDialog`; empty button titles omit the button.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogConfig {
    pub title: String,
    pub message: Option<String>,
    pub primary: String,
    pub secondary: String,
    pub close: String,
    /// The button that answers Enter.
    pub default_button: Option<DialogResult>,
}

/// An open `ContentDialog`; dropping it hides the dialog without reporting.
pub struct Dialog {
    raw: NonNull<ffi::Dialog>,
    _main_thread: PhantomData<Rc<()>>,
}

impl Drop for Dialog {
    fn drop(&mut self) {
        // SAFETY: This owned wrapper is released exactly once on the UI thread.
        unsafe { ffi::zpd_winui3_dialog_release(self.raw.as_ptr()) };
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileDialogMode {
    #[default]
    Open = 0,
    Save = 1,
    Folder = 2,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDialogOptions {
    pub mode: FileDialogMode,
    pub multiple: bool,
    pub file_name: Option<String>,
    pub commit_text: Option<String>,
    /// Extensions without dots; empty allows every type.
    pub extensions: Vec<String>,
}

/// Owns a one-shot callback for a native completion.
struct Once<F>(RefCell<Option<F>>);

impl<F> Once<F> {
    fn into_raw(callback: F) -> *const c_void {
        Box::into_raw(Box::new(Self(RefCell::new(Some(callback))))).cast()
    }

    /// # Safety
    /// `data` must come from [`Once::into_raw`] with the same `F` and still be live.
    unsafe fn take(data: *const c_void) -> Option<F> {
        // SAFETY: The caller guarantees `data` points to a live `Once<F>`.
        unsafe { &*data.cast::<Self>() }.0.borrow_mut().take()
    }

    unsafe extern "C" fn release(data: *const c_void) {
        if !data.is_null() {
            // SAFETY: Native code releases the one Box transferred to it exactly once.
            unsafe { drop(Box::from_raw(data.cast_mut().cast::<Self>())) };
        }
    }
}

impl Window {
    /// Shows a dialog; `completed` runs once on the UI thread with the choice.
    pub fn show_dialog<F>(&self, config: &DialogConfig, completed: F) -> Result<Dialog>
    where
        F: FnOnce(DialogResult) + 'static,
    {
        unsafe extern "C" fn invoke<F: FnOnce(DialogResult)>(data: *const c_void, result: i32) {
            abort_on_panic(|| {
                let result = match result {
                    1 => DialogResult::Primary,
                    2 => DialogResult::Secondary,
                    3 => DialogResult::Close,
                    _ => DialogResult::None,
                };
                // SAFETY: Native code passes the data installed below while it is live.
                if let Some(completed) = unsafe { Once::<F>::take(data) } {
                    completed(result);
                }
            });
        }

        let default_button = match config.default_button {
            Some(DialogResult::Primary) => 1,
            Some(DialogResult::Secondary) => 2,
            Some(DialogResult::Close) => 3,
            Some(DialogResult::None) | None => 0,
        };
        let data = Once::into_raw(completed);
        // SAFETY: Native code owns `data` and borrows the strings for this call.
        let raw = unsafe {
            ffi::zpd_winui3_dialog_show(
                self.raw.as_ptr(),
                ffi::StringRef::new(&config.title),
                ffi::StringRef::new(config.message.as_deref().unwrap_or("")),
                ffi::StringRef::new(&config.primary),
                ffi::StringRef::new(&config.secondary),
                ffi::StringRef::new(&config.close),
                default_button,
                data,
                invoke::<F>,
                Once::<F>::release,
            )
        };
        Ok(Dialog {
            raw: NonNull::new(raw).ok_or(Error(-1))?,
            _main_thread: PhantomData,
        })
    }

    /// Shows a file picker; `completed` receives the chosen paths, or `None`
    /// when the user cancelled.
    pub fn show_file_dialog<F>(&self, options: &FileDialogOptions, completed: F) -> Result<()>
    where
        F: FnOnce(Option<Vec<String>>) + 'static,
    {
        unsafe extern "C" fn invoke<F: FnOnce(Option<Vec<String>>)>(
            data: *const c_void,
            picked: bool,
            paths: *const ffi::StringRef,
            length: usize,
        ) {
            abort_on_panic(|| {
                let paths = picked.then(|| {
                    // SAFETY: Native code passes `length` valid UTF-8 paths for this call.
                    unsafe { std::slice::from_raw_parts(paths, length) }
                        .iter()
                        // SAFETY: Each path buffer is valid for this call.
                        .map(|path| unsafe { super::string_from_ref(*path) })
                        .collect()
                });
                // SAFETY: Native code passes the data installed below while it is live.
                if let Some(completed) = unsafe { Once::<F>::take(data) } {
                    completed(paths);
                }
            });
        }

        let extensions: Vec<_> = options
            .extensions
            .iter()
            .map(|extension| ffi::StringRef::new(extension))
            .collect();
        let raw_options = ffi::FileDialogOptions {
            mode: options.mode as i32,
            multiple: options.multiple,
            file_name: ffi::StringRef::new(options.file_name.as_deref().unwrap_or("")),
            commit_text: ffi::StringRef::new(options.commit_text.as_deref().unwrap_or("")),
            extensions: extensions.as_ptr(),
            extensions_length: extensions.len(),
        };
        // SAFETY: Native code owns the callback data and borrows the options for this call.
        check(unsafe {
            ffi::zpd_winui3_file_dialog_show(
                self.raw.as_ptr(),
                &raw const raw_options,
                Once::into_raw(completed),
                invoke::<F>,
                Once::<F>::release,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Runs the native completion protocol twice and reports which calls ran.
    fn complete_twice<F: FnOnce()>(callback: F) -> (bool, bool) {
        let data = Once::into_raw(callback);
        // SAFETY: `data` was created above for `F` and stays live until released.
        let first = unsafe { Once::<F>::take(data) }.map(|callback| callback());
        // SAFETY: As above; the slot is now empty.
        let second = unsafe { Once::<F>::take(data) };
        // SAFETY: The test releases its single allocation exactly once.
        unsafe { Once::<F>::release(data) };
        (first.is_some(), second.is_some())
    }

    #[test]
    fn one_shot_callbacks_run_once() {
        // Verifies a dialog or picker completion cannot run twice.
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        assert_eq!(
            complete_twice(move || counter.set(counter.get() + 1)),
            (true, false)
        );
        assert_eq!(calls.get(), 1);
    }
}
