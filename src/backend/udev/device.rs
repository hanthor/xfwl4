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
    cell::RefCell,
    collections::{VecDeque, hash_map::HashMap},
    path::Path,
    time::Duration,
};

use crate::{
    backend::udev::{
        GbmGpuManager, UdevData,
        render::{SurfaceData, UdevRenderer},
        udev_do_render,
    },
    core::{render::*, shell::WindowRenderElement, state::Xfwl4State},
    protocols::{wlr_gamma_control::WlrGammaControlState, wlr_output_power_management::WlrOutputPowerManagementState},
};

use anyhow::{Context, anyhow};
use bytes::Bytes;
use smithay::{
    backend::{
        allocator::{
            Fourcc, Modifier,
            dmabuf::Dmabuf,
            format::FormatSet,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        },
        drm::{
            CreateDrmNodeError, DrmDevice, DrmDeviceFd, DrmDeviceNotifier, DrmError, DrmEvent, DrmNode, DrmSurface,
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutputManager, DrmOutputRenderElements},
        },
        egl::{self, EGLDevice, EGLDisplay},
        renderer::{
            DebugFlags, ImportDma,
            gles::GlesRenderer,
            multigpu::{GpuManager, gbm::GbmGlesBackend},
        },
        session::{
            Session,
            libseat::{self},
        },
    },
    delegate_dmabuf, delegate_drm_lease, delegate_drm_syncobj,
    desktop::utils::OutputPresentationFeedback,
    output::{Mode as WlMode, Output, PhysicalProperties},
    reexports::{
        calloop::{
            InsertError, LoopHandle, RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        drm::{
            Device as _,
            control::{Device, ModeFlags, ModeTypeFlags, connector, crtc},
        },
        rustix::fs::OFlags,
        wayland_protocols::wp::linux_dmabuf::zv1::server::zwp_linux_dmabuf_feedback_v1,
        wayland_protocols_wlr::output_power_management::v1::server::zwlr_output_power_v1::Mode as PowerMode,
    },
    utils::DeviceFd,
    wayland::{
        dmabuf::{DmabufFeedbackBuilder, DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
        drm_lease::{DrmLease, DrmLeaseBuilder, DrmLeaseHandler, DrmLeaseRequest, DrmLeaseState, LeaseRejected},
        drm_syncobj::{DrmSyncobjHandler, DrmSyncobjState},
    },
};
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};
use tracing::{debug, error, info, warn};

// we cannot simply pick the first supported format of the intersection of *all* formats, because:
// - we do not want something like Abgr4444, which looses color information, if something better is available
// - some formats might perform terribly
// - we might need some work-arounds, if one supports modifiers, but the other does not
//
// So lets just pick `ARGB2101010` (10-bit) or `ARGB8888` (8-bit) for now, they are widely supported.
const SUPPORTED_FORMATS: &[Fourcc] = &[Fourcc::Abgr2101010, Fourcc::Argb2101010, Fourcc::Abgr8888, Fourcc::Argb8888];
const SUPPORTED_FORMATS_8BIT_ONLY: &[Fourcc] = &[Fourcc::Abgr8888, Fourcc::Argb8888];
const SURFACE_DESTROY_DELAY: Duration = Duration::from_secs(2);

pub(super) type GbmDrmOutputManager =
    DrmOutputManager<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, Option<OutputPresentationFeedback>, DrmDeviceFd>;

