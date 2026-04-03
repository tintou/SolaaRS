// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Logitech HID++ protocol communication layer.
//!
//! This crate provides a Rust implementation of the Logitech HID++ low-level
//! communication protocol, using the [`hidapi`] crate for device I/O.
//!
//! # Structure
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`base`] | Core HID++ message read/write/request/ping ([`base::HidppDevice`]) |
//! | [`message`] | Message constants and [`message::HidppNotification`] |
//! | [`common`] | Shared types: [`common::Battery`], [`common::FirmwareInfo`], … |
//! | [`hidpp10_constants`] | HID++ 1.0 enums: [`hidpp10_constants::Register`], [`hidpp10_constants::NotificationFlag`], … |
//! | [`hidpp20_constants`] | HID++ 2.0 enums: [`hidpp20_constants::SupportedFeature`], [`hidpp20_constants::FeatureFlag`], … |
//! | [`base_usb`] | Known Logitech receiver product IDs: [`base_usb::ReceiverInfo`], [`base_usb::get_receiver_info`] |
//! | [`hidpp10`] | HID++ 1.0 protocol operations: [`hidpp10::read_register`], [`hidpp10::Hidpp10`] |
//! | [`hidpp20`] | HID++ 2.0 protocol: [`hidpp20::FeaturesArray`], [`hidpp20::Hidpp20`] |
//! | [`device`] | Device abstraction: [`device::Device`] |
//! | [`receiver`] | Receiver abstraction: [`receiver::Receiver`] |
//! | [`listener`] | Background notification listener: [`listener::EventsListener`] |
//! | [`error`] | Error types: [`error::Error`] |
//!
//! # Quick start
//!
//! ```no_run
//! use std::ffi::CString;
//! use std::sync::Arc;
//! use hidapi::HidApi;
//! use logitech_hidpp::base::HidppDevice;
//!
//! let api = HidApi::new().expect("failed to init hidapi");
//! let path = CString::new("/dev/hidraw0").expect("invalid path");
//! let raw = api.open_path(&path).expect("open failed");
//! let shared = Arc::new(HidppDevice::new(raw));
//!
//! // Ping device slot 1 to check connectivity and discover the HID++ version.
//! let version = shared.ping(1, false, None).expect("ping failed");
//! println!("device 1 protocol: {:?}", version);
//! ```

pub mod base;
pub mod base_usb;
pub mod common;
pub mod device;
pub mod error;
pub mod hidpp10;
pub mod hidpp10_constants;
pub mod hidpp20;
pub mod hidpp20_constants;
pub mod listener;
pub mod message;
pub mod onboard_profiles;
pub mod receiver;

// Top-level re-exports for the most common types.
pub use base::{HidppDevice, NotificationsHook, RequestOptions};
pub use common::{Battery, BatteryStatus, FirmwareInfo, FirmwareKind};
pub use device::Device;
pub use error::{Error, Hidpp10ErrorCode, Hidpp20ErrorCode};
pub use message::HidppNotification;
pub use receiver::Receiver;
