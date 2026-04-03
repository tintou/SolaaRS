// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

use thiserror::Error;

/// Errors that may be produced by the HID++ communication layer.
#[derive(Debug, Error)]
pub enum Error {
    /// The receiver/device is no longer available (physically removed or driver unloaded).
    #[error("receiver no longer available: {0}")]
    NoReceiver(String),

    /// The requested device number is not paired to the receiver.
    #[error("no such device: number={number}, request={request:#06x}")]
    NoSuchDevice { number: u8, request: u16 },

    /// The device exists but is currently unreachable (e.g. turned off).
    #[error("device unreachable: number={number}, request={request:#06x}")]
    DeviceUnreachable { number: u8, request: u16 },

    /// The device returned a HID++ 2.0 feature call error.
    #[error("feature call error: number={number}, request={request:#06x}, error={error}")]
    FeatureCallError { number: u8, request: u16, error: u8 },

    /// An error from the underlying hidapi library.
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    /// A mutex was poisoned (a thread panicked while holding the lock).
    #[error("internal lock poisoned")]
    LockPoisoned,

    /// A protocol-level error (unexpected response, missing feature, etc.)
    #[error("protocol error: {0}")]
    Protocol(String),
}

// Re-export the canonical error code types from the constants modules so that
// callers don't need to import them separately.
pub use crate::hidpp10_constants::ErrorCode as Hidpp10ErrorCode;
pub use crate::hidpp20_constants::ErrorCode as Hidpp20ErrorCode;
