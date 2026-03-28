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

use std::collections::HashMap;

use bytes::Bytes;
use smithay::{
    desktop::{layer_map_for_output, space::SpaceElement},
    output::{Mode, Output, Scale, WeakOutput},
    reexports::{calloop::LoopHandle, wayland_server::backend::GlobalId},
    utils::{Logical, Physical, Point, Raw, Rectangle, Size, Transform},
};
use xfconf::ChannelExtManual;

use crate::{
    backend::Backend,
    core::{
        drawing::zoom::ZoomState,
        placement::StackLocation,
        shell::{WindowElement, WindowState},
        state::Xfwl4State,
        util::{Direction, OutputExt, is_laptop_display_name},
    },
    protocols::output_management::{
        OutputManagementState,
        wlr_output_management::{
            ConfiguredMode, OutputConfigurationUpdate, WlrOutputConfiguration, WlrOutputManagementHandler, WlrOutputManagementState,
        },
        xfce_output_management::{XfceOutputManagementHandler, XfceOutputManagementState},
    },
};

const DISPLAYS_CHANNEL_NAME: &str = "displays-wl";
const DPI_AT_1X_SCALE: u32 = 132;

pub struct OutputsConfig {
    initialized: bool,
    configs: Vec<OutputConfig>,
    output_management_state: OutputManagementState,
}

impl OutputsConfig {
    pub fn new(output_management_state: OutputManagementState) -> Self {
        Self {
            initialized: false,
            configs: Vec::new(),
            output_management_state,
        }
    }

    pub(in crate::core) fn outputs(&self) -> Vec<(GlobalId, Output)> {
        self.configs
            .iter()
            .flat_map(|config| {
                config
                    .global_id
                    .as_ref()
                    .and_then(|global_id| config.output.upgrade().map(|output| (global_id.clone(), output)))
            })
            .collect()
    }

