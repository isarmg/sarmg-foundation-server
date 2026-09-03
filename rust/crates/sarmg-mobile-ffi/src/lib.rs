#![allow(unsafe_code)]
//! All unsafe pointer access and panic catching for product FFI exports lives here.

use std::{
    any::Any,
    ffi::CString,
    os::raw::c_char,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Mutex,
};
pub const ABI_REVISION: u32 = 1;
pub const SARMG_FFI_OK: i32 = 0;
pub const SARMG_FFI_INVALID_ARGUMENT: i32 = 1;
pub const SARMG_FFI_INVALID_HANDLE: i32 = 2;
pub const SARMG_FFI_INTERNAL_PANIC: i32 = 255;
#[repr(C)]
pub struct SarmgFfiResultV1 {
    pub abi_revision: u32,
    pub status: i32,
    pub message: *mut c_char,
}
impl Default for SarmgFfiResultV1 {
    fn default() -> Self {
        Self {
            abi_revision: ABI_REVISION,
            status: SARMG_FFI_OK,
            message: std::ptr::null_mut(),
        }
    }
}
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn guard<F>(output: *mut SarmgFfiResultV1, operation: F) -> i32
where
    F: FnOnce() -> Result<(), FfiError>,
{
    if output.is_null() {
        return SARMG_FFI_INVALID_ARGUMENT;
    }
    let result = catch_unwind(AssertUnwindSafe(operation));
    let (status, message) = match result {
        Ok(Ok(())) => (SARMG_FFI_OK, None),
        Ok(Err(error)) => (error.status, Some(error.public_message)),
        Err(_) => (SARMG_FFI_INTERNAL_PANIC, Some("internal panic")),
    };
    let message = message
        .and_then(|m| CString::new(m).ok())
        .map_or(std::ptr::null_mut(), CString::into_raw);
    unsafe {
        output.write(SarmgFfiResultV1 {
            abi_revision: ABI_REVISION,
            status,
            message,
        });
    }
    status
}
#[unsafe(no_mangle)]
/// Releases a message returned by this crate.
///
/// # Safety
/// `value` must be null or a pointer returned in [`SarmgFfiResultV1::message`]
/// that has not previously been freed.
pub unsafe extern "C" fn sarmg_ffi_string_free_v1(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}
#[derive(Debug)]
pub struct FfiError {
    pub status: i32,
    pub public_message: &'static str,
}
impl FfiError {
    pub const fn invalid_argument() -> Self {
        Self {
            status: SARMG_FFI_INVALID_ARGUMENT,
            public_message: "invalid argument",
        }
    }
    pub const fn invalid_handle() -> Self {
        Self {
            status: SARMG_FFI_INVALID_HANDLE,
            public_message: "invalid handle",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Handle {
    pub slot: u32,
    pub generation: u32,
}
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}
pub struct HandleRegistry<T> {
    slots: Mutex<Vec<Slot<T>>>,
}
impl<T> Default for HandleRegistry<T> {
    fn default() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
        }
    }
}
impl<T> HandleRegistry<T> {
    pub fn insert(&self, value: T) -> Handle {
        let mut slots = self
            .slots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((index, slot)) = slots
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.value.is_none())
        {
            slot.generation = slot.generation.wrapping_add(1).max(1);
            slot.value = Some(value);
            return Handle {
                slot: index as u32,
                generation: slot.generation,
            };
        }
        slots.push(Slot {
            generation: 1,
            value: Some(value),
        });
        Handle {
            slot: (slots.len() - 1) as u32,
            generation: 1,
        }
    }
    pub fn with<R>(&self, handle: Handle, operation: impl FnOnce(&T) -> R) -> Result<R, FfiError> {
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let slot = slots
            .get(handle.slot as usize)
            .filter(|s| s.generation == handle.generation)
            .and_then(|s| s.value.as_ref())
            .ok_or_else(FfiError::invalid_handle)?;
        Ok(operation(slot))
    }
    pub fn remove(&self, handle: Handle) -> Result<T, FfiError> {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let slot = slots
            .get_mut(handle.slot as usize)
            .filter(|s| s.generation == handle.generation)
            .ok_or_else(FfiError::invalid_handle)?;
        slot.value.take().ok_or_else(FfiError::invalid_handle)
    }
}
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn checked_input<'a>(
    pointer: *const u8,
    length: usize,
    max_length: usize,
) -> Result<&'a [u8], FfiError> {
    if length > max_length || pointer.is_null() && length != 0 {
        return Err(FfiError::invalid_argument());
    }
    if length == 0 {
        return Ok(&[]);
    }
    Ok(unsafe { std::slice::from_raw_parts(pointer, length) })
}
pub fn panic_message(_payload: &Box<dyn Any + Send>) -> &'static str {
    "internal panic"
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_handles_never_alias_reused_slots() {
        let registry = HandleRegistry::default();
        let old = registry.insert("old");
        assert_eq!(registry.remove(old).unwrap(), "old");
        let new = registry.insert("new");
        assert_eq!(old.slot, new.slot);
        assert_ne!(old.generation, new.generation);
        assert!(registry.with(old, |v| *v).is_err());
    }
    #[test]
    fn panics_do_not_cross_boundary() {
        let mut output = SarmgFfiResultV1::default();
        let status = guard(&mut output, || -> Result<(), FfiError> { panic!("secret") });
        assert_eq!(status, SARMG_FFI_INTERNAL_PANIC);
        unsafe {
            sarmg_ffi_string_free_v1(output.message);
        }
    }
}
