//! High-level keyboard operations built on a [`Transport`].

use crate::error::{Error, Result};
use crate::lighting::LightingSettings;
use crate::protocol::{parse_firmware_version, Report};
use crate::transport::Transport;

/// Default time to wait for a response, in milliseconds.
pub const DEFAULT_TIMEOUT_MS: i32 = 1000;

/// A connected keyboard.
#[derive(Debug)]
pub struct Keyboard<T> {
    transport: T,
    timeout_ms: i32,
}

impl<T: Transport> Keyboard<T> {
    /// Wraps an open transport.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    /// Sets how long to wait for responses.
    pub fn with_timeout(mut self, timeout_ms: i32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Returns the underlying transport.
    pub fn into_inner(self) -> T {
        self.transport
    }

    /// Reads the firmware version as `major.minor.patch`.
    pub fn firmware_version(&mut self) -> Result<String> {
        let response = self.request(&Report::get_firmware_version())?;
        parse_firmware_version(&response)
    }

    /// Applies lighting settings. They are lost on power-off unless saved.
    pub fn apply_lighting(&mut self, settings: &LightingSettings) -> Result<()> {
        self.send(&Report::set_lighting(settings))
    }

    /// Persists current settings to the active onboard profile.
    pub fn save(&mut self) -> Result<()> {
        self.send(&Report::save())
    }

    fn send(&mut self, report: &Report) -> Result<()> {
        self.transport.write(&report.encode())
    }

    fn request(&mut self, report: &Report) -> Result<Report> {
        self.send(report)?;
        let raw = self.transport.read(self.timeout_ms)?;
        let response = Report::decode(&raw)?;
        let expected = report.command();
        let actual = response.command();
        if actual != expected {
            let msg = format!("expected {expected:?} response, got {actual:?}");
            return Err(Error::MalformedReport(msg));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockTransport;
    use crate::protocol::Command;

    const FW: Command = Command::GetFirmwareVersion;

    fn keyboard() -> (MockTransport, Keyboard<MockTransport>) {
        let mock = MockTransport::new();
        (mock.clone(), Keyboard::new(mock).with_timeout(10))
    }

    fn written_commands(mock: &MockTransport) -> Vec<Command> {
        let mut commands = Vec::new();
        for raw in mock.written() {
            commands.push(Report::decode(&raw).unwrap().command());
        }
        commands
    }

    #[test]
    fn firmware_version_success() {
        let (mock, mut kb) = keyboard();
        let response = Report::new(FW, &[1, 0, 7]).unwrap();
        mock.queue_response(&response);
        assert_eq!(kb.firmware_version(), Ok("1.0.7".to_string()));
        assert_eq!(written_commands(&mock), [FW]);
    }

    #[test]
    fn firmware_version_rejects_mismatched_response() {
        let (mock, mut kb) = keyboard();
        mock.queue_response(&Report::save());
        let result = kb.firmware_version();
        assert!(matches!(result, Err(Error::MalformedReport(_))));
    }

    #[test]
    fn firmware_version_times_out_without_response() {
        let (_, mut kb) = keyboard();
        assert_eq!(kb.firmware_version(), Err(Error::Timeout));
    }

    #[test]
    fn firmware_version_rejects_garbage() {
        let (mock, mut kb) = keyboard();
        mock.queue_raw([0xAA; crate::REPORT_LEN]);
        let result = kb.firmware_version();
        assert!(matches!(result, Err(Error::MalformedReport(_))));
    }

    #[test]
    fn apply_and_save_write_reports() {
        let (mock, mut kb) = keyboard();
        kb.apply_lighting(&LightingSettings::default()).unwrap();
        kb.save().unwrap();
        let expected = [Command::SetLighting, Command::Save];
        assert_eq!(written_commands(&mock), expected);
    }

    #[test]
    fn write_errors_propagate() {
        let (mock, mut kb) = keyboard();
        mock.fail_writes_with(Error::Transport("unplugged".into()));
        assert!(kb.apply_lighting(&LightingSettings::default()).is_err());
        assert!(kb.save().is_err());
        assert!(kb.firmware_version().is_err());
    }

    #[test]
    fn into_inner_returns_transport() {
        let (mock, kb) = keyboard();
        let mut inner = kb.into_inner();
        inner.write(&Report::save().encode()).unwrap();
        assert_eq!(mock.written().len(), 1);
    }
}
