//! Gamepads (gilrs) -> button masks, one per console port.
//!
//! [`Ports::Shared`] ORs every connected pad into port 1 (Strawberry's single port). [`Ports::Assigned`]
//! gives each pad a port: pads listed by UUID take those ports, the rest fill the free ports in
//! connection order (the first pad connected is player 1). The left stick always acts as the
//! D-pad of its pad's port, through the map's `MenuRoles` arrows.

use basket_ui::input::{MenuRoles, PadMap};
use gilrs::{Axis, Button, EventType, Gilrs};

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

pub struct Gamepads {
    gilrs: Option<Gilrs>,
    ports: Ports,
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
        Gamepads { gilrs, ports }
    }

    pub fn set_ports(&mut self, ports: Ports) {
        self.ports = ports;
    }

    /// One port: [`Gamepads::poll_ports`] with a single map.
    pub fn poll(&mut self, map: &PadMap) -> PadPoll {
        self.poll_ports(&[map]).pop().unwrap_or_default()
    }

    /// Drain pending events and return one [`PadPoll`] per map (port): the OR of the key masks of
    /// the pads on that port through its map (plus their left sticks as D-pad), and the buttons
    /// that went down on them (for rebinding). Pads without a port are ignored.
    pub fn poll_ports(&mut self, maps: &[&PadMap]) -> Vec<PadPoll> {
        let mut out = vec![PadPoll::default(); maps.len()];
        let Some(g) = self.gilrs.as_mut() else { return out };
        let mut pads: Vec<(usize, [u8; 16])> = g.gamepads().map(|(id, pad)| (usize::from(id), pad.uuid())).collect();
        pads.sort_by_key(|(id, _)| *id);
        let uuids: Vec<[u8; 16]> = pads.iter().map(|(_, u)| *u).collect();
        let port_of = assign(&self.ports, &uuids);
        let port_for = |id: usize| pads.iter().position(|(p, _)| *p == id).and_then(|i| port_of[i]).filter(|p| *p < maps.len());
        while let Some(ev) = g.next_event() {
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
