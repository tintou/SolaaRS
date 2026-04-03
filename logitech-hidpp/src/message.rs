/// Report ID for a HID++ short message (7 bytes total).
pub const HIDPP_SHORT_MESSAGE_ID: u8 = 0x10;
/// Report ID for a HID++ long message (20 bytes total).
pub const HIDPP_LONG_MESSAGE_ID: u8 = 0x11;
/// Report ID for a DJ message (15 bytes total).
pub const DJ_MESSAGE_ID: u8 = 0x20;
/// Report ID for a long DJ message (32 bytes total).
pub const DJ_LONG_MESSAGE_ID: u8 = 0x21;

pub const SHORT_MESSAGE_SIZE: usize = 7;
pub const LONG_MESSAGE_SIZE: usize = 20;
pub const MEDIUM_MESSAGE_SIZE: usize = 15;
pub const MAX_READ_SIZE: usize = 32;

/// The Logitech USB vendor ID.
pub const LOGITECH_VENDOR_ID: u16 = 0x046D;

/// Device number used for addressing the receiver itself.
pub const RECEIVER_DEVICE_NUMBER: u8 = 0xFF;

/// A decoded HID++ notification from a device or receiver.
///
/// Notifications are unsolicited messages sent by devices to report events
/// such as button presses, battery status changes, or pairing events.
#[derive(Debug, Clone)]
pub struct HidppNotification {
    pub report_id: u8,
    pub devnumber: u8,
    pub sub_id: u8,
    pub address: u8,
    /// Payload bytes after the sub_id and address bytes.
    pub data: Vec<u8>,
}

impl HidppNotification {
    /// Try to parse a raw HID++ message as a notification.
    ///
    /// Returns `None` if the message is a request reply rather than a
    /// notification, or if it represents a no-op / DJ input record.
    ///
    /// `data` is the raw message body starting from byte 2 (after report_id
    /// and devnumber), as returned by `_read`.
    pub fn from_raw(report_id: u8, devnumber: u8, data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let sub_id = data[0];

        // HID++ 1.0 register read/write replies and error replies have bit 7 set.
        if sub_id & 0x80 == 0x80 {
            return None;
        }

        // DJ input records (sub_id < 0x10) are not notifications.
        if report_id == DJ_MESSAGE_ID && sub_id < 0x10 {
            return None;
        }

        let address = data.get(1).copied().unwrap_or(0);

        // No-op notification: sub_id=0x00 and low nibble of address is 0.
        if sub_id == 0x00 && address & 0x0F == 0x00 {
            return None;
        }

        let is_notification = sub_id >= 0x40
            // HID++ 1.0 battery events
            || ((sub_id == 0x07 || sub_id == 0x0D) && data.len() == 5 && data[4] == 0x00)
            // HID++ 1.0 illumination event
            || (sub_id == 0x17 && data.len() == 5)
            // HID++ 2.0 feature notifications have SoftwareID == 0 in the low nibble of address
            || (address & 0x0F == 0x00);

        if is_notification {
            Some(HidppNotification {
                report_id,
                devnumber,
                sub_id,
                address,
                data: data.get(2..).unwrap_or(&[]).to_vec(),
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for HidppNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Notification({:02x},{},{:02X},{:02X},{:?})",
            self.report_id, self.devnumber, self.sub_id, self.address, self.data
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HidppNotification::from_raw ──────────────────────────────────────────

    /// HID++ 2.0 feature notification: sub_id ≥ 0x40, SoftwareID == 0 (notification).
    #[test]
    fn from_raw_hidpp20_feature_notification() {
        // sub_id = 0x40, address low nibble = 0 → notification
        let data = [0x40u8, 0x00, 0x01, 0x02, 0x03];
        let n = HidppNotification::from_raw(HIDPP_SHORT_MESSAGE_ID, 0x01, &data)
            .expect("should be a notification");
        assert_eq!(n.sub_id, 0x40);
        assert_eq!(n.address, 0x00);
        assert_eq!(n.data, &[0x01, 0x02, 0x03]);
    }

    /// A message with sub_id < 0x40 and non-zero SoftwareID is a request reply, not a notification.
    #[test]
    fn from_raw_hidpp20_reply_is_not_notification() {
        // sub_id = 0x20 < 0x40, address low nibble = 3 (non-zero SoftwareID) → reply
        let data = [0x20u8, 0x03, 0xAA, 0xBB];
        let result = HidppNotification::from_raw(HIDPP_SHORT_MESSAGE_ID, 0x01, &data);
        assert!(
            result.is_none(),
            "request reply should not be a notification"
        );
    }

    /// HID++ 1.0 register reply: sub_id has bit 7 set → not a notification.
    #[test]
    fn from_raw_hidpp10_register_reply_is_not_notification() {
        let data = [0x81u8, 0x00, 0x00, 0x00, 0x00]; // sub_id 0x81 has bit 7 set
        let result = HidppNotification::from_raw(HIDPP_SHORT_MESSAGE_ID, 0xFF, &data);
        assert!(
            result.is_none(),
            "register reply should not be a notification"
        );
    }

    /// HID++ 1.0 battery event: sub_id 0x07, 5 bytes, last byte == 0 → notification.
    #[test]
    fn from_raw_hidpp10_battery_event() {
        let data = [0x07u8, 0x20, 0x01, 0x00, 0x00];
        let n = HidppNotification::from_raw(HIDPP_SHORT_MESSAGE_ID, 0x01, &data)
            .expect("battery event should be a notification");
        assert_eq!(n.sub_id, 0x07);
    }

    /// DJ input records (report_id = DJ_MESSAGE_ID, sub_id < 0x10) → not notifications.
    #[test]
    fn from_raw_dj_input_record_is_not_notification() {
        let data = [0x01u8, 0x00, 0x00]; // sub_id = 0x01 < 0x10
        let result = HidppNotification::from_raw(DJ_MESSAGE_ID, 0x00, &data);
        assert!(
            result.is_none(),
            "DJ input record should not be a notification"
        );
    }

    /// DJ non-input record (sub_id ≥ 0x10 on DJ channel) → is a notification.
    #[test]
    fn from_raw_dj_non_input_is_notification() {
        let data = [0x40u8, 0x00, 0x00];
        let n = HidppNotification::from_raw(DJ_MESSAGE_ID, 0x00, &data)
            .expect("DJ non-input should be a notification");
        assert_eq!(n.sub_id, 0x40);
    }

    /// No-op: sub_id == 0x00 and low nibble of address == 0 → not a notification.
    #[test]
    fn from_raw_noop_is_not_notification() {
        let data = [0x00u8, 0x00, 0x00, 0x00, 0x00];
        let result = HidppNotification::from_raw(HIDPP_SHORT_MESSAGE_ID, 0x01, &data);
        assert!(result.is_none(), "no-op should not be a notification");
    }

    /// Empty data → not a notification.
    #[test]
    fn from_raw_empty_data() {
        let result = HidppNotification::from_raw(HIDPP_SHORT_MESSAGE_ID, 0x01, &[]);
        assert!(result.is_none());
    }

    /// Display format is correct.
    #[test]
    fn notification_display() {
        let n = HidppNotification {
            report_id: 0x10,
            devnumber: 1,
            sub_id: 0x40,
            address: 0x00,
            data: vec![0xAA, 0xBB],
        };
        let s = format!("{n}");
        assert!(s.contains("10"), "should include report_id");
        assert!(s.contains("40"), "should include sub_id");
    }
}
