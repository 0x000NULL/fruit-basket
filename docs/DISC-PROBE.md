# Which fruit plays a disc: the probe

Status: approved by the Orchestrator (2026-10-21). It replaces the launcher's
[shared-extensions probe](LAUNCHER-CONTRACT.md#shared-extensions-the-disc-probe) (launcher
v1.6.0) and is the rule for the apps' importers too (Android `GameStore`, iOS). Launcher and apps
move to it when the next disc fruit (Peach, Mango or Orange) ships.

## The rule

Disc images share extensions (`.cue`, `.chd`, `.iso`, `.bin`, `.zip` today; `.gdi` with Orange).
A file one installed fruit plays goes to it, as before. Once two fruits claim an extension, the
extension alone never decides:

1. Run **every** installed disc fruit's probe on the file. A probe is a signature check (fixed
   bytes at a fixed place), never a heuristic, and never "this is a CUE, so it must be mine".
2. **Exactly one** match: that fruit. The order the probes run in never matters.
3. **None**: refuse it, "not a recognised disc", with the file's name. No lowest-number fallback.
4. **More than one**: ask the player which console it is for, and report it: two real signatures
   matching one disc is a bug in a probe, fixed in that fruit's repo.
5. A read error is reported as one ("couldn't read the disc"), not as "not recognised".

A result is cached by path, size and modification time. A file in a fruit's own `games/` folder
is never probed (launcher), and an app's import routes once, at import time.

## Containers

The probe reads the track the console boots from, through the same `FbFile` reader the cores use:

- **ISO / BIN**: 2048-byte sectors, or raw 2352-byte ones (mode 1 data at offset 16; mode 2
  form 1 at offset 24).
- **CUE**: the first data track its `FILE`/`TRACK` lines name, beside it. **Dreamcast is the
  exception**: a GD-ROM's boot track is the first data track of the high-density area (track 3
  in a Redump CUE/GDI), not track 1, whose low-density data track holds no IP.BIN.
- **GDI**: its track list, the same rule (track 3).
- **CHD**: through its own decoder (libchdr, already in the apps for Kumquat). The CHD's track
  metadata says what it holds before any hunk is read: CD tracks (`CHT2`/`CHCD`), GD-ROM tracks
  (`CHGT`/`CHGD`), or a DVD (`DVD `), which already separates Dreamcast and PSP/PS2/GameCube
  from CD consoles. The signature is then read from the boot track as above: one hunk.
- **ZIP**: its first entry ending `.iso .gcm .ciso .bin .img`, as launcher v1.6.0 does (a stored
  entry read in place, a compressed one inflated no further than the first 4 MB). Durian's Neo
  Geo sets stay under `archives`, never probed as discs.
- **CISO / CSO**: through their block maps.

## The signatures and who owns them

Each fruit owns its probe's exact bytes in its own repo; this table only says where. A new disc
fruit adds its row here and its probe in its repo **before** it ships, or it gets nothing for the
shared extensions.

| No. | Fruit | Console | Signature (the owner's doc is the authority) | Owner, where |
|---:|---|---|---|---|
| 1 | Pomegranate | PS2 | ISO9660 primary volume descriptor (sector 16, `CD001`) system identifier `PLAYSTATION`, and `SYSTEM.CNF` in the root with a `BOOT2` line | `ps2emu` (desktop only); today only the launcher's marker, no core probe |
| 4 | Fig | PS1 | PVD system identifier `PLAYSTATION`, and `SYSTEM.CNF` with a `BOOT` line and no `BOOT2`, **or** no `SYSTEM.CNF` and `PSX.EXE` in the root (early discs boot it by default) | `fig`: **owed**. Fig's FFI (`docs/ffi-v1.3.md`) has no probe yet; Fig adds one with exactly this check |
| 6 | Mango | GameCube | `C2339F3Dh` at `1Ch` (a CISO: of its first data block) | `mango` (0x000NULL/mango) `docs/ffi-v1.7.md`, part 2 › Additions › 7 "Core-less probe for the library", `fb_mango_probe` |
| 12 | Orange | Dreamcast | IP.BIN at the boot track's first sector starts `SEGA SEGAKATANA ` | `orange`: **owed** (no FFI doc yet) |
| 13 | Peach | Saturn | sector 0 of the first data track starts `SEGA SEGASATURN ` | `peach` `docs/ffi-v1.8.md`, `fb_peach_probe` |
| 14 | Kumquat | PSP | PVD system identifier `PSP GAME` | `kumquat` `docs/ffi-v1.6.md`, `fb_kumquat_probe` |

Notes:

- Each signature's source is recorded in its owner's CLEAN-ROOM; a row is final only once the
  owner has done so. Fig's PS1 row and Orange's Dreamcast row stay owed until then.
- PS1 and PS2 share the PVD identifier `PLAYSTATION`; the `BOOT`/`BOOT2` line is what separates
  them. The `PSX.EXE` alternative is the one place a PS1 disc has no `SYSTEM.CNF`. Fig confirms
  it against its compat discs before the row is final.
- The Saturn and Dreamcast strings are 16 bytes with the trailing space. Sega CD
  (`SEGADISCSYSTEM`) has no fruit and is "not recognised".
- A probe that needs the core (a title, a game id) still answers "mine or not" from the
  signature alone; the extra fields are for the library.

## What changes from launcher v1.6.0

- No match is refused, not given to the lowest-numbered fruit.
- CHDs are probed (through their metadata and one hunk), not skipped.
- Saturn and Dreamcast get rows; PS1 gains the no-`SYSTEM.CNF` case.
- Two matches ask instead of picking.
- The bytes live in each fruit's repo; the launcher and the apps implement this table and link to
  them.
