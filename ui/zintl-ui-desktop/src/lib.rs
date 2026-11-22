//! Desktop apps: the app runner and the views it shows.
//!
//! `use zintl_ui_desktop::*;` covers ordinary apps. Menus, canvases, dialogs,
//! and the other areas below have their own modules.

mod app;

pub use app::*;
pub use zintl_ui::element::{ElementFactory, IntoElement};
pub use zintl_ui::store::{Local, Store, Trigger};
pub use zintl_ui::view::View;
pub use zintl_ui_layout::{Axis, CrossAxisAlignment, MainAxisDistribution, Size};
pub use zintl_ui_native::graphics::{AttributedText, Font, FontWeight};
pub use zintl_ui_native::{
    Attach, BarChart, Button, ButtonRole, ButtonStyle, ByItemKey, CalendarDate, ChartBar, Color,
    ColorWell, ComboBox, Context, Control, DatePicker, DatePickerStyle, DisclosureGroup, Grid,
    GridCell, GridRow, HStack, IntegerField, ItemIdFn, ItemIdSource, ItemKey, Layout, List,
    ListFactory, ListRow, ListSection, ListStyle, NumberField, OptionalDatePicker, PaletteColor,
    Picker, PickerStyle, Point, ProgressIndicator, Rect, RenderNode, ScrollAlignment, ScrollAxes,
    ScrollStatus, ScrollView, SearchField, Separator, Sidebar, SidebarItem, SidebarSection, Slider,
    SortOrder, Spacer, SplitView, SystemColor, Tab, TabView, Table, TableColumn, TableRow, Text,
    TextAlignment, TextColor, TextEditor, TextField, TitleBar, Toggle, ToggleStyle, Toolbar,
    ToolbarItem, UndoHandler, VStack, ViewId, Window, ZStack, graphics, list,
};

/// Menus, their items, and keyboard shortcuts.
pub mod menu {
    pub use zintl_ui_native::{
        EnglishMenuLocalizer, JapaneseMenuLocalizer, MainMenu, Menu, MenuItem, MenuLocalizer,
        Modifier, NamedKey, Shortcut, ShortcutKey, StandardAction, StandardMenu, StandardText,
    };
}

/// Views drawn by the app, and the input they receive.
pub mod canvas {
    pub use zintl_ui_native::{
        Canvas2D, Canvas2DHandler, CanvasEvent, CanvasEventContext, CanvasId, CanvasSetup, Cursor,
        Frame, GesturePhase, Invalidator, Key, KeyEvent, KeyPhase, MagnifyEvent, Modifiers,
        NamedKey, Palette, PointerButton, PointerEvent, PointerPhase, Redraw, RedrawPolicy,
        ScrollEvent,
    };
    #[cfg(feature = "wgpu")]
    pub use zintl_ui_native::{
        WgpuCanvas, WgpuCanvasHandler, WgpuContext, WgpuError, WgpuSetup, WgpuTarget, wgpu,
    };
}

/// What assistive apps are told about a view.
pub mod accessibility {
    pub use zintl_ui_native::{AccessibilityRole, AnnouncementPriority};
}

/// Alerts, panels, and popovers presented over a window.
pub mod dialog {
    pub use zintl_ui_native::{
        Alert, AlertButton, AlertStyle, ColorPanel, FileDialog, Popover, PopoverEdge,
    };
}

/// The pieces custom views are built from.
///
/// A view composed of other views takes the shared modifiers (`id`, sizes,
/// `context_menu`, `accessibility_*`) by implementing [`NativeView`]: keep a
/// [`ViewModifiers`] field, add [`modifiers!`] for its accessors, render the
/// root view's `native_element`, and implement `View` with [`native_view!`].
/// See [`NativeView`] for how modifiers layer and when it applies, and
/// `examples/custom_view.rs` for a complete view.
///
/// Apps cannot add native control kinds; a new control is a change to
/// `zintl-ui-backend-core` and every backend. Outside this crate, build
/// views by composing existing ones, or draw them with
/// [`Canvas2D`](crate::canvas::Canvas2D) and report to the app with
/// `CanvasEventContext::emit` and `on_action`.
pub mod render {
    pub use zintl_ui::element::{Element, ElementFactory, IntoElement, KeyedElement};
    pub use zintl_ui_layout::{ChildSizing, LayoutDimension, LayoutStyle};
    pub use zintl_ui_native::{
        Event, EventKind, LayoutOverrides, ListFactory, NativeView, RenderNode, TextEstimate,
        ViewKind, ViewModifiers, modifiers, native_view,
    };
}

/// Values shared with other threads.
pub mod thread {
    pub use zintl_ui_native::{Signal, StoreSender};
}

/// A headless harness for inspecting views in tests.
pub mod testing {
    pub use zintl_ui_native::testing::{Harness, RenderedNode};
}

#[cfg(test)]
mod tests {
    use crate::testing::Harness;
    use crate::*;

    #[test]
    fn glob_import_covers_an_ordinary_app() {
        // Verifies the crate root alone declares a window of text, fields, and buttons.
        let harness = Harness::new(Window::new("Zintl").size(Size::new(320.0, 240.0)).content(
            VStack::new(list![
                Text::new("Name").font(Font::system(13.0)),
                TextField::new().id("name"),
                HStack::new(list![Spacer::new(), Button::new("Save").enabled(false)]),
            ]),
        ));

        assert!(matches!(harness.tree().value, RenderNode::Window { .. }));
        assert!(harness.find("name").is_some());
    }

    #[test]
    fn feature_modules_hold_their_names() {
        // Verifies each feature module re-exports the names apps and custom views import from it.
        #[allow(unused_imports)]
        use crate::{
            accessibility::{AccessibilityRole, AnnouncementPriority},
            canvas::{Canvas2D, Canvas2DHandler, CanvasEvent, Frame, PointerEvent},
            dialog::{Alert, ColorPanel, FileDialog, Popover},
            menu::{MainMenu, Menu, MenuItem, NamedKey, Shortcut, StandardMenu},
            render::{
                Element, Event, NativeView, TextEstimate, ViewKind, ViewModifiers, modifiers,
                native_view,
            },
            testing::RenderedNode,
            thread::{Signal, StoreSender},
        };
    }

    #[test]
    fn menu_module_names_shortcut_keys() {
        // Verifies a named-key shortcut needs only the menu module, not canvas.
        use crate::menu::{MenuItem, NamedKey, Shortcut, ShortcutKey};

        let shortcut = Shortcut::command(NamedKey::Backspace);
        assert_eq!(shortcut.key, ShortcutKey::from(NamedKey::Backspace));
        MenuItem::new("Delete…").shortcut(shortcut);
    }
}
