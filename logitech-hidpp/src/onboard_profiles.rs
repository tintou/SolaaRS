// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Onboard Profiles support (HID++ 2.0 feature 0x8100).
//!
//! Mirrors the Python `OnboardProfiles` / `OnboardProfile` / `Button` /
//! `LEDEffectSetting` classes from `logitech_receiver/hidpp20.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::hidpp20::Hidpp20Device;
use crate::hidpp20_constants::SupportedFeature;

/// Call `feature_request` and unwrap the `Option`, returning `Protocol` error if `None`.
macro_rules! freq {
    ($dev:expr, $feat:expr, $func:expr, $params:expr) => {
        $dev.feature_request($feat, $func, $params)?
            .ok_or_else(|| {
                Error::Protocol(format!(
                    "no reply for feature {:?} fn {:#04x}",
                    $feat, $func
                ))
            })?
    };
}

// ── LED effects ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedEffect {
    Disabled,
    Static {
        red: u8,
        green: u8,
        blue: u8,
        ramp: u8,
    },
    Pulse {
        red: u8,
        green: u8,
        blue: u8,
        speed: u8,
    },
    Cycle {
        period: u16,
        intensity: u8,
    },
    Boot,
    Demo,
    Breathe {
        red: u8,
        green: u8,
        blue: u8,
        period: u16,
        form: u8,
        intensity: u8,
    },
    Ripple {
        red: u8,
        green: u8,
        blue: u8,
        period: u16,
    },
    Decomposition {
        period: u16,
        intensity: u8,
    },
    Signature1 {
        period: u16,
        intensity: u8,
    },
    Signature2 {
        period: u16,
        intensity: u8,
    },
    CycleS {
        saturation: u8,
        period: u16,
        intensity: u8,
    },
    Unknown {
        id: u8,
        raw: String,
    },
}

