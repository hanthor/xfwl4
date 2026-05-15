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

use std::{ffi::OsString, process::Command, time::Duration};

use gtk::gdk::ModifierType;
use smithay::{
    backend::input::{ButtonState, KeyState, ProximityState, Switch, SwitchState, TabletToolTipState, TouchSlot},
    desktop::{WindowSurfaceType, layer_map_for_output},
    input::{
        keyboard::{FilterResult, Keycode, Keysym, keysyms as xkb},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent, GesturePinchEndEvent,
            GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent,
            GrabStartData as PointerGrabStartData, MotionEvent, PointerHandle, RelativeMotionEvent,
        },
        touch::{DownEvent, UpEvent},
    },
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        wayland_server::protocol::wl_pointer,
    },
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Serial, Size},
    wayland::{
        compositor::RegionAttributes,
        input_method::InputMethodSeat,
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitorSeat,
        pointer_constraints::{PointerConstraint, with_pointer_constraint},
        seat::WaylandFocus,
        shell::wlr_layer::{KeyboardInteractivity, Layer as WlrLayer},
        tablet_manager::TabletSeatTrait,
    },
};
use tracing::{debug, error, info};

use crate::{
    backend::{
        Backend, DeviceCapabilities, KeyboardInputEvent, PointerInputEvent, SwitchInputEvent, TabletInputEvent, TabletToolAxisData,
        TabletToolButtonData, TabletToolProximityData, TabletToolTipData, TouchInputEvent, TranslatedInput,
    },
    core::{
        config::{ShortcutKey, WmShortcutAction},
        edge::ScreenEdge,
        focus::{KeyboardFocusTarget, PointerFocusTarget},
        handlers::xfwl4_compositor_ui::ActionLocation,
        shell::{GrabTrigger, ResizeEdge, SSD, TileMode, WindowElement},
        state::{Xfwl4Core, Xfwl4State},
        util::{BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, Direction, LaptopLidState, XkbStateGdkExt},
    },
    protocols::xfwl4_compositor_ui::TabwinConfig,
};

#[derive(Default)]
struct PointerConstraintState {
    locked: bool,
    confined: bool,
    confine_region: Option<RegionAttributes>,
}

