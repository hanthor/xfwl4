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
//
// Portions of this file are based on "anvil", an example compositor
// based on the smithay crate, and are licensed under the MIT license
// with the following terms:
//
// Copyright (C) Victor Berger <victor.berger@m4x.org>
// Copyright (C) Drakulix (Victoria Brekenfeld)
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::{
    collections::VecDeque,
    io,
    ops::Not,
    sync::Once,
    time::{Duration, Instant},
};

use crate::{
    backend::udev::{UdevData, UdevOutputId},
    core::{
        render::*,
        state::{Xfwl4Core, Xfwl4State},
    },
};

use anyhow::{Context, anyhow};
use smithay::{
    backend::{
        SwapBuffersError,
        allocator::gbm::GbmAllocator,
        drm::{
            DrmAccessError, DrmDeviceFd, DrmError, DrmEventMetadata, DrmEventTime, DrmNode,
            compositor::{FrameFlags, PrimaryPlaneElement},
            exporter::gbm::GbmFramebufferExporter,
            output::DrmOutput,
        },
        renderer::{
            damage::Error as OutputDamageTrackerError,
            element::RenderElementStates,
            gles::{GlesError, GlesRenderer},
            multigpu::{self, MultiRenderer, gbm::GbmGlesBackend},
        },
    },
    desktop::utils::OutputPresentationFeedback,
    output::Output,
    reexports::{
        calloop::{
            RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        drm::control::{connector, crtc},
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
    },
    utils::{Monotonic, Time},
    wayland::presentation::Refresh,
};
use tracing::{error, trace, warn};

const RENDER_DURATIONS_SLIDING_WINDOW_MIN: usize = 4;
pub(super) const RENDER_DURATIONS_SLIDING_WINDOW_MAX: usize = 16;
const REPAINT_DELAY_SAFETY_MARGIN: Duration = Duration::from_millis(2);

pub(super) struct GpuRenderDuration {
    pub node: DrmNode,
    pub crtc: crtc::Handle,
    pub duration: Duration,
}

pub(super) type UdevRenderer<'a> =
    MultiRenderer<'a, 'a, GbmGlesBackend<GlesRenderer, DrmDeviceFd>, GbmGlesBackend<GlesRenderer, DrmDeviceFd>>;
pub(super) type UdevRendererError = multigpu::Error<GbmGlesBackend<GlesRenderer, DrmDeviceFd>, GbmGlesBackend<GlesRenderer, DrmDeviceFd>>;

pub(super) type GbmDrmOutput =
    DrmOutput<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, Option<OutputPresentationFeedback>, DrmDeviceFd>;

impl crate::backend::FromGlesError for UdevRendererError {
    fn from_gles_error(err: GlesError) -> Self {
        multigpu::Error::Render(err)
    }
}

impl crate::backend::AsGlesRenderer for UdevRenderer<'_> {
    fn gles_renderer(&self) -> &GlesRenderer {
        self.as_ref()
    }

    fn gles_renderer_mut(&mut self) -> &mut GlesRenderer {
        self.as_mut()
    }

    fn gles_frame<'a, 'frame, 'buffer>(
        frame: &'a Self::Frame<'frame, 'buffer>,
    ) -> &'a smithay::backend::renderer::gles::GlesFrame<'frame, 'buffer> {
        frame.as_ref()
    }

    fn gles_frame_mut<'a, 'frame, 'buffer>(
        frame: &'a mut Self::Frame<'frame, 'buffer>,
    ) -> &'a mut smithay::backend::renderer::gles::GlesFrame<'frame, 'buffer> {
        frame.as_mut()
    }
}

