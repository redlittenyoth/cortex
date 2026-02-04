//! Compaction configuration.

use serde::{Deserialize, Deserializer, Serialize};

/// Configuration for auto-compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Whether auto-compaction is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Token threshold to trigger compaction (ratio 0.0-1.0 of max context).
    #[serde(default = "default_threshold", deserialize_with = "deserialize_threshold_percent")]
    pub threshold_percent: f32,
    /// Minimum tokens to keep after compaction.
    #[serde(default = "default_min_tokens")]
    pub min_tokens_after: usize,
    /// Maximum tokens for user messages in compaction.
    #[serde(default = "default_max_user_tokens")]
    pub max_user_message_tokens: usize,
    /// Whether to compact tool outputs.
    #[serde(default = "default_true")]
    pub compact_tool_outputs: bool,
    /// Whether to preserve recent turns.
    #[serde(default = "default_preserve_recent")]
    pub preserve_recent_turns: usize,
}

/// Deserialize threshold_percent with validation (must be 0.0-1.0).
fn deserialize_threshold_percent<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = f32::deserialize(deserializer)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(serde::de::Error::custom(
            "threshold_percent must be between 0.0 and 1.0",
        ));
    }
    Ok(value)
}

fn default_true() -> bool {
    true
}

fn default_threshold() -> f32 {
    0.85 // 85% of context
}

fn default_min_tokens() -> usize {
    4000
}

fn default_max_user_tokens() -> usize {
    20000
}

fn default_preserve_recent() -> usize {
    2
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_percent: default_threshold(),
            min_tokens_after: default_min_tokens(),
            max_user_message_tokens: default_max_user_tokens(),
            compact_tool_outputs: true,
            preserve_recent_turns: default_preserve_recent(),
        }
    }
}

impl CompactionConfig {
    /// Check if compaction should be triggered.
    pub fn should_compact(&self, current_tokens: usize, max_tokens: usize) -> bool {
        if !self.enabled {
            return false;
        }
        let threshold = (max_tokens as f32 * self.threshold_percent) as usize;
        current_tokens >= threshold
    }
}