impl<BackendData: Backend> Xfwl4State<BackendData> {
    pub(in crate::core) fn process_common_key_action(&mut self, action: KeyAction, serial: Serial) {
        let focused_window = || {
            self.core
                .seat
                .get_keyboard()
                .and_then(|keyboard| keyboard.current_focus())
                .and_then(|focus| match focus {
                    KeyboardFocusTarget::Window(w) => self.core.workspace_manager.active_workspace().find_window(|elem| elem.0 == w),
                    _ => None,
                })
        };

        match action {
            KeyAction::None => (),

            KeyAction::Quit => {
                info!("Quitting.");
                self.shutdown();
            }

            KeyAction::VtSwitch(num) => {
                self.backend.switch_vt(num);
            }

            KeyAction::Run(argv0, args) => {
                info!("Starting program: {}", argv0.display());
                if let Err(err) = Command::new(&argv0).args(args).spawn() {
                    error!("Failed to start program {}: {err}", argv0.display());
                }
            }

            KeyAction::WmAction(WmShortcutAction::PopupMenu) => {
                if let Some(window) = focused_window() {
                    let seat = self.core.seat.clone();
                    let pointer_location = self.core.pointer.current_location().to_i32_round();
                    self.pop_up_window_menu(&window, &seat, serial, ActionLocation::Absolute(pointer_location));
                }
            }

            KeyAction::WmAction(WmShortcutAction::CloseWindow) => {
                if let Some(window) = focused_window() {
                    window.close();
                }
            }

            KeyAction::WmAction(WmShortcutAction::MaximizeHoriz) => (), // TODO
            KeyAction::WmAction(WmShortcutAction::MaximizeVert) => (),  // TODO
            KeyAction::WmAction(WmShortcutAction::MaximizeWindow) => {
                if let Some(window) = focused_window() {
                    if !window.maximized() {
                        self.set_window_maximized(&window, None);
                    } else {
                        self.set_window_unmaximized(&window, None);
                    }
                }
            }

            KeyAction::WmAction(WmShortcutAction::FillWindow) => {
                if let Some(window) = focused_window() {
                    self.set_window_filled(&window);
                }
            }

            KeyAction::WmAction(WmShortcutAction::TileDown) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::Down);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileLeft) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::Left);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileRight) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::Right);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileUp) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::Up);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileDownLeft) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::DownLeft);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileDownRight) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::DownRight);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileUpLeft) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::UpLeft);
                }
            }
            KeyAction::WmAction(WmShortcutAction::TileUpRight) => {
                if let Some(window) = focused_window() {
                    self.toggle_window_tiled(&window, TileMode::UpRight);
                }
            }

            KeyAction::WmAction(WmShortcutAction::ShadeWindow) => {
                if let Some(window) = focused_window() {
                    self.set_window_shaded(&window, !window.shaded());
                }
            }

            KeyAction::WmAction(WmShortcutAction::StickWindow) => {
                if let Some(window) = focused_window() {
                    self.set_window_sticky(&window, !window.sticky());
                }
            }

            KeyAction::WmAction(WmShortcutAction::ToggleFullscreen) => {
                if let Some(window) = focused_window() {
                    let is_fullscreen = window.fullscreened();
                    if is_fullscreen {
                        self.set_window_unfullscreen(&window);
                    } else {
                        self.set_window_fullscreen(&window, None);
                    }
                }
            }

            KeyAction::WmAction(WmShortcutAction::HideWindow) => {
                if let Some(window) = focused_window() {
                    self.set_window_minimized(&window);
                }
            }

            KeyAction::WmAction(WmShortcutAction::Move) => {
                if let Some(window) = focused_window() {
                    let seat = self.core.seat.clone();
                    self.start_window_move(window, seat, serial, GrabTrigger::Keyboard);
                }
            }

            KeyAction::WmAction(WmShortcutAction::Resize) => {
                if let Some(window) = focused_window() {
                    let seat = self.core.seat.clone();
                    self.start_window_resize(window, seat, serial, ResizeEdge::BOTTOM_RIGHT, GrabTrigger::Keyboard);
                }
            }

            KeyAction::WmAction(WmShortcutAction::ToggleAbove) => {
                if let Some(window) = focused_window() {
                    if window.always_on_top() {
                        self.set_window_normal_stacking(&window);
                    } else {
                        self.set_window_always_on_top(&window);
                    }
                }
            }

            KeyAction::WmAction(WmShortcutAction::LowerWindow) => {
                if let Some(window) = focused_window() {
                    self.lower_window(&window, serial, None);
                }
            }

            KeyAction::WmAction(WmShortcutAction::RaiseWindow) => {
                if let Some(window) = focused_window() {
                    self.raise_window(&window, serial, true);
                }
            }

            KeyAction::WmAction(WmShortcutAction::RaiseLowerWindow) => {
                if let Some(window) = focused_window() {
                    let workspace = self.core.workspace_manager.active_workspace_mut();
                    let is_top = workspace.visible_windows().last().is_some_and(|last| last == &window);
                    if is_top {
                        self.lower_window(&window, serial, None);
                    } else {
                        self.raise_window(&window, serial, true);
                    }
                }
            }

            KeyAction::WmAction(WmShortcutAction::UpWorkspace) => {
                if let Some(index) = self
                    .core
                    .workspace_manager
                    .workspace_index_for_direction(Direction::Up, self.core.config.wrap_layout())
                {
                    self.set_active_workspace(index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::DownWorkspace) => {
                if let Some(index) = self
                    .core
                    .workspace_manager
                    .workspace_index_for_direction(Direction::Down, self.core.config.wrap_layout())
                {
                    self.set_active_workspace(index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::LeftWorkspace) => {
                if let Some(index) = self
                    .core
                    .workspace_manager
                    .workspace_index_for_direction(Direction::Left, self.core.config.wrap_layout())
                {
                    self.set_active_workspace(index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::RightWorkspace) => {
                if let Some(index) = self
                    .core
                    .workspace_manager
                    .workspace_index_for_direction(Direction::Right, self.core.config.wrap_layout())
                {
                    self.set_active_workspace(index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::NextWorkspace) => {
                if let Some(index) = self.core.workspace_manager.next_workspace_index(self.core.config.wrap_cycle()) {
                    self.set_active_workspace(index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::PrevWorkspace) => {
                if let Some(index) = self.core.workspace_manager.prev_workspace_index(self.core.config.wrap_cycle()) {
                    self.set_active_workspace(index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::Workspace1) => self.toggle_active_workspace(0),
            KeyAction::WmAction(WmShortcutAction::Workspace2) => self.toggle_active_workspace(1),
            KeyAction::WmAction(WmShortcutAction::Workspace3) => self.toggle_active_workspace(2),
            KeyAction::WmAction(WmShortcutAction::Workspace4) => self.toggle_active_workspace(3),
            KeyAction::WmAction(WmShortcutAction::Workspace5) => self.toggle_active_workspace(4),
            KeyAction::WmAction(WmShortcutAction::Workspace6) => self.toggle_active_workspace(5),
            KeyAction::WmAction(WmShortcutAction::Workspace7) => self.toggle_active_workspace(6),
            KeyAction::WmAction(WmShortcutAction::Workspace8) => self.toggle_active_workspace(7),
            KeyAction::WmAction(WmShortcutAction::Workspace9) => self.toggle_active_workspace(8),
            KeyAction::WmAction(WmShortcutAction::Workspace10) => self.toggle_active_workspace(9),
            KeyAction::WmAction(WmShortcutAction::Workspace11) => self.toggle_active_workspace(10),
            KeyAction::WmAction(WmShortcutAction::Workspace12) => self.toggle_active_workspace(11),
            KeyAction::WmAction(WmShortcutAction::AddWorkspace) => self.append_workspace(),
            KeyAction::WmAction(WmShortcutAction::AddAdjacentWorkspace) => {
                let cur_num = self.core.workspace_manager.active_workspace_index();
                self.insert_workspace(cur_num + 1);
            }
            KeyAction::WmAction(WmShortcutAction::DelWorkspace) => {
                let n_workspaces = self.core.workspace_manager.workspaces().len() as u32;
                self.remove_workspace(n_workspaces - 1);
            }
            KeyAction::WmAction(WmShortcutAction::DelActiveWorkspace) => {
                let cur_num = self.core.workspace_manager.active_workspace_index();
                self.remove_workspace(cur_num);
            }
            KeyAction::WmAction(WmShortcutAction::MoveUpWorkspace) => {
                if let Some(window) = focused_window()
                    && let Some(new_index) = self.move_window_to_workspace_in_direction(&window, Direction::Up)
                {
                    self.set_active_workspace(new_index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveDownWorkspace) => {
                if let Some(window) = focused_window()
                    && let Some(new_index) = self.move_window_to_workspace_in_direction(&window, Direction::Down)
                {
                    self.set_active_workspace(new_index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveLeftWorkspace) => {
                if let Some(window) = focused_window()
                    && let Some(new_index) = self.move_window_to_workspace_in_direction(&window, Direction::Left)
                {
                    self.set_active_workspace(new_index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveRightWorkspace) => {
                if let Some(window) = focused_window()
                    && let Some(new_index) = self.move_window_to_workspace_in_direction(&window, Direction::Right)
                {
                    self.set_active_workspace(new_index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MovePrevWorkspace) => {
                if let Some(window) = focused_window()
                    && let Some(new_index) = self.move_window_to_previous_workspace(&window)
                {
                    self.set_active_workspace(new_index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveNextWorkspace) => {
                if let Some(window) = focused_window()
                    && let Some(new_index) = self.move_window_to_next_workspace(&window)
                {
                    self.set_active_workspace(new_index);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace1) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 0)
                {
                    self.set_active_workspace(0);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace2) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 1)
                {
                    self.set_active_workspace(1);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace3) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 2)
                {
                    self.set_active_workspace(2);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace4) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 3)
                {
                    self.set_active_workspace(3);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace5) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 4)
                {
                    self.set_active_workspace(4);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace6) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 5)
                {
                    self.set_active_workspace(5);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace7) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 6)
                {
                    self.set_active_workspace(6);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace8) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 7)
                {
                    self.set_active_workspace(7);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace9) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 8)
                {
                    self.set_active_workspace(8);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace10) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 9)
                {
                    self.set_active_workspace(9);
                }
            }

            KeyAction::WmAction(WmShortcutAction::MoveWorkspace11) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 10)
                {
                    self.set_active_workspace(10);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveWorkspace12) => {
                if let Some(window) = focused_window()
                    && self.move_window_to_workspace_index(&window, 11)
                {
                    self.set_active_workspace(11);
                }
            }

            KeyAction::WmAction(WmShortcutAction::MoveToMonitorDown) => {
                if let Some(window) = focused_window() {
                    self.move_window_to_output_in_direction(&window, Direction::Down);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveToMonitorLeft) => {
                if let Some(window) = focused_window() {
                    self.move_window_to_output_in_direction(&window, Direction::Left);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveToMonitorRight) => {
                tracing::debug!("move to monitor right");
                if let Some(window) = focused_window() {
                    self.move_window_to_output_in_direction(&window, Direction::Right);
                }
            }
            KeyAction::WmAction(WmShortcutAction::MoveToMonitorUp) => {
                if let Some(window) = focused_window() {
                    self.move_window_to_output_in_direction(&window, Direction::Up);
                }
            }

            KeyAction::WmAction(action @ WmShortcutAction::CycleWindows)
            | KeyAction::WmAction(action @ WmShortcutAction::CycleReverseWindows) => {
                if let Some(output) = self.output_under_pointer() {
                    let windows = self.collect_tabwin_windows(&output);

                    let initial_selection = if action == WmShortcutAction::CycleWindows {
                        windows.get(1).or_else(|| windows.first())
                    } else {
                        windows.last()
                    }
                    .map(|client| client.window_id);

                    let get_shortcut = |action: WmShortcutAction| -> Option<(Keysym, ModifierType)> {
                        self.core.shortcut_for_wm_action(action).map(|key| (key.keysym, key.modifiers))
                    };

                    if let Some(initial_selection) = initial_selection {
                        let tabwin_config = TabwinConfig {
                            mode: self.core.config.cycle_tabwin_mode().into(),
                            window_opacity: (self.core.config.popup_opacity() as f64 / 100.).clamp(0., 1.),
                            show_window_previews: self.core.config.cycle_preview(),
                            windows,
                            initial_selection,
                            next_shortcut: get_shortcut(WmShortcutAction::CycleWindows),
                            prev_shortcut: get_shortcut(WmShortcutAction::CycleReverseWindows),
                            up_shortcut: get_shortcut(WmShortcutAction::Up),
                            down_shortcut: get_shortcut(WmShortcutAction::Down),
                            left_shortcut: get_shortcut(WmShortcutAction::Left),
                            right_shortcut: get_shortcut(WmShortcutAction::Right),
                            cancel_shortcut: get_shortcut(WmShortcutAction::Cancel),
                        };

                        if let Err(err) = self.core.compositor_ui_state.create_tabwin::<Self>(tabwin_config) {
                            tracing::warn!("Failed to create tabwin: {err}");
                        } else {
                            self.core.cycling_windows = true;
                        }
                    }
                }
            }

            KeyAction::WmAction(WmShortcutAction::FillHoriz) => (),         // TODO
            KeyAction::WmAction(WmShortcutAction::FillVert) => (),          // TODO
            KeyAction::WmAction(WmShortcutAction::ShowDesktop) => (),       // TODO
            KeyAction::WmAction(WmShortcutAction::SwitchApplication) => (), // TODO
            KeyAction::WmAction(WmShortcutAction::SwitchWindow) => (),      // TODO
            KeyAction::WmAction(WmShortcutAction::MoveLeft) => (),          // TODO: move window left by pixel
            KeyAction::WmAction(WmShortcutAction::MoveRight) => (),         // TODO: move window right by pixel
            KeyAction::WmAction(WmShortcutAction::MoveUp) => (),            // TODO: move window up by pixel
            KeyAction::WmAction(WmShortcutAction::MoveDown) => (),          // TODO: move window down by pixel

            KeyAction::WmAction(
                WmShortcutAction::Cancel | WmShortcutAction::Up | WmShortcutAction::Down | WmShortcutAction::Left | WmShortcutAction::Right,
            ) => {
                // I'm pretty sure we should never get here, as up/down/left/right/cancel are
                // explicitly ignored by the keyboard shortcut handler.  These are only used in
                // special circumstances like tabwin navigation and keyboard-interactive
                // move/resize.
                tracing::debug!("Got {action:?}, which is unexpected here");
            }
        }
    }

    pub(in crate::core) fn on_keyboard_key(&mut self, keycode: u32, state: KeyState, time: u32) -> (KeyAction, Serial) {
        let keycode = Keycode::new(keycode);
        debug!(?keycode, ?state, "key");
        let serial = SERIAL_COUNTER.next_serial();
        let mut suppressed_keys = self.core.suppressed_keys.clone();
        let keyboard = self.core.seat.get_keyboard().unwrap();

        for layer in self.core.shell_protocol_delegates.layer_surfaces().rev().collect::<Vec<_>>() {
            let exclusive = layer.with_cached_state(|data| {
                data.keyboard_interactivity == KeyboardInteractivity::Exclusive
                    && (data.layer == WlrLayer::Top || data.layer == WlrLayer::Overlay)
            });
            if exclusive {
                let surface = self.core.workspace_manager.outputs().find_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().find(|l| l.layer_surface() == &layer).cloned()
                });
                if let Some(surface) = surface {
                    self.focus_target(surface, serial, None);
                    keyboard.input::<(), _>(self, keycode, state, serial, time, |_, _, _| FilterResult::Forward);
                    return (KeyAction::None, serial);
                };
            }
        }

        let inhibited = self
            .core
            .workspace_manager
            .active_workspace()
            .window_under(self.core.pointer.current_location())
            .and_then(|(window, _)| {
                let surface = window.wl_surface()?;
                self.core.seat.keyboard_shortcuts_inhibitor_for_surface(&surface)
            })
            .map(|inhibitor| inhibitor.is_active())
            .unwrap_or(false);

        let modifier_mask = keyboard.with_xkb_state(self, |ctx| {
            let xkb = ctx.xkb().lock().unwrap();
            // SAFETY: I won't hold this reference longer than the Xkb instance above.
            let state = unsafe { xkb.state() };

            state.gdk_modifier_mask()
        });

        let action = keyboard
            .input(self, keycode, state, serial, time, |data, modifiers, handle| {
                let keysym = handle.modified_sym();
                let raw_keysym = handle.raw_latin_sym_or_raw_current_sym();

                debug!(
                    ?state,
                    mods = ?modifiers,
                    keysym = ::xkbcommon::xkb::keysym_get_name(keysym),
                    "keysym"
                );

                // If the key is pressed and triggered a action
                // we will not forward the key to the client.
                // Additionally add the key to the suppressed keys
                // so that we can decide on a release if the key
                // should be forwarded to the client or not.
                if let KeyState::Pressed = state {
                    if !inhibited {
                        let action = data
                            .process_keyboard_shortcut(modifier_mask, keysym)
                            .or_else(|| raw_keysym.and_then(|raw_keysym| data.process_keyboard_shortcut(modifier_mask, raw_keysym)));

                        if action.is_some() {
                            suppressed_keys.push(keysym);
                        }

                        action.map(FilterResult::Intercept).unwrap_or(FilterResult::Forward)
                    } else {
                        FilterResult::Forward
                    }
                } else {
                    let suppressed = suppressed_keys.contains(&keysym);
                    if suppressed {
                        suppressed_keys.retain(|k| *k != keysym);
                        FilterResult::Intercept(KeyAction::None)
                    } else {
                        FilterResult::Forward
                    }
                }
            })
            .unwrap_or(KeyAction::None);

        if matches!(action, KeyAction::None)
            && let Some(current_focus) = keyboard.current_focus()
            && let KeyboardFocusTarget::Window(window) = current_focus
            && let Some(window) = self.core.workspace_manager.active_workspace().find_window(|elem| elem.0 == window)
        {
            self.core.update_last_user_interaction(&window);
        }

        self.core.suppressed_keys = suppressed_keys;
        (action, serial)
    }

    pub(in crate::core) fn on_pointer_motion_relative(
        &mut self,
        delta: Point<f64, Logical>,
        delta_unaccel: Point<f64, Logical>,
        utime: u64,
    ) {
        if let Some(output_bbox) = self.output_bounding_box() {
            let pointer_location = self.core.pointer.current_location();
            let pointer = self.core.pointer.clone();
            let under = self.surface_under(pointer_location);
            let constraints = self.check_pointer_constraints(&pointer, pointer_location, &under);

            pointer.relative_motion(
                self,
                under.clone(),
                &RelativeMotionEvent {
                    delta,
                    delta_unaccel,
                    utime,
                },
            );

            let unclamped = pointer_location + delta;
            let clamped = self.clamp_to_outputs(unclamped, &output_bbox);

            let current_output_geo = self
                .core
                .workspace_manager
                .outputs()
                .find(|o| {
                    self.core
                        .workspace_manager
                        .output_geometry(o)
                        .is_some_and(|geo| geo.contains(pointer_location.to_i32_round()))
                })
                .and_then(|o| self.core.workspace_manager.output_geometry(o));

            let has_adjacent_output = self.core.workspace_manager.outputs().any(|o| {
                self.core
                    .workspace_manager
                    .output_geometry(o)
                    .is_some_and(|geo| geo.to_f64().contains(unclamped))
            });

            let edge_switching_allowed = if self.core.active_move_grab.is_some() {
                self.core.config.wrap_windows()
            } else {
                !self.core.pointer.is_grabbed() && self.core.config.wrap_workspaces()
            };

            let new_pos = if edge_switching_allowed
                && !has_adjacent_output
                && let Some(output_geo) = current_output_geo
                && let Some(edge) = self.core.edge_resistance.update(
                    unclamped,
                    clamped,
                    &output_geo,
                    (utime / 1000) as u32,
                    self.core.config.wrap_resistance(),
                ) {
                self.edge_switch_workspace(edge, clamped, &output_bbox)
            } else {
                clamped
            };

            self.apply_pointer_motion(&pointer, new_pos, (utime / 1000) as u32, &under, &constraints);
        }
    }

    pub(in crate::core) fn on_pointer_motion_absolute(&mut self, position: Point<f64, Logical>, time: u32) {
        if let Some(bbox) = self.output_bounding_box() {
            let old_pos = self.core.pointer.current_location();
            let new_pos = self.clamp_to_outputs(
                Point::<f64, Logical>::new(
                    position.x * bbox.size.w as f64 + bbox.loc.x as f64,
                    position.y * bbox.size.h as f64 + bbox.loc.y as f64,
                ),
                &bbox,
            );
            let pointer = self.core.pointer.clone();
            let old_under = self.surface_under(old_pos);
            let constraints = self.check_pointer_constraints(&pointer, old_pos, &old_under);

            self.apply_pointer_motion(&pointer, new_pos, time, &old_under, &constraints);
        }
    }

    pub(in crate::core) fn on_pointer_button(&mut self, button: u32, state: ButtonState, time: u32) {
        let serial = SERIAL_COUNTER.next_serial();

        let location = self.core.pointer.current_location();
        let surface_result = self.surface_under(location);
        debug!(
            "pointer_button: button={button} state={state:?} location={location:?} surface_under={surface_result:?}",
        );
        let (target, window) = surface_result
            .and_then(|(target, _)| self.window_for_pointer_focus_target(&target).map(|window| (target, window)))
            .unzip();

        let activate = self.core.config.click_to_focus();

        if state == ButtonState::Pressed {
            if let Some(window) = &window {
                if !window.has_modal_child() {
                    let do_raise = self.core.config.raise_on_click() && (button == BTN_LEFT || self.core.config.raise_with_any_button());

                    if do_raise {
                        self.raise_window(window, serial, activate);
                    } else {
                        self.activate_window(window, false, true, None);
                    }
                }
            } else if activate {
                self.update_keyboard_focus(self.core.pointer.current_location(), serial);
            }
        }

        let swallow_event = if state == ButtonState::Pressed
            && self.easy_key_pressed()
            && let Some(target) = target
            && let Some(window) = &window
        {
            if button == BTN_LEFT {
                let start_data = PointerGrabStartData {
                    focus: Some((target, location)),
                    button,
                    location,
                };
                self.start_maybe_window_move(
                    window.clone(),
                    self.core.seat.clone(),
                    serial,
                    GrabTrigger::Pointer,
                    Some(start_data),
                );
                true
            } else if button == BTN_RIGHT {
                let start_data = PointerGrabStartData {
                    focus: Some((target, location)),
                    button,
                    location,
                };

                let edges = self
                    .core
                    .workspace_manager
                    .active_workspace()
                    .window_geometry(window)
                    .map(|geom| {
                        let location = location.to_i32_round::<i32>() - geom.loc;
                        let corner_size = Size::<_, Logical>::from(((geom.size.w / 3).max(50), (geom.size.h / 3).max(50)));
                        let x_dist = geom.size.w / 2 - (geom.size.w / 2 - location.x).abs();
                        let y_dist = geom.size.h / 2 - (geom.size.h / 2 - location.y).abs();

                        if x_dist < corner_size.w && y_dist < corner_size.h {
                            if location.x < geom.size.w / 2 {
                                if location.y < geom.size.h / 2 {
                                    ResizeEdge::TOP_LEFT
                                } else {
                                    ResizeEdge::BOTTOM_LEFT
                                }
                            } else if location.y < geom.size.h / 2 {
                                ResizeEdge::TOP_RIGHT
                            } else {
                                ResizeEdge::BOTTOM_RIGHT
                            }
                        } else if x_dist / corner_size.w < y_dist / corner_size.h {
                            if location.x < geom.size.w / 2 {
                                ResizeEdge::LEFT
                            } else {
                                ResizeEdge::RIGHT
                            }
                        } else if location.y < geom.size.h / 2 {
                            ResizeEdge::TOP
                        } else {
                            ResizeEdge::BOTTOM
                        }
                    })
                    .unwrap_or(ResizeEdge::TOP);

                self.start_maybe_window_resize(
                    window.clone(),
                    self.core.seat.clone(),
                    serial,
                    edges,
                    GrabTrigger::Pointer,
                    Some(start_data),
                );

                true
            } else if button == BTN_MIDDLE {
                self.lower_window(window, serial, None);
                true
            } else {
                false
            }
        } else {
            false
        };

        if swallow_event {
            debug!("pointer_button: SWALLOWED button={button} state={state:?}");
        } else {
            let pointer = self.core.pointer.clone();
            let current_focus = pointer.current_focus();
            debug!("pointer_button: delivering button={button} state={state:?} to current_focus={current_focus:?} swallow_event=false");
            pointer.button(
                self,
                &ButtonEvent {
                    button,
                    state: wl_pointer::ButtonState::from(state).try_into().unwrap(),
                    serial,
                    time,
                },
            );
            pointer.frame(self);

            if state == ButtonState::Pressed
                && let Some(window) = window
            {
                self.core.update_last_user_interaction(&window);
            }
        }
    }

    fn easy_key_pressed(&mut self) -> bool {
        if let Some(keyboard) = self.core.seat.get_keyboard() {
            let easy_key = self.core.config.easy_click();
            let modifier_mask = keyboard.with_xkb_state(self, |ctx| {
                let xkb = ctx.xkb().lock().unwrap();
                // SAFETY: 'state' won't live longer than 'xkb'.
                let state = unsafe { xkb.state() };
                state.gdk_modifier_mask()
            }) & !ModifierType::LOCK_MASK;

            let result = modifier_mask == easy_key.modifier_mask();
            debug!("easy_key_pressed: modifier_mask={modifier_mask:?} easy_key={:?} easy_key_mask={:?} result={result}", easy_key, easy_key.modifier_mask());
            result
        } else {
            false
        }
    }

    pub(in crate::core) fn on_pointer_axis(&mut self, frame: AxisFrame) {
        let vertical_amount = frame.axis.1;
        if self.easy_key_pressed() {
            self.core.workspace_manager.reset_scroll_amount();

            if self.core.config.zoom_desktop()
                && let Some(output) = self
                    .core
                    .workspace_manager
                    .output_under(self.core.pointer.current_location())
                    .next()
                && let Some(zoom_state) = self.core.outputs_config.zoom_state_for_output_mut(output)
            {
                zoom_state.scrolled_for_zoom(vertical_amount);
            }
        } else {
            if let Some(output) = self
                .core
                .workspace_manager
                .output_under(self.core.pointer.current_location())
                .next()
                && let Some(zoom_state) = self.core.outputs_config.zoom_state_for_output_mut(output)
            {
                zoom_state.reset_scroll_amount();
            }

            if vertical_amount != 0.0
                && self.core.config.scroll_workspaces()
                && self
                    .core
                    .workspace_manager
                    .active_workspace()
                    .window_under(self.core.pointer.current_location())
                    .is_none()
            {
                self.scrolled_for_workspace_switch(vertical_amount);
            } else {
                self.core.workspace_manager.reset_scroll_amount();
            }

            let pointer = self.core.pointer.clone();
            pointer.axis(self, frame);
            pointer.frame(self);
        }
    }

    pub(in crate::core) fn on_gesture_swipe_begin(&mut self, time: u32, fingers: u32) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.core.pointer.clone();
        pointer.gesture_swipe_begin(self, &GestureSwipeBeginEvent { serial, time, fingers });
    }

    pub(in crate::core) fn on_gesture_swipe_update(&mut self, time: u32, delta: Point<f64, Logical>) {
        let pointer = self.core.pointer.clone();
        pointer.gesture_swipe_update(self, &GestureSwipeUpdateEvent { time, delta });
    }

    pub(in crate::core) fn on_gesture_swipe_end(&mut self, time: u32, cancelled: bool) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.core.pointer.clone();
        pointer.gesture_swipe_end(self, &GestureSwipeEndEvent { serial, time, cancelled });
    }

    pub(in crate::core) fn on_gesture_pinch_begin(&mut self, time: u32, fingers: u32) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.core.pointer.clone();
        pointer.gesture_pinch_begin(self, &GesturePinchBeginEvent { serial, time, fingers });
    }

    pub(in crate::core) fn on_gesture_pinch_update(&mut self, time: u32, delta: Point<f64, Logical>, scale: f64, rotation: f64) {
        let pointer = self.core.pointer.clone();
        pointer.gesture_pinch_update(
            self,
            &GesturePinchUpdateEvent {
                time,
                delta,
                scale,
                rotation,
            },
        );
    }

    pub(in crate::core) fn on_gesture_pinch_end(&mut self, time: u32, cancelled: bool) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.core.pointer.clone();
        pointer.gesture_pinch_end(self, &GesturePinchEndEvent { serial, time, cancelled });
    }

    pub(in crate::core) fn on_gesture_hold_begin(&mut self, time: u32, fingers: u32) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.core.pointer.clone();
        pointer.gesture_hold_begin(self, &GestureHoldBeginEvent { serial, time, fingers });
    }

    pub(in crate::core) fn on_gesture_hold_end(&mut self, time: u32, cancelled: bool) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.core.pointer.clone();
        pointer.gesture_hold_end(self, &GestureHoldEndEvent { serial, time, cancelled });
    }

    pub(in crate::core) fn on_touch_down(&mut self, slot: TouchSlot, position: Point<f64, Logical>, time: u32) {
        let Some(handle) = self.core.seat.get_touch() else {
            return;
        };
        let Some(touch_location) = self.touch_location_from_normalized(position) else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
        self.update_keyboard_focus(touch_location, serial);
        let under = self.surface_under(touch_location);
        handle.down(
            self,
            under.clone(),
            &DownEvent {
                slot,
                location: touch_location,
                serial,
                time,
            },
        );

        if let Some((focus, _)) = under
            && let Some(window) = match focus {
                PointerFocusTarget::WlSurface(surface) => self.window_for_surface(&surface),
                PointerFocusTarget::X11Surface(surface) => self
                    .core
                    .workspace_manager
                    .find_window(|elem| elem.0.x11_surface().is_some_and(|surf| surf == &surface)),
                PointerFocusTarget::SSD(SSD(window)) => Some(window),
            }
        {
            self.core.update_last_user_interaction(&window);
        }
    }

    pub(in crate::core) fn on_touch_up(&mut self, slot: TouchSlot, time: u32) {
        let Some(handle) = self.core.seat.get_touch() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
        handle.up(self, &UpEvent { slot, serial, time })
    }

    pub(in crate::core) fn on_touch_motion(&mut self, slot: TouchSlot, position: Point<f64, Logical>, time: u32) {
        let Some(handle) = self.core.seat.get_touch() else {
            return;
        };
        let Some(touch_location) = self.touch_location_from_normalized(position) else {
            return;
        };
        let under = self.surface_under(touch_location);
        handle.motion(
            self,
            under,
            &smithay::input::touch::MotionEvent {
                slot,
                location: touch_location,
                time,
            },
        );
    }

    pub(in crate::core) fn on_touch_frame(&mut self) {
        let Some(handle) = self.core.seat.get_touch() else {
            return;
        };
        handle.frame(self);
    }

    pub(in crate::core) fn on_touch_cancel(&mut self) {
        let Some(handle) = self.core.seat.get_touch() else {
            return;
        };
        handle.cancel(self);
    }

    pub(in crate::core) fn on_tablet_tool_proximity(&mut self, data: TabletToolProximityData) {
        let TabletToolProximityData {
            descriptor,
            tablet,
            state,
            position,
            time,
        } = data;
        let dh = self.core.display_handle.clone();
        let tablet_seat = self.core.seat.tablet_seat();

        if let Some(pointer_location) = self.touch_location_from_normalized(position) {
            tablet_seat.add_tool::<Self>(self, &dh, &descriptor);

            let pointer = self.core.pointer.clone();
            let under = self.surface_under(pointer_location);
            let tablet_seat = self.core.seat.tablet_seat();
            let tablet_handle = tablet_seat.get_tablet(&tablet);
            let tool_handle = tablet_seat.get_tool(&descriptor);

            pointer.motion(
                self,
                under.clone(),
                &MotionEvent {
                    location: pointer_location,
                    serial: SERIAL_COUNTER.next_serial(),
                    time,
                },
            );
            pointer.frame(self);

            if let (Some(under), Some(tablet_handle), Some(tool_handle)) = (
                under.and_then(|(f, loc)| f.wl_surface().map(|s| (s.into_owned(), loc))),
                tablet_handle,
                tool_handle,
            ) {
                match state {
                    ProximityState::In => {
                        tool_handle.proximity_in(pointer_location, under, &tablet_handle, SERIAL_COUNTER.next_serial(), time)
                    }
                    ProximityState::Out => tool_handle.proximity_out(time),
                }
            }
        }
    }

    pub(in crate::core) fn on_tablet_tool_axis(&mut self, data: TabletToolAxisData) {
        let TabletToolAxisData {
            descriptor,
            tablet,
            position,
            pressure,
            distance,
            tilt,
            slider,
            rotation,
            wheel,
            time,
        } = data;
        let tablet_seat = self.core.seat.tablet_seat();

        if let Some(pointer_location) = self.touch_location_from_normalized(position) {
            let pointer = self.core.pointer.clone();
            let under = self.surface_under(pointer_location);
            let tablet_handle = tablet_seat.get_tablet(&tablet);
            let tool_handle = tablet_seat.get_tool(&descriptor);

            pointer.motion(
                self,
                under.clone(),
                &MotionEvent {
                    location: pointer_location,
                    serial: SERIAL_COUNTER.next_serial(),
                    time,
                },
            );

            if let (Some(tablet_handle), Some(tool_handle)) = (tablet_handle, tool_handle) {
                if let Some(pressure) = pressure {
                    tool_handle.pressure(pressure);
                }
                if let Some(distance) = distance {
                    tool_handle.distance(distance);
                }
                if let Some(tilt) = tilt {
                    tool_handle.tilt(tilt);
                }
                if let Some(slider) = slider {
                    tool_handle.slider_position(slider);
                }
                if let Some(rotation) = rotation {
                    tool_handle.rotation(rotation);
                }
                if let Some((delta, delta_discrete)) = wheel {
                    tool_handle.wheel(delta, delta_discrete);
                }

                tool_handle.motion(
                    pointer_location,
                    under.and_then(|(f, loc)| f.wl_surface().map(|s| (s.into_owned(), loc))),
                    &tablet_handle,
                    SERIAL_COUNTER.next_serial(),
                    time,
                );
            }

            pointer.frame(self);
        }
    }

    pub(in crate::core) fn on_tablet_tool_tip(&mut self, data: TabletToolTipData) {
        let TabletToolTipData {
            descriptor,
            position: _,
            tip_state,
            time,
        } = data;
        let tool_handle = self.core.seat.tablet_seat().get_tool(&descriptor);

        if let Some(tool_handle) = tool_handle {
            match tip_state {
                TabletToolTipState::Down => {
                    let serial = SERIAL_COUNTER.next_serial();
                    tool_handle.tip_down(serial, time);
                    // change the keyboard focus
                    self.update_keyboard_focus(self.core.pointer.current_location(), serial);
                }
                TabletToolTipState::Up => {
                    tool_handle.tip_up(time);
                }
            }
        }
    }

    pub(in crate::core) fn on_tablet_tool_button(&mut self, data: TabletToolButtonData) {
        let TabletToolButtonData {
            descriptor,
            button,
            state,
            time,
        } = data;
        let tool_handle = self.core.seat.tablet_seat().get_tool(&descriptor);

        if let Some(tool_handle) = tool_handle {
            tool_handle.button(button, state, SERIAL_COUNTER.next_serial(), time);
        }
    }

    pub(in crate::core) fn on_switch(&mut self, switch: Switch, state: SwitchState) {
        if switch == Switch::Lid {
            self.core.laptop_lid_state = Some(match state {
                SwitchState::On => LaptopLidState::Closed,
                SwitchState::Off => LaptopLidState::Open,
            });
        }
    }

    pub(in crate::core) fn on_device_added(&mut self, caps: DeviceCapabilities) {
        if caps.has_keyboard
            && let Some(led_state) = self.core.seat.get_keyboard().map(|keyboard| keyboard.led_state())
        {
            self.backend.update_led_state(led_state);
        }
        if caps.has_touch && self.core.seat.get_touch().is_none() {
            self.core.seat.add_touch();
        }
        if let Some(tablet_descriptor) = caps.tablet_descriptor {
            self.core
                .seat
                .tablet_seat()
                .add_tablet::<Self>(&self.core.display_handle.clone(), &tablet_descriptor);
        }
    }

    pub(in crate::core) fn on_device_removed(&mut self, caps: DeviceCapabilities) {
        if let Some(tablet_descriptor) = caps.tablet_descriptor {
            let tablet_seat = self.core.seat.tablet_seat();
            tablet_seat.remove_tablet(&tablet_descriptor);
            // If there are no tablets in seat we can remove all tools
            if tablet_seat.count_tablets() == 0 {
                tablet_seat.clear_tools();
            }
        }
    }

    pub(crate) fn dispatch_translated_input(&mut self, input: TranslatedInput) -> KeyAction {
        match &input {
            TranslatedInput::DeviceAdded(_) | TranslatedInput::DeviceRemoved(_) => {}
            _ => self.core.notify_activity(&self.core.seat.clone()),
        }

        match input {
            TranslatedInput::Keyboard(KeyboardInputEvent::Key { keycode, state, time }) => {
                let (action, serial) = self.on_keyboard_key(keycode, state, time);
                self.process_common_key_action(action, serial);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::MotionRelative {
                delta,
                delta_unaccel,
                utime,
            }) => {
                self.on_pointer_motion_relative(delta, delta_unaccel, utime);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::MotionAbsolute { position, time }) => {
                self.on_pointer_motion_absolute(position, time);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::Button { button, state, time }) => {
                self.on_pointer_button(button, state, time);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::Axis { frame }) => {
                self.on_pointer_axis(frame);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GestureSwipeBegin { time, fingers }) => {
                self.on_gesture_swipe_begin(time, fingers);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GestureSwipeUpdate { time, delta }) => {
                self.on_gesture_swipe_update(time, delta);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GestureSwipeEnd { time, cancelled }) => {
                self.on_gesture_swipe_end(time, cancelled);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GesturePinchBegin { time, fingers }) => {
                self.on_gesture_pinch_begin(time, fingers);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GesturePinchUpdate {
                time,
                delta,
                scale,
                rotation,
            }) => {
                self.on_gesture_pinch_update(time, delta, scale, rotation);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GesturePinchEnd { time, cancelled }) => {
                self.on_gesture_pinch_end(time, cancelled);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GestureHoldBegin { time, fingers }) => {
                self.on_gesture_hold_begin(time, fingers);
                KeyAction::None
            }
            TranslatedInput::Pointer(PointerInputEvent::GestureHoldEnd { time, cancelled }) => {
                self.on_gesture_hold_end(time, cancelled);
                KeyAction::None
            }
            TranslatedInput::Touch(TouchInputEvent::Down { slot, position, time }) => {
                self.on_touch_down(slot, position, time);
                KeyAction::None
            }
            TranslatedInput::Touch(TouchInputEvent::Up { slot, time }) => {
                self.on_touch_up(slot, time);
                KeyAction::None
            }
            TranslatedInput::Touch(TouchInputEvent::Motion { slot, position, time }) => {
                self.on_touch_motion(slot, position, time);
                KeyAction::None
            }
            TranslatedInput::Touch(TouchInputEvent::Frame) => {
                self.on_touch_frame();
                KeyAction::None
            }
            TranslatedInput::Touch(TouchInputEvent::Cancel) => {
                self.on_touch_cancel();
                KeyAction::None
            }
            TranslatedInput::Tablet(TabletInputEvent::ToolProximity(data)) => {
                self.on_tablet_tool_proximity(data);
                KeyAction::None
            }
            TranslatedInput::Tablet(TabletInputEvent::ToolAxis(data)) => {
                self.on_tablet_tool_axis(data);
                KeyAction::None
            }
            TranslatedInput::Tablet(TabletInputEvent::ToolTip(data)) => {
                self.on_tablet_tool_tip(data);
                KeyAction::None
            }
            TranslatedInput::Tablet(TabletInputEvent::ToolButton(data)) => {
                self.on_tablet_tool_button(data);
                KeyAction::None
            }
            TranslatedInput::Switch(SwitchInputEvent { switch, state }) => {
                self.on_switch(switch, state);
                KeyAction::None
            }
            TranslatedInput::DeviceAdded(caps) => {
                self.on_device_added(caps);
                KeyAction::None
            }
            TranslatedInput::DeviceRemoved(caps) => {
                self.on_device_removed(caps);
                KeyAction::None
            }
        }
    }

    pub(in crate::core) fn update_keyboard_focus(&mut self, location: Point<f64, Logical>, serial: Serial) {
        let keyboard = self.core.seat.get_keyboard().unwrap();
        let touch = self.core.seat.get_touch();
        let input_method = self.core.seat.input_method();
        // change the keyboard focus unless the pointer or keyboard is grabbed
        // We test for any matching surface type here but always use the root
        // (in case of a window the toplevel) surface for the focus.
        // So for example if a user clicks on a subsurface or popup the toplevel
        // will receive the keyboard focus. Directly assigning the focus to the
        // matching surface leads to issues with clients dismissing popups and
        // subsurface menus (for example firefox-wayland).
        // see here for a discussion about that issue:
        // https://gitlab.freedesktop.org/wayland/wayland/-/issues/294
        if !self.core.pointer.is_grabbed()
            && (!keyboard.is_grabbed() || input_method.keyboard_grabbed())
            && !touch.map(|touch| touch.is_grabbed()).unwrap_or(false)
        {
            let output = self.core.workspace_manager.output_under(location).next().cloned();
            if let Some(output) = output.as_ref() {
                let output_geo = self.core.workspace_manager.output_geometry(output).unwrap();
                if let Some(window) = self.core.workspace_manager.active_workspace().fullscreen_window_for_output(output)
                    && let Some((_, _)) = window.surface_under(location - output_geo.loc.to_f64(), WindowSurfaceType::ALL)
                {
                    #[cfg(feature = "xwayland")]
                    if self.core.config.raise_on_focus()
                        && let Some(surface) = window.0.x11_surface()
                        && let Some(xwm) = self.core.xwayland.as_mut().map(|xw| &mut xw.xwm)
                    {
                        let _ = xwm.raise_window(surface);
                    }
                    self.focus_window(&window, serial, None);
                    return;
                }

                let layers = layer_map_for_output(output);

                if let Some(layer) = [WlrLayer::Overlay, WlrLayer::Top, WlrLayer::Bottom, WlrLayer::Background]
                    .into_iter()
                    .find_map(|wlr_layer| {
                        let layer = layers.layer_under(wlr_layer, location - output_geo.loc.to_f64())?;
                        let layer_loc = layers.layer_geometry(layer).unwrap().loc;
                        layer
                            .surface_under(location - output_geo.loc.to_f64() - layer_loc.to_f64(), WindowSurfaceType::POPUP)
                            .map(|_| layer)
                    })
                {
                    self.focus_target(layer.clone(), serial, None);
                    return;
                }

                if let Some(layer) = layers
                    .layer_under(WlrLayer::Overlay, location - output_geo.loc.to_f64())
                    .or_else(|| layers.layer_under(WlrLayer::Top, location - output_geo.loc.to_f64()))
                    && layer.can_receive_keyboard_focus()
                    && let Some((_, _)) = layer.surface_under(
                        location - output_geo.loc.to_f64() - layers.layer_geometry(layer).unwrap().loc.to_f64(),
                        WindowSurfaceType::TOPLEVEL,
                    )
                {
                    self.focus_target(layer.clone(), serial, None);
                    return;
                }
            }

            let workspace = self.core.workspace_manager.active_workspace_mut();
            if let Some((window, _)) = workspace.window_under(location).map(|(w, p)| (w.clone(), p)) {
                if self.core.config.raise_on_focus() {
                    workspace.raise_window(&window, true);
                } else {
                    workspace.activate_window(&window);
                }
                #[cfg(feature = "xwayland")]
                if self.core.config.raise_on_focus()
                    && let Some(surface) = window.0.x11_surface()
                    && let Some(xwm) = self.core.xwayland.as_mut().map(|xw| &mut xw.xwm)
                {
                    let _ = xwm.raise_window(surface);
                }
                self.focus_window(&window, serial, None);
                return;
            }

            if let Some(output) = output.as_ref() {
                let output_geo = self.core.workspace_manager.output_geometry(output).unwrap();
                let layers = layer_map_for_output(output);
                if let Some(layer) = layers
                    .layer_under(WlrLayer::Bottom, location - output_geo.loc.to_f64())
                    .or_else(|| layers.layer_under(WlrLayer::Background, location - output_geo.loc.to_f64()))
                    && layer.can_receive_keyboard_focus()
                    && let Some((_, _)) = layer.surface_under(
                        location - output_geo.loc.to_f64() - layers.layer_geometry(layer).unwrap().loc.to_f64(),
                        WindowSurfaceType::TOPLEVEL,
                    )
                {
                    self.focus_target(layer.clone(), serial, None);
                }
            };
        }
    }

    pub(in crate::core) fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(PointerFocusTarget, Point<f64, Logical>)> {
        self.surface_under_for_workspace(pos, self.core.workspace_manager.active_workspace_index())
    }

    pub(in crate::core) fn surface_under_for_workspace(
        &self,
        pos: Point<f64, Logical>,
        workspace_num: u32,
    ) -> Option<(PointerFocusTarget, Point<f64, Logical>)> {
        if let Some(workspace) = self.core.workspace_manager.workspaces().get(workspace_num as usize) {
            let output = self.core.workspace_manager.outputs().find(|o| {
                let geometry = self.core.workspace_manager.output_geometry(o).unwrap();
                geometry.contains(pos.to_i32_round())
            })?;
            let output_geo = self.core.workspace_manager.output_geometry(output).unwrap();
            let layers = layer_map_for_output(output);

            let mut under = None;
            if let Some((surface, loc)) = workspace
                .fullscreen_window_for_output(output)
                .and_then(|w| w.surface_under(pos - output_geo.loc.to_f64(), WindowSurfaceType::ALL))
            {
                under = Some((surface, loc + output_geo.loc));
            } else if let Some(focus) = [WlrLayer::Overlay, WlrLayer::Top, WlrLayer::Bottom, WlrLayer::Background]
                .into_iter()
                .find_map(|wlr_layer| {
                    let layer = layers.layer_under(wlr_layer, pos - output_geo.loc.to_f64())?;
                    let layer_loc = layers.layer_geometry(layer).unwrap().loc;
                    layer
                        .surface_under(pos - output_geo.loc.to_f64() - layer_loc.to_f64(), WindowSurfaceType::POPUP)
                        .map(|(surface, loc)| (PointerFocusTarget::from(surface), loc + layer_loc + output_geo.loc))
                })
            {
                under = Some(focus)
            } else if let Some(focus) = layers
                .layer_under(WlrLayer::Overlay, pos - output_geo.loc.to_f64())
                .or_else(|| layers.layer_under(WlrLayer::Top, pos - output_geo.loc.to_f64()))
                .and_then(|layer| {
                    let layer_loc = layers.layer_geometry(layer).unwrap().loc;
                    layer
                        .surface_under(pos - output_geo.loc.to_f64() - layer_loc.to_f64(), WindowSurfaceType::TOPLEVEL)
                        .map(|(surface, loc)| (PointerFocusTarget::from(surface), loc + layer_loc + output_geo.loc))
                })
            {
                under = Some(focus)
            } else if let Some(focus) = workspace.window_under(pos).and_then(|(window, loc)| {
                let local = pos - loc.to_f64();
                let surf = window.surface_under(local, WindowSurfaceType::ALL);
                debug!("window_under: pos={pos:?} render_loc={loc:?} local={local:?} surface={:?}", surf.as_ref().map(|(s, l)| (format!("{s:?}"), l)));
                surf.map(|(surface, surf_loc)| (surface, surf_loc + loc))
            }) {
                under = Some(focus);
            } else if let Some(focus) = layers
                .layer_under(WlrLayer::Bottom, pos - output_geo.loc.to_f64())
                .or_else(|| layers.layer_under(WlrLayer::Background, pos - output_geo.loc.to_f64()))
                .and_then(|layer| {
                    let layer_loc = layers.layer_geometry(layer).unwrap().loc;
                    layer
                        .surface_under(pos - output_geo.loc.to_f64() - layer_loc.to_f64(), WindowSurfaceType::TOPLEVEL)
                        .map(|(surface, loc)| (PointerFocusTarget::from(surface), loc + layer_loc + output_geo.loc))
                })
            {
                under = Some(focus)
            };
            under.map(|(s, l)| (s, l.to_f64()))
        } else {
            None
        }
    }

    pub(in crate::core) fn output_under_pointer(&self) -> Option<smithay::output::Output> {
        let pos = self.core.pointer.current_location().to_i32_round();
        self.core
            .workspace_manager
            .outputs()
            .find(|o| self.core.workspace_manager.output_geometry(o).unwrap().contains(pos))
            .cloned()
    }

    pub(crate) fn release_all_keys(&mut self) {
        let keyboard = self.core.seat.get_keyboard().unwrap();
        for keycode in keyboard.pressed_keys() {
            keyboard.input(self, keycode, KeyState::Released, SERIAL_COUNTER.next_serial(), 0, |_, _, _| {
                FilterResult::Forward::<bool>
            });
        }
    }

    fn touch_location_from_normalized(&self, position: Point<f64, Logical>) -> Option<Point<f64, Logical>> {
        let output = self
            .core
            .workspace_manager
            .outputs()
            .find(|output| output.name().starts_with("eDP"))
            .or_else(|| self.core.workspace_manager.outputs().next())?;
        let output_geometry = self.core.workspace_manager.output_geometry(output)?;
        let transform = output.current_transform();
        let size = transform.invert().transform_size(output_geometry.size);
        let scaled = Point::<f64, Logical>::from((position.x * size.w as f64, position.y * size.h as f64));
        Some(transform.transform_point_in(scaled, &size.to_f64()) + output_geometry.loc.to_f64())
    }

    fn check_pointer_constraints(
        &self,
        pointer: &PointerHandle<Xfwl4State<BackendData>>,
        pointer_location: Point<f64, Logical>,
        under: &Option<(PointerFocusTarget, Point<f64, Logical>)>,
    ) -> PointerConstraintState {
        if let Some((surface, surface_loc)) = under.as_ref().and_then(|(target, l)| Some((target.wl_surface()?, l))) {
            with_pointer_constraint(&surface, pointer, |constraint| match constraint {
                Some(constraint)
                    if constraint.is_active()
                        && constraint
                            .region()
                            .is_none_or(|x| x.contains((pointer_location - *surface_loc).to_i32_round())) =>
                {
                    match &*constraint {
                        PointerConstraint::Locked(_) => PointerConstraintState {
                            locked: true,
                            ..Default::default()
                        },
                        PointerConstraint::Confined(confine) => PointerConstraintState {
                            confined: true,
                            confine_region: confine.region().cloned(),
                            ..Default::default()
                        },
                    }
                }
                _ => PointerConstraintState::default(),
            })
        } else {
            PointerConstraintState::default()
        }
    }

    fn pointer_leaves_confinement(
        &self,
        new_location: Point<f64, Logical>,
        old_under: &Option<(PointerFocusTarget, Point<f64, Logical>)>,
        new_under: &Option<(PointerFocusTarget, Point<f64, Logical>)>,
        confine_region: &Option<RegionAttributes>,
    ) -> bool {
        if let Some((surface, surface_loc)) = old_under {
            if new_under.as_ref().and_then(|(under, _)| under.wl_surface()) != surface.wl_surface() {
                true
            } else if let Some(region) = confine_region {
                !region.contains((new_location - *surface_loc).to_i32_round())
            } else {
                false
            }
        } else {
            false
        }
    }

    fn apply_pointer_motion(
        &mut self,
        pointer: &PointerHandle<Xfwl4State<BackendData>>,
        new_pos: Point<f64, Logical>,
        time: u32,
        old_under: &Option<(PointerFocusTarget, Point<f64, Logical>)>,
        constraints: &PointerConstraintState,
    ) {
        if constraints.locked {
            pointer.frame(self);
        } else {
            let new_under = self.surface_under(new_pos);

            if constraints.confined && self.pointer_leaves_confinement(new_pos, old_under, &new_under, &constraints.confine_region) {
                pointer.frame(self);
            } else {
                pointer.motion(
                    self,
                    new_under.clone(),
                    &MotionEvent {
                        location: new_pos,
                        serial: SERIAL_COUNTER.next_serial(),
                        time,
                    },
                );
                pointer.frame(self);

                let pointer_window = new_under
                    .as_ref()
                    .and_then(|(target, _)| self.window_for_pointer_focus_target(target));
                if pointer_window != self.core.pointer_window {
                    self.core.cancel_focus_follows_mouse_timers();

                    let do_activate = |state: &mut Self, pointer_window: WindowElement| {
                        let raise_on_focus = state.core.config.raise_on_focus();
                        let raise_delay = state.core.config.raise_delay();
                        let raise_now = raise_on_focus && raise_delay <= 0;

                        state.activate_window(&pointer_window, raise_now, false, None);

                        if raise_on_focus && raise_delay > 0 {
                            state.core.raise_timeout = state
                                .core
                                .handle
                                .insert_source(
                                    Timer::from_duration(Duration::from_millis(raise_delay as u64)),
                                    move |_, _, state| {
                                        if !state.core.pointer.is_grabbed() {
                                            state.raise_window(&pointer_window, SERIAL_COUNTER.next_serial(), true);
                                        }
                                        TimeoutAction::Drop
                                    },
                                )
                                .ok();
                        }
                    };

                    if let Some(pointer_window) = &pointer_window
                        && !self.core.config.click_to_focus()
                        && !pointer_window.active()
                    {
                        let focus_delay = self.core.config.focus_delay();
                        if focus_delay > 0 {
                            let pointer_window = pointer_window.clone();
                            self.core.focus_timeout = self
                                .core
                                .handle
                                .insert_source(
                                    Timer::from_duration(Duration::from_millis(focus_delay as u64)),
                                    move |_, _, state| {
                                        if !state.core.pointer.is_grabbed() {
                                            do_activate(state, pointer_window.clone());
                                        }
                                        TimeoutAction::Drop
                                    },
                                )
                                .ok();
                        } else if !self.core.pointer.is_grabbed() {
                            do_activate(self, pointer_window.clone());
                        }
                    }

                    self.core.pointer_window = pointer_window;
                }

                self.try_activate_pointer_constraint(pointer, new_pos, new_under);
            }
        }
    }

    fn try_activate_pointer_constraint(
        &self,
        pointer: &PointerHandle<Xfwl4State<BackendData>>,
        pointer_location: Point<f64, Logical>,
        under: Option<(PointerFocusTarget, Point<f64, Logical>)>,
    ) {
        if let Some((under, surface_location)) = under.and_then(|(target, loc)| Some((target.wl_surface()?.into_owned(), loc))) {
            with_pointer_constraint(&under, pointer, |constraint| match constraint {
                Some(constraint) if !constraint.is_active() => {
                    let point = (pointer_location - surface_location).to_i32_round();
                    if constraint.region().is_none_or(|region| region.contains(point)) {
                        constraint.activate();
                    }
                }
                _ => {}
            });
        }
    }

    fn edge_switch_workspace(&mut self, edge: ScreenEdge, pos: Point<f64, Logical>, bbox: &Rectangle<i32, Logical>) -> Point<f64, Logical> {
        let direction = match edge {
            ScreenEdge::Left => Direction::Left,
            ScreenEdge::Right => Direction::Right,
            ScreenEdge::Top => Direction::Up,
            ScreenEdge::Bottom => Direction::Down,
        };

        if let Some(new_index) = self
            .core
            .workspace_manager
            .workspace_index_for_direction(direction, self.core.config.wrap_layout())
        {
            let bbox_f64 = bbox.to_f64();
            let warped: Point<f64, Logical> = match edge {
                ScreenEdge::Left => (bbox_f64.loc.x + bbox_f64.size.w - 2.0, pos.y).into(),
                ScreenEdge::Right => (bbox_f64.loc.x + 1.0, pos.y).into(),
                ScreenEdge::Top => (pos.x, bbox_f64.loc.y + bbox_f64.size.h - 2.0).into(),
                ScreenEdge::Bottom => (pos.x, bbox_f64.loc.y + 1.0).into(),
            };

            let move_window = self.core.active_move_grab.as_ref().map(|g| g.window());
            if let Some(window) = move_window
                && let Some((old_index, _)) = self.core.workspace_manager.workspace_for_window_with_index(&window)
                && old_index != new_index
                && let Some(current_loc) = self
                    .core
                    .wireframe
                    .as_ref()
                    .map(|wf| wf.geometry().loc)
                    .or_else(|| self.core.workspace_manager.active_workspace().window_location(&window))
                && self.move_window_to_workspace_old_new_index(&window, old_index, new_index)
            {
                let new_loc = current_loc + (warped - pos).to_i32_round();
                if let Some(wireframe) = self.core.wireframe.as_mut() {
                    wireframe.update_location(new_loc);
                } else {
                    self.core.workspace_manager.relocate_window(&window, new_loc, true);
                }
                if let Some(move_grab) = self.core.active_move_grab.as_ref() {
                    move_grab.reset_location_after_warp(warped, new_loc);
                }
            }

            self.set_active_workspace(new_index);

            warped
        } else {
            pos
        }
    }

    fn output_bounding_box(&self) -> Option<Rectangle<i32, Logical>> {
        self.core
            .workspace_manager
            .outputs()
            .filter_map(|o| self.core.workspace_manager.output_geometry(o))
            .reduce(|acc, geo| acc.merge(geo))
    }

    fn clamp_to_outputs(&self, pos: Point<f64, Logical>, bbox: &Rectangle<i32, Logical>) -> Point<f64, Logical> {
        let bbox_f64 = bbox.to_f64();
        let clamped_x = pos.x.clamp(bbox_f64.loc.x, bbox_f64.loc.x + bbox_f64.size.w - 1.0);
        let clamped_y = pos.y.clamp(bbox_f64.loc.y, bbox_f64.loc.y + bbox_f64.size.h - 1.0);

        // Find the nearest output to snap the pointer onto, avoiding gaps between outputs.
        let nearest_output_geo = self
            .core
            .workspace_manager
            .outputs()
            .filter_map(|o| self.core.workspace_manager.output_geometry(o))
            .min_by_key(|geo| {
                let dx = if clamped_x < geo.loc.x as f64 {
                    (geo.loc.x as f64 - clamped_x) as i64
                } else if clamped_x >= (geo.loc.x + geo.size.w) as f64 {
                    (clamped_x - (geo.loc.x + geo.size.w - 1) as f64) as i64
                } else {
                    0
                };
                let dy = if clamped_y < geo.loc.y as f64 {
                    (geo.loc.y as f64 - clamped_y) as i64
                } else if clamped_y >= (geo.loc.y + geo.size.h) as f64 {
                    (clamped_y - (geo.loc.y + geo.size.h - 1) as f64) as i64
                } else {
                    0
                };
                dx * dx + dy * dy
            });

        if let Some(geo) = nearest_output_geo {
            let geo_f64 = geo.to_f64();
            (
                clamped_x.clamp(geo_f64.loc.x, geo_f64.loc.x + geo_f64.size.w - 1.0),
                clamped_y.clamp(geo_f64.loc.y, geo_f64.loc.y + geo_f64.size.h - 1.0),
            )
                .into()
        } else {
            (clamped_x, clamped_y).into()
        }
    }

    fn process_keyboard_shortcut(&self, modifier_mask: ModifierType, keysym: Keysym) -> Option<KeyAction> {
        // We ignore some modifiers when matching shortcuts.
        let modifier_mask = modifier_mask & !(ModifierType::LOCK_MASK | ModifierType::MOD4_MASK);

        if modifier_mask == (ModifierType::CONTROL_MASK | ModifierType::MOD1_MASK) && keysym == Keysym::BackSpace {
            Some(KeyAction::Quit)
        } else if (xkb::KEY_XF86Switch_VT_1..=xkb::KEY_XF86Switch_VT_12).contains(&keysym.raw()) {
            Some(KeyAction::VtSwitch((keysym.raw() - xkb::KEY_XF86Switch_VT_1 + 1) as i32))
        } else if !self.core.cycling_windows {
            let key = ShortcutKey::new(keysym, modifier_mask);

            #[allow(clippy::manual_map)]
            if let Some(action) = self.core.wm_shortcuts.find(&key) {
                match action {
                    WmShortcutAction::Up
                    | WmShortcutAction::Down
                    | WmShortcutAction::Left
                    | WmShortcutAction::Right
                    | WmShortcutAction::Cancel => {
                        // These actions are only handled if the compositor is in a
                        // particular state, like the tabwin is up, or a
                        // keyboard-interactive resize or move is active.  Otherwise,
                        // these keys need to be passed to the focused client.
                        None
                    }
                    _ => Some(KeyAction::WmAction(action)),
                }
            } else if let Some(command) = self.core.command_shortcuts.find(&key) {
                Some(KeyAction::Run(command.argv0.clone(), command.args.clone()))
            } else {
                None
            }
        } else {
            // We don't handle any other keybindings when cycling windows; the tabwin itself
            // handles all events.
            None
        }
    }
}

impl<BackendData: Backend + 'static> Xfwl4Core<BackendData> {
    fn shortcut_for_wm_action(&self, action: WmShortcutAction) -> Option<ShortcutKey> {
        self.wm_shortcuts.find_by_action(&action)
    }
}

/// Possible results of a keyboard action
#[derive(Debug)]
pub enum KeyAction {
    Quit,
    VtSwitch(i32),
    Run(OsString, Vec<OsString>),
    WmAction(WmShortcutAction),
    None,
}
