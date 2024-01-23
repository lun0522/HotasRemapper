use std::ffi::c_char;
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

use crate::utils::new_string_from_ptr;

pub(crate) struct BluetoothManager {
    object: StrongPtr,
}

impl BluetoothManager {
    pub fn new() -> Self {
        static REGISTER_CLASS: Once = Once::new();
        REGISTER_CLASS.call_once(Self::register_class);
        let manager = Self {
            object: unsafe { new_object(class!(BluetoothManager)) },
        };
        unsafe { manager.register_for_device_connection_notifications() };
        manager
    }

    /// This function should only be called once globally.
    fn register_class() {
        let super_class = class!(NSObject);
        let mut decl = ClassDecl::new("BluetoothManager", super_class).unwrap();
        unsafe {
            decl.add_method(
                Self::on_device_connected_selector(),
                Self::on_device_connected as extern "C" fn(&Object, _, _, _),
            );
        }
        decl.register();
    }

    unsafe fn register_for_device_connection_notifications(&self) {
        let () = msg_send![
            class!(IOBluetoothDevice),
            registerForConnectNotifications: *self.object
            selector: Self::on_device_connected_selector()];
    }

    #[inline]
    fn on_device_connected_selector() -> Sel {
        sel!(notification:fromDevice:)
    }

    extern "C" fn on_device_connected(
        _this: &Object,
        _selector: Sel,
        _notification: *const Object,
        device: *const Object,
    ) {
        let device_name: *const Object = unsafe { msg_send![device, name] };
        let device_name_ptr: *const c_char =
            unsafe { msg_send![device_name, UTF8String] };
        match unsafe { new_string_from_ptr(device_name_ptr) } {
            Ok(device_name) => println!("Found device {}", device_name),
            Err(e) => println!("Failed to get device name: {:?}", e),
        }
    }
}

unsafe fn new_object(obj_class: &Class) -> StrongPtr {
    let obj: *mut Object = msg_send![obj_class, alloc];
    let obj: *mut Object = msg_send![obj, init];
    StrongPtr::new(obj)
}
