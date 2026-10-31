//! Canvas views that a handler draws outside the view update cycle.
//!
//! A [`CanvasNode`] carries a [`Canvas2DHandler`] through the render tree.
//! The backend keeps the handler of the first mounted node; later renders hand
//! their handler to [`Canvas2DHandler::reconcile`] instead of replacing it, so
//! drawing state survives view updates.

use std::any::{Any, TypeId, type_name};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use zintl_ui_graphics::{DrawCommand, Graphics2D, Point, RecordingBackend, Rect};

use crate::MainTaskSender;

/// Whether a handler needs to draw again after [`Canvas2DHandler::reconcile`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redraw {
    Yes,
    No,
}

/// When a canvas draws.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RedrawPolicy {
    /// Draws after a resize, a reconcile that returns [`Redraw::Yes`], or an
    /// invalidation. The native view keeps the last frame meanwhile.
    #[default]
    OnDemand,
    /// Draws every display refresh, like `requestAnimationFrame`.
    ///
    /// `preferred_fps` caps the rate; `None` follows the display.
    Continuous { preferred_fps: Option<f32> },
}

/// Timing and geometry of the frame being drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The canvas bounds in logical points; the origin is the top-left.
    pub bounds: Rect,
    /// Device pixels per logical point.
    pub scale: f64,
    /// Seconds on a monotonic clock at which the frame is expected to appear.
    pub time: f64,
    /// Seconds since the previous frame, or `0.0` for the first one.
    pub delta: f64,
}

impl Frame {
    pub fn width(&self) -> f64 {
        self.bounds.width
    }

    pub fn height(&self) -> f64 {
        self.bounds.height
    }
}

/// Draws a canvas and handles its input.
///
/// All methods run on the main thread, which also builds views. `Clone` lets
/// the declaring view be rendered repeatedly.
pub trait Canvas2DHandler: Clone + 'static {
    /// Runs once before the first draw or event.
    fn setup(&mut self, cx: &mut CanvasSetup) {
        let _ = cx;
    }

    /// Draws one frame.
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame);

    /// Handles pointer, keyboard, and focus input; returns whether it was consumed.
    ///
    /// Unconsumed input continues to the platform, for example to a parent
    /// scroll view or the window's key handling.
    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        let _ = (event, cx);
        false
    }

    /// Receives the handler from a later render of the same canvas.
    ///
    /// The default replaces `self` and draws again. Override it to keep state
    /// that only the handler owns, such as hover or cached resources.
    fn reconcile(&mut self, next: Self) -> Redraw {
        *self = next;
        Redraw::Yes
    }
}

/// Context for [`Canvas2DHandler::setup`].
pub struct CanvasSetup {
    invalidator: Invalidator,
}

impl CanvasSetup {
    /// Returns a handle that requests a redraw from any thread.
    pub fn invalidator(&self) -> Invalidator {
        self.invalidator.clone()
    }
}

/// Requests made while handling a [`CanvasEvent`]; they apply after the handler returns.
#[derive(Debug, Default)]
pub struct CanvasEventContext {
    redraw: bool,
    focus: Option<bool>,
}

impl CanvasEventContext {
    /// Draws again at the next display refresh without rebuilding views.
    pub fn invalidate(&mut self) {
        self.redraw = true;
    }

    /// Makes the canvas receive keyboard input. Requires a focusable canvas.
    pub fn request_focus(&mut self) {
        self.focus = Some(true);
    }

    pub fn resign_focus(&mut self) {
        self.focus = Some(false);
    }
}

/// Requests a redraw of a mounted canvas from any thread.
///
/// The request is queued to the main thread, so it never draws synchronously.
/// It does nothing before the canvas is mounted or after it is removed.
#[derive(Clone)]
pub struct Invalidator {
    canvas: u64,
    sender: Option<Arc<MainTaskSender>>,
}

impl Invalidator {
    /// Queues a redraw; returns `false` when the main thread no longer accepts work.
    pub fn invalidate(&self) -> bool {
        let Some(sender) = &self.sender else {
            return false;
        };
        let canvas = self.canvas;
        sender(Box::new(move || CanvasRegistry::invalidate(canvas)))
    }
}

