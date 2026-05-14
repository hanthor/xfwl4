use std::{
    ops::{Deref, DerefMut},
    sync::LazyLock,
};

use glib::{
    translate::{FromGlib, FromGlibPtrNone, ToGlibPtr, ToGlibPtrMut},
    value::{FromValue, GenericValueTypeChecker, ValueType},
    StaticType, ToValue,
};

static G_TYPE_PTR_ARRAY: LazyLock<glib::Type> = LazyLock::new(|| {
    // SAFETY: g_ptr_array_get_type() will always return a valid GType
    unsafe { glib::Type::from_glib(glib::ffi::g_ptr_array_get_type()) }
});

pub struct Array<T>(Vec<T>);

impl<T> Array<T> {
    pub fn inner(&self) -> &[T] {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut [T] {
        &mut self.0
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> IntoIterator for Array<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> Deref for Array<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: ValueType + StaticType> StaticType for Array<T> {
    fn static_type() -> glib::Type {
        *G_TYPE_PTR_ARRAY
    }
}

impl<T> From<Vec<T>> for Array<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T> From<Array<T>> for Vec<T> {
    fn from(value: Array<T>) -> Self {
        value.0
    }
}

unsafe impl<'a, T: ValueType + StaticType> FromValue<'a> for Array<T> {
    type Checker = GenericValueTypeChecker<Array<T>>;

    unsafe fn from_value(value: &'a glib::Value) -> Self {
        if value.is_type(*G_TYPE_PTR_ARRAY) {
            let gvalue = value.to_glib_none();

            // SAFETY: We have verified above that value/v contains a GPtrArray, which is a Glib
            // boxed type.
            let array = unsafe { glib::gobject_ffi::g_value_get_boxed(gvalue.0) }
                as *mut glib::ffi::GPtrArray;

            if array.is_null() {
                Self(Vec::default())
            } else {
                // SAFETY: We know that 'array' is a GPtrArray, and is not NULL.
                let mut items = unsafe { *array }.pdata as *mut *mut glib::gobject_ffi::GValue;
                let n_items = unsafe { *array }.len;

                let mut vec = Vec::<T>::default();

                if !items.is_null() {
                    // Assert that items is a reasonable pointer, and that as we walk to the end of the
                    // array, we won't wrap around.
                    assert!(
                        n_items as isize + size_of::<*mut glib::gobject_ffi::GValue>() as isize
                            <= isize::MAX - items as isize
                    );

                    for _ in 0..n_items {
                        // SAFETY: 'items' is not NULL.
                        let item = unsafe { *items };

                        if !item.is_null() {
                            // SAFETY: Unfortunately there is no way to prove this safe.  We *hope*
                            // that the caller has given us a GPtrArray full of GValue pointers,
                            // but there's no way to verify that at runtime.
                            let item_v = unsafe { glib::Value::from_glib_none(item) };

                            match item_v
                                .transform::<T>()
                                .map_err(|err| {
                                    format!(
                                        "Failed to transform value in array from type {} to type {}: {err}",
                                        item_v.type_(),
                                        T::static_type()
                                    )
                                })
                                .and_then(|item_v| item_v.get::<T>().map_err(|err| format!("Failed to get value from GValue: {err}")))
                            {
                                Ok(v) => vec.push(v),
                                Err(err) => glib::g_warning!("xfconf", "{}", err),
                            }
                        }

                        // SAFETY: We've asserted above that we won't wrap around in the address
                        // space as long as we stay within the bounds of 0..len.
                        items = unsafe { items.add(1) };
                    }
                } else {
                    glib::g_warning!(
                        "xfconf",
                        "Failed to convert GPtrArray items to Vec items, as the array pointer was NULL"
                    );
                }

                Self(vec)
            }
        } else {
            Self(Vec::default())
        }
    }
}

impl<T: ToValue> ToValue for Array<T> {
    fn to_value(&self) -> glib::Value {
        // SAFETY: This should always be safe as self.0.len() is a positive integer.
        let ptr_array = unsafe { glib::ffi::g_ptr_array_sized_new(self.0.len() as u32) };

        for item in &self.0 {
            let value = item.to_value();

            // SAFETY: We know that 'value' is a valid GValue.
            unsafe {
                glib::ffi::g_ptr_array_add(ptr_array, value.to_glib_full() as *mut _);
            }
        }

        let mut value = glib::Value::from_type(*G_TYPE_PTR_ARRAY);

        // SAFETY: We know 'value' holds a GPtrArray (a boxed type).
        unsafe {
            glib::gobject_ffi::g_value_set_boxed(value.to_glib_none_mut().0, ptr_array as *mut _);
        }

        value
    }

    fn value_type(&self) -> glib::Type {
        *G_TYPE_PTR_ARRAY
    }
}
