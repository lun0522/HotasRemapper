use std::ffi::c_char;
use std::ffi::c_void;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::Once;

use anyhow::Result;
use objc::class;
use objc::declare::ClassDecl;
use objc::msg_send;
use objc::rc::StrongPtr;
use objc::runtime::Class;
use objc::runtime::Object;
use objc::runtime::Sel;
use objc::sel;
use objc::sel_impl;

use crate::utils::new_string_from_ptr;

type PinnedPointer = *mut c_void;

const PINNED_POINTER_VAR: &str = "pinnedPointer";

pub(crate) trait SelectDevice {
    fn is_target_device(&self, device_info: &DeviceInfo) -> bool;
}

pub(crate) struct DeviceInfo {
    pub name: String,
    pub mac_address: String,
}

impl DeviceInfo {
    pub fn new(device: *const Object) -> Self {
        Self {
            name: unsafe { Self::get_name(device) },
            mac_address: unsafe { Self::get_mac_address(device) },
        }
    }

    unsafe fn get_name(device: *const Object) -> String {
        let name: *const Object = unsafe { msg_send![device, name] };
        unsafe { new_string_from_nsstring(name) }
            .unwrap_or("Unknown name".to_string())
    }

    unsafe fn get_mac_address(device: *const Object) -> String {
        let mac_address: *const Object =
            unsafe { msg_send![device, addressString] };
        unsafe { new_string_from_nsstring(mac_address) }
            .unwrap_or("Unknown MAC address".to_string())
    }
}

impl Display for DeviceInfo {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!(
            "{{device name: {:?}, MAC address: {}}}",
            self.name, self.mac_address
        ))
    }
}

pub(crate) struct BluetoothManager<T: SelectDevice> {
    this: StrongPtr,
    device_selector: T,
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
        }
        decl.register();
    }

    unsafe fn store_self_pointer(self: Pin<&mut Self>) {
        let self_ptr = &*self as *const BluetoothManager<T> as *mut _;
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

    fn handle_device_connected(&mut self, device: *const Object) {
        let device_info = DeviceInfo::new(device);
        if !self.device_selector.is_target_device(&device_info) {
            println!("Ignoring Bluetooth device: {}", device_info);
            return;
        }
        println!("Found target Bluetooth device: {}", device_info);
    }
}

unsafe fn new_object(obj_class: &Class) -> StrongPtr {
    let obj: *mut Object = msg_send![obj_class, alloc];
    let obj: *mut Object = msg_send![obj, init];
    StrongPtr::new(obj)
}

unsafe fn new_string_from_nsstring(nsstring: *const Object) -> Result<String> {
    let string_ptr: *const c_char = msg_send![nsstring, UTF8String];
    unsafe { new_string_from_ptr(string_ptr) }
}

#[inline]
fn on_device_connected_selector() -> Sel {
    sel!(notification:fromDevice:)
}

extern "C" fn on_device_connected<T: SelectDevice>(
    this: &Object,
    _selector: Sel,
    _notification: *const Object,
    device: *const Object,
) {
    let manager_ptr =
        unsafe { this.get_ivar::<PinnedPointer>(PINNED_POINTER_VAR) };
    match unsafe { (*manager_ptr as *mut BluetoothManager<T>).as_mut() } {
        Some(manager) => manager.handle_device_connected(device),
        None => println!(
            "Cannot handle device connected because manager_ptr is null!"
        ),
    }
}
