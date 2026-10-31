//! An `NSView` that a delegate draws with Core Graphics and that reports raw input.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::OnceLock;

use zpd_objc::{Id, RcIvar, Strong};

use crate::actor::{ActorRef, WindowEventKind};
use crate::geometry::{Point, Rect};
use crate::graphics::GraphicsContext;
use crate::native;
use crate::runloop::Application;

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {
    fn CACurrentMediaTime() -> f64;
}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    static NSRunLoopCommonModes: Id;
}

const LAYER_CONTENTS_REDRAW_DURING_VIEW_RESIZE: native::Integer = 2;
const TRACKING_MOUSE_ENTERED_AND_EXITED: u64 = 0x01;
const TRACKING_MOUSE_MOVED: u64 = 0x02;
const TRACKING_ACTIVE_IN_ACTIVE_APP: u64 = 0x40;
const TRACKING_IN_VISIBLE_RECT: u64 = 0x200;
/// Points scrolled per line for devices without precise deltas.
const LINE_HEIGHT: f64 = 10.0;

#[repr(C)]
#[derive(Clone, Copy)]
struct FrameRateRange {
    minimum: f32,
    maximum: f32,
    preferred: f32,
}

/// Timing and geometry of the frame a [`CanvasDelegate`] draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasFrame {
    /// The view bounds in flipped logical points.
    pub bounds: Rect,
    pub scale: f64,
    /// `CACurrentMediaTime` seconds at which the frame is expected to appear.
    pub time: f64,
    /// Seconds since the previous draw, or `0.0` for the first one.
    pub delta: f64,
}

/// Modifier keys held during an input event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub option: bool,
    pub command: bool,
}

