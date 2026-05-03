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

use smithay::{
    backend::renderer::{
        Frame as _, Renderer,
        element::{AsRenderElements, Element, RenderElement},
        gles::{GlesRenderer, GlesTarget},
        utils::with_renderer_surface_state,
    },
    delegate_image_copy_capture,
    reexports::wayland_server::{Resource, protocol::wl_shm},
    utils::{Rectangle, Scale, Transform},
    wayland::{
        dmabuf::get_dmabuf,
        image_capture_source::ImageCaptureSource,
        image_copy_capture::{
            BufferConstraints, CaptureFailureReason, DmabufConstraints, Frame, ImageCopyCaptureHandler, ImageCopyCaptureState, Session,
            SessionRef,
        },
        seat::WaylandFocus,
        shm,
    },
};

use crate::{
    backend::Backend,
    core::{
        handlers::image_capture_source::CaptureSource,
        render::{ImageCopyError, render_to_capture_buffer},
        shell::{WindowElement, WindowRenderElement},
        state::Xfwl4State,
        util::{OutputImageCopyExt, WindowImageCopyExt},
    },
};

impl<BackendData: Backend + 'static> ImageCopyCaptureHandler for Xfwl4State<BackendData> {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.core.protocol_delegates.image_copy_capture_state
    }

    fn capture_constraints(&mut self, source: &ImageCaptureSource) -> Option<BufferConstraints> {
        match source.user_data().get::<CaptureSource>() {
            Some(CaptureSource::Output(output)) => output.upgrade().and_then(|output| {
                output.current_mode().map(|mode| {
                    #[cfg(any(feature = "udev", feature = "winit"))]
                    let dmabuf_constraints = self.backend.dmabuf_constraints(None);

                    BufferConstraints {
                        size: (mode.size.w, mode.size.h).into(),
                        shm: vec![wl_shm::Format::Argb8888],
                        #[cfg(any(feature = "udev", feature = "winit"))]
                        dma: dmabuf_constraints,
                    }
                })
            }),

            Some(CaptureSource::Toplevel(window)) => window.0.wl_surface().filter(|surf| surf.is_alive()).and_then(|wl_surface| {
                let scale = self
                    .core
                    .workspace_manager
                    .outputs_for_window(window)
                    .first()
                    .map(|output| output.current_scale().fractional_scale())
                    .unwrap_or(1.);

                with_renderer_surface_state(&wl_surface, |state| {
                    if let Some(buffer) = state.buffer()
                        && let Some(buffer_size) = state.buffer_size()
                    {
                        let shm_formats = shm::with_buffer_contents(buffer, |_, _, data| data.format)
                            .into_iter()
                            .collect::<Vec<_>>();

                        #[cfg(any(feature = "udev", feature = "winit"))]
                        let dmabuf_constraints: Option<DmabufConstraints> = {
                            let node = get_dmabuf(buffer).ok().and_then(|dmabuf| dmabuf.node());
                            self.backend.dmabuf_constraints(node)
                        };

                        Some(BufferConstraints {
                            size: buffer_size.to_f64().to_buffer(scale, Transform::Normal).to_i32_round(),
                            shm: shm_formats,
                            #[cfg(any(feature = "udev", feature = "winit"))]
                            dma: dmabuf_constraints,
                        })
                    } else {
                        None
                    }
                })
                .flatten()
            }),

            None => None,
        }
    }

    fn new_session(&mut self, session: Session) {
        match session.source().user_data().get::<CaptureSource>() {
            Some(CaptureSource::Output(output)) => {
                if let Some(output) = output.upgrade() {
                    output.add_image_copy_session(session);
                } else {
                    session.stop();
                }
            }

            Some(CaptureSource::Toplevel(window)) => {
                if window.0.wl_surface().is_some_and(|surf| surf.is_alive()) {
                    window.0.add_image_copy_session(session);
                } else {
                    session.stop();
                }
            }

            None => {
                tracing::warn!("new_session: source has no CaptureSource user data");
                session.stop();
            }
        }
    }

    fn frame(&mut self, session: &SessionRef, frame: Frame) {
        match session.source().user_data().get::<CaptureSource>() {
            Some(CaptureSource::Output(output)) => {
                if let Some(output) = output.upgrade() {
                    output.queue_image_copy_frame(session, frame);
                } else {
                    frame.fail(CaptureFailureReason::Stopped);
                }
            }

            Some(CaptureSource::Toplevel(window)) => {
                if let Some(wl_surface) = window.0.wl_surface()
                    && wl_surface.is_alive()
                {
                    if let Err(err) = self.render_window_to_frame(window, session, &frame) {
                        tracing::warn!("Failed to render window to image capture frame: {err}");
                        let reason = match err {
                            ImageCopyError::MissingBufferConstraints => CaptureFailureReason::BufferConstraints,
                            _ => CaptureFailureReason::Unknown,
                        };
                        frame.fail(reason);
                    } else {
                        frame.success(Transform::Normal, None, self.core.clock.now());
                    }
                } else {
                    frame.fail(CaptureFailureReason::Stopped);
                }
            }

            None => (),
        }
    }

    fn session_destroyed(&mut self, session: SessionRef) {
        match session.source().user_data().get::<CaptureSource>() {
            Some(CaptureSource::Output(output)) => {
                if let Some(output) = output.upgrade() {
                    output.remove_image_copy_session(&session);
                }
            }

            Some(CaptureSource::Toplevel(window)) => {
                window.0.remove_image_copy_session(&session);
            }

            None => (),
        }
    }
}

