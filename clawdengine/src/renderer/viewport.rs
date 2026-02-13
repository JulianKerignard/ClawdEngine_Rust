pub struct ViewportTexture {
    pub color_view: wgpu::TextureView,
    pub msaa_color_view: wgpu::TextureView,
    pub depth_view: wgpu::TextureView,
    pub egui_texture_id: egui::TextureId,
    pub width: u32,
    pub height: u32,
    _color_texture: wgpu::Texture,
    _msaa_color_texture: wgpu::Texture,
    _depth_texture: wgpu::Texture,
}

impl ViewportTexture {
    pub fn new(
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        width: u32,
        height: u32,
    ) -> Self {
        let (color_tex, color_view) = Self::create_color(device, width, height);
        let (msaa_tex, msaa_view) = Self::create_msaa_color(device, width, height);
        let (depth_tex, depth_view) = Self::create_depth(device, width, height);

        let egui_texture_id = egui_renderer.register_native_texture(
            device,
            &color_view,
            wgpu::FilterMode::Linear,
        );

        Self {
            color_view,
            msaa_color_view: msaa_view,
            depth_view,
            egui_texture_id,
            width,
            height,
            _color_texture: color_tex,
            _msaa_color_texture: msaa_tex,
            _depth_texture: depth_tex,
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        width: u32,
        height: u32,
    ) {
        if (width == self.width && height == self.height) || width == 0 || height == 0 {
            return;
        }

        let (color_tex, color_view) = Self::create_color(device, width, height);
        let (msaa_tex, msaa_view) = Self::create_msaa_color(device, width, height);
        let (depth_tex, depth_view) = Self::create_depth(device, width, height);

        egui_renderer.update_egui_texture_from_wgpu_texture(
            device,
            &color_view,
            wgpu::FilterMode::Linear,
            self.egui_texture_id,
        );

        self._color_texture = color_tex;
        self.color_view = color_view;
        self._msaa_color_texture = msaa_tex;
        self.msaa_color_view = msaa_view;
        self._depth_texture = depth_tex;
        self.depth_view = depth_view;
        self.width = width;
        self.height = height;
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }

    fn create_color(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Color"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    fn create_msaa_color(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport MSAA Color"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    pub fn capture_to_png(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = self.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Screenshot encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self._color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).ok();
        rx.recv()??;

        let data = buffer_slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((self.width * self.height * bytes_per_pixel) as usize);
        for row in 0..self.height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (self.width * bytes_per_pixel) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        staging.unmap();

        let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_raw(self.width, self.height, pixels)
                .ok_or("Failed to create image buffer")?;

        // Resize to 256x144 thumbnail
        let thumb = image::imageops::resize(&img, 256, 144, image::imageops::FilterType::Lanczos3);
        thumb.save(path)?;
        Ok(())
    }

    fn create_depth(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }
}
