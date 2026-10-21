# Shared disc-probe test files

Synthetic disc images and sheets for the rule in
[docs/DISC-PROBE.md](../../docs/DISC-PROBE.md). The launcher, the apps' importers and each
fruit's own probe test against these same files, so their copies of the signature table
can't drift apart.

- **Synthetic only.** Every byte is built by code: ISO9660 headers, a SYSTEM.CNF, the
  GameCube magic, the Sega boot strings, CD sector framing, CHD v5 headers with an
  uncompressed map. Nothing comes from a commercial disc. MIT, like the rest of this repo.
- **Real layouts.** The files mirror real Redump dumps and chdman CHDs, only shorter.
  - Raw tracks carry each data sector's 12-byte sync, BCD address (LBA + 150) and mode,
    with a valid EDC and ECC. PS1 and PS2 discs are mode 2 form 1, with a data
    subheader; Saturn and Dreamcast are mode 1.
  - Sheets are Redump's: `INDEX 01` on every track, and an audio track's file starting
    with its 2-second pregap (`INDEX 00 00:00:00`, `INDEX 01 00:02:00`).
  - The CHDs carry chdman's track metadata, one entry a track.
  - **A probe that needs a quirk of these files to pass has found a fixture bug.** Report
    it, and fix the builder, not the probe.
- **Made by** the launcher's `probe::tests::shared_fixtures()`
  (`DISC_PROBE_OUT=<dir> cargo test write_shared_fixtures -- --ignored` in
  `0x000NULL/fruit-basket-launcher`). The launcher keeps a copy in `tests/data/disc-probe/`
  and tests that it is byte for byte what its builders make. To change a file, change the
  builder there, regenerate, and copy the set here.
- **`MANIFEST.tsv`** lists every file with the systems a probe must find: the feed's
  `system` values (`GameCube`, `PS2`, `PS1`, `PSP`, `Saturn`, `Dreamcast`),
  comma-separated; `none` when the file reads fine but no signature matches; or
  `read-error` when the container can't be read (DISC-PROBE.md rule 5: "couldn't read
  the disc", not "not recognised"). The `none-*` and `error-*` file names match those
  outcomes.

## Using them

Probe each file in the folder and compare the sorted set of systems with its line. Keep the
folder together: sheets name their tracks by relative path.

**PS1, PS2 and Saturn**
- `ps1.cue` is a Redump two-track sheet: `ps1-track1.bin` (data, `MODE2/2352`) and
  `ps1-track2.bin` (audio: 150 pregap sectors, then 2 of audio).
- `ps1-audio-first.cue` puts an AUDIO track first, as a mixed-mode disc does.
  - The probe reads the first *data* track, from its `INDEX 01`.
  - `ps1-audio-first-track2.bin` starts with its 150-sector pregap, so the file on its own
    is `none`. Only the sheet's `INDEX 01 00:02:00` finds the volume.
- `ps1-psx-exe.iso` has no SYSTEM.CNF, only `PSX.EXE` in the root: an early PS1 disc.
- `ps2-cd.bin` is a raw PS2 CD-ROM (mode 2), and `ps2.iso` a cooked one.
- `saturn.cue` reads `saturn-track1.bin`, raw mode 1.

**Dreamcast**
- `dreamcast.cue` is Redump's GD-ROM sheet. Under `REM SINGLE-DENSITY AREA` come track 1
  (data) and track 2 (audio, with its pregap). Under `REM HIGH-DENSITY AREA` comes
  track 3, whose first sector, at LBA 45000 (address 10:02:00), starts IP.BIN with
  `SEGA SEGAKATANA `.
- `dreamcast (Track 1).bin` is an ISO9660 volume with nothing on it (`none`). A Dreamcast
  is found only through track 3: the `.cue`, the `.gdi`, or the track file itself.
- `dreamcast.gdi` describes the same disc. Its audio track starts at `INDEX 01`
  (LBA 20 + 150 = 170), without the pregap, so it reads `dreamcast-track02.raw`. Tracks 1
  and 3 are the same files as the cue's.
- `dreamcast-gd.chd` is the same disc as chdman makes it from those: three `CHGD` tracks,
  with track 2 padded to frame 45000, so track 3 starts at frame 45000. A probe decides it
  from the metadata alone. A core reading it finds IP.BIN in hunk 5625. Zero hunks share
  one stored copy, so the file is small.

**CHDs in general**
- They are v5 with an uncompressed map, so they test the metadata and hunk logic, not the
  codecs.
- The CD CHDs (`ps1-cd.chd`, `saturn-cd.chd`) have one `CHT2` entry a track. The audio
  track's pregap is stored (`PREGAP:150 PGTYPE:VAUDIO`).
- Two CHDs are read errors and must give `read-error`, with no panic:
  - `error-truncated.chd` is cut off inside its map.
  - `error-hunk-cap.chd` has a valid header that claims 2^40 bytes of data, which is
    268 M hunks, past the 4.5 M-hunk cap. A probe must refuse it from the header,
    before the decoder allocates the expanded map, which would be about 3 GB.

**The rest**
- `double-gamecube-ps2.iso` matches two signatures on purpose: a PS2 volume that also
  carries the GameCube magic. A probe must find both. Per the rule, two matches are a
  probe bug and are reported, not settled by order.
- Launcher note: until its UI milestone, the desktop launcher can't report a read error,
  so its own test counts `read-error` as `none`.
- The images are 16-20 sectors, since ISO9660 puts its volume descriptor at sector 16. The
  audio tracks carry a real 150-sector pregap. The set is about 2 MB.