impl LedEffect {
    /// Parse 11 raw bytes from a profile sector into an [`LedEffect`].
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 11 {
            return Self::Unknown {
                id: 0xFF,
                raw: hex::encode(bytes),
            };
        }
        let id = bytes[0];
        let b = &bytes[1..];
        match id {
            0x00 => Self::Disabled,
            0x01 => Self::Static {
                red: b[0],
                green: b[1],
                blue: b[2],
                ramp: b[3],
            },
            0x02 => Self::Pulse {
                red: b[0],
                green: b[1],
                blue: b[2],
                speed: b[3],
            },
            0x03 => Self::Cycle {
                period: u16::from_le_bytes([b[5], b[6]]),
                intensity: b[7],
            },
            0x08 => Self::Boot,
            0x09 => Self::Demo,
            0x0A => Self::Breathe {
                red: b[0],
                green: b[1],
                blue: b[2],
                period: u16::from_le_bytes([b[3], b[4]]),
                form: b[5],
                intensity: b[6],
            },
            0x0B => Self::Ripple {
                red: b[0],
                green: b[1],
                blue: b[2],
                period: u16::from_le_bytes([b[4], b[5]]),
            },
            0x0E => Self::Decomposition {
                period: u16::from_le_bytes([b[6], b[7]]),
                intensity: b[8],
            },
            0x0F => Self::Signature1 {
                period: u16::from_le_bytes([b[5], b[6]]),
                intensity: b[7],
            },
            0x10 => Self::Signature2 {
                period: u16::from_le_bytes([b[5], b[6]]),
                intensity: b[7],
            },
            0x15 => Self::CycleS {
                saturation: b[1],
                period: u16::from_le_bytes([b[6], b[7]]),
                intensity: b[8],
            },
            _ => Self::Unknown {
                id,
                raw: hex::encode(&bytes[1..]),
            },
        }
    }

    /// Serialise to 11 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = vec![0u8; 10];
        let id: u8;
        match self {
            Self::Disabled => {
                return vec![0x00; 11];
            }
            Self::Static {
                red,
                green,
                blue,
                ramp,
            } => {
                id = 0x01;
                b[0] = *red;
                b[1] = *green;
                b[2] = *blue;
                b[3] = *ramp;
            }
            Self::Pulse {
                red,
                green,
                blue,
                speed,
            } => {
                id = 0x02;
                b[0] = *red;
                b[1] = *green;
                b[2] = *blue;
                b[3] = *speed;
            }
            Self::Cycle { period, intensity } => {
                id = 0x03;
                let p = period.to_le_bytes();
                b[5] = p[0];
                b[6] = p[1];
                b[7] = *intensity;
            }
            Self::Boot => {
                id = 0x08;
            }
            Self::Demo => {
                id = 0x09;
            }
            Self::Breathe {
                red,
                green,
                blue,
                period,
                form,
                intensity,
            } => {
                id = 0x0A;
                b[0] = *red;
                b[1] = *green;
                b[2] = *blue;
                let p = period.to_le_bytes();
                b[3] = p[0];
                b[4] = p[1];
                b[5] = *form;
                b[6] = *intensity;
            }
            Self::Ripple {
                red,
                green,
                blue,
                period,
            } => {
                id = 0x0B;
                b[0] = *red;
                b[1] = *green;
                b[2] = *blue;
                let p = period.to_le_bytes();
                b[4] = p[0];
                b[5] = p[1];
            }
            Self::Decomposition { period, intensity } => {
                id = 0x0E;
                let p = period.to_le_bytes();
                b[6] = p[0];
                b[7] = p[1];
                b[8] = *intensity;
            }
            Self::Signature1 { period, intensity } => {
                id = 0x0F;
                let p = period.to_le_bytes();
                b[5] = p[0];
                b[6] = p[1];
                b[7] = *intensity;
            }
            Self::Signature2 { period, intensity } => {
                id = 0x10;
                let p = period.to_le_bytes();
                b[5] = p[0];
                b[6] = p[1];
                b[7] = *intensity;
            }
            Self::CycleS {
                saturation,
                period,
                intensity,
            } => {
                id = 0x15;
                b[1] = *saturation;
                let p = period.to_le_bytes();
                b[6] = p[0];
                b[7] = p[1];
                b[8] = *intensity;
            }
            Self::Unknown { id: uid, raw } => {
                let mut out = vec![*uid];
                let decoded = hex::decode(raw).unwrap_or_else(|_| vec![0xFF; 10]);
                out.extend_from_slice(&decoded[..10.min(decoded.len())]);
                out.resize(11, 0xFF);
                return out;
            }
        }
        let mut out = vec![id];
        out.extend_from_slice(&b);
        out
    }
}

// ── Button mapping ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
pub enum Button {
    Macro {
        sector: u16,
        address: u16,
    },
    MacroStop {
        sector: u16,
        address: u16,
    },
    /// Send a HID event.
    Send {
        /// 0=no_action, 1=button, 2=modifier_and_key, 3=consumer_key
        mapping_type: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        modifiers: Option<u8>,
        /// Mouse button / key / consumer key code, or 0 for no_action.
        value: u32,
    },
    /// Device function (DPI, profile switch, etc.)
    Function {
        function: u8,
        data: u8,
    },
    /// Unrecognised button – stored as hex so round-tripping is lossless.
    Unknown {
        raw: String,
    },
    /// Disabled / no mapping.
    None,
}

