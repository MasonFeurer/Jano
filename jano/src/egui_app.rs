#[cfg(not(feature = "wgpu_19"))]
compile_error!("To use `egui_27` feature, you must also use `wgpu_19` feature");

#[cfg(not(feature = "egui-wgpu"))]
compile_error!("To use `egui_27` feature, you must also use `egui-wgpu` feature");

#[cfg(not(feature = "pollster"))]
compile_error!("To use `egui_27` feature, you must also use `pollster` feature");

use super::{egui, wgpu};
use crate::graphics::Gpu;
use crate::{
    android, scale_factor, translate_input_event, AppState, FrameStats, Picture, PtrButton,
    TouchEvent, TouchTranslater,
};
use android_activity::MainEvent;
use glam::{uvec2, vec2, UVec2};

pub struct Egui {
    pub window: crate::Window,
    pub gpu: Gpu,
    pub ctx: egui::Context,
    pub renderer: egui_wgpu::Renderer,
    pub texture_handles: std::collections::HashMap<egui::Id, egui::TextureHandle>,
}
impl Egui {
    pub fn obtain_tex_handle_for_pic(
        &mut self,
        id: impl Into<egui::Id>,
        pic: &Picture,
    ) -> egui::TextureHandle {
        let id = id.into();
        if let Some(handle) = self.texture_handles.get(&id) {
            return handle.clone();
        }
        let size = [pic.size.x as usize, pic.size.y as usize];
        let img = egui::ColorImage::from_rgba_unmultiplied(size, &pic.data);
        let debug_id = "auto-tex:obtain_tex_handle_for_pic";
        let handle = self.ctx.load_texture(debug_id, img, Default::default());
        self.texture_handles.insert(id, handle.clone());
        handle
    }
    pub fn obtain_tex_handle_for_img(
        &mut self,
        id: impl Into<egui::Id>,
        img: egui::ColorImage,
    ) -> egui::TextureHandle {
        let id = id.into();
        if let Some(handle) = self.texture_handles.get(&id) {
            return handle.clone();
        }
        let debug_id = "auto-tex:obtain_tex_handle_for_img";
        let handle = self.ctx.load_texture(debug_id, img, Default::default());
        self.texture_handles.insert(id, handle.clone());
        handle
    }

    pub async fn new(
        window: crate::Window,
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: UVec2,
    ) -> Self {
        let gpu = Gpu::new(instance, surface, size).await;
        let ctx = egui::Context::default();
        let renderer = egui_wgpu::Renderer::new(&gpu.device, gpu.surface_config.format, None, 1);
        Self {
            window,
            gpu,
            ctx,
            renderer,
            texture_handles: Default::default(),
        }
    }
}

pub struct EguiAppState<A>(pub EguiInput, pub Option<Egui>, pub A);
impl<A> EguiAppState<A> {
    #[inline(always)]
    pub fn new(app: A) -> Self {
        Self(Default::default(), None, app)
    }
}

pub trait EguiApp {
    fn on_pause(&mut self) {}
    fn on_resume(&mut self) {}
    fn on_save_state(&mut self) {}

    fn draw_frame(&mut self, egui: &mut Egui, ctx: &egui::Context, stats: FrameStats);
    fn on_picture_taken(&mut self, _egui: &Option<Egui>, _pic: Picture) {}
}

impl<A: EguiApp> AppState for EguiAppState<A> {
    fn on_main_event(&mut self, event: MainEvent, draw_frames: &mut bool) -> bool {
        let EguiAppState(_input, egui, app) = self;
        match event {
            MainEvent::SaveState { .. } => app.on_save_state(),
            MainEvent::Pause => {
                log::info!("App paused...");
                app.on_pause();
                *draw_frames = false;
            }
            MainEvent::Resume { .. } => {
                log::info!("App resumed...");
                crate::input::set_scale_factor(5.0);
                app.on_resume();
                *draw_frames = true;
            }
            MainEvent::InitWindow { .. } => {
                log::info!("Window initialized - creating Surface...");
                let window = crate::android().native_window();

                if let Some(win) = window {
                    let instance = wgpu::Instance::new(Default::default());
                    let surface = crate::graphics::create_wgpu_surface(&instance, &win);

                    let size = uvec2(win.width() as u32, win.height() as u32);
                    *egui = Some(pollster::block_on(Egui::new(win, instance, surface, size)));
                } else {
                    log::error!("native_window() returned None during InitWindow callback");
                }
            }
            MainEvent::TerminateWindow { .. } => {
                log::info!("App terminated...");
                *egui = None;
            }
            MainEvent::WindowResized { .. } => log::info!("Window resized..."),
            MainEvent::RedrawNeeded { .. } => {}
            MainEvent::InputAvailable { .. } => {}
            MainEvent::ConfigChanged { .. } => {}
            MainEvent::LowMemory => log::warn!("Recieved LowMemory Event..."),
            MainEvent::Destroy => {
                log::info!("App destroyed...");
                return true;
            }
            _ => {}
        }
        false
    }

    fn on_frame(&mut self, stats: FrameStats) {
        let EguiAppState(input, egui, app) = self;
        let Some(egui) = egui else {
            log::warn!("Frame drawing canceled: egui is None");
            return;
        };

        // Handle input
        'i: {
            input.translater.set_scale_factor(scale_factor());
            input.update();
            let mut iter = match android().input_events_iter() {
                Ok(iter) => iter,
                Err(err) => {
                    log::warn!("Failed to get input events iterator: {err:?}");
                    break 'i;
                }
            };
            while iter.next(|event| {
                translate_input_event(event, &mut input.translater, |touch_event| {
                    input.raw.events.push(touch_event.into())
                })
            }) {}
        }

