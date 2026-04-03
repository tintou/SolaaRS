// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! HID++ 2.0 protocol constants.
//!
//! Ported from `logitech_receiver/hidpp20_constants.py`.

/// HID++ 2.0 features supported by Logitech devices.
///
/// A particular device will support only a subset of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SupportedFeature {
    Root = 0x0000,
    FeatureSet = 0x0001,
    FeatureInfo = 0x0002,
    // Common
    DeviceFwVersion = 0x0003,
    DeviceUnitId = 0x0004,
    DeviceName = 0x0005,
    DeviceGroups = 0x0006,
    DeviceFriendlyName = 0x0007,
    KeepAlive = 0x0008,
    PropertyAccess = 0x0011,
    ConfigChange = 0x0020,
    CryptoId = 0x0021,
    TargetSoftware = 0x0030,
    WirelessSignalStrength = 0x0080,
    DfucontrolLegacy = 0x00C0,
    DfucontrolUnsigned = 0x00C1,
    DfucontrolSigned = 0x00C2,
    Dfucontrol = 0x00C3,
    Dfu = 0x00D0,
    BatteryStatus = 0x1000,
    BatteryVoltage = 0x1001,
    UnifiedBattery = 0x1004,
    ChargingControl = 0x1010,
    LedControl = 0x1300,
    ForcePairing = 0x1500,
    GenericTest = 0x1800,
    DeviceReset = 0x1802,
    Oobstate = 0x1805,
    ConfigDeviceProps = 0x1806,
    ChangeHost = 0x1814,
    HostsInfo = 0x1815,
    BleProPrePairing = 0x1816,
    Backlight = 0x1981,
    Backlight2 = 0x1982,
    Backlight3 = 0x1983,
    Illumination = 0x1990,
    ForceSensingButton = 0x19C0,
    Haptic = 0x19B0,
    PresenterControl = 0x1A00,
    Sensor3D = 0x1A01,
    ReprogControls = 0x1B00,
    ReprogControlsV2 = 0x1B01,
    ReprogControlsV2_2 = 0x1B02,
    ReprogControlsV3 = 0x1B03,
    ReprogControlsV4 = 0x1B04,
    FullKeyCustomization = 0x1B05,
    ControlList = 0x1B10,
    SwitchSwapability = 0x1B20,
    DeviceMode = 0x1B30,
    ReportHidUsage = 0x1BC0,
    PersistentRemappableAction = 0x1C00,
    WirelessDeviceStatus = 0x1D4B,
    RemainingPairing = 0x1DF0,
    EnableHiddenFeatures = 0x1E00,
    FirmwareProperties = 0x1F1F,
    AdcMeasurement = 0x1F20,
    // Mouse
    LeftRightSwap = 0x2001,
    SwapButtonCancel = 0x2005,
    PointerAxisOrientation = 0x2006,
    VerticalScrolling = 0x2100,
    SmartShift = 0x2110,
    SmartShiftEnhanced = 0x2111,
    HiResScrolling = 0x2120,
    HiresWheel = 0x2121,
    LowresWheel = 0x2130,
    ThumbWheel = 0x2150,
    MousePointer = 0x2200,
    AdjustableDpi = 0x2201,
    ExtendedAdjustableDpi = 0x2202,
    PointerSpeed = 0x2205,
    AngleSnapping = 0x2230,
    SurfaceTuning = 0x2240,
    XyStats = 0x2250,
    WheelStats = 0x2251,
    HybridTracking = 0x2400,
    // Keyboard
    FnInversion = 0x40A0,
    NewFnInversion = 0x40A2,
    K375sFnInversion = 0x40A3,
    Encryption = 0x4100,
    LockKeyState = 0x4220,
    SolarDashboard = 0x4301,
    KeyboardLayout = 0x4520,
    KeyboardDisableKeys = 0x4521,
    KeyboardDisableByUsage = 0x4522,
    KeyboardDisableControls = 0x4523,
    Dualplatform = 0x4530,
    Multiplatform = 0x4531,
    KeyboardLayout2 = 0x4540,
    Crown = 0x4600,
    // Touchpad
    TouchpadFwItems = 0x6010,
    TouchpadSwItems = 0x6011,
    TouchpadWin8FwItems = 0x6012,
    TapEnable = 0x6020,
    TapEnableExtended = 0x6021,
    CursorBallistic = 0x6030,
    TouchpadResolution = 0x6040,
    TouchpadRawXy = 0x6100,
    TouchmouseRawPoints = 0x6110,
    Touchmouse6120 = 0x6120,
    Gesture = 0x6500,
    Gesture2 = 0x6501,
    // Gaming
    Gkey = 0x8010,
    Mkeys = 0x8020,
    Mr = 0x8030,
    BrightnessControl = 0x8040,
    LogiModifiers = 0x8051,
    ReportRate = 0x8060,
    ExtendedAdjustableReportRate = 0x8061,
    ColorLedEffects = 0x8070,
    RgbEffects = 0x8071,
    RpmIndicator = 0x807A,
    RpmLedPattern = 0x807B,
    PerKeyLighting = 0x8080,
    PerKeyLightingV2 = 0x8081,
    ModeStatus = 0x8090,
    LegacyAxisResponseCurve = 0x80A3,
    AxisResponseCurve = 0x80A4,
    BandedAxis = 0x80B1,
    CombinedPedals = 0x80D0,
    BunnyHopping = 0x80E0,
    OnboardProfiles = 0x8100,
    ProfileManagement = 0x8101,
    MouseButtonSpy = 0x8110,
    LatencyMonitoring = 0x8111,
    GamingAttachments = 0x8120,
    ForceFeedback = 0x8123,
    DualClutch = 0x8127,
    WheelCenterPosition = 0x812C,
    DisplayGameData = 0x8130,
    CenterSpring = 0x8131,
    AxisMapping = 0x8132,
    GlobalDamping = 0x8133,
    BrakeForce = 0x8134,
    PedalStatus = 0x8135,
    TorqueLimit = 0x8136,
    ConfigurationProfiles = 0x8137,
    OperatingRange = 0x8138,
    TrueForce = 0x8139,
    FfbFilter = 0x8140,
    // Headsets
    Sidetone = 0x8300,
    Equalizer = 0x8310,
    HeadsetOut = 0x8320,
    // Solaar internal
    MouseGesture = 0xFE00,
}

