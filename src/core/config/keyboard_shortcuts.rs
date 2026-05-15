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

use std::{ffi::OsString, fmt, str::FromStr};

use anyhow::anyhow;
use gtk::gdk::{self, ModifierType};
use xkbcommon::xkb::Keysym;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ShortcutKey {
    pub keysym: Keysym,
    pub modifiers: gdk::ModifierType,
}

impl ShortcutKey {
    pub const DEFAULT_CYCLE_WINDOWS: Self = Self::new(Keysym::Tab, ModifierType::MOD1_MASK);
    pub const DEFAULT_CYCLE_REVERSE_WINDOWS: Self = Self::new(
        Keysym::ISO_Left_Tab,
        ModifierType::from_bits_truncate(ModifierType::MOD1_MASK.bits() | ModifierType::SHIFT_MASK.bits()),
    );
    pub const DEFAULT_UP: Self = Self::new(Keysym::Up, ModifierType::empty());
    pub const DEFAULT_DOWN: Self = Self::new(Keysym::Down, ModifierType::empty());
    pub const DEFAULT_LEFT: Self = Self::new(Keysym::Left, ModifierType::empty());
    pub const DEFAULT_RIGHT: Self = Self::new(Keysym::Right, ModifierType::empty());
    pub const DEFAULT_CANCEL: Self = Self::new(Keysym::Escape, ModifierType::empty());

    pub const fn new(keysym: Keysym, modifiers: gdk::ModifierType) -> Self {
        Self { keysym, modifiers }
    }
}

