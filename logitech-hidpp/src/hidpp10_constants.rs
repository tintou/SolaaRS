// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! HID++ 1.0 protocol constants.
//!
//! Ported from `logitech_receiver/hidpp10_constants.py`.

bitflags::bitflags! {
    /// Notification flag bits used in the NOTIFICATIONS register (0x00).
    ///
    /// Some flags apply to both receivers and devices; see the Python source for
    /// the distinction.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NotificationFlag: u32 {
        const NUMPAD_NUMERICAL_KEYS      = 0x800000;
        const F_LOCK_STATUS              = 0x400000;
        const ROLLER_H                   = 0x200000;
        /// Send battery-charge notifications (sub_id 0x07 or 0x0D).
        const BATTERY_STATUS             = 0x100000;
        const MOUSE_EXTRA_BUTTONS        = 0x080000;
        const ROLLER_V                   = 0x040000;
        /// System control keys such as Sleep.
        const POWER_KEYS                 = 0x020000;
        /// Consumer controls such as Mute and Calculator.
        const KEYBOARD_MULTIMEDIA_RAW    = 0x010000;
        /// Notify on multi-touch changes.
        const MULTI_TOUCH                = 0x001000;
        /// Software is controlling part of device behaviour.
        const SOFTWARE_PRESENT           = 0x000800;
        /// Notify on link quality changes.
        const LINK_QUALITY               = 0x000400;
        /// Notify on UI changes.
        const UI                         = 0x000200;
        /// Notify when the device wireless goes on/off-line.
        const WIRELESS                   = 0x000100;
        const CONFIGURATION_COMPLETE     = 0x000004;
        const VOIP_TELEPHONY             = 0x000002;
        const THREED_GESTURE             = 0x000001;
    }
}

bitflags::bitflags! {
    /// Device capability flags for HID++ 1.0 devices.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DeviceFeature: u32 {
        const RESERVED1                  = 0x010000;
        const SPECIAL_BUTTONS            = 0x020000;
        const ENHANCED_KEY_USAGE         = 0x040000;
        const FAST_FW_REV                = 0x080000;
        const RESERVED2                  = 0x100000;
        const RESERVED3                  = 0x200000;
        const SCROLL_ACCEL               = 0x400000;
        const BUTTONS_CONTROL_RESOLUTION = 0x800000;
        const INHIBIT_LOCK_KEY_SOUND     = 0x000001;
        const RESERVED4                  = 0x000002;
        const MX_AIR_3D_ENGINE           = 0x000004;
        const HOST_CONTROL_LEDS          = 0x000008;
        const RESERVED5                  = 0x000010;
        const RESERVED6                  = 0x000020;
        const RESERVED7                  = 0x000040;
        const RESERVED8                  = 0x000080;
    }
}

/// Device kind codes as used by HID++ 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceKind {
    Unknown = 0x00,
    Keyboard = 0x01,
    Mouse = 0x02,
    Numpad = 0x03,
    Presenter = 0x04,
    Remote = 0x07,
    Trackball = 0x08,
    Touchpad = 0x09,
    Tablet = 0x0A,
    Gamepad = 0x0B,
    Joystick = 0x0C,
    Headset = 0x0D,
    RemoteControl = 0x0E,
    Receiver = 0x0F,
}

impl From<u8> for DeviceKind {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::Keyboard,
            0x02 => Self::Mouse,
            0x03 => Self::Numpad,
            0x04 => Self::Presenter,
            0x07 => Self::Remote,
            0x08 => Self::Trackball,
            0x09 => Self::Touchpad,
            0x0A => Self::Tablet,
            0x0B => Self::Gamepad,
            0x0C => Self::Joystick,
            0x0D => Self::Headset,
            0x0E => Self::RemoteControl,
            0x0F => Self::Receiver,
            _ => Self::Unknown,
        }
    }
}

