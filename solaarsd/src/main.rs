// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! `solaarsd` — D-Bus daemon for Logitech HID++ device management.
//!
//! Exposes Logitech receivers and their paired devices on the system D-Bus,
//! mirroring the structure BlueZ uses for Bluetooth:
//!
//! ```text
//! /org/solaarsd                        ← ObjectManager
//! /org/solaarsd/receiver0              ← Receiver1 (like Adapter1)
//! /org/solaarsd/receiver0/dev01        ← Device1   (like Device1)
//! …
//! ```
//!
//! Receiver plug/unplug is detected via `rusb` USB hotplug notifications
//! (requires libusb hotplug support, available on Linux/macOS).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hidapi::HidApi;
use log::{info, warn};
use rusb::{GlobalContext, Hotplug, HotplugBuilder, UsbContext as _};
use tokio::sync::mpsc::{self, UnboundedSender};
use zbus::connection::Builder;
use zbus::fdo::ObjectManager;
use zvariant::OwnedObjectPath;

use logitech_hidpp::base::HidppDevice;
use logitech_hidpp::base_usb::{get_receiver_info, is_receiver_product_id};
use logitech_hidpp::device::Device;
use logitech_hidpp::hidpp10_constants::SUB_ID_LOCK_INFORMATION;
use logitech_hidpp::listener::EventsListener;
use logitech_hidpp::message::{LOGITECH_VENDOR_ID, RECEIVER_DEVICE_NUMBER};
use logitech_hidpp::receiver::Receiver;

mod interfaces;
mod state;

use interfaces::{device::DeviceInterface, receiver::ReceiverInterface};
use state::{DaemonState, ReceiverState};

const BUS_NAME: &str = "org.solaarsd";
const ROOT_PATH: &str = "/org/solaarsd";

// ── Hotplug event types ───────────────────────────────────────────────────────

/// Events sent from the rusb hotplug thread to the tokio event handler.
enum HotplugEvent {
    ReceiverArrived,
    ReceiverLeft { product_id: u16 },
}

/// rusb hotplug callback — filters for Logitech receiver devices and forwards
/// events to the async runtime via an mpsc channel.
struct HotplugHandler {
    tx: UnboundedSender<HotplugEvent>,
}

impl Hotplug<GlobalContext> for HotplugHandler {
    fn device_arrived(&mut self, device: rusb::Device<GlobalContext>) {
        if let Ok(desc) = device.device_descriptor() {
            if is_receiver_product_id(desc.product_id()) {
                let _ = self.tx.send(HotplugEvent::ReceiverArrived);
            }
        }
    }

    fn device_left(&mut self, device: rusb::Device<GlobalContext>) {
        if let Ok(desc) = device.device_descriptor() {
            if is_receiver_product_id(desc.product_id()) {
                let _ = self.tx.send(HotplugEvent::ReceiverLeft {
                    product_id: desc.product_id(),
                });
            }
        }
    }
}

// ── point Entry ──────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    let api = HidApi::new()?;

    // ── Enumerate initial receivers and their devices ────────────────
    let initial_raw = open_receivers(&api);

    if initial_raw.is_empty() {
        warn!("no supported Logitech receiver found at startup");
    }

    // Enumerate devices before moving receivers into state.
    let initial_work: Vec<(Receiver, Vec<Device>)> = initial_raw
        .into_iter()
        .map(|recv| {
            let devices = enumerate_devices(&recv);
            (recv, devices)
        })
        .collect();

    // ── Build initial D-Bus state ──────────────────────────────────
    let state = Arc::new(Mutex::new(DaemonState::default()));
    let initial_states: Vec<(ReceiverState, Vec<Device>)> = {
        let mut s = state.lock().unwrap();
        initial_work
            .into_iter()
            .map(|(recv, devices)| {
                let rs = build_receiver_state(&mut s, recv, &devices);
                (rs, devices)
            })
            .collect()
    };

    // ── Build D-Bus connection ─────────────────────────────────────────────────
    let conn = Builder::system()?
        .name(BUS_NAME)?
        .serve_at(ROOT_PATH, ObjectManager)?
        .build()
        .await?;

    // ── Register initial objects ───────────────────────────────────────────────
    for (rs, devices) in initial_states {
        register_receiver(&conn, &rs, devices).await?;
        spawn_lock_listener(&conn, &rs).await;
        info!(
            "Registered receiver '{}' at {}",
            rs.name,
            DaemonState::receiver_path(rs.index)
        );
    }

    info!("solaarsd started — listening on {BUS_NAME}");

    // ── Hotplug event channel ──────────────────────────────────────────────────
    let (tx, mut rx) = mpsc::unbounded_channel::<HotplugEvent>();
    spawn_hotplug_watcher(tx);

    let hp_conn = conn.clone();
    let hp_state = Arc::clone(&state);

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let result = match event {
                HotplugEvent::ReceiverArrived => {
                    // Give the kernel a moment to create the hidraw node.
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    handle_arrival(&hp_conn, &hp_state).await
                }
                HotplugEvent::ReceiverLeft { product_id } => {
                    handle_departure(&hp_conn, &hp_state, product_id).await
                }
            };
            if let Err(e) = result {
                warn!("hotplug handler error: {e}");
            }
        }
    });

    // Block until killed.
    std::future::pending::<()>().await;
    Ok(())
}

