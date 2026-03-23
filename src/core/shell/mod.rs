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

use std::{cell::RefCell, sync::Mutex, time::Duration};

#[cfg(feature = "xwayland")]
use smithay::desktop::WindowSurface;
#[cfg(feature = "udev")]
use smithay::wayland::drm_syncobj::DrmSyncobjCachedState;

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    desktop::{LayerSurface, PopupKind, WindowSurfaceType, layer_map_for_output},
    input::pointer::{CursorImageStatus, CursorImageSurfaceData},
    output::{Output, WeakOutput},
    reexports::{
        calloop::{
            Interest, RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        wayland_server::{
            Client, Resource,
            protocol::{wl_buffer::WlBuffer, wl_output, wl_surface::WlSurface},
        },
    },
    utils::{IsAlive, Logical, Monotonic, Rectangle, Time},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            BufferAssignment, CompositorClientState, CompositorHandler, CompositorState, SurfaceAttributes, TraversalAction, add_blocker,
            add_pre_commit_hook, get_parent, is_sync_subsurface, with_states, with_surface_tree_upward,
        },
        dmabuf::get_dmabuf,
        idle_inhibit::IdleInhibitHandler,
        shell::{
            wlr_layer::{Layer, LayerSurface as WlrLayerSurface, LayerSurfaceData, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{PopupSurface, XdgShellState, XdgToplevelSurfaceData, dialog::XdgDialogState},
        },
    },
};

use crate::{
    backend::Backend,
    core::state::{ClientState, Xfwl4State},
    util::icon::IconSource,
};

mod element;
mod element_impls;
mod grabs;
mod layout;
pub(crate) mod ssd;
#[cfg(feature = "xwayland")]
mod x11;
pub(crate) mod xdg;

pub use self::element::*;
pub use self::grabs::*;
pub use self::layout::*;

const MAX_URGENT_BLINK_ITERATIONS: u32 = 10;
const URGENT_BLINK_TIMEOUT: Duration = Duration::from_millis(500);

pub struct ShellProtocolDelegates {
    compositor_state: CompositorState,
    layer_shell_state: WlrLayerShellState,
    _xdg_dialog_state: XdgDialogState,
    xdg_shell_state: XdgShellState,
    #[cfg(feature = "xwayland")]
    xwayland_shell_state: smithay::wayland::xwayland_shell::XWaylandShellState,
}

impl ShellProtocolDelegates {
    pub fn new(
        compositor_state: CompositorState,
        layer_shell_state: WlrLayerShellState,
        xdg_dialog_state: XdgDialogState,
        xdg_shell_state: XdgShellState,
        #[cfg(feature = "xwayland")] xwayland_shell_state: smithay::wayland::xwayland_shell::XWaylandShellState,
    ) -> Self {
        Self {
            compositor_state,
            layer_shell_state,
            _xdg_dialog_state: xdg_dialog_state,
            xdg_shell_state,
            #[cfg(feature = "xwayland")]
            xwayland_shell_state,
        }
    }

