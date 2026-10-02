//! The emulator-independent half of a Fruit Basket frontend. The app supplies the console:
//!
//! * [`audio`]: SPSC ring, linear resampler and cpal stream; the app picks the [`audio::Buffering`].
//! * [`pacing`]: frame clock with an adaptive sleep margin and audio-fill correction.
//! * [`saves`]: `<rom>.sav` persistence over any [`saves::SaveRam`], and `write_atomic`.
//! * [`slots`]: eight save-state slots per game in the app's [`slots::SlotFormat`], over any
//!   [`slots::Snapshot`].
//! * [`prefs`]: the settings file's mechanics (unknown keys kept, a bad value costs only its key)
//!   and name-based key and pad bindings.
//! * [`library`]: folder scan, display names, the index cache and label art, with the console
//!   behind [`library::Platform`].
//! * [`pads`]: gilrs gamepads to per-port button masks.
//! * [`icon`]: the window icon on Windows and X11.

pub mod audio;
pub mod icon;
pub mod library;
pub mod pacing;
pub mod pads;
pub mod prefs;
pub mod saves;
pub mod slots;
