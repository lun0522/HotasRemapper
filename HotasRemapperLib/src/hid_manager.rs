use std::cell::RefCell;
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
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueCallback;
use io_kit_sys::hid::base::IOHIDValueRef;
use io_kit_sys::hid::keys::kIOHIDDeviceUsageKey;
use io_kit_sys::hid::keys::kIOHIDDeviceUsagePageKey;
use io_kit_sys::hid::keys::kIOHIDOptionsTypeNone;
use io_kit_sys::hid::keys::kIOHIDTransportBluetoothValue;
use io_kit_sys::hid::keys::kIOHIDTransportKey;
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

use crate::hid_device::HandleInputValue;
use crate::utils::new_cf_string;

// Apple actually uses "Bluetooth Low Energy" instead of "BluetoothLowEnergy"
// defined in <IOKit/hid/IOHIDKeys.h>.
#[allow(non_upper_case_globals)]
const kIOHIDTransportBluetoothLowEnergyValue: *const c_char =
    b"Bluetooth Low Energy\x00" as *const [u8; 21usize] as *const _;

pub(crate) trait HandleDeviceEvent {
    fn handle_device_matched(
        &mut self,
        device: IOHIDDeviceRef,
        input_value_handler: &dyn HandleInputValue,
    );
    fn handle_device_removed(&mut self, device: IOHIDDeviceRef);
    fn handle_input_value(&mut self, value: IOHIDValueRef);
}

/// A struct wrapping `IOHIDManagerRef` from IOKit.
pub(crate) struct HIDManager<T: HandleDeviceEvent> {
    manager_ref: IOHIDManagerRef,
    device_event_handler: RefCell<T>,
    // We want to make sure the `HIDManager` doesn't get moved, so the user can
    // rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl<T: HandleDeviceEvent> HIDManager<T> {
    pub fn new(device_event_handler: T) -> Result<Pin<Box<Self>>> {
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
            device_event_handler: RefCell::new(device_event_handler),
            _pinned_marker: PhantomPinned,
        });
        // Safe because we are passing in a pointer to a pinned `HIDManager`.
        unsafe {
            set_device_callbacks::<T>(
                &*manager.as_mut() as *const HIDManager<T> as *mut _,
            );
        }
        Ok(manager)
    }

    #[inline]
    fn handle_device_matched(&self, device: IOHIDDeviceRef) {
        self.device_event_handler
            .borrow_mut()
            .handle_device_matched(device, self);
    }

    #[inline]
    fn handle_device_removed(&self, device: IOHIDDeviceRef) {
        self.device_event_handler
            .borrow_mut()
            .handle_device_removed(device);
    }

    #[inline]
    fn handle_input_value(&self, value: IOHIDValueRef) {
        self.device_event_handler
            .borrow_mut()
            .handle_input_value(value);
    }
}

impl<T: HandleDeviceEvent> Drop for HIDManager<T> {
    fn drop(&mut self) {
        println!("Dropping HID manager");
        // Safe because we haven't called `CFRelease()` until this point.
        unsafe {
            stop_manager(&self.manager_ref);
            close_and_release_manager(&self.manager_ref);
        }
    }
}

impl<T: HandleDeviceEvent> HandleInputValue for HIDManager<T> {
    fn get_pinned_pointer(&self) -> *mut c_void {
        &*self as *const Self as *mut _
    }

    fn get_callback(&self) -> IOHIDValueCallback {
        handle_input_value::<T>
    }
}

fn create_manager() -> IOHIDManagerRef {
    // Trivially safe.
    unsafe { IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone) }
}

/// The caller must ensure `manager_ref` is still alive.
unsafe fn set_device_matching_criteria(manager_ref: &IOHIDManagerRef) {
    let make_string =
        |val: *const c_char| new_cf_string(val).unwrap().as_CFType();
    let make_number = |val: u32| CFNumber::from(val as i32).as_CFType();
    let usage_page_criteria = (
        make_string(kIOHIDDeviceUsagePageKey),
        make_number(kHIDPage_GenericDesktop),
    );
    let hotas_devices = CFDictionary::from_CFType_pairs(&[
        usage_page_criteria.clone(),
        (
            make_string(kIOHIDDeviceUsageKey),
            make_number(kHIDUsage_GD_Joystick),
        ),
    ]);
    let bluetooth_devices = CFDictionary::from_CFType_pairs(&[
        usage_page_criteria.clone(),
        (
            make_string(kIOHIDTransportKey),
            make_string(kIOHIDTransportBluetoothValue),
        ),
    ]);
    let bluetooth_low_energy_devices = CFDictionary::from_CFType_pairs(&[
        usage_page_criteria,
        (
            make_string(kIOHIDTransportKey),
            make_string(kIOHIDTransportBluetoothLowEnergyValue),
        ),
    ]);
    IOHIDManagerSetDeviceMatchingMultiple(
        *manager_ref,
        CFArray::from_CFTypes(&[
            hotas_devices.as_CFType(),
            bluetooth_devices.as_CFType(),
            bluetooth_low_energy_devices.as_CFType(),
        ])
        .as_concrete_TypeRef(),
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
unsafe fn set_device_callbacks<T: HandleDeviceEvent>(
    hid_manager: *mut HIDManager<T>,
) {
    IOHIDManagerRegisterDeviceMatchingCallback(
        (*hid_manager).manager_ref,
        handle_device_matched::<T>,
        hid_manager as *mut _,
    );
    IOHIDManagerRegisterDeviceRemovalCallback(
        (*hid_manager).manager_ref,
        handle_device_removed::<T>,
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

extern "C" fn handle_device_matched<T: HandleDeviceEvent>(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `HIDManager`.
    if let Some(manager) = unsafe { (context as *const HIDManager<T>).as_ref() }
    {
        manager.handle_device_matched(device);
    }
}

extern "C" fn handle_device_removed<T: HandleDeviceEvent>(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `HIDManager`.
    if let Some(manager) = unsafe { (context as *const HIDManager<T>).as_ref() }
    {
        manager.handle_device_removed(device);
    }
}

extern "C" fn handle_input_value<T: HandleDeviceEvent>(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    value: IOHIDValueRef,
) {
    // Safe because we stored a pointer to a pinned `HIDManager`.
    if let Some(manager) = unsafe { (context as *const HIDManager<T>).as_ref() }
    {
        manager.handle_input_value(value);
    }
}