// ── Hotplug thread watcher ─────────────────────────────────────────────────────

/// Spawns a dedicated OS thread that runs the libusb event loop and forwards
/// hotplug events to the async runtime via `tx`.
fn spawn_hotplug_watcher(tx: UnboundedSender<HotplugEvent>) {
    std::thread::spawn(move || {
        if !rusb::has_hotplug() {
            warn!("USB hotplug not supported on this platform — receiver plug/unplug will not be detected");
            return;
        }

        let context = GlobalContext::default();

        let _reg = match HotplugBuilder::new()
            .enumerate(false)
            .vendor_id(LOGITECH_VENDOR_ID)
            .register(context, Box::new(HotplugHandler { tx }))
        {
            Ok(r) => r,
            Err(e) => {
                warn!("failed to register USB hotplug handler: {e}");
                return;
            }
        };

        info!("USB hotplug watcher active");
        loop {
            if let Err(e) = context.handle_events(None) {
                warn!("hotplug event loop error: {e}");
                break;
            }
        }
    });
}

// ── Arrival handler ────

/// Called when a new USB device appears.  Re-enumerates hidapi devices,
/// registers any new receivers on D-Bus, and emits `InterfacesAdded`.
async fn handle_arrival(
    conn: &zbus::Connection,
    state: &Arc<Mutex<DaemonState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let existing_paths: HashSet<String> = {
        let s = state.lock().unwrap();
        s.receivers.iter().map(|rs| rs.path.clone()).collect()
    };

    let new_work: Vec<(Receiver, Vec<Device>)> = tokio::task::block_in_place(|| {
        let api = HidApi::new()?;
        let result = open_receivers(&api)
            .into_iter()
            .filter(|r| !existing_paths.contains(&r.path))
            .map(|recv| {
                let devices = enumerate_devices(&recv);
                (recv, devices)
            })
            .collect();
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(result)
    })?;

    let new_states: Vec<(ReceiverState, Vec<Device>)> = {
        let mut s = state.lock().unwrap();
        new_work
            .into_iter()
            .map(|(recv, devices)| {
                let rs = build_receiver_state(&mut s, recv, &devices);
                (rs, devices)
            })
            .collect()
    };

    for (rs, devices) in new_states {
        info!("Receiver arrived: '{}' at {}", rs.name, rs.path);

        register_receiver(conn, &rs, devices).await?;
        spawn_lock_listener(conn, &rs).await;
    }

    Ok(())
}

// ── Departure handler ──────────────────────────────────────────────────────────

/// Called when a USB device is removed.  Finds receivers whose hidraw node is
/// gone, unregisters them from D-Bus, and emits `InterfacesRemoved`.
async fn handle_departure(
    conn: &zbus::Connection,
    state: &Arc<Mutex<DaemonState>>,
    product_id: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let departed: Vec<ReceiverState> = {
        let s = state.lock().unwrap();
        s.receivers
            .iter()
            .filter(|rs| rs.product_id == product_id && !std::path::Path::new(&rs.path).exists())
            .cloned()
            .collect()
    };

    for rs in &departed {
        info!("Receiver departed: '{}' at {}", rs.name, rs.path);

        for &num in &rs.device_numbers {
            let dev_path = DaemonState::device_path(rs.index, num);
            let _ = conn
                .object_server()
                .remove::<DeviceInterface, _>(dev_path.as_str())
                .await;
        }

        let recv_path = DaemonState::receiver_path(rs.index);
        let _ = conn
            .object_server()
            .remove::<ReceiverInterface, _>(recv_path.as_str())
            .await;
    }

    let gone_paths: HashSet<String> = departed.iter().map(|rs| rs.path.clone()).collect();
    state
        .lock()
        .unwrap()
        .receivers
        .retain(|rs| !gone_paths.contains(&rs.path));

    Ok(())
}

// ── helpers D-Bus ───────────────────────────────────────────────

/// Wrap a newly-opened [`Receiver`] into a [`ReceiverState`], add it to
/// [`DaemonState`], and return it.
fn build_receiver_state(s: &mut DaemonState, recv: Receiver, devices: &[Device]) -> ReceiverState {
    let idx = s.next_receiver_index();
    let rs = ReceiverState {
        path: recv.path.clone(),
        product_id: recv.product_id,
        name: recv.name.to_string(),
        receiver: Arc::new(tokio::sync::Mutex::new(recv)),
        index: idx,
        device_numbers: devices.iter().map(|d| d.number).collect(),
    };
    s.receivers.push(rs.clone());
    rs
}