impl Modifiers {
    fn from_flags(flags: u64) -> Self {
        Self {
            shift: flags & native::NS_EVENT_MODIFIER_FLAG_SHIFT != 0,
            control: flags & native::NS_EVENT_MODIFIER_FLAG_CONTROL != 0,
            option: flags & native::NS_EVENT_MODIFIER_FLAG_OPTION != 0,
            command: flags & native::NS_EVENT_MODIFIER_FLAG_COMMAND != 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    /// AppKit's `buttonNumber` for other buttons.
    Other(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MousePhase {
    Down,
    Up,
    /// Hovering, or dragging when a button is reported.
    Moved,
    Entered,
    Exited,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MouseInput {
    pub phase: MousePhase,
    pub button: Option<MouseButton>,
    pub location: Point,
    pub modifiers: Modifiers,
    pub click_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollInput {
    pub location: Point,
    /// Points; positive values move content right and down.
    pub delta_x: f64,
    pub delta_y: f64,
    pub precise: bool,
    pub modifiers: Modifiers,
}

/// A key identified by its virtual key code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedKey {
    Return,
    Tab,
    Space,
    Delete,
    ForwardDelete,
    Escape,
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,
    Home,
    End,
    PageUp,
    PageDown,
    Function(u8),
}

impl NamedKey {
    fn from_key_code(code: u16) -> Option<Self> {
        Some(match code {
            36 | 76 => Self::Return,
            48 => Self::Tab,
            49 => Self::Space,
            51 => Self::Delete,
            53 => Self::Escape,
            115 => Self::Home,
            116 => Self::PageUp,
            117 => Self::ForwardDelete,
            119 => Self::End,
            121 => Self::PageDown,
            123 => Self::LeftArrow,
            124 => Self::RightArrow,
            125 => Self::DownArrow,
            126 => Self::UpArrow,
            122 => Self::Function(1),
            120 => Self::Function(2),
            99 => Self::Function(3),
            118 => Self::Function(4),
            96 => Self::Function(5),
            97 => Self::Function(6),
            98 => Self::Function(7),
            100 => Self::Function(8),
            101 => Self::Function(9),
            109 => Self::Function(10),
            103 => Self::Function(11),
            111 => Self::Function(12),
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyInput {
    pub down: bool,
    pub named: Option<NamedKey>,
    /// `charactersIgnoringModifiers`.
    pub unmodified: String,
    /// Printable `characters`, or `None` for control and function keys.
    pub text: Option<String>,
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub key_code: u16,
}

/// Raw input a canvas view receives.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasInput {
    Mouse(MouseInput),
    Scroll(ScrollInput),
    Key(KeyInput),
    ModifiersChanged(Modifiers),
    Focus(bool),
}

/// Draws a canvas view and receives its input on the main thread.
pub trait CanvasDelegate {
    fn draw(&self, context: &mut GraphicsContext<'_>, frame: CanvasFrame);

    /// Returns whether the input was consumed; unconsumed input goes to the next responder.
    fn input(&self, input: &CanvasInput) -> bool;
}

/// When a canvas view draws.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CanvasRedraw {
    #[default]
    OnDemand,
    /// Draws on every display-link tick; `preferred_fps` caps the rate.
    Continuous { preferred_fps: Option<f32> },
}

struct CanvasState {
    actor: ActorRef,
    delegate: RefCell<Option<Rc<dyn CanvasDelegate>>>,
    focusable: Cell<bool>,
    display_link: RefCell<Option<Strong>>,
    /// The display link's target time for the next draw.
    target_time: Cell<Option<f64>>,
    last_time: Cell<Option<f64>>,
}

impl CanvasState {
    /// Sends input to the delegate, then to the event route if one is attached.
    fn deliver(&self, input: CanvasInput) -> bool {
        let delegate = self.delegate.borrow().clone();
        let handled = delegate.is_some_and(|delegate| delegate.input(&input));
        // Unrouted canvases skip the message loop, which matters for mouse moves.
        if self.actor.has_event_route()
            && let Some(tree) = self.actor.tree_handle()
        {
            tree.emit(&self.actor, WindowEventKind::CanvasInput(input));
        }
        handled
    }

    fn stop_display_link(&self) {
        if let Some(link) = self.display_link.borrow_mut().take() {
            // SAFETY: Invalidating removes the live link from its run loop and releases its target.
            unsafe { zpd_objc::msg_send!(link.as_ptr(), zpd_objc::sel!("invalidate"), () => ()) };
        }
    }
}

impl Drop for CanvasState {
    fn drop(&mut self) {
        self.stop_display_link();
    }
}

static STATE: RcIvar<CanvasState> = RcIvar::new(c"_zpdCanvasState");

/// Returns the canvas state of `object`, if it was created by [`CanvasView::new`].
///
/// # Safety
/// `object` must be an instance of the canvas view class.
unsafe fn state(object: Id) -> Option<Rc<CanvasState>> {
    // SAFETY: Guaranteed by the caller.
    unsafe { STATE.get(object) }
}

unsafe extern "C" fn draw_rect(object: Id, _: zpd_objc::Sel, _dirty: native::Rect) {
    // SAFETY: This method is installed only on the canvas view class.
    let Some(state) = (unsafe { state(object) }) else {
        return;
    };
    let Some(delegate) = state.delegate.borrow().clone() else {
        return;
    };
    // SAFETY: The receiver is a live NSView drawing inside AppKit's current graphics context.
    let (bounds, scale, context) = unsafe {
        let bounds = zpd_objc::msg_send!(object, zpd_objc::sel!("bounds"), () => native::Rect);
        let window = zpd_objc::msg_send!(object, zpd_objc::sel!("window"), () => Id);
        let scale = if window.is_null() {
            1.0
        } else {
            zpd_objc::msg_send!(window, zpd_objc::sel!("backingScaleFactor"), () => f64)
        };
        let current = zpd_objc::msg_send!(zpd_objc::class!("NSGraphicsContext"), zpd_objc::sel!("currentContext"), () => Id);
        if current.is_null() {
            return;
        }
        let context =
            zpd_objc::msg_send!(current, zpd_objc::sel!("CGContext"), () => *mut std::ffi::c_void);
        (Rect::from(bounds), scale, context)
    };
    if context.is_null() {
        return;
    }
    // SAFETY: CACurrentMediaTime has no preconditions.
    let time = state
        .target_time
        .take()
        .unwrap_or_else(|| unsafe { CACurrentMediaTime() });
    let delta = state
        .last_time
        .replace(Some(time))
        .map_or(0.0, |last| (time - last).max(0.0));
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: The context belongs to this drawRect: call and outlives the borrow.
        let mut graphics = unsafe { GraphicsContext::new(context) };
        delegate.draw(
            &mut graphics,
            CanvasFrame {
                bounds,
                scale,
                time,
                delta,
            },
        );
    });
}

unsafe extern "C" fn yes(_: Id, _: zpd_objc::Sel) -> bool {
    true
}

unsafe extern "C" fn accepts_first_responder(object: Id, _: zpd_objc::Sel) -> bool {
    // SAFETY: This method is installed only on the canvas view class.
    unsafe { state(object) }.is_some_and(|state| state.focusable.get())
}

unsafe extern "C" fn become_first_responder(object: Id, selector: zpd_objc::Sel) -> bool {
    // SAFETY: NSView implements the responder method being overridden.
    let accepted = unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), selector, () => bool)
    };
    // SAFETY: This method is installed only on the canvas view class.
    if accepted && let Some(state) = unsafe { state(object) } {
        zpd_objc::abort_on_unwind(|| state.deliver(CanvasInput::Focus(true)));
    }
    accepted
}

unsafe extern "C" fn resign_first_responder(object: Id, selector: zpd_objc::Sel) -> bool {
    // SAFETY: NSView implements the responder method being overridden.
    let accepted = unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), selector, () => bool)
    };
    // SAFETY: This method is installed only on the canvas view class.
    if accepted && let Some(state) = unsafe { state(object) } {
        zpd_objc::abort_on_unwind(|| state.deliver(CanvasInput::Focus(false)));
    }
    accepted
}

