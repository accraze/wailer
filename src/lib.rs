//! A sound-system dub siren, after the classic Jamaican dual-tone siren boxes.
//!
//! Signal chain:
//! 1. Square oscillator whose pitch is swung between Tone and Tone + Interval by the wobble LFO
//!    (square = two-tone, triangle = wail, rise/fall = sweeps).
//! 2. Howl: the clipped output is band-passed and fed back into the drive stage's input, so the
//!    loop squeals on its own as Howl goes up.
//! 3. Drive stage borrowed from Rodent: LM308 RAT gain stage (bandwidth + slew limited, rail
//!    clipping) into a 1N914 diode clipper, then the RAT's passive filter.
//! 4. Gate envelope and volume. MIDI notes or the Trigger button open the gate.
//!
//! Oscillator and drive stage run at 4x oversampling to keep aliasing down.

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::f32::consts::PI;
use std::sync::Arc;

mod editor;

const OVERSAMPLING: usize = 4;

/// Oscillator level into the drive stage, about a hot guitar pickup
const OSC_LEVEL: f32 = 0.3;
/// Loop gain at full Howl; above 1 so the loop can self-oscillate
const HOWL_MAX: f32 = 1.5;
const HOWL_LO: f32 = 300.0;
const HOWL_HI: f32 = 3000.0;

// RAT circuit values, see Rodent
const R_DIST_MAX: f32 = 100e3;
const R_LEG_HI: f32 = 47.0;
const C_LEG_HI: f32 = 2.2e-6;
const R_LEG_LO: f32 = 560.0;
const C_LEG_LO: f32 = 4.7e-6;
const C_FEEDBACK: f32 = 100e-12;
const OPAMP_GBW: f32 = 1e6;
const OPAMP_SLEW: f32 = 0.3e6;
const RAIL: f32 = 4.5;
const DIODE_V: f32 = 0.6;
const R_FILTER_MIN: f32 = 1.5e3;
const R_FILTER_POT: f32 = 100e3;
const C_FILTER: f32 = 3.3e-9;

/// Gate fade in/out times in seconds: short enough to feel instant, long enough not to click
const ATTACK: f32 = 0.003;
const RELEASE: f32 = 0.03;

/// Wobble LFO shape, i.e. how the siren moves between its two tones.
#[derive(Enum, PartialEq, Clone, Copy, Debug)]
enum Shape {
    /// Classic two-tone hi-lo
    #[name = "Two-tone"]
    Square,
    /// Smooth wail up and down
    #[name = "Wail"]
    Triangle,
    /// Sweep up, then snap back
    Rise,
    /// Snap up, then sweep down
    Fall,
}

impl Shape {
    /// Maps LFO phase in [0, 1) to a position between tone A (0) and tone B (1).
    fn at(self, phase: f32) -> f32 {
        match self {
            Shape::Square => (phase >= 0.5) as u8 as f32,
            Shape::Triangle => 1.0 - (2.0 * phase - 1.0).abs(),
            Shape::Rise => phase,
            Shape::Fall => 1.0 - phase,
        }
    }
}

/// Topology-preserving one-pole filter, gives both low-pass and high-pass outputs.
#[derive(Default, Clone, Copy)]
struct OnePole {
    s: f32,
}

impl OnePole {
    /// `g` is `tan(pi * fc / fs)`. Returns the low-pass output.
    fn lp(&mut self, x: f32, g: f32) -> f32 {
        let v = (x - self.s) * g / (1.0 + g);
        let y = v + self.s;
        self.s = y + v;
        y
    }

    fn hp(&mut self, x: f32, g: f32) -> f32 {
        x - self.lp(x, g)
    }
}

fn prewarp(fc: f32, fs: f32) -> f32 {
    (PI * fc.min(fs * 0.45) / fs).tan()
}

#[derive(Default, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn lowpass(fc: f32, fs: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * fc / fs;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos) / 2.0 / a0,
            b1: (1.0 - cos) / a0,
            b2: (1.0 - cos) / 2.0 / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// 8th order Butterworth low-pass used as the decimation filter.
#[derive(Default, Clone, Copy)]
struct AntiAlias([Biquad; 4]);

