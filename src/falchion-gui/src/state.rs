//! UI-independent application state. Never touches devices directly; see
//! [`crate::worker`]. Every edit the lighting controls make goes through a
//! method here so it stays valid and testable.

use falchion_core::{
    Brightness, ColorMode, DeviceInfo, DeviceStatus, Direction, Effect, LightingSettings, Result,
    Rgb, Speed, Thickness,
};

use crate::worker::{Request, Response};

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
    /// Lighting settings being edited. Always valid.
    pub settings: LightingSettings,
    /// Keyboards found by the last refresh.
    pub devices: Vec<DeviceInfo>,
    /// What the first keyboard reported on the last refresh.
    pub keyboard: Option<DeviceStatus>,
    /// Latest status message.
    pub status: Status,
    /// True while a request is in flight; the UI disables actions.
    pub busy: bool,
    /// Send every edit to the keyboard straight away (not saved).
    pub live_preview: bool,
    /// Edits not yet sent to the keyboard.
    pub pending: bool,
    /// Whether the effect selection was loaded from the keyboard.
    synced: bool,
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
            Response::Devices { devices, status } => self.devices_loaded(devices, status),
            Response::Applied { result, save } => self.applied(result, save),
        }
    }

    fn devices_loaded(
        &mut self,
        devices: Result<Vec<DeviceInfo>>,
        status: Option<Result<DeviceStatus>>,
    ) {
        match devices {
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
        self.keyboard = match status {
            Some(Ok(status)) => Some(status),
            Some(Err(err)) => {
                let msg = format!("Found a keyboard, but could not read it: {err}");
                self.status = Status::Error(msg);
                None
            }
            None => None,
        };
        // Show the keyboard's current effect once, unless the user started editing.
        let awake = self.keyboard.as_ref().filter(|k| k.firmware.is_some());
        let effect = awake.and_then(DeviceStatus::effect);
        if let Some(effect) = effect.filter(|_| !self.synced && !self.pending) {
            self.settings = self.settings.with_effect(effect);
            self.synced = true;
        }
    }

    fn applied(&mut self, result: Result<()>, save: bool) {
        self.status = match result {
            Ok(()) if save => Status::Info("Lighting applied and saved to the keyboard.".into()),
            Ok(()) => Status::Info("Lighting applied (not saved yet).".into()),
            Err(err) => Status::Error(err.to_string()),
        };
    }

    /// One-line description of the connected keyboard.
    pub fn keyboard_summary(&self) -> String {
        let Some(device) = self.devices.first() else {
            return "No keyboard connected".into();
        };
        let mut text = format!("{} ({})", device.name, device.connection);
        if let Some(status) = &self.keyboard {
            match &status.firmware {
                Some(firmware) => {
                    text.push_str(&format!(" · firmware {firmware}"));
                    text.push_str(&format!(" · profile {}", status.profile));
                }
                None => text.push_str(" · keyboard asleep or off: press a key, then Refresh"),
            }
        }
        text
    }

    /// A request to apply the current settings, clearing [`Self::pending`].
    pub fn apply_request(&mut self, save: bool) -> Request {
        self.pending = false;
        Request::Apply {
            settings: self.settings.clone(),
            save,
        }
    }

    /// With live preview on, returns a request for unsent edits once the
    /// worker is idle.
    pub fn take_live_request(&mut self) -> Option<Request> {
        if self.live_preview && self.pending && !self.busy {
            Some(self.apply_request(false))
        } else {
            None
        }
    }

    fn edit(&mut self, change: impl FnOnce(&mut LightingSettings)) {
        let mut next = self.settings.clone();
        change(&mut next);
        match next.validate() {
            Ok(()) if next != self.settings => {
                self.settings = next;
                self.pending = true;
            }
            Ok(()) => {}
            Err(err) => self.status = Status::Error(err.to_string()),
        }
    }

    /// Switches effect, keeping the options it can use.
    pub fn select_effect(&mut self, effect: Effect) {
        let next = self.settings.with_effect(effect);
        self.edit(|s| *s = next);
    }

    /// Restores Armoury Crate's factory default (rainbow wave, 50%).
    pub fn reset_to_factory(&mut self) {
        self.edit(|s| *s = LightingSettings::factory_default());
    }

    /// Picks a color mode the effect supports.
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.edit(|s| s.color_mode = mode);
    }

    /// Sets color `index` (as listed by [`Self::color_labels`]).
    pub fn set_color(&mut self, index: usize, color: Rgb) {
        self.edit(|s| {
            if let Some(slot) = s.colors.get_mut(index) {
                *slot = color;
            }
        });
    }

    /// Whether another gradient color can be added.
    pub fn can_add_color(&self) -> bool {
        self.gradient_bounds()
            .is_some_and(|(_, max)| self.settings.colors.len() < max)
    }

    /// Whether a gradient color can be removed.
    pub fn can_remove_color(&self) -> bool {
        self.gradient_bounds()
            .is_some_and(|(min, _)| self.settings.colors.len() > min)
    }

    fn gradient_bounds(&self) -> Option<(usize, usize)> {
        if self.settings.color_mode != ColorMode::Gradient {
            return None;
        }
        let range = self.settings.effect.gradient_len()?;
        Some((*range.start(), *range.end()))
    }

    /// Adds a gradient color, continuing the rainbow.
    pub fn add_color(&mut self) {
        if self.can_add_color() {
            let next = falchion_core::RAINBOW[self.settings.colors.len() % 7];
            self.edit(|s| s.colors.push(next));
        }
    }

    /// Removes the last gradient color.
    pub fn remove_color(&mut self) {
        if self.can_remove_color() {
            self.edit(|s| {
                s.colors.pop();
            });
        }
    }

    /// Sets or clears the background color.
    pub fn set_background(&mut self, background: Option<Rgb>) {
        self.edit(|s| s.background = background);
    }

    /// Sets the animation direction.
    pub fn set_direction(&mut self, direction: Direction) {
        self.edit(|s| s.direction = direction);
    }

    /// Sets thickness or density.
    pub fn set_thickness(&mut self, thickness: Thickness) {
        self.edit(|s| s.thickness = thickness);
    }

    /// Sets brightness; invalid values leave it unchanged and set an error.
    pub fn set_brightness(&mut self, value: u8) {
        match Brightness::new(value) {
            Ok(level) => self.edit(|s| s.brightness = level),
            Err(err) => self.status = Status::Error(err.to_string()),
        }
    }

    /// Sets speed; invalid values leave it unchanged and set an error.
    pub fn set_speed(&mut self, value: u8) {
        match Speed::new(value) {
            Ok(speed) => self.edit(|s| s.speed = speed),
            Err(err) => self.status = Status::Error(err.to_string()),
        }
    }

    /// Labels for the color pickers to show, one per editable color.
    pub fn color_labels(&self) -> Vec<String> {
        let settings = &self.settings;
        if settings.effect.color_modes().is_empty() {
            return Vec::new();
        }
        let count = settings.active_colors().len();
        match settings.color_mode {
            ColorMode::Single => vec!["Color".into()],
            ColorMode::Double => vec!["Color 1".into(), "Color 2".into()],
            ColorMode::Random => Vec::new(),
            ColorMode::Gradient if settings.effect == Effect::Quicksand => (1..=count)
                .map(|row| match row {
                    1 => "Row 1 (top)".to_string(),
                    _ => format!("Row {row}"),
                })
                .collect(),
            ColorMode::Gradient => (1..=count).map(|i| format!("Stop {i}")).collect(),
        }
    }

    /// Heading for the thickness control: "Density" for Starry night.
    pub fn thickness_heading(&self) -> &'static str {
        match self.settings.effect {
            Effect::StarryNight => "Density",
            _ => "Thickness",
        }
    }
}

