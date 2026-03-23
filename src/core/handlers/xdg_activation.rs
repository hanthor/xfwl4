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

use std::time::Duration;

use smithay::{
    input::Seat,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        seat::WaylandFocus,
        xdg_activation::{XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData},
    },
};

use crate::{
    backend::Backend,
    core::{focus::KeyboardFocusTarget, state::Xfwl4State},
};

const MAX_TOKEN_LIFETIME: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct ActivationTokenExtraData {
    serial_is_from_current_focus: Option<bool>,
    surface_is_focused: Option<bool>,
}

impl<BackendData: Backend> XdgActivationHandler for Xfwl4State<BackendData> {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.core.protocol_delegates.xdg_activation_state
    }

    fn token_created(&mut self, _token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        let (surface_is_focused, serial_is_from_current_focus) = data
            .serial
            .and_then(|(serial, seat)| {
                Seat::<Self>::from_resource(&seat)
                    .and_then(|seat| seat.get_keyboard())
                    .map(|keyboard| {
                        let surface_is_focused = data
                            .surface
                            .as_ref()
                            .and_then(|surface| {
                                keyboard.current_focus().map(|focus| match focus {
                                    KeyboardFocusTarget::Window(window) => {
                                        window.wl_surface().map(|focus| focus.as_ref() == surface).unwrap_or(false)
                                    }
                                    KeyboardFocusTarget::Popup(popup) => popup.wl_surface() == surface,
                                    KeyboardFocusTarget::LayerSurface(layer_surf) => layer_surf.wl_surface() == surface,
                                })
                            })
                            .unwrap_or(false);

                        let serial_is_from_current_focus =
                            keyboard.last_enter().is_some_and(|last_enter| serial.is_no_older_than(&last_enter));

                        (surface_is_focused, serial_is_from_current_focus)
                    })
            })
            .unzip();

        let extra_data = ActivationTokenExtraData {
            serial_is_from_current_focus,
            surface_is_focused,
        };
        data.user_data.insert_if_missing(|| extra_data);

        true
    }

    fn request_activation(&mut self, _token: XdgActivationToken, token_data: XdgActivationTokenData, surface: WlSurface) {
        if let Some((window, _, workspace)) = self
            .core
            .workspace_manager
            .find_window_and_workspace_mut(|elem| elem.wl_surface().is_some_and(|elem_surface| elem_surface.as_ref() == &surface))
        {
            let current_focus = token_data
                .serial
                .as_ref()
                .and_then(|(_, seat)| Seat::<Self>::from_resource(seat))
                .and_then(|seat| seat.get_keyboard())
                .and_then(|keyboard| keyboard.current_focus());

        if do_activate {
            let active_workspace_index = self.core.workspace_manager.active_workspace_index();
            if let Some((window, index, _)) = self
                .core
                .workspace_manager
                .find_window_and_workspace_mut(|elem| elem.wl_surface().is_some_and(|elem_surface| elem_surface.as_ref() == &surface))
            {
                let raise_on_focus = self.core.config.raise_on_focus();
                let seat = token_data.serial.and_then(|(_, seat)| Seat::from_resource(&seat));

                if do_activate {
                    let raise_on_focus = self.core.config.raise_on_focus();
                    let seat = token_data.serial.and_then(|(_, seat)| Seat::from_resource(&seat));
                    self.activate_window(&window, raise_on_focus, true, seat);
                } else {
                    if let Some(topmost_window) = workspace.visible_windows().last().cloned() {
                        workspace.lower_window_below(&window, &topmost_window);
                    } else {
                        workspace.raise_window(&window, false);
                    }

                    self.set_window_urgent_state(&window, true);
                }
            }
        }
    }
}
