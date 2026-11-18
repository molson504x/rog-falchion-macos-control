//! Abstractions over how reports reach the keyboard.

use crate::device::DeviceInfo;
use crate::error::{Error, Result};
use crate::protocol::RawReport;

/// A bidirectional channel to one keyboard.
pub trait Transport {
    /// Sends one report.
    fn write(&mut self, report: &RawReport) -> Result<()>;

    /// Reads one report, waiting at most `timeout_ms` milliseconds.
    fn read(&mut self, timeout_ms: i32) -> Result<RawReport>;
}

/// Finds and opens keyboards.
pub trait DeviceProvider {
    /// The transport type this provider opens.
    type Transport: Transport;

    /// Lists connected, supported keyboards.
    fn list(&self) -> Result<Vec<DeviceInfo>>;

    /// Opens `device`.
    fn open(&self, device: &DeviceInfo) -> Result<Self::Transport>;

    /// Opens the first connected keyboard.
    fn open_first(&self) -> Result<(DeviceInfo, Self::Transport)> {
        let device = self.list()?.into_iter().next();
        let device = device.ok_or(Error::DeviceNotFound)?;
        let transport = self.open(&device)?;
        Ok((device, transport))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockProvider;

    #[test]
    fn open_first_returns_first_device() {
        let provider = MockProvider::with_device();
        let (device, _) = provider.open_first().unwrap();
        assert_eq!(device, provider.devices[0]);
    }

    #[test]
    fn open_first_without_devices_fails() {
        let provider = MockProvider::default();
        assert_eq!(provider.open_first().err(), Some(Error::DeviceNotFound));
    }

    #[test]
    fn open_first_propagates_open_error() {
        let provider = MockProvider {
            open_error: Some(Error::Transport("busy".into())),
            ..MockProvider::with_device()
        };
        let err = provider.open_first().err();
        assert_eq!(err, Some(Error::Transport("busy".into())));
    }
}
