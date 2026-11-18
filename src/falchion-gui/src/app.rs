//! egui rendering of [`AppState`]. Keep this file free of logic: it is
//! excluded from coverage because it needs a display. Device access goes
//! through [`Worker`] only; never call a provider from here.

use eframe::egui;
use falchion_core::{Brightness, DeviceProvider, Direction, Effect, Rgb, Speed};

use crate::state::{AppState, Status};
use crate::worker::{Request, Worker};

/// The eframe application.
pub struct FalchionApp {
    state: AppState,
    worker: Worker,
}

impl FalchionApp {
    /// Starts the device worker and requests an initial scan.
    pub fn new<P, F>(ctx: egui::Context, make_provider: F) -> Self
    where
        P: DeviceProvider,
        F: FnOnce() -> P + Send + 'static,
    {
        let worker = Worker::spawn(make_provider, move || ctx.request_repaint());
        let mut app = Self {
            state: AppState::default(),
            worker,
        };
        app.send(Request::Refresh);
        app
    }

    fn send(&mut self, request: Request) {
        let accepted = self.worker.send(request);
        self.state.on_sent(accepted);
    }

    fn devices_ui(&mut self, ui: &mut egui::Ui) {
        let idle = !self.state.busy;
        ui.horizontal(|ui| {
            ui.heading("Keyboards");
            if ui.add_enabled(idle, egui::Button::new("Refresh")).clicked() {
                self.send(Request::Refresh);
            }
        });
        if self.state.devices.is_empty() {
            ui.label("None connected.");
        }
        for device in &self.state.devices {
            ui.label(device.to_string());
        }
    }

    fn lighting_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Lighting");

        let effect = &mut self.state.settings.effect;
        egui::ComboBox::from_label("Effect")
            .selected_text(effect.name())
            .show_ui(ui, |ui| {
                for option in Effect::ALL {
                    ui.selectable_value(effect, option, option.name());
                }
            });
        let effect = *effect;

        if effect.uses_color() {
            let mut color = self.state.settings.color.to_bytes();
            ui.horizontal(|ui| {
                ui.label("Color");
                if ui.color_edit_button_srgb(&mut color).changed() {
                    self.state.settings.color = Rgb::from(color);
                }
            });
        }

        let mut brightness = self.state.settings.brightness.value();
        let slider = egui::Slider::new(&mut brightness, 0..=Brightness::MAX);
        if ui.add(slider.text("Brightness")).changed() {
            self.state.set_brightness(brightness);
        }

        if effect.uses_speed() {
            let mut speed = self.state.settings.speed.value();
            let slider = egui::Slider::new(&mut speed, 0..=Speed::MAX);
            if ui.add(slider.text("Speed")).changed() {
                self.state.set_speed(speed);
            }
        }

        if effect.uses_direction() {
            let direction = &mut self.state.settings.direction;
            ui.horizontal(|ui| {
                ui.label("Direction");
                for option in Direction::ALL {
                    ui.radio_value(direction, option, option.name());
                }
            });
        }

        let idle = !self.state.busy;
        let settings = self.state.settings;
        ui.horizontal(|ui| {
            if ui.add_enabled(idle, egui::Button::new("Apply")).clicked() {
                self.send(Request::Apply {
                    settings,
                    save: false,
                });
            }
            let save_button = egui::Button::new("Apply & Save");
            if ui.add_enabled(idle, save_button).clicked() {
                self.send(Request::Apply {
                    settings,
                    save: true,
                });
            }
        });
    }

    fn status_ui(&self, ui: &mut egui::Ui) {
        match &self.state.status {
            Status::Idle => {}
            Status::Info(msg) => {
                ui.label(msg.as_str());
            }
            Status::Error(msg) => {
                ui.colored_label(egui::Color32::RED, msg.as_str());
            }
        }
    }
}

impl eframe::App for FalchionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(response) = self.worker.try_recv() {
            self.state.on_response(response);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.devices_ui(ui);
            ui.separator();
            self.lighting_ui(ui);
            ui.separator();
            self.status_ui(ui);
        });
    }
}
