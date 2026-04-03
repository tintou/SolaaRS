// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `config` subcommand – read and write device settings.

use logitech_hidpp::receiver::Receiver;

use crate::discovery::find_device;
use crate::settings::{
    all_settings, read_setting, supported_settings, validator_kind_str, validator_values_str,
    write_setting,
};

pub fn run(
    receivers: &mut [Receiver],
    device_arg: &str,
    setting_name: Option<&str>,
    value_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, dev) = find_device(receivers, &device_arg.to_lowercase())
        .ok_or_else(|| format!("no online device found matching '{device_arg}'"))?;

    if !dev.online {
        return Err(format!("device '{device_arg}' is offline").into());
    }

    let defs = all_settings();

    // ── List all supported settings ──────────────────────────────────────────
    if setting_name.is_none() {
        let supported = supported_settings(&dev, &defs);
        if supported.is_empty() {
            return Err(format!("no configurable settings found for '{}'", device_arg).into());
        }

        let codename = dev.codename.as_deref().unwrap_or("(unknown)");
        let serial = dev.serial.as_deref().unwrap_or("?");
        let wpid = dev.wpid.unwrap_or(0);
        println!("{codename} [{wpid:04X}:{serial}]");

        for def in supported {
            println!();
            println!("# {}", def.label);
            if !def.description.is_empty() {
                println!("# {}", def.description);
            }
            println!(
                "# possible values: {}",
                validator_values_str(&def.validator)
            );
            match read_setting(&dev, def) {
                Ok(Some(v)) => println!("{} = {v}", def.name),
                Ok(None) => println!("{} = ? (not supported)", def.name),
                Err(e) => println!("{} = ? (error: {e})", def.name),
            }
        }
        return Ok(());
    }

    let name = setting_name.unwrap().to_lowercase();

    // Find the matching setting definition that this device supports.
    let matching: Vec<_> = supported_settings(&dev, &defs)
        .into_iter()
        .filter(|d| d.name == name)
        .collect();

    if matching.is_empty() {
        // Give a better error: was the name valid at all?
        let any = defs.iter().any(|d| d.name == name);
        if any {
            return Err(
                format!("setting '{name}' exists but is not supported by this device").into(),
            );
        }
        return Err(format!("unknown setting '{name}'. Run `solaars config {device_arg}` to list available settings.").into());
    }

    // ── Read ─────────────────────────────────────────────────────────────────
    if value_str.is_none() {
        let def = matching[0];
        println!("# {}", def.label);
        println!("# {}", def.description);
        println!(
            "# kind: {} | possible values: {}",
            validator_kind_str(&def.validator),
            validator_values_str(&def.validator)
        );
        match read_setting(&dev, def) {
            Ok(Some(v)) => println!("{} = {v}", def.name),
            Ok(None) => println!("{} = ? (not supported by device)", def.name),
            Err(e) => return Err(format!("failed to read '{}': {e}", def.name).into()),
        }
        return Ok(());
    }

    // ── Write ─────────────────────────────────────────────────────────────────
    let val = value_str.unwrap();
    let def = matching[0];

    println!(
        "Setting {} of {} to {val}",
        def.name,
        dev.codename.as_deref().unwrap_or(device_arg)
    );

    match write_setting(&dev, def, val) {
        Ok(true) => {
            // Verify by reading back.
            match read_setting(&dev, def) {
                Ok(Some(v)) => println!("{} = {v}", def.name),
                _ => println!("(write sent; could not read back value)"),
            }
        }
        Ok(false) => {
            return Err(format!("device did not accept the new value for '{}'", def.name).into())
        }
        Err(e) => return Err(format!("failed to write '{}': {e}", def.name).into()),
    }

    Ok(())
}
