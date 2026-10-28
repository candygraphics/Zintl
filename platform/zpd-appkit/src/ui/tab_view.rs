use std::cell::Cell;
use std::rc::Rc;

use crate::actor::{ActorRef, WindowEventKind};
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

/// A native `NSTabView` whose pages are caller-owned views.
pub struct TabView {
    view: ViewActor,
    items: std::cell::RefCell<Vec<ActorRef>>,
    // Programmatic selection must not be reported as a user change.
    quiet: Rc<Cell<bool>>,
}
impl TabView {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: NSTabView implements alloc/init.
        let native = unsafe {
            Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSTabView"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("init"), () => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let view = ViewActor::new(application, native);
        let emit = callback::emitter(view.actor());
        let quiet = Rc::new(Cell::new(false));
        let silenced = quiet.clone();
        callback::install_delegate(view.actor(), move |tab_view| {
            if silenced.get() {
                return;
            }
            // SAFETY: The delegate message passes the live NSTabView.
            let index = unsafe {
                let item =
                    zpd_objc::msg_send!(tab_view, zpd_objc::sel!("selectedTabViewItem"), () => Id);
                zpd_objc::msg_send!(tab_view, zpd_objc::sel!("indexOfTabViewItem:"), ((item): Id) => native::Integer)
            };
            if let Ok(index) = usize::try_from(index) {
                emit(WindowEventKind::SelectionChanged { index });
            }
        })?;
        Ok(Self {
            view,
            items: std::cell::RefCell::new(Vec::new()),
            quiet,
        })
    }

    /// Replaces the tabs with `(title, page)` pairs, reusing existing items.
    ///
    /// Each page view becomes owned by its tab item.
    pub fn set_tabs(&self, tabs: &[(&str, ViewRef<'_>)]) -> Result<(), UiError> {
        let tree = self.view.actor().tree_handle().ok_or(UiError::Closed)?;
        let mut items = self.items.borrow_mut();
        self.quiet.set(true);
        let result = (|| {
            while items.len() > tabs.len() {
                let item = items.pop().expect("length was checked");
                self.view.as_view().with(|tab_view| {
                    item.with(|item| {
                        // SAFETY: The item belongs to this live NSTabView.
                        unsafe {
                            zpd_objc::msg_send!(tab_view, zpd_objc::sel!("removeTabViewItem:"), ((item): Id) => ())
                        }
                    })
                })??;
                item.remove();
            }
            for (index, (title, page)) in tabs.iter().enumerate() {
                if index == items.len() {
                    // SAFETY: NSTabViewItem's designated initializer accepts a nil identifier.
                    let item = unsafe {
                        Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSTabViewItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithIdentifier:"), ((zpd_objc::NIL): Id) => Id))
                    }
                    .ok_or(UiError::NativeCreationFailed)?;
                    self.view.as_view().with(|tab_view| {
                        // SAFETY: NSTabView retains the item it adds.
                        unsafe {
                            zpd_objc::msg_send!(tab_view, zpd_objc::sel!("addTabViewItem:"), ((item.as_ptr()): Id) => ())
                        }
                    })?;
                    items.push(tree.insert_child(self.view.actor(), item)?);
                }
                let item = &items[index];
                let label = native::nsstring(title);
                item.with(|item| {
                    page.with(|page| {
                        // SAFETY: Both objects are live; the item retains its page view.
                        unsafe {
                            zpd_objc::msg_send!(item, zpd_objc::sel!("setLabel:"), ((label.as_ptr()): Id) => ());
                            if zpd_objc::msg_send!(item, zpd_objc::sel!("view"), () => Id) != page {
                                zpd_objc::msg_send!(item, zpd_objc::sel!("setView:"), ((page): Id) => ());
                            }
                        }
                    })
                })??;
                tree.reparent(page.actor(), item)?;
            }
            Ok(())
        })();
        self.quiet.set(false);
        result
    }

    pub fn selected(&self) -> Result<Option<usize>, UiError> {
        self.view.as_view().with(|tab_view| {
            // SAFETY: The live view is an NSTabView.
            let index = unsafe {
                let item = zpd_objc::msg_send!(tab_view, zpd_objc::sel!("selectedTabViewItem"), () => Id);
                if item.is_null() {
                    -1
                } else {
                    zpd_objc::msg_send!(tab_view, zpd_objc::sel!("indexOfTabViewItem:"), ((item): Id) => native::Integer)
                }
            };
            usize::try_from(index).ok()
        })
    }

    pub fn select(&self, index: usize) -> Result<(), UiError> {
        if self.selected()? == Some(index) || index >= self.items.borrow().len() {
            return Ok(());
        }
        self.quiet.set(true);
        let result = self.view.as_view().with(|tab_view| {
            // SAFETY: The index addresses an existing tab item.
            unsafe {
                zpd_objc::msg_send!(tab_view, zpd_objc::sel!("selectTabViewItemAtIndex:"), ((index as native::Integer): native::Integer) => ())
            }
        });
        self.quiet.set(false);
        result
    }

    /// Returns the page area in tab view coordinates.
    pub fn content_rect(&self) -> Result<Rect, UiError> {
        let rect = self.view.as_view().with(|tab_view| {
            // SAFETY: The live view is an NSTabView returning an NSRect.
            unsafe {
                zpd_objc::msg_send!(tab_view, zpd_objc::sel!("contentRect"), () => native::Rect)
            }
        })?;
        Ok(rect.into())
    }
}
impl AsView for TabView {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
