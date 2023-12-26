use std::convert::From;
use std::ffi::c_char;
use std::ffi::c_void;
use std::marker::PhantomPinned;
use std::pin::Pin;

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

use crate::device_manager::DeviceManager;
use crate::utils::new_cf_string;

/// A struct wrapping `IOHIDManagerRef` from IOKit.
pub(crate) struct HIDManager {
    manager_ref: IOHIDManagerRef,
    device_manager: DeviceManager,
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
            set_device_matching_criteria(&manager_ref);
            if let Err(e) = open_manager(&manager_ref) {
                close_and_release_manager(&manager_ref);
                return Err(e);
            }
            start_manager(&manager_ref);
        }
        let mut manager = Box::pin(Self {
            manager_ref,
            device_manager: DeviceManager::new(),
            _pinned_marker: PhantomPinned,
        });
        // Safe because we are passing in a pointer to a pinned `HIDManager`.
        unsafe {
            set_device_callbacks(
                &*manager.as_mut() as *const HIDManager as *mut _
            );
        }
        Ok(manager)
    }

    #[inline]
    fn handle_device_matched(&mut self, device: IOHIDDeviceRef) {
        self.device_manager.handle_device_matched(device);
    }

    #[inline]
    fn handle_device_removed(&mut self, device: IOHIDDeviceRef) {
        self.device_manager.handle_device_removed(device);
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
        CFArray::from_CFTypes(&[criteria]).as_concrete_TypeRef(),
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

/// The caller must ensure `hid_manager` is a valid pointer, and the
/// `HIDManager` itself is pinned in memory.
unsafe fn set_device_callbacks(hid_manager: *mut HIDManager) {
    IOHIDManagerRegisterDeviceMatchingCallback(
        (*hid_manager).manager_ref,
        handle_device_matched,
        hid_manager as *mut _,
    );
    IOHIDManagerRegisterDeviceRemovalCallback(
        (*hid_manager).manager_ref,
        handle_device_removed,
        hid_manager as *mut _,
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

extern "C" fn handle_device_matched(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `HIDManager`.
    if let Some(manager) = unsafe { (context as *mut HIDManager).as_mut() } {
        manager.handle_device_matched(device);
    }
}

extern "C" fn handle_device_removed(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `HIDManager`.
    if let Some(manager) = unsafe { (context as *mut HIDManager).as_mut() } {
        manager.handle_device_removed(device);
    }
}
