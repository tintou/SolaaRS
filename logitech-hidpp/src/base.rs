use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use log::{debug, error, warn};

use crate::error::{Error, Hidpp10ErrorCode, Hidpp20ErrorCode};
use crate::message::{
    DJ_MESSAGE_ID, HIDPP_LONG_MESSAGE_ID, HIDPP_SHORT_MESSAGE_ID, HidppNotification,
    LONG_MESSAGE_SIZE, MAX_READ_SIZE, MEDIUM_MESSAGE_SIZE, RECEIVER_DEVICE_NUMBER,
    SHORT_MESSAGE_SIZE,
};

/// Timeout for requests directed at the receiver itself (fast local reply).
const RECEIVER_REQUEST_TIMEOUT: Duration = Duration::from_millis(900);
/// Timeout for requests directed at a wireless device (round-trip over air).
const DEVICE_REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
/// Timeout used for ping operations.
const PING_TIMEOUT: Duration = Duration::from_secs(4);

/// A cyclic software-ID counter (0x02–0x0F) used to distinguish replies from
/// notifications.  A non-zero SoftwareID in the low nibble of the address byte
/// marks a request reply; value 0 indicates a notification.
static SW_ID: AtomicU8 = AtomicU8::new(0x0F);

fn next_sw_id() -> u8 {
    // Cycle 0x02 → 0x0F, wrapping back to 0x02 after 0x0F.
    let mut old = SW_ID.load(Ordering::Relaxed);
    loop {
        let next = if old < 0x0F { old + 1 } else { 0x02 };
        match SW_ID.compare_exchange_weak(old, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(actual) => old = actual,
        }
    }
}

/// Options for a [`HidppDevice::request`] call.
#[derive(Debug, Default, Clone)]
pub struct RequestOptions {
    /// When `true`, fire-and-forget: do not wait for a reply.
    pub no_reply: bool,
    /// When `true`, return a HID++ 1.0 error code instead of `None` on error.
    pub return_error: bool,
    /// Force the request to use the long (20-byte) HID++ message format.
    pub long_message: bool,
    /// Protocol version of the target device (`1.0` or `2.0`).
    /// Used to decide whether to set the SoftwareID in the request.
    pub protocol: f32,
}

/// A callback invoked with every notification received while waiting for a
/// reply inside [`HidppDevice::request`] or [`HidppDevice::ping`].
pub type NotificationsHook = Box<dyn Fn(HidppNotification) + Send>;

/// A thread-safe wrapper around a `hidapi::HidDevice` that implements the
/// Logitech HID++ protocol framing, request/response cycle and ping.
///
/// All public methods acquire an internal mutex to serialise access to the
/// underlying HID handle, matching the per-handle locking model of the
/// original Python implementation.
#[derive(Debug)]
pub struct HidppDevice {
    device: Mutex<hidapi::HidDevice>,
}

