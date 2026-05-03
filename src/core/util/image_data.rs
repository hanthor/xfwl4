// xfwl4 -- Wayland compositor for the Xfce Desktop Environment
//
// Copyright (C) 2026 Brian Tarricone <brian@tarricone.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::path::PathBuf;

use anyhow::anyhow;
use gtk::gdk_pixbuf;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Bind, ExportMem, Frame, ImportDma, Offscreen, Renderer, Texture, TextureMapping,
            element::{AsRenderElements, Element, RenderElement},
            utils::Buffer,
        },
    },
    desktop::Window,
    reexports::wayland_server::protocol::wl_shm,
    utils::{Logical, Rectangle, Scale, Size, Transform},
    wayland::{dmabuf::get_dmabuf, shm::with_buffer_contents},
};

use crate::{
    backend::Backend,
    core::{state::Xfwl4State, util::icon_theme::IconTheme},
    protocols::xfwl4_compositor_ui::Pixels,
};

#[derive(Debug)]
pub enum ImageData {
    NamedIcon(String),
    File(PathBuf),
    // cairo::Surface and gdk_pixbuf::Pixbuf cannot be made Send, so send the raw bytes.
    RgbaPixels { bytes: Vec<u8>, width: u32, height: u32 },
}

impl ImageData {
    pub fn load<IT: IconTheme>(&self, final_width: u32, final_height: u32, scale: f64, icon_theme: &IT) -> Option<gdk_pixbuf::Pixbuf> {
        match self {
            Self::NamedIcon(icon_name) => icon_theme.load_icon(icon_name, final_width.min(final_height) as i32, scale).ok(),

            Self::File(path) => gdk_pixbuf::Pixbuf::from_file(path)
                .ok()
                .and_then(|icon| scale_aspect(icon, final_width * scale as u32, final_height * scale as u32).ok()),

            Self::RgbaPixels { bytes, width, height } => {
                let bytes = glib::Bytes::from(bytes);
                let icon = gdk_pixbuf::Pixbuf::from_bytes(
                    &bytes,
                    gdk_pixbuf::Colorspace::Rgb,
                    true,
                    8,
                    *width as i32,
                    *height as i32,
                    (*width * 4) as i32,
                );
                scale_aspect(
                    icon,
                    (final_width as f64 * scale).floor() as u32,
                    (final_height as f64 * scale).floor() as u32,
                )
                .ok()
            }
        }
    }

    pub fn load_with_fallback<IT: IconTheme>(
        &self,
        final_width: u32,
        final_height: u32,
        scale: f64,
        icon_theme: &IT,
        fallback_name: &str,
    ) -> Option<gdk_pixbuf::Pixbuf> {
        self.load(final_width, final_height, scale, icon_theme).or_else(|| {
            icon_theme
                .load_icon(fallback_name, final_width.min(final_height) as i32, scale)
                .ok()
        })
    }
}

pub(super) fn scale_aspect(pixbuf: gdk_pixbuf::Pixbuf, width: u32, height: u32) -> anyhow::Result<gdk_pixbuf::Pixbuf> {
    if pixbuf.width() as u32 != width || pixbuf.height() as u32 != height {
        let aspect = pixbuf.width() as f64 / pixbuf.height() as f64;
        let final_aspect = width as f64 / height as f64;

        let (scale_width, scale_height) = if aspect > final_aspect {
            (width, (width as f64 / aspect).round() as u32)
        } else {
            ((height as f64 * aspect).round() as u32, height)
        };

        pixbuf
            .scale_simple(scale_width as i32, scale_height as i32, gdk_pixbuf::InterpType::Bilinear)
            .ok_or_else(|| anyhow!("Failed to scale pixbuf to requested size"))
    } else {
        Ok(pixbuf)
    }
}