pub(super) struct BackendData {
    pub surfaces: HashMap<crtc::Handle, SurfaceData>,
    pub non_desktop_connectors: Vec<(connector::Handle, crtc::Handle)>,
    pub leasing_global: Option<DrmLeaseState>,
    pub active_leases: Vec<DrmLease>,
    pub drm_output_manager: GbmDrmOutputManager,
    drm_scanner: DrmScanner,
    render_node: Option<DrmNode>,
    registration_token: RegistrationToken,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct UdevOutputId {
    pub device_id: DrmNode,
    pub crtc: crtc::Handle,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum DeviceAddError {
    #[error("Failed to open device using libseat: {0}")]
    DeviceOpen(libseat::Error),
    #[error("Failed to initialize drm device: {0}")]
    DrmDevice(DrmError),
    #[error("Failed to initialize gbm device: {0}")]
    GbmDevice(std::io::Error),
    #[error("Failed to access drm node: {0}")]
    DrmNode(CreateDrmNodeError),
    #[error("Failed to add device to GpuManager: {0}")]
    AddNode(egl::Error),
    #[error("The device has no render node")]
    NoRenderNode,
    #[error("Primary GPU is missing")]
    PrimaryGpuMissing,
    #[error("Failed to insert source into event loop: {0}")]
    EventLoop(InsertError<DrmDeviceNotifier>),
}

struct DisplayInfo {
    edid: Bytes,
    info: Option<libdisplay_info::info::Info>,
}

impl Xfwl4State<UdevData> {
    pub(super) fn device_added(
        &mut self,
        handle: LoopHandle<'_, Xfwl4State<UdevData>>,
        node: DrmNode,
        path: &Path,
    ) -> Result<(), DeviceAddError> {
        // Try to open the device
        let fd = self
            .backend
            .session
            .open(path, OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK)
            .map_err(DeviceAddError::DeviceOpen)?;

        let fd = DrmDeviceFd::new(DeviceFd::from(fd));

        let (drm, notifier) = DrmDevice::new(fd.clone(), true).map_err(DeviceAddError::DrmDevice)?;
        let gbm = GbmDevice::new(fd).map_err(DeviceAddError::GbmDevice)?;

        let registration_token = handle
            .insert_source(notifier, move |event, metadata, state: &mut Xfwl4State<_>| match event {
                DrmEvent::VBlank(crtc) => {
                    profiling::scope!("vblank", &format!("{crtc:?}"));
                    state.frame_finish(node, crtc, metadata);
                }
                DrmEvent::Error(error) => {
                    error!("{:?}", error);
                }
            })
            .map_err(DeviceAddError::EventLoop)?;

        let mut try_initialize_gpu = || {
            let display = unsafe { EGLDisplay::new(gbm.clone()).map_err(DeviceAddError::AddNode)? };
            let egl_device = EGLDevice::device_for_display(&display).map_err(DeviceAddError::AddNode)?;

            if egl_device.is_software() {
                warn!("EGL device is software-rendered (llvmpipe); proceeding anyway for VM/test environments");
            }

            let render_node = egl_device.try_get_render_node().ok().flatten().unwrap_or(node);
            self.backend
                .gpus
                .as_mut()
                .add_node(render_node, gbm.clone())
                .map_err(DeviceAddError::AddNode)?;

            std::result::Result::<DrmNode, DeviceAddError>::Ok(render_node)
        };

        let render_node = try_initialize_gpu()
            .inspect_err(|err| {
                warn!(?err, "failed to initialize gpu");
            })
            .ok();

        let allocator = render_node
            .is_some()
            .then(|| GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT))
            .or_else(|| {
                self.backend
                    .backends
                    .get(&self.backend.primary_gpu)
                    .or_else(|| {
                        self.backend
                            .backends
                            .values()
                            .find(|backend| backend.render_node == Some(self.backend.primary_gpu))
                    })
                    .map(|backend| backend.drm_output_manager.allocator().clone())
            })
            .ok_or(DeviceAddError::PrimaryGpuMissing)?;

        let framebuffer_exporter = GbmFramebufferExporter::new(gbm.clone(), render_node.into());

        let color_formats = if self.backend.disable_10bit_color {
            SUPPORTED_FORMATS_8BIT_ONLY
        } else {
            SUPPORTED_FORMATS
        };
        let mut renderer = self
            .backend
            .gpus
            .single_renderer(&render_node.unwrap_or(self.backend.primary_gpu))
            .map_err(|_| DeviceAddError::NoRenderNode)?;
        let render_formats = renderer
            .as_mut()
            .egl_context()
            .dmabuf_render_formats()
            .iter()
            .filter(|format| render_node.is_some() || format.modifier == Modifier::Linear)
            .copied()
            .collect::<FormatSet>();

        let drm_output_manager = DrmOutputManager::new(
            drm,
            allocator,
            framebuffer_exporter,
            Some(gbm),
            color_formats.iter().copied(),
            render_formats,
        );

        self.backend.backends.insert(
            node,
            BackendData {
                registration_token,
                drm_output_manager,
                drm_scanner: DrmScanner::new(),
                non_desktop_connectors: Vec::new(),
                render_node,
                surfaces: HashMap::new(),
                leasing_global: DrmLeaseState::new::<Xfwl4State<UdevData>>(&self.core.display_handle, &node)
                    .inspect_err(|err| {
                        warn!(?err, "Failed to initialize drm lease global for: {}", node);
                    })
                    .ok(),
                active_leases: Vec::new(),
            },
        );

        self.device_changed(node);

        Ok(())
    }

    fn connector_connected(&mut self, node: DrmNode, connector: connector::Info, crtc: crtc::Handle) -> anyhow::Result<()> {
        if let Some(device) = self.backend.backends.get_mut(&node)
            && let Some(surface) = device.surfaces.get_mut(&crtc)
        {
            // We already know about this connector; so just destroy the timer; nothing else we
            // need to do.
            if let Some(token) = surface.destroy_timeout.take() {
                self.core.unregister_timer(token);
            }
            Ok(())
        } else {
            self.add_connector(node, connector, crtc)
        }
    }

    fn add_connector(&mut self, node: DrmNode, connector: connector::Info, crtc: crtc::Handle) -> anyhow::Result<()> {
        if let Some(device) = self.backend.backends.get_mut(&node) {
            let output_name = format!("{}-{}", connector.interface().as_str(), connector.interface_id());
            info!(?crtc, "Trying to setup connector {}", output_name,);

            let drm_device = device.drm_output_manager.device();

            let non_desktop = drm_device
                .get_properties(connector.handle())
                .ok()
                .and_then(|props| {
                    let (info, value) = props
                        .into_iter()
                        .filter_map(|(handle, value)| {
                            let info = drm_device.get_property(handle).ok()?;

                            Some((info, value))
                        })
                        .find(|(info, _)| info.name().to_str() == Ok("non-desktop"))?;

                    info.value_type().convert_value(value).as_boolean()
                })
                .unwrap_or(false);

            let (edid, display_info) = display_info_for_connector(drm_device, connector.handle())
                .map(|DisplayInfo { edid, info }| (edid, info))
                .unwrap_or_else(|| (Bytes::new(), None));

            let make = display_info
                .as_ref()
                .and_then(|info| info.make())
                .unwrap_or_else(|| "Unknown".into());

            let model = display_info
                .as_ref()
                .and_then(|info| info.model())
                .unwrap_or_else(|| "Unknown".into());

            let serial_number = display_info
                .as_ref()
                .and_then(|info| info.serial())
                .unwrap_or_else(|| "Unknown".into());

            if non_desktop {
                info!("Connector {} is non-desktop, setting up for leasing", output_name);
                device.non_desktop_connectors.push((connector.handle(), crtc));
                if let Some(lease_state) = device.leasing_global.as_mut() {
                    lease_state.add_connector::<Xfwl4State<UdevData>>(connector.handle(), output_name, format!("{make} {model}"));
                }
            } else {
                let drm_mode = connector
                    .modes()
                    .iter()
                    .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
                    .or_else(|| connector.modes().first())
                    .ok_or_else(|| anyhow!("No valid modes for connector"))?;
                let wl_mode = WlMode::from(*drm_mode);

                let (phys_w, phys_h) = connector.size().unwrap_or((0, 0));
                let output = Output::new(
                    output_name.clone(),
                    PhysicalProperties {
                        size: (phys_w as i32, phys_h as i32).into(),
                        subpixel: connector.subpixel().into(),
                        make,
                        model,
                        serial_number,
                    },
                );
                output.add_mode(wl_mode);
                for drm_mode in connector.modes() {
                    output.add_mode(WlMode::from(*drm_mode));
                }

                output.set_preferred(wl_mode);
                output.change_current_state(Some(wl_mode), None, None, None);

                output.user_data().insert_if_missing(|| UdevOutputId { crtc, device_id: node });

                let surface = SurfaceData {
                    device_id: node,
                    connector: connector.handle(),
                    render_node: device.render_node,
                    output: output.clone(),
                    drm_output: None,
                    disable_direct_scanout: self.backend.disable_direct_scanout,
                    dmabuf_feedback: None,
                    last_presentation_time: None,
                    vblank_throttle_timer: None,
                    render_durations: VecDeque::new(),
                    repaint_timeout: None,
                    destroy_timeout: None,
                };

                device.surfaces.insert(crtc, surface);

                self.output_created(&output, edid);
            }
        }

        Ok(())
    }

    fn connector_disconnected(&mut self, node: DrmNode, connector: connector::Info, crtc: crtc::Handle) -> anyhow::Result<()> {
        if let Some(device) = self.backend.backends.get_mut(&node)
            && let Some(surface) = device.surfaces.get_mut(&crtc)
        {
            // Sometimes we can get spurious disconnects when reconfiguring a connector/CRTC (e.g.
            // DisplayPort link training glitches), so we schedule a timer before actually tearing
            // down the SurfaceData.

            if let Some(token) = surface.destroy_timeout.take() {
                self.core.unregister_timer(token);
            }

            let connector = RefCell::new(Some(connector));
            surface.destroy_timeout = Some(self.core.register_timer(Timer::from_duration(SURFACE_DESTROY_DELAY), move |state| {
                if let Some(connector) = connector.borrow_mut().take()
                    && let Err(err) = state.destroy_connector(node, connector, crtc)
                {
                    tracing::warn!("Failed to destroy connector on crtc {crtc:?}: {err}");
                }
                TimeoutAction::Drop
            }));
        }

        Ok(())
    }

    fn destroy_connector(&mut self, node: DrmNode, connector: connector::Info, crtc: crtc::Handle) -> anyhow::Result<()> {
        if let Some(device) = self.backend.backends.get_mut(&node) {
            let destroyed_output = if let Some(pos) = device
                .non_desktop_connectors
                .iter()
                .position(|(handle, _)| *handle == connector.handle())
            {
                let _ = device.non_desktop_connectors.remove(pos);
                if let Some(leasing_state) = device.leasing_global.as_mut() {
                    leasing_state.withdraw_connector(connector.handle());
                }
                None
            } else {
                device.surfaces.remove(&crtc).map(|surface| surface.output.clone())
            };

            let render_node = device.render_node.unwrap_or(self.backend.primary_gpu);
            let mut renderer = self.backend.gpus.single_renderer(&render_node).context("Failed to get renderer")?;
            let _ = device
                .drm_output_manager
                .lock()
                .try_to_restore_modifiers::<_, OutputRenderElements<UdevRenderer<'_>, WindowRenderElement<UdevRenderer<'_>>>>(
                    &mut renderer,
                    // FIXME: For a flicker free operation we should return the actual elements for this output..
                    // Instead we just use black to "simulate" a modeset :)
                    &DrmOutputRenderElements::default(),
                );

            if let Some(output) = destroyed_output {
                self.backend.wlr_output_power_management_state.output_destroyed(&output);
                self.backend.wlr_gamma_control_state.output_destroyed(&output);

                self.output_destroyed(&output);
            }
        }

        Ok(())
    }

    pub(super) fn device_changed(&mut self, node: DrmNode) {
        let device = if let Some(device) = self.backend.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        let scan_result = match device.drm_scanner.scan_connectors(device.drm_output_manager.device()) {
            Ok(scan_result) => scan_result,
            Err(err) => {
                tracing::warn!(?err, "Failed to scan connectors");
                return;
            }
        };

        for event in scan_result {
            if let Err(err) = match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => self.connector_connected(node, connector, crtc),
                DrmScanEvent::Disconnected {
                    connector,
                    crtc: Some(crtc),
                } => self.connector_disconnected(node, connector, crtc),
                _ => Ok(()),
            } {
                warn!("Failed to handle DRM scanner event: {err}");
            }
        }
    }

