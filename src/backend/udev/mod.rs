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

use std::{collections::hash_map::HashMap, path::PathBuf};

use crate::{
    backend::{
        Backend,
        udev::{
            device::{BackendData, DeviceAddError, UdevOutputId, get_surface_dmabuf_feedback},
            render::udev_do_render,
        },
    },
    core::{config::PointerConfig, input_handler::KeyAction, state::Xfwl4State, util::ClientExt},
    protocols::{wlr_gamma_control::WlrGammaControlState, wlr_output_power_management::WlrOutputPowerManagementState},
};

use anyhow::{Context, anyhow};
#[cfg(feature = "egl")]
use smithay::backend::renderer::ImportEgl;
use smithay::{
    backend::{
        allocator::{Fourcc, Modifier, dmabuf::Dmabuf},
        drm::{DrmDeviceFd, DrmNode, NodeType},
        egl::{self, EGLContext, context::ContextPriority},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            Bind, DebugFlags, ImportDma, ImportMemWl,
            gles::{Capability, GlesRenderer},
            multigpu::{GpuManager, MultiTexture, gbm::GbmGlesBackend},
        },
        session::{Event as SessionEvent, Session, libseat::LibSeatSession},
        udev::{UdevBackend, UdevEvent, all_gpus, primary_gpu},
    },
    input::keyboard::LedState,
    output::{Mode, Output},
    reexports::{
        calloop::{
            Dispatcher, EventLoop, LoopHandle, channel,
            timer::{TimeoutAction, Timer},
        },
        input::Libinput,
        wayland_server::{Display, protocol::wl_surface},
    },
    wayland::{
        dmabuf::{DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
        drm_syncobj::{DrmSyncobjState, supports_syncobj_eventfd},
        image_copy_capture::DmabufConstraints,
    },
};
use tracing::{error, info, warn};

pub mod device;
mod handlers;
pub mod input_handler;
pub mod render;

type GbmGpuManager = GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>;

pub struct UdevConfig {
    pub drm_device: Option<PathBuf>,
    pub disable_gles_instancing: bool,
    pub disable_10bit_color: bool,
    pub disable_direct_scanout: bool,
}

pub struct UdevData {
    pub session: LibSeatSession,
    dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    syncobj_state: Option<DrmSyncobjState>,
    primary_gpu: DrmNode,
    gpus: GbmGpuManager,
    backends: HashMap<DrmNode, BackendData>,
    debug_flags: DebugFlags,
    keyboards: Vec<smithay::reexports::input::Device>,
    pointers: Vec<(smithay::reexports::input::Device, PointerConfig)>,
    disable_10bit_color: bool,
    disable_direct_scanout: bool,
    pub(self) wlr_gamma_control_state: WlrGammaControlState,
    pub(self) wlr_output_power_management_state: WlrOutputPowerManagementState,
    gpu_render_duration_tx: channel::Sender<render::GpuRenderDuration>,
}

impl UdevData {
    pub fn set_debug_flags(&mut self, flags: DebugFlags) {
        if self.debug_flags != flags {
            self.debug_flags = flags;

            for (_, backend) in self.backends.iter_mut() {
                for (_, surface) in backend.surfaces.iter_mut() {
                    if let Some(drm_output) = &surface.drm_output {
                        drm_output.set_debug_flags(flags);
                    }
                }
            }
        }
    }

    pub fn debug_flags(&self) -> DebugFlags {
        self.debug_flags
    }
}

impl Backend for UdevData {
    const HAS_RELATIVE_MOTION: bool = true;
    const HAS_GESTURES: bool = true;

    type RendererError = render::UdevRendererError;
    type RendererTextureId = MultiTexture;
    type Renderer<'a>
        = render::UdevRenderer<'a>
    where
        Self: 'a;

    fn backend_type(&self) -> super::BackendType {
        super::BackendType::Tty
    }

    fn seat_name(&self) -> String {
        self.session.seat()
    }

    fn reset_buffers(&mut self, output: &Output) {
        if let Some(id) = output.user_data().get::<UdevOutputId>()
            && let Some(gpu) = self.backends.get_mut(&id.device_id)
            && let Some(surface) = gpu.surfaces.get_mut(&id.crtc)
            && let Some(drm_output) = surface.drm_output.as_ref()
        {
            drm_output.reset_buffers();
        }
    }

