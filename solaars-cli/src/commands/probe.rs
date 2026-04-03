// Copyright (C) 2020  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `probe` subcommand – raw register dump for debugging.

use logitech_hidpp::hidpp10;
use logitech_hidpp::hidpp10_constants::{InfoSubRegister, Register};
use logitech_hidpp::message::RECEIVER_DEVICE_NUMBER;
use logitech_hidpp::receiver::Receiver;
use logitech_hidpp::{Hidpp10ErrorCode, RequestOptions};

use crate::commands::show::{print_device, print_receiver};
use crate::discovery::{enumerate_paired_devices, find_receiver_mut};

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

    let devices = enumerate_paired_devices(receiver);
    if devices.len() == 1 && receiver.max_devices == 1 {
        print_device(&devices[0], Some(1));
        return Ok(());
    }

    print_receiver(receiver);

    println!();
    println!("  Register Dump");

    let rgst = hidpp10::read_register(receiver, Register::Notifications, &[]);
    println!(
        "    Notifications         {:#04x}: {}",
        Register::Notifications as u16 & 0xFF,
        fmt_reg(&rgst)
    );

    let rgst = hidpp10::read_register(receiver, Register::ReceiverConnection, &[]);
    println!(
        "    Connection State      {:#04x}: {}",
        Register::ReceiverConnection as u16 & 0xFF,
        fmt_reg(&rgst)
    );

    let rgst = hidpp10::read_register(receiver, Register::DevicesActivity, &[]);
    println!(
        "    Device Activity       {:#04x}: {}",
        Register::DevicesActivity as u16 & 0xFF,
        fmt_reg(&rgst)
    );

    // Pairing sub-registers 0x00–0x0F.
    for sub_reg in 0u8..16 {
        let rgst = hidpp10::read_register(receiver, Register::ReceiverInfo, &[sub_reg]);
        println!(
            "    Pairing Register {:#04x} {:#04x}: {}",
            Register::ReceiverInfo as u16 & 0xFF,
            sub_reg,
            fmt_reg(&rgst)
        );
    }

    // Per-device pairing registers.
    for device in 0u8..7 {
        for base in [
            InfoSubRegister::PairingInformation as u8,
            InfoSubRegister::ExtendedPairingInformation as u8,
            InfoSubRegister::BoltPairingInformation as u8,
        ] {
            let sub = base + device;
            let rgst = hidpp10::read_register(receiver, Register::ReceiverInfo, &[sub]);
            println!(
                "    Pairing Register {:#04x} {:#04x}: {}",
                Register::ReceiverInfo as u16 & 0xFF,
                sub,
                fmt_reg(&rgst)
            );
        }

        let name_sub = InfoSubRegister::DeviceName as u8 + device;
        let rgst = hidpp10::read_register(receiver, Register::ReceiverInfo, &[name_sub]);
        let name_str = match &rgst {
            Ok(Some(data)) => {
                let len = data.get(1).copied().unwrap_or(0) as usize;
                String::from_utf8_lossy(data.get(2..2 + len).unwrap_or(&[])).to_string()
            }
            _ => "None".to_string(),
        };
        println!(
            "    Pairing Name     {:#04x} {:#02x}: {}",
            Register::ReceiverInfo as u16 & 0xFF,
            name_sub,
            name_str
        );

        for part in 1u8..4 {
            let bolt_sub = InfoSubRegister::BoltDeviceName as u8 + device;
            let rgst = hidpp10::read_register(receiver, Register::ReceiverInfo, &[bolt_sub, part]);
            let (len, s) = match &rgst {
                Ok(Some(data)) => {
                    let l = data.get(2).copied().unwrap_or(0) as usize;
                    (
                        l,
                        String::from_utf8_lossy(data.get(3..3 + l).unwrap_or(&[])).to_string(),
                    )
                }
                _ => (0, "None".to_string()),
            };
            println!(
                "    Pairing Name     {:#04x} {:#02x} {:#02x}: {:2} {}",
                Register::ReceiverInfo as u16 & 0xFF,
                bolt_sub,
                part,
                len,
                s
            );
        }
    }

    for sub_reg in 0u8..5 {
        let rgst = hidpp10::read_register(
            receiver,
            Register::ReceiverInfo,
            &[InfoSubRegister::FwVersion as u8 + sub_reg],
        );
        println!(
            "    Firmware         {:#04x} {:#04x}: {}",
            Register::ReceiverInfo as u16 & 0xFF,
            sub_reg,
            fmt_reg(&rgst)
        );
    }

    // Raw register sweep using low-level request with return_error=true.
    println!();
    let hidpp = receiver.hidpp_handle();
    'outer: for reg in 0u16..0xFF {
        for (offset, reg_type) in [(0x00u16, "Short"), (0x200u16, "Long")] {
            let mut last: Option<Vec<u8>> = None;
            for sub in 0u16..0xFF {
                let request_id = 0x8100u16 | (offset + reg);
                let opts = RequestOptions {
                    return_error: true,
                    ..Default::default()
                };
                let result = hidpp.request(
                    RECEIVER_DEVICE_NUMBER,
                    request_id,
                    &[sub as u8],
                    &opts,
                    None,
                );
                match result {
                    Ok(Some(data)) if data.len() == 1 => {
                        // return_error=true encodes HID++ 1.0 error code as a single byte.
                        let code = Hidpp10ErrorCode::from(data[0]);
                        if code == Hidpp10ErrorCode::InvalidAddress {
                            break;
                        }
                        if code == Hidpp10ErrorCode::InvalidValue {
                            continue;
                        }
                        // Other error codes: treat as empty and continue.
                        last = None;
                    }
                    Ok(Some(data)) => {
                        if last.as_deref() != Some(&data) {
                            println!(
                                "    Register {reg_type:<6} {reg:#04x} {sub:#04x}: 0x{}",
                                hex_str(&data)
                            );
                        }
                        last = Some(data);
                    }
                    Ok(None) | Err(_) => break,
                }
            }
            if reg == 0xFE && offset == 0x200 {
                break 'outer;
            }
        }
    }

    Ok(())
}

fn fmt_reg(r: &Result<Option<Vec<u8>>, logitech_hidpp::Error>) -> String {
    match r {
        Ok(Some(data)) => format!("0x{}", hex_str(data)),
        _ => "None".to_string(),
    }
}

fn hex_str(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}
