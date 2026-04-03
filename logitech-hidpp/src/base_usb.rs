// Copyright (C) 2012-2013  Daniel Pavel
// Copyright (C) 2014-2024  Solaar Contributors https://pwr-solaar.github.io/Solaar/
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.

//! Known Logitech USB receiver product IDs and metadata.
//!
//! Ported from `logitech_receiver/base_usb.py`.

use crate::common::LOGITECH_VENDOR_ID;

/// High-level receiver kind, determining which protocol variant to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverKind {
    /// Bolt receivers (yellow lightning bolt logo).
    Bolt,
    /// Standard Unifying receivers (orange logo).
    Unifying,
    /// Nano receivers (budget, typically one device).
    Nano,
    /// Lightspeed receivers (gaming).
    Lightspeed,
    /// Old-style EX100 27 MHz receivers.
    Ex100_27Mhz,
}

/// Static information about a known Logitech receiver.
#[derive(Debug, Clone)]
pub struct ReceiverInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    /// USB interface number to open.
    pub usb_interface: u8,
    pub name: &'static str,
    pub kind: ReceiverKind,
    /// Maximum number of simultaneously paired devices.
    pub max_devices: u8,
    /// Whether the user can initiate unpairing.
    pub may_unpair: bool,
    /// Whether this receiver replaces (re-pairs) the current paired device.
    pub re_pairs: bool,
}

macro_rules! bolt {
    ($pid:expr) => {
        ReceiverInfo {
            vendor_id: LOGITECH_VENDOR_ID,
            product_id: $pid,
            usb_interface: 2,
            name: "Bolt Receiver",
            kind: ReceiverKind::Bolt,
            max_devices: 6,
            may_unpair: true,
            re_pairs: false,
        }
    };
}

macro_rules! unifying {
    ($pid:expr) => {
        ReceiverInfo {
            vendor_id: LOGITECH_VENDOR_ID,
            product_id: $pid,
            usb_interface: 2,
            name: "Unifying Receiver",
            kind: ReceiverKind::Unifying,
            max_devices: 6,
            may_unpair: true,
            re_pairs: false,
        }
    };
}

macro_rules! nano {
    ($pid:expr, max: $max:expr, may_unpair: $mu:expr, re_pairs: $rp:expr) => {
        ReceiverInfo {
            vendor_id: LOGITECH_VENDOR_ID,
            product_id: $pid,
            usb_interface: 1,
            name: "Nano Receiver",
            kind: ReceiverKind::Nano,
            max_devices: $max,
            may_unpair: $mu,
            re_pairs: $rp,
        }
    };
}

macro_rules! lightspeed {
    ($pid:expr) => {
        ReceiverInfo {
            vendor_id: LOGITECH_VENDOR_ID,
            product_id: $pid,
            usb_interface: 2,
            name: "Lightspeed Receiver",
            kind: ReceiverKind::Lightspeed,
            max_devices: 1,
            may_unpair: false,
            re_pairs: true,
        }
    };
}

macro_rules! ex100 {
    ($pid:expr) => {
        ReceiverInfo {
            vendor_id: LOGITECH_VENDOR_ID,
            product_id: $pid,
            usb_interface: 1,
            name: "EX100 Receiver 27 MHz",
            kind: ReceiverKind::Ex100_27Mhz,
            max_devices: 4,
            may_unpair: false,
            re_pairs: true,
        }
    };
}