    fn early_import(&mut self, surface: &wl_surface::WlSurface) {
        if let Err(err) = self.gpus.early_import(self.primary_gpu, surface) {
            warn!("Early buffer import failed: {}", err);
        }
    }

    fn update_led_state(&mut self, led_state: LedState) {
        for keyboard in self.keyboards.iter_mut() {
            keyboard.led_update(led_state.into());
        }
    }

    fn renderer(&mut self, node: Option<smithay::backend::drm::DrmNode>) -> anyhow::Result<Self::Renderer<'_>> {
        let node = node.as_ref().unwrap_or(&self.primary_gpu);
        Ok(self.gpus.single_renderer(node)?)
    }

    fn renderer_for_output(&mut self, output: &Output) -> anyhow::Result<Self::Renderer<'_>> {
        let surface_render_data = self.backends.values_mut().find_map(|backend_data| {
            backend_data.surfaces.values_mut().find_map(|surface| {
                if surface.output == *output
                    && let Some(drm_output) = surface.drm_output.as_ref()
                {
                    Some((surface.render_node, drm_output.format()))
                } else {
                    None
                }
            })
        });

        let renderer = if let Some((render_node, format)) = surface_render_data {
            let render_node = render_node.unwrap_or(self.primary_gpu);
            if render_node == self.primary_gpu {
                self.gpus.single_renderer(&render_node)
            } else {
                self.gpus.renderer(&self.primary_gpu, &render_node, format)
            }
        } else {
            self.gpus.single_renderer(&self.primary_gpu)
        }?;
        Ok(renderer)
    }

    fn dmabuf_constraints(&mut self, node: Option<DrmNode>) -> Option<DmabufConstraints> {
        let node = node.unwrap_or(self.primary_gpu);
        let renderer = self.gpus.single_renderer(&node).ok()?;
        let formats = Bind::<Dmabuf>::supported_formats(&renderer)?
            .iter()
            .fold(HashMap::<Fourcc, Vec<Modifier>>::new(), |mut map, fmt| {
                map.entry(fmt.code).or_default().push(fmt.modifier);
                map
            })
            .into_iter()
            .collect();
        Some(DmabufConstraints { node, formats })
    }

    fn set_output_mode(&mut self, handle: LoopHandle<'_, Xfwl4State<Self>>, output: &Output, mode: Mode) -> anyhow::Result<(bool, Mode)> {
        self.change_output_mode(handle, output, mode)
    }

    fn disable_output(&mut self, output: &Output) -> anyhow::Result<()> {
        self.disable_output_internal(output)
    }

    fn switch_vt(&mut self, num: i32) {
        use smithay::backend::session::Session;
        info!(to = num, "Trying to switch vt");
        if let Err(err) = self.session.change_vt(num) {
            error!(num, "Error switching vt: {}", err);
        }
    }
}

