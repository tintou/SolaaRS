// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `solaars` – command-line interface for Logitech HID++ device management.
//!
//! Mirrors the Python `solaar` CLI from `lib/solaar/cli/`.

use clap::{Parser, Subcommand};
use hidapi::HidApi;

mod commands;
mod discovery;
mod settings;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "solaars",
    version,
    about = "Manage Logitech HID++ devices",
    after_help = "For details on individual actions, run `solaars <action> --help`."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show information about device(s) or receiver(s).
    Show {
        /// Device to show: a slot number (1-6), serial, codename, name substring, or "all".
        #[arg(default_value = "all")]
        device: String,
    },

    /// Probe a receiver (raw register dump, debugging use only).
    Probe {
        /// Select receiver by name substring or serial when more than one is present.
        receiver: Option<String>,
    },

    /// Pair a new device with a receiver.
    ///
    /// The Logitech Unifying Receiver supports up to 6 paired devices at a time.
    Pair {
        /// Select receiver by name substring or serial when more than one is present.
        receiver: Option<String>,
    },

    /// Unpair a device from its receiver.
    Unpair {
        /// Device to unpair: a slot number (1-6), serial, or codename.
        device: String,
    },

    /// Print or load device-specific settings.
    Config {
        /// Device to configure.
        device: String,
        /// Setting name (omit to list all settings).
        setting: Option<String>,
        /// New value or key for keyed settings.
        value_key: Option<String>,
        /// Value for keyed settings or sub-key for sub-keyed settings.
        extra_subkey: Option<String>,
        /// Value for sub-keyed settings.
        extra2: Option<String>,
    },

    /// Print or load onboard profiles.
    Profiles {
        /// Device to read or load profiles.
        device: String,
        /// File containing YAML dump of profiles to load.
        profiles: Option<String>,
    },
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("solaars: error: failed to initialise HID API: {e}");
            std::process::exit(1);
        }
    };

    println!("solaars version {VERSION}");
    println!();

    let mut receivers = discovery::open_receivers(&api);

    if receivers.is_empty() {
        eprintln!(
            "solaars: error: no supported device found. \
             Use `lsusb` to list connected devices."
        );
        std::process::exit(1);
    }

    let result = match cli.command {
        Command::Show { device } => commands::show::run(&mut receivers, &device),
        Command::Probe { receiver } => commands::probe::run(&mut receivers, receiver.as_deref()),
        Command::Pair { receiver } => commands::pair::run(&mut receivers, receiver.as_deref()),
        Command::Unpair { device } => commands::unpair::run(&mut receivers, &device),
        Command::Config {
            device,
            setting,
            value_key,
            ..
        } => commands::config::run(
            &mut receivers,
            &device,
            setting.as_deref(),
            value_key.as_deref(),
        ),
        Command::Profiles { device, profiles } => {
            commands::profiles::run(&mut receivers, &device, profiles.as_deref())
        }
    };

    if let Err(e) = result {
        eprintln!("solaars: error: {e}");
        std::process::exit(1);
    }
}
