// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! HID++ 1.0 register-based protocol operations.
//!
//! Ported from `logitech_receiver/hidpp10.py`.

use log::warn;

use crate::common::{
    Battery, BatteryLevelApproximation, BatteryStatus, FirmwareInfo, FirmwareKind,
};
use crate::error::Error;
use crate::hidpp10_constants::{NotificationFlag, Register};

// ─────────────────────────────────────────────────────────────────────────────
// Low-level register accessors
// ─────────────────────────────────────────────────────────────────────────────

/// Trait implemented by any object that can make raw HID++ requests.
///
/// Both [`crate::device::Device`] and [`crate::receiver::Receiver`] implement
/// this trait, allowing the HID++ 1.0 helpers to work generically.
pub trait Hidpp10Device {
    fn request(&self, request_id: u16, params: &[u8]) -> Result<Option<Vec<u8>>, Error>;
    fn protocol(&self) -> f32;
    /// Whether this object is a device (as opposed to a receiver).
    fn is_device(&self) -> bool;
    /// The list of HID++ 1.0 registers this device is known to support.
    fn registers(&self) -> &[Register];
    fn add_register(&mut self, r: Register);
}

/// Read a HID++ 1.0 register (sub-ID 0x81xx for short, 0x83xx for long).
///
/// `params` may hold up to 3 bytes of register address/sub-register.
pub fn read_register<D: Hidpp10Device>(
    device: &D,
    register: Register,
    params: &[u8],
) -> Result<Option<Vec<u8>>, Error> {
    let request_id = 0x8100 | (register.as_u16() & 0x2FF);
    device.request(request_id, params)
}

/// Write a HID++ 1.0 register (sub-ID 0x80xx for short, 0x82xx for long).
pub fn write_register<D: Hidpp10Device>(
    device: &D,
    register: Register,
    value: &[u8],
) -> Result<Option<Vec<u8>>, Error> {
    let request_id = 0x8000 | (register.as_u16() & 0x2FF);
    device.request(request_id, value)
}

// ─────────────────────────────────────────────────────────────────────────────
// Receiver-specific helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Read and return the configuration pending flags from the receiver.
pub fn get_configuration_pending_flags<D: Hidpp10Device>(
    receiver: &D,
) -> Result<Option<u8>, Error> {
    let reply = read_register(receiver, Register::DevicesConfiguration, &[])?;
    Ok(reply.and_then(|r| r.first().copied()))
}

/// Set the configuration pending flags on the receiver.
pub fn set_configuration_pending_flags<D: Hidpp10Device>(
    receiver: &D,
    devices: u8,
) -> Result<bool, Error> {
    let reply = write_register(receiver, Register::DevicesConfiguration, &[devices])?;
    Ok(reply.is_some())
}

// ─────────────────────────────────────────────────────────────────────────────
// Hidpp10 operations
// ─────────────────────────────────────────────────────────────────────────────

/// Collection of HID++ 1.0 device operations.
pub struct Hidpp10;

impl Hidpp10 {
    /// Read the battery status from a HID++ 1.0 device.
    ///
    /// Returns `None` if the device is a HID++ 2.0 device (which uses features
    /// instead of registers), or if no battery register is found.
    pub fn get_battery<D: Hidpp10Device>(&self, device: &mut D) -> Result<Option<Battery>, Error> {
        if device.protocol() >= 2.0 {
            return Ok(None);
        }

        // Try the registers we already know this device has.
        for &r in &[Register::BatteryStatus, Register::BatteryCharge] {
            if device.registers().contains(&r) {
                let reply = read_register(device, r, &[])?;
                return Ok(reply.and_then(|data| parse_battery_status(r, &data)));
            }
        }

        // Unknown – probe both registers.
        let reply = read_register(device, Register::BatteryCharge, &[])?;
        if let Some(data) = reply {
            device.add_register(Register::BatteryCharge);
            return Ok(parse_battery_status(Register::BatteryCharge, &data));
        }

        let reply = read_register(device, Register::BatteryStatus, &[])?;
        if let Some(data) = reply {
            device.add_register(Register::BatteryStatus);
            return Ok(parse_battery_status(Register::BatteryStatus, &data));
        }

        Ok(None)
    }

