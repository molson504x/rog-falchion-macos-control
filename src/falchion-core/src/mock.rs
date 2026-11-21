//! In-memory test doubles for [`Transport`] and [`DeviceProvider`].
//!
//! Used by unit tests across the workspace so front-end logic can be tested
//! without hardware.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::device::{DeviceInfo, KNOWN_DEVICES};
use crate::error::{Error, Result};
use crate::protocol::{RawReport, Report};
use crate::transport::{DeviceProvider, Transport};

#[derive(Debug, Default)]
struct MockState {
    written: Vec<RawReport>,
    responses: VecDeque<Result<RawReport>>,
    write_error: Option<Error>,
    echo: bool,
}

/// A transport that records writes and replays queued responses.
///
/// Clones share state, so a test can keep a handle while code under test owns
/// another. Reading with nothing queued returns [`Error::Timeout`]. An
/// [echoing](MockTransport::echoing) mock also queues an echo of every write,
/// like the real keyboard's acknowledgement.
#[derive(Debug, Clone, Default)]
pub struct MockTransport {
    state: Rc<RefCell<MockState>>,
}

impl MockTransport {
    /// Creates an empty mock.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a mock that acknowledges every write by echoing it.
    pub fn echoing() -> Self {
        let mock = Self::default();
        mock.state.borrow_mut().echo = true;
        mock
    }

    /// Queues `report` to be returned by the next unanswered read.
    pub fn queue_response(&self, report: &Report) {
        self.push(Ok(report.encode()));
    }

    /// Queues raw bytes to be returned by a read.
    pub fn queue_raw(&self, raw: RawReport) {
        self.push(Ok(raw));
    }

    /// Queues an error to be returned by a read.
    pub fn queue_error(&self, error: Error) {
        self.push(Err(error));
    }

    /// Makes every subsequent write fail with `error`.
    pub fn fail_writes_with(&self, error: Error) {
        self.state.borrow_mut().write_error = Some(error);
    }

    /// Every report written so far.
    pub fn written(&self) -> Vec<RawReport> {
        self.state.borrow().written.clone()
    }

    fn push(&self, response: Result<RawReport>) {
        self.state.borrow_mut().responses.push_back(response);
    }
}

impl Transport for MockTransport {
    fn write(&mut self, report: &RawReport) -> Result<()> {
        let mut state = self.state.borrow_mut();
        if let Some(err) = state.write_error.clone() {
            return Err(err);
        }
        state.written.push(*report);
        if state.echo {
            state.responses.push_back(Ok(*report));
        }
        Ok(())
    }

    fn read(&mut self, _timeout_ms: i32) -> Result<RawReport> {
        let next = self.state.borrow_mut().responses.pop_front();
        next.unwrap_or(Err(Error::Timeout))
    }
}

/// A provider with a fixed device list and a shared [`MockTransport`].
#[derive(Debug, Clone, Default)]
pub struct MockProvider {
    /// Devices returned by `list`.
    pub devices: Vec<DeviceInfo>,
    /// When set, `list` fails with this error.
    pub list_error: Option<Error>,
    /// When set, `open` fails with this error.
    pub open_error: Option<Error>,
    /// Transport handed out by `open` (clones share state).
    pub transport: MockTransport,
}

impl MockProvider {
    /// A provider with one wired Falchion attached that acknowledges every
    /// command (see [`MockTransport::echoing`]).
    pub fn with_device() -> Self {
        let device = DeviceInfo::from_known(&KNOWN_DEVICES[0], "mock://0", None);
        Self {
            devices: vec![device],
            transport: MockTransport::echoing(),
            ..Self::default()
        }
    }
}

impl DeviceProvider for MockProvider {
    type Transport = MockTransport;

    fn list(&self) -> Result<Vec<DeviceInfo>> {
        match &self.list_error {
            Some(err) => Err(err.clone()),
            None => Ok(self.devices.clone()),
        }
    }

    fn open(&self, _device: &DeviceInfo) -> Result<MockTransport> {
        match &self.open_error {
            Some(err) => Err(err.clone()),
            None => Ok(self.transport.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_writes_through_clones() {
        let handle = MockTransport::new();
        let mut owned = handle.clone();
        let raw = Report::save().encode();
        owned.write(&raw).unwrap();
        assert_eq!(handle.written(), vec![raw]);
    }

    #[test]
    fn replays_responses_in_order_then_times_out() {
        let mut mock = MockTransport::new();
        let raw = Report::save().encode();
        mock.queue_raw(raw);
        mock.queue_response(&Report::device_info());
        mock.queue_error(Error::Transport("x".into()));
        assert_eq!(mock.read(0), Ok(raw));
        assert_eq!(mock.read(0).map(|r| r[0]), Ok(0x12));
        assert_eq!(mock.read(0), Err(Error::Transport("x".into())));
        assert_eq!(mock.read(0), Err(Error::Timeout));
    }

    #[test]
    fn echoing_mock_acknowledges_writes() {
        let mut mock = MockTransport::echoing();
        let raw = Report::save().encode();
        mock.write(&raw).unwrap();
        assert_eq!(mock.read(0), Ok(raw));
        assert_eq!(MockTransport::new().read(0), Err(Error::Timeout));
    }

    #[test]
    fn write_failure_is_reported_and_not_recorded() {
        let mut mock = MockTransport::new();
        mock.fail_writes_with(Error::Timeout);
        assert_eq!(mock.write(&[0; 64]), Err(Error::Timeout));
        assert!(mock.written().is_empty());
    }

    #[test]
    fn provider_lists_and_opens() {
        let provider = MockProvider::with_device();
        assert_eq!(provider.list().unwrap().len(), 1);
        assert!(provider.open(&provider.devices[0]).is_ok());
    }

    #[test]
    fn provider_errors() {
        let provider = MockProvider {
            list_error: Some(Error::Timeout),
            open_error: Some(Error::DeviceNotFound),
            ..MockProvider::with_device()
        };
        assert_eq!(provider.list(), Err(Error::Timeout));
        let err = provider.open(&provider.devices[0]).err();
        assert_eq!(err, Some(Error::DeviceNotFound));
    }
}
