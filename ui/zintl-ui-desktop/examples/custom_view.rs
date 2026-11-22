//! Views composed from other views that take the shared modifiers.

use zintl_ui_desktop::render::{Element, NativeView, ViewModifiers, modifiers, native_view};
use zintl_ui_desktop::*;

/// A titled box around other views.
///
/// Its root is one stack, so `id`, sizes, `context_menu`, and the
/// accessibility modifiers reach that stack.
#[derive(Clone)]
pub struct Card {
    title: String,
    content: ElementFactory<RenderNode>,
    modifiers: ViewModifiers,
}

impl Card {
    pub fn new<V>(title: impl Into<String>, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            title: title.into(),
            content: ElementFactory::new(content),
            modifiers: ViewModifiers::default(),
        }
    }
}

impl NativeView for Card {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        VStack::new(list![
            Text::new(self.title.clone()).font(Font::system(15.0).bold()),
            Separator::horizontal(),
            self.content.clone(),
        ])
        .padding(12.0)
        .native_element(cx)
    }
}

native_view!(Card);

/// A count with buttons to change it, kept locally unless the app binds a Store.
///
/// It has `View::init`, so it implements `View` by hand and must be mounted
/// as a child rather than rendered from another view's `base_element`.
#[derive(Clone)]
pub struct Counter {
    count: Local<i32>,
    modifiers: ViewModifiers,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            count: Local::new(0),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Reads and writes the count through `store`.
    pub fn bind(mut self, store: Store<i32>) -> Self {
        self.count.bind(store);
        self
    }
}

impl View for Counter {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.count.init(cx);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        self.native_element(cx)
    }
}

impl NativeView for Counter {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let count = self.count.store();
        let value = *cx.get(count);
        HStack::new(list![
            Button::icon("minus")
                .accessibility_label("Decrease")
                .on_click(move |cx| cx.update(count, |count| *count -= 1)),
            Text::new(value.to_string()),
            Button::icon("plus")
                .accessibility_label("Increase")
                .on_click(move |cx| cx.update(count, |count| *count += 1)),
        ])
        .accessibility_value(value.to_string())
        .native_element(cx)
    }
}

#[derive(Default)]
pub struct MainView {
    shared: Store<i32>,
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.shared = cx.store(10);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let shared = self.shared;
        Window::new("Custom Views")
            .size(Size::new(420.0, 320.0))
            .content(
                VStack::new(list![
                    Card::new("Local", Counter::new().id("local"))
                        .accessibility_label("Local counter")
                        .fill_width(),
                    Card::new(
                        "Bound",
                        VStack::new(list![
                            Counter::new().bind(shared).id("bound"),
                            cx.watch(shared, |count| Text::new(format!("Store holds {count}"))),
                        ]),
                    )
                    .context_menu(menu::Menu::new(
                        "",
                        [menu::MenuItem::new("Reset")
                            .on_select(move |cx| cx.update(shared, |count| *count = 0))],
                    ))
                    .fill_width(),
                ])
                .padding(16.0),
            )
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    App::new(MainView::default()).run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
