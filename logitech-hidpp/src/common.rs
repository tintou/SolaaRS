// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Common types shared across HID++ 1.0 and 2.0 protocol layers.

/// Logitech USB vendor ID.
pub const LOGITECH_VENDOR_ID: u16 = 0x046D;

/// USB bus types as reported by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BusId {
    Usb = 0x03,
    Bluetooth = 0x05,
    Unknown(u8),
}

impl From<u8> for BusId {
    fn from(v: u8) -> Self {
        match v {
            0x03 => Self::Usb,
            0x05 => Self::Bluetooth,
            other => Self::Unknown(other),
        }
    }
}

// ─────────────────────────────────────────────
// Firmware
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FirmwareKind {
    Firmware = 0x00,
    Bootloader = 0x01,
    Hardware = 0x02,
    Other = 0x03,
}

impl From<u8> for FirmwareKind {
    fn from(v: u8) -> Self {
        match v {
            0x00 => Self::Firmware,
            0x01 => Self::Bootloader,
            0x02 => Self::Hardware,
            _ => Self::Other,
        }
    }
}

/// Firmware information returned by a device.
#[derive(Debug, Clone)]
pub struct FirmwareInfo {
    pub kind: FirmwareKind,
    pub name: String,
    pub version: String,
    pub extras: Option<Vec<u8>>,
}

// ─────────────────────────────────────────────
// Battery
// ─────────────────────────────────────────────

/// Approximate battery level thresholds used when only a qualitative level is
/// available (HID++ 1.0 devices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BatteryLevelApproximation {
    Empty = 0,
    Critical = 5,
    Low = 20,
    Good = 50,
    Full = 90,
}

/// Battery charging / discharging status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Discharging,
    Recharging,
    AlmostFull,
    Full,
    SlowRecharge,
    InvalidBattery,
    ThermalError,
}

impl BatteryStatus {
    pub fn is_charging(self) -> bool {
        matches!(
            self,
            Self::Recharging | Self::AlmostFull | Self::Full | Self::SlowRecharge
        )
    }
}

/// Battery state reported by a device.
#[derive(Debug, Clone)]
pub struct Battery {
    /// Battery level — either a percentage (0–100) or a
    /// [`BatteryLevelApproximation`] value for HID++ 1.0 devices.
    pub level: Option<u8>,
    /// Predicted next charge level (if reported).
    pub next_level: Option<u8>,
    pub status: Option<BatteryStatus>,
    /// Battery voltage in mV (if reported).
    pub voltage: Option<u16>,
    /// Ambient light level for solar-charged devices.
    pub light_level: Option<u16>,
}

impl Battery {
    const ATTENTION_LEVEL: u8 = 5;

    pub fn new(
        level: Option<u8>,
        next_level: Option<u8>,
        status: Option<BatteryStatus>,
        voltage: Option<u16>,
    ) -> Self {
        let mut b = Self {
            level,
            next_level,
            status,
            voltage,
            light_level: None,
        };
        // Infer level from status when not explicitly provided.
        if b.level.is_none() {
            b.level = match b.status {
                Some(BatteryStatus::Full) => Some(BatteryLevelApproximation::Full as u8),
                Some(BatteryStatus::AlmostFull) | Some(BatteryStatus::Recharging) => {
                    Some(BatteryLevelApproximation::Good as u8)
                }
                Some(BatteryStatus::SlowRecharge) => Some(BatteryLevelApproximation::Low as u8),
                _ => None,
            };
        }
        b
    }

    pub fn ok(&self) -> bool {
        !matches!(
            self.status,
            Some(BatteryStatus::InvalidBattery) | Some(BatteryStatus::ThermalError)
        ) && self.level.is_none_or(|l| l > Self::ATTENTION_LEVEL)
    }

    pub fn charging(&self) -> bool {
        self.status.is_some_and(|s| s.is_charging())
    }
}

impl Default for Battery {
    fn default() -> Self {
        Self::new(None, None, None, None)
    }
}

// ─────────────────────────────────────────────
// Notification codes used in the common layer
// ─────────────────────────────────────────────

