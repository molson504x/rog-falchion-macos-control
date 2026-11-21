//! High-level keyboard operations built on a [`Transport`].

use crate::error::{Error, Result};
use crate::lighting::LightingSettings;
use crate::protocol::{parse_device_info, DeviceStatus, Report};
use crate::transport::Transport;

/// Default time to wait for each reply, in milliseconds. Armoury Crate waits
/// up to 1500 ms.
pub const DEFAULT_TIMEOUT_MS: i32 = 1500;

/// Unrelated reports (e.g. stale replies) skipped while waiting for an ack.
const MAX_SKIPPED_REPLIES: usize = 8;

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

    /// Sets how long to wait for each reply.
    pub fn with_timeout(mut self, timeout_ms: i32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Returns the underlying transport.
    pub fn into_inner(self) -> T {
        self.transport
    }

    /// Reads firmware versions, the active profile and the active effect.
    pub fn status(&mut self) -> Result<DeviceStatus> {
        let reply = self.transact(&Report::device_info())?;
        parse_device_info(&reply)
    }

    /// Reads the keyboard firmware version, e.g. `13.61.31`.
    pub fn firmware_version(&mut self) -> Result<String> {
        let status = self.status()?;
        status.firmware.ok_or(Error::DeviceNotFound)
    }

    /// Applies lighting settings. They may be lost on power-off unless saved.
    /// Invalid settings are rejected before anything is sent.
    pub fn apply_lighting(&mut self, settings: &LightingSettings) -> Result<()> {
        let report = Report::set_effect(settings)?;
        self.transact(&report).map(drop)
    }

    /// Saves current settings to the active onboard profile.
    pub fn save(&mut self) -> Result<()> {
        self.transact(&Report::save()).map(drop)
    }

    /// Sends `request` and waits for its echo, skipping unrelated reports.
    fn transact(&mut self, request: &Report) -> Result<Report> {
        self.transport.write(&request.encode())?;
        for _ in 0..=MAX_SKIPPED_REPLIES {
            let reply = Report::from_raw(self.transport.read(self.timeout_ms)?);
            if reply.is_rejection() {
                let [a, b] = request.command_bytes();
                return Err(Error::Rejected(format!("{a:02X} {b:02X}")));
            }
            if reply.acknowledges(request) {
                return Ok(reply);
            }
        }
        let msg = "no acknowledgement among the keyboard's replies".to_string();
        Err(Error::MalformedReport(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lighting::Effect;
    use crate::mock::MockTransport;
    use crate::protocol::{Command, REJECTED, REPORT_LEN};

    fn keyboard(mock: &MockTransport) -> Keyboard<MockTransport> {
        Keyboard::new(mock.clone()).with_timeout(10)
    }

    fn written_commands(mock: &MockTransport) -> Vec<Option<Command>> {
        let written = mock.written().into_iter();
        written.map(|raw| Report::from_raw(raw).command()).collect()
    }

    fn reply(bytes: &[u8]) -> [u8; REPORT_LEN] {
        let mut raw = [0u8; REPORT_LEN];
        raw[..bytes.len()].copy_from_slice(bytes);
        raw
    }

    #[test]
    fn status_and_firmware_success() {
        let mock = MockTransport::new();
        let info = reply(&[0x12, 0x00, 0, 0, 0x13, 0x61, 0x31, 0, 6, 4, 1]);
        mock.queue_raw(info);
        mock.queue_raw(info);
        let mut kb = keyboard(&mock);
        let status = kb.status().unwrap();
        assert_eq!(status.effect(), Some(Effect::Wave));
        assert_eq!(status.profile, 1);
        assert_eq!(kb.firmware_version(), Ok("13.61.31".to_string()));
        let fw = Some(Command::DeviceInfo);
        assert_eq!(written_commands(&mock), [fw, fw]);
    }

    #[test]
    fn firmware_fails_when_dongle_has_no_keyboard() {
        let mock = MockTransport::new();
        let mut raw = reply(&[0x12, 0x00, 0, 0, 7, 0, 2]);
        raw[11] = 0xFF;
        mock.queue_raw(raw);
        assert_eq!(
            keyboard(&mock).firmware_version(),
            Err(Error::DeviceNotFound)
        );
    }

    #[test]
    fn skips_unrelated_replies_before_the_echo() {
        let mock = MockTransport::new();
        mock.queue_raw(reply(&[0x43, 0x02]));
        mock.queue_raw(reply(&[0x50, 0x55]));
        mock.queue_raw(reply(&[0x12, 0x00, 0, 0, 1, 2, 3]));
        let status = keyboard(&mock).status().unwrap();
        assert_eq!(status.firmware.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn gives_up_after_too_many_unrelated_replies() {
        let mock = MockTransport::new();
        for _ in 0..=MAX_SKIPPED_REPLIES {
            mock.queue_raw(reply(&[0x43, 0x02]));
        }
        let result = keyboard(&mock).save();
        assert!(matches!(result, Err(Error::MalformedReport(_))));
    }

    #[test]
    fn rejection_is_reported() {
        let mock = MockTransport::new();
        mock.queue_raw(reply(&REJECTED));
        let result = keyboard(&mock).apply_lighting(&LightingSettings::default());
        assert_eq!(result, Err(Error::Rejected("51 2C".into())));
    }

    #[test]
    fn times_out_without_reply() {
        let mock = MockTransport::new();
        assert_eq!(keyboard(&mock).status(), Err(Error::Timeout));
        assert_eq!(keyboard(&mock).save(), Err(Error::Timeout));
    }

    #[test]
    fn apply_and_save_wait_for_echoes() {
        let mock = MockTransport::echoing();
        let mut kb = keyboard(&mock);
        kb.apply_lighting(&LightingSettings::default()).unwrap();
        kb.save().unwrap();
        let expected = [Some(Command::SetEffect), Some(Command::Save)];
        assert_eq!(written_commands(&mock), expected);
    }

    #[test]
    fn invalid_settings_are_not_sent() {
        let mock = MockTransport::echoing();
        let mut settings = LightingSettings::for_effect(Effect::Wave);
        settings.colors.clear();
        let result = keyboard(&mock).apply_lighting(&settings);
        assert!(matches!(result, Err(Error::InvalidSettings(_))));
        assert!(mock.written().is_empty());
    }

    #[test]
    fn write_errors_propagate() {
        let mock = MockTransport::echoing();
        mock.fail_writes_with(Error::Transport("unplugged".into()));
        let mut kb = keyboard(&mock);
        assert!(kb.apply_lighting(&LightingSettings::default()).is_err());
        assert!(kb.save().is_err());
        assert!(kb.status().is_err());
    }

    #[test]
    fn into_inner_returns_transport() {
        let mock = MockTransport::new();
        let mut inner = keyboard(&mock).into_inner();
        inner.write(&Report::save().encode()).unwrap();
        assert_eq!(mock.written().len(), 1);
    }
}
