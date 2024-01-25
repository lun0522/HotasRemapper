use std::ffi::c_char;
use std::ffi::CStr;

use anyhow::bail;
use anyhow::Result;
use core_foundation::string::kCFStringEncodingUTF8;
use core_foundation::string::CFString;
use core_foundation::string::CFStringGetCString;
use core_foundation::string::CFStringGetLength;
use core_foundation::string::CFStringRef;

/// Safety: see safety comments of `CStr::from_ptr()`.
pub(crate) unsafe fn new_string_from_ptr(ptr: *const c_char) -> Result<String> {
    match CStr::from_ptr(ptr).to_str() {
        Ok(string) => Ok(string.to_string()),
        Err(e) => bail!(e),
    }
}

/// Safety: see safety comments of `CStr::from_ptr()`.
pub(crate) unsafe fn new_cf_string_from_ptr(
    ptr: *const c_char,
) -> Result<CFString> {
    match CStr::from_ptr(ptr).to_str() {
        Ok(string) => Ok(CFString::from_static_string(string)),
        Err(e) => bail!(e),
    }
}

/// Safety: `string_ref` must be valid.
#[deny(unsafe_op_in_unsafe_fn)]
pub(crate) unsafe fn new_string_from_cf_string(
    string_ref: CFStringRef,
) -> Result<String> {
    // Safe because the caller guarantees `string_ref` is valid.
    let buffer_size = unsafe { CFStringGetLength(string_ref) } + 1;
    let mut buffer: Vec<u8> = vec![0; buffer_size as usize];
    // Safe because the caller guarantees `string_ref` is valid, and `buffer`
    // outlives this function call.
    if unsafe {
        CFStringGetCString(
            string_ref,
            buffer.as_mut_ptr() as *mut i8,
            buffer.len() as isize,
            kCFStringEncodingUTF8,
        )
    } == 0
    {
        bail!("CFStringGetCString() failed");
    }
    match CStr::from_bytes_with_nul(buffer.as_slice()) {
        Ok(string) => Ok(string.to_string_lossy().into_owned()),
        Err(e) => bail!("CStr::from_bytes_with_nul() failed: {}", e),
    }
}
