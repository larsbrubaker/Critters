//! Persisted player settings — the original's two `localStorage` keys
//! (`critterstack-best-score-v2`, `critterstack-muted`). The core only knows
//! the [`SettingsStore`] trait; `critters-wasm` implements it on
//! `localStorage` with the original keys and `critters-native` on a small
//! text file, so a best score survives restarts on both targets.

use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub best_score: u32,
    pub muted: bool,
}

impl Settings {
    /// Serialize as `key=value` lines (the native file format).
    pub fn to_text(&self) -> String {
        format!(
            "best_score={}\nmuted={}\n",
            self.best_score,
            if self.muted { 1 } else { 0 }
        )
    }

    /// Parse `key=value` lines; unknown keys are ignored, bad values default.
    pub fn from_text(text: &str) -> Self {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "best_score" => s.best_score = v.trim().parse().unwrap_or(0),
                "muted" => s.muted = v.trim() == "1",
                _ => {}
            }
        }
        s
    }
}

pub trait SettingsStore {
    fn load(&self) -> Settings;
    fn save(&self, settings: &Settings);
}

/// Keeps settings for the session only (tests, or when storage is unavailable).
#[derive(Default)]
pub struct InMemorySettingsStore {
    settings: Mutex<Settings>,
}

impl SettingsStore for InMemorySettingsStore {
    fn load(&self) -> Settings {
        self.settings.lock().map(|s| *s).unwrap_or_default()
    }
    fn save(&self, settings: &Settings) {
        if let Ok(mut s) = self.settings.lock() {
            *s = *settings;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips() {
        let s = Settings {
            best_score: 12345,
            muted: true,
        };
        assert_eq!(Settings::from_text(&s.to_text()), s);
        assert_eq!(
            Settings::from_text("garbage\nmuted=0\nbest_score=x"),
            Settings::default()
        );
    }

    #[test]
    fn in_memory_store_keeps_the_last_save() {
        let store = InMemorySettingsStore::default();
        assert_eq!(store.load(), Settings::default());
        store.save(&Settings {
            best_score: 7,
            muted: false,
        });
        assert_eq!(store.load().best_score, 7);
    }
}
