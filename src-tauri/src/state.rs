use std::path::PathBuf;

use vtl_core::audio::{Player, Recorder};
use vtl_core::config::AppConfig;
use vtl_core::engine as engine_mod;
use vtl_core::history::HistoryItem;

pub struct AppState {
    pub(crate) config: AppConfig,
    pub(crate) history: Vec<HistoryItem>,
    pub(crate) history_path: PathBuf,
    pub(crate) recorder: Recorder,
    pub(crate) player: Player,
    pub(crate) engine: Option<Box<dyn engine_mod::Engine>>,
    pub(crate) hotkey_registration: Vec<serde_json::Value>,
}
