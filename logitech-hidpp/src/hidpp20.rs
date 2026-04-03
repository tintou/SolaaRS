// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! HID++ 2.0 feature-based protocol operations.
//!
//! Ported from `logitech_receiver/hidpp20.py`.

use std::collections::HashMap;

use log::{error, warn};

use crate::common::{Battery, BatteryStatus, FirmwareInfo, FirmwareKind};
use crate::error::Error;
use crate::hidpp20_constants::{FeatureFlag, SupportedFeature};

// ─────────────────────────────────────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait implemented by HID++ 2.0 capable devices.
///
/// `feature_request` is the single entry-point used by all HID++ 2.0 helpers.
pub trait Hidpp20Device {
    /// Send a HID++ 2.0 feature call and return the reply payload.
    ///
    /// The implementation must resolve `feature` to a device-assigned feature
    /// index, then issue the request `(feature_index << 8) | (function & 0xFF)`.
    fn feature_request(
        &self,
        feature: SupportedFeature,
        function: u8,
        params: &[u8],
    ) -> Result<Option<Vec<u8>>, Error>;

    fn is_online(&self) -> bool;
    fn protocol(&self) -> f32;
}

// ─────────────────────────────────────────────────────────────────────────────
// Feature cache
// ─────────────────────────────────────────────────────────────────────────────

/// Cached HID++ 2.0 feature table entry.
#[derive(Debug, Clone)]
pub struct FeatureEntry {
    /// Index assigned by the device (0x00–0xFF).
    pub index: u8,
    /// Feature version returned by the device.
    pub version: u8,
    /// Feature flags (hidden, obsolete, internal).
    pub flags: FeatureFlag,
}

/// Lazy cache of a device's HID++ 2.0 feature table.
///
/// Feature look-ups send a `ROOT` request to the device on first access, then
/// cache the result.  All subsequent look-ups are answered from the cache.
///
/// This is an internal building block; use [`Device::feature_request`] instead.
#[derive(Debug)]
pub struct FeaturesArray {
    /// `feature_id -> FeatureEntry` for each discovered feature.
    entries: HashMap<u16, FeatureEntry>,
    /// Reverse map: `index -> feature_id`.
    inverse: HashMap<u8, u16>,
    /// Total number of features (including ROOT) as reported by the device.
    pub count: usize,
    /// `Some(false)` once we know the device does not support FEATURE_SET.
    supported: Option<bool>,
}

