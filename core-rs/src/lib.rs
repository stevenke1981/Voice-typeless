pub mod audio;
pub mod config;
pub mod engine;
pub mod history;
pub mod hotkey;
pub mod paste;
pub mod processor;

// hound is used only by integration tests (sense_voice_integration.rs)
// but is visible to the lib when compiled with --test via [dev-dependencies].
#[cfg(test)]
#[allow(unused_imports)]
use hound as _;
