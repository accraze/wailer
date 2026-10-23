//! Siren-box editor painted by hand: two rows of knobs, a wobble shape switch and a big red
//! momentary trigger button. Knob, screw and window scaling come from Rodent's pedal editor.

use crate::{Preset, Shape, WailerParams, PRESETS};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{
    self, emath::TSTransform, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2,
};
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

pub const WIDTH: u32 = 608;
pub const HEIGHT: u32 = 486;
const DESIGN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(WIDTH as f32, HEIGHT as f32));

const KNOB_RADIUS: f32 = 26.0;
/// Knobs turn +-135 degrees from straight up
const SWEEP: f32 = 0.75 * PI;
const WHITE: Color32 = Color32::from_rgb(236, 234, 228);
const ENCLOSURE: Color32 = Color32::from_rgb(26, 32, 27);
const RED: Color32 = Color32::from_rgb(200, 30, 25);
const GOLD: Color32 = Color32::from_rgb(240, 190, 30);
const GREEN: Color32 = Color32::from_rgb(20, 140, 60);

pub fn create(params: Arc<WailerParams>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_, _| {},
        move |ctx, setter, _| {
            ResizableWindow::new("resize")
                .min_size(DESIGN.size() / 2.0)
                .show(ctx, &params.editor_state, |ui| {
                    let window = ui.max_rect();
                    ui.painter()
                        .rect_filled(window, 0.0, Color32::from_gray(48));

                    // Draw at the design size on its own layer, then scale that layer (input
                    // included) to fit the window. The window corner stays in the margin so the
                    // resize handle underneath remains visible and grabbable.
                    let scale =
                        (window.width() / DESIGN.width()).min(window.height() / DESIGN.height());
                    let offset = window.center().to_vec2() - DESIGN.center().to_vec2() * scale;
                    let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("box"));
                    ctx.set_transform_layer(layer, TSTransform::new(offset, scale));
                    ui.scope_builder(UiBuilder::new().layer_id(layer).max_rect(DESIGN), |ui| {
                        ui.set_clip_rect(DESIGN);
                        siren_box(ui, setter, &params);
                    });
                });
        },
    )
}

fn siren_box(ui: &Ui, setter: &ParamSetter, params: &WailerParams) {
    let body = ui.max_rect().shrink(12.0);
    let p = ui.painter();
    p.rect_filled(body, 14.0, ENCLOSURE);
    p.rect_stroke(
        body,
        14.0,
        Stroke::new(2.0_f32, Color32::from_gray(64)),
        StrokeKind::Inside,
    );
    for corner in [
        body.left_top(),
        body.right_top(),
        body.left_bottom(),
        body.right_bottom(),
    ] {
        screw(p, corner + (body.center() - corner).normalized() * 22.0);
    }

    let title = Pos2::new(body.left() + 40.0, body.top() + 38.0);
    // The default font has no bold weight, so fake one by overdrawing
    for offset in [
        Vec2::ZERO,
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::splat(1.0),
    ] {
        p.text(
            title + offset,
            Align2::LEFT_CENTER,
            "WAILER",
            FontId::proportional(40.0),
            WHITE,
        );
    }
    p.text(
        Pos2::new(body.right() - 40.0, title.y + 8.0),
        Align2::RIGHT_CENTER,
        "D U B   S I R E N",
        FontId::proportional(12.0),
        WHITE,
    );
    for (i, color) in [RED, GOLD, GREEN].into_iter().enumerate() {
        let y = body.top() + 64.0 + 3.0 * i as f32;
        p.line_segment(
            [
                Pos2::new(body.left() + 24.0, y),
                Pos2::new(body.right() - 24.0, y),
            ],
            Stroke::new(3.0_f32, color),
        );
    }

    // Five columns on the left, the trigger alone in the wider right column
    let col = |i: usize| body.left() + 12.0 + 88.0 * (i as f32 + 0.5);
    let right = (col(4) + 44.0 + body.right()) / 2.0;
    let (row1, row2, row3) = (body.top() + 144.0, body.top() + 270.0, body.top() + 412.0);
    let rows: [(f32, &[(&FloatParam, &str)]); 3] = [
        (
            row1,
            &[
                (&params.tone, "TONE"),
                (&params.interval, "INTERVAL"),
                (&params.rate, "RATE"),
                (&params.depth, "DEPTH"),
            ],
        ),
        (
            row2,
            &[
                (&params.howl, "HOWL"),
                (&params.drive, "DRIVE"),
                (&params.filter, "FILTER"),
                (&params.volume, "VOLUME"),
                (&params.release, "RELEASE"),
            ],
        ),
        (
            row3,
            &[
                (&params.echo_time, "TIME"),
                (&params.echo_feedback, "FEEDBACK"),
                (&params.echo_wow, "WOW"),
                (&params.echo_mix, "MIX"),
            ],
        ),
    ];
    for (y, knobs) in rows {
        for (i, &(param, label)) in knobs.iter().enumerate() {
            knob(ui, setter, param, label, Pos2::new(col(i), y));
        }
    }

    // Divider with a caption setting the echo row apart from the siren
    let divider = Pos2::new(body.left() + 24.0, (row2 + row3) / 2.0 - 4.0);
    let caption = p.text(
        divider,
        Align2::LEFT_CENTER,
        "T A P E   E C H O",
        FontId::proportional(11.0),
        Color32::from_gray(170),
    );
    p.line_segment(
        [
            Pos2::new(caption.right() + 10.0, divider.y),
            Pos2::new(body.right() - 24.0, divider.y),
        ],
        Stroke::new(1.0_f32, Color32::from_gray(80)),
    );

    shape_switch(ui, setter, &params.shape, Pos2::new(col(4), row1));
    preset_selector(ui, setter, params, Pos2::new((col(4) + right) / 2.0, row3));
    trigger(ui, setter, &params.trigger, Pos2::new(right, row2));
}