impl From<ShortcutKey> for (Keysym, gdk::ModifierType) {
    fn from(value: ShortcutKey) -> Self {
        (value.keysym, value.modifiers)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WmShortcutAction {
    Cancel,
    Down,
    Left,
    Right,
    Up,
    AddWorkspace,
    AddAdjacentWorkspace,
    CloseWindow,
    CycleWindows,
    CycleReverseWindows,
    DelWorkspace,
    DelActiveWorkspace,
    DownWorkspace,
    FillHoriz,
    FillVert,
    FillWindow,
    HideWindow,
    LeftWorkspace,
    LowerWindow,
    Move,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MaximizeHoriz,
    MaximizeVert,
    MaximizeWindow,
    MoveToMonitorDown,
    MoveToMonitorLeft,
    MoveToMonitorRight,
    MoveToMonitorUp,
    MoveDownWorkspace,
    MoveLeftWorkspace,
    MoveNextWorkspace,
    MovePrevWorkspace,
    MoveRightWorkspace,
    MoveUpWorkspace,
    NextWorkspace,
    PopupMenu,
    PrevWorkspace,
    RaiseWindow,
    RaiseLowerWindow,
    Resize,
    RightWorkspace,
    ShadeWindow,
    ShowDesktop,
    StickWindow,
    SwitchApplication,
    SwitchWindow,
    TileDown,
    TileLeft,
    TileRight,
    TileUp,
    TileDownLeft,
    TileDownRight,
    TileUpLeft,
    TileUpRight,
    ToggleAbove,
    ToggleFullscreen,
    UpWorkspace,
    MoveWorkspace1,
    MoveWorkspace2,
    MoveWorkspace3,
    MoveWorkspace4,
    MoveWorkspace5,
    MoveWorkspace6,
    MoveWorkspace7,
    MoveWorkspace8,
    MoveWorkspace9,
    MoveWorkspace10,
    MoveWorkspace11,
    MoveWorkspace12,
    Workspace1,
    Workspace2,
    Workspace3,
    Workspace4,
    Workspace5,
    Workspace6,
    Workspace7,
    Workspace8,
    Workspace9,
    Workspace10,
    Workspace11,
    Workspace12,
}

impl WmShortcutAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cancel => "cancel_key",
            Self::Down => "down_key",
            Self::Left => "left_key",
            Self::Right => "right_key",
            Self::Up => "up_key",
            Self::AddWorkspace => "add_workspace_key",
            Self::AddAdjacentWorkspace => "add_adjacent_workspace_key",
            Self::CloseWindow => "close_window_key",
            Self::CycleWindows => "cycle_windows_key",
            Self::CycleReverseWindows => "cycle_reverse_windows_key",
            Self::DelWorkspace => "del_workspace_key",
            Self::DelActiveWorkspace => "del_active_workspace_key",
            Self::DownWorkspace => "down_workspace_key",
            Self::FillHoriz => "fill_horiz_key",
            Self::FillVert => "fill_vert_key",
            Self::FillWindow => "fill_window_key",
            Self::HideWindow => "hide_window_key",
            Self::LeftWorkspace => "left_workspace_key",
            Self::LowerWindow => "lower_window_key",
            Self::Move => "move_window_key",
            Self::MoveLeft => "move_window_left_key",
            Self::MoveRight => "move_window_right_key",
            Self::MoveUp => "move_window_up_key",
            Self::MoveDown => "move_window_down_key",
            Self::MaximizeHoriz => "maximize_horiz_key",
            Self::MaximizeVert => "maximize_vert_key",
            Self::MaximizeWindow => "maximize_window_key",
            Self::MoveToMonitorDown => "move_window_to_monitor_down_key",
            Self::MoveToMonitorLeft => "move_window_to_monitor_left_key",
            Self::MoveToMonitorRight => "move_window_to_monitor_right_key",
            Self::MoveToMonitorUp => "move_window_to_monitor_up_key",
            Self::MoveDownWorkspace => "move_window_down_workspace_key",
            Self::MoveLeftWorkspace => "move_window_left_workspace_key",
            Self::MoveNextWorkspace => "move_window_next_workspace_key",
            Self::MovePrevWorkspace => "move_window_prev_workspace_key",
            Self::MoveRightWorkspace => "move_window_right_workspace_key",
            Self::MoveUpWorkspace => "move_window_up_workspace_key",
            Self::NextWorkspace => "next_workspace_key",
            Self::PopupMenu => "popup_menu_key",
            Self::PrevWorkspace => "prev_workspace_key",
            Self::RaiseWindow => "raise_window_key",
            Self::RaiseLowerWindow => "raiselower_window_key",
            Self::Resize => "resize_window_key",
            Self::RightWorkspace => "right_workspace_key",
            Self::ShadeWindow => "shade_window_key",
            Self::ShowDesktop => "show_desktop_key",
            Self::StickWindow => "stick_window_key",
            Self::SwitchApplication => "switch_application_key",
            Self::SwitchWindow => "switch_window_key",
            Self::TileDown => "tile_down_key",
            Self::TileLeft => "tile_left_key",
            Self::TileRight => "tile_right_key",
            Self::TileUp => "tile_up_key",
            Self::TileDownLeft => "tile_down_left_key",
            Self::TileDownRight => "tile_down_right_key",
            Self::TileUpLeft => "tile_up_left_key",
            Self::TileUpRight => "tile_up_right_key",
            Self::ToggleAbove => "above_key",
            Self::ToggleFullscreen => "fullscreen_key",
            Self::UpWorkspace => "up_workspace_key",
            Self::MoveWorkspace1 => "move_window_workspace_1_key",
            Self::MoveWorkspace2 => "move_window_workspace_2_key",
            Self::MoveWorkspace3 => "move_window_workspace_3_key",
            Self::MoveWorkspace4 => "move_window_workspace_4_key",
            Self::MoveWorkspace5 => "move_window_workspace_5_key",
            Self::MoveWorkspace6 => "move_window_workspace_6_key",
            Self::MoveWorkspace7 => "move_window_workspace_7_key",
            Self::MoveWorkspace8 => "move_window_workspace_8_key",
            Self::MoveWorkspace9 => "move_window_workspace_9_key",
            Self::MoveWorkspace10 => "move_window_workspace_10_key",
            Self::MoveWorkspace11 => "move_window_workspace_11_key",
            Self::MoveWorkspace12 => "move_window_workspace_12_key",
            Self::Workspace1 => "workspace_1_key",
            Self::Workspace2 => "workspace_2_key",
            Self::Workspace3 => "workspace_3_key",
            Self::Workspace4 => "workspace_4_key",
            Self::Workspace5 => "workspace_5_key",
            Self::Workspace6 => "workspace_6_key",
            Self::Workspace7 => "workspace_7_key",
            Self::Workspace8 => "workspace_8_key",
            Self::Workspace9 => "workspace_9_key",
            Self::Workspace10 => "workspace_10_key",
            Self::Workspace11 => "workspace_11_key",
            Self::Workspace12 => "workspace_12_key",
        }
    }
}

