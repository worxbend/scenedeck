/// Stable identifier for an OBS audio input.  Matches `inputName` in the
/// WebSocket protocol; OBS guarantees uniqueness within a scene collection.
pub type InputId = String;

#[derive(Debug, Clone)]
pub struct AudioInput {
    pub id: InputId,
    pub name: String,
    pub muted: bool,
    /// Linear gain multiplier (1.0 = 0 dB, 0.0 = silence).
    pub volume_mul: f64,
    /// Gain in decibels.
    pub volume_db: f64,
}
