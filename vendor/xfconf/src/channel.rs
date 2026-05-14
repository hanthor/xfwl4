use std::collections::HashMap;

use crate::{Channel, ToXfconfValue, TryFromXfconfValue};
use glib::{gobject_ffi::GValue, prelude::*, translate::*};

pub trait ChannelExtManual {
    #[doc(alias = "xfconf_channel_get_properties")]
    fn get_properties(&self, property_base: Option<&str>) -> HashMap<glib::GString, glib::Value>;

    #[doc(alias = "xfconf_channel_get_property")]
    fn get_property<V: TryFromXfconfValue>(&self, property: &str) -> Option<V>;

    #[doc(alias = "xfconf_channel_get_property")]
    fn get_property_value(&self, property: &str) -> Option<glib::Value>;

    #[doc(alias = "xfconf_channel_set_property")]
    fn set_property<V: ToXfconfValue>(&self, property: &str, value: V) -> bool;
}

impl<O: IsA<Channel>> ChannelExtManual for O {
    // gir isn't handling a GHashTable return properly
    fn get_properties(&self, property_base: Option<&str>) -> HashMap<glib::GString, glib::Value> {
        unsafe {
            let hashtable = ffi::xfconf_channel_get_properties(
                self.as_ref().to_glib_none().0,
                property_base.to_glib_none().0,
            );
            if hashtable.is_null() {
                HashMap::new()
            } else {
                ghashtable_into_hashmap_string_value(hashtable)
            }
        }
    }

    fn get_property<V: TryFromXfconfValue>(&self, property: &str) -> Option<V> {
        self.get_property_value(property).and_then(|v| {
            let ret = V::try_from_xfconf_value(&v);
            if let Err(err) = &ret {
                glib::g_warning!(
                    "xfconf",
                    "Value for property '{property}' could not be converted to type {}: {err}",
                    V::xfconf_display_name(),
                );
            }
            ret.ok()
        })
    }

    fn get_property_value(&self, property: &str) -> Option<glib::Value> {
        let mut value = glib::gobject_ffi::GValue {
            g_type: glib::Type::INVALID.into_glib(),
            data: [
                glib::gobject_ffi::GValue_data { v_int: 0 },
                glib::gobject_ffi::GValue_data { v_int: 0 },
            ],
        };

        let ret: bool = unsafe {
            from_glib(ffi::xfconf_channel_get_property(
                self.as_ref().to_glib_none().0,
                property.to_glib_none().0,
                &mut value as *mut _,
            ))
        };
        let ret = if !ret {
            None
        } else {
            let value: glib::Value = unsafe { from_glib_none(&mut value as *mut _) };
            if value.type_().is_valid() {
                Some(value)
            } else {
                None
            }
        };

        unsafe {
            glib::gobject_ffi::g_value_unset(&mut value as *mut _);
        }

        ret
    }

    fn set_property<V: ToXfconfValue>(&self, property: &str, value: V) -> bool {
        unsafe {
            from_glib(ffi::xfconf_channel_set_property(
                self.as_ref().to_glib_none().0,
                property.to_glib_none().0,
                value.to_xfconf_value().to_glib_none().0,
            ))
        }
    }
}

unsafe fn ghashtable_into_hashmap_string_value(
    ptr: *mut glib::ffi::GHashTable,
) -> HashMap<glib::GString, glib::Value> {
    unsafe extern "C" fn read_string_hash_table(
        key: glib::ffi::gpointer,
        value: glib::ffi::gpointer,
        hash_map: glib::ffi::gpointer,
    ) -> i32 {
        let key: glib::GString = from_glib_full(key as *const libc::c_char);
        let value: glib::Value = from_glib_full(value as *const GValue);
        let hash_map: &mut HashMap<glib::GString, glib::Value> =
            &mut *(hash_map as *mut HashMap<glib::GString, glib::Value>);
        hash_map.insert(key, value);
        1
    }
    let mut map = HashMap::with_capacity(glib::ffi::g_hash_table_size(ptr) as usize);
    glib::ffi::g_hash_table_foreach_steal(
        ptr,
        Some(read_string_hash_table),
        &mut map as *mut HashMap<glib::GString, glib::Value> as *mut _,
    );
    glib::ffi::g_hash_table_destroy(ptr);
    map
}