/// Physical location of the power switch on a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerSwitchLocation {
    Unknown = 0x00,
    Base = 0x01,
    TopCase = 0x02,
    EdgeOfTopRightCorner = 0x03,
    TopLeftCorner = 0x05,
    BottomLeftCorner = 0x06,
    TopRightCorner = 0x07,
    BottomRightCorner = 0x08,
    TopEdge = 0x09,
    RightEdge = 0x0A,
    LeftEdge = 0x0B,
    BottomEdge = 0x0C,
}

impl From<u8> for PowerSwitchLocation {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::Base,
            0x02 => Self::TopCase,
            0x03 => Self::EdgeOfTopRightCorner,
            0x05 => Self::TopLeftCorner,
            0x06 => Self::BottomLeftCorner,
            0x07 => Self::TopRightCorner,
            0x08 => Self::BottomRightCorner,
            0x09 => Self::TopEdge,
            0x0A => Self::RightEdge,
            0x0B => Self::LeftEdge,
            0x0C => Self::BottomEdge,
            _ => Self::Unknown,
        }
    }
}

/// HID++ 1.0 error codes returned in a sub_id=0x8F reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorCode {
    InvalidSubIdCommand = 0x01,
    InvalidAddress = 0x02,
    InvalidValue = 0x03,
    ConnectionRequestFailed = 0x04,
    TooManyDevices = 0x05,
    AlreadyExists = 0x06,
    Busy = 0x07,
    UnknownDevice = 0x08,
    ResourceError = 0x09,
    RequestUnavailable = 0x0A,
    UnsupportedParameterValue = 0x0B,
    WrongPinCode = 0x0C,
    Unknown(u8),
}

impl From<u8> for ErrorCode {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::InvalidSubIdCommand,
            0x02 => Self::InvalidAddress,
            0x03 => Self::InvalidValue,
            0x04 => Self::ConnectionRequestFailed,
            0x05 => Self::TooManyDevices,
            0x06 => Self::AlreadyExists,
            0x07 => Self::Busy,
            0x08 => Self::UnknownDevice,
            0x09 => Self::ResourceError,
            0x0A => Self::RequestUnavailable,
            0x0B => Self::UnsupportedParameterValue,
            0x0C => Self::WrongPinCode,
            other => Self::Unknown(other),
        }
    }
}

/// HID++ 1.0 pairing error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PairingError {
    DeviceTimeout = 0x01,
    DeviceNotSupported = 0x02,
    TooManyDevices = 0x03,
    SequenceTimeout = 0x06,
    Unknown(u8),
}

impl From<u8> for PairingError {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::DeviceTimeout,
            0x02 => Self::DeviceNotSupported,
            0x03 => Self::TooManyDevices,
            0x06 => Self::SequenceTimeout,
            other => Self::Unknown(other),
        }
    }
}

impl PairingError {
    pub fn label(self) -> &'static str {
        match self {
            Self::DeviceTimeout => "device timeout",
            Self::DeviceNotSupported => "device not supported",
            Self::TooManyDevices => "too many devices",
            Self::SequenceTimeout => "sequence timeout",
            Self::Unknown(_) => "unknown error",
        }
    }
}

/// Bolt receiver pairing error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BoltPairingError {
    DeviceTimeout = 0x01,
    Failed = 0x02,
    Unknown(u8),
}

impl From<u8> for BoltPairingError {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::DeviceTimeout,
            0x02 => Self::Failed,
            other => Self::Unknown(other),
        }
    }
}

impl BoltPairingError {
    pub fn label(self) -> &'static str {
        match self {
            Self::DeviceTimeout => "device timeout",
            Self::Failed => "failed",
            Self::Unknown(_) => "unknown error",
        }
    }
}