impl FeaturesArray {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            inverse: HashMap::new(),
            count: 0,
            supported: None,
        }
    }

    /// Returns `true` if the feature set has been successfully initialised.
    pub fn is_initialised(&self) -> bool {
        self.count > 0
    }

    /// Initialise the feature table by discovering `ROOT` and `FEATURE_SET`.
    ///
    /// Returns `true` on success, `false` if the device does not support
    /// HID++ 2.0 or refused the request.
    ///
    /// `request_fn` sends a raw HID++ request:
    ///   `request_fn(request_id, params) -> Option<Vec<u8>>`
    pub fn init<F>(&mut self, mut request_fn: F) -> bool
    where
        F: FnMut(u16, &[u8]) -> Option<Vec<u8>>,
    {
        if self.supported == Some(false) {
            return false;
        }
        if self.is_initialised() {
            return true;
        }

        // Ask ROOT (index 0x0000) for the index of FEATURE_SET (0x0001).
        let feature_set_id = SupportedFeature::FeatureSet.as_u16();
        let reply = request_fn(0x0000, &feature_set_id.to_be_bytes());
        let reply = match reply {
            Some(r) => r,
            None => {
                self.supported = Some(false);
                return false;
            }
        };

        let fs_index = reply[0];
        if fs_index == 0 {
            self.supported = Some(false);
            return false;
        }

        // Ask FEATURE_SET for the feature count (function 0x00 = getCount).
        let count_reply = request_fn((fs_index as u16) << 8, &[]);
        let count = match count_reply {
            Some(r) => r[0] as usize + 1, // +1 because ROOT is not counted
            None => {
                warn!("FEATURE_SET found but failed to read feature count");
                return false;
            }
        };

        self.count = count;
        self.supported = Some(true);

        // Seed the two entries we already know.
        self.insert(SupportedFeature::Root.as_u16(), 0, 0, FeatureFlag::empty());
        self.insert(
            SupportedFeature::FeatureSet.as_u16(),
            fs_index,
            0,
            FeatureFlag::empty(),
        );

        true
    }

    fn insert(&mut self, feature_id: u16, index: u8, version: u8, flags: FeatureFlag) {
        self.entries.insert(
            feature_id,
            FeatureEntry {
                index,
                version,
                flags,
            },
        );
        self.inverse.insert(index, feature_id);
    }

    /// Look up the feature index for `feature`, fetching it from the device
    /// if not yet cached.
    ///
    /// Returns `None` if the device does not support the feature, or if the
    /// feature table has not been initialised.
    pub fn get_index<F>(&mut self, feature: SupportedFeature, mut request_fn: F) -> Option<u8>
    where
        F: FnMut(u16, &[u8]) -> Option<Vec<u8>>,
    {
        let fid = feature.as_u16();
        if let Some(entry) = self.entries.get(&fid) {
            return Some(entry.index);
        }
        if !self.is_initialised() {
            return None;
        }

        // Ask ROOT for this feature's index.
        let reply = request_fn(0x0000, &fid.to_be_bytes())?;
        let index = reply[0];
        if index == 0 {
            return None; // Feature not supported by device
        }
        let flags = FeatureFlag::from_bits_truncate(reply[1]);
        let version = reply[2];
        self.insert(fid, index, version, flags);
        Some(index)
    }

    /// Look up the feature for an `index` (reverse map).
    pub fn get_feature(
        &mut self,
        index: u8,
        mut request_fn: impl FnMut(u16, &[u8]) -> Option<Vec<u8>>,
    ) -> Option<SupportedFeature> {
        if let Some(&fid) = self.inverse.get(&index) {
            return SupportedFeature::from_u16(fid);
        }
        if !self.is_initialised() {
            return None;
        }
        // Ask FEATURE_SET for info about this index.
        let fs_index = self
            .entries
            .get(&SupportedFeature::FeatureSet.as_u16())?
            .index;
        // Function 0x10 = getFeatureID
        let reply = request_fn(((fs_index as u16) << 8) | 0x10, &[index])?;
        let fid = u16::from_be_bytes([reply[0], reply[1]]);
        let flags = FeatureFlag::from_bits_truncate(reply[2]);
        let version = reply[3];
        self.insert(fid, index, version, flags);
        SupportedFeature::from_u16(fid)
    }

    pub fn contains<F>(&mut self, feature: SupportedFeature, request_fn: F) -> bool
    where
        F: FnMut(u16, &[u8]) -> Option<Vec<u8>>,
    {
        self.get_index(feature, request_fn).is_some()
    }

    pub fn get_entry(&self, feature: SupportedFeature) -> Option<&FeatureEntry> {
        self.entries.get(&feature.as_u16())
    }
}

impl Default for FeaturesArray {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hidpp20 operations
// ─────────────────────────────────────────────────────────────────────────────

/// Collection of HID++ 2.0 device operations.
pub struct Hidpp20;

impl Hidpp20 {
    /// Read firmware information from a HID++ 2.0 device using the
    /// `DEVICE_FW_VERSION` feature (0x0003).
    pub fn get_firmware<D: Hidpp20Device>(
        &self,
        device: &D,
    ) -> Result<Option<Vec<FirmwareInfo>>, Error> {
        let count_reply = device.feature_request(SupportedFeature::DeviceFwVersion, 0x00, &[])?;
        let count = match count_reply {
            Some(r) => r[0] as usize,
            None => return Ok(None),
        };

        let mut fw = Vec::with_capacity(count);
        for index in 0..count {
            let fw_info =
                device.feature_request(SupportedFeature::DeviceFwVersion, 0x10, &[index as u8])?;
            if let Some(data) = fw_info {
                let level = data[0] & 0x0F;
                let info = match level {
                    0 | 1 => {
                        let name = String::from_utf8_lossy(&data[1..4]).into_owned();
                        let version = format!("{:02X}.{:02X}", data[4], data[5]);
                        let build = u16::from_be_bytes([data[6], data[7]]);
                        let version = if build != 0 {
                            format!("{version}.B{build:04X}")
                        } else {
                            version
                        };
                        let extras: Vec<u8> =
                            data[9..].iter().copied().take_while(|&b| b != 0).collect();
                        FirmwareInfo {
                            kind: FirmwareKind::from(level),
                            name,
                            version,
                            extras: if extras.is_empty() {
                                None
                            } else {
                                Some(extras)
                            },
                        }
                    }
                    0x02 => FirmwareInfo {
                        kind: FirmwareKind::Hardware,
                        name: String::new(),
                        version: data[1].to_string(),
                        extras: None,
                    },
                    _ => FirmwareInfo {
                        kind: FirmwareKind::Other,
                        name: String::new(),
                        version: String::new(),
                        extras: None,
                    },
                };
                fw.push(info);
            }
        }
        Ok(Some(fw))
    }