impl AntiAlias {
    fn new(fs_base: f32) -> Self {
        let fs = fs_base * OVERSAMPLING as f32;
        let fc = (fs_base * 0.45).min(20e3);
        let qs = [0.5098, 0.6013, 0.9000, 2.5629];
        Self(qs.map(|q| Biquad::lowpass(fc, fs, q)))
    }

    fn process(&mut self, x: f32) -> f32 {
        self.0.iter_mut().fold(x, |x, bq| bq.process(x))
    }
}

/// PolyBLEP residual for a step at phase 0, `dt` is the phase increment.
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        2.0 * t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

/// Plain knob values, so tests can drive the DSP without a plugin instance.
#[derive(Clone, Copy)]
struct Settings {
    tone: f32,
    interval: f32,
    rate: f32,
    depth: f32,
    shape: Shape,
    howl: f32,
    drive: f32,
    filter: f32,
}

/// Per-sample coefficients derived from `Settings`.
struct Coeffs {
    tone: f32,
    interval: f32,
    lfo_step: f32,
    depth: f32,
    shape: Shape,
    howl: f32,
    g_howl_lo: f32,
    g_howl_hi: f32,
    r_dist: f32,
    g_leg_hi: f32,
    g_leg_lo: f32,
    g_opamp: f32,
    max_step: f32,
    g_tone: f32,
    fs_os: f32,
}

impl Coeffs {
    fn new(s: &Settings, fs: f32) -> Self {
        let fs_os = fs * OVERSAMPLING as f32;
        // Audio taper (log) pot for drive, linear pot for the filter like the RAT
        let r_dist = R_DIST_MAX * (10f32.powf(2.0 * s.drive) - 1.0) / 99.0;
        let r_ground = 1.0 / (1.0 / R_LEG_HI + 1.0 / R_LEG_LO);
        let noise_gain = 1.0 + r_dist / r_ground;
        let opamp_fc =
            (1.0 / (2.0 * PI * r_dist.max(1.0) * C_FEEDBACK)).min(OPAMP_GBW / noise_gain);
        let tone_fc = 1.0 / (2.0 * PI * (R_FILTER_MIN + R_FILTER_POT * s.filter) * C_FILTER);

        Self {
            tone: s.tone,
            interval: s.interval,
            lfo_step: s.rate / fs,
            depth: s.depth,
            shape: s.shape,
            howl: s.howl * HOWL_MAX,
            g_howl_lo: prewarp(HOWL_LO, fs_os),
            g_howl_hi: prewarp(HOWL_HI, fs_os),
            r_dist,
            g_leg_hi: prewarp(1.0 / (2.0 * PI * R_LEG_HI * C_LEG_HI), fs_os),
            g_leg_lo: prewarp(1.0 / (2.0 * PI * R_LEG_LO * C_LEG_LO), fs_os),
            g_opamp: prewarp(opamp_fc, fs_os),
            max_step: OPAMP_SLEW / fs_os,
            g_tone: prewarp(tone_fc, fs),
            fs_os,
        }
    }
}

/// The whole siren. Mono: the output is copied to every channel.
#[derive(Default, Clone, Copy)]
struct Voice {
    lfo_phase: f32,
    osc_phase: f32,
    howl_hp: OnePole,
    howl_lp: OnePole,
    feedback: f32,
    leg_hi: OnePole,
    leg_lo: OnePole,
    opamp_bw: OnePole,
    opamp_out: f32,
    down: AntiAlias,
    tone: OnePole,
}

impl Voice {
    fn new(fs: f32) -> Self {
        Self {
            down: AntiAlias::new(fs),
            ..Self::default()
        }
    }

    /// Restarts the wobble so every hit starts on tone A.
    fn retrigger(&mut self) {
        self.lfo_phase = 0.0;
    }