pub(super) struct SurfaceData {
    pub device_id: DrmNode,
    pub connector: connector::Handle,
    pub render_node: Option<DrmNode>,
    pub output: Output,
    pub drm_output: Option<GbmDrmOutput>,
    pub disable_direct_scanout: bool,
    pub dmabuf_feedback: Option<SurfaceDmabufFeedback>,
    pub last_presentation_time: Option<Time<Monotonic>>,
    pub vblank_throttle_timer: Option<RegistrationToken>,
    pub render_durations: VecDeque<Duration>,
    pub repaint_timeout: Option<RegistrationToken>,
    pub destroy_timeout: Option<RegistrationToken>,
    /// A frame has been submitted and we are waiting for the DRM vblank event.
    /// No new frame should be queued until `frame_finish` clears this.
    pub waiting_for_vblank: bool,
    /// A redraw was requested while `waiting_for_vblank` was true.
    /// `frame_finish` will schedule an immediate repaint when it sees this set.
    pub redraw_needed: bool,
}

impl Xfwl4State<UdevData> {
    pub(super) fn frame_finish(&mut self, dev_id: DrmNode, crtc: crtc::Handle, metadata: &mut Option<DrmEventMetadata>) {
        profiling::scope!("frame_finish", &format!("{crtc:?}"));

        let device_backend = match self.backend.backends.get_mut(&dev_id) {
            Some(backend) => backend,
            None => {
                error!("Trying to finish frame on non-existent backend {}", dev_id);
                return;
            }
        };

        let surface = match device_backend.surfaces.get_mut(&crtc) {
            Some(surface) => surface,
            None => {
                error!("Trying to finish frame on non-existent crtc {:?}", crtc);
                return;
            }
        };

        let Some(drm_output) = surface.drm_output.as_ref() else {
            error!("No DRM output for surface");
            return;
        };

        if let Some(timer_token) = surface.vblank_throttle_timer.take() {
            self.core.unregister_timer(timer_token);
        }

        let output = if surface.output.user_data().get::<UdevOutputId>()
            == Some(&UdevOutputId {
                device_id: surface.device_id,
                crtc,
            }) {
            surface.output.clone()
        } else {
            // somehow we got called with an invalid output
            return;
        };

        let Some(frame_duration) = output
            .current_mode()
            .map(|mode| Duration::from_secs_f64(1_000f64 / mode.refresh as f64))
        else {
            return;
        };

        let tp = metadata.as_ref().and_then(|metadata| match metadata.time {
            smithay::backend::drm::DrmEventTime::Monotonic(tp) => tp.is_zero().not().then_some(tp),
            smithay::backend::drm::DrmEventTime::Realtime(_) => None,
        });

        let seq = metadata.as_ref().map(|metadata| metadata.sequence).unwrap_or(0);

        let (clock, flags) = if let Some(tp) = tp {
            (
                tp.into(),
                wp_presentation_feedback::Kind::Vsync
                    | wp_presentation_feedback::Kind::HwClock
                    | wp_presentation_feedback::Kind::HwCompletion,
            )
        } else {
            (self.core.now(), wp_presentation_feedback::Kind::Vsync)
        };

        let vblank_remaining_time = surface
            .last_presentation_time
            .map(|last_presentation_time| frame_duration.saturating_sub(Time::elapsed(&last_presentation_time, clock)));

        if let Some(vblank_remaining_time) = vblank_remaining_time
            && vblank_remaining_time > frame_duration / 2
        {
            static WARN_ONCE: Once = Once::new();
            WARN_ONCE.call_once(|| warn!("display running faster than expected, throttling vblanks and disabling HwClock"));
            let throttled_time = tp.map(|tp| tp.saturating_add(vblank_remaining_time)).unwrap_or(Duration::ZERO);
            let throttled_metadata = DrmEventMetadata {
                sequence: seq,
                time: DrmEventTime::Monotonic(throttled_time),
            };
            let timer_token = self.core.register_timer(Timer::from_duration(vblank_remaining_time), move |state| {
                state.frame_finish(dev_id, crtc, &mut Some(throttled_metadata));
                TimeoutAction::Drop
            });
            surface.vblank_throttle_timer = Some(timer_token);
            return;
        }
        surface.last_presentation_time = Some(clock);
        surface.waiting_for_vblank = false;
        let redraw_needed = std::mem::replace(&mut surface.redraw_needed, false);

        let submit_result = drm_output.frame_submitted().map_err(Into::<SwapBuffersError>::into);

        let schedule_render = match submit_result {
            Ok(user_data) => {
                if let Some(mut feedback) = user_data.flatten() {
                    feedback.presented(clock, Refresh::fixed(frame_duration), seq as u64, flags);
                }

                true
            }
            Err(SwapBuffersError::TemporaryFailure(err)) if matches!(err.downcast_ref::<DrmError>(), Some(&DrmError::DeviceInactive)) => {
                // If the device has been deactivated do not reschedule, this will be done
                // by session resume
                false
            }
            Err(err) => {
                warn!("Error during rendering: {:?}", err);
                match err {
                    SwapBuffersError::AlreadySwapped => true,
                    SwapBuffersError::TemporaryFailure(err) => matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(DrmError::Access(DrmAccessError {
                            source,
                            ..
                        })) if source.kind() == io::ErrorKind::PermissionDenied
                    ),
                    SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {err}"),
                }
            }
        };