/// Converts an `NSEvent` received by `view` into canvas input.
///
/// # Safety
/// `view` must be a live canvas view and `event` the live NSEvent sent to `selector`.
unsafe fn input(view: Id, selector: &str, event: Id) -> Option<CanvasInput> {
    // SAFETY: Guaranteed by the caller; each message matches its event type.
    unsafe {
        let flags = zpd_objc::msg_send!(event, zpd_objc::sel!("modifierFlags"), () => u64);
        let modifiers = Modifiers::from_flags(flags);
        let location = || {
            let point =
                zpd_objc::msg_send!(event, zpd_objc::sel!("locationInWindow"), () => native::Point);
            Point::from(
                zpd_objc::msg_send!(view, zpd_objc::sel!("convertPoint:fromView:"), ((point): native::Point, (zpd_objc::NIL): Id) => native::Point),
            )
        };
        let other_button = || {
            let number =
                zpd_objc::msg_send!(event, zpd_objc::sel!("buttonNumber"), () => native::Integer);
            if number == 2 {
                MouseButton::Middle
            } else {
                MouseButton::Other(number.clamp(0, u8::MAX as native::Integer) as u8)
            }
        };
        let click_count = || {
            zpd_objc::msg_send!(event, zpd_objc::sel!("clickCount"), () => native::Integer) as u32
        };
        let mouse = |phase, button: Option<MouseButton>, click_count: u32| {
            Some(CanvasInput::Mouse(MouseInput {
                phase,
                button,
                location: location(),
                modifiers,
                click_count,
            }))
        };
        let key = |down: bool| {
            let key_code = zpd_objc::msg_send!(event, zpd_objc::sel!("keyCode"), () => u16);
            let characters = native::rust_string(
                zpd_objc::msg_send!(event, zpd_objc::sel!("characters"), () => Id),
            );
            let unmodified = native::rust_string(
                zpd_objc::msg_send!(event, zpd_objc::sel!("charactersIgnoringModifiers"), () => Id),
            );
            let repeat =
                down && zpd_objc::msg_send!(event, zpd_objc::sel!("isARepeat"), () => bool);
            // Control characters and AppKit's function-key code points are not text.
            let printable = !characters.is_empty()
                && characters
                    .chars()
                    .all(|c| !c.is_control() && !('\u{F700}'..='\u{F8FF}').contains(&c));
            Some(CanvasInput::Key(KeyInput {
                down,
                named: NamedKey::from_key_code(key_code),
                unmodified,
                text: printable.then_some(characters),
                modifiers,
                repeat,
                key_code,
            }))
        };
        match selector {
            "mouseDown:" => mouse(MousePhase::Down, Some(MouseButton::Left), click_count()),
            "mouseUp:" => mouse(MousePhase::Up, Some(MouseButton::Left), click_count()),
            "mouseDragged:" => mouse(MousePhase::Moved, Some(MouseButton::Left), 0),
            "rightMouseDown:" => mouse(MousePhase::Down, Some(MouseButton::Right), click_count()),
            "rightMouseUp:" => mouse(MousePhase::Up, Some(MouseButton::Right), click_count()),
            "rightMouseDragged:" => mouse(MousePhase::Moved, Some(MouseButton::Right), 0),
            "otherMouseDown:" => mouse(MousePhase::Down, Some(other_button()), click_count()),
            "otherMouseUp:" => mouse(MousePhase::Up, Some(other_button()), click_count()),
            "otherMouseDragged:" => mouse(MousePhase::Moved, Some(other_button()), 0),
            "mouseMoved:" => mouse(MousePhase::Moved, None, 0),
            "mouseEntered:" => mouse(MousePhase::Entered, None, 0),
            "mouseExited:" => mouse(MousePhase::Exited, None, 0),
            "scrollWheel:" => {
                let precise = zpd_objc::msg_send!(event, zpd_objc::sel!("hasPreciseScrollingDeltas"), () => bool);
                let scale = if precise { 1.0 } else { LINE_HEIGHT };
                Some(CanvasInput::Scroll(ScrollInput {
                    location: location(),
                    delta_x: zpd_objc::msg_send!(event, zpd_objc::sel!("scrollingDeltaX"), () => f64)
                        * scale,
                    delta_y: zpd_objc::msg_send!(event, zpd_objc::sel!("scrollingDeltaY"), () => f64)
                        * scale,
                    precise,
                    modifiers,
                }))
            }
            "keyDown:" => key(true),
            "keyUp:" => key(false),
            "flagsChanged:" => Some(CanvasInput::ModifiersChanged(modifiers)),
            _ => None,
        }
    }
}

