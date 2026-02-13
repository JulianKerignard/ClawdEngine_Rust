use std::collections::HashMap;

pub struct GpuTexture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

pub struct TextureStore {
    textures: Vec<GpuTexture>,
    path_map: HashMap<String, usize>,
    default_white: usize,
    default_normal: usize,
}

fn mip_level_count(w: u32, h: u32) -> u32 {
    (w.max(h) as f32).log2().floor() as u32 + 1
}

impl TextureStore {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut store = Self {
            textures: Vec::new(),
            path_map: HashMap::new(),
            default_white: 0,
            default_normal: 0,
        };
        // Create 1x1 white fallback texture
        let white_id = store.create_from_rgba(device, queue, 1, 1, &[255, 255, 255, 255]);
        store.default_white = white_id;
        // Create 1x1 flat normal map (pointing up in tangent space)
        let normal_id = store.create_from_rgba(device, queue, 1, 1, &[128, 128, 255, 255]);
        store.default_normal = normal_id;
        store
    }

    pub fn default_id(&self) -> usize {
        self.default_white
    }

    pub fn default_normal_id(&self) -> usize {
        self.default_normal
    }

    pub fn get(&self, id: usize) -> &GpuTexture {
        &self.textures[id]
    }

    pub fn load(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> usize {
        if let Some(&id) = self.path_map.get(path) {
            return id;
        }
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let id = self.create_from_rgba(device, queue, w, h, &rgba);
                self.path_map.insert(path.to_string(), id);
                log::info!("Loaded texture: {} ({}x{})", path, w, h);
                id
            }
            Err(e) => {
                log::error!("Failed to load texture {}: {}", path, e);
                self.default_white
            }
        }
    }

    /// Upload a texture with pre-computed mip levels (no CPU resize needed).
    pub fn upload_precomputed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache_key: &str,
        width: u32,
        height: u32,
        mip_levels: &[Vec<u8>],
    ) -> usize {
        if let Some(&id) = self.path_map.get(cache_key) {
            return id;
        }

        let mip_count = mip_levels.len() as u32;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Loaded Texture"),
            size,
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, mip_data) in mip_levels.iter().enumerate() {
            let mip_w = (width >> level).max(1);
            let mip_h = (height >> level).max(1);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mip_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * mip_w),
                    rows_per_image: Some(mip_h),
                },
                wgpu::Extent3d {
                    width: mip_w,
                    height: mip_h,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let id = self.textures.len();
        self.textures.push(GpuTexture { view, sampler });
        self.path_map.insert(cache_key.to_string(), id);
        log::info!("Uploaded precomputed texture: {} ({}x{}, {} mips)", cache_key, width, height, mip_count);
        id
    }

    fn create_from_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> usize {
        let mip_count = mip_level_count(width, height);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Loaded Texture"),
            size,
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload mip level 0
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        // Generate and upload remaining mip levels (CPU-side downscale)
        if mip_count > 1 {
            if let Some(mut current) =
                image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(width, height, data.to_vec())
            {
                for level in 1..mip_count {
                    let mip_w = (width >> level).max(1);
                    let mip_h = (height >> level).max(1);
                    current = image::imageops::resize(
                        &current,
                        mip_w,
                        mip_h,
                        image::imageops::FilterType::Triangle,
                    );
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: level,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &current,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * mip_w),
                            rows_per_image: Some(mip_h),
                        },
                        wgpu::Extent3d {
                            width: mip_w,
                            height: mip_h,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let id = self.textures.len();
        self.textures.push(GpuTexture { view, sampler });
        id
    }
}
