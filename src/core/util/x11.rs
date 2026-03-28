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

use std::{cell::RefCell, collections::HashMap};

use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, GetPropertyReply, PropMode, Window},
    wrapper::ConnectionExt,
};

use crate::core::util::ImageData;

pub struct X11<C: Connection + ConnectionExt> {
    x11_conn: C,
    atom_cache: RefCell<HashMap<String, Atom>>,
}

impl<C: Connection + ConnectionExt> X11<C> {
    pub fn new(x11_conn: C) -> Self {
        Self {
            x11_conn,
            atom_cache: RefCell::new(HashMap::default()),
        }
    }

    pub fn get_atom(&self, name: &str) -> Option<Atom> {
        if let Some(atom) = self.atom_cache.borrow().get(name) {
            Some(*atom)
        } else if let Some(atom) = self
            .x11_conn
            .intern_atom(false, name.as_bytes())
            .inspect_err(|err| tracing::warn!("Failed to send X11 InternAtom request for atom {name}: {err}"))
            .ok()
            .and_then(|cookie| {
                cookie
                    .reply()
                    .inspect_err(|err| tracing::warn!("Failed to receive X11 InternAtom reply for atom {name}: {err}"))
                    .ok()
            })
            .map(|reply| Atom::from(reply.atom))
        {
            self.atom_cache.borrow_mut().insert(name.to_owned(), atom);
            Some(atom)
        } else {
            None
        }
    }

    fn get_property<T: Into<Atom>>(&self, window_id: Window, name: &str, type_: T, length: u32) -> Option<GetPropertyReply> {
        let property = self.get_atom(name)?;
        let cookie = self
            .x11_conn
            .get_property(false, window_id, property, type_, 0, length)
            .inspect_err(|err| tracing::warn!("Failed to send request for {name} for window {window_id}: {err}"))
            .ok()?;
        cookie
            .reply()
            .inspect_err(|err| tracing::warn!("Failed to fetch reply for {name} for window {window_id}: {err}"))
            .ok()
    }

    pub fn get_user_time(&self, window_id: Window) -> Option<u32> {
        let reply = self.get_property(window_id, "_NET_WM_USER_TIME", AtomEnum::CARDINAL, 1)?;
        reply.value32().and_then(|mut values| values.next())
    }

    pub fn get_net_wm_state(&self, window_id: Window) -> Option<Vec<Atom>> {
        let reply = self.get_property(window_id, "_NET_WM_STATE", AtomEnum::ATOM, u32::MAX)?;
        reply.value32().map(|iter| iter.collect::<Vec<_>>())
    }

    pub fn update_net_wm_state(&self, window_id: Window, add: &[&str], remove: &[&str]) -> Option<Vec<Atom>> {
        let mut state_atoms = self.get_net_wm_state(window_id)?;

        let add = add.iter().map(|name| self.get_atom(name)).collect::<Option<Vec<_>>>()?;
        let remove = remove.iter().map(|name| self.get_atom(name)).collect::<Option<Vec<_>>>()?;
        state_atoms.retain(|atom| !remove.contains(atom));
        state_atoms.extend(add);

        let net_wm_state = self.get_atom("_NET_WM_STATE")?;
        if let Err(err) = self
            .x11_conn
            .change_property32(PropMode::REPLACE, window_id, net_wm_state, AtomEnum::ATOM, &state_atoms)
        {
            tracing::warn!("Failed to update _NET_WM_STATE for window {window_id}: {err}");
            None
        } else {
            Some(state_atoms)
        }
    }

    pub fn get_net_wm_icon(&self, window_id: Window) -> Option<ImageData> {
        let net_wm_icon = self.get_atom("_NET_WM_ICON")?;
        let reply = self
            .x11_conn
            .get_property(false, window_id, net_wm_icon, AtomEnum::CARDINAL, 0, u32::MAX)
            .inspect_err(|err| tracing::warn!("Failed to send request for _NET_WM_ICON for window {window_id}: {err}"))
            .ok()
            .and_then(|cookie| {
                cookie
                    .reply()
                    .inspect_err(|err| tracing::warn!("Failed to fetch reply for _NET_WM_ICON for window {window_id}: {err}"))
                    .ok()
            })?;
        let mut prop_data = reply.value32()?;

        let mut icons = Vec::new();
        while let (Some(width), Some(height)) = (prop_data.next(), prop_data.next()) {
            let n_pixels = (width * height) as usize;
            let bytes = prop_data
                .by_ref()
                .take(n_pixels)
                .flat_map(|argb| {
                    [
                        ((argb >> 16) & 0xff) as u8,
                        ((argb >> 8) & 0xff) as u8,
                        (argb & 0xff) as u8,
                        ((argb >> 24) & 0xff) as u8,
                    ]
                })
                .collect::<Vec<u8>>();

            if bytes.len() == n_pixels {
                icons.push(ImageData::RgbaPixels { bytes, width, height });
            } else {
                break;
            }
        }

        // XXX: This just picks the largest icon, which may not be what we really want
        icons.into_iter().max_by_key(|data| match data {
            ImageData::RgbaPixels { width, .. } => *width,
            _ => 0,
        })
    }
}