pub fn init(config: UdevConfig) -> anyhow::Result<(EventLoop<'static, Xfwl4State<UdevData>>, Xfwl4State<UdevData>)> {
    let event_loop = EventLoop::try_new().context("Failed to create event loop")?;
    let handle = event_loop.handle();
    let display = Display::new().context("Failed to create Wayland display")?;
    let display_handle = display.handle();

    /*
     * Initialize session
     */
    let (session, notifier) = LibSeatSession::new().context("Failed to intialize libseat session")?;
    let seat_name = session.seat();

    /*
     * Initialize the compositor
     */
    let primary_gpu = if let Some(var) = config.drm_device {
        DrmNode::from_path(var).context("Invalid DRM device path for GPU")
    } else {
        match primary_gpu(session.seat())
            .context("Failed to find primary GPU")?
            .and_then(|x| DrmNode::from_path(x).ok()?.node_with_type(NodeType::Render)?.ok())
        {
            Some(node) => Ok(node),
            None => all_gpus(session.seat())
                .context("Failed to query all GPUS")?
                .into_iter()
                .find_map(|x| DrmNode::from_path(x).ok())
                .ok_or_else(|| anyhow!("No usable GPU found")),
        }
    }?;
    info!("Using {primary_gpu} as primary GPU");

    let gpus = GpuManager::new(GbmGlesBackend::with_factory(move |display| {
        let context = EGLContext::new_with_priority(display, ContextPriority::High)?;
        let mut capabilities = unsafe { GlesRenderer::supported_capabilities(&context)? };
        if config.disable_gles_instancing {
            capabilities.retain(|capability| *capability != Capability::Instancing);
        }
        Ok(unsafe { GlesRenderer::with_capabilities(context, capabilities)? })
    }))
    .context("Failed to initialize GPU manager")?;

    let wlr_gamma_control_state =
        WlrGammaControlState::new::<Xfwl4State<UdevData>, _>(&display_handle, |client| !client.has_security_context());
    let wlr_output_power_management_state =
        WlrOutputPowerManagementState::new::<Xfwl4State<UdevData>, _>(&display_handle, |client| !client.has_security_context());

    let (gpu_render_duration_tx, gpu_render_duration_rx) = channel::channel();

    let data = UdevData {
        dmabuf_state: None,
        syncobj_state: None,
        session,
        primary_gpu,
        gpus,
        backends: HashMap::new(),
        debug_flags: DebugFlags::empty(),
        keyboards: Vec::new(),
        pointers: Vec::new(),
        disable_10bit_color: config.disable_10bit_color,
        disable_direct_scanout: config.disable_direct_scanout,
        wlr_gamma_control_state,
        wlr_output_power_management_state,
        gpu_render_duration_tx,
    };
    let mut state = Xfwl4State::init(display, event_loop.handle(), event_loop.get_signal(), data, true);

    /*
     * Initialize the udev backend.
     *
     * We wrap it in a Dispatcher so we can access device_list() from the
     * ActivateSession handler (needed for the first-boot path when the session
     * was not yet active at init time).
     */
    let udev_backend = UdevBackend::new(&seat_name).context("Failed to intialize udev backend")?;
    let handle_for_udev = handle.clone();
    let udev_dispatcher = Dispatcher::new(udev_backend, move |event, _, state: &mut Xfwl4State<UdevData>| match event {
        UdevEvent::Added { device_id, path } => {
            if !state.backend.session.is_active() {
                return;
            }
            if let Err(err) = DrmNode::from_dev_id(device_id)
                .map_err(DeviceAddError::DrmNode)
                .and_then(|node| state.device_added(handle_for_udev.clone(), node, &path))
            {
                error!("Skipping device {device_id}: {err}");
            }
        }
        UdevEvent::Changed { device_id } => {
            if !state.backend.session.is_active() {
                return;
            }
            if let Ok(node) = DrmNode::from_dev_id(device_id) {
                state.device_changed(node)
            }
        }
        UdevEvent::Removed { device_id } => {
            if !state.backend.session.is_active() {
                return;
            }
            if let Ok(node) = DrmNode::from_dev_id(device_id) {
                state.device_removed(handle_for_udev.clone(), node)
            }
        }
    });
    let udev_dispatcher_for_activate = udev_dispatcher.clone();

    /*
     * Initialize libinput backend
     */
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(state.backend.session.clone().into());
    libinput_context
        .udev_assign_seat(&seat_name)
        .map_err(|_| anyhow!("Failed to assign libinput context to seat"))?;

    // If the session is not yet active (e.g. we are not the foreground VT),
    // suspend libinput until ActivateSession fires.
    if !state.backend.session.is_active() {
        libinput_context.suspend();
    }

    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    /*
     * Bind all our objects that get driven by the event loop
     */
    event_loop
        .handle()
        .insert_source(libinput_backend, move |event, _, state| {
            if let Some(input) = state.backend.translate_input_event(event) {
                let key_action = state.dispatch_translated_input(input);
                if !matches!(key_action, KeyAction::None) {
                    tracing::warn!("Unhandled key action {key_action:?} returned to backend");
                }
            }
        })
        .map_err(|err| anyhow!("Failed to register libinput event source: {err}"))?;

    let display_handle_for_activate = display_handle.clone();
    let handle_for_activate = handle.clone();
    event_loop
        .handle()
        .insert_source(notifier, move |event, &mut (), state| match event {
            SessionEvent::PauseSession => {
                libinput_context.suspend();
                info!("pausing session");

                for backend in state.backend.backends.values_mut() {
                    backend.drm_output_manager.pause();
                    backend.active_leases.clear();
                    if let Some(lease_global) = backend.leasing_global.as_mut() {
                        lease_global.suspend();
                    }
                }
            }
            SessionEvent::ActivateSession => {
                info!("resuming session");

                if let Err(err) = libinput_context.resume() {
                    error!("Failed to resume libinput context: {:?}", err);
                }

                // First-boot path: if no backends are registered yet, the session was
                // inactive when init() ran and we deferred device enumeration to here.
                if state.backend.backends.is_empty() {
                    info!("First ActivateSession: enumerating GPU devices");

                    let primary_node = primary_gpu
                        .node_with_type(NodeType::Primary)
                        .and_then(|n| n.ok());
                    let devices: Vec<_> = udev_dispatcher_for_activate
                        .as_source_ref()
                        .device_list()
                        .map(|(id, p)| (id, p.to_owned()))
                        .collect();

                    let primary_device = devices.iter().find(|(device_id, _)| {
                        primary_node
                            .map(|n| *device_id == n.dev_id())
                            .unwrap_or(false)
                            || *device_id == primary_gpu.dev_id()
                    });

                    if let Some((device_id, path)) = primary_device {
                        match DrmNode::from_dev_id(*device_id) {
                            Err(err) => {
                                error!("Failed to get primary GPU node: {err}");
                                return;
                            }
                            Ok(node) => {
                                if let Err(err) = state.device_added(handle_for_activate.clone(), node, path) {
                                    error!("Failed to initialize primary GPU: {err}");
                                    return;
                                }
                            }
                        }
                    }

                    let primary_device_id = primary_device.map(|(id, _)| *id);
                    for (device_id, path) in &devices {
                        if Some(*device_id) == primary_device_id {
                            continue;
                        }
                        if let Err(err) = DrmNode::from_dev_id(*device_id)
                            .map_err(DeviceAddError::DrmNode)
                            .and_then(|node| state.device_added(handle_for_activate.clone(), node, path))
                        {
                            error!("Skipping device {device_id}: {err}");
                        }
                    }

                    #[cfg_attr(not(feature = "egl"), allow(unused_mut))]
                    let mut renderer = match state.backend.gpus.single_renderer(&primary_gpu) {
                        Err(err) => {
                            error!("Failed to get renderer for primary GPU on ActivateSession: {err}");
                            return;
                        }
                        Ok(r) => r,
                    };
                    state.core.update_shm_formats(renderer.shm_formats());

                    #[cfg(feature = "egl")]
                    {
                        info!(?primary_gpu, "Trying to initialize EGL Hardware Acceleration");
                        match renderer.bind_wl_display(&display_handle_for_activate) {
                            Ok(_) => info!("EGL hardware-acceleration enabled"),
                            Err(egl::Error::EglExtensionNotSupported(exts))
                                if exts.iter().all(|e| *e == "EGL_WL_bind_wayland_display") =>
                            {
                                info!("EGL hardware-acceleration not supported (safe to ignore)");
                            }
                            Err(err) => warn!(?err, "Failed to initialize EGL hardware-acceleration"),
                        }
                    }

                    let dmabuf_formats = renderer.dmabuf_formats();
                    match DmabufFeedbackBuilder::new(primary_gpu.dev_id(), dmabuf_formats).build() {
                        Err(err) => {
                            error!("Failed to build DMABUF feedback: {err}");
                            return;
                        }
                        Ok(default_feedback) => {
                            let mut dmabuf_state = DmabufState::new();
                            let global = dmabuf_state
                                .create_global_with_default_feedback::<Xfwl4State<UdevData>>(
                                    &display_handle_for_activate,
                                    &default_feedback,
                                );
                            state.backend.dmabuf_state = Some((dmabuf_state, global));
                        }
                    }

                    let gpus = &mut state.backend.gpus;
                    state.backend.backends.iter_mut().for_each(|(node, backend_data)| {
                        backend_data.surfaces.values_mut().for_each(|surface_data| {
                            if let Some(drm_output) = surface_data.drm_output.as_ref() {
                                surface_data.dmabuf_feedback =
                                    surface_data.dmabuf_feedback.take().or_else(|| {
                                        drm_output.with_compositor(|compositor| {
                                            get_surface_dmabuf_feedback(
                                                primary_gpu,
                                                surface_data.render_node,
                                                *node,
                                                gpus,
                                                compositor.surface(),
                                            )
                                        })
                                    });
                            }
                        });
                    });

                    if let Some(primary_node) = state
                        .backend
                        .primary_gpu
                        .node_with_type(NodeType::Primary)
                        .and_then(|x| x.ok())
                        && let Some(backend) = state.backend.backends.get(&primary_node)
                    {
                        let import_device = backend.drm_output_manager.device().device_fd().clone();
                        if supports_syncobj_eventfd(&import_device) {
                            let syncobj_state = DrmSyncobjState::new::<Xfwl4State<UdevData>>(
                                &display_handle_for_activate,
                                import_device,
                            );
                            state.backend.syncobj_state = Some(syncobj_state);
                        }
                    }
                } else {
                    // Regular resume: reconcile against the current device list.
                    // Devices may have been hot-plugged or removed while we were suspended.
                    let current_ids: std::collections::HashSet<u64> = udev_dispatcher_for_activate
                        .as_source_ref()
                        .device_list()
                        .map(|(id, _)| id)
                        .collect();

                    // Remove backends whose devices disappeared while suspended.
                    let stale: Vec<DrmNode> = state
                        .backend
                        .backends
                        .keys()
                        .filter(|node| !current_ids.contains(&node.dev_id()))
                        .copied()
                        .collect();
                    for node in stale {
                        info!("Device {node} disappeared during suspend; removing");
                        state.device_removed(handle_for_activate.clone(), node);
                    }

                    // Add devices that appeared while we were suspended.
                    let new_devices: Vec<_> = udev_dispatcher_for_activate
                        .as_source_ref()
                        .device_list()
                        .filter(|(id, _)| {
                            DrmNode::from_dev_id(*id)
                                .map_or(false, |n| !state.backend.backends.contains_key(&n))
                        })
                        .map(|(id, p)| (id, p.to_owned()))
                        .collect();
                    for (device_id, path) in new_devices {
                        if let Err(err) = DrmNode::from_dev_id(device_id)
                            .map_err(DeviceAddError::DrmNode)
                            .and_then(|node| state.device_added(handle_for_activate.clone(), node, &path))
                        {
                            error!("Failed to add new device {device_id} on resume: {err}");
                        }
                    }
                }
                for (node, backend) in state.backend.backends.iter_mut().map(|(handle, backend)| (*handle, backend)) {
                    backend
                        .drm_output_manager
                        .lock()
                        .activate(false)
                        .expect("failed to activate drm backend");
                    if let Some(lease_global) = backend.leasing_global.as_mut() {
                        lease_global.resume::<Xfwl4State<UdevData>>();
                    }

                    for (crtc, surface) in backend.surfaces.iter_mut() {
                        let crtc = *crtc;
                        let output = surface.output.clone();
                        let token = state.core.register_timer(Timer::immediate(), move |state| {
                            let frame_target = state.core.now();
                            udev_do_render(state, &output, node, crtc, frame_target);
                            TimeoutAction::Drop
                        });
                        surface.repaint_timeout = Some(token);
                    }
                }
            }
        })
        .map_err(|err| anyhow!("Failed to register session notifier event source: {err}"))?;

    // We try to initialize the primary node before others to make sure
    // any display only node can fall back to the primary node for rendering.
    // If the session is not yet active (background VT), skip device enumeration
    // entirely — the ActivateSession handler will do it on first wake-up.
    if state.backend.session.is_active() {
        let primary_node = primary_gpu.node_with_type(NodeType::Primary).and_then(|node| node.ok());
        let devices: Vec<_> = udev_dispatcher
            .as_source_ref()
            .device_list()
            .map(|(id, p)| (id, p.to_owned()))
            .collect();
        let primary_device = devices.iter().find(|(device_id, _)| {
            primary_node
                .map(|primary_node| *device_id == primary_node.dev_id())
                .unwrap_or(false)
                || *device_id == primary_gpu.dev_id()
        });

        if let Some((device_id, path)) = primary_device {
            let node = DrmNode::from_dev_id(*device_id).context("Failed to get primary GPU node")?;
            state
                .device_added(handle.clone(), node, path)
                .context("Failed to initialize primary GPU node")?;
        }

        let primary_device_id = primary_device.map(|(device_id, _)| *device_id);
        for (device_id, path) in &devices {
            if Some(*device_id) == primary_device_id {
                continue;
            }
            if let Err(err) = DrmNode::from_dev_id(*device_id)
                .map_err(DeviceAddError::DrmNode)
                .and_then(|node| state.device_added(handle.clone(), node, path))
            {
                error!("Skipping device {device_id}: {err}");
            }
        }

        #[cfg_attr(not(feature = "egl"), allow(unused_mut))]
        let mut renderer = state
            .backend
            .gpus
            .single_renderer(&primary_gpu)
            .or_else(|_| {
                // primary_gpu is a render-type node (e.g. renderD128); on virtio-gpu+llvmpipe
                // EGL reports no render node so we register the card-type node instead.
                // Try the corresponding card-type node (e.g. card0) as a fallback.
                primary_gpu
                    .node_with_type(NodeType::Primary)
                    .ok()
                    .flatten()
                    .and_then(|card_node| state.backend.gpus.single_renderer(&card_node).ok())
                    .ok_or_else(|| anyhow::anyhow!("no renderer available for either render or card node"))
            })
            .context("Failed to get renderer for primary GPU")?;

        state.core.update_shm_formats(renderer.shm_formats());

        #[cfg(feature = "egl")]
        {
            info!(?primary_gpu, "Trying to initialize EGL Hardware Acceleration",);
            match renderer.bind_wl_display(&display_handle) {
                Ok(_) => info!("EGL hardware-acceleration enabled"),
                Err(egl::Error::EglExtensionNotSupported(exts)) if exts.iter().all(|ext| *ext == "EGL_WL_bind_wayland_display") => {
                    info!("Failed to intialize EGL hardware-acceleration; this error is safe to ignore");
                }
                Err(err) => warn!(?err, "Failed to initialize EGL hardware-acceleration"),
            }
        }

        // init dmabuf support with format list from our primary gpu
        let dmabuf_formats = renderer.dmabuf_formats();
        let default_feedback = DmabufFeedbackBuilder::new(primary_gpu.dev_id(), dmabuf_formats)
            .build()
            .context("Failed to build default DMABUF feedback")?;
        let mut dmabuf_state = DmabufState::new();
        let global = dmabuf_state.create_global_with_default_feedback::<Xfwl4State<UdevData>>(&display_handle, &default_feedback);
        state.backend.dmabuf_state = Some((dmabuf_state, global));

        let gpus = &mut state.backend.gpus;
        state.backend.backends.iter_mut().for_each(|(node, backend_data)| {
            // Update the per drm surface dmabuf feedback
            backend_data.surfaces.values_mut().for_each(|surface_data| {
                if let Some(drm_output) = surface_data.drm_output.as_ref() {
                    surface_data.dmabuf_feedback = surface_data.dmabuf_feedback.take().or_else(|| {
                        drm_output.with_compositor(|compositor| {
                            get_surface_dmabuf_feedback(primary_gpu, surface_data.render_node, *node, gpus, compositor.surface())
                        })
                    });
                }
            });
        });

        // Expose syncobj protocol if supported by primary GPU
        if let Some(primary_node) = state.backend.primary_gpu.node_with_type(NodeType::Primary).and_then(|x| x.ok())
            && let Some(backend) = state.backend.backends.get(&primary_node)
        {
            let import_device = backend.drm_output_manager.device().device_fd().clone();
            if supports_syncobj_eventfd(&import_device) {
                let syncobj_state = DrmSyncobjState::new::<Xfwl4State<UdevData>>(&display_handle, import_device);
                state.backend.syncobj_state = Some(syncobj_state);
            }
        }
    } else {
        info!("Session not yet active at init; deferring GPU device enumeration to ActivateSession");
    }

    event_loop
        .handle()
        .insert_source(gpu_render_duration_rx, |event, _, state| {
            if let channel::Event::Msg(msg) = event
                && let Some(device) = state.backend.backends.get_mut(&msg.node)
                && let Some(surface) = device.surfaces.get_mut(&msg.crtc)
            {
                surface.render_durations.push_back(msg.duration);
                if surface.render_durations.len() > render::RENDER_DURATIONS_SLIDING_WINDOW_MAX {
                    let _ = surface.render_durations.pop_front();
                }
            }
        })
        .map_err(|err| anyhow!("Failed to register GPU render duration channel: {err}"))?;

    event_loop
        .handle()
        .register_dispatcher(udev_dispatcher)
        .map_err(|err| anyhow!("Failed to register udev event source: {err}"))?;

    Ok((event_loop, state))
}

//pub type RenderSurface = GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, Option<OutputPresentationFeedback>>;

//pub type GbmDrmCompositor =
//    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmDevice<DrmDeviceFd>, Option<OutputPresentationFeedback>, DrmDeviceFd>;
