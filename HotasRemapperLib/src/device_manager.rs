use std::collections::HashMap;
use std::convert::From;
use std::ffi::c_char;
use std::ffi::c_void;
use std::marker::PhantomPinned;
use std::pin::Pin;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use io_kit_sys::hid::base::IOHIDDeviceCallback;
use io_kit_sys::hid::base::IOHIDDeviceRef;
use io_kit_sys::hid::base::IOHIDValueCallback;
use io_kit_sys::hid::base::IOHIDValueRef;
use io_kit_sys::ret::IOReturn;
use protobuf::text_format::parse_from_str as parse_proto_from_str;

use crate::input_reader::hid_device::DeviceProperty;
use crate::input_reader::hid_device::DeviceType as HIDDeviceType;
use crate::input_reader::hid_device::HIDDevice;
use crate::input_reader::hid_device::HandleInputEvent;
use crate::input_reader::hid_manager::HIDManager;
use crate::input_reader::hid_manager::HandleDeviceEvent;
use crate::input_remapper::InputRemapper;
use crate::settings::Settings;
use crate::utils::new_string_from_ptr;
use crate::virtual_device::VirtualDevice;
use crate::ConnectionStatusCallback;
use crate::ConnectionType;

pub(crate) struct DeviceManager {
    hid_manager: HIDManager,
    hid_devices: HashMap<IOHIDDeviceRef, HIDDevice>,
    virtual_deivce: VirtualDevice,
    input_remapper: InputRemapper,
    connection_status_callback: ConnectionStatusCallback,
    // We want to make sure the `DeviceManager` doesn't get moved, so the user
    // can rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl DeviceManager {
    /// `settings_ptr` must point to a UTF-8 encoded `Settings` message.
    #[deny(unsafe_op_in_unsafe_fn)]
    pub unsafe fn new(
        settings_ptr: *const c_char,
        connection_status_callback: ConnectionStatusCallback,
    ) -> Result<Pin<Box<Self>>> {
        // Safe because the caller guarantees `settings_ptr` is valid.
        let settings = match unsafe { load_settings(settings_ptr) } {
            Ok(settings) => settings,
            Err(e) => {
                println!("Failed to load settings: {:?}", e);
                println!("Please reload settings and restart!");
                Settings::new()
            }
        };
        println!("Initializing with settings: {}", dump_settings(&settings));

        let mut manager = Box::pin(Self {
            hid_manager: HIDManager::new(&settings.input_reader_settings)?,
            hid_devices: Default::default(),
            virtual_deivce: VirtualDevice::new(
                &settings.virtual_device_settings,
                connection_status_callback,
            )?,
            input_remapper: InputRemapper::new(),
            connection_status_callback,
            _pinned_marker: PhantomPinned,
        });
        // Safe because we won't move `DeviceManager` out of the pinned object,
        // and it outlives its member `HIDManager`.
        unsafe {
            let pinned_manager_ptr =
                manager.as_mut().get_unchecked_mut() as *mut Self as *mut _;
            manager
                .as_ref()
                .hid_manager
                .set_device_callbacks(pinned_manager_ptr);
        }
        Ok(manager)
    }

    /// `input_remapping_ptr` must point to a UTF-8 encoded `InputRemapping`
    /// message.
    pub unsafe fn load_input_remapping(
        &mut self,
        input_remapping_ptr: *const c_char,
    ) -> Result<()> {
        let encoded_input_remapping = new_string_from_ptr(input_remapping_ptr)
            .map_err(|e| anyhow!("Invalid input_remapping_ptr: {}", e))?;
        self.input_remapper
            .load_input_remapping(&encoded_input_remapping)
    }

    fn report_connection_status(
        &self,
        device_type: HIDDeviceType,
        is_connected: bool,
    ) {
        // Safe because the caller guarantees the callback remains a valid
        // function pointer.
        unsafe {
            (self.connection_status_callback)(device_type.into(), is_connected)
        };
    }

