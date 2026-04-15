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
    hash::{DefaultHasher, Hash, Hasher},
    sync::Mutex,
};

use gtk::gio::{self, traits::AppInfoExt};
use smithay::{
    desktop::{
        PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurface, WindowSurfaceType,
        find_popup_root_surface, get_popup_toplevel_coords, layer_map_for_output,
        space::{RenderZindex, SpaceElement},
    },
    input::{
        Seat,
        pointer::{CursorIcon, Focus, MotionEvent},
    },
    output::Output,
    reexports::{
        wayland_protocols::xdg::{decoration as xdg_decoration, shell::server::xdg_toplevel},
        wayland_server::{
            Resource,
            protocol::{wl_output, wl_seat, wl_surface::WlSurface},
        },
    },
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Serial, Size},
    wayland::{
        compositor::{self, with_states},
        shell::xdg::{
            Configure, PopupSurface, PositionerState, SurfaceCachedState, ToplevelCachedState, ToplevelSurface, XdgShellHandler,
            XdgShellState, XdgToplevelSurfaceData, dialog::XdgDialogHandler,
        },
        xdg_toplevel_icon::ToplevelIconCachedState,
    },
};
use tracing::warn;

use crate::{
    backend::Backend,
    core::{
        focus::KeyboardFocusTarget,
        handlers::xfwl4_compositor_ui::ActionLocation,
        placement::StackResult,
        shell::{GrabTrigger, WindowFlags, WindowIcon, WindowState, XdgToplevelIconState},
        state::Xfwl4State,
        util::{prettify_name, shm_buffer_to_image_data},
    },
    ui::window_menu::WINDOW_MENU_TOPLEVEL_TITLE,
};

use super::{ResizeEdge, ResizeState, SurfaceData, WindowElement};

#[derive(Debug, Default)]
pub struct XdgSurfacePropsInner {
    pub is_minimized: bool,
}

#[derive(Debug, Default)]
pub struct XdgSurfaceProps(pub Mutex<XdgSurfacePropsInner>);