/// Unit vector for a knob angle, measured clockwise from straight up.
fn dir(angle: f32) -> Vec2 {
    Vec2::new(angle.sin(), -angle.cos())
}

fn knob(ui: &Ui, setter: &ParamSetter, param: &FloatParam, label: &str, center: Pos2) {
    let rect = Rect::from_center_size(center, Vec2::splat(2.0 * KNOB_RADIUS + 12.0));
    let response = ui.interact(rect, ui.id().with(param.name()), Sense::click_and_drag());

    // Accumulate drags in egui memory since the param value can lag behind the host round trip
    if response.drag_started() {
        setter.begin_set_parameter(param);
        ui.data_mut(|d| d.insert_temp(response.id, param.unmodulated_normalized_value()));
    }
    if response.dragged() {
        let speed = if ui.input(|i| i.modifiers.shift) {
            0.0005
        } else {
            0.005
        };
        let delta = response.drag_delta();
        let start = ui
            .data(|d| d.get_temp(response.id))
            .unwrap_or(param.unmodulated_normalized_value());
        let value = (start + (delta.x - delta.y) * speed).clamp(0.0, 1.0);
        ui.data_mut(|d| d.insert_temp(response.id, value));
        setter.set_parameter_normalized(param, value);
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.default_plain_value());
        setter.end_set_parameter(param);
    }

    if response.hovered() {
        let (notches, shift) = ui.input(|i| {
            let notches: f32 = i
                .events
                .iter()
                .map(|e| match e {
                    // Smooth-scrolling touchpads report points; treat ~50 points as one notch
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta,
                        ..
                    } => delta.y / 50.0,
                    egui::Event::MouseWheel { delta, .. } => delta.y,
                    _ => 0.0,
                })
                .sum();
            (notches, i.modifiers.shift)
        });
        if notches != 0.0 {
            let step = if shift { 0.002 } else { 0.02 };
            let value = (param.unmodulated_normalized_value() + notches * step).clamp(0.0, 1.0);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, value);
            setter.end_set_parameter(param);
        }
    }

    let value = param.unmodulated_normalized_value();
    let angle = -SWEEP + 2.0 * SWEEP * value;
    let p = ui.painter();

    for i in 0..=10 {
        let a = -SWEEP + 2.0 * SWEEP * i as f32 / 10.0;
        let r = if i % 5 == 0 { 2.0 } else { 1.2 };
        p.circle_filled(center + dir(a) * (KNOB_RADIUS + 9.0), r, WHITE);
    }

    // Knurled skirt that turns with the knob
    p.circle_filled(
        center + Vec2::new(0.0, 3.0),
        KNOB_RADIUS,
        Color32::from_black_alpha(160),
    );
    p.circle_filled(center, KNOB_RADIUS, Color32::from_gray(8));
    for i in 0..40 {
        let a = angle + i as f32 * TAU / 40.0;
        p.line_segment(
            [
                center + dir(a) * (KNOB_RADIUS - 6.0),
                center + dir(a) * (KNOB_RADIUS - 0.5),
            ],
            Stroke::new(1.5_f32, Color32::from_gray(42)),
        );
    }
    p.circle_filled(center, KNOB_RADIUS - 7.0, Color32::from_gray(26));
    p.circle_stroke(
        center,
        KNOB_RADIUS - 7.0,
        Stroke::new(1.0_f32, Color32::from_gray(58)),
    );
    p.line_segment(
        [
            center + dir(angle) * 6.0,
            center + dir(angle) * (KNOB_RADIUS - 9.0),
        ],
        Stroke::new(3.0_f32, WHITE),
    );

    p.text(
        center - Vec2::new(0.0, KNOB_RADIUS + 16.0),
        Align2::CENTER_BOTTOM,
        label,
        FontId::proportional(12.0),
        WHITE,
    );
    if response.hovered() || response.dragged() {
        p.text(
            center + Vec2::new(0.0, KNOB_RADIUS + 14.0),
            Align2::CENTER_TOP,
            param.normalized_value_to_string(value, true),
            FontId::proportional(12.0),
            Color32::from_gray(170),
        );
    }
}