impl fmt::Debug for Invalidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Invalidator")
            .field("canvas", &self.canvas)
            .finish_non_exhaustive()
    }
}

/// Native operations a backend provides to its mounted canvas.
pub trait CanvasHost {
    /// Schedules a draw at the next display refresh.
    fn invalidate(&self);
    fn set_focus(&self, focused: bool);
}

/// The handler methods without the `Sized` parts of [`Canvas2DHandler`].
trait ErasedHandler {
    fn setup(&mut self, cx: &mut CanvasSetup);
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame);
    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool;
    /// `next` is a `Box<Self>` produced by [`ErasedHandler::into_any`].
    fn reconcile(&mut self, next: Box<dyn Any>) -> Redraw;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<H: Canvas2DHandler> ErasedHandler for H {
    fn setup(&mut self, cx: &mut CanvasSetup) {
        Canvas2DHandler::setup(self, cx);
    }

    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        Canvas2DHandler::draw(self, g, frame);
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        Canvas2DHandler::event(self, event, cx)
    }

    fn reconcile(&mut self, next: Box<dyn Any>) -> Redraw {
        let next = next
            .downcast::<H>()
            .expect("canvases reconcile only handlers of the same type");
        Canvas2DHandler::reconcile(self, *next)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

struct CanvasCell {
    id: u64,
    handler_type: TypeId,
    handler_name: &'static str,
    /// `None` after a mounted canvas adopted this handler.
    handler: RefCell<Option<Box<dyn ErasedHandler>>>,
    host: RefCell<Option<Rc<dyn CanvasHost>>>,
    sender: RefCell<Option<Arc<MainTaskSender>>>,
    set_up: Cell<bool>,
}

impl Drop for CanvasCell {
    fn drop(&mut self) {
        CanvasRegistry::remove(self.id);
    }
}

/// A canvas handler shared between the render tree and the native view.
///
/// Nodes compare equal only when they share the same handler instance.
#[derive(Clone)]
pub struct CanvasNode(Rc<CanvasCell>);

impl CanvasNode {
    pub fn new<H: Canvas2DHandler>(handler: H) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self(Rc::new(CanvasCell {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            handler_type: TypeId::of::<H>(),
            handler_name: type_name::<H>(),
            handler: RefCell::new(Some(Box::new(handler))),
            host: RefCell::new(None),
            sender: RefCell::new(None),
            set_up: Cell::new(false),
        }))
    }

    /// Returns the concrete handler type; canvases of different types are different kinds.
    pub fn handler_type(&self) -> TypeId {
        self.0.handler_type
    }

    /// Connects this canvas to its native view.
    ///
    /// `sender` queues work to the main thread for [`Invalidator`]s.
    pub fn attach(&self, host: Rc<dyn CanvasHost>, sender: Option<Arc<MainTaskSender>>) {
        *self.0.host.borrow_mut() = Some(host);
        *self.0.sender.borrow_mut() = sender;
        CanvasRegistry::insert(self.0.id, Rc::downgrade(&self.0));
    }

    /// Disconnects the native view; later invalidations do nothing.
    pub fn detach(&self) {
        self.0.host.borrow_mut().take();
        CanvasRegistry::remove(self.0.id);
    }

    /// Moves the handler of a re-rendered `next` node into this mounted canvas.
    pub fn adopt(&self, next: &CanvasNode) {
        if Rc::ptr_eq(&self.0, &next.0) {
            return;
        }
        let Some(incoming) = next.0.handler.borrow_mut().take() else {
            return;
        };
        let redraw = {
            let mut handler = self
                .0
                .handler
                .try_borrow_mut()
                .expect("a canvas cannot be re-rendered while it draws");
            match handler.as_mut() {
                Some(handler) => handler.reconcile(incoming.into_any()),
                None => {
                    *handler = Some(incoming);
                    Redraw::Yes
                }
            }
        };
        if redraw == Redraw::Yes {
            self.invalidate();
        }
    }

    /// Draws one frame with the handler.
    pub fn draw(&self, g: &mut Graphics2D<'_>, frame: &Frame) {
        self.with_handler(|handler| handler.draw(g, frame));
    }

    /// Delivers input to the handler and applies its requests; returns whether it was consumed.
    pub fn event(&self, event: &CanvasEvent) -> bool {
        let mut cx = CanvasEventContext::default();
        let handled = self
            .with_handler(|handler| handler.event(event, &mut cx))
            .unwrap_or(false);
        // Hosts may deliver focus events synchronously, so the handler is released first.
        if cx.redraw {
            self.invalidate();
        }
        if let Some(focused) = cx.focus
            && let Some(host) = self.host()
        {
            host.set_focus(focused);
        }
        handled
    }

    /// Schedules a draw if the canvas is mounted.
    pub fn invalidate(&self) {
        if let Some(host) = self.host() {
            host.invalidate();
        }
    }

    /// Draws one frame into a [`RecordingBackend`] and returns its commands.
    pub fn record(&self, frame: &Frame) -> Vec<DrawCommand> {
        let mut backend = RecordingBackend::new();
        let mut graphics = Graphics2D::new(&mut backend, frame.bounds, frame.scale);
        self.draw(&mut graphics, frame);
        backend.into_commands()
    }

    fn host(&self) -> Option<Rc<dyn CanvasHost>> {
        self.0.host.borrow().clone()
    }

    /// Runs `operation` on the handler, setting it up first; `None` if it was adopted.
    fn with_handler<R>(&self, operation: impl FnOnce(&mut dyn ErasedHandler) -> R) -> Option<R> {
        let mut handler = self
            .0
            .handler
            .try_borrow_mut()
            .expect("canvas handlers must not re-enter themselves");
        let handler = handler.as_mut()?;
        if !self.0.set_up.replace(true) {
            let mut cx = CanvasSetup {
                invalidator: Invalidator {
                    canvas: self.0.id,
                    sender: self.0.sender.borrow().clone(),
                },
            };
            handler.setup(&mut cx);
        }
        Some(operation(handler.as_mut()))
    }
}

