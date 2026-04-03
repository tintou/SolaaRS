// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `unpair` subcommand – unpair a device from its receiver.

use logitech_hidpp::receiver::Receiver;

use crate::discovery::find_device;

pub fn run(receivers: &mut [Receiver], device_arg: &str) -> Result<(), Box<dyn std::error::Error>> {
    let name = device_arg.to_lowercase();

    // find_device iterates receivers and returns (receiver_index, device).
    let (ri, dev) = find_device(receivers, &name)
        .ok_or_else(|| format!("no device found matching '{device_arg}'"))?;

    let receiver = &receivers[ri];

    if !receiver.may_unpair {
        eprintln!(
            "solaars: warning: receiver {:04x} for {} does not support unpairing, attempting anyway.",
            receiver.product_id,
            dev.codename.as_deref().unwrap_or("(unknown)")
        );
    }

    let number = dev.number;
    let codename = dev
        .codename
        .clone()
        .unwrap_or_else(|| "(unknown)".to_string());
    let wpid = dev.wpid.unwrap_or(0);
    let serial = dev.serial.clone().unwrap_or_else(|| "?".to_string());

    receiver.unpair_device(number)?;

    println!("Unpaired {number}: {codename} [{wpid:04X}:{serial}]");
    Ok(())
}
