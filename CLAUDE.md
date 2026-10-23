# Wailer

Sound-system dub siren instrument plugin (CLAP/VST3) in Rust using nih-plug. Sibling of `../rodent`
(same layout, toolchain, CI and release process). Single crate, all DSP in `src/lib.rs`.
The user runs it in Bitwig Studio on Linux (CLAP).

## Commands

```sh
cargo xtask bundle wailer --release && cp -r target/bundled/Wailer.clap ~/.clap/   # build + install
cargo test --release --lib -- --nocapture                                          # DSP tests
cargo clippy --release --lib
```

- The system has an old distro Rust (1.75) in `/usr/bin` that comes before `~/.cargo/bin` on PATH and
  can't build nih-plug. Prefix commands with `PATH="$HOME/.cargo/bin:$PATH"`.
- Bitwig needs the plugin re-added or a restart to pick up a new build.
- Add user-facing changes to `CHANGELOG.md` under `[Unreleased]` as you make them.
- Releases: follow `docs/RELEASING.md` (CI publishes on annotated `v*` tags; never move an existing tag).
- The "trailing semicolon in macro" warnings come from nih-plug's export macros, not our code.

## DSP structure

- `Settings` holds plain knob values; `Coeffs::new(&settings, fs)` maps them to coefficients. Both
  `process()` and the tests go through it, so change mappings there only.
- `Voice::process` runs the LFO at the base rate, then oscillator (PolyBLEP square), Howl feedback and the
  RAT stage at `OVERSAMPLING`x, then the tone filter at the base rate. Output is mono, copied to all channels.
- The RAT stage is copied from Rodent (fixed 1N914 diodes); keep circuit constants tied to real values.
- Howl feeds the clipped signal back through a 300 Hz–3 kHz band-pass. The high-pass part is what keeps the
  loop from latching to DC (`howl_changes_the_sound` checks this).
- Gate = held MIDI notes or the `trigger` param. A rising edge restarts the LFO. `env_step` is a linear 3 ms
  attack and an exponential release (`release` param = time to -60 dB), snapping to 0 below -100 dB. The voice keeps running while
  the gate is closed so the release tail and filter states stay clean.
- `Echo` runs after the gate envelope and keeps running when the gate is closed, so repeats ring out. Its buffer
  is allocated in `initialize()` (never in `process()`/`reset()`, which only clears it). Echo Time has a slow
  300 ms smoother on purpose: the glide is the tape pitch-bend. The `tanh` on the write keeps >100% feedback bounded.
- GUI lives in `src/editor.rs`: a hand-painted siren box (egui `Painter`) whose `knob()`, `screw()` and window
  scaling are copied from Rodent's pedal editor (see `../rodent/CLAUDE.md` for how scaling works). The trigger
  button is momentary: it holds the `trigger` param on only while the mouse is down.
- README screenshot: build a standalone copy in the scratchpad (`standalone` feature, `rlib` crate type, a
  `main.rs` calling `nih_export_standalone`), run it with `--backend dummy`, and grab the window with `import`. Match its title exactly (`"Wailer": ()` in `xwininfo -root -tree`):
  an open Bitwig instance has a "Wailer / Wailer" window that a loose grep picks up instead.
- The audio thread must not allocate (`assert_process_allocs` is enabled).
- `CLAP_ID` and `VST3_CLASS_ID` must never change, or saved Bitwig projects lose the plugin.