impl Button {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 4 {
            return Self::Unknown {
                raw: hex::encode(bytes),
            };
        }
        // All-0xFF means disabled.
        if bytes == [0xFF, 0xFF, 0xFF, 0xFF] {
            return Self::None;
        }
        let behavior = bytes[0] >> 4;
        match behavior {
            0x0 | 0x1 => {
                // MacroExecute (0) / MacroStop (1)
                let sector = ((bytes[0] as u16 & 0x0F) << 8) | bytes[1] as u16;
                let address = ((bytes[2] as u16) << 8) | bytes[3] as u16;
                if behavior == 0 {
                    Self::Macro { sector, address }
                } else {
                    Self::MacroStop { sector, address }
                }
            }
            0x8 => {
                // Send
                let mapping_type = bytes[1];
                match mapping_type {
                    0 => Self::Send {
                        mapping_type,
                        modifiers: None,
                        value: 0,
                    },
                    1 | 3 => {
                        let value = ((bytes[2] as u32) << 8) | bytes[3] as u32;
                        Self::Send {
                            mapping_type,
                            modifiers: None,
                            value,
                        }
                    }
                    2 => {
                        let modifiers = bytes[2];
                        let value = bytes[3] as u32;
                        Self::Send {
                            mapping_type,
                            modifiers: Some(modifiers),
                            value,
                        }
                    }
                    _ => Self::Unknown {
                        raw: hex::encode(bytes),
                    },
                }
            }
            0x9 => {
                // Function
                let function = bytes[1];
                let data = bytes[3];
                Self::Function { function, data }
            }
            _ => Self::Unknown {
                raw: hex::encode(bytes),
            },
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::None => vec![0xFF, 0xFF, 0xFF, 0xFF],
            Self::Macro { sector, address } => {
                let b0 = (*sector >> 8) as u8 & 0x0F;
                let b1 = (*sector & 0xFF) as u8;
                vec![b0, b1, (*address >> 8) as u8, (*address & 0xFF) as u8]
            }
            Self::MacroStop { sector, address } => {
                let b0 = (0x01u8 << 4) | ((*sector >> 8) as u8 & 0x0F);
                let b1 = (*sector & 0xFF) as u8;
                vec![b0, b1, (*address >> 8) as u8, (*address & 0xFF) as u8]
            }
            Self::Send {
                mapping_type,
                modifiers,
                value,
            } => {
                let b0 = 0x80u8;
                match *mapping_type {
                    0 => vec![b0, 0, 0xFF, 0xFF],
                    1 | 3 => vec![
                        b0,
                        *mapping_type,
                        (*value >> 8) as u8,
                        (*value & 0xFF) as u8,
                    ],
                    2 => vec![
                        b0,
                        *mapping_type,
                        modifiers.unwrap_or(0),
                        (*value & 0xFF) as u8,
                    ],
                    _ => vec![0xFF; 4],
                }
            }
            Self::Function { function, data } => {
                vec![0x90, *function, 0xFF, *data]
            }
            Self::Unknown { raw } => {
                let decoded = hex::decode(raw).unwrap_or_else(|_| vec![0xFF; 4]);
                let mut out = decoded;
                out.resize(4, 0xFF);
                out
            }
        }
    }
}

// ── OnboardProfile ─────────────────────────────────────────────────────────────

/// A single onboard profile stored on the device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardProfile {
    /// Flash sector index where this profile lives.
    pub sector: u16,
    /// Whether this profile is enabled (1) or disabled (0).
    pub enabled: u8,
    /// Report rate in milliseconds (1 = 1000 Hz, 4 = 250 Hz, …).
    pub report_rate: u8,
    pub resolution_default_index: u8,
    pub resolution_shift_index: u8,
    /// DPI resolution presets (up to 5), little-endian u16 each.
    pub resolutions: Vec<u16>,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub power_mode: u8,
    pub angle_snap: u8,
    pub write_count: u16,
    pub ps_timeout: u16,
    pub po_timeout: u16,
    pub buttons: Vec<Button>,
    pub gbuttons: Vec<Button>,
    pub name: String,
    pub lighting: Vec<LedEffect>,
}

impl OnboardProfile {
    pub fn from_bytes(
        sector: u16,
        enabled: u8,
        buttons: usize,
        gbuttons: usize,
        data: &[u8],
    ) -> Self {
        let resolutions = (0..5)
            .map(|i| u16::from_le_bytes([data[3 + i * 2], data[4 + i * 2]]))
            .collect();

        let write_count = u16::from_le_bytes([data[18], data[19]]);
        let ps_timeout = u16::from_le_bytes([data[28], data[29]]);
        let po_timeout = u16::from_le_bytes([data[30], data[31]]);

        let btn_list = (0..buttons)
            .map(|i| Button::from_bytes(&data[32 + i * 4..32 + i * 4 + 4]))
            .collect();
        let gbtn_list = (0..gbuttons)
            .map(|i| Button::from_bytes(&data[96 + i * 4..96 + i * 4 + 4]))
            .collect();

        let name_bytes = &data[160..208];
        let name = decode_utf16le_null(name_bytes);

        let lighting = (0..4)
            .map(|i| LedEffect::from_bytes(&data[208 + i * 11..219 + i * 11]))
            .collect();

        Self {
            sector,
            enabled,
            report_rate: data[0],
            resolution_default_index: data[1],
            resolution_shift_index: data[2],
            resolutions,
            red: data[13],
            green: data[14],
            blue: data[15],
            power_mode: data[16],
            angle_snap: data[17],
            write_count,
            ps_timeout,
            po_timeout,
            buttons: btn_list,
            gbuttons: gbtn_list,
            name,
            lighting,
        }
    }

