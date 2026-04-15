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

use smithay::{
    output::Output,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Rectangle},
    wayland::shell::xdg::ToplevelState,
};

use crate::{
    backend::Backend,
    core::{shell::WindowElement, workspaces::WorkspaceManager},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileMode {
    Left,
    Right,
    Up,
    Down,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl TileMode {
    pub fn geometry_in_zone(&self, zone: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        match self {
            TileMode::Up => Rectangle::new(zone.loc, (zone.size.w, zone.size.h / 2).into()),
            TileMode::Left => Rectangle::new(zone.loc, (zone.size.w / 2, zone.size.h).into()),
            TileMode::Right => Rectangle::new(
                (zone.loc.x + zone.size.w / 2, zone.loc.y).into(),
                (zone.size.w - zone.size.w / 2, zone.size.h).into(),
            ),
            TileMode::Down => Rectangle::new(
                (zone.loc.x, zone.loc.y + zone.size.h / 2).into(),
                (zone.size.w, zone.size.h - zone.size.h / 2).into(),
            ),
            TileMode::UpLeft => Rectangle::new(zone.loc, zone.size / 2),
            TileMode::UpRight => Rectangle::new(
                (zone.loc.x + zone.size.w / 2, zone.loc.y).into(),
                (zone.size.w - zone.size.w / 2, zone.size.h / 2).into(),
            ),
            TileMode::DownLeft => Rectangle::new(
                (zone.loc.x, zone.loc.y + zone.size.h / 2).into(),
                (zone.size.w / 2, zone.size.h - zone.size.h / 2).into(),
            ),
            TileMode::DownRight => Rectangle::new(
                (zone.loc.x + zone.size.w / 2, zone.loc.y + zone.size.h / 2).into(),
                (zone.size.w - zone.size.w / 2, zone.size.h - zone.size.h / 2).into(),
            ),
        }
    }

    pub fn as_xdg_toplevel_states(&self) -> &'static [xdg_toplevel::State] {
        match self {
            TileMode::Left => &[
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledBottom,
                xdg_toplevel::State::ConstrainedLeft,
                xdg_toplevel::State::ConstrainedTop,
                xdg_toplevel::State::ConstrainedBottom,
            ],
            TileMode::Right => &[
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledBottom,
                xdg_toplevel::State::ConstrainedRight,
                xdg_toplevel::State::ConstrainedTop,
                xdg_toplevel::State::ConstrainedBottom,
            ],
            TileMode::Up => &[
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::ConstrainedLeft,
                xdg_toplevel::State::ConstrainedTop,
                xdg_toplevel::State::ConstrainedRight,
            ],
            TileMode::Down => &[
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::TiledBottom,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::ConstrainedLeft,
                xdg_toplevel::State::ConstrainedBottom,
                xdg_toplevel::State::ConstrainedRight,
            ],
            TileMode::UpLeft => &[
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::ConstrainedTop,
                xdg_toplevel::State::ConstrainedLeft,
            ],
            TileMode::UpRight => &[
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::ConstrainedTop,
                xdg_toplevel::State::ConstrainedRight,
            ],
            TileMode::DownLeft => &[
                xdg_toplevel::State::TiledBottom,
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::ConstrainedBottom,
                xdg_toplevel::State::ConstrainedLeft,
            ],
            TileMode::DownRight => &[
                xdg_toplevel::State::TiledBottom,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::ConstrainedBottom,
                xdg_toplevel::State::ConstrainedRight,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowLayout {
    Normal,
    Maximized,
    Tiled(TileMode),
}

impl WindowLayout {
    pub fn geometry_in_zone(&self, zone: Rectangle<i32, Logical>) -> Option<Rectangle<i32, Logical>> {
        match self {
            WindowLayout::Normal => None,
            WindowLayout::Maximized => Some(zone),
            WindowLayout::Tiled(mode) => Some(mode.geometry_in_zone(zone)),
        }
    }

    pub fn as_xdg_toplevel_states(&self) -> &'static [xdg_toplevel::State] {
        match self {
            WindowLayout::Normal => &[],
            WindowLayout::Maximized => &[xdg_toplevel::State::Maximized],
            WindowLayout::Tiled(mode) => mode.as_xdg_toplevel_states(),
        }
    }
}

pub fn remove_tiled_states(state: &mut ToplevelState) {
    for s in [
        xdg_toplevel::State::TiledLeft,
        xdg_toplevel::State::TiledRight,
        xdg_toplevel::State::TiledTop,
        xdg_toplevel::State::TiledBottom,
        xdg_toplevel::State::ConstrainedLeft,
        xdg_toplevel::State::ConstrainedRight,
        xdg_toplevel::State::ConstrainedTop,
        xdg_toplevel::State::ConstrainedBottom,
    ] {
        state.states.unset(s);
    }
}

pub fn remove_all_layout_states(state: &mut ToplevelState) {
    remove_tiled_states(state);
    state.states.unset(xdg_toplevel::State::Maximized);
}

pub fn output_and_geom_for_anchored_layout<BackendData: Backend + 'static>(
    workspace_manager: &WorkspaceManager<BackendData>,
    window: &WindowElement,
) -> Option<(Output, Rectangle<i32, Logical>)> {
    window
        .props()
        .anchored_output
        .as_ref()
        .and_then(|weak| weak.upgrade())
        .or_else(|| workspace_manager.outputs_for_window(window).first().cloned())
        .or_else(|| workspace_manager.outputs().next().cloned())
        .and_then(|output| workspace_manager.output_geometry(&output).map(|geom| (output, geom)))
}
