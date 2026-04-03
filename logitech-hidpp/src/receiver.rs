// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Logitech HID++ receiver abstraction.
//!
//! A [`Receiver`] is a USB dongle that supports one or more wirelessly paired
//! devices.  It communicates at device number `0xFF` and exposes sub-registers
//! for discovering paired devices.

use std::sync::Arc;

use log::{info, warn};

use crate::base::{HidppDevice, RequestOptions};
use crate::base_usb::{ReceiverInfo, ReceiverKind};
// (Device is used indirectly via PairingInfo in device.rs)
use crate::error::Error;
use crate::hidpp10::{self, Hidpp10, Hidpp10Device};
use crate::hidpp10_constants::{InfoSubRegister, NotificationFlag, Register};
use crate::message::RECEIVER_DEVICE_NUMBER;

// ─────────────────────────────────────────────────────────────────────────────
// Pairing state machine
// ─────────────────────────────────────────────────────────────────────────────

/// Callback type for receiver state changes.
pub type ReceiverStatusCallback = Box<dyn Fn(&Receiver, Option<String>) + Send>;

/// State of the current or most-recent pairing operation.
#[derive(Debug, Default, Clone)]
pub struct Pairing {
    pub lock_open: bool,
    pub discovering: bool,
    pub counter: Option<u8>,
    pub device_address: Option<Vec<u8>>,
    pub device_authentication: Option<u8>,
    pub device_kind: Option<u8>,
    pub device_name: Option<String>,
    pub device_passkey: Option<String>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Receiver
// ─────────────────────────────────────────────────────────────────────────────

/// A Logitech USB receiver dongle.
///
/// Receivers communicate using HID++ 1.0 registers.  Paired wireless devices
/// are enumerated by querying `RECEIVER_INFO` sub-registers.
pub struct Receiver {
    hidpp: Arc<HidppDevice>,
    pub path: String,
    pub product_id: u16,
    pub kind: ReceiverKind,
    pub name: &'static str,
    pub may_unpair: bool,
    pub re_pairs: bool,
    pub serial: Option<String>,
    pub max_devices: u8,
    pub notification_flags: Option<NotificationFlag>,
    pub pairing: Pairing,
    /// Firmware info cache.
    firmware: Option<Vec<crate::common::FirmwareInfo>>,
    /// Remaining pairing slots cache.
    remaining_pairings: Option<i32>,
    /// Callback invoked when receiver state changes.
    pub status_callback: Option<ReceiverStatusCallback>,
}

impl Receiver {
    /// Open a receiver from its [`ReceiverInfo`] descriptor and an already-open
    /// [`HidppDevice`].
    pub fn new(
        hidpp: Arc<HidppDevice>,
        path: String,
        product_id: u16,
        info: &'static ReceiverInfo,
    ) -> Self {
        let mut r = Self {
            hidpp,
            path,
            product_id,
            kind: info.kind,
            name: info.name,
            may_unpair: info.may_unpair,
            re_pairs: info.re_pairs,
            serial: None,
            max_devices: info.max_devices,
            notification_flags: None,
            pairing: Pairing::default(),
            firmware: None,
            remaining_pairings: None,
            status_callback: None,
        };
        r.initialize(info);
        r
    }

    /// Read basic receiver info from RECEIVER_INFO registers.
    fn initialize(&mut self, info: &ReceiverInfo) {
        let reply = self.read_register_raw(
            Register::ReceiverInfo,
            &[InfoSubRegister::ReceiverInformation as u8],
        );
        if let Ok(Some(data)) = reply {
            self.serial = Some(hidpp10::extract_serial(&data));
            let max = hidpp10::extract_max_devices(&data);
            if (1..=6).contains(&max) {
                self.max_devices = max;
            }
        } else {
            self.max_devices = info.max_devices;
        }

        // Signal to the receiver that the software is present.
        let _ = hidpp10::set_configuration_pending_flags(self, 0xFF);
    }

    // ── Raw register I/O ─────────────────────────────────────────────────────

    fn read_register_raw(
        &self,
        register: Register,
        params: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        let request_id = 0x8100 | (register.as_u16() & 0x2FF);
        let opts = RequestOptions::default();
        self.hidpp
            .request(RECEIVER_DEVICE_NUMBER, request_id, params, &opts, None)
    }

    fn write_register_raw(
        &self,
        register: Register,
        value: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        let request_id = 0x8000 | (register.as_u16() & 0x2FF);
        let opts = RequestOptions::default();
        self.hidpp
            .request(RECEIVER_DEVICE_NUMBER, request_id, value, &opts, None)
    }

    // ── Paired device discovery ───────────────────────────────────────────────

    /// Read the codename of the device at slot `n`.
    pub fn device_codename(&self, n: u8) -> Result<Option<String>, Error> {
        let sub = InfoSubRegister::DeviceName as u8 + n - 1;
        let reply = self.read_register_raw(Register::ReceiverInfo, &[sub])?;
        Ok(reply.and_then(|data| hidpp10::extract_codename(&data)))
    }

    /// Count the number of currently connected devices.
    pub fn count(&self) -> u8 {
        let reply = self.read_register_raw(Register::ReceiverConnection, &[]);
        match reply {
            Ok(Some(data)) => hidpp10::extract_connection_count(&data),
            _ => 0,
        }
    }

