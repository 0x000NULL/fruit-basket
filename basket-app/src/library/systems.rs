//! Which system each ROM is for: `<cache_dir>/systems.toml`.
//!
//! A fruit that plays more than one system (Olive: Game Boy and Game Boy Color) tags each ROM
//! through [`super::Platform::probe_system`], and the library tabs filter on the tag. It sits
//! beside `index.toml`, not in it, so the index file and its types stay as they were. One row per
//! ROM the scan has read, untagged ones too (with `system = ""`): a row tells the scan the file
//! was read by a build that tags, so an index hit with no row here is read once more.

use crate::saves::write_atomic;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "systems.toml";

#[derive(Serialize, Deserialize)]
struct Row {
    path: String,
    #[serde(default)]
    system: String,
}

#[derive(Serialize, Deserialize, Default)]
struct SystemsFile {
    #[serde(default, rename = "rom")]
    rows: Vec<Row>,
}

/// Path to system tag; `""` = untagged.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Systems {
    map: BTreeMap<PathBuf, String>,
}

impl Systems {
    /// Read `<dir>/systems.toml`. A missing or unreadable file is empty: like the index, it is
    /// only an accelerator.
    pub fn load(dir: &Path) -> Systems {
        let path = dir.join(FILE_NAME);
        let Ok(text) = std::fs::read_to_string(&path) else { return Systems::default() };
        match toml::from_str::<SystemsFile>(&text) {
            Ok(f) => Systems { map: f.rows.into_iter().map(|r| (PathBuf::from(r.path), r.system)).collect() },
            Err(e) => {
                eprintln!("warning: ignoring {}: {e}", path.display());
                Systems::default()
            }
        }
    }

    /// Write `<dir>/systems.toml` atomically, rows sorted by path.
    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let rows = self.map.iter().map(|(p, s)| Row { path: p.to_string_lossy().into_owned(), system: s.clone() }).collect();
        let text = toml::to_string_pretty(&SystemsFile { rows }).context("serialising the library systems")?;
        write_atomic(&dir.join(FILE_NAME), text.as_bytes())
    }

    /// The tag of `path`, `""` when untagged or unknown.
    pub fn get(&self, path: &Path) -> &str {
        self.map.get(path).map_or("", String::as_str)
    }

    /// True when the scan has recorded `path` (tagged or not).
    pub fn knows(&self, path: &Path) -> bool {
        self.map.contains_key(path)
    }

    /// Record `path`'s tag. Returns true when that changed the tag it reports.
    pub fn set(&mut self, path: &Path, system: &str) -> bool {
        match self.map.get_mut(path) {
            Some(s) if s == system => false,
            Some(s) => {
                *s = system.to_string();
                true
            }
            None => {
                self.map.insert(path.to_path_buf(), system.to_string());
                !system.is_empty()
            }
        }
    }

    /// Keep only the rows `keep` says yes to.
    pub fn retain(&mut self, mut keep: impl FnMut(&Path) -> bool) {
        self.map.retain(|p, _| keep(p));
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.map.keys().map(PathBuf::as_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("basket-systems-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn pinned_file_and_round_trip() {
        let dir = temp_dir("pin");
        let mut s = Systems::default();
        assert!(s.set(Path::new("C:/Games/Zelda DX.gbc"), "gbc"));
        assert!(!s.set(Path::new("C:/Games/Tetris.gb"), ""), "a new untagged row reports no change");
        assert!(!s.set(Path::new("C:/Games/Zelda DX.gbc"), "gbc"));
        s.save(&dir).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert_eq!(text, "[[rom]]\npath = \"C:/Games/Tetris.gb\"\nsystem = \"\"\n\n[[rom]]\npath = \"C:/Games/Zelda DX.gbc\"\nsystem = \"gbc\"\n");
        let back = Systems::load(&dir);
        assert_eq!(back, s);
        assert_eq!(back.get(Path::new("C:/Games/Zelda DX.gbc")), "gbc");
        assert_eq!(back.get(Path::new("C:/Games/Tetris.gb")), "");
        assert!(back.knows(Path::new("C:/Games/Tetris.gb")));
        assert!(!back.knows(Path::new("C:/Games/Other.gb")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_and_corrupt_files_are_empty() {
        let dir = temp_dir("bad");
        assert_eq!(Systems::load(&dir), Systems::default());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), "[[rom]\nnot toml").unwrap();
        assert_eq!(Systems::load(&dir), Systems::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
