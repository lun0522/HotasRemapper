use std::ffi::c_void;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::Once;

use objc::class;
use objc::declare::ClassDecl;
use objc::msg_send;
use objc::rc::StrongPtr;
use objc::runtime::Class;
use objc::runtime::Object;
use objc::runtime::Sel;
use objc::sel;
use objc::sel_impl;

use super::bluetooth_device::BluetoothDevice;
use super::bluetooth_device::DeviceEventHandler;
use super::bluetooth_device::DeviceInfo;

type PinnedPointer = *mut c_void;

const PINNED_POINTER_VAR: &str = "pinnedPointer";

pub(crate) trait SelectDevice {
    fn is_target_device(&self, device_info: &DeviceInfo) -> bool;
}

pub(crate) struct BluetoothManager<T: SelectDevice> {
    this: StrongPtr,
    device_selector: T,
    target_device: Option<BluetoothDevice>,
    // We want to make sure the `BluetoothManager` doesn't get moved, so
    // callback functions can rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl<T: SelectDevice> BluetoothManager<T> {
    pub fn new(device_selector: T) -> Pin<Box<Self>> {
        static REGISTER_CLASS: Once = Once::new();
        REGISTER_CLASS.call_once(Self::register_class);
        let mut manager = Box::pin(Self {
            this: unsafe { new_object(class!(BluetoothManager)) },
            device_selector,
            target_device: None,
            _pinned_marker: PhantomPinned,
        });
        unsafe { manager.as_mut().store_self_pointer() };
        unsafe { manager.register_for_device_connection_notifications() };
        manager
    }

    /// This function should only be called once globally.
    fn register_class() {
        let super_class = class!(NSObject);
        let mut decl = ClassDecl::new("BluetoothManager", super_class).unwrap();
        unsafe {
            decl.add_ivar::<PinnedPointer>(PINNED_POINTER_VAR);
            decl.add_method(
                on_device_connected_selector(),
                on_device_connected::<T> as extern "C" fn(&Object, _, _, _),
            );
            decl.add_method(
                on_device_disconnected_selector(),
                on_target_device_disconnected::<T>
                    as extern "C" fn(&Object, _, _, _),
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

    unsafe fn get_pinned_manager(this: &Object) -> Option<&mut Self> {
        let self_ptr = this.get_ivar::<PinnedPointer>(PINNED_POINTER_VAR);
        (*self_ptr as *mut Self).as_mut()
    }

    unsafe fn register_for_device_connection_notifications(&self) {
        let () = msg_send![
            class!(IOBluetoothDevice),
            registerForConnectNotifications: *self.this
            selector: on_device_connected_selector()];
    }

    fn handle_device_connected(&mut self, device: *const Object) {
        let device_info = DeviceInfo::new(device);
        if !self.device_selector.is_target_device(&device_info) {
            println!("Ignoring Bluetooth device: {}", device_info);
            return;
        }
        println!("Found target Bluetooth device: {}", device_info);
        if self.target_device.is_none() {
            self.target_device = Some(BluetoothDevice::new(
                device,
                DeviceEventHandler {
                    handler_ptr: *self.this,
                    on_device_disconnected: on_device_disconnected_selector(),
                },
            ));
        }
    }

    fn handle_target_device_disconnected(&mut self) {
        println!("Target Bluetooth device disconnected");
        self.target_device = None;
    }
}

unsafe fn new_object(obj_class: &Class) -> StrongPtr {
    let obj: *mut Object = msg_send![obj_class, alloc];
    let obj: *mut Object = msg_send![obj, init];
    StrongPtr::new(obj)
}

#[inline]
fn on_device_connected_selector() -> Sel {
    sel!(didConnectWithNotification:fromDevice:)
}

extern "C" fn on_device_connected<T: SelectDevice>(
    this: &Object,
    _selector: Sel,
    _notification: *const Object,
    device: *const Object,
) {
    match unsafe { BluetoothManager::<T>::get_pinned_manager(this) } {
        Some(manager) => manager.handle_device_connected(device),
        None => {
            println!("Cannot handle device connected because manager is None!")
        }
    }
}

#[inline]
fn on_device_disconnected_selector() -> Sel {
    sel!(didDisconnectWithNotification:fromDevice:)
}

extern "C" fn on_target_device_disconnected<T: SelectDevice>(
    this: &Object,
    _selector: Sel,
    _notification: *const Object,
    _device: *const Object,
) {
    match unsafe { BluetoothManager::<T>::get_pinned_manager(this) } {
        Some(manager) => manager.handle_target_device_disconnected(),
        None => println!(
            "Cannot handle target device disconnected because manager is None!"
        ),
    }
}
