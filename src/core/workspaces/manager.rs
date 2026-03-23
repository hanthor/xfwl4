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

use std::collections::HashSet;

use anyhow::anyhow;
use smithay::{
    desktop::space::{RenderZindex, SpaceElement},
    output::{Output, Scale as OutputScale},
    reexports::{calloop::LoopHandle, wayland_server::DisplayHandle},
    utils::{Logical, Point, Rectangle, Size},
};
use xfconf::ChannelExtManual;

use crate::{
    backend::Backend,
    core::{
        config::XFWM4_CHANNEL_NAME,
        shell::{WindowElement, WorkspaceLocation, ssd::DecorationInput},
        state::Xfwl4State,
        util::{CalloopXfconfSource, Direction, ScrollAccumulator, zip_all_first},
        workspaces::Workspace,
    },
    protocols::ext_workspace::{ExtWorkspaceHandler, ExtWorkspaceState, WorkspaceChangedInput, WorkspaceCreatedInput},
};

const PROP_WORKSPACE_COUNT: &str = "/general/workspace_count";
const PROP_WORKSPACE_NAMES: &str = "/general/workspace_names";
const PROP_WORKSPACE_NROWS: &str = "/general/workspace_nrows";

const WIN_LAYER_BACKGROUND: u8 = RenderZindex::Background as u8;
const WIN_LAYER_BOTTOM: u8 = RenderZindex::Bottom as u8;
const WIN_LAYER_NORMAL: u8 = RenderZindex::Shell as u8;
const WIN_LAYER_TOP: u8 = RenderZindex::Top as u8;
const WIN_LAYER_OVERLAY: u8 = RenderZindex::Overlay as u8;
const WIN_LAYER_SYSTEM: u8 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum WindowStackingLayer {
    Background = WIN_LAYER_BACKGROUND,
    AlwaysOnBottom = WIN_LAYER_BOTTOM,
    Normal = WIN_LAYER_NORMAL,
    AlwaysOnTop = WIN_LAYER_TOP,
    Overlay = WIN_LAYER_OVERLAY,
    System = WIN_LAYER_SYSTEM,
}

impl TryFrom<u8> for WindowStackingLayer {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            WIN_LAYER_BACKGROUND => Ok(WindowStackingLayer::Background),
            WIN_LAYER_BOTTOM => Ok(WindowStackingLayer::AlwaysOnBottom),
            WIN_LAYER_NORMAL => Ok(WindowStackingLayer::Normal),
            WIN_LAYER_TOP => Ok(WindowStackingLayer::AlwaysOnTop),
            WIN_LAYER_OVERLAY => Ok(WindowStackingLayer::Overlay),
            WIN_LAYER_SYSTEM => Ok(WindowStackingLayer::System),
            unknown => Err(anyhow!("Unknown stacking layer {unknown}")),
        }
    }
}

pub struct WorkspaceManager<BackendData: Backend + 'static> {
    channel: xfconf::Channel,
    workspaces: Vec<Workspace>,
    active_space: u32,
    previous_active_space: u32,
    geometry: Size<u32, Logical>,

    scroll_accum: ScrollAccumulator,

    ext_workspace_state: ExtWorkspaceState<Xfwl4State<BackendData>>,
}