    /// Read the device's full name using the `DEVICE_NAME` feature (0x0005).
    pub fn get_name<D: Hidpp20Device>(&self, device: &D) -> Result<Option<String>, Error> {
        let len_reply = device.feature_request(SupportedFeature::DeviceName, 0x00, &[])?;
        let name_length = match len_reply {
            Some(r) => r[0] as usize,
            None => return Ok(None),
        };

        let mut name = Vec::with_capacity(name_length);
        while name.len() < name_length {
            let fragment =
                device.feature_request(SupportedFeature::DeviceName, 0x10, &[name.len() as u8])?;
            match fragment {
                Some(data) => {
                    let take = (name_length - name.len()).min(data.len());
                    name.extend_from_slice(&data[..take]);
                }
                None => {
                    error!(
                        "failed to read full device name (got {}/{} chars)",
                        name.len(),
                        name_length
                    );
                    return Ok(None);
                }
            }
        }
        Ok(String::from_utf8(name).ok())
    }

    /// Read the device's friendly name using `DEVICE_FRIENDLY_NAME` (0x0007).
    pub fn get_friendly_name<D: Hidpp20Device>(&self, device: &D) -> Result<Option<String>, Error> {
        let len_reply = device.feature_request(SupportedFeature::DeviceFriendlyName, 0x00, &[])?;
        let name_length = match len_reply {
            Some(r) => r[0] as usize,
            None => return Ok(None),
        };

        let mut name = Vec::with_capacity(name_length);
        while name.len() < name_length {
            let fragment = device.feature_request(
                SupportedFeature::DeviceFriendlyName,
                0x10,
                &[name.len() as u8],
            )?;
            match fragment {
                Some(data) => {
                    // Friendly name fragments start with a position byte.
                    let take = (name_length - name.len()).min(data.len().saturating_sub(1));
                    name.extend_from_slice(&data[1..1 + take]);
                }
                None => {
                    error!(
                        "failed to read full friendly name (got {}/{} chars)",
                        name.len(),
                        name_length
                    );
                    return Ok(None);
                }
            }
        }
        Ok(String::from_utf8(name).ok())
    }

    /// Read the device kind using the `DEVICE_NAME` feature function 0x20.
    pub fn get_kind<D: Hidpp20Device>(
        &self,
        device: &D,
    ) -> Result<Option<crate::hidpp20_constants::DeviceKind>, Error> {
        let reply = device.feature_request(SupportedFeature::DeviceName, 0x20, &[])?;
        Ok(reply.map(|r| crate::hidpp20_constants::DeviceKind::from(r[0])))
    }

    // ── Battery reading ───────────────────────────────────────────────────────

    /// Try all known battery features and return the first result.
    ///
    /// Returns `Some((feature, battery))` where `feature` is the
    /// [`SupportedFeature`] that succeeded, allowing callers to cache which
    /// feature to use on subsequent calls.
    pub fn get_battery<D: Hidpp20Device>(
        &self,
        device: &D,
        preferred: Option<SupportedFeature>,
    ) -> Result<Option<(SupportedFeature, Battery)>, Error> {
        let candidates: &[SupportedFeature] = match preferred {
            Some(f) => {
                // Try the preferred feature first via a single-item slice.
                // Fall through to the full list if it fails.
                if let Some(result) = self.try_battery_feature(device, f)? {
                    return Ok(Some((f, result)));
                }
                &[
                    SupportedFeature::UnifiedBattery,
                    SupportedFeature::BatteryVoltage,
                    SupportedFeature::BatteryStatus,
                ]
            }
            None => &[
                SupportedFeature::UnifiedBattery,
                SupportedFeature::BatteryVoltage,
                SupportedFeature::BatteryStatus,
            ],
        };

        for &feature in candidates {
            if let Some(battery) = self.try_battery_feature(device, feature)? {
                return Ok(Some((feature, battery)));
            }
        }
        Ok(None)
    }