    /// Read firmware information from a HID++ 1.0 device.
    pub fn get_firmware<D: Hidpp10Device>(
        &self,
        device: &D,
    ) -> Result<Option<Vec<FirmwareInfo>>, Error> {
        let reply = read_register(device, Register::Firmware, &[0x01])?;
        let reply = match reply {
            Some(r) => r,
            None => return Ok(None),
        };

        let fw_version = format!("{:02X}.{:02X}", reply[1], reply[2]);
        let fw_version =
            if let Some(build_reply) = read_register(device, Register::Firmware, &[0x02])? {
                format!("{fw_version}.B{:02X}{:02X}", build_reply[1], build_reply[2])
            } else {
                fw_version
            };

        let mut firmware = vec![FirmwareInfo {
            kind: FirmwareKind::Firmware,
            name: String::new(),
            version: fw_version,
            extras: None,
        }];

        if let Some(bl_reply) = read_register(device, Register::Firmware, &[0x04])? {
            let bl_version = format!("{:02X}.{:02X}", bl_reply[1], bl_reply[2]);
            firmware.push(FirmwareInfo {
                kind: FirmwareKind::Bootloader,
                name: String::new(),
                version: bl_version,
                extras: None,
            });
        }

        if let Some(o_reply) = read_register(device, Register::Firmware, &[0x03])? {
            let o_version = format!("{:02X}.{:02X}", o_reply[1], o_reply[2]);
            firmware.push(FirmwareInfo {
                kind: FirmwareKind::Other,
                name: String::new(),
                version: o_version,
                extras: None,
            });
        }

        Ok(Some(firmware))
    }

    /// Read the current notification flags from a device/receiver.
    pub fn get_notification_flags<D: Hidpp10Device>(
        &self,
        device: &D,
    ) -> Result<Option<NotificationFlag>, Error> {
        if device.protocol() >= 2.0 {
            return Ok(None);
        }
        let flags = self.get_register_u24(device, Register::Notifications)?;
        Ok(flags.map(NotificationFlag::from_bits_truncate))
    }

    /// Write notification flags to a device/receiver.
    pub fn set_notification_flags<D: Hidpp10Device>(
        &self,
        device: &D,
        flags: NotificationFlag,
    ) -> Result<bool, Error> {
        if device.protocol() >= 2.0 {
            return Ok(false);
        }
        let v = flags.bits();
        let result = write_register(
            device,
            Register::Notifications,
            &[(v >> 16) as u8, (v >> 8) as u8, v as u8],
        )?;
        Ok(result.is_some())
    }

    /// Read the device feature flags (MOUSE_BUTTON_FLAGS register).
    pub fn get_device_features<D: Hidpp10Device>(&self, device: &D) -> Result<Option<u32>, Error> {
        self.get_register_u24(device, Register::MouseButtonFlags)
    }

    /// Set the 3-LED battery indicator LEDs on a device that supports them.
    pub fn set_3leds<D: Hidpp10Device>(
        &self,
        device: &D,
        battery_level: Option<u8>,
        charging: bool,
        warning: bool,
    ) -> Result<(), Error> {
        if !device.registers().contains(&Register::ThreeLeds) {
            return Ok(());
        }

        let (v1, v2) = if let Some(level) = battery_level {
            let (mut v1, mut v2) = if level < BatteryLevelApproximation::Low as u8 {
                // Critical or low: 1 orange LED
                (0x22u8, 0x00u8)
            } else if level < BatteryLevelApproximation::Good as u8 {
                (0x20, 0x00)
            } else if level < BatteryLevelApproximation::Full as u8 {
                (0x20, 0x02)
            } else {
                (0x20, 0x22)
            };
            if warning {
                v1 |= v1 >> 1;
                v2 |= v2 >> 1;
            }
            (v1, v2)
        } else if charging {
            (0x30u8, 0x33u8)
        } else if warning {
            (0x02, 0x00)
        } else {
            (0x11, 0x11)
        };

        write_register(device, Register::ThreeLeds, &[v1, v2])?;
        Ok(())
    }

