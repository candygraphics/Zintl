#ifndef ZPD_WINUI3_H
#define ZPD_WINUI3_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct zpd_winui3_app_context zpd_winui3_app_context;
typedef struct zpd_winui3_dispatcher zpd_winui3_dispatcher;
typedef struct zpd_winui3_dispatcher_source zpd_winui3_dispatcher_source;
typedef struct zpd_winui3_dispatcher_signaler zpd_winui3_dispatcher_signaler;
typedef struct zpd_winui3_window zpd_winui3_window;
typedef struct zpd_winui3_element zpd_winui3_element;

typedef struct zpd_winui3_string {
  const uint8_t* data;
  size_t length;
} zpd_winui3_string;

typedef struct zpd_winui3_thickness {
  double left;
  double top;
  double right;
  double bottom;
} zpd_winui3_thickness;

typedef struct zpd_winui3_grid_length {
  int32_t kind;
  double value;
} zpd_winui3_grid_length;

enum zpd_winui3_menu_item_kind {
  ZPD_WINUI3_MENU_ITEM = 0,
  ZPD_WINUI3_MENU_SUBMENU = 1,
  ZPD_WINUI3_MENU_SEPARATOR = 2,
};

/* Standard text-editing actions applied to the focused text control. */
enum zpd_winui3_menu_role {
  ZPD_WINUI3_MENU_ROLE_NONE = 0,
  ZPD_WINUI3_MENU_ROLE_CUT = 1,
  ZPD_WINUI3_MENU_ROLE_COPY = 2,
  ZPD_WINUI3_MENU_ROLE_PASTE = 3,
  ZPD_WINUI3_MENU_ROLE_SELECT_ALL = 4,
  ZPD_WINUI3_MENU_ROLE_UNDO = 5,
  ZPD_WINUI3_MENU_ROLE_REDO = 6,
  ZPD_WINUI3_MENU_ROLE_DELETE = 7,
  /* Rich text formatting applied to the focused RichEditBox. */
  ZPD_WINUI3_MENU_ROLE_BOLD = 8,
  ZPD_WINUI3_MENU_ROLE_ITALIC = 9,
  ZPD_WINUI3_MENU_ROLE_UNDERLINE = 10,
  ZPD_WINUI3_MENU_ROLE_TITLE = 11,
  ZPD_WINUI3_MENU_ROLE_HEADING = 12,
  ZPD_WINUI3_MENU_ROLE_SUBHEADING = 13,
  ZPD_WINUI3_MENU_ROLE_BODY = 14,
  ZPD_WINUI3_MENU_ROLE_MONOSPACED = 15,
  ZPD_WINUI3_MENU_ROLE_BULLETED = 16,
  ZPD_WINUI3_MENU_ROLE_DASHED = 17,
  ZPD_WINUI3_MENU_ROLE_NUMBERED = 18,
  ZPD_WINUI3_MENU_ROLE_CHECKLIST = 19,
};

typedef struct zpd_winui3_menu_item {
  int32_t kind;
  zpd_winui3_string id;
  zpd_winui3_string title;
  zpd_winui3_string key;
  uint32_t modifiers;
  bool enabled;
  int32_t role;
  const struct zpd_winui3_menu_item* children;
  size_t children_length;
} zpd_winui3_menu_item;

typedef struct zpd_winui3_menu {
  zpd_winui3_string title;
  const zpd_winui3_menu_item* items;
  size_t items_length;
} zpd_winui3_menu;

typedef struct zpd_winui3_color {
  double red;
  double green;
  double blue;
  double alpha;
} zpd_winui3_color;

/* Native controls created by zpd_winui3_control_create. */
enum zpd_winui3_control_kind {
  ZPD_WINUI3_CONTROL_CANVAS = 0,
  ZPD_WINUI3_CONTROL_TEXT_BLOCK = 1,
  ZPD_WINUI3_CONTROL_TEXT_BOX = 2,
  ZPD_WINUI3_CONTROL_BUTTON = 3,
  ZPD_WINUI3_CONTROL_TOGGLE_BUTTON = 4,
  ZPD_WINUI3_CONTROL_CHECK_BOX = 5,
  ZPD_WINUI3_CONTROL_RADIO_BUTTON = 6,
  ZPD_WINUI3_CONTROL_TOGGLE_SWITCH = 7,
  ZPD_WINUI3_CONTROL_SLIDER = 8,
  ZPD_WINUI3_CONTROL_COMBO_BOX = 9,
  ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX = 10,
  ZPD_WINUI3_CONTROL_SELECTOR_BAR = 11,
  ZPD_WINUI3_CONTROL_NUMBER_BOX = 12,
  ZPD_WINUI3_CONTROL_AUTO_SUGGEST_BOX = 13,
  ZPD_WINUI3_CONTROL_COLOR_BUTTON = 14,
  ZPD_WINUI3_CONTROL_COLOR_PICKER = 15,
  ZPD_WINUI3_CONTROL_SCROLL_VIEWER = 16,
  ZPD_WINUI3_CONTROL_TAB_VIEW = 17,
  ZPD_WINUI3_CONTROL_NAVIGATION_VIEW = 18,
  ZPD_WINUI3_CONTROL_RICH_EDIT_BOX = 19,
  /* A selectable list of rows, with optional section headers and column headers. */
  ZPD_WINUI3_CONTROL_LIST_VIEW = 20,
  ZPD_WINUI3_CONTROL_PROGRESS_RING = 21,
  /* Shows NUMBER_VALUE in 0...1. */
  ZPD_WINUI3_CONTROL_PROGRESS_BAR = 22,
  /* A one-pixel line along the longer side of its frame. */
  ZPD_WINUI3_CONTROL_SEPARATOR = 23,
};