    /// Serialise to `length` bytes (including a 2-byte CRC at the end).
    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(length);

        out.push(self.report_rate);
        out.push(self.resolution_default_index);
        out.push(self.resolution_shift_index);
        for &r in &self.resolutions {
            out.extend_from_slice(&r.to_le_bytes());
        }
        // Pad resolutions to 5 entries.
        for _ in self.resolutions.len()..5 {
            out.extend_from_slice(&0u16.to_le_bytes());
        }
        out.push(self.red);
        out.push(self.green);
        out.push(self.blue);
        out.push(self.power_mode);
        out.push(self.angle_snap);
        out.extend_from_slice(&self.write_count.to_le_bytes());
        // reserved bytes 20-27
        out.extend_from_slice(&[0xFF; 8]);
        out.extend_from_slice(&self.ps_timeout.to_le_bytes());
        out.extend_from_slice(&self.po_timeout.to_le_bytes());

        // Buttons (16 slots × 4 bytes = 64 bytes, starting at offset 32).
        for i in 0..16 {
            if i < self.buttons.len() {
                out.extend_from_slice(&self.buttons[i].to_bytes());
            } else {
                out.extend_from_slice(&[0xFF; 4]);
            }
        }
        // G-buttons (16 slots × 4 bytes = 64 bytes, starting at offset 96).
        for i in 0..16 {
            if i < self.gbuttons.len() {
                out.extend_from_slice(&self.gbuttons[i].to_bytes());
            } else {
                out.extend_from_slice(&[0xFF; 4]);
            }
        }

        // Name: 48 bytes as UTF-16LE (24 chars max), or 0xFF padding.
        if self.name.is_empty() {
            out.extend_from_slice(&[0xFF; 48]);
        } else {
            let encoded: Vec<u8> = self
                .name
                .chars()
                .take(24)
                .flat_map(|c| {
                    let mut buf = [0u16; 2];
                    let s = c.encode_utf16(&mut buf);
                    s[0].to_le_bytes().to_vec()
                })
                .collect();
            out.extend_from_slice(&encoded);
            // Pad to 48 bytes with null.
            let needed = 48usize.saturating_sub(encoded.len());
            out.extend(std::iter::repeat_n(0u8, needed));
        }

        // 4 LED effect settings × 11 bytes = 44 bytes.
        for i in 0..4 {
            if i < self.lighting.len() {
                out.extend_from_slice(&self.lighting[i].to_bytes());
            } else {
                out.extend_from_slice(&[0xFF; 11]);
            }
        }

        // Pad with 0xFF up to length - 2, then append CRC-16.
        while out.len() < length - 2 {
            out.push(0xFF);
        }
        let crc = crc16(&out);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }
}

// ── OnboardProfiles ───────────────────────────────────────────────────────────

/// Schema version written to the YAML dump so we can reject stale files.
pub const ONBOARD_PROFILES_VERSION: u32 = 3;

/// The complete onboard profiles configuration for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardProfiles {
    pub version: u32,
    pub device_name: String,
    pub count: u8,
    pub buttons: u8,
    pub gbuttons: u8,
    pub sectors: u8,
    /// Sector size in bytes (includes the 2-byte CRC).
    pub size: u16,
    /// Profile index (1-based) → profile data.
    pub profiles: HashMap<u8, OnboardProfile>,
}

