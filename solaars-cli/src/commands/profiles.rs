// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `profiles` subcommand – dump or load onboard device profiles.

use logitech_hidpp::onboard_profiles::{OnboardProfiles, ONBOARD_PROFILES_VERSION};
use logitech_hidpp::receiver::Receiver;

use crate::discovery::find_device;

pub fn run(
    receivers: &mut [Receiver],
    device_arg: &str,
    profiles_file: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, dev) = find_device(receivers, &device_arg.to_lowercase())
        .ok_or_else(|| format!("no online device found matching '{device_arg}'"))?;

    if !dev.online {
        return Err(format!("device '{}' is offline", device_arg).into());
    }

    let device_name = dev.codename.as_deref().unwrap_or(device_arg);

    // We need a Hidpp20Device view of the Device.  Use the built-in wrapper.
    let profiles = match dev.get_onboard_profiles(device_name) {
        Ok(Some(p)) => p,
        Ok(None) => {
            println!(
                "Device {} has no onboard profiles that solaars supports.",
                device_name
            );
            return Ok(());
        }
        Err(e) => {
            return Err(format!("failed to read profiles from {device_name}: {e}").into());
        }
    };

    match profiles_file {
        None => {
            // Dump profiles as YAML to stdout.
            println!("# Dumping profiles from {device_name}");
            let yaml = serde_yaml::to_string(&profiles)?;
            print!("{yaml}");
        }
        Some(path) => {
            // Load profiles from YAML file and write them to the device.
            let yaml_text = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read profiles file '{path}': {e}"))?;
            let loaded: OnboardProfiles = serde_yaml::from_str(&yaml_text)
                .map_err(|e| format!("invalid profiles file '{path}': {e}"))?;

            if loaded.version != ONBOARD_PROFILES_VERSION {
                return Err(format!(
                    "incompatible profile version {} in '{}' (expected {})",
                    loaded.version, path, ONBOARD_PROFILES_VERSION
                )
                .into());
            }
            if loaded.device_name != device_name {
                return Err(format!(
                    "profiles file '{}' was created for '{}', but connected device is '{}'",
                    path, loaded.device_name, device_name
                )
                .into());
            }

            println!("Reading profiles from {path}");
            println!("Loading profiles into {device_name}");
            let written = dev.write_onboard_profiles(&loaded)?;
            println!("Wrote {written} sector(s) to {device_name}");
        }
    }

    Ok(())
}
