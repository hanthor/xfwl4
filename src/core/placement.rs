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
// Based on xfm4/src/placement.c, which is:
//
// Copyright (C) 2002-2022 Olivier Fourdan

use std::collections::HashMap;

use smithay::{
    desktop::{WindowSurface, find_popup_root_surface, layer_map_for_output, space::SpaceElement},
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Size},
    wayland::seat::WaylandFocus,
};

use crate::{
    backend::Backend,
    core::{
        config::PlacementMode,
        focus::KeyboardFocusTarget,
        shell::WindowElement,
        state::{WindowClient, Xfwl4State},
        workspaces::{Workspace, WorkspaceManager},
    },
};

/// X11 timestamp ordering with 32-bit wraparound handling. A timestamp of 0 is "before
/// everything"; a difference greater than half the u32 range is treated as wraparound.
#[cfg(feature = "xwayland")]
fn timestamp_is_before(t1: u32, t2: u32) -> bool {
    t1 == 0 || (t2 != 0 && (t2.wrapping_sub(t1) as i32) > 0)
}

struct Frame {
    position: Point<i32, Logical>,
    content_size: Size<i32, Logical>,
    extents: FrameExtents<i32, Logical>,
}

impl Frame {
    /// `position` is the visible frame top-left in workspace coordinates.
    /// `content_size` is the visible content size, excluding both server-side decorations
    /// and CSD shadows. `extent_width()`/`extent_height()` produce the visible frame
    /// rectangle (decorations + content) for any window type.
    fn new(window: &WindowElement, position: Point<i32, Logical>, content_size: Size<i32, Logical>) -> Self {
        let extents = window
            .decoration_state()
            .window_decorations()
            .map(|d| d.decorations_extents())
            .unwrap_or_else(|| FrameExtents::new(0, 0, 0, 0));
        Self {
            position,
            content_size,
            extents,
        }
    }

    /// X coordinate of the frame's outer left edge in workspace coordinates.
    /// xfwm4: `frameExtentX(c)`.
    fn extent_x(&self) -> i32 {
        self.position.x
    }

    /// Y coordinate of the frame's outer top edge in workspace coordinates.
    /// xfwm4: `frameExtentY(c)`.
    fn extent_y(&self) -> i32 {
        self.position.y
    }

    /// Left decoration margin. xfwm4: `frameExtentLeft(c)`.
    #[allow(unused)]
    fn extent_left(&self) -> i32 {
        self.extents.left
    }

    /// Right decoration margin. xfwm4: `frameExtentRight(c)`.
    #[allow(unused)]
    fn extent_right(&self) -> i32 {
        self.extents.right
    }

    /// Top decoration margin. xfwm4: `frameExtentTop(c)`.
    #[allow(unused)]
    fn extent_top(&self) -> i32 {
        self.extents.top
    }

    /// Bottom decoration margin. xfwm4: `frameExtentBottom(c)`.
    #[allow(unused)]
    fn extent_bottom(&self) -> i32 {
        self.extents.bottom
    }

    /// Total width including decorations. xfwm4: `frameExtentWidth(c)`.
    fn extent_width(&self) -> i32 {
        self.extents.left + self.content_size.w + self.extents.right
    }