        if schedule_render {
            // Cancel any existing repaint timer before scheduling a new one.
            if let Some(old_token) = surface.repaint_timeout.take() {
                self.core.unregister_timer(old_token);
            }

            // What are we trying to solve by introducing a delay here:
            //
            // Basically it is all about latency of client provided buffers.
            // A client driven by frame callbacks will wait for a frame callback
            // to repaint and submit a new buffer. As we send frame callbacks
            // as part of the repaint in the compositor the latency would always
            // be approx. 2 frames. By introducing a delay before we repaint in
            // the compositor we can reduce the latency to approx. 1 frame + the
            // remaining duration from the repaint to the next VBlank.
            //
            // With the delay it is also possible to further reduce latency if
            // the client is driven by presentation feedback. As the presentation
            // feedback is directly sent after a VBlank the client can submit a
            // new buffer during the repaint delay that can hit the very next
            // VBlank, thus reducing the potential latency to below one frame.
            //
            // We could just hard-code a delay (like perhaps 60% of the frame time), but that
            // doesn't account for situations when it takes longer to render, like when a new
            // window appears and we have new things to render (like loading a window icon).
            //
            // So instead, we use a sliding-window approach.  We track up to the last
            // RENDER_DURATIONS_SLIDING_WINDOW_MAX frames worth of actual render times, and use
            // the 90th percentile value to help calculate our delay, with some minimums and
            // maximums and safety margins mixed in.

            let next_frame_target = clock + frame_duration;

            let timer = if redraw_needed {
                // A redraw was requested while a frame was in flight — render immediately.
                trace!("redraw needed after vblank, scheduling immediate repaint on {:?}", crtc);
                Timer::immediate()
            } else if surface.render_durations.len() < RENDER_DURATIONS_SLIDING_WINDOW_MIN
                && surface
                    .render_node
                    .map(|render_node| render_node != self.backend.primary_gpu)
                    .unwrap_or(true)
            {
                // If we have to copy from a different GPU, and we can't compare to recent frames,
                // let's reschedule with no delay to be safe.  Yes, this will increase latency, but
                // we don't want to thrash performance.
                trace!("scheduling repaint timer immediately on {:?}", crtc);
                Timer::immediate()
            } else {
                let predicted_render_time = if surface.render_durations.len() > RENDER_DURATIONS_SLIDING_WINDOW_MIN {
                    let mut sorted = surface.render_durations.iter().copied().collect::<Vec<_>>();
                    sorted.sort();
                    let p90_idx = (sorted.len() * 9) / 10;
                    *sorted.get(p90_idx).unwrap()
                } else {
                    // Not enough data; use a conservative guess.
                    frame_duration.mul_f64(0.4)
                };

                // Give the client a minimum amount of time (20% of the frame duration).
                let min_client_time = frame_duration.mul_f32(0.2);
                // Never delay more than 90% of the frame duration.
                let max_delay = frame_duration.mul_f32(0.8);

                let repaint_delay = frame_duration
                    .saturating_sub(predicted_render_time)
                    .saturating_sub(REPAINT_DELAY_SAFETY_MARGIN)
                    .max(min_client_time)
                    .min(max_delay);
                tracing::trace!(
                    ?predicted_render_time,
                    ?repaint_delay,
                    ?frame_duration,
                    "scheduling repaint on {:?}",
                    crtc,
                );
                Timer::from_duration(repaint_delay)
            };

            let token = self.core.register_timer(timer, move |state| {
                udev_do_render(state, &output, dev_id, crtc, next_frame_target);
                TimeoutAction::Drop
            });
            surface.repaint_timeout = Some(token);
        }
    }
}

