#![allow(dead_code)]

use std::ffi::{c_int, c_long};

use zpd_objc::{Id, Strong, msg_send};

pub type CFloat = f64;
pub type Integer = c_long;
pub type Boolean = i8;

pub const NS_UTF8_STRING_ENCODING: u64 = 4;

pub const NS_APPLICATION_ACTIVATION_POLICY_REGULAR: Integer = 0;
pub const NS_EVENT_TYPE_APPLICATION_DEFINED: Integer = 15;

pub const NS_EVENT_MODIFIER_FLAG_SHIFT: u64 = 1 << 17;
pub const NS_EVENT_MODIFIER_FLAG_CONTROL: u64 = 1 << 18;
pub const NS_EVENT_MODIFIER_FLAG_OPTION: u64 = 1 << 19;
pub const NS_EVENT_MODIFIER_FLAG_COMMAND: u64 = 1 << 20;

pub const NS_WINDOW_STYLE_MASK_TITLED: u64 = 1 << 0;
pub const NS_WINDOW_STYLE_MASK_CLOSABLE: u64 = 1 << 1;
pub const NS_WINDOW_STYLE_MASK_MINIATURIZABLE: u64 = 1 << 2;
pub const NS_WINDOW_STYLE_MASK_RESIZABLE: u64 = 1 << 3;
pub const NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW: u64 = 1 << 15;
pub const NS_BACKING_STORE_BUFFERED: u64 = 2;

pub const NS_TITLEBAR_SEPARATOR_STYLE_AUTOMATIC: Integer = 0;
pub const NS_TITLEBAR_SEPARATOR_STYLE_NONE: Integer = 1;
pub const NS_TOOLBAR_DISPLAY_MODE_ICON_ONLY: u64 = 2;
pub const NS_LINE_BREAK_BY_WORD_WRAPPING: u64 = 0;
pub const NS_LINE_BREAK_BY_CLIPPING: u64 = 2;
pub const NS_LINE_BREAK_BY_TRUNCATING_TAIL: u64 = 4;
pub const NS_TABLE_COLUMN_AUTORESIZING_MASK: u64 = 1 << 0;
pub const NS_TABLE_VIEW_STYLE_SOURCE_LIST: Integer = 3;
pub const NS_TABLE_VIEW_ROW_SIZE_STYLE_CUSTOM: Integer = 0;

pub const NS_WINDOW_STYLE_MASK_UTILITY_WINDOW: u64 = 1 << 4;
pub const NS_CONTROL_STATE_VALUE_ON: Integer = 1;
pub const NS_SEGMENT_SWITCH_TRACKING_SELECT_ONE: u64 = 0;
pub const NS_SEGMENT_DISTRIBUTION_FILL_EQUALLY: Integer = 2;
pub const NS_NUMBER_FORMATTER_DECIMAL_STYLE: u64 = 1;
pub const NS_VIEW_WIDTH_SIZABLE: u64 = 1 << 1;
pub const NS_VIEW_HEIGHT_SIZABLE: u64 = 1 << 4;
pub const NS_VIEW_MIN_X_MARGIN: u64 = 1 << 0;
pub const NS_IMAGE_LEADING: u64 = 7;
pub const NS_IMAGE_ONLY: u64 = 1;
pub const NS_POPOVER_BEHAVIOR_TRANSIENT: Integer = 1;
pub const NS_ALERT_FIRST_BUTTON_RETURN: Integer = 1000;
pub const NS_MODAL_RESPONSE_OK: Integer = 1;
pub const NS_MODAL_RESPONSE_ABORT: Integer = -1001;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: CFloat,
    pub y: CFloat,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
    pub width: CFloat,
    pub height: CFloat,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    pub static NSFontWeightRegular: CFloat;
}
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}
#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {}
#[link(name = "UniformTypeIdentifiers", kind = "framework")]
unsafe extern "C" {}
unsafe extern "C" {
    pub fn pthread_main_np() -> c_int;
}

pub fn nsstring(value: &str) -> Strong {
    // SAFETY: NSString copies the supplied UTF-8 bytes during initialization.
    unsafe {
        let allocated = msg_send!(zpd_objc::class!("NSString"), zpd_objc::sel!("alloc"), () => Id);
        Strong::from_retained(msg_send!(allocated, zpd_objc::sel!("initWithBytes:length:encoding:"), ((value.as_ptr()): *const u8, (value.len()): usize, (NS_UTF8_STRING_ENCODING): u64) => Id))
        .expect("NSString allocation failed")
    }
}

pub unsafe fn rust_string(value: Id) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: Both messages are valid for NSString.
    let length = unsafe {
        msg_send!(value, zpd_objc::sel!("lengthOfBytesUsingEncoding:"), ((NS_UTF8_STRING_ENCODING): u64) => usize)
    };
    let bytes = unsafe {
        msg_send!(value, zpd_objc::sel!("cStringUsingEncoding:"), ((NS_UTF8_STRING_ENCODING): u64) => *const u8)
    };
    if bytes.is_null() {
        return String::new();
    }
    // SAFETY: NSString exposes at least `length` UTF-8 bytes at this pointer.
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(bytes, length) }).into_owned()
}

pub fn alloc_init(class: zpd_objc::Class) -> Strong {
    // SAFETY: The named Objective-C class implements alloc/init.
    unsafe {
        let value = msg_send!(class, zpd_objc::sel!("alloc"), () => Id);
        Strong::from_retained(msg_send!(value, zpd_objc::sel!("init"), () => Id))
            .expect("Objective-C allocation failed")
    }
}

/// Builds an immutable-by-convention `NSMutableArray` of `NSString`s.
pub fn nsstring_array<S: AsRef<str>>(values: &[S]) -> Strong {
    let array = alloc_init(zpd_objc::class!("NSMutableArray"));
    for value in values {
        let value = nsstring(value.as_ref());
        // SAFETY: NSMutableArray retains the live NSString it receives.
        unsafe {
            msg_send!(array.as_ptr(), zpd_objc::sel!("addObject:"), ((value.as_ptr()): Id) => ())
        };
    }
    array
}

/// Returns an autoreleased SF Symbol image, or nil when the name is unknown.
pub fn symbol_image(name: &str, description: &str) -> Id {
    let name = nsstring(name);
    let description = nsstring(description);
    // SAFETY: The class factory accepts live NSString arguments and may return nil.
    unsafe {
        msg_send!(zpd_objc::class!("NSImage"), zpd_objc::sel!("imageWithSystemSymbolName:accessibilityDescription:"), ((name.as_ptr()): Id, (description.as_ptr()): Id) => Id)
    }
}
