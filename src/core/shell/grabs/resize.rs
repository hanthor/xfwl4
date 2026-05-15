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
    sync::{Arc, Mutex},
};

use smithay::{
    backend::input::KeyState,
    desktop::{WindowSurface, space::SpaceElement},
    input::{
        SeatHandler,
        keyboard::{GrabStartData as KeyboardGrabStartData, KeyboardGrab, KeyboardInnerHandle, ModifiersState},
        pointer::{
            AxisFrame, ButtonEvent, CursorImageStatus, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
            GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent,
            GrabStartData as PointerGrabStartData, MotionEvent, PointerGrab, PointerInnerHandle, RelativeMotionEvent,
        },
        touch::{GrabStartData as TouchGrabStartData, TouchGrab},
    },
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{IsAlive, Logical, Point, SERIAL_COUNTER, Serial, Size},
    wayland::compositor::with_states,
};
#[cfg(feature = "xwayland")]
use smithay::{utils::Rectangle, xwayland::xwm::ResizeEdge as X11ResizeEdge};
use xkbcommon::xkb::Keycode;

use smithay::desktop::layer_map_for_output;

use crate::{
    backend::Backend,
    core::{
        cursor::CursorName,
        drawing::wireframe::Wireframe,
        focus::PointerFocusTarget,
        shell::{
            SurfaceData, WindowElement,
            grabs::common::{MoveResizeAction, keyboard_move_resize_get_action},
        },
        snap,
        state::Xfwl4State,
    },
};

const KEY_RESIZE_BASE: i32 = 10;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ResizeEdge: u32 {
        const TOP = 1;
        const LEFT = 2;
        const RIGHT = 4;
        const BOTTOM = 8;

        const TOP_LEFT = Self::TOP.bits() | Self::LEFT.bits();
        const TOP_RIGHT = Self::TOP.bits() | Self::RIGHT.bits();
        const BOTTOM_LEFT = Self::BOTTOM.bits() | Self::LEFT.bits();
        const BOTTOM_RIGHT = Self::BOTTOM.bits() | Self::RIGHT.bits();
    }
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xdg_toplevel::ResizeEdge) -> Self {
        match x {
            xdg_toplevel::ResizeEdge::None => Self::empty(),
            xdg_toplevel::ResizeEdge::Top => Self::TOP,
            xdg_toplevel::ResizeEdge::Left => Self::LEFT,
            xdg_toplevel::ResizeEdge::Right => Self::RIGHT,
            xdg_toplevel::ResizeEdge::Bottom => Self::BOTTOM,
            xdg_toplevel::ResizeEdge::TopLeft => Self::TOP_LEFT,
            xdg_toplevel::ResizeEdge::TopRight => Self::TOP_RIGHT,
            xdg_toplevel::ResizeEdge::BottomLeft => Self::BOTTOM_LEFT,
            xdg_toplevel::ResizeEdge::BottomRight => Self::BOTTOM_RIGHT,
            _ => Self::empty(),
        }
    }
}

impl From<ResizeEdge> for xdg_toplevel::ResizeEdge {
    #[inline]
    fn from(x: ResizeEdge) -> Self {
        match x {
            ResizeEdge::TOP => Self::Top,
            ResizeEdge::LEFT => Self::Left,
            ResizeEdge::RIGHT => Self::Right,
            ResizeEdge::BOTTOM => Self::Bottom,
            ResizeEdge::TOP_LEFT => Self::TopLeft,
            ResizeEdge::TOP_RIGHT => Self::TopRight,
            ResizeEdge::BOTTOM_LEFT => Self::BottomLeft,
            ResizeEdge::BOTTOM_RIGHT => Self::BottomRight,
            _ => Self::None,
        }
    }
}

#[cfg(feature = "xwayland")]
impl From<X11ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(edge: X11ResizeEdge) -> Self {
        match edge {
            X11ResizeEdge::Bottom => ResizeEdge::BOTTOM,
            X11ResizeEdge::BottomLeft => ResizeEdge::BOTTOM_LEFT,
            X11ResizeEdge::BottomRight => ResizeEdge::BOTTOM_RIGHT,
            X11ResizeEdge::Left => ResizeEdge::LEFT,
            X11ResizeEdge::Right => ResizeEdge::RIGHT,
            X11ResizeEdge::Top => ResizeEdge::TOP,
            X11ResizeEdge::TopLeft => ResizeEdge::TOP_LEFT,
            X11ResizeEdge::TopRight => ResizeEdge::TOP_RIGHT,
        }
    }
}