    fn handle_device_matched(&mut self, device_ref: IOHIDDeviceRef) {
        let device_property = DeviceProperty::from_device(device_ref);
        if let Some(device_type) = self
            .hid_manager
            .try_get_device_type(&device_property.device_name)
        {
            // Open a new device only if we haven't found any devices of the
            // same type.
            if !self
                .hid_devices
                .iter()
                .any(|(_, device)| device.device_type() == device_type)
            {
                println!("Found {:?} device: {}", device_type, device_property);
                let pinned_manager_ptr = self as *mut DeviceManager;
                // Safe because the device is alive, and `self` outlives it.
                self.hid_devices.insert(device_ref, unsafe {
                    HIDDevice::open_device(
                        device_ref,
                        device_type,
                        pinned_manager_ptr,
                    )
                });
                self.report_connection_status(
                    device_type,
                    /* is_connected= */ true,
                );
                return;
            }
        }
        println!("Ignoring device: {}", device_property);
    }

    fn handle_device_removed(&mut self, device_ref: IOHIDDeviceRef) {
        if let Some(device) = self.hid_devices.remove(&device_ref) {
            let device_type = device.device_type();
            self.report_connection_status(
                device_type,
                /* is_connected= */ false,
            );
            println!("Removed {:?} device", device_type);
        }
    }

    fn handle_input_received(&mut self, value: IOHIDValueRef) {
        if let Some(raw_input_event) = HIDDevice::read_raw_input_event(value) {
            if let Some(device) =
                self.hid_devices.get(&raw_input_event.device_ref)
            {
                if let Some(input_event) =
                    device.interpret_raw_input_event(raw_input_event)
                {
                    if let Some(key_events) =
                        self.input_remapper.remap_input_event(&input_event)
                    {
                        for key_event in key_events {
                            self.virtual_deivce.send_output_with_new_key_event(
                                key_event.key_code,
                                key_event.is_pressed,
                            )
                        }
                    }
                }
                return;
            }
        }
    }
}

impl HandleDeviceEvent for DeviceManager {
    fn device_matched_callback() -> IOHIDDeviceCallback {
        handle_device_matched
    }

    fn device_removed_callback() -> IOHIDDeviceCallback {
        handle_device_removed
    }
}

impl HandleInputEvent for DeviceManager {
    fn input_received_callback() -> IOHIDValueCallback {
        handle_input_received
    }
}

impl From<HIDDeviceType> for ConnectionType {
    fn from(value: HIDDeviceType) -> Self {
        match value {
            HIDDeviceType::Joystick => ConnectionType::Joystick,
            HIDDeviceType::Throttle => ConnectionType::Throttle,
        }
    }
}

/// `settings_ptr` must point to a UTF-8 encoded `Settings` message.
unsafe fn load_settings(settings_ptr: *const c_char) -> Result<Settings> {
    let encoded_settings = new_string_from_ptr(settings_ptr)
        .map_err(|e| anyhow!("Invalid settings_ptr: {}", e))?;
    if encoded_settings.is_empty() {
        bail!("No settings provided!");
    }
    parse_proto_from_str::<Settings>(&encoded_settings)
        .map_err(|e| anyhow!("Failed to parse as text proto: {}", e))
}

extern "C" fn handle_device_matched(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `DeviceManager`.
    if let Some(manager) = unsafe { (context as *mut DeviceManager).as_mut() } {
        manager.handle_device_matched(device);
    }
}

extern "C" fn handle_device_removed(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    // Safe because we stored a pointer to a pinned `DeviceManager`.
    if let Some(manager) = unsafe { (context as *mut DeviceManager).as_mut() } {
        manager.handle_device_removed(device);
    }
}

extern "C" fn handle_input_received(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    value: IOHIDValueRef,
) {
    // Safe because we stored a pointer to a pinned `DeviceManager`.
    if let Some(manager) = unsafe { (context as *mut DeviceManager).as_mut() } {
        manager.handle_input_received(value);
    }
}

fn dump_settings(settings: &Settings) -> String {
    format!(
        "
\tJoystick device name: {:?}
\tThrottle device name: {:?}
\tHost MAC address: {}
\tRFCOMM channel ID: {}
",
        settings.input_reader_settings.joystick_device_name,
        settings.input_reader_settings.throttle_device_name,
        settings.virtual_device_settings.host_mac_address,
        settings.virtual_device_settings.rfcomm_channel_id
    )
}
