use std::ffi::c_char;
use std::ffi::CStr;

use anyhow::bail;
use anyhow::Result;
use core_foundation::string::kCFStringEncodingUTF8;
use core_foundation::string::CFString;
use core_foundation::string::CFStringGetCStringPtr;
use core_foundation::string::CFStringRef;

/// Safety: see safety comments of `CStr::from_ptr()`.
pub(crate) unsafe fn new_cf_string(ptr: *const c_char) -> Result<CFString> {
    match CStr::from_ptr(ptr).to_str() {
        Ok(string) => Ok(CFString::from_static_string(string)),
        Err(e) => bail!("Not valid UTF-8: {}", e),
    }
}

/// Safety: see safety comments of `CStr::from_ptr()`. Assumes UTF-8 encoding.
pub(crate) unsafe fn new_string(string_ref: CFStringRef) -> Result<String> {
    let string_ptr = CFStringGetCStringPtr(string_ref, kCFStringEncodingUTF8);
    match CStr::from_ptr(string_ptr).to_str() {
        Ok(string) => Ok(string.to_owned()),
        Err(e) => bail!("Not valid UTF-8: {}", e),
    }
}
