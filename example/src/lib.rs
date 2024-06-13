use jano::android_activity::{AndroidApp, MainEvent};
use jano::glam::uvec2;
use jano::graphics::Gpu;
use jano::{wgpu, FrameStats, Window};

#[no_mangle]
fn android_main(android: AndroidApp) {
    jano::android_main(android, App::default());
}

#[derive(Default)]
struct App {
    gpu: Option<Gpu>,
    window: Option<Window>,
    hue: f64,
}
impl jano::AppState for App {
    fn on_main_event(&mut self, event: MainEvent, draw_frames: &mut bool) -> bool {
        match event {
            MainEvent::Pause => *draw_frames = false,
            MainEvent::Resume { .. } => *draw_frames = true,
            MainEvent::InitWindow { .. } => {
                self.window = jano::android().native_window();

                if let Some(win) = &self.window {
                    let instance = wgpu::Instance::new(Default::default());
                    let surface = jano::graphics::create_wgpu_surface(&instance, win);

                    let size = uvec2(win.width() as u32, win.height() as u32);
                    self.gpu = Some(pollster::block_on(Gpu::new(instance, surface, size)));
                } else {
                    eprintln!("native_window() returned None during InitWindow callback");
                }
            }
            MainEvent::TerminateWindow { .. } => {
                self.gpu = None;
                self.window = None;
            }
            MainEvent::Destroy => return true,
            _ => {}
        }
        false
    }
    fn on_frame(&mut self, _stats: FrameStats) {
        let Some(gpu) = &self.gpu else {
            return;
        };

        let (output, view) = match gpu.get_output() {
            Ok(v) => v,
            Err(err) => {
                eprintln!("GPU surface error: {err:?}");
                return;
            }
        };
        let mut encoder = gpu.create_command_encoder();
        // --- render pass ---

        let (r, g, b) = hsv::hsv_to_rgb(self.hue, 1.0, 1.0);
        let (r, g, b, a) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0);
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            occlusion_query_set: None,
            timestamp_writes: None,
            label: Some("#render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
        });
        std::mem::drop(pass);

        // --- submit passes ---
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.hue += 2.0;
        if self.hue >= 360.0 {
            self.hue = 0.0;
        }
    }
}
