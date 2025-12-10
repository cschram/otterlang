use crate::CodegenOptLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get current time in milliseconds since epoch
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compilation tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompilationTier {
    /// Tier 1: Quick compilation with minimal optimizations
    /// Used for initial compilation to reduce startup time
    Quick = 1,

    /// Tier 2: Balanced optimization for warm functions
    /// Applied when functions show moderate activity
    Optimized = 2,

    /// Tier 3: Aggressive optimization for hot functions
    /// Maximum optimization for frequently executed code
    Aggressive = 3,
}

impl CompilationTier {
    /// Convert tier to codegen optimization level
    pub fn to_opt_level(self) -> CodegenOptLevel {
        match self {
            CompilationTier::Quick => CodegenOptLevel::None,
            CompilationTier::Optimized => CodegenOptLevel::Default,
            CompilationTier::Aggressive => CodegenOptLevel::Aggressive,
        }
    }

    /// Get the next higher tier, if any
    pub fn next_tier(self) -> Option<CompilationTier> {
        match self {
            CompilationTier::Quick => Some(CompilationTier::Optimized),
            CompilationTier::Optimized => Some(CompilationTier::Aggressive),
            CompilationTier::Aggressive => None,
        }
    }

    /// Get tier name for display
    pub fn name(self) -> &'static str {
        match self {
            CompilationTier::Quick => "Quick",
            CompilationTier::Optimized => "Optimized",
            CompilationTier::Aggressive => "Aggressive",
        }
    }
}

/// Metadata about a compiled function
#[derive(Debug, Clone)]
pub struct FunctionTierInfo {
    /// Current compilation tier
    pub tier: CompilationTier,

    /// Number of times this function has been called
    pub call_count: u64,

    /// Number of times this function has been recompiled
    pub recompilation_count: u32,

    /// Timestamp of last compilation (milliseconds since epoch)
    pub last_compiled_ms: u64,

    /// Total time spent compiling this function (microseconds)
    pub total_compilation_time_us: u64,
}

impl FunctionTierInfo {
    pub fn new(tier: CompilationTier) -> Self {
        Self {
            tier,
            call_count: 0,
            recompilation_count: 0,
            last_compiled_ms: current_time_ms(),
            total_compilation_time_us: 0,
        }
    }

    /// Check if this function should be promoted to the next tier
    pub fn should_promote(&self, config: &TieredConfig) -> bool {
        if !config.enabled {
            return false;
        }

        // Check if we're at max tier
        if self.tier.next_tier().is_none() {
            return false;
        }

        // Check cooldown period
        let elapsed_ms = current_time_ms().saturating_sub(self.last_compiled_ms);
        if elapsed_ms < config.recompilation_cooldown_ms {
            return false;
        }

        // Check if call count exceeds threshold
        if let Some(threshold) = config.threshold_for_tier(self.tier) {
            self.call_count >= threshold
        } else {
            false
        }
    }

    /// Record a function call
    pub fn record_call(&mut self) {
        self.call_count += 1;
    }

    /// Record a recompilation
    pub fn record_recompilation(&mut self, compilation_time_us: u64) {
        self.recompilation_count += 1;
        self.last_compiled_ms = current_time_ms();
        self.total_compilation_time_us += compilation_time_us;
    }

    /// Promote to the next tier
    pub fn promote(&mut self) -> Option<CompilationTier> {
        if let Some(next_tier) = self.tier.next_tier() {
            self.tier = next_tier;
            Some(next_tier)
        } else {
            None
        }
    }
}

/// Statistics about tiered compilation
#[derive(Debug, Clone, Default)]
pub struct TieredStats {
    /// Number of functions at each tier
    pub functions_per_tier: HashMap<CompilationTier, usize>,

    /// Total number of tier promotions
    pub total_promotions: u64,

    /// Total compilation time per tier (microseconds)
    pub compilation_time_per_tier: HashMap<CompilationTier, u64>,

    /// Total number of recompilations
    pub total_recompilations: u64,
}

impl TieredStats {
    pub fn new() -> Self {
        let mut stats = Self::default();
        stats.functions_per_tier.insert(CompilationTier::Quick, 0);
        stats
            .functions_per_tier
            .insert(CompilationTier::Optimized, 0);
        stats
            .functions_per_tier
            .insert(CompilationTier::Aggressive, 0);
        stats
            .compilation_time_per_tier
            .insert(CompilationTier::Quick, 0);
        stats
            .compilation_time_per_tier
            .insert(CompilationTier::Optimized, 0);
        stats
            .compilation_time_per_tier
            .insert(CompilationTier::Aggressive, 0);
        stats
    }

    /// Get total number of functions
    pub fn total_functions(&self) -> usize {
        self.functions_per_tier.values().sum()
    }

    /// Get average compilation time for a tier (microseconds)
    pub fn avg_compilation_time(&self, tier: CompilationTier) -> f64 {
        let count = *self.functions_per_tier.get(&tier).unwrap_or(&0);
        if count == 0 {
            return 0.0;
        }
        let total = *self.compilation_time_per_tier.get(&tier).unwrap_or(&0);
        total as f64 / count as f64
    }
}

/// Configuration for tiered compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredConfig {
    /// Call count threshold to promote from Quick to Optimized
    pub quick_to_optimized_threshold: u64,

    /// Call count threshold to promote from Optimized to Aggressive
    pub optimized_to_aggressive_threshold: u64,

    /// Enable tiered compilation (if false, always use Aggressive)
    pub enabled: bool,

    /// Minimum time between recompilations (in milliseconds)
    pub recompilation_cooldown_ms: u64,
}

impl Default for TieredConfig {
    fn default() -> Self {
        Self {
            quick_to_optimized_threshold: 100,
            optimized_to_aggressive_threshold: 1000,
            enabled: true,
            recompilation_cooldown_ms: 100,
        }
    }
}

impl TieredConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("OTTER_TIER_ENABLED") {
            config.enabled = val.parse().unwrap_or(true);
        }

        if let Ok(val) = std::env::var("OTTER_TIER_QUICK_THRESHOLD") {
            config.quick_to_optimized_threshold = val.parse().unwrap_or(100);
        }

        if let Ok(val) = std::env::var("OTTER_TIER_OPTIMIZED_THRESHOLD") {
            config.optimized_to_aggressive_threshold = val.parse().unwrap_or(1000);
        }

        if let Ok(val) = std::env::var("OTTER_TIER_COOLDOWN_MS") {
            config.recompilation_cooldown_ms = val.parse().unwrap_or(100);
        }

        config
    }

    /// Get the threshold for promoting to the next tier
    pub fn threshold_for_tier(&self, current_tier: CompilationTier) -> Option<u64> {
        match current_tier {
            CompilationTier::Quick => Some(self.quick_to_optimized_threshold),
            CompilationTier::Optimized => Some(self.optimized_to_aggressive_threshold),
            CompilationTier::Aggressive => None,
        }
    }
}
