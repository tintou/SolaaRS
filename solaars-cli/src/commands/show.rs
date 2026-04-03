// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `show` subcommand – display information about receivers and devices.

use std::sync::Arc;

use logitech_hidpp::device::Device;
use logitech_hidpp::hidpp10::{self, Hidpp10};
use logitech_hidpp::hidpp10_constants::{NotificationFlag, Register};
use logitech_hidpp::message::LOGITECH_VENDOR_ID;
use logitech_hidpp::receiver::Receiver;

use crate::discovery::enumerate_paired_devices;

pub fn run(receivers: &mut [Receiver], device_arg: &str) -> Result<(), Box<dyn std::error::Error>> {
    let name = device_arg.to_lowercase();

    if name == "all" {
        for receiver in receivers.iter_mut() {
            print_receiver(receiver);
            let devices = enumerate_paired_devices(receiver);
            for dev in &devices {
                println!();
                print_device(dev, Some(dev.number));
            }
            println!();
        }
        return Ok(());
    }

    // Try to match a receiver by name or serial (need mut for firmware()).
    let recv_match = receivers.iter().position(|r| {
        r.name.to_lowercase().contains(&name)
            || r.serial
                .as_deref()
                .map(|s| s.to_lowercase() == name)
                .unwrap_or(false)
    });
    if let Some(idx) = recv_match {
        print_receiver(&mut receivers[idx]);
        return Ok(());
    }

    // Try to match a device across all receivers.
    for receiver in receivers.iter() {
        let handle = receiver.hidpp_handle();
        for n in 1..=receiver.max_devices {
            let pairing_info = match receiver.paired_device_info(n) {
                Ok(Some(i)) => i,
                _ => continue,
            };

            let codename_match = pairing_info
                .codename
                .as_deref()
                .map(|c| c.to_lowercase() == name)
                .unwrap_or(false);
            let serial_match = pairing_info
                .serial
                .as_deref()
                .map(|s| s.to_lowercase() == name)
                .unwrap_or(false);
            let slot_match = name.parse::<u8>().ok().map(|s| s == n).unwrap_or(false);
            let name_substr = pairing_info
                .codename
                .as_deref()
                .map(|c| c.to_lowercase().contains(&name))
                .unwrap_or(false);

            if codename_match || serial_match || slot_match || name_substr {
                let mut dev = Device::with_receiver(Arc::clone(&handle), n, false, pairing_info);
                let _ = dev.ping(None);
                print_device(&dev, Some(n));
                return Ok(());
            }
        }
    }

    Err(format!("no device found matching '{device_arg}'").into())
}

// ──────────────────────────
// Receiver display
// ─────────────────────────────────────────────────────

