# Releasing

How a change that touches the shared crates reaches players. The same order holds for a
dependency refresh across every repo ([DEPS.md](DEPS.md)) and for a single fruit-basket feature
that a fruit or the launcher needs.

## The order

```
1. fruit-basket        tests green → signed tag vX.Y.Z → tag CI green
        │
        ▼
2. consumers           Strawberry, Crabapple and the launcher re-pin to vX.Y.Z,
                       fix the fallout, tests green → release on GitHub
        │
        ▼
3. the site            mirror each release, edit its notes, update LAUNCHER and oldest
                       → build the feed → launcher e2e against the local feed
                       → sign the feed with the real key → site tests
        │
        ▼
4. deploy              upload → download the live feed and verify its signature
```

Each step starts only when the one before it is done. A consumer never pins an untagged
fruit-basket commit, and the site never mirrors a release that is not on GitHub.

### 1. fruit-basket

1. On `main`: `cargo test --workspace` (this includes the `tokens.json` sync tests) and
   `cargo clippy --workspace --all-targets`, with no new warnings.
2. Bump `version` in the workspace `Cargo.toml` and in `basket-app`'s `basket-ui` dependency, and
   the tags in the README's "Using it" block. Add the release to `CHANGELOG.md`, listing every
   breaking change an app has to fix when it re-pins.
3. Push `main` and wait for CI to pass: the tests on stable and on the workspace `rust-version`.
4. Create a signed, annotated tag (`git tag -s vX.Y.Z -m "fruit-basket X.Y.Z"`), push it, and
   wait for the tag's CI run to pass too.

Commits and tags in this repo are signed. Tags are never moved or reused: a broken release gets a
new patch version.

### 2. Consumers

For each of Strawberry, Crabapple and the launcher:

1. Change every `tag = "vOLD"` for `basket-ui`, `basket-app` and `basket-build` to the new tag,
   including the `dev-dependencies` entry with the `testing` feature.
2. Move the repo's own shared dependencies to the versions in [DEPS.md](DEPS.md), so no crate is
   built at two majors.
3. Fix the fallout using the new tag's `CHANGELOG.md` entry, and run the repo's full test suite.
4. Release it on GitHub the usual way for that repo, with release notes.

Pomegranate does not use the crates. It joins a coordinated refresh only for its own
dependencies.

### 3. The site

For each new release:

1. Mirror it from GitHub into the fruit's `releases/<tag>/`, checking each asset against its
   published SHA-256.
2. Write its line in the fruit's `CHANGELOG.txt` by hand from the release notes. The feed's
   `notes` and the launcher's update prompt come from that line.
3. If the release adds a flag the templates should use, update the fruit's `LAUNCHER` and raise
   `oldest` to that release (see [LAUNCHER-CONTRACT.md](LAUNCHER-CONTRACT.md#oldest)).
4. Update the copy: the fruit's `README`, and the versions on the fruit-basket index pages.

Then the gates, in this order:

1. Build the feed and the site locally. The generator refuses an inconsistent feed: a missing
   asset, a hash that disagrees with its `.sha256`, a growing fruit with builds, a released fruit
   without, or a stable release older than `oldest`.
2. Serve the built site locally, sign the local feed with a throwaway key, and run the launcher's
   end-to-end test against it (a debug launcher build reads `FRUITBASKET_FEED` and
   `FRUITBASKET_KEY`). It fetches and verifies the feed, installs a fruit, switches channel, rolls
   back and forward, and refuses a wrong hash.
3. Only then sign `feed.json` with the real key and verify the signature.
4. Run the site's own tests.

> **Mirroring a launcher release auto-updates every launcher from v1.0.0 on.** The newest
> release in the site's `launcher/` directory becomes the feed's `launcher` entry, and every
> v1.0.0+ install downloads it, checks it and offers Restart now. Mirror a launcher release only
> when it is meant to go out to everyone. Releases from step 2 can sit on GitHub until then.

### 4. Deploy

Rebuild and re-sign right before deploying, so the feed's `generated` time is the publish time.
The launcher rejects a feed older than one it has already accepted, so a feed must never go out
with an earlier `generated` than the live one. Upload the site, then download the live
`feed.json` and `feed.json.minisig` and verify them against the public key (key ID
`9A7C56F99E6460E9`). Check that the new builds are listed with the right SHA-256.
