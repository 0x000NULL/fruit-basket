//! The game library: which ROMs exist, what they are called, how they look, how long they were
//! played. Backs screen 01 (`screens::library`).
//!
//! * `scan` walks the configured folders on a background thread, cheapest first (the index cache
//!   answers for unchanged files; only new or changed ROMs are read).
//! * `index` is the TOML cache (`<cache_dir>/index.toml`): scan results plus play statistics.
//!   A warm start shows the whole case from it before the scan has confirmed anything.
//! * `systems` is its sidecar (`<cache_dir>/systems.toml`): the system tag
//!   [`Platform::probe_system`] gave each ROM, which the library tabs filter on.
//! * `capture` finds label art: newest save-state frame, else a cached title capture, else none;
//!   missing captures are made by one background thread running the ROM headless.
//!
//! Everything that knows the console goes through [`Platform`]: which files are ROMs, what a ROM
//! file says about itself, its save-state picture and its title screen.
//! * `names` turns file names into titles and subtitles.
//!
//! The owner calls [`Library::poll`] once per frame and [`Library::record_played`] when a game
//! ends.
//!
//! This module is wired in by the integrator; until then most of it is unused.
#![allow(dead_code)]

pub mod capture;
pub mod index;
pub mod names;
pub mod scan;
pub mod systems;

pub use capture::Art;

use index::Index;
use systems::Systems;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// One ROM file as the library knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEntry {
    pub path: PathBuf,
    /// File size in bytes (index key).
    pub size: u64,
    /// File modification time, unix seconds (index key).
    pub mtime: i64,
    pub title: String,
    pub subtitle: String,
    /// Four-character game code from the header, e.g. `B8CE`; empty when unreadable.
    pub code: String,
    pub version: u8,
    /// The platform's save memory id (`"none"` when the cartridge has none), stored as is in the
    /// index.
    pub save: String,
    /// Length of the ROM image itself (after unzipping).
    pub rom_len: u64,
    /// Why the ROM could not be read, if it could not.
    pub unreadable: Option<String>,
}

impl LibraryEntry {
    /// The file name, for unreadable carts and the details list.
    pub fn file_name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    fn sort_key(&self) -> (String, String, PathBuf) {
        (self.title.to_lowercase(), self.subtitle.to_lowercase(), self.path.clone())
    }
}

/// What [`Platform::probe`] learns from one ROM file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RomInfo {
    /// Length of the ROM image (after unzipping), also when it is unreadable.
    pub rom_len: u64,
    /// Short game code shown on the label; empty if the ROM has none.
    pub code: String,
    pub version: u8,
    /// Save memory id, see [`LibraryEntry::save`].
    pub save: String,
    /// The title stored in the ROM, used when the file name gives none.
    pub header_title: String,
    /// Why the file cannot be played; the other fields are then ignored except `rom_len`.
    pub unreadable: Option<String>,
}

/// The console-specific half of the library. Every method runs on a background thread.
pub trait Platform: Send + Sync + 'static {
    /// File extensions the library lists, lowercase and without the dot.
    fn extensions(&self) -> &'static [&'static str];
    /// Describe one ROM file. A panic counts as "could not be decoded".
    fn probe(&self, path: &Path) -> RomInfo;
    /// [`Platform::probe`] plus the system the ROM is for, for a fruit that plays more than one
    /// (`"gbc"` for a Game Boy Color game): a short tag of the fruit's choosing, `""` for none.
    /// The library keeps it per file ([`Library::system_of`]) and the library tabs filter on it
    /// ([`Library::systems`], [`Library::entries_in`]). The library calls this, not `probe`;
    /// the default tags nothing.
    fn probe_system(&self, path: &Path) -> (RomInfo, String) {
        (self.probe(path), String::new())
    }
    /// The picture of the newest loadable save state, if the game has one.
    fn slot_art(&self, rom: &Path) -> Option<Art>;
    /// Run the game headless and save its title screen as a PNG at `out`.
    fn capture_title(&self, rom: &Path, out: &Path) -> anyhow::Result<Art>;
}