impl fmt::Display for WmShortcutAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WmShortcutAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cancel_key" => Ok(Self::Cancel),
            "down_key" => Ok(Self::Down),
            "left_key" => Ok(Self::Left),
            "right_key" => Ok(Self::Right),
            "up_key" => Ok(Self::Up),
            "add_workspace_key" => Ok(Self::AddWorkspace),
            "add_adjacent_workspace_key" => Ok(Self::AddAdjacentWorkspace),
            "close_window_key" => Ok(Self::CloseWindow),
            "cycle_windows_key" => Ok(Self::CycleWindows),
            "cycle_reverse_windows_key" => Ok(Self::CycleReverseWindows),
            "del_workspace_key" => Ok(Self::DelWorkspace),
            "del_active_workspace_key" => Ok(Self::DelActiveWorkspace),
            "down_workspace_key" => Ok(Self::DownWorkspace),
            "fill_horiz_key" => Ok(Self::FillHoriz),
            "fill_vert_key" => Ok(Self::FillVert),
            "fill_window_key" => Ok(Self::FillWindow),
            "hide_window_key" => Ok(Self::HideWindow),
            "left_workspace_key" => Ok(Self::LeftWorkspace),
            "lower_window_key" => Ok(Self::LowerWindow),
            "move_window_key" => Ok(Self::Move),
            "move_window_left_key" => Ok(Self::MoveLeft),
            "move_window_right_key" => Ok(Self::MoveRight),
            "move_window_up_key" => Ok(Self::MoveUp),
            "move_window_down_key" => Ok(Self::MoveDown),
            "maximize_horiz_key" => Ok(Self::MaximizeHoriz),
            "maximize_vert_key" => Ok(Self::MaximizeVert),
            "maximize_window_key" => Ok(Self::MaximizeWindow),
            "move_window_to_monitor_down_key" => Ok(Self::MoveToMonitorDown),
            "move_window_to_monitor_left_key" => Ok(Self::MoveToMonitorLeft),
            "move_window_to_monitor_right_key" => Ok(Self::MoveToMonitorRight),
            "move_window_to_monitor_up_key" => Ok(Self::MoveToMonitorUp),
            "move_window_down_workspace_key" => Ok(Self::MoveDownWorkspace),
            "move_window_left_workspace_key" => Ok(Self::MoveLeftWorkspace),
            "move_window_next_workspace_key" => Ok(Self::MoveNextWorkspace),
            "move_window_prev_workspace_key" => Ok(Self::MovePrevWorkspace),
            "move_window_right_workspace_key" => Ok(Self::MoveRightWorkspace),
            "move_window_up_workspace_key" => Ok(Self::MoveUpWorkspace),
            "next_workspace_key" => Ok(Self::NextWorkspace),
            "popup_menu_key" => Ok(Self::PopupMenu),
            "prev_workspace_key" => Ok(Self::PrevWorkspace),
            "raise_window_key" => Ok(Self::RaiseWindow),
            "raiselower_window_key" => Ok(Self::RaiseLowerWindow),
            "resize_window_key" => Ok(Self::Resize),
            "right_workspace_key" => Ok(Self::RightWorkspace),
            "shade_window_key" => Ok(Self::ShadeWindow),
            "show_desktop_key" => Ok(Self::ShowDesktop),
            "stick_window_key" => Ok(Self::StickWindow),
            "switch_application_key" => Ok(Self::SwitchApplication),
            "switch_window_key" => Ok(Self::SwitchWindow),
            "tile_down_key" => Ok(Self::TileDown),
            "tile_left_key" => Ok(Self::TileLeft),
            "tile_right_key" => Ok(Self::TileRight),
            "tile_up_key" => Ok(Self::TileUp),
            "tile_down_left_key" => Ok(Self::TileDownLeft),
            "tile_down_right_key" => Ok(Self::TileDownRight),
            "tile_up_left_key" => Ok(Self::TileUpLeft),
            "tile_up_right_key" => Ok(Self::TileUpRight),
            "above_key" => Ok(Self::ToggleAbove),
            "fullscreen_key" => Ok(Self::ToggleFullscreen),
            "up_workspace_key" => Ok(Self::UpWorkspace),
            "move_window_workspace_1_key" => Ok(Self::MoveWorkspace1),
            "move_window_workspace_2_key" => Ok(Self::MoveWorkspace2),
            "move_window_workspace_3_key" => Ok(Self::MoveWorkspace3),
            "move_window_workspace_4_key" => Ok(Self::MoveWorkspace4),
            "move_window_workspace_5_key" => Ok(Self::MoveWorkspace5),
            "move_window_workspace_6_key" => Ok(Self::MoveWorkspace6),
            "move_window_workspace_7_key" => Ok(Self::MoveWorkspace7),
            "move_window_workspace_8_key" => Ok(Self::MoveWorkspace8),
            "move_window_workspace_9_key" => Ok(Self::MoveWorkspace9),
            "move_window_workspace_10_key" => Ok(Self::MoveWorkspace10),
            "move_window_workspace_11_key" => Ok(Self::MoveWorkspace11),
            "move_window_workspace_12_key" => Ok(Self::MoveWorkspace12),
            "workspace_1_key" => Ok(Self::Workspace1),
            "workspace_2_key" => Ok(Self::Workspace2),
            "workspace_3_key" => Ok(Self::Workspace3),
            "workspace_4_key" => Ok(Self::Workspace4),
            "workspace_5_key" => Ok(Self::Workspace5),
            "workspace_6_key" => Ok(Self::Workspace6),
            "workspace_7_key" => Ok(Self::Workspace7),
            "workspace_8_key" => Ok(Self::Workspace8),
            "workspace_9_key" => Ok(Self::Workspace9),
            "workspace_10_key" => Ok(Self::Workspace10),
            "workspace_11_key" => Ok(Self::Workspace11),
            "workspace_12_key" => Ok(Self::Workspace12),
            unknown => Err(anyhow!("unknown keyboard shortcut name: {unknown}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandShortcut {
    pub argv0: OsString,
    pub args: Vec<OsString>,
}

impl fmt::Display for CommandShortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.argv0.display(),
            self.args.join(OsString::from_str(" ").unwrap().as_os_str()).display(),
        )
    }
}

