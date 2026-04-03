// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Shared state types for solaarsd.

use std::sync::Arc;

use tokio::sync::Mutex;
use zvariant::ObjectPath;

use logitech_hidpp::receiver::Receiver;

/// Per-receiver runtime state kept in [`DaemonState`].
///
/// The `path`, `product_id` and `name` fields are cached copies of the
/// corresponding fields on [`Receiver`] so that they can be read without
/// acquiring the async mutex (e.g. inside a sync `std::sync::MutexGuard`).
#[derive(Clone)]
pub struct ReceiverState {
    pub receiver: Arc<Mutex<Receiver>>,
    pub index: usize,
    /// Slot numbers of the paired devices registered on D-Bus.
    pub device_numbers: Vec<u8>,
    /// Cached `Receiver::path` — the hidraw device node.
    pub path: String,
    /// Cached `Receiver::product_id`.
    pub product_id: u16,
    /// Cached `Receiver::name`.
    pub name: String,
}

/// Root daemon state.
#[derive(Default)]
pub struct DaemonState {
    pub receivers: Vec<ReceiverState>,
    /// Monotonically increasing counter used to assign stable D-Bus object
    /// indices to receivers.  Never reused, so paths remain stable within a
    /// session even after receiver reconnects.
    next_receiver_index: usize,
}

impl DaemonState {
    /// Allocate the next receiver index.
    pub fn next_receiver_index(&mut self) -> usize {
        let idx = self.next_receiver_index;
        self.next_receiver_index += 1;
        idx
    }
    /// Object path for receiver N.
    pub fn receiver_path<'a>(index: usize) -> ObjectPath<'a> {
        ObjectPath::try_from(format!("/org/solaarsd/receiver{index}")).unwrap()
    }

    /// Object path for device M on receiver N.
    pub fn device_path<'a>(receiver_index: usize, number: u8) -> ObjectPath<'a> {
        ObjectPath::try_from(format!(
            "/org/solaarsd/receiver{receiver_index}/dev{number}"
        ))
        .unwrap()
    }
}