impl<BackendData: Backend + 'static> WorkspaceManager<BackendData> {
    pub fn new(dh: &DisplayHandle, loop_handle: &LoopHandle<'static, Xfwl4State<BackendData>>) -> Self {
        let mut manager = Self {
            channel: xfconf::Channel::new(XFWM4_CHANNEL_NAME),
            workspaces: Default::default(),
            active_space: 0,
            previous_active_space: 0,
            geometry: (1, 1).into(),
            scroll_accum: ScrollAccumulator::default(),
            ext_workspace_state: ExtWorkspaceState::new(dh),
        };

        let source = CalloopXfconfSource::new(
            manager.channel.clone(),
            [PROP_WORKSPACE_COUNT, PROP_WORKSPACE_NAMES, PROP_WORKSPACE_NROWS],
        );
        loop_handle
            .insert_source(source, |(property_name, value), _, state| match property_name.as_str() {
                PROP_WORKSPACE_COUNT => {
                    if let Ok(new_count) = value.get::<i32>()
                        && new_count > 0
                        && let Some(new_ws_num) = state.core.workspace_manager.on_workspace_count_changed(new_count as u32)
                    {
                        #[cfg(feature = "xwayland")]
                        {
                            state.x11_update_workspace_count(new_ws_num);
                            state.x11_update_workspace_layout(state.core.workspace_manager.geometry);
                            state.x11_update_workarea();
                        }
                        state.set_active_workspace(new_ws_num);
                    }
                }

                PROP_WORKSPACE_NAMES => {
                    if let Ok(new_names) = value.get::<xfconf::Array<String>>().map(|v| v.into_inner()) {
                        state.core.workspace_manager.on_workspace_names_changed(new_names.clone());
                        #[cfg(feature = "xwayland")]
                        state.x11_update_workspace_names(new_names);
                    }
                }

                PROP_WORKSPACE_NROWS => {
                    if let Ok(new_num_rows) = value.get::<i32>()
                        && new_num_rows > 0
                    {
                        state.core.workspace_manager.on_workspace_num_rows_changed(new_num_rows as u32);
                        #[cfg(feature = "xwayland")]
                        state.x11_update_workspace_layout(state.core.workspace_manager.geometry);
                    }
                }
                _ => (),
            })
            .unwrap();

        manager.init_workspaces();
        manager.active_workspace_mut().set_active(true);

        manager
    }

    fn init_workspaces(&mut self) {
        let count = self
            .channel
            .get_property::<i32>(PROP_WORKSPACE_COUNT)
            .filter(|count| *count > 0)
            .unwrap_or(1) as u32;
        let names = self.get_workspace_names_uncached();
        let nrows = self
            .channel
            .get_property::<i32>(PROP_WORKSPACE_NROWS)
            .filter(|nrows| *nrows > 0)
            .unwrap_or(1) as u32;

        self.update_geometry(nrows, count);

        self.workspaces = zip_all_first(0..count, names)
            .map(|(i, name)| {
                let name = name.unwrap_or_else(|| format!("Workspace {}", i + 1));
                let position = position_for_workspace_index(i, self.geometry, count);
                Workspace::new(name, position)
            })
            .collect::<Vec<_>>();

        for (i, workspace) in self.workspaces.iter().enumerate() {
            self.ext_workspace_state.workspace_created(WorkspaceCreatedInput {
                id: workspace.id(),
                name: workspace.name(),
                coordinates: workspace.position(),
                is_active: self.active_space as usize == i,
            });
        }
    }

    pub fn map_output<P: Into<Point<i32, Logical>>>(&mut self, output: &Output, position: P) {
        let position = position.into();
        for workspace in self.workspaces.iter_mut() {
            workspace.map_output(output, position);
        }

        self.ext_workspace_state.output_enter(output);
    }

    pub fn unmap_output(&mut self, output: &Output) {
        for workspace in self.workspaces.iter_mut() {
            workspace.unmap_output(output);
        }

        self.ext_workspace_state.output_leave(output);
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.active_workspace().outputs()
    }

    pub fn geometry(&self) -> Size<u32, Logical> {
        self.geometry
    }

    pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
        self.active_workspace().output_geometry(output)
    }

    pub fn output_under<P: Into<Point<f64, Logical>>>(&self, point: P) -> impl Iterator<Item = &Output> {
        self.active_workspace().output_under(point)
    }

    pub fn workspace_names(&self) -> Vec<String> {
        self.workspaces.iter().map(|workspace| workspace.name().to_owned()).collect()
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn workspaces_mut(&mut self) -> &mut [Workspace] {
        &mut self.workspaces
    }

    pub(super) fn set_active_workspace(&mut self, num: u32) -> Option<(Option<&Workspace>, &Workspace)> {
        if (num as usize) < self.workspaces.len() && self.active_space != num {
            tracing::debug!("Switching active workspace from {} to {num}", self.active_space);

            let prev_ws_num = if let Some(old_active_space) = self.workspaces.get_mut(self.active_space as usize) {
                old_active_space.set_active(false);
                self.ext_workspace_state.workspace_changed(
                    old_active_space.id(),
                    WorkspaceChangedInput {
                        name: None,
                        coordinates: None,
                        is_active: Some(false),
                    },
                );
                Some(self.active_space)
            } else {
                None
            };

            self.previous_active_space = self.active_space;
            self.active_space = num;

            let new_ws_num = if let Some(new_active_space) = self.workspaces.get_mut(self.active_space as usize) {
                new_active_space.set_active(true);
                self.ext_workspace_state.workspace_changed(
                    new_active_space.id(),
                    WorkspaceChangedInput {
                        name: None,
                        coordinates: None,
                        is_active: Some(true),
                    },
                );
                Some(self.active_space)
            } else {
                None
            };

            match (prev_ws_num, new_ws_num) {
                (Some(prev_ws_num), Some(new_ws_num)) => {
                    if prev_ws_num < new_ws_num {
                        let (left, right) = self.workspaces.split_at(new_ws_num as usize);
                        Some((left.get(prev_ws_num as usize), right.first().unwrap()))
                    } else {
                        let (left, right) = self.workspaces.split_at(prev_ws_num as usize);
                        Some((right.first(), left.get(new_ws_num as usize).unwrap()))
                    }
                }
                (Some(_), None) => None,
                (None, Some(new_ws_num)) => Some((None, self.workspaces.get(new_ws_num as usize).unwrap())),
                (None, None) => None,
            }
        } else {
            None
        }
    }

    pub fn scrolled_for_switch(&mut self, amount: f64) -> i32 {
        self.scroll_accum.accumulate(amount)
    }

    pub fn reset_scroll_amount(&mut self) {
        self.scroll_accum.reset();
    }

    /// Returns the grid position of the workspace in the specified
    /// direction, or None if there is no neighbor (either because we're
    /// at the grid edge with wrap=false, or because wrapping would
    /// return to the same workspace).
    fn position_for_direction(&self, from_workspace: &Workspace, direction: Direction, wrap: bool) -> Option<Point<u32, Logical>> {
        let cur_pos = from_workspace.position();
        let cols = self.geometry.w;
        let rows = self.geometry.h;
        let n = self.workspaces.len() as u32;

        if n <= 1 {
            None
        } else {
            let (new_col, new_row) = match direction {
                Direction::Left => {
                    if cur_pos.x > 0 {
                        let col = cur_pos.x - 1;
                        (col, cur_pos.y)
                    } else if wrap {
                        let col = cols - 1;
                        if workspace_index_for_position(col, cur_pos.y, self.geometry, n).is_some() {
                            (col, cur_pos.y)
                        } else {
                            let last_col = (n - 1) % cols;
                            (last_col, cur_pos.y)
                        }
                    } else {
                        return None;
                    }
                }
                Direction::Right => {
                    let col = cur_pos.x + 1;
                    if workspace_index_for_position(col, cur_pos.y, self.geometry, n).is_some() {
                        (col, cur_pos.y)
                    } else if wrap {
                        (0, cur_pos.y)
                    } else {
                        return None;
                    }
                }
                Direction::Up => {
                    if cur_pos.y > 0 {
                        let mut row = cur_pos.y - 1;
                        while workspace_index_for_position(cur_pos.x, row, self.geometry, n).is_none() {
                            if row > 0 {
                                row -= 1;
                            } else {
                                return None;
                            }
                        }
                        (cur_pos.x, row)
                    } else if wrap {
                        let mut row = rows - 1;
                        while workspace_index_for_position(cur_pos.x, row, self.geometry, n).is_none() {
                            if row > 0 {
                                row -= 1;
                            } else {
                                return None;
                            }
                        }
                        (cur_pos.x, row)
                    } else {
                        return None;
                    }
                }
                Direction::Down => {
                    let mut row = cur_pos.y + 1;
                    if workspace_index_for_position(cur_pos.x, row, self.geometry, n).is_some() {
                        (cur_pos.x, row)
                    } else if wrap {
                        row = 0;
                        while workspace_index_for_position(cur_pos.x, row, self.geometry, n).is_none() {
                            row += 1;
                            if row >= rows {
                                return None;
                            }
                        }
                        (cur_pos.x, row)
                    } else {
                        return None;
                    }
                }
            };

            if new_col == cur_pos.x && new_row == cur_pos.y {
                None
            } else {
                Some((new_col, new_row).into())
            }
        }
    }

    pub fn workspace_index_for_direction(&self, direction: Direction, wrap: bool) -> Option<u32> {
        self.position_for_direction(self.active_workspace(), direction, wrap)
            .and_then(|position| workspace_index_for_position(position.x, position.y, self.geometry, self.workspaces.len() as u32))
    }

    pub fn sequential_workspace_index(&self, steps: i32, wrap: bool) -> Option<u32> {
        let n = self.workspaces.len() as i32;
        let new_index = self.active_space as i32 + steps;
        if wrap {
            Some(new_index.rem_euclid(n) as u32)
        } else if new_index >= 0 && new_index < n {
            Some(new_index as u32)
        } else {
            None
        }
    }

    pub fn next_workspace_index(&self, wrap: bool) -> Option<u32> {
        self.sequential_workspace_index(1, wrap)
    }

    pub fn prev_workspace_index(&self, wrap: bool) -> Option<u32> {
        self.sequential_workspace_index(-1, wrap)
    }

    pub fn active_workspace_index(&self) -> u32 {
        self.active_space
    }

    pub fn previous_active_workspace_index(&self) -> u32 {
        self.previous_active_space
    }

    pub fn active_workspace(&self) -> &Workspace {
        self.workspaces
            .get(self.active_space as usize)
            .expect("active_space should not be out of range")
    }

    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        self.workspaces
            .get_mut(self.active_space as usize)
            .expect("active_space should not be out of range")
    }

    fn update_geometry(&mut self, nrows: u32, nworkspaces: u32) {
        self.geometry = (nworkspaces.div_ceil(nrows), nrows).into();
    }

    pub(super) fn add_workspace(&mut self) {
        let count = self.workspaces.len();
        self.insert_workspace(count as u32);
    }

    pub(super) fn insert_workspace(&mut self, index: u32) {
        let count = self.workspaces.len() as u32;

        if index == count {
            // Let the xfconf callbacks handle everything.
            self.set_xfconf_workspace_count(count + 1);
        } else {
            // If we're inserting at or below the current workspace, increment active_space so we
            // don't switch workspaces because of this.  Also update previous_active space if it's
            // in the same situation, so toggling will send the user to the right place.
            if index <= self.active_space {
                // This is one of the *only* times it's ok to set this directly and not go through
                // the setter.
                self.active_space += 1;
            }
            if index <= self.previous_active_space {
                // Ditto.
                self.previous_active_space += 1;
            }
            self.update_geometry(self.geometry.h, count + 1);

            let new_name = format!("Workspace {}", index + 1);

            // Insert the new workspace ourselves, because the xfconf handler will just append to
            // the end.
            let new_position = position_for_workspace_index(index, self.geometry, count + 1);
            let mut new_workspace = Workspace::new(&new_name, new_position);
            for output in self.outputs() {
                if let Some(output_geom) = self.output_geometry(output) {
                    new_workspace.map_output(output, output_geom.loc);
                }
            }

            self.workspaces.insert(index as usize, new_workspace);
            self.set_xfconf_workspace_count(count + 1);

            // Add a new workspace name so the other workspaces don't change names.
            let mut names = self
                .workspaces
                .iter()
                .map(|workspace| workspace.name().to_owned())
                .collect::<Vec<_>>();
            names.insert(index as usize, new_name);
            self.set_xfconf_workspace_names(names);

            let workspace = self.workspaces.get(index as usize).unwrap();
            self.ext_workspace_state.workspace_created(WorkspaceCreatedInput {
                id: workspace.id(),
                name: workspace.name(),
                coordinates: workspace.position(),
                is_active: false,
            });

            // Now update all the other workspace coordinates.
            for (i, workspace) in self.workspaces.iter_mut().enumerate().map(|(i, workspace)| (i as u32, workspace)) {
                update_workspace_position(workspace, i, count + 1, self.geometry, &mut self.ext_workspace_state);
            }

            // Map any sticky windows into the new workspace.
            let (visible_sticky_windows, minimized_sticky_windows) = {
                let active_workspace = self.workspaces.get(self.active_space as usize).unwrap();
                let visible = active_workspace
                    .visible_windows()
                    .filter(|window| window.sticky())
                    .map(|window| (window.clone(), active_workspace.window_location(window).unwrap_or_default()))
                    .collect::<Vec<_>>();
                let minimized = active_workspace
                    .minimized_windows()
                    .filter(|window| window.sticky())
                    .map(|window| {
                        (
                            window.clone(),
                            active_workspace.minimized_window_location(window).unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>();
                (visible, minimized)
            };

            let workspace = self.workspaces.get_mut(index as usize).unwrap();
            for (window, location) in visible_sticky_windows {
                // TODO: need to make sure parents are mapped before children
                let parent = window.parent();
                workspace.map_window(window, location, false, parent.as_ref());
            }
            for (window, location) in minimized_sticky_windows {
                workspace.add_minimized_window(window, location);
            }
        }
    }

    pub(super) fn remove_workspace(&mut self, index: u32) -> Option<u32> {
        let count = self.workspaces.len() as u32;

        if count == 1 {
            // Never remove the last workspace.
            None
        } else if index == count - 1 {
            // Let the xfconf callbacks handle everything.
            self.set_xfconf_workspace_count(count - 1);
            None
        } else if index < count {
            let removed_workspace = self.workspaces.remove(index as usize);

            let target_workspace_index = index.saturating_sub(1);
            let target_workspace = self.workspaces.get_mut(target_workspace_index as usize).unwrap();

            // Move non-sticky windows to the target workspace.
            for window in removed_workspace.visible_windows().cloned() {
                if !window.sticky() {
                    // TODO: need to make sure parents are mapped before children
                    let location = removed_workspace.window_location(&window).unwrap_or_default();
                    let parent = window.parent();
                    target_workspace.map_window(window, location, false, parent.as_ref());
                }
            }
            for window in removed_workspace.minimized_windows().cloned() {
                if !window.sticky() {
                    let location = removed_workspace.minimized_window_location(&window).unwrap_or_default();
                    target_workspace.add_minimized_window(window, location);
                }
            }

            self.set_xfconf_workspace_count(count - 1);
            // Update the workspace names list so other existing workspaces don't change names.
            let names = self
                .workspaces
                .iter()
                .map(|workspace| workspace.name().to_owned())
                .collect::<Vec<_>>();
            self.set_xfconf_workspace_names(names);

            self.ext_workspace_state.workspace_destroyed(removed_workspace.id());

            // Now update all the workspace coordinates.
            for (i, workspace) in self.workspaces.iter_mut().enumerate().map(|(i, workspace)| (i as u32, workspace)) {
                update_workspace_position(workspace, i, count - 1, self.geometry, &mut self.ext_workspace_state);
            }

            let switch_to_workspace = if self.active_space == index {
                // We removed the active workspace, so switch to the workspace where we moved all
                // the windows to.
                // FIXME: We aren't able to reset active/focus on the new workspace from here.
                Some(target_workspace_index)
            } else if self.active_space > index {
                //  We removed a workspace "before" the active one, so to keep ourselves on the
                //  active workspace, we have to decrement the active_space.  This is one of the
                //  *only* times it's ok to set this directly and not go through the setter.
                self.active_space -= 1;
                None
            } else {
                None
            };

            if self.previous_active_space == index {
                // The previously active workspace is gone, so reset it to the current workspace
                // since there's nothing to toggle back to.
                self.previous_active_space = self.active_space;
            } else if self.previous_active_space > index {
                // We removed a workspace "before" the previous one, so to keep the previous
                // workspace pointing to the same workspace, decrement it.  This is one of the
                // *only* times it's ok to set this directly and not go through the setter.
                self.previous_active_space -= 1;
            }

            switch_to_workspace
        } else {
            None
        }
    }

    pub fn refresh_spaces(&mut self) {
        profiling::scope!("refresh_spaces");
        for workspace in &mut self.workspaces {
            workspace.refresh();
        }
    }

    pub fn find_window<P>(&self, predicate: P) -> Option<WindowElement>
    where
        P: Fn(&WindowElement) -> bool + Clone,
    {
        self.workspaces
            .iter()
            .find_map(|workspace| workspace.find_window(predicate.clone()))
    }

    pub fn find_window_and_workspace<P>(&mut self, predicate: P) -> Option<(WindowElement, u32, &Workspace)>
    where
        P: Fn(&WindowElement) -> bool + Clone,
    {
        self.workspaces.iter().enumerate().find_map(|(index, workspace)| {
            workspace
                .find_window(predicate.clone())
                .map(|window| (window, index as u32, workspace))
        })
    }

    pub fn find_window_and_workspace_mut<P>(&mut self, predicate: P) -> Option<(WindowElement, u32, &mut Workspace)>
    where
        P: Fn(&WindowElement) -> bool + Clone,
    {
        self.workspaces.iter_mut().enumerate().find_map(|(index, workspace)| {
            workspace
                .find_window(predicate.clone())
                .map(|window| (window, index as u32, workspace))
        })
    }

    pub fn outputs_for_window(&self, window: &WindowElement) -> Vec<Output> {
        self.workspaces
            .iter()
            .find_map(|workspace| {
                let outputs = workspace.outputs_for_window(window);
                (!outputs.is_empty()).then_some(outputs)
            })
            .unwrap_or_else(Vec::new)
    }

    pub fn workspace_for_window_with_index(&self, window: &WindowElement) -> Option<(u32, &Workspace)> {
        if self
            .active_workspace()
            .window_location(window)
            .or_else(|| self.active_workspace().minimized_window_location(window))
            .is_some()
        {
            Some((self.active_space, self.active_workspace()))
        } else {
            self.workspaces().iter().enumerate().find_map(|(i, workspace)| {
                workspace
                    .window_location(window)
                    .or_else(|| workspace.minimized_window_location(window))
                    .map(|_| (i as u32, workspace))
            })
        }
    }

    pub fn workspace_for_window(&mut self, window: &WindowElement) -> Option<&Workspace> {
        if self
            .active_workspace()
            .window_location(window)
            .or_else(|| self.active_workspace().minimized_window_location(window))
            .is_some()
        {
            Some(self.active_workspace())
        } else {
            self.workspaces().iter().find(|workspace| {
                workspace
                    .window_location(window)
                    .or_else(|| workspace.minimized_window_location(window))
                    .is_some()
            })
        }
    }

    pub fn workspace_for_window_mut(&mut self, window: &WindowElement) -> Option<&mut Workspace> {
        if self
            .active_workspace()
            .window_location(window)
            .or_else(|| self.active_workspace().minimized_window_location(window))
            .is_some()
        {
            Some(self.active_workspace_mut())
        } else {
            self.workspaces_mut().iter_mut().find(|workspace| {
                workspace
                    .window_location(window)
                    .or_else(|| workspace.minimized_window_location(window))
                    .is_some()
            })
        }
    }

    pub fn workspace_for_window_with_index_mut(&mut self, window: &WindowElement) -> Option<(u32, &mut Workspace)> {
        if self
            .active_workspace()
            .window_location(window)
            .or_else(|| self.active_workspace().minimized_window_location(window))
            .is_some()
        {
            Some((self.active_space, self.active_workspace_mut()))
        } else {
            self.workspaces_mut().iter_mut().enumerate().find_map(|(i, workspace)| {
                workspace
                    .window_location(window)
                    .or_else(|| workspace.minimized_window_location(window))
                    .map(|_| (i as u32, workspace))
            })
        }
    }

    pub fn move_window_by_index(&mut self, window: &WindowElement, old_index: u32, new_index: u32) -> bool {
        let count = self.workspaces.len() as u32;
        if old_index < count && new_index < count && old_index != new_index {
            let mut props = window.props();

            match props.workspace_loc {
                WorkspaceLocation::Single(_) => {
                    let workspace = self.workspaces.get_mut(old_index as usize).unwrap();
                    let location = workspace.window_location(window).unwrap_or_default();
                    let parent = window.parent();
                    workspace.unmap_window(window);

                    let workspace = self.workspaces.get_mut(new_index as usize).unwrap();
                    // TODO: need to make sure parents are mapped before children
                    workspace.map_window(window.clone(), location, true, parent.as_ref());

                    props.workspace_loc = WorkspaceLocation::Single(new_index);
                }

                WorkspaceLocation::All => (),
            }

            true
        } else {
            false
        }
    }

    pub(super) fn move_window_by_direction(&mut self, window: &WindowElement, direction: Direction, wrap: bool) -> Option<u32> {
        let geometry = self.geometry;
        let count = self.workspaces.len() as u32;

        if let Some((old_index, workspace)) = self.workspace_for_window_with_index(window)
            && let Some(new_pos) = self.position_for_direction(workspace, direction, wrap)
            && let Some(new_index) = workspace_index_for_position(new_pos.x, new_pos.y, geometry, count)
        {
            self.move_window_by_index(window, old_index, new_index).then_some(new_index)
        } else {
            None
        }
    }

    pub(super) fn move_window_to(&mut self, window: &WindowElement, new_index: u32) -> bool {
        if let Some((old_index, _)) = self.workspace_for_window_with_index(window) {
            self.move_window_by_index(window, old_index, new_index)
        } else {
            false
        }
    }

    pub(super) fn move_window_next(&mut self, window: &WindowElement, wrap: bool) -> Option<u32> {
        if let Some((old_index, _)) = self.workspace_for_window_with_index(window) {
            let n = self.workspaces.len() as u32;
            let new_index = if old_index + 1 < n {
                Some(old_index + 1)
            } else if wrap {
                Some(0)
            } else {
                None
            };
            new_index.and_then(|new_index| self.move_window_by_index(window, old_index, new_index).then_some(new_index))
        } else {
            None
        }
    }

    pub(super) fn move_window_previous(&mut self, window: &WindowElement, wrap: bool) -> Option<u32> {
        if let Some((old_index, _)) = self.workspace_for_window_with_index(window) {
            let new_index = if old_index > 0 {
                Some(old_index - 1)
            } else if wrap {
                Some(self.workspaces.len() as u32 - 1)
            } else {
                None
            };
            new_index.and_then(|new_index| self.move_window_by_index(window, old_index, new_index).then_some(new_index))
        } else {
            None
        }
    }

    pub(super) fn new_window<P: Into<Point<i32, Logical>>>(
        &mut self,
        window: WindowElement,
        location: P,
        activate: bool,
        workspace_number: Option<u32>,
        parent: Option<&WindowElement>,
    ) {
        let (ws_num, workspace) = if let Some((ws_num, workspace)) =
            workspace_number.and_then(|num| self.workspaces.get_mut(num as usize).map(|workspace| (num, workspace)))
        {
            (ws_num, workspace)
        } else {
            (self.active_space, self.workspaces.get_mut(self.active_space as usize).unwrap())
        };

        if let Some(parent) = parent {
            window.props().workspace_loc = parent.props().workspace_loc;
        } else {
            window.props().workspace_loc = WorkspaceLocation::Single(ws_num);
        }
        workspace.map_window(window.clone(), location, activate, parent);
        self.update_window_decorations_scale(&window);
    }

    pub(super) fn remove_window(&mut self, window: &WindowElement) {
        for workspace in self.workspaces_mut() {
            workspace.unmap_window(window);
            workspace.remove_minimized_window(window);
        }
    }

    pub fn relocate_window<P: Into<Point<i32, Logical>>>(&mut self, window: &WindowElement, location: P, activate: bool) {
        let location = location.into();
        for workspace in self.workspaces_mut() {
            workspace.relocate_window(window, location, activate);
        }
        self.update_window_decorations_scale(window);
    }

    /// The output whose scale the window's decorations should be drawn at: whichever shows the most
    /// of the titlebar (the most visible, detailed part of the decoration), so it stays crisp even
    /// when the window straddles outputs of different scales.  Before the window's geometry has been
    /// committed (e.g. the moment it is first mapped) we fall back to its mapped location, which is
    /// valid immediately, so a window opening on a non-default output still picks the right scale.
    /// We intersect against all outputs geometrically rather than the window's tracked output set,
    /// which isn't populated until the window has been through a refresh cycle.
    pub(in crate::core) fn decorations_scale_for_window(&self, window: &WindowElement) -> OutputScale {
        let geometry = self.window_geometry(window);
        let titlebar = window
            .decoration_state()
            .window_decorations()
            .map(|d| d.decorations_extents().top)
            .filter(|top| *top > 0)
            .zip(geometry.filter(|geom| !geom.size.is_empty()))
            .map(|(top, geom)| Rectangle::new(geom.loc, Size::from((geom.size.w, top))));
        let region = titlebar.or_else(|| geometry.map(|geom| Rectangle::new(geom.loc, Size::from((1, 1)))));

        region
            .and_then(|region| {
                self.outputs()
                    .filter_map(|output| {
                        self.output_geometry(output)
                            .and_then(|geom| geom.intersection(region))
                            .map(|overlap| (output, overlap.size.w * overlap.size.h))
                    })
                    .max_by_key(|(_, area)| *area)
                    .map(|(output, _)| output.current_scale())
            })
            .or_else(|| self.outputs().next().map(|output| output.current_scale()))
            .unwrap_or(OutputScale::Integer(1))
    }

    /// The scale of the output containing `pos` (global logical), used to hit-test decorations
    /// against the layout that output actually rendered (decorations are native px per output).
    /// Falls back to the first output, then to scale 1.
    pub(in crate::core) fn output_scale_at(&self, pos: Point<f64, Logical>) -> OutputScale {
        let pos = pos.to_i32_round();
        self.outputs()
            .find(|output| self.output_geometry(output).is_some_and(|geom| geom.contains(pos)))
            .or_else(|| self.outputs().next())
            .map(|output| output.current_scale())
            .unwrap_or(OutputScale::Integer(1))
    }

    /// Recomputes a window's decoration scale from its current output and applies it.  Called
    /// whenever the window's primary output can change (mapping, moving, output reconfiguration);
    /// `WindowDecorations::update` is a no-op when the scale is unchanged.
    pub(in crate::core) fn update_window_decorations_scale(&self, window: &WindowElement) {
        let scale = self.decorations_scale_for_window(window);
        if let Some(decorations) = window.decoration_state_mut().window_decorations_mut() {
            decorations.update(DecorationInput::Scale(scale));
        }
    }

    pub(super) fn set_window_stacking_layer(&mut self, window: &WindowElement, layer: WindowStackingLayer) {
        if window.z_index() != layer as u8 {
            window.0.override_z_index(layer as u8);

            if !window.sticky()
                && let Some(workspace) = self.workspace_for_window_mut(window)
            {
                workspace.raise_window(window, false);
            } else {
                for workspace in self.workspaces.iter_mut() {
                    workspace.raise_window(window, false);
                }
            }
        }
    }

    pub(super) fn set_window_minimized(&mut self, window: &WindowElement) -> bool {
        self.workspaces.iter_mut().any(|workspace| workspace.set_window_minimized(window))
    }

    pub(super) fn set_window_unminimized(&mut self, window: &WindowElement, activate: bool) -> bool {
        self.workspaces
            .iter_mut()
            .any(|workspace| workspace.set_window_unminimized(window, activate))
    }

    pub(super) fn set_window_fullscreen(&mut self, window: &WindowElement, output: &Output) -> Vec<WindowElement> {
        if !window.sticky() {
            if let Some(workspace) = self.workspace_for_window_mut(window) {
                workspace.set_window_fullscreen(window, output).into_iter().collect()
            } else {
                vec![]
            }
        } else {
            self.workspaces
                .iter_mut()
                .flat_map(|workspace| workspace.set_window_fullscreen(window, output))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect()
        }
    }

    pub(super) fn set_window_unfullscreen(&mut self, window: &WindowElement) -> Vec<Output> {
        if !window.sticky() {
            if let Some(workspace) = self.workspace_for_window_mut(window) {
                workspace.set_window_unfullscreen(window).into_iter().collect()
            } else {
                vec![]
            }
        } else {
            self.workspaces
                .iter_mut()
                .flat_map(|workspace| workspace.set_window_unfullscreen(window))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect()
        }
    }

    pub(super) fn set_window_workspace_num(&mut self, window: &WindowElement, workspace_loc: WorkspaceLocation) {
        let is_minimized = window.minimized();

        let workspace_loc = match workspace_loc {
            WorkspaceLocation::All => {
                if let Some((ws_num, workspace)) = self.workspace_for_window_with_index(window)
                    && let Some(location) = if !is_minimized {
                        workspace.window_location(window)
                    } else {
                        workspace.minimized_window_location(window)
                    }
                {
                    for (i, workspace) in self.workspaces_mut().iter_mut().enumerate() {
                        if ws_num as usize != i {
                            if !is_minimized {
                                // TODO: need to make sure parent is mapped before children
                                let parent = window.parent();
                                workspace.map_window(window.clone(), location, true, parent.as_ref());
                            } else {
                                workspace.add_minimized_window(window.clone(), location);
                            }
                        }
                    }

                    Some(WorkspaceLocation::All)
                } else {
                    None
                }
            }

            WorkspaceLocation::Single(ws_num) => {
                let ws_num = if (ws_num as usize) < self.workspaces.len() {
                    ws_num
                } else {
                    self.active_workspace_index()
                };

                for (i, workspace) in self.workspaces_mut().iter_mut().enumerate() {
                    if ws_num as usize != i {
                        if !is_minimized {
                            workspace.unmap_window(window);
                        } else {
                            workspace.remove_minimized_window(window);
                        }
                    }
                }

                Some(WorkspaceLocation::Single(ws_num))
            }
        };

        if let Some(workspace_loc) = workspace_loc {
            window.props().workspace_loc = workspace_loc;
        }
    }

    pub fn window_geometry(&self, window: &WindowElement) -> Option<Rectangle<i32, Logical>> {
        self.workspaces.iter().find_map(|workspace| workspace.window_geometry(window))
    }

    pub(in crate::core) fn window_location(&self, window: &WindowElement) -> Option<Point<i32, Logical>> {
        self.workspaces.iter().find_map(|workspace| workspace.window_location(window))
    }

    fn get_workspace_names_uncached(&self) -> Vec<String> {
        self.channel
            .get_property::<Vec<String>>(PROP_WORKSPACE_NAMES)
            .unwrap_or_else(Vec::new)
    }

    fn set_xfconf_workspace_count(&self, num: u32) {
        self.channel.set_property(PROP_WORKSPACE_COUNT, num as i32);
    }

    fn set_xfconf_workspace_names(&self, names: Vec<String>) {
        self.channel.set_property(PROP_WORKSPACE_NAMES, names);
    }

    fn on_workspace_count_changed(&mut self, new_count: u32) -> Option<u32> {
        assert!(self.workspaces.len() <= i32::MAX as usize);
        let old_count = self.workspaces.len() as u32;
        self.update_geometry(self.geometry.h, new_count);

        if new_count > old_count {
            let names = self.get_workspace_names_uncached();
            let outputs = self
                .outputs()
                .flat_map(|output| self.output_geometry(output).map(|geom| (output.clone(), geom)))
                .collect::<Vec<_>>();

            let start = old_count;
            let new_workspaces = zip_all_first(start..new_count, names.into_iter().skip(start as usize)).map(|(i, name)| {
                let name = name.unwrap_or_else(|| format!("Workspace {}", i + 1));
                let position = position_for_workspace_index(i, self.geometry, new_count);
                let mut new_workspace = Workspace::new(name, position);
                for (output, geom) in &outputs {
                    new_workspace.map_output(output, geom.loc);
                }
                new_workspace
            });

            self.workspaces.extend(new_workspaces);

            for (i, workspace) in self.workspaces.iter_mut().enumerate().map(|(i, workspace)| (i as u32, workspace)) {
                if i < old_count {
                    update_workspace_position(workspace, i, new_count, self.geometry, &mut self.ext_workspace_state);
                } else {
                    self.ext_workspace_state.workspace_created(WorkspaceCreatedInput {
                        id: workspace.id(),
                        name: workspace.name(),
                        coordinates: workspace.position(),
                        is_active: false,
                    });
                }
            }

            None
        } else if new_count < old_count {
            let removed = self.workspaces.split_off(new_count as usize);
            let target_workspace = self.workspaces.last_mut().unwrap();

            for removed_workspace in removed.into_iter().rev() {
                // Move non-sticky windows to the target workspace.
                for window in removed_workspace.visible_windows().cloned() {
                    if !window.sticky() {
                        let location = removed_workspace.window_location(&window).unwrap_or_default();
                        // TODO: need to make sure parents are mapped before children
                        let parent = window.parent();
                        target_workspace.map_window(window, location, false, parent.as_ref());
                    }
                }
                for window in removed_workspace.minimized_windows().cloned() {
                    if !window.sticky() {
                        let location = removed_workspace.minimized_window_location(&window).unwrap_or_default();
                        target_workspace.add_minimized_window(window, location);
                    }
                }

                self.ext_workspace_state.workspace_destroyed(removed_workspace.id());
            }

            for (i, workspace) in self.workspaces.iter_mut().enumerate().map(|(i, workspace)| (i as u32, workspace)) {
                update_workspace_position(workspace, i, new_count, self.geometry, &mut self.ext_workspace_state);
            }

            (self.active_space >= new_count).then_some(new_count - 1)
        } else {
            None
        }
    }

    fn on_workspace_names_changed(&mut self, new_names: Vec<String>) {
        for (i, (workspace, new_name)) in zip_all_first(self.workspaces.iter_mut(), new_names).enumerate() {
            let new_name = new_name.unwrap_or_else(|| format!("Workspace {}", i + 1));
            if new_name != workspace.name() {
                workspace.set_name(new_name);
                self.ext_workspace_state.workspace_changed(
                    workspace.id(),
                    WorkspaceChangedInput {
                        name: Some(workspace.name()),
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn on_workspace_num_rows_changed(&mut self, new_nrows: u32) {
        if new_nrows != self.geometry.h {
            let nworkspaces = self.workspaces.len() as u32;
            self.update_geometry(new_nrows, nworkspaces);

            for (i, workspace) in self.workspaces.iter_mut().enumerate().map(|(i, workspace)| (i as u32, workspace)) {
                update_workspace_position(workspace, i, nworkspaces, self.geometry, &mut self.ext_workspace_state);
            }
        }
    }
}

impl<BackendData: Backend + 'static> ExtWorkspaceHandler for Xfwl4State<BackendData> {
    fn ext_workspace_state(&mut self) -> &mut ExtWorkspaceState<Self> {
        &mut self.core.workspace_manager.ext_workspace_state
    }

    fn on_workspace_activate(&mut self, workspace_id: &str) {
        if let Some(workspace_num) = self
            .core
            .workspace_manager
            .workspaces
            .iter()
            .position(|workspace| workspace.id() == workspace_id)
        {
            self.set_active_workspace(workspace_num as u32);
        }
    }

    fn on_workspace_deactivate(&mut self, _workspace_id: &str) {
        // We don't support deactivating a workspace without activating another, so we just do
        // nothing here.
    }
}

#[inline]
fn position_for_workspace_index(index: u32, geometry: Size<u32, Logical>, nworkspaces: u32) -> Point<u32, Logical> {
    debug_assert!(nworkspaces > 0);
    debug_assert!(geometry.w > 0);
    debug_assert!(geometry.h > 0);

    let y = index / geometry.w;
    let x = index % geometry.w;
    Point::new(x, y)
}

#[inline]
fn workspace_index_for_position(col: u32, row: u32, geometry: Size<u32, Logical>, nworkspaces: u32) -> Option<u32> {
    debug_assert!(nworkspaces > 0);
    debug_assert!(geometry.w > 0);
    debug_assert!(geometry.h > 0);

    if row < geometry.h && col < geometry.w {
        let index = row * geometry.w + col;
        (index < nworkspaces).then_some(index)
    } else {
        None
    }
}

fn update_workspace_position<BackendData: Backend + 'static>(
    workspace: &mut Workspace,
    index: u32,
    nworkspaces: u32,
    geometry: Size<u32, Logical>,
    ext_workspace_state: &mut ExtWorkspaceState<Xfwl4State<BackendData>>,
) {
    let new_position = position_for_workspace_index(index, geometry, nworkspaces);
    if new_position != workspace.position() {
        workspace.set_position(new_position);
        ext_workspace_state.workspace_changed(
            workspace.id(),
            WorkspaceChangedInput {
                coordinates: Some(new_position),
                ..Default::default()
            },
        );
    }
}