impl<BackendData: Backend + 'static> Xfwl4State<BackendData> {
    fn render_window_to_frame(&mut self, window: &WindowElement, session: &SessionRef, frame: &Frame) -> Result<(), ImageCopyError> {
        let wl_buffer = frame.buffer();
        let dmabuf = get_dmabuf(&wl_buffer).ok().cloned();

        let scale = self
            .core
            .workspace_manager
            .outputs_for_window(window)
            .first()
            .map(|output| output.current_scale().fractional_scale())
            .unwrap_or(1.);
        let render_scale = Scale::from(scale);

        let constraints = session
            .current_constraints()
            .ok_or_else(|| ImageCopyError::MissingBufferConstraints)?;
        let size = constraints.size;

        #[cfg(feature = "udev")]
        let mut renderer = self.backend.renderer(dmabuf.as_ref().and_then(|d| d.node()))?;
        #[cfg(not(feature = "udev"))]
        let mut renderer = self.backend.renderer()?;
        let gles: &mut GlesRenderer = renderer.as_mut();

        let elements: Vec<WindowRenderElement<GlesRenderer>> =
            AsRenderElements::render_elements(window, gles, (0, 0).into(), render_scale, 1.0);

        render_to_capture_buffer(
            gles,
            size,
            dmabuf,
            &wl_buffer,
            |gles: &mut GlesRenderer, target: &mut GlesTarget<'_>| {
                let mut render_frame = gles.render(target, (size.w, size.h).into(), Transform::Normal)?;
                render_frame.clear([0., 0., 0., 0.].into(), &[Rectangle::from_size((size.w, size.h).into())])?;
                for element in &elements {
                    let geom = element.geometry(render_scale);
                    let opaque = element.opaque_regions(render_scale);
                    if let Err(err) = element.draw(&mut render_frame, element.src(), geom, &[geom], &opaque, None) {
                        tracing::debug!("Failed to draw window capture element: {err}");
                    }
                }
                let sync = render_frame.finish()?;
                gles.wait(&sync)?;
                Ok(())
            },
        )?;

        Ok(())
    }
}

delegate_image_copy_capture!(@<BackendData: Backend + 'static> Xfwl4State<BackendData>);
