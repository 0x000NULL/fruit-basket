# Which fruit plays a disc: the probe

Status: approved by the Orchestrator (2026-10-21), amended the same day after launcher-7e's
review. It replaces the launcher's
[shared-extensions probe](LAUNCHER-CONTRACT.md#shared-extensions-the-disc-probe) (launcher
v1.6.0) and is the rule for the apps' importers too (Android `GameStore`, iOS). When each moves
to it is under [Versions](#versions).

## The rule

Disc images share extensions (`.cue`, `.chd`, `.iso`, `.bin`, `.zip` today; `.gdi` with Orange).
A file whose extension belongs to a fruit with a row in [the table](#the-signatures-and-who-owns-them)
is always probed, even when only one installed fruit claims it; the extension alone never decides.
(The one exception is under Containers: `.cso`.)

1. Run **every** installed disc fruit's probe on the file. A probe is a signature check (fixed
   bytes at a fixed place), never a heuristic, and never "this is a CUE, so it must be mine".
2. **Exactly one** match: that fruit. The order the probes run in never matters.
3. **None**: refuse it, "not a recognised disc", with the file's name. No lowest-number fallback.
4. **More than one**: ask the player which console it is for, and report it: two real signatures
   matching one disc is a bug in a probe, fixed in that fruit's repo.
5. A read error is reported as one ("couldn't read the disc"), not as "not recognised".
6. **Desktop only:** a probe that hasn't answered yet (a slow share) is "pending", never "not
   recognised"; a later scan finishes it. The apps run their probes synchronously at import, so
   their results are only match, none, tied or read error.

A refused file can still be given to a fruit by hand ("Play with…") wherever the UI has that.
A result is cached by path, size and modification time. A file in a fruit's own `games/` folder
is never probed (launcher), and an app's import routes once, at import time.

## Containers

The probe reads the track the console boots from, through the same `FbFile` reader the cores use:

- **ISO / BIN**: 2048-byte sectors; raw 2352-byte ones, with the user data at the offset the
  sector's own mode byte gives (byte 15: 1 → 16, 2 → 24, mode 2 form 1); or MODE2/2336 images:
  2336-byte stored sectors (sync and header not stored), the user data at offset 8 of each stored
  sector, after the 8-byte subheader.
- **CUE**: the first **data** track its `FILE`/`TRACK` lines name, beside it (AUDIO tracks are
  skipped). **Dreamcast is the exception**: a GD-ROM's boot track is the first data track of the
  high-density area (track 3 in a Redump CUE/GDI), not track 1, whose low-density data track holds
  no IP.BIN.
- **GDI**: its track list, the same rule (track 3).
- **CHD**: through its own decoder (libchdr on desktop; in the apps the `chd` crate, its Rust
  port, already there for Kumquat).
  - The track metadata says what it holds before any hunk is read: CD tracks (`CHT2` current,
    `CHTR` older, `CHCD` legacy), GD-ROM tracks (`CHGD` current, the only one in Orange's
    images; `CHGT` legacy, still read), or a DVD (`DVD `). The GD-ROM metadata alone decides
    Dreamcast. (Tag names as the `chd` crate 0.3.4 defines them, `src/metadata.rs`.)
  - Otherwise it reads the hunks holding the signature: one for GameCube and Saturn; two to four
    for the PlayStation rows (the PVD in hunk 2, the root directory, `SYSTEM.CNF`; a CD hunk is
    8 frames). A DVD CHD's hunks are 2048 or 4096 bytes.
  - **Caps**, checked before the decoder is called, in the launcher and the apps alike: a hunk
    map of at most 16 MB, at most 4.5 M hunks, a hunk of at most 1 MB. Past a cap the file is
    a read error. Raising the caps for odd layouts is not required.
- **ZIP**: its first entry ending `.iso .gcm .ciso .bin .img`, as launcher v1.6.0 does (a stored
  entry read in place, a compressed one inflated no further than the first 4 MB). Durian's Neo
  Geo sets stay under `archives`, never probed as discs.
- **CISO** means GameCube's block map only. PSP's `.cso` is not probed while Kumquat is the only
  fruit that claims it.

## The signatures and who owns them

Each fruit owns its probe's exact bytes in its own repo; this table only says where. A new disc
fruit adds its row here and its probe in its repo **before** it ships, or it gets nothing for the
shared extensions.

| No. | Fruit | Console | Signature (the owner's doc is the authority) | Owner, where |
|---:|---|---|---|---|
| 1 | Pomegranate | PS2 | ISO9660 primary volume descriptor (sector 16, `CD001`) system identifier `PLAYSTATION`, and `SYSTEM.CNF` in the root with a `BOOT2` line | `ps2emu` (desktop only); today only the launcher's marker, no core probe |
| 4 | Fig | PS1 | PVD system identifier `PLAYSTATION`, and `SYSTEM.CNF` with a `BOOT` line and no `BOOT2`, **or** no `SYSTEM.CNF` and `PSX.EXE` in the root (early discs boot it by default) | `fig`: **owed**. Fig's FFI (`docs/ffi-v1.3.md`) has no probe yet; Fig adds one with exactly this check |
| 6 | Mango | GameCube | `C2339F3Dh` at `1Ch` (a CISO: at `1Ch` of its first stored block, which follows the `8000h`-byte header: file offset `801Ch`) | `mango` (0x000NULL/mango) `docs/ffi-v1.7.md`, part 2 › Additions › 7 "Core-less probe for the library", `fb_mango_probe` |
| 12 | Orange | Dreamcast | IP.BIN at the boot track's first sector starts `SEGA SEGAKATANA ` | `orange` `docs/ffi-v1.9.md`, `fb_orange_probe` |
| 13 | Peach | Saturn | sector 0 of the first data track starts `SEGA SEGASATURN ` | `peach` `docs/ffi-v1.8.md`, `fb_peach_probe` |
| 14 | Kumquat | PSP | PVD system identifier `PSP GAME` | `kumquat` `docs/ffi-v1.6.md`, `fb_kumquat_probe` |

Notes:

- Each signature's source is recorded in its owner's CLEAN-ROOM; a row is final only once the
  owner has done so. Fig's PS1 row stays owed until then.
- PS1 and PS2 share the PVD identifier `PLAYSTATION`; the `BOOT`/`BOOT2` line is what separates
  them. The `PSX.EXE` alternative is the one place a PS1 disc has no `SYSTEM.CNF`. Fig confirms
  it against its compat discs before the row is final.
- The Saturn and Dreamcast strings are 16 bytes with the trailing space. Sega CD
  (`SEGADISCSYSTEM`) has no fruit and is "not recognised".
- A probe that needs the core (a title, a game id) still answers "mine or not" from the
  signature alone; the extra fields are for the library.

## Test headers

The launcher, the apps and each fruit's probe test against the same files, kept in basket: a
few KB each, **synthetic only** (built by code, no bytes from any commercial disc), MIT. The set:
GameCube, CISO, PS1 and PS2 cooked and raw, a PS1 disc with `PSX.EXE` and no `SYSTEM.CNF`, PSP,
Saturn, and Dreamcast as CUE, GDI and CHD. Launcher-7e's builders are the first set.

## What changes from launcher v1.6.0

- No match is refused, not given to the lowest-numbered fruit.
- CHDs are probed (metadata, then the hunks holding the signature, under the caps), not skipped.
- Saturn and Dreamcast get rows; PS1 gains the no-`SYSTEM.CNF` case; a CUE skips AUDIO tracks.
- Files are probed even with a single claimant.
- Two matches ask instead of picking.
- The bytes live in each fruit's repo; the launcher and the apps implement this table and link to
  them.

## Versions

- **Desktop launcher:** the rule applies from the launcher's UI milestone (version: to be filled
  in). Earlier installs, the next sega-probe release included, keep the fallback: none or a tie
  goes to the lowest-numbered fruit, with a log line on a tie.
- **Android:** from the next disc fruit's pin (Peach), the importer refuses with the reason in
  the import summary (the summary lists reasons, not only a count), and asks about tied files in
  one sheet at the end of the import. Until that sheet exists, a tie is skipped ("matches more
  than one console") and logged naming both fruits; it never falls through.
- **iOS:** from the next disc fruit's pin (Peach), the importer refuses unmatched discs with the
  reason in the import result and asks about tied files in one sheet at the end of the import
  (UI-SPEC §22, fruit-basket-ios `842fede`).
- **Both apps** check the PS1 row themselves until fig's own probe lands, and test that check
  against `testdata/disc-probe` and its `MANIFEST.tsv`. Android's is `rust/fbandroid/src/ps1probe.rs`
  in fruit-basket-android.
