//! UI-independent application state. Never touches devices directly; see
//! [`crate::worker`].

use falchion_core::{Brightness, DeviceInfo, LightingSettings, Result, Speed};

use crate::worker::Response;

/// Message shown in the status area.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    /// Nothing to report.
    #[default]
    Idle,
    /// A successful or in-progress operation.
    Info(String),
    /// A failed operation.
    Error(String),
}

/// State backing the GUI.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// Lighting settings being edited.
    pub settings: LightingSettings,
    /// Keyboards found by the last refresh.
    pub devices: Vec<DeviceInfo>,
    /// Latest status message.
    pub status: Status,
    /// True while a request is in flight; the UI disables actions.
    pub busy: bool,
}

impl AppState {
    /// Records whether the worker accepted a request.
    pub fn on_sent(&mut self, accepted: bool) {
        if accepted {
            self.busy = true;
            self.status = Status::Info("Working…".into());
        } else {
            let msg = "Device worker stopped; restart the app.";
            self.status = Status::Error(msg.into());
        }
    }

    /// Applies a worker response.
    pub fn on_response(&mut self, response: Response) {
        self.busy = false;
        match response {
            Response::Devices(result) => self.devices_loaded(result),
            Response::Applied { result, save } => self.applied(result, save),
        }
    }

    fn devices_loaded(&mut self, result: Result<Vec<DeviceInfo>>) {
        match result {
            Ok(devices) => {
                self.status = match devices.len() {
                    0 => Status::Error("No ROG Falchion keyboards found.".into()),
                    n => Status::Info(format!("Found {n} keyboard(s).")),
                };
                self.devices = devices;
            }
            Err(err) => {
                self.devices.clear();
                self.status = Status::Error(err.to_string());
            }
        }
    }

    fn applied(&mut self, result: Result<()>, save: bool) {
        self.status = match result {
            Ok(()) if save => Status::Info("Lighting applied and saved.".into()),
            Ok(()) => Status::Info("Lighting applied.".into()),
            Err(err) => Status::Error(err.to_string()),
        };
    }

    /// Sets brightness; invalid values leave it unchanged and set an error.
    pub fn set_brightness(&mut self, value: u8) {
        match Brightness::new(value) {
            Ok(level) => self.settings.brightness = level,
            Err(err) => self.status = Status::Error(err.to_string()),
        }
    }

    /// Sets speed; invalid values leave it unchanged and set an error.
    pub fn set_speed(&mut self, value: u8) {
        match Speed::new(value) {
            Ok(speed) => self.settings.speed = speed,
            Err(err) => self.status = Status::Error(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::{handle_request, Request};
    use falchion_core::{Error, MockProvider};

    fn refreshed(provider: &MockProvider) -> AppState {
        let mut state = AppState::default();
        state.on_response(handle_request(provider, Request::Refresh));
        state
    }

    fn applied(result: Result<()>, save: bool) -> Response {
        Response::Applied { result, save }
    }

    #[test]
    fn on_sent_marks_busy_or_reports_dead_worker() {
        let mut state = AppState::default();
        state.on_sent(true);
        assert!(state.busy);
        assert!(matches!(state.status, Status::Info(_)));

        let mut state = AppState::default();
        state.on_sent(false);
        assert!(!state.busy);
        assert!(matches!(state.status, Status::Error(_)));
    }

    #[test]
    fn refresh_finds_devices_and_clears_busy() {
        let mut state = AppState::default();
        state.on_sent(true);
        let response = handle_request(&MockProvider::with_device(), Request::Refresh);
        state.on_response(response);
        assert!(!state.busy);
        assert_eq!(state.devices.len(), 1);
        assert_eq!(state.status, Status::Info("Found 1 keyboard(s).".into()));
    }

    #[test]
    fn refresh_without_devices_reports_error() {
        let state = refreshed(&MockProvider::default());
        assert!(matches!(state.status, Status::Error(_)));
    }

    #[test]
    fn refresh_failure_clears_devices() {
        let mut state = refreshed(&MockProvider::with_device());
        let failure = Response::Devices(Err(Error::Transport("hid".into())));
        state.on_response(failure);
        assert!(state.devices.is_empty());
        assert!(matches!(&state.status, Status::Error(m) if m.contains("hid")));
    }

    #[test]
    fn applied_messages() {
        let mut state = AppState::default();
        state.on_response(applied(Ok(()), false));
        assert_eq!(state.status, Status::Info("Lighting applied.".into()));
        state.on_response(applied(Ok(()), true));
        let saved = Status::Info("Lighting applied and saved.".into());
        assert_eq!(state.status, saved);
    }

    #[test]
    fn applied_failure_reports_error() {
        let mut state = AppState::default();
        state.on_response(applied(Err(Error::DeviceNotFound), false));
        let expected = Status::Error(Error::DeviceNotFound.to_string());
        assert_eq!(state.status, expected);
    }

    #[test]
    fn setters_validate_range() {
        let mut state = AppState::default();
        state.set_brightness(1);
        state.set_speed(2);
        assert_eq!(state.settings.brightness.value(), 1);
        assert_eq!(state.settings.speed.value(), 2);
        assert_eq!(state.status, Status::Idle);

        state.set_brightness(99);
        assert_eq!(state.settings.brightness.value(), 1);
        assert!(matches!(state.status, Status::Error(_)));

        state.status = Status::Idle;
        state.set_speed(99);
        assert_eq!(state.settings.speed.value(), 2);
        assert!(matches!(state.status, Status::Error(_)));
    }
}