    pub(super) fn device_removed(&mut self, handle: LoopHandle<'_, Xfwl4State<UdevData>>, node: DrmNode) {
        let device = if let Some(device) = self.backend.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        let crtcs: Vec<_> = device.drm_scanner.crtcs().map(|(info, crtc)| (info.clone(), crtc)).collect();

        for (connector, crtc) in crtcs {
            if let Err(err) = self.connector_disconnected(node, connector, crtc) {
                warn!("Failed to disconnect connector for removed device node {node}: {err}");
            }
        }

        debug!("Surfaces dropped");

        // drop the backends on this side
        if let Some(mut backend_data) = self.backend.backends.remove(&node) {
            if let Some(mut leasing_global) = backend_data.leasing_global.take() {
                leasing_global.disable_global::<Xfwl4State<UdevData>>();
            }

            if let Some(render_node) = backend_data.render_node {
                self.backend.gpus.as_mut().remove_node(&render_node);
            }

            handle.remove(backend_data.registration_token);

            debug!("Dropping device");
        }
    }
}

impl UdevData {
    fn node_and_crtc_for_output(&self, output: &Output) -> Option<(DrmNode, crtc::Handle)> {
        self.backends.iter().find_map(|(node, backend_data)| {
            backend_data.surfaces.iter().find_map(
                |(crtc, surface)| {
                    if surface.output == *output { Some((*node, *crtc)) } else { None }
                },
            )
        })
    }