/// Vertical 4-way slide switch for the wobble shape.
fn shape_switch(ui: &Ui, setter: &ParamSetter, param: &EnumParam<Shape>, center: Pos2) {
    const LABELS: [&str; 4] = ["2-TONE", "WAIL", "RISE", "FALL"];
    const SPACING: f32 = 18.0;
    let slot = Rect::from_center_size(
        center - Vec2::new(22.0, 0.0),
        Vec2::new(14.0, SPACING * LABELS.len() as f32),
    );
    let hit = Rect::from_min_max(slot.left_top(), slot.right_bottom() + Vec2::new(60.0, 0.0));

    let response = ui.interact(hit.expand(4.0), ui.id().with("shape"), Sense::click());
    if let (true, Some(pos)) = (response.clicked(), response.interact_pointer_pos()) {
        let index = ((pos.y - slot.top()) / SPACING).clamp(0.0, 3.0) as usize;
        setter.begin_set_parameter(param);
        setter.set_parameter(param, Shape::from_index(index));
        setter.end_set_parameter(param);
    }

    let p = ui.painter();
    p.rect_filled(slot, 7.0, Color32::from_gray(4));
    p.rect_stroke(
        slot,
        7.0,
        Stroke::new(1.0_f32, Color32::from_gray(60)),
        StrokeKind::Outside,
    );
    let y = |i: usize| slot.top() + SPACING * (i as f32 + 0.5);
    let selected = param.value().to_index();
    for (i, label) in LABELS.iter().enumerate() {
        p.text(
            Pos2::new(slot.right() + 10.0, y(i)),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(11.0),
            if i == selected {
                WHITE
            } else {
                Color32::from_gray(130)
            },
        );
    }
    let lever = Pos2::new(slot.center().x, y(selected));
    p.circle_filled(lever, 6.0, Color32::from_gray(185));
    p.circle_filled(lever + Vec2::new(-1.5, -1.5), 2.0, Color32::from_gray(240));
    p.text(
        Pos2::new(center.x, center.y - KNOB_RADIUS - 16.0),
        Align2::CENTER_BOTTOM,
        "WOBBLE",
        FontId::proportional(12.0),
        WHITE,
    );
}

/// Big red arcade-style button. Sounds only while held, like the button on a siren box.
fn trigger(ui: &Ui, setter: &ParamSetter, param: &BoolParam, center: Pos2) {
    const RADIUS: f32 = 40.0;
    let rect = Rect::from_center_size(center, Vec2::splat(2.0 * RADIUS));
    let response = ui.interact(rect, ui.id().with("trigger"), Sense::click_and_drag());
    let down = response.is_pointer_button_down_on();
    if down != param.value() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, down);
        setter.end_set_parameter(param);
    }

    // Lit while the gate is open, also when automated
    let on = param.value();
    let p = ui.painter();
    if on {
        p.circle_filled(
            center,
            RADIUS + 10.0,
            Color32::from_rgba_unmultiplied(255, 40, 20, 40),
        );
    }
    p.circle_filled(center, RADIUS, Color32::from_gray(150));
    p.circle_stroke(center, RADIUS, Stroke::new(1.5_f32, Color32::from_gray(70)));
    p.circle_filled(center, RADIUS - 6.0, Color32::from_gray(30));
    let (cap, color) = if on {
        (RADIUS - 10.0, Color32::from_rgb(255, 60, 40))
    } else {
        (RADIUS - 8.0, RED)
    };
    p.circle_filled(center, cap, color);
    p.circle_filled(
        center + Vec2::new(-7.0, -7.0),
        6.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, 60),
    );
    p.text(
        center - Vec2::new(0.0, RADIUS + 16.0),
        Align2::CENTER_BOTTOM,
        "TRIGGER",
        FontId::proportional(12.0),
        WHITE,
    );
}