    #[inline]
    pub(super) fn layer_surfaces(&self) -> impl DoubleEndedIterator<Item = smithay::wayland::shell::wlr_layer::LayerSurface> {
        self.layer_shell_state.layer_surfaces()
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct WindowState: u8 {
        const ACTIVATED = (1 << 0);
        const MINIMIZED = (1 << 1);
        const MAXIMIZED = (1 << 2);
        const SHADED = (1 << 3);
        const STICKY = (1 << 4);
        const FULLSCREEN = (1 << 5);
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct WindowFlags: u8 {
        const NO_CYCLE = (1 << 0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkspaceLocation {
    Single(u32),
    All,
}

impl Default for WorkspaceLocation {
    fn default() -> Self {
        Self::Single(0)
    }
}

#[derive(Debug)]
pub struct UrgentNotificationState {
    pub token: RegistrationToken,
    pub iterations: u32,
}

#[derive(Debug, Default)]
pub struct WindowPropsInner {
    pub flags: WindowFlags,
    pub saved_geom: Option<Rectangle<i32, Logical>>,
    pub anchored_output: Option<WeakOutput>,
    pub tile_mode: Option<TileMode>,
    pub workspace_loc: WorkspaceLocation,
    pub is_shaded: bool,
    pub is_opacity_locked: bool,
    pub hide_titlebar_when_maximized: bool,
    pub toplevel_icon_state_hash: u64,
    pub window_icon: IconSource,
    pub urgent: Option<UrgentNotificationState>,
    pub last_user_interaction: Option<Time<Monotonic>>,
}

#[derive(Debug, Default)]
pub struct WindowProps(pub Mutex<WindowPropsInner>);

impl<BackendData: Backend> BufferHandler for Xfwl4State<BackendData> {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl<BackendData: Backend> CompositorHandler for Xfwl4State<BackendData> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.core.shell_protocol_delegates.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        #[cfg(feature = "xwayland")]
        if let Some(state) = client.get_data::<smithay::xwayland::XWaylandClientData>() {
            return &state.compositor_state;
        }

        if let Some(state) = client.get_data::<ClientState>() {
            &state.compositor_state
        } else {
            panic!("Unknown client data type");
        }
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        add_pre_commit_hook::<Self, _>(surface, move |state, _dh, surface| {
            #[cfg(feature = "udev")]
            let mut acquire_point = None;
            let maybe_dmabuf = with_states(surface, |surface_data| {
                #[cfg(feature = "udev")]
                acquire_point.clone_from(&surface_data.cached_state.get::<DrmSyncobjCachedState>().pending().acquire_point);
                surface_data
                    .cached_state
                    .get::<SurfaceAttributes>()
                    .pending()
                    .buffer
                    .as_ref()
                    .and_then(|assignment| match assignment {
                        BufferAssignment::NewBuffer(buffer) => get_dmabuf(buffer).cloned().ok(),
                        _ => None,
                    })
            });
            if let Some(dmabuf) = maybe_dmabuf {
                #[cfg(feature = "udev")]
                if let Some(acquire_point) = acquire_point
                    && let Ok((blocker, source)) = acquire_point.generate_blocker()
                {
                    let client = surface.client().unwrap();
                    let res = state.core.handle.insert_source(source, move |_, _, data| {
                        let dh = data.core.display_handle.clone();
                        data.client_compositor_state(&client).blocker_cleared(data, &dh);
                        Ok(())
                    });
                    if res.is_ok() {
                        add_blocker(surface, blocker);
                        return;
                    }
                }
                if let Ok((blocker, source)) = dmabuf.generate_blocker(Interest::READ)
                    && let Some(client) = surface.client()
                {
                    let res = state.core.handle.insert_source(source, move |_, _, data| {
                        let dh = data.core.display_handle.clone();
                        data.client_compositor_state(&client).blocker_cleared(data, &dh);
                        Ok(())
                    });
                    if res.is_ok() {
                        add_blocker(surface, blocker);
                    }
                }
            }
        });
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        self.backend.early_import(surface);

        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.window_for_surface(&root) {
                window.0.on_commit();

                if &root == surface {
                    let buffer_offset = with_states(surface, |states| {
                        states.cached_state.get::<SurfaceAttributes>().current().buffer_delta.take()
                    });

                    if let Some(buffer_offset) = buffer_offset {
                        let workspace = self.core.workspace_manager.active_workspace_mut();
                        let current_loc = workspace.window_location(&window).unwrap();
                        self.core
                            .workspace_manager
                            .relocate_window(&window, current_loc + buffer_offset, false);
                    }
                }
            }
        }
        self.core.popups.commit(surface);

        if matches!(&self.core.pointer_element.status(), CursorImageStatus::Surface(cursor_surface) if cursor_surface == surface) {
            with_states(surface, |states| {
                let cursor_image_attributes = states.data_map.get::<CursorImageSurfaceData>();

                if let Some(mut cursor_image_attributes) = cursor_image_attributes.map(|attrs| attrs.lock().unwrap()) {
                    let buffer_delta = states.cached_state.get::<SurfaceAttributes>().current().buffer_delta.take();
                    if let Some(buffer_delta) = buffer_delta {
                        tracing::trace!(hotspot = ?cursor_image_attributes.hotspot, ?buffer_delta, "decrementing cursor hotspot");
                        cursor_image_attributes.hotspot -= buffer_delta;
                    }
                }
            });
        }

        if matches!(&self.core.dnd_icon, Some(icon) if &icon.surface == surface) {
            let dnd_icon = self.core.dnd_icon.as_mut().unwrap();
            with_states(&dnd_icon.surface, |states| {
                let buffer_delta = states
                    .cached_state
                    .get::<SurfaceAttributes>()
                    .current()
                    .buffer_delta
                    .take()
                    .unwrap_or_default();
                tracing::trace!(offset = ?dnd_icon.offset, ?buffer_delta, "moving dnd offset");
                dnd_icon.offset += buffer_delta;
            });
        }

        self.ensure_initial_configure(surface)
    }

    fn destroyed(&mut self, surface: &WlSurface) {
        self.uninhibit(surface.clone());
        self.core.pending_windows.retain(|a_surface, _| surface != a_surface);

        if let Some(window) = self.window_for_surface(surface) {
            self.remove_window(&window);
            self.core.toplevel_destroyed(&window);
        }
    }
}

impl<BackendData: Backend> WlrLayerShellHandler for Xfwl4State<BackendData> {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.core.shell_protocol_delegates.layer_shell_state
    }

    fn new_layer_surface(&mut self, surface: WlrLayerSurface, wl_output: Option<wl_output::WlOutput>, _layer: Layer, namespace: String) {
        let output = wl_output
            .as_ref()
            .and_then(Output::from_resource)
            .unwrap_or_else(|| self.core.workspace_manager.outputs().next().unwrap().clone());
        let mut map = layer_map_for_output(&output);
        map.map_layer(&LayerSurface::new(surface, namespace)).unwrap();
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        let output = self
            .core
            .workspace_manager
            .outputs()
            .find(|o| layer_map_for_output(o).layers().any(|layer| layer.layer_surface() == &surface))
            .cloned();

        if let Some(output) = output {
            let mut map = layer_map_for_output(&output);
            let layer = map.layers().find(|&layer| layer.layer_surface() == &surface).cloned();
            if let Some(layer) = layer {
                map.unmap_layer(&layer);
            }
            drop(map);

            self.reapply_anchored_layouts_on_output(&output);

            #[cfg(feature = "xwayland")]
            self.x11_update_workarea();
        }
    }

    fn new_popup(&mut self, _parent: WlrLayerSurface, popup: PopupSurface) {
        self.unconstrain_popup(&popup);

        if let Err(err) = popup.send_configure() {
            tracing::warn!("Failed to send configure event to popup with layer-shell parent: {err}");
        } else if let Err(err) = self.core.popups.track_popup(PopupKind::from(popup)) {
            tracing::warn!("Failed to track popup with layer-shell parent: {err}");
        }
    }
}

#[derive(Default)]
pub struct SurfaceData {
    pub geometry: Option<Rectangle<i32, Logical>>,
    pub resize_state: ResizeState,
}

impl<BackendData: Backend> Xfwl4State<BackendData> {
    pub(in crate::core) fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.core
            .workspace_manager
            .find_window(|window| window.wl_surface().map(|s| &*s == surface).unwrap_or(false))
            .or_else(|| self.core.pending_windows.get(surface).cloned())
    }

    fn ensure_initial_configure(&mut self, surface: &WlSurface) {
        with_surface_tree_upward(
            surface,
            (),
            |_, _, _| TraversalAction::DoChildren(()),
            |_, states, _| {
                states.data_map.insert_if_missing(|| RefCell::new(SurfaceData::default()));
            },
            |_, _, _| true,
        );

        if let Some(window) = self.window_for_surface(surface) {
            // send the initial configure if relevant
            #[cfg_attr(not(feature = "xwayland"), allow(irrefutable_let_patterns))]
            if let Some(toplevel) = window.0.toplevel() {
                let initial_configure_sent = with_states(surface, |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .initial_configure_sent
                });
                if !initial_configure_sent {
                    toplevel.send_configure();
                }
            }

            #[cfg(feature = "xwayland")]
            if window.is_x11() {
                // For wayland windows, the post-commit hook will handler transitioning out of
                // resizing and into NotResizing, but X11 works differently because the X protocol
                // supports an atomic resize+move operation.
                with_states(surface, |states| {
                    let mut data = states.data_map.get::<RefCell<SurfaceData>>().unwrap().borrow_mut();
                    if let ResizeState::WaitingForCommit(_) = data.resize_state {
                        data.resize_state = ResizeState::NotResizing;
                    }
                });
            }

            return;
        }

        if let Some(popup) = self.core.popups.find_popup(surface) {
            let popup = match popup {
                PopupKind::Xdg(ref popup) => popup,
                // Doesn't require configure
                PopupKind::InputMethod(ref _input_popup) => {
                    return;
                }
            };

            if !popup.is_initial_configure_sent() {
                // NOTE: This should never fail as the initial configure is always
                // allowed.
                popup.send_configure().expect("initial configure failed");
            }

            return;
        };

        let output = self
            .core
            .workspace_manager
            .outputs()
            .find(|o| {
                let map = layer_map_for_output(o);
                map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL).is_some()
            })
            .cloned();