enum zpd_winui3_text_property {
  /* Content, value, or label text. */
  ZPD_WINUI3_TEXT = 0,
  ZPD_WINUI3_TEXT_PLACEHOLDER = 1,
  /* A Segoe Fluent Icons glyph shown before button content. */
  ZPD_WINUI3_TEXT_GLYPH = 2,
};

enum zpd_winui3_number_property {
  ZPD_WINUI3_NUMBER_VALUE = 0,
  ZPD_WINUI3_NUMBER_MINIMUM = 1,
  ZPD_WINUI3_NUMBER_MAXIMUM = 2,
  ZPD_WINUI3_NUMBER_STEP = 3,
  /* Number of slider tick marks; zero hides them. */
  ZPD_WINUI3_NUMBER_TICK_COUNT = 4,
  /* Selected item index; negative clears the selection. */
  ZPD_WINUI3_NUMBER_SELECTED_INDEX = 5,
  ZPD_WINUI3_NUMBER_FRACTION_DIGITS = 6,
};

enum zpd_winui3_bool_property {
  ZPD_WINUI3_BOOL_CHECKED = 0,
  ZPD_WINUI3_BOOL_ENABLED = 1,
  ZPD_WINUI3_BOOL_READ_ONLY = 2,
  ZPD_WINUI3_BOOL_MULTILINE = 3,
  ZPD_WINUI3_BOOL_BORDERLESS = 4,
  ZPD_WINUI3_BOOL_DESTRUCTIVE = 5,
  ZPD_WINUI3_BOOL_SNAP_TO_TICKS = 6,
  ZPD_WINUI3_BOOL_HORIZONTAL_SCROLL = 7,
  ZPD_WINUI3_BOOL_VERTICAL_SCROLL = 8,
  /* Draws a round help button. */
  ZPD_WINUI3_BOOL_HELP = 9,
  /* Accent style; the button answers Enter. */
  ZPD_WINUI3_BOOL_DEFAULT_ACTION = 10,
  /* The button answers Escape. */
  ZPD_WINUI3_BOOL_CANCEL_ACTION = 11,
  ZPD_WINUI3_BOOL_ALPHA = 12,
  ZPD_WINUI3_BOOL_BORDERED = 13,
  ZPD_WINUI3_BOOL_DRAWS_BACKGROUND = 14,
  ZPD_WINUI3_BOOL_SELECTABLE = 15,
  /* Alternates list row backgrounds. */
  ZPD_WINUI3_BOOL_STRIPED = 16,
  /* A click shows the context menu below the button instead of reporting it. */
  ZPD_WINUI3_BOOL_SHOWS_MENU = 17,
  /* Clicks are reported as CLICKED and keep the declared state. */
  ZPD_WINUI3_BOOL_KEEPS_STATE = 18,
};

enum zpd_winui3_event_kind {
  ZPD_WINUI3_EVENT_CLICKED = 0,
  ZPD_WINUI3_EVENT_TEXT_CHANGED = 1,
  ZPD_WINUI3_EVENT_STATE_CHANGED = 2,
  ZPD_WINUI3_EVENT_VALUE_CHANGED = 3,
  ZPD_WINUI3_EVENT_SELECTION_CHANGED = 4,
  ZPD_WINUI3_EVENT_COLOR_CHANGED = 5,
  /* x and y hold the top-left offset. */
  ZPD_WINUI3_EVENT_SCROLLED = 6,
  /* x and y hold the new width and height. */
  ZPD_WINUI3_EVENT_SIZE_CHANGED = 7,
  /* text holds the selected navigation item id. */
  ZPD_WINUI3_EVENT_NAVIGATION_SELECTED = 8,
  ZPD_WINUI3_EVENT_SEARCH_CHANGED = 9,
  /* rich holds the document. */
  ZPD_WINUI3_EVENT_RICH_TEXT_CHANGED = 10,
  /* text holds the selected row id; flag is false when nothing is selected. */
  ZPD_WINUI3_EVENT_LIST_SELECTION_CHANGED = 11,
  ZPD_WINUI3_EVENT_TOOLBAR_CLICKED = 12,
  ZPD_WINUI3_EVENT_TOOLBAR_SEARCH_CHANGED = 13,
  /* text holds the chosen menu item id. */
  ZPD_WINUI3_EVENT_MENU_SELECTED = 14,
  /* Editing ended with Enter or a focus change; text holds the final text. */
  ZPD_WINUI3_EVENT_TEXT_COMMITTED = 15,
  ZPD_WINUI3_EVENT_WINDOW_CLOSE_REQUESTED = 16,
  ZPD_WINUI3_EVENT_UNDO = 17,
  ZPD_WINUI3_EVENT_REDO = 18,
  /* A toggle button was clicked and changed; flag holds the new state. */
  ZPD_WINUI3_EVENT_TOGGLED = 19,
};

/* A user change reported by a control; fields unused by `kind` are zero. */
typedef struct zpd_winui3_event {
  int32_t kind;
  double x;
  double y;
  int64_t index;
  bool flag;
  zpd_winui3_string text;
  zpd_winui3_color color;
  /* For SIZE_CHANGED of a canvas, the height of its top part under the title bar. */
  double inset;
  const struct zpd_winui3_rich_text* rich;
} zpd_winui3_event;

typedef struct zpd_winui3_navigation_item {
  /* Headers are nonselectable section titles. */
  bool header;
  zpd_winui3_string id;
  zpd_winui3_string title;
  zpd_winui3_string glyph;
  /* Trailing text such as a count; empty for none. */
  zpd_winui3_string badge;
  /* The item's context menu; choices report MENU_SELECTED. */
  const zpd_winui3_menu_item* menu;
  size_t menu_length;
} zpd_winui3_navigation_item;