impl From<ResizeEdge> for CursorName {
    fn from(value: ResizeEdge) -> Self {
        match value {
            ResizeEdge::TOP_LEFT => Self::TopLeftCorner,
            ResizeEdge::TOP_RIGHT => Self::TopRightCorner,
            ResizeEdge::BOTTOM_LEFT => Self::BottomLeftCorner,
            ResizeEdge::BOTTOM_RIGHT => Self::BottomRightCorner,
            ResizeEdge::TOP => Self::TopSide,
            ResizeEdge::BOTTOM => Self::BottomSide,
            ResizeEdge::LEFT => Self::LeftSide,
            ResizeEdge::RIGHT => Self::RightSide,
            _ => Self::Default,
        }
    }
}

/// Information about the resize operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ResizeData {
    /// The edges the surface is being resized with.
    pub edges: ResizeEdge,
    /// The initial window location.
    pub initial_window_location: Point<i32, Logical>,
    /// The initial window size (geometry width and height).
    pub initial_window_size: Size<i32, Logical>,
    /// Whether the pointer should be warped to the resized edge on commit.
    pub warp_pointer: bool,
    /// Set by the commit handler before warping so the pointer grab can
    /// distinguish commit-driven warps from real pointer motion.
    pub warp_in_progress: bool,
}

/// State of the resize operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ResizeState {
    /// The surface is not being resized.
    #[default]
    NotResizing,
    /// The surface is currently being resized.
    Resizing(ResizeData),
    /// The resize has finished, and the surface needs to commit its final state.
    WaitingForCommit(ResizeData),
}

pub(super) struct SharedResizeState {
    pub(super) window: WindowElement,
    pub(super) edges: ResizeEdge,
    pub(super) initial_window_location: Point<i32, Logical>,
    pub(super) initial_window_size: Size<i32, Logical>,
    pub(super) last_window_size: Size<i32, Logical>,
    pub(super) pointer_start_location: Point<f64, Logical>,
    pub(super) pointer_start_size: Size<i32, Logical>,
    pub(super) button_pressed: bool,
    pub(super) finished: bool,
    pub(super) skip_next_pointer_motion: bool,
}

fn clamp_size(size: Size<i32, Logical>, min_size: Size<i32, Logical>, max_size: Size<i32, Logical>) -> Size<i32, Logical> {
    let min_w = min_size.w.max(1);
    let min_h = min_size.h.max(1);
    let max_w = if max_size.w == 0 { i32::MAX } else { max_size.w };
    let max_h = if max_size.h == 0 { i32::MAX } else { max_size.h };
    (size.w.max(min_w).min(max_w), size.h.max(min_h).min(max_h)).into()
}

fn snap_resize_size<BackendData: Backend>(
    data: &Xfwl4State<BackendData>,
    window: &WindowElement,
    edges: ResizeEdge,
    initial_window_location: Point<i32, Logical>,
    initial_window_size: Size<i32, Logical>,
    new_size: Size<i32, Logical>,
) -> Size<i32, Logical> {
    if !data.core.config.snap_to_windows() {
        return new_size;
    }

    let snap_width = data.core.config.snap_width();
    let decor_offset = window
        .decoration_state()
        .window_decorations()
        .map(|d| d.decorations_offset())
        .unwrap_or_default();
    let decor_size = window
        .decoration_state()
        .window_decorations()
        .map(|d| {
            Size::<i32, Logical>::from((
                d.left_decoration_width() + d.right_decoration_width(),
                d.top_decoration_height() + d.bottom_decoration_height(),
            ))
        })
        .unwrap_or_default();

    let frame_loc = initial_window_location - decor_offset;
    let frame_left = frame_loc.x;
    let frame_top = frame_loc.y;
    let frame_right = frame_loc.x + initial_window_size.w + decor_size.w;
    let frame_bottom = frame_loc.y + initial_window_size.h + decor_size.h;

    let outputs: Vec<_> = data.core.workspace_manager.outputs().cloned().collect();
    let workspace = data.core.workspace_manager.active_workspace();
    let mut snap_targets: Vec<Rectangle<i32, Logical>> = workspace
        .visible_windows()
        .filter(|w| *w != window)
        .filter_map(|w| workspace.window_geometry(w))
        .collect();
    for output in &outputs {
        if let Some(geo) = data.core.workspace_manager.output_geometry(output) {
            let layer_map = layer_map_for_output(output);
            snap_targets.extend(layer_map.layers().filter_map(|surface| {
                let layer_geo = layer_map.layer_geometry(surface)?;
                Some(Rectangle::new(geo.loc + layer_geo.loc, layer_geo.size))
            }));
        }
    }

    let mut snapped_w = new_size.w;
    let mut snapped_h = new_size.h;

    if edges.intersects(ResizeEdge::LEFT) {
        let proposed_left = frame_right - (new_size.w + decor_size.w);
        let snapped_left = snap::snap_resize_edge_to_windows(proposed_left, frame_top, frame_bottom, &snap_targets, snap_width, true);
        snapped_w = frame_right - snapped_left - decor_size.w;
    } else if edges.intersects(ResizeEdge::RIGHT) {
        let proposed_right = frame_left + new_size.w + decor_size.w;
        let snapped_right = snap::snap_resize_edge_to_windows(proposed_right, frame_top, frame_bottom, &snap_targets, snap_width, true);
        snapped_w = snapped_right - frame_left - decor_size.w;
    }

    if edges.intersects(ResizeEdge::TOP) {
        let proposed_top = frame_bottom - (new_size.h + decor_size.h);
        let snapped_top = snap::snap_resize_edge_to_windows(proposed_top, frame_left, frame_right, &snap_targets, snap_width, false);
        snapped_h = frame_bottom - snapped_top - decor_size.h;
    } else if edges.intersects(ResizeEdge::BOTTOM) {
        let proposed_bottom = frame_top + new_size.h + decor_size.h;
        let snapped_bottom = snap::snap_resize_edge_to_windows(proposed_bottom, frame_left, frame_right, &snap_targets, snap_width, false);
        snapped_h = snapped_bottom - frame_top - decor_size.h;
    }

    let (min_size, max_size) = window.min_max_sizes();
    clamp_size((snapped_w, snapped_h).into(), min_size, max_size)
}

