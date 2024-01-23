use std::ffi::c_char;
use std::ffi::c_void;
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

pub(crate) struct BluetoothManager {
    object: StrongPtr,
    // We want to make sure the `BluetoothManager` doesn't get moved, so
    // callback functions can rely on an everlasting pointer to it.
    _pinned_marker: PhantomPinned,
}

impl BluetoothManager {
    pub fn new() -> Pin<Box<Self>> {
        static REGISTER_CLASS: Once = Once::new();
        REGISTER_CLASS.call_once(Self::register_class);
        let mut manager = Box::pin(Self {
            object: unsafe { new_object(class!(BluetoothManager)) },
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
                on_device_connected as extern "C" fn(&Object, _, _, _),
            );
        }
        decl.register();
    }

    unsafe fn store_self_pointer(self: Pin<&mut Self>) {
        let self_ptr = &*self as *const BluetoothManager as *mut _;
        self.object
            .as_mut()
            .unwrap()
            .set_ivar::<PinnedPointer>(PINNED_POINTER_VAR, self_ptr);
    }

    unsafe fn register_for_device_connection_notifications(&self) {
        let () = msg_send![
            class!(IOBluetoothDevice),
            registerForConnectNotifications: *self.object
            selector: on_device_connected_selector()];
    }

    fn handle_device_connected(&mut self, device: *const Object) {
        let device_name: *const Object = unsafe { msg_send![device, name] };
        match unsafe { new_string_from_nsstring(device_name) } {
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

unsafe fn new_string_from_nsstring(nsstring: *const Object) -> Result<String> {
    let string_ptr: *const c_char = msg_send![nsstring, UTF8String];
    unsafe { new_string_from_ptr(string_ptr) }
}

#[inline]
fn on_device_connected_selector() -> Sel {
    sel!(notification:fromDevice:)
}

extern "C" fn on_device_connected(
    this: &Object,
    _selector: Sel,
    _notification: *const Object,
    device: *const Object,
) {
    let manager_ptr =
        unsafe { this.get_ivar::<PinnedPointer>(PINNED_POINTER_VAR) };
    match unsafe { (*manager_ptr as *mut BluetoothManager).as_mut() } {
        Some(manager) => manager.handle_device_connected(device),
        None => println!(
            "Cannot handle device connected because manager_ptr is null!"
        ),
    }
}
