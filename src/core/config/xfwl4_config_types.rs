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

use std::str::FromStr;

use anyhow::anyhow;
use gtk::gdk::ModifierType;

#[derive(Debug, Clone, PartialEq)]
pub struct TitlebarButtonLayout {
    pub start: Vec<TitlebarButton>,
    pub end: Vec<TitlebarButton>,
}

impl TitlebarButtonLayout {
    pub fn includes(&self, btn: TitlebarButton) -> bool {
        self.start.contains(&btn) || self.end.contains(&btn)
    }
}

impl FromStr for TitlebarButtonLayout {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let buttons = value.chars().map(TitlebarButton::try_from).collect::<Result<Vec<_>, _>>()?;
        if let Some(separator_pos) = buttons.iter().position(|button| *button == TitlebarButton::SideSeparator) {
            Ok(TitlebarButtonLayout {
                start: buttons.iter().take(separator_pos).cloned().collect(),
                end: buttons
                    .into_iter()
                    .skip(separator_pos + 1)
                    .filter(|button| *button != TitlebarButton::SideSeparator)
                    .collect(),
            })
        } else {
            Ok(TitlebarButtonLayout {
                start: buttons,
                end: Vec::new(),
            })
        }
    }
}

impl Default for TitlebarButtonLayout {
    fn default() -> Self {
        "O|SHMC".parse().unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TitlebarButton {
    Menu,
    Stick,
    Shade,
    Hide,
    Maximize,
    Close,
    SideSeparator,
}

impl TryFrom<char> for TitlebarButton {
    type Error = anyhow::Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'O' => Ok(Self::Menu),
            'T' => Ok(Self::Stick),
            'S' => Ok(Self::Shade),
            'H' => Ok(Self::Hide),
            'M' => Ok(Self::Maximize),
            'C' => Ok(Self::Close),
            '|' => Ok(Self::SideSeparator),
            invalid => Err(anyhow!("Invalid char '{invalid}' for titlebar button layout")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ActivateAction {
    #[default]
    None,
    Bring,
    Switch,
}

impl FromStr for ActivateAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" | "click_to_focus" => Ok(Self::None),
            "bring" => Ok(Self::Bring),
            "switch" => Ok(Self::Switch),
            invalid => Err(anyhow!("Invalid activate_action '{invalid}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DoubleClickAction {
    #[default]
    None,
    Maximize,
    Shade,
    Fill,
    Above,
    Hide,
}

impl FromStr for DoubleClickAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "maximize" => Ok(Self::Maximize),
            "shade" => Ok(Self::Shade),
            "fill" => Ok(Self::Fill),
            "above" => Ok(Self::Above),
            "hide" => Ok(Self::Hide),
            invalid => Err(anyhow!("Invalid double_click_action '{invalid}'")),
        }
    }
}

// FIXME: I think xfwm4 actually allows any arbitrary modifier key, not just those listed below
// (which is what the settings dialog allows you to select).  Maybe will need to handle the case
// where a user modifies xfconf to set a modifier key outside of this list?
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum EasyClickKey {
    None,
    #[default]
    Alt,
    Control,
    Hyper,
    Meta,
    Shift,
    Super,
    Mod1,
    Mod2,
    Mod3,
    Mod4,
    Mod5,
}

impl EasyClickKey {
    pub fn modifier_mask(&self) -> ModifierType {
        match self {
            Self::None => ModifierType::empty(),
            Self::Alt => ModifierType::MOD1_MASK,
            Self::Control => ModifierType::CONTROL_MASK,
            Self::Hyper => ModifierType::HYPER_MASK,
            Self::Meta => ModifierType::META_MASK,
            Self::Shift => ModifierType::SHIFT_MASK,
            Self::Super => ModifierType::SUPER_MASK | ModifierType::MOD4_MASK,
            Self::Mod1 => ModifierType::MOD1_MASK,
            Self::Mod2 => ModifierType::MOD2_MASK,
            Self::Mod3 => ModifierType::MOD3_MASK,
            Self::Mod4 => ModifierType::MOD4_MASK,
            Self::Mod5 => ModifierType::MOD5_MASK,
        }
    }
}

impl FromStr for EasyClickKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" => Ok(Self::None),
            "Alt" | "true" => Ok(Self::Alt),
            "Control" => Ok(Self::Control),
            "Hyper" => Ok(Self::Hyper),
            "Meta" => Ok(Self::Meta),
            "Shift" => Ok(Self::Shift),
            "Super" => Ok(Self::Super),
            "Mod1" => Ok(Self::Mod1),
            "Mod2" => Ok(Self::Mod2),
            "Mod3" => Ok(Self::Mod3),
            "Mod4" => Ok(Self::Mod4),
            "Mod5" => Ok(Self::Mod5),
            invalid => Err(anyhow!("Invalid easy click key '{invalid}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PlacementMode {
    Mouse,
    #[default]
    Center,
}

impl FromStr for PlacementMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mouse" => Ok(Self::Mouse),
            "center" => Ok(Self::Center),
            invalid => Err(anyhow!("Invalid placement mode '{invalid}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TitleAlignment {
    Left,
    Right,
    #[default]
    Center,
}

impl FromStr for TitleAlignment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "center" => Ok(Self::Center),
            invalid => Err(anyhow!("Invalid title alignment '{invalid}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(i32)]
pub enum TitleShadow {
    #[default]
    None = 0,
    Under = 1,
    Frame = 2,
}

impl FromStr for TitleShadow {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" | "false" => Ok(Self::None),
            "under" | "true" => Ok(Self::Under),
            "frame" => Ok(Self::Frame),
            invalid => Err(anyhow!("Invalid title shadow position '{invalid}'")),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{TitlebarButton, TitlebarButtonLayout};

    #[test]
    pub fn test_titlebar_button_layouts() {
        let cases = [
            (
                "O|SHMC",
                vec![TitlebarButton::Menu],
                vec![
                    TitlebarButton::Shade,
                    TitlebarButton::Hide,
                    TitlebarButton::Maximize,
                    TitlebarButton::Close,
                ],
            ),
            ("|TC", vec![], vec![TitlebarButton::Stick, TitlebarButton::Close]),
            ("OC", vec![TitlebarButton::Menu, TitlebarButton::Close], vec![]),
            (
                "OS|TC",
                vec![TitlebarButton::Menu, TitlebarButton::Shade],
                vec![TitlebarButton::Stick, TitlebarButton::Close],
            ),
        ];

        for (value, start, end) in cases {
            assert_eq!(value.parse::<TitlebarButtonLayout>().unwrap(), TitlebarButtonLayout { start, end });
        }
    }
}