/// Generates an event override that forwards unconsumed events to `NSView`.
macro_rules! event_methods {
    ($($function:ident => $selector:literal),* $(,)?) => {
        $(
            unsafe extern "C" fn $function(object: Id, selector: zpd_objc::Sel, event: Id) {
                // SAFETY: These methods are installed only on the canvas view class.
                let handled = unsafe { state(object) }.is_some_and(|state| {
                    zpd_objc::abort_on_unwind(|| {
                        if $selector == "mouseDown:" {
                            // SAFETY: The receiver is a live canvas view.
                            unsafe { CanvasView::focus_on_click(object, &state) };
                        }
                        // SAFETY: AppKit sent `event` with this selector.
                        unsafe { input(object, $selector, event) }
                            .is_some_and(|input| state.deliver(input))
                    })
                });
                if !handled {
                    // SAFETY: NSView implements every overridden event method.
                    unsafe {
                        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), selector, ((event): Id) => ())
                    };
                }
            }
        )*
    };
}

event_methods!(
    mouse_down => "mouseDown:",
    mouse_up => "mouseUp:",
    mouse_dragged => "mouseDragged:",
    right_mouse_down => "rightMouseDown:",
    right_mouse_up => "rightMouseUp:",
    right_mouse_dragged => "rightMouseDragged:",
    other_mouse_down => "otherMouseDown:",
    other_mouse_up => "otherMouseUp:",
    other_mouse_dragged => "otherMouseDragged:",
    mouse_moved => "mouseMoved:",
    mouse_entered => "mouseEntered:",
    mouse_exited => "mouseExited:",
    scroll_wheel => "scrollWheel:",
    key_down => "keyDown:",
    key_up => "keyUp:",
    flags_changed => "flagsChanged:",
);

unsafe extern "C" fn backing_properties_changed(object: Id, selector: zpd_objc::Sel) {
    // SAFETY: NSView implements this notification method; the receiver is a live view.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), selector, () => ());
        zpd_objc::msg_send!(object, zpd_objc::sel!("setNeedsDisplay:"), ((true): bool) => ());
    }
}

unsafe extern "C" fn dealloc(object: Id, _: zpd_objc::Sel) {
    // SAFETY: The receiver is a canvas view; its Rust state is released before NSView finishes.
    unsafe {
        drop(STATE.take(object));
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), zpd_objc::sel!("dealloc"), () => ());
    }
}

/// A layer-backed, flipped `NSView` drawn by a [`CanvasDelegate`].
pub struct CanvasView {
    view: ViewActor,
    state: Rc<CanvasState>,
}

