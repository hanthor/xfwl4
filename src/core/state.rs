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
    collections::{HashMap, HashSet},
    ffi::CString,
    os::fd::AsFd,
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use smithay::{
    backend::renderer::Texture,
    desktop::PopupManager,
    input::{
        Seat, SeatState,
        keyboard::{Keysym, XkbConfig},
        pointer::{CursorIcon, CursorImageStatus, PointerHandle},
    },
    reexports::{
        calloop::{
            Interest, LoopHandle, LoopSignal, Mode, PostAction, RegistrationToken,
            channel::{self, Sender},
            generic::Generic,
            timer::{TimeoutAction, Timer},
        },
        rustix::process::Pid,
        wayland_server::{
            Client, Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{Clock, Monotonic, Point, Time},
    wayland::{
        alpha_modifier::AlphaModifierState,
        commit_timing::CommitTimingManagerState,
        compositor::{CompositorClientState, CompositorState},
        cursor_shape::CursorShapeManagerState,
        fifo::FifoManagerState,
        fixes::FixesState,
        fractional_scale::FractionalScaleManagerState,
        idle_inhibit::IdleInhibitManagerState,
        idle_notify::IdleNotifierState,
        image_copy_capture::ImageCopyCaptureState,
        input_method::InputMethodManagerState,
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        output::OutputManagerState,
        pointer_constraints::PointerConstraintsState,
        pointer_gestures::PointerGesturesState,
        presentation::PresentationState,
        relative_pointer::RelativePointerManagerState,
        security_context::{SecurityContext, SecurityContextState},
        selection::{data_device::DataDeviceState, primary_selection::PrimarySelectionState, wlr_data_control::DataControlState},
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{XdgShellState, dialog::XdgDialogState},
        },
        shm::ShmState,
        single_pixel_buffer::SinglePixelBufferState,
        socket::ListeningSocketSource,
        tablet_manager::TabletManagerState,
        text_input::TextInputManagerState,
        viewporter::ViewporterState,
        virtual_keyboard::VirtualKeyboardManagerState,
        xdg_activation::XdgActivationState,
        xdg_foreign::XdgForeignState,
        xdg_toplevel_icon::XdgToplevelIconManager,
    },
};
use tracing::{error, info, warn};

use crate::{
    backend::{Backend, BackendType},
    core::{
        config::{
            CommandShortcut, DEFAULT_KEY_REPEAT_DELAY, DEFAULT_KEY_REPEAT_RATE, KeyboardConfig, KeyboardShorctutsConfig, OutputsConfig,
            UiSettings, WmShortcutAction, Xfwl4Config,
        },
        cursor::CursorTheme,
        cycle::CycleList,
        drawing::{
            PointerElement,
            decorations::{DecorBackgroundState, DecorButtonName, DecorButtonState, DecorationTheme},
            wireframe::Wireframe,
        },
        edge::EdgeResistanceState,
        handlers::{
            DecorationState, ExtImageCaptureSourceState, ExtSessionLockState, ForeignToplevelState, ProtocolDelegates,
            data_device::DndIcon, xfwl4_compositor_ui::PendingWindowMenuState,
        },
        input_handler::PendingCycleKey,
        shell::{ActiveMoveGrab, ShellProtocolDelegates, WindowElement, ssd::DecorationInput},
        util::{ClientExt, FreedesktopIconsIconTheme, LaptopLidState, get_laptop_lid_state},
        workspaces::WorkspaceManager,
    },
    protocols::{
        output_management::OutputManagementState,
        wlr_screencopy::WlrScreencopyState,
        xfwl4_compositor_ui::{CompositorUiState, IconSizeHints},
    },
    ui::MainComms,
    util::io::{read_exact, write_all},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WindowClient {
    Wayland(ClientId),
    X11(u32),
}

#[derive(Debug)]
pub(crate) struct ClientState {
    pub compositor_state: CompositorClientState,
    pub security_context: Option<SecurityContext>,
    disconnect_tx: Sender<ClientId>,
}

impl ClientState {
    pub fn new(client_disconnect_tx: Sender<ClientId>) -> Self {
        Self {
            compositor_state: CompositorClientState::default(),
            security_context: None,
            disconnect_tx: client_disconnect_tx,
        }
    }

    pub fn with_security_context(client_disconnect_tx: Sender<ClientId>, security_context: SecurityContext) -> Self {
        Self {
            compositor_state: CompositorClientState::default(),
            security_context: Some(security_context),
            disconnect_tx: client_disconnect_tx,
        }
    }
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, client_id: ClientId, _reason: DisconnectReason) {
        let _ = self.disconnect_tx.send(client_id);
    }
}

pub struct Xfwl4State<BackendData: Backend + 'static> {
    pub(crate) core: Xfwl4Core<BackendData>,
    pub(crate) backend: BackendData,
}