impl SupportedFeature {
    pub fn from_u16(v: u16) -> Option<Self> {
        // A macro-free approach: use a lookup via a match.
        Some(match v {
            0x0000 => Self::Root,
            0x0001 => Self::FeatureSet,
            0x0002 => Self::FeatureInfo,
            0x0003 => Self::DeviceFwVersion,
            0x0004 => Self::DeviceUnitId,
            0x0005 => Self::DeviceName,
            0x0006 => Self::DeviceGroups,
            0x0007 => Self::DeviceFriendlyName,
            0x0008 => Self::KeepAlive,
            0x0011 => Self::PropertyAccess,
            0x0020 => Self::ConfigChange,
            0x0021 => Self::CryptoId,
            0x0030 => Self::TargetSoftware,
            0x0080 => Self::WirelessSignalStrength,
            0x00C0 => Self::DfucontrolLegacy,
            0x00C1 => Self::DfucontrolUnsigned,
            0x00C2 => Self::DfucontrolSigned,
            0x00C3 => Self::Dfucontrol,
            0x00D0 => Self::Dfu,
            0x1000 => Self::BatteryStatus,
            0x1001 => Self::BatteryVoltage,
            0x1004 => Self::UnifiedBattery,
            0x1010 => Self::ChargingControl,
            0x1300 => Self::LedControl,
            0x1500 => Self::ForcePairing,
            0x1800 => Self::GenericTest,
            0x1802 => Self::DeviceReset,
            0x1805 => Self::Oobstate,
            0x1806 => Self::ConfigDeviceProps,
            0x1814 => Self::ChangeHost,
            0x1815 => Self::HostsInfo,
            0x1816 => Self::BleProPrePairing,
            0x1981 => Self::Backlight,
            0x1982 => Self::Backlight2,
            0x1983 => Self::Backlight3,
            0x1990 => Self::Illumination,
            0x19C0 => Self::ForceSensingButton,
            0x19B0 => Self::Haptic,
            0x1A00 => Self::PresenterControl,
            0x1A01 => Self::Sensor3D,
            0x1B00 => Self::ReprogControls,
            0x1B01 => Self::ReprogControlsV2,
            0x1B02 => Self::ReprogControlsV2_2,
            0x1B03 => Self::ReprogControlsV3,
            0x1B04 => Self::ReprogControlsV4,
            0x1B05 => Self::FullKeyCustomization,
            0x1B10 => Self::ControlList,
            0x1B20 => Self::SwitchSwapability,
            0x1B30 => Self::DeviceMode,
            0x1BC0 => Self::ReportHidUsage,
            0x1C00 => Self::PersistentRemappableAction,
            0x1D4B => Self::WirelessDeviceStatus,
            0x1DF0 => Self::RemainingPairing,
            0x1E00 => Self::EnableHiddenFeatures,
            0x1F1F => Self::FirmwareProperties,
            0x1F20 => Self::AdcMeasurement,
            0x2001 => Self::LeftRightSwap,
            0x2005 => Self::SwapButtonCancel,
            0x2006 => Self::PointerAxisOrientation,
            0x2100 => Self::VerticalScrolling,
            0x2110 => Self::SmartShift,
            0x2111 => Self::SmartShiftEnhanced,
            0x2120 => Self::HiResScrolling,
            0x2121 => Self::HiresWheel,
            0x2130 => Self::LowresWheel,
            0x2150 => Self::ThumbWheel,
            0x2200 => Self::MousePointer,
            0x2201 => Self::AdjustableDpi,
            0x2202 => Self::ExtendedAdjustableDpi,
            0x2205 => Self::PointerSpeed,
            0x2230 => Self::AngleSnapping,
            0x2240 => Self::SurfaceTuning,
            0x2250 => Self::XyStats,
            0x2251 => Self::WheelStats,
            0x2400 => Self::HybridTracking,
            0x40A0 => Self::FnInversion,
            0x40A2 => Self::NewFnInversion,
            0x40A3 => Self::K375sFnInversion,
            0x4100 => Self::Encryption,
            0x4220 => Self::LockKeyState,
            0x4301 => Self::SolarDashboard,
            0x4520 => Self::KeyboardLayout,
            0x4521 => Self::KeyboardDisableKeys,
            0x4522 => Self::KeyboardDisableByUsage,
            0x4523 => Self::KeyboardDisableControls,
            0x4530 => Self::Dualplatform,
            0x4531 => Self::Multiplatform,
            0x4540 => Self::KeyboardLayout2,
            0x4600 => Self::Crown,
            0x6010 => Self::TouchpadFwItems,
            0x6011 => Self::TouchpadSwItems,
            0x6012 => Self::TouchpadWin8FwItems,
            0x6020 => Self::TapEnable,
            0x6021 => Self::TapEnableExtended,
            0x6030 => Self::CursorBallistic,
            0x6040 => Self::TouchpadResolution,
            0x6100 => Self::TouchpadRawXy,
            0x6110 => Self::TouchmouseRawPoints,
            0x6120 => Self::Touchmouse6120,
            0x6500 => Self::Gesture,
            0x6501 => Self::Gesture2,
            0x8010 => Self::Gkey,
            0x8020 => Self::Mkeys,
            0x8030 => Self::Mr,
            0x8040 => Self::BrightnessControl,
            0x8051 => Self::LogiModifiers,
            0x8060 => Self::ReportRate,
            0x8061 => Self::ExtendedAdjustableReportRate,
            0x8070 => Self::ColorLedEffects,
            0x8071 => Self::RgbEffects,
            0x807A => Self::RpmIndicator,
            0x807B => Self::RpmLedPattern,
            0x8080 => Self::PerKeyLighting,
            0x8081 => Self::PerKeyLightingV2,
            0x8090 => Self::ModeStatus,
            0x80A3 => Self::LegacyAxisResponseCurve,
            0x80A4 => Self::AxisResponseCurve,
            0x80B1 => Self::BandedAxis,
            0x80D0 => Self::CombinedPedals,
            0x80E0 => Self::BunnyHopping,
            0x8100 => Self::OnboardProfiles,
            0x8101 => Self::ProfileManagement,
            0x8110 => Self::MouseButtonSpy,
            0x8111 => Self::LatencyMonitoring,
            0x8120 => Self::GamingAttachments,
            0x8123 => Self::ForceFeedback,
            0x8127 => Self::DualClutch,
            0x812C => Self::WheelCenterPosition,
            0x8130 => Self::DisplayGameData,
            0x8131 => Self::CenterSpring,
            0x8132 => Self::AxisMapping,
            0x8133 => Self::GlobalDamping,
            0x8134 => Self::BrakeForce,
            0x8135 => Self::PedalStatus,
            0x8136 => Self::TorqueLimit,
            0x8137 => Self::ConfigurationProfiles,
            0x8138 => Self::OperatingRange,
            0x8139 => Self::TrueForce,
            0x8140 => Self::FfbFilter,
            0x8300 => Self::Sidetone,
            0x8310 => Self::Equalizer,
            0x8320 => Self::HeadsetOut,
            0xFE00 => Self::MouseGesture,
            _ => return None,
        })
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

bitflags::bitflags! {
    /// Per-feature flags returned in the feature enumeration response.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FeatureFlag: u8 {
        const INTERNAL = 0x20;
        const HIDDEN   = 0x40;
        const OBSOLETE = 0x80;
    }
}

/// Device kind codes as used by HID++ 2.0 (different from 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceKind {
    Keyboard = 0x00,
    RemoteControl = 0x01,
    Numpad = 0x02,
    Mouse = 0x03,
    Touchpad = 0x04,
    Trackball = 0x05,
    Presenter = 0x06,
    Receiver = 0x07,
}

