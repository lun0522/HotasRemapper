use std::ffi::c_char;
use std::ffi::c_void;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::Once;

use io_kit_sys::ret::kIOReturnSuccess;
use io_kit_sys::ret::IOReturn;
use objc::class;
use objc::declare::ClassDecl;
use objc::msg_send;
use objc::rc::StrongPtr;
use objc::runtime::Class;
use objc::runtime::Object;
use objc::runtime::Sel;
use objc::sel;
use objc::sel_impl;

use super::bluetooth_device::on_device_connected_selector;
use super::bluetooth_device::on_device_disconnected_selector;
use super::bluetooth_device::on_rfcomm_channel_closed_selector;
use super::bluetooth_device::on_rfcomm_channel_opened_selector;
use super::bluetooth_device::BluetoothDevice;
use super::bluetooth_device::DeviceInfo;
use crate::ConnectionStatusCallback;
use crate::ConnectionType;

type PinnedPointer = *mut c_void;

const PINNED_POINTER_VAR: &str = "pinnedPointer";

/// A trait to determine whether the newly found Bluetooth device is our target
/// device.
pub(crate) trait SelectDevice {
    fn is_target_device(&self, device_info: &DeviceInfo) -> bool;
}

pub(crate) struct BluetoothManager<T: SelectDevice> {
    this: StrongPtr,
    target_device_selector: T,
    target_device: Option<BluetoothDevice>,
    rfcomm_channel_id: u8,
    connection_status_callback: ConnectionStatusCallback,
    // We want to make sure the `BluetoothManager` doesn't get moved, so
    // callback functions can rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl<T: SelectDevice> BluetoothManager<T> {
    pub fn new(
        target_device_selector: T,
        rfcomm_channel_id: u8,
        connection_status_callback: ConnectionStatusCallback,
    ) -> Pin<Box<Self>> {
        static REGISTER_CLASS: Once = Once::new();
        REGISTER_CLASS.call_once(Self::register_class);
        let mut manager = Box::pin(Self {
            this: unsafe { new_object(class!(BluetoothManager)) },
            target_device_selector,
            target_device: None,
            rfcomm_channel_id,
            connection_status_callback,
            _pinned_marker: PhantomPinned,
        });
        unsafe { manager.as_mut().store_self_pointer() };
        unsafe { manager.register_for_device_connection_notifications() };
        manager
    }

    pub fn send_data_to_target_device(&self, data: *const c_char, length: u32) {
        if let Some(device) = self.target_device.as_ref() {
            device.send_data(data, length);
        }
    }

    /// This function should only be called once globally.
    fn register_class() {
        let super_class = class!(NSObject);
        let mut decl = ClassDecl::new("BluetoothManager", super_class).unwrap();
        unsafe {
            decl.add_ivar::<PinnedPointer>(PINNED_POINTER_VAR);
            decl.add_method(
                on_device_connected_selector(),
                Self::on_device_connected as extern "C" fn(&Object, _, _, _),
            );
            decl.add_method(
                on_device_disconnected_selector(),
                Self::on_target_device_disconnected
                    as extern "C" fn(&Object, _, _, _),
            );
            decl.add_method(
                on_rfcomm_channel_opened_selector(),
                Self::on_rfcomm_channel_opened
                    as extern "C" fn(&Object, _, _, _),
            );
            decl.add_method(
                on_rfcomm_channel_closed_selector(),
                Self::on_rfcomm_channel_closed as extern "C" fn(&Object, _, _),
            );
        }
        decl.register();
    }

    unsafe fn store_self_pointer(self: Pin<&mut Self>) {
        let self_ptr = &*self as *const Self as *mut _;
        self.this
            .as_mut()
            .unwrap()
            .set_ivar::<PinnedPointer>(PINNED_POINTER_VAR, self_ptr);
    }

    unsafe fn register_for_device_connection_notifications(&self) {
        let () = msg_send![
            class!(IOBluetoothDevice),
            registerForConnectNotifications: *self.this
            selector: on_device_connected_selector()];
    }

    unsafe fn get_pinned_manager(this: &Object) -> Option<&mut Self> {
        let self_ptr = this.get_ivar::<PinnedPointer>(PINNED_POINTER_VAR);
        (*self_ptr as *mut Self).as_mut()
    }