pub struct Xfwl4Core<BackendData: Backend + 'static> {
    pub(in crate::core) is_running: bool,
    pub(in crate::core) socket_name: Option<String>,
    pub(crate) display_handle: DisplayHandle,
    pub(in crate::core) stop_signal: LoopSignal,
    pub(in crate::core) handle: LoopHandle<'static, Xfwl4State<BackendData>>,
    pub(in crate::core) clients_with_windows: HashSet<WindowClient>,
    pub(in crate::core) client_disconnect_tx: Sender<ClientId>,

    pub(in crate::core) config: Xfwl4Config,
    pub(in crate::core) outputs_config: OutputsConfig,

    // desktop
    pub(in crate::core) workspace_manager: WorkspaceManager<BackendData>,
    pub(in crate::core) cycle_list: CycleList,
    pub(in crate::core) popups: PopupManager,
    pub(in crate::core) pending_windows: HashMap<WlSurface, WindowElement>,
    pub(in crate::core) decoration_theme: Option<DecorationTheme>,
    pub(in crate::core) font_map: gtk::pango::FontMap,
    pub(in crate::core) font_options: gtk::cairo::FontOptions,
    pub(in crate::core) icon_theme: FreedesktopIconsIconTheme,
    pub(in crate::core) cursor_theme: CursorTheme,
    pub(in crate::core) ui_settings: UiSettings,
    pub(in crate::core) dnd_drag_threshold: i32,
    pub(in crate::core) double_click_distance: f64,
    pub(in crate::core) double_click_time: Duration,
    pub(in crate::core) laptop_lid_state: Option<LaptopLidState>,

    // UI thread communication
    pub(in crate::core) compositor_ui_state: CompositorUiState,
    window_id_counter: u32,
    pub(in crate::core) cycling_windows: bool,
    pub(in crate::core) tabwin_grabs_active: bool,
    pub(in crate::core) pending_cycle_key: Option<PendingCycleKey>,
    pub(in crate::core) window_menu_anchor: Option<WindowElement>,
    pub(in crate::core) pending_window_menu_state: Option<PendingWindowMenuState<Xfwl4State<BackendData>>>,
    pub(in crate::core) showing_desktop: bool,

    // smithay state
    pub(in crate::core) protocol_delegates: ProtocolDelegates<BackendData>,
    pub(in crate::core) shell_protocol_delegates: ShellProtocolDelegates,

    // rendering
    pub(in crate::core) pointer_element: PointerElement,
    pub(in crate::core) dnd_icon: Option<DndIcon>,
    pub(in crate::core) wireframe: Option<Wireframe>,
    pub(in crate::core) active_move_grab: Option<ActiveMoveGrab>,
    #[cfg(feature = "debug")]
    pub(in crate::core) debug: Option<crate::core::debug::BackendDebug>,

    // input-related fields
    pub(in crate::core) suppressed_keys: Vec<Keysym>,
    pub(in crate::core) seat: Seat<Xfwl4State<BackendData>>,
    pub(in crate::core) keyboard_config: KeyboardConfig,
    pub(in crate::core) clock: Clock<Monotonic>,
    pub(in crate::core) pointer: PointerHandle<Xfwl4State<BackendData>>,
    pub(in crate::core) pointer_window: Option<WindowElement>,
    pub(in crate::core) pointer_constraint_cursor_hint: Option<(WlSurface, Point<f64, Logical>)>,
    pub(in crate::core) edge_resistance: EdgeResistanceState,
    pub(in crate::core) focus_timeout: Option<RegistrationToken>,
    pub(in crate::core) raise_timeout: Option<RegistrationToken>,
    pub(in crate::core) wm_shortcuts: KeyboardShorctutsConfig<WmShortcutAction>,
    pub(in crate::core) command_shortcuts: KeyboardShorctutsConfig<CommandShortcut>,
    pub(in crate::core) last_user_interaction: Time<Monotonic>,

    #[cfg(feature = "xwayland")]
    pub(in crate::core) xwayland_crash_history: crate::core::x11_wm::XWaylandCrashHistory,
    #[cfg(feature = "xwayland")]
    pub(in crate::core) xdisplay: Option<u32>,
    #[cfg(feature = "xwayland")]
    pub(in crate::core) x11conn: Option<(x11rb::rust_connection::RustConnection, usize)>,
    #[cfg(feature = "xwayland")]
    pub(in crate::core) x11_client_mask: u32,

    #[cfg(feature = "debug")]
    pub renderdoc: Option<renderdoc::RenderDoc<renderdoc::V141>>,
}

