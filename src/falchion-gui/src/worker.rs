//! Runs all device I/O on a background thread.
//!
//! **Never call a `DeviceProvider` on the GUI thread.** On macOS, hidapi pumps
//! the calling thread's run loop while enumerating devices. On the main thread
//! that re-enters winit's event handler, which panics inside a C callback and
//! aborts the process.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use falchion_core::{DeviceInfo, DeviceProvider, DeviceStatus, Keyboard, LightingSettings, Result};

/// Work the GUI asks the worker to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Re-scan for keyboards and read the first one's status.
    Refresh,
    /// Apply lighting to the first keyboard, optionally saving it.
    Apply {
        /// Settings to send.
        settings: LightingSettings,
        /// Whether to persist to the onboard profile.
        save: bool,
    },
}

/// Result of a [`Request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Outcome of [`Request::Refresh`].
    Devices {
        /// Keyboards found.
        devices: Result<Vec<DeviceInfo>>,
        /// Status of the first keyboard; `None` when none was found.
        status: Option<Result<DeviceStatus>>,
    },
    /// Outcome of [`Request::Apply`].
    Applied {
        /// Whether it succeeded.
        result: Result<()>,
        /// Echo of the request's `save` flag.
        save: bool,
    },
}

/// Executes one request against `provider`. Runs on the worker thread.
pub fn handle_request<P: DeviceProvider>(provider: &P, request: Request) -> Response {
    match request {
        Request::Refresh => {
            let devices = provider.list();
            let found = matches!(&devices, Ok(list) if !list.is_empty());
            let status = found.then(|| read_status(provider));
            Response::Devices { devices, status }
        }
        Request::Apply { settings, save } => {
            let result = apply(provider, &settings, save);
            Response::Applied { result, save }
        }
    }
}

fn read_status<P: DeviceProvider>(provider: &P) -> Result<DeviceStatus> {
    let (_, transport) = provider.open_first()?;
    Keyboard::new(transport).status()
}

fn apply<P: DeviceProvider>(provider: &P, settings: &LightingSettings, save: bool) -> Result<()> {
    let (_, transport) = provider.open_first()?;
    let mut keyboard = Keyboard::new(transport);
    keyboard.apply_lighting(settings)?;
    if save {
        keyboard.save()?;
    }
    Ok(())
}

/// Handle to the background device thread. Dropping it stops the thread.
#[derive(Debug)]
pub struct Worker {
    requests: Sender<Request>,
    responses: Receiver<Response>,
}

impl Worker {
    /// Starts the worker thread.
    ///
    /// `make_provider` runs on the worker thread, so the provider does not
    /// need to be `Send`. `notify` is called after each response is queued,
    /// typically to request a GUI repaint.
    pub fn spawn<P, F, N>(make_provider: F, notify: N) -> Self
    where
        P: DeviceProvider,
        F: FnOnce() -> P + Send + 'static,
        N: Fn() + Send + 'static,
    {
        let (requests, request_rx) = mpsc::channel::<Request>();
        let (response_tx, responses) = mpsc::channel();
        thread::spawn(move || {
            let provider = make_provider();
            for request in request_rx {
                let response = handle_request(&provider, request);
                if response_tx.send(response).is_err() {
                    break;
                }
                notify();
            }
        });
        Self {
            requests,
            responses,
        }
    }

    /// Queues a request. Returns `false` if the worker has stopped.
    pub fn send(&self, request: Request) -> bool {
        self.requests.send(request).is_ok()
    }

    /// Returns a finished response without blocking, if one is ready.
    pub fn try_recv(&self) -> Option<Response> {
        self.responses.try_recv().ok()
    }

    /// Waits up to `timeout` for a response.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Response> {
        self.responses.recv_timeout(timeout).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use falchion_core::protocol::{Command as Wire, Report};
    use falchion_core::{Error, MockProvider};

    const WAIT: Duration = Duration::from_secs(5);

    fn apply_request(save: bool) -> Request {
        Request::Apply {
            settings: LightingSettings::default(),
            save,
        }
    }

    fn info_reply(effect: u8) -> Report {
        Report::new(Wire::DeviceInfo, &[0, 0, 0x13, 0x61, 0x31, 0, 6, effect, 3]).unwrap()
    }