    /// Total height including decorations. xfwm4: `frameExtentHeight(c)`.
    fn extent_height(&self) -> i32 {
        self.extents.top + self.content_size.h + self.extents.bottom
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StackLocation {
    Top,
    Below(WindowElement),
}

pub struct StackResult {
    pub location: StackLocation,
    pub allow_activate: bool,
    pub needs_attention: bool,
}

impl<BackendData: Backend + 'static> Xfwl4State<BackendData> {
    fn window_can_focus(&self, window: &WindowElement) -> bool {
        if window.has_modal_child() {
            false
        } else {
            match window.0.underlying_surface() {
                WindowSurface::Wayland(_) => true,
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(surface) => {
                    let input_hint = surface.hints().and_then(|hints| hints.input).unwrap_or(true);
                    let is_modal = self
                        .core
                        .xwayland
                        .as_ref()
                        .and_then(|xw| {
                            let net_wm_state_modal = xw.x11.get_atom("_NET_WM_STATE_MODAL")?;
                            xw.x11
                                .get_net_wm_state(surface.window_id())
                                .map(|state_atoms| state_atoms.contains(&net_wm_state_modal))
                        })
                        .unwrap_or(false);
                    input_hint || is_modal
                }
            }
        }
    }

    fn get_user_time(&self, window: &WindowElement) -> Option<u32> {
        match window.0.underlying_surface() {
            WindowSurface::Wayland(_) => None,
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(surface) => self.core.xwayland.as_ref().and_then(|xw| xw.x11.get_user_time(surface.window_id())),
        }
    }

    fn is_moving_or_resizing(&self) -> bool {
        self.core
            .workspace_manager
            .active_workspace()
            .visible_windows()
            .any(|window| window.moving() || window.resizing())
    }

    pub(in crate::core) fn stack_new_window(&mut self, window: &WindowElement) -> StackResult {
        let accept_focus = self.window_can_focus(window);
        let user_time = self.get_user_time(window);

        let (allow_activate, prevented) = if !accept_focus {
            (false, false)
        } else if window.modal() {
            (true, false)
        } else if user_time == Some(0) {
            // X11 client setting _NET_WM_USER_TIME to 0 means it doesn't want focus on map.
            (false, false)
        } else {
            let current_focus = self.core.seat.get_keyboard().and_then(|keyboard| keyboard.current_focus());

            if let Some(current_focus) = current_focus
                && self.core.config.prevent_focus_stealing()
            {
                let current_focus_window = self
                    .core
                    .workspace_manager
                    .active_workspace()
                    .find_window(|elem| elem.wl_surface().is_some() && elem.wl_surface() == current_focus.wl_surface());
                let current_focus_user_time = current_focus_window
                    .as_ref()
                    .and_then(|window| window.0.x11_surface().cloned())
                    .and_then(|surface| self.core.xwayland.as_ref().and_then(|xw| xw.x11.get_user_time(surface.window_id())));

                #[allow(clippy::if_same_then_else)]
                if current_focus.stacking_layer() > window.stacking_layer() {
                    (false, true)
                } else if current_focus.stacking_layer() < window.stacking_layer() {
                    (true, false)
                } else if self.is_moving_or_resizing() {
                    (false, true)
                } else if current_focus_user_time
                    .zip(user_time)
                    .filter(|(current_focus_user_time, user_time)| current_focus_user_time >= user_time)
                    .is_none()
                {
                    (false, true)
                } else {
                    (self.core.config.focus_new(), false)
                }
            } else {
                (true, false)
            }
        };

        let location = if allow_activate {
            StackLocation::Top
        } else {
            let current_focus = self.core.seat.get_keyboard().and_then(|keyboard| keyboard.current_focus());
            if prevented
                && let Some(current_focus) = current_focus
                && current_focus.stacking_layer() == window.stacking_layer()
            {
                match current_focus {
                    KeyboardFocusTarget::LayerSurface(_) => StackLocation::Top,
                    KeyboardFocusTarget::Popup(popup) => find_popup_root_surface(&popup)
                        .ok()
                        .and_then(|root| self.core.workspace_manager.active_workspace().window_for_surface(&root))
                        .or_else(|| self.core.workspace_manager.active_workspace().visible_windows().last().cloned())
                        .map(StackLocation::Below)
                        .unwrap_or(StackLocation::Top),
                    KeyboardFocusTarget::Window(win) => self
                        .core
                        .workspace_manager
                        .active_workspace()
                        .find_window(|elem| elem.0 == win)
                        .map(StackLocation::Below)
                        .unwrap_or(StackLocation::Top),
                }
            } else {
                StackLocation::Top
            }
        };

        StackResult {
            location,
            allow_activate,
            needs_attention: prevented,
        }
    }

    pub(in crate::core) fn place_window(
        &mut self,
        window: &WindowElement,
        content_size: Size<i32, Logical>,
        stack_location: StackLocation,
        allow_activate: bool,
    ) {
        let is_new_window = self.core.workspace_manager.workspace_for_window_mut(window).is_none();
        let pointer_location = self.core.pointer.current_location();

        let frame = Frame::new(window, Point::default(), content_size);

        let output = self
            .core
            .workspace_manager
            .output_under(pointer_location)
            .next()
            .or_else(|| self.core.workspace_manager.outputs().next())
            .cloned();
        let output_geometry = output
            .as_ref()
            .and_then(|o| {
                let geo = self.core.workspace_manager.output_geometry(o)?;
                let map = layer_map_for_output(o);
                let zone = map.non_exclusive_zone();
                Some(Rectangle::new(geo.loc + zone.loc, zone.size))
            })
            .unwrap_or_else(|| Rectangle::from_size((800, 800).into()));

        let output_geometries = self
            .core
            .workspace_manager
            .outputs()
            .flat_map(|output| self.core.workspace_manager.output_geometry(output));

        let full_geometry = {
            let mut iter = output_geometries.into_iter();
            iter.next()
                .map(|first_geometry| iter.fold(first_geometry, |geometry_accum, geometry| geometry_accum.merge(geometry)))
                .unwrap_or_else(|| Rectangle::from_size((800, 800).into()))
        };

        let location = place_at_requested_location(window, &frame, output_geometry, full_geometry)
            .or_else(|| place_as_child_window(window, &frame, &self.core.workspace_manager))
            .or_else(|| place_at_existing_position(window, &frame))
            .unwrap_or_else(|| {
                let placement_ratio = self.core.config.placement_ratio();
                let placement_mode = self.core.config.placement_mode();

                if placement_ratio >= 100
                    || 100 * frame.extent_width() * frame.extent_height()
                        < placement_ratio * output_geometry.size.w * output_geometry.size.h
                {
                    if placement_mode == PlacementMode::Mouse {
                        place_under_pointer(&frame, output_geometry, pointer_location)
                    } else {
                        place_in_center(&frame, output_geometry)
                    }
                } else if frame.extent_width() >= output_geometry.size.w && frame.extent_height() >= output_geometry.size.h {
                    place_in_center(&frame, output_geometry)
                } else {
                    place_smartly(window, &frame, output_geometry, self.core.workspace_manager.active_workspace())
                }
            });

        // If the window is partiall off-screen in either dimension, try to move it left/up to get
        // it to fit, but don't push it farther left/up than the output area's bounds.
        let location: Point<_, Logical> = (
            if location.x + frame.extent_width() > output_geometry.loc.x + output_geometry.size.w {
                output_geometry.loc.x + output_geometry.size.w - frame.extent_width()
            } else {
                location.x
            }
            .max(output_geometry.loc.x),
            if location.y + frame.extent_height() > output_geometry.loc.y + output_geometry.size.h {
                output_geometry.loc.y + output_geometry.size.h - frame.extent_height()
            } else {
                location.y
            }
            .max(output_geometry.loc.y),
        )
            .into();

        let should_fullscreen = stack_location == StackLocation::Fullscreen;

        if frame.extent_width() >= output_geometry.size.w && frame.extent_height() >= output_geometry.size.h && can_auto_maximize(window) {
            // If the window is larger than the output area's bounds in *both* dimensions, maximize
            // the window.
            if is_new_window {
                self.new_window(window.clone(), location, allow_activate, None);
                if let StackLocation::Below(below_window) = stack_location {
                    self.lower_window(window, SERIAL_COUNTER.next_serial(), Some(below_window));
                }
            }
            self.set_window_maximized(window, None);
        } else if is_new_window {
            self.new_window(window.clone(), location, allow_activate, None);
            if let StackLocation::Below(below_window) = stack_location {
                self.lower_window(window, SERIAL_COUNTER.next_serial(), Some(below_window));
            }
        } else {
            self.core.workspace_manager.relocate_window(window, location, allow_activate);
            if allow_activate {
                self.focus_window(window, SERIAL_COUNTER.next_serial(), None);
            }
        }

        if should_fullscreen {
            self.set_window_fullscreen(window, output);
        }
    }

    pub(in crate::core) fn fill_window(&mut self, window: &WindowElement, fill_mode: FillMode) {
        if !window.fullscreened()
            && !window.maximized()
            && !window.shaded()
            && let Some(workspace) = self.core.workspace_manager.workspace_for_window(window)
            && let Some(geom) = workspace.window_geometry(window)
            && let Some(output_geometry) = {
                let center = Point::<_, Logical>::new(geom.loc.x + (geom.size.w / 2), geom.loc.y + (geom.size.h / 2));
                workspace
                    .outputs_for_window(window)
                    .iter()
                    .map(|output| layer_map_for_output(output).non_exclusive_zone())
                    .find(|rect| rect.contains(center))
            }
        {
            let content_size = if let Some(decorations) = window.decoration_state().window_decorations() {
                let e = decorations.decorations_extents();
                Size::new(geom.size.w - e.left - e.right, geom.size.h - e.top - e.bottom)
            } else {
                geom.size
            };
            let frame = Frame::new(window, geom.loc, content_size);
            let mut new_geom = place_filled(window, &frame, output_geometry, workspace, fill_mode);

            if let Some(decorations) = window.decoration_state().window_decorations() {
                let e = decorations.decorations_extents();
                new_geom.size.w -= e.left + e.right;
                new_geom.size.h -= e.top + e.bottom;
            }

            match window.0.underlying_surface() {
                WindowSurface::Wayland(surface) => {
                    surface.with_pending_state(|state| {
                        state.size = Some(new_geom.size);
                    });
                    self.core.workspace_manager.relocate_window(window, new_geom.loc, false);

                    if surface.is_initial_configure_sent() {
                        surface.send_configure();
                    }
                }

                #[cfg(feature = "xwayland")]
                WindowSurface::X11(surface) => {
                    let _ = surface.configure(window.grow_rect_by_gtk_frame_extents(new_geom));
                    self.core.workspace_manager.relocate_window(window, new_geom.loc, false);
                }
            }
        }
    }
}

fn can_auto_maximize(window: &WindowElement) -> bool {
    match window.0.underlying_surface() {
        WindowSurface::Wayland(_) => true,
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(surface) => {
            use smithay::xwayland::xwm::WmWindowType;
            surface.window_type().is_none_or(|ty| matches!(ty, WmWindowType::Normal))
        }
    }
}

/// Wayland doesn't generally allow clients to choose positions for their windows, but in the
/// legacy Xwayland case, X11 apps may rely on being able to do this in order to behave correctly.
/// So this function exists solely to allow X11 windows to set their own postition.
fn place_at_requested_location(
    window: &WindowElement,
    frame: &Frame,
    output_geometry: Rectangle<i32, Logical>,
    full_geometry: Rectangle<i32, Logical>,
) -> Option<Point<i32, Logical>> {
    match window.0.underlying_surface() {
        WindowSurface::Wayland(_) => {
            #[cfg(not(feature = "xwayland"))]
            let _ = (frame, output_geometry, full_geometry);
            None
        }

        #[cfg(feature = "xwayland")]
        WindowSurface::X11(surface) => {
            surface.size_hints().and_then(|hints| hints.position).map(|(_, x, y)| {
                use crate::core::util::RectangleExt;
                use smithay::xwayland::xwm::WmWindowType;

                // Some clients place dialogs at (0, 0).  Other clients aren't multihead aware
                // and try to put dialogs near the center of the full desktop.  Move them in both
                // cases to the center of the output.
                let is_dialog = surface.window_type().is_some_and(|ty| ty == WmWindowType::Dialog);
                if is_dialog && ((x == 0 && y == 0) || full_geometry.is_near_center(surface.geometry())) {
                    (
                        output_geometry.loc.x + (output_geometry.size.w / 2 - frame.extent_width() / 2),
                        output_geometry.loc.y + (output_geometry.size.h / 2 - frame.extent_height() / 2),
                    )
                        .into()
                } else {
                    (x, y).into()
                }
            })
        }
    }
}

/// Windows that are children of other windows should generally be placed in an "obvious" place
/// that's related to their parent, so we place them centered over their parent.
fn place_as_child_window<BackendData: Backend + 'static>(
    window: &WindowElement,
    frame: &Frame,
    workspace_manager: &WorkspaceManager<BackendData>,
) -> Option<Point<i32, Logical>> {
    window
        .parent()
        .and_then(|parent| workspace_manager.window_geometry(&parent))
        .map(|parent_geometry| {
            (
                parent_geometry.loc.x + parent_geometry.size.w / 2 - frame.extent_width() / 2,
                parent_geometry.loc.y + parent_geometry.size.h / 2 - frame.extent_height() / 2,
            )
                .into()
        })
}