impl<BackendData: Backend + 'static> Xfwl4State<BackendData> {
    pub(in crate::core) fn dmabuf_to_image_data(&mut self, buffer: &Buffer) -> anyhow::Result<ImageData> {
        let dmabuf = get_dmabuf(buffer)?;
        #[cfg(feature = "udev")]
        let mut renderer = self.backend.renderer(dmabuf.node())?;
        #[cfg(not(feature = "udev"))]
        let mut renderer = self.backend.renderer()?;
        let texture = renderer.import_dmabuf(dmabuf, None)?;

        let width = texture.width();
        let height = texture.height();
        let region = Rectangle::new((0, 0).into(), (width as i32, height as i32).into());

        let mapping = renderer.copy_texture(&texture, region, Fourcc::Abgr8888)?;
        let texture_data = renderer.map_texture(&mapping)?;

        let stride = (width * 4) as usize;
        let mut bytes = Vec::with_capacity((width * height * 4) as usize);

        if mapping.flipped() {
            for y in (0..height).rev() {
                let row_start = y as usize * stride;
                bytes.extend_from_slice(&texture_data[row_start..row_start + stride]);
            }
        } else {
            bytes.extend_from_slice(&texture_data[..stride * height as usize]);
        }

        Ok(ImageData::RgbaPixels { bytes, width, height })
    }

    pub(in crate::core) fn window_to_image_data(
        &mut self,
        window: &Window,
        max_size: u32,
        output_scale: Scale<f64>,
    ) -> anyhow::Result<Pixels> {
        #[cfg(feature = "debug")]
        let start = std::time::Instant::now();

        let logical_size = window.bbox().size;
        if logical_size.w <= 0 || logical_size.h <= 0 {
            return Err(anyhow!("invalid window size"));
        }

        let aspect_ratio = logical_size.w as f64 / logical_size.h as f64;
        let (thumbnail_logical_w, thumbnail_logical_h) = if logical_size.w >= logical_size.h {
            (max_size as i32, (max_size as f64 / aspect_ratio).round() as i32)
        } else {
            ((max_size as f64 * aspect_ratio).round() as i32, max_size as i32)
        };

        let (thumbnail_logical_w, thumbnail_logical_h) = if thumbnail_logical_w > logical_size.w || thumbnail_logical_h > logical_size.h {
            (logical_size.w, logical_size.h)
        } else {
            (thumbnail_logical_w, thumbnail_logical_h)
        };

        let thumbnail_scale_factor =
            (thumbnail_logical_w as f64 / logical_size.w as f64).min(thumbnail_logical_h as f64 / logical_size.h as f64);
        let render_scale = Scale::from(thumbnail_scale_factor * output_scale.x);

        #[cfg(feature = "udev")]
        let mut renderer = self.backend.renderer(None)?;
        #[cfg(not(feature = "udev"))]
        let mut renderer = self.backend.renderer()?;

        let elements = AsRenderElements::render_elements::<<Window as AsRenderElements<BackendData::Renderer<'_>>>::RenderElement>(
            window,
            &mut renderer,
            (0, 0).into(),
            render_scale,
            1.0,
        );

        let thumbnail_logical_size: Size<i32, Logical> = (thumbnail_logical_w, thumbnail_logical_h).into();
        let thumbnail_physical_size = thumbnail_logical_size.to_physical_precise_round(output_scale);
        let buffer_size = (thumbnail_physical_size.w, thumbnail_physical_size.h).into();

        let mut offscreen = renderer.create_buffer(Fourcc::Argb8888, buffer_size)?;
        let mut framebuffer = renderer.bind(&mut offscreen)?;
        let mut frame = renderer.render(&mut framebuffer, thumbnail_physical_size, Transform::Normal)?;

        frame.clear([0., 0., 0., 0.].into(), &[Rectangle::from_size(thumbnail_physical_size)])?;

        for element in &elements {
            let element_geometry = element.geometry(render_scale);
            let damage = [element_geometry];
            let opaque_regions = element.opaque_regions(render_scale);
            if let Err(err) = element.draw(&mut frame, element.src(), element_geometry, &damage, &opaque_regions, None) {
                tracing::debug!("Failed to draw element: {}", err);
            }
        }

        let sync = frame.finish()?;
        renderer.wait(&sync)?;

        let region = Rectangle::from_size(buffer_size);
        let mapping = renderer.copy_framebuffer(&framebuffer, region, Fourcc::Abgr8888)?;
        let bytes = renderer.map_texture(&mapping)?.to_vec();

        #[cfg(feature = "debug")]
        tracing::trace!(
            "Rendered window thumbnail ({}x{} -> {}x{}) in {:.2}ms",
            logical_size.w,
            logical_size.h,
            thumbnail_physical_size.w,
            thumbnail_physical_size.h,
            start.elapsed().as_secs_f64() * 1000.0
        );

        Ok(Pixels {
            bytes,
            width: thumbnail_physical_size.w as u32,
            height: thumbnail_physical_size.h as u32,
        })
    }
}

pub fn shm_buffer_to_image_data(buffer: &Buffer) -> anyhow::Result<ImageData> {
    with_buffer_contents(buffer, |ptr, _len, data| {
        let width = data.width as u32;
        let height = data.height as u32;
        let stride = data.stride as usize;

        let has_alpha = match data.format {
            wl_shm::Format::Argb8888 | wl_shm::Format::Abgr8888 => Ok(true),
            wl_shm::Format::Xrgb8888 | wl_shm::Format::Xbgr8888 => Ok(false),
            _ => Err(anyhow!("unsupported shm format {:?}", data.format)),
        }?;
        let bgr = matches!(data.format, wl_shm::Format::Abgr8888 | wl_shm::Format::Xbgr8888);

        let mut bytes = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let row_start = data.offset as usize + y as usize * stride;
            for x in 0..width {
                let pixel_offset = row_start + x as usize * 4;
                let pixel = unsafe { std::slice::from_raw_parts(ptr.add(pixel_offset), 4) };
                let (r, g, b, a) = if bgr {
                    (pixel[0], pixel[1], pixel[2], if has_alpha { pixel[3] } else { 255 })
                } else {
                    (pixel[2], pixel[1], pixel[0], if has_alpha { pixel[3] } else { 255 })
                };
                bytes.extend_from_slice(&[r, g, b, a]);
            }
        }

        Ok(ImageData::RgbaPixels { bytes, width, height })
    })?
}
