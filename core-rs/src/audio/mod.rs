//! Audio capture, playback, and device enumeration.
//!
//! This module provides the concrete `Recorder`, `Player`, and `Enumerator`
//! implementations backed by [cpal], along with a simple energy-based VAD.

pub mod types;
pub mod traits;
pub mod vad;
pub mod recorder;
pub mod player;
pub mod enumerator;

pub use types::*;
pub use traits::*;
pub use recorder::Recorder;
pub use player::Player;
pub use enumerator::Enumerator;
pub use vad::is_speech;