impl<BackendData: Backend + 'static> Xfwl4State<BackendData> {
    pub fn init(
        display: Display<Xfwl4State<BackendData>>,
        handle: LoopHandle<'static, Xfwl4State<BackendData>>,
        stop_signal: LoopSignal,
        backend_data: BackendData,
        listen_on_socket: bool,
    ) -> Xfwl4State<BackendData> {
        let dh = display.handle();

        let clock = Clock::new();
        let last_user_interaction = clock.now();

        // init wayland clients
        let socket_name = if listen_on_socket {
            let source = ListeningSocketSource::new_auto().unwrap();
            let socket_name = source.socket_name().to_string_lossy().into_owned();
            handle
                .insert_source(source, |client_stream, _, state| {
                    if let Err(err) = state
                        .core
                        .display_handle
                        .insert_client(client_stream, Arc::new(ClientState::new(state.core.client_disconnect_tx.clone())))
                    {
                        warn!("Failed to get credentials for new client: {err}");
                    }
                })
                .expect("Failed to init wayland socket source");
            info!(name = socket_name, "Listening on wayland socket");
            Some(socket_name)
        } else {
            None
        };
        handle
            .insert_source(Generic::new(display, Interest::READ, Mode::Level), |_, display, data| {
                profiling::scope!("dispatch_clients");
                // Safety: we don't drop the display
                unsafe {
                    display.get_mut().dispatch_clients(data).unwrap();
                }
                Ok(PostAction::Continue)
            })
            .expect("Failed to init wayland server source");

        let (config, config_notifier) = Xfwl4Config::new(handle.clone()).expect("Failed to load initial config");
        handle
            .insert_source(config_notifier, |event, _, state| {
                if let channel::Event::Msg(property_name) = event {
                    if property_name == "theme" {
                        if let Err(err) = state.load_decoration_theme() {
                            tracing::warn!("Failed to load theme: {err}");
                        }
                    } else if state.core.config.is_decoration_setting(&property_name) {
                        state.update_window_decorations_properties();
                    } else if property_name == "cycle_tabwin_mode" || property_name == "cycle_preview" {
                        state.update_toplevel_icon_sizes();
                    }
                }
            })
            .expect("Failed to register xfconf xfwm4 source with event loop");

        // init globals
        let compositor_state = CompositorState::new::<Self>(&dh);
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh);
        let data_control_state =
            DataControlState::new::<Self, _>(&dh, Some(&primary_selection_state), |client| !client.has_security_context());
        let mut seat_state = SeatState::new();
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Self>(&dh);
        let decoration_state = DecorationState::new::<BackendData>(&dh, handle.clone());
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_dialog_state = XdgDialogState::new::<Self>(&dh);
        let xdg_toplevel_icon_manager = XdgToplevelIconManager::new::<Self>(&dh);
        let presentation_state = PresentationState::new::<Self>(&dh, clock.id() as u32);
        let fractional_scale_manager_state = FractionalScaleManagerState::new::<Self>(&dh);
        let xdg_foreign_state = XdgForeignState::new::<Self>(&dh);
        let single_pixel_buffer_state = SinglePixelBufferState::new::<Self>(&dh);
        let fifo_manager_state = FifoManagerState::new::<Self>(&dh);
        let commit_timing_manager_state = CommitTimingManagerState::new::<Self>(&dh);
        let cursor_shape_manager_state = CursorShapeManagerState::new::<Self>(&dh);
        TextInputManagerState::new::<Self>(&dh);
        InputMethodManagerState::new::<Self, _>(&dh, |client| !client.has_security_context());
        VirtualKeyboardManagerState::new::<Self, _>(&dh, |client| !client.has_security_context());
        // Expose global only if backend supports relative motion events
        if BackendData::HAS_RELATIVE_MOTION {
            RelativePointerManagerState::new::<Self>(&dh);
        }
        PointerConstraintsState::new::<Self>(&dh);
        if BackendData::HAS_GESTURES {
            PointerGesturesState::new::<Self>(&dh);
        }
        TabletManagerState::new::<Self>(&dh);
        SecurityContextState::new::<Self, _>(&dh, |client| !client.has_security_context());
        FixesState::new::<Self>(&dh);
        AlphaModifierState::new::<Self>(&dh);

        // init input
        let seat_name = backend_data.seat_name();
        let mut seat = seat_state.new_wl_seat(&dh, seat_name.clone());

        let pointer = seat.add_pointer();

        seat.add_keyboard(XkbConfig::default(), DEFAULT_KEY_REPEAT_DELAY, DEFAULT_KEY_REPEAT_RATE)
            .expect("Failed to initialize the keyboard");
        let keyboard_config = KeyboardConfig::new(handle.clone());

        let wm_shortcuts = KeyboardShorctutsConfig::<WmShortcutAction>::new("xfwm4");
        let command_shortcuts = KeyboardShorctutsConfig::<CommandShortcut>::new("commands");

        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);

        let ext_idle_notifier_state = IdleNotifierState::new(&dh, handle.clone());
        IdleInhibitManagerState::new::<Self>(&dh);

        let ext_session_lock_state = ExtSessionLockState::new::<BackendData>(&dh);

        let foreign_toplevel_state = ForeignToplevelState::<BackendData>::new(&dh);

        let ext_image_capture_source_state = ExtImageCaptureSourceState::new::<BackendData>(&dh);
        let image_copy_capture_state = ImageCopyCaptureState::new_with_filter::<Self, _>(&dh, |client| !client.has_security_context());
        let wlr_screencopy_state = WlrScreencopyState::new::<Self, _>(&dh, |client| !client.has_security_context());

        let output_management_state = OutputManagementState::new::<Self, _>(&dh, |client| !client.has_security_context());
        let outputs_config = OutputsConfig::new(output_management_state);

        #[cfg(feature = "xwayland")]
        let xwayland_shell_state = smithay::wayland::xwayland_shell::XWaylandShellState::new::<Self>(&dh.clone());

        #[cfg(feature = "xwayland")]
        smithay::wayland::xwayland_keyboard_grab::XWaylandKeyboardGrabState::new::<Self>(&dh.clone());

        let workspace_manager = WorkspaceManager::new(&dh, &handle);

        let (cursor_theme, notifier) = CursorTheme::new(handle.clone());
        handle
            .insert_source(notifier, |_, _, _state| {
                #[cfg(feature = "xwayland")]
                {
                    _state.x11_update_scale();
                    _state.x11_update_xrm_xcursor();
                }
            })
            .unwrap();

        let ui_settings = UiSettings::new(handle.clone());
        let icon_theme = FreedesktopIconsIconTheme::new(ui_settings.icon_theme_name());
        let font_options = {
            let mut options = gtk::cairo::FontOptions::new().expect("creating cairo FontOptions should not fail");
            options.set_hint_metrics(gtk::cairo::HintMetrics::On);
            options.set_hint_style(ui_settings.hint_style());
            options.set_subpixel_order(ui_settings.subpixel_order());
            options.set_antialias(ui_settings.antialias());
            options
        };
        let dnd_drag_threshold = ui_settings.dnd_drag_threshold();
        let double_click_distance = ui_settings.double_click_distance();
        let double_click_time = ui_settings.double_click_time();

        let laptop_lid_state = get_laptop_lid_state();

        let icon_size_hints = IconSizeHints {
            tabwin_mode: config.cycle_tabwin_mode().into(),
            tabwin_show_window_previews: config.cycle_preview(),
        };
        let compositor_ui_state = CompositorUiState::new::<Self>(&dh, icon_size_hints);

        let (client_disconnect_tx, client_disconnect_rx) = channel::channel();
        handle
            .insert_source(client_disconnect_rx, |event, _, state| {
                if let channel::Event::Msg(client_id) = event {
                    state.core.clients_with_windows.remove(&WindowClient::Wayland(client_id));
                }
            })
            .expect("Failed to insert client disconnect source");

        Xfwl4State {
            backend: backend_data,
            core: Xfwl4Core {
                is_running: true,
                display_handle: dh,
                socket_name,
                stop_signal,
                handle,
                clients_with_windows: HashSet::default(),
                client_disconnect_tx,
                config,
                outputs_config,
                workspace_manager,
                cycle_list: CycleList::default(),
                popups: PopupManager::default(),
                pending_windows: HashMap::new(),
                decoration_theme: None,
                font_map: pangocairo::FontMap::new(),
                font_options,
                icon_theme,
                cursor_theme,
                ui_settings,
                dnd_drag_threshold,
                double_click_distance,
                double_click_time,
                laptop_lid_state,
                compositor_ui_state,
                window_id_counter: 0,
                cycling_windows: false,
                tabwin_grabs_active: false,
                pending_cycle_key: None,
                window_menu_anchor: None,
                pending_window_menu_state: None,
                showing_desktop: false,

                protocol_delegates: ProtocolDelegates::new(
                    commit_timing_manager_state,
                    cursor_shape_manager_state,
                    data_control_state,
                    data_device_state,
                    decoration_state,
                    ext_idle_notifier_state,
                    ext_image_capture_source_state,
                    ext_session_lock_state,
                    fifo_manager_state,
                    foreign_toplevel_state,
                    fractional_scale_manager_state,
                    image_copy_capture_state,
                    keyboard_shortcuts_inhibit_state,
                    output_manager_state,
                    presentation_state,
                    primary_selection_state,
                    seat_state,
                    shm_state,
                    single_pixel_buffer_state,
                    viewporter_state,
                    wlr_screencopy_state,
                    xdg_activation_state,
                    xdg_foreign_state,
                    xdg_toplevel_icon_manager,
                ),
                shell_protocol_delegates: ShellProtocolDelegates::new(
                    compositor_state,
                    layer_shell_state,
                    xdg_dialog_state,
                    xdg_shell_state,
                    #[cfg(feature = "xwayland")]
                    xwayland_shell_state,
                ),

                pointer_element: PointerElement::default(),
                dnd_icon: None,
                wireframe: None,
                active_move_grab: None,
                #[cfg(feature = "debug")]
                debug: crate::core::debug::BackendDebug::new(),

                suppressed_keys: Vec::new(),
                seat,
                keyboard_config,
                pointer,
                pointer_window: None,
                pointer_constraint_cursor_hint: None,
                edge_resistance: EdgeResistanceState::new(),
                focus_timeout: None,
                raise_timeout: None,
                clock,
                wm_shortcuts,
                command_shortcuts,
                last_user_interaction,

                #[cfg(feature = "xwayland")]
                xwayland_crash_history: Default::default(),
                #[cfg(feature = "xwayland")]
                xdisplay: None,
                #[cfg(feature = "xwayland")]
                x11conn: None,
                #[cfg(feature = "xwayland")]
                x11_client_mask: 0,
                #[cfg(feature = "debug")]
                renderdoc: renderdoc::RenderDoc::new().ok(),
            },
        }
    }

    pub fn socket_name(&self) -> Option<&str> {
        self.core.socket_name.as_deref()
    }

    pub fn backend_type(&self) -> BackendType {
        self.backend.backend_type()
    }

    pub fn register_ui_comms(&self, main_comms: MainComms) {
        if let Some(socket_name) = self.socket_name()
            && let Ok(cstr) = CString::new(socket_name)
            && let Err(err) = write_all(&main_comms.to_supervisor, cstr.to_bytes_with_nul())
        {
            tracing::error!("Failed to write Wayland socket name to UI supervisor: {err}");
        }

        self.core
            .handle
            .insert_source(
                Generic::new(main_comms.from_supervisor, Interest::READ, Mode::Level),
                move |_, fd, state| {
                    if let Err(err) = state.handle_ui_client_pid(&fd) {
                        tracing::error!("Failed to read UI client PID from pipe: {err}");
                    }

                    if let Err(err) = write_all(&main_comms.to_supervisor, b"\0") {
                        tracing::error!("Failed to write ACK to UI supervisor process: {err}");
                    }

                    Ok(PostAction::Continue)
                },
            )
            .expect("unable to insert UI supervisor thread comms FD into event loop");
    }

    fn handle_ui_client_pid<FD: AsFd>(&mut self, fd: FD) -> anyhow::Result<()> {
        let mut pid_bytes = [0u8; 4];
        read_exact(fd, &mut pid_bytes)?;
        if let Some(pid) = Pid::from_raw(libc::pid_t::from_ne_bytes(pid_bytes)) {
            self.core.compositor_ui_state.set_ui_client_pid(Some(pid));
            Ok(())
        } else {
            Err(anyhow!("UI process PID invalid"))
        }
    }

    #[cfg(feature = "xwayland")]
    pub fn start_xwayland(&mut self, display_number: Option<u32>) -> anyhow::Result<u32> {
        use smithay::xwayland::{XWayland, XWaylandEvent};
        use std::{cell::RefCell, process::Stdio, rc::Rc};

        let (xwayland, client) = XWayland::spawn(
            &self.core.display_handle,
            display_number,
            std::iter::empty::<(String, String)>(),
            true,
            Stdio::null(),
            Stdio::null(),
            |_| (),
        )?;

        let display_number = xwayland.display_number();

        let xwayland_token = Rc::new(RefCell::new(None));
        let token = self
            .core
            .handle
            .insert_source(xwayland, {
                let xwayland_token = Rc::clone(&xwayland_token);
                move |event, _, data| match event {
                    XWaylandEvent::Ready {
                        x11_socket,
                        display_number,
                    } => {
                        use crate::core::x11_wm::X11;

                    let cursor = data
                        .core
                        .cursor_theme
                        .load_cursor(CursorName::Default)
                        .unwrap_or_else(|_| data.core.cursor_theme.fallback_cursor());
                    let (image, _) = cursor.get_image(1, Duration::ZERO);
                    wm.set_cursor(
                        &image.pixels_rgba,
                        Size::from((image.width as u16, image.height as u16)),
                        Point::from((image.xhot as u16, image.yhot as u16)),
                    )
                    .expect("Failed to set xwayland default cursor");
                    data.core.xwm = Some(wm);
                    data.core.xdisplay = Some(display_number);

                    data.core.x11conn = match x11rb::connect(Some(&format!(":{display_number}"))) {
                        Err(err) => {
                            tracing::warn!("Failed to connect back to XWayland: {err}");
                            None
                        }
                        Ok((x11conn, screen_num)) => {
                            use x11rb::connection::Connection;

                            // The resource mask helps us determine if two `X11Surface`s belong to
                            // the same X11 client.  We can't check the Wayland surface's Client,
                            // because they are all the same client (it's the XWayland connection
                            // with our compositor).  Most (all?) X11 server implementations
                            // reserve a portion of the Window ID to uniquely identify the client
                            // that created the window.  This is not fixed; it's a runtime setting
                            // that the X server returns as part of connection setup.  The mask is
                            // inverted from what we need (it identifies the resource portion of
                            // the ID, not the client portion), so we need to invert it.  We also
                            // don't use the full number of bits, because X11 only uses 29 bits for
                            // resource IDs.
                            data.core.x11_client_mask = x11conn.setup().resource_id_mask & 0x1fffffff;

                            Some((x11conn, screen_num))
                        }
                    }
                }
                XWaylandEvent::Error => {
                    warn!("XWayland crashed on startup");
                    data.core.xwm = None;
                    data.core.xdisplay = None;
                    data.core.x11conn = None;
                }
            })
            .map_err(|err| anyhow!("Failed to insert the XWaylandSource into the event loop: {err}"))?;
        *xwayland_token.borrow_mut() = Some(token);

        Ok(display_number)
    }

    pub fn load_decoration_theme(&mut self) -> anyhow::Result<DecorationTheme> {
        let theme_path = self.core.config.theme_path().ok_or_else(|| anyhow!("Unable to find theme path"))?;
        let renderer = self.backend.renderer(
            #[cfg(feature = "udev")]
            None,
        )?;
        let decoration_theme = DecorationTheme::load(renderer, theme_path, &self.core.config.resolved_theme_colors())?;
        self.core.decoration_theme = Some(decoration_theme.clone());

        self.update_window_decorations_theme(&decoration_theme);
        self.update_toplevel_icon_sizes();

        tracing::debug!("loaded decoration theme");

        Ok(decoration_theme)
    }

    fn update_toplevel_icon_sizes(&mut self) {
        self.core.replace_toplevel_icon_sizes(std::iter::empty());
        if let Some(menu_button) = self
            .core
            .decoration_theme
            .as_ref()
            .and_then(|theme| theme.button_texture(DecorButtonName::Menu, DecorButtonState::Active, DecorBackgroundState::Active))
        {
            let icon_size = menu_button.size().w.min(menu_button.size().h);
            self.core.add_toplevel_icon_size(icon_size);
            self.core.replace_toplevel_icon_sizes(std::iter::once(icon_size));
        }

        self.core.compositor_ui_state.set_icon_size_hints(IconSizeHints {
            tabwin_mode: self.core.config.cycle_tabwin_mode().into(),
            tabwin_show_window_previews: self.core.config.cycle_preview(),
        });
    }

    fn update_window_decorations_theme(&self, decoration_theme: &DecorationTheme) {
        for workspace in self.core.workspace_manager.workspaces() {
            for window in workspace.visible_windows() {
                if let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                    window_decorations.update(DecorationInput::Theme(decoration_theme.clone()));
                }
                #[cfg(feature = "xwayland")]
                self.x11_update_window_frame_extents(window);
            }
        }
    }

    pub(in crate::core) fn update_window_decorations_icon_theme(&self) {
        for workspace in self.core.workspace_manager.workspaces() {
            for window in workspace.visible_windows() {
                if let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                    window_decorations.update(DecorationInput::IconThemeReloaded);
                }
                #[cfg(feature = "xwayland")]
                self.x11_update_window_frame_extents(window);
            }
        }
    }

    pub(in crate::core) fn update_window_decorations_properties(&mut self) {
        for workspace in self.core.workspace_manager.workspaces() {
            for window in workspace.visible_windows() {
                if let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                    window_decorations.update(DecorationInput::ThemePropertiesReloaded);
                }
                #[cfg(feature = "xwayland")]
                self.x11_update_window_frame_extents(window);
            }
        }

        let outputs: Vec<_> = self.core.workspace_manager.outputs().cloned().collect();
        for output in &outputs {
            self.reapply_anchored_layouts_on_output(output);
        }
    }

    pub(in crate::core) fn update_window_decorations_font_options(&self) {
        for workspace in self.core.workspace_manager.workspaces() {
            for window in workspace.visible_windows() {
                if let Some(window_decorations) = window.decoration_state_mut().window_decorations_mut() {
                    window_decorations.update(DecorationInput::FontOptions(self.core.font_options.clone()));
                }
                #[cfg(feature = "xwayland")]
                self.x11_update_window_frame_extents(window);
            }
        }
    }

    pub fn refresh_and_flush_clients(&mut self) {
        profiling::scope!("refresh_and_flush_clients");
        self.core.workspace_manager.refresh_spaces();
        self.core.popups.cleanup();

        if let Err(err) = self.core.display_handle.flush_clients() {
            error!("Fatal error: Failed to flush Wayland clients: {err}");
            std::process::exit(1);
        }
    }

    pub fn shutdown(&mut self) {
        self.core.is_running = false;
        self.core.compositor_ui_state.send_quit();
        self.core.stop_signal.stop();
        self.core.stop_signal.wakeup();
    }
}