enum zpd_winui3_window_style {
  ZPD_WINUI3_WINDOW_NORMAL = 0,
  /* An always-on-top utility window. */
  ZPD_WINUI3_WINDOW_PANEL = 1,
};

enum zpd_winui3_placement {
  ZPD_WINUI3_PLACEMENT_BOTTOM = 0,
  ZPD_WINUI3_PLACEMENT_TOP = 1,
  ZPD_WINUI3_PLACEMENT_LEFT = 2,
  ZPD_WINUI3_PLACEMENT_RIGHT = 3,
};

enum zpd_winui3_dialog_result {
  ZPD_WINUI3_DIALOG_NONE = 0,
  ZPD_WINUI3_DIALOG_PRIMARY = 1,
  ZPD_WINUI3_DIALOG_SECONDARY = 2,
  ZPD_WINUI3_DIALOG_CLOSE = 3,
};

enum zpd_winui3_file_dialog_mode {
  ZPD_WINUI3_FILE_DIALOG_OPEN = 0,
  ZPD_WINUI3_FILE_DIALOG_SAVE = 1,
  ZPD_WINUI3_FILE_DIALOG_FOLDER = 2,
};

typedef struct zpd_winui3_file_dialog_options {
  int32_t mode;
  bool multiple;
  zpd_winui3_string file_name;
  zpd_winui3_string commit_text;
  /* Extensions without dots; empty allows every type. */
  const zpd_winui3_string* extensions;
  size_t extensions_length;
} zpd_winui3_file_dialog_options;

typedef struct zpd_winui3_popover zpd_winui3_popover;
typedef struct zpd_winui3_dialog zpd_winui3_dialog;

typedef void (*zpd_winui3_launch_fn)(const zpd_winui3_app_context*, const void*);
typedef void (*zpd_winui3_invoke_fn)(const void*);
typedef void (*zpd_winui3_string_fn)(const void*, zpd_winui3_string);
typedef void (*zpd_winui3_release_fn)(const void*);
typedef void (*zpd_winui3_event_fn)(const void*, const zpd_winui3_event*);
typedef void (*zpd_winui3_int_fn)(const void*, int32_t);
/* `picked` is false when the user cancelled. */
typedef void (*zpd_winui3_paths_fn)(
    const void*, bool picked, const zpd_winui3_string* paths, size_t length);

int32_t zpd_winui3_application_run(
    const void* user_data,
    zpd_winui3_launch_fn launch,
    zpd_winui3_release_fn release);

zpd_winui3_dispatcher* zpd_winui3_app_dispatcher(const zpd_winui3_app_context* context);
/* Describes the last failure on this thread; empty after a success. */
zpd_winui3_string zpd_winui3_last_error(void);
int32_t zpd_winui3_application_exit(void);

zpd_winui3_dispatcher* zpd_winui3_dispatcher_clone(const zpd_winui3_dispatcher* dispatcher);
void zpd_winui3_dispatcher_release(zpd_winui3_dispatcher* dispatcher);
bool zpd_winui3_dispatcher_try_enqueue(
    const zpd_winui3_dispatcher* dispatcher,
    int32_t priority,
    const void* user_data,
    zpd_winui3_invoke_fn invoke,
    zpd_winui3_release_fn release);
zpd_winui3_dispatcher_source* zpd_winui3_dispatcher_source_create(
    const zpd_winui3_dispatcher* dispatcher,
    const void* user_data,
    zpd_winui3_invoke_fn invoke,
    zpd_winui3_release_fn release);
void zpd_winui3_dispatcher_source_release(zpd_winui3_dispatcher_source* source);
zpd_winui3_dispatcher_signaler* zpd_winui3_dispatcher_source_signaler(
    const zpd_winui3_dispatcher_source* source);
zpd_winui3_dispatcher_signaler* zpd_winui3_dispatcher_signaler_clone(
    const zpd_winui3_dispatcher_signaler* signaler);
void zpd_winui3_dispatcher_signaler_release(zpd_winui3_dispatcher_signaler* signaler);
bool zpd_winui3_dispatcher_signaler_signal(
    const zpd_winui3_dispatcher_signaler* signaler);

void zpd_winui3_window_release(zpd_winui3_window* window);
int32_t zpd_winui3_window_set_title(zpd_winui3_window* window, zpd_winui3_string title);
int32_t zpd_winui3_window_resize(zpd_winui3_window* window, int32_t width, int32_t height);
int32_t zpd_winui3_window_activate(zpd_winui3_window* window);
int32_t zpd_winui3_window_close(zpd_winui3_window* window);
int32_t zpd_winui3_window_set_content(zpd_winui3_window* window, const zpd_winui3_element* element);
int32_t zpd_winui3_window_extend_content_into_title_bar(zpd_winui3_window* window, bool enabled);
int32_t zpd_winui3_window_set_title_bar(
    zpd_winui3_window* window,
    const zpd_winui3_element* element);
int32_t zpd_winui3_window_set_backdrop(zpd_winui3_window* window, int32_t backdrop);
int32_t zpd_winui3_window_set_menu_bar(
    zpd_winui3_window* window,
    const zpd_winui3_menu* menus,
    size_t menus_length,
    const void* user_data,
    zpd_winui3_string_fn invoke,
    zpd_winui3_release_fn release);
int32_t zpd_winui3_window_clear_menu_bar(zpd_winui3_window* window);

