// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Background event-listening thread for HID++ notifications.
//!
//! [`EventsListener`] runs as a daemon thread that continuously reads from a
//! HID++ device handle and forwards notifications to a user-supplied callback.
//!
//! Ported from `logitech_receiver/listener.py`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use log::{error, warn};

use crate::base::HidppDevice;
use crate::error::Error;
use crate::message::HidppNotification;

/// How long each `read` call blocks waiting for a packet before checking
/// whether the thread should stop.  Short enough to be responsive, long
/// enough not to busy-spin.
const EVENT_READ_TIMEOUT: Duration = Duration::from_secs(1);

/// Notification callback type: receives every decoded [`HidppNotification`]
/// produced while the listener is running.
pub type NotificationCallback = Box<dyn Fn(HidppNotification) + Send + 'static>;

/// Handle to a running [`EventsListener`] background thread.
#[derive(Debug)]
pub struct EventsListener {
    active: Arc<AtomicBool>,
    // Channel for re-queuing notifications that could not be delivered
    // immediately (e.g. because a handler triggered another notification).
    _requeue_tx: mpsc::SyncSender<HidppNotification>,
    join: Option<JoinHandle<()>>,
}

impl EventsListener {
    /// Spawn a listener thread that reads from `device` and calls `callback`
    /// for every decoded notification.
    ///
    /// The returned [`EventsListener`] must be kept alive for the duration of
    /// listening; dropping it requests the thread to stop.
    pub fn spawn(device: Arc<HidppDevice>, callback: NotificationCallback) -> Self {
        let active = Arc::new(AtomicBool::new(true));
        let active_clone = Arc::clone(&active);

        // Bounded channel for queued notifications (mirrors Python's Queue(16)).
        let (tx, rx) = mpsc::sync_channel::<HidppNotification>(16);
        let tx_clone = tx.clone();

        let join = thread::Builder::new()
            .name("hidpp-listener".into())
            .spawn(move || {
                run_listener(device, active_clone, tx_clone, rx, callback);
            })
            .expect("failed to spawn listener thread");

        Self {
            active,
            _requeue_tx: tx,
            join: Some(join),
        }
    }

    /// Request the listener thread to stop and wait for it to exit.
    pub fn stop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }

    /// Returns `true` while the listener thread is running.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

impl Drop for EventsListener {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Thread body
// ─────────────────────────────────────────────────────────────────────────────

fn run_listener(
    device: Arc<HidppDevice>,
    active: Arc<AtomicBool>,
    _tx: mpsc::SyncSender<HidppNotification>,
    rx: mpsc::Receiver<HidppNotification>,
    callback: NotificationCallback,
) {
    while active.load(Ordering::Relaxed) {
        // Deliver any re-queued notifications first.
        while let Ok(n) = rx.try_recv() {
            dispatch(&callback, n);
        }

        // Block-read with a short timeout so we can poll `active` regularly.
        match device.read(EVENT_READ_TIMEOUT) {
            Ok(Some((report_id, devnumber, data))) => {
                if let Some(n) = HidppNotification::from_raw(report_id, devnumber, &data) {
                    dispatch(&callback, n);
                }
            }
            Ok(None) => {
                // Timeout — loop and re-check `active`.
            }
            Err(Error::NoReceiver(reason)) => {
                warn!("listener: device disconnected: {reason}");
                active.store(false, Ordering::Relaxed);
                break;
            }
            Err(e) => {
                error!("listener: unexpected read error: {e}");
                active.store(false, Ordering::Relaxed);
                break;
            }
        }
    }
}

fn dispatch(callback: &NotificationCallback, n: HidppNotification) {
    // Any panic in the callback is caught so the listener thread survives.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(n)));
    if let Err(e) = result {
        error!("listener: callback panicked: {:?}", e);
    }
}
