# Shared dependencies

Strawberry, Crabapple, Mulberry, Olive and the launcher link the fruit-basket crates, and the API
carries types from some of their dependencies (`minifb::Key`, `tiny_skia::Path`, `toml::Table`).
Two majors of one crate in a build are two different types, so the Rust repos keep these crates
in step.

## The table

The versions in fruit-basket v0.4.0, unchanged in v0.5.0. The consumers match them when they
re-pin to either.

| Crate | Requirement | Locked in v0.4.0 | In the fruit-basket API | Used by |
|---|---|---|---|---|
| `minifb` | `0.29` | 0.29.0 | yes: `Key`, `Window` | basket-ui, basket-app |
| `tiny-skia` | `0.12` | 0.12.0 | yes: `Pixmap`, `Path`, `Transform` | basket-ui |
| `gilrs` | `0.11` | 0.11.2 | yes: `Button` | basket-ui, basket-app |
| `toml` | `1` | 1.1.6 | yes: `Table` | basket-app |
| `cpal` | `0.18` | 0.18.2 | no | basket-app |
| `png` | `0.18` | 0.18.1 | no | basket-app; basket-ui tests |
| `dirs` | `7` | 7.0.0 | no | basket-app |
| `serde` | `1` | 1.0.229 | no | basket-app |
| `anyhow` | `1` | 1.0.104 | yes: `Result` | basket-ui, basket-app |
| `chrono` | `0.4` | 0.4.45 | no | basket-ui |
| `fontdue` | `0.9` | 0.9.4 | no | basket-ui |
| `raw-window-handle` | `0.6` | 0.6.2 | no (must match minifb's) | basket-app |
| `winresource` | `0.1.31` | 0.1.31 | no | basket-build (Windows) |

Toolchain: edition 2024, `rust-version = "1.89"`, resolver 3. Every Rust repo in Fruit Basket
declares the same `rust-version`.

## The rule

**A consumer never pins a shared crate at a different major than fruit-basket.** For crates
before 1.0 the major is the minor (`0.29`). This applies even to crates the fruit-basket API does
not expose (`cpal`, `png`, `dirs`): two majors in one binary build twice, and the next re-pin
then has to fix two sets of fallout at once.

The patch level can differ. Each repo has its own `Cargo.lock` and runs `cargo update` on its own
schedule.

## bincode stays on 1.x

The emulator cores (Strawberry's and Crabapple's) serialize save states with `bincode` 1.x. That
is on purpose, and a dependency refresh leaves it alone. bincode 2 changed both the API and the
default encoding, so a state saved by an older build would no longer load, and the slot files
players already have would be lost. Moving off bincode 1 is a save-state format change: it needs
its own release, with a format version bump and a reader for the old format. fruit-basket does
not depend on bincode; it stores states as bytes in the format each app's `SlotFormat` defines.

## A coordinated bump

1. **Check each new major before taking it.** Read its changelog and, for anything that touches
   paths or file formats, diff its sources. If a bump changes behaviour players would see (a
   moved config, cache or data directory, a different audio device name, a changed save format),
   stay on the old major and say why in `CHANGELOG.md`. For example, the launcher finds covers
   at `{cache}/<app>/covers/<stem>.png`, so a `dirs` that moves the cache directory silently
   loses every cover.
2. **fruit-basket first.** Bump the requirements, run `cargo update`, fix the fallout in all
   three crates and their tests, and raise `rust-version` to the minimum the new set needs (the
   CI `msrv` job checks it). Release it as described in [RELEASING.md](RELEASING.md), with every
   breaking change listed in `CHANGELOG.md`.
3. **Then every consumer**, to the same majors as the new table, in one change per repo together
   with the re-pin.
4. **Update this table** in the same fruit-basket release.