zpd_winui3_element* zpd_winui3_button_create(zpd_winui3_string title);
int32_t zpd_winui3_button_set_title(zpd_winui3_element* button, zpd_winui3_string title);
int32_t zpd_winui3_button_set_enabled(zpd_winui3_element* button, bool enabled);
int32_t zpd_winui3_button_set_click_handler(
    zpd_winui3_element* button,
    const void* user_data,
    zpd_winui3_invoke_fn invoke,
    zpd_winui3_release_fn release);
int32_t zpd_winui3_button_clear_click_handler(zpd_winui3_element* button);

zpd_winui3_element* zpd_winui3_text_create(zpd_winui3_string text);
int32_t zpd_winui3_text_set_text(zpd_winui3_element* text, zpd_winui3_string value);
int32_t zpd_winui3_text_set_wrapping(zpd_winui3_element* text, bool enabled);
int32_t zpd_winui3_text_set_alignment(zpd_winui3_element* text, int32_t alignment);

zpd_winui3_element* zpd_winui3_text_field_create(zpd_winui3_string value);
int32_t zpd_winui3_text_field_set_value(zpd_winui3_element* field, zpd_winui3_string value);
int32_t zpd_winui3_text_field_get_value(
    const zpd_winui3_element* field,
    const void* user_data,
    zpd_winui3_string_fn receive);
int32_t zpd_winui3_text_field_set_placeholder(zpd_winui3_element* field, zpd_winui3_string value);
int32_t zpd_winui3_text_field_set_read_only(zpd_winui3_element* field, bool read_only);
int32_t zpd_winui3_text_field_set_change_handler(
    zpd_winui3_element* field,
    const void* user_data,
    zpd_winui3_string_fn invoke,
    zpd_winui3_release_fn release);
int32_t zpd_winui3_text_field_clear_change_handler(zpd_winui3_element* field);

zpd_winui3_element* zpd_winui3_stack_panel_create(int32_t orientation, double spacing);
int32_t zpd_winui3_panel_append(zpd_winui3_element* panel, const zpd_winui3_element* child);
int32_t zpd_winui3_panel_clear(zpd_winui3_element* panel);

zpd_winui3_element* zpd_winui3_grid_create(void);
int32_t zpd_winui3_grid_set_rows(
    zpd_winui3_element* grid,
    const zpd_winui3_grid_length* rows,
    size_t length);
int32_t zpd_winui3_grid_set_columns(
    zpd_winui3_element* grid,
    const zpd_winui3_grid_length* columns,
    size_t length);
int32_t zpd_winui3_grid_add(
    zpd_winui3_element* grid,
    const zpd_winui3_element* child,
    int32_t row,
    int32_t column,
    int32_t row_span,
    int32_t column_span);

void zpd_winui3_element_release(zpd_winui3_element* element);
int32_t zpd_winui3_element_set_margin(zpd_winui3_element* element, zpd_winui3_thickness margin);
int32_t zpd_winui3_element_set_width(zpd_winui3_element* element, double width);
int32_t zpd_winui3_element_set_height(zpd_winui3_element* element, double height);
int32_t zpd_winui3_element_set_horizontal_alignment(zpd_winui3_element* element, int32_t alignment);
int32_t zpd_winui3_element_set_vertical_alignment(zpd_winui3_element* element, int32_t alignment);

/* Places an element inside its parent Canvas. */
int32_t zpd_winui3_element_set_frame(
    zpd_winui3_element* element, double x, double y, double width, double height);
int32_t zpd_winui3_element_set_context_menu(
    zpd_winui3_element* element,
    const zpd_winui3_menu_item* items,
    size_t items_length,
    const void* user_data,
    zpd_winui3_string_fn invoke,
    zpd_winui3_release_fn release);
int32_t zpd_winui3_element_clear_context_menu(zpd_winui3_element* element);

zpd_winui3_window* zpd_winui3_window_create_styled(int32_t style);
/* Moves the window and sizes its client area in device-independent pixels. */
int32_t zpd_winui3_window_move_and_resize(
    zpd_winui3_window* window, double x, double y, double width, double height);
/* Invokes `invoke` after the user or the app closed the window. */
int32_t zpd_winui3_window_set_closed_handler(
    zpd_winui3_window* window,
    const void* user_data,
    zpd_winui3_invoke_fn invoke,
    zpd_winui3_release_fn release);
/* Returns whether a dialog or file picker is open on the window. */
bool zpd_winui3_window_has_sheet(const zpd_winui3_window* window);

zpd_winui3_element* zpd_winui3_control_create(int32_t kind);
int32_t zpd_winui3_control_set_text(
    zpd_winui3_element* control, int32_t property, zpd_winui3_string value);
int32_t zpd_winui3_control_get_text(
    const zpd_winui3_element* control,
    int32_t property,
    const void* user_data,
    zpd_winui3_string_fn receive);
int32_t zpd_winui3_control_set_number(zpd_winui3_element* control, int32_t property, double value);
int32_t zpd_winui3_control_get_number(
    const zpd_winui3_element* control, int32_t property, double* value);
int32_t zpd_winui3_control_set_bool(zpd_winui3_element* control, int32_t property, bool value);
int32_t zpd_winui3_control_get_bool(
    const zpd_winui3_element* control, int32_t property, bool* value);
int32_t zpd_winui3_control_set_color(zpd_winui3_element* control, zpd_winui3_color value);
int32_t zpd_winui3_control_get_color(const zpd_winui3_element* control, zpd_winui3_color* value);
// Colors a control's text; a null value restores the theme's color.
int32_t zpd_winui3_control_set_foreground(zpd_winui3_element* control, const zpd_winui3_color* value);
/* Sets choices, segments, or tab titles. */
int32_t zpd_winui3_control_set_items(
    zpd_winui3_element* control, const zpd_winui3_string* items, size_t length);