/// Register a receiver and its devices on the D-Bus object server.
async fn register_receiver(
    conn: &zbus::Connection,
    rs: &ReceiverState,
    devices: Vec<Device>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let recv_path = DaemonState::receiver_path(rs.index);
    let recv_obj_path = OwnedObjectPath::try_from(recv_path.to_string())?;

    conn.object_server()
        .at(
            recv_path.as_str(),
            ReceiverInterface {
                receiver: Arc::clone(&rs.receiver),
            },
        )
        .await?;

    for dev in devices {
        let num = dev.number;
        let dev_path = DaemonState::device_path(rs.index, num);
        info!(
            "  Registered device '{}' ({}) at {}",
            dev.codename.as_deref().unwrap_or("unknown"),
            dev.serial.as_deref().unwrap_or("unknown"),
            dev_path,
        );
        conn.object_server()
            .at(
                dev_path.as_str(),
                DeviceInterface {
                    device: tokio::sync::Mutex::new(dev),
                    receiver: recv_obj_path.clone(),
                    receiver_handle: Arc::clone(&rs.receiver),
                },
            )
            .await?;
    }

    Ok(())
}

// ── Lock-state listener ────────────────────────────────────────────────────────

/// Spawn a background [`EventsListener`] for `rs` that watches for
/// lock-information notifications (sub-ID `0x4A`) from the receiver.
///
/// When such a notification arrives:
/// - `data[0] == 0x01` → pairing window opened; `Discovering` → `true`.
/// - any other value  → pairing window closed or timed out; `Discovering` → `false`.
///
/// The listener thread and the async handler task are both self-contained: they
/// exit automatically when the receiver is unplugged (the HID device returns
/// [`logitech_hidpp::error::Error::NoReceiver`], which closes the channel).
async fn spawn_lock_listener(conn: &zbus::Connection, rs: &ReceiverState) {
    let hidpp_handle = rs.receiver.lock().await.hidpp_handle();
    let recv_dbus_path = DaemonState::receiver_path(rs.index).to_string();

    let (lock_tx, mut lock_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();

    let listener = EventsListener::spawn(
        hidpp_handle,
        Box::new(move |n| {
            if n.devnumber == RECEIVER_DEVICE_NUMBER && n.sub_id == SUB_ID_LOCK_INFORMATION {
                // data[0] == 0x01  → lock opened (discovering)
                // data[0] == 0x00  → lock closed / timed out
                // data[0] == 0x02+ → pairing in progress, window still technically
                //                    open but no longer in "discovery" mode
                let discovering = n.data.first().map(|&b| b == 0x01).unwrap_or(false);
                let _ = lock_tx.send(discovering);
            }
        }),
    );

    let recv_arc = Arc::clone(&rs.receiver);
    let conn_clone = conn.clone();

    tokio::spawn(async move {
        // Keep the listener alive for as long as this task runs.
        let _listener = listener;

        while let Some(discovering) = lock_rx.recv().await {
            let mut recv = recv_arc.lock().await;
            if recv.pairing.discovering == discovering {
                continue; // no change — skip redundant PropertiesChanged
            }
            recv.pairing.lock_open = discovering;
            recv.pairing.discovering = discovering;
            drop(recv);

            if let Ok(iface_ref) = conn_clone
                .object_server()
                .interface::<_, ReceiverInterface>(recv_dbus_path.as_str())
                .await
            {
                let emitter = iface_ref.signal_emitter();
                let _ = iface_ref.get().await.discovering_changed(emitter).await;
            }
        }
        // Channel closed → listener thread stopped (device unplugged).
    });
}

// ── Device enumeration ─────────────────────────────────────────────────────────

fn enumerate_devices(receiver: &Receiver) -> Vec<Device> {
    let mut devices = Vec::new();
    let handle = receiver.hidpp_handle();

    for n in 1..=receiver.max_devices {
        let pairing_info = match receiver.paired_device_info(n) {
            Ok(Some(info)) => info,
            _ => continue,
        };

        let mut dev = Device::with_receiver(Arc::clone(&handle), n, false, pairing_info);
        let _ = dev.ping(None);
        devices.push(dev);
    }

    devices
}

// ── discovery Receiver ──────────────────────

fn open_receivers(api: &HidApi) -> Vec<Receiver> {
    let mut receivers = Vec::new();
    let mut seen_paths = HashSet::new();

    for dev_info in api.device_list() {
        if dev_info.vendor_id() != LOGITECH_VENDOR_ID {
            continue;
        }
        let pid = dev_info.product_id();
        if !is_receiver_product_id(pid) {
            continue;
        }

        let usage_page = dev_info.usage_page();
        let iface = dev_info.interface_number();
        if usage_page != 0xFF00 && iface != 2 && iface != 0 {
            continue;
        }

        let path = dev_info.path().to_string_lossy().to_string();
        if !seen_paths.insert(path.clone()) {
            continue;
        }

        let info = match get_receiver_info(pid) {
            Some(i) => i,
            None => continue,
        };

        match api.open_path(dev_info.path()) {
            Ok(raw) => {
                let shared = Arc::new(HidppDevice::new(raw));
                receivers.push(Receiver::new(shared, path, pid, info));
            }
            Err(e) => {
                warn!("could not open {path}: {e}");
            }
        }
    }

    receivers
}
