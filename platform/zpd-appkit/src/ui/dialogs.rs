//! Window-modal sheets and the shared color panel.

use crate::actor::{ActorRef, EventRouteToken, WindowEventKind};
use crate::native;
use zpd_objc::block::StackBlock;
use zpd_objc::{Id, Strong};

use super::{Color, UiError, Window, callback};

pub use crate::model::{AlertButton, AlertButtonRole, AlertConfig, AlertStyle};

/// An `NSAlert` presented as a sheet on a Window.
///
/// The sheet emits [`WindowEventKind::AlertResponse`] once when it ends.
pub struct AlertSheet {
    actor: ActorRef,
    window: ActorRef,
}
impl AlertSheet {
    pub fn present(
        window: &Window<'_>,
        config: &AlertConfig,
        route: Option<EventRouteToken>,
    ) -> Result<Self, UiError> {
        let window_actor = window.actor_ref();
        let tree = window_actor.tree_handle().ok_or(UiError::Closed)?;
        let alert = native::alloc_init(zpd_objc::class!("NSAlert"));
        let title = native::nsstring(&config.title);
        let message = native::nsstring(config.message.as_deref().unwrap_or(""));
        // SAFETY: The retained NSAlert copies strings and owns the buttons it creates.
        unsafe {
            zpd_objc::msg_send!(alert.as_ptr(), zpd_objc::sel!("setMessageText:"), ((title.as_ptr()): Id) => ());
            zpd_objc::msg_send!(alert.as_ptr(), zpd_objc::sel!("setInformativeText:"), ((message.as_ptr()): Id) => ());
            zpd_objc::msg_send!(alert.as_ptr(), zpd_objc::sel!("setAlertStyle:"), ((config.style as u64): u64) => ());
            for button in &config.buttons {
                let title = native::nsstring(&button.title);
                let native_button = zpd_objc::msg_send!(alert.as_ptr(), zpd_objc::sel!("addButtonWithTitle:"), ((title.as_ptr()): Id) => Id);
                match button.role {
                    AlertButtonRole::Normal => {}
                    AlertButtonRole::Cancel => {
                        let escape = native::nsstring("\u{1b}");
                        zpd_objc::msg_send!(native_button, zpd_objc::sel!("setKeyEquivalent:"), ((escape.as_ptr()): Id) => ());
                    }
                    AlertButtonRole::Destructive => {
                        zpd_objc::msg_send!(native_button, zpd_objc::sel!("setHasDestructiveAction:"), ((true): bool) => ());
                    }
                }
            }
        }
        let actor = tree.insert_child(&window_actor, alert)?;
        actor.set_event_route(route)?;
        let emit = callback::emitter(&actor);
        let count = config.buttons.len();
        let completion = StackBlock::new(move |response: native::Integer| {
            let button = usize::try_from(response - native::NS_ALERT_FIRST_BUTTON_RETURN)
                .ok()
                .filter(|button| *button < count);
            emit(WindowEventKind::AlertResponse { button });
        });
        actor.with(|alert| {
            window_actor.with(|window| {
                // SAFETY: AppKit copies the completion block before this call returns.
                unsafe {
                    zpd_objc::msg_send!(alert, zpd_objc::sel!("beginSheetModalForWindow:completionHandler:"), ((window): Id, (completion.as_id()): Id) => ())
                }
            })
        })??;
        Ok(Self {
            actor,
            window: window_actor,
        })
    }
    pub fn set_event_route(&self, route: Option<EventRouteToken>) -> Result<(), UiError> {
        self.actor.set_event_route(route).map_err(Into::into)
    }
    /// Ends the sheet without choosing a button.
    pub fn dismiss(&self) {
        let _ = self.actor.with(|alert| {
            self.window.with(|window| {
                // SAFETY: Both objects are live; ending an absent sheet does nothing.
                unsafe {
                    let sheet = zpd_objc::msg_send!(alert, zpd_objc::sel!("window"), () => Id);
                    if zpd_objc::msg_send!(window, zpd_objc::sel!("attachedSheet"), () => Id) == sheet {
                        zpd_objc::msg_send!(window, zpd_objc::sel!("endSheet:returnCode:"), ((sheet): Id, (native::NS_MODAL_RESPONSE_ABORT): native::Integer) => ());
                    }
                }
            })
        });
    }
}
impl Drop for AlertSheet {
    fn drop(&mut self) {
        self.dismiss();
        self.actor.remove();
    }
}

pub use crate::model::{FileDialogConfig, FileDialogMode};