/* Sets Canvas children, scroll content, tab pages, or navigation content. */
int32_t zpd_winui3_control_set_children(
    zpd_winui3_element* control, const zpd_winui3_element* const* children, size_t length);
/* Installs the one handler that receives this control's user changes. */
int32_t zpd_winui3_control_set_event_handler(
    zpd_winui3_element* control,
    const void* user_data,
    zpd_winui3_event_fn invoke,
    zpd_winui3_release_fn release);

int32_t zpd_winui3_scroll_viewer_set_extent(
    zpd_winui3_element* scroll_viewer, double width, double height);
int32_t zpd_winui3_scroll_viewer_scroll_to(zpd_winui3_element* scroll_viewer, double x, double y);

int32_t zpd_winui3_navigation_set_items(
    zpd_winui3_element* navigation, const zpd_winui3_navigation_item* items, size_t length);
/* An empty id clears the selection. */
int32_t zpd_winui3_navigation_select(zpd_winui3_element* navigation, zpd_winui3_string id);
int32_t zpd_winui3_navigation_set_search(
    zpd_winui3_element* navigation,
    bool enabled,
    zpd_winui3_string value,
    zpd_winui3_string placeholder);

zpd_winui3_popover* zpd_winui3_popover_create(
    const zpd_winui3_element* content,
    const void* user_data,
    zpd_winui3_invoke_fn closed,
    zpd_winui3_release_fn release);
void zpd_winui3_popover_release(zpd_winui3_popover* popover);
int32_t zpd_winui3_popover_show(
    zpd_winui3_popover* popover, const zpd_winui3_element* anchor, int32_t placement);
int32_t zpd_winui3_popover_hide(zpd_winui3_popover* popover);
bool zpd_winui3_popover_is_open(const zpd_winui3_popover* popover);

/* Shows a ContentDialog; empty button titles omit the button. */
zpd_winui3_dialog* zpd_winui3_dialog_show(
    zpd_winui3_window* window,
    zpd_winui3_string title,
    zpd_winui3_string message,
    zpd_winui3_string primary,
    zpd_winui3_string secondary,
    zpd_winui3_string close,
    int32_t default_button,
    const void* user_data,
    zpd_winui3_int_fn completed,
    zpd_winui3_release_fn release);
/* Hides the dialog if it is still open; `completed` then reports NONE. */
void zpd_winui3_dialog_release(zpd_winui3_dialog* dialog);

int32_t zpd_winui3_file_dialog_show(
    zpd_winui3_window* window,
    const zpd_winui3_file_dialog_options* options,
    const void* user_data,
    zpd_winui3_paths_fn completed,
    zpd_winui3_release_fn release);

/* ---- Canvases ---- */

typedef struct zpd_winui3_draw zpd_winui3_draw;

enum zpd_winui3_canvas_mode {
  /* The canvas draws each frame with Direct2D through zpd_winui3_draw_*. */
  ZPD_WINUI3_CANVAS_DIRECT2D = 0,
  /* Another renderer presents to the panel, such as wgpu. */
  ZPD_WINUI3_CANVAS_EXTERNAL = 1,
};

/* Geometry and timing of one canvas frame, in DIPs and seconds. */
typedef struct zpd_winui3_canvas_frame {
  double width;
  double height;
  double scale;
  double time;
  double delta;
  /* The height of the canvas's top part that the window's title bar covers. */
  double title_bar_inset;
} zpd_winui3_canvas_frame;

enum zpd_winui3_canvas_input_kind {
  ZPD_WINUI3_INPUT_POINTER_DOWN = 0,
  ZPD_WINUI3_INPUT_POINTER_UP = 1,
  ZPD_WINUI3_INPUT_POINTER_MOVED = 2,
  ZPD_WINUI3_INPUT_POINTER_ENTERED = 3,
  ZPD_WINUI3_INPUT_POINTER_EXITED = 4,
  ZPD_WINUI3_INPUT_SCROLL = 5,
  ZPD_WINUI3_INPUT_KEY_DOWN = 6,
  ZPD_WINUI3_INPUT_KEY_UP = 7,
  /* Text that no key press produced, such as from the emoji panel. */
  ZPD_WINUI3_INPUT_TEXT = 8,
  ZPD_WINUI3_INPUT_MODIFIERS = 9,
  ZPD_WINUI3_INPUT_FOCUS = 10,
};

enum zpd_winui3_modifier {
  ZPD_WINUI3_MODIFIER_SHIFT = 1,
  ZPD_WINUI3_MODIFIER_CONTROL = 2,
  ZPD_WINUI3_MODIFIER_ALT = 4,
  ZPD_WINUI3_MODIFIER_META = 8,
};

/* Input delivered to a canvas; fields unused by `kind` are zero. */
typedef struct zpd_winui3_canvas_input {
  int32_t kind;
  double x;
  double y;
  /* -1 for none, then 0 primary, 1 secondary, 2 middle, and further buttons. */
  int32_t button;
  uint32_t modifiers;
  uint32_t click_count;
  double delta_x;
  double delta_y;
  bool precise;
  /* The Windows virtual key, its hardware scan code, and its unmodified character. */
  uint32_t key;
  uint32_t code;
  uint32_t character;
  bool repeat;
  /* Inserted text for key presses; gained focus for FOCUS. */
  zpd_winui3_string text;
  bool flag;
} zpd_winui3_canvas_input;

/* Draws one frame; `draw` is null for external canvases. */
typedef void (*zpd_winui3_canvas_draw_fn)(
    const void*, zpd_winui3_draw* draw, const zpd_winui3_canvas_frame* frame);