impl CanvasView {
    fn class() -> zpd_objc::Class {
        static CLASS: OnceLock<usize> = OnceLock::new();
        *CLASS.get_or_init(|| {
            zpd_objc::decl!(ZpdRustCanvasView: [zpd_objc::class!("NSView")] {
                fields { _zpdCanvasState: ptr }
                methods {
                    "drawRect:": "v@:{CGRect={CGPoint=dd}{CGSize=dd}}" => draw_rect,
                    "isFlipped": "B@:" => yes,
                    "acceptsFirstResponder": "B@:" => accepts_first_responder,
                    "becomeFirstResponder": "B@:" => become_first_responder,
                    "resignFirstResponder": "B@:" => resign_first_responder,
                    "mouseDown:": "v@:@" => mouse_down,
                    "mouseUp:": "v@:@" => mouse_up,
                    "mouseDragged:": "v@:@" => mouse_dragged,
                    "rightMouseDown:": "v@:@" => right_mouse_down,
                    "rightMouseUp:": "v@:@" => right_mouse_up,
                    "rightMouseDragged:": "v@:@" => right_mouse_dragged,
                    "otherMouseDown:": "v@:@" => other_mouse_down,
                    "otherMouseUp:": "v@:@" => other_mouse_up,
                    "otherMouseDragged:": "v@:@" => other_mouse_dragged,
                    "mouseMoved:": "v@:@" => mouse_moved,
                    "mouseEntered:": "v@:@" => mouse_entered,
                    "mouseExited:": "v@:@" => mouse_exited,
                    "scrollWheel:": "v@:@" => scroll_wheel,
                    "keyDown:": "v@:@" => key_down,
                    "keyUp:": "v@:@" => key_up,
                    "flagsChanged:": "v@:@" => flags_changed,
                    "viewDidChangeBackingProperties": "v@:" => backing_properties_changed,
                    "dealloc": "v@:" => dealloc,
                }
            }) as usize
        }) as zpd_objc::Class
    }

    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: The registered NSView subclass uses NSView's designated frame initializer.
        let native = unsafe {
            let object = zpd_objc::msg_send!(Self::class(), zpd_objc::sel!("alloc"), () => Id);
            Strong::from_retained(zpd_objc::msg_send!(object, zpd_objc::sel!("initWithFrame:"), ((native::Rect::default()): native::Rect) => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let view = ViewActor::new(application, native);
        let state = Rc::new(CanvasState {
            actor: view.actor().clone(),
            delegate: RefCell::new(None),
            focusable: Cell::new(false),
            display_link: RefCell::new(None),
            target_time: Cell::new(None),
            last_time: Cell::new(None),
        });
        view.as_view().with(|object| {
            // SAFETY: The view is a live canvas view with the state ivar; AppKit retains the tracking area.
            unsafe {
                STATE.set(object, state.clone());
                zpd_objc::msg_send!(object, zpd_objc::sel!("setWantsLayer:"), ((true): bool) => ());
                zpd_objc::msg_send!(object, zpd_objc::sel!("setLayerContentsRedrawPolicy:"), ((LAYER_CONTENTS_REDRAW_DURING_VIEW_RESIZE): native::Integer) => ());
                zpd_objc::msg_send!(object, zpd_objc::sel!("setClipsToBounds:"), ((true): bool) => ());
                let options = TRACKING_MOUSE_ENTERED_AND_EXITED
                    | TRACKING_MOUSE_MOVED
                    | TRACKING_ACTIVE_IN_ACTIVE_APP
                    | TRACKING_IN_VISIBLE_RECT;
                let area = zpd_objc::msg_send!(zpd_objc::class!("NSTrackingArea"), zpd_objc::sel!("alloc"), () => Id);
                let area = Strong::from_retained(zpd_objc::msg_send!(area, zpd_objc::sel!("initWithRect:options:owner:userInfo:"), ((native::Rect::default()): native::Rect, (options): u64, (object): Id, (zpd_objc::NIL): Id) => Id))
                    .expect("NSTrackingArea allocation failed");
                zpd_objc::msg_send!(object, zpd_objc::sel!("addTrackingArea:"), ((area.as_ptr()): Id) => ());
            }
        })?;
        Ok(Self { view, state })
    }

    pub fn set_delegate(&self, delegate: Rc<dyn CanvasDelegate>) {
        *self.state.delegate.borrow_mut() = Some(delegate);
    }

    /// Returns a handle that does not keep the view alive.
    pub fn handle(&self) -> CanvasHandle {
        CanvasHandle {
            actor: self.view.actor().clone(),
        }
    }

    pub fn set_redraw(&self, redraw: CanvasRedraw) -> Result<(), UiError> {
        let CanvasRedraw::Continuous { preferred_fps } = redraw else {
            self.state.stop_display_link();
            return Ok(());
        };
        let range = preferred_fps.map_or(
            FrameRateRange {
                minimum: 0.0,
                maximum: 0.0,
                preferred: 0.0,
            },
            |fps| FrameRateRange {
                minimum: 0.0,
                maximum: fps,
                preferred: fps,
            },
        );
        if let Some(link) = self.state.display_link.borrow().as_ref() {
            // SAFETY: The link is a live CADisplayLink.
            unsafe {
                zpd_objc::msg_send!(link.as_ptr(), zpd_objc::sel!("setPreferredFrameRateRange:"), ((range): FrameRateRange) => ())
            };
            return Ok(());
        }
        let actor = self.view.actor().clone();
        let state = Rc::downgrade(&self.state);
        let target = callback::target(move |link| Self::tick(&actor, &state, link));
        let link = self.as_view().with(|view| {
            // SAFETY: NSView vends a CADisplayLink that retains `target`; it is added to the main run loop.
            unsafe {
                let link = Strong::retain(zpd_objc::msg_send!(view, zpd_objc::sel!("displayLinkWithTarget:selector:"), ((target.as_ptr()): Id, (zpd_objc::sel!("invoke:")): zpd_objc::Sel) => Id))
                    .ok_or(UiError::NativeCreationFailed)?;
                zpd_objc::msg_send!(link.as_ptr(), zpd_objc::sel!("setPreferredFrameRateRange:"), ((range): FrameRateRange) => ());
                let run_loop = zpd_objc::msg_send!(zpd_objc::class!("NSRunLoop"), zpd_objc::sel!("mainRunLoop"), () => Id);
                zpd_objc::msg_send!(link.as_ptr(), zpd_objc::sel!("addToRunLoop:forMode:"), ((run_loop): Id, (NSRunLoopCommonModes): Id) => ());
                Ok::<_, UiError>(link)
            }
        })??;
        *self.state.display_link.borrow_mut() = Some(link);
        Ok(())
    }

    /// Records the display link's target time and schedules a draw.
    fn tick(actor: &ActorRef, state: &Weak<CanvasState>, link: Id) {
        let Some(state) = state.upgrade() else {
            return;
        };
        // SAFETY: The action sender is the live CADisplayLink.
        let target =
            unsafe { zpd_objc::msg_send!(link, zpd_objc::sel!("targetTimestamp"), () => f64) };
        state.target_time.set(Some(target));
        let _ = actor.with(|view| {
            // SAFETY: The actor's native object is a live NSView.
            unsafe {
                zpd_objc::msg_send!(view, zpd_objc::sel!("setNeedsDisplay:"), ((true): bool) => ())
            }
        });
    }

    /// Lets clicks and [`CanvasView::focus`] make the view the first responder.
    pub fn set_focusable(&self, focusable: bool) {
        self.state.focusable.set(focusable);
    }

    /// Focuses a focusable canvas before it receives the click.
    ///
    /// # Safety
    /// `view` must be a live canvas view.
    unsafe fn focus_on_click(view: Id, state: &CanvasState) {
        if !state.focusable.get() {
            return;
        }
        // SAFETY: Guaranteed by the caller; the window is checked for nil.
        unsafe {
            let window = zpd_objc::msg_send!(view, zpd_objc::sel!("window"), () => Id);
            if !window.is_null()
                && zpd_objc::msg_send!(window, zpd_objc::sel!("firstResponder"), () => Id) != view
            {
                zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((view): Id) => bool);
            }
        }
    }
}

/// A non-owning handle for redraw and focus requests on a [`CanvasView`].
#[derive(Clone)]
pub struct CanvasHandle {
    actor: ActorRef,
}

impl CanvasHandle {
    fn as_view(&self) -> ViewRef<'_> {
        ViewRef::from_actor(&self.actor)
    }

