mod bluetooth_device;
mod bluetooth_manager;

use std::ffi::c_char;
use std::pin::Pin;
use std::time::Instant;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use bluetooth_device::DeviceInfo;
use bluetooth_manager::SelectDevice;

use crate::settings::VirtualDeviceSettings;
use crate::ConnectionStatusCallback;

type BluetoothManager =
    bluetooth_manager::BluetoothManager<VirtualDeviceSelector>;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum KeyEvent {
    Press(c_char),
    Release(c_char),
    PressAndRelease(c_char),
    ReleaseAndPress {
        to_release: c_char,
        to_press: c_char,
    },
}

struct SentKeyEvent {
    pub event: KeyEvent,
    pub timestamp: Instant,
}

struct VirtualDeviceSelector {
    mac_address: String,
}

impl VirtualDeviceSelector {
    pub fn new(settings: &VirtualDeviceSettings) -> Self {
        Self {
            mac_address: settings.mac_address.clone(),
        }
    }
}

impl SelectDevice for VirtualDeviceSelector {
    fn is_target_device(&self, device_info: &DeviceInfo) -> bool {
        device_info.mac_address == self.mac_address
    }
}

/// This device is connected via Bluetooth, responsible for forwarding HID
/// keyboard input events generated by us.
pub(crate) struct VirtualDevice {
    bluetooth_manager: Pin<Box<BluetoothManager>>,
    input_report: KeyboardInputReport,
    last_sent_key_event: Option<SentKeyEvent>,
    rate_limiting_threshold_ms: u128,
}

impl VirtualDevice {
    pub fn new(
        settings: &VirtualDeviceSettings,
        connection_status_callback: ConnectionStatusCallback,
    ) -> Result<Self> {
        let rfcomm_channel_id = u8::try_from(settings.rfcomm_channel_id)
            .map_err(|e| {
                anyhow!("Cannot convert rfcomm_channel_id to u8: {}", e)
            })?;
        let rate_limiting_threshold_ms = settings.rate_limiting_threshold_ms;
        if rate_limiting_threshold_ms < 0 {
            bail!("rate_limiting_threshold_ms must be non-negative!");
        }
        Ok(Self {
            bluetooth_manager: BluetoothManager::new(
                VirtualDeviceSelector::new(settings),
                rfcomm_channel_id,
                connection_status_callback,
            ),
            input_report: KeyboardInputReport::new(),
            last_sent_key_event: None,
            rate_limiting_threshold_ms: rate_limiting_threshold_ms as u128,
        })
    }

    pub fn send_key_event(&mut self, key_event: KeyEvent) {
        if !self.should_send_key_event(key_event) {
            return;
        }
        self.last_sent_key_event = Some(SentKeyEvent {
            event: key_event,
            timestamp: Instant::now(),
        });
        match key_event {
            KeyEvent::Press(key_code) => self.send_key_press_event(key_code),
            KeyEvent::Release(key_code) => {
                self.send_key_release_event(key_code)
            }
            KeyEvent::PressAndRelease(key_code) => {
                self.send_key_press_event(key_code);
                self.send_key_release_event(key_code);
            }
            KeyEvent::ReleaseAndPress {
                to_release,
                to_press,
            } => {
                self.send_key_release_event(to_release);
                self.send_key_press_event(to_press);
            }
        }
    }

    fn should_send_key_event(&self, new_event: KeyEvent) -> bool {
        match &self.last_sent_key_event {
            Some(last_event) => {
                if new_event != last_event.event {
                    true
                } else {
                    Instant::now()
                        .duration_since(last_event.timestamp)
                        .as_millis()
                        >= self.rate_limiting_threshold_ms
                }
            }
            None => true,
        }
    }

    fn update_and_send_input_report(
        &mut self,
        key_code: c_char,
        is_pressed: bool,
    ) {
        self.input_report.update_key_state(key_code, is_pressed);
        self.bluetooth_manager
            .send_data_to_target_device(self.input_report.report());
    }

    #[inline]
    fn send_key_press_event(&mut self, key_code: c_char) {
        self.update_and_send_input_report(key_code, /* is_pressed= */ true)
    }

    #[inline]
    fn send_key_release_event(&mut self, key_code: c_char) {
        self.update_and_send_input_report(
            key_code, /* is_pressed= */ false,
        )
    }
}
enum ReportField {
    _ModifierKeyStates = 0,
    KeyStatesBegin = 1,
    KeyStatesEndExclusive = 7,
}

const REPORT_LENGTH: usize = ReportField::KeyStatesEndExclusive as usize;

struct KeyboardInputReport {
    report: [c_char; REPORT_LENGTH],
}

impl KeyboardInputReport {
    pub fn new() -> Self {
        Self {
            report: [0; REPORT_LENGTH],
        }
    }

    pub fn report(&self) -> &[c_char; REPORT_LENGTH] {
        &self.report
    }

    pub fn update_key_state(&mut self, key_code: c_char, is_pressed: bool) {
        // If this key has been pressed previously, remove it from the report if
        // it is now released.
        if let Some(key_state) = self.find_key_state(key_code) {
            if !is_pressed {
                *key_state = 0x00;
            }
            return;
        }
        // If a new key is pressed, find an available slot for it.
        if is_pressed {
            match self.find_key_state(0x00) {
                Some(key_state) => *key_state = key_code,
                None => println!(
                    "No available slot for key press (keycode: {})",
                    key_code
                ),
            }
        }
    }

    fn find_key_state(&mut self, key_code: c_char) -> Option<&mut c_char> {
        for index in ReportField::KeyStatesBegin as usize
            ..ReportField::KeyStatesEndExclusive as usize
        {
            if self.report[index] == key_code {
                return Some(&mut self.report[index]);
            }
        }
        None
    }
}