impl OnboardProfiles {
    /// Read the full profile set from a live device.
    pub fn from_device<D: Hidpp20Device>(
        device: &D,
        device_name: &str,
    ) -> Result<Option<Self>, Error> {
        let response = freq!(device, SupportedFeature::OnboardProfiles, 0x00, &[]);
        let memory = response[0];
        let profile_type = response[1];

        if memory != 0x01 || profile_type > 0x05 {
            return Ok(None);
        }

        let count = response[3];
        let buttons = response[5];
        let sectors = response[6];
        let size = u16::from_be_bytes([response[7], response[8]]);
        let shift = response[9];
        let gbuttons = if shift & 0x3 == 0x2 { buttons } else { 0 };

        let headers = Self::get_profile_headers(device)?;
        let mut profiles = HashMap::new();
        for (idx, (sector, enabled)) in headers.into_iter().enumerate() {
            let data = Self::read_sector(device, sector, size as usize)?;
            let profile = OnboardProfile::from_bytes(
                sector,
                enabled,
                buttons as usize,
                gbuttons as usize,
                &data,
            );
            profiles.insert((idx + 1) as u8, profile);
        }

        Ok(Some(Self {
            version: ONBOARD_PROFILES_VERSION,
            device_name: device_name.to_string(),
            count,
            buttons,
            gbuttons,
            sectors,
            size,
            profiles,
        }))
    }

    /// Write profiles back to the device. Returns the number of sectors written.
    pub fn write<D: Hidpp20Device>(&self, device: &D) -> Result<usize, Error> {
        let mut written = 0;

        // Write the control sector (sector 0) first.
        let ctrl_bytes = self.control_sector_bytes();
        if Self::write_sector(device, 0, &ctrl_bytes)? {
            written += 1;
        }

        // Write each profile sector.
        let mut indices: Vec<u8> = self.profiles.keys().copied().collect();
        indices.sort();
        for idx in indices {
            let p = &self.profiles[&idx];
            if p.sector as usize >= self.sectors as usize {
                return Err(Error::Protocol(format!(
                    "sector {} is not writable",
                    p.sector
                )));
            }
            let profile_bytes = p.to_bytes(self.size as usize);
            if Self::write_sector(device, p.sector, &profile_bytes)? {
                written += 1;
            }
        }

        Ok(written)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn get_profile_headers<D: Hidpp20Device>(device: &D) -> Result<Vec<(u16, u8)>, Error> {
        let mut headers = Vec::new();
        let mut i: u16 = 0;
        let mut sector_src = 0x00u8;

        let first_chunk = freq!(
            device,
            SupportedFeature::OnboardProfiles,
            0x50,
            &[sector_src, 0, 0, 0]
        );

        // If first 4 bytes are all 0x00 or 0xFF, look in ROM instead.
        if first_chunk[0..4] == [0x00; 4] || first_chunk[0..4] == [0xFF; 4] {
            sector_src = 0x01;
        }

        loop {
            let chunk = freq!(
                device,
                SupportedFeature::OnboardProfiles,
                0x50,
                &[sector_src, 0, 0, (i * 4) as u8]
            );

            if chunk[0..2] == [0xFF, 0xFF] {
                break;
            }

            let sector = u16::from_be_bytes([chunk[0], chunk[1]]);
            let enabled = chunk[2];
            headers.push((sector, enabled));
            i += 1;
        }

        Ok(headers)
    }

    fn read_sector<D: Hidpp20Device>(
        device: &D,
        sector: u16,
        size: usize,
    ) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();
        let mut offset: usize = 0;

        while offset < size.saturating_sub(15) {
            let chunk = freq!(
                device,
                SupportedFeature::OnboardProfiles,
                0x50,
                &[
                    (sector >> 8) as u8,
                    (sector & 0xFF) as u8,
                    (offset >> 8) as u8,
                    (offset & 0xFF) as u8,
                ]
            );
            data.extend_from_slice(&chunk);
            offset += 16;
        }

        // Read the last (possibly overlapping) chunk.
        let last_offset = size.saturating_sub(16);
        let last_chunk = freq!(
            device,
            SupportedFeature::OnboardProfiles,
            0x50,
            &[
                (sector >> 8) as u8,
                (sector & 0xFF) as u8,
                (last_offset >> 8) as u8,
                (last_offset & 0xFF) as u8,
            ]
        );
        // Keep only the bytes that fill out `size`.
        let already_have = data.len();
        let need = size.saturating_sub(already_have);
        let skip = 16usize.saturating_sub(need);
        data.extend_from_slice(&last_chunk[skip..skip + need.min(last_chunk.len())]);

        Ok(data)
    }