    pub(super) fn change_output_mode(
        &mut self,
        handle: LoopHandle<'_, Xfwl4State<Self>>,
        output: &Output,
        mode: WlMode,
    ) -> anyhow::Result<(bool, WlMode)> {
        let (node, crtc) = self
            .node_and_crtc_for_output(output)
            .ok_or_else(|| anyhow!("Unable to find surface for output {}", output.name()))?;

        let backend_data = self
            .backends
            .get_mut(&node)
            .ok_or_else(|| anyhow!("Unable to find backend for node"))?;
        let surface = backend_data
            .surfaces
            .get_mut(&crtc)
            .ok_or_else(|| anyhow!("Unable to find surface for crtc"))?;
        let device = backend_data.drm_output_manager.device();

        let connector = device
            .get_connector(surface.connector, false)
            .map_err(|err| anyhow!("Failed to get connector for output: {err}"))?;

        let drm_mode = connector
            .modes()
            .iter()
            .filter(|drm_mode| drm_mode.size().0 as i32 == mode.size.w && drm_mode.size().1 as i32 == mode.size.h)
            .min_by_key(|drm_mode| {
                tracing::debug!(
                    "drm vrefresh: {}, target vrefresh: {}",
                    vrefresh_rate_for_drm_mode(drm_mode),
                    mode.refresh
                );
                (vrefresh_rate_for_drm_mode(drm_mode) as i32 - mode.refresh).abs()
            })
            .ok_or_else(|| anyhow!("Unable to find DRM mode for mode"))?;

        let needed_enable = if let Some(drm_output) = surface.drm_output.as_ref() {
            drm_output.with_compositor(|compositor| compositor.use_mode(*drm_mode))?;
            false
        } else {
            enable_connector(
                &mut backend_data.drm_output_manager,
                &mut self.gpus,
                self.primary_gpu,
                surface,
                *drm_mode,
                self.debug_flags,
                handle,
                &mut self.wlr_output_power_management_state,
                &mut self.wlr_gamma_control_state,
            )?;
            true
        };

        Ok((
            needed_enable,
            WlMode {
                size: (drm_mode.size().0 as i32, drm_mode.size().1 as i32).into(),
                refresh: vrefresh_rate_for_drm_mode(drm_mode) as i32,
            },
        ))
    }