    /// Read comprehensive pairing info for device slot `n` (1-based).
    ///
    /// Returns `None` if the slot is empty.
    pub fn paired_device_info(&self, n: u8) -> Result<Option<crate::device::PairingInfo>, Error> {
        use crate::hidpp10_constants::{DeviceKind, PowerSwitchLocation};

        let sub = InfoSubRegister::PairingInformation as u8 + n - 1;
        let pair_info = match self.read_register_raw(Register::ReceiverInfo, &[sub])? {
            Some(d) => d,
            None => return Ok(None),
        };

        let wpid = u16::from_be_bytes([
            pair_info.get(3).copied().unwrap_or(0),
            pair_info.get(4).copied().unwrap_or(0),
        ]);
        if wpid == 0 {
            return Ok(None);
        }

        let polling_rate = hidpp10::extract_polling_rate(&pair_info);
        let kind = DeviceKind::from(pair_info.get(7).copied().unwrap_or(0));

        let ext_sub = InfoSubRegister::ExtendedPairingInformation as u8 + n - 1;
        let ext_info = self.read_register_raw(Register::ReceiverInfo, &[ext_sub])?;

        let serial = ext_info.as_ref().map(|d| hidpp10::extract_serial(d));
        let power_switch = ext_info
            .as_ref()
            .map(|d| PowerSwitchLocation::from(hidpp10::extract_power_switch_location(d)));

        let codename = self.device_codename(n)?;

        Ok(Some(crate::device::PairingInfo {
            wpid,
            kind: Some(kind),
            serial,
            polling_rate: Some(polling_rate),
            power_switch,
            codename,
        }))
    }

    // ── Pairing ───────────────────────────────────────────────────────────────

    /// Open or close the pairing window on this receiver.
    ///
    /// `lock_closed = false` opens pairing; `lock_closed = true` closes it.
    /// `device` is the slot number (0 for any slot), `timeout` is seconds.
    pub fn set_lock(&self, lock_closed: bool, device: u8, timeout: u8) -> Result<(), Error> {
        let action: u8 = if lock_closed { 0x02 } else { 0x01 };
        self.write_register_raw(Register::ReceiverPairing, &[action, device, timeout])?;
        Ok(())
    }

    /// Unpair device at slot `number` from this receiver.
    pub fn unpair_device(&self, number: u8) -> Result<(), Error> {
        self.write_register_raw(Register::ReceiverPairing, &[0x03, number])?;
        Ok(())
    }

    /// Trigger the receiver to send link notifications for all currently
    /// paired devices.
    pub fn notify_devices(&self) -> Result<(), Error> {
        if self
            .write_register_raw(Register::ReceiverConnection, &[0x02])?
            .is_none()
        {
            warn!("{}: failed to trigger device link notifications", self.name);
        }
        Ok(())
    }

    // ── Firmware ─────────────────────────────────────────────────────────────

    /// Read (and cache) receiver firmware information.
    pub fn firmware(&mut self) -> Result<Option<&Vec<crate::common::FirmwareInfo>>, Error> {
        if self.firmware.is_none() {
            let h10 = Hidpp10;
            self.firmware = h10.get_firmware(self)?;
        }
        Ok(self.firmware.as_ref())
    }

    // ── Remaining pairings ────────────────────────────────────────────────────

    /// How many more devices can be paired (None = unknown, -1 = unlimited).
    pub fn remaining_pairings(&mut self, refresh: bool) -> Result<Option<i32>, Error> {
        if self.remaining_pairings.is_none() || refresh {
            let reply = self.read_register_raw(Register::ReceiverConnection, &[])?;
            if let Some(data) = reply {
                self.remaining_pairings = Some(hidpp10::extract_remaining_pairings(&data));
            }
        }
        Ok(self.remaining_pairings)
    }

    // ── Notification flags ────────────────────────────────────────────────────

    /// Enable or disable device (dis)connection notifications on this receiver.
    pub fn enable_connection_notifications(
        &self,
        enable: bool,
    ) -> Result<Option<NotificationFlag>, Error> {
        let flags = if enable {
            NotificationFlag::WIRELESS | NotificationFlag::SOFTWARE_PRESENT
        } else {
            NotificationFlag::empty()
        };
        let h10 = Hidpp10;
        let ok = h10.set_notification_flags(self, flags)?;
        if !ok {
            warn!(
                "{}: failed to {} receiver notifications",
                self.name,
                if enable { "enable" } else { "disable" }
            );
        }
        let current = h10.get_notification_flags(self)?;
        if let Some(f) = current {
            info!(
                "{}: receiver notifications {} => {:?}",
                self.name,
                if enable { "enabled" } else { "disabled" },
                f
            );
        }
        Ok(current)
    }

    // ── State change ─────────────────────────────────────────────────────────

    pub fn changed(&self, reason: Option<String>) {
        if let Some(cb) = &self.status_callback {
            cb(self, reason);
        }
    }

    // ── Underlying handle ─────────────────────────────────────────────────────

    /// Return a clone of the shared [`HidppDevice`] handle so that
    /// paired [`Device`] instances can share it.
    pub fn hidpp_handle(&self) -> Arc<HidppDevice> {
        Arc::clone(&self.hidpp)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hidpp10Device impl for Receiver
// ─────────────────────────────────────────────────────────────────────────────

impl Hidpp10Device for Receiver {
    fn request(&self, request_id: u16, params: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let opts = RequestOptions::default();
        self.hidpp
            .request(RECEIVER_DEVICE_NUMBER, request_id, params, &opts, None)
    }

    fn protocol(&self) -> f32 {
        // Receivers always speak HID++ 1.0 at the device-number level.
        1.0
    }

    fn is_device(&self) -> bool {
        false
    }

    fn registers(&self) -> &[Register] {
        // Receivers always have these registers available.
        &[]
    }

    fn add_register(&mut self, _r: Register) {
        // No-op for receivers.
    }
}