        let (output, view) = match egui.gpu.get_output() {
            Ok(v) => v,
            Err(err) => {
                log::error!("GPU surface error: {err:?}");
                return;
            }
        };
        let mut encoder = egui.gpu.create_command_encoder();

        // --- egui ---
        {
            // --- create scene ---
            // let screen_rect = {
            //     let size = egui::vec2(egui.window.width() as f32, egui.window.height() as f32);
            //     egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size)
            // };
            let content_rect = {
                let size = vec2(egui.window.width() as f32, egui.window.height() as f32);
                let (min, max) = crate::display_cutout(size);
                let (min, max) = (min / crate::scale_factor(), max / crate::scale_factor());
                egui::Rect::from_min_max(egui::pos2(min.x, min.y), egui::pos2(max.x, max.y))
            };
            let mut input: egui::RawInput = input.take(content_rect);
            let viewport = input
                .viewports
                .get_mut(&egui::viewport::ViewportId::ROOT)
                .unwrap();
            viewport.native_pixels_per_point = Some(crate::scale_factor());
            viewport.inner_rect = Some(content_rect);
            input.screen_rect = Some(content_rect);
            let ctx = egui.ctx.clone();
            let egui_output = ctx.run(input, |ctx| {
                app.draw_frame(egui, ctx, stats);
            });
            let egui_prims = egui
                .ctx
                .tessellate(egui_output.shapes, egui_output.pixels_per_point);
            let screen_desc = egui_wgpu::ScreenDescriptor {
                size_in_pixels: egui.gpu.surface_size().into(),
                pixels_per_point: egui_output.pixels_per_point,
            };

            // --- update buffers ---
            for (id, image) in egui_output.textures_delta.set {
                log::info!("Updating egui_renderer texture {id:?}");
                egui.renderer
                    .update_texture(&egui.gpu.device, &egui.gpu.queue, id, &image);
            }
            egui.renderer.update_buffers(
                &egui.gpu.device,
                &egui.gpu.queue,
                &mut encoder,
                &egui_prims,
                &screen_desc,
            );
            // --- render pass ---
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                occlusion_query_set: None,
                timestamp_writes: None,
                label: Some("#egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });
            egui.renderer.render(&mut pass, &egui_prims, &screen_desc);
            std::mem::drop(pass);

            for id in egui_output.textures_delta.free {
                egui.renderer.free_texture(&id);
            }
        };

        // --- submit passes ---
        egui.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn on_picture_taken(&mut self, pic: Picture) {
        let EguiAppState(_input, egui, app) = self;
        app.on_picture_taken(egui, pic)
    }
}

impl From<PtrButton> for egui::PointerButton {
    fn from(ptr: PtrButton) -> Self {
        match ptr {
            PtrButton::Primary => egui::PointerButton::Primary,
            PtrButton::Secondary => egui::PointerButton::Secondary,
            PtrButton::Middle => egui::PointerButton::Middle,
            PtrButton::Extra1 => egui::PointerButton::Extra1,
            PtrButton::Extra2 => egui::PointerButton::Extra2,
        }
    }
}
impl From<TouchEvent> for egui::Event {
    fn from(ptr: TouchEvent) -> Self {
        match ptr {
            TouchEvent::KeyBackspace { pressed } => egui::Event::Key {
                key: egui::Key::Backspace,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Default::default(),
            },
            TouchEvent::Text(text) => egui::Event::Text(text),
            TouchEvent::PtrMoved(pos) => egui::Event::PointerMoved(egui::pos2(pos.x, pos.y)),
            TouchEvent::PtrPressed(button, pos) => egui::Event::PointerButton {
                pos: egui::pos2(pos.x, pos.y),
                button: button.into(),
                pressed: true,
                modifiers: Default::default(),
            },
            TouchEvent::PtrReleased(button, pos) => egui::Event::PointerButton {
                pos: egui::pos2(pos.x, pos.y),
                button: button.into(),
                pressed: false,
                modifiers: Default::default(),
            },
            TouchEvent::PtrLeft => egui::Event::PointerGone,
            TouchEvent::Zoom(factor, _pos) => egui::Event::Zoom(factor),
            TouchEvent::Scroll(delta) => egui::Event::Scroll(egui::vec2(delta.x, delta.y)),
        }
    }
}

#[derive(Default)]
pub struct EguiInput {
    pub raw: egui::RawInput,
    pub translater: TouchTranslater,
}
impl EguiInput {
    pub fn take_raw(&mut self) -> egui::RawInput {
        self.raw.take()
    }

    pub fn take(&mut self, content_rect: egui::Rect) -> egui::RawInput {
        let mut input = self.raw.take();
        let viewport = input
            .viewports
            .get_mut(&egui::viewport::ViewportId::ROOT)
            .unwrap();
        viewport.native_pixels_per_point = Some(scale_factor());
        viewport.inner_rect = Some(content_rect);
        input.screen_rect = Some(content_rect);
        input
    }

    pub fn update(&mut self) {
        self.translater.update(|e| self.raw.events.push(e.into()));
    }
}