    // Internal helper: read a 3-byte (24-bit) register and return as u32.
    fn get_register_u24<D: Hidpp10Device>(
        &self,
        device: &D,
        register: Register,
    ) -> Result<Option<u32>, Error> {
        if device.protocol() >= 2.0 {
            return Ok(None);
        }
        let reply = read_register(device, register, &[])?;
        if let Some(data) = reply {
            if data.len() < 3 {
                warn!(
                    "register {:?} reply too short: {} bytes",
                    register,
                    data.len()
                );
                return Ok(None);
            }
            let value = (data[0] as u32) << 16 | (data[1] as u32) << 8 | data[2] as u32;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Battery parsing helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a raw HID++ 1.0 battery register reply into a [`Battery`] value.
pub fn parse_battery_status(register: Register, reply: &[u8]) -> Option<Battery> {
    match register {
        Register::BatteryCharge => {
            if reply.is_empty() {
                return None;
            }
            let charge = reply[0];
            let status_byte = if reply.len() > 2 { reply[2] & 0xF0 } else { 0 };
            let status = match status_byte {
                0x30 => Some(BatteryStatus::Discharging),
                0x50 => Some(BatteryStatus::Recharging),
                0x90 => Some(BatteryStatus::Full),
                _ => None,
            };
            Some(Battery::new(Some(charge), None, status, None))
        }

        Register::BatteryStatus => {
            if reply.len() < 2 {
                return None;
            }
            let status_byte = reply[0];
            let charging_byte = reply[1];

            let status = if charging_byte == 0x00 {
                Some(BatteryStatus::Discharging)
            } else if charging_byte & 0x21 == 0x21 {
                Some(BatteryStatus::Recharging)
            } else if charging_byte & 0x22 == 0x22 {
                Some(BatteryStatus::Full)
            } else {
                warn!(
                    "could not parse 0x07 battery status: charging_byte={charging_byte:02X} status_byte={status_byte:02X}"
                );
                None
            };

            let charge = if charging_byte & 0x03 != 0 && status_byte == 0 {
                None // Charging notification with no level
            } else {
                Some(status_byte_to_charge(status_byte))
            };

            Some(Battery::new(charge, None, status, None))
        }

        _ => None,
    }
}

fn status_byte_to_charge(status_byte: u8) -> u8 {
    match status_byte {
        7 => BatteryLevelApproximation::Full as u8,
        5 => BatteryLevelApproximation::Good as u8,
        3 => BatteryLevelApproximation::Low as u8,
        1 => BatteryLevelApproximation::Critical as u8,
        _ => BatteryLevelApproximation::Empty as u8,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Receiver info extraction helpers (used by receiver.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the serial number from a RECEIVER_INFO response.
pub fn extract_serial(response: &[u8]) -> String {
    hex::encode(&response[1..5]).to_uppercase()
}

/// Extract the max device count from a RECEIVER_INFO response.
pub fn extract_max_devices(response: &[u8]) -> u8 {
    response.get(6).copied().unwrap_or(1).clamp(1, 6)
}

/// Extract remaining pairings from a RECEIVER_CONNECTION response.
pub fn extract_remaining_pairings(response: &[u8]) -> i32 {
    let ps = response.get(2).copied().unwrap_or(0) as i32;
    if ps >= 5 { ps - 5 } else { -1 }
}

/// Decode a device codename from a DEVICE_NAME sub-register reply.
pub fn extract_codename(response: &[u8]) -> Option<String> {
    let len = *response.get(1)? as usize;
    let bytes = response.get(2..2 + len)?;
    String::from_utf8(bytes.to_vec()).ok()
}

/// Extract the wireless product ID (wpid) from a response.
pub fn extract_wpid(response: &[u8]) -> String {
    hex::encode(response).to_uppercase()
}

/// Extract the polling rate (in ms) from a PAIRING_INFORMATION response.
pub fn extract_polling_rate(response: &[u8]) -> u8 {
    response.get(2).copied().unwrap_or(0)
}

/// Extract the power switch location byte from an EXTENDED_PAIRING_INFORMATION reply.
pub fn extract_power_switch_location(response: &[u8]) -> u8 {
    response.get(9).copied().unwrap_or(0) & 0x0F
}

/// Extract the connection count from a RECEIVER_CONNECTION reply.
pub fn extract_connection_count(response: &[u8]) -> u8 {
    response.get(1).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{BatteryLevelApproximation, BatteryStatus};
    use crate::hidpp10_constants::Register;

    // ── parse_battery_status: BatteryCharge register ─────
    // Mirrors device_charge* cases from test_hidpp10.py::test_hidpp10_get_battery.

    #[test]
    fn parse_battery_charge_discharging() {
        // response "550030": charge=0x55, status_byte & 0xF0 = 0x30 → Discharging
        let b = parse_battery_status(Register::BatteryCharge, &[0x55, 0x00, 0x30]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(0x55));
        assert_eq!(b.status, Some(BatteryStatus::Discharging));
    }

    #[test]
    fn parse_battery_charge_recharging() {
        // response "440050": charge=0x44, status=0x50 → Recharging
        let b = parse_battery_status(Register::BatteryCharge, &[0x44, 0x00, 0x50]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(0x44));
        assert_eq!(b.status, Some(BatteryStatus::Recharging));
    }

    #[test]
    fn parse_battery_charge_full() {
        // response "600090": charge=0x60, status=0x90 → Full
        let b = parse_battery_status(Register::BatteryCharge, &[0x60, 0x00, 0x90]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(0x60));
        assert_eq!(b.status, Some(BatteryStatus::Full));
    }

    #[test]
    fn parse_battery_charge_unknown_status() {
        // response "220000": charge=0x22, status nibble=0x00 → no known status
        let b = parse_battery_status(Register::BatteryCharge, &[0x22, 0x00, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(0x22));
        assert_eq!(b.status, None);
    }

    #[test]
    fn parse_battery_charge_empty_reply() {
        assert!(parse_battery_status(Register::BatteryCharge, &[]).is_none());
    }

    // ── parse_battery_status: BatteryStatus register ──────────────────────────
    // Mirrors device_status* cases from test_hidpp10.py::test_hidpp10_get_battery.

    #[test]
    fn parse_battery_status_full() {
        // response "072200": status_byte=7 → Full level, charging_byte=0x22 → Full
        let b = parse_battery_status(Register::BatteryStatus, &[0x07, 0x22, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(BatteryLevelApproximation::Full as u8));
        assert_eq!(b.status, Some(BatteryStatus::Full));
    }

    #[test]
    fn parse_battery_status_good_recharging() {
        // response "052100": status_byte=5 → Good level, charging_byte=0x21 → Recharging
        let b = parse_battery_status(Register::BatteryStatus, &[0x05, 0x21, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(BatteryLevelApproximation::Good as u8));
        assert_eq!(b.status, Some(BatteryStatus::Recharging));
    }

    #[test]
    fn parse_battery_status_low_full() {
        // response "032200": status_byte=3 → Low level, charging_byte=0x22 → Full
        let b = parse_battery_status(Register::BatteryStatus, &[0x03, 0x22, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(BatteryLevelApproximation::Low as u8));
        assert_eq!(b.status, Some(BatteryStatus::Full));
    }

    #[test]
    fn parse_battery_status_critical_no_status() {
        // response "010100": status_byte=1 → Critical, charging_byte=0x01 (unknown)
        let b = parse_battery_status(Register::BatteryStatus, &[0x01, 0x01, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(BatteryLevelApproximation::Critical as u8));
        assert_eq!(b.status, None);
    }

    #[test]
    fn parse_battery_status_empty_discharging() {
        // response "000000": status_byte=0 → Empty level, charging_byte=0x00 → Discharging
        let b = parse_battery_status(Register::BatteryStatus, &[0x00, 0x00, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(BatteryLevelApproximation::Empty as u8));
        assert_eq!(b.status, Some(BatteryStatus::Discharging));
    }

    #[test]
    fn parse_battery_status_no_level_full() {
        // response "002200": status_byte=0, charging_byte=0x22
        // 0x22 & 0x03 = 0x02 (bit 1 set), status_byte=0 → charge=None → inferred as Full level
        let b = parse_battery_status(Register::BatteryStatus, &[0x00, 0x22, 0x00]);
        let b = b.expect("should parse");
        assert_eq!(b.level, Some(BatteryLevelApproximation::Full as u8));
        assert_eq!(b.status, Some(BatteryStatus::Full));
    }

    #[test]
    fn parse_battery_status_short_reply() {
        assert!(parse_battery_status(Register::BatteryStatus, &[0x07]).is_none());
    }

    #[test]
    fn parse_battery_status_wrong_register() {
        // Only BatteryCharge and BatteryStatus are handled.
        assert!(parse_battery_status(Register::Firmware, &[0x01, 0x02, 0x03]).is_none());
    }

    // ── extract helpers (ported from receiver-info decode tests) ─────────────

    #[test]
    fn extract_serial_from_bytes() {
        // Serial occupies bytes [1..5]
        let response = [0x00u8, 0xAB, 0xCD, 0xEF, 0x01, 0x00, 0x06];
        assert_eq!(extract_serial(&response), "ABCDEF01");
    }

    #[test]
    fn extract_max_devices_normal() {
        // max_devices at byte [6]
        let response = [0u8; 7];
        let mut r = response;
        r[6] = 6;
        assert_eq!(extract_max_devices(&r), 6);
    }

    #[test]
    fn extract_max_devices_clamped_to_one() {
        // 0 is clamped to 1
        let response = [0u8; 7];
        assert_eq!(extract_max_devices(&response), 1);
    }

    #[test]
    fn extract_max_devices_clamped_to_six() {
        let mut r = [0u8; 7];
        r[6] = 10; // clamped to 6
        assert_eq!(extract_max_devices(&r), 6);
    }

    #[test]
    fn extract_remaining_pairings_positive() {
        // ps=7 → 7-5=2
        let mut r = [0u8; 5];
        r[2] = 7;
        assert_eq!(extract_remaining_pairings(&r), 2);
    }

    #[test]
    fn extract_remaining_pairings_negative() {
        // ps=3 < 5 → -1
        let mut r = [0u8; 5];
        r[2] = 3;
        assert_eq!(extract_remaining_pairings(&r), -1);
    }

    #[test]
    fn extract_codename_ascii() {
        // bytes: [padding, len, 'A', 'B', 'C']
        let response = [0x00u8, 0x03, b'A', b'B', b'C', 0x00];
        let name = extract_codename(&response).expect("should decode");
        assert_eq!(name, "ABC");
    }

    #[test]
    fn extract_polling_rate_from_response() {
        let mut r = [0u8; 5];
        r[2] = 0x08; // 8ms polling
        assert_eq!(extract_polling_rate(&r), 8);
    }

    #[test]
    fn extract_power_switch_location_nibble() {
        let mut r = [0u8; 11];
        r[9] = 0x3F; // & 0x0F → 0x0F
        assert_eq!(extract_power_switch_location(&r), 0x0F);
    }

    #[test]
    fn extract_connection_count_from_response() {
        let r = [0x00u8, 0x03, 0x00];
        assert_eq!(extract_connection_count(&r), 3);
    }
}
