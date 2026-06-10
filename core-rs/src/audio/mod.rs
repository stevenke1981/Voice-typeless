//! Audio capture, playback, and device enumeration.
//!
//! This module provides the concrete `Recorder`, `Player`, and `Enumerator`
//! implementations backed by [cpal], along with a simple energy-based VAD.

pub mod enumerator;
pub mod player;
pub mod recorder;
pub mod traits;
pub mod types;
pub mod vad;

pub use enumerator::Enumerator;
pub use player::Player;
pub use recorder::Recorder;
pub use traits::*;
pub use types::*;
pub use vad::is_speech;
