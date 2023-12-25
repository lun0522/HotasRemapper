use std::ffi::c_char;
use std::ffi::CStr;

use anyhow::anyhow;
use anyhow::Result;
use core_foundation::string::CFString;

/// Safety: see safety comments of `CStr::from_ptr()`.
pub(crate) unsafe fn new_cf_string(ptr: *const c_char) -> Result<CFString> {
    match CStr::from_ptr(ptr).to_str() {
        Ok(string) => Ok(CFString::from_static_string(string)),
        Err(e) => Err(anyhow!("String conversion failed: {}", e)),
    }
}
