//! Gamepads (gilrs) -> button masks, one per console port.
//!
//! [`Ports::Shared`] ORs every connected pad into port 1 (Strawberry's single port). [`Ports::Assigned`]
//! gives each pad a port: pads listed by UUID take those ports, the rest fill the free ports in
//! connection order (the first pad connected is player 1). The left stick always acts as the
//! D-pad of its pad's port, through the map's `MenuRoles` arrows.
//!
//! Rumble goes the other way: [`Gamepads::set_rumble`] drives the motors of the pads on a port
//! through gilrs's force feedback. Pads without force feedback, and machines without gamepads,
//! ignore it.

use basket_ui::input::{MenuRoles, PadMap};
use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Replay, Ticks};
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use std::time::{Duration, Instant};

/// Left-stick deflection needed to register as a D-pad press.
pub const STICK_DEADZONE: f32 = 0.5;

/// Map an analog stick (gilrs convention: +x right, +y up) to the arrow bits of `roles`.
pub fn stick_to_dpad(x: f32, y: f32, deadzone: f32, roles: &MenuRoles) -> u16 {
    let mut m = 0;
    if x >= deadzone {
        m |= roles.right;
    } else if x <= -deadzone {
        m |= roles.left;
    }
    if y >= deadzone {
        m |= roles.up;
    } else if y <= -deadzone {
        m |= roles.down;
    }
    m
}

/// How connected pads map onto the console's ports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ports {
    /// Every pad drives port 1; their states are OR-ed.
    Shared,
    /// `count` ports. A pad whose UUID is `ids[i]` takes port `i` (for `i < count`); the other
    /// pads fill the free ports in connection order.
    Assigned { count: usize, ids: Vec<[u8; 16]> },
}

/// The port of each connected pad (`connected`: UUIDs in connection order); `None` = no free port.
pub fn assign(ports: &Ports, connected: &[[u8; 16]]) -> Vec<Option<usize>> {
    match ports {
        Ports::Shared => vec![Some(0); connected.len()],
        Ports::Assigned { count, ids } => {
            let mut out = vec![None; connected.len()];
            let mut taken = vec![false; *count];
            for (i, uuid) in connected.iter().enumerate() {
                if let Some(p) = ids.iter().position(|id| id == uuid).filter(|p| *p < *count && !taken[*p]) {
                    out[i] = Some(p);
                    taken[p] = true;
                }
            }
            for slot in out.iter_mut().filter(|s| s.is_none()) {
                if let Some(p) = taken.iter().position(|t| !t) {
                    *slot = Some(p);
                    taken[p] = true;
                }
            }
            out
        }
    }
}

/// A pad UUID as 32 lowercase hex digits (for a settings file).
pub fn uuid_text(uuid: &[u8; 16]) -> String {
    uuid.iter().map(|b| format!("{b:02x}")).collect()
}

/// Parse [`uuid_text`] output; hyphens are ignored, so the 8-4-4-4-12 form works too.
pub fn parse_uuid(text: &str) -> Option<[u8; 16]> {
    let hex: Vec<u8> = text.bytes().filter(|b| *b != b'-').collect();
    if hex.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, pair) in hex.chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}

/// How long rumble lasts after the last [`Gamepads::set_rumble`] call that asked for it. The app
/// calls it every emulated frame, so when emulation stops (a pause menu, a closed game) without a
/// final `set_rumble(0.0)`, the motors stop on their own at the next poll.
pub const RUMBLE_HOLD: Duration = Duration::from_millis(250);

/// Rumble strength changes smaller than this are not sent to the pads.
const RUMBLE_STEP: f32 = 1.0 / 64.0;

/// A rumble strength as a gain in 0..=1: out-of-range values are clamped and NaN is off.
pub fn rumble_gain(strength: f32) -> f32 {
    if strength.is_nan() { 0.0 } else { strength.clamp(0.0, 1.0) }
}

