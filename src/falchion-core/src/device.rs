//! Supported hardware identifiers.
//!
//! Product IDs and the usage page are UNVERIFIED. Confirm them on real hardware
//! (see `docs/PROTOCOL.md`) before relying on them.

use std::fmt;

/// USB vendor ID for ASUSTek.
pub const ASUS_VENDOR_ID: u16 = 0x0B05;

/// HID usage page of the vendor-defined control interface. PLACEHOLDER.
pub const CONTROL_USAGE_PAGE: u16 = 0xFF00;

/// How the keyboard is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// USB cable.
    Wired,
    /// 2.4 GHz USB dongle.
    Wireless,
}

impl fmt::Display for ConnectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ConnectionKind::Wired => "wired",
            ConnectionKind::Wireless => "wireless",
        })
    }
}

/// A USB product this application knows how to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownDevice {
    /// Marketing name.
    pub name: &'static str,
    /// USB product ID.
    pub product_id: u16,
    /// Connection the product ID corresponds to.
    pub connection: ConnectionKind,
}

/// Every supported product. UNVERIFIED product IDs.
pub const KNOWN_DEVICES: &[KnownDevice] = &[
    KnownDevice {
        name: "ROG Falchion",
        product_id: 0x193C,
        connection: ConnectionKind::Wired,
    },
    KnownDevice {
        name: "ROG Falchion",
        product_id: 0x193E,
        connection: ConnectionKind::Wireless,
    },
];

/// Looks up a supported product by USB IDs.
pub fn identify(vendor_id: u16, product_id: u16) -> Option<&'static KnownDevice> {
    if vendor_id != ASUS_VENDOR_ID {
        return None;
    }
    KNOWN_DEVICES.iter().find(|d| d.product_id == product_id)
}

/// A connected, supported keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Marketing name.
    pub name: String,
    /// Connection type.
    pub connection: ConnectionKind,
    /// USB vendor ID.
    pub vendor_id: u16,
    /// USB product ID.
    pub product_id: u16,
    /// OS-specific HID path used to open the device.
    pub path: String,
    /// Serial number, if the device reports one.
    pub serial: Option<String>,
}

impl DeviceInfo {
    /// Builds device info for a known product found at `path`.
    pub fn from_known(
        known: &KnownDevice,
        path: impl Into<String>,
        serial: Option<String>,
    ) -> Self {
        Self {
            name: known.name.to_string(),
            connection: known.connection,
            vendor_id: ASUS_VENDOR_ID,
            product_id: known.product_id,
            path: path.into(),
            serial,
        }
    }
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) {:04x}:{:04x} at {}",
            self.name, self.connection, self.vendor_id, self.product_id, self.path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_known_products() {
        for known in KNOWN_DEVICES {
            assert_eq!(identify(ASUS_VENDOR_ID, known.product_id), Some(known));
        }
    }

    #[test]
    fn rejects_unknown_product_and_vendor() {
        assert_eq!(identify(ASUS_VENDOR_ID, 0x0001), None);
        assert_eq!(identify(0x046D, KNOWN_DEVICES[0].product_id), None);
    }

    #[test]
    fn from_known_copies_fields() {
        let info = DeviceInfo::from_known(&KNOWN_DEVICES[1], "p", Some("s".into()));
        assert_eq!(info.connection, ConnectionKind::Wireless);
        assert_eq!(info.vendor_id, ASUS_VENDOR_ID);
        assert_eq!(info.product_id, KNOWN_DEVICES[1].product_id);
        assert_eq!(info.serial.as_deref(), Some("s"));
    }

    #[test]
    fn display_formats() {
        let info = DeviceInfo::from_known(&KNOWN_DEVICES[0], "/dev/x", None);
        let expected = "ROG Falchion (wired) 0b05:193c at /dev/x";
        assert_eq!(info.to_string(), expected);
        assert_eq!(ConnectionKind::Wireless.to_string(), "wireless");
    }
}