/// Table of all known Logitech receivers indexed by product ID.
pub static KNOWN_RECEIVERS: &[(u16, ReceiverInfo)] = &[
    // Bolt
    (0xC548, bolt!(0xC548)),
    // Unifying
    (0xC52B, unifying!(0xC52B)),
    (0xC532, unifying!(0xC532)),
    // Nano
    (
        0xC52F,
        nano!(0xC52F, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC518,
        nano!(0xC518, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC51A,
        nano!(0xC51A, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC51B,
        nano!(0xC51B, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC521,
        nano!(0xC521, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC525,
        nano!(0xC525, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC526,
        nano!(0xC526, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC52E,
        nano!(0xC52E, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC531,
        nano!(0xC531, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC534,
        nano!(0xC534, max: 2, may_unpair: false, re_pairs: true),
    ),
    (
        0xC535,
        nano!(0xC535, max: 1, may_unpair: false, re_pairs: true),
    ),
    (
        0xC537,
        nano!(0xC537, max: 1, may_unpair: false, re_pairs: true),
    ),
    // Lightspeed
    (0xC539, lightspeed!(0xC539)),
    (0xC53A, lightspeed!(0xC53A)),
    (0xC53D, lightspeed!(0xC53D)),
    (0xC53F, lightspeed!(0xC53F)),
    (0xC541, lightspeed!(0xC541)),
    (0xC545, lightspeed!(0xC545)),
    (0xC547, lightspeed!(0xC547)),
    (0xC54D, lightspeed!(0xC54D)),
    // EX100
    (0xC517, ex100!(0xC517)),
];

/// Returns the [`ReceiverInfo`] for the given USB product ID, or `None` if the
/// product ID is not a known Logitech receiver.
pub fn get_receiver_info(product_id: u16) -> Option<&'static ReceiverInfo> {
    KNOWN_RECEIVERS
        .iter()
        .find(|(pid, _)| *pid == product_id)
        .map(|(_, info)| info)
}

/// Returns `true` if the USB product ID falls in the range of (possibly
/// unknown) Logitech receivers.
pub fn is_receiver_product_id(product_id: u16) -> bool {
    (0xC500..=0xC5FF).contains(&product_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_receiver_product_id_within_range() {
        assert!(is_receiver_product_id(0xC500));
        assert!(is_receiver_product_id(0xC52B));
        assert!(is_receiver_product_id(0xC548));
        assert!(is_receiver_product_id(0xC5FF));
    }

    #[test]
    fn is_receiver_product_id_outside_range() {
        assert!(!is_receiver_product_id(0xC4FF));
        assert!(!is_receiver_product_id(0xC600));
        assert!(!is_receiver_product_id(0x0000));
        assert!(!is_receiver_product_id(0xFFFF));
    }

    #[test]
    fn get_receiver_info_known_bolt() {
        let info = get_receiver_info(0xC548).expect("Bolt receiver 0xC548 should be known");
        assert_eq!(info.product_id, 0xC548);
        assert_eq!(info.kind, ReceiverKind::Bolt);
        assert_eq!(info.max_devices, 6);
        assert_eq!(info.usb_interface, 2);
        assert!(!info.re_pairs);
    }

    #[test]
    fn get_receiver_info_known_unifying() {
        let info = get_receiver_info(0xC52B).expect("Unifying receiver 0xC52B should be known");
        assert_eq!(info.kind, ReceiverKind::Unifying);
        assert_eq!(info.max_devices, 6);
    }

    #[test]
    fn get_receiver_info_known_nano() {
        let info = get_receiver_info(0xC52F).expect("Nano receiver 0xC52F should be known");
        assert_eq!(info.kind, ReceiverKind::Nano);
        assert_eq!(info.max_devices, 1);
    }

    #[test]
    fn get_receiver_info_unknown_pid() {
        assert!(get_receiver_info(0x1234).is_none());
        assert!(get_receiver_info(0xC500).is_none());
    }

    /// Mirrors test_base_usb.py::test_ensure_known_receivers_mappings_are_valid.
    /// Every entry in KNOWN_RECEIVERS must have its key == entry.product_id.
    #[test]
    fn known_receivers_keys_match_product_ids() {
        for (key, info) in KNOWN_RECEIVERS {
            assert_eq!(
                *key, info.product_id,
                "KNOWN_RECEIVERS key {key:#06X} does not match product_id {:#06X}",
                info.product_id
            );
        }
    }

    /// Every receiver must have the Logitech vendor ID.
    #[test]
    fn known_receivers_have_logitech_vendor_id() {
        use crate::common::LOGITECH_VENDOR_ID;
        for (_key, info) in KNOWN_RECEIVERS {
            assert_eq!(
                info.vendor_id, LOGITECH_VENDOR_ID,
                "Receiver {:#06X} has unexpected vendor_id {:#06X}",
                info.product_id, info.vendor_id
            );
        }
    }

    /// Bolt receivers use USB interface 2; all others use 0 or 1.
    #[test]
    fn bolt_receivers_use_interface_2() {
        for (_key, info) in KNOWN_RECEIVERS {
            if info.kind == ReceiverKind::Bolt {
                assert_eq!(
                    info.usb_interface, 2,
                    "Bolt receiver {:#06X} should use USB interface 2",
                    info.product_id
                );
            }
        }
    }

    #[test]
    fn get_receiver_info_bolt_0xc548() {
        let info = get_receiver_info(0xC548).expect("Bolt receiver 0xC548 must be known");
        assert_eq!(info.product_id, 0xC548);
        assert_eq!(info.kind, ReceiverKind::Bolt);
        assert_eq!(info.max_devices, 6);
        assert_eq!(info.usb_interface, 2);
        assert!(info.may_unpair);
        assert!(!info.re_pairs);
    }
}