/* Returns whether the input was consumed. */
typedef bool (*zpd_winui3_canvas_input_fn)(const void*, const zpd_winui3_canvas_input* input);
/* Returns whether a press at canvas point (x, y) under the title bar belongs to the canvas. */
typedef bool (*zpd_winui3_canvas_hit_fn)(const void*, double x, double y);

typedef struct zpd_winui3_canvas_callbacks {
  zpd_winui3_canvas_draw_fn draw;
  zpd_winui3_canvas_input_fn input;
  zpd_winui3_canvas_hit_fn title_bar_hit;
} zpd_winui3_canvas_callbacks;

enum zpd_winui3_cursor {
  ZPD_WINUI3_CURSOR_ARROW = 0,
  ZPD_WINUI3_CURSOR_HAND = 1,
  ZPD_WINUI3_CURSOR_IBEAM = 2,
  ZPD_WINUI3_CURSOR_CROSSHAIR = 3,
  ZPD_WINUI3_CURSOR_OPEN_HAND = 4,
  ZPD_WINUI3_CURSOR_CLOSED_HAND = 5,
  ZPD_WINUI3_CURSOR_RESIZE_LEFT_RIGHT = 6,
  ZPD_WINUI3_CURSOR_RESIZE_UP_DOWN = 7,
  ZPD_WINUI3_CURSOR_NOT_ALLOWED = 8,
};

zpd_winui3_element* zpd_winui3_canvas_create(int32_t mode);
int32_t zpd_winui3_canvas_set_callbacks(
    zpd_winui3_element* canvas,
    const void* user_data,
    zpd_winui3_canvas_callbacks callbacks,
    zpd_winui3_release_fn release);
/* A zero or negative `fps` follows the display; `continuous` false draws on demand. */
int32_t zpd_winui3_canvas_set_redraw(zpd_winui3_element* canvas, bool continuous, double fps);
int32_t zpd_winui3_canvas_set_focusable(zpd_winui3_element* canvas, bool focusable);

/* A weak reference to a canvas; its operations do nothing once the canvas is gone. */
typedef struct zpd_winui3_canvas_handle zpd_winui3_canvas_handle;
zpd_winui3_canvas_handle* zpd_winui3_canvas_handle_create(const zpd_winui3_element* canvas);
void zpd_winui3_canvas_handle_release(zpd_winui3_canvas_handle* handle);
/* Schedules one draw; repeated requests before it runs are coalesced. */
void zpd_winui3_canvas_handle_invalidate(const zpd_winui3_canvas_handle* handle);
void zpd_winui3_canvas_handle_set_focus(const zpd_winui3_canvas_handle* handle, bool focused);
void zpd_winui3_canvas_handle_set_cursor(const zpd_winui3_canvas_handle* handle, int32_t cursor);
/* Returns the canvas's ISwapChainPanelNative; it scales a swap chain set through it to DIPs. */
void* zpd_winui3_canvas_swap_chain_panel(zpd_winui3_element* canvas);

enum zpd_winui3_path_verb {
  ZPD_WINUI3_PATH_MOVE = 0,
  ZPD_WINUI3_PATH_LINE = 1,
  ZPD_WINUI3_PATH_QUAD = 2,
  ZPD_WINUI3_PATH_CUBIC = 3,
  ZPD_WINUI3_PATH_CLOSE = 4,
};

/* One path element; the end point comes last in `points`. */
typedef struct zpd_winui3_path_element {
  int32_t verb;
  double points[6];
} zpd_winui3_path_element;

typedef struct zpd_winui3_path {
  const zpd_winui3_path_element* elements;
  size_t length;
  /* False for non-zero winding, true for even-odd. */
  bool even_odd;
} zpd_winui3_path;

typedef struct zpd_winui3_gradient_stop {
  double offset;
  zpd_winui3_color color;
} zpd_winui3_gradient_stop;

enum zpd_winui3_paint_kind {
  ZPD_WINUI3_PAINT_SOLID = 0,
  ZPD_WINUI3_PAINT_LINEAR = 1,
  ZPD_WINUI3_PAINT_RADIAL = 2,
};

/* Linear gradients run from (x0, y0) to (x1, y1); radial ones center on (x0, y0). */
typedef struct zpd_winui3_paint {
  int32_t kind;
  zpd_winui3_color color;
  double x0;
  double y0;
  double x1;
  double y1;
  double radius;
  const zpd_winui3_gradient_stop* stops;
  size_t stops_length;
} zpd_winui3_paint;

typedef struct zpd_winui3_stroke {
  double width;
  /* Butt, round, square. */
  int32_t cap;
  /* Miter, round, bevel. */
  int32_t join;
  double miter_limit;
  const double* dashes;
  size_t dashes_length;
  double dash_offset;
} zpd_winui3_stroke;

typedef struct zpd_winui3_font {
  /* Empty for the system font. */
  zpd_winui3_string family;
  double size;
  int32_t weight;
  bool italic;
} zpd_winui3_font;

typedef struct zpd_winui3_text_metrics {
  double width;
  double ascent;
  double descent;
  double leading;
} zpd_winui3_text_metrics;

/* An affine transform mapping (x, y) to (a*x + c*y + tx, b*x + d*y + ty). */
typedef struct zpd_winui3_transform {
  double a;
  double b;
  double c;
  double d;
  double tx;
  double ty;
} zpd_winui3_transform;

