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
    pub inset: f64,
    pub rich: *const RichText,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NavigationItem {
    pub header: bool,
    pub id: StringRef,
    pub title: StringRef,
    pub glyph: StringRef,
    pub badge: StringRef,
    pub menu: *const MenuFlyoutItem,
    pub menu_length: usize,
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
    pub fn zpd_winui3_control_set_foreground(control: *mut Element, value: *const Color) -> i32;
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

#[repr(C)]
pub struct Draw {
    _private: [u8; 0],
}

#[repr(C)]
pub struct CanvasHandle {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CanvasFrame {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    pub time: f64,
    pub delta: f64,
    pub title_bar_inset: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CanvasInput {
    pub kind: i32,
    pub x: f64,
    pub y: f64,
    pub button: i32,
    pub modifiers: u32,
    pub click_count: u32,
    pub delta_x: f64,
    pub delta_y: f64,
    pub precise: bool,
    pub key: u32,
    pub code: u32,
    pub character: u32,
    pub repeat: bool,
    pub text: StringRef,
    pub flag: bool,
}

pub type CanvasDrawFn = unsafe extern "C" fn(*const c_void, *mut Draw, *const CanvasFrame);
pub type CanvasInputFn = unsafe extern "C" fn(*const c_void, *const CanvasInput) -> bool;
pub type CanvasHitFn = unsafe extern "C" fn(*const c_void, f64, f64) -> bool;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CanvasCallbacks {
    pub draw: CanvasDrawFn,
    pub input: CanvasInputFn,
    pub title_bar_hit: CanvasHitFn,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PathElement {
    pub verb: i32,
    pub points: [f64; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Path {
    pub elements: *const PathElement,
    pub length: usize,
    pub even_odd: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Paint {
    pub kind: i32,
    pub color: Color,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub radius: f64,
    pub stops: *const GradientStop,
    pub stops_length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Stroke {
    pub width: f64,
    pub cap: i32,
    pub join: i32,
    pub miter_limit: f64,
    pub dashes: *const f64,
    pub dashes_length: usize,
    pub dash_offset: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Font {
    pub family: StringRef,
    pub size: f64,
    pub weight: i32,
    pub italic: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TextMetrics {
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

unsafe extern "C" {
    pub fn zpd_winui3_canvas_create(mode: i32) -> *mut Element;
    pub fn zpd_winui3_canvas_set_callbacks(
        canvas: *mut Element,
        data: *const c_void,
        callbacks: CanvasCallbacks,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_canvas_set_redraw(canvas: *mut Element, continuous: bool, fps: f64) -> i32;
    pub fn zpd_winui3_canvas_set_focusable(canvas: *mut Element, focusable: bool) -> i32;
    pub fn zpd_winui3_canvas_swap_chain_panel(canvas: *mut Element) -> *mut c_void;
    pub fn zpd_winui3_canvas_handle_create(canvas: *const Element) -> *mut CanvasHandle;
    pub fn zpd_winui3_canvas_handle_release(handle: *mut CanvasHandle);
    pub fn zpd_winui3_canvas_handle_invalidate(handle: *const CanvasHandle);
    pub fn zpd_winui3_canvas_handle_set_focus(handle: *const CanvasHandle, focused: bool);
    pub fn zpd_winui3_canvas_handle_set_cursor(handle: *const CanvasHandle, cursor: i32);

    pub fn zpd_winui3_draw_save(draw: *mut Draw);
    pub fn zpd_winui3_draw_restore(draw: *mut Draw);
    pub fn zpd_winui3_draw_concat(draw: *mut Draw, transform: Transform);
    pub fn zpd_winui3_draw_set_alpha(draw: *mut Draw, alpha: f64);
    pub fn zpd_winui3_draw_set_antialias(draw: *mut Draw, enabled: bool);
    pub fn zpd_winui3_draw_clip(draw: *mut Draw, path: *const Path);
    pub fn zpd_winui3_draw_clear(
        draw: *mut Draw,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Color,
    );
    pub fn zpd_winui3_draw_fill(draw: *mut Draw, path: *const Path, paint: *const Paint);
    pub fn zpd_winui3_draw_stroke(
        draw: *mut Draw,
        path: *const Path,
        paint: *const Paint,
        stroke: *const Stroke,
    );
    pub fn zpd_winui3_draw_text(
        draw: *mut Draw,
        text: StringRef,
        x: f64,
        y: f64,
        font: *const Font,
        color: Color,
    );
    #[allow(clippy::too_many_arguments)]
    pub fn zpd_winui3_draw_image(
        draw: *mut Draw,
        id: u64,
        width: u32,
        height: u32,
        pixels: *const u8,
        x: f64,
        y: f64,
        rect_width: f64,
        rect_height: f64,
    );
    pub fn zpd_winui3_measure_text(
        text: StringRef,
        font: *const Font,
        metrics: *mut TextMetrics,
    ) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextStyle {
    pub font: Font,
    pub color: i32,
    pub custom: Color,
    pub alignment: i32,
    pub max_lines: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RichRun {
    pub text: StringRef,
    pub traits: u32,
    pub link: StringRef,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RichText {
    pub runs: *const RichRun,
    pub runs_length: usize,
    pub paragraphs: *const i32,
    pub paragraphs_length: usize,
}

pub type RichTextFn = unsafe extern "C" fn(*const c_void, *const RichText);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ListEntry {
    pub header: bool,
    pub id: StringRef,
    pub title: StringRef,
    pub cells: *const *const Element,
    pub cells_length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ToolbarItem {
    pub kind: i32,
    pub id: StringRef,
    pub label: StringRef,
    pub glyph: StringRef,
    pub enabled: bool,
    pub value: StringRef,
    pub placeholder: StringRef,
}

unsafe extern "C" {
    pub fn zpd_winui3_control_set_text_style(control: *mut Element, style: *const TextStyle)
    -> i32;
    pub fn zpd_winui3_rich_text_set(control: *mut Element, text: *const RichText) -> i32;
    pub fn zpd_winui3_rich_text_get(
        control: *const Element,
        data: *const c_void,
        receive: RichTextFn,
    ) -> i32;
    pub fn zpd_winui3_list_set_entries(
        list: *mut Element,
        entries: *const ListEntry,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_list_set_columns(
        list: *mut Element,
        titles: *const StringRef,
        widths: *const f64,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_list_set_row_heights(
        list: *mut Element,
        heights: *const f64,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_list_select(list: *mut Element, id: StringRef) -> i32;
    pub fn zpd_winui3_list_reveal(list: *mut Element, id: StringRef) -> i32;
    pub fn zpd_winui3_element_measure(
        element: *const Element,
        available_width: f64,
        available_height: f64,
        width: *mut f64,
        height: *mut f64,
    ) -> i32;
    pub fn zpd_winui3_element_set_tool_tip(element: *mut Element, text: StringRef) -> i32;
    pub fn zpd_winui3_element_set_identifier(element: *mut Element, id: StringRef) -> i32;
    pub fn zpd_winui3_element_set_accessibility(
        element: *mut Element,
        name: StringRef,
        help_text: StringRef,
        item_status: StringRef,
        view: i32,
    ) -> i32;
    pub fn zpd_winui3_element_set_labeled_by(element: *mut Element, label: *const Element) -> i32;
    pub fn zpd_winui3_element_focus(element: *mut Element) -> i32;
    pub fn zpd_winui3_element_title_bar_inset(element: *const Element) -> f64;
    pub fn zpd_winui3_scroll_viewer_reveal(
        scroll_viewer: *mut Element,
        id: StringRef,
        alignment: i32,
    ) -> i32;
    pub fn zpd_winui3_window_set_event_handler(
        window: *mut Window,
        data: *const c_void,
        invoke: EventFn,
        release: ReleaseFn,
    ) -> i32;
    pub fn zpd_winui3_window_set_toolbar(
        window: *mut Window,
        items: *const ToolbarItem,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_window_announce(window: *mut Window, text: StringRef, priority: i32) -> i32;
    pub fn zpd_winui3_window_set_document(
        window: *mut Window,
        confirms_close: bool,
        undo: bool,
        can_undo: bool,
        can_redo: bool,
    ) -> i32;
    pub fn zpd_winui3_window_set_autosave(window: *mut Window, name: StringRef) -> i32;
    pub fn zpd_winui3_window_set_passthrough(
        window: *mut Window,
        elements: *const *const Element,
        length: usize,
    ) -> i32;
    pub fn zpd_winui3_set_session_end_handler(
        data: *const c_void,
        invoke: InvokeFn,
        release: ReleaseFn,
    );
}