fn compute_resize_from_pointer_delta(
    edges: ResizeEdge,
    pointer_start_size: Size<i32, Logical>,
    delta: Point<f64, Logical>,
    window: &WindowElement,
) -> Size<i32, Logical> {
    let (dx, dy) = delta.into();
    let mut new_w = pointer_start_size.w;
    let mut new_h = pointer_start_size.h;

    if edges.intersects(ResizeEdge::LEFT | ResizeEdge::RIGHT) {
        let dx = if edges.intersects(ResizeEdge::LEFT) { -dx } else { dx };
        new_w = (pointer_start_size.w as f64 + dx) as i32;
    }

    if edges.intersects(ResizeEdge::TOP | ResizeEdge::BOTTOM) {
        let dy = if edges.intersects(ResizeEdge::TOP) { -dy } else { dy };
        new_h = (pointer_start_size.h as f64 + dy) as i32;
    }

    let (min_size, max_size) = window.min_max_sizes();
    clamp_size((new_w, new_h).into(), min_size, max_size)
}

fn send_resize_configure<BackendData: Backend>(data: &mut Xfwl4State<BackendData>, window: &WindowElement, size: Size<i32, Logical>) {
    match window.0.underlying_surface() {
        WindowSurface::Wayland(xdg) => {
            xdg.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Resizing);
                state.size = Some(size);
            });
            xdg.send_pending_configure();
        }
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x11) => {
            if let Some(location) = data.core.workspace_manager.active_workspace().window_location(window) {
                let deco_offset = window
                    .decoration_state()
                    .window_decorations()
                    .map(|d| d.decorations_offset())
                    .unwrap_or_default();
                let _ = x11.configure(Rectangle::new(location + deco_offset, size));
            }
        }
    }
}

fn transition_to_waiting_for_commit(window: &WindowElement) {
    if let Some(surface) = window.wl_surface() {
        with_states(&surface, |states| {
            let mut data = states.data_map.get::<RefCell<SurfaceData>>().unwrap().borrow_mut();
            if let ResizeState::Resizing(resize_data) = data.resize_state {
                data.resize_state = ResizeState::WaitingForCommit(resize_data);
            }
        });
    }
}

fn finish_resize_op<BackendData: Backend>(
    data: &mut Xfwl4State<BackendData>,
    window: &WindowElement,
    edges: ResizeEdge,
    initial_window_location: Point<i32, Logical>,
    initial_window_size: Size<i32, Logical>,
    last_window_size: Size<i32, Logical>,
) {
    if !window.alive() {
        return;
    }

    match window.0.underlying_surface() {
        WindowSurface::Wayland(xdg) => {
            xdg.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Resizing);
                state.size = Some(last_window_size);
            });
            xdg.send_pending_configure();

            if edges.intersects(ResizeEdge::TOP_LEFT) {
                let inner_geometry = SpaceElement::geometry(&window.0);
                let decorations_offset = window
                    .decoration_state()
                    .window_decorations()
                    .map(|d| d.decorations_offset())
                    .unwrap_or_default();
                let workspace = data.core.workspace_manager.active_workspace_mut();
                if let Some(mut location) = workspace.window_location(window) {
                    if edges.intersects(ResizeEdge::LEFT) {
                        location.x = initial_window_location.x + (initial_window_size.w - inner_geometry.size.w) - decorations_offset.x;
                    }
                    if edges.intersects(ResizeEdge::TOP) {
                        location.y = initial_window_location.y + (initial_window_size.h - inner_geometry.size.h) - decorations_offset.y;
                    }
                    data.core.workspace_manager.relocate_window(window, location, true);
                }
            }

            transition_to_waiting_for_commit(window);
        }
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x11) => {
            let workspace = data.core.workspace_manager.active_workspace_mut();
            if let Some(mut location) = workspace.window_location(window) {
                if edges.intersects(ResizeEdge::TOP_LEFT) {
                    let inner_geometry = SpaceElement::geometry(&window.0);
                    let decorations_offset = window
                        .decoration_state()
                        .window_decorations()
                        .map(|d| d.decorations_offset())
                        .unwrap_or_default();
                    if edges.intersects(ResizeEdge::LEFT) {
                        location.x = initial_window_location.x + (initial_window_size.w - inner_geometry.size.w) - decorations_offset.x;
                    }
                    if edges.intersects(ResizeEdge::TOP) {
                        location.y = initial_window_location.y + (initial_window_size.h - inner_geometry.size.h) - decorations_offset.y;
                    }
                    data.core.workspace_manager.relocate_window(window, location, true);
                }
                let deco_offset = window
                    .decoration_state()
                    .window_decorations()
                    .map(|d| d.decorations_offset())
                    .unwrap_or_default();
                let _ = x11.configure(Rectangle::new(location + deco_offset, last_window_size));
            }

            transition_to_waiting_for_commit(window);
        }
    }
}