pub fn print_receiver(receiver: &mut Receiver) {
    let paired_count = receiver.count();

    println!("{}", receiver.name);
    println!("  Device path  : {}", receiver.path);
    println!(
        "  USB id       : {:04x}:{:04x}",
        LOGITECH_VENDOR_ID, receiver.product_id
    );
    if let Some(ref serial) = receiver.serial.clone() {
        println!("  Serial       : {serial}");
    }

    if let Ok(Some(fw_list)) = receiver.firmware() {
        for fw in fw_list.clone() {
            println!("    {:<11}: {}", format!("{:?}", fw.kind), fw.version);
        }
    }

    println!(
        "  Has {} paired device(s) out of a maximum of {}.",
        paired_count, receiver.max_devices
    );

    if let Ok(Some(remaining)) = receiver.remaining_pairings(false) {
        if remaining >= 0 {
            println!("  Has {remaining} successful pairing(s) remaining.");
        }
    }

    let h10 = Hidpp10;
    if let Ok(Some(flags)) = h10.get_notification_flags(receiver) {
        if flags.is_empty() {
            println!("  Notifications: (none)");
        } else {
            let names = notification_flag_names(flags);
            println!(
                "  Notifications: {} ({:#08X})",
                names.join(", "),
                flags.bits()
            );
        }
    }

    if let Ok(Some(activity)) = hidpp10::read_register(receiver, Register::DevicesActivity, &[]) {
        let pairs: Vec<String> = (1..receiver.max_devices)
            .filter_map(|d| {
                let a = *activity.get((d - 1) as usize)?;
                if a > 0 {
                    Some(format!("{d}={a}"))
                } else {
                    None
                }
            })
            .collect();
        if pairs.is_empty() {
            println!("  Device activity counters: (empty)");
        } else {
            println!("  Device activity counters: {}", pairs.join(", "));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Device display
// ─────────────

pub fn print_device(dev: &Device, num: Option<u8>) {
    let n = num.unwrap_or(dev.number);
    let display_name = dev.codename.as_deref().unwrap_or("(unknown)");

    if n > 0 && n < 8 {
        println!("  {n}: {display_name}");
    } else {
        println!("{display_name}");
    }

    if let Some(ref path) = dev.path {
        println!("     Device path  : {path}");
    }
    if let Some(wpid) = dev.wpid {
        println!("     WPID         : {wpid:04X}");
    }
    if let Some(pid) = dev.product_id {
        println!("     USB id       : {:04x}:{pid:04x}", LOGITECH_VENDOR_ID);
    }
    if let Some(ref codename) = dev.codename {
        println!("     Codename     : {codename}");
    }
    if let Some(ref kind) = dev.kind {
        println!("     Kind         : {kind:?}");
    }
    match dev.protocol {
        Some(p) => println!("     Protocol     : HID++ {p:.1}"),
        None => println!("     Protocol     : unknown (device is offline)"),
    }
    if let Some(rate) = dev.polling_rate {
        if rate > 0 {
            println!("     Report Rate  : {rate}ms");
        }
    }
    if let Some(ref serial) = dev.serial {
        println!("     Serial number: {serial}");
    }

    if let Ok(Some(fw_list)) = dev.read_firmware(None) {
        for fw in &fw_list {
            let name_ver = format!("{} {}", fw.name, fw.version).trim().to_string();
            println!("       {:<11}: {}", format!("{:?}", fw.kind), name_ver);
        }
    }

    if let Some(ref loc) = dev.power_switch_location {
        println!("     The power switch is located on the {loc:?}.");
    }

    if dev.online {
        let h10 = Hidpp10;
        if let Ok(Some(flags)) = h10.get_notification_flags(dev) {
            if flags.is_empty() {
                println!("     Notifications: (none).");
            } else {
                let names = notification_flag_names(flags);
                println!(
                    "     Notifications: {} ({:#08X}).",
                    names.join(", "),
                    flags.bits()
                );
            }
        }
        if let Ok(Some(df)) = h10.get_device_features(dev) {
            if df == 0 {
                println!("     Features: (none)");
            } else {
                println!("     Features: {df:#010X}");
            }
        }
    }

    if dev.online && dev.protocol.unwrap_or(0.0) >= 2.0 {
        print_hidpp20_info(dev);
    }

    if dev.online {
        print_battery_line(dev);
    } else {
        println!("     Battery: unknown (device is offline).");
    }
}

fn print_battery_line(dev: &Device) {
    if let Ok(Some(b)) = dev.get_battery_hidpp20() {
        let level_str = b
            .level
            .map(|l| format!("{l}%"))
            .unwrap_or_else(|| "N/A".to_string());
        let voltage_str = b.voltage.map(|v| format!(" {v}mV")).unwrap_or_default();
        let next_str = b
            .next_level
            .map(|n| format!(", next level {n}%"))
            .unwrap_or_default();
        println!(
            "     Battery: {level_str}{voltage_str}, {:?}{next_str}.",
            b.status
        );
        return;
    }
    if let Some(b) = dev.battery_info() {
        let level_str = b
            .level
            .map(|l| format!("{l}%"))
            .unwrap_or_else(|| "N/A".to_string());
        let voltage_str = b.voltage.map(|v| format!(" {v}mV")).unwrap_or_default();
        let next_str = b
            .next_level
            .map(|n| format!(", next level {n}%"))
            .unwrap_or_default();
        println!(
            "     Battery: {level_str}{voltage_str}, {:?}{next_str}.",
            b.status
        );
        return;
    }
    println!("     Battery status unavailable.");
}

fn print_hidpp20_info(dev: &Device) {
    if let Ok(Some(name)) = dev.get_name_hidpp20() {
        println!("     Name (2.0)   : {name}");
    }
    if let Ok(Some(friendly)) = dev.get_friendly_name_hidpp20() {
        println!("     Friendly name: {friendly}");
    }
    if let Ok(Some(kind)) = dev.get_kind_hidpp20() {
        println!("     Kind (2.0)   : {kind:?}");
    }
    if let Ok(Some(rate)) = dev.get_polling_rate_hidpp20() {
        println!("     Report Rate  : {rate}ms (HID++ 2.0)");
    }
}

fn notification_flag_names(flags: NotificationFlag) -> Vec<&'static str> {
    let all: &[(NotificationFlag, &str)] = &[
        (NotificationFlag::BATTERY_STATUS, "battery status"),
        (
            NotificationFlag::KEYBOARD_MULTIMEDIA_RAW,
            "keyboard multimedia raw",
        ),
        (NotificationFlag::SOFTWARE_PRESENT, "software present"),
        (NotificationFlag::UI, "ui"),
        (NotificationFlag::WIRELESS, "wireless"),
        (
            NotificationFlag::CONFIGURATION_COMPLETE,
            "configuration complete",
        ),
    ];
    all.iter()
        .filter(|(f, _)| flags.contains(*f))
        .map(|(_, name)| *name)
        .collect()
}
