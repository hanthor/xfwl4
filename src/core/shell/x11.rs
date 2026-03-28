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
    os::unix::io::OwnedFd,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use smithay::{
    desktop::{Window, WindowSurface, space::SpaceElement},
    reexports::{
        calloop::{
            RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        rustix::{process, system},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Rectangle, SERIAL_COUNTER, Size},
    wayland::{
        seat::WaylandFocus,
        selection::{
            SelectionTarget,
            data_device::{
                clear_data_device_selection, current_data_device_selection_userdata, request_data_device_client_selection,
                set_data_device_selection,
            },
            primary_selection::{
                clear_primary_selection, current_primary_selection_userdata, request_primary_client_selection, set_primary_selection,
            },
        },
        xwayland_keyboard_grab::XWaylandKeyboardGrabHandler,
        xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
    },
    xwayland::{
        X11Surface, X11Wm, XwmHandler,
        xwm::{PingError, Reorder, ResizeEdge as X11ResizeEdge, WmWindowProperty, WmWindowType, XwmId},
    },
};
use tracing::{error, trace};

use crate::{
    backend::Backend,
    core::{
        config::ActivateAction,
        focus::KeyboardFocusTarget,
        placement::StackResult,
        shell::{GrabTrigger, WindowState},
        state::{WindowClient, Xfwl4State},
        util::ImageData,
    },
};

use super::{WindowElement, WindowLayout};

const WINDOW_PING_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct X11ClientId(pub u32);

#[derive(Debug, Default)]
pub struct X11WindowPropsInner {
    ping_timeout_token: Option<RegistrationToken>,
}

#[derive(Debug, Default)]
pub struct X11WindowProps(pub Mutex<X11WindowPropsInner>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct X11ClientId(pub u32);

impl<BackendData: Backend> XWaylandShellHandler for Xfwl4State<BackendData> {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.core.shell_protocol_delegates.xwayland_shell_state
    }
}

impl<BackendData: Backend> XwmHandler for Xfwl4State<BackendData> {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.core.xwayland.as_mut().unwrap().xwm()
    }

    fn new_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let internal_window_id = self.core.next_window_id();

        if let Some(xw) = self.core.xwayland.as_mut() {
            surface
                .user_data()
                .insert_if_missing(|| X11ClientId(surface.window_id() & xw.client_resource_mask()));
            let window = WindowElement::new(Window::new_x11_window(surface.clone()), internal_window_id, &self.core.config);

            if let Err(err) = xw.init_window_as_pending(window) {
                tracing::info!("Failed to add new pending X11 window: {err}");
            }
        }
    }

    fn new_override_redirect_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let internal_window_id = self.core.next_window_id();

        if let Some(xw) = self.core.xwayland.as_mut() {
            surface
                .user_data()
                .insert_if_missing(|| X11ClientId(surface.window_id() & xw.client_resource_mask()));
            let window = WindowElement::new(Window::new_x11_window(surface.clone()), internal_window_id, &self.core.config);

            if let Err(err) = xw.init_window_as_pending(window) {
                tracing::info!("Failed to add new pending X11 window: {err}");
            }
        }
    }

    fn map_window_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(window) = self
            .core
            .xwayland
            .as_mut()
            .and_then(|xw| xw.remove_pending_window(surface.window_id()))
            .or_else(|| {
                self.core
                    .workspace_manager
                    .find_window(|elem| matches!(elem.0.x11_surface(), Some(s) if s == &surface))
            })
        {
            let parent = surface.is_transient_for().and_then(|window_id| {
                self.core.workspace_manager.active_workspace().find_window(
                    |elem| matches!(elem.0.underlying_surface(), WindowSurface::X11(surface) if surface.window_id() == window_id),
                )
            });

        let _ = surface.set_mapped(true);
        surface
            .user_data()
            .insert_if_missing(|| X11ClientId(surface.window_id() & self.core.x11_client_mask));
        let window = WindowElement::new(
            Window::new_x11_window(surface.clone()),
            self.core.next_window_id(),
            &self.core.config,
        );
        self.set_window_parent(&window, parent.clone());

            self.set_window_parent(&window, parent.clone());

            window
                .props()
                .window_icon
                .update_app_id(Some(surface.class()).filter(|s| !s.is_empty()));
            self.x11_update_window_icon(&window);

            if !surface.is_decorated() {
                self.enable_decorations_for_window(&window);
            } else {
                self.disable_decorations_for_window(&window);
            }

            let content_size = self.x11_window_content_size(&surface);

            let StackResult {
                location,
                allow_activate,
                needs_attention,
            } = self.stack_new_window(&window);
            self.place_window(&window, content_size, location, allow_activate);

            if needs_attention {
                self.set_window_urgent_state(&window, true);
            }

            let workspace = self.core.workspace_manager.active_workspace_mut();
            if let Some(loc) = workspace.window_location(&window) {
                let visible_rect = Rectangle::new(loc, content_size);
                let mut buffer_rect = window.grow_rect_by_gtk_frame_extents(visible_rect);
                buffer_rect.size = self.x11_constrain_to_size_hints(&surface, buffer_rect.size);
                let _ = surface.configure(Some(buffer_rect));
            }

            if surface.is_maximized() {
                self.set_window_maximized(&window, None);
            }
            if surface.is_shaded() {
                self.set_window_shaded(&window, true);
            }
            if surface.is_sticky() {
                self.set_window_sticky(&window, true);
            }
            if surface.is_hidden() {
                self.set_window_minimized(&window);
            }

            let outputs = self.core.workspace_manager.active_workspace_mut().outputs_for_window(&window);
            self.core.toplevel_created::<Self>(&window, outputs, parent.as_ref());

            self.x11_update_window_allowed_actions(&window);
        }

        let StackResult {
            location,
            allow_activate,
            needs_attention,
        } = self.stack_new_window(&window);
        self.place_window(&window, surface.geometry().size, location, allow_activate);

        if needs_attention {
            self.set_window_urgent_state(&window, true);
        }

        let workspace = self.core.workspace_manager.active_workspace_mut();
        if let Some(bbox) = workspace.window_bbox(&window) {
            let _ = surface.configure(Some(bbox));
        }

        let outputs = self.core.workspace_manager.active_workspace_mut().outputs_for_window(&window);
        self.core.toplevel_created::<Self>(&window, outputs, parent.as_ref());
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let location = surface.geometry().loc;
        surface
            .user_data()
            .insert_if_missing(|| X11ClientId(surface.window_id() & self.core.x11_client_mask));
        let window = WindowElement::new(Window::new_x11_window(surface), self.core.next_window_id(), &self.core.config);
        self.new_window(window, location, true, None);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let target_id = window.window_id();
        for workspace in self.core.workspace_manager.workspaces_mut() {
            let maybe = workspace
                .visible_windows()
                .find(|e| matches!(e.0.x11_surface(), Some(s) if s.window_id() == target_id))
                .cloned();
            if let Some(elem) = maybe {
                // FIXME: is this what we really want?
                self.set_window_minimized(&elem);
                break;
            }
        }
        if !window.is_override_redirect() {
            window.set_mapped(false).unwrap();
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, surface: X11Surface) {
        let target_id = surface.window_id();
        let found = self
            .core
            .workspace_manager
            .find_window(|elem| matches!(elem.0.underlying_surface(), WindowSurface::X11(s) if s.window_id() == target_id));
        if let Some(window) = found {
            window.handle_destroyed();
            self.remove_window(&window);
            self.core.toplevel_destroyed(&window);

            if let Some(xw) = self.core.xwayland.as_ref() {
                let client_mask = xw.x11_client_mask;
                let surface_client_id = surface.window_id() & client_mask;
                let has_remaining = self.core.workspace_manager.workspaces().iter().any(|workspace| {
                    workspace.all_windows().any(
                        |w| matches!(w.0.underlying_surface(), WindowSurface::X11(s) if s.window_id() & client_mask == surface_client_id),
                    )
                });
                if !has_remaining {
                    self.core.clients_with_windows.remove(&WindowClient::X11(surface_client_id));
                }
            }
        }

        // X11Wm will re-set window stacking on window destroy, which will be incorrect, because
        // X11Wm doesn't actually know the correct stacking order.  `self.remove_window()` above
        // should fix it up, but let's be safe.
        self.x11_update_window_stacking_order();
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        surface: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let surface_geometry = surface.last_configure();

        let location = if (surface.is_override_redirect()
            || surface
                .window_type()
                .is_some_and(|ty| !matches!(ty, WmWindowType::Normal | WmWindowType::Dialog)))
            && (x.is_some() || y.is_some())
            && let Some((window, _, workspace)) = self
                .core
                .workspace_manager
                .find_window_and_workspace_mut(|elem| elem.0.x11_surface() == Some(&surface))
                && let Some(location) = workspace.window_location(&window)
            {
                let location = (x.unwrap_or(location.x), y.unwrap_or(location.y)).into();
                self.core.workspace_manager.relocate_window(&window, location, false);
                location
            } else {
                // Maybe it's a pending window.
                (x.unwrap_or(surface_geometry.loc.x), y.unwrap_or(surface_geometry.loc.y)).into()
            }
        } else {
            // Other kinds of windows don't get to move around freely.
            surface_geometry.loc
        };

        let configure_geometry = Rectangle::new(
            location,
            (
                w.unwrap_or(surface_geometry.size.w as u32) as i32,
                h.unwrap_or(surface_geometry.size.h as u32) as i32,
            )
                .into(),
        );
        let _ = surface.configure(configure_geometry);
    }

    fn configure_notify(&mut self, _xwm: XwmId, window: X11Surface, geometry: Rectangle<i32, Logical>, _above: Option<u32>) {
        if let Some(elem) = self
            .core
            .workspace_manager
            .find_window(|elem| matches!(elem.0.x11_surface(), Some(w) if w == &window))
        {
            // geometry.loc is the content position; subtract decoration offset to get frame position.
            let deco_offset = elem
                .decoration_state()
                .window_decorations()
                .map(|d| d.decorations_offset())
                .unwrap_or_default();
            self.core.workspace_manager.relocate_window(&elem, geometry.loc - deco_offset, false);
            // TODO: We don't properly handle the order of override-redirect windows here,
            //       they are always mapped top and then never reordered.
        }
    }

    fn minimize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(window) = self
            .core
            .workspace_manager
            .find_window(|e| matches!(e.0.x11_surface(), Some(w) if w == &window))
        {
            self.set_window_minimized(&window);
        }
    }

    fn maximize_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_maximized(&window, None);
        }
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_unmaximized(&window, None);
        }
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(elem) = self
            .core
            .workspace_manager
            .active_workspace()
            .find_window(|e| matches!(e.0.x11_surface(), Some(w) if w == &window))
        {
            self.set_window_fullscreen(&elem, None);
        }
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        // This is kinda dumb, but keeps the borrow checker happy
        if let Some(window) = self
            .core
            .workspace_manager
            .find_window(|e| matches!(e.0.x11_surface(), Some(w) if w == &window))
        {
            self.set_window_unfullscreen(&window);
        }
    }

    fn above_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_always_on_top(&window);
        }
    }

    fn unabove_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_normal_stacking(&window);
        }
    }

    fn below_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_always_on_bottom(&window);
        }
    }

    fn unbelow_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_normal_stacking(&window);
        }
    }

    fn shade_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_shaded(&window, true);
        }
    }

    fn unshade_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_shaded(&window, false);
        }
    }

    fn stick_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_sticky(&window, true);
        }
    }

    fn unstick_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_sticky(&window, false);
        }
    }

    fn demands_attention_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
            && !window.active()
        {
            self.set_window_urgent_state(&window, true);
        }
    }

    fn undemands_attention_request(&mut self, _xwm: XwmId, surface: X11Surface) {
        if let Some(wl_surface) = surface.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.set_window_urgent_state(&window, false);
        }
    }

    fn resize_request(&mut self, _xwm: XwmId, window: X11Surface, _button: u32, edges: X11ResizeEdge) {
        if let Some(wl_surface) = window.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.start_window_resize(
                window,
                self.core.seat.clone(),
                SERIAL_COUNTER.next_serial(),
                edges.into(),
                GrabTrigger::Pointer,
            );
        }
    }

    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, _button: u32) {
        if let Some(wl_surface) = window.wl_surface()
            && let Some(window) = self.window_for_surface(&wl_surface)
        {
            self.start_window_move(window, self.core.seat.clone(), SERIAL_COUNTER.next_serial(), GrabTrigger::Pointer);
        }
    }

    fn active_window_request(&mut self, _xwm: XwmId, surface: X11Surface, _timestamp: u32, currently_active_window: Option<X11Surface>) {
        // Smithay doesn't expose the 'source' field of the _NET_ACTIVE_WINDOW message, which
        // can tell us if the source was a pager application.  An X11 WM might unconditionally
        // allow the activation request for pagers, but that field can of course be spoofed.  Here
        // we are going to deviate from that behavior, because on Wayland, it would be surprising
        // if the pager app was an X11 app and not a Wayland app using foreign-toplevel-management
        // to do the activation.  (And if we did have an X11 pager app, it would only be able to
        // see other X11 windows, and not be that useful anyway.)

        let currently_active_window = currently_active_window.and_then(|caw| {
            self.core
                .workspace_manager
                .find_window(|elem| elem.0.x11_surface().is_some_and(|surf| *surf == caw))
        });

        if let Some(wl_surface) = surface.wl_surface()
            && let Some((window, _, workspace)) = self
                .core
                .workspace_manager
                .find_window_and_workspace_mut(|elem| elem.0.wl_surface().is_some_and(|surf| surf.as_ref() == &wl_surface))
        {
            if currently_active_window.is_some_and(|caw| window.same_application_as(&caw)) {
                self.activate_window(&window, self.core.config.raise_on_focus(), false, None);
            } else if self.core.config.prevent_focus_stealing()
                && (window
                    .last_user_interaction()
                    .is_none_or(|lui| lui < self.core.last_user_interaction)
                    || self.core.config.activate_action() == ActivateAction::None)
            {
                if let Some(topmost_window) = workspace.visible_windows().last().cloned() {
                    workspace.lower_window_below(&window, &topmost_window);
                } else {
                    workspace.raise_window(&window, false);
                }
            } else {
                self.set_window_urgent_state(&window, true);
                let current_focus = self.core.seat.get_keyboard().and_then(|keyboard| keyboard.current_focus());

                if current_focus != Some(window.clone().into()) {
                    self.activate_window(&window, self.core.config.raise_on_focus(), false, None);
                }
            }
        }
    }

    fn allow_selection_access(&mut self, xwm: XwmId, _selection: SelectionTarget) -> bool {
        if let Some(keyboard) = self.core.seat.get_keyboard() {
            // check that an X11 window is focused
            if let Some(KeyboardFocusTarget::Window(w)) = keyboard.current_focus()
                && let Some(surface) = w.x11_surface()
                && surface.xwm_id().as_ref().is_some_and(|id| id == &xwm)
            {
                return true;
            }
        }
        false
    }

    fn send_selection(&mut self, _xwm: XwmId, selection: SelectionTarget, mime_type: String, fd: OwnedFd) {
        match selection {
            SelectionTarget::Clipboard => {
                if let Err(err) = request_data_device_client_selection(&self.core.seat, mime_type, fd) {
                    error!(?err, "Failed to request current wayland clipboard for Xwayland",);
                }
            }
            SelectionTarget::Primary => {
                if let Err(err) = request_primary_client_selection(&self.core.seat, mime_type, fd) {
                    error!(?err, "Failed to request current wayland primary selection for Xwayland",);
                }
            }
        }
    }

    fn new_selection(&mut self, _xwm: XwmId, selection: SelectionTarget, mime_types: Vec<String>) {
        trace!(?selection, ?mime_types, "Got Selection from X11",);
        // TODO check, that focused windows is X11 window before doing this
        match selection {
            SelectionTarget::Clipboard => set_data_device_selection(&self.core.display_handle, &self.core.seat, mime_types, ()),
            SelectionTarget::Primary => set_primary_selection(&self.core.display_handle, &self.core.seat, mime_types, ()),
        }
    }

    fn cleared_selection(&mut self, _xwm: XwmId, selection: SelectionTarget) {
        match selection {
            SelectionTarget::Clipboard => {
                if current_data_device_selection_userdata(&self.core.seat).is_some() {
                    clear_data_device_selection(&self.core.display_handle, &self.core.seat)
                }
            }
            SelectionTarget::Primary => {
                if current_primary_selection_userdata(&self.core.seat).is_some() {
                    clear_primary_selection(&self.core.display_handle, &self.core.seat)
                }
            }
        }
    }

    fn show_desktop_request(&mut self, _xwm: XwmId) {
        self.activate_show_desktop();
    }

    fn unshow_desktop_request(&mut self, _xwm: XwmId) {
        self.deactivate_show_desktop();
    }

    fn property_notify(&mut self, _xwm: XwmId, surface: X11Surface, property: WmWindowProperty) {
        if let Some(window) = surface.wl_surface().and_then(|surf| self.window_for_surface(&surf)) {
            match property {
                WmWindowProperty::Title => self.core.toplevel_changed(
                    &window,
                    Some(&surface.title()),
                    None,
                    WindowState::empty(),
                    WindowState::empty(),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
                WmWindowProperty::Class => self.core.toplevel_changed(
                    &window,
                    None,
                    Some(&surface.class()),
                    WindowState::empty(),
                    WindowState::empty(),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
                WmWindowProperty::TransientFor => {
                    if let Some(workspace) = self.core.workspace_manager.workspace_for_window_mut(&window) {
                        let parent = surface.is_transient_for().and_then(|window_id| {
                            workspace.find_window(|elem| matches!(elem.0.underlying_surface(), WindowSurface::X11(surface) if surface.window_id() == window_id))
                        });

                        self.set_window_parent(&window, parent.clone());

                        self.core.toplevel_changed(
                            &window,
                            None,
                            None,
                            WindowState::empty(),
                            WindowState::empty(),
                            Vec::new(),
                            Vec::new(),
                            Some(parent.as_ref()),
                        );
                    }
                }
                WmWindowProperty::Hints => {
                    let urgent = surface.hints().map(|hints| hints.urgent);
                    self.set_window_urgent_state(&window, urgent.unwrap_or(false));
                }
                WmWindowProperty::FrameExtents => {
                    // The frame extents (shadow widths) changed, so a tiled window's
                    // visible-content edge may no longer line up with its anchor.  Re-apply
                    // the anchored layout to keep it snapped.
                    let layout = window.current_layout();
                    if layout != WindowLayout::Normal {
                        let output_and_geom = window
                            .props()
                            .anchored_output
                            .as_ref()
                            .and_then(|weak| weak.upgrade())
                            .and_then(|output| self.core.workspace_manager.output_geometry(&output).map(|geom| (output, geom)));
                        if let Some((output, output_geom)) = output_and_geom
                            && self.apply_anchored_layout(&window, layout, &output, output_geom).is_none()
                        {
                            self.set_window_untiled(&window, None);
                        }
                    }
                }
                // TODO: need to manually add a property notify for _NET_WM_STATE
                _ => (),
            }
        }
    }

    fn ping_acked(&mut self, _xwm: XwmId, surface: X11Surface, _timestamp: u32) {
        if let Some(token) = surface
            .user_data()
            .get_or_insert(X11WindowProps::default)
            .0
            .lock()
            .unwrap()
            .ping_timeout_token
            .take()
        {
            self.core.handle.remove(token);
        }
    }

    fn disconnected(&mut self, _xwm: XwmId) {
        if let Some(display_number) = self.xwayland_destroyed()
            && self.core.is_running
        {
            self.maybe_schedule_xwayland_restart(display_number);
        }
    }
}

impl<BackendData: Backend + 'static> XWaylandKeyboardGrabHandler for Xfwl4State<BackendData> {
    fn keyboard_focus_for_xsurface(&self, surface: &WlSurface) -> Option<KeyboardFocusTarget> {
        let elem = self
            .core
            .workspace_manager
            .active_workspace()
            .visible_windows()
            .find(|elem: &&WindowElement| elem.wl_surface().as_deref() == Some(surface))?;
        Some(KeyboardFocusTarget::Window(elem.0.clone()))
    }
}

