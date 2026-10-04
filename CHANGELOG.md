# Changelog

All three crates (`basket-ui`, `basket-app`, `basket-build`) share one version and one tag. Apps
pin them by git tag (see the README).

## Unreleased

Docs only; no crate changes. `docs/LAUNCHER-CONTRACT.md` after the v0.4.0 tag:

- `notes` and `assets` describe what feedgen.py actually emits; the four platforms are the ones in
  use, not a whitelist.
- A template's flags must exist in every build the feed lists, from `oldest` up; a key that would
  only repeat `launch` is left out rather than `oldest` raised.
- In an extra folder, a file whose extension several fruits list goes to the first fruit in feed
  order; overlaps must be resolved before the second such fruit ships.
- `couch`: the "only listed builds" rule is launcher v0.5.1 behaviour (was an unexplained "(v0.5.1)").

## v0.4.0 (2026-10-03)

A coordinated dependency refresh. No fruit-basket type, function or module was renamed, moved or
removed, and the behaviour of every module is unchanged, but the crates' public API carries types
from `minifb`, `tiny-skia` and `toml`, and those crates moved a major version. An app that re-pins
to v0.4.0 has to move its own dependencies on them to the same versions.

### Breaking changes for apps re-pinning from v0.3.0

1. **Rust 1.89 or later** (`rust-version = "1.89"`, was `"1.82"`). Every Fruit Basket Rust repo
   declares the same minimum. What sets it: gilrs 0.11 pulls in uuid 1.27, which needs 1.89 (and
   the launcher's zip 8 needs 1.88); edition 2024 needs 1.85, as do cpal 0.18 and toml 1;
   fontdue 0.9.4, already in v0.3.0's lockfile, calls `cast_signed` (1.87), so the old 1.82 never
   actually built. The workspace uses resolver 3, so `cargo update` stays on dependency versions
   that build on 1.89; an app on an older resolver should move to 3 (the default for edition 2024
   packages).
2. **minifb 0.28 → 0.29.** These items take or return `minifb::Key` or `minifb::Window`, now
   0.29's types:
   - `basket_ui::input`: `KeyMap` (`keys`, `new`, `key_for`, `mask`, `rebind`), `UiInput`
     (`pressed`, `repeated`, `down`, and its methods), `actions`, `key_name`, `key_from_name`,
     `bindable`
   - `basket_app::icon::install`
   - `basket_app::prefs::KeyBindings::from_keymap` and `to_keymap`, through `KeyMap`

   minifb 0.29 itself breaks two things in app code: `WindowOptions` has a new `use_gpu` field,
   so a struct literal without `..WindowOptions::default()` stops compiling, and `MouseButton`
   has new `Back` and `Forward` variants, so an exhaustive `match` on it stops compiling.
   `minifb::Key` is unchanged.
3. **tiny-skia 0.11 → 0.12.** `basket_ui::Canvas::pix` is a 0.12 `Pixmap`. `Canvas::fill_path`,
   `Canvas::stroke_path` and `Canvas::draw_pixmap` take 0.12 `Path`, `Transform` and `Pixmap`.
   `canvas::round_rect_path` and `canvas::segs_to_path` return a 0.12 `Path`. In app code,
   `RadialGradient::new` now takes a start radius: pass `0.0` for the old behaviour.
4. **toml 0.8 → 1.** `basket_app::prefs::read` returns, and `prefs::take` and `prefs::write`
   take, toml 1's `toml::Table`. The settings files themselves read and write the same.
5. Unchanged, no action: **gilrs** stays at 0.11, so `gilrs::Button` in `basket_ui::input`
   (`PadMap`, `UiInput::pad_buttons`, `pad_button_name`, `pad_button_from_name`),
   `basket_app::pads::PadPoll::pressed` and `prefs::PadBindings` keep working.

These moved too, but no fruit-basket API exposes them. An app that uses them directly should
still match the versions in [docs/DEPS.md](docs/DEPS.md):

- **cpal 0.15 → 0.18.** `Device::name()` is gone: use `device.description()?.name()` (or
  `device.id()`). Errors are one `cpal::Error` with `kind()`. `build_output_stream` takes
  `StreamConfig` by value. `SampleRate` is a plain `u32` (no `.0`). Streams start paused on every
  backend, so call `play()`. `default_output_config()` can now return `I24`, `I32` and other
  integer formats where it used to give `I16`.
- **png 0.17 → 0.18.** `Decoder::new` needs a `BufRead + Seek` reader: wrap a `File` in
  `BufReader` and bytes in `Cursor`. `Reader::output_buffer_size()` returns `Option<usize>`.
- **dirs 6 → 7.** Only `preference_dir()` moved, on Windows only, from Local to Roaming AppData.
  See below.

### Changed

- cpal 0.18.2. `basket_app::audio::AudioOut` behaves as before:
  - Device names are the strings cpal 0.15's `Device::name()` gave: the WASAPI friendly name on
    Windows, the CoreAudio device name on macOS, and the ALSA PCM id (`default`,
    `hw:CARD=PCH,DEV=0`) on Linux. An `audio_device` saved by an older build still finds its
    device.
  - Every sample format a device can report is converted from the same f32 frame. cpal 0.18 ranks
    `I32` and `I24` above `I16`, so a device without `F32` can now report a format v0.3.0 refused.
  - Xrun reports, which cpal 0.17 and later send to the error callback on some backends, are not
    printed. Underruns are counted by the callback, as before.
  - Windows default-device streams now follow a change of system default device (cpal 0.18).
- minifb 0.29.0, tiny-skia 0.12.0, png 0.18.1, toml 1.1.6, gilrs 0.11.2 (unchanged).
- **dirs 7.0.0.** The only change from 6.0.0 is `preference_dir()` on Windows, which fruit-basket
  does not call. Checked against the dirs 7 changelog and a diff of the 6.0.0 and 7.0.0 sources:
  `config_dir`, `cache_dir`, `data_dir`, `data_local_dir` and `home_dir` return the same paths on
  Windows, Linux and macOS, from the same dirs-sys 0.5.0. So `settings.toml` stays where it was,
  and so do the cover copies the launcher's `art` key reads (`{cache}/<app>/covers/<stem>.png`).
- Edition 2024. `cargo fix --edition` found nothing to migrate, and the code behind Linux and
  macOS `cfg`s, which it does not see, was checked by hand: there are no `extern` blocks,
  `unsafe` code or environment writes in the crates. Nested `if let`s are now let chains, and
  fixed-size `chunks_exact` loops use `as_chunks`, as clippy asks for at the new MSRV.
- `cargo update`: every other compatible transitive bump.
- CI: an `msrv` job builds and tests the workspace on Rust 1.89 from the committed lockfile.

## v0.3.0 (2026-10-03)

`basket-app`: save pictures and covers in files the Fruit Basket launcher can read. The launcher
shows save pictures and uses them as covers, but cannot read the picture inside a `.state` header
or reproduce the path hash in `captures/<code>-<fnv>.png`.

### Added

- `Snapshot::picture_rgb` (default `None`). When an app gives its picture as RGB8,
  `SlotFormat::save` also writes `<rom stem>.s<N>.png` beside the state, after the state, through
  a temp file and rename. A failure there is only a warning.
- `slots::delete` removes the picture too. `slots::picture_path`, `slots::encode_png`.
- The label-art worker copies every capture it finds or makes to
  `<cache_dir>/covers/<rom stem>.png`. The last one wins on a shared stem.

Additive: apps that do not implement `picture_rgb` are unaffected.

## v0.2.0 (2026-10-03)

`basket-app`: controller connect and disconnect, for the launcher's controller name and its
couch mode, which opens when a pad is plugged in.

### Added

- `Gamepads::connected()`, the names of the connected pads.
- `PadPoll::connected` and `PadPoll::disconnected`, the pads that came and went since the last
  poll.

Additive: v0.1.0 users are unaffected.

## v0.1.0 (2026-10-02)

The first release: the shared frontend crates of the Fruit Basket emulators, moved out of
Strawberry's `gba-frontend`.

- `basket-ui`: the design language ("paper, ink, one red") as a window-resolution software
  renderer. `tokens` (kept in step with `tokens.json` by a test), `canvas` (tiny-skia and direct
  pixel writes), `text` (embedded OFL faces rasterised by fontdue), `widgets`, `drawings` (an
  SVG-subset parser for line art), `mark` (16x16 pixel marks), `fmt` (copy rules), and `input`
  (the per-frame input snapshot, rebindable key and pad maps over a `ButtonSet`, menu navigation
  through `MenuRoles`). The `testing` feature exposes `Canvas::data` and `fmt::test_clock`.
- `basket-app`: `audio` (SPSC ring, resampler, cpal stream), `pacing`, `saves`, `slots` (eight
  save-state slots per game in an app-defined `SlotFormat`), `prefs`, `library` (folder scan,
  display names, index cache, label art), `pads` (gilrs gamepads with per-port assignment), and
  `icon`.
- `basket-build`: the `build.rs` helper for the exe icon and version block on Windows.
- Edition 2021, `rust-version = "1.82"`.
