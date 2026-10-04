# fruit-basket

The shared frontend crates of the Fruit Basket emulators (Strawberry, a Game Boy Advance
emulator, and Crabapple, an NES emulator) and of the Fruit Basket launcher: one design language,
one software renderer, and the desktop plumbing every emulator needs, written once.

| crate | what it is |
|---|---|
| `basket-ui` | The design language ("paper, ink, one red") as a window-resolution software renderer: `tokens` (with `tokens.json`, kept in step by a test), `canvas` (tiny-skia + direct pixel writes), `text` (embedded OFL faces rasterised by fontdue), `widgets`, `drawings` (an SVG-subset parser for line art), `mark` (16x16 pixel marks from SVG), `fmt` (copy rules), `input` (per-frame input snapshot, rebindable key and pad maps over a `ButtonSet`, menu navigation through `MenuRoles`). |
| `basket-app` | The emulator-independent half of a frontend: `audio` (SPSC ring, resampler, cpal stream), `pacing` (frame clock), `saves` (`.sav` files over a `SaveRam`), `slots` (eight save-state slots per game in an app-defined `SlotFormat` over a `Snapshot`, plus a `<stem>.s<N>.png` picture when `Snapshot::picture_rgb` gives one), `prefs` (settings-file mechanics: unknown keys kept, a bad value costs only its key), `library` (folder scan, display names, index cache, label art and `covers/<stem>.png` copies of title captures, with the console behind a `Platform`), `pads` (gilrs gamepads to per-port masks), `icon` (the window icon on Windows and X11). |
| `basket-build` | `build.rs` helper: the exe icon and version block on Windows. |

An app supplies its console through small types: the picture size (`basket_ui::Screen`), a
colour table for `Canvas::blit_indexed` (texel bit 15 = transparent), a `ButtonSet`, and
implementations of `SaveRam`, `Snapshot` and `Platform`.

## Using it

```toml
[dependencies]
basket-ui = { git = "https://github.com/0x000NULL/fruit-basket", tag = "v0.4.0" }
basket-app = { git = "https://github.com/0x000NULL/fruit-basket", tag = "v0.4.0" }

[build-dependencies]
basket-build = { git = "https://github.com/0x000NULL/fruit-basket", tag = "v0.4.0" }
```

Apps that pin their renders can enable `basket-ui`'s `testing` feature in their
dev-dependencies: it exposes `Canvas::data` and `fmt::test_clock` (a frozen UTC clock, so date
text does not depend on the host).

The crates use edition 2024 and need Rust 1.89 or later. They expose `minifb`, `tiny-skia`,
`gilrs` and `toml` types in their API, so an app pins those crates at the majors in
[docs/DEPS.md](docs/DEPS.md); [CHANGELOG.md](CHANGELOG.md) lists what changed in each release.

Linux builds need ALSA and udev headers (`libasound2-dev libudev-dev`).

## Docs

- [CHANGELOG.md](CHANGELOG.md): every release, and what an app fixes when it re-pins.
- [docs/ECOSYSTEM.md](docs/ECOSYSTEM.md): every fruit, the launcher, this repo and the site, and
  who depends on what.
- [docs/LAUNCHER-CONTRACT.md](docs/LAUNCHER-CONTRACT.md): the `LAUNCHER` file keys, the feed's
  fields, and the flags and files a fruit must support.
- [docs/RELEASING.md](docs/RELEASING.md): the release order from a fruit-basket tag to the signed
  feed.
- [docs/DEPS.md](docs/DEPS.md): the shared dependency versions and how to bump them together.

## License

MIT (see `LICENSE`). The bundled fonts (Archivo, Source Serif 4, IBM Plex Mono) are under the
SIL Open Font License 1.1; their licence texts are in `basket-ui/assets/fonts/`.