/// This is an (unlikely) catch-all for edge cases around X11 dock/dialog/etc. type windows where
/// they've been mapped both without PPosition/UPosition set, and without a parent.  We still want
/// them to be placed where they intended to be placed (via the coordinates passed to the
/// XCreateWindow() call), so we handle that here.
fn place_at_existing_position(window: &WindowElement, frame: &Frame) -> Option<Point<i32, Logical>> {
    match window.0.underlying_surface() {
        WindowSurface::Wayland(_) => {
            #[cfg(not(feature = "xwayland"))]
            let _ = frame;
            None
        }

        #[cfg(feature = "xwayland")]
        WindowSurface::X11(surface) => {
            use smithay::xwayland::xwm::WmWindowType;

            let type_matches = surface.window_type().is_some_and(|ty| {
                matches!(
                    ty,
                    WmWindowType::Desktop | WmWindowType::Dock | WmWindowType::Splash | WmWindowType::Utility | WmWindowType::Dialog
                )
            });
            let orphaned_transient = surface.is_transient_for().is_some() && !window.has_parent();

            (type_matches || orphaned_transient).then(|| {
                let location = surface.geometry().loc;
                (location.x - frame.extents.left, location.y - frame.extents.top).into()
            })
        }
    }
}

/// Places the window centered under the pointer.
fn place_under_pointer(
    frame: &Frame,
    output_geometry: Rectangle<i32, Logical>,
    pointer_location: Point<f64, Logical>,
) -> Point<i32, Logical> {
    let frame_width = frame.extent_width() as f64;
    let frame_height = frame.extent_height() as f64;
    let output_geometry = output_geometry.to_f64();

    Point::<f64, Logical>::new(
        (pointer_location.x - frame_width / 2.)
            .min(output_geometry.loc.x + output_geometry.size.w - frame_width)
            .max(output_geometry.loc.x),
        (pointer_location.y - frame_height / 2.)
            .min(output_geometry.loc.y + output_geometry.size.h - frame_height)
            .max(output_geometry.loc.y),
    )
    .to_i32_round()
}