fn cancel_resize_op<BackendData: Backend>(
    _data: &mut Xfwl4State<BackendData>,
    window: &WindowElement,
    initial_window_location: Point<i32, Logical>,
    initial_window_size: Size<i32, Logical>,
) {
    if !window.alive() {
        return;
    }

    if let Some(surface) = window.wl_surface() {
        with_states(&surface, |states| {
            if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                let mut data = data.borrow_mut();
                if let ResizeState::Resizing(ref mut resize_data) = data.resize_state {
                    resize_data.edges = ResizeEdge::TOP_LEFT;
                    resize_data.initial_window_location = initial_window_location;
                    resize_data.initial_window_size = initial_window_size;
                }
            }
        });
    }

    match window.0.underlying_surface() {
        WindowSurface::Wayland(xdg) => {
            xdg.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Resizing);
                state.size = Some(initial_window_size);
            });
            xdg.send_pending_configure();

            transition_to_waiting_for_commit(window);
        }
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x11) => {
            let _ = x11.configure(Rectangle::new(initial_window_location, initial_window_size));

            transition_to_waiting_for_commit(window);
        }
    }
}

fn warp_pointer_to_edge<BackendData: Backend>(
    data: &mut Xfwl4State<BackendData>,
    window: &WindowElement,
    edges: ResizeEdge,
    last_window_size: Size<i32, Logical>,
    initial_window_location: Point<i32, Logical>,
) {
    if let Some(surface) = window.wl_surface() {
        let decorations_offset = window
            .decoration_state()
            .window_decorations()
            .map(|d| d.decorations_offset())
            .unwrap_or_default();
        let window_location = data
            .core
            .workspace_manager
            .active_workspace()
            .window_location(window)
            .unwrap_or(initial_window_location - decorations_offset)
            + decorations_offset;
        with_states(&surface, |states| {
            if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                let mut data = data.borrow_mut();
                if let ResizeState::Resizing(ref mut resize_data) = data.resize_state {
                    resize_data.edges = edges;
                    resize_data.initial_window_location = window_location;
                    resize_data.initial_window_size = last_window_size;
                }
            }
        });
    }

    let element_loc = data
        .core
        .workspace_manager
        .active_workspace()
        .window_location(window)
        .unwrap_or_default();
    data.warp_pointer_to_resize_edge(window, element_loc, edges);
}

fn handle_keyboard_resize_edge_change<BackendData: Backend + 'static>(
    shared: &Arc<Mutex<SharedResizeState>>,
    data: &mut Xfwl4State<BackendData>,
    new_edge: ResizeEdge,
) {
    shared.lock().unwrap().edges = new_edge;
    let (window, last_size, initial_loc) = {
        let mut state = shared.lock().unwrap();
        state.skip_next_pointer_motion = true;
        (state.window.clone(), state.last_window_size, state.initial_window_location)
    };
    warp_pointer_to_edge(data, &window, new_edge, last_size, initial_loc);
    let mut state = shared.lock().unwrap();
    state.pointer_start_location = data.core.pointer.current_location();
    state.pointer_start_size = state.last_window_size;
}

fn update_wireframe_for_resize(
    wireframe: &mut Wireframe,
    window: &WindowElement,
    edges: ResizeEdge,
    initial_window_location: Point<i32, Logical>,
    initial_window_size: Size<i32, Logical>,
    new_client_size: Size<i32, Logical>,
) {
    let decorations_offset = window
        .decoration_state()
        .window_decorations()
        .map(|d| d.decorations_offset())
        .unwrap_or_default();
    let decorations_size = window
        .decoration_state()
        .window_decorations()
        .map(|d| {
            Size::<i32, Logical>::from((
                d.left_decoration_width() + d.right_decoration_width(),
                d.top_decoration_height() + d.bottom_decoration_height(),
            ))
        })
        .unwrap_or_default();

    let new_full_size = Size::<i32, Logical>::from((new_client_size.w + decorations_size.w, new_client_size.h + decorations_size.h));

    let mut loc = initial_window_location - decorations_offset;
    if edges.intersects(ResizeEdge::LEFT) {
        loc.x += initial_window_size.w - new_client_size.w;
    }
    if edges.intersects(ResizeEdge::TOP) {
        loc.y += initial_window_size.h - new_client_size.h;
    }

    wireframe.update_location(loc);
    wireframe.update_size(new_full_size);
}

