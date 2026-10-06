# The Fruit Basket ecosystem

Fruit Basket is a set of emulators and a recompiler by Ethan Aldrich, one fruit for each console,
plus a launcher that installs, updates and starts them from one window. The fruit is the name of
the program. Consoles whose pixels you can see get a pixel-art fruit; the rest are drawn sharp.

Public site: https://projects.ethanaldrich.net/fruit-basket/

## The fruits

Status is the fruit's `status` in the launcher feed (`released` or `growing`); Grenadine has no
feed entry. Repos marked private are not public on GitHub; their builds are published on the site.
Latest is the newest release on the site as of 2026-10-06.

| No. | Fruit | System | Repo | Binary | Status | Latest |
|---:|---|---|---|---|---|---|
| 1 | Pomegranate | PlayStation 2 | `ps2emu` (private) | `ps2emu` | released | v0.5.0 |
| 2 | Strawberry | Game Boy Advance | `GBA_Emulator` (private) | `strawberry` | released | v1.7.1 |
| 3 | Grenadine | PlayStation 2 static recompiler | `recomp` (private) | `recomp` (a tool, not an emulator) | paused, no build, not in the feed | none |
| 4 | Fig | PlayStation | none yet | none yet | growing | none |
| 5 | Starfruit | Nintendo 64 | none yet | none yet | growing | none |
| 6 | Mangosteen | GameCube | none yet | none yet | growing | none |
| 7 | Crabapple | NES / Famicom | `crabapple` (private) | `crabapple` | released | v0.12.1 |
| 8 | Mulberry | SNES | `mulberry` (private) | `mulberry` | released | v0.3.1 |
| 9 | Olive | Game Boy and Game Boy Color | `olive` (private) | `olive` | released | v0.3.0 |
| 10 | Pear | DS and DSi | none yet | none yet | growing | none |

- **Pomegranate** (No. 1) is a PS2 emulator in C++20 for Windows and Linux. It needs no BIOS
  dump. It does not use the fruit-basket crates. It keeps its saves and settings in a data folder
  (`{data}` in its launch templates).
- **Strawberry** (No. 2) is a GBA emulator in Rust for Windows, macOS and Linux. Its frontend is
  built on `basket-ui`, `basket-app` and `basket-build`.
- **Grenadine** (No. 3) turns a PS2 game's ELF or disc image into a native binary: Pomegranate,
  pressed. The tool and its repo are both called `recomp`. It is paused after milestone M3.4,
  with no build, so it has no `LAUNCHER` file and no feed entry.
- **Crabapple** (No. 7) is an NES and Famicom emulator in Rust for Windows, macOS and Linux,
  built on the fruit-basket crates like Strawberry.
- **Mulberry** (No. 8) is an SNES emulator in Rust for Windows, macOS and Linux (`.sfc`,
  `.smc`), on the fruit-basket crates.
- **Olive** (No. 9) is a Game Boy and Game Boy Color emulator in Rust for Windows, macOS and
  Linux (`.gb`, `.gbc`), on the fruit-basket crates. It is the first user of v0.5.0's rumble
  (MBC5 rumble carts) and library system tags (its All / Game Boy / Color tabs).
- **Growing** fruits are planned. They are listed in the feed with no builds, so the launcher can
  show them and alert when one ripens (is released).

The launcher reads every fruit's details (extensions, launch arguments, cover paths) from the
feed. [LAUNCHER-CONTRACT.md](LAUNCHER-CONTRACT.md) describes them.

## The other pieces

- **The launcher**: repo [`fruit-basket-launcher`](https://github.com/0x000NULL/fruit-basket-launcher)
  (public), binary `fruitbasket`. A Rust app on the fruit-basket crates. It shows the game
  library, installs, updates and rolls back fruits, starts games (with couch mode for a
  controller and a TV), and updates itself from v1.0.0 on.
- **fruit-basket** (this repo, public): `basket-ui`, `basket-app` and `basket-build`, the frontend
  crates the Rust fruits and the launcher share: the design language and software renderer,
  audio, pacing, saves, save-state slots, settings, the library, gamepads, the window icon and
  the Windows exe resources. Apps pin it by git tag.
- **The site and its feed**: https://projects.ethanaldrich.net/fruit-basket/ (its source repo is
  private). Every fruit has a directory with its `README`, `CHANGELOG.txt`, mirrored GitHub
  releases, any nightly build, and a hand-written `LAUNCHER` file. A generator turns those into
  `feed.json`: every fruit, every build with its SHA-256 and size, and the launcher's own newest
  release. The feed is signed with minisign (`feed.json.minisig`, key ID `9A7C56F99E6460E9`).
  The launcher trusts a build only when its hash is in a feed whose signature checks out.
- **Fruit Basket for iOS**: repo `fruit-basket-ios` (private), App Store name "Fruit Basket Emu",
  on TestFlight since 2026-10-04. One SwiftUI app with Strawberry, Crabapple, Olive and Mulberry
  built in: the App Store forbids downloading code, so it does not use the feed, the `LAUNCHER`
  files or the launcher's install flow, and its Basket tab lists the built-in and growing fruits.
  It links each core in-process through a C ABI (`gba-ffi` in GBA_Emulator, `nes-ffi` in
  crabapple, `olive-ffi` in olive, `snes-ffi` in mulberry, each pinned by git rev; the contract is
  the iOS repo's `docs/CORE-ABI.md`). It uses none of the
  fruit-basket crates. Its design comes from the mobile mockups and their token file, a sibling of
  `basket-ui/tokens.json`. No PS2 for now.

## Who depends on what

```
                         fruit-basket (this repo, git tags)
                     basket-ui  ·  basket-app  ·  basket-build
                       │               │                 │
              ┌────────┘               │                 └────────┐
              ▼                        ▼                          ▼
     Strawberry, Crabapple,      (all four fruits)          the launcher
      Mulberry, Olive                                 (fruit-basket-launcher)
              │                        │                          ▲
              │  GitHub releases       │                          │ reads and verifies
              ▼                        ▼                          │ feed.json + .minisig,
   ┌──────────────────────────────────────────────────┐           │ downloads builds
   │ the site: mirrors releases, LAUNCHER files       │───────────┘
   │ → feed.json, signed                              │
   └──────────────────────────────────────────────────┘
              ▲                        ▲
              │ GitHub releases        │ its own GitHub releases
         Pomegranate              the launcher
          (ps2emu,           (the feed's "launcher" entry is
     no fruit-basket crates)   its self-update)
```

- Strawberry, Crabapple, Mulberry, Olive and the launcher depend on all three fruit-basket
  crates, each pinning a tag. After a dependency refresh ([DEPS.md](DEPS.md)) all of them pin the
  same one. A feature release need only be adopted by the apps that use it. As of 2026-10-06 the
  four fruits pin v0.5.0 and the launcher pins v0.4.0: it uses neither rumble nor system tags,
  and v0.5.0 changed no dependency, so both tags share one set of majors.
- Pomegranate and Grenadine are C++ and use none of them.
- The launcher depends on every fruit only through the feed: the `LAUNCHER` contract and each
  fruit's command-line flags and file names. It never links against a fruit.
- The site depends on GitHub releases from every fruit and from the launcher.
- The iOS app depends on the four Rust cores through their `*-ffi` crates, not on this repo. A
  change to a core's save-state format breaks iOS players' states (the app falls back to the
  battery save); a change to an `fb_*` signature needs a CORE-ABI version bump.

[RELEASING.md](RELEASING.md) gives the order a coordinated release goes in, and
[DEPS.md](DEPS.md) the dependency versions the Rust repos share.
