use zintl_ui_desktop::*;

/// Reports its first setup and render; the view never changes its size or state.
#[derive(Clone)]
struct Probe {
    status: StoreSender<String>,
    reported: bool,
}

impl WgpuCanvasHandler for Probe {
    fn setup(&mut self, _cx: &mut WgpuSetup<'_>) {
        self.status.set("Set up".to_owned());
    }

    fn render(&mut self, target: &mut WgpuTarget<'_>, _frame: &Frame) {
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
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        target.queue().submit([encoder.finish()]);
        if !self.reported {
            self.reported = true;
            self.status.set("Rendered".to_owned());
        }
    }

    // Keeps the mounted handler so status updates do not request redraws.
    fn reconcile(&mut self, _next: Self) -> Redraw {
        Redraw::No
    }
}

/// An on-demand wgpu canvas that nothing redraws after it is attached.
pub struct MainView {
    status: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            status: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.status = cx.store("Waiting".to_owned());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let canvas = WgpuCanvas::new(Probe {
            status: cx.sender(self.status),
            reported: false,
        })
        .redraw(RedrawPolicy::OnDemand)
        .fill_width()
        .grow();
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 320.0),
            "Zintl Wgpu First Frame UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                cx.watch(self.status, |status| Text::new(status.clone())
                    .id("render-status")),
                canvas,
            ])
            .spacing(12.0)
            .fill_width()
            .grow(),
        )
    }
}
