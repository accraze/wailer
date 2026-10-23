# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/accraze/wailer/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/accraze/wailer/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/accraze/wailer/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/accraze/wailer/releases/tag/v0.1.0
