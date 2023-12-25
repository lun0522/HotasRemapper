use std::convert::From;
use std::ffi::c_char;
use std::ffi::c_void;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::null_mut;

use anyhow::bail;
use anyhow::Result;
use core_foundation::array::CFArray;
use core_foundation::base::kCFAllocatorDefault;
use core_foundation::base::CFRelease;
use core_foundation::base::TCFType;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::runloop::kCFRunLoopDefaultMode;
use core_foundation::runloop::CFRunLoopGetCurrent;
use core_foundation::string::CFString;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::keys::kIOHIDDeviceUsageKey;
use io_kit_sys::hid::keys::kIOHIDDeviceUsagePageKey;
use io_kit_sys::hid::keys::kIOHIDOptionsTypeNone;
use io_kit_sys::hid::manager::IOHIDManagerClose;
use io_kit_sys::hid::manager::IOHIDManagerCreate;
use io_kit_sys::hid::manager::IOHIDManagerOpen;
use io_kit_sys::hid::manager::IOHIDManagerRef;
use io_kit_sys::hid::manager::IOHIDManagerRegisterDeviceMatchingCallback;
use io_kit_sys::hid::manager::IOHIDManagerRegisterDeviceRemovalCallback;
use io_kit_sys::hid::manager::IOHIDManagerScheduleWithRunLoop;
use io_kit_sys::hid::manager::IOHIDManagerSetDeviceMatchingMultiple;
use io_kit_sys::hid::manager::IOHIDManagerUnscheduleFromRunLoop;
use io_kit_sys::hid::usage_tables::kHIDPage_GenericDesktop;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Joystick;
use io_kit_sys::ret::kIOReturnSuccess;
use io_kit_sys::ret::IOReturn;

use crate::bindings::new_cf_string;

pub(crate) struct HIDManager {
    manager_ref: IOHIDManagerRef,
    // We want to make sure the `HIDManager` doesn't get moved, so the user can
    // rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl HIDManager {
    pub fn new() -> Result<Pin<Box<Self>>> {
        println!("Creating HID manager");
        let manager_ref = create_manager();
        // Safe because the manager will be alive until we call `CFRelease()`.
        unsafe {
            set_device_matching_criteria(&manager_ref)?;
            if let Err(e) = open_manager(&manager_ref) {
                close_and_release_manager(&manager_ref);
                return Err(e);
            }
            start_manager(&manager_ref);
        }
        Ok(Box::pin(Self {
            manager_ref,
            _pinned_marker: PhantomPinned,
        }))
    }
}

impl Drop for HIDManager {
    fn drop(&mut self) {
        println!("Dropping HID manager");
        // Safe because we haven't called `CFRelease()` until this point.
        unsafe {
            stop_manager(&self.manager_ref);
            close_and_release_manager(&self.manager_ref);
        }
    }
}

fn create_manager() -> IOHIDManagerRef {
    // Trivially safe.
    unsafe { IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone) }
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn set_device_matching_criteria(
    manager_ref: &IOHIDManagerRef,
) -> Result<()> {
    let new_kv_pair =
        |key: *const c_char, value: u32| -> Result<(CFString, CFNumber)> {
            Ok((new_cf_string(key)?, CFNumber::from(value as i32)))
        };
    let criteria = CFDictionary::from_CFType_pairs(&[
        new_kv_pair(kIOHIDDeviceUsagePageKey, kHIDPage_GenericDesktop)?,
        new_kv_pair(kIOHIDDeviceUsageKey, kHIDUsage_GD_Joystick)?,
    ]);
    IOHIDManagerSetDeviceMatchingMultiple(
        *manager_ref,
        CFArray::from_CFTypes(&[criteria]).as_concrete_TypeRef(),
    );
    Ok(())
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn open_manager(manager_ref: &IOHIDManagerRef) -> Result<()> {
    let ret = IOHIDManagerOpen(*manager_ref, kIOHIDOptionsTypeNone);
    if ret != kIOReturnSuccess {
        bail!("Failed to open HID manager (error code: {})", ret);
    }
    Ok(())
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn start_manager(manager_ref: &IOHIDManagerRef) {
    IOHIDManagerScheduleWithRunLoop(
        *manager_ref,
        CFRunLoopGetCurrent(),
        kCFRunLoopDefaultMode,
    );
    IOHIDManagerRegisterDeviceMatchingCallback(
        *manager_ref,
        handle_device_matched,
        null_mut(),
    );
    IOHIDManagerRegisterDeviceRemovalCallback(
        *manager_ref,
        handle_device_removed,
        null_mut(),
    );
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn stop_manager(manager_ref: &IOHIDManagerRef) {
    IOHIDManagerUnscheduleFromRunLoop(
        *manager_ref,
        CFRunLoopGetCurrent(),
        kCFRunLoopDefaultMode,
    );
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn close_and_release_manager(manager_ref: &IOHIDManagerRef) {
    IOHIDManagerClose(*manager_ref, kIOHIDOptionsTypeNone);
    CFRelease(*manager_ref as *mut c_void);
}

extern "C" fn handle_device_matched(
    _context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    _device: IOHIDDeviceRef,
) {
    println!("Device matched");
}

extern "C" fn handle_device_removed(
    _context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    _device: IOHIDDeviceRef,
) {
    println!("Device removed");
}
