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

use std::fmt;
#[cfg(feature = "udev")]
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Parser;
#[cfg(feature = "udev")]
use xfwl4::backend::udev::UdevConfig;
#[cfg(feature = "x11")]
use xfwl4::backend::x11::X11Config;

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum ChosenBackend {
    /// Autodetect the backend
    #[default]
    Auto,
    /// Run as a TTY udev client
    #[cfg(feature = "udev")]
    Tty,
    /// Run as an X11 or Wayland client using winit
    #[cfg(feature = "winit")]
    Winit,
    /// Run as an X11 client
    #[cfg(feature = "x11")]
    X11,
}

impl fmt::Display for ChosenBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            #[cfg(feature = "udev")]
            Self::Tty => f.write_str("tty"),
            Self::Winit => f.write_str("winit"),
            #[cfg(feature = "x11")]
            Self::X11 => f.write_str("x11"),
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Which backend to use
    #[arg(long, value_enum, default_value_t)]
    pub backend: ChosenBackend,

    /// Do not set up a "session" on startup when running using the tty backend; that is, do not
    /// export environment details to the user-session systemd or dbus-daemon instances.
    #[cfg(feature = "udev")]
    #[arg(long, default_value_t = false)]
    pub no_session: bool,

    /// GPU DRM device path (backend=tty)
    #[cfg(feature = "udev")]
    #[arg(long, value_name = "PATH")]
    pub drm_device: Option<PathBuf>,

    /// Disable OpenGL ES instancing (allows drawing many objects with a single render call)
    /// (backend=tty)
    #[cfg(feature = "udev")]
    #[arg(long, default_value_t = false)]
    pub disable_gles_instancing: bool,

    /// Disable 10-bit color (backend=tty)
    #[cfg(feature = "udev")]
    #[arg(long, default_value_t = false)]
    pub disable_10bit_color: bool,

    /// Disable direct scanout (backend=tty)
    #[cfg(feature = "udev")]
    #[arg(long, default_value_t = false)]
    pub disable_direct_scanout: bool,

    /// Disable use of Vulkan (backend=x11)
    #[cfg(feature = "x11")]
    #[arg(long, default_value_t = false)]
    pub disable_vulkan: bool,

    /// UI scale factor for XWayland clients (can be fractional)
    #[cfg(feature = "xwayland")]
    #[arg(long, default_value_t = 1.0)]
    pub xwayland_scale: f64,
}

#[cfg(feature = "x11")]
impl From<Cli> for X11Config {
    fn from(value: Cli) -> Self {
        Self {
            disable_vulkan: value.disable_vulkan,
        }
    }
}

#[cfg(feature = "udev")]
impl From<Cli> for UdevConfig {
    fn from(value: Cli) -> Self {
        Self {
            drm_device: value.drm_device,
            disable_gles_instancing: value.disable_gles_instancing,
            disable_10bit_color: value.disable_10bit_color,
            disable_direct_scanout: value.disable_direct_scanout,
        }
    }
}

pub fn parse() -> anyhow::Result<Cli> {
    let mut cli = Cli::parse();

    cli.backend = if let ChosenBackend::Auto = cli.backend {
        if std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("WAYLAND_SOCKET").is_ok() {
            if cfg!(feature = "winit") {
                Ok(ChosenBackend::Winit)
            } else {
                Err(anyhow!(
                    "A Wayland session is already running, but the Winit backend is not enabled"
                ))
            }
        } else if std::env::var("DISPLAY").is_ok() {
            #[cfg(feature = "x11")]
            {
                Ok(ChosenBackend::X11)
            }
            #[cfg(not(feature = "x11"))]
            {
                if cfg!(feature = "winit") {
                    Ok(ChosenBackend::Winit)
                } else {
                    Err(anyhow!(
                        "An X11 session is already running, but neither the Winit nor X11 backends are enabled"
                    ))
                }
            }
        } else {
            #[cfg(feature = "udev")]
            {
                Ok(ChosenBackend::Tty)
            }
            #[cfg(not(feature = "udev"))]
            {
                Err(anyhow!("No suitable backend is availble"))
            }
        }
    } else {
        Ok(cli.backend)
    }?;

    Ok(cli)
}
