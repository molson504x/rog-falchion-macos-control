//! Selects supported keyboards from raw HID enumeration results.

use falchion_core::device::{identify, DeviceInfo, CONTROL_USAGE_PAGE};

/// One HID interface as reported by the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// USB vendor ID.
    pub vendor_id: u16,
    /// USB product ID.
    pub product_id: u16,
    /// HID usage page of this interface.
    pub usage_page: u16,
    /// OS-specific path.
    pub path: String,
    /// Serial number, if any.
    pub serial: Option<String>,
}

/// Keeps control interfaces of supported keyboards, de-duplicated by path.
pub fn select_devices<I>(candidates: I) -> Vec<DeviceInfo>
where
    I: IntoIterator<Item = Candidate>,
{
    let mut found: Vec<DeviceInfo> = Vec::new();
    for candidate in candidates {
        if candidate.usage_page != CONTROL_USAGE_PAGE {
            continue;
        }
        let Some(known) = identify(candidate.vendor_id, candidate.product_id) else {
            continue;
        };
        if found.iter().any(|d| d.path == candidate.path) {
            continue;
        }
        let device = DeviceInfo::from_known(known, candidate.path, candidate.serial);
        found.push(device);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use falchion_core::device::{ASUS_VENDOR_ID, KNOWN_DEVICES};

    fn candidate(product_id: u16, usage_page: u16, path: &str) -> Candidate {
        Candidate {
            vendor_id: ASUS_VENDOR_ID,
            product_id,
            usage_page,
            path: path.to_string(),
            serial: None,
        }
    }

    #[test]
    fn keeps_control_interface_of_known_device() {
        let pid = KNOWN_DEVICES[0].product_id;
        let found = select_devices([candidate(pid, CONTROL_USAGE_PAGE, "a")]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "a");
    }

    #[test]
    fn skips_other_interfaces_and_unknown_devices() {
        let pid = KNOWN_DEVICES[0].product_id;
        let keyboard_page = 0x0001;
        let mut other_vendor = candidate(pid, CONTROL_USAGE_PAGE, "c");
        other_vendor.vendor_id = 0x046D;
        let found = select_devices([
            candidate(pid, keyboard_page, "a"),
            candidate(0x0001, CONTROL_USAGE_PAGE, "b"),
            other_vendor,
        ]);
        assert!(found.is_empty());
    }

    #[test]
    fn deduplicates_by_path() {
        let pid = KNOWN_DEVICES[1].product_id;
        let dup = candidate(pid, CONTROL_USAGE_PAGE, "same");
        let found = select_devices([dup.clone(), dup]);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(select_devices(Vec::new()).is_empty());
    }
}
