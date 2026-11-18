//! hidapi wrapper. Requires real hardware, so it is excluded from coverage;
//! keep logic out of this file (put it in `discovery.rs` or `falchion-core`).

use std::ffi::CString;

use falchion_core::{DeviceInfo, DeviceProvider, Error, RawReport, Result, Transport, REPORT_LEN};
use hidapi::{HidApi, HidDevice, HidError};

use crate::discovery::{select_devices, Candidate};

fn transport_error(err: HidError) -> Error {
    Error::Transport(err.to_string())
}

fn candidate(info: &hidapi::DeviceInfo) -> Candidate {
    Candidate {
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        usage_page: info.usage_page(),
        path: info.path().to_string_lossy().into_owned(),
        serial: info.serial_number().map(str::to_owned),
    }
}

/// An open connection to one keyboard's control interface.
pub struct HidTransport {
    device: HidDevice,
}

impl Transport for HidTransport {
    fn write(&mut self, report: &RawReport) -> Result<()> {
        // hidapi expects the report ID first; 0x00 = unnumbered. PLACEHOLDER.
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[1..].copy_from_slice(report);
        self.device.write(&buf).map_err(transport_error)?;
        Ok(())
    }

    fn read(&mut self, timeout_ms: i32) -> Result<RawReport> {
        let mut buf = [0u8; REPORT_LEN];
        let read = self.device.read_timeout(&mut buf, timeout_ms);
        match read.map_err(transport_error)? {
            0 => Err(Error::Timeout),
            _ => Ok(buf),
        }
    }
}

/// Discovers and opens keyboards through the OS HID stack.
#[derive(Debug, Clone, Copy, Default)]
pub struct HidProvider;

impl DeviceProvider for HidProvider {
    type Transport = HidTransport;

    fn list(&self) -> Result<Vec<DeviceInfo>> {
        let api = HidApi::new().map_err(transport_error)?;
        Ok(select_devices(api.device_list().map(candidate)))
    }

    fn open(&self, device: &DeviceInfo) -> Result<HidTransport> {
        let api = HidApi::new().map_err(transport_error)?;
        let path = CString::new(device.path.as_str());
        let path = path.map_err(|e| Error::Transport(e.to_string()))?;
        let device = api.open_path(&path).map_err(transport_error)?;
        Ok(HidTransport { device })
    }
}
