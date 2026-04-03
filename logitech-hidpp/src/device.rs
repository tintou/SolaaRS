// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Logitech HID++ device abstraction.
//!
//! A [`Device`] represents a single physical device — either directly connected
//! via USB/Bluetooth, or paired wirelessly through a receiver.  It wraps a
//! [`HidppDevice`] handle and adds caching, protocol negotiation, and the
//! HID++ 2.0 feature-lookup layer.

use std::sync::{Arc, Mutex};

use log::{debug, warn};

use crate::base::{HidppDevice, NotificationsHook, RequestOptions};
use crate::common::{Battery, FirmwareInfo};
use crate::error::Error;
use crate::hidpp10::{Hidpp10, Hidpp10Device};
use crate::hidpp10_constants::{NotificationFlag, Register};
use crate::hidpp20::{FeaturesArray, Hidpp20, Hidpp20Device};
use crate::hidpp20_constants::SupportedFeature;

// ─────────────────────────────────────────────────────────────────────────────
// Pairing info (supplied by the receiver when a device pairs)
// ─────────────────────────────────────────────────────────────────────────────

/// Callback type for device state changes.
pub type StatusCallback = Box<dyn Fn(&Device, Option<String>) + Send>;

/// Callback type for per-device notification handlers.
pub type NotificationHandler =
    Box<dyn Fn(&Device, &crate::message::HidppNotification) -> Option<bool> + Send>;

/// Information supplied by the receiver about a paired wireless device.
#[derive(Debug, Clone)]
pub struct PairingInfo {
    /// Wireless Product ID (unique per device model).
    pub wpid: u16,
    pub kind: Option<crate::hidpp10_constants::DeviceKind>,
    /// 8-character hex serial number.
    pub serial: Option<String>,
    /// Polling rate in milliseconds.
    pub polling_rate: Option<u8>,
    pub power_switch: Option<crate::hidpp10_constants::PowerSwitchLocation>,
    /// Short device codename (e.g. "MX518").
    pub codename: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Device
// ─────────────────────────────────────────────────────────────────────────────

/// A Logitech HID++ device (keyboard, mouse, etc.).
///
/// Call [`Device::ping`] first to verify the device is online and discover
/// the protocol version before making other requests.
pub struct Device {
    /// The communication channel.  May be shared with a [`crate::receiver::Receiver`].
    hidpp: Arc<HidppDevice>,
    /// HID++ device number: 0xFF for directly-connected, 0x01-0x0F for paired.
    pub number: u8,
    /// Whether to use long (20-byte) HID++ messages.
    pub long_message: bool,
    /// Negotiated HID++ protocol version (`1.0`, `2.0`, …).  `None` before a ping.
    pub protocol: Option<f32>,
    /// Whether the device is currently online.
    pub online: bool,
    /// Wireless Product ID (None for directly connected devices).
    pub wpid: Option<u16>,
    /// USB Product ID (for directly-connected devices).
    pub product_id: Option<u16>,
    /// Device path (e.g. `/dev/hidraw0`).
    pub path: Option<String>,
    /// Short device codename (e.g. "MX518").
    pub codename: Option<String>,
    /// Device serial number.
    pub serial: Option<String>,
    /// Device kind (keyboard, mouse, etc.).
    pub kind: Option<crate::hidpp10_constants::DeviceKind>,
    /// Polling rate in milliseconds.
    pub polling_rate: Option<u8>,
    /// Location of the physical power switch.
    pub power_switch_location: Option<crate::hidpp10_constants::PowerSwitchLocation>,

    /// HID++ 1.0 registers this device is known to support.
    registers: Vec<Register>,
    /// HID++ 2.0 feature cache.
    features: Mutex<FeaturesArray>,
    /// Cached battery information.
    battery_info: Option<Battery>,
    /// Which HID++ 2.0 battery feature was last successfully used.
    battery_feature: Option<SupportedFeature>,

