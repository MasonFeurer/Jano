use super::wgpu;
use glam::{uvec2, UVec2};

pub fn create_wgpu_surface(
    instance: &wgpu::Instance,
    window: &crate::Window,
) -> wgpu::Surface<'static> {
    let win_ptr = window.ptr().cast();
    unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: wgpu::rwh::AndroidDisplayHandle::new().into(),
                raw_window_handle: wgpu::rwh::AndroidNdkWindowHandle::new(win_ptr).into(),
            })
            .unwrap()
    }
}

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    // Using ManuallyDrop to prevent the `drop` implementation of wgpu::Surface from running.
    // In wgpu 0.20, wgpu::Surface::drop crashes on a null-pointer exception if ran after a `TerminateWindow` event is recieved.
    pub surface: std::mem::ManuallyDrop<wgpu::Surface<'static>>,
    pub surface_config: wgpu::SurfaceConfiguration,
}
impl Gpu {
    pub async fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: UVec2,
    ) -> Self {
        // Handle to the graphics device
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // device: Open connection to graphics device
        // queue: Handle to a command queue on the device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::default(),
                    required_limits: adapter.limits(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_config = surface
            .get_default_config(&adapter, size[0], size[1])
            .unwrap();
        surface.configure(&device, &surface_config);

        Self {
            surface: std::mem::ManuallyDrop::new(surface),
            device,
            surface_config,
            queue,
        }
    }

    pub fn resize(&mut self, new_size: UVec2) {
        self.surface_config.width = new_size[0];
        self.surface_config.height = new_size[1];
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_command_encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&Default::default())
    }

    pub fn surface_size(&self) -> UVec2 {
        uvec2(self.surface_config.width, self.surface_config.height)
    }

    pub fn get_output(
        &self,
    ) -> Result<(wgpu::SurfaceTexture, wgpu::TextureView), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        Ok((output, view))
    }
}