pub struct Library {
    platform: Option<Arc<dyn Platform>>,
    folders: Vec<PathBuf>,
    cache_dir: Option<PathBuf>,
    entries: Vec<LibraryEntry>,
    index: Index,
    /// Each ROM's system tag (`systems.toml`).
    systems: Systems,
    scan_rx: Option<Receiver<scan::TaggedMsg>>,
    scanning: bool,
    art_tx: Option<Sender<capture::ArtJob>>,
    art_rx: Option<Receiver<capture::ArtMsg>>,
    arts: HashMap<PathBuf, Art>,
    /// Games already handed to the art thread.
    queued: HashSet<PathBuf>,
    /// Bumped whenever entries, art or statistics change (screens can cache on it).
    revision: u64,
    /// Entries seen by the running scan, to drop stale warm-start rows when it finishes.
    seen_this_scan: HashSet<PathBuf>,
    /// Cached `index.last_played(entries)`; screens ask for it every frame.
    last_played: Option<PathBuf>,
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Library {
    /// Start a library over `folders`. `cache_dir` None = no index file and no title captures
    /// (tests). The scan and the art thread start immediately.
    pub fn new(folders: Vec<PathBuf>, cache_dir: Option<PathBuf>, platform: Arc<dyn Platform>) -> Library {
        let mut lib = Library::bare(folders, cache_dir);
        lib.platform = Some(platform);
        lib.start_art_thread();
        lib.rescan();
        lib
    }

    /// A library with fixed entries and no threads (screen tests); it never scans.
    pub fn from_entries(folders: Vec<PathBuf>, entries: Vec<LibraryEntry>) -> Library {
        let mut lib = Library::bare(folders, None);
        for e in &entries {
            lib.index.upsert(e);
        }
        lib.entries = entries;
        lib.sort();
        lib.refresh_last_played();
        lib
    }

    fn bare(folders: Vec<PathBuf>, cache_dir: Option<PathBuf>) -> Library {
        let index = cache_dir.as_deref().map(Index::load).unwrap_or_default();
        let systems = cache_dir.as_deref().map(Systems::load).unwrap_or_default();
        let mut lib = Library {
            platform: None,
            folders,
            cache_dir,
            entries: Vec::new(),
            index,
            systems,
            scan_rx: None,
            scanning: false,
            art_tx: None,
            art_rx: None,
            arts: HashMap::new(),
            queued: HashSet::new(),
            revision: 0,
            seen_this_scan: HashSet::new(),
            last_played: None,
        };
        // warm start: whatever the index remembers about files in these folders
        let known: Vec<LibraryEntry> =
            lib.index.entries.iter().map(|ie| ie.to_entry()).filter(|e| e.path.parent().is_some_and(|p| lib.in_folders(p))).collect();
        lib.entries = known;
        lib.sort();
        lib.refresh_last_played();
        lib
    }

    fn refresh_last_played(&mut self) {
        self.last_played = self.index.last_played(self.entries.iter().map(|e| e.path.as_path()));
    }

    fn in_folders(&self, dir: &Path) -> bool {
        self.folders.iter().any(|f| f.as_path() == dir)
    }

    fn start_art_thread(&mut self) {
        let Some(platform) = self.platform.clone() else { return };
        let captures = self.cache_dir.as_deref().map(capture::captures_dir);
        let (tx, rx) = capture::spawn(captures, platform);
        self.art_tx = Some(tx);
        self.art_rx = Some(rx);
        self.queue_art_jobs();
    }

    fn queue_art_jobs(&mut self) {
        let Some(tx) = &self.art_tx else { return };
        for e in &self.entries {
            if e.unreadable.is_some() || self.queued.contains(&e.path) {
                continue;
            }
            self.queued.insert(e.path.clone());
            let _ = tx.send(capture::ArtJob::Resolve { path: e.path.clone(), code: e.code.clone() });
        }
    }

    /// Start (or restart) the folder scan.
    pub fn rescan(&mut self) {
        let Some(platform) = self.platform.clone() else { return };
        self.seen_this_scan.clear();
        self.scanning = true;
        let tagged = self.systems.paths().map(Path::to_path_buf).collect();
        self.scan_rx = Some(scan::spawn_tagged(self.folders.clone(), self.index.entries.clone(), tagged, platform));
    }

    /// Ingest scan and art results. Call once per frame; never blocks.
    pub fn poll(&mut self) {
        let mut changed = false;
        let mut done: Option<Vec<PathBuf>> = None;
        if let Some(rx) = &self.scan_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    scan::TaggedMsg::Entry(e, system) => {
                        self.seen_this_scan.insert(e.path.clone());
                        if let Some(s) = system {
                            changed |= self.systems.set(&e.path, &s);
                        }
                        self.index.upsert(&e);
                        match self.entries.iter_mut().find(|x| x.path == e.path) {
                            Some(slot) => {
                                if *slot != e {
                                    *slot = e;
                                    changed = true;
                                }
                            }
                            None => {
                                self.entries.push(e);
                                changed = true;
                            }
                        }
                    }
                    scan::TaggedMsg::Done { seen } => {
                        done = Some(seen);
                        break;
                    }
                }
            }
        }
        if let Some(seen) = done {
            self.scan_rx = None;
            self.scanning = false;
            let seen: HashSet<PathBuf> = seen.into_iter().collect();
            let before = self.entries.len();
            self.entries.retain(|e| seen.contains(&e.path));
            changed |= self.entries.len() != before;
            // rows for files that vanished from the scanned folders go too; stubs for games played
            // from elsewhere (not under a configured folder) are kept
            let folders = self.folders.clone();
            self.index.entries.retain(|ie| {
                let p = Path::new(&ie.path);
                seen.contains(p) || !p.parent().is_some_and(|d| folders.iter().any(|f| f.as_path() == d))
            });
            let index = &self.index;
            self.systems.retain(|p| index.get(p).is_some());
            self.save_index();
            self.save_systems();
        }
        if changed {
            self.refresh_last_played();
            self.sort();
            self.queue_art_jobs();
            self.revision += 1;
        }
        let mut arts = Vec::new();
        if let Some(rx) = &self.art_rx {
            while let Ok(m) = rx.try_recv() {
                arts.push(m);
            }
        }
        for m in arts {
            self.arts.insert(m.path, m.art);
            self.revision += 1;
        }
    }

    fn sort(&mut self) {
        self.entries.sort_by_key(|a| a.sort_key());
    }

    fn save_index(&self) {
        if let Some(dir) = &self.cache_dir
            && let Err(e) = self.index.save(dir) {
                eprintln!("warning: library index: {e:#}");
            }
    }

    fn save_systems(&self) {
        if let Some(dir) = &self.cache_dir
            && let Err(e) = self.systems.save(dir) {
                eprintln!("warning: library systems: {e:#}");
            }
    }

    pub fn entries(&self) -> &[LibraryEntry] {
        &self.entries
    }

    /// The system tag [`Platform::probe_system`] gave `path`; `""` when it gave none (always, for
    /// a fruit that does not tag).
    pub fn system_of(&self, path: &Path) -> &str {
        self.systems.get(path)
    }

    /// The distinct system tags among the entries, sorted, without `""`: the library tabs after
    /// "All". Empty for a fruit that does not tag.
    pub fn systems(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.entries.iter().map(|e| self.systems.get(&e.path)).filter(|s| !s.is_empty()).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The entries of one library tab, in [`Library::entries`] order: `None` is every entry,
    /// `Some(tag)` those tagged `tag` (`Some("")`: the untagged ones).
    pub fn entries_in<'a>(&'a self, system: Option<&'a str>) -> impl Iterator<Item = &'a LibraryEntry> + 'a {
        self.entries.iter().filter(move |e| system.is_none_or(|s| self.systems.get(&e.path) == s))
    }

    pub fn entry(&self, path: &Path) -> Option<&LibraryEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    pub fn folders(&self) -> &[PathBuf] {
        &self.folders
    }

    pub fn is_scanning(&self) -> bool {
        self.scanning
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Label art for a game, once the art thread has found or made it.
    pub fn art_for(&self, path: &Path) -> Option<&Art> {
        self.arts.get(path)
    }

    /// Total play time in seconds (0 = never played).
    pub fn played_secs(&self, path: &Path) -> u64 {
        self.index.played_secs(path)
    }

    /// The game most recently played that is still in the library.
    pub fn last_played(&self) -> Option<PathBuf> {
        self.last_played.clone()
    }

    /// Book `secs` of play time against `path` and stamp it as the last played game. Also asks
    /// the art thread to look at the game's save states again.
    pub fn record_played(&mut self, path: &Path, secs: u64) {
        self.index.record_played(path, secs, now_secs());
        self.refresh_last_played();
        self.save_index();
        if let Some(tx) = &self.art_tx {
            let _ = tx.send(capture::ArtJob::Refresh { path: path.to_path_buf() });
        }
        self.revision += 1;
    }

    /// Test hook: set a game's last-played stamp without play time.
    pub fn mark_played_at(&mut self, path: &Path, when: i64) {
        self.index.record_played(path, 0, when);
        self.refresh_last_played();
        self.revision += 1;
    }

    /// Test and render hook: tag a game's system directly (for [`Library::from_entries`]).
    pub fn set_system(&mut self, path: &Path, system: &str) {
        if self.systems.set(path, system) {
            self.revision += 1;
        }
    }

    /// Test and render hook: attach art directly.
    pub fn set_art(&mut self, path: &Path, art: Art) {
        self.arts.insert(path.to_path_buf(), art);
        self.revision += 1;
    }
}