void zpd_winui3_draw_save(zpd_winui3_draw* draw);
void zpd_winui3_draw_restore(zpd_winui3_draw* draw);
/* Applies `transform` before the current transform. */
void zpd_winui3_draw_concat(zpd_winui3_draw* draw, zpd_winui3_transform transform);
void zpd_winui3_draw_set_alpha(zpd_winui3_draw* draw, double alpha);
void zpd_winui3_draw_set_antialias(zpd_winui3_draw* draw, bool enabled);
void zpd_winui3_draw_clip(zpd_winui3_draw* draw, const zpd_winui3_path* path);
/* Replaces the pixels of `rect`, including their alpha, with `color`. */
void zpd_winui3_draw_clear(
    zpd_winui3_draw* draw, double x, double y, double width, double height, zpd_winui3_color color);
void zpd_winui3_draw_fill(
    zpd_winui3_draw* draw, const zpd_winui3_path* path, const zpd_winui3_paint* paint);
void zpd_winui3_draw_stroke(
    zpd_winui3_draw* draw,
    const zpd_winui3_path* path,
    const zpd_winui3_paint* paint,
    const zpd_winui3_stroke* stroke);
/* Draws one line of text whose baseline starts at (x, y). */
void zpd_winui3_draw_text(
    zpd_winui3_draw* draw,
    zpd_winui3_string text,
    double x,
    double y,
    const zpd_winui3_font* font,
    zpd_winui3_color color);
/* Draws straight-alpha RGBA8 pixels, cached by `id` while consecutive frames draw them. */
void zpd_winui3_draw_image(
    zpd_winui3_draw* draw,
    uint64_t id,
    uint32_t width,
    uint32_t height,
    const uint8_t* pixels,
    double x,
    double y,
    double rect_width,
    double rect_height);
/* Measures one line of text with DirectWrite. */
int32_t zpd_winui3_measure_text(
    zpd_winui3_string text, const zpd_winui3_font* font, zpd_winui3_text_metrics* metrics);

/* ---- Text styles ---- */

enum zpd_winui3_text_color {
  ZPD_WINUI3_TEXT_COLOR_PRIMARY = 0,
  ZPD_WINUI3_TEXT_COLOR_SECONDARY = 1,
  ZPD_WINUI3_TEXT_COLOR_TERTIARY = 2,
  ZPD_WINUI3_TEXT_COLOR_ACCENT = 3,
  ZPD_WINUI3_TEXT_COLOR_CUSTOM = 4,
};

typedef struct zpd_winui3_text_style {
  /* A zero size keeps the control's font. */
  zpd_winui3_font font;
  int32_t color;
  zpd_winui3_color custom;
  /* Leading, center, trailing. */
  int32_t alignment;
  /* Lines before the last one is truncated; zero keeps the control's wrapping. */
  int32_t max_lines;
} zpd_winui3_text_style;

/* Styles a TextBlock or TextBox. */
int32_t zpd_winui3_control_set_text_style(
    zpd_winui3_element* control, const zpd_winui3_text_style* style);

/* ---- Rich text ---- */

enum zpd_winui3_text_trait {
  ZPD_WINUI3_TRAIT_BOLD = 1,
  ZPD_WINUI3_TRAIT_ITALIC = 2,
  ZPD_WINUI3_TRAIT_UNDERLINE = 4,
  ZPD_WINUI3_TRAIT_STRIKETHROUGH = 8,
  ZPD_WINUI3_TRAIT_MONOSPACE = 16,
};

enum zpd_winui3_paragraph_style {
  ZPD_WINUI3_PARAGRAPH_TITLE = 0,
  ZPD_WINUI3_PARAGRAPH_HEADING = 1,
  ZPD_WINUI3_PARAGRAPH_SUBHEADING = 2,
  ZPD_WINUI3_PARAGRAPH_BODY = 3,
  ZPD_WINUI3_PARAGRAPH_MONOSPACED = 4,
  ZPD_WINUI3_PARAGRAPH_BULLETED = 5,
  ZPD_WINUI3_PARAGRAPH_DASHED = 6,
  ZPD_WINUI3_PARAGRAPH_NUMBERED = 7,
  ZPD_WINUI3_PARAGRAPH_CHECKLIST = 8,
  ZPD_WINUI3_PARAGRAPH_CHECKLIST_DONE = 9,
};

/* A run of text with one set of traits; paragraphs end with '\n'. */
typedef struct zpd_winui3_rich_run {
  zpd_winui3_string text;
  uint32_t traits;
  /* Empty when the run is not a link. */
  zpd_winui3_string link;
} zpd_winui3_rich_run;

/* Runs that cover the text, and one style per paragraph. */
typedef struct zpd_winui3_rich_text {
  const zpd_winui3_rich_run* runs;
  size_t runs_length;
  const int32_t* paragraphs;
  size_t paragraphs_length;
} zpd_winui3_rich_text;

typedef void (*zpd_winui3_rich_text_fn)(const void*, const zpd_winui3_rich_text*);

/* Replaces a RichEditBox's document without reporting a change. */
int32_t zpd_winui3_rich_text_set(zpd_winui3_element* control, const zpd_winui3_rich_text* text);
int32_t zpd_winui3_rich_text_get(
    const zpd_winui3_element* control, const void* user_data, zpd_winui3_rich_text_fn receive);

/* ---- Lists ---- */

/* Rows are this much narrower than their list, which positions cells and column headers. */
#define ZPD_WINUI3_LIST_ROW_INSET 24.0

/* A section header, or a row whose cells are laid out side by side. */
typedef struct zpd_winui3_list_entry {
  bool header;
  zpd_winui3_string id;
  zpd_winui3_string title;
  const zpd_winui3_element* const* cells;
  size_t cells_length;
} zpd_winui3_list_entry;