    pub(super) fn disable_output_internal(&mut self, output: &Output) -> anyhow::Result<()> {
        let (node, crtc) = self
            .node_and_crtc_for_output(output)
            .ok_or_else(|| anyhow!("Unable to find surface for output {}", output.name()))?;

        let backend_data = self
            .backends
            .get_mut(&node)
            .ok_or_else(|| anyhow!("Unable to find backend for node"))?;
        let surface = backend_data
            .surfaces
            .get_mut(&crtc)
            .ok_or_else(|| anyhow!("Unable to find surface for crtc"))?;

        // Dropping the DrmOutput causes smithay to reset all planes, connectors, CRTCs and
        // fully disable the output.
        surface.drm_output = None;

        self.wlr_output_power_management_state.output_destroyed(output);
        self.wlr_gamma_control_state.output_destroyed(output);

        Ok(())
    }
}

impl DmabufHandler for Xfwl4State<UdevData> {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.backend.dmabuf_state.as_mut().unwrap().0
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        if self
            .backend
            .gpus
            .single_renderer(&self.backend.primary_gpu)
            .and_then(|mut renderer| renderer.import_dmabuf(&dmabuf, None))
            .is_ok()
        {
            dmabuf.set_node(self.backend.primary_gpu);
            let _ = notifier.successful::<Xfwl4State<UdevData>>();
        } else {
            notifier.failed();
        }
    }
}
delegate_dmabuf!(Xfwl4State<UdevData>);