impl PartialEq for CanvasNode {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for CanvasNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CanvasNode")
            .field("id", &self.0.id)
            .field("handler", &self.0.handler_name)
            .finish()
    }
}

/// Mounted canvases of the main thread, addressed by [`Invalidator`]s.
struct CanvasRegistry;

thread_local! {
    static CANVASES: RefCell<HashMap<u64, Weak<CanvasCell>>> = RefCell::new(HashMap::new());
}

impl CanvasRegistry {
    fn insert(id: u64, canvas: Weak<CanvasCell>) {
        CANVASES.with(|canvases| canvases.borrow_mut().insert(id, canvas));
    }

    fn remove(id: u64) {
        // The registry may already be gone while the thread shuts down.
        let _ = CANVASES.try_with(|canvases| canvases.borrow_mut().remove(&id));
    }

    fn invalidate(id: u64) {
        let canvas = CANVASES.with(|canvases| canvases.borrow().get(&id).and_then(Weak::upgrade));
        if let Some(canvas) = canvas {
            CanvasNode(canvas).invalidate();
        }
    }
}

/// Keyboard modifier keys held during an input event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    /// Option on macOS.
    pub alt: bool,
    /// Command on macOS and the Windows key on Windows.
    pub meta: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    /// Another button, numbered from 0 for the primary button.
    Other(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Down,
    Up,
    /// Hovering, or dragging when [`PointerEvent::button`] is set.
    Moved,
    Entered,
    Exited,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    pub phase: PointerPhase,
    /// The pressed button for `Down` and `Up`, or the dragging button for `Moved`.
    pub button: Option<PointerButton>,
    /// Location in canvas points from the top-left.
    pub location: Point,
    pub modifiers: Modifiers,
    /// Consecutive clicks for `Down` and `Up`, such as 2 for a double click.
    pub click_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollEvent {
    pub location: Point,
    /// Scroll distance in points; positive values move content right and down.
    pub delta_x: f64,
    pub delta_y: f64,
    /// Whether the device reports precise deltas, such as a trackpad.
    pub precise: bool,
    pub modifiers: Modifiers,
}

/// A key that does not produce text by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Escape,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    /// A function key from F1.
    F(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Named(NamedKey),
    /// The character the key produces without modifiers, such as `"a"`.
    Character(String),
    Unidentified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPhase {
    Down,
    Up,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyEvent {
    pub phase: KeyPhase,
    pub key: Key,
    /// Text the key inserts with its modifiers applied, if any.
    pub text: Option<String>,
    pub modifiers: Modifiers,
    /// Whether this `Down` repeats because the key is held.
    pub repeat: bool,
    /// The platform's hardware key code.
    pub code: u32,
}

/// Input delivered to a canvas.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasEvent {
    Pointer(PointerEvent),
    Scroll(ScrollEvent),
    Key(KeyEvent),
    ModifiersChanged(Modifiers),
    /// The canvas gained (`true`) or lost keyboard focus.
    Focus(bool),
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use zintl_ui_graphics::{Color, Path};

    use super::*;

    /// Draws a `value`-wide bar, plus a marker while hovered.
    #[derive(Clone)]
    struct Bar {
        value: u32,
        hover: bool,
        setups: Rc<Cell<u32>>,
    }

    impl Bar {
        fn new(value: u32) -> Self {
            Self {
                value,
                hover: false,
                setups: Rc::new(Cell::new(0)),
            }
        }
    }

    impl Canvas2DHandler for Bar {
        fn setup(&mut self, _cx: &mut CanvasSetup) {
            self.setups.set(self.setups.get() + 1);
        }

        fn draw(&mut self, g: &mut Graphics2D<'_>, _frame: &Frame) {
            g.fill_rect(Rect::from_size(self.value as f64, 1.0), Color::BLACK);
            if self.hover {
                g.fill_rect(Rect::from_size(1.0, 1.0), Color::WHITE);
            }
        }

        fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
            let CanvasEvent::Pointer(pointer) = event else {
                return false;
            };
            self.hover = pointer.phase != PointerPhase::Exited;
            cx.invalidate();
            cx.request_focus();
            true
        }

        fn reconcile(&mut self, next: Self) -> Redraw {
            let redraw = if next.value == self.value {
                Redraw::No
            } else {
                Redraw::Yes
            };
            self.value = next.value;
            redraw
        }
    }

    /// Reports the native requests a canvas makes.
    #[derive(Default)]
    struct Host {
        invalidations: Cell<u32>,
        focus: Cell<Option<bool>>,
    }

    impl CanvasHost for Host {
        fn invalidate(&self) {
            self.invalidations.set(self.invalidations.get() + 1);
        }

        fn set_focus(&self, focused: bool) {
            self.focus.set(Some(focused));
        }
    }

    fn frame() -> Frame {
        Frame {
            bounds: Rect::from_size(10.0, 10.0),
            scale: 2.0,
            time: 0.0,
            delta: 0.0,
        }
    }

    fn pointer(phase: PointerPhase) -> CanvasEvent {
        CanvasEvent::Pointer(PointerEvent {
            phase,
            button: None,
            location: Point::ZERO,
            modifiers: Modifiers::default(),
            click_count: 0,
        })
    }

    fn filled_paths(commands: &[DrawCommand]) -> Vec<Path> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Fill { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn adopt_keeps_mounted_handler_state() {
        // Verifies a re-render reconciles into the mounted handler instead of replacing it.
        let mounted = CanvasNode::new(Bar::new(1));
        mounted.event(&pointer(PointerPhase::Entered));
        mounted.adopt(&CanvasNode::new(Bar::new(4)));

        assert_eq!(
            filled_paths(&mounted.record(&frame())),
            [
                Path::rect(Rect::from_size(4.0, 1.0)),
                Path::rect(Rect::from_size(1.0, 1.0)),
            ]
        );
    }

    #[test]
    fn reconcile_result_controls_redraw() {
        // Verifies only reconciles that return Redraw::Yes invalidate the host.
        let host = Rc::new(Host::default());
        let mounted = CanvasNode::new(Bar::new(1));
        mounted.attach(host.clone(), None);

        mounted.adopt(&CanvasNode::new(Bar::new(1)));
        assert_eq!(host.invalidations.get(), 0);
        mounted.adopt(&CanvasNode::new(Bar::new(2)));
        assert_eq!(host.invalidations.get(), 1);
    }

    #[test]
    fn setup_runs_once_before_first_use() {
        // Verifies setup precedes the first draw and is not repeated by reconciles.
        let handler = Bar::new(1);
        let setups = handler.setups.clone();
        let mounted = CanvasNode::new(handler);
        assert_eq!(setups.get(), 0);
        mounted.record(&frame());
        mounted.adopt(&CanvasNode::new(Bar::new(2)));
        mounted.record(&frame());
        assert_eq!(setups.get(), 1);
    }

    #[test]
    fn event_requests_apply_after_the_handler_returns() {
        // Verifies invalidate and focus requests reach the host once the event is handled.
        let host = Rc::new(Host::default());
        let mounted = CanvasNode::new(Bar::new(1));
        mounted.attach(host.clone(), None);

        assert!(mounted.event(&pointer(PointerPhase::Down)));
        assert_eq!(host.invalidations.get(), 1);
        assert_eq!(host.focus.get(), Some(true));
        assert!(!mounted.event(&CanvasEvent::Focus(true)));
    }

    #[test]
    fn nodes_are_equal_only_when_shared() {
        // Verifies each render produces a distinct node so backends always see the new handler.
        let first = CanvasNode::new(Bar::new(1));
        assert_eq!(first, first.clone());
        assert_ne!(first, CanvasNode::new(Bar::new(1)));
    }

    /// Hands the setup invalidator to the test.
    #[derive(Clone)]
    struct Capture(Rc<RefCell<Option<Invalidator>>>);

    impl Canvas2DHandler for Capture {
        fn setup(&mut self, cx: &mut CanvasSetup) {
            *self.0.borrow_mut() = Some(cx.invalidator());
        }

        fn draw(&mut self, _g: &mut Graphics2D<'_>, _frame: &Frame) {}
    }

    #[test]
    fn invalidator_reaches_the_canvas_through_the_main_thread() {
        // Verifies off-thread invalidation is queued and invalidates the host when the task runs.
        let host = Rc::new(Host::default());
        let queued = Arc::new(Mutex::new(Vec::new()));
        let queue = queued.clone();
        let sender: Arc<MainTaskSender> = Arc::new(move |task| {
            queue.lock().unwrap().push(task);
            true
        });
        let captured = Rc::new(RefCell::new(None));
        let canvas = CanvasNode::new(Capture(captured.clone()));
        canvas.attach(host.clone(), Some(sender));
        canvas.record(&frame());

        let invalidator = captured.borrow().clone().unwrap();
        std::thread::spawn(move || assert!(invalidator.invalidate()))
            .join()
            .unwrap();
        assert_eq!(host.invalidations.get(), 0);
        for task in queued.lock().unwrap().drain(..) {
            task();
        }
        assert_eq!(host.invalidations.get(), 1);
    }

    #[test]
    fn invalidator_ignores_removed_canvases() {
        // Verifies a queued invalidation for a dropped canvas does nothing.
        let captured = Rc::new(RefCell::new(None));
        let host = Rc::new(Host::default());
        let queued = Arc::new(Mutex::new(Vec::new()));
        let queue = queued.clone();
        let canvas = CanvasNode::new(Capture(captured.clone()));
        canvas.attach(
            host.clone(),
            Some(Arc::new(move |task| {
                queue.lock().unwrap().push(task);
                true
            })),
        );
        canvas.record(&frame());
        let invalidator = captured.borrow_mut().take().unwrap();
        drop(canvas);

        assert!(invalidator.invalidate());
        for task in queued.lock().unwrap().drain(..) {
            task();
        }
        assert_eq!(host.invalidations.get(), 0);
    }
}