/// Index of the preset the current (unmodulated) settings match, if any.
fn matching_preset(params: &WailerParams) -> Option<usize> {
    PRESETS.iter().position(|p| {
        params.shape.unmodulated_plain_value() == p.shape
            && params.preset_values(p).iter().all(|(param, value)| {
                (param.unmodulated_normalized_value() - param.preview_normalized(*value)).abs()
                    < 1e-4
            })
    })
}

fn apply_preset(setter: &ParamSetter, params: &WailerParams, preset: &Preset) {
    for (param, value) in params.preset_values(preset) {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
    setter.begin_set_parameter(&params.shape);
    setter.set_parameter(&params.shape, preset.shape);
    setter.end_set_parameter(&params.shape);
}

/// `‹ NAME ›` selector. Shows "CUSTOM" once the knobs no longer match a preset.
fn preset_selector(ui: &Ui, setter: &ParamSetter, params: &WailerParams, center: Pos2) {
    let slot = Rect::from_center_size(center, Vec2::new(150.0, 22.0));
    let arrow_size = Vec2::new(22.0, slot.height());
    let prev_rect = Rect::from_min_size(slot.left_top(), arrow_size);
    let next_rect =
        Rect::from_min_size(slot.right_top() - Vec2::new(arrow_size.x, 0.0), arrow_size);
    let prev = ui.interact(
        prev_rect.expand(4.0),
        ui.id().with("preset-prev"),
        Sense::click(),
    );
    let next = ui.interact(
        next_rect.expand(4.0),
        ui.id().with("preset-next"),
        Sense::click(),
    );

    let current = matching_preset(params);
    // Remember the last pick so stepping from CUSTOM continues from where you were
    let memory_id = ui.id().with("preset-last");
    let last = current.or_else(|| ui.data(|d| d.get_temp::<usize>(memory_id)));
    let n = PRESETS.len();
    let step = if prev.clicked() {
        Some(last.map_or(n - 1, |i| (i + n - 1) % n))
    } else if next.clicked() {
        Some(last.map_or(0, |i| (i + 1) % n))
    } else {
        None
    };
    if let Some(i) = step {
        apply_preset(setter, params, &PRESETS[i]);
        ui.data_mut(|d| d.insert_temp(memory_id, i));
    }

    let p = ui.painter();
    p.rect_filled(slot, 11.0, Color32::from_gray(4));
    p.rect_stroke(
        slot,
        11.0,
        Stroke::new(1.0_f32, Color32::from_gray(60)),
        StrokeKind::Outside,
    );
    for (rect, response, flip) in [(prev_rect, &prev, -1.0), (next_rect, &next, 1.0)] {
        let c = rect.center();
        let color = if response.hovered() {
            WHITE
        } else {
            Color32::from_gray(150)
        };
        let triangle = vec![
            c + Vec2::new(4.0 * flip, 0.0),
            c + Vec2::new(-3.0 * flip, -5.0),
            c + Vec2::new(-3.0 * flip, 5.0),
        ];
        p.add(egui::Shape::convex_polygon(triangle, color, Stroke::NONE));
    }
    let (name, color) = match current {
        Some(i) => (PRESETS[i].name.to_uppercase(), WHITE),
        None => ("CUSTOM".to_string(), Color32::from_gray(150)),
    };
    p.text(
        slot.center(),
        Align2::CENTER_CENTER,
        name,
        FontId::proportional(12.0),
        color,
    );
    p.text(
        Pos2::new(center.x, center.y - KNOB_RADIUS - 16.0),
        Align2::CENTER_BOTTOM,
        "PRESET",
        FontId::proportional(12.0),
        WHITE,
    );
}

fn screw(p: &Painter, center: Pos2) {
    p.circle_filled(center, 5.5, Color32::from_gray(90));
    p.circle_stroke(center, 5.5, Stroke::new(1.0_f32, Color32::from_gray(40)));
    p.line_segment(
        [center - dir(0.6) * 4.0, center + dir(0.6) * 4.0],
        Stroke::new(1.5_f32, Color32::from_gray(35)),
    );
}
