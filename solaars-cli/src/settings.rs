// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Device settings catalogue and read/write helpers.
//!
//! Mirrors the Python `settings_templates.py` / `settings.py` architecture,
//! but restricted to simple (toggle / range / choice) settings that do not
//! require complex custom validators.

use logitech_hidpp::device::Device;
use logitech_hidpp::hidpp10;
use logitech_hidpp::hidpp10_constants::Register;
use logitech_hidpp::hidpp20_constants::SupportedFeature;

// ─────────────────────────────
// Descriptor types
// ────────────────────────────────────────────────────────

/// How to decode/encode a setting value.
#[derive(Debug, Clone)]
pub enum Validator {
    /// Single boolean value, possibly sharing bits with other settings.
    Toggle {
        true_value: Vec<u8>,
        false_value: Vec<u8>,
        /// Bitmask: only these bits carry the value.
        mask: Vec<u8>,
        /// Skip this many bytes at the start of the device reply before reading.
        read_skip: usize,
        /// Prepend these bytes when writing.
        write_prefix: Vec<u8>,
    },
    /// Integer in a range.
    Range {
        min: i64,
        max: i64,
        byte_count: usize,
        read_skip: usize,
        write_prefix: Vec<u8>,
    },
    /// One of several named integer values.
    Choice {
        options: Vec<(i64, &'static str)>,
        byte_count: usize,
        read_skip: usize,
        write_prefix: Vec<u8>,
    },
}

/// Where the setting lives on the device.
#[derive(Debug, Clone)]
pub enum Source {
    /// HID++ 2.0 feature.
    Feature {
        feature: SupportedFeature,
        /// Function byte for the read request (e.g. `0x00`, `0x10`, `0x20`, …).
        read_fnid: u8,
        /// Function byte for the write request.
        write_fnid: u8,
        /// Prepend to the read and write request payloads.
        prefix: &'static [u8],
        /// Append to the write request payload.
        suffix: &'static [u8],
    },
    /// HID++ 1.0 register (only meaningful if device.protocol < 2.0).
    Register { register: Register },
}

/// Static description of one device setting.
pub struct SettingDef {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub source: Source,
    pub validator: Validator,
}

// ────────────────────────────────────
// Runtime value
// ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SettingValue {
    Toggle(bool),
    Range(i64),
    Choice(String),
}

impl std::fmt::Display for SettingValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Toggle(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Self::Range(n) => write!(f, "{n}"),
            Self::Choice(s) => write!(f, "{s}"),
        }
    }
}

// ────────────────────────────────────────────────
// Settings catalogue
// ─────────────────────────────────────────────────────────────────────────