    #[test]
    fn refresh_lists_devices_and_reads_status() {
        let provider = MockProvider::with_device();
        provider.transport.queue_response(&info_reply(4));
        let response = handle_request(&provider, Request::Refresh);
        let Response::Devices { devices, status } = response else {
            panic!("unexpected {response:?}");
        };
        assert_eq!(devices.unwrap().len(), 1);
        let status = status.unwrap().unwrap();
        assert_eq!((status.profile, status.effect_code), (3, 4));
    }

    #[test]
    fn refresh_reports_status_errors_separately() {
        let provider = MockProvider {
            transport: falchion_core::MockTransport::new(),
            ..MockProvider::with_device()
        };
        let response = handle_request(&provider, Request::Refresh);
        let expected = Response::Devices {
            devices: Ok(provider.devices.clone()),
            status: Some(Err(Error::Timeout)),
        };
        assert_eq!(response, expected);
    }

    #[test]
    fn refresh_skips_status_without_devices_or_on_error() {
        let response = handle_request(&MockProvider::default(), Request::Refresh);
        let expected = Response::Devices {
            devices: Ok(vec![]),
            status: None,
        };
        assert_eq!(response, expected);

        let provider = MockProvider {
            list_error: Some(Error::Timeout),
            ..MockProvider::default()
        };
        let response = handle_request(&provider, Request::Refresh);
        let expected = Response::Devices {
            devices: Err(Error::Timeout),
            status: None,
        };
        assert_eq!(response, expected);
    }

    #[test]
    fn apply_writes_lighting_and_optionally_saves() {
        let provider = MockProvider::with_device();
        let response = handle_request(&provider, apply_request(false));
        let expected = Response::Applied {
            result: Ok(()),
            save: false,
        };
        assert_eq!(response, expected);
        let commands = |p: &MockProvider| -> Vec<[u8; 2]> {
            let written = p.transport.written().into_iter();
            written.map(|raw| [raw[0], raw[1]]).collect()
        };
        assert_eq!(commands(&provider), [Wire::SetEffect.bytes()]);

        handle_request(&provider, apply_request(true));
        let expected = [
            Wire::SetEffect.bytes(),
            Wire::SetEffect.bytes(),
            Wire::Save.bytes(),
        ];
        assert_eq!(commands(&provider), expected);
    }

    #[test]
    fn apply_without_device_fails() {
        let response = handle_request(&MockProvider::default(), apply_request(true));
        let expected = Response::Applied {
            result: Err(Error::DeviceNotFound),
            save: true,
        };
        assert_eq!(response, expected);
    }

    #[test]
    fn apply_reports_save_failure() {
        let provider = MockProvider::with_device();
        provider.transport.fail_writes_with(Error::Timeout);
        let response = handle_request(&provider, apply_request(true));
        assert!(matches!(response, Response::Applied { result: Err(_), .. }));
    }

    #[test]
    fn worker_runs_requests_off_thread_and_notifies() {
        let (notify_tx, notified) = mpsc::channel();
        let notify = move || {
            notify_tx.send(()).expect("notification receiver");
        };
        let worker = Worker::spawn(MockProvider::with_device, notify);
        assert!(worker.send(Request::Refresh));
        let response = worker.recv_timeout(WAIT).expect("response");
        assert!(matches!(response, Response::Devices { devices: Ok(_), .. }));
        notified.recv_timeout(WAIT).expect("notification");
        assert_eq!(notified.try_recv(), Err(mpsc::TryRecvError::Empty));
    }

    #[test]
    fn try_recv_is_empty_before_any_request() {
        let worker = Worker::spawn(MockProvider::default, || {});
        assert_eq!(worker.try_recv(), None);
    }

    #[test]
    fn try_recv_returns_finished_response() {
        let worker = Worker::spawn(MockProvider::default, || {});
        worker.send(apply_request(false));
        let mut response = None;
        for _ in 0..500 {
            response = worker.try_recv();
            if response.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(response, Some(Response::Applied { .. })));
    }

    #[test]
    fn send_fails_after_worker_thread_dies() {
        let worker = Worker::spawn(|| -> MockProvider { panic!("boom") }, || {});
        // The thread panics in make_provider, dropping its receiver.
        assert_eq!(worker.recv_timeout(WAIT), None);
        assert!(!worker.send(Request::Refresh));
    }
}