    fn process(&mut self, c: &Coeffs) -> f32 {
        let pos = c.shape.at(self.lfo_phase) * c.depth;
        self.lfo_phase = (self.lfo_phase + c.lfo_step).fract();
        let freq = c.tone * 2f32.powf(c.interval * pos / 12.0);
        let dt = (freq / c.fs_os).min(0.5);

        let mut out = 0.0;
        for _ in 0..OVERSAMPLING {
            let p = self.osc_phase;
            let square =
                if p < 0.5 { 1.0 } else { -1.0 } + blep(p, dt) - blep((p + 0.5).fract(), dt);
            self.osc_phase = (p + dt).fract();

            let u = OSC_LEVEL * square + c.howl * self.feedback;

            // RAT gain stage: current into the two ground legs times the feedback resistor
            let i_ground =
                self.leg_hi.hp(u, c.g_leg_hi) / R_LEG_HI + self.leg_lo.hp(u, c.g_leg_lo) / R_LEG_LO;
            let gained = u + self.opamp_bw.lp(c.r_dist * i_ground, c.g_opamp);
            let target = RAIL * (gained / RAIL).tanh();
            self.opamp_out += (target - self.opamp_out).clamp(-c.max_step, c.max_step);
            let clipped = DIODE_V * (self.opamp_out / DIODE_V).tanh();

            // Band-passed so the howl rings in the squeal range instead of latching to DC
            self.feedback = self
                .howl_lp
                .lp(self.howl_hp.hp(clipped, c.g_howl_lo), c.g_howl_hi);
            out = self.down.process(clipped);
        }

        self.tone.lp(out, c.g_tone)
    }
}

struct Wailer {
    params: Arc<WailerParams>,
    sample_rate: f32,
    voice: Voice,
    /// MIDI notes currently held
    notes: u8,
    gate: bool,
    env: f32,
}

#[derive(Params)]
struct WailerParams {
    // Renamed from "editor-state" so projects saved with the 0.1.0 slider UI open at the new size
    #[persist = "editor-size"]
    editor_state: Arc<EguiState>,

    #[id = "trigger"]
    trigger: BoolParam,
    #[id = "tone"]
    tone: FloatParam,
    #[id = "interval"]
    interval: FloatParam,
    #[id = "rate"]
    rate: FloatParam,
    #[id = "depth"]
    depth: FloatParam,
    #[id = "shape"]
    shape: EnumParam<Shape>,
    #[id = "howl"]
    howl: FloatParam,
    #[id = "drive"]
    drive: FloatParam,
    #[id = "filter"]
    filter: FloatParam,
    #[id = "volume"]
    volume: FloatParam,
}

fn knob(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_smoother(SmoothingStyle::Linear(20.0))
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

fn hz(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-2.0),
        },
    )
    .with_smoother(SmoothingStyle::Logarithmic(20.0))
    .with_unit(" Hz")
    .with_value_to_string(formatters::v2s_f32_rounded(2))
}