impl<BackendData: Backend> Xfwl4State<BackendData> {
    /// Try to find a sensible content size for a newly-mapped X11 window.  smithay's
    /// `SpaceElement::geometry()` returns the visible-content rect (the bounding box with
    /// any frame extents already subtracted, matching the Wayland path's convention), but
    /// can be 0x0 for clients that rely on the WM to size them; fall through ICCCM size
    /// hints (`base_size`, `min_size`) before defaulting.
    pub(in crate::core) fn x11_window_content_size(&self, surface: &X11Surface) -> Size<i32, Logical> {
        let geometry = SpaceElement::geometry(surface);
        if geometry.size.w > 0 && geometry.size.h > 0 {
            geometry.size
        } else if let Some(base) = surface.base_size().filter(|s| s.w > 0 && s.h > 0) {
            base
        } else if let Some(min) = surface.min_size().filter(|s| s.w > 0 && s.h > 0) {
            min
        } else {
            Size::from((100, 100))
        }
    }

    pub(in crate::core) fn x11_constrain_to_size_hints(&self, surface: &X11Surface, requested: Size<i32, Logical>) -> Size<i32, Logical> {
        let mut size = requested;

        let min = surface.min_size();
        let max = surface.max_size();
        let base = surface.base_size();
        let hints = surface.size_hints();

        if let Some(max) = max {
            if max.w > 0 {
                size.w = size.w.min(max.w);
            }
            if max.h > 0 {
                size.h = size.h.min(max.h);
            }
        }
        if let Some(min) = min {
            size.w = size.w.max(min.w);
            size.h = size.h.max(min.h);
        }

        if let Some(hints) = hints
            && let Some((inc_w_client, inc_h_client)) = hints.size_increment
        {
            let scale = self.xwayland_client_scale(surface);
            let inc_w = ((inc_w_client as f64) / scale).round().max(1.0) as i32;
            let inc_h = ((inc_h_client as f64) / scale).round().max(1.0) as i32;
            let base = base.unwrap_or_else(|| Size::from((0, 0)));

            if inc_w > 1 && size.w >= base.w {
                size.w = base.w + ((size.w - base.w) / inc_w) * inc_w;
            }
            if inc_h > 1 && size.h >= base.h {
                size.h = base.h + ((size.h - base.h) / inc_h) * inc_h;
            }
        }

        if let Some(min) = min {
            size.w = size.w.max(min.w);
            size.h = size.h.max(min.h);
        }

        size
    }

