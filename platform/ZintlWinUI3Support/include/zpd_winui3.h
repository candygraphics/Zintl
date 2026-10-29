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
} zpd_winui3_event;

typedef struct zpd_winui3_navigation_item {
  /* Headers are nonselectable section titles. */
  bool header;
  zpd_winui3_string id;
  zpd_winui3_string title;
  zpd_winui3_string glyph;
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
/* Moves and resizes the window in device-independent pixels. */
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

#ifdef __cplusplus
}
#endif

#endif