impl HidppDevice {
    /// Wrap an already-opened `hidapi::HidDevice`.
    pub fn new(device: hidapi::HidDevice) -> Self {
        Self {
            device: Mutex::new(device),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, hidapi::HidDevice>, Error> {
        self.device.lock().map_err(|_| Error::LockPoisoned)
    }

    // ------------------------------------------------------------------
    // Low-level read / write
    // ------------------------------------------------------------------

    /// Write a HID++ message to the device.
    ///
    /// `data` must start with the SubId and address bytes (up to 18 bytes for
    /// a long message, or 5 bytes for a short one).  The correct report ID and
    /// device number are prepended automatically.
    pub fn write(&self, devnumber: u8, data: &[u8], long_message: bool) -> Result<(), Error> {
        let dev = self.lock()?;
        write_inner(&dev, devnumber, data, long_message)
    }

    /// Read one message from the device, blocking for up to `timeout`.
    ///
    /// Returns `Some((report_id, devnumber, payload))` where `payload` is the
    /// message body starting after the report-ID and device-number bytes.
    /// Returns `None` on timeout or if the message fails a sanity check.
    pub fn read(&self, timeout: Duration) -> Result<Option<(u8, u8, Vec<u8>)>, Error> {
        let dev = self.lock()?;
        read_inner(&dev, timeout)
    }

    // ------------------------------------------------------------------
    // Request / response
    // ------------------------------------------------------------------

    /// Make a feature/register call and wait for the matching reply.
    ///
    /// `request_id` is a 16-bit feature or register identifier.
    /// `params` are the optional parameter bytes to append.
    /// Notifications received while waiting are forwarded to `hook` if provided.
    ///
    /// Returns the reply payload (bytes after the echoed request_id), or
    /// `None` on timeout or when `options.no_reply` is set.
    pub fn request(
        &self,
        devnumber: u8,
        mut request_id: u16,
        params: &[u8],
        options: &RequestOptions,
        hook: Option<&NotificationsHook>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let dev = self.lock()?;

        // Set a non-zero SoftwareID for peripheral requests (HID++ 2.0 or
        // non-receiver targets) so notifications can be distinguished from
        // replies (notifications have SoftwareID 0).
        if (devnumber != RECEIVER_DEVICE_NUMBER || options.protocol >= 2.0) && request_id < 0x8000 {
            let sw_id = next_sw_id();
            request_id = (request_id & 0xFFF0) | sw_id as u16;
        }

        let timeout = if devnumber == RECEIVER_DEVICE_NUMBER {
            RECEIVER_REQUEST_TIMEOUT
        } else {
            DEVICE_REQUEST_TIMEOUT
        };
        // Long register reads need extra time.
        let timeout = if request_id & 0xFF00 == 0x8300 {
            timeout * 2
        } else {
            timeout
        };

        let request_data = build_request(request_id, params);

        // Drain anything already queued in the kernel input buffer.
        flush_input_buffer(&dev, hook)?;

        write_inner(&dev, devnumber, &request_data, options.long_message)?;

        if options.no_reply {
            return Ok(None);
        }

        let started = Instant::now();
        loop {
            let elapsed = started.elapsed();
            if elapsed >= timeout {
                break;
            }
            let remaining = timeout - elapsed;

            if let Some((report_id, reply_devnum, reply_data)) = read_inner(&dev, remaining)? {
                // Replies may come back with devnumber XOR 0xFF on Bluetooth.
                if reply_devnum == devnumber || reply_devnum == devnumber ^ 0xFF {
                    // HID++ 1.0 error reply: sub_id = 0x8F.
                    if report_id == HIDPP_SHORT_MESSAGE_ID
                        && reply_data.first() == Some(&0x8F)
                        && reply_data.get(1..3) == Some(&request_data[..2])
                    {
                        let error_code = reply_data.get(3).copied().unwrap_or(0);
                        let code = Hidpp10ErrorCode::from(error_code);
                        debug!(
                            "device {:#04x} error on request {:#06x}: {:?}",
                            devnumber, request_id, code
                        );
                        return if options.return_error {
                            // Encode the raw error byte in the return payload
                            // so callers can inspect it without matching on Err.
                            Ok(Some(vec![error_code]))
                        } else {
                            Ok(None)
                        };
                    }

                    // HID++ 2.0 error reply: sub_id = 0xFF.
                    if reply_data.first() == Some(&0xFF)
                        && reply_data.get(1..3) == Some(&request_data[..2])
                    {
                        let error_code = reply_data.get(3).copied().unwrap_or(0);
                        let code = Hidpp20ErrorCode::from(error_code);
                        error!(
                            "device {} feature call error on {:#06x}: {:?}",
                            devnumber, request_id, code
                        );
                        return Err(Error::FeatureCallError {
                            number: devnumber,
                            request: request_id,
                            error: error_code,
                        });
                    }

                    // Successful reply: the first two bytes echo the request_id.
                    if reply_data.get(..2) == Some(&request_data[..2]) {
                        if devnumber == RECEIVER_DEVICE_NUMBER {
                            // Some receiver requests require the first *parameter* byte
                            // to also match (0x83B5 = short register read,
                            // 0x81F1 = long register read).
                            if request_id == 0x83B5 || request_id == 0x81F1 {
                                if reply_data.get(2) == params.first() {
                                    return Ok(Some(reply_data[2..].to_vec()));
                                }
                                // Not our reply; keep waiting.
                                continue;
                            }
                        }
                        return Ok(Some(reply_data[2..].to_vec()));
                    }
                }

                // The reply didn't match our request in any way — treat any
                // notifications in it, but reset the timeout so we keep waiting.
                if let Some(hook) = hook
                    && let Some(n) =
                        HidppNotification::from_raw(report_id, reply_devnum, &reply_data)
                {
                    hook(n);
                }
            }
        }

        warn!(
            "timeout ({:.2?}) on device {:#04x} request {:#06x} params {:?}",
            timeout, devnumber, request_id, params
        );
        Ok(None)
    }

    // ------------------------------------------------------------------
    // Ping
    // ------------------------------------------------------------------

    /// Verify that a device is reachable and return its HID++ protocol version.
    ///
    /// Returns `Some(version)` where `version` is e.g. `1.0` or `2.0` for a
    /// responsive device, or `None` if the device did not reply within the
    /// timeout.
    pub fn ping(
        &self,
        devnumber: u8,
        long_message: bool,
        hook: Option<&NotificationsHook>,
    ) -> Result<Option<f64>, Error> {
        debug!("pinging device {}", devnumber);
        let dev = self.lock()?;

        // Drain the input buffer first.
        flush_input_buffer(&dev, hook)?;

        // Build a ping request with a random mark byte in the last position so
        // we can identify the reply.
        let sw_id = next_sw_id();
        let request_id: u16 = 0x0010 | sw_id as u16;
        let mark: u8 = rand_byte();
        let mut request_data = Vec::with_capacity(5);
        request_data.extend_from_slice(&request_id.to_be_bytes()); // bytes 0-1
        request_data.push(0x00); // byte 2
        request_data.push(0x00); // byte 3
        request_data.push(mark); // byte 4 — the mark to identify the reply

        write_inner(&dev, devnumber, &request_data, long_message)?;

        let started = Instant::now();
        loop {
            let elapsed = started.elapsed();
            if elapsed >= PING_TIMEOUT {
                break;
            }
            let remaining = PING_TIMEOUT - elapsed;

            if let Some((report_id, reply_devnum, reply_data)) = read_inner(&dev, remaining)? {
                if reply_devnum == devnumber || reply_devnum == devnumber ^ 0xFF {
                    // Successful HID++ 2.0+ ping reply.
                    if reply_data.get(..2) == Some(&request_data[..2])
                        && reply_data.get(4) == Some(&mark)
                    {
                        let major = reply_data.get(2).copied().unwrap_or(0);
                        let minor = reply_data.get(3).copied().unwrap_or(0);
                        return Ok(Some(major as f64 + minor as f64 / 10.0));
                    }

                    // HID++ 1.0 error reply.
                    if report_id == HIDPP_SHORT_MESSAGE_ID
                        && reply_data.first() == Some(&0x8F)
                        && reply_data.get(1..3) == Some(&request_data[..2])
                    {
                        let error_code =
                            Hidpp10ErrorCode::from(reply_data.get(3).copied().unwrap_or(0));
                        match error_code {
                            Hidpp10ErrorCode::InvalidSubIdCommand => {
                                // Valid reply from a HID++ 1.0 device.
                                return Ok(Some(1.0));
                            }
                            Hidpp10ErrorCode::ResourceError
                            | Hidpp10ErrorCode::ConnectionRequestFailed => {
                                return Ok(None); // device unreachable
                            }
                            Hidpp10ErrorCode::UnknownDevice => {
                                return Err(Error::NoSuchDevice {
                                    number: devnumber,
                                    request: request_id,
                                });
                            }
                            _ => {}
                        }
                    }
                }

                // Treat as notification and keep waiting.
                if let Some(hook) = hook
                    && let Some(n) =
                        HidppNotification::from_raw(report_id, reply_devnum, &reply_data)
                {
                    hook(n);
                }
            }
        }

        warn!(
            "timeout ({:.2?}) on device {} ping",
            PING_TIMEOUT, devnumber
        );
        Ok(None)
    }
}

// ------------------------------------------------------------------
// Private helpers (operate on a borrowed `HidDevice` to avoid
// double-locking from within `request` / `ping`).
// ------------------------------------------------------------------

fn write_inner(
    dev: &hidapi::HidDevice,
    devnumber: u8,
    data: &[u8],
    long_message: bool,
) -> Result<(), Error> {
    let wdata =
        if long_message || data.len() > SHORT_MESSAGE_SIZE - 2 || data.first() == Some(&0x82) {
            let mut buf = vec![HIDPP_LONG_MESSAGE_ID, devnumber];
            buf.extend_from_slice(data);
            buf.resize(LONG_MESSAGE_SIZE, 0u8);
            buf
        } else {
            let mut buf = vec![HIDPP_SHORT_MESSAGE_ID, devnumber];
            buf.extend_from_slice(data);
            buf.resize(SHORT_MESSAGE_SIZE, 0u8);
            buf
        };

    debug!(
        "<= w[{:02X} {:02X} {:02X}{:02X} {}]",
        wdata[0],
        devnumber,
        wdata.get(2).copied().unwrap_or(0),
        wdata.get(3).copied().unwrap_or(0),
        hex_str(&wdata[4..]),
    );

    dev.write(&wdata)
        .map_err(|e| Error::NoReceiver(e.to_string()))?;
    Ok(())
}

fn read_inner(
    dev: &hidapi::HidDevice,
    timeout: Duration,
) -> Result<Option<(u8, u8, Vec<u8>)>, Error> {
    let timeout_ms = timeout.as_millis() as i32;
    let mut buf = vec![0u8; MAX_READ_SIZE];

    let n = dev
        .read_timeout(&mut buf, timeout_ms)
        .map_err(|e| Error::NoReceiver(e.to_string()))?;

    if n == 0 {
        return Ok(None); // timeout
    }

    let data = &buf[..n];

    if !is_relevant_message(data) {
        return Ok(None);
    }

    let report_id = data[0];
    let devnumber = data[1];

    debug!(
        "=> r[{:02X} {:02X} {:02X}{:02X} {}]",
        report_id,
        devnumber,
        data.get(2).copied().unwrap_or(0),
        data.get(3).copied().unwrap_or(0),
        hex_str(data.get(4..).unwrap_or(&[])),
    );

    Ok(Some((report_id, devnumber, data[2..].to_vec())))
}

/// Drain any messages already queued in the kernel HID input buffer,
/// forwarding decoded notifications to `hook` if provided.
fn flush_input_buffer(
    dev: &hidapi::HidDevice,
    hook: Option<&NotificationsHook>,
) -> Result<(), Error> {
    let mut buf = vec![0u8; MAX_READ_SIZE];
    loop {
        let n = dev
            .read_timeout(&mut buf, 0) // non-blocking
            .map_err(|e| Error::NoReceiver(e.to_string()))?;
        if n == 0 {
            return Ok(());
        }
        let data = &buf[..n];
        if is_relevant_message(data)
            && let Some(hook) = hook
        {
            let report_id = data[0];
            let devnumber = data[1];
            if let Some(n) = HidppNotification::from_raw(report_id, devnumber, &data[2..]) {
                hook(n);
            }
        }
    }
}

/// Returns `true` if `data` has a known HID++ or DJ report ID and the correct
/// length for that report type.
fn is_relevant_message(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let expected_len = match data[0] {
        HIDPP_SHORT_MESSAGE_ID => SHORT_MESSAGE_SIZE,
        HIDPP_LONG_MESSAGE_ID => LONG_MESSAGE_SIZE,
        DJ_MESSAGE_ID => MEDIUM_MESSAGE_SIZE,
        0x21 => MAX_READ_SIZE,
        _ => return false,
    };
    if data.len() != expected_len {
        warn!(
            "unexpected message size: report_id {:02X} got {} expected {}",
            data[0],
            data.len(),
            expected_len
        );
        return false;
    }
    true
}

/// Pack a 16-bit request_id (big-endian) followed by params into a byte vector.
fn build_request(request_id: u16, params: &[u8]) -> Vec<u8> {
    let mut v = request_id.to_be_bytes().to_vec();
    v.extend_from_slice(params);
    v
}

/// Return a simple hex-encoded string for logging.
fn hex_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A simple pseudo-random byte drawn from the process's address space entropy.
/// Used for the ping mark byte.  In the Python code this was `getrandbits(8)`.
fn rand_byte() -> u8 {
    // Use the low byte of the current time's nanosecond component as cheap
    // entropy — sufficient for distinguishing ping replies.
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u8)
        .unwrap_or(0xA5)
}
