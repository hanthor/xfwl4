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

use std::{os::fd::OwnedFd, rc::Rc};

use gtk::traits::WidgetExt;

use crate::{
    ui::{
        UiProcessState,
        compositor_ui_protocol::{self},
        do_exit, gtk_settings,
        gtk_settings_sync::GtkSettingsSync,
        window_menu,
    },
    util::io::read_until,
};

/// # Safety
///
/// Must be started before the app spawns any other threads.
pub(super) fn run_ui(from_supervisor_rx: OwnedFd) -> anyhow::Result<()> {
    // Wait until the supervisor sends a NUL byte.
    let mut buf = [0u8; 1];
    read_until(&from_supervisor_rx, &mut buf, b"\0")?;
    drop(from_supervisor_rx);

    if let Err(err) = ui_main() {
        tracing::error!("UI process failed to start: {err}");
        do_exit(1);
    } else {
        do_exit(0);
    }
}

fn ui_main() -> anyhow::Result<()> {
    gtk::gdk::set_allowed_backends("wayland");
    gtk::init()?;

    xfconf::init()?;

    let window_menu_anchor = window_menu::create_anchor_window();
    window_menu_anchor.show_all();

    let state = UiProcessState {
        source: None,
        registry: None,
        ui_manager: None,
        tabwin_state: None,
        tabwin: None,
        tabwin_style_provider: None,
        window_menu_anchor,
        window_menu_state: None,
        window_menu: None,
    };

    // Ensure we have a valid display before trying to connect
    let display = gtk::gdk::Display::default()
        .ok_or_else(|| anyhow::anyhow!("Failed to get GTK display - Wayland socket may not be ready"))?;
    let display_name = display.name();
    let state = compositor_ui_protocol::connect(&display_name, state)?;

    let _settings_sync = GtkSettingsSync::new();
    let settings_notifiers = gtk_settings::init_notifiers(Rc::clone(&state));

    gtk::main();

    if let Some(source) = state.borrow_mut().source.take() {
        source.destroy();
    }

    // Safely handle settings cleanup
    if let Some(settings) = gtk::Settings::default() {
        for id in settings_notifiers {
            glib::signal_handler_disconnect(&settings, id);
        }
    }

    Ok(())
}
