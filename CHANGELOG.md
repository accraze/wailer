# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Drop knob (0–36 semitones): the pitch glides down over the Release time after you let go, for the
  classic "pew" ending. Off by default; the Laser and Dread Drop presets now use it.

### Changed
- Volume now comes after the echo, so it no longer changes how hard the echo saturates. Volume moved to
  the bottom row to make room for Drop.

## [0.5.0] - 2026-10-23

### Added
- Factory presets with a ‹ › selector: Classic, Sound System, Police Wail, Laser, Rising Howl, Dread Drop
  and Runaway. Level-matched to within 1 dB of Classic; shows CUSTOM once you tweak a knob.

## [0.4.0] - 2026-10-23

### Added
- Tape echo (Time, Feedback, Wow, Mix). Repeats darken and saturate, run away past 100% feedback, and
  bend in pitch while you turn Time. Mix defaults to 0, so existing projects sound the same.

## [0.3.0] - 2026-10-23

### Added
- Release knob (0.01–5 s) for a decaying tail after you let go of the trigger. Defaults to the old
  fixed 30 ms, so existing projects sound the same.

## [0.2.0] - 2026-10-23

### Changed
- Hand-painted siren-box editor: knobs, a Two-tone/Wail/Rise/Fall switch and a big red momentary trigger
  button. Resizable (drag the bottom-right corner). Fixes the bottom controls being cut off.

## [0.1.0] - 2026-10-23

### Added
- Dual-tone siren: square oscillator wobbling between Tone and Tone + Interval, with Two-tone,
  Wail, Rise and Fall wobble shapes.
- Howl feedback and Rodent's RAT drive stage (Drive, Filter), 4x oversampled.
- MIDI notes and a momentary Trigger pad gate the siren.

[Unreleased]: https://github.com/accraze/wailer/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/accraze/wailer/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/accraze/wailer/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/accraze/wailer/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/accraze/wailer/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/accraze/wailer/releases/tag/v0.1.0
