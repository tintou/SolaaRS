// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `org.solaarsd.Device1` — analogous to BlueZ's `org.bluez.Device1`.

use std::sync::Arc;

use log::warn;
use logitech_hidpp::common::BatteryStatus;
use logitech_hidpp::device::Device;
use logitech_hidpp::hidpp10_constants::DeviceKind;
use logitech_hidpp::receiver::Receiver;
use tokio::sync::Mutex;
use zbus::fdo;
use zbus::interface;
use zvariant::OwnedObjectPath;

/// D-Bus interface for a single Logitech HID++ paired device.
///
/// Exposed at `/org/solaarsd/receiver{N}/dev{M}`.
pub struct DeviceInterface {
    pub device: Mutex<Device>,
    /// D-Bus object path of the parent receiver (`Receiver` property).
    pub receiver: OwnedObjectPath,
    /// Shared HID++ handle of the parent receiver, used for unpair commands.
    pub receiver_handle: Arc<Mutex<Receiver>>,
}

#[interface(name = "org.solaarsd.Device1")]
impl DeviceInterface {
    /// Short model code, e.g. "MX Master 3".
    #[zbus(property)]
    async fn codename(&self) -> String {
        self.device
            .lock()
            .await
            .codename
            .clone()
            .unwrap_or_default()
    }

    #[zbus(property)]
    async fn serial(&self) -> String {
        self.device.lock().await.serial.clone().unwrap_or_default()
    }

    /// Device category: "mouse", "keyboard", "trackball", …
    #[zbus(property)]
    async fn kind(&self) -> &str {
        match self.device.lock().await.kind {
            Some(DeviceKind::Keyboard) => "keyboard",
            Some(DeviceKind::Mouse) => "mouse",
            Some(DeviceKind::Numpad) => "numpad",
            Some(DeviceKind::Presenter) => "presenter",
            Some(DeviceKind::Trackball) => "trackball",
            Some(DeviceKind::Touchpad) => "touchpad",
            Some(DeviceKind::Headset) => "headset",
            Some(DeviceKind::Receiver) => "receiver",
            _ => "unknown",
        }
    }

    /// Wireless Product ID (WPID).
    #[zbus(property)]
    async fn wpid(&self) -> u16 {
        self.device.lock().await.wpid.unwrap_or(0)
    }

    /// Report interval in milliseconds (0 = unknown).
    #[zbus(property)]
    async fn polling_rate(&self) -> u8 {
        self.device.lock().await.polling_rate.unwrap_or(0)
    }

    /// Object path of the parent receiver.
    #[zbus(property)]
    fn receiver(&self) -> OwnedObjectPath {
        self.receiver.clone()
    }

    #[zbus(property)]
    async fn connected(&self) -> bool {
        self.device.lock().await.online
    }

    /// Battery percentage, or -1 when unavailable.
    #[zbus(property)]
    async fn battery_level(&self) -> i32 {
        let device = self.device.lock().await;
        if device.online {
            match device.get_battery_hidpp20() {
                Ok(battery) => battery.and_then(|b| b.level).map_or(-1, |l| l as i32),
                Err(e) => {
                    warn!("{e}");
                    -1
                }
            }
        } else {
            -1
        }
    }

    /// "full", "almost_full", "good", "slow_recharge", "recharging",
    /// "discharging", "unknown".
    #[zbus(property)]
    async fn battery_status(&self) -> &str {
        let device = self.device.lock().await;
        if device.online {
            match device.get_battery_hidpp20() {
                Ok(battery) => match battery.and_then(|b| b.status) {
                    Some(BatteryStatus::Full) => "full",
                    Some(BatteryStatus::AlmostFull) => "almost_full",
                    Some(BatteryStatus::Recharging) => "recharging",
                    Some(BatteryStatus::SlowRecharge) => "slow_recharge",
                    Some(BatteryStatus::Discharging) => "discharging",
                    Some(BatteryStatus::InvalidBattery) => "invalid",
                    Some(BatteryStatus::ThermalError) => "thermal_error",
                    _ => "unknown",
                },
                Err(e) => {
                    warn!("{e}");
                    "unknown"
                }
            }
        } else {
            "unknown"
        }
    }

    // ── Methods ───────────────────────────────────────────────────────────────

    /// Unpair this device from its receiver.
    ///
    /// Instructs the receiver to remove the device from its pairing table.
    /// The corresponding D-Bus object is removed and `InterfacesRemoved` is
    /// emitted by the daemon's hotplug departure handler once the hidraw node
    /// disappears.
    ///
    /// Returns an error if the HID++ unpair command fails.
    async fn unpair(&self) -> fdo::Result<()> {
        let number = self.device.lock().await.number;

        let recv = self.receiver_handle.lock().await;
        if !recv.may_unpair {
            warn!(
                "receiver {:04X} does not advertise unpairing support for device {number}; \
                 attempting anyway",
                recv.product_id,
            );
        }
        recv.unpair_device(number)
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }
}
