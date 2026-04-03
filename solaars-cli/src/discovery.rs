// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Device and receiver discovery helpers.

use std::sync::Arc;

use hidapi::HidApi;
use logitech_hidpp::base::HidppDevice;
use logitech_hidpp::base_usb::{get_receiver_info, is_receiver_product_id};
use logitech_hidpp::device::Device;
use logitech_hidpp::message::LOGITECH_VENDOR_ID;
use logitech_hidpp::receiver::Receiver;

/// Open all connected Logitech receivers.
///
/// Iterates through all HID devices, filters for known Logitech receivers,
/// and returns a list of initialised [`Receiver`] instances — one per unique
/// device path (hidapi lists every HID interface of a USB device separately,
/// so we deduplicate by path to avoid opening the same receiver multiple times).
pub fn open_receivers(api: &HidApi) -> Vec<Receiver> {
    let mut receivers = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for dev_info in api.device_list() {
        if dev_info.vendor_id() != LOGITECH_VENDOR_ID {
            continue;
        }
        let pid = dev_info.product_id();
        if !is_receiver_product_id(pid) {
            continue;
        }

        // Among the multiple HID interfaces that hidapi enumerates for the
        // same physical USB device, prefer the vendor-specific usage page
        // (0xFF00) which is the HID++ interface.  Fall back to interface 2
        // (Unifying) or 0 (Nano/Bolt).  Skip any other interfaces.
        let usage_page = dev_info.usage_page();
        let iface = dev_info.interface_number();
        if usage_page != 0xFF00 && iface != 2 && iface != 0 {
            continue;
        }

        let path = dev_info.path().to_string_lossy().to_string();

        // Skip if we already opened a handle for this path.
        if !seen_paths.insert(path.clone()) {
            continue;
        }

        let info = match get_receiver_info(pid) {
            Some(i) => i,
            None => continue,
        };

        match api.open_path(dev_info.path()) {
            Ok(raw) => {
                let shared = Arc::new(HidppDevice::new(raw));
                let r = Receiver::new(shared, path, pid, info);
                receivers.push(r);
            }
            Err(e) => {
                eprintln!("solaars: warning: could not open {path}: {e}");
            }
        }
    }

    receivers
}

/// Try to create a [`Device`] for each paired slot of a receiver.
///
/// Pings each slot 1..=max_devices and returns only those that respond.
pub fn enumerate_paired_devices(receiver: &Receiver) -> Vec<Device> {
    let mut devices = Vec::new();
    let handle = receiver.hidpp_handle();

    for n in 1..=receiver.max_devices {
        let pairing_info = match receiver.paired_device_info(n) {
            Ok(Some(info)) => info,
            _ => continue,
        };

        let mut dev = Device::with_receiver(Arc::clone(&handle), n, false, pairing_info);

        // Ping to confirm the device is online and learn the protocol version.
        let _ = dev.ping(None);
        devices.push(dev);
    }

    devices
}

/// Find a receiver mutably by name substring or serial number.
pub fn find_receiver_mut<'a>(
    receivers: &'a mut [Receiver],
    name: &str,
) -> Option<&'a mut Receiver> {
    let name = name.to_lowercase();
    receivers.iter_mut().find(|r| {
        r.name.to_lowercase().contains(&name)
            || r.serial
                .as_deref()
                .map(|s| s.to_lowercase() == name)
                .unwrap_or(false)
    })
}

/// Find a paired device across all receivers.
///
/// `name` may be a device slot number (1-6), a serial number (case-insensitive),
/// a codename, or a substring of the device name.
pub fn find_device(receivers: &[Receiver], name: &str) -> Option<(usize, Device)> {
    let name_lc = name.to_lowercase();

    // Try to parse as a slot number (1-6).
    let slot: Option<u8> = name_lc.parse().ok().filter(|&n: &u8| (1..=6).contains(&n));

    for (ri, receiver) in receivers.iter().enumerate() {
        let handle = receiver.hidpp_handle();

        // If name is a number, try that slot directly first.
        if let Some(n) = slot {
            if let Ok(Some(info)) = receiver.paired_device_info(n) {
                let mut dev = Device::with_receiver(Arc::clone(&handle), n, false, info);
                let _ = dev.ping(None);
                return Some((ri, dev));
            }
        }

        // Otherwise search all paired slots.
        for n in 1..=receiver.max_devices {
            let pairing_info = match receiver.paired_device_info(n) {
                Ok(Some(i)) => i,
                _ => continue,
            };

            let codename_match = pairing_info
                .codename
                .as_deref()
                .map(|c| c.to_lowercase() == name_lc)
                .unwrap_or(false);
            let serial_match = pairing_info
                .serial
                .as_deref()
                .map(|s| s.to_lowercase() == name_lc)
                .unwrap_or(false);

            if codename_match || serial_match {
                let mut dev = Device::with_receiver(Arc::clone(&handle), n, false, pairing_info);
                let _ = dev.ping(None);
                return Some((ri, dev));
            }
        }
    }

    None
}