/// An `NSOpenPanel` or `NSSavePanel` presented as a sheet on a Window.
///
/// The sheet emits [`WindowEventKind::FileDialogCompleted`] once when it ends.
pub struct FileDialogSheet {
    actor: ActorRef,
}
impl FileDialogSheet {
    pub fn present(
        window: &Window<'_>,
        config: &FileDialogConfig,
        route: Option<EventRouteToken>,
    ) -> Result<Self, UiError> {
        let window_actor = window.actor_ref();
        let tree = window_actor.tree_handle().ok_or(UiError::Closed)?;
        let open = config.mode == FileDialogMode::Open;
        // SAFETY: The panel factories return autoreleased panels retained here.
        let panel = unsafe {
            Strong::retain(if open {
                zpd_objc::msg_send!(zpd_objc::class!("NSOpenPanel"), zpd_objc::sel!("openPanel"), () => Id)
            } else {
                zpd_objc::msg_send!(zpd_objc::class!("NSSavePanel"), zpd_objc::sel!("savePanel"), () => Id)
            })
        }
        .ok_or(UiError::NativeCreationFailed)?;
        // SAFETY: The retained panel accepts these NSSavePanel/NSOpenPanel settings.
        unsafe { Self::configure(panel.as_ptr(), config, open) };
        let actor = tree.insert_child(&window_actor, panel)?;
        actor.set_event_route(route)?;
        let emit = callback::emitter(&actor);
        let source = actor.clone();
        let completion = StackBlock::new(move |response: native::Integer| {
            let paths = (response == native::NS_MODAL_RESPONSE_OK).then(|| {
                source
                    .with(|panel| {
                        // SAFETY: The completed panel is live and its URLs are file URLs.
                        unsafe { Self::selected_paths(panel, open) }
                    })
                    .unwrap_or_default()
            });
            emit(WindowEventKind::FileDialogCompleted { paths });
        });
        actor.with(|panel| {
            window_actor.with(|window| {
                // SAFETY: AppKit copies the completion block before this call returns.
                unsafe {
                    zpd_objc::msg_send!(panel, zpd_objc::sel!("beginSheetModalForWindow:completionHandler:"), ((window): Id, (completion.as_id()): Id) => ())
                }
            })
        })??;
        Ok(Self { actor })
    }

    /// # Safety
    /// `panel` must be a live `NSOpenPanel` when `open` is true, else an `NSSavePanel`.
    unsafe fn configure(panel: Id, config: &FileDialogConfig, open: bool) {
        let set_string = |selector: zpd_objc::Sel, value: &Option<String>| {
            if let Some(value) = value {
                let value = native::nsstring(value);
                // SAFETY: Each selector is an NSSavePanel string setter.
                unsafe { zpd_objc::msg_send!(panel, selector, ((value.as_ptr()): Id) => ()) };
            }
        };
        set_string(zpd_objc::sel!("setTitle:"), &config.title);
        set_string(zpd_objc::sel!("setMessage:"), &config.message);
        set_string(zpd_objc::sel!("setPrompt:"), &config.prompt);
        set_string(
            zpd_objc::sel!("setNameFieldStringValue:"),
            &config.file_name,
        );
        // SAFETY: Guaranteed by the caller; arrays and URLs are retained by the panel.
        unsafe {
            zpd_objc::msg_send!(panel, zpd_objc::sel!("setCanCreateDirectories:"), ((config.can_create_directories): bool) => ());
            if let Some(directory) = &config.directory {
                let path = native::nsstring(directory);
                let url = zpd_objc::msg_send!(zpd_objc::class!("NSURL"), zpd_objc::sel!("fileURLWithPath:isDirectory:"), ((path.as_ptr()): Id, (true): bool) => Id);
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setDirectoryURL:"), ((url): Id) => ());
            }
            if !config.allowed_extensions.is_empty() {
                let types = native::alloc_init(zpd_objc::class!("NSMutableArray"));
                for extension in &config.allowed_extensions {
                    let extension = native::nsstring(extension);
                    let content_type = zpd_objc::msg_send!(zpd_objc::class!("UTType"), zpd_objc::sel!("typeWithFilenameExtension:"), ((extension.as_ptr()): Id) => Id);
                    if !content_type.is_null() {
                        zpd_objc::msg_send!(types.as_ptr(), zpd_objc::sel!("addObject:"), ((content_type): Id) => ());
                    }
                }
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setAllowedContentTypes:"), ((types.as_ptr()): Id) => ());
            }
            if open {
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setCanChooseFiles:"), ((config.can_choose_files): bool) => ());
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setCanChooseDirectories:"), ((config.can_choose_directories): bool) => ());
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setAllowsMultipleSelection:"), ((config.allows_multiple): bool) => ());
            }
        }
    }