/// Known HID++ 1.0 registers.
///
/// Devices generally support only a subset of these.  Some registers are only
/// applicable to certain device kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Register {
    // Generally applicable
    Notifications = 0x00,
    Firmware = 0xF1,

    // Receiver-only
    ReceiverConnection = 0x02,
    ReceiverPairing = 0xB2,
    DevicesActivity = 0x2B3,
    ReceiverInfo = 0x2B5,
    BoltDeviceDiscovery = 0xC0,
    BoltPairing = 0x2C1,
    BoltUniqueId = 0x02FB,

    // Device-only
    MouseButtonFlags = 0x01,
    // KeyboardHandDetection shares the same number as MouseButtonFlags
    DevicesConfiguration = 0x03,
    BatteryStatus = 0x07,
    KeyboardFnSwap = 0x09,
    BatteryCharge = 0x0D,
    KeyboardIllumination = 0x17,
    ThreeLeds = 0x51,
    MouseDpi = 0x63,

    // Notification registers
    PasskeyRequestNotification = 0x4D,
    PasskeyPressedNotification = 0x4E,
    DeviceDiscoveryNotification = 0x4F,
    DiscoveryStatusNotification = 0x53,
    PairingStatusNotification = 0x54,
}

impl Register {
    /// Returns the raw 16-bit register number.
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Sub-ID of the lock-information notification emitted by a receiver when
/// its pairing window opens or closes (Unifying / Nano / Lightspeed).
///
/// The first data byte encodes the new state:
/// - `0x01` → pairing window opened; `Discovering` should become `true`.
/// - `0x02` / `0x03` → pairing in progress (device found, exchanging keys).
/// - `0x00` → pairing window closed (timeout or explicit close).
///
/// Device number in these notifications is always `0xFF` (the receiver itself).
pub const SUB_ID_LOCK_INFORMATION: u8 = 0x4A;

/// Sub-register offsets for the `RECEIVER_INFO` register (0x2B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InfoSubRegister {
    SerialNumber = 0x01,
    FwVersion = 0x02,
    ReceiverInformation = 0x03,
    /// 0x2N – by connected device (N = device index).
    PairingInformation = 0x20,
    /// 0x3N – by connected device.
    ExtendedPairingInformation = 0x30,
    /// 0x4N – by connected device.
    DeviceName = 0x40,
    /// 0x5N – by connected device (Bolt only).
    BoltPairingInformation = 0x50,
    /// 0x6N01 – by connected device (Bolt only).
    BoltDeviceName = 0x60,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DeviceKind ───────────────────────────────────────────────────────────

    #[test]
    fn device_kind_from_known_bytes() {
        assert_eq!(DeviceKind::from(0x01), DeviceKind::Keyboard);
        assert_eq!(DeviceKind::from(0x02), DeviceKind::Mouse);
        assert_eq!(DeviceKind::from(0x03), DeviceKind::Numpad);
        assert_eq!(DeviceKind::from(0x04), DeviceKind::Presenter);
        assert_eq!(DeviceKind::from(0x08), DeviceKind::Trackball);
        assert_eq!(DeviceKind::from(0x09), DeviceKind::Touchpad);
        assert_eq!(DeviceKind::from(0x0D), DeviceKind::Headset);
        assert_eq!(DeviceKind::from(0x0F), DeviceKind::Receiver);
    }

    #[test]
    fn device_kind_from_unknown_byte() {
        assert_eq!(DeviceKind::from(0x00), DeviceKind::Unknown);
        assert_eq!(DeviceKind::from(0xFF), DeviceKind::Unknown);
    }

    // ── Register ─────────────────────────────────────────────────────────────

    #[test]
    fn register_as_u16() {
        assert_eq!(Register::Notifications.as_u16(), 0x00);
        assert_eq!(Register::Firmware.as_u16(), 0xF1);
        assert_eq!(Register::ReceiverInfo.as_u16(), 0x2B5);
        assert_eq!(Register::MouseButtonFlags.as_u16(), 0x01);
        assert_eq!(Register::BatteryStatus.as_u16(), 0x07);
    }

    // ── NotificationFlag ─────────────────────────────────────────────────────

