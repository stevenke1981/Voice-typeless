use std::path::PathBuf;

use vtl_core::history::HistoryItem;

pub(crate) fn load_history(path: &PathBuf) -> Vec<HistoryItem> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_history(path: &PathBuf, items: &[HistoryItem]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