    fn try_battery_feature<D: Hidpp20Device>(
        &self,
        device: &D,
        feature: SupportedFeature,
    ) -> Result<Option<Battery>, Error> {
        match feature {
            SupportedFeature::BatteryStatus => self.get_battery_status(device),
            SupportedFeature::BatteryVoltage => self.get_battery_voltage(device),
            SupportedFeature::UnifiedBattery => self.get_battery_unified(device),
            _ => Ok(None),
        }
    }

    /// Read battery via `BATTERY_STATUS` feature (0x1000).
    pub fn get_battery_status<D: Hidpp20Device>(
        &self,
        device: &D,
    ) -> Result<Option<Battery>, Error> {
        let report = device.feature_request(SupportedFeature::BatteryStatus, 0x00, &[])?;
        Ok(report.and_then(|r| decipher_battery_status(&r)))
    }

    /// Read battery via `UNIFIED_BATTERY` feature (0x1004), function 0x10.
    pub fn get_battery_unified<D: Hidpp20Device>(
        &self,
        device: &D,
    ) -> Result<Option<Battery>, Error> {
        let report = device.feature_request(SupportedFeature::UnifiedBattery, 0x10, &[])?;
        Ok(report.and_then(|r| decipher_battery_unified(&r)))
    }

    /// Read battery via `BATTERY_VOLTAGE` feature (0x1001).
    pub fn get_battery_voltage<D: Hidpp20Device>(
        &self,
        device: &D,
    ) -> Result<Option<Battery>, Error> {
        let report = device.feature_request(SupportedFeature::BatteryVoltage, 0x00, &[])?;
        Ok(report.and_then(|r| decipher_battery_voltage(&r)))
    }