    #[test]
    fn notification_flag_bitwise_combine() {
        let combined = NotificationFlag::BATTERY_STATUS | NotificationFlag::WIRELESS;
        assert!(combined.contains(NotificationFlag::BATTERY_STATUS));
        assert!(combined.contains(NotificationFlag::WIRELESS));
        assert!(!combined.contains(NotificationFlag::POWER_KEYS));
    }

    #[test]
    fn notification_flag_empty() {
        let empty = NotificationFlag::empty();
        assert!(!empty.contains(NotificationFlag::BATTERY_STATUS));
    }

    // ── ErrorCode ────────────────────────────────────────────────────────────

    #[test]
    fn error_code_from_u8() {
        assert_eq!(ErrorCode::from(0x01), ErrorCode::InvalidSubIdCommand);
        assert_eq!(ErrorCode::from(0x02), ErrorCode::InvalidAddress);
        assert_eq!(ErrorCode::from(0x05), ErrorCode::TooManyDevices);
        assert_eq!(ErrorCode::from(0xFF), ErrorCode::Unknown(0xFF));
    }

    // ── PairingError labels (mirrors test_hidpp10.py::test_pairing_error) ────

    #[test]
    fn pairing_error_device_not_supported_label() {
        assert_eq!(
            PairingError::DeviceNotSupported.label(),
            "device not supported"
        );
    }

    #[test]
    fn pairing_error_device_timeout_label() {
        assert_eq!(PairingError::DeviceTimeout.label(), "device timeout");
    }

    #[test]
    fn pairing_error_too_many_devices_label() {
        assert_eq!(PairingError::TooManyDevices.label(), "too many devices");
    }

    #[test]
    fn pairing_error_sequence_timeout_label() {
        assert_eq!(PairingError::SequenceTimeout.label(), "sequence timeout");
    }

    #[test]
    fn pairing_error_from_u8() {
        assert_eq!(PairingError::from(0x01), PairingError::DeviceTimeout);
        assert_eq!(PairingError::from(0x02), PairingError::DeviceNotSupported);
        assert_eq!(PairingError::from(0x03), PairingError::TooManyDevices);
        assert_eq!(PairingError::from(0x06), PairingError::SequenceTimeout);
        assert_eq!(PairingError::from(0xFF), PairingError::Unknown(0xFF));
    }

    // ── BoltPairingError labels ───────────────────────────────────────────────

    #[test]
    fn bolt_pairing_error_labels() {
        assert_eq!(BoltPairingError::DeviceTimeout.label(), "device timeout");
        assert_eq!(BoltPairingError::Failed.label(), "failed");
        assert_eq!(
            BoltPairingError::from(0x01),
            BoltPairingError::DeviceTimeout
        );
        assert_eq!(BoltPairingError::from(0x02), BoltPairingError::Failed);
        assert_eq!(
            BoltPairingError::from(0xFF),
            BoltPairingError::Unknown(0xFF)
        );
    }

    // ── NotificationFlag known bit values ────────────────────────────────────

    /// Mirrors test_notification_flag_str values from test_hidpp10.py.
    #[test]
    fn notification_flag_known_values() {
        assert_eq!(NotificationFlag::MULTI_TOUCH.bits(), 0x001000);
        assert_eq!(NotificationFlag::MOUSE_EXTRA_BUTTONS.bits(), 0x080000);
        assert_eq!(NotificationFlag::BATTERY_STATUS.bits(), 0x100000);
        assert_eq!(NotificationFlag::WIRELESS.bits(), 0x000100);
        assert_eq!(NotificationFlag::LINK_QUALITY.bits(), 0x000400);
    }

    #[test]
    fn notification_flag_combined_battery_wireless() {
        // Python test_set_notification_flags: BATTERY_STATUS | WIRELESS → 0x100100
        let combined = NotificationFlag::BATTERY_STATUS | NotificationFlag::WIRELESS;
        assert_eq!(combined.bits(), 0x100100);
    }
}