fn finish_resize<BackendData: Backend>(
    data: &mut Xfwl4State<BackendData>,
    window: &WindowElement,
    edges: ResizeEdge,
    initial_loc: Point<i32, Logical>,
    initial_size: Size<i32, Logical>,
    last_size: Size<i32, Logical>,
) {
    if last_size != initial_size {
        data.clear_window_tiled_metadata(window);
    }

    if data.core.wireframe.is_some() {
        finish_wireframe_resize(data, window, edges, initial_loc, initial_size, last_size);
    } else {
        window.set_resizing_state(false);
        finish_resize_op(data, window, edges, initial_loc, initial_size, last_size);
    }
}

fn finish_wireframe_resize<BackendData: Backend>(
    data: &mut Xfwl4State<BackendData>,
    window: &WindowElement,
    edges: ResizeEdge,
    initial_window_location: Point<i32, Logical>,
    initial_window_size: Size<i32, Logical>,
    last_window_size: Size<i32, Logical>,
) {
    data.core.wireframe = None;
    window.set_resizing_state(false);

    if let Some(surface) = window.wl_surface() {
        with_states(&surface, |states| {
            if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                data.borrow_mut().resize_state = ResizeState::NotResizing;
            }
        });
    }

    if last_window_size != initial_window_size {
        if edges.intersects(ResizeEdge::TOP_LEFT) {
            let decorations_offset = window
                .decoration_state()
                .window_decorations()
                .map(|d| d.decorations_offset())
                .unwrap_or_default();
            let mut element_loc = initial_window_location - decorations_offset;
            if edges.intersects(ResizeEdge::LEFT) {
                element_loc.x += initial_window_size.w - last_window_size.w;
            }
            if edges.intersects(ResizeEdge::TOP) {
                element_loc.y += initial_window_size.h - last_window_size.h;
            }
            data.core.workspace_manager.relocate_window(window, element_loc, true);
        }

        match window.0.underlying_surface() {
            WindowSurface::Wayland(xdg) => {
                xdg.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                    state.size = Some(last_window_size);
                });
                xdg.send_pending_configure();
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x11) => {
                let location = data
                    .core
                    .workspace_manager
                    .active_workspace()
                    .window_location(window)
                    .unwrap_or_default();
                let deco_offset = window
                    .decoration_state()
                    .window_decorations()
                    .map(|d| d.decorations_offset())
                    .unwrap_or_default();
                let _ = x11.configure(Rectangle::new(location + deco_offset, last_window_size));
            }
        }
    }
}

// -- Pointer resize grab --

pub struct PointerResizeSurfaceGrab<BackendData: Backend + 'static> {
    pub(super) start_data: PointerGrabStartData<Xfwl4State<BackendData>>,
    pub(super) state: Arc<Mutex<SharedResizeState>>,
}