    fn handle_device_connected(&mut self, device: *const Object) {
        let device_info = DeviceInfo::new(device);
        if !self.target_device_selector.is_target_device(&device_info) {
            println!("Ignoring Bluetooth device: {}", device_info);
            return;
        }
        println!("Found target Bluetooth device: {}", device_info);
        // We may get notified more than once for the same device.
        if self.target_device.is_some() {
            return;
        }

        match BluetoothDevice::new(device, self.rfcomm_channel_id, *self.this) {
            Ok(bluetooth_device) => {
                self.report_connection_status(
                    ConnectionType::VirtualDevice,
                    /* is_connected= */ true,
                );
                self.target_device = Some(bluetooth_device);
            }
            Err(e) => println!("Failed to connect to it: {:?}", e),
        }
    }

    fn handle_target_device_disconnected(&mut self) {
        println!("Target Bluetooth device disconnected");
        self.report_connection_status(
            ConnectionType::VirtualDevice,
            /* is_connected= */ false,
        );
        self.target_device = None;
    }

    fn handle_rfcomm_channel_opened(&mut self, status: IOReturn) {
        if status != kIOReturnSuccess {
            println!("Failed to open RFCOMM channel: {}", status);
            return;
        }

        println!("RFCOMM channel opened");
        self.report_connection_status(
            ConnectionType::RFCOMMChannel,
            /* is_connected= */ true,
        );
        match self.target_device.as_mut() {
            Some(device) => {
                device.update_rfcomm_channel_status(/* is_opened= */ true)
            }
            None => println!("Target device is None!"),
        }
    }

    fn handle_rfcomm_channel_closed(&mut self) {
        println!("RFCOMM channel closed");
        self.report_connection_status(
            ConnectionType::RFCOMMChannel,
            /* is_connected= */ false,
        );
        match self.target_device.as_mut() {
            Some(device) => {
                device.update_rfcomm_channel_status(/* is_opened= */ false)
            }
            None => println!("Target device is None!"),
        }
    }

    fn report_connection_status(
        &self,
        connection_type: ConnectionType,
        is_connected: bool,
    ) {
        unsafe {
            (self.connection_status_callback)(connection_type, is_connected);
        }
    }

    extern "C" fn on_device_connected(
        this: &Object,
        _selector: Sel,
        notification: *const Object,
        device: *const Object,
    ) {
        match unsafe { BluetoothManager::<T>::get_pinned_manager(this) } {
            Some(manager) => manager.handle_device_connected(device),
            None => {
                println!(
                    "Cannot handle device connected because manager is None!"
                );
                unsafe { unregister_notification(notification) };
            }
        }
    }

    extern "C" fn on_target_device_disconnected(
        this: &Object,
        _selector: Sel,
        notification: *const Object,
        _device: *const Object,
    ) {
        match unsafe { BluetoothManager::<T>::get_pinned_manager(this) } {
            Some(manager) => manager.handle_target_device_disconnected(),
            None => {
                println!(
                    "Cannot handle target device disconnected because manager \
                    is None!"
                );
                unsafe { unregister_notification(notification) };
            }
        }
    }

    extern "C" fn on_rfcomm_channel_opened(
        this: &Object,
        _selector: Sel,
        _channel: *const Object,
        status: IOReturn,
    ) {
        match unsafe { BluetoothManager::<T>::get_pinned_manager(this) } {
            Some(manager) => manager.handle_rfcomm_channel_opened(status),
            None => println!(
                "Cannot handle RFCOMM channel opened because manager is None!"
            ),
        }
    }

    extern "C" fn on_rfcomm_channel_closed(
        this: &Object,
        _selector: Sel,
        _channel: *const Object,
    ) {
        match unsafe { BluetoothManager::<T>::get_pinned_manager(this) } {
            Some(manager) => manager.handle_rfcomm_channel_closed(),
            None => println!(
                "Cannot handle RFCOMM channel closed because manager is None!"
            ),
        }
    }
}

impl<T: SelectDevice> Drop for BluetoothManager<T> {
    fn drop(&mut self) {
        // Prevent any callbacks to be invoked after dropping.
        unsafe {
            self.this
                .as_mut()
                .unwrap()
                .set_ivar::<PinnedPointer>(PINNED_POINTER_VAR, null_mut());
        }
    }
}

unsafe fn new_object(obj_class: &Class) -> StrongPtr {
    let obj: *mut Object = msg_send![obj_class, alloc];
    let obj: *mut Object = msg_send![obj, init];
    StrongPtr::new(obj)
}

unsafe fn unregister_notification(notification: *const Object) {
    let () = msg_send![notification, unregister];
}