/// Which connected pads rumble for `port`: those [`assign`] put on it (`port_of`) that support
/// force feedback (`ff`, in the same order). Returns their indexes.
pub fn rumble_targets(port_of: &[Option<usize>], ff: &[bool], port: usize) -> Vec<usize> {
    port_of.iter().zip(ff).enumerate().filter(|(_, (p, ff))| **p == Some(port) && **ff).map(|(i, _)| i).collect()
}

/// The effect for one port's pads.
struct Rumble {
    effect: Effect,
    pads: Vec<GamepadId>,
    gain: f32,
    playing: bool,
    until: Instant,
}

pub struct Gamepads {
    gilrs: Option<Gilrs>,
    ports: Ports,
    /// Per port, its rumble effect once one was asked for.
    rumble: Vec<Option<Rumble>>,
}

impl Gamepads {
    pub fn new(ports: Ports) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                for (_, pad) in g.gamepads() {
                    println!("gamepad: {}", pad.name());
                }
                Some(g)
            }
            Err(e) => {
                eprintln!("warning: gamepad support unavailable ({e})");
                None
            }
        };
        Gamepads { gilrs, ports, rumble: Vec::new() }
    }

    /// The names of the connected pads, in connection order.
    pub fn connected(&self) -> Vec<String> {
        let Some(g) = self.gilrs.as_ref() else { return Vec::new() };
        let mut pads: Vec<(usize, String)> = g.gamepads().map(|(id, pad)| (usize::from(id), pad.name().to_string())).collect();
        pads.sort_by_key(|(id, _)| *id);
        pads.into_iter().map(|(_, name)| name).collect()
    }

    pub fn set_ports(&mut self, ports: Ports) {
        self.ports = ports;
    }

    /// True when at least one connected pad supports force feedback.
    pub fn rumble_supported(&self) -> bool {
        self.gilrs.as_ref().is_some_and(|g| g.gamepads().any(|(_, pad)| pad.is_ff_supported()))
    }

    /// Rumble the pads on port 1: [`Gamepads::set_rumble_port`] with port 0.
    pub fn set_rumble(&mut self, strength: f32) {
        self.set_rumble_port(0, strength);
    }

    /// Rumble the pads on `port` at `strength` (0 = off, 1 = full; clamped). Call it every
    /// emulated frame with the core's motor state: rumble stops by itself [`RUMBLE_HOLD`] after
    /// the last call that asked for it. Pads without force feedback, a port with no pads and a
    /// machine without gamepad support ignore it; it never fails.
    pub fn set_rumble_port(&mut self, port: usize, strength: f32) {
        let gain = rumble_gain(strength);
        let Some(g) = self.gilrs.as_mut() else { return };
        if gain == 0.0 {
            if let Some(r) = self.rumble.get_mut(port).and_then(Option::as_mut).filter(|r| r.playing) {
                let _ = r.effect.stop();
                r.playing = false;
            }
            return;
        }
        if self.rumble.len() <= port {
            self.rumble.resize_with(port + 1, || None);
        }
        let slot = &mut self.rumble[port];
        let mut pads: Vec<(usize, [u8; 16], bool, GamepadId)> =
            g.gamepads().map(|(id, pad)| (usize::from(id), pad.uuid(), pad.is_ff_supported(), id)).collect();
        pads.sort_by_key(|(n, ..)| *n);
        let uuids: Vec<[u8; 16]> = pads.iter().map(|p| p.1).collect();
        let ff: Vec<bool> = pads.iter().map(|p| p.2).collect();
        let targets: Vec<GamepadId> = rumble_targets(&assign(&self.ports, &uuids), &ff, port).into_iter().map(|i| pads[i].3).collect();
        if slot.as_ref().is_none_or(|r| r.pads != targets) {
            // first rumble on this port, or its pads changed: a new effect (dropping the old one
            // stops it)
            *slot = None;
            if targets.is_empty() {
                return;
            }
            let full = |kind| BaseEffect { kind, scheduling: Replay { play_for: Ticks::from_ms(100), ..Replay::default() }, ..BaseEffect::default() };
            let built = EffectBuilder::new()
                .add_effect(full(BaseEffectType::Strong { magnitude: u16::MAX }))
                .add_effect(full(BaseEffectType::Weak { magnitude: u16::MAX }))
                .gamepads(&targets)
                .gain(gain)
                .finish(g);
            match built {
                Ok(effect) => *slot = Some(Rumble { effect, pads: targets, gain, playing: false, until: Instant::now() }),
                Err(e) => {
                    eprintln!("warning: rumble unavailable ({e})");
                    return;
                }
            }
        }
        let Some(r) = slot.as_mut() else { return };
        if (r.gain - gain).abs() >= RUMBLE_STEP {
            let _ = r.effect.set_gain(gain);
            r.gain = gain;
        }
        if !r.playing {
            r.playing = r.effect.play().is_ok();
        }
        r.until = Instant::now() + RUMBLE_HOLD;
    }

    /// Stop the rumble whose [`RUMBLE_HOLD`] ran out.
    fn expire_rumble(&mut self, now: Instant) {
        for r in self.rumble.iter_mut().flatten().filter(|r| r.playing && now >= r.until) {
            let _ = r.effect.stop();
            r.playing = false;
        }
    }

    /// One port: [`Gamepads::poll_ports`] with a single map.
    pub fn poll(&mut self, map: &PadMap) -> PadPoll {
        self.poll_ports(&[map]).pop().unwrap_or_default()
    }

    /// Drain pending events and return one [`PadPoll`] per map (port): the OR of the key masks of
    /// the pads on that port through its map (plus their left sticks as D-pad), and the buttons
    /// that went down on them (for rebinding). Pads without a port are ignored.
    pub fn poll_ports(&mut self, maps: &[&PadMap]) -> Vec<PadPoll> {
        self.expire_rumble(Instant::now());
        let mut out = vec![PadPoll::default(); maps.len()];
        let Some(g) = self.gilrs.as_mut() else { return out };
        let mut pads: Vec<(usize, [u8; 16])> = g.gamepads().map(|(id, pad)| (usize::from(id), pad.uuid())).collect();
        pads.sort_by_key(|(id, _)| *id);
        let uuids: Vec<[u8; 16]> = pads.iter().map(|(_, u)| *u).collect();
        let port_of = assign(&self.ports, &uuids);
        let port_for = |id: usize| pads.iter().position(|(p, _)| *p == id).and_then(|i| port_of[i]).filter(|p| *p < maps.len());
        let mut plugged = (Vec::new(), Vec::new());
        while let Some(ev) = g.next_event() {
            match ev.event {
                EventType::Connected => plugged.0.push(g.gamepad(ev.id).name().to_string()),
                EventType::Disconnected => plugged.1.push(g.gamepad(ev.id).name().to_string()),
                _ => {}
            }
            if let EventType::ButtonPressed(b, _) = ev.event {
                let Some(p) = (if self.ports == Ports::Shared { Some(0).filter(|_| !maps.is_empty()) } else { port_for(usize::from(ev.id)) }) else { continue };
                if b != Button::Unknown && !out[p].pressed.contains(&b) {
                    out[p].pressed.push(b);
                }
            }
        }
        for (id, pad) in g.gamepads() {
            let Some(p) = port_for(usize::from(id)) else { continue };
            let map = maps[p];
            out[p].mask |= map.mask(|b| pad.is_pressed(b));
            let x = pad.value(Axis::LeftStickX);
            let y = pad.value(Axis::LeftStickY);
            out[p].mask |= stick_to_dpad(x, y, STICK_DEADZONE, &map.set().roles);
        }
        // Plugging is per machine, not per port: every poll reports it.
        for poll in out.iter_mut() {
            poll.connected.clone_from(&plugged.0);
            poll.disconnected.clone_from(&plugged.1);
        }
        out
    }
}

