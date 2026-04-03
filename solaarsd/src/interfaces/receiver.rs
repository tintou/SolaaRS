// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `org.solaarsd.Receiver1` — analogous to BlueZ's `org.bluez.Adapter1`.

use std::sync::Arc;

use logitech_hidpp::hidpp10::Hidpp10;
use logitech_hidpp::hidpp10_constants::NotificationFlag;
use logitech_hidpp::receiver::Receiver;
use tokio::sync::Mutex;
use zbus::fdo;
use zbus::interface;
use zbus::object_server::SignalEmitter;

const PAIR_TIMEOUT_SECS: u8 = 30;

/// D-Bus interface for a Logitech receiver (HID++ dongle).
///
/// Exposed at `/org/solaarsd/receiver{N}`.
pub struct ReceiverInterface {
    pub receiver: Arc<Mutex<Receiver>>,
}

#[interface(name = "org.solaarsd.Receiver1")]
impl ReceiverInterface {
    /// Human-readable model name, e.g. "Unifying Receiver".
    #[zbus(property)]
    async fn name(&self) -> &str {
        &self.receiver.lock().await.name
    }

    /// HID device node, e.g. "/dev/hidraw0".
    #[zbus(property)]
    async fn address(&self) -> String {
        self.receiver.lock().await.path.clone()
    }

    #[zbus(property)]
    async fn product_id(&self) -> u16 {
        self.receiver.lock().await.product_id
    }

    #[zbus(property)]
    async fn max_devices(&self) -> u8 {
        self.receiver.lock().await.max_devices
    }

    /// Whether the receiver is currently in pairing mode.
    ///
    /// Becomes `true` after `StartDiscovery()` and `false` after
    /// `StopDiscovery()` or when the pairing timeout expires.
    #[zbus(property)]
    async fn discovering(&self) -> bool {
        self.receiver.lock().await.pairing.discovering
    }

    // ── Methods ───────────────────────────────────────────────────────────────

    /// Open the receiver's pairing window for [`PAIR_TIMEOUT_SECS`] seconds.
    ///
    /// Sets `Discovering` to `true` and emits `PropertiesChanged`.
    /// Returns an error if the HID++ pairing command fails.
    async fn start_discovery(
        &self,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        let mut recv = self.receiver.lock().await;

        // Ensure wireless link notifications are enabled so the daemon can
        // detect when a new device pairs.
        let h10 = Hidpp10;
        let old_flags = h10
            .get_notification_flags(&*recv)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?
            .unwrap_or(NotificationFlag::empty());
        if !old_flags.contains(NotificationFlag::WIRELESS) {
            let _ = h10.set_notification_flags(&*recv, old_flags | NotificationFlag::WIRELESS);
        }

        recv.set_lock(false, 0, PAIR_TIMEOUT_SECS)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        recv.pairing.discovering = true;
        drop(recv);

        self.discovering_changed(&ctxt).await?;
        Ok(())
    }

    /// Close the receiver's pairing window.
    ///
    /// Sets `Discovering` to `false` and emits `PropertiesChanged`.
    async fn stop_discovery(
        &self,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        let mut recv = self.receiver.lock().await;
        recv.set_lock(true, 0, 0)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        recv.pairing.discovering = false;
        drop(recv);

        self.discovering_changed(&ctxt).await?;
        Ok(())
    }
}