impl DrmLeaseHandler for Xfwl4State<UdevData> {
    fn drm_lease_state(&mut self, node: DrmNode) -> &mut DrmLeaseState {
        self.backend.backends.get_mut(&node).unwrap().leasing_global.as_mut().unwrap()
    }

    fn lease_request(&mut self, node: DrmNode, request: DrmLeaseRequest) -> Result<DrmLeaseBuilder, LeaseRejected> {
        let backend = self.backend.backends.get(&node).ok_or(LeaseRejected::default())?;

        let drm_device = backend.drm_output_manager.device();
        let mut builder = DrmLeaseBuilder::new(drm_device);
        for conn in request.connectors {
            if let Some((_, crtc)) = backend.non_desktop_connectors.iter().find(|(handle, _)| *handle == conn) {
                builder.add_connector(conn);
                builder.add_crtc(*crtc);
                let planes = drm_device.planes(crtc).map_err(LeaseRejected::with_cause)?;
                let (primary_plane, primary_plane_claim) = planes
                    .primary
                    .iter()
                    .find_map(|plane| drm_device.claim_plane(plane.handle, *crtc).map(|claim| (plane, claim)))
                    .ok_or_else(LeaseRejected::default)?;
                builder.add_plane(primary_plane.handle, primary_plane_claim);
                if let Some((cursor, claim)) = planes
                    .cursor
                    .iter()
                    .find_map(|plane| drm_device.claim_plane(plane.handle, *crtc).map(|claim| (plane, claim)))
                {
                    builder.add_plane(cursor.handle, claim);
                }
            } else {
                tracing::warn!(?conn, "Lease requested for desktop connector, denying request");
                return Err(LeaseRejected::default());
            }
        }

        Ok(builder)
    }

    fn new_active_lease(&mut self, node: DrmNode, lease: DrmLease) {
        if let Some(backend) = self.backend.backends.get_mut(&node) {
            backend.active_leases.push(lease);
        } else {
            warn!("Matching backend for node {node} not found for new active DRM lease");
        }
    }

    fn lease_destroyed(&mut self, node: DrmNode, lease: u32) {
        if let Some(backend) = self.backend.backends.get_mut(&node) {
            backend.active_leases.retain(|l| l.id() != lease);
        } else {
            warn!("Matching backend for node {node} not found for destroyed DRM lease");
        }
    }
}

