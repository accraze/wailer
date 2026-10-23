//! Placeholder editor: nih-plug's generic sliders plus a momentary trigger pad for live playing.

use crate::WailerParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Button, RichText};
use nih_plug_egui::{create_egui_editor, widgets::ParamSlider};
use std::sync::Arc;

pub const WIDTH: u32 = 320;
pub const HEIGHT: u32 = 420;

pub fn create(params: Arc<WailerParams>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_, _| {},
        move |ctx, setter, _| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("WAILER");

                // Sounds only while held down, like the button on a siren box
                let pad = ui.add_sized(
                    [ui.available_width(), 80.0],
                    Button::new(RichText::new("TRIGGER").size(24.0))
                        .selected(params.trigger.value()),
                );
                let down = pad.is_pointer_button_down_on();
                if down != params.trigger.value() {
                    setter.begin_set_parameter(&params.trigger);
                    setter.set_parameter(&params.trigger, down);
                    setter.end_set_parameter(&params.trigger);
                }
                ui.add_space(8.0);

                macro_rules! row {
                    ($($param:ident),*) => {$(
                        ui.label(params.$param.name());
                        ui.add(ParamSlider::for_param(&params.$param, setter));
                    )*};
                }
                row!(tone, interval, rate, depth, shape, howl, drive, filter, volume);
            });
        },
    )
}
