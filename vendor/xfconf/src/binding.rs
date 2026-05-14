use std::num::NonZeroU64;

use glib::{prelude::*, translate::*};

use crate::Channel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingId(NonZeroU64);

impl BindingId {
    // rustdoc-stripper-ignore-next
    /// Returns the internal binding ID.
    pub fn as_raw(&self) -> u64 {
        self.0.get()
    }
}

impl FromGlib<u64> for BindingId {
    #[inline]
    unsafe fn from_glib(val: u64) -> Self {
        debug_assert_ne!(val, 0);
        Self(NonZeroU64::new_unchecked(val))
    }
}

pub trait ChannelBindingExt {
    /// Binds an Xfconf property to a [`glib::Object`][crate::glib::Object] property. If the property
    /// is changed via either the [`glib::Object`][crate::glib::Object] or Xfconf, the corresponding
    /// property will also be updated. The binding is initialized from the
    /// Xfconf property, i.e. the initial value of the [`glib::Object`][crate::glib::Object] property is
    /// overwritten.
    ///
    /// Note that `xfconf_property_type` is required since `xfconf_property`
    /// may or may not already exist in the Xfconf store. The type of
    /// `object_property` will be determined automatically. If the two
    /// types do not match, a conversion will be attempted.
    /// ## `channel`
    /// An [`Channel`][crate::Channel].
    /// ## `xfconf_property`
    /// A property on `channel`.
    /// ## `xfconf_property_type`
    /// The type of `xfconf_property`.
    /// ## `object`
    /// A [`glib::Object`][crate::glib::Object].
    /// ## `object_property`
    /// A valid property on `object`.
    ///
    /// # Returns
    ///
    /// an ID number that can be used to later remove the
    ///  binding.
    #[doc(alias = "xfconf_g_property_bind")]
    fn bind_property(
        &self,
        xfconf_property: &str,
        xfconf_property_type: glib::Type,
        object: &impl IsA<glib::Object>,
        object_property: &str,
    ) -> BindingId;

    /// Binds an Xfconf property to a [`glib::Object`][crate::glib::Object] property of type
    /// GDK_TYPE_RGBA (aka a `GdkRGBA` struct). If the property
    /// is changed via either the [`glib::Object`][crate::glib::Object] or Xfconf, the corresponding
    /// property will also be updated. The binding is initialized from the
    /// Xfconf property, i.e. the initial value of the [`glib::Object`][crate::glib::Object] property is
    /// overwritten.
    ///
    /// This is a special-case binding; the GdkRGBA struct is not
    /// ideal as-is for binding to a property, so it is stored in the
    /// Xfconf store as four 16-bit unsigned ints (red, green, blue, alpha).
    /// ## `channel`
    /// An [`Channel`][crate::Channel].
    /// ## `xfconf_property`
    /// A property on `channel`.
    /// ## `object`
    /// A [`glib::Object`][crate::glib::Object].
    /// ## `object_property`
    /// A valid property on `object`.
    ///
    /// # Returns
    ///
    /// an ID number that can be used to later remove the
    ///  binding.
    #[doc(alias = "xfconf_g_property_bind_gdkrgba")]
    fn bind_gdkrgba_property(
        &self,
        xfconf_property: &str,
        object: &impl IsA<glib::Object>,
        object_property: &str,
    ) -> BindingId;

    /// Removes an Xfconf/GObject property binding based on the binding
    /// ID number. See [`bind_property()`][crate::bind_property()].
    /// ## `id`
    /// A binding ID number.
    #[doc(alias = "xfconf_g_property_unbind")]
    fn unbind(&self, id: BindingId);

    /// Causes an Xfconf channel previously bound to a [`glib::Object`][crate::glib::Object] property
    /// (see [`bind_property()`][crate::bind_property()]) to no longer be bound.
    /// ## `channel`
    /// An [`Channel`][crate::Channel].
    /// ## `xfconf_property`
    /// A bound property on `channel`.
    /// ## `object`
    /// A [`glib::Object`][crate::glib::Object].
    /// ## `object_property`
    /// A bound property on `object`.
    #[doc(alias = "xfconf_g_property_unbind_by_property")]
    fn unbind_by_property(
        &self,
        xfconf_property: &str,
        object: &impl IsA<glib::Object>,
        object_property: &str,
    );

    /// Unbinds all Xfconf channel bindings (see [`bind_property()`][crate::bind_property()])
    /// to `object`. If `object` is an [`Channel`][crate::Channel], it will unbind all
    /// xfconf properties on that channel. If `object` is a regular [`glib::Object`][crate::glib::Object]
    /// with properties bound to a channel, all those bindings will be
    /// removed.
    /// ## `channel_or_object`
    /// A [`glib::Object`][crate::glib::Object] or [`Channel`][crate::Channel].
    #[doc(alias = "xfconf_g_property_unbind_all")]
    fn unbind_all(&self);
}

impl<O: IsA<Channel>> ChannelBindingExt for O {
    fn bind_property(
        &self,
        xfconf_property: &str,
        xfconf_property_type: glib::Type,
        object: &impl IsA<glib::Object>,
        object_property: &str,
    ) -> BindingId {
        let id = unsafe {
            ffi::xfconf_g_property_bind(
                self.as_ref().to_glib_none().0,
                xfconf_property.to_glib_none().0,
                xfconf_property_type.into_glib(),
                object.as_ref().to_glib_none().0,
                object_property.to_glib_none().0,
            )
        };
        BindingId(NonZeroU64::new(id).unwrap())
    }

    fn bind_gdkrgba_property(
        &self,
        xfconf_property: &str,
        object: &impl IsA<glib::Object>,
        object_property: &str,
    ) -> BindingId {
        let id = unsafe {
            ffi::xfconf_g_property_bind_gdkrgba(
                self.as_ref().to_glib_none().0,
                xfconf_property.to_glib_none().0,
                object.as_ref().to_glib_none().0,
                object_property.to_glib_none().0,
            )
        };
        BindingId(NonZeroU64::new(id).unwrap())
    }

    fn unbind(&self, id: BindingId) {
        unsafe {
            ffi::xfconf_g_property_unbind(id.as_raw());
        }
    }

    fn unbind_by_property(
        &self,
        xfconf_property: &str,
        object: &impl IsA<glib::Object>,
        object_property: &str,
    ) {
        unsafe {
            ffi::xfconf_g_property_unbind_by_property(
                self.as_ref().to_glib_none().0,
                xfconf_property.to_glib_none().0,
                object.as_ref().to_glib_none().0,
                object_property.to_glib_none().0,
            );
        }
    }

    fn unbind_all(&self) {
        unsafe {
            ffi::xfconf_g_property_unbind_all(self.as_object_ref().to_glib_none().0);
        }
    }
}

pub trait ObjectChannelBindingExt {
    // rustdoc-stripper-ignore-next
    /// Unbinds all xfconf properties bound on this object.
    fn unbind_xfconf_properties(&self);
}

impl<O: IsA<glib::Object>> ObjectChannelBindingExt for O {
    #[doc(alias = "xfconf_g_property_unbind_all")]
    fn unbind_xfconf_properties(&self) {
        unsafe {
            ffi::xfconf_g_property_unbind_all(self.as_ref().to_glib_none().0);
        }
    }
}