delegate_drm_lease!(Xfwl4State<UdevData>);

impl DrmSyncobjHandler for Xfwl4State<UdevData> {
    fn drm_syncobj_state(&mut self) -> Option<&mut DrmSyncobjState> {
        self.backend.syncobj_state.as_mut()
    }
}
delegate_drm_syncobj!(Xfwl4State<UdevData>);

pub(super) fn get_surface_dmabuf_feedback(
    primary_gpu: DrmNode,
    render_node: Option<DrmNode>,
    scanout_node: DrmNode,
    gpus: &mut GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    surface: &DrmSurface,
) -> Option<SurfaceDmabufFeedback> {
    let primary_formats = gpus.single_renderer(&primary_gpu).ok()?.dmabuf_formats();
    let render_formats = if let Some(render_node) = render_node {
        gpus.single_renderer(&render_node).ok()?.dmabuf_formats()
    } else {
        FormatSet::default()
    };

    let all_render_formats = primary_formats.iter().chain(render_formats.iter()).copied().collect::<FormatSet>();

    let planes = surface.planes().clone();

    // We limit the scan-out tranche to formats we can also render from
    // so that there is always a fallback render path available in case
    // the supplied buffer can not be scanned out directly
    let planes_formats = surface
        .plane_info()
        .formats
        .iter()
        .copied()
        .chain(planes.overlay.into_iter().flat_map(|p| p.formats))
        .collect::<FormatSet>()
        .intersection(&all_render_formats)
        .copied()
        .collect::<FormatSet>();

    let builder = DmabufFeedbackBuilder::new(primary_gpu.dev_id(), primary_formats);
    let render_feedback = if let Some(render_node) = render_node {
        builder
            .clone()
            .add_preference_tranche(render_node.dev_id(), None, render_formats.clone())
            .build()
    } else {
        builder.clone().build()
    };

    render_feedback
        .inspect_err(|err| warn!("Failed to build DMABUF renderer feedback: {err}"))
        .ok()
        .and_then(|render_feedback| {
            surface
                .device_fd()
                .dev_id()
                .inspect_err(|err| warn!("Unable to get device ID for DMABUF feedback surface: {err}"))
                .ok()
                .and_then(|surface_dev_id| {
                    builder
                        .add_preference_tranche(
                            surface_dev_id,
                            Some(zwp_linux_dmabuf_feedback_v1::TrancheFlags::Scanout),
                            planes_formats,
                        )
                        .add_preference_tranche(scanout_node.dev_id(), None, render_formats)
                        .build()
                        .inspect_err(|err| warn!("Failed to build DMABUF scanout feedback: {err}"))
                        .ok()
                        .map(|scanout_feedback| SurfaceDmabufFeedback {
                            render_feedback,
                            scanout_feedback,
                        })
                })
        })
}

