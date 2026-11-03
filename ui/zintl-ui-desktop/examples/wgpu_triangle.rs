//! A triangle rendered with `WgpuCanvas`.
//!
//! "Spin" switches between continuous and on-demand redraw. Drag the canvas to
//! turn the triangle, and click it to change its color.

use zintl_ui_desktop::*;

const SHADER: &str = r#"
struct Uniforms {
    angle: f32,
    aspect: f32,
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) shade: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let corner = f32(index) * 2.0943951 + uniforms.angle;
    var out: Vertex;
    out.position = vec4<f32>(0.7 * sin(corner) / uniforms.aspect, 0.7 * cos(corner), 0.0, 1.0);
    out.shade = 0.6 + 0.2 * f32(index);
    return out;
}

@fragment
fn fs_main(vertex: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(uniforms.color.rgb * vertex.shade, 1.0);
}
"#;

const COLORS: [[f32; 3]; 4] = [
    [0.95, 0.45, 0.25],
    [0.25, 0.65, 0.95],
    [0.35, 0.8, 0.45],
    [0.85, 0.4, 0.85],
];

/// GPU objects created once in setup.
#[derive(Clone)]
struct Gpu {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

/// Rendering state owned by the canvas; it survives view updates.
#[derive(Clone)]
struct Triangle {
    spin: bool,
    gpu: Option<Gpu>,
    /// Angle set by dragging, in radians.
    angle: f64,
    drag_start: Option<(f64, f64)>,
    color: usize,
}

impl Triangle {
    fn new(spin: bool) -> Self {
        Self {
            spin,
            gpu: None,
            angle: 0.0,
            drag_start: None,
            color: 0,
        }
    }
}

impl WgpuCanvasHandler for Triangle {
    fn setup(&mut self, cx: &mut WgpuSetup<'_>) {
        let device = cx.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(cx.format().into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        self.gpu = Some(Gpu {
            pipeline,
            uniforms,
            bind_group,
        });
    }

    fn render(&mut self, target: &mut WgpuTarget<'_>, frame: &Frame) {
        let Some(gpu) = &self.gpu else {
            return;
        };
        let angle = if self.spin {
            self.angle + frame.time
        } else {
            self.angle
        };
        let [red, green, blue] = COLORS[self.color];
        let uniforms = [
            angle as f32,
            (frame.width() / frame.height().max(1.0)) as f32,
            0.0,
            0.0,
            red,
            green,
            blue,
            1.0,
        ];
        let bytes: Vec<u8> = uniforms
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        target.queue().write_buffer(&gpu.uniforms, 0, &bytes);

        let mut encoder = target
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Linear values; an sRGB target brightens them.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.008,
                            g: 0.01,
                            b: 0.016,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        target.queue().submit([encoder.finish()]);
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        let CanvasEvent::Pointer(pointer) = event else {
            return false;
        };
        match (pointer.phase, pointer.button) {
            (PointerPhase::Down, Some(PointerButton::Primary)) => {
                self.drag_start = Some((pointer.location.x, self.angle));
            }
            (PointerPhase::Moved, Some(PointerButton::Primary)) => {
                if let Some((x, angle)) = self.drag_start {
                    self.angle = angle + (pointer.location.x - x) / 100.0;
                }
            }
            (PointerPhase::Up, Some(PointerButton::Primary)) => {
                // A click without dragging changes the color.
                if self
                    .drag_start
                    .take()
                    .is_some_and(|(x, _)| x == pointer.location.x)
                {
                    self.color = (self.color + 1) % COLORS.len();
                }
            }
            _ => return false,
        }
        cx.invalidate();
        true
    }

    fn reconcile(&mut self, next: Self) -> Redraw {
        if next.spin == self.spin {
            return Redraw::No;
        }
        self.spin = next.spin;
        Redraw::Yes
    }
}

#[derive(Default)]
struct TriangleExample {
    spin: Store<bool>,
}

impl View for TriangleExample {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.spin = cx.store(true);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let spin = self.spin;
        Window::new(Rect::new(120.0, 120.0, 640.0, 520.0), "wgpu Triangle").content(
            VStack::new(list![
                cx.watch(spin, move |on| {
                    Toggle::new("Spin")
                        .on(*on)
                        .on_change(move |cx, on| cx.update(spin, |spin| *spin = on))
                }),
                cx.watch(spin, |on| {
                    let redraw = if *on {
                        RedrawPolicy::Continuous {
                            preferred_fps: None,
                        }
                    } else {
                        RedrawPolicy::OnDemand
                    };
                    WgpuCanvas::new(Triangle::new(*on))
                        .redraw(redraw)
                        .fill_width()
                        .grow()
                }),
            ])
            .spacing(12.0)
            .padding(16.0)
            .fill_width()
            .grow(),
        )
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    App::new(TriangleExample::default()).run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
