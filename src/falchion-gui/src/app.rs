//! egui rendering of [`AppState`]. Keep this file free of logic: it is
//! excluded from coverage because it needs a display. Edits go through
//! `AppState` methods; device access goes through [`Worker`] only. Never call
//! a provider from here.

use eframe::egui::{self, Color32, RichText, Ui};
use falchion_core::{Brightness, DeviceProvider, Effect, Rgb, Speed, Thickness};

use crate::state::{
    brightness_label, direction_glyph, speed_label, thickness_label, AppState, Status,
    DIRECTION_PAD,
};
use crate::worker::{Request, Worker};

const EFFECT_COLUMNS: usize = 5;
const EFFECT_BUTTON: [f32; 2] = [104.0, 30.0];
const PAD_BUTTON: [f32; 2] = [34.0, 30.0];

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

    fn header_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("ROG Falchion");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let idle = !self.state.busy;
                let refresh = ui.add_enabled(idle, egui::Button::new("⟳ Refresh"));
                if refresh.on_hover_text("Look for keyboards again").clicked() {
                    self.send(Request::Refresh);
                }
            });
        });
        let connected = !self.state.devices.is_empty();
        let (dot, color) = if connected {
            ("●", Color32::from_rgb(60, 180, 90))
        } else {
            ("●", Color32::GRAY)
        };
        ui.horizontal(|ui| {
            ui.colored_label(color, dot);
            ui.label(self.state.keyboard_summary());
        });
    }

    fn effect_ui(&mut self, ui: &mut Ui) {
        section(ui, "Effect");
        let current = self.state.settings.effect;
        egui::Grid::new("effects")
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for (i, effect) in Effect::ALL.into_iter().enumerate() {
                    let button = egui::SelectableLabel::new(effect == current, effect.label());
                    let clicked = ui.add_sized(EFFECT_BUTTON, button);
                    if clicked.on_hover_text(effect.description()).clicked() {
                        self.state.select_effect(effect);
                    }
                    if (i + 1) % EFFECT_COLUMNS == 0 {
                        ui.end_row();
                    }
                }
            });
        let effect = self.state.settings.effect;
        ui.label(RichText::new(effect.description()).weak());
        if effect.reacts_to_keys() {
            ui.label(
                RichText::new("Press keys to see this effect.")
                    .weak()
                    .italics(),
            );
        }
    }

    fn colors_ui(&mut self, ui: &mut Ui) {
        let effect = self.state.settings.effect;
        let modes = effect.color_modes();
        if modes.is_empty() && !effect.uses_background() {
            return;
        }
        section(ui, "Colors");
        if modes.len() > 1 {
            ui.horizontal(|ui| {
                for &mode in modes {
                    let selected = self.state.settings.color_mode == mode;
                    if ui.selectable_label(selected, mode.label()).clicked() {
                        self.state.set_color_mode(mode);
                    }
                }
            });
        }
        let labels = self.state.color_labels();
        if !labels.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (i, label) in labels.iter().enumerate() {
                    let mut rgb = self.state.settings.colors[i].to_bytes();
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).small());
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            self.state.set_color(i, Rgb::from(rgb));
                        }
                    });
                }
                if self.state.can_add_color() || self.state.can_remove_color() {
                    ui.vertical(|ui| {
                        let add =
                            ui.add_enabled(self.state.can_add_color(), egui::Button::new("+"));
                        if add.on_hover_text("Add a color").clicked() {
                            self.state.add_color();
                        }
                        let remove =
                            ui.add_enabled(self.state.can_remove_color(), egui::Button::new("−"));
                        if remove.on_hover_text("Remove the last color").clicked() {
                            self.state.remove_color();
                        }
                    });
                }
            });
        }
        if effect.uses_background() {
            let background = self.state.settings.background;
            ui.horizontal(|ui| {
                let mut enabled = background.is_some();
                if ui.checkbox(&mut enabled, "Background color").changed() {
                    let color = enabled.then_some(background.unwrap_or(Rgb::new(0, 0, 0x60)));
                    self.state.set_background(color);
                }
                if let Some(color) = background {
                    let mut rgb = color.to_bytes();
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        self.state.set_background(Some(Rgb::from(rgb)));
                    }
                } else {
                    ui.label(RichText::new("(unlit keys stay dark)").weak());
                }
            });
        }
    }

    fn motion_ui(&mut self, ui: &mut Ui) {
        let effect = self.state.settings.effect;
        if !(effect.uses_speed() || effect.uses_direction() || effect.uses_thickness()) {
            return;
        }
        section(ui, "Motion");
        egui::Grid::new("motion")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                if effect.uses_speed() {
                    ui.label("Speed");
                    let mut speed = self.state.settings.speed.value();
                    let slider = egui::Slider::new(&mut speed, 0..=Speed::MAX)
                        .custom_formatter(|v, _| speed_label(v as u8).to_string());
                    if ui.add(slider).changed() {
                        self.state.set_speed(speed);
                    }
                    ui.end_row();
                }
                if effect == Effect::Wave {
                    ui.label("Direction");
                    self.direction_pad_ui(ui);
                    ui.end_row();
                } else if effect.uses_direction() {
                    ui.label("Direction");
                    ui.horizontal(|ui| {
                        for &direction in effect.directions() {
                            let selected = self.state.settings.direction == direction;
                            let text =
                                format!("{} {}", direction_glyph(direction), direction.label());
                            if ui.selectable_label(selected, text).clicked() {
                                self.state.set_direction(direction);
                            }
                        }
                    });
                    ui.end_row();
                }
                if effect.uses_thickness() {
                    ui.label(self.state.thickness_heading());
                    ui.horizontal(|ui| {
                        for thickness in Thickness::ALL {
                            let selected = self.state.settings.thickness == thickness;
                            let text = thickness_label(effect, thickness);
                            if ui.selectable_label(selected, text).clicked() {
                                self.state.set_thickness(thickness);
                            }
                        }
                    });
                    ui.end_row();
                }
            });
    }

    fn direction_pad_ui(&mut self, ui: &mut Ui) {
        let current = self.state.settings.direction;
        ui.vertical(|ui| {
            egui::Grid::new("direction-pad")
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    for row in DIRECTION_PAD {
                        for direction in row {
                            let glyph = RichText::new(direction_glyph(direction)).size(18.0);
                            let button = egui::SelectableLabel::new(direction == current, glyph);
                            let response = ui.add_sized(PAD_BUTTON, button);
                            if response.on_hover_text(direction.label()).clicked() {
                                self.state.set_direction(direction);
                            }
                        }
                        ui.end_row();
                    }
                });
            ui.label(RichText::new(current.label()).weak());
        });
    }

    fn brightness_ui(&mut self, ui: &mut Ui) {
        section(ui, "Brightness");
        let mut brightness = self.state.settings.brightness.value();
        let slider = egui::Slider::new(&mut brightness, 0..=Brightness::MAX)
            .custom_formatter(|v, _| brightness_label(v as u8));
        if ui.add(slider).changed() {
            self.state.set_brightness(brightness);
        }
    }

    fn actions_ui(&mut self, ui: &mut Ui) {
        let idle = !self.state.busy && !self.state.devices.is_empty();
        ui.horizontal(|ui| {
            let apply = ui.add_enabled(idle, egui::Button::new("Apply"));
            if apply
                .on_hover_text("Show this on the keyboard until it powers off")
                .clicked()
            {
                let request = self.state.apply_request(false);
                self.send(request);
            }
            let save = ui.add_enabled(idle, egui::Button::new("Apply & Save"));
            if save
                .on_hover_text("Show this and store it in the keyboard's active profile")
                .clicked()
            {
                let request = self.state.apply_request(true);
                self.send(request);
            }
            ui.checkbox(&mut self.state.live_preview, "Live preview")
                .on_hover_text("Send every change to the keyboard straight away (not saved)");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let reset = ui.button("Factory default");
                if reset
                    .on_hover_text("Rainbow wave at 50% brightness (click Apply to send)")
                    .clicked()
                {
                    self.state.reset_to_factory();
                }
            });
        });
        if self.state.pending && !self.state.live_preview {
            ui.label(RichText::new("Changes not sent yet: click Apply.").weak());
        }
    }

    fn status_ui(&self, ui: &mut Ui) {
        match &self.state.status {
            Status::Idle => {
                ui.label("");
            }
            Status::Info(msg) => {
                ui.label(msg.as_str());
            }
            Status::Error(msg) => {
                ui.colored_label(Color32::from_rgb(220, 60, 60), msg.as_str());
            }
        }
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).strong().size(15.0));
    ui.add_space(2.0);
}

impl eframe::App for FalchionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(response) = self.worker.try_recv() {
            self.state.on_response(response);
        }
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(4.0);
            self.header_ui(ui);
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("actions").show(ctx, |ui| {
            ui.add_space(6.0);
            self.actions_ui(ui);
            ui.separator();
            self.status_ui(ui);
            ui.add_space(2.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.effect_ui(ui);
                self.colors_ui(ui);
                self.motion_ui(ui);
                self.brightness_ui(ui);
            });
        });
        if let Some(request) = self.state.take_live_request() {
            self.send(request);
        }
    }
}
