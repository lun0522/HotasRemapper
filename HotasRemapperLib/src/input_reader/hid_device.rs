use std::collections::HashMap;
use std::ffi::c_char;
use std::ffi::c_void;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::ptr::null_mut;

use core_foundation::array::CFArrayGetCount;
use core_foundation::array::CFArrayGetValueAtIndex;
use core_foundation::base::TCFType;
use core_foundation::string::CFStringRef;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDElementRef;
use io_kit_sys::hid::base::IOHIDValueCallback;
use io_kit_sys::hid::base::IOHIDValueRef;
use io_kit_sys::hid::device::IOHIDDeviceCopyMatchingElements;
use io_kit_sys::hid::device::IOHIDDeviceGetProperty;
use io_kit_sys::hid::device::IOHIDDeviceRegisterInputValueCallback;
use io_kit_sys::hid::element::IOHIDElementGetCookie;
use io_kit_sys::hid::element::IOHIDElementGetDevice;
use io_kit_sys::hid::keys::kIOHIDOptionsTypeNone;
use io_kit_sys::hid::keys::kIOHIDProductIDKey;
use io_kit_sys::hid::keys::kIOHIDProductKey;
use io_kit_sys::hid::keys::kIOHIDTransportKey;
use io_kit_sys::hid::keys::kIOHIDVendorIDKey;
use io_kit_sys::hid::keys::IOHIDElementCookie;
use io_kit_sys::hid::value::IOHIDValueGetElement;
use io_kit_sys::hid::value::IOHIDValueGetIntegerValue;

use super::hid_device_input::DeviceInput;
use super::hid_device_input::InputType;
use crate::utils::new_cf_string_from_ptr;
use crate::utils::new_string_from_cf_string;

/// A trait to provide what we need for calling
/// `IOHIDDeviceRegisterInputValueCallback()`.
pub(crate) trait HandleInputEvent {
    fn input_received_callback() -> IOHIDValueCallback;
}

pub(crate) struct DeviceProperty {
    pub device_name: String,
    pub vendor_id: u32,
    pub product_id: u32,
    pub transport: String,
}

impl DeviceProperty {
    pub fn from_device(device_ref: IOHIDDeviceRef) -> Self {
        // Safe because `device_ref` is alive, and `key` will be static strings.
        let get_property = |key: *const c_char| unsafe {
            IOHIDDeviceGetProperty(
                device_ref,
                new_cf_string_from_ptr(key).unwrap().as_concrete_TypeRef(),
            )
        };
        // Safe because the system guarantees `IOHIDDeviceGetProperty()` returns
        // a pointer that is either NULL or points to a valid string.
        let get_string_property = |key: *const c_char, default: &str| unsafe {
            get_property(key)
                .as_ref()
                .and_then(|name| {
                    let name_cf_string = name as *const c_void as CFStringRef;
                    new_string_from_cf_string(name_cf_string).ok()
                })
                .unwrap_or(default.to_string())
        };
        Self {
            device_name: get_string_property(
                kIOHIDProductKey,
                "Unknown device",
            ),
            vendor_id: get_property(kIOHIDVendorIDKey) as u32,
            product_id: get_property(kIOHIDProductIDKey) as u32,
            transport: get_string_property(
                kIOHIDTransportKey,
                "Unknown transport",
            ),
        }
    }
}

impl Display for DeviceProperty {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!(
            "{{device name: {:?}, vendor id: {:#x}, product id: {:#x}, \
            transport: {:?}}}",
            self.device_name, self.vendor_id, self.product_id, self.transport,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DeviceType {
    Joystick,
    Throttle,
}

impl TryFrom<&DeviceProperty> for DeviceType {
    type Error = &'static str;

    fn try_from(property: &DeviceProperty) -> Result<Self, Self::Error> {
        match property.device_name.as_str() {
            "Joystick - HOTAS Warthog" => Ok(Self::Joystick),
            "Throttle - HOTAS Warthog" => Ok(Self::Throttle),
            _ => Err("Unknown type"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RawInputEvent {
    pub device_ref: IOHIDDeviceRef,
    pub input_id: IOHIDElementCookie,
    pub value: i32,
}

pub(crate) struct InputEvent {
    pub device_type: DeviceType,
    pub device_input: DeviceInput,
    pub value: i32,
}

/// A struct wrapping `IOHIDDeviceRef` from IOKit.
pub(crate) struct HIDDevice {
    device_type: DeviceType,
    input_map: HashMap<IOHIDElementCookie, DeviceInput>,
}

impl HIDDevice {
    /// Safety: the caller must ensure the device is alive, and the pinned
    /// handler outlives the device.
    pub unsafe fn open_device<T: HandleInputEvent>(
        device: IOHIDDeviceRef,
        device_type: DeviceType,
        pinned_handler_ptr: *mut T,
    ) -> Self {
        IOHIDDeviceRegisterInputValueCallback(
            device,
            T::input_received_callback(),
            pinned_handler_ptr as *mut _,
        );
        Self {
            device_type,
            input_map: build_input_map(device, device_type),
        }
    }

    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }

    pub fn interpret_raw_input_event(
        &self,
        input_event: RawInputEvent,
    ) -> Option<InputEvent> {
        match self.input_map.get(&input_event.input_id) {
            Some(device_input) => {
                if !matches!(device_input.input_type, InputType::Other) {
                    return Some(InputEvent {
                        device_type: self.device_type,
                        device_input: *device_input,
                        value: input_event.value,
                    });
                }
            }
            None => println!(
                "Unknown input event from {:?}: {:?}",
                self.device_type, input_event,
            ),
        }
        None
    }

    pub fn read_raw_input_event(value: IOHIDValueRef) -> Option<RawInputEvent> {
        // Safe because the system guarantees these references are valid.
        unsafe {
            if value.is_null() {
                return None;
            }
            let element = IOHIDValueGetElement(value);
            if element.is_null() {
                return None;
            }
            Some(RawInputEvent {
                device_ref: IOHIDElementGetDevice(element),
                input_id: IOHIDElementGetCookie(element),
                value: IOHIDValueGetIntegerValue(value) as i32,
            })
        }
    }
}

/// Safety: the caller must ensure the device is alive.
unsafe fn build_input_map(
    device: IOHIDDeviceRef,
    device_type: DeviceType,
) -> HashMap<IOHIDElementCookie, DeviceInput> {
    let mut input_map = HashMap::<IOHIDElementCookie, DeviceInput>::new();
    let mut index_tracker = HashMap::<InputType, i32>::new();
    let elements = IOHIDDeviceCopyMatchingElements(
        device,
        null_mut(),
        kIOHIDOptionsTypeNone,
    );
    for i in 0..CFArrayGetCount(elements) {
        let element = CFArrayGetValueAtIndex(elements, i) as IOHIDElementRef;
        if let Some((identifier, device_input)) =
            DeviceInput::try_new(element, &mut index_tracker)
        {
            input_map.insert(identifier, device_input);
        }
    }
    println!("Found {:?} inputs: {:?}", device_type, index_tracker);
    input_map
}