    /// # Safety
    /// Same contract as [`Self::configure`].
    unsafe fn selected_paths(panel: Id, open: bool) -> Vec<String> {
        // SAFETY: Guaranteed by the caller; URL accessors return autoreleased objects.
        unsafe {
            let path = |url: Id| {
                native::rust_string(zpd_objc::msg_send!(url, zpd_objc::sel!("path"), () => Id))
            };
            if open {
                let urls = zpd_objc::msg_send!(panel, zpd_objc::sel!("URLs"), () => Id);
                let count = zpd_objc::msg_send!(urls, zpd_objc::sel!("count"), () => u64);
                (0..count)
                    .map(|index| path(zpd_objc::msg_send!(urls, zpd_objc::sel!("objectAtIndex:"), ((index): u64) => Id)))
                    .collect()
            } else {
                let url = zpd_objc::msg_send!(panel, zpd_objc::sel!("URL"), () => Id);
                if url.is_null() {
                    Vec::new()
                } else {
                    vec![path(url)]
                }
            }
        }
    }

    pub fn set_event_route(&self, route: Option<EventRouteToken>) -> Result<(), UiError> {
        self.actor.set_event_route(route).map_err(Into::into)
    }
    /// Cancels the sheet if it is still open.
    pub fn dismiss(&self) {
        let _ = self.actor.with(|panel| {
            // SAFETY: The panel is live; cancelling a closed panel does nothing.
            unsafe {
                if zpd_objc::msg_send!(panel, zpd_objc::sel!("isVisible"), () => bool) {
                    zpd_objc::msg_send!(panel, zpd_objc::sel!("cancel:"), ((zpd_objc::NIL): Id) => ());
                }
            }
        });
    }
}
impl Drop for FileDialogSheet {
    fn drop(&mut self) {
        self.dismiss();
        self.actor.remove();
    }
}

/// Connects the shared `NSColorPanel` to a Window until dropped.
///
/// Only one session should be presented at a time because AppKit shares the
/// panel across the application.
pub struct ColorPanelSession {
    actor: ActorRef,
}
impl ColorPanelSession {
    pub fn present(
        window: &Window<'_>,
        color: Color,
        shows_alpha: bool,
        route: Option<EventRouteToken>,
    ) -> Result<Self, UiError> {
        let window_actor = window.actor_ref();
        let tree = window_actor.tree_handle().ok_or(UiError::Closed)?;
        // SAFETY: sharedColorPanel returns the process-wide panel, retained here.
        let panel = unsafe {
            Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSColorPanel"), zpd_objc::sel!("sharedColorPanel"), () => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let actor = tree.insert_child(&window_actor, panel)?;
        actor.set_event_route(route)?;
        let emit = callback::emitter(&actor);
        callback::install_action(&actor, move |panel| {
            // SAFETY: The action sender is the live color panel.
            let color = unsafe {
                Color::from_native(zpd_objc::msg_send!(panel, zpd_objc::sel!("color"), () => Id))
            };
            emit(WindowEventKind::ColorChanged { color });
        })?;
        let emit = callback::emitter(&actor);
        callback::install_delegate(&actor, move |_| emit(WindowEventKind::ColorPanelClosed))?;
        tree.add_teardown(&actor, |panel| {
            // SAFETY: Hiding the shared panel ends this session's presentation.
            unsafe {
                zpd_objc::msg_send!(panel, zpd_objc::sel!("orderOut:"), ((zpd_objc::NIL): Id) => ())
            }
        })?;
        let session = Self { actor };
        session.set_color(color)?;
        session.actor.with(|panel| {
            // SAFETY: The panel is live; continuous mode reports changes while dragging.
            unsafe {
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setContinuous:"), ((true): bool) => ());
                zpd_objc::msg_send!(panel, zpd_objc::sel!("setShowsAlpha:"), ((shows_alpha): bool) => ());
                zpd_objc::msg_send!(panel, zpd_objc::sel!("orderFront:"), ((zpd_objc::NIL): Id) => ());
            }
        })?;
        Ok(session)
    }
    pub fn set_event_route(&self, route: Option<EventRouteToken>) -> Result<(), UiError> {
        self.actor.set_event_route(route).map_err(Into::into)
    }
    pub fn color(&self) -> Result<Color, UiError> {
        self.actor
            .with(|panel| {
                // SAFETY: The Actor holds the live shared color panel.
                unsafe {
                    Color::from_native(
                        zpd_objc::msg_send!(panel, zpd_objc::sel!("color"), () => Id),
                    )
                }
            })
            .map_err(Into::into)
    }
    pub fn set_color(&self, color: Color) -> Result<(), UiError> {
        self.actor
            .with(|panel| {
                // SAFETY: The Actor holds the live shared color panel.
                unsafe {
                    zpd_objc::msg_send!(panel, zpd_objc::sel!("setColor:"), ((color.native()): Id) => ())
                }
            })
            .map_err(Into::into)
    }
}
impl Drop for ColorPanelSession {
    fn drop(&mut self) {
        self.actor.remove();
    }
}