    pub(in crate::core::shell) fn ping_x11_window(&self, window: &WindowElement, surface: &X11Surface) {
        let ping_pending = window.x11_props().map(|props| props.ping_timeout_token.is_some()).unwrap_or(false);

        if !ping_pending {
            match surface.send_ping(self.core.clock.now().as_millis()) {
                Err(PingError::NotSupported | PingError::InvalidTimestamp | PingError::PingAlreadyPending(_)) => (),
                Err(PingError::Connection(err)) => tracing::info!("Failed to send ping to X11 window 0x{:08x}: {err}", surface.window_id()),
                Ok(_) => {
                    if let Some(mut props) = window.x11_props() {
                        if let Some(token) = props.ping_timeout_token.take() {
                            self.core.handle.remove(token);
                        }

                        let surface = surface.clone();

                        let token = self
                            .core
                            .handle
                            .insert_source(Timer::from_duration(WINDOW_PING_TIMEOUT), move |_, _, state| {
                                if let Some(xw) = state.core.xwayland.as_ref() {
                                    if xw
                                        .get_wm_client_machine(surface.window_id())
                                        .is_some_and(|client_machine| client_machine.as_c_str() == system::uname().nodename())
                                        && let Ok(client_pid) = surface.get_client_pid().map(|pid| pid as process::RawPid)
                                        && client_pid > 0
                                        && let Some(client_pid) = process::Pid::from_raw(client_pid)
                                    {
                                        let _ = process::kill_process(client_pid, process::Signal::KILL);
                                    }

                                    xw.kill_client_by_window(surface.window_id());
                                }

                                TimeoutAction::Drop
                            })
                            .ok();
                        props.ping_timeout_token = token;
                    }
                }
            }
        }
    }
}

impl WindowElement {
    pub(in crate::core) fn x11_props(&self) -> Option<MutexGuard<'_, X11WindowPropsInner>> {
        self.0
            .x11_surface()
            .map(|surface| surface.user_data().get_or_insert(X11WindowProps::default))
            .map(|x11_props| x11_props.0.lock().unwrap())
    }

    /// Given a rect in visible-content coordinates, return the rect in X11-window coordinates
    /// by shifting the origin outward and growing the size by the window's frame extents
    /// (drop-shadow widths reported via `_GTK_FRAME_EXTENTS`). No-op for non-X11 windows, and for
    /// X11 windows without any frame extents set.
    pub(in crate::core) fn grow_rect_by_gtk_frame_extents(&self, rect: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        if let Some(surface) = self.0.x11_surface() {
            rect + surface.frame_extents()
        } else {
            rect
        }
    }
}
