use std::collections::HashMap;
use std::ffi::c_char;
use std::ffi::c_void;
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

use crate::hid_device_input::DeviceInput;
use crate::hid_device_input::InputType;
use crate::utils::new_cf_string;
use crate::utils::new_string;

/// A trait to provide what we need for calling
/// `IOHIDDeviceRegisterInputValueCallback()`.
pub(crate) trait HandleInputValue {
    fn get_pinned_pointer(&self) -> *mut c_void;
    fn get_callback(&self) -> IOHIDValueCallback;
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
                new_cf_string(key).unwrap().as_concrete_TypeRef(),
            )
        };
        // Safe because the system guarantees `IOHIDDeviceGetProperty()` returns
        // a pointer that is either NULL or points to a valid string.
        let get_string_property = |key: *const c_char, default: &str| unsafe {
            get_property(key)
                .as_ref()
                .and_then(|name| {
                    new_string(name as *const c_void as CFStringRef).ok()
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

impl std::fmt::Display for DeviceProperty {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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
pub(crate) struct InputEvent {
    pub device_ref: IOHIDDeviceRef,
    pub input_id: IOHIDElementCookie,
    pub value: i32,
}

/// A struct wrapping `IOHIDDeviceRef` from IOKit.
pub(crate) struct HIDDevice {
    device_type: DeviceType,
    input_map: HashMap<IOHIDElementCookie, DeviceInput>,
}

impl HIDDevice {
    /// Safety: the caller must ensure the device is alive.
    pub unsafe fn open_device(
        device: IOHIDDeviceRef,
        device_type: DeviceType,
        input_value_handler: &dyn HandleInputValue,
    ) -> Self {
        IOHIDDeviceRegisterInputValueCallback(
            device,
            input_value_handler.get_callback(),
            input_value_handler.get_pinned_pointer(),
        );
        Self {
            device_type,
            input_map: build_input_map(device, device_type),
        }
    }

    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }

    pub fn handle_input_event(&self, input_event: InputEvent) {
        match self.input_map.get(&input_event.input_id) {
            Some(device_input) => {
                if !matches!(device_input.input_type, InputType::Other) {
                    println!(
                        "New input from {:?} {}: {}",
                        self.device_type, device_input.name, input_event.value,
                    );
                }
            }
            None => println!(
                "Unknown input event from {:?}: {:?}",
                self.device_type, input_event,
            ),
        }
    }

    pub fn read_input_event(value: IOHIDValueRef) -> Option<InputEvent> {
        // Safe because the system guarantees these references are valid.
        unsafe {
            if value.is_null() {
                return None;
            }
            let element = IOHIDValueGetElement(value);
            if element.is_null() {
                return None;
            }
            Some(InputEvent {
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