/// Places in the center of the monitor.
fn place_in_center(frame: &Frame, output_geometry: Rectangle<i32, Logical>) -> Point<i32, Logical> {
    (
        (output_geometry.loc.x + (output_geometry.size.w - frame.extent_width()) / 2).max(output_geometry.loc.x),
        (output_geometry.loc.y + (output_geometry.size.h - frame.extent_height()) / 2).max(output_geometry.loc.y),
    )
        .into()
}

/// Smart placement tries to place windows with the minumum amount of overlap with other windows.
/// It's a bit of a slow process, as it goes through the list of existing windows over and over and
/// over, trying to find the best (lowest) set of overlaps.
fn place_smartly(
    window: &WindowElement,
    frame: &Frame,
    output_geometry: Rectangle<i32, Logical>,
    workspace: &Workspace,
) -> Point<i32, Logical> {
    let frame_size = Size::<_, Logical>::from((frame.extent_width(), frame.extent_height()));

    // xfwm4 does the min/max & test checks in "content space", that is, only considering the
    // are of the window contents.  xfwl4 needs to do the checks in "frame space", that is,
    // considering the entire frame, including window decorations (if any), because when we map
    // something into a Space, it includes the coordinate offset of the frame.
    let max = Point::<_, Logical>::new(
        output_geometry.loc.x + output_geometry.size.w - frame.extent_width(),
        output_geometry.loc.y + output_geometry.size.h - frame.extent_height(),
    );
    let min = output_geometry.loc;

    let mut best_overlaps = f32::MAX;
    let mut best = min;

    let workspace_windows = workspace
        .visible_windows()
        .filter(|other| *other != window)
        .filter(|other| match other.0.underlying_surface() {
            WindowSurface::Wayland(_) => true,
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(surface) => {
                use smithay::xwayland::xwm::WmWindowType;
                surface.window_type().is_none_or(|ty| ty != WmWindowType::Desktop)
            }
        })
        .collect::<Vec<_>>();
    tracing::trace!("Analyzing {} windows", workspace_windows.len());

    let mut test = Point::<_, Logical>::new(0, min.y);
    'outer: loop {
        let mut next_test = Point::<_, Logical>::new(0, i32::MAX);
        let mut first_test_x = true;

        tracing::trace!("Testing y position {}", test.y);

        test.x = min.x;
        loop {
            let mut count_overlaps = 0f32;
            next_test.x = i32::MAX;
            let mut c2_next_test = Point::<_, Logical>::new(0, 0);

            tracing::trace!("Testing x position {}", test.x);

            for other in &workspace_windows {
                if let Some(other_geom) = workspace.window_geometry(other)
                    && output_geometry.intersection(other_geom).is_some()
                {
                    let other_loc = other_geom.loc;
                    let frame_other = Frame::new(other, other_loc, SpaceElement::geometry(&other.0).size);

                    count_overlaps += overlap(
                        test.x,
                        test.y,
                        test.x + frame_size.w,
                        test.y + frame_size.h,
                        other_loc.x,
                        other_loc.y,
                        other_loc.x + frame_other.extent_width(),
                        other_loc.y + frame_other.extent_height(),
                    ) as f32;

                    // Find next x-bounds for the step, clamping to the coordinate of the right
                    // side of the window.
                    let other_x = if test.x > other_loc.x {
                        other_loc.x + frame_other.extent_width()
                    } else {
                        other_loc.x
                    };
                    c2_next_test.x = other_x.min(max.x);

                    if c2_next_test.x < next_test.x && c2_next_test.x > test.x {
                        next_test.x = c2_next_test.x;
                    }

                    if first_test_x {
                        // Find the next y-bounds for the step, clamping to the coordiate of the
                        // bottom side of the window.
                        let other_y = if test.y > other_loc.y {
                            other_geom.loc.y + frame_other.extent_height()
                        } else {
                            other_geom.loc.y
                        };
                        c2_next_test.y = other_y.min(max.y);

                        if c2_next_test.y < next_test.y && c2_next_test.y > test.y {
                            next_test.y = c2_next_test.y;
                        }
                    }
                }
            }

            first_test_x = false;

            if count_overlaps < best_overlaps {
                // Great, we found a position with fewer overlaps than what was previously the best
                // position found.
                best = test;
                best_overlaps = count_overlaps;

                if count_overlaps == 0. {
                    // Holy grail: zero overlaps.  No need to continue.
                    tracing::trace!("Found position without overlap");
                    break 'outer best;
                }
            }

            if next_test.x != i32::MAX {
                // Never go past the right edge of the monitor.
                test.x = next_test.x;
            } else {
                test.x += 1;
            }

            if test.x > max.x {
                // Our x test position is past the right edge of the monitor, so continue on to the
                // next y test.
                break;
            }
        }

        if next_test.y != i32::MAX {
            // Never go past the bottom edge of the monitor.
            test.y = next_test.y;
        } else {
            test.y += 1;
        }

        if test.y > max.y {
            // Our y test position is past the bottom edge of the monitor, so return whatever best
            // result we've gotten so far.
            tracing::trace!("Returning best fit");
            break best;
        }
    }
}