/// A stand-in console for the library's own tests: `.rom` files hold a four-character code then
/// a title; fewer than four bytes is unreadable. `<stem>.art` beside a ROM makes a green slot
/// picture; title captures always fail.
#[cfg(test)]
pub mod fake {
    use super::{Art, Platform, RomInfo};
    use std::path::Path;
    use std::sync::Arc;

    pub struct FakePlatform;

    pub fn platform() -> Arc<dyn Platform> {
        Arc::new(FakePlatform)
    }

    impl Platform for FakePlatform {
        fn extensions(&self) -> &'static [&'static str] {
            &["rom"]
        }
        fn probe(&self, path: &Path) -> RomInfo {
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => return RomInfo { unreadable: Some(e.to_string()), ..RomInfo::default() },
            };
            let rom_len = bytes.len() as u64;
            if bytes.len() < 4 {
                return RomInfo { rom_len, unreadable: Some(format!("only {rom_len} bytes: too small")), ..RomInfo::default() };
            }
            let text = String::from_utf8_lossy(&bytes);
            RomInfo { rom_len, code: text[..4].to_string(), version: 0, save: "battery".into(), header_title: text[4..].to_string(), unreadable: None }
        }
        fn slot_art(&self, rom: &Path) -> Option<Art> {
            rom.with_extension("art").is_file().then(|| Art::new(1, 1, vec![0, 255, 0]))
        }
        fn capture_title(&self, _rom: &Path, _out: &Path) -> anyhow::Result<Art> {
            anyhow::bail!("no title captures in tests")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("basket-lib-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn wait_scan(lib: &mut Library) {
        for _ in 0..500 {
            lib.poll();
            if !lib.is_scanning() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("scan did not finish");
    }

    #[test]
    fn scan_index_and_warm_start() {
        let games = temp_dir("games");
        let cache = temp_dir("cache");
        std::fs::write(games.join("Mario Kart - Super Circuit (USA).rom"), "AMKEMARIOKART").unwrap();
        std::fs::write(games.join("Broken (USA).rom"), [0u8; 2]).unwrap();
        let mut lib = Library::new(vec![games.clone()], Some(cache.clone()), fake::platform());
        wait_scan(&mut lib);
        assert_eq!(lib.entries().len(), 2);
        assert_eq!(lib.entries()[0].title, "Broken (USA).rom", "sorted by title, unreadable by file name");
        let mk = lib.entries().iter().find(|e| e.code == "AMKE").unwrap().path.clone();
        lib.record_played(&mk, 120);
        lib.record_played(&mk, 60);
        assert_eq!(lib.played_secs(&mk), 180);
        assert_eq!(lib.last_played(), Some(mk.clone()));
        assert!(cache.join("index.toml").is_file());
        drop(lib);

        // the second start shows everything immediately, before any scan result is polled
        let lib2 = Library::new(vec![games.clone()], Some(cache.clone()), fake::platform());
        assert_eq!(lib2.entries().len(), 2);
        assert_eq!(lib2.played_secs(&mk), 180);
        assert_eq!(lib2.last_played(), Some(mk.clone()));
        // a different folder does not inherit them
        let other = Library::new(vec![temp_dir("other")], Some(cache.clone()), fake::platform());
        assert!(other.entries().is_empty());

        // deleting a file drops its entry once the scan confirms
        std::fs::remove_file(&mk).unwrap();
        let mut lib3 = Library::new(vec![games.clone()], Some(cache.clone()), fake::platform());
        wait_scan(&mut lib3);
        assert_eq!(lib3.entries().len(), 1);
        assert_eq!(lib3.last_played(), None);
        let _ = std::fs::remove_dir_all(&games);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn no_cache_dir_means_no_files_written() {
        let games = temp_dir("nocache");
        std::fs::write(games.join("A.rom"), "AAAAA").unwrap();
        let mut lib = Library::new(vec![games.clone()], None, fake::platform());
        wait_scan(&mut lib);
        assert_eq!(lib.entries().len(), 1);
        lib.record_played(&lib.entries()[0].path.clone(), 5);
        assert_eq!(std::fs::read_dir(&games).unwrap().count(), 1, "nothing but the ROM");
        let _ = std::fs::remove_dir_all(&games);
    }

    #[test]
    fn slot_art_reaches_the_library() {
        let games = temp_dir("art");
        let path = games.join("A.rom");
        std::fs::write(&path, "AAAAA").unwrap();
        std::fs::write(games.join("A.art"), b"").unwrap();
        let mut lib = Library::new(vec![games.clone()], None, fake::platform());
        let mut got = None;
        for _ in 0..500 {
            lib.poll();
            if let Some(a) = lib.art_for(&path) {
                got = Some(a.rgb[..3].to_vec());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(got, Some(vec![0, 255, 0]));
        let _ = std::fs::remove_dir_all(&games);
    }

    /// [`fake::FakePlatform`] that tags a ROM `"color"` when its code starts with `C`, and counts
    /// the files it reads.
    struct Tagging(std::sync::atomic::AtomicUsize);

    impl Platform for Tagging {
        fn extensions(&self) -> &'static [&'static str] {
            fake::FakePlatform.extensions()
        }
        fn probe(&self, path: &Path) -> RomInfo {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            fake::FakePlatform.probe(path)
        }
        fn probe_system(&self, path: &Path) -> (RomInfo, String) {
            let info = self.probe(path);
            let tag = if info.code.starts_with('C') { "color" } else { "" };
            (info, tag.into())
        }
        fn slot_art(&self, rom: &Path) -> Option<Art> {
            fake::FakePlatform.slot_art(rom)
        }
        fn capture_title(&self, rom: &Path, out: &Path) -> anyhow::Result<Art> {
            fake::FakePlatform.capture_title(rom, out)
        }
    }

    fn tagging() -> Arc<Tagging> {
        Arc::new(Tagging(std::sync::atomic::AtomicUsize::new(0)))
    }

    fn titles<'a>(it: impl Iterator<Item = &'a LibraryEntry>) -> Vec<&'a str> {
        it.map(|e| e.title.as_str()).collect()
    }

    #[test]
    fn system_tags_reach_the_tabs_and_survive_a_warm_start() {
        let games = temp_dir("tags");
        let cache = temp_dir("tags-cache");
        std::fs::write(games.join("Tetris (World).rom"), "DMGTTETRIS").unwrap();
        std::fs::write(games.join("Zelda DX (USA).rom"), "CZ7EZELDA").unwrap();
        std::fs::write(games.join("Bad.rom"), [0u8; 1]).unwrap();
        let p = tagging();
        let mut lib = Library::new(vec![games.clone()], Some(cache.clone()), p.clone());
        wait_scan(&mut lib);
        let zelda = games.join("Zelda DX (USA).rom");
        assert_eq!(lib.system_of(&zelda), "color");
        assert_eq!(lib.system_of(&games.join("Tetris (World).rom")), "");
        assert_eq!(lib.systems(), vec!["color"]);
        assert_eq!(titles(lib.entries_in(None)), vec!["Bad.rom", "Tetris", "Zelda DX"]);
        assert_eq!(titles(lib.entries_in(Some("color"))), vec!["Zelda DX"]);
        assert_eq!(titles(lib.entries_in(Some(""))), vec!["Bad.rom", "Tetris"]);
        assert!(cache.join(systems::FILE_NAME).is_file());
        assert_eq!(p.0.load(std::sync::atomic::Ordering::SeqCst), 3);
        drop(lib);

        // the warm start has the tags before the scan, and the scan reads nothing
        let p2 = tagging();
        let mut lib2 = Library::new(vec![games.clone()], Some(cache.clone()), p2.clone());
        assert_eq!(lib2.systems(), vec!["color"]);
        assert_eq!(titles(lib2.entries_in(Some("color"))), vec!["Zelda DX"]);
        wait_scan(&mut lib2);
        assert_eq!(p2.0.load(std::sync::atomic::Ordering::SeqCst), 0, "every file was an index hit");
        assert_eq!(lib2.system_of(&zelda), "color");

        // a deleted ROM loses its row
        std::fs::remove_file(&zelda).unwrap();
        let mut lib3 = Library::new(vec![games.clone()], Some(cache.clone()), tagging());
        wait_scan(&mut lib3);
        assert!(lib3.systems().is_empty());
        assert!(!Systems::load(&cache).knows(&zelda));
        let _ = std::fs::remove_dir_all(&games);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn an_index_from_before_tags_is_read_once_more() {
        let games = temp_dir("upgrade");
        let cache = temp_dir("upgrade-cache");
        std::fs::write(games.join("Zelda DX (USA).rom"), "CZ7EZELDA").unwrap();
        // a v0.4.0 library: index.toml only
        let mut old = Library::new(vec![games.clone()], Some(cache.clone()), fake::platform());
        wait_scan(&mut old);
        drop(old);
        std::fs::remove_file(cache.join(systems::FILE_NAME)).unwrap();
        let p = tagging();
        let mut lib = Library::new(vec![games.clone()], Some(cache.clone()), p.clone());
        wait_scan(&mut lib);
        assert_eq!(p.0.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(lib.systems(), vec!["color"]);
        let _ = std::fs::remove_dir_all(&games);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn a_fruit_that_does_not_tag_has_one_tab() {
        let games = temp_dir("untagged");
        std::fs::write(games.join("A.rom"), "CAAAA").unwrap();
        let mut lib = Library::new(vec![games.clone()], None, fake::platform());
        wait_scan(&mut lib);
        assert_eq!(lib.system_of(&games.join("A.rom")), "");
        assert!(lib.systems().is_empty());
        assert_eq!(lib.entries_in(None).count(), 1);
        assert_eq!(lib.entries_in(Some("")).count(), 1);
        let _ = std::fs::remove_dir_all(&games);
    }

    #[test]
    fn fixed_libraries_take_tags_by_hand() {
        let entry = |name: &str| LibraryEntry {
            path: PathBuf::from(format!("Games/{name}.rom")),
            size: 1,
            mtime: 0,
            title: name.into(),
            subtitle: String::new(),
            code: String::new(),
            version: 0,
            save: "none".into(),
            rom_len: 1,
            unreadable: None,
        };
        let mut lib = Library::from_entries(vec![PathBuf::from("Games")], vec![entry("B"), entry("A")]);
        let rev = lib.revision();
        lib.set_system(Path::new("Games/B.rom"), "gbc");
        assert!(lib.revision() > rev);
        assert_eq!(lib.systems(), vec!["gbc"]);
        assert_eq!(titles(lib.entries_in(Some("gbc"))), vec!["B"]);
        assert_eq!(titles(lib.entries_in(None)), vec!["A", "B"]);
    }

    #[test]
    fn a_fixed_library_never_scans() {
        let mut lib = Library::from_entries(vec![PathBuf::from("Games")], Vec::new());
        lib.rescan();
        assert!(!lib.is_scanning());
    }
}
