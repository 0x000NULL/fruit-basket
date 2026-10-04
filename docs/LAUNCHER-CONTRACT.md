# The launcher contract

What a fruit has to provide so the Fruit Basket launcher can install, start and show it. Three
parts:

1. its `LAUNCHER` file on the site, which the feed generator turns into the fruit's entry in
   `feed.json`
2. the command-line flags its emulator takes
3. the names of the files its emulator writes

The launcher is [`fruit-basket-launcher`](https://github.com/0x000NULL/fruit-basket-launcher).
"Launcher version" below is the first launcher release that acts on a key. Older launchers parse
the feed but ignore keys they do not know. The feed never denies unknown fields, so a newer feed
stays readable by an older launcher.

## The LAUNCHER file

One file per fruit, in the fruit's directory on the site. Each line is `key<TAB>value`. Lines
starting with `#`, and blank lines, are skipped. A fruit without a `LAUNCHER` file is left out of
the feed. Grenadine, which has no build, is one.

Lists (`ext`, `archives`, the templates, `carry`, `art`, `slots`) are separated by spaces.
Templates are argument lists: each word becomes one argument after its placeholders are filled.

| Key | Required | Meaning | Placeholders | Launcher version |
|---|---|---|---|---|
| `no` | yes | The fruit's number, a whole number. No two fruits may share one. The feed is sorted by it. | none | v0.1.0 |
| `system` | yes | The console, as shown (`GBA`, `GB · GBC`). | none | v0.1.0 |
| `ext` | yes | The file extensions the fruit plays, with the dot (`.chd .iso`). Matched case-insensitively against the end of the file name. In a fruit's own `games/` folder that is enough; in an extra folder a file goes to the *first* fruit in feed order that lists its extension, so overlapping `ext` lists between fruits (today `.cue .chd .iso` between Pomegranate and the growing Fig and Mangosteen) send the file to the lower-numbered fruit. Resolve overlaps before the second fruit ships. | none | v0.1.0 |
| `status` | yes | `released` or `growing`. A growing fruit must have no builds; a released one must have at least one. | none | v0.1.0 |
| `blurb` | yes | One or two sentences about the fruit's name and mark. | none | v0.1.0 |
| `bin` | when `released` | The emulator's executable name, without `.exe`. | none | v0.2.0 |
| `pixel` | no | `yes` for a pixel-art fruit. Default `no`. | none | v0.1.0 |
| `dump_db` | no | The dump database its games are checked against (`No-Intro`, `Redump`). | none | v0.3.0 |
| `bios` | no | What the fruit needs for a BIOS, as shown (`not needed`, `built in`). | none | v0.1.0 |
| `launch` | no | The arguments that start a game. Default `{rom}`. | `{rom}` `{slot}` `{data}` | v0.3.0 |
| `load_slot` | no | The arguments that start a game from a save slot. Without it, the launcher cannot load saves. | `{rom}` `{slot}` `{data}` | v0.5.0 |
| `open` | no | The arguments that open the fruit with no game. | `{rom}` `{slot}` `{data}` | v0.4.0 |
| `couch` | no | Arguments added after the filled `launch` or `load_slot` when a game starts from couch mode, never after `open`. Since launcher v0.5.1 they go only to builds the feed still lists, so set `oldest` to the first build that takes them. | `{rom}` `{slot}` `{data}` | v0.5.0 |
| `fresh` | no | The arguments Start fresh uses in place of `launch`, for a fruit that would otherwise resume where it left off. Without it, Start fresh uses `launch`. Couch mode adds `couch` after them. | `{rom}` `{slot}` `{data}` | v1.1.2 |
| `carry` | no | Files the emulator keeps beside its exe, moved into each new build so they survive an update. If any template uses `{data}`, they move into the data folder once instead. | none | v0.2.0 (into the data folder from v0.4.0) |
| `art` | no | Where the emulator keeps a game's cover picture: PNG paths, tried in order. Read only as pictures. A path with a placeholder that has nothing to fill it is skipped. | `{rom_dir}` `{stem}` `{data}` `{code}` `{cache}` | v1.1.0 |
| `slots` | no | The lowest and highest save slot the emulator loads, two whole numbers, lowest first (`1 8`). Saves outside them are hidden from the saves lists, covers and Continue. Without it, every slot shows. | none | v1.1.1 |
| `archives` | no | Archive extensions the fruit opens itself (`.zip`), written like `ext`. See [Archives](#archives). | none | v1.2.0 |
| `oldest` | no | The oldest release tag the feed offers. See [oldest](#oldest). | none | site only, not in the feed |

### Placeholders

| Placeholder | Filled with |
|---|---|
| `{rom}` | The full path of the game file. For a game in an archive, the archive itself. |
| `{slot}` | The save slot number, when starting from a slot. |
| `{data}` | The fruit's data folder in the launcher's basket (`<basket>/<fruit>/data`). Any template using it moves the fruit's saves and settings there (launcher v0.4.0 and later). |
| `{rom_dir}` | The folder the game file is in. |
| `{stem}` | The game file's name without its last extension. For a `.zip`, the zip's own stem: `Game (USA).zip` gives `Game (USA)`. |
| `{code}` | The game's serial, when the launcher knows it: a GBA cartridge's header code, a disc serial from the file name, or the serial from the fruit's dump list. |
| `{cache}` | The user's cache directory: `%LOCALAPPDATA%` on Windows, `$XDG_CACHE_HOME` or `~/.cache` on Linux, `~/Library/Caches` on macOS. It is the same directory the fruits' own `dirs::cache_dir()` returns. |

A template placeholder with nothing to fill it is an error: the launcher refuses to start the game
and never passes the placeholder on as text.

### Example

```
# What the Fruit Basket launcher needs; feedgen.py reads it.
no	7
system	NES
pixel	yes
ext	.nes
status	released
bin	crabapple
dump_db	No-Intro
bios	not needed
launch	{rom}
load_slot	{rom} --slot {slot}
fresh	{rom} --no-resume
couch	--fullscreen --exit-on-quit
slots	1 8
art	{cache}/crabapple/covers/{stem}.png
oldest	v0.11.0
blurb	Small, hard and wild: the apple from before the orchard. Five seeds in a star.
```

### oldest

`oldest` is read by the feed generator, not the launcher:

- Releases older than `oldest` stay on the site, but they are left out of the feed's `releases`.
  The launcher can neither install them nor roll back to them.
- If the newest stable release is older than `oldest`, the generator refuses to write the feed.
- The launcher passes the templates to every build the feed lists. So when a release adds a flag
  the templates use (`--slot`, `--no-resume`, `--fullscreen`, `{data}`), raise `oldest` to that
  release in the same change. Otherwise an older listed build gets a flag it does not know.

Versions compare by their numeric parts (`v0.11.0` is newer than `v0.9.0`).

### Archives

A fruit with `archives` (launcher v1.2.0 and later) can be given a game inside an archive:

- A fruit claims an archive only if a file inside it has one of the fruit's `ext`. Several fruits
  can list `.zip`, and a GBA zip and an NES zip in the same folder each go to their own fruit.
- The emulator is handed the archive itself as `{rom}`, and must open it.
- `{stem}` is the archive's own stem, so covers and saves go by the archive's name, not the name
  of the file inside it.
- A fruit without `archives` never takes an archive. Launchers before v1.2.0 ignore the key.

## The feed

`feed.json`, schema 1, is written by the site's `feedgen.py` and signed with minisign
(`feed.json.minisig`, key ID `9A7C56F99E6460E9`). The launcher accepts it only with a good
signature, and only if its `generated` is not older than the last feed it accepted. All fields
are present since launcher v0.1.0 unless the table says otherwise.

### Top level

| Field | Type | Meaning |
|---|---|---|
| `schema` | number | `1`. A launcher refuses any other schema. |
| `generated` | string | UTC time the feed was built, `2026-10-02T12:00:00Z`. |
| `base` | string | URL of the fruit-basket directory on the site. |
| `launcher` | build or null | The launcher's own newest release. Launchers from v1.0.0 on install it as a self-update. |
| `fruits` | array | One entry per fruit with a `LAUNCHER` file, sorted by `no`. |

### A fruit

| Field | Type | From | Launcher version |
|---|---|---|---|
| `id` | string | The fruit's directory name (`strawberry`). | v0.1.0 |
| `no` | number | `no` | v0.1.0 |
| `name` | string | The first line of the fruit's `README`. | v0.1.0 |
| `system` | string | `system` | v0.1.0 |
| `pixel` | bool | `pixel` | v0.1.0 |
| `ext` | string[] | `ext` | v0.1.0 |
| `archives` | string[] | `archives`, empty when unset | v1.2.0 |
| `status` | `"released"` or `"growing"` | `status` | v0.1.0 |
| `summary` | string | The fruit's line in the site's fruit-basket `DESCRIPTIONS`. | v0.1.0 |
| `blurb` | string | `blurb` | v0.1.0 |
| `bin` | string or null | `bin` | v0.2.0 |
| `bios` | string or null | `bios` | v0.1.0 |
| `dump_db` | string or null | `dump_db` | v0.3.0 |
| `launch` | string[] | `launch`, `["{rom}"]` when unset | v0.3.0 |
| `load_slot` | string[] or null | `load_slot` | v0.5.0 |
| `open` | string[] | `open` | v0.4.0 |
| `couch` | string[] | `couch` | v0.5.0 |
| `fresh` | string[] | `fresh` | v1.1.2 |
| `carry` | string[] | `carry` | v0.2.0 |
| `art` | string[] | `art` | v1.1.0 |
| `slots` | string[] | `slots`: two numbers as strings, or empty | v1.1.1 |
| `url` | string | The fruit's directory on the site. | v0.1.0 |
| `readme_url` | string | Its `README`. | v0.1.0 |
| `compat` | file or null | `compat.txt`: title, serial, status, notes per game. | v0.3.0 |
| `dumps` | file or null | `dumps.txt`: SHA-1, serial, title from the fruit's dump database. | v0.3.0 |
| `changelog` | entry[] | The newest 20 lines of `CHANGELOG.txt`. | v0.1.0 |
| `stable` | build or null | The newest release in `releases`. | v0.2.0 |
| `nightly` | build or null | The nightly build, if the fruit has one. | v0.2.0 |
| `releases` | build[] | Every release from `oldest` on, newest first. | v0.2.0 (roll back from v0.4.0) |

### A build

| Field | Type | Meaning |
|---|---|---|
| `build` | string | A release tag (`v1.5.0`) or a nightly's commit. |
| `date` | string or null | `YYYY-MM-DD`. |
| `notes` | string[] | Release notes: the build's changelog line with its lead-in (`Release vX:` / `Nightly build at X`) dropped, split on `; `, parts starting `builds in ` dropped, each part's first letter capitalised. `[]` when no changelog line matches. |
| `notes_url` | string | The release's `README`, or for a nightly the fruit's `CHANGELOG.txt`. |
| `assets` | object | Keyed by platform, `<os>-<arch>` (in use today: `windows-x64`, `macos-arm64`, `macos-x64`, `linux-x64`). Not a whitelist: a release's keys come from its `DESCRIPTIONS` labels (`Windows\|macOS\|Linux <arch>`, lowercased; other labels are not builds), and a nightly's key is its `NIGHTLY` `platform` value as written. Shipping all four platforms for a launcher release is a site process rule, not a feed check. |

An asset is `{name, url, size, sha256}`. The SHA-256 is hashed from the file itself, and the
launcher checks every download against it. A file reference (`compat`, `dumps`) is
`{url, sha256}`, and the launcher uses the file only if it matches. A changelog entry is
`{date, text}`.

## Flags a fruit must support

The templates above use these flags. A fruit that sets the matching key must accept them:

| Flag | Meaning | Used by |
|---|---|---|
| `--slot N` | Start the game from save slot `N`. A slot the emulator cannot load (Crabapple's resume state) is refused, and left outside `slots`. | `load_slot` |
| `--no-resume` | Start the game fresh, without the state the emulator would otherwise resume from. | `fresh` |
| `--fullscreen` | Start in full screen, for that run only; the fruit's own setting is not changed. | `couch` |
| `--exit-on-quit` | Quit in the emulator's pause menu saves as usual and exits the emulator, instead of returning to its own library, so the player lands back in the launcher's couch mode. | `couch` |

A fruit can use other flags in its templates (Pomegranate's `play {rom} --data {data}`), but
these four are the ones the launcher's features rely on. A fruit adds a flag to its templates only
in the release that introduces it, and raises `oldest` with it.

Every template in the feed is handed to every build the feed lists, from `oldest` up, and the
fruits reject unknown flags. So before adding a key whose template uses a new flag, check the flag
exists in every listed build, not just the newest. If the key would only repeat `launch` (Strawberry
never resumes, so its `fresh` would equal `launch`), leave it out rather than raise `oldest`: the
launcher already falls back to `launch`.

## Files a fruit writes

The launcher finds a game's saves and pictures by name, so a fruit names them like this. `<stem>`
is the game file's stem, which for an archive is the archive's own stem. The launcher matches
names case-insensitively.

| File | Where | Meaning |
|---|---|---|
| `<stem>.s<N>.state` | Beside the game, or in the fruit's save folders | Save state in slot `N`. The launcher lists, loads (through `load_slot`) and deletes them. |
| `<stem>.s<N>.png` | Beside the state | The state's picture. The saves lists show it, and the newest save's picture is the game's cover. Deleting a save deletes it too. |
| `{cache}/<app>/covers/<stem>.png` | The user's cache directory | The game's cover, for an `art` entry like `{cache}/crabapple/covers/{stem}.png`. `<app>` is the fruit's own directory name in the cache (`strawberry`, `crabapple`). |

The fruit's save folders, besides the game's own folder, are its data folder when its templates
use `{data}`, else the folder of the build that is installed, each with a `states` subfolder.

`basket-app` names all three for an app that uses it: `slots::SlotFormat::save` writes the state
(`slots::slot_path`) and, when `Snapshot::picture_rgb` gives a picture, the `.png` beside it
(`slots::picture_path`). The library's label-art worker copies each title capture to
`<cache_dir>/covers/<stem>.png`, where `cache_dir` is what the app passes to `Library::new`:
`dirs::cache_dir()` joined with the app's directory name.
