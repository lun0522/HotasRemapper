use std::convert::From;
use std::ffi::c_char;

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
use io_kit_sys::hid::base::IOHIDDeviceCallback;
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

use crate::utils::new_cf_string;

/// A trait to provide what we need for calling
/// `IOHIDManagerRegisterDeviceMatchingCallback()` and
/// `IOHIDManagerRegisterDeviceRemovalCallback()`.
pub(crate) trait HandleDeviceEvent {
    fn device_matched_callback() -> IOHIDDeviceCallback;
    fn device_removed_callback() -> IOHIDDeviceCallback;
}

/// A struct wrapping `IOHIDManagerRef` from IOKit.
pub(crate) struct HIDManager {
    manager_ref: IOHIDManagerRef,
}

impl HIDManager {
    pub fn new() -> Result<Self> {
        println!("Creating HID manager");
        let manager_ref = create_manager();
        // Safe because the manager will be alive until we call `CFRelease()`.
        unsafe {
            set_device_matching_criteria(&manager_ref);
            if let Err(e) = open_manager(&manager_ref) {
                close_and_release_manager(&manager_ref);
                return Err(e);
            }
            start_manager(&manager_ref);
        }
        Ok(Self { manager_ref })
    }

    /// Safety: the caller must ensure the pinned handler lives longer than
    /// `HIDManager`.
    pub unsafe fn set_device_callbacks<T: HandleDeviceEvent>(
        &self,
        pinned_handler_ptr: *mut T,
    ) {
        set_device_callbacks::<T>(&self.manager_ref, pinned_handler_ptr);
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
unsafe fn set_device_matching_criteria(manager_ref: &IOHIDManagerRef) {
    let new_kv_pair =
        |key: *const c_char, value: u32| -> (CFString, CFNumber) {
            (new_cf_string(key).unwrap(), CFNumber::from(value as i32))
        };
    let criteria = CFDictionary::from_CFType_pairs(&[
        new_kv_pair(kIOHIDDeviceUsagePageKey, kHIDPage_GenericDesktop),
        new_kv_pair(kIOHIDDeviceUsageKey, kHIDUsage_GD_Joystick),
    ]);
    IOHIDManagerSetDeviceMatchingMultiple(
        *manager_ref,
        CFArray::from_CFTypes(&[criteria.as_CFType()]).as_concrete_TypeRef(),
    );
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
unsafe fn set_device_callbacks<T: HandleDeviceEvent>(
    manager_ref: &IOHIDManagerRef,
    pinned_handler_ptr: *mut T,
) {
    IOHIDManagerRegisterDeviceMatchingCallback(
        *manager_ref,
        T::device_matched_callback(),
        pinned_handler_ptr as *mut _,
    );
    IOHIDManagerRegisterDeviceRemovalCallback(
        *manager_ref,
        T::device_removed_callback(),
        pinned_handler_ptr as *mut _,
    );
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn start_manager(manager_ref: &IOHIDManagerRef) {
    IOHIDManagerScheduleWithRunLoop(
        *manager_ref,
        CFRunLoopGetCurrent(),
        kCFRunLoopDefaultMode,
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
    CFRelease(*manager_ref as *mut _);
}