    /// Returns `true` if the sector was actually written (content differed).
    fn write_sector<D: Hidpp20Device>(device: &D, sector: u16, data: &[u8]) -> Result<bool, Error> {
        // Skip writing if the stored content matches.
        let current = Self::read_sector(device, sector, data.len())?;
        if current.len() >= data.len() - 2 && current[..data.len() - 2] == data[..data.len() - 2] {
            return Ok(false);
        }

        // Begin erase + write.
        freq!(
            device,
            SupportedFeature::OnboardProfiles,
            0x60,
            &[
                (sector >> 8) as u8,
                (sector & 0xFF) as u8,
                0,
                0,
                (data.len() >> 8) as u8,
                (data.len() & 0xFF) as u8,
            ]
        );

        let mut offset = 0;
        while offset < data.len() {
            let end = (offset + 16).min(data.len());
            let mut chunk = data[offset..end].to_vec();
            chunk.resize(16, 0xFF);
            freq!(device, SupportedFeature::OnboardProfiles, 0x70, &chunk);
            offset += 16;
        }

        freq!(device, SupportedFeature::OnboardProfiles, 0x80, &[]);
        Ok(true)
    }

    /// Build the control sector 0 bytes (profile directory + CRC).
    fn control_sector_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut indices: Vec<u8> = self.profiles.keys().copied().collect();
        indices.sort();
        for idx in &indices {
            let p = &self.profiles[idx];
            out.extend_from_slice(&p.sector.to_be_bytes());
            out.push(p.enabled);
            out.push(0x00);
        }
        // End marker.
        out.extend_from_slice(&[0xFF, 0xFF, 0x00, 0x00]);
        while out.len() < self.size as usize - 2 {
            out.push(0xFF);
        }
        let crc = crc16(&out);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }
}

// ── Utility ───────────────────────────────────────────────────────────────────

/// Decode a UTF-16LE byte slice, stopping at the first null or 0xFFFF codeunit.
fn decode_utf16le_null(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0x0000 && u != 0xFFFF)
        .collect();
    String::from_utf16_lossy(&units).to_string()
}