fn place_filled(
    window: &WindowElement,
    frame: &Frame,
    output_geometry: Rectangle<i32, Logical>,
    workspace: &Workspace,
    fill_mode: FillMode,
) -> Rectangle<i32, Logical> {
    let layer = window.stacking_layer();

    let other_windows = workspace.visible_windows().filter(|w| w.stacking_layer() == layer && *w != window);

    let mut neighbors = HashMap::<Direction, WindowElement>::new();

    let frame_for_window = |window: &WindowElement| {
        workspace
            .window_geometry(window)
            .map(|geom| Frame::new(window, geom.loc, SpaceElement::geometry(&window.0).size))
    };

    for other in other_windows {
        if let Some(other_geom) = workspace.window_geometry(other) {
            let other_frame = Frame::new(other, other_geom.loc, SpaceElement::geometry(&other.0).size);

            if fill_mode == FillMode::Horizontal
                || fill_mode == FillMode::Both
                    && segment_overlap(
                        frame.extent_y(),
                        frame.extent_y() + frame.extent_height(),
                        other_frame.extent_y(),
                        other_frame.extent_y() + other_frame.extent_height(),
                    ) != 0
            {
                if other_frame.extent_x() + other_frame.extent_width() <= frame.extent_x() {
                    if let Some(west) = neighbors.get(&Direction::Left)
                        && let Some(west_frame) = frame_for_window(west)
                    {
                        if west_frame.extent_x() + west_frame.extent_width() < other_frame.extent_x() + other_frame.extent_width() {
                            neighbors.insert(Direction::Left, other.clone());
                        }
                    } else {
                        neighbors.insert(Direction::Left, other.clone());
                    }
                }

                if frame.extent_x() + frame.extent_width() <= other_frame.extent_x() {
                    if let Some(east) = neighbors.get(&Direction::Right)
                        && let Some(east_frame) = frame_for_window(east)
                    {
                        if other_frame.extent_x() < east_frame.extent_x() {
                            neighbors.insert(Direction::Right, other.clone());
                        }
                    } else {
                        neighbors.insert(Direction::Right, other.clone());
                    }
                }
            }

            if fill_mode == FillMode::Vertical
                || fill_mode == FillMode::Both
                    && segment_overlap(
                        frame.extent_x(),
                        frame.extent_x() + frame.extent_width(),
                        other_frame.extent_x(),
                        other_frame.extent_x() + other_frame.extent_width(),
                    ) != 0
            {
                if other_frame.extent_y() + other_frame.extent_height() <= frame.extent_y() {
                    if let Some(north) = neighbors.get(&Direction::Up)
                        && let Some(north_frame) = frame_for_window(north)
                    {
                        if north_frame.extent_y() + north_frame.extent_height() < other_frame.extent_y() + other_frame.extent_height() {
                            neighbors.insert(Direction::Up, other.clone());
                        }
                    } else {
                        neighbors.insert(Direction::Up, other.clone());
                    }
                }

                if frame.extent_y() + frame.extent_height() <= other_frame.extent_y() {
                    if let Some(south) = neighbors.get(&Direction::Down)
                        && let Some(south_frame) = frame_for_window(south)
                    {
                        if other_frame.extent_y() < south_frame.extent_y() {
                            neighbors.insert(Direction::Down, other.clone());
                        }
                    } else {
                        neighbors.insert(Direction::Down, other.clone());
                    }
                }
            }
        }
    }

    let full = match fill_mode {
        FillMode::Both => output_geometry,
        FillMode::Vertical => Rectangle::new(
            (frame.extent_x(), output_geometry.loc.y).into(),
            (frame.extent_width(), output_geometry.size.h).into(),
        ),
        FillMode::Horizontal => Rectangle::new(
            (output_geometry.loc.x, frame.extent_y()).into(),
            (output_geometry.size.w, frame.extent_height()).into(),
        ),
    };

    let x = if let Some(west) = neighbors.get(&Direction::Left)
        && let Some(west_frame) = frame_for_window(west)
    {
        full.loc.x + (west_frame.extent_x() + west_frame.extent_width() - full.loc.x).max(0)
    } else {
        full.loc.x
    };

    let width_base = full.size.w - (x - full.loc.x);
    let width = if let Some(east) = neighbors.get(&Direction::Right)
        && let Some(east_frame) = frame_for_window(east)
    {
        width_base - (full.size.w - (east_frame.extent_x() - full.loc.x)).max(0)
    } else {
        width_base
    };

    let y = if let Some(north) = neighbors.get(&Direction::Up)
        && let Some(north_frame) = frame_for_window(north)
    {
        full.loc.y + (north_frame.extent_y() + north_frame.extent_height() - full.loc.y).max(0)
    } else {
        full.loc.y
    };

    let height_base = full.size.h - (y - full.loc.y);
    let height = if let Some(south) = neighbors.get(&Direction::Down)
        && let Some(south_frame) = frame_for_window(south)
    {
        height_base - (full.size.h - (south_frame.extent_y() - full.loc.y)).max(0)
    } else {
        height_base
    };

    Rectangle::<_, Logical>::new((x, y).into(), (width, height).into())
}

#[allow(clippy::too_many_arguments)]
fn overlap(x0: i32, y0: i32, x1: i32, y1: i32, tx0: i32, ty0: i32, tx1: i32, ty1: i32) -> u64 {
    segment_overlap(x0, x1, tx0, tx1) * segment_overlap(y0, y1, ty0, ty1)
}

fn segment_overlap(x0: i32, x1: i32, tx0: i32, tx1: i32) -> u64 {
    let x0 = if tx0 > x0 { tx0 } else { x0 };
    let x1 = if tx1 < x1 { tx1 } else { x1 };

    (x1 - x0).max(0) as u64
}