impl<BackendData: Backend> PointerGrab<Xfwl4State<BackendData>> for PointerResizeSurfaceGrab<BackendData> {
    fn motion(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        _focus: Option<(PointerFocusTarget, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let mut state = self.state.lock().unwrap();
        if state.finished {
            // already done
        } else if !state.window.alive() {
            state.finished = true;
            state.window.set_resizing_state(false);
            drop(state);
            handle.unset_grab(self, data, event.serial, event.time, true);
            let seat = data.core.seat.clone();
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.unset_grab(data);
            }
            if let Some(touch) = seat.get_touch() {
                touch.unset_grab(data);
            }
        } else {
            data.core.cursor_status = CursorImageStatus::default_named();
            data.core.set_cursor(state.edges.into());

            if state.skip_next_pointer_motion {
                state.skip_next_pointer_motion = false;
                state.pointer_start_location = event.location;
            } else {
                let is_commit_warp = state.window.wl_surface().is_some_and(|surface| {
                    with_states(&surface, |states| {
                        if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                            let mut data = data.borrow_mut();
                            if let ResizeState::Resizing(ref mut rd) = data.resize_state
                                && rd.warp_in_progress
                            {
                                rd.warp_in_progress = false;
                                return true;
                            }
                        }
                        false
                    })
                });

                if is_commit_warp {
                    let committed_size = SpaceElement::geometry(&state.window.0).size;
                    state.pointer_start_location = event.location;
                    state.pointer_start_size = committed_size;
                } else {
                    let delta = event.location - state.pointer_start_location;
                    let window = state.window.clone();
                    let edges = state.edges;
                    let pointer_start_size = state.pointer_start_size;
                    let initial_window_location = state.initial_window_location;
                    let initial_window_size = state.initial_window_size;
                    drop(state);

                    if let Some(surface) = window.wl_surface() {
                        with_states(&surface, |states| {
                            if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>()
                                && let ResizeState::Resizing(rd) = &mut data.borrow_mut().resize_state
                            {
                                rd.warp_pointer = false;
                            }
                        });
                    }

                    let new_size = compute_resize_from_pointer_delta(edges, pointer_start_size, delta, &window);
                    let new_size = snap_resize_size(data, &window, edges, initial_window_location, initial_window_size, new_size);
                    self.state.lock().unwrap().last_window_size = new_size;

                    if let Some(wireframe) = data.core.wireframe.as_mut() {
                        update_wireframe_for_resize(wireframe, &window, edges, initial_window_location, initial_window_size, new_size);
                    } else {
                        send_resize_configure(data, &window, new_size);
                    }
                }
            }
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        focus: Option<(PointerFocusTarget, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        let mut state = self.state.lock().unwrap();
        if !state.finished {
            if !handle.current_pressed().is_empty() {
                state.button_pressed = true;
            } else if state.button_pressed {
                state.finished = true;
                let window = state.window.clone();
                let edges = state.edges;
                let initial_loc = state.initial_window_location;
                let initial_size = state.initial_window_size;
                let last_size = state.last_window_size;
                drop(state);
                finish_resize(data, &window, edges, initial_loc, initial_size, last_size);
                handle.unset_grab(self, data, event.serial, event.time, true);
                let seat = data.core.seat.clone();
                if let Some(keyboard) = seat.get_keyboard() {
                    keyboard.unset_grab(data);
                }
                if let Some(touch) = seat.get_touch() {
                    touch.unset_grab(data);
                }

                let location = handle.current_location();
                let focus = data.surface_under(location);
                handle.motion(
                    data,
                    focus,
                    &MotionEvent {
                        location,
                        serial: SERIAL_COUNTER.next_serial(),
                        time: data.core.clock.now().as_millis(),
                    },
                );
                handle.frame(data);
            }
        }
    }

    fn axis(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(&mut self, data: &mut Xfwl4State<BackendData>, handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut PointerInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &PointerGrabStartData<Xfwl4State<BackendData>> {
        &self.start_data
    }

    fn unset(&mut self, data: &mut Xfwl4State<BackendData>) {
        let mut state = self.state.lock().unwrap();
        if !state.finished {
            state.finished = true;
            let window = state.window.clone();
            let edges = state.edges;
            let initial_loc = state.initial_window_location;
            let initial_size = state.initial_window_size;
            let last_size = state.last_window_size;
            drop(state);
            if data.core.wireframe.is_some() {
                finish_wireframe_resize(data, &window, edges, initial_loc, initial_size, last_size);
            } else {
                window.set_resizing_state(false);
            }
            let seat = data.core.seat.clone();
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.unset_grab(data);
            }
            if let Some(touch) = seat.get_touch() {
                touch.unset_grab(data);
            }
        }
    }
}

// -- Touch resize grab --

pub struct TouchResizeSurfaceGrab<BackendData: Backend + 'static> {
    pub(super) start_data: TouchGrabStartData<Xfwl4State<BackendData>>,
    pub(super) state: Arc<Mutex<SharedResizeState>>,
}

impl<BackendData: Backend> TouchGrab<Xfwl4State<BackendData>> for TouchResizeSurfaceGrab<BackendData> {
    fn down(
        &mut self,
        _data: &mut Xfwl4State<BackendData>,
        _handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        _focus: Option<(
            <Xfwl4State<BackendData> as smithay::input::SeatHandler>::TouchFocus,
            Point<f64, Logical>,
        )>,
        _event: &smithay::input::touch::DownEvent,
        _seq: Serial,
    ) {
    }

    fn up(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &smithay::input::touch::UpEvent,
        _seq: Serial,
    ) {
        if event.slot != self.start_data.slot {
            return;
        }

        let mut state = self.state.lock().unwrap();
        if !state.finished {
            state.finished = true;
            let window = state.window.clone();
            let edges = state.edges;
            let initial_loc = state.initial_window_location;
            let initial_size = state.initial_window_size;
            let last_size = state.last_window_size;
            drop(state);
            if data.core.wireframe.is_some() {
                finish_wireframe_resize(data, &window, edges, initial_loc, initial_size, last_size);
            } else {
                window.set_resizing_state(false);
                finish_resize_op(data, &window, edges, initial_loc, initial_size, last_size);
            }
            handle.unset_grab(self, data);
            let pointer = data.core.pointer.clone();
            pointer.unset_grab(data, SERIAL_COUNTER.next_serial(), data.core.clock.now().as_millis());
            let seat = data.core.seat.clone();
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.unset_grab(data);
            }
        }
    }