        if let Some(output) = output {
            let initial_configure_sent = with_states(surface, |states| {
                states
                    .data_map
                    .get::<LayerSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });

            let mut map = layer_map_for_output(&output);

            // arrange the layers before sending the initial configure
            // to respect any size the client may have sent
            let layout_changed = map.arrange();

            // send the initial configure if relevant
            if !initial_configure_sent {
                let layer = map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL).unwrap();

                layer.layer_surface().send_configure();
            }
            drop(map);

            if layout_changed {
                #[cfg(feature = "xwayland")]
                self.x11_update_workarea();
                self.reapply_anchored_layouts_on_output(&output);
            }
        };
    }

    pub fn set_window_urgent_state(&mut self, window: &WindowElement, is_urgent: bool) {
        let mut props = window.props();
        if is_urgent != props.urgent.is_some() {
            if let Some(urgent_state) = props.urgent.take() {
                self.core.handle.remove(urgent_state.token);

                if let Some(decorations) = window.decoration_state_mut().window_decorations_mut() {
                    decorations.disable_titlebar_blink();
                }
            } else if self.core.config.urgent_blink() && !window.active() {
                let window = window.clone();

                let token = self
                    .core
                    .handle
                    .insert_source(Timer::from_duration(URGENT_BLINK_TIMEOUT), move |_, _, state| {
                        let mut props = window.props();
                        if window.alive()
                            && let Some(mut urgent_state) = props.urgent.take()
                            && (urgent_state.iterations < MAX_URGENT_BLINK_ITERATIONS || state.core.config.repeat_urgent_blink())
                        {
                            if urgent_state.iterations < MAX_URGENT_BLINK_ITERATIONS {
                                urgent_state.iterations += 1;
                            } else {
                                urgent_state.iterations = 0;
                            }
                            props.urgent = Some(urgent_state);

                            if let Some(decorations) = window.decoration_state_mut().window_decorations_mut() {
                                decorations.toggle_titlebar_blink_state();
                            }

                            TimeoutAction::ToDuration(URGENT_BLINK_TIMEOUT)
                        } else {
                            if let Some(decorations) = window.decoration_state_mut().window_decorations_mut() {
                                decorations.disable_titlebar_blink();
                            }
                            TimeoutAction::Drop
                        }
                    })
                    .expect("Failed to register urgent blink timeout with event loop");

                let urgent_state = UrgentNotificationState { token, iterations: 0 };
                props.urgent = Some(urgent_state);
            }

            #[cfg(feature = "xwayland")]
            if let WindowSurface::X11(x11_surface) = window.0.underlying_surface() {
                let _ = x11_surface.set_demands_attention(props.urgent.is_some());
            }
        }
    }
}
