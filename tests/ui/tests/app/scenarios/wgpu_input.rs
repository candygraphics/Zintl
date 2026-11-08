use zintl_ui_desktop::*;

/// Clears the canvas to red or blue and consumes clicks and keys.
#[derive(Clone)]
struct Fill {
    red: bool,
}

impl WgpuCanvasHandler for Fill {
    fn render(&mut self, target: &mut WgpuTarget<'_>, _frame: &Frame) {
        let color = if self.red {
            wgpu::Color::RED
        } else {
            wgpu::Color::BLUE
        };
        let mut encoder = target
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        target.queue().submit([encoder.finish()]);
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        match event {
            CanvasEvent::Pointer(pointer) if pointer.phase == PointerPhase::Down => {
                cx.request_focus();
                true
            }
            CanvasEvent::Key(_) => true,
            _ => false,
        }
    }
}

/// A wgpu canvas whose color follows a Store and whose input reaches on_event.
pub struct MainView {
    red: Store<bool>,
    clicks: Store<u32>,
    key: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            red: Store::default(),
            clicks: Store::default(),
            key: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.red = cx.store(false);
        self.clicks = cx.store(0);
        self.key = cx.store(String::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (red, clicks, key) = (self.red, self.clicks, self.key);
        Window::new(Rect::new(100.0, 100.0, 480.0, 400.0), "Zintl Wgpu Input UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    cx.watch(clicks, |clicks| Text::new(format!("Clicks: {clicks}")).id("clicks")),
                    cx.watch(key, |key| Text::new(format!("Key: {key}")).id("key")),
                    Button::new("Paint Red")
                        .id("paint-red")
                        .on_click(move |cx| cx.update(red, |red| *red = true)),
                    WgpuCanvas::new(Fill { red: *cx.get(red) })
                        .focusable(true)
                        .on_event(move |cx, event| match event {
                            CanvasEvent::Pointer(pointer) if pointer.phase == PointerPhase::Down => {
                                cx.update(clicks, |clicks| *clicks += 1);
                            }
                            CanvasEvent::Key(KeyEvent {
                                phase: KeyPhase::Down,
                                key: Key::Character(character),
                                ..
                            }) => {
                                let character = character.clone();
                                cx.update(key, |key| *key = character);
                            }
                            _ => {}
                        })
                        .fill_width()
                        .grow(),
                ])
                .spacing(8.0)
                .fill_width()
                .grow(),
            )
    }
}