impl<BackendData: Backend + 'static> Xfwl4Core<BackendData> {
    pub(in crate::core) fn next_window_id(&mut self) -> u32 {
        let id = self.window_id_counter;
        self.window_id_counter += 1;
        id
    }

    pub(in crate::core) fn client_is_ui_thread(&self, client: Option<Client>) -> bool {
        client
            .and_then(|client| client.get_credentials(&self.display_handle).ok())
            .is_some_and(|creds| {
                self.compositor_ui_state
                    .client_pid()
                    .as_ref()
                    .is_some_and(|pid| pid.as_raw_pid() == creds.pid)
            })
    }

    pub(in crate::core) fn update_last_user_interaction(&mut self, window: &WindowElement) {
        let now = self.clock.now();
        window.props().last_user_interaction = Some(now);
        self.last_user_interaction = now;
    }

    pub(in crate::core) fn set_cursor(&mut self, cursor_name: CursorName) {
        if let Ok(cursor) = self.cursor_theme.load_cursor(cursor_name) {
            self.pointer_image = cursor;
        }

    pub(in crate::core) fn cancel_focus_follows_mouse_timers(&mut self) {
        if let Some(token) = self.focus_timeout.take() {
            self.handle.remove(token);
        }
        if let Some(token) = self.raise_timeout.take() {
            self.handle.remove(token);
        }
    }

    pub(in crate::core) fn set_cursor(&mut self, cursor_icon: CursorIcon) {
        self.pointer_element.set_status(CursorImageStatus::Named(cursor_icon));
    }

    pub(in crate::core) fn is_laptop_lid_open(&self) -> bool {
        self.laptop_lid_state.as_ref().is_some_and(|state| *state == LaptopLidState::Open)
    }

    pub(crate) fn register_timer<F>(&self, timer: Timer, mut timer_fn: F) -> RegistrationToken
    where
        F: FnMut(&mut Xfwl4State<BackendData>) -> TimeoutAction + 'static,
    {
        self.handle
            .insert_source(timer, move |_, _, state| timer_fn(state))
            .expect("Failed to register timer source with event loop")
    }

    pub(crate) fn unregister_timer(&self, token: RegistrationToken) {
        self.handle.remove(token);
    }
}

smithay::delegate_dispatch2!(@<BackendData: Backend + 'static> Xfwl4State<BackendData>);
