# Shared disc-probe test files

Synthetic disc images and sheets for the rule in
[docs/DISC-PROBE.md](../../docs/DISC-PROBE.md). The launcher, the apps' importers and each
fruit's own probe test against these same files, so their copies of the signature table
can't drift apart.

- **Synthetic only.** Every byte is built by code: ISO9660 headers, a SYSTEM.CNF, the
  GameCube magic, the Sega boot strings, CHD v5 headers with an uncompressed map. Nothing
  comes from a commercial disc. MIT, like the rest of this repo.
- **Made by** the launcher's `probe::tests::shared_fixtures()`
  (`DISC_PROBE_OUT=<dir> cargo test write_shared_fixtures -- --ignored` in
  `0x000NULL/fruit-basket-launcher`). The launcher keeps a copy in `tests/data/disc-probe/`
  and tests that it is byte for byte what its builders make. To change a file, change the
  builder there, regenerate, and copy the set here.
- **`MANIFEST.tsv`** lists every file with the systems a probe must find: the feed's
  `system` values (`GameCube`, `PS2`, `PS1`, `PSP`, `Saturn`, `Dreamcast`),
  comma-separated, or `none`.

## Using them

Probe each file in the folder and compare the sorted set of systems with its line.

- Sheets name their tracks by relative path: `ps1.cue` and `ps1-audio-first.cue` read
  `ps1-track1.bin` and `ps1-track2.bin`; `saturn.cue` reads `saturn-track1.bin`; and
  `dreamcast.cue` and `dreamcast.gdi` read `dreamcast (Track N).bin` (quoted, with
  spaces). Keep the folder together.
- `ps1-audio-first.cue` puts an AUDIO track first: the probe reads the first *data* track.
- `ps1-psx-exe.iso` has no SYSTEM.CNF, only `PSX.EXE` in the root: an early PS1 disc.
- `dreamcast (Track 1).bin` is an ISO9660 volume with nothing on it (`none`). A Dreamcast
  is found only through track 3, as the `.cue`, the `.gdi` or the track itself.
- The CHDs are v5 with an uncompressed map, so they test the metadata and hunk logic, not
  the codecs. `dreamcast-gd.chd` is decided by its `CHGD` metadata alone; its hunk is
  zeros. `none-truncated.chd` is cut off inside its map and must be `none`, with no
  panic.
- `double-gamecube-ps2.iso` matches two signatures on purpose: a PS2 volume that also
  carries the GameCube magic. A probe must find both. Per the rule, two matches are a
  probe bug and are reported, not settled by order.
- The images are 20 sectors (ISO9660 puts its volume descriptor at sector 16), so most
  are 40-50 KB. The set is about 0.9 MB.
