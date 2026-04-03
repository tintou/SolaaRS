// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `pair` subcommand – pair a new device with a receiver.

use std::time::{Duration, Instant};

use logitech_hidpp::hidpp10::Hidpp10;
use logitech_hidpp::hidpp10_constants::NotificationFlag;
use logitech_hidpp::message::HidppNotification;
use logitech_hidpp::receiver::Receiver;
use logitech_hidpp::HidppDevice;

use crate::discovery::find_receiver_mut;

const PAIR_TIMEOUT_SECS: u64 = 30;

pub fn run(
    receivers: &mut [Receiver],
    receiver_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let receiver = if let Some(name) = receiver_name {
        find_receiver_mut(receivers, name)
            .ok_or_else(|| format!("no receiver found matching '{name}'"))?
    } else {
        receivers.first_mut().ok_or("no receivers found")?
    };

    let h10 = Hidpp10;

    // Save notification flags so we can restore them afterward.
    let old_flags = h10
        .get_notification_flags(receiver)?
        .unwrap_or(NotificationFlag::empty());
    if !old_flags.contains(NotificationFlag::WIRELESS) {
        h10.set_notification_flags(receiver, old_flags | NotificationFlag::WIRELESS)?;
    }

    // Collect already-paired slot numbers.
    let known: Vec<u8> = (1..=receiver.max_devices)
        .filter(|&n| receiver.paired_device_info(n).ok().flatten().is_some())
        .collect();

    // Open the pairing window.
    if let Err(e) = receiver.set_lock(false, 0, PAIR_TIMEOUT_SECS as u8) {
        eprintln!("solaars: warning: could not open pairing window: {e}");
    }

    println!("Pairing: Turn your device on or press, hold, and release");
    println!("a channel button or the channel switch button.");
    println!("Timing out in {PAIR_TIMEOUT_SECS} seconds.");

    let hidpp_handle = receiver.hidpp_handle();
    let max_devices = receiver.max_devices;
    let new_device_slot = poll_for_new_device(&hidpp_handle, &known, max_devices);

    // Restore notification flags.
    if !old_flags.contains(NotificationFlag::WIRELESS) {
        let _ = h10.set_notification_flags(receiver, old_flags);
    }

    match new_device_slot {
        Some(n) => {
            if let Ok(Some(info)) = receiver.paired_device_info(n) {
                let codename = info.codename.as_deref().unwrap_or("(unknown)");
                let serial = info.serial.as_deref().unwrap_or("?");
                println!("Paired device {n}: {codename} [{:04X}:{serial}]", info.wpid);
            } else {
                println!("Paired device {n}");
            }
        }
        None => {
            println!("Pairing timed out — no new device was detected.");
        }
    }

    Ok(())
}

/// Poll for a connection notification indicating a newly paired device.
///
/// Returns the slot number of the new device, or `None` on timeout.
fn poll_for_new_device(hidpp: &HidppDevice, known: &[u8], max_devices: u8) -> Option<u8> {
    let deadline = Instant::now() + Duration::from_secs(PAIR_TIMEOUT_SECS + 5);

    while Instant::now() < deadline {
        // read() returns (report_id, devnumber, data)
        if let Ok(Some((report_id, devnumber, data))) = hidpp.read(Duration::from_millis(500)) {
            if let Some(n) = HidppNotification::from_raw(report_id, devnumber, &data) {
                // sub_id 0x41 = device connection notification.
                if n.sub_id == 0x41
                    && !known.contains(&n.devnumber)
                    && (1..=max_devices).contains(&n.devnumber)
                {
                    return Some(n.devnumber);
                }
            }
        }
    }
    None
}