/// Label for one thickness option, worded for the effect.
pub fn thickness_label(effect: Effect, thickness: Thickness) -> &'static str {
    match (effect, thickness) {
        (Effect::StarryNight, Thickness::Slim) => "Fewer stars",
        (Effect::StarryNight, Thickness::Normal) => "Normal",
        (Effect::StarryNight, Thickness::Thick) => "More stars",
        (_, Thickness::Slim) => "Slim",
        (_, Thickness::Normal) => "Normal",
        (_, Thickness::Thick) => "Thick",
    }
}

/// Label for a brightness level: `Off` or a percentage.
pub fn brightness_label(level: u8) -> String {
    match Brightness::new(level) {
        Ok(b) if b.value() == 0 => "Off".into(),
        Ok(b) => format!("{}%", b.percent()),
        Err(_) => "?".into(),
    }
}

/// Label for a speed level, e.g. `Normal`.
pub fn speed_label(level: u8) -> &'static str {
    Speed::new(level).map_or("?", Speed::label)
}

/// Arrow for a direction button.
pub fn direction_glyph(direction: Direction) -> &'static str {
    match direction {
        Direction::Right => "→",
        Direction::DownRight => "↘",
        Direction::Down => "↓",
        Direction::DownLeft => "↙",
        Direction::Left => "←",
        Direction::UpLeft => "↖",
        Direction::Up => "↑",
        Direction::UpRight => "↗",
        Direction::Out => "✳",
    }
}