/// Well-known notification sub_id values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Notification {
    NoOperation = 0x00,
    ConnectDisconnect = 0x40,
    DjPairing = 0x41,
    Connected = 0x42,
    RawInput = 0x49,
    PairingLock = 0x4A,
    Power = 0x4B,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BatteryStatus ────────────────────────────────────────────────────────

    #[test]
    fn battery_status_charging_variants() {
        assert!(BatteryStatus::Recharging.is_charging());
        assert!(BatteryStatus::AlmostFull.is_charging());
        assert!(BatteryStatus::Full.is_charging());
        assert!(BatteryStatus::SlowRecharge.is_charging());
    }

    #[test]
    fn battery_status_not_charging_variants() {
        assert!(!BatteryStatus::Discharging.is_charging());
        assert!(!BatteryStatus::InvalidBattery.is_charging());
        assert!(!BatteryStatus::ThermalError.is_charging());
    }

    // ── Battery::new level inference ─────────────────────────────────────────

    #[test]
    fn battery_new_infers_level_from_full_status() {
        let b = Battery::new(None, None, Some(BatteryStatus::Full), None);
        assert_eq!(b.level, Some(BatteryLevelApproximation::Full as u8));
    }

    #[test]
    fn battery_new_infers_level_from_almost_full() {
        let b = Battery::new(None, None, Some(BatteryStatus::AlmostFull), None);
        assert_eq!(b.level, Some(BatteryLevelApproximation::Good as u8));
    }

    #[test]
    fn battery_new_infers_level_from_recharging() {
        let b = Battery::new(None, None, Some(BatteryStatus::Recharging), None);
        assert_eq!(b.level, Some(BatteryLevelApproximation::Good as u8));
    }

    #[test]
    fn battery_new_infers_level_from_slow_recharge() {
        let b = Battery::new(None, None, Some(BatteryStatus::SlowRecharge), None);
        assert_eq!(b.level, Some(BatteryLevelApproximation::Low as u8));
    }

    #[test]
    fn battery_new_keeps_explicit_level() {
        let b = Battery::new(Some(75), None, Some(BatteryStatus::Discharging), None);
        assert_eq!(b.level, Some(75));
    }

    #[test]
    fn battery_new_no_level_no_status() {
        let b = Battery::new(None, None, None, None);
        assert_eq!(b.level, None);
    }

    // ── Battery::ok / charging ───────────────────────────────────────────────

    #[test]
    fn battery_ok_normal() {
        let b = Battery::new(Some(80), None, Some(BatteryStatus::Discharging), None);
        assert!(b.ok());
    }

    #[test]
    fn battery_ok_returns_false_for_invalid_battery() {
        let b = Battery::new(Some(80), None, Some(BatteryStatus::InvalidBattery), None);
        assert!(!b.ok());
    }

    #[test]
    fn battery_ok_returns_false_for_thermal_error() {
        let b = Battery::new(Some(80), None, Some(BatteryStatus::ThermalError), None);
        assert!(!b.ok());
    }

    #[test]
    fn battery_ok_returns_false_when_level_at_attention_threshold() {
        let b = Battery::new(Some(5), None, Some(BatteryStatus::Discharging), None);
        assert!(!b.ok());
    }

    #[test]
    fn battery_ok_returns_true_just_above_threshold() {
        let b = Battery::new(Some(6), None, Some(BatteryStatus::Discharging), None);
        assert!(b.ok());
    }

    #[test]
    fn battery_charging() {
        let b = Battery::new(Some(60), None, Some(BatteryStatus::Recharging), None);
        assert!(b.charging());
    }

    #[test]
    fn battery_not_charging_when_discharging() {
        let b = Battery::new(Some(60), None, Some(BatteryStatus::Discharging), None);
        assert!(!b.charging());
    }

    // ── BusId ────────────────────────────────────────────────────────────────

    #[test]
    fn bus_id_from_u8() {
        assert_eq!(BusId::from(0x03), BusId::Usb);
        assert_eq!(BusId::from(0x05), BusId::Bluetooth);
        assert_eq!(BusId::from(0x42), BusId::Unknown(0x42));
    }

    // ── FirmwareKind ─────────────────────────────────────────────────────────

    #[test]
    fn firmware_kind_from_u8() {
        assert_eq!(FirmwareKind::from(0x00), FirmwareKind::Firmware);
        assert_eq!(FirmwareKind::from(0x01), FirmwareKind::Bootloader);
        assert_eq!(FirmwareKind::from(0x02), FirmwareKind::Hardware);
        assert_eq!(FirmwareKind::from(0xFF), FirmwareKind::Other);
    }
}
