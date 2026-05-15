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

use std::collections::VecDeque;

use smithay::{
    desktop::{WindowSurface, layer_map_for_output, space::SpaceElement},
    input::Seat,
    output::Output,
    reexports::{wayland_protocols::xdg::shell::server::xdg_toplevel, wayland_server::Resource},
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Serial},
};

use crate::{
    backend::Backend,
    core::{
        config::ActivateAction,
        focus::KeyboardFocusTarget,
        shell::{
            TileMode, WindowElement, WindowFlags, WindowLayout, WindowState, WorkspaceLocation, output_and_geom_for_anchored_layout,
            remove_all_layout_states, remove_tiled_states, xdg::XdgSurfaceProps,
        },
        state::Xfwl4State,
        util::Direction,
    },
};

mod manager;
mod workspace;

pub use manager::{WindowStackingLayer, WorkspaceManager};
pub use workspace::Workspace;

impl<BackendData: Backend + 'static> Xfwl4State<BackendData> {
    pub(in crate::core) fn set_active_workspace(&mut self, workspace_number: u32) {
        // Annoying, need to do this first before 'self' gets borrowed mutably below.
        let window_under_pointer = self
            .surface_under_for_workspace(self.core.pointer.current_location(), workspace_number)
            .and_then(|(target, _)| self.window_for_pointer_focus_target(&target));

        let changed = if let Some((prev_workspace, new_workspace)) = self.core.workspace_manager.set_active_workspace(workspace_number) {
            let new_active_window = if self.core.config.click_to_focus() {
                new_workspace.active_window().cloned()
            } else {
                window_under_pointer.clone()
            }
            .or_else(|| new_workspace.visible_windows().last().cloned());

            if let Some(prev_workspace) = prev_workspace
                && let Some(prev_active_window) = prev_workspace.active_window().cloned()
            {
                self.core.toplevel_changed(
                    &prev_active_window,
                    None,
                    None,
                    WindowState::empty(),
                    WindowState::ACTIVATED,
                    Vec::new(),
                    Vec::new(),
                    None,
                );
            }

            if let Some(active_window) = new_active_window {
                self.activate_window(&active_window, true, true, None);
            }

            self.core.pointer_window = window_under_pointer;
            true
        } else {
            false
        };

        if changed {
            #[cfg(feature = "xwayland")]
            {
                self.x11_update_active_workspace(workspace_number);
                self.x11_update_window_stacking_order();
            }
            self.core.cancel_focus_follows_mouse_timers();
        }
    }

    pub(in crate::core) fn toggle_active_workspace(&mut self, workspace_number: u32) {
        if workspace_number == self.core.workspace_manager.active_workspace_index() && self.core.config.toggle_workspaces() {
            let prev_ws_num = self.core.workspace_manager.previous_active_workspace_index();
            self.set_active_workspace(prev_ws_num);
        } else {
            self.set_active_workspace(workspace_number);
        }
    }

    pub(in crate::core) fn scrolled_for_workspace_switch(&mut self, amount: f64) {
        let steps = self.core.workspace_manager.scrolled_for_switch(amount);
        if steps != 0 {
            let wrap = self.core.config.wrap_cycle();
            let new_index = self.core.workspace_manager.sequential_workspace_index(steps, wrap);
            if let Some(index) = new_index {
                self.set_active_workspace(index);
            }
        }
    }

    pub(in crate::core) fn append_workspace(&mut self) {
        self.core.workspace_manager.add_workspace();

        #[cfg(feature = "xwayland")]
        {
            self.x11_update_workspace_count(self.core.workspace_manager.workspaces().len() as u32);
            self.x11_update_workspace_names(self.core.workspace_manager.workspace_names());
            self.x11_update_workspace_layout(self.core.workspace_manager.geometry());
            self.x11_update_workarea();
            self.x11_update_window_stacking_order();
        }
    }

    pub(in crate::core) fn insert_workspace(&mut self, at_index: u32) {
        self.core.workspace_manager.insert_workspace(at_index);

        #[cfg(feature = "xwayland")]
        {
            self.x11_update_workspace_count(self.core.workspace_manager.workspaces().len() as u32);
            self.x11_update_workspace_names(self.core.workspace_manager.workspace_names());
            self.x11_update_workspace_layout(self.core.workspace_manager.geometry());
            self.x11_update_active_workspace(self.core.workspace_manager.active_workspace_index());
            self.x11_update_workarea();
            self.x11_update_window_stacking_order();
        }
    }

    pub(in crate::core) fn remove_workspace(&mut self, index: u32) {
        if let Some(new_ws_num) = self.core.workspace_manager.remove_workspace(index) {
            self.set_active_workspace(new_ws_num);
        }

        #[cfg(feature = "xwayland")]
        {
            self.x11_update_workspace_count(self.core.workspace_manager.workspaces().len() as u32);
            self.x11_update_workspace_names(self.core.workspace_manager.workspace_names());
            self.x11_update_workspace_layout(self.core.workspace_manager.geometry());
            self.x11_update_workarea();
            self.x11_update_window_stacking_order();
        }
    }

    pub(in crate::core) fn new_window<P: Into<Point<i32, Logical>>>(
        &mut self,
        window: WindowElement,
        location: P,
        allow_activate: bool,
        workspace_number: Option<u32>,
    ) {
        if !window.props().flags.contains(WindowFlags::NO_CYCLE) {
            self.core.cycle_list.add_new(window.clone());
        }

        let workspace_number = workspace_number.unwrap_or_else(|| self.core.workspace_manager.active_workspace_index());
        window.props().workspace_loc = WorkspaceLocation::Single(workspace_number);

        let give_focus = allow_activate
            && self.core.config.focus_new()
            && workspace_number == self.core.workspace_manager.active_workspace_index()
            && !self.core.cycling_windows;
        let parent = window.parent();

        self.core
            .workspace_manager
            .new_window(window.clone(), location, give_focus, Some(workspace_number), parent.as_ref());

        #[cfg(feature = "xwayland")]
        self.x11_update_window_workspace_location(&window);

        if give_focus {
            self.focus_window(&window, SERIAL_COUNTER.next_serial(), None);
        }

        if self.core.cycling_windows {
            self.add_window_to_tabwin(&window);
        }

        #[cfg(feature = "xwayland")]
        self.x11_update_window_stacking_order();
    }

    pub(in crate::core) fn focus_window(&mut self, window: &WindowElement, serial: Serial, seat: Option<Seat<Self>>) {
        self.core.cycle_list.focused(window);
        self.focus_target(window.clone(), serial, seat);
    }

    pub(in crate::core) fn focus_target<F: Into<KeyboardFocusTarget>>(&mut self, focus: F, serial: Serial, seat: Option<Seat<Self>>) {
        if let Some(keyboard) = seat.as_ref().unwrap_or(&self.core.seat).get_keyboard() {
            let focus = focus.into();

            if let KeyboardFocusTarget::Window(window) = &focus
                && let Some(window) = self.core.workspace_manager.active_workspace().find_window(|elem| elem.0 == *window)
                && let Some(urgent_state) = window.props().urgent.take()
            {
                self.core.handle.remove(urgent_state.token);

                if let Some(decorations) = window.decoration_state_mut().window_decorations_mut() {
                    decorations.disable_titlebar_blink();
                }
            }

            keyboard.set_focus(self, Some(focus), serial);
        }
    }

    pub(in crate::core) fn unset_focus(&mut self, serial: Serial, seat: Option<Seat<Self>>) {
        if let Some(keyboard) = seat.as_ref().unwrap_or(&self.core.seat).get_keyboard() {
            keyboard.set_focus(self, None, serial);
        }
    }

    pub(in crate::core) fn remove_window(&mut self, window: &WindowElement) {
        self.core.cycle_list.remove(window);
        self.core.workspace_manager.remove_window(window);
        self.core.compositor_ui_state.tabwin_remove_window(window.window_id());

        if !self.core.cycling_windows
            && let Some(window) = { self.core.workspace_manager.active_workspace().visible_windows().last().cloned() }
        {
            self.activate_window(&window, true, false, None);
        }

        #[cfg(feature = "xwayland")]
        self.x11_update_window_stacking_order();
    }

    pub(in crate::core) fn activate_window(&mut self, window: &WindowElement, raise: bool, force: bool, seat: Option<Seat<Self>>) {
        let active_workspace_index = self.core.workspace_manager.active_workspace_index();
        let cur_workspace_index = match window.props().workspace_loc {
            WorkspaceLocation::Single(num) => num,
            WorkspaceLocation::All => active_workspace_index,
        };

        if force || active_workspace_index == cur_workspace_index || self.core.config.activate_action() != ActivateAction::None {
            let cur_workspace_index = if active_workspace_index != cur_workspace_index {
                match self.core.config.activate_action() {
                    ActivateAction::Bring => {
                        self.core
                            .workspace_manager
                            .move_window_by_index(window, cur_workspace_index, active_workspace_index);
                        active_workspace_index
                    }
                    ActivateAction::Switch => {
                        self.set_active_workspace(cur_workspace_index);
                        cur_workspace_index
                    }
                    ActivateAction::None if force => {
                        self.set_active_workspace(cur_workspace_index);
                        cur_workspace_index
                    }
                    ActivateAction::None => cur_workspace_index,
                }
            } else {
                cur_workspace_index
            };

            if raise {
                self.raise_window(window, SERIAL_COUNTER.next_serial(), true);
            } else if let Some(workspace) = self.core.workspace_manager.workspaces_mut().get_mut(cur_workspace_index as usize) {
                workspace.set_active_window(Some(window));
            }

            if let Some(workspace) = self.core.workspace_manager.workspaces_mut().get_mut(cur_workspace_index as usize)
                && workspace.active()
            {
                let old_active_window = workspace.visible_windows().find(|window| window.active()).cloned();
                if old_active_window.as_ref().is_none_or(|old| old != window) {
                    let serial = SERIAL_COUNTER.next_serial();
                    self.focus_window(window, serial, seat);

                    if let Some(old_active_window) = &old_active_window {
                        self.core.toplevel_changed(
                            old_active_window,
                            None,
                            None,
                            WindowState::empty(),
                            WindowState::ACTIVATED,
                            Vec::new(),
                            Vec::new(),
                            None,
                        );
                    }

                    if old_active_window.as_ref() != Some(window) {
                        self.core.toplevel_changed(
                            window,
                            None,
                            None,
                            WindowState::ACTIVATED,
                            WindowState::empty(),
                            Vec::new(),
                            Vec::new(),
                            None,
                        );
                    }
                }
            }
        }
    }

    fn update_minimized_state(&self, window: &WindowElement, is_minimized: bool) {
        match window.0.underlying_surface() {
            WindowSurface::Wayland(surface) => {
                surface.with_pending_state(|state| {
                    if is_minimized {
                        state.states.set(xdg_toplevel::State::Suspended);
                    } else {
                        state.states.unset(xdg_toplevel::State::Suspended);
                    }
                });

                window
                    .0
                    .user_data()
                    .get_or_insert(XdgSurfaceProps::default)
                    .0
                    .lock()
                    .unwrap()
                    .is_minimized = is_minimized;
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x11_surface) => {
                if x11_surface.is_hidden() != is_minimized {
                    let _ = x11_surface.set_hidden(is_minimized);
                }
            }
        }
    }

    fn set_window_minimized_internal(&mut self, window: &WindowElement) {
        if self.core.workspace_manager.set_window_minimized(window) {
            if !self.core.config.cycle_minimized() {
                self.core.cycle_list.move_to_back(window);
            }
            self.update_minimized_state(window, true);
            window.set_activate(false);

            #[cfg(feature = "xwayland")]
            self.x11_update_window_allowed_actions(window);

            self.core.toplevel_changed(
                window,
                None,
                None,
                WindowState::MINIMIZED,
                WindowState::empty(),
                Vec::new(),
                Vec::new(),
                None,
            );
        }
    }

    pub(in crate::core) fn set_window_minimized(&mut self, window: &WindowElement) {
        // Here we do a breadth-first traversal, but upside down.
        let mut windows = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(window.clone());

        while let Some(window) = queue.pop_front() {
            for child in window.children() {
                queue.push_back(child);
            }
            windows.push(window);
        }

        let was_active = windows.into_iter().rev().fold(false, |was_active_accum, window| {
            self.set_window_minimized_internal(&window);
            was_active_accum | window.active()
        });

        if was_active && let Some(window) = { self.core.workspace_manager.active_workspace().visible_windows().last().cloned() } {
            self.activate_window(&window, true, true, None);
        }
    }

    fn set_window_unminimized_internal(&mut self, window: &WindowElement, serial: Serial, activate: bool) {
        if self.core.workspace_manager.set_window_unminimized(window, activate) {
            self.set_window_shaded(window, false);
            self.update_minimized_state(window, false);

            if activate {
                self.focus_window(window, serial, None);
            }

            #[cfg(feature = "xwayland")]
            {
                self.x11_update_window_allowed_actions(window);
                self.x11_update_window_stacking_order();
            }

            self.core.toplevel_changed(
                window,
                None,
                None,
                WindowState::empty(),
                WindowState::MINIMIZED,
                Vec::new(),
                Vec::new(),
                None,
            );
        }
    }

    pub(in crate::core) fn set_window_unminimized(&mut self, window: &WindowElement, serial: Serial, activate: bool) {
        self.maybe_clear_show_desktop_for(window);

        let mut windows = vec![window.clone()];
        while let Some(parent) = window.parent() {
            windows.push(parent);
        }

        for w in windows.into_iter().rev() {
            self.set_window_unminimized_internal(&w, serial, activate && &w == window);
        }
    }

    pub(in crate::core) fn set_window_maximized(&mut self, window: &WindowElement, anchor: Option<Point<f64, Logical>>) {
        self.set_window_untiled(window, None);

        if let Some((output, output_geom)) = output_and_geom_for_anchored_layout(&self.core.workspace_manager, window) {
            let old_geom = self.core.workspace_manager.window_geometry(window);
            let mut props = window.props();
            if props.saved_geom.is_none() {
                props.saved_geom = old_geom;
            }
            drop(props);

            if let Some(window_decorations) = window.decoration_state().window_decorations_mut() {
                window_decorations.update_maximized_state(true);
            }

            self.apply_anchored_layout(window, WindowLayout::Maximized, &output, output_geom);

            self.core.toplevel_changed(
                window,
                None,
                None,
                WindowState::MAXIMIZED,
                WindowState::empty(),
                Vec::new(),
                Vec::new(),
                None,
            );
        }
    }

    pub(in crate::core) fn set_window_unmaximized(&mut self, window: &WindowElement, new_location: Option<Point<i32, Logical>>) {
        if window.maximized() {
            if let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                window_decorations.update(DecorationInput::Maximized(false));
            }
            #[cfg(feature = "xwayland")]
            {
                self.x11_update_window_frame_extents(window);
                self.x11_update_window_allowed_actions(window);
            }

            let mut props = window.props();
            let old_geom = props.saved_geom.take();
            props.anchored_output = None;
            drop(props);

            let new_location = new_location.or_else(|| old_geom.map(|geom| geom.loc));

            match window.0.underlying_surface() {
                WindowSurface::Wayland(surface) => {
                    surface.with_pending_state(|state| {
                        state.states.unset(xdg_toplevel::State::Maximized);
                        state.size = None;
                    });

                    // The protocol demands us to always reply with a configure,
                    // regardless of we fulfilled the request or not
                    if surface.is_initial_configure_sent() {
                        surface.send_configure();
                    }
                }

                #[cfg(feature = "xwayland")]
                WindowSurface::X11(surface) => {
                    let _ = surface.set_maximized(false);
                    if let Some(old_geom) = old_geom {
                        let deco_offset = window
                            .decoration_state()
                            .window_decorations()
                            .map(|d| d.decorations_offset())
                            .unwrap_or_default();
                        let deco_size = window
                            .decoration_state()
                            .window_decorations()
                            .map(|d| (
                                d.left_decoration_width() + d.right_decoration_width(),
                                d.top_decoration_height() + d.bottom_decoration_height(),
                            ))
                            .unwrap_or((0, 0));
                        let _ = surface.configure(Rectangle::new(
                            old_geom.loc + deco_offset,
                            (old_geom.size.w - deco_size.0, old_geom.size.h - deco_size.1).into(),
                        ));
                    }
                }
            }

            if let Some(new_location) = new_location {
                self.core.workspace_manager.relocate_window(window, new_location, false);
            }

            self.core.toplevel_changed(
                window,
                None,
                None,
                WindowState::empty(),
                WindowState::MAXIMIZED,
                Vec::new(),
                Vec::new(),
                None,
            );
        }
    }

    pub(in crate::core) fn set_window_filled(&mut self, window: &WindowElement) {
        if window.maximized() {
            self.set_window_unmaximized(window, None);
        }

        let outputs_for_window = self.core.workspace_manager.outputs_for_window(window);
        if let Some(output) = outputs_for_window.first().or_else(|| {
            // The window hasn't been mapped yet, use the primary output instead
            self.core.workspace_manager.outputs().next()
        }) {
            let layer_map = layer_map_for_output(output);
            let mut geometry = layer_map.non_exclusive_zone();
            drop(layer_map);

            if let Some(window_decorations) = window.decoration_state().window_decorations_mut() {
                geometry.size.w -= window_decorations.left_decoration_width() + window_decorations.right_decoration_width();
                geometry.size.h -= window_decorations.top_decoration_height() + window_decorations.bottom_decoration_height();
            }

            match window.0.underlying_surface() {
                WindowSurface::Wayland(surface) => {
                    surface.with_pending_state(|state| {
                        state.size = Some(geometry.size);
                    });
                    self.core.workspace_manager.relocate_window(window, geometry.loc, false);

                    if surface.is_initial_configure_sent() {
                        surface.send_configure();
                    }
                }

                #[cfg(feature = "xwayland")]
                WindowSurface::X11(surface) => {
                    let deco_offset = window
                        .decoration_state()
                        .window_decorations()
                        .map(|d| d.decorations_offset())
                        .unwrap_or_default();
                    let _ = surface.configure(Rectangle::new(geometry.loc + deco_offset, geometry.size));
                    self.core.workspace_manager.relocate_window(window, geometry.loc, false);
                }
            }
        }
    }

    pub(in crate::core) fn set_window_tiled(&mut self, window: &WindowElement, mode: TileMode, anchor: Option<Point<f64, Logical>>) {
        if window.can_tile() {
            self.set_window_unmaximized(window, None);

            if let Some((output, output_geom)) = output_and_geom_for_anchored_layout(&self.core.workspace_manager, window) {
                let old_geom = self.core.workspace_manager.window_geometry(window);
                let mut props = window.props();
                props.tile_mode = Some(mode);
                let saved_geom_was_empty = props.saved_geom.is_none();
                if saved_geom_was_empty {
                    props.saved_geom = old_geom;
                }
                drop(props);

                if self
                    .apply_anchored_layout(window, WindowLayout::Tiled(mode), &output, output_geom)
                    .is_none()
                {
                    let mut props = window.props();
                    props.tile_mode = None;
                    if saved_geom_was_empty {
                        props.saved_geom = None;
                    }
                }
            }
        }
    }

    pub(in crate::core) fn apply_anchored_layout(
        &mut self,
        window: &WindowElement,
        layout: WindowLayout,
        output: &Output,
        output_geom: Rectangle<i32, Logical>,
    ) -> Option<Vec<Output>> {
        let zone = layer_map_for_output(output).non_exclusive_zone();
        let zone = Rectangle::new(output_geom.loc + zone.loc, zone.size);

        if let Some(mut geometry) = layout.geometry_in_zone(zone) {
            if let Some(window_decorations) = window.decoration_state().window_decorations_mut() {
                window_decorations.refresh_layout();
                geometry.size.w -= window_decorations.left_decoration_width() + window_decorations.right_decoration_width();
                geometry.size.h -= window_decorations.top_decoration_height() + window_decorations.bottom_decoration_height();
            }

            let fits_hints = if matches!(layout, WindowLayout::Tiled(_)) {
                let (min, max) = window.min_max_sizes();
                geometry.size.w >= min.w
                    && geometry.size.h >= min.h
                    && (max.w == 0 || geometry.size.w <= max.w)
                    && (max.h == 0 || geometry.size.h <= max.h)
            } else {
                true
            };

            if fits_hints {
                window.props().anchored_output = Some(output.downgrade());

                let xdg_states = layout.as_xdg_toplevel_states();

                match window.0.underlying_surface() {
                    WindowSurface::Wayland(surface) => {
                        surface.with_pending_state(|state| {
                            remove_all_layout_states(state);
                            for s in xdg_states {
                                state.states.set(*s);
                            }
                            state.size = Some(geometry.size);
                            state.bounds = Some(geometry.size);
                        });
                        if surface.is_initial_configure_sent() {
                            surface.send_pending_configure();
                        }
                    }

                    #[cfg(feature = "xwayland")]
                    WindowSurface::X11(surface) => {
                        let _ = surface.set_maximized(matches!(layout, WindowLayout::Maximized));
                        let deco_offset = window
                            .decoration_state()
                            .window_decorations()
                            .map(|d| d.decorations_offset())
                            .unwrap_or_default();
                        let _ = surface.configure(Rectangle::new(geometry.loc + deco_offset, geometry.size));
                    }
                }

                if !window.minimized() {
                    self.core.workspace_manager.relocate_window(window, geometry.loc, false);
                }

                Some(self.core.workspace_manager.output_under(geometry.loc.to_f64()).cloned().collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(in crate::core) fn toggle_window_tiled(&mut self, window: &WindowElement, mode: TileMode) {
        if window.tile_mode().is_some_and(|tile_mode| tile_mode == mode) {
            self.set_window_untiled(window, None);
        } else {
            self.set_window_tiled(window, mode, None);
        }
    }

    pub(in crate::core) fn clear_window_tiled_metadata(&mut self, window: &WindowElement) {
        let mut props = window.props();
        if props.tile_mode.is_some() {
            props.tile_mode = None;
            props.anchored_output = None;
            props.saved_geom = None;
            drop(props);

            if let WindowSurface::Wayland(surface) = window.0.underlying_surface() {
                surface.with_pending_state(remove_tiled_states);
            }
        }
    }

    pub(in crate::core) fn set_window_untiled(&mut self, window: &WindowElement, new_location: Option<Point<i32, Logical>>) {
        let mut props = window.props();
        if props.tile_mode.is_some() {
            props.tile_mode = None;
            props.anchored_output = None;
            let saved_geom = props.saved_geom.take();
            drop(props);

            match window.0.underlying_surface() {
                WindowSurface::Wayland(surface) => {
                    surface.with_pending_state(|state| {
                        remove_tiled_states(state);
                        state.size = None;
                    });
                    if surface.is_initial_configure_sent() {
                        surface.send_configure();
                    }
                }

                #[cfg(feature = "xwayland")]
                WindowSurface::X11(surface) => {
                    if let Some(saved_geom) = saved_geom {
                        let deco_offset = window
                            .decoration_state()
                            .window_decorations()
                            .map(|d| d.decorations_offset())
                            .unwrap_or_default();
                        let deco_size = window
                            .decoration_state()
                            .window_decorations()
                            .map(|d| (
                                d.left_decoration_width() + d.right_decoration_width(),
                                d.top_decoration_height() + d.bottom_decoration_height(),
                            ))
                            .unwrap_or((0, 0));
                        let _ = surface.configure(Rectangle::new(
                            saved_geom.loc + deco_offset,
                            (saved_geom.size.w - deco_size.0, saved_geom.size.h - deco_size.1).into(),
                        ));
                    }
                }
            }

            if let Some(new_location) = new_location.or_else(|| saved_geom.map(|geom| geom.loc)) {
                self.core.workspace_manager.relocate_window(window, new_location, false);
            }

            #[cfg(feature = "xwayland")]
            self.x11_update_window_allowed_actions(window);
        }
    }

    pub(in crate::core) fn set_window_shaded(&self, window: &WindowElement, is_shaded: bool) {
        let mut props = window.props();
        let changed = if props.is_shaded != is_shaded {
            props.is_shaded = is_shaded;
            if let Some(decorations) = window.decoration_state_mut().window_decorations_mut() {
                decorations.update(DecorationInput::Shaded(is_shaded));
            }
            #[cfg(feature = "xwayland")]
            self.x11_update_window_frame_extents(window);

            true
        } else {
            false
        };
        drop(props);

        if changed {
            #[cfg(feature = "xwayland")]
            if let WindowSurface::X11(x11_surface) = window.0.underlying_surface() {
                let _ = x11_surface.set_shaded(is_shaded);
            }
        }
    }

    fn set_window_sticky_internal(&mut self, window: &WindowElement, is_sticky: bool) {
        let cur_is_sticky = window.props().workspace_loc == WorkspaceLocation::All;
        if cur_is_sticky != is_sticky {
            let new_ws_loc = if is_sticky {
                WorkspaceLocation::All
            } else {
                WorkspaceLocation::Single(self.core.workspace_manager.active_workspace_index())
            };

            self.core.workspace_manager.set_window_workspace_num(window, new_ws_loc);

            if let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                window_decorations.update(DecorationInput::Sticky(is_sticky));
            }

            #[cfg(feature = "xwayland")]
            if let WindowSurface::X11(x11_surface) = window.0.underlying_surface() {
                let _ = x11_surface.set_sticky(is_sticky);
                self.x11_update_window_workspace_location(window);
            }

            let (added, removed) = if is_sticky {
                (WindowState::STICKY, WindowState::empty())
            } else {
                (WindowState::empty(), WindowState::STICKY)
            };
            self.core
                .toplevel_changed(window, None, None, added, removed, Vec::new(), Vec::new(), None);

            #[cfg(feature = "xwayland")]
            self.x11_update_window_stacking_order();
        }
    }

    pub(in crate::core) fn set_window_sticky(&mut self, window: &WindowElement, is_sticky: bool) {
        let mut root = window.clone();
        while let Some(parent) = root.parent() {
            root = parent;
        }

        // Do a breadth-first traversal, (un)sticking each window as we go down the tree.
        let mut queue = VecDeque::new();
        queue.push_back(root);
        while let Some(child) = queue.pop_front() {
            self.set_window_sticky_internal(&child, is_sticky);
            for child in child.children() {
                queue.push_back(child);
            }
        }
    }

    pub(in crate::core) fn set_window_stacking_layer(&mut self, window: &WindowElement, layer: WindowStackingLayer) {
        self.core.workspace_manager.set_window_stacking_layer(window, layer);

        #[cfg(feature = "xwayland")]
        if let WindowSurface::X11(surface) = window.0.underlying_surface() {
            let _ = surface.set_below(matches!(
                layer,
                WindowStackingLayer::AlwaysOnBottom | WindowStackingLayer::Background
            ));
            let _ = surface.set_above(matches!(
                layer,
                WindowStackingLayer::AlwaysOnTop | WindowStackingLayer::Overlay | WindowStackingLayer::System
            ));
            self.x11_update_window_stacking_order();
        }
    }

    pub(in crate::core) fn set_window_always_on_top(&mut self, window: &WindowElement) {
        self.set_window_stacking_layer(window, WindowStackingLayer::AlwaysOnTop);
    }

    pub(in crate::core) fn set_window_always_on_bottom(&mut self, window: &WindowElement) {
        self.set_window_stacking_layer(window, WindowStackingLayer::AlwaysOnBottom);
    }

    pub(in crate::core) fn set_window_normal_stacking(&mut self, window: &WindowElement) {
        self.set_window_stacking_layer(window, WindowStackingLayer::Normal);
    }

    pub(in crate::core) fn set_window_fullscreen(&mut self, window: &WindowElement, output: Option<Output>) {
        let workspace = self.core.workspace_manager.active_workspace_mut();
        let output_and_geometry = output
            .or_else(|| workspace.outputs_for_window(window).into_iter().next())
            .or_else(|| self.core.workspace_manager.outputs().next().cloned())
            .and_then(|output| self.core.workspace_manager.output_geometry(&output).map(|geom| (output, geom)));

        if let Some((output, geometry)) = output_and_geometry {
            // NOTE: This is only one part of the solution. We can set the
            // location and configure size here, but the surface should be rendered fullscreen
            // independently from its buffer size

            let (fullscreened, old_fullscreen_windows) = match window.0.underlying_surface() {
                WindowSurface::Wayland(surface) => {
                    let (fullscreened, old_fullscreen_window) =
                        if let Ok(client) = self.core.display_handle.get_client(surface.wl_surface().id()) {
                            let wl_output = output.client_outputs(&client).last();

                            self.disable_decorations_for_window(window);
                            surface.with_pending_state(|state| {
                                state.states.set(xdg_toplevel::State::Fullscreen);
                                state.size = Some(geometry.size);
                                state.fullscreen_output = wl_output;
                            });
                            tracing::trace!("Fullscreening: {:?}", window);
                            (true, self.core.workspace_manager.set_window_fullscreen(window, &output))
                        } else {
                            (false, vec![])
                        };

                    // The protocol demands us to always reply with a configure,
                    // regardless of we fulfilled the request or not
                    if surface.is_initial_configure_sent() {
                        surface.send_configure();
                    }

                    (fullscreened, old_fullscreen_window)
                }

                #[cfg(feature = "xwayland")]
                WindowSurface::X11(surface) => {
                    self.disable_decorations_for_window(window);
                    let _ = surface.set_fullscreen(true);
                    let _ = surface.configure(window.grow_rect_by_gtk_frame_extents(geometry));
                    tracing::trace!("Fullscreening: {:?}", window);
                    (true, self.core.workspace_manager.set_window_fullscreen(window, &output))
                }
            };

            self.backend.reset_buffers(&output);

            for old_fullscreen_window in old_fullscreen_windows {
                self.set_window_unfullscreen(&old_fullscreen_window);
            }

            if fullscreened {
                self.core.toplevel_changed(
                    window,
                    None,
                    None,
                    WindowState::FULLSCREEN,
                    WindowState::empty(),
                    Vec::new(),
                    Vec::new(),
                    None,
                );
            }
        }
    }

    pub(in crate::core) fn set_window_unfullscreen(&mut self, window: &WindowElement) {
        match window.0.underlying_surface() {
            WindowSurface::Wayland(surface) => {
                surface.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Fullscreen);
                    state.size = None;
                    state.fullscreen_output = None;
                });

                // The protocol demands us to always reply with a configure,
                // regardless of we fulfilled the request or not
                if surface.is_initial_configure_sent() {
                    surface.send_configure();
                }
            }

            #[cfg(feature = "xwayland")]
            WindowSurface::X11(surface) => {
                let _ = surface.set_fullscreen(false);
                // Re-enable decorations first so deco_offset is correct for the configure.
                if !surface.is_decorated() {
                    self.enable_decorations_for_window(window);
                } else {
                    self.disable_decorations_for_window(window);
                }
                if let Some(workspace) = self.core.workspace_manager.workspace_for_window(window) {
                    if let Some(element_loc) = workspace.window_location(window) {
                        let deco_offset = window
                            .decoration_state()
                            .window_decorations()
                            .map(|d| d.decorations_offset())
                            .unwrap_or_default();
                        let _ = surface.configure(Some(Rectangle::new(
                            element_loc + deco_offset,
                            surface.geometry().size,
                        )));
                    }
                }
            }
        }

        for output in self.core.workspace_manager.set_window_unfullscreen(window) {
            self.backend.reset_buffers(&output);
        }

        self.core.toplevel_changed(
            window,
            None,
            None,
            WindowState::empty(),
            WindowState::FULLSCREEN,
            Vec::new(),
            Vec::new(),
            None,
        );
    }

    fn raise_window_internal(&mut self, window: &WindowElement, root_stacking: WindowStackingLayer, serial: Serial, activate: bool) {
        self.set_window_stacking_layer(window, root_stacking);

        let active_ws_num = self.core.workspace_manager.active_workspace_index();
        let workspace_and_index = if !window.sticky() {
            self.core.workspace_manager.workspace_for_window_with_index_mut(window)
        } else {
            Some((active_ws_num, self.core.workspace_manager.active_workspace_mut()))
        };

        if let Some((ws_num, workspace)) = workspace_and_index {
            workspace.raise_window(window, activate);
            if ws_num == active_ws_num && activate {
                self.focus_window(window, serial, None);
            }
        }

        #[cfg(feature = "xwayland")]
        self.x11_update_window_stacking_order();
    }

    pub(in crate::core) fn raise_window(&mut self, window: &WindowElement, serial: Serial, activate: bool) {
        let mut root = window.clone();
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let root_stacking = root.stacking_layer();

        // Do a breadth-first traversal, raising each window as we go down the tree.
        let mut queue = VecDeque::new();
        queue.push_back(root);
        while let Some(child) = queue.pop_front() {
            self.raise_window_internal(&child, root_stacking, serial, activate && &child == window);
            for child in child.children() {
                queue.push_back(child);
            }
        }
    }

    fn lower_window_internal(&mut self, window: &WindowElement, below: Option<&WindowElement>) {
        if let Some(workspace) = if !window.sticky() {
            self.core.workspace_manager.workspace_for_window_mut(window)
        } else {
            Some(self.core.workspace_manager.active_workspace_mut())
        } {
            if let Some(below) = below {
                workspace.lower_window_below(window, below);
            } else {
                workspace.lower_window(window);
            }
        }
    }

    pub(in crate::core) fn lower_window(&mut self, window: &WindowElement, serial: Serial, below: Option<WindowElement>) {
        let mut root = window.clone();
        while let Some(parent) = root.parent() {
            root = parent;
        }

        // Do a breadth-first traversal, lowering each window as we go down the tree.
        let mut queue = VecDeque::new();
        let mut was_active = false;
        queue.push_back(root);
        while let Some(child) = queue.pop_front() {
            was_active |= child.active();
            self.lower_window_internal(&child, below.as_ref());
            for child in child.children() {
                queue.push_back(child);
            }
        }

        let active_ws_num = self.core.workspace_manager.active_workspace_index();
        let workspace_and_index = if !window.sticky() {
            self.core.workspace_manager.workspace_for_window_with_index_mut(window)
        } else {
            Some((active_ws_num, self.core.workspace_manager.active_workspace_mut()))
        };

        if let Some((ws_num, workspace)) = workspace_and_index
            && was_active
        {
            // Next activate and give focus to the now-top window in the stack.
            if let Some(new_focus) = workspace.visible_windows().last().cloned() {
                workspace.raise_window(&new_focus, true);
                if ws_num == active_ws_num {
                    self.focus_window(&new_focus, serial, None);
                }
            }
        }

        #[cfg(feature = "xwayland")]
        self.x11_update_window_stacking_order();
    }

    pub(in crate::core) fn move_window_to_workspace_in_direction(&mut self, window: &WindowElement, direction: Direction) -> Option<u32> {
        let new_ws_num = self
            .core
            .workspace_manager
            .move_window_by_direction(window, direction, self.core.config.wrap_layout());

        #[cfg(feature = "xwayland")]
        if new_ws_num.is_some() {
            self.x11_update_window_workspace_location(window);
            self.x11_update_window_stacking_order();
        }

        new_ws_num
    }

    pub(in crate::core) fn move_window_to_workspace_index(&mut self, window: &WindowElement, new_index: u32) -> bool {
        let updated = self.core.workspace_manager.move_window_to(window, new_index);

        #[cfg(feature = "xwayland")]
        if updated {
            self.x11_update_window_workspace_location(window);
            self.x11_update_window_stacking_order();
        }

        updated
    }

    pub(in crate::core) fn move_window_to_workspace_old_new_index(
        &mut self,
        window: &WindowElement,
        old_index: u32,
        new_index: u32,
    ) -> bool {
        let updated = self.core.workspace_manager.move_window_by_index(window, old_index, new_index);

        #[cfg(feature = "xwayland")]
        if updated {
            self.x11_update_window_workspace_location(window);
            self.x11_update_window_stacking_order();
        }

        updated
    }

    pub(in crate::core) fn move_window_to_previous_workspace(&mut self, window: &WindowElement) -> Option<u32> {
        let new_ws_num = self
            .core
            .workspace_manager
            .move_window_previous(window, self.core.config.wrap_layout());

        #[cfg(feature = "xwayland")]
        if new_ws_num.is_some() {
            self.x11_update_window_workspace_location(window);
            self.x11_update_window_stacking_order();
        }

        new_ws_num
    }

    pub(in crate::core) fn move_window_to_next_workspace(&mut self, window: &WindowElement) -> Option<u32> {
        let new_ws_num = self.core.workspace_manager.move_window_next(window, self.core.config.wrap_layout());

        #[cfg(feature = "xwayland")]
        if new_ws_num.is_some() {
            self.x11_update_window_workspace_location(window);
            self.x11_update_window_stacking_order();
        }

        new_ws_num
    }

    pub(in crate::core) fn move_window_to_output_in_direction(&mut self, window: &WindowElement, direction: Direction) {
        if let Some(current_output_and_rect) = self.output_and_rect_for_window(window)
            && let outputs_and_rects = self.outputs_and_rects()
            && let Some(OutputAndRect {
                output: new_output,
                rect: new_output_rect,
            }) = adjacent_monitor_in_direction(&outputs_and_rects, &current_output_and_rect, direction)
            && let Some(current_window_loc) = self.core.workspace_manager.active_workspace().window_location(window)
        {
            let OutputAndRect {
                output: current_output,
                rect: current_output_rect,
            } = current_output_and_rect;

            let current_zone_rect = {
                let mut zone_rect = layer_map_for_output(&current_output).non_exclusive_zone();
                zone_rect.loc += current_output_rect.loc;
                zone_rect
            };
            let new_zone_rect = {
                let mut zone_rect = layer_map_for_output(&new_output).non_exclusive_zone();
                zone_rect.loc += new_output_rect.loc;
                zone_rect
            };

            let offset_in_current_output = current_window_loc - current_zone_rect.loc;
            let new_location = new_zone_rect.loc + offset_in_current_output;
            self.core.workspace_manager.relocate_window(window, new_location, false);

            let layout = window.current_layout();
            if layout != WindowLayout::Normal {
                self.apply_anchored_layout(window, layout, &new_output, new_output_rect);
            }
        }
    }

    pub(in crate::core) fn set_window_parent(&mut self, window: &WindowElement, parent: Option<WindowElement>) {
        let old_parent = window.parent();
        if window.set_parent(parent.clone()) {
            if let Some(old_parent) = old_parent {
                old_parent.remove_child(window);
            }

            if let Some(parent) = &parent {
                parent.add_child(window.clone());
            }
        }

        if let Some(parent) = parent {
            let workspace_loc = parent.props().workspace_loc;
            match workspace_loc {
                WorkspaceLocation::Single(num) => {
                    self.set_window_sticky(window, false);

                    if let Some(workspace) = self.core.workspace_manager.workspaces_mut().get_mut(num as usize)
                        && workspace.window_location(window).is_some()
                    {
                        let activate = workspace.active_window().is_some_and(|active| active == &parent);
                        workspace.raise_window_above(window, &parent, activate);
                    }
                }

                WorkspaceLocation::All => {
                    self.set_window_sticky(window, true);

                    for workspace in self.core.workspace_manager.workspaces_mut() {
                        if workspace.window_location(window).is_some() {
                            let activate = workspace.active_window().is_some_and(|active| active == &parent);
                            workspace.raise_window_above(window, &parent, activate);
                        }
                    }
                }
            }
        }

        #[cfg(feature = "xwayland")]
        self.x11_update_window_stacking_order();
    }
}