/// One frame of gamepad input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PadPoll {
    /// Button mask of every connected pad.
    pub mask: u16,
    /// Buttons that went down since the last poll.
    pub pressed: Vec<Button>,
    /// Names of the pads plugged in since the last poll (some backends also report the pads
    /// already there at start; use [`Gamepads::connected`] for the current list).
    pub connected: Vec<String>,
    /// Names of the pads unplugged since the last poll.
    pub disconnected: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: MenuRoles = MenuRoles { up: 1, down: 2, left: 4, right: 8, confirm: 16, back: 32, prev_page: 0, next_page: 0, start: 0 };

    #[test]
    fn stick_mapping() {
        assert_eq!(stick_to_dpad(0.0, 0.0, 0.5, &R), 0);
        assert_eq!(stick_to_dpad(0.3, -0.3, 0.5, &R), 0);
        assert_eq!(stick_to_dpad(1.0, 0.0, 0.5, &R), 8);
        assert_eq!(stick_to_dpad(-0.6, 0.9, 0.5, &R), 4 | 1);
        assert_eq!(stick_to_dpad(0.0, -1.0, 0.5, &R), 2);
    }

    #[test]
    fn ports_follow_the_list_then_connection_order() {
        let (a, b, c) = ([1u8; 16], [2u8; 16], [3u8; 16]);
        assert_eq!(assign(&Ports::Shared, &[a, b]), vec![Some(0), Some(0)]);
        let first_come = Ports::Assigned { count: 2, ids: vec![] };
        assert_eq!(assign(&first_come, &[a, b, c]), vec![Some(0), Some(1), None], "first connected is player 1");
        let listed = Ports::Assigned { count: 2, ids: vec![c, a] };
        assert_eq!(assign(&listed, &[a, b, c]), vec![Some(1), None, Some(0)]);
        let partial = Ports::Assigned { count: 2, ids: vec![b] };
        assert_eq!(assign(&partial, &[a, b]), vec![Some(1), Some(0)], "the listed pad keeps port 1");
        let beyond = Ports::Assigned { count: 1, ids: vec![a, b] };
        assert_eq!(assign(&beyond, &[b, a]), vec![None, Some(0)], "ids past the port count are ignored");
    }

    #[test]
    fn rumble_gain_is_clamped() {
        assert_eq!(rumble_gain(0.0), 0.0);
        assert_eq!(rumble_gain(0.5), 0.5);
        assert_eq!(rumble_gain(3.0), 1.0);
        assert_eq!(rumble_gain(-1.0), 0.0);
        assert_eq!(rumble_gain(f32::NAN), 0.0);
    }

    #[test]
    fn rumble_goes_to_the_ports_force_feedback_pads() {
        let port_of = [Some(0), Some(1), Some(0), None, Some(0)];
        let ff = [true, true, false, true, true];
        assert_eq!(rumble_targets(&port_of, &ff, 0), vec![0, 4], "pad 2 has no motors");
        assert_eq!(rumble_targets(&port_of, &ff, 1), vec![1]);
        assert_eq!(rumble_targets(&port_of, &ff, 2), Vec::<usize>::new());
        assert_eq!(rumble_targets(&[], &[], 0), Vec::<usize>::new());
    }

    #[test]
    fn rumble_without_gamepad_support_is_a_no_op() {
        let mut pads = Gamepads { gilrs: None, ports: Ports::Shared, rumble: Vec::new() };
        assert!(!pads.rumble_supported());
        pads.set_rumble(1.0);
        pads.set_rumble_port(3, 0.5);
        pads.set_rumble(0.0);
        assert!(pads.rumble.is_empty());
        assert!(pads.poll_ports(&[]).is_empty());
    }

    #[test]
    fn uuid_text_round_trips() {
        let u = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0, 1, 2, 3, 4, 5, 6, 0xff];
        let t = uuid_text(&u);
        assert_eq!(t, "0123456789abcdef00010203040506ff");
        assert_eq!(parse_uuid(&t), Some(u));
        assert_eq!(parse_uuid("01234567-89ab-cdef-0001-0203040506ff"), Some(u));
        assert_eq!(parse_uuid("0123"), None);
        assert_eq!(parse_uuid("zz23456789abcdef00010203040506ff"), None);
    }
}