impl UdevData {
    pub(super) fn render(
        &mut self,
        core: &mut Xfwl4Core<UdevData>,
        output: &Output,
        node: DrmNode,
        crtc: crtc::Handle,
        frame_target: Time<Monotonic>,
    ) -> Result<(Option<SurfaceDmabufFeedback>, Option<RenderElementStates>), RenderFailure> {
        profiling::scope!("render", &format!("{crtc:?}"));

        let device = self.backends.get_mut(&node).ok_or(RenderFailure::NotNeeded)?;
        let surface = device.surfaces.get_mut(&crtc).ok_or(RenderFailure::NotNeeded)?;
        let drm_output = surface
            .drm_output
            .as_mut()
            .ok_or_else(|| RenderFailure::Error(anyhow!("No DRM output for surface")))?;

        let start = Instant::now();

        let primary_gpu = self.primary_gpu;
        let render_node = surface.render_node.unwrap_or(primary_gpu);
        let mut renderer = if primary_gpu == render_node {
            self.gpus.single_renderer(&render_node)
        } else {
            let format = drm_output.format();
            self.gpus.renderer(&primary_gpu, &render_node, format)
        }
        .context("Failed to find renderer for surface")
        .map_err(RenderFailure::Error)?;

        let (elements, clear_color) = core.prepare_render(output, frame_target, &mut renderer);

        let frame_mode = if surface.disable_direct_scanout {
            FrameFlags::empty()
        } else {
            FrameFlags::DEFAULT
        };
        let result = drm_output
            .render_frame(&mut renderer, &elements, clear_color, frame_mode)
            .map_err(|err| match err {
                smithay::backend::drm::compositor::RenderFrameError::PrepareFrame(err) => SwapBuffersError::from(err),
                smithay::backend::drm::compositor::RenderFrameError::RenderFrame(OutputDamageTrackerError::Rendering(err)) => {
                    SwapBuffersError::from(err)
                }
                _ => unreachable!(),
            })
            .and_then(|render_frame_result| {
                let sync = if let PrimaryPlaneElement::Swapchain(ref element) = render_frame_result.primary_element {
                    Some(element.sync.clone())
                } else {
                    None
                };

                if !render_frame_result.is_empty {
                    let output_presentation_feedback = core.take_presentation_feedback(output, &render_frame_result.states);
                    drm_output
                        .queue_frame(Some(output_presentation_feedback))
                        .map_err(Into::<SwapBuffersError>::into)
                        .map(|_| (!render_frame_result.is_empty, render_frame_result.states, sync))
                } else {
                    Ok((!render_frame_result.is_empty, render_frame_result.states, sync))
                }
            });

        let (reschedule, dmabuf_feedback, states) = match result {
            Ok((has_rendered, states, sync)) => {
                if has_rendered {
                    // Frame was submitted; wait for vblank before rendering again.
                    surface.waiting_for_vblank = true;
                    surface.redraw_needed = false;
                    let tx = self.gpu_render_duration_tx.clone();
                    std::thread::spawn(move || {
                        if let Some(sync) = sync {
                            let _ = sync.wait();
                        }
                        let elapsed = start.elapsed();
                        tracing::trace!(?elapsed, "rendered surface (gpu)");
                        let _ = tx.send(GpuRenderDuration {
                            node,
                            crtc,
                            duration: elapsed,
                        });
                    });
                }
                let dmabuf_feedback = surface.dmabuf_feedback.clone();
                (!has_rendered, dmabuf_feedback, Some(states))
            }
            Err(SwapBuffersError::TemporaryFailure(err)) if matches!(err.downcast_ref::<DrmError>(), Some(&DrmError::DeviceInactive)) => {
                // If the device has been deactivated do not reschedule, this will be done
                // by session resume
                (false, None, None)
            }
            Err(err) => {
                warn!("Error during rendering: {:#?}", err);
                let reschedule = match err {
                    SwapBuffersError::AlreadySwapped => false,
                    SwapBuffersError::TemporaryFailure(err) => match err.downcast_ref::<DrmError>() {
                        Some(DrmError::Access(DrmAccessError { source, .. })) => source.kind() == io::ErrorKind::PermissionDenied,
                        _ => false,
                    },
                    SwapBuffersError::ContextLost(err) => match err.downcast_ref::<DrmError>() {
                        Some(DrmError::TestFailed(_)) => {
                            // reset the complete state, disabling all connectors and planes in case we hit a test failed
                            // most likely we hit this after a tty switch when a foreign master changed CRTC <-> connector bindings
                            // and we run in a mismatch
                            device
                                .drm_output_manager
                                .device_mut()
                                .reset_state()
                                .expect("failed to reset drm device");
                            true
                        }
                        _ => panic!("Rendering loop lost: {err}"),
                    },
                };
                (reschedule, None, None)
            }
        };

        if reschedule && let Some(output_refresh) = output.current_mode().map(|mode| mode.refresh) {
            // If reschedule is true we either hit a temporary failure or more likely rendering
            // did not cause any damage on the output. In this case we just re-schedule a repaint
            // after approx. one frame to re-test for damage.
            let next_frame_target = frame_target + Duration::from_millis(1_000_000 / output_refresh as u64);
            let reschedule_timeout = Duration::from(next_frame_target).saturating_sub(core.now().into());
            trace!("reschedule repaint timer with delay {:?} on {:?}", reschedule_timeout, crtc,);
            // Cancel any existing repaint timer before scheduling a new one to prevent stacking.
            if let Some(old_token) = surface.repaint_timeout.take() {
                core.unregister_timer(old_token);
            }
            let timer = Timer::from_duration(reschedule_timeout);
            let output = output.clone();
            let token = core.register_timer(timer, move |state| {
                udev_do_render(state, &output, node, crtc, next_frame_target);
                TimeoutAction::Drop
            });
            surface.repaint_timeout = Some(token);
        }

        profiling::finish_frame!();

        Ok((dmabuf_feedback, states))
    }
}

pub(super) fn udev_do_render(
    state: &mut Xfwl4State<UdevData>,
    output: &Output,
    node: DrmNode,
    crtc: crtc::Handle,
    frame_target: Time<Monotonic>,
) {
    // If a frame is already in flight, defer until frame_finish() clears the vblank wait.
    if let Some(surface) = state
        .backend
        .backends
        .get_mut(&node)
        .and_then(|b| b.surfaces.get_mut(&crtc))
    {
        if surface.waiting_for_vblank {
            surface.redraw_needed = true;
            return;
        }
        // The timer fired — clear its token so frame_finish won't try to cancel a stale one.
        surface.repaint_timeout = None;
    }

    state.render(output, frame_target, |backend, core| {
        backend.render(core, output, node, crtc, frame_target)
    });
}
