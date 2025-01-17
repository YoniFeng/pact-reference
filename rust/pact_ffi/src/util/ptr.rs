//! Utility functions for working with raw pointers.

// All of this module is `pub(crate)` and should not appear in the C header file
// or documentation.

use std::mem;
use std::ptr;

/// Get a raw pointer to a value on the heap.
///
/// Allocates the value on the heap by boxing it, and then gets a mutable raw pointer
/// to the memory location, to pass to the C-side of the FFI boundary. It is then the
/// responsibility of the C code to call the relevant FFI destructor function, which will
/// re-construct a `Box` to the pointer, and then drop the `Box` to deallocate the memory.
#[inline]
pub(crate) fn raw_to<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}

/// Drop the value pointed to by a raw pointer.
#[inline]
pub(crate) fn drop_raw<T>(raw: *mut T) {
    mem::drop(unsafe { Box::from_raw(raw) })
}

/// Get a constant null pointer to the given type.
#[inline]
#[allow(dead_code)]
#[deprecated(note = "Use std::ptr::null() instead", since = "0.4.0")]
pub(crate) fn null_to<T>() -> *const T {
    ptr::null() as *const T
}

/// Get a mutable null pointer to the given type.
#[inline]
#[deprecated(note = "Use std::ptr::null_mut() instead", since = "0.4.0")]
pub(crate) fn null_mut_to<T>() -> *mut T {
    ptr::null_mut() as *mut T
}

/// Get an immutable reference from a raw pointer
#[macro_export]
macro_rules! as_ref {
    ( $name:expr ) => {{
        unsafe { $name.as_ref() }.ok_or(::anyhow::anyhow!(concat!(
            stringify!($name),
            " is null"
        )))?
    }};
}

/// Get a mutable reference from a raw pointer
#[macro_export]
macro_rules! as_mut {
    ( $name:expr ) => {{
        unsafe { $name.as_mut() }.ok_or(::anyhow::anyhow!(concat!(
            stringify!($name),
            " is null"
        )))?
    }};
}