/// Wave directions laid out as a 3 × 3 pad, by where the light moves to.
pub const DIRECTION_PAD: [[Direction; 3]; 3] = [
    [Direction::UpLeft, Direction::Up, Direction::UpRight],
    [Direction::Left, Direction::Out, Direction::Right],
    [Direction::DownLeft, Direction::Down, Direction::DownRight],
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::handle_request;
    use falchion_core::protocol::{Command as Wire, Report};
    use falchion_core::{ConnectionKind, Error, MockProvider, RAINBOW};

    fn status(effect: u8, firmware: bool) -> DeviceStatus {
        DeviceStatus {
            firmware: firmware.then(|| "13.61.31".to_string()),
            dongle_firmware: None,
            profile: 2,
            effect_code: effect,
        }
    }

    fn devices(effect: u8) -> Response {
        Response::Devices {
            devices: Ok(MockProvider::with_device().devices),
            status: Some(Ok(status(effect, true))),
        }
    }

    fn applied(result: Result<()>, save: bool) -> Response {
        Response::Applied { result, save }
    }

    fn state_with(effect: Effect) -> AppState {
        let mut state = AppState::default();
        state.select_effect(effect);
        state.pending = false;
        state
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
    fn refresh_finds_devices_and_selects_current_effect() {
        let provider = MockProvider::with_device();
        let info = [0, 0, 0x13, 0x61, 0x31, 0, 6, Effect::Ripple.code(), 2];
        provider
            .transport
            .queue_response(&Report::new(Wire::DeviceInfo, &info).unwrap());
        let mut state = AppState::default();
        state.on_sent(true);
        state.on_response(handle_request(&provider, crate::worker::Request::Refresh));
        assert!(!state.busy);
        assert_eq!(state.devices.len(), 1);
        assert_eq!(state.status, Status::Info("Found 1 keyboard(s).".into()));
        assert_eq!(state.settings.effect, Effect::Ripple);
        assert!(!state.pending, "loading is not an edit");
    }

    #[test]
    fn refresh_selects_effect_only_once_and_not_over_edits() {
        let mut state = AppState::default();
        state.on_response(devices(Effect::Static.code()));
        assert_eq!(state.settings.effect, Effect::Static);
        state.on_response(devices(Effect::Wave.code()));
        assert_eq!(state.settings.effect, Effect::Static, "already synced");

        let mut edited = AppState::default();
        edited.select_effect(Effect::Current);
        edited.on_response(devices(Effect::Static.code()));
        assert_eq!(edited.settings.effect, Effect::Current);
    }

    #[test]
    fn refresh_ignores_effect_when_dongle_has_no_keyboard() {
        let mut state = AppState::default();
        state.on_response(Response::Devices {
            devices: Ok(MockProvider::with_device().devices),
            status: Some(Ok(status(Effect::Static.code(), false))),
        });
        assert_eq!(state.settings, LightingSettings::factory_default());
    }

    #[test]
    fn refresh_ignores_unknown_effect() {
        let mut state = AppState::default();
        state.on_response(devices(15));
        assert_eq!(state.settings, LightingSettings::factory_default());
        assert!(state.keyboard.is_some());
    }

    #[test]
    fn refresh_without_devices_reports_error() {
        let mut state = AppState::default();
        state.on_response(handle_request(
            &MockProvider::default(),
            crate::worker::Request::Refresh,
        ));
        assert!(matches!(state.status, Status::Error(_)));
        assert_eq!(state.keyboard_summary(), "No keyboard connected");
    }

    #[test]
    fn refresh_failure_clears_devices() {
        let mut state = AppState::default();
        state.on_response(devices(0));
        state.on_response(Response::Devices {
            devices: Err(Error::Transport("hid".into())),
            status: None,
        });
        assert!(state.devices.is_empty());
        assert!(state.keyboard.is_none());
        assert!(matches!(&state.status, Status::Error(m) if m.contains("hid")));
    }

    #[test]
    fn status_read_failure_is_reported() {
        let mut state = AppState::default();
        state.on_response(Response::Devices {
            devices: Ok(MockProvider::with_device().devices),
            status: Some(Err(Error::Timeout)),
        });
        assert_eq!(state.devices.len(), 1);
        assert!(matches!(&state.status, Status::Error(m) if m.contains("could not read")));
    }

    #[test]
    fn keyboard_summary_shows_firmware_and_profile() {
        let mut state = AppState::default();
        state.on_response(devices(0));
        let summary = state.keyboard_summary();
        assert_eq!(
            summary,
            "ROG Falchion (wired) · firmware 13.61.31 · profile 2"
        );
        state.keyboard = Some(status(0, false));
        let asleep = state.keyboard_summary();
        assert!(
            asleep.contains("asleep") && !asleep.contains("profile"),
            "{asleep}"
        );
        state.keyboard = None;
        state.devices[0].connection = ConnectionKind::Wireless;
        assert_eq!(state.keyboard_summary(), "ROG Falchion (wireless)");
    }

    #[test]
    fn applied_messages() {
        let mut state = AppState::default();
        state.on_response(applied(Ok(()), false));
        assert!(matches!(&state.status, Status::Info(m) if m.contains("not saved")));
        state.on_response(applied(Ok(()), true));
        assert!(matches!(&state.status, Status::Info(m) if m.contains("saved to")));
        state.on_response(applied(Err(Error::DeviceNotFound), false));
        let expected = Status::Error(Error::DeviceNotFound.to_string());
        assert_eq!(state.status, expected);
    }

    #[test]
    fn edits_mark_pending_and_apply_clears_it() {
        let mut state = AppState::default();
        assert!(!state.pending);
        state.set_brightness(1);
        assert!(state.pending);
        let request = state.apply_request(true);
        assert!(!state.pending);
        let Request::Apply { settings, save } = request else {
            panic!("expected apply");
        };
        assert!(save);
        assert_eq!(settings.brightness.value(), 1);
    }

    #[test]
    fn no_op_edits_are_not_pending() {
        let mut state = AppState::default();
        state.set_brightness(state.settings.brightness.value());
        state.select_effect(state.settings.effect);
        assert!(!state.pending);
    }

    #[test]
    fn live_preview_sends_pending_edits_when_idle() {
        let mut state = AppState::default();
        state.set_speed(4);
        assert_eq!(state.take_live_request(), None, "live preview off");
        state.live_preview = true;
        state.busy = true;
        assert_eq!(state.take_live_request(), None, "worker busy");
        state.busy = false;
        assert!(matches!(
            state.take_live_request(),
            Some(Request::Apply { save: false, .. })
        ));
        assert_eq!(state.take_live_request(), None, "nothing pending");
    }

    #[test]
    fn setters_validate_range() {
        let mut state = AppState::default();
        state.set_brightness(1);
        state.set_speed(4);
        assert_eq!(state.settings.brightness.value(), 1);
        assert_eq!(state.settings.speed.value(), 4);
        assert_eq!(state.status, Status::Idle);

        state.set_brightness(99);
        assert_eq!(state.settings.brightness.value(), 1);
        assert!(matches!(state.status, Status::Error(_)));

        state.status = Status::Idle;
        state.set_speed(99);
        assert_eq!(state.settings.speed.value(), 4);
        assert!(matches!(state.status, Status::Error(_)));
    }

    #[test]
    fn invalid_edits_are_refused() {
        let mut state = state_with(Effect::Static);
        state.set_color_mode(ColorMode::Random);
        assert_eq!(state.settings.color_mode, ColorMode::Single);
        assert!(matches!(state.status, Status::Error(_)));
        state.set_background(Some(Rgb::BLUE));
        assert_eq!(state.settings.background, None);
        let mut quicksand = state_with(Effect::Quicksand);
        quicksand.set_direction(Direction::Left);
        assert_eq!(quicksand.settings.direction, Direction::Up);
        assert!(!quicksand.pending);
    }

    #[test]
    fn valid_option_edits_apply() {
        let mut state = state_with(Effect::StarryNight);
        state.set_color_mode(ColorMode::Double);
        state.set_color(1, Rgb::GREEN);
        state.set_background(Some(Rgb::BLUE));
        state.set_thickness(Thickness::Slim);
        assert_eq!(state.settings.colors[1], Rgb::GREEN);
        assert_eq!(state.settings.background, Some(Rgb::BLUE));
        assert_eq!(state.settings.thickness, Thickness::Slim);
        state.set_color(99, Rgb::GREEN);
        assert_eq!(state.status, Status::Idle, "out-of-range index is ignored");
        let mut wave = state_with(Effect::Wave);
        wave.set_direction(Direction::Out);
        assert_eq!(wave.settings.direction, Direction::Out);
    }

    #[test]
    fn gradient_colors_can_grow_and_shrink_within_limits() {
        let mut state = state_with(Effect::Wave);
        assert_eq!(state.settings.colors.len(), 7);
        assert!(!state.can_add_color());
        state.add_color();
        assert_eq!(state.settings.colors.len(), 7);
        for _ in 0..10 {
            state.remove_color();
        }
        assert_eq!(state.settings.colors.len(), 1);
        assert!(!state.can_remove_color());
        state.add_color();
        assert_eq!(state.settings.colors, RAINBOW[..2]);
    }

    #[test]
    fn non_gradient_colors_cannot_grow() {
        let quicksand = state_with(Effect::Quicksand);
        assert!(!quicksand.can_add_color() && !quicksand.can_remove_color());
        let mut single = state_with(Effect::Breathing);
        single.add_color();
        single.remove_color();
        assert!(!single.pending);
    }

    #[test]
    fn reset_restores_factory_default() {
        let mut state = state_with(Effect::Static);
        state.reset_to_factory();
        assert_eq!(state.settings, LightingSettings::factory_default());
        assert!(state.pending);
    }

    #[test]
    fn color_labels_follow_mode() {
        let mut state = state_with(Effect::Breathing);
        assert_eq!(state.color_labels(), ["Color"]);
        state.set_color_mode(ColorMode::Double);
        assert_eq!(state.color_labels(), ["Color 1", "Color 2"]);
        state.set_color_mode(ColorMode::Random);
        assert!(state.color_labels().is_empty());
        let wave = state_with(Effect::Wave);
        assert_eq!(wave.color_labels().len(), 7);
        assert_eq!(wave.color_labels()[6], "Stop 7");
        let quicksand = state_with(Effect::Quicksand);
        assert_eq!(quicksand.color_labels()[0], "Row 1 (top)");
        assert_eq!(quicksand.color_labels().len(), 6);
        assert!(state_with(Effect::ColorCycle).color_labels().is_empty());
    }

    #[test]
    fn labels_for_thickness_brightness_and_speed() {
        assert_eq!(
            state_with(Effect::StarryNight).thickness_heading(),
            "Density"
        );
        assert_eq!(state_with(Effect::Wave).thickness_heading(), "Thickness");
        assert_eq!(
            thickness_label(Effect::StarryNight, Thickness::Thick),
            "More stars"
        );
        assert_eq!(thickness_label(Effect::Wave, Thickness::Slim), "Slim");
        assert_eq!(brightness_label(0), "Off");
        assert_eq!(brightness_label(3), "75%");
        assert_eq!(brightness_label(9), "?");
        assert_eq!(speed_label(4), "Fastest");
        assert_eq!(speed_label(9), "?");
    }

    #[test]
    fn direction_pad_covers_every_direction_once() {
        let mut seen: Vec<Direction> = DIRECTION_PAD.iter().flatten().copied().collect();
        seen.sort_by_key(|d| d.code());
        assert_eq!(seen, Direction::ALL);
        for direction in Direction::ALL {
            assert!(!direction_glyph(direction).is_empty());
        }
        assert_eq!(direction_glyph(Direction::Up), "↑");
    }
}