    fn motion(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        _focus: Option<(
            <Xfwl4State<BackendData> as smithay::input::SeatHandler>::TouchFocus,
            Point<f64, Logical>,
        )>,
        event: &smithay::input::touch::MotionEvent,
        _seq: Serial,
    ) {
        if event.slot != self.start_data.slot {
            return;
        }

        let mut state = self.state.lock().unwrap();
        if !state.finished {
            if !state.window.alive() {
                state.finished = true;
                state.window.set_resizing_state(false);
                drop(state);
                handle.unset_grab(self, data);
                let pointer = data.core.pointer.clone();
                pointer.unset_grab(data, SERIAL_COUNTER.next_serial(), data.core.clock.now().as_millis());
                let seat = data.core.seat.clone();
                if let Some(keyboard) = seat.get_keyboard() {
                    keyboard.unset_grab(data);
                }
            } else {
                let delta = event.location - state.pointer_start_location;
                let window = state.window.clone();
                let edges = state.edges;
                let pointer_start_size = state.pointer_start_size;
                let initial_window_location = state.initial_window_location;
                let initial_window_size = state.initial_window_size;
                drop(state);

                let new_size = compute_resize_from_pointer_delta(edges, pointer_start_size, delta, &window);
                let new_size = snap_resize_size(data, &window, edges, initial_window_location, initial_window_size, new_size);
                self.state.lock().unwrap().last_window_size = new_size;

                if let Some(wireframe) = data.core.wireframe.as_mut() {
                    update_wireframe_for_resize(wireframe, &window, edges, initial_window_location, initial_window_size, new_size);
                } else {
                    send_resize_configure(data, &window, new_size);
                }
            }
        }
    }

    fn frame(
        &mut self,
        _data: &mut Xfwl4State<BackendData>,
        _handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        _seq: Serial,
    ) {
    }

    fn cancel(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        seq: Serial,
    ) {
        handle.cancel(data, seq);
        handle.unset_grab(self, data);
    }

    fn shape(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &smithay::input::touch::ShapeEvent,
        seq: Serial,
    ) {
        handle.shape(data, event, seq);
    }

    fn orientation(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut smithay::input::touch::TouchInnerHandle<'_, Xfwl4State<BackendData>>,
        event: &smithay::input::touch::OrientationEvent,
        seq: Serial,
    ) {
        handle.orientation(data, event, seq);
    }

    fn start_data(&self) -> &smithay::input::touch::GrabStartData<Xfwl4State<BackendData>> {
        &self.start_data
    }

    fn unset(&mut self, data: &mut Xfwl4State<BackendData>) {
        let mut state = self.state.lock().unwrap();
        if !state.finished {
            state.finished = true;
            let window = state.window.clone();
            let edges = state.edges;
            let initial_loc = state.initial_window_location;
            let initial_size = state.initial_window_size;
            let last_size = state.last_window_size;
            drop(state);
            if data.core.wireframe.is_some() {
                finish_wireframe_resize(data, &window, edges, initial_loc, initial_size, last_size);
            } else {
                window.set_resizing_state(false);
            }
            let pointer = data.core.pointer.clone();
            pointer.unset_grab(data, SERIAL_COUNTER.next_serial(), data.core.clock.now().as_millis());
            let seat = data.core.seat.clone();
            if let Some(keyboard) = seat.get_keyboard() {
                keyboard.unset_grab(data);
            }
        }
    }
}

// -- Keyboard resize grab --

pub struct KeyboardResizeSurfaceGrab<BackendData: Backend + 'static> {
    pub(super) start_data: KeyboardGrabStartData<Xfwl4State<BackendData>>,
    pub(super) state: Arc<Mutex<SharedResizeState>>,
}

