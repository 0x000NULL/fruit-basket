# Changelog

All three crates (`basket-ui`, `basket-app`, `basket-build`) share one version and one tag. Apps
pin them by git tag (see the README).

## v0.6.0 — 2026-10-08

One additive feature: analog sticks and two-motor rumble for consoles that have them (Fig's
DualShock first). Nothing was renamed, moved or removed, no public struct gained a field and no
public enum a variant, so an app on v0.5.0 re-pins with no code changes. Dependencies are
unchanged ([docs/DEPS.md](docs/DEPS.md) still holds). Nothing an app writes, and nothing the
launcher reads, changed.

### Breaking changes for apps re-pinning from v0.5.0

None.

### Added

- **Analog sticks** (`basket_app::pads`). `Gamepads::axes(port)` returns the sticks of the
  pads on a port as of the last poll: `[left X, left Y, right X, right Y]`, each
  -32767..=32767, 0 at the centre, +x right and +y down (CORE-ABI's convention, so desktop and
  mobile hosts hand a core the same numbers). Several pads on one port combine axis by axis, the
  most deflected one winning (`combine_axes`), so two pads on a shared port never cancel out. No
  pads, or no gamepad support: centred. The left stick still acts as the D-pad in `PadPoll`'s
  mask, as before. Helper: `axis_i16` (gilrs's -1..1 to the 16-bit value, Y flipped on request).
- **Two-motor rumble.** `Gamepads::set_rumble_dual(port, strong, weak)` drives the strong
  (low-frequency) and weak (high-frequency) motors at strengths of their own, each 0.0 to 1.0
  (clamped, NaN is off): one gilrs effect per motor, each at its own gain. It behaves as
  `set_rumble_port` otherwise: call it every emulated frame, it stops by itself `RUMBLE_HOLD`
  after the last call that asked for it, and it never fails. Use it or `set_rumble_port` on a
  port, not both. `set_rumble` and `set_rumble_port` are unchanged.

### Docs (after the v0.5.0 tag)

- `docs/ECOSYSTEM.md`: Mulberry and Olive are released (repos, binaries, platforms); the fruit
  table gains each fruit's latest release; the iOS app has all four Rust fruits built in; the four
  fruits pin v0.5.0 and the launcher v0.4.0.
- `docs/RELEASING.md`: Mulberry and Olive are consumers; how to check that a release is additive
  by building every consumer against it with `[patch]`; where v0.5.0 stands.
- `docs/DEPS.md`: Mulberry and Olive link the crates; the table is unchanged in v0.5.0.
- 2026-10-07: Crabapple v1.0.0 and launcher v1.3.0 are released; the launcher now pins v0.5.0
  too (ECOSYSTEM, RELEASING).

## v0.5.0 (2026-10-06)

Two additive features: rumble and per-ROM system tags for the library. Nothing was renamed, moved
or removed, no public struct gained a field and no public enum a variant, so an app on v0.4.0
re-pins with no code changes. Dependencies are unchanged ([docs/DEPS.md](docs/DEPS.md) still
holds). Save files, save-state slots, covers and `index.toml` are written exactly as before;
nothing the launcher reads changed.

### Breaking changes for apps re-pinning from v0.4.0

None.

### Added

- **Rumble** (`basket_app::pads`). `Gamepads::set_rumble(strength)` rumbles the pads on port 1,
  and `Gamepads::set_rumble_port(port, strength)` those on any port, at `strength` 0.0 (off) to
  1.0 (full; clamped, NaN is off), through gilrs force feedback on both motors. Call it every
  emulated frame with the core's motor state. Rumble stops by itself `RUMBLE_HOLD` (250 ms) after
  the last call that asked for it, at the next `poll`/`poll_ports`, so a pause menu or a closed
  game never leaves a pad buzzing. Pads without force feedback, a port with no pads and a machine
  without gamepad support ignore it; it never fails. `Gamepads::rumble_supported()` says whether
  any connected pad has motors (for a settings toggle). Helpers: `rumble_gain`,
  `rumble_targets`.
- **System tags** (`basket_app::library`), for a fruit that plays more than one system:
  - `Platform::probe_system(&self, path) -> (RomInfo, String)`, a provided method: the ROM's
    description plus a short system tag of the fruit's choosing (`"gbc"`), `""` for none. The
    default calls `probe` and tags nothing, so a fruit that does not override it is unchanged.
    The library's scan now calls `probe_system` instead of `probe`; the public `scan::scan`,
    `scan::spawn` and `scan::scan_collect` behave as before.
  - `Library::system_of(path) -> &str`, `Library::systems() -> Vec<&str>` (the distinct tags in
    the library, sorted, without `""`: the tabs after "All"), and
    `Library::entries_in(Option<&str>)` (one tab's entries in `entries()` order; `None` is all,
    `Some("")` the untagged ones). `Library::set_system` tags an entry by hand, for
    `Library::from_entries` tests and renders.
  - The tags are cached in a new `<cache_dir>/systems.toml` beside `index.toml` (module
    `library::systems`), so a warm start has them before the scan. The first scan after the
    update reads every ROM once more, in the background, because v0.4.0's index has no tags;
    after that only new or changed files are read, as before.

### Docs

`docs/LAUNCHER-CONTRACT.md` after the v0.4.0 tag:

- `notes` and `assets` describe what feedgen.py actually emits; the four platforms are the ones in
  use, not a whitelist.
- A template's flags must exist in every build the feed lists, from `oldest` up; a key that would
  only repeat `launch` is left out rather than `oldest` raised.
- In an extra folder, a file whose extension several fruits list goes to the first fruit in feed
  order; overlaps must be resolved before the second such fruit ships.
- `couch`: the "only listed builds" rule is launcher v0.5.1 behaviour (was an unexplained "(v0.5.1)").

`docs/ECOSYSTEM.md` adds Fruit Basket for iOS: one app with Strawberry and Crabapple built in,
linked through `gba-ffi` and `nes-ffi`, outside the feed and not on these crates.

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