impl From<u8> for DeviceKind {
    fn from(v: u8) -> Self {
        match v {
            0x00 => Self::Keyboard,
            0x01 => Self::RemoteControl,
            0x02 => Self::Numpad,
            0x03 => Self::Mouse,
            0x04 => Self::Touchpad,
            0x05 => Self::Trackball,
            0x06 => Self::Presenter,
            _ => Self::Receiver,
        }
    }
}

/// Onboard profile modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OnboardMode {
    NoChange = 0x00,
    Onboard = 0x01,
    Host = 0x02,
}

/// Approximate charge level thresholds (HID++ 2.0 UNIFIED_BATTERY feature).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ChargeLevel {
    Average = 50,
    Full = 90,
    Critical = 5,
}

/// Charge type reported by devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChargeType {
    Standard = 0x00,
    Fast = 0x01,
    Slow = 0x02,
}

impl From<u8> for ChargeType {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::Fast,
            0x02 => Self::Slow,
            _ => Self::Standard,
        }
    }
}

/// HID++ 2.0 error codes returned in a sub_id=0xFF reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorCode {
    Unknown = 0x01,
    InvalidArgument = 0x02,
    OutOfRange = 0x03,
    HardwareError = 0x04,
    LogitechError = 0x05,
    InvalidFeatureIndex = 0x06,
    InvalidFunction = 0x07,
    Busy = 0x08,
    Unsupported = 0x09,
    UnknownCode(u8),
}

