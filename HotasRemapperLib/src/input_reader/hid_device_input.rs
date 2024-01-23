use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use io_kit_sys::hid::base::IOHIDElementRef;
use io_kit_sys::hid::element::IOHIDElementGetCookie;
use io_kit_sys::hid::element::IOHIDElementGetType;
use io_kit_sys::hid::element::IOHIDElementGetUsage;
use io_kit_sys::hid::keys::kIOHIDElementTypeCollection;
use io_kit_sys::hid::keys::kIOHIDElementTypeInput_Button;
use io_kit_sys::hid::keys::kIOHIDElementTypeInput_Misc;
use io_kit_sys::hid::keys::kIOHIDElementTypeOutput;
use io_kit_sys::hid::keys::IOHIDElementCookie;
use io_kit_sys::hid::keys::IOHIDElementType;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Hatswitch;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Rx;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Ry;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Rz;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Slider;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_X;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Y;
use io_kit_sys::hid::usage_tables::kHIDUsage_GD_Z;

#[allow(non_upper_case_globals)]
const kIOHIDElementTypeInput_NULL: IOHIDElementType = 5;
#[allow(non_upper_case_globals)]
const kHIDUsage_Ignored: u32 = kHIDUsage_GD_X - 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum InputType {
    Button,
    Hat,
    XAxis,
    YAxis,
    ZAxis,
    RxAxis,
    RyAxis,
    RzAxis,
    Slider,
    Other,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) struct DeviceInput {
    pub input_type: InputType,
    pub index: i32,
}

impl DeviceInput {
    /// Safety: the caller must ensure the element is alive.
    #[allow(non_upper_case_globals)]
    #[deny(unsafe_op_in_unsafe_fn)]
    pub unsafe fn try_new(
        element: IOHIDElementRef,
        index_tracker: &mut HashMap<InputType, i32>,
    ) -> Option<(IOHIDElementCookie, Self)> {
        let mut new_input = |input_type: InputType| {
            let index: &mut i32 = index_tracker.entry(input_type).or_default();
            let curr_index = *index;
            *index += 1;
            Self {
                input_type,
                index: curr_index,
            }
        };

        // Safe because the caller guarantees `element` is valid.
        let (identifier, element_type, usage) = unsafe {
            (
                IOHIDElementGetCookie(element),
                IOHIDElementGetType(element),
                IOHIDElementGetUsage(element),
            )
        };
        match element_type {
            kIOHIDElementTypeInput_Button => {
                return Some((identifier, new_input(InputType::Button)))
            }
            kIOHIDElementTypeInput_Misc => match usage {
                0..=kHIDUsage_Ignored => {
                    return Some((identifier, new_input(InputType::Other)));
                }
                kHIDUsage_GD_X => {
                    return Some((identifier, new_input(InputType::XAxis)));
                }
                kHIDUsage_GD_Y => {
                    return Some((identifier, new_input(InputType::YAxis)));
                }
                kHIDUsage_GD_Z => {
                    return Some((identifier, new_input(InputType::ZAxis)));
                }
                kHIDUsage_GD_Rx => {
                    return Some((identifier, new_input(InputType::RxAxis)));
                }
                kHIDUsage_GD_Ry => {
                    return Some((identifier, new_input(InputType::RyAxis)));
                }
                kHIDUsage_GD_Rz => {
                    return Some((identifier, new_input(InputType::RzAxis)));
                }
                kHIDUsage_GD_Slider => {
                    return Some((identifier, new_input(InputType::Slider)));
                }
                kHIDUsage_GD_Hatswitch => {
                    return Some((identifier, new_input(InputType::Hat)));
                }
                _ => (),
            },
            kIOHIDElementTypeInput_NULL
            | kIOHIDElementTypeOutput
            | kIOHIDElementTypeCollection => {
                return Some((identifier, new_input(InputType::Other)));
            }
            _ => (),
        }
        println!(
            "Unknown input: {{id {}, type {}, usage {:#x}}}",
            identifier, element_type, usage,
        );
        None
    }
}

impl Display for DeviceInput {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_fmt(format_args!("{:?}{}", self.input_type, self.index))
    }
}