/// CRC-16/CCITT-FALSE used by the device firmware.
pub(crate) fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── crc16 ────────────────────────────────────────────────────────────────

    /// Known-answer test for CRC-16/CCITT-FALSE.
    /// The string "123456789" has a well-known CRC of 0x29B1.
    #[test]
    fn crc16_known_answer() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn crc16_empty() {
        // CRC of empty slice is the initial value 0xFFFF.
        assert_eq!(crc16(&[]), 0xFFFF);
    }

    #[test]
    fn crc16_single_zero_byte() {
        assert_eq!(crc16(&[0x00]), 0xE1F0);
    }

    // ── LedEffect round-trips ────────────────────────────────────────────────

    fn led_roundtrip(effect: LedEffect) {
        let bytes = effect.to_bytes();
        assert_eq!(bytes.len(), 11, "LED effect must always be 11 bytes");
        let parsed = LedEffect::from_bytes(&bytes);
        assert_eq!(parsed, effect, "round-trip mismatch for {effect:?}");
    }

    #[test]
    fn led_effect_disabled_roundtrip() {
        led_roundtrip(LedEffect::Disabled);
    }

    #[test]
    fn led_effect_static_roundtrip() {
        led_roundtrip(LedEffect::Static {
            red: 0xFF,
            green: 0x80,
            blue: 0x00,
            ramp: 0x01,
        });
    }

    #[test]
    fn led_effect_pulse_roundtrip() {
        led_roundtrip(LedEffect::Pulse {
            red: 0x10,
            green: 0x20,
            blue: 0x30,
            speed: 0x04,
        });
    }

    #[test]
    fn led_effect_cycle_roundtrip() {
        led_roundtrip(LedEffect::Cycle {
            period: 1000,
            intensity: 0xC0,
        });
    }

    #[test]
    fn led_effect_boot_roundtrip() {
        led_roundtrip(LedEffect::Boot);
    }

    #[test]
    fn led_effect_demo_roundtrip() {
        led_roundtrip(LedEffect::Demo);
    }

    #[test]
    fn led_effect_breathe_roundtrip() {
        led_roundtrip(LedEffect::Breathe {
            red: 0xAA,
            green: 0xBB,
            blue: 0xCC,
            period: 2000,
            form: 0x01,
            intensity: 0x80,
        });
    }

    #[test]
    fn led_effect_ripple_roundtrip() {
        led_roundtrip(LedEffect::Ripple {
            red: 0x11,
            green: 0x22,
            blue: 0x33,
            period: 500,
        });
    }

    #[test]
    fn led_effect_decomposition_roundtrip() {
        led_roundtrip(LedEffect::Decomposition {
            period: 3000,
            intensity: 0x7F,
        });
    }

    #[test]
    fn led_effect_signature1_roundtrip() {
        led_roundtrip(LedEffect::Signature1 {
            period: 1500,
            intensity: 0xFF,
        });
    }

    #[test]
    fn led_effect_signature2_roundtrip() {
        led_roundtrip(LedEffect::Signature2 {
            period: 750,
            intensity: 0x40,
        });
    }

    #[test]
    fn led_effect_cycles_roundtrip() {
        led_roundtrip(LedEffect::CycleS {
            saturation: 0x80,
            period: 4000,
            intensity: 0x60,
        });
    }

    #[test]
    fn led_effect_from_bytes_too_short_is_unknown() {
        let bytes = [0x01u8, 0x02]; // fewer than 11 bytes
        let effect = LedEffect::from_bytes(&bytes);
        assert!(matches!(effect, LedEffect::Unknown { .. }));
    }

    // ── Button round-trips ───────────────────────────────────────────────────

    fn button_roundtrip(btn: Button) {
        let bytes = btn.to_bytes();
        assert_eq!(bytes.len(), 4, "Button must always serialize to 4 bytes");
        let parsed = Button::from_bytes(&bytes);
        assert_eq!(parsed, btn, "round-trip mismatch for {btn:?}");
    }

    #[test]
    fn button_none_roundtrip() {
        button_roundtrip(Button::None);
    }

    #[test]
    fn button_macro_roundtrip() {
        button_roundtrip(Button::Macro {
            sector: 0x01,
            address: 0x0042,
        });
    }

    #[test]
    fn button_macro_stop_roundtrip() {
        button_roundtrip(Button::MacroStop {
            sector: 0x02,
            address: 0x00FF,
        });
    }

    #[test]
    fn button_send_no_action_roundtrip() {
        button_roundtrip(Button::Send {
            mapping_type: 0,
            modifiers: None,
            value: 0,
        });
    }

    #[test]
    fn button_send_button_roundtrip() {
        button_roundtrip(Button::Send {
            mapping_type: 1,
            modifiers: None,
            value: 0x0001,
        });
    }

    #[test]
    fn button_send_modifier_key_roundtrip() {
        button_roundtrip(Button::Send {
            mapping_type: 2,
            modifiers: Some(0x04),
            value: 0x04,
        });
    }

    #[test]
    fn button_send_consumer_key_roundtrip() {
        button_roundtrip(Button::Send {
            mapping_type: 3,
            modifiers: None,
            value: 0x00E2,
        });
    }

    #[test]
    fn button_function_roundtrip() {
        button_roundtrip(Button::Function {
            function: 0x03,
            data: 0x01,
        });
    }

    #[test]
    fn button_from_bytes_too_short_is_unknown() {
        let bytes = [0xAAu8, 0xBB];
        let btn = Button::from_bytes(&bytes);
        assert!(matches!(btn, Button::Unknown { .. }));
    }
}