impl FromStr for CommandShortcut {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        glib::shell_parse_argv(s)
            .map_err(|err| anyhow!("failed to parse command line '{s}': {err}"))
            .and_then(|argv| {
                let mut iter = argv.into_iter();
                if let Some(argv0) = iter.next() {
                    Ok(Self {
                        argv0,
                        args: iter.collect(),
                    })
                } else {
                    Err(anyhow!("Command is empty"))
                }
            })
    }
}

pub(in crate::core) fn parse_accelerator(accelerator: &str) -> Option<ShortcutKey> {
    const MODIFIER_NAMES: &[(&str, ModifierType)] = &[
        ("release>", ModifierType::RELEASE_MASK),
        // GDK has GDK_CONTROL_MASK hard-coded for <Primary> the Wayland backend
        ("primary>", ModifierType::CONTROL_MASK),
        ("control>", ModifierType::CONTROL_MASK),
        ("shift>", ModifierType::SHIFT_MASK),
        ("shft>", ModifierType::SHIFT_MASK),
        ("ctrl>", ModifierType::CONTROL_MASK),
        ("ctl>", ModifierType::CONTROL_MASK),
        ("alt>", ModifierType::MOD1_MASK),
        ("meta>", ModifierType::META_MASK),
        ("hyper>", ModifierType::HYPER_MASK),
        ("super>", ModifierType::SUPER_MASK),
        ("mod1>", ModifierType::MOD1_MASK),
        ("mod2>", ModifierType::MOD2_MASK),
        ("mod3>", ModifierType::MOD3_MASK),
        ("mod4>", ModifierType::MOD4_MASK),
        ("mod5>", ModifierType::MOD5_MASK),
    ];

    fn test_and_strip_modifier(s: &str) -> Option<(ModifierType, &str)> {
        MODIFIER_NAMES.iter().find_map(|(suffix, mask)| {
            s.get(..suffix.len())
                .filter(|prefix| prefix.eq_ignore_ascii_case(suffix))
                .map(|_| (*mask, &s[suffix.len()..]))
        })
    }

    let mut s = accelerator;

    let mut modifiers = ModifierType::empty();
    while let Some(rest) = s.strip_prefix('<') {
        if let Some((found_mask, rest)) = test_and_strip_modifier(rest) {
            modifiers |= found_mask;
            s = rest;
        } else {
            s = rest.find('>').map_or("", |i| &rest[(i + 1)..]);
        }
    }

    let keysym = xkbcommon::xkb::keysym_from_name(s, xkbcommon::xkb::KEYSYM_NO_FLAGS);
    if keysym == Keysym::NoSymbol && modifiers == ModifierType::empty() {
        None
    } else {
        Some(ShortcutKey { keysym, modifiers })
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortcutKey, parse_accelerator};
    use gtk::gdk::ModifierType;
    use xkbcommon::xkb::Keysym;

    const PRIMARY: ModifierType = ModifierType::CONTROL_MASK;

    fn parse(s: &str) -> Option<ShortcutKey> {
        parse_accelerator(s)
    }

    fn key(keysym: Keysym, modifiers: ModifierType) -> Option<ShortcutKey> {
        Some(ShortcutKey { keysym, modifiers })
    }

    #[test]
    fn bare_keys() {
        assert_eq!(parse("Down"), key(Keysym::Down, ModifierType::empty()));
        assert_eq!(parse("Escape"), key(Keysym::Escape, ModifierType::empty()));
        assert_eq!(parse("Left"), key(Keysym::Left, ModifierType::empty()));
        assert_eq!(parse("Right"), key(Keysym::Right, ModifierType::empty()));
        assert_eq!(parse("Up"), key(Keysym::Up, ModifierType::empty()));
        assert_eq!(parse("Print"), key(Keysym::Print, ModifierType::empty()));
        assert_eq!(parse("F9"), key(Keysym::F9, ModifierType::empty()));
    }

    #[test]
    fn single_modifier() {
        assert_eq!(parse("<Alt>F1"), key(Keysym::F1, ModifierType::MOD1_MASK));
        assert_eq!(parse("<Alt>space"), key(Keysym::space, ModifierType::MOD1_MASK));
        assert_eq!(parse("<Alt>grave"), key(Keysym::grave, ModifierType::MOD1_MASK));
        assert_eq!(parse("<Alt>Delete"), key(Keysym::Delete, ModifierType::MOD1_MASK));
        assert_eq!(parse("<Super>l"), key(Keysym::l, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>d"), key(Keysym::d, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>f"), key(Keysym::f, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Shift>Print"), key(Keysym::Print, ModifierType::SHIFT_MASK));
        assert_eq!(parse("<Primary>Escape"), key(Keysym::Escape, PRIMARY));
    }

    #[test]
    fn two_modifiers() {
        assert_eq!(
            parse("<Primary><Alt>Delete"),
            key(Keysym::Delete, PRIMARY | ModifierType::MOD1_MASK)
        );
        assert_eq!(parse("<Primary><Alt>Left"), key(Keysym::Left, PRIMARY | ModifierType::MOD1_MASK));
        assert_eq!(parse("<Primary><Alt>Down"), key(Keysym::Down, PRIMARY | ModifierType::MOD1_MASK));
        assert_eq!(parse("<Primary><Alt>KP_1"), key(Keysym::KP_1, PRIMARY | ModifierType::MOD1_MASK));
        assert_eq!(
            parse("<Alt><Super>s"),
            key(Keysym::s, ModifierType::MOD1_MASK | ModifierType::SUPER_MASK)
        );
        assert_eq!(
            parse("<Primary><Shift>Escape"),
            key(Keysym::Escape, PRIMARY | ModifierType::SHIFT_MASK)
        );
        assert_eq!(
            parse("<Shift><Alt>Page_Down"),
            key(Keysym::Page_Down, ModifierType::SHIFT_MASK | ModifierType::MOD1_MASK)
        );
    }

    #[test]
    fn three_modifiers() {
        assert_eq!(
            parse("<Primary><Shift><Alt>Left"),
            key(Keysym::Left, PRIMARY | ModifierType::SHIFT_MASK | ModifierType::MOD1_MASK),
        );
        assert_eq!(
            parse("<Primary><Shift><Alt>Right"),
            key(Keysym::Right, PRIMARY | ModifierType::SHIFT_MASK | ModifierType::MOD1_MASK),
        );
    }

    #[test]
    fn xf86_keys() {
        assert_eq!(parse("XF86Mail"), key(Keysym::XF86_Mail, ModifierType::empty()));
        assert_eq!(parse("XF86Display"), key(Keysym::XF86_Display, ModifierType::empty()));
        assert_eq!(parse("XF86WWW"), key(Keysym::XF86_WWW, ModifierType::empty()));
    }

    #[test]
    fn modifier_aliases() {
        assert_eq!(parse("<Control>F1"), key(Keysym::F1, ModifierType::CONTROL_MASK));
        assert_eq!(parse("<Ctrl>F1"), key(Keysym::F1, ModifierType::CONTROL_MASK));
        assert_eq!(parse("<Ctl>F1"), key(Keysym::F1, ModifierType::CONTROL_MASK));
        assert_eq!(parse("<Primary>F1"), key(Keysym::F1, PRIMARY));
    }

    #[test]
    fn case_insensitive_modifiers() {
        assert_eq!(parse("<alt>F1"), key(Keysym::F1, ModifierType::MOD1_MASK));
        assert_eq!(parse("<ALT>F1"), key(Keysym::F1, ModifierType::MOD1_MASK));
        assert_eq!(parse("<SUPER>l"), key(Keysym::l, ModifierType::SUPER_MASK));
        assert_eq!(parse("<SHIFT>Print"), key(Keysym::Print, ModifierType::SHIFT_MASK));
    }

    #[test]
    fn mod_n_modifiers() {
        assert_eq!(parse("<Mod1>a"), key(Keysym::a, ModifierType::MOD1_MASK));
        assert_eq!(parse("<Mod2>a"), key(Keysym::a, ModifierType::MOD2_MASK));
        assert_eq!(parse("<Mod3>a"), key(Keysym::a, ModifierType::MOD3_MASK));
        assert_eq!(parse("<Mod4>a"), key(Keysym::a, ModifierType::MOD4_MASK));
        assert_eq!(parse("<Mod5>a"), key(Keysym::a, ModifierType::MOD5_MASK));
    }

    #[test]
    fn hex_keysym() {
        assert_eq!(parse("0xff0d"), key(Keysym::Return, ModifierType::empty()));
        assert_eq!(parse("0xff08"), key(Keysym::BackSpace, ModifierType::empty()));
        assert_eq!(parse("0xff1b"), key(Keysym::Escape, ModifierType::empty()));
        assert_eq!(parse("0xff09"), key(Keysym::Tab, ModifierType::empty()));
        assert_eq!(parse("0x0061"), key(Keysym::a, ModifierType::empty()));
        assert_eq!(parse("0x0020"), key(Keysym::space, ModifierType::empty()));
        assert_eq!(parse("0xFFE1"), key(Keysym::Shift_L, ModifierType::empty()));
        assert_eq!(parse("<Alt>0xff08"), key(Keysym::BackSpace, ModifierType::MOD1_MASK));
        assert_eq!(
            parse("<Primary><Alt>0xff0d"),
            key(Keysym::Return, PRIMARY | ModifierType::MOD1_MASK)
        );
        assert_eq!(parse("0x0000"), None);
    }

    #[test]
    fn keypad_keys() {
        assert_eq!(parse("<Super>KP_Down"), key(Keysym::KP_Down, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_Home"), key(Keysym::KP_Home, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_Left"), key(Keysym::KP_Left, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_Right"), key(Keysym::KP_Right, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_Up"), key(Keysym::KP_Up, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_Page_Up"), key(Keysym::KP_Page_Up, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_Next"), key(Keysym::KP_Next, ModifierType::SUPER_MASK));
        assert_eq!(parse("<Super>KP_End"), key(Keysym::KP_End, ModifierType::SUPER_MASK));
    }

    #[test]
    fn invalid_input() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("not_a_real_key_name"), None);
    }

    #[test]
    fn unknown_tag_skipped() {
        assert_eq!(parse("<Bogus>F1"), key(Keysym::F1, ModifierType::empty()));
        assert_eq!(parse("<Bogus><Alt>F1"), key(Keysym::F1, ModifierType::MOD1_MASK));
    }

    #[test]
    fn modifiers_only() {
        let result = parse("<Alt>").unwrap();
        assert_eq!(result.modifiers, ModifierType::MOD1_MASK);
        assert_eq!(result.keysym, Keysym::NoSymbol);
    }

    #[test]
    fn super_tab() {
        assert_eq!(parse("<Super>Tab"), key(Keysym::Tab, ModifierType::SUPER_MASK));
    }
}