    /// Schedules a draw at the next display refresh.
    pub fn set_needs_display(&self) -> Result<(), UiError> {
        self.as_view().with(|view| {
            // SAFETY: The receiver is a live NSView.
            unsafe {
                zpd_objc::msg_send!(view, zpd_objc::sel!("setNeedsDisplay:"), ((true): bool) => ())
            }
        })
    }

    /// Makes the view its window's first responder; returns whether it accepted.
    pub fn focus(&self) -> Result<bool, UiError> {
        self.as_view().with(|view| {
            // SAFETY: The receiver is a live NSView; a nil window cannot focus it.
            unsafe {
                let window = zpd_objc::msg_send!(view, zpd_objc::sel!("window"), () => Id);
                !window.is_null()
                    && zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((view): Id) => bool)
            }
        })
    }

    /// Gives up first responder status if the view has it.
    pub fn resign_focus(&self) -> Result<(), UiError> {
        self.as_view().with(|view| {
            // SAFETY: The receiver is a live NSView; the window is checked for nil.
            unsafe {
                let window = zpd_objc::msg_send!(view, zpd_objc::sel!("window"), () => Id);
                if !window.is_null()
                    && zpd_objc::msg_send!(window, zpd_objc::sel!("firstResponder"), () => Id) == view
                {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((zpd_objc::NIL): Id) => bool);
                }
            }
        })
    }
}

impl AsView for CanvasView {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
