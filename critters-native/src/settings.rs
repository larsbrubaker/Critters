//! Native settings persistence: the best score and mute flag as a small
//! `key=value` file under the OS data directory (`critters/settings.txt`),
//! the counterpart of the web build's `localStorage` keys.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use critters_core::{InMemorySettingsStore, Settings, SettingsStore};

pub struct FileSettingsStore {
    path: PathBuf,
}

impl FileSettingsStore {
    pub fn new() -> Option<Self> {
        let mut p = dirs::data_dir()?;
        p.push("critters");
        let _ = fs::create_dir_all(&p);
        p.push("settings.txt");
        Some(Self { path: p })
    }

    /// The file store, or an in-memory one when no data directory exists.
    pub fn into_shared() -> Arc<dyn SettingsStore> {
        match Self::new() {
            Some(store) => Arc::new(store),
            None => Arc::new(InMemorySettingsStore::default()),
        }
    }
}

impl SettingsStore for FileSettingsStore {
    fn load(&self) -> Settings {
        fs::read_to_string(&self.path)
            .map(|t| Settings::from_text(&t))
            .unwrap_or_default()
    }

    fn save(&self, settings: &Settings) {
        if let Err(err) = fs::write(&self.path, settings.to_text()) {
            eprintln!(
                "critters: could not save settings to {}: {err}",
                self.path.display()
            );
        }
    }
}
