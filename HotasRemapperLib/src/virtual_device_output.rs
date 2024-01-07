use std::ffi::c_char;

enum ReportField {
    _ModifierKeyStates = 0,
    KeyStatesBegin = 1,
    KeyStatesEndExclusive = 7,
}

const REPORT_LENGTH: usize = ReportField::KeyStatesEndExclusive as usize;

pub(crate) struct VirtualDeviceOutput {
    report: [c_char; REPORT_LENGTH],
}

impl VirtualDeviceOutput {
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
