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

mod keyboard_config;
mod keyboard_shortcuts;
mod keyboard_shortcuts_config;
mod output_config;
#[cfg(feature = "udev")]
mod pointer_config;
mod ui_settings;
mod xfwl4_config;
mod xfwl4_config_types;
#[cfg(feature = "xwayland")]
mod xsettings_manager;

pub use keyboard_config::{DEFAULT_KEY_REPEAT_DELAY, DEFAULT_KEY_REPEAT_RATE, KeyboardConfig, XkbConfigOwned};
pub use keyboard_shortcuts::{CommandShortcut, ShortcutKey, WmShortcutAction};
pub use keyboard_shortcuts_config::KeyboardShorctutsConfig;
pub use output_config::{
    OutputAndRect, OutputConfig, OutputConfigChange, OutputsConfig, adjacent_monitor_in_direction, scale_from_fractional,
};
#[cfg(feature = "udev")]
pub use pointer_config::PointerConfig;
pub use ui_settings::UiSettings;
pub use xfwl4_config::Xfwl4Config;
pub use xfwl4_config_types::*;
#[cfg(feature = "xwayland")]
pub use xsettings_manager::XSettingsManager;

pub const XFWM4_CHANNEL_NAME: &str = "xfwm4";