#[allow(clippy::too_many_arguments)]
fn enable_connector(
    drm_output_manager: &mut GbmDrmOutputManager,
    gpus: &mut GbmGpuManager,
    primary_gpu: DrmNode,
    surface: &mut SurfaceData,
    drm_mode: smithay::reexports::drm::control::Mode,
    debug_flags: DebugFlags,
    handle: LoopHandle<'_, Xfwl4State<UdevData>>,
    wlr_output_power_management_state: &mut WlrOutputPowerManagementState,
    wlr_gamma_control_state: &mut WlrGammaControlState,
) -> anyhow::Result<()> {
    let UdevOutputId {
        crtc,
        device_id: scanout_node,
    } = surface
        .output
        .user_data()
        .get::<UdevOutputId>()
        .cloned()
        .ok_or_else(|| anyhow!("No crtc or scanout node found for output {}", surface.output.name()))?;

    let drm_device = drm_output_manager.device();

    let (mut red, mut green, mut blue) = (Vec::default(), Vec::default(), Vec::default());
    let orig_gamma = match drm_device.get_gamma(crtc, &mut red, &mut green, &mut blue) {
        Ok(_) => Some((red, green, blue)),
        Err(err) => {
            warn!("Failed to get current gamma ramps for output: {err}");
            None
        }
    };
    let crtc_info = drm_device.get_crtc(crtc);

    let driver = drm_device.get_driver().context("Failed to query DRM driver")?;

    let mut planes = drm_device.planes(&crtc).context("Failed to query crtc planes")?;

    // Using an overlay plane on a nvidia card breaks
    if driver.name().to_string_lossy().to_lowercase().contains("nvidia")
        || driver.description().to_string_lossy().to_lowercase().contains("nvidia")
    {
        planes.overlay = vec![];
    }

    let render_node = surface.render_node.as_ref().unwrap_or(&primary_gpu);
    let mut renderer = gpus.single_renderer(render_node).context("Failed to get renderer")?;

    let drm_output = drm_output_manager
        .lock()
        .initialize_output::<_, OutputRenderElements<UdevRenderer<'_>, WindowRenderElement<UdevRenderer<'_>>>>(
            crtc,
            drm_mode,
            &[surface.connector],
            &surface.output,
            Some(planes),
            &mut renderer,
            &DrmOutputRenderElements::default(),
        )
        .context("Failed to initialize drm output")?;

    let dmabuf_feedback = drm_output.with_compositor(|compositor| {
        compositor.set_debug_flags(debug_flags);

        get_surface_dmabuf_feedback(primary_gpu, surface.render_node, scanout_node, gpus, compositor.surface())
    });

    surface.drm_output = Some(drm_output);
    surface.dmabuf_feedback = dmabuf_feedback;

    match crtc_info {
        Ok(crtc_info) => wlr_gamma_control_state.output_created(&surface.output, orig_gamma, crtc_info.gamma_length()),
        Err(err) => warn!("Failed to get CRTC info from DRM device: {err}"),
    }

    wlr_output_power_management_state.output_created::<Xfwl4State<UdevData>>(&surface.output, PowerMode::On);

    // kick-off rendering
    handle
        .insert_source(Timer::immediate(), {
            let output = surface.output.clone();
            move |_, _, state| {
                udev_do_render(state, &output, scanout_node, crtc, state.core.now());
                TimeoutAction::Drop
            }
        })
        .expect("Failed to insert rendering timer source");

    Ok(())
}

// mode.vrefresh() returns a rounded value in Hz, but we really want mHz
fn vrefresh_rate_for_drm_mode(mode: &smithay::reexports::drm::control::Mode) -> u32 {
    let htotal = mode.hsync().2 as u32;
    let vtotal = mode.vsync().2 as u32;
    let mut refresh = (mode.clock() as u64 * 1000000_u64 / htotal as u64 + vtotal as u64 / 2) / vtotal as u64;

    if mode.flags().contains(ModeFlags::INTERLACE) {
        refresh *= 2;
    }
    if mode.flags().contains(ModeFlags::DBLSCAN) {
        refresh /= 2;
    }
    if mode.vscan() > 1 {
        refresh /= mode.vscan() as u64;
    }

    refresh as u32
}

// Copied from smithay-drm-extras, modified to return a hash of the EDID.
fn display_info_for_connector(device: &impl Device, connector: connector::Handle) -> Option<DisplayInfo> {
    let props = device.get_properties(connector).ok()?;

    let (info, value) = props
        .into_iter()
        .filter_map(|(handle, value)| {
            let info = device.get_property(handle).ok()?;

            Some((info, value))
        })
        .find(|(info, _)| info.name().to_str() == Ok("EDID"))?;

    let blob = info.value_type().convert_value(value).as_blob()?;
    let data = device.get_property_blob(blob).ok()?;

    let info = libdisplay_info::info::Info::parse_edid(&data).ok();

    Some(DisplayInfo {
        edid: Bytes::from(data),
        info,
    })
}
