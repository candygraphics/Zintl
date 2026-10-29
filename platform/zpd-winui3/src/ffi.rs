use std::ffi::c_void;

#[repr(C)]
pub struct AppContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct DispatcherQueue {
    _private: [u8; 0],
}

#[repr(C)]
pub struct DispatcherQueueSource {
    _private: [u8; 0],
}

#[repr(C)]
pub struct DispatcherQueueSignaler {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Window {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Element {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringRef {
    pub data: *const u8,
    pub length: usize,
}

impl StringRef {
    pub fn new(value: &str) -> Self {
        Self {
            data: value.as_ptr(),
            length: value.len(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Thickness {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GridLength {
    pub kind: i32,
    pub value: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MenuFlyoutItem {
    pub kind: i32,
    pub id: StringRef,
    pub title: StringRef,
    pub key: StringRef,
    pub modifiers: u32,
    pub enabled: bool,
    pub role: i32,
    pub children: *const MenuFlyoutItem,
    pub children_length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MenuBarItem {
    pub title: StringRef,
    pub items: *const MenuFlyoutItem,
    pub items_length: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Event {
    pub kind: i32,
    pub x: f64,
    pub y: f64,
    pub index: i64,
    pub flag: bool,
    pub text: StringRef,
    pub color: Color,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NavigationItem {
    pub header: bool,
    pub id: StringRef,
    pub title: StringRef,
    pub glyph: StringRef,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileDialogOptions {
    pub mode: i32,
    pub multiple: bool,
    pub file_name: StringRef,
    pub commit_text: StringRef,
    pub extensions: *const StringRef,
    pub extensions_length: usize,
}

#[repr(C)]
pub struct Popover {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Dialog {
    _private: [u8; 0],
}

pub type LaunchFn = unsafe extern "C" fn(*const AppContext, *const c_void);
pub type InvokeFn = unsafe extern "C" fn(*const c_void);
pub type StringFn = unsafe extern "C" fn(*const c_void, StringRef);
pub type ReleaseFn = unsafe extern "C" fn(*const c_void);
pub type EventFn = unsafe extern "C" fn(*const c_void, *const Event);
pub type IntFn = unsafe extern "C" fn(*const c_void, i32);
pub type PathsFn = unsafe extern "C" fn(*const c_void, bool, *const StringRef, usize);

unsafe extern "C" {
    pub fn zpd_winui3_application_run(
        data: *const c_void,
        launch: LaunchFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_app_dispatcher(context: *const AppContext) -> *mut DispatcherQueue;
    pub fn zpd_winui3_last_error() -> StringRef;
    pub fn zpd_winui3_application_exit() -> i32;
    pub fn zpd_winui3_dispatcher_clone(dispatcher: *const DispatcherQueue) -> *mut DispatcherQueue;
    pub fn zpd_winui3_dispatcher_release(dispatcher: *mut DispatcherQueue);
    pub fn zpd_winui3_dispatcher_try_enqueue(
        dispatcher: *const DispatcherQueue,
        priority: i32,
        data: *const c_void,
        invoke: InvokeFn,
        release: ReleaseFn,
    ) -> bool;
    pub fn zpd_winui3_dispatcher_source_create(
        dispatcher: *const DispatcherQueue,
        data: *const c_void,
        invoke: InvokeFn,
        release: ReleaseFn,
    ) -> *mut DispatcherQueueSource;
    pub fn zpd_winui3_dispatcher_source_release(source: *mut DispatcherQueueSource);
    pub fn zpd_winui3_dispatcher_source_signaler(
        source: *const DispatcherQueueSource,
    ) -> *mut DispatcherQueueSignaler;
    pub fn zpd_winui3_dispatcher_signaler_clone(
        signaler: *const DispatcherQueueSignaler,
    ) -> *mut DispatcherQueueSignaler;
    pub fn zpd_winui3_dispatcher_signaler_release(signaler: *mut DispatcherQueueSignaler);
    pub fn zpd_winui3_dispatcher_signaler_signal(signaler: *const DispatcherQueueSignaler) -> bool;
    pub fn zpd_winui3_window_release(window: *mut Window);
    pub fn zpd_winui3_window_set_title(window: *mut Window, title: StringRef) -> i32;
    pub fn zpd_winui3_window_resize(window: *mut Window, width: i32, height: i32) -> i32;
    pub fn zpd_winui3_window_activate(window: *mut Window) -> i32;
    pub fn zpd_winui3_window_close(window: *mut Window) -> i32;
    pub fn zpd_winui3_window_set_content(window: *mut Window, element: *const Element) -> i32;
    pub fn zpd_winui3_window_extend_content_into_title_bar(
        window: *mut Window,
        enabled: bool,
    ) -> i32;
    pub fn zpd_winui3_window_set_title_bar(window: *mut Window, element: *const Element) -> i32;
    pub fn zpd_winui3_window_set_backdrop(window: *mut Window, backdrop: i32) -> i32;
    pub fn zpd_winui3_window_set_menu_bar(
        window: *mut Window,
        menus: *const MenuBarItem,
        length: usize,
        data: *const c_void,
        invoke: StringFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_window_clear_menu_bar(window: *mut Window) -> i32;
    pub fn zpd_winui3_button_create(title: StringRef) -> *mut Element;
    pub fn zpd_winui3_button_set_title(button: *mut Element, title: StringRef) -> i32;
    pub fn zpd_winui3_button_set_enabled(button: *mut Element, enabled: bool) -> i32;
    pub fn zpd_winui3_button_set_click_handler(
        button: *mut Element,
        data: *const c_void,
        invoke: InvokeFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_button_clear_click_handler(button: *mut Element) -> i32;
    pub fn zpd_winui3_text_create(text: StringRef) -> *mut Element;
    pub fn zpd_winui3_text_set_text(text: *mut Element, value: StringRef) -> i32;
    pub fn zpd_winui3_text_set_wrapping(text: *mut Element, enabled: bool) -> i32;
    pub fn zpd_winui3_text_set_alignment(text: *mut Element, alignment: i32) -> i32;
    pub fn zpd_winui3_text_field_create(value: StringRef) -> *mut Element;
    pub fn zpd_winui3_text_field_set_value(field: *mut Element, value: StringRef) -> i32;
    pub fn zpd_winui3_text_field_get_value(
        field: *const Element,
        data: *const c_void,
        receive: StringFn,
    ) -> i32;
    pub fn zpd_winui3_text_field_set_placeholder(field: *mut Element, value: StringRef) -> i32;
    pub fn zpd_winui3_text_field_set_read_only(field: *mut Element, read_only: bool) -> i32;
    pub fn zpd_winui3_text_field_set_change_handler(
        field: *mut Element,
        data: *const c_void,
        invoke: StringFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_text_field_clear_change_handler(field: *mut Element) -> i32;
    pub fn zpd_winui3_stack_panel_create(orientation: i32, spacing: f64) -> *mut Element;
    pub fn zpd_winui3_panel_append(panel: *mut Element, child: *const Element) -> i32;
    pub fn zpd_winui3_panel_clear(panel: *mut Element) -> i32;
    pub fn zpd_winui3_grid_create() -> *mut Element;
    pub fn zpd_winui3_grid_set_rows(
        grid: *mut Element,
        rows: *const GridLength,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_grid_set_columns(
        grid: *mut Element,
        columns: *const GridLength,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_grid_add(
        grid: *mut Element,
        child: *const Element,
        row: i32,
        column: i32,
        row_span: i32,
        column_span: i32,
    ) -> i32;
    pub fn zpd_winui3_element_release(element: *mut Element);
    pub fn zpd_winui3_element_set_margin(element: *mut Element, margin: Thickness) -> i32;
    pub fn zpd_winui3_element_set_width(element: *mut Element, width: f64) -> i32;
    pub fn zpd_winui3_element_set_height(element: *mut Element, height: f64) -> i32;
    pub fn zpd_winui3_element_set_horizontal_alignment(
        element: *mut Element,
        alignment: i32,
    ) -> i32;
    pub fn zpd_winui3_element_set_vertical_alignment(element: *mut Element, alignment: i32) -> i32;
    pub fn zpd_winui3_element_set_frame(
        element: *mut Element,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> i32;
    pub fn zpd_winui3_element_set_context_menu(
        element: *mut Element,
        items: *const MenuFlyoutItem,
        length: usize,
        data: *const c_void,
        invoke: StringFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_element_clear_context_menu(element: *mut Element) -> i32;

    pub fn zpd_winui3_window_create_styled(style: i32) -> *mut Window;
    pub fn zpd_winui3_window_move_and_resize(
        window: *mut Window,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> i32;
    pub fn zpd_winui3_window_set_closed_handler(
        window: *mut Window,
        data: *const c_void,
        invoke: InvokeFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_window_has_sheet(window: *const Window) -> bool;

    pub fn zpd_winui3_control_create(kind: i32) -> *mut Element;
    pub fn zpd_winui3_control_set_text(
        control: *mut Element,
        property: i32,
        value: StringRef,
    ) -> i32;
    pub fn zpd_winui3_control_get_text(
        control: *const Element,
        property: i32,
        data: *const c_void,
        receive: StringFn,
    ) -> i32;
    pub fn zpd_winui3_control_set_number(control: *mut Element, property: i32, value: f64) -> i32;
    pub fn zpd_winui3_control_get_number(
        control: *const Element,
        property: i32,
        value: *mut f64,
    ) -> i32;
    pub fn zpd_winui3_control_set_bool(control: *mut Element, property: i32, value: bool) -> i32;
    pub fn zpd_winui3_control_get_bool(
        control: *const Element,
        property: i32,
        value: *mut bool,
    ) -> i32;
    pub fn zpd_winui3_control_set_color(control: *mut Element, value: Color) -> i32;
    pub fn zpd_winui3_control_get_color(control: *const Element, value: *mut Color) -> i32;
    pub fn zpd_winui3_control_set_items(
        control: *mut Element,
        items: *const StringRef,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_control_set_children(
        control: *mut Element,
        children: *const *const Element,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_control_set_event_handler(
        control: *mut Element,
        data: *const c_void,
        invoke: EventFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_scroll_viewer_set_extent(
        scroll_viewer: *mut Element,
        width: f64,
        height: f64,
    ) -> i32;
    pub fn zpd_winui3_scroll_viewer_scroll_to(scroll_viewer: *mut Element, x: f64, y: f64) -> i32;
    pub fn zpd_winui3_navigation_set_items(
        navigation: *mut Element,
        items: *const NavigationItem,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_navigation_select(navigation: *mut Element, id: StringRef) -> i32;
    pub fn zpd_winui3_navigation_set_search(
        navigation: *mut Element,
        enabled: bool,
        value: StringRef,
        placeholder: StringRef,
    ) -> i32;

    pub fn zpd_winui3_popover_create(
        content: *const Element,
        data: *const c_void,
        closed: InvokeFn,
        release: ReleaseFn,
    ) -> *mut Popover;
    pub fn zpd_winui3_popover_release(popover: *mut Popover);
    pub fn zpd_winui3_popover_show(
        popover: *mut Popover,
        anchor: *const Element,
        placement: i32,
    ) -> i32;
    pub fn zpd_winui3_popover_hide(popover: *mut Popover) -> i32;
    pub fn zpd_winui3_popover_is_open(popover: *const Popover) -> bool;

    #[allow(clippy::too_many_arguments)]
    pub fn zpd_winui3_dialog_show(
        window: *mut Window,
        title: StringRef,
        message: StringRef,
        primary: StringRef,
        secondary: StringRef,
        close: StringRef,
        default_button: i32,
        data: *const c_void,
        completed: IntFn,
        release: ReleaseFn,
    ) -> *mut Dialog;
    pub fn zpd_winui3_dialog_release(dialog: *mut Dialog);

    pub fn zpd_winui3_file_dialog_show(
        window: *mut Window,
        options: *const FileDialogOptions,
        data: *const c_void,
        completed: PathsFn,
        release: ReleaseFn,
    ) -> i32;
}