impl From<u8> for ErrorCode {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::Unknown,
            0x02 => Self::InvalidArgument,
            0x03 => Self::OutOfRange,
            0x04 => Self::HardwareError,
            0x05 => Self::LogitechError,
            0x06 => Self::InvalidFeatureIndex,
            0x07 => Self::InvalidFunction,
            0x08 => Self::Busy,
            0x09 => Self::Unsupported,
            other => Self::UnknownCode(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FeatureFlag (mirrors test_hidpp20_simple.py::test_feature_flag_names) ─

    #[test]
    fn feature_flag_bit_values() {
        assert_eq!(FeatureFlag::INTERNAL.bits(), 0x20);
        assert_eq!(FeatureFlag::HIDDEN.bits(), 0x40);
        assert_eq!(FeatureFlag::OBSOLETE.bits(), 0x80);
    }

    #[test]
    fn feature_flag_from_bits_internal() {
        let f = FeatureFlag::from_bits_truncate(0x20);
        assert!(f.contains(FeatureFlag::INTERNAL));
        assert!(!f.contains(FeatureFlag::HIDDEN));
        assert!(!f.contains(FeatureFlag::OBSOLETE));
    }

    #[test]
    fn feature_flag_from_bits_hidden() {
        let f = FeatureFlag::from_bits_truncate(0x40);
        assert!(!f.contains(FeatureFlag::INTERNAL));
        assert!(f.contains(FeatureFlag::HIDDEN));
    }

    #[test]
    fn feature_flag_from_bits_obsolete() {
        let f = FeatureFlag::from_bits_truncate(0x80);
        assert!(f.contains(FeatureFlag::OBSOLETE));
    }

    #[test]
    fn feature_flag_from_bits_all_known() {
        // 0xE0 = INTERNAL | HIDDEN | OBSOLETE
        let f = FeatureFlag::from_bits_truncate(0xE0);
        assert!(f.contains(FeatureFlag::INTERNAL));
        assert!(f.contains(FeatureFlag::HIDDEN));
        assert!(f.contains(FeatureFlag::OBSOLETE));
    }

    #[test]
    fn feature_flag_unknown_bits_truncated() {
        // Bits below 0x20 are not defined; from_bits_truncate silently ignores them.
        let f = FeatureFlag::from_bits_truncate(0x01);
        assert_eq!(f, FeatureFlag::empty());
    }

    #[test]
    fn feature_flag_empty_no_flags() {
        assert_eq!(FeatureFlag::empty().bits(), 0x00);
    }

    // ── DeviceKind (HID++ 2.0) ─────────────────

    #[test]
    fn device_kind_hidpp20_from_u8() {
        assert_eq!(DeviceKind::from(0x00), DeviceKind::Keyboard);
        assert_eq!(DeviceKind::from(0x03), DeviceKind::Mouse);
        assert_eq!(DeviceKind::from(0x04), DeviceKind::Touchpad);
        assert_eq!(DeviceKind::from(0x07), DeviceKind::Receiver);
    }

    // ── SupportedFeature known values ────────────────────────────────────────

    #[test]
    fn supported_feature_known_ids() {
        assert_eq!(SupportedFeature::Root.as_u16(), 0x0000);
        assert_eq!(SupportedFeature::FeatureSet.as_u16(), 0x0001);
        assert_eq!(SupportedFeature::DeviceFwVersion.as_u16(), 0x0003);
        assert_eq!(SupportedFeature::BatteryStatus.as_u16(), 0x1000);
        assert_eq!(SupportedFeature::BatteryVoltage.as_u16(), 0x1001);
        assert_eq!(SupportedFeature::UnifiedBattery.as_u16(), 0x1004);
        assert_eq!(SupportedFeature::OnboardProfiles.as_u16(), 0x8100);
    }

    #[test]
    fn supported_feature_from_u16_roundtrip() {
        assert_eq!(
            SupportedFeature::from_u16(0x1000),
            Some(SupportedFeature::BatteryStatus)
        );
        assert_eq!(
            SupportedFeature::from_u16(0x1001),
            Some(SupportedFeature::BatteryVoltage)
        );
        assert_eq!(
            SupportedFeature::from_u16(0x0000),
            Some(SupportedFeature::Root)
        );
        assert_eq!(SupportedFeature::from_u16(0xFFFF), None);
    }

    // ── ErrorCode (HID++ 2.0) ────────────────────────────────────────────────

    #[test]
    fn hidpp20_error_code_from_u8() {
        assert_eq!(ErrorCode::from(0x01), ErrorCode::Unknown);
        assert_eq!(ErrorCode::from(0x02), ErrorCode::InvalidArgument);
        assert_eq!(ErrorCode::from(0x09), ErrorCode::Unsupported);
        assert_eq!(ErrorCode::from(0xFF), ErrorCode::UnknownCode(0xFF));
    }
}