impl<BackendData: Backend> XdgShellHandler for Xfwl4State<BackendData> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.core.shell_protocol_delegates.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Do not send a configure here, the initial configure
        // of a xdg_surface has to be sent during the commit if
        // the surface is not already configured

        // Set the initial toplevel bounds so the client knows what size to use
        let pointer_location = self.core.pointer.current_location();
        let output = self
            .core
            .workspace_manager
            .output_under(pointer_location)
            .next()
            .or_else(|| self.core.workspace_manager.outputs().next())
            .cloned();
        let output_geometry = output
            .and_then(|o| {
                let geo = self.core.workspace_manager.output_geometry(&o)?;
                let map = layer_map_for_output(&o);
                let zone = map.non_exclusive_zone();
                Some(Rectangle::new(geo.loc + zone.loc, zone.size))
            })
            .unwrap_or_else(|| Rectangle::from_size((800, 800).into()));
        surface.with_pending_state(|state| {
            state.bounds = Some(output_geometry.size);
        });

        let window = WindowElement::new(
            Window::new_wayland_window(surface.clone()),
            self.core.next_window_id(),
            &self.core.config,
        );
        self.core.pending_windows.insert(surface.wl_surface().clone(), window);

        compositor::add_post_commit_hook(surface.wl_surface(), |state: &mut Self, _, surface| {
            state.handle_toplevel_commit(surface);
        });
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        // Do not send a configure here, the initial configure
        // of a xdg_surface has to be sent during the commit if
        // the surface is not already configured

        self.unconstrain_popup(&surface);

        if let Err(err) = self.core.popups.track_popup(PopupKind::from(surface)) {
            warn!("Failed to track popup: {}", err);
        }
    }

    fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, token: u32) {
        surface.with_pending_state(|state| {
            let geometry = positioner.get_geometry();
            state.geometry = geometry;
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn parent_changed(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.window_for_surface(surface.wl_surface()) {
            let parent = compositor::with_states(surface.wl_surface(), |states| {
                states.data_map.get::<XdgToplevelSurfaceData>().and_then(|data| {
                    data.lock()
                        .unwrap()
                        .parent
                        .as_ref()
                        .and_then(|wl_surface| self.window_for_surface(wl_surface))
                })
            });
            self.set_window_parent(&window, parent);
        }
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        if let Some(window) = self.window_for_surface(surface.wl_surface()) {
            let seat: Seat<Xfwl4State<BackendData>> = Seat::from_resource(&seat).unwrap();
            self.start_window_move(window, seat, serial, GrabTrigger::Pointer);
        }
    }

    fn resize_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial, edges: xdg_toplevel::ResizeEdge) {
        if let Some(window) = self.window_for_surface(surface.wl_surface()) {
            let seat: Seat<Xfwl4State<BackendData>> = Seat::from_resource(&seat).unwrap();
            self.start_window_resize(window, seat, serial, edges.into(), GrabTrigger::Pointer);
        }
    }

    fn ack_configure(&mut self, surface: WlSurface, configure: Configure) {
        if let Configure::Toplevel(configure) = configure
            && let Some(window) = self.window_for_surface(&surface)
        {
            use xdg_decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
            let is_ssd = configure
                .state
                .decoration_mode
                .map(|mode| !configure.state.states.contains(xdg_toplevel::State::Fullscreen) && mode == Mode::ServerSide)
                .unwrap_or(false);
            if is_ssd && !window.decoration_state().has_decorations() {
                self.enable_decorations_for_window(&window);
            } else if !is_ssd && window.decoration_state().has_decorations() {
                self.disable_decorations_for_window(&window);
            }

            let update_window_icon = with_states(&surface, |states| {
                let mut icon_state = states.cached_state.get::<ToplevelIconCachedState>();
                let current = icon_state.current();

                let new_icon_state_hash = {
                    let mut hasher = DefaultHasher::new();
                    current.icon_name().hash(&mut hasher);
                    for (buffer, scale) in current.buffers() {
                        buffer.hash(&mut hasher);
                        scale.hash(&mut hasher);
                    }
                    hasher.finish()
                };

                let mut props = window.props();
                if new_icon_state_hash != props.toplevel_icon_state_hash {
                    let rasters = current
                        .buffers()
                        .iter()
                        .flat_map(|(buffer, scale)| shm_buffer_to_image_data(buffer, (*scale).max(1) as u32).ok())
                        .collect::<Vec<_>>();

                    props.toplevel_icon_state_hash = new_icon_state_hash;
                    props.window_icon.update_name(current.icon_name().map(ToOwned::to_owned));
                    props.window_icon.update_rasters(rasters);
                    true
                } else {
                    false
                }
            });
            if update_window_icon && let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                let depends_on_theme = window.props().window_icon.depends_on_theme();
                window_decorations.update(DecorationInput::IconChanged { depends_on_theme });
            }
            // TODO: notify foreign-toplevel of icon change
        }
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, wl_output: Option<wl_output::WlOutput>) {
        if let Some(window) = self
            .core
            .workspace_manager
            .active_workspace()
            .window_for_surface(surface.wl_surface())
        {
            self.set_window_fullscreen(&window, wl_output.as_ref().and_then(Output::from_resource));
        }
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.window_for_surface(surface.wl_surface()) {
            self.set_window_unfullscreen(&window);
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let workspace = self.core.workspace_manager.active_workspace_mut();
        if let Some(window) = workspace.window_for_surface(surface.wl_surface()) {
            self.set_window_maximized(&window, None);
        }
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let workspace = self.core.workspace_manager.active_workspace_mut();
        if let Some(window) = workspace.window_for_surface(surface.wl_surface()) {
            self.set_window_unmaximized(&window, None);
        }
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let seat: Seat<Xfwl4State<BackendData>> = Seat::from_resource(&seat).unwrap();
        let kind = PopupKind::Xdg(surface);
        if let Some(root) = find_popup_root_surface(&kind).ok().and_then(|root| {
            if let Some(window_menu_anchor) = self.core.window_menu_anchor.as_ref()
                && window_menu_anchor.wl_surface().is_some_and(|surf| surf.as_ref() == &root)
            {
                Some(KeyboardFocusTarget::from(window_menu_anchor.clone()))
            } else {
                let workspace = self.core.workspace_manager.active_workspace();

                workspace.window_for_surface(&root).map(KeyboardFocusTarget::from).or_else(|| {
                    self.core
                        .workspace_manager
                        .outputs()
                        .find_map(|o| {
                            let map = layer_map_for_output(o);
                            map.layer_for_surface(&root, WindowSurfaceType::TOPLEVEL).cloned()
                        })
                        .map(KeyboardFocusTarget::LayerSurface)
                })
            }
        }) {
            let ret = self.core.popups.grab_popup(root, kind, &seat, serial);

            if let Ok(mut grab) = ret {
                if let Some(keyboard) = seat.get_keyboard() {
                    if keyboard.is_grabbed() && !(keyboard.has_grab(serial) || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    if let Some(current_grab) = grab.current_grab() {
                        self.focus_target(current_grab, serial, None);
                    } else {
                        self.unset_focus(serial, None);
                    }
                    keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = seat.get_pointer() {
                    if pointer.is_grabbed()
                        && !(pointer.has_grab(serial) || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
        }
    }

    fn show_window_menu(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial, location: Point<i32, Logical>) {
        if let Some(window) = self
            .core
            .workspace_manager
            .active_workspace()
            .find_window(|e| e.0.toplevel() == Some(&surface))
            && let Some(seat) = Seat::<Self>::from_resource(&seat)
        {
            self.pop_up_window_menu(&window, &seat, serial, ActionLocation::WindowRelative(location));
        }
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        compositor::with_states(surface.wl_surface(), |states| {
            if let Some(data) = states.data_map.get::<XdgToplevelSurfaceData>()
                && let Some(elem) = self.window_for_toplevel_surface(&surface)
            {
                let data = data.lock().unwrap();

                if let Some(window_decorations) = elem.decoration_state_mut().window_decorations_mut() {
                    window_decorations.update(DecorationInput::Title(Some(data.title.clone().unwrap_or_default())));
                }

                self.core.toplevel_changed(
                    &elem,
                    data.title.as_deref(),
                    None,
                    WindowState::empty(),
                    WindowState::empty(),
                    Vec::new(),
                    Vec::new(),
                    None,
                );
            }
        });
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        if let Some(elem) = self.window_for_toplevel_surface(&surface) {
            let app_id = compositor::with_states(surface.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .and_then(|data| data.lock().unwrap().app_id.clone())
            });

            let mut props = elem.props();

            if props.window_icon.update_app_id(app_id.clone())
                && let Some(window_decorations) = elem.decoration_state_mut().window_decorations_mut()
            {
                let depends_on_theme = props.window_icon.depends_on_theme();
                window_decorations.update(DecorationInput::IconChanged { depends_on_theme });
            }

            // TODO: signal window icon update too
            self.core.toplevel_changed(
                &elem,
                None,
                app_id.as_deref(),
                WindowState::empty(),
                WindowState::empty(),
                Vec::new(),
                Vec::new(),
                None,
            );
        }
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        if let Some(elem) = self.window_for_toplevel_surface(&surface) {
            self.set_window_minimized(&elem);
        }
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        if let Some(window) = self.window_for_surface(surface.wl_surface()) {
            if self.window_is_tabwin(&window, surface.wl_surface()) {
                self.core.compositor_ui_state.tabwin_closed();
                self.end_window_cycling();
            }
            window.handle_destroyed();
            self.remove_window(&window);
            self.core.toplevel_destroyed(&window);
        }
    }

    fn popup_destroyed(&mut self, surface: PopupSurface) {
        if let Some(parent) = surface.get_parent_surface()
            && let Some(anchor) = self.core.window_menu_anchor.as_ref()
            && anchor.wl_surface().as_deref() == Some(&parent)
        {
            self.core.pending_window_menu_state = None;
        }
    }
}

impl<BackendData: Backend> XdgDialogHandler for Xfwl4State<BackendData> {}

impl<BackendData: Backend> Xfwl4State<BackendData> {
    pub(super) fn unconstrain_popup(&self, popup: &PopupSurface) {
        let workspace = self.core.workspace_manager.active_workspace();

        // The popup's `state.geometry.loc` is relative to its parent surface's `window_geometry`
        // rect (see `xdg_popup.configure`). To constrain in those coords, compute the screen
        // position of that rect's upper-left in global (space) coordinates: for an xdg_toplevel
        // parent that's `mapped_loc + ssd_offset + window_geometry.loc` (xfwl4 maps windows at the
        // SSD top-left); for a layer-shell parent it's `layer_geometry.loc + output.loc`
        // (`layer_geometry` is output-local, so the output's global origin must be added).
        if let Some((mut outputs_for_window, parent_geometry_origin)) =
            find_popup_root_surface(&PopupKind::Xdg(popup.clone())).ok().and_then(|root| {
                workspace
                    .window_for_surface(&root)
                    .and_then(|window| {
                        let outputs = workspace.outputs_for_window(&window);
                        if !outputs.is_empty()
                            && let Some(geom) = workspace.window_geometry(&window)
                        {
                            let decorations_offset = window
                                .decoration_state()
                                .window_decorations()
                                .map(|d| d.decorations_offset())
                                .unwrap_or_default();
                            let window_geometry_loc = compositor::with_states(&root, |states| {
                                states
                                    .cached_state
                                    .get::<SurfaceCachedState>()
                                    .current()
                                    .geometry
                                    .map(|g| g.loc)
                                    .unwrap_or_default()
                            });
                            Some((outputs, geom.loc + decorations_offset + window_geometry_loc))
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        self.core.workspace_manager.outputs().find_map(|output| {
                            let output_loc = self
                                .core
                                .workspace_manager
                                .output_geometry(output)
                                .map(|geom| geom.loc)
                                .unwrap_or_default();
                            let layer_map = layer_map_for_output(output);
                            layer_map
                                .layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                                .and_then(|layer_surface| layer_map.layer_geometry(layer_surface))
                                .map(|geom| (vec![output.clone()], geom.loc + output_loc))
                        })
                    })
            })
        {
            // Get a union of all outputs' geometries, minus any exclusive zones set by layer-shell
            // surfaces.
            let first = outputs_for_window.pop().unwrap();
            let first_zone = layer_map_for_output(&first).non_exclusive_zone();
            let mut outputs_geo = self
                .core
                .workspace_manager
                .output_geometry(&first)
                .map(|geom| {
                    let zone = Rectangle::new(geom.loc + first_zone.loc, first_zone.size);
                    geom.intersection(zone).unwrap_or(geom)
                })
                .unwrap_or(first_zone);
            for output in outputs_for_window {
                let zone = layer_map_for_output(&output).non_exclusive_zone();
                let geom = self
                    .core
                    .workspace_manager
                    .output_geometry(&output)
                    .map(|geom| {
                        let zone = Rectangle::new(geom.loc + zone.loc, zone.size);
                        geom.intersection(zone).unwrap_or(geom)
                    })
                    .unwrap_or(zone);
                outputs_geo = outputs_geo.merge(geom);
            }

            // The target geometry for the positioner should be relative to its parent's geometry, so
            // we will compute that here.
            let mut target = outputs_geo;
            target.loc -= get_popup_toplevel_coords(&PopupKind::Xdg(popup.clone()));
            target.loc -= parent_geometry_origin;

            popup.with_pending_state(|state| {
                state.geometry = state.positioner.get_unconstrained_geometry(target);
            });
        }
    }

    /// Should be called on `WlSurface::commit` of xdg toplevel
    fn handle_toplevel_commit(&mut self, surface: &WlSurface) -> Option<()> {
        if let Some(window) = self.core.pending_windows.get(surface) {
            if self.handle_new_window_placement(window.clone(), surface) {
                self.core.pending_windows.remove(surface);
            }
        } else {
            let window = self
                .core
                .workspace_manager
                .active_workspace()
                .visible_windows()
                .find(|w: &&WindowElement| w.wl_surface().as_deref() == Some(surface))
                .cloned()?;

            if self.window_is_tabwin(&window, surface) {
                if let Some(size) = self.find_window_content_size(&window) {
                    self.place_tabwin(&window, size);
                } else if let Some(toplevel_surface) = window.0.toplevel() {
                    toplevel_surface.send_configure();
                }
            } else {
                let space = self.core.workspace_manager.active_workspace_mut();
                let mut window_loc = space.window_location(&window)?;
                let inner_geometry = SpaceElement::geometry(&window.0);
                let decorations_offset = window
                    .decoration_state()
                    .window_decorations()
                    .map(|d| d.decorations_offset())
                    .unwrap_or_default();

                let resize_result = with_states(window.wl_surface().as_deref()?, |states| {
                    let mut data = states.data_map.get::<RefCell<SurfaceData>>()?.borrow_mut();

                    let resize_data = match data.resize_state {
                        ResizeState::Resizing(d) => Some(d),
                        ResizeState::WaitingForCommit(d) => {
                            let still_resizing = states
                                .cached_state
                                .get::<ToplevelCachedState>()
                                .current()
                                .last_acked
                                .as_ref()
                                .is_some_and(|c| c.state.states.contains(xdg_toplevel::State::Resizing));
                            if !still_resizing {
                                data.resize_state = ResizeState::NotResizing;
                            }
                            Some(d)
                        }
                        ResizeState::NotResizing => None,
                    }?;

                    let edges = resize_data.edges;
                    let loc = resize_data.initial_window_location;
                    let size = resize_data.initial_window_size;

                    let new_loc = edges.intersects(ResizeEdge::TOP_LEFT).then(|| {
                        let new_x = edges
                            .intersects(ResizeEdge::LEFT)
                            .then_some(loc.x + (size.w - inner_geometry.size.w));

                        let new_y = edges
                            .intersects(ResizeEdge::TOP)
                            .then_some(loc.y + (size.h - inner_geometry.size.h));

                        Point::<Option<i32>, Logical>::from((new_x, new_y))
                    });

                    Some((new_loc, edges, resize_data.warp_pointer))
                });

                if let Some((new_loc, edges, warp_pointer)) = resize_result {
                    if let Some(new_loc) = new_loc {
                        if let Some(new_x) = new_loc.x {
                            window_loc.x = new_x - decorations_offset.x;
                        }
                        if let Some(new_y) = new_loc.y {
                            window_loc.y = new_y - decorations_offset.y;
                        }

                        self.core.workspace_manager.relocate_window(&window, window_loc, false);
                    }

                    if warp_pointer {
                        if let Some(surface) = window.wl_surface() {
                            with_states(&surface, |states| {
                                if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                                    let mut data = data.borrow_mut();
                                    if let ResizeState::Resizing(ref mut rd) = data.resize_state {
                                        rd.warp_in_progress = true;
                                    }
                                }
                            });
                        }
                        self.warp_pointer_to_resize_edge(&window, window_loc, edges);
                    }
                }
            }
        }

        Some(())
    }

    pub(in crate::core) fn warp_pointer_to_resize_edge(
        &mut self,
        window: &WindowElement,
        window_loc: Point<i32, Logical>,
        edges: ResizeEdge,
    ) {
        let inner_geometry = SpaceElement::geometry(&window.0);
        let geometry = window
            .decoration_state()
            .window_decorations()
            .map(|decorations| {
                let e = decorations.decorations_extents();
                Rectangle::new(
                    window_loc,
                    (inner_geometry.size.w + e.left + e.right, inner_geometry.size.h + e.top + e.bottom).into(),
                )
            })
            .unwrap_or_else(|| Rectangle::new(window_loc, inner_geometry.size));

        let new_pointer_location: Option<(CursorIcon, Point<i32, Logical>)> = match edges {
            ResizeEdge::TOP => Some((CursorIcon::NResize, (geometry.loc.x + geometry.size.w / 2, geometry.loc.y).into())),
            ResizeEdge::LEFT => Some((CursorIcon::WResize, (geometry.loc.x, geometry.loc.y + geometry.size.h / 2).into())),
            ResizeEdge::RIGHT => Some((
                CursorIcon::EResize,
                (geometry.loc.x + geometry.size.w, geometry.loc.y + geometry.size.h / 2).into(),
            )),
            ResizeEdge::BOTTOM => Some((
                CursorIcon::SResize,
                (geometry.loc.x + geometry.size.w / 2, geometry.loc.y + geometry.size.h).into(),
            )),
            ResizeEdge::BOTTOM_RIGHT => Some((
                CursorIcon::SeResize,
                (geometry.loc.x + geometry.size.w, geometry.loc.y + geometry.size.h).into(),
            )),
            _ => None,
        };

        if let Some((cursor_icon, location)) = new_pointer_location {
            let pointer = self.core.pointer.clone();
            let event = MotionEvent {
                location: location.to_f64(),
                serial: SERIAL_COUNTER.next_serial(),
                time: self.core.now().as_millis(),
            };
            pointer.motion(self, None, &event);
            self.core.set_cursor(cursor_icon);
        }
    }

    fn handle_new_window_placement(&mut self, window: WindowElement, surface: &WlSurface) -> bool {
        if self.handle_new_window_menu_parent(&window) {
            if let Some(toplevel_surface) = window.0.toplevel() {
                toplevel_surface.send_pending_configure();
            }

            true
        } else if let Some(size) = self.find_window_content_size(&window) {
            if self.window_is_tabwin(&window, surface) {
                self.place_tabwin(&window, size);
                self.focus_window(&window, SERIAL_COUNTER.next_serial(), None);
            } else {
                let StackResult {
                    location,
                    allow_activate,
                    needs_attention,
                } = self.stack_new_window(&window);
                self.place_window(&window, size, location, allow_activate);

                if needs_attention {
                    self.set_window_urgent_state(&window, true);
                }

                let workspace = self.core.workspace_manager.active_workspace_mut();
                workspace.refresh();
                let outputs = workspace.outputs_for_window(&window);

                let parent = if let WindowSurface::Wayland(toplevel) = window.0.underlying_surface() {
                    toplevel
                        .parent()
                        .and_then(|parent_surface| self.window_for_surface(&parent_surface))
                } else {
                    None
                };

                self.core.toplevel_created::<Self>(&window, outputs, parent.as_ref());
            }

            if let Some(toplevel_surface) = window.0.toplevel() {
                toplevel_surface.send_pending_configure();
            }

            true
        } else {
            tracing::debug!("No window size available during initial placement; sending configure");
            if let Some(toplevel_surface) = window.0.toplevel() {
                toplevel_surface.send_configure();
            }
            false
        }
    }

    fn handle_new_window_menu_parent(&mut self, window: &WindowElement) -> bool {
        if let Some(toplevel_surface) = window.0.toplevel()
            && self.core.client_is_ui_thread(toplevel_surface.wl_surface().client())
            && let Some(title) = compositor::with_states(toplevel_surface.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .and_then(|data| data.lock().unwrap().title.clone())
            })
            && title == WINDOW_MENU_TOPLEVEL_TITLE
        {
            window.props().flags = WindowFlags::NO_CYCLE;
            self.core.window_menu_anchor = Some(window.clone());
            window.0.override_z_index(RenderZindex::Overlay as u8);

            toplevel_surface.with_pending_state(move |state| {
                state.size = Some((1, 1).into());
                state.decoration_mode = Some(xdg_decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ServerSide);
            });

            if toplevel_surface.is_initial_configure_sent() {
                toplevel_surface.send_pending_configure();
            }

            true
        } else {
            false
        }
    }

    fn find_window_content_size(&mut self, window: &WindowElement) -> Option<Size<i32, Logical>> {
        // For unmapped windows, some of these may be 0x0.  Use the inner Window's geometry
        // (content area only, without SSD decorations).
        let geometry = SpaceElement::geometry(&window.0);
        let bbox = SpaceElement::bbox(&window.0);
        let xdg_geometry = window.0.toplevel().and_then(|toplevel| {
            with_states(toplevel.wl_surface(), |states| {
                states.cached_state.get::<SurfaceCachedState>().current().geometry
            })
        });

        if geometry.size.w > 0 && geometry.size.h > 0 {
            Some(geometry.size)
        } else if bbox.size.w > 0 && bbox.size.h > 0 {
            Some(bbox.size)
        } else if let Some(xdg_geom) = xdg_geometry
            && xdg_geom.size.w > 0
            && xdg_geom.size.h > 0
        {
            Some(xdg_geom.size)
        } else {
            None
        }
    }

    fn window_for_toplevel_surface(&self, surface: &ToplevelSurface) -> Option<WindowElement> {
        self.core
            .workspace_manager
            .find_window(|elem| elem.0.toplevel().is_some_and(|surf| surf == surface))
            .or_else(|| self.core.pending_windows.get(surface.wl_surface()).cloned())
    }
}

pub fn app_id_for_xdg_toplevel(toplevel_surface: &ToplevelSurface) -> Option<String> {
    compositor::with_states(toplevel_surface.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|state| state.lock().unwrap().app_id.clone())
    })
}

pub fn desktop_app_info_for_xdg_toplevel(toplevel_surface: &ToplevelSurface) -> Option<gio::DesktopAppInfo> {
    compositor::with_states(toplevel_surface.wl_surface(), |states| {
        states.data_map.get::<XdgToplevelSurfaceData>().and_then(|state| {
            let s = state.lock().unwrap();
            s.app_id.as_ref().and_then(|app_id| {
                let desktop_name = if app_id.ends_with(".desktop") {
                    app_id
                } else {
                    &format!("{app_id}.desktop")
                };
                gio::DesktopAppInfo::new(desktop_name)
            })
        })
    })
}

pub fn app_name_for_xdg_toplevel(toplevel_surface: &ToplevelSurface, desktop_app_info: Option<&gio::DesktopAppInfo>) -> Option<String> {
    desktop_app_info
        .as_ref()
        .and_then(|app_info| {
            let name = app_info.name().to_string();
            (!name.is_empty()).then_some(name)
        })
        .or_else(|| {
            compositor::with_states(toplevel_surface.wl_surface(), |states| {
                states.data_map.get::<XdgToplevelSurfaceData>().and_then(|state| {
                    let s = state.lock().unwrap();
                    s.app_id.as_ref().and_then(|s| prettify_name(s))
                })
            })
        })
}

pub fn window_title_for_xdg_toplevel(surface: &ToplevelSurface) -> Option<String> {
    compositor::with_states(surface.wl_surface(), |states| {
        states.data_map.get::<XdgToplevelSurfaceData>().and_then(|data| {
            let d = data.lock().unwrap();
            d.title.clone()
        })
    })
}