pub fn all_settings() -> Vec<SettingDef> {
    vec![
        // ── Fn-key swap ───────────────────────────────────────────
        SettingDef {
            name: "fn-swap",
            label: "Swap Fx function",
            description: "When set, the F1..F12 keys activate their special function and FN activates standard.",
            source: Source::Feature {
                feature: SupportedFeature::FnInversion,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x01],
                false_value: vec![0x00],
                mask: vec![0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "fn-swap",
            label: "Swap Fx function (new)",
            description: "When set, the F1..F12 keys activate their special function and FN activates standard.",
            source: Source::Feature {
                feature: SupportedFeature::NewFnInversion,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x01],
                false_value: vec![0x00],
                mask: vec![0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        // ── High-resolution scrolling ─────────────────────────────────────────
        SettingDef {
            name: "hi-res-scroll",
            label: "Scroll Wheel High Resolution",
            description: "High-sensitivity mode for vertical scroll with the wheel.",
            source: Source::Feature {
                feature: SupportedFeature::HiResScrolling,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x01],
                false_value: vec![0x00],
                mask: vec![0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "lowres-scroll-mode",
            label: "Scroll Wheel Diversion (low-res)",
            description: "Make scroll wheel send LOWRES_WHEEL HID++ notifications.",
            source: Source::Feature {
                feature: SupportedFeature::LowresWheel,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x01],
                false_value: vec![0x00],
                mask: vec![0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "hires-smooth-invert",
            label: "Scroll Wheel Direction",
            description: "Invert direction for vertical scroll with wheel.",
            source: Source::Feature {
                feature: SupportedFeature::HiresWheel,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x04],
                false_value: vec![0x00],
                mask: vec![0x04],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "hires-smooth-resolution",
            label: "Scroll Wheel Resolution",
            description: "High-sensitivity mode for vertical scroll with the wheel.",
            source: Source::Feature {
                feature: SupportedFeature::HiresWheel,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x02],
                false_value: vec![0x00],
                mask: vec![0x02],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "hires-scroll-mode",
            label: "Scroll Wheel Diversion (hi-res)",
            description: "Make scroll wheel send HIRES_WHEEL HID++ notifications.",
            source: Source::Feature {
                feature: SupportedFeature::HiresWheel,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x01],
                false_value: vec![0x00],
                mask: vec![0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        // ── Thumb wheel ───────────────────────────────────────────────────────
        SettingDef {
            name: "thumb-scroll-mode",
            label: "Thumb Wheel Diversion",
            description: "Make thumb wheel send THUMB_WHEEL HID++ notifications.",
            source: Source::Feature {
                feature: SupportedFeature::ThumbWheel,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x01, 0x00],
                false_value: vec![0x00, 0x00],
                mask: vec![0x01, 0x00],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "thumb-scroll-invert",
            label: "Thumb Wheel Direction",
            description: "Invert thumb wheel scroll direction.",
            source: Source::Feature {
                feature: SupportedFeature::ThumbWheel,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Toggle {
                true_value: vec![0x00, 0x01],
                false_value: vec![0x00, 0x00],
                mask: vec![0x00, 0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        // ── Smart shift ratchet / ────────────────────────────────
        SettingDef {
            name: "scroll-ratchet",
            label: "Scroll Wheel Ratcheted",
            description: "Switch the mouse wheel between speed-controlled ratcheting and always freespin.",
            source: Source::Feature {
                feature: SupportedFeature::SmartShift,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Choice {
                options: vec![(1, "Freespinning"), (2, "Ratcheted")],
                byte_count: 1,
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "smart-shift",
            label: "Scroll Wheel Ratchet Speed",
            description: "Use the mouse wheel speed to switch between ratcheted and freespinning (1-50).",
            source: Source::Feature {
                feature: SupportedFeature::SmartShift,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            // Smart-shift uses byte 1 for the threshold; byte 0 is the mode.
            validator: Validator::Range {
                min: 1,
                max: 50,
                byte_count: 1,
                read_skip: 1,
                write_prefix: vec![0x02], // mode = smart-shift
            },
        },
        SettingDef {
            name: "scroll-ratchet-enhanced",
            label: "Scroll Wheel Ratcheted (Enhanced Smart Shift)",
            description: "Switch the mouse wheel between speed-controlled ratcheting and always freespin.",
            source: Source::Feature {
                feature: SupportedFeature::SmartShiftEnhanced,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Choice {
                options: vec![(1, "Freespinning"), (2, "Ratcheted")],
                byte_count: 1,
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "smart-shift-enhanced",
            label: "Scroll Wheel Ratchet Speed (Enhanced)",
            description: "Use the mouse wheel speed to switch between ratcheted and freespinning (1-50).",
            source: Source::Feature {
                feature: SupportedFeature::SmartShiftEnhanced,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Range {
                min: 1,
                max: 50,
                byte_count: 1,
                read_skip: 1,
                write_prefix: vec![0x02],
            },
        },
        // ── speed Pointer ──────────────────────────────────────────────
        SettingDef {
            name: "pointer-speed",
            label: "Sensitivity (Pointer Speed)",
            description: "Speed multiplier for mouse (256 = normal). Range: 46–511.",
            source: Source::Feature {
                feature: SupportedFeature::PointerSpeed,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Range {
                min: 0x002E,
                max: 0x01FF,
                byte_count: 2,
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        // ── DPI ───────────────────────────────────────────────────────────────
        SettingDef {
            name: "dpi",
            label: "Sensitivity (DPI)",
            description: "Mouse movement sensitivity. May need Onboard Profiles disabled to take effect.",
            source: Source::Feature {
                feature: SupportedFeature::AdjustableDpi,
                read_fnid: 0x20,
                write_fnid: 0x30,
                prefix: &[],
                suffix: &[],
            },
            // DPI is a 2-byte big-endian value; byte 0 is the sensor index prefix.
            validator: Validator::Range {
                min: 100,
                max: 25600,
                byte_count: 2,
                read_skip: 1, // byte 0 = sensor index; bytes 1-2 = current DPI
                write_prefix: vec![0x00], // sensor index 0
            },
        },
        // ── Audio ─────────────────────────────────────────────────────────────
        SettingDef {
            name: "sidetone",
            label: "Sidetone",
            description: "Microphone sidetone level (0–100).",
            source: Source::Feature {
                feature: SupportedFeature::Sidetone,
                read_fnid: 0x00,
                write_fnid: 0x10,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Range {
                min: 0,
                max: 100,
                byte_count: 1,
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        // ── Report rate ───────────────────────────────────────────────────────
        SettingDef {
            name: "report-rate",
            label: "Report Rate",
            description: "USB report rate in ms (1 = 1000 Hz, 2 = 500 Hz, …, 8 = 125 Hz).",
            source: Source::Feature {
                feature: SupportedFeature::ReportRate,
                read_fnid: 0x10,
                write_fnid: 0x20,
                prefix: &[],
                suffix: &[],
            },
            validator: Validator::Choice {
                options: vec![
                    (1, "1ms (1000 Hz)"),
                    (2, "2ms (500 Hz)"),
                    (4, "4ms (250 Hz)"),
                    (8, "8ms (125 Hz)"),
                ],
                byte_count: 1,
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        // ── HID++ 1.0 register-based settings (protocol < 2.0) ────────────────
        SettingDef {
            name: "smooth-scroll",
            label: "Scroll Wheel Smooth Scrolling",
            description: "High-sensitivity mode for vertical scroll with the wheel.",
            source: Source::Register { register: Register::MouseButtonFlags },
            validator: Validator::Toggle {
                true_value: vec![0x40],
                false_value: vec![0x00],
                mask: vec![0x40],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "side-scroll",
            label: "Side Scrolling",
            description: "When disabled, pushing the wheel sideways sends custom button events.",
            source: Source::Register { register: Register::MouseButtonFlags },
            validator: Validator::Toggle {
                true_value: vec![0x02],
                false_value: vec![0x00],
                mask: vec![0x02],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "hand-detection",
            label: "Hand Detection",
            description: "Turn on illumination when the hands hover over the keyboard.",
            source: Source::Register { register: Register::MouseButtonFlags },
            validator: Validator::Toggle {
                true_value: vec![0x00, 0x00, 0x00],
                false_value: vec![0x00, 0x00, 0x30],
                mask: vec![0x00, 0x00, 0xFF],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
        SettingDef {
            name: "fn-swap-old",
            label: "Swap Fx function (register-based)",
            description: "When set, the F1..F12 keys activate their special function.",
            source: Source::Register { register: Register::KeyboardFnSwap },
            validator: Validator::Toggle {
                true_value: vec![0x00, 0x01],
                false_value: vec![0x00, 0x00],
                mask: vec![0x00, 0x01],
                read_skip: 0,
                write_prefix: vec![],
            },
        },
    ]
}

// ────────────────────────────────────
// Read/write helpers
// ─────────────────────────────────────────────────────────────

/// Read the current value of a setting from the device.
///
/// Returns `None` if the feature/register is not supported by this device.
pub fn read_setting(
    device: &Device,
    def: &SettingDef,
) -> Result<Option<SettingValue>, Box<dyn std::error::Error>> {
    let raw = match &def.source {
        Source::Feature {
            feature,
            read_fnid,
            prefix,
            ..
        } => device
            .feature_request(*feature, *read_fnid, prefix, None)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?,
        Source::Register { register } => hidpp10::read_register(device, *register, &[])
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?,
    };

    let raw = match raw {
        Some(r) => r,
        None => return Ok(None), // feature/register not supported
    };

    Ok(Some(decode_value(&def.validator, &raw)?))
}

fn decode_value(
    validator: &Validator,
    raw: &[u8],
) -> Result<SettingValue, Box<dyn std::error::Error>> {
    match validator {
        Validator::Toggle {
            true_value,
            false_value,
            mask,
            read_skip,
            ..
        } => {
            let data = &raw[*read_skip..];
            let n = mask.len().min(data.len());
            let masked: Vec<u8> = (0..n).map(|i| data[i] & mask[i]).collect();
            let tv_masked: Vec<u8> = (0..n).map(|i| true_value[i] & mask[i]).collect();
            let fv_masked: Vec<u8> = (0..n).map(|i| false_value[i] & mask[i]).collect();
            if masked == tv_masked {
                Ok(SettingValue::Toggle(true))
            } else if masked == fv_masked {
                Ok(SettingValue::Toggle(false))
            } else {
                // Unrecognised bit pattern: treat as false
                Ok(SettingValue::Toggle(false))
            }
        }
        Validator::Range {
            byte_count,
            read_skip,
            ..
        } => {
            let start = *read_skip;
            let end = start + byte_count;
            if raw.len() < end {
                return Err(format!("response too short ({} < {end})", raw.len()).into());
            }
            let mut v: i64 = 0;
            for b in &raw[start..end] {
                v = (v << 8) | (*b as i64);
            }
            Ok(SettingValue::Range(v))
        }
        Validator::Choice {
            options,
            byte_count,
            read_skip,
            ..
        } => {
            let start = *read_skip;
            let end = start + byte_count;
            if raw.len() < end {
                return Err(format!("response too short ({} < {end})", raw.len()).into());
            }
            let mut v: i64 = 0;
            for b in &raw[start..end] {
                v = (v << 8) | (*b as i64);
            }
            let label = options
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, l)| l.to_string())
                .unwrap_or_else(|| format!("unknown ({v})"));
            Ok(SettingValue::Choice(label))
        }
    }
}

/// Write a new value to a setting.
///
/// `value_str` is the user-supplied string (e.g. `"true"`, `"256"`, `"Ratcheted"`).
///
/// Returns `true` if the write succeeded, `false` if the feature is not supported.
pub fn write_setting(
    device: &Device,
    def: &SettingDef,
    value_str: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Encode the new value.
    let (encoded, needs_rmw) = encode_value(&def.validator, value_str)?;

    // For register settings with shared bits (needs_rmw), read first.
    let payload = if needs_rmw {
        let current_raw = match &def.source {
            Source::Feature {
                feature,
                read_fnid,
                prefix,
                ..
            } => device
                .feature_request(*feature, *read_fnid, prefix, None)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?,
            Source::Register { register } => hidpp10::read_register(device, *register, &[])
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?,
        };
        match current_raw {
            Some(cur) => merge_with_mask(&def.validator, &cur, &encoded),
            None => return Ok(false),
        }
    } else {
        encoded
    };

    // Send the write.
    match &def.source {
        Source::Feature {
            feature,
            write_fnid,
            prefix,
            suffix,
            ..
        } => {
            let mut params = prefix.to_vec();
            params.extend_from_slice(&payload);
            params.extend_from_slice(suffix);
            let result = device
                .feature_request(*feature, *write_fnid, &params, None)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            Ok(result.is_some())
        }
        Source::Register { register } => {
            let result = hidpp10::write_register(device, *register, &payload)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            Ok(result.is_some())
        }
    }
}

/// Encode a user-supplied string into the raw bytes for a write, and indicate
/// whether a read-modify-write is needed (mask ≠ all-0xFF).
fn encode_value(
    validator: &Validator,
    value_str: &str,
) -> Result<(Vec<u8>, bool), Box<dyn std::error::Error>> {
    match validator {
        Validator::Toggle {
            true_value,
            false_value,
            mask,
            write_prefix,
            ..
        } => {
            let new_bool = parse_bool(value_str)
                .ok_or_else(|| format!("expected true/false/on/off/1/0, got '{value_str}'"))?;
            let raw = if new_bool { true_value } else { false_value };
            let needs_rmw = mask.iter().any(|&m| m != 0xFF);
            let mut out = write_prefix.clone();
            out.extend_from_slice(raw);
            Ok((out, needs_rmw))
        }
        Validator::Range {
            min,
            max,
            byte_count,
            write_prefix,
            ..
        } => {
            let n: i64 = value_str
                .parse()
                .map_err(|_| format!("expected integer, got '{value_str}'"))?;
            if n < *min || n > *max {
                return Err(format!("value {n} out of range [{min}, {max}]").into());
            }
            let mut out = write_prefix.clone();
            for i in (0..*byte_count).rev() {
                out.push(((n >> (i * 8)) & 0xFF) as u8);
            }
            Ok((out, false))
        }
        Validator::Choice {
            options,
            byte_count,
            write_prefix,
            ..
        } => {
            // Accept either the label or the raw integer.
            let chosen = options.iter().find(|(k, label)| {
                label.eq_ignore_ascii_case(value_str) || value_str.parse::<i64>().ok() == Some(*k)
            });
            let (k, _) = chosen.ok_or_else(|| {
                let valid: Vec<String> =
                    options.iter().map(|(v, l)| format!("{l} ({v})")).collect();
                format!(
                    "unknown value '{value_str}'. Valid options: {}",
                    valid.join(", ")
                )
            })?;
            let mut out = write_prefix.clone();
            for i in (0..*byte_count).rev() {
                out.push(((*k >> (i * 8)) & 0xFF) as u8);
            }
            Ok((out, false))
        }
    }
}

/// Merge encoded bytes with a current register value, applying the validator mask.
fn merge_with_mask(validator: &Validator, current: &[u8], encoded: &[u8]) -> Vec<u8> {
    if let Validator::Toggle {
        mask,
        write_prefix,
        true_value,
        ..
    } = validator
    {
        let prefix_len = write_prefix.len();
        let value_part = &encoded[prefix_len..]; // strip write_prefix
        let n = mask.len().min(current.len()).min(value_part.len());
        let mut merged = current.to_vec();
        for i in 0..n {
            merged[i] = (merged[i] & !mask[i]) | (value_part[i] & mask[i]);
        }
        // Re-prepend write_prefix.
        let mut out = write_prefix.clone();
        out.extend_from_slice(&merged[..n.max(true_value.len())]);
        out
    } else {
        encoded.to_vec()
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" | "t" | "y" => Some(true),
        "false" | "no" | "off" | "0" | "f" | "n" => Some(false),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers for the CLI
// ─────────────────────────────────────────────────────────────────────────────

/// Return all settings definitions whose source feature/register is supported
/// by this device.
pub fn supported_settings<'a>(device: &Device, defs: &'a [SettingDef]) -> Vec<&'a SettingDef> {
    let is_hidpp2 = device.protocol.unwrap_or(0.0) >= 2.0;
    let is_hidpp1 = device.protocol.unwrap_or(0.0) < 2.0 && device.protocol.is_some();

    defs.iter()
        .filter(|def| match &def.source {
            Source::Feature { .. } => is_hidpp2,
            Source::Register { .. } => is_hidpp1,
        })
        .collect()
}

/// Return the kind string for display.
pub fn validator_kind_str(v: &Validator) -> &'static str {
    match v {
        Validator::Toggle { .. } => "toggle",
        Validator::Range { .. } => "range",
        Validator::Choice { .. } => "choice",
    }
}

/// Return the possible values string for display.
pub fn validator_values_str(v: &Validator) -> String {
    match v {
        Validator::Toggle { .. } => "true / false".to_string(),
        Validator::Range { min, max, .. } => format!("{min}..{max}"),
        Validator::Choice { options, .. } => options
            .iter()
            .map(|(k, l)| format!("{l} ({k})"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}