impl Default for WailerParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(editor::WIDTH, editor::HEIGHT),
            trigger: BoolParam::new("Trigger", false),
            tone: hz("Tone", 700.0, 100.0, 3000.0),
            interval: FloatParam::new(
                "Interval",
                5.0,
                FloatRange::Linear {
                    min: -24.0,
                    max: 24.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit(" st")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            rate: hz("Wobble Rate", 3.0, 0.1, 30.0),
            depth: knob("Wobble Depth", 1.0),
            shape: EnumParam::new("Wobble Shape", Shape::Square),
            howl: knob("Howl", 0.0),
            drive: knob("Drive", 0.3),
            filter: knob("Filter", 0.3),
            volume: FloatParam::new(
                "Volume",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 12.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Default for Wailer {
    fn default() -> Self {
        Self {
            params: Arc::new(WailerParams::default()),
            sample_rate: 48000.0,
            voice: Voice::default(),
            notes: 0,
            gate: false,
            env: 0.0,
        }
    }
}

impl Plugin for Wailer {
    const NAME: &'static str = "Wailer";
    const VENDOR: &'static str = "accraze";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.reset();
        true
    }

    fn reset(&mut self) {
        self.voice = Voice::new(self.sample_rate);
        self.notes = 0;
        self.gate = false;
        self.env = 0.0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let attack = 1.0 / (ATTACK * self.sample_rate);
        let release = 1.0 / (RELEASE * self.sample_rate);
        let p = &self.params;
        let mut next_event = context.next_event();

        for (i, mut frame) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > i as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { .. } => self.notes = self.notes.saturating_add(1),
                    NoteEvent::NoteOff { .. } => self.notes = self.notes.saturating_sub(1),
                    _ => {}
                }
                next_event = context.next_event();
            }

            let gate = self.notes > 0 || p.trigger.value();
            if gate && !self.gate {
                self.voice.retrigger();
            }
            self.gate = gate;

            let settings = Settings {
                tone: p.tone.smoothed.next(),
                interval: p.interval.smoothed.next(),
                rate: p.rate.smoothed.next(),
                depth: p.depth.smoothed.next(),
                shape: p.shape.value(),
                howl: p.howl.smoothed.next(),
                drive: p.drive.smoothed.next(),
                filter: p.filter.smoothed.next(),
            };
            let volume = p.volume.smoothed.next();
            self.env = if gate {
                (self.env + attack).min(1.0)
            } else {
                (self.env - release).max(0.0)
            };

            // Keep running while silent so the howl loop and filters stay warm
            let y = self
                .voice
                .process(&Coeffs::new(&settings, self.sample_rate))
                * volume
                * self.env;
            for sample in frame.iter_mut() {
                *sample = y;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Wailer {
    const CLAP_ID: &'static str = "com.accraze.wailer";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Sound-system dub siren");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for Wailer {
    const VST3_CLASS_ID: [u8; 16] = *b"WailerDubSiren01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(Wailer);
nih_export_vst3!(Wailer);

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f32 = 48000.0;

    fn settings() -> Settings {
        Settings {
            tone: 700.0,
            interval: 5.0,
            rate: 3.0,
            depth: 1.0,
            shape: Shape::Square,
            howl: 0.0,
            drive: 0.3,
            filter: 0.3,
        }
    }

    /// Runs the voice for `secs` seconds and returns the output.
    fn render(s: &Settings, fs: f32, secs: f32) -> Vec<f32> {
        let c = Coeffs::new(s, fs);
        let mut v = Voice::new(fs);
        (0..(fs * secs) as usize).map(|_| v.process(&c)).collect()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0, |m, y| m.max(y.abs()))
    }

    /// Frequency estimate from rising zero crossings.
    fn freq(x: &[f32], fs: f32) -> f32 {
        let n = x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        n as f32 * fs / x.len() as f32
    }

    #[test]
    fn output_is_bounded_and_not_silent() {
        for fs in [44100.0, 48000.0, 96000.0, 192000.0] {
            for shape in [Shape::Square, Shape::Triangle, Shape::Rise, Shape::Fall] {
                for (howl, drive, tone) in
                    [(0.0, 0.0, 100.0), (0.5, 0.3, 700.0), (1.0, 1.0, 3000.0)]
                {
                    let s = Settings {
                        shape,
                        howl,
                        drive,
                        tone,
                        interval: 24.0,
                        rate: 30.0,
                        ..settings()
                    };
                    let out = render(&s, fs, 0.5);
                    assert!(out.iter().all(|y| y.is_finite()));
                    let peak = peak(&out);
                    // Decimation filter ringing lets peaks overshoot the diode clip a bit
                    assert!(
                        peak > 0.05 && peak < 1.5 * DIODE_V,
                        "fs {fs} {shape:?} howl {howl} drive {drive} tone {tone}: peak {peak}"
                    );
                }
            }
        }
    }

    #[test]
    fn two_tone_alternates_between_tone_and_interval() {
        // 1 Hz square wobble: first half second on tone A, second half an octave up
        let s = Settings {
            tone: 500.0,
            interval: 12.0,
            rate: 1.0,
            drive: 0.0,
            filter: 0.0,
            ..settings()
        };
        let out = render(&s, FS, 1.0);
        let half = out.len() / 2;
        let (a, b) = (freq(&out[..half], FS), freq(&out[half..], FS));
        assert!((a - 500.0).abs() < 10.0, "tone A {a}");
        assert!((b - 1000.0).abs() < 10.0, "tone B {b}");
    }

    #[test]
    fn zero_depth_holds_tone_a() {
        let s = Settings {
            tone: 500.0,
            depth: 0.0,
            drive: 0.0,
            filter: 0.0,
            ..settings()
        };
        let f = freq(&render(&s, FS, 1.0), FS);
        assert!((f - 500.0).abs() < 5.0, "{f}");
    }

    #[test]
    fn howl_changes_the_sound() {
        let base = render(&settings(), FS, 0.5);
        let howl = render(
            &Settings {
                howl: 1.0,
                ..settings()
            },
            FS,
            0.5,
        );
        let diff = base
            .iter()
            .zip(&howl)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / base.len() as f32;
        assert!(diff > 0.05, "mean difference {diff}");
        // Must not latch to DC
        let mean = howl.iter().sum::<f32>() / howl.len() as f32;
        assert!(mean.abs() < 0.05, "DC {mean}");
    }
}