int32_t zpd_winui3_list_set_entries(
    zpd_winui3_element* list, const zpd_winui3_list_entry* entries, size_t length);
/* Shows column headers with the given widths; no columns hides them. */
int32_t zpd_winui3_list_set_columns(
    zpd_winui3_element* list, const zpd_winui3_string* titles, const double* widths, size_t length);
/* Sizes the rows in order, skipping headers. */
int32_t zpd_winui3_list_set_row_heights(
    zpd_winui3_element* list, const double* heights, size_t length);
/* An empty id clears the selection. */
int32_t zpd_winui3_list_select(zpd_winui3_element* list, zpd_winui3_string id);
int32_t zpd_winui3_list_reveal(zpd_winui3_element* list, zpd_winui3_string id);

/* ---- Elements ---- */

/* Measures the element's desired size within the available size; infinity leaves an axis open. */
int32_t zpd_winui3_element_measure(
    const zpd_winui3_element* element,
    double available_width,
    double available_height,
    double* width,
    double* height);
/* An empty text removes the tooltip. */
int32_t zpd_winui3_element_set_tool_tip(zpd_winui3_element* element, zpd_winui3_string text);
/* Sets the automation id that scroll viewers reveal by. */
int32_t zpd_winui3_element_set_identifier(zpd_winui3_element* element, zpd_winui3_string id);

/* Which UI Automation views include an element. */
enum zpd_winui3_accessibility_view {
  /* The element's own choice: controls appear, panels do not. */
  ZPD_WINUI3_ACCESSIBILITY_VIEW_DEFAULT = 0,
  /* Hidden from screen readers. */
  ZPD_WINUI3_ACCESSIBILITY_VIEW_RAW = 1,
  /* Read by screen readers, such as a named panel. */
  ZPD_WINUI3_ACCESSIBILITY_VIEW_CONTENT = 2,
};

/* Sets the automation name, help text, item status, and view; empty strings
   restore the element's own values. A changed item status is announced. */
int32_t zpd_winui3_element_set_accessibility(
    zpd_winui3_element* element,
    zpd_winui3_string name,
    zpd_winui3_string help_text,
    zpd_winui3_string item_status,
    int32_t view);
/* Names the element after `label`; null removes the link. */
int32_t zpd_winui3_element_set_labeled_by(
    zpd_winui3_element* element, const zpd_winui3_element* label);
int32_t zpd_winui3_element_focus(zpd_winui3_element* element);
/* The height of the element's top part under the window's title bar. */
double zpd_winui3_element_title_bar_inset(const zpd_winui3_element* element);

enum zpd_winui3_scroll_alignment {
  ZPD_WINUI3_SCROLL_NEAREST = 0,
  ZPD_WINUI3_SCROLL_START = 1,
  ZPD_WINUI3_SCROLL_CENTER = 2,
  ZPD_WINUI3_SCROLL_END = 3,
};

/* Scrolls the descendant with automation id `id` into view once the viewer is laid out. */
int32_t zpd_winui3_scroll_viewer_reveal(
    zpd_winui3_element* scroll_viewer, zpd_winui3_string id, int32_t alignment);

/* ---- Windows ---- */

enum zpd_winui3_toolbar_item_kind {
  ZPD_WINUI3_TOOLBAR_BUTTON = 0,
  ZPD_WINUI3_TOOLBAR_SEARCH = 1,
  ZPD_WINUI3_TOOLBAR_FLEXIBLE_SPACE = 2,
};

typedef struct zpd_winui3_toolbar_item {
  int32_t kind;
  zpd_winui3_string id;
  zpd_winui3_string label;
  zpd_winui3_string glyph;
  bool enabled;
  zpd_winui3_string value;
  zpd_winui3_string placeholder;
} zpd_winui3_toolbar_item;

/* Receives toolbar, close request, and undo events of the window. */
int32_t zpd_winui3_window_set_event_handler(
    zpd_winui3_window* window,
    const void* user_data,
    zpd_winui3_event_fn invoke,
    zpd_winui3_release_fn release);
/* Shows a command bar above the content; no items hide it. */
int32_t zpd_winui3_window_set_toolbar(
    zpd_winui3_window* window, const zpd_winui3_toolbar_item* items, size_t length);
/* While `confirms_close` is set, closing reports a close request instead. */
enum zpd_winui3_announcement_priority {
  /* Speaks only the latest of queued announcements. */
  ZPD_WINUI3_ANNOUNCEMENT_LOW = 0,
  /* Speaks every announcement in turn. */
  ZPD_WINUI3_ANNOUNCEMENT_MEDIUM = 1,
  /* Interrupts other speech. */
  ZPD_WINUI3_ANNOUNCEMENT_HIGH = 2,
};

/* Asks screen readers to speak `text` without moving focus. */
int32_t zpd_winui3_window_announce(
    zpd_winui3_window* window, zpd_winui3_string text, int32_t priority);
int32_t zpd_winui3_window_set_document(
    zpd_winui3_window* window, bool confirms_close, bool undo, bool can_undo, bool can_redo);
/* Restores the placement saved under `name` and saves it again when the window closes. */
int32_t zpd_winui3_window_set_autosave(zpd_winui3_window* window, zpd_winui3_string name);
/* Lets clicks under the title bar reach these elements instead of moving the window. */
int32_t zpd_winui3_window_set_passthrough(
    zpd_winui3_window* window, const zpd_winui3_element* const* elements, size_t length);

/* Runs `invoke` once when Windows ends the session while the app runs. */
void zpd_winui3_set_session_end_handler(
    const void* user_data, zpd_winui3_invoke_fn invoke, zpd_winui3_release_fn release);

#ifdef __cplusplus
}
#endif

#endif