    /// Optional callback invoked when the device state changes.
    pub status_callback: Option<StatusCallback>,
    /// Notification handlers registered by name.
    notification_handlers: Mutex<std::collections::HashMap<String, NotificationHandler>>,
}

impl Device {
    /// Wrap an already-opened [`HidppDevice`] as a directly-connected device.
    ///
    /// `number` should be `0xFF` for Bluetooth/USB devices, or `0x01–0x0F`
    /// when the device is accessed through a receiver handle.
    pub fn new(hidpp: Arc<HidppDevice>, number: u8, long_message: bool) -> Self {
        Self {
            hidpp,
            number,
            long_message,
            protocol: None,
            online: false,
            wpid: None,
            product_id: None,
            path: None,
            codename: None,
            serial: None,
            kind: None,
            polling_rate: None,
            power_switch_location: None,
            registers: Vec::new(),
            features: Mutex::new(FeaturesArray::new()),
            battery_info: None,
            battery_feature: None,
            status_callback: None,
            notification_handlers: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Create a device that communicates through a receiver's handle.
    pub fn with_receiver(
        receiver_hidpp: Arc<HidppDevice>,
        number: u8,
        long_message: bool,
        pairing_info: PairingInfo,
    ) -> Self {
        let mut dev = Self::new(receiver_hidpp, number, long_message);
        dev.wpid = Some(pairing_info.wpid);
        dev.kind = pairing_info.kind;
        dev.serial = pairing_info.serial;
        dev.polling_rate = pairing_info.polling_rate;
        dev.power_switch_location = pairing_info.power_switch;
        dev.codename = pairing_info.codename;
        dev
    }

    // ── Core request methods ─────────────────────────────────────────────────

    /// Send a raw HID++ request to this device and wait for the reply.
    pub fn request(
        &self,
        request_id: u16,
        params: &[u8],
        hook: Option<&NotificationsHook>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let opts = RequestOptions {
            long_message: self.long_message,
            protocol: self.protocol.unwrap_or(1.0),
            ..Default::default()
        };
        self.hidpp
            .request(self.number, request_id, params, &opts, hook)
    }

    /// Send a HID++ 2.0 feature call.
    ///
    /// Resolves `feature` to a device-assigned index via the feature cache,
    /// then issues `(index << 8) | (function & 0xFF)`.
    pub fn feature_request(
        &self,
        feature: SupportedFeature,
        function: u8,
        params: &[u8],
        hook: Option<&NotificationsHook>,
    ) -> Result<Option<Vec<u8>>, Error> {
        if self.protocol.unwrap_or(0.0) < 2.0 || !self.online {
            return Ok(None);
        }

        let mut cache = self.features.lock().map_err(|_| Error::LockPoisoned)?;
        let hidpp = &self.hidpp;
        let number = self.number;
        let long_message = self.long_message;
        let protocol = self.protocol.unwrap_or(2.0);

        // Initialise the feature cache on first use.
        if !cache.is_initialised() {
            let init_ok = cache.init(|request_id, p| {
                let opts = RequestOptions {
                    long_message,
                    protocol,
                    ..Default::default()
                };
                hidpp
                    .request(number, request_id, p, &opts, None)
                    .ok()
                    .flatten()
            });
            if !init_ok {
                return Ok(None);
            }
        }

        let index = cache.get_index(feature, |request_id, p| {
            let opts = RequestOptions {
                long_message,
                protocol,
                ..Default::default()
            };
            hidpp
                .request(number, request_id, p, &opts, None)
                .ok()
                .flatten()
        });

        let index = match index {
            Some(i) => i,
            None => return Ok(None),
        };

        let request_id = (index as u16) << 8 | (function as u16 & 0xFF);
        drop(cache); // Release lock before the (potentially blocking) request.

        let opts = RequestOptions {
            long_message: self.long_message,
            protocol: self.protocol.unwrap_or(2.0),
            ..Default::default()
        };
        self.hidpp
            .request(self.number, request_id, params, &opts, hook)
    }

    /// Ping the device to verify it is reachable and discover the protocol version.
    ///
    /// Returns `true` if the device responded.
    pub fn ping(&mut self, hook: Option<&NotificationsHook>) -> Result<bool, Error> {
        match self.hidpp.ping(self.number, self.long_message, hook) {
            Ok(Some(version)) => {
                debug!("device {} ping ok: protocol {:.1}", self.number, version);
                self.protocol = Some(version as f32);
                self.online = true;
                Ok(true)
            }
            Ok(None) => {
                debug!("device {} ping timeout", self.number);
                self.online = false;
                Ok(false)
            }
            Err(Error::NoSuchDevice { .. }) => {
                self.online = false;
                Err(Error::NoSuchDevice {
                    number: self.number,
                    request: 0x0010,
                })
            }
            Err(e) => Err(e),
        }
    }

    // ── Battery ──────────────────────────────────────────────────────────────

    /// Read battery information from the device and update the internal cache.
    pub fn read_battery(&mut self, hook: Option<&NotificationsHook>) -> Result<(), Error> {
        if !self.online {
            return Ok(());
        }
        let battery = if self.protocol.unwrap_or(0.0) >= 2.0 {
            let h20 = Hidpp20;
            // Pass a wrapper that satisfies Hidpp20Device.
            let wrapper = DeviceFeatureWrapper { device: self, hook };
            h20.get_battery(&wrapper, self.battery_feature)?
                .map(|(feat, bat)| {
                    self.battery_feature = Some(feat);
                    bat
                })
        } else {
            let h10 = Hidpp10;
            h10.get_battery(self)?
        };

        self.set_battery_info(battery.unwrap_or_default());
        Ok(())
    }

    fn set_battery_info(&mut self, info: Battery) {
        let reason = if !info.ok() {
            warn!(
                "device {}: battery alert, status {:?}",
                self.number, info.status
            );
            Some(format!("battery {:?}", info.status))
        } else {
            None
        };

        self.battery_info = Some(info);
        self.changed(None, reason);
    }

    pub fn battery_info(&self) -> Option<&Battery> {
        self.battery_info.as_ref()
    }

    // ── Firmware ─────────────────────────────────────────────────────────────

    /// Read firmware information from the device.
    pub fn read_firmware(
        &self,
        hook: Option<&NotificationsHook>,
    ) -> Result<Option<Vec<FirmwareInfo>>, Error> {
        if !self.online {
            return Ok(None);
        }
        if self.protocol.unwrap_or(0.0) >= 2.0 {
            let wrapper = DeviceFeatureWrapper { device: self, hook };
            Hidpp20.get_firmware(&wrapper)
        } else {
            Hidpp10.get_firmware(self)
        }
    }

    // ── HID++ 2.0 convenience queries ────────────────────────────────────────

    fn hidpp20_wrapper(&self) -> Option<DeviceFeatureWrapper<'_>> {
        if self.online && self.protocol.unwrap_or(0.0) >= 2.0 {
            Some(DeviceFeatureWrapper {
                device: self,
                hook: None,
            })
        } else {
            None
        }
    }

    /// Get the device name via HID++ 2.0 `DEVICE_NAME` feature.
    pub fn get_name_hidpp20(&self) -> Result<Option<String>, Error> {
        match self.hidpp20_wrapper() {
            Some(w) => Hidpp20.get_name(&w),
            None => Ok(None),
        }
    }

    /// Get the device friendly name via HID++ 2.0 `DEVICE_FRIENDLY_NAME` feature.
    pub fn get_friendly_name_hidpp20(&self) -> Result<Option<String>, Error> {
        match self.hidpp20_wrapper() {
            Some(w) => Hidpp20.get_friendly_name(&w),
            None => Ok(None),
        }
    }

    /// Get the device kind via HID++ 2.0.
    pub fn get_kind_hidpp20(&self) -> Result<Option<crate::hidpp20_constants::DeviceKind>, Error> {
        match self.hidpp20_wrapper() {
            Some(w) => Hidpp20.get_kind(&w),
            None => Ok(None),
        }
    }

    /// Get battery info via any available HID++ 2.0 battery feature.
    pub fn get_battery_hidpp20(&self) -> Result<Option<Battery>, Error> {
        let Some(w) = self.hidpp20_wrapper() else {
            return Ok(None);
        };
        Ok(Hidpp20.get_battery(&w, None)?.map(|(_, b)| b))
    }

    /// Get the polling rate via HID++ 2.0.
    pub fn get_polling_rate_hidpp20(&self) -> Result<Option<u8>, Error> {
        match self.hidpp20_wrapper() {
            Some(w) => Hidpp20.get_polling_rate(&w),
            None => Ok(None),
        }
    }

    /// Read onboard profiles from a HID++ 2.0 device (feature 0x8100).
    pub fn get_onboard_profiles(
        &self,
        device_name: &str,
    ) -> Result<Option<crate::onboard_profiles::OnboardProfiles>, Error> {
        match self.hidpp20_wrapper() {
            Some(w) => crate::onboard_profiles::OnboardProfiles::from_device(&w, device_name),
            None => Ok(None),
        }
    }

    /// Write onboard profiles back to a HID++ 2.0 device (feature 0x8100).
    pub fn write_onboard_profiles(
        &self,
        profiles: &crate::onboard_profiles::OnboardProfiles,
    ) -> Result<usize, Error> {
        match self.hidpp20_wrapper() {
            Some(w) => profiles.write(&w),
            None => Err(Error::Protocol("device does not support HID++ 2.0".into())),
        }
    }

    // ── Connection notifications (HID++ 1.0 devices) ─────────────────────────

    /// Enable or disable device (dis)connection notifications on this device
    /// (only meaningful for HID++ 1.0 devices connected through a receiver).
    pub fn enable_connection_notifications(
        &self,
        enable: bool,
    ) -> Result<Option<NotificationFlag>, Error> {
        if self.protocol.unwrap_or(0.0) >= 2.0 {
            return Ok(None);
        }
        let flags = if enable {
            NotificationFlag::BATTERY_STATUS
                | NotificationFlag::UI
                | NotificationFlag::CONFIGURATION_COMPLETE
        } else {
            NotificationFlag::empty()
        };
        let h10 = Hidpp10;
        h10.set_notification_flags(self, flags)?;
        h10.get_notification_flags(self)
    }

    // ── Notification handlers ─────────────────────────────────────────────────

    /// Register a notification callback under `id`.
    pub fn add_notification_handler(&self, id: String, handler: NotificationHandler) {
        if let Ok(mut handlers) = self.notification_handlers.lock() {
            handlers.insert(id, handler);
        }
    }

    /// Remove the notification callback registered under `id`.
    pub fn remove_notification_handler(&self, id: &str) {
        if let Ok(mut handlers) = self.notification_handlers.lock() {
            handlers.remove(id);
        }
    }

    /// Dispatch a notification to all registered handlers.
    ///
    /// Returns `Some(true)` if a handler consumed the notification.
    pub fn handle_notification(
        &self,
        notification: &crate::message::HidppNotification,
    ) -> Option<bool> {
        if let Ok(handlers) = self.notification_handlers.lock() {
            for handler in handlers.values() {
                if let Some(result) = handler(self, notification) {
                    return Some(result);
                }
            }
        }
        None
    }

    // ── State change callback ─────────────────────────────────────────────────

    /// Invoke the status callback when the device state changes.
    fn changed(&self, active: Option<bool>, reason: Option<String>) {
        if let Some(cb) = &self.status_callback {
            let _ = active; // `active` can be used by the callback via Device fields.
            cb(self, reason);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hidpp10Device impl for Device
// ─────────────────────────────────────────────────────────────────────────────

impl Hidpp10Device for Device {
    fn request(&self, request_id: u16, params: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        Device::request(self, request_id, params, None)
    }

    fn protocol(&self) -> f32 {
        self.protocol.unwrap_or(0.0)
    }

    fn is_device(&self) -> bool {
        true
    }

    fn registers(&self) -> &[Register] {
        &self.registers
    }

    fn add_register(&mut self, r: Register) {
        if !self.registers.contains(&r) {
            self.registers.push(r);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hidpp20Device wrapper for Device
// ─────────────────────────────────────────────────────────────────────────────

/// Thin wrapper that satisfies [`Hidpp20Device`] while holding a borrow to
/// the underlying [`Device`] and an optional notifications hook.
struct DeviceFeatureWrapper<'a> {
    device: &'a Device,
    hook: Option<&'a NotificationsHook>,
}

impl<'a> Hidpp20Device for DeviceFeatureWrapper<'a> {
    fn feature_request(
        &self,
        feature: SupportedFeature,
        function: u8,
        params: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.device
            .feature_request(feature, function, params, self.hook)
    }

    fn is_online(&self) -> bool {
        self.device.online
    }

    fn protocol(&self) -> f32 {
        self.device.protocol.unwrap_or(0.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FeaturesArray helper exposed so receiver code can check initialisation
// ─────────────────────────────────────────────────────────────────────────────