    /// Read the current polling rate (in ms) via `REPORT_RATE` (0x8060).
    pub fn get_polling_rate<D: Hidpp20Device>(&self, device: &D) -> Result<Option<u8>, Error> {
        let reply = device.feature_request(SupportedFeature::ReportRate, 0x10, &[])?;
        Ok(reply.map(|r| r[0]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Battery decoding helpers
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn decipher_battery_status(report: &[u8]) -> Option<Battery> {
    if report.len() < 3 {
        return None;
    }
    let level = report[0];
    let next_level = report[1];
    let status_byte = report[2];
    let status = match status_byte & 0x0F {
        0x00 => Some(BatteryStatus::Discharging),
        0x01 => Some(BatteryStatus::Recharging),
        0x02 => Some(BatteryStatus::AlmostFull),
        0x03 => Some(BatteryStatus::Full),
        0x04 => Some(BatteryStatus::SlowRecharge),
        0x05 => Some(BatteryStatus::InvalidBattery),
        0x06 => Some(BatteryStatus::ThermalError),
        _ => None,
    };
    Some(Battery::new(Some(level), Some(next_level), status, None))
}

pub(crate) fn decipher_battery_unified(report: &[u8]) -> Option<Battery> {
    if report.len() < 4 {
        return None;
    }
    let level = report[0];
    let next_level = report[1];
    let status_byte = report[2];
    let _charge_type = report[3]; // ChargeType

    let status = match status_byte {
        0x00 => Some(BatteryStatus::Discharging),
        0x01 => Some(BatteryStatus::Recharging),
        0x02 => Some(BatteryStatus::AlmostFull),
        0x03 => Some(BatteryStatus::Full),
        0x04 => Some(BatteryStatus::SlowRecharge),
        0x05 => Some(BatteryStatus::InvalidBattery),
        0x06 => Some(BatteryStatus::ThermalError),
        _ => None,
    };
    Some(Battery::new(Some(level), Some(next_level), status, None))
}

pub(crate) fn decipher_battery_voltage(report: &[u8]) -> Option<Battery> {
    if report.len() < 3 {
        return None;
    }
    let voltage = u16::from_be_bytes([report[0], report[1]]);
    let flags = report[2];
    let status = if flags & 0x80 != 0 {
        Some(BatteryStatus::Recharging)
    } else {
        Some(BatteryStatus::Discharging)
    };
    // Approximate level from voltage (3.5V = 0%, 4.2V = 100%).
    let level = if voltage >= 4200 {
        100u8
    } else if voltage <= 3500 {
        0
    } else {
        ((voltage - 3500) * 100 / 700) as u8
    };
    Some(Battery::new(Some(level), None, status, Some(voltage)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::BatteryStatus;

    // ── decipher_battery_status ───────────────────────────────────────────────
    // Mirrors test_hidpp20_simple.py::test_decipher_battery_status.

    #[test]
    fn decipher_battery_status_discharging() {
        // report b"\x50\x20\x00\xff\xff"
        let report = [0x50u8, 0x20, 0x00, 0xFF, 0xFF];
        let b = decipher_battery_status(&report).expect("should parse");
        assert_eq!(b.level, Some(80));
        assert_eq!(b.next_level, Some(32));
        assert_eq!(b.status, Some(BatteryStatus::Discharging));
    }

    #[test]
    fn decipher_battery_status_recharging() {
        let report = [0x60u8, 0x00, 0x01, 0xFF, 0xFF]; // status nibble 0x01 → Recharging
        let b = decipher_battery_status(&report).expect("should parse");
        assert_eq!(b.level, Some(96));
        assert_eq!(b.status, Some(BatteryStatus::Recharging));
    }

    #[test]
    fn decipher_battery_status_full() {
        let report = [0x64u8, 0x00, 0x03, 0xFF, 0xFF]; // status nibble 0x03 → Full
        let b = decipher_battery_status(&report).expect("should parse");
        assert_eq!(b.level, Some(100));
        assert_eq!(b.status, Some(BatteryStatus::Full));
    }

    #[test]
    fn decipher_battery_status_almost_full() {
        let report = [0x5Au8, 0x00, 0x02, 0xFF, 0xFF]; // 0x02 → AlmostFull
        let b = decipher_battery_status(&report).expect("should parse");
        assert_eq!(b.status, Some(BatteryStatus::AlmostFull));
    }

    #[test]
    fn decipher_battery_status_too_short() {
        assert!(decipher_battery_status(&[0x50, 0x20]).is_none());
    }

    // ── decipher_battery_unified ──────────────────────────────────────────────
    // Mirrors test_hidpp20_simple.py::test_decipher_battery_unified.

    #[test]
    fn decipher_battery_unified_discharging() {
        // report b"\x50\x01\x00\xff\xff"
        let report = [0x50u8, 0x01, 0x00, 0xFF, 0xFF];
        let b = decipher_battery_unified(&report).expect("should parse");
        assert_eq!(b.level, Some(80));
        assert_eq!(b.status, Some(BatteryStatus::Discharging));
    }

    #[test]
    fn decipher_battery_unified_recharging() {
        let report = [0x64u8, 0x00, 0x01, 0x00, 0xFF];
        let b = decipher_battery_unified(&report).expect("should parse");
        assert_eq!(b.level, Some(100));
        assert_eq!(b.status, Some(BatteryStatus::Recharging));
    }

    #[test]
    fn decipher_battery_unified_too_short() {
        assert!(decipher_battery_unified(&[0x50, 0x01, 0x00]).is_none());
    }

    // ── decipher_battery_voltage ──────────────────────────────────────────────
    // Mirrors test_hidpp20_simple.py::test_decipher_battery_voltage.

    #[test]
    fn decipher_battery_voltage_recharging() {
        // report b"\x10\x00\xff\xff\xff" → voltage=0x1000=4096mV, bit7 set (0xFF) → Recharging
        let report = [0x10u8, 0x00, 0xFF, 0xFF, 0xFF];
        let b = decipher_battery_voltage(&report).expect("should parse");
        assert_eq!(b.voltage, Some(0x1000));
        assert_eq!(b.status, Some(BatteryStatus::Recharging));
        // Voltage 4096mV → level = (4096-3500)*100/700 = 85
        assert_eq!(b.level, Some(85));
    }

    #[test]
    fn decipher_battery_voltage_discharging() {
        // voltage=3700mV, bit7 clear → Discharging
        let report = [0x0Eu8, 0x74, 0x00, 0xFF, 0xFF]; // 0x0E74 = 3700
        let b = decipher_battery_voltage(&report).expect("should parse");
        assert_eq!(b.voltage, Some(3700));
        assert_eq!(b.status, Some(BatteryStatus::Discharging));
        // (3700-3500)*100/700 = 20000/700 = 28
        assert_eq!(b.level, Some(28));
    }

    #[test]
    fn decipher_battery_voltage_max() {
        let report = [0x10u8, 0x68, 0x00, 0xFF, 0xFF]; // 0x1068 = 4200mV
        let b = decipher_battery_voltage(&report).expect("should parse");
        assert_eq!(b.level, Some(100));
    }

    #[test]
    fn decipher_battery_voltage_min() {
        let report = [0x0Du8, 0xAC, 0x00, 0xFF, 0xFF]; // 0x0DAC = 3500mV
        let b = decipher_battery_voltage(&report).expect("should parse");
        assert_eq!(b.level, Some(0));
    }

    #[test]
    fn decipher_battery_voltage_too_short() {
        assert!(decipher_battery_voltage(&[0x10, 0x00]).is_none());
    }

    // ── FeaturesArray::init ───────────────────────────────────────────────────
    // Mirrors test_hidpp20_complex.py::test_FeaturesArray_check.

    #[test]
    fn features_array_init_no_reply_returns_false() {
        // Device does not respond → init returns false.
        let mut fa = FeaturesArray::new();
        let result = fa.init(|_req, _params| None);
        assert!(!result);
        assert!(!fa.is_initialised());
    }

    #[test]
    fn features_array_init_zero_feature_set_index_returns_false() {
        // ROOT replies with feature_set_index=0 → not supported.
        let mut fa = FeaturesArray::new();
        let result = fa.init(|req, _params| {
            if req == 0x0000 {
                Some(vec![0x00, 0x00, 0x00, 0x00]) // fs_index = 0
            } else {
                None
            }
        });
        assert!(!result);
    }

    #[test]
    fn features_array_init_success() {
        // ROOT says FeatureSet is at index 1; FeatureSet says count=8 (returns 7 → +1=8).
        let mut fa = FeaturesArray::new();
        let result = fa.init(|req, _params| match req {
            0x0000 => Some(vec![0x01, 0x00, 0x00, 0x00]), // fs_index = 1
            0x0100 => Some(vec![0x07, 0x00, 0x00, 0x00]), // count = 7+1 = 8
            _ => None,
        });
        assert!(result);
        assert!(fa.is_initialised());
        assert_eq!(fa.count, 8);
    }

    #[test]
    fn features_array_init_idempotent() {
        // Calling init twice should return true on second call without issuing more requests.
        let mut fa = FeaturesArray::new();
        let mut call_count = 0usize;
        fa.init(|req, _params| {
            call_count += 1;
            match req {
                0x0000 => Some(vec![0x01, 0x00, 0x00, 0x00]),
                0x0100 => Some(vec![0x02, 0x00, 0x00, 0x00]),
                _ => None,
            }
        });
        let first_calls = call_count;
        let result2 = fa.init(|_req, _params| {
            call_count += 1;
            None
        });
        // Second call must not query the device again.
        assert!(result2);
        assert_eq!(
            call_count, first_calls,
            "init should not make requests when already initialised"
        );
    }

    #[test]
    fn features_array_not_initialised_when_rejected() {
        // Device rejected (supported = Some(false)) → second call returns false immediately.
        let mut fa = FeaturesArray::new();
        fa.init(|_req, _params| None); // sets supported = Some(false)
        let mut called = false;
        let result = fa.init(|_req, _params| {
            called = true;
            Some(vec![0x01, 0x00, 0x00, 0x00])
        });
        assert!(!result);
        assert!(
            !called,
            "should not call request_fn when already known unsupported"
        );
    }
}