    pub(in crate::core) fn zoom_state_for_output_mut<'a>(&'a mut self, output: &Output) -> Option<&'a mut ZoomState> {
        self.config_for_output_mut(output).map(|config| &mut config.zoom_state)
    }

    fn config_for_output_mut(&mut self, output: &Output) -> Option<&mut OutputConfig> {
        self.configs.iter_mut().find(|config| config.output == *output)
    }

    fn remove_config_for_output(&mut self, output: &Output) -> Option<OutputConfig> {
        if let Some(pos) = self.configs.iter().position(|config| config.output == *output) {
            Some(self.configs.remove(pos))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct OutputConfig {
    pub global_id: Option<GlobalId>,
    pub output: WeakOutput,
    pub edid: Bytes,
    pub enabled: bool,
    pub preferred_mode: Option<Mode>,
    pub current_mode: Option<Mode>,
    pub scale: Scale,
    pub transform: Transform,
    pub location: Point<i32, Logical>,
    pub zoom_state: ZoomState,
}

impl OutputConfig {
    fn new(output: Output, edid: Bytes) -> Self {
        Self {
            global_id: None,
            output: output.downgrade(),
            edid,
            enabled: false,
            preferred_mode: output.preferred_mode(),
            current_mode: output.current_mode(),
            scale: output.current_scale(),
            transform: output.current_transform(),
            location: output.current_location(),
            zoom_state: ZoomState::default(),
        }
    }

    fn is_laptop_display(&self) -> bool {
        self.output
            .upgrade()
            .map(|output| is_laptop_display_name(&output.name()))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub struct OutputConfigChange {
    pub preferred_mode: Option<Option<Mode>>,
    pub current_mode: Option<Option<Mode>>,
    pub scale: Option<Scale>,
    pub transform: Option<Transform>,
    pub location: Option<Point<i32, Logical>>,
}

impl OutputConfigChange {
    pub fn new_disabled() -> Self {
        Self {
            current_mode: Some(None),
            preferred_mode: None,
            scale: None,
            transform: None,
            location: None,
        }
    }
}

enum OutputChange {
    Removed {
        output: Output,
        windows_on_output: Vec<WindowElement>,
    },
    Resized {
        output: Output,
        windows_on_output: Vec<WindowElement>,
    },
}

#[derive(Debug)]
struct DefaultDisplayConfig {
    mode: Mode,
    position: Point<i32, Logical>,
    transform: Transform,
    scale: Option<Scale>,
}

impl DefaultDisplayConfig {
    fn load(channel: &xfconf::Channel, connector: &str, target_edid_hash: &str) -> Option<Self> {
        let mkprop = |connector: &str, prop_name: &str| format!("/Default/{connector}/{prop_name}");
        let parse_resolution = |s: String| {
            let mut parts = s.splitn(2, "x");
            let x = parts.next()?;
            let y = parts.next()?;
            if parts.next().is_none() {
                let x = x.parse::<u32>().ok()? as i32;
                let y = y.parse::<u32>().ok()? as i32;
                Some((x, y))
            } else {
                None
            }
        };
        let parse_refresh = |rr: f64| (rr > 0.).then(|| (rr * 1000.).round() as i32);
        let parse_transform = |reflection: Option<String>, rotation: Option<i32>| {
            let reflection = reflection.as_deref().unwrap_or("0");
            let rotation = rotation.unwrap_or(0);

            match reflection {
                "X" => match rotation {
                    90 => Transform::Flipped90,
                    180 => Transform::Flipped180,
                    270 => Transform::Flipped270,
                    _ => Transform::Flipped,
                },

                "Y" => match rotation {
                    90 => Transform::Flipped270,
                    180 => Transform::Flipped,
                    270 => Transform::Flipped90,
                    _ => Transform::Flipped180,
                },

                "XY" => match rotation {
                    90 => Transform::_270,
                    180 => Transform::Normal,
                    270 => Transform::_90,
                    _ => Transform::_180,
                },

                _ => match rotation {
                    90 => Transform::_90,
                    180 => Transform::_180,
                    270 => Transform::_270,
                    _ => Transform::Normal,
                },
            }
        };
        let parse_scale = |scale: f64| Scale::Custom {
            advertised_integer: scale.ceil() as i32,
            fractional: scale,
        };

        if let Some(true) = channel.get_property::<bool>(&mkprop(connector, "Active"))
            && let Some(edid_hash) = channel.get_property::<String>(&mkprop(connector, "EDID"))
            && edid_hash == target_edid_hash
            && let Some((xres, yres)) = channel
                .get_property::<String>(&mkprop(connector, "Resolution"))
                .and_then(parse_resolution)
            && let Some(refresh_rate_millihz) = channel
                .get_property::<f64>(&mkprop(connector, "RefreshRate"))
                .and_then(parse_refresh)
            && let Some(xpos) = channel.get_property::<i32>(&mkprop(connector, "Position/X"))
            && let Some(ypos) = channel.get_property::<i32>(&mkprop(connector, "Position/Y"))
        {
            let reflection = channel.get_property::<String>(&mkprop(connector, "Reflection"));
            let rotation = channel.get_property::<i32>(&mkprop(connector, "Rotation"));
            let transform = parse_transform(reflection, rotation);
            let scale = channel.get_property::<f64>(&mkprop(connector, "Scale")).map(parse_scale);

            Some(Self {
                mode: Mode {
                    size: (xres, yres).into(),
                    refresh: refresh_rate_millihz,
                },
                position: (xpos, ypos).into(),
                transform,
                scale,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputAndRect {
    pub output: Output,
    pub rect: Rectangle<i32, Logical>,
}

impl<BackendData: Backend + 'static> Xfwl4State<BackendData> {
    pub fn initialize_outputs(&mut self) {
        let mut enabled_outputs = Vec::new();

        // First try to look up the default configurations for all outputs in xfconf, and enable
        // them if successful.
        let channel = xfconf::Channel::new(DISPLAYS_CHANNEL_NAME);
        for config in &mut self.core.outputs_config.configs {
            if let Some(output) = config.output.upgrade() {
                let edid_hash = {
                    let edid = config.edid.clone();
                    let edid_bytes = glib::Bytes::from_owned(edid);
                    glib::compute_checksum_for_bytes(glib::ChecksumType::Sha1, &edid_bytes)
                        .as_ref()
                        .map(ToString::to_string)
                };

                if let Some(edid_hash) = edid_hash
                    && let Some(default_config) = DefaultDisplayConfig::load(&channel, &output.name(), &edid_hash)
                {
                    match self.backend.set_output_mode(self.core.handle.clone(), &output, default_config.mode) {
                        Ok((_, new_mode)) => {
                            tracing::info!(
                                "Enabled output {} at {}x{}@{}Hz",
                                output.name(),
                                new_mode.size.w,
                                new_mode.size.h,
                                new_mode.refresh as f64 / 1_000.
                            );

                            let scale = default_config.scale.unwrap_or_else(|| {
                                guess_output_scale(output.physical_properties().size, Some(default_config.mode.size), &output.name())
                            });
                            output.change_current_state(
                                Some(new_mode),
                                Some(default_config.transform),
                                Some(scale),
                                Some(default_config.position),
                            );

                            enabled_outputs.push(output);
                        }
                        Err(err) => tracing::warn!("Failed to configure output {}: {err}", output.name()),
                    }
                } else {
                    tracing::debug!("No default configuration found for output {}", output.name());
                }
            }
        }

        if enabled_outputs.is_empty() {
            tracing::debug!("No outputs from default profile enabled; attempting to enable everything");

            for config in &mut self.core.outputs_config.configs {
                if let Some(output) = config.output.upgrade() {
                    if let Some(mode) = output
                        .current_mode()
                        .or_else(|| output.preferred_mode())
                        .or_else(|| output.modes().first().cloned())
                    {
                        match self.backend.set_output_mode(self.core.handle.clone(), &output, mode) {
                            Ok((_, new_mode)) => {
                                tracing::info!(
                                    "Enabled output {} at {}x{}@{}Hz",
                                    output.name(),
                                    new_mode.size.w,
                                    new_mode.size.h,
                                    new_mode.refresh as f64 / 1_000.
                                );

                                let x = enabled_outputs.iter().fold(0, |acc, o| {
                                    let width = o
                                        .current_mode()
                                        .map(|mode| mode.size.to_f64().to_logical(o.current_scale().fractional_scale()).to_i32_round().w)
                                        .unwrap_or(0);
                                    acc + width
                                });
                                let position = (x, 0).into();

                                output.change_current_state(Some(new_mode), None, None, Some(position));
                                enabled_outputs.push(output);
                            }
                            Err(err) => tracing::warn!("Failed to configure output {}: {err}", output.name()),
                        }
                    } else {
                        tracing::info!("No valid mode found for output {}", output.name());
                    }
                }
            }
        }

        if self.core.outputs_config.configs.is_empty() {
            tracing::info!("No outputs present to enable");
        } else if enabled_outputs.is_empty() {
            tracing::warn!("Failed to enable any outputs");
        } else {
            for output in enabled_outputs {
                self.output_enabled(&output);
            }
        }

        self.core.outputs_config.initialized = true;
    }

    pub(crate) fn output_created(&mut self, output: &Output, edid: Bytes) {
        tracing::debug!("New output {}", output.name());
        let mut config = OutputConfig::new(output.clone(), edid);

        #[cfg(feature = "debug")]
        if let Some(debug) = self.core.debug.as_ref() {
            output
                .user_data()
                .insert_if_missing(|| std::cell::RefCell::new(crate::core::debug::RenderDebug::new(debug)));
        }

        config.scale = guess_output_scale(
            output.physical_properties().size,
            output.current_mode().map(|mode| mode.size),
            &output.name(),
        );
        output.change_current_state(None, None, Some(config.scale), None);

        let edid = config.edid.clone();
        self.core.outputs_config.configs.push(config);
        self.core
            .outputs_config
            .output_management_state
            .output_created::<Self>(output, edid);

        if self.core.outputs_config.initialized
            && !self.core.outputs_config.configs.iter().any(|config| config.enabled)
            && let Some(mode) = output.current_mode().or_else(|| output.preferred_mode())
        {
            tracing::debug!("Output connected and no other outputs enabled; trying to enable this one");
            if try_enable_output(&mut self.backend, &self.core.handle, output, mode) {
                self.output_enabled(output);
            }
        }
    }

    pub(crate) fn output_enabled(&mut self, output: &Output) {
        if let Some(config) = self.core.outputs_config.config_for_output_mut(output)
            && config.global_id.is_none()
        {
            let global_id = output.create_global::<Self>(&self.core.display_handle);
            config.global_id = Some(global_id);

            self.output_changed_internal(output);
        }
    }

    pub(crate) fn output_changed(&mut self, output: &Output) {
        self.output_changed_internal(output);
    }

    fn output_changed_internal(&mut self, output: &Output) {
        let pre_change_windows_on_output = self.windows_visible_on_output(output);
        let mut refresh_decoration_scale = false;

        if let Some(config) = self.core.outputs_config.config_for_output_mut(output) {
            if config.global_id.is_some() {
                let newly_enabled = !config.enabled;
                let size_changed = config.current_mode != output.current_mode()
                    || config.scale.integer_scale() != output.current_scale().integer_scale()
                    || config.scale.fractional_scale() != output.current_scale().fractional_scale()
                    || config.transform != output.current_transform();
                refresh_decoration_scale = size_changed;
                let location_changed = config.location != output.current_location();
                let old_location = config.location;

                config.enabled = true;
                config.preferred_mode = output.preferred_mode();
                config.current_mode = output.current_mode();
                config.scale = output.current_scale();
                config.transform = output.current_transform();
                config.location = output.current_location();

                if newly_enabled || location_changed {
                    self.core.workspace_manager.map_output(output, config.location);
                }

                if newly_enabled || location_changed || size_changed {
                    layer_map_for_output(output).arrange();
                    self.core.workspace_manager.refresh_spaces();
                }

                if size_changed {
                    if !newly_enabled {
                        self.fixup_window_positions(OutputChange::Resized {
                            output: output.clone(),
                            windows_on_output: pre_change_windows_on_output,
                        });
                    }
                    self.backend.reset_buffers(output);
                } else if location_changed && !newly_enabled {
                    let delta = output.current_location() - old_location;
                    for window in &pre_change_windows_on_output {
                        let current_loc = self
                            .core
                            .workspace_manager
                            .workspaces()
                            .iter()
                            .find_map(|workspace| workspace.window_location(window));
                        if let Some(loc) = current_loc {
                            self.core.workspace_manager.relocate_window(window, loc + delta, false);
                        }
                    }
                    self.reapply_anchored_layouts_on_output(output);
                }

                self.core
                    .outputs_config
                    .output_management_state
                    .output_changed::<Self>(output, true);
            } else if config.enabled {
                config.enabled = false;

                output.leave_all();
                self.core.workspace_manager.unmap_output(output);
                self.core.workspace_manager.refresh_spaces();
                self.fixup_window_positions(OutputChange::Removed {
                    output: output.clone(),
                    windows_on_output: pre_change_windows_on_output,
                });

                self.core
                    .outputs_config
                    .output_management_state
                    .output_changed::<Self>(output, false);
            }
        } else {
            tracing::warn!("Got output_changed for unknown output {}", output.name());
        }

        if refresh_decoration_scale {
            for window in self.windows_visible_on_output(output) {
                self.core.workspace_manager.update_window_decorations_scale(&window);
            }
        }

        #[cfg(feature = "xwayland")]
        {
            self.x11_update_desktop_geometry();
            self.x11_update_workarea();
            self.x11_update_scale();
        }
    }

    fn windows_visible_on_output(&self, output: &Output) -> Vec<WindowElement> {
        self.core
            .workspace_manager
            .workspaces()
            .iter()
            .enumerate()
            .flat_map(|(ws_num, workspace)| {
                workspace
                    .visible_windows()
                    .filter(move |window| {
                        (!window.sticky() || ws_num == 0) && workspace.outputs_for_window(window).iter().any(|o| o == output)
                    })
                    .cloned()
            })
            .collect()
    }

    fn output_disabled(&mut self, output: &Output) {
        if let Some(config) = self.core.outputs_config.config_for_output_mut(output)
            && let Some(global_id) = config.global_id.take()
        {
            self.output_changed_internal(output);
            self.core.display_handle.remove_global::<Self>(global_id);
        }
    }

    pub(crate) fn output_destroyed(&mut self, output: &Output) {
        self.output_disabled(output);
        if self.core.outputs_config.remove_config_for_output(output).is_some() {
            self.core.outputs_config.output_management_state.output_destroyed(output);
        }

        if self.core.outputs_config.initialized && !self.core.outputs_config.configs.iter().any(|config| config.enabled) {
            tracing::debug!("Output destroyed and no other outputs enabled; trying to enable one");
            let output_info = self
                .core
                .outputs_config
                .configs
                .iter()
                .position(|output| output.is_laptop_display() && self.core.is_laptop_lid_open())
                .or_else(|| (!self.core.outputs_config.configs.is_empty()).then_some(0))
                .and_then(|i| self.core.outputs_config.configs.get_mut(i))
                .and_then(|config| config.output.upgrade())
                .and_then(|output| output.current_mode().map(|mode| (output, mode)));

            if let Some((output, mode)) = output_info
                && try_enable_output(&mut self.backend, &self.core.handle, &output, mode)
            {
                self.output_enabled(&output);
            }
        }
    }

    #[cfg(feature = "xwayland")]
    pub(in crate::core) fn x11_update_desktop_geometry(&self) {
        if let Some(xw) = self.core.xwayland.as_ref() {
            let full_geometry = self
                .core
                .outputs_config
                .configs
                .iter()
                .flat_map(|config| {
                    config.output.upgrade().and_then(|output| {
                        output.geometry().map(|geom| {
                            geom.to_f64()
                                .to_physical(output.current_scale().fractional_scale())
                                .to_i32_round::<i32>()
                        })
                    })
                })
                .reduce(|accum, geom| accum.merge(geom))
                .unwrap_or_default();
            xw.update_net_desktop_geometry((full_geometry.size.w as u32, full_geometry.size.h as u32).into());
        }
    }

    fn fixup_window_positions(&mut self, change: OutputChange) {
        let (affected_output, pre_captured, is_removal) = match change {
            OutputChange::Removed { output, windows_on_output } => (output, windows_on_output, true),
            OutputChange::Resized { output, windows_on_output } => (output, windows_on_output, false),
        };

        let mut affected: Vec<WindowElement> = pre_captured;
        for (workspace_num, workspace) in self.core.workspace_manager.workspaces().iter().enumerate() {
            for window in workspace.visible_windows() {
                if (!window.sticky() || workspace_num == 0)
                    && window.current_layout() != WindowLayout::Normal
                    && window.props().anchored_output.as_ref().and_then(|w| w.upgrade()).as_ref() == Some(&affected_output)
                    && !affected.iter().any(|w| w == window)
                {
                    affected.push(window.clone());
                }
            }
        }

        let pointer_output_and_geometry = self
            .core
            .workspace_manager
            .output_under(self.core.pointer.current_location())
            .next()
            .or_else(|| self.core.workspace_manager.outputs().next())
            .and_then(|output| {
                self.core
                    .workspace_manager
                    .output_geometry(output)
                    .map(|geom| (output.clone(), geom))
            });

        if let Some((pointer_output, pointer_output_geometry)) = pointer_output_and_geometry {
            #[allow(clippy::mutable_key_type)]
            let all_output_geometries = self
                .core
                .workspace_manager
                .outputs()
                .flat_map(|output| {
                    self.core
                        .workspace_manager
                        .output_geometry(output)
                        .map(|geom| (output.clone(), geom))
                })
                .collect::<HashMap<_, _>>();

            let remaining_geometries: Vec<Rectangle<i32, Logical>> = all_output_geometries.values().cloned().collect();

            let mut relayout_windows: Vec<(WindowElement, WindowLayout, Output, Rectangle<i32, Logical>)> = Vec::new();
            let mut orphaned_windows = Vec::new();
            let mut untile_windows = Vec::new();
            let mut added_outputs = Vec::new();
            let mut removed_outputs = Vec::new();

            for window in &affected {
                let layout = window.current_layout();
                if layout != WindowLayout::Normal {
                    let (output, output_geom) = window
                        .props()
                        .anchored_output
                        .as_ref()
                        .and_then(WeakOutput::upgrade)
                        .and_then(|output| all_output_geometries.get(&output).cloned().map(|geom| (output, geom)))
                        .unwrap_or_else(|| (pointer_output.clone(), pointer_output_geometry));
                    relayout_windows.push((window.clone(), layout, output, output_geom));
                } else {
                    let window_location = self
                        .core
                        .workspace_manager
                        .workspaces()
                        .iter()
                        .find_map(|workspace| workspace.window_location(window));
                    if let Some(window_location) = window_location
                        && !remaining_geometries.iter().any(|g| g.contains(window_location))
                    {
                        orphaned_windows.push(window.clone());
                    }
                }

                if is_removal {
                    removed_outputs.push(window.clone());
                }
            }

            for (window, layout, output, output_geom) in relayout_windows.into_iter() {
                match self.apply_anchored_layout(&window, layout, &output, output_geom) {
                    Some(new_outputs) if !new_outputs.is_empty() => added_outputs.push((window, new_outputs)),
                    Some(_) => (),
                    None => untile_windows.push(window),
                }
            }

            for window in untile_windows {
                self.set_window_untiled(&window, None);
                let loc = self
                    .core
                    .workspace_manager
                    .workspaces()
                    .iter()
                    .find_map(|workspace| workspace.window_location(&window));
                if let Some(loc) = loc
                    && !remaining_geometries.iter().any(|g| g.contains(loc))
                {
                    orphaned_windows.push(window);
                }
            }

            for window in orphaned_windows.into_iter() {
                self.place_window(&window, SpaceElement::geometry(&window.0).size, StackLocation::Top, false);
            }

            for window in removed_outputs {
                self.core.toplevel_changed(
                    &window,
                    None,
                    None,
                    WindowState::empty(),
                    WindowState::empty(),
                    Vec::new(),
                    vec![affected_output.clone()],
                    None,
                );
            }

            for (window, outputs_added) in added_outputs {
                self.core.toplevel_changed(
                    &window,
                    None,
                    None,
                    WindowState::empty(),
                    WindowState::empty(),
                    outputs_added,
                    Vec::new(),
                    None,
                );
            }
        }
    }

    pub(in crate::core) fn outputs_and_rects(&self) -> Vec<OutputAndRect> {
        self.core
            .outputs_config
            .outputs()
            .into_iter()
            .flat_map(|(_, output)| output.geometry().map(|rect| OutputAndRect { output, rect }))
            .collect()
    }

    pub(in crate::core) fn output_and_rect_for_window(&self, window: &WindowElement) -> Option<OutputAndRect> {
        let outputs = self.core.outputs_config.outputs();
        self.core
            .workspace_manager
            .active_workspace()
            .outputs_for_window(window)
            .into_iter()
            .next()
            .and_then(|output| {
                outputs.iter().find_map(|(_, an_output)| {
                    if output == *an_output {
                        output.geometry().map(|geom| (output.clone(), geom))
                    } else {
                        None
                    }
                })
            })
            .map(|(output, rect)| OutputAndRect { output, rect })
    }
}

pub fn scale_from_fractional(scale: f64) -> Scale {
    Scale::Custom {
        advertised_integer: scale.ceil() as i32,
        fractional: scale,
    }
}

impl<BackendData: Backend + 'static> WlrOutputManagementHandler for Xfwl4State<BackendData> {
    fn wlr_output_management_state(&mut self) -> &mut WlrOutputManagementState {
        self.core.outputs_config.output_management_state.wlr_output_management_state()
    }

    fn on_test_configuration(&mut self, configuration: WlrOutputConfiguration) {
        tracing::debug!("test configuration {configuration:?}");

        if configuration
            .updates()
            .iter()
            .any(|update| matches!(update, OutputConfigurationUpdate::Enable(head) if head.adaptive_sync().is_some()))
        {
            configuration.send_failed();
        } else {
            configuration.send_succeeded();
        }
    }

    fn on_apply_configuration(&mut self, configuration: WlrOutputConfiguration) {
        tracing::debug!("apply configuration {configuration:?}");

        #[derive(Debug, Default)]
        struct OutputChanges {
            enabled: Vec<Output>,
            changed: Vec<Output>,
            disabled: Vec<Output>,
        }

        let res = configuration
            .updates()
            .iter()
            .try_fold(OutputChanges::default(), |mut changes, update| {
                if let Some((output, config_change)) = match update {
                    OutputConfigurationUpdate::Enable(head) => head.output().map(|output| {
                        (
                            output,
                            OutputConfigChange {
                                current_mode: head.mode().map(|mode| {
                                    Some(match mode {
                                        ConfiguredMode::Advertised(mode) => mode,
                                        ConfiguredMode::Custom { width, height, refresh } => smithay::output::Mode {
                                            size: (width, height).into(),
                                            refresh,
                                        },
                                    })
                                }),
                                scale: head.scale().map(scale_from_fractional),
                                transform: head.transform(),
                                location: head.position(),
                                preferred_mode: None,
                            },
                        )
                    }),
                    OutputConfigurationUpdate::Disable(output) => {
                        output.upgrade().map(|output| (output, OutputConfigChange::new_disabled()))
                    }
                } {
                    match apply_output_config_change(self.core.handle.clone(), &mut self.backend, &output, config_change) {
                        Ok(ApplyResult::NeededEnable(new_mode)) => {
                            tracing::info!(
                                "Enabled output {} at {}x{}@{}Hz",
                                output.name(),
                                new_mode.size.w,
                                new_mode.size.h,
                                new_mode.refresh as f64 / 1_000.
                            );
                            changes.enabled.push(output);
                            Ok(changes)
                        }
                        Ok(ApplyResult::AlreadyEnabled(_)) => {
                            tracing::debug!("Successfully applied config change to output {}", output.name());
                            changes.changed.push(output);
                            Ok(changes)
                        }
                        Ok(ApplyResult::Disabled) => {
                            tracing::debug!("Successfully disabled output {}", output.name());
                            changes.disabled.push(output);
                            Ok(changes)
                        }
                        Err(err) => {
                            tracing::warn!("Failed to apply output config change to output {}: {err}", output.name());
                            Err(changes)
                        }
                    }
                } else {
                    tracing::debug!("No valid output for config; bailing");
                    Err(changes)
                }
            });

        if res.is_ok() {
            configuration.send_succeeded();
        } else {
            configuration.send_failed();
        }

        let changes = match res {
            Ok(res) => res,
            Err(res) => res,
        };

        for output in changes.disabled {
            self.output_disabled(&output);
        }

        for output in changes.changed {
            self.output_changed(&output);
        }

        for output in changes.enabled {
            self.output_enabled(&output);
        }
    }
}

impl<BackendData: Backend + 'static> XfceOutputManagementHandler for Xfwl4State<BackendData> {
    fn xfce_output_management_state(&mut self) -> &mut XfceOutputManagementState {
        self.core.outputs_config.output_management_state.xfce_output_management_state()
    }
}

enum ApplyResult {
    NeededEnable(Mode),
    AlreadyEnabled(Option<Mode>),
    Disabled,
}

fn apply_output_config_change<BackendData: Backend + 'static>(
    handle: LoopHandle<'_, Xfwl4State<BackendData>>,
    backend: &mut BackendData,
    output: &Output,
    config_change: OutputConfigChange,
) -> anyhow::Result<ApplyResult> {
    let result = match config_change.current_mode {
        Some(Some(new_mode)) => {
            let (needed_enable, applied_mode) = backend.set_output_mode(handle, output, new_mode)?;
            if needed_enable {
                ApplyResult::NeededEnable(applied_mode)
            } else {
                ApplyResult::AlreadyEnabled(Some(applied_mode))
            }
        }
        Some(None) => {
            backend.disable_output(output)?;
            ApplyResult::Disabled
        }
        None => ApplyResult::AlreadyEnabled(None),
    };

    let new_mode = match result {
        ApplyResult::NeededEnable(mode) => Some(mode),
        ApplyResult::AlreadyEnabled(mode) => mode,
        ApplyResult::Disabled => None,
    };

    output.change_current_state(new_mode, config_change.transform, config_change.scale, config_change.location);

    Ok(result)
}

fn try_enable_output<BackendData: Backend>(
    backend: &mut BackendData,
    handle: &LoopHandle<'_, Xfwl4State<BackendData>>,
    output: &Output,
    mode: Mode,
) -> bool {
    match backend.set_output_mode(handle.clone(), output, mode) {
        Ok((_, new_mode)) => {
            tracing::info!(
                "Enabled output {} at {}x{}@{}Hz",
                output.name(),
                new_mode.size.w,
                new_mode.size.h,
                new_mode.refresh as f64 / 1_000.
            );

            output.change_current_state(Some(new_mode), None, None, None);
            true
        }
        Err(err) => {
            tracing::warn!("Failed to configure output {}: {err}", output.name());
            false
        }
    }
}

fn guess_output_scale(phys_size: Size<i32, Raw>, resolution: Option<Size<i32, Physical>>, name: &str) -> Scale {
    let Size { w: phys_w, h: phys_h, .. } = phys_size;
    let scale = if phys_w > 0
        && phys_h > 0
        && let Some(Size { w: px_w, h: px_h, .. }) = resolution
    {
        let phys_w = phys_w as f64;
        let phys_h = phys_h as f64;

        let dpi_w = (px_w as f64 / phys_w) * 25.4;
        let dpi_h = (px_h as f64 / phys_h) * 25.4;
        let dpi = ((dpi_w + dpi_h) / 2.).round();

        let iscale = (dpi / (DPI_AT_1X_SCALE as f64)).ceil() as i32;
        // Fractional scale is rounded up to the nearest 0.25 (with a minimum value of 1.0) when
        // we're trying to guess a good scale (but *only* when we're guessing; what the user sets
        // later is what they get).
        let fscale = round_quarter(dpi / (DPI_AT_1X_SCALE as f64)).max(1.);

        Scale::Custom {
            advertised_integer: iscale,
            fractional: fscale,
        }
    } else {
        Scale::Integer(1)
    };

    tracing::debug!("Guessing output scale as {:?} for output {}", scale, name);

    scale
}

#[inline]
fn round_quarter(v: f64) -> f64 {
    (v * 4.).ceil() / 4.
}

pub fn adjacent_monitor_in_direction(
    outputs_and_rects: &[OutputAndRect],
    current_output_and_rect: &OutputAndRect,
    direction: Direction,
) -> Option<OutputAndRect> {
    let cur_rect = current_output_and_rect.rect;
    outputs_and_rects
        .iter()
        .filter(|OutputAndRect { output, .. }| output != &current_output_and_rect.output)
        .filter(|OutputAndRect { rect, .. }| {
            let (in_direction, has_overlap) = match direction {
                Direction::Left => (
                    rect.loc.x + rect.size.w <= cur_rect.loc.x,
                    rect.loc.y < cur_rect.loc.y + cur_rect.size.h && rect.loc.y + rect.size.h > cur_rect.loc.y,
                ),
                Direction::Right => (
                    rect.loc.x >= cur_rect.loc.x + cur_rect.size.w,
                    rect.loc.y < cur_rect.loc.y + cur_rect.size.h && rect.loc.y + rect.size.h > cur_rect.loc.y,
                ),
                Direction::Up => (
                    rect.loc.y + rect.size.h <= cur_rect.loc.y,
                    rect.loc.x < cur_rect.loc.x + cur_rect.size.w && rect.loc.x + rect.size.w > cur_rect.loc.x,
                ),
                Direction::Down => (
                    rect.loc.y >= cur_rect.loc.y + cur_rect.size.h,
                    rect.loc.x < cur_rect.loc.x + cur_rect.size.w && rect.loc.x + rect.size.w > cur_rect.loc.x,
                ),
            };
            in_direction && has_overlap
        })
        .min_by_key(|OutputAndRect { rect, .. }| match direction {
            Direction::Left => cur_rect.loc.x - (rect.loc.x + rect.size.w),
            Direction::Right => rect.loc.x - (cur_rect.loc.x + cur_rect.size.w),
            Direction::Up => cur_rect.loc.y - (rect.loc.y + rect.size.h),
            Direction::Down => rect.loc.y - (cur_rect.loc.y + cur_rect.size.h),
        })
        .cloned()
}