impl<BackendData: Backend + 'static> KeyboardGrab<Xfwl4State<BackendData>> for KeyboardResizeSurfaceGrab<BackendData> {
    fn input(
        &mut self,
        data: &mut Xfwl4State<BackendData>,
        handle: &mut KeyboardInnerHandle<'_, Xfwl4State<BackendData>>,
        keycode: Keycode,
        key_state: KeyState,
        _modifiers: Option<ModifiersState>,
        serial: Serial,
        _time: u32,
    ) {
        {
            let state = self.state.lock().unwrap();
            if state.finished {
                return;
            }
        }

        if let Some(action) = keyboard_move_resize_get_action(data, handle, keycode, key_state) {
            let edges = self.state.lock().unwrap().edges;

            match action {
                MoveResizeAction::Left | MoveResizeAction::Right | MoveResizeAction::Up | MoveResizeAction::Down => {
                    let pointer_delta: Option<Point<f64, Logical>> = match (action, edges) {
                        (MoveResizeAction::Left, ResizeEdge::LEFT | ResizeEdge::RIGHT) => Some((-KEY_RESIZE_BASE as f64, 0.0).into()),
                        (MoveResizeAction::Right, ResizeEdge::LEFT | ResizeEdge::RIGHT) => Some((KEY_RESIZE_BASE as f64, 0.0).into()),
                        (MoveResizeAction::Up, ResizeEdge::TOP | ResizeEdge::BOTTOM) => Some((0.0, -KEY_RESIZE_BASE as f64).into()),
                        (MoveResizeAction::Down, ResizeEdge::TOP | ResizeEdge::BOTTOM) => Some((0.0, KEY_RESIZE_BASE as f64).into()),
                        (MoveResizeAction::Left, _) => {
                            handle_keyboard_resize_edge_change(&self.state, data, ResizeEdge::LEFT);
                            None
                        }
                        (MoveResizeAction::Right, _) => {
                            handle_keyboard_resize_edge_change(&self.state, data, ResizeEdge::RIGHT);
                            None
                        }
                        (MoveResizeAction::Up, _) => {
                            handle_keyboard_resize_edge_change(&self.state, data, ResizeEdge::TOP);
                            None
                        }
                        (MoveResizeAction::Down, _) => {
                            handle_keyboard_resize_edge_change(&self.state, data, ResizeEdge::BOTTOM);
                            None
                        }
                        _ => unreachable!(),
                    };

                    if let Some(delta) = pointer_delta {
                        let pointer = data.core.pointer.clone();
                        let location = pointer.current_location() + delta;
                        pointer.motion(
                            data,
                            None,
                            &MotionEvent {
                                location,
                                serial: SERIAL_COUNTER.next_serial(),
                                time: data.core.clock.now().as_millis(),
                            },
                        );
                        pointer.frame(data);
                    }
                }

                MoveResizeAction::Finish => {
                    let (window, edges, initial_loc, initial_size, last_size) = {
                        let mut state = self.state.lock().unwrap();
                        state.finished = true;
                        (
                            state.window.clone(),
                            state.edges,
                            state.initial_window_location,
                            state.initial_window_size,
                            state.last_window_size,
                        )
                    };
                    if data.core.wireframe.is_some() {
                        finish_wireframe_resize(data, &window, edges, initial_loc, initial_size, last_size);
                    } else {
                        window.set_resizing_state(false);
                        finish_resize_op(data, &window, edges, initial_loc, initial_size, last_size);
                    }
                    handle.unset_grab(self, data, serial, true);
                    let pointer = data.core.pointer.clone();
                    pointer.unset_grab(data, serial, data.core.clock.now().as_millis());
                    if let Some(touch) = data.core.seat.clone().get_touch() {
                        touch.unset_grab(data);
                    }
                }

                MoveResizeAction::Cancel => {
                    let (window, initial_loc, initial_size) = {
                        let mut state = self.state.lock().unwrap();
                        state.last_window_size = state.initial_window_size;
                        state.finished = true;
                        (state.window.clone(), state.initial_window_location, state.initial_window_size)
                    };
                    if data.core.wireframe.is_some() {
                        data.core.wireframe = None;
                        window.set_resizing_state(false);
                    } else {
                        window.set_resizing_state(false);
                        cancel_resize_op(data, &window, initial_loc, initial_size);
                    }
                    handle.unset_grab(self, data, serial, true);
                    let pointer = data.core.pointer.clone();
                    pointer.unset_grab(data, serial, data.core.clock.now().as_millis());
                    if let Some(touch) = data.core.seat.clone().get_touch() {
                        touch.unset_grab(data);
                    }
                }
            };
        }
    }

    fn set_focus(
        &mut self,
        _data: &mut Xfwl4State<BackendData>,
        _handle: &mut KeyboardInnerHandle<'_, Xfwl4State<BackendData>>,
        _focus: Option<<Xfwl4State<BackendData> as SeatHandler>::KeyboardFocus>,
        _serial: Serial,
    ) {
    }

    fn unset(&mut self, data: &mut Xfwl4State<BackendData>) {
        let mut state = self.state.lock().unwrap();
        if !state.finished {
            state.finished = true;
            let window = state.window.clone();
            let edges = state.edges;
            let initial_loc = state.initial_window_location;
            let initial_size = state.initial_window_size;
            let last_size = state.last_window_size;
            drop(state);
            if data.core.wireframe.is_some() {
                finish_wireframe_resize(data, &window, edges, initial_loc, initial_size, last_size);
            } else {
                window.set_resizing_state(false);
            }
            let pointer = data.core.pointer.clone();
            pointer.unset_grab(data, SERIAL_COUNTER.next_serial(), data.core.clock.now().as_millis());
            let seat = data.core.seat.clone();
            if let Some(touch) = seat.get_touch() {
                touch.unset_grab(data);
            }
        }
    }

    fn start_data(&self) -> &KeyboardGrabStartData<Xfwl4State<BackendData>> {
        &self.start_data
    }
}
