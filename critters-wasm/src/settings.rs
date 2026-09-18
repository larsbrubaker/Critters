//! Browser settings persistence on `localStorage`, using the original's
//! keys (`critterstack-best-score-v2`, `critterstack-muted`) and value
//! formats so a best score from the JavaScript version carries over when the
//! game is served from the same origin.

use critters_core::config::{BEST_SCORE_KEY, MUTED_KEY};
use critters_core::{Settings, SettingsStore};

pub struct LocalStorageSettingsStore;

impl LocalStorageSettingsStore {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }
}

impl SettingsStore for LocalStorageSettingsStore {
    fn load(&self) -> Settings {
        let Some(ls) = Self::storage() else {
            return Settings::default();
        };
        let best_score = ls
            .get_item(BEST_SCORE_KEY)
            .ok()
            .flatten()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let muted = ls
            .get_item(MUTED_KEY)
            .ok()
            .flatten()
            .map(|s| s == "1")
            .unwrap_or(false);
        Settings { best_score, muted }
    }

    fn save(&self, settings: &Settings) {
        if let Some(ls) = Self::storage() {
            let _ = ls.set_item(BEST_SCORE_KEY, &settings.best_score.to_string());
            let _ = ls.set_item(MUTED_KEY, if settings.muted { "1" } else { "0" });
        }
    }
}
