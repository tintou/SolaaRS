# SolaaRS

SolaaRS is a Linux manager for Logitech keyboards, mice, and other devices that
connect wirelessly via a Unifying, Bolt, Lightspeed, or Nano receiver.

This repository is a **Rust reimplementation** of the original Solaar project.
It exposes the same device-management functionality through two complementary
components:

- **`solaars`** — a command-line interface for direct device management.
- **`solaarsd`** — a D-Bus system daemon that continuously monitors receivers
  and paired devices, mirroring the BlueZ object hierarchy.

[![License: GPL v2](https://img.shields.io/badge/License-GPL%20v2+-blue.svg)](LICENSE.txt)

## Repository layout

| Crate / directory | Description |
|---|---|
| `logitech-hidpp` | Rust library implementing the Logitech HID++ 1.0 and 2.0 protocol |
| `solaars-cli` | `solaars` binary — CLI device management tool |
| `solaarsd` | `solaarsd` binary — D-Bus daemon |
| `rules.d` | udev rules granting non-root access to Logitech HID devices |

## Building

### Prerequisites

- Rust (edition 2021 or later) with Cargo
- Meson ≥ 1.0
- `libhidapi` (linux-native / hidraw backend)
- `libusb` with hotplug support (for `solaarsd` receiver plug/unplug detection)
- `libdbus` / `zbus` runtime (for `solaarsd`)

### Steps

```sh
meson setup build
meson compile -C build
```

Both binaries are placed under `build/`.

## Installation

```sh
meson install -C build
```

This installs:

- `solaars` and `solaarsd` binaries.
- The udev rule `42-logitech-unify-permissions.rules` into the appropriate
  `udev/rules.d` directory, granting seat users raw HID access.

## udev rules

The file `rules.d/42-logitech-unify-permissions.rules` grants the logged-in
seat user (via `uaccess`) read/write access to Logitech HID devices without
requiring root privileges.  It is installed automatically by `meson install`.

## `solaars` — CLI

```
solaars <command> [options]
```

| Command | Description |
|---------|-------------|
| `show [device]` | Show information about device(s) or receiver(s). `device` can be a slot number (1–6), serial, codename, name substring, or `all` (default). |
| `probe [receiver]` | Raw register dump of a receiver (debugging). |
| `pair [receiver]` | Open the pairing window on a receiver. |
| `unpair <device>` | Unpair a device from its receiver. |
| `config <device> [setting [value]]` | Read or write a device setting. |
| `profiles <device> [file]` | Read or load onboard profiles (YAML). |

Run `solaars <command> --help` for details on each command.

## `solaarsd` — D-Bus daemon

`solaarsd` connects to the system D-Bus under the well-known name
`org.solaarsd` and exposes a BlueZ-style object hierarchy:

```
/org/solaarsd                       ← org.freedesktop.DBus.ObjectManager
/org/solaarsd/receiver{N}           ← org.solaarsd.Receiver1
/org/solaarsd/receiver{N}/dev{NN}   ← org.solaarsd.Device1
```

### `org.solaarsd.Receiver1` properties

| Property | Type | Description |
|----------|------|-------------|
| `Name` | `s` | Human-readable receiver model name |
| `Address` | `s` | HID device node (e.g. `/dev/hidraw0`) |
| `ProductId` | `q` | USB product ID |
| `MaxDevices` | `y` | Maximum simultaneously paired devices |
| `Discovering` | `b` | `true` while the pairing window is open |

### `org.solaarsd.Device1` properties

| Property | Type | Description |
|----------|------|-------------|
| `Codename` | `s` | Device model name |
| `Serial` | `s` | Serial number (hex string) |
| `Kind` | `s` | Device category: `mouse`, `keyboard`, `trackball`, … |
| `Wpid` | `q` | Wireless Product ID |
| `PollingRate` | `y` | Report rate in milliseconds (0 = unknown) |
| `Receiver` | `o` | Object path of the paired receiver |
| `Connected` | `b` | `true` when the device is powered on and in range |
| `BatteryLevel` | `i` | Battery charge 0–100, or `-1` if unavailable |
| `BatteryStatus` | `s` | `full`, `discharging`, `recharging`, … |

USB hotplug (receiver plug/unplug) is handled via `libusb` hotplug
notifications; `InterfacesAdded` / `InterfacesRemoved` signals are emitted on
the `ObjectManager` interface accordingly.

### Running

```sh
# start the daemon (foreground, logs to stderr via RUST_LOG)
RUST_LOG=info solaarsd
```

A systemd unit file (`solaarsd.service`) is provided and installed by
`meson install`.

## License

GPL-2.0-or-later — see [LICENSE.txt](LICENSE.txt).
