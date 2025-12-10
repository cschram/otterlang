//! Tiered compilation system for adaptive optimization
//!
//! This module implements a multi-tier compilation strategy that balances
//! compilation time with execution performance. Functions are initially compiled
//! with minimal optimizations and promoted to higher tiers as they become hot.

use otterc_config::{CompilationTier, FunctionTierInfo, TieredConfig, TieredStats};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Tiered compilation manager
pub struct TieredCompiler {
    config: Arc<RwLock<TieredConfig>>,
    function_info: Arc<RwLock<HashMap<String, FunctionTierInfo>>>,
    stats: Arc<RwLock<TieredStats>>,
}

impl TieredCompiler {
    pub fn new() -> Self {
        Self::with_config(TieredConfig::default())
    }

    pub fn with_config(config: TieredConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            function_info: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(TieredStats::new())),
        }
    }

    /// Get the current tier for a function
    pub fn get_tier(&self, function_name: &str) -> CompilationTier {
        self.function_info
            .read()
            .get(function_name)
            .map(|info| info.tier)
            .unwrap_or(CompilationTier::Quick)
    }

    /// Record a function call and check if promotion is needed
    pub fn record_call(&self, function_name: &str) -> Option<CompilationTier> {
        let mut info_map = self.function_info.write();
        let config = self.config.read();

        let info = info_map
            .entry(function_name.to_string())
            .or_insert_with(|| FunctionTierInfo::new(CompilationTier::Quick));

        info.record_call();

        info.should_promote(&config)
            .then(|| info.promote())
            .flatten()
            .inspect(|_| {
                let mut stats = self.stats.write();
                stats.total_promotions += 1;
            })
    }

    /// Register a new function at a specific tier
    pub fn register_function(&self, function_name: &str, tier: CompilationTier) {
        let mut info_map = self.function_info.write();
        info_map.insert(function_name.to_string(), FunctionTierInfo::new(tier));

        // Update stats
        let mut stats = self.stats.write();
        *stats.functions_per_tier.entry(tier).or_insert(0) += 1;
    }

    /// Record a compilation event
    pub fn record_compilation(
        &self,
        function_name: &str,
        tier: CompilationTier,
        compilation_time_us: u64,
    ) {
        let mut info_map = self.function_info.write();
        if let Some(info) = info_map.get_mut(function_name) {
            info.record_recompilation(compilation_time_us);

            // Update stats
            let mut stats = self.stats.write();
            *stats.compilation_time_per_tier.entry(tier).or_insert(0) += compilation_time_us;
            stats.total_recompilations += 1;
        }
    }

    /// Get functions that need recompilation
    pub fn get_functions_to_recompile(&self) -> Vec<(String, CompilationTier)> {
        let info_map = self.function_info.read();
        let config = self.config.read();

        info_map
            .iter()
            .filter_map(|(name, info)| {
                if info.should_promote(&config) {
                    info.tier.next_tier().map(|tier| (name.clone(), tier))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> TieredStats {
        self.stats.read().clone()
    }

    /// Get function info
    pub fn get_function_info(&self, function_name: &str) -> Option<FunctionTierInfo> {
        self.function_info.read().get(function_name).cloned()
    }

    /// Get all function info
    pub fn get_all_function_info(&self) -> HashMap<String, FunctionTierInfo> {
        self.function_info.read().clone()
    }

    /// Update configuration
    pub fn set_config(&self, config: TieredConfig) {
        *self.config.write() = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> TieredConfig {
        self.config.read().clone()
    }

    /// Determine initial tier for a program
    pub fn initial_tier(&self) -> CompilationTier {
        if self.config.read().enabled {
            CompilationTier::Quick
        } else {
            CompilationTier::Aggressive
        }
    }

    /// Check if a function should be compiled at a specific tier
    pub fn should_compile_at_tier(
        &self,
        function_name: &str,
        target_tier: CompilationTier,
    ) -> bool {
        let current_tier = self.get_tier(function_name);
        target_tier > current_tier
    }
}

impl Default for TieredCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_ordering() {
        assert!(CompilationTier::Quick < CompilationTier::Optimized);
        assert!(CompilationTier::Optimized < CompilationTier::Aggressive);
    }

    #[test]
    fn test_tier_promotion() {
        assert_eq!(
            CompilationTier::Quick.next_tier(),
            Some(CompilationTier::Optimized)
        );
        assert_eq!(
            CompilationTier::Optimized.next_tier(),
            Some(CompilationTier::Aggressive)
        );
        assert_eq!(CompilationTier::Aggressive.next_tier(), None);
    }

    #[test]
    fn test_function_promotion() {
        let config = TieredConfig {
            recompilation_cooldown_ms: 0,
            ..TieredConfig::default()
        };
        let compiler = TieredCompiler::with_config(config);
        compiler.register_function("test_fn", CompilationTier::Quick);

        // Call function enough times to trigger promotion
        for _ in 0..100 {
            compiler.record_call("test_fn");
        }

        // Should be promoted to Optimized
        assert_eq!(compiler.get_tier("test_fn"), CompilationTier::Optimized);
    }

    #[test]
    fn test_stats_tracking() {
        let compiler = TieredCompiler::new();
        compiler.register_function("fn1", CompilationTier::Quick);
        compiler.register_function("fn2", CompilationTier::Optimized);

        let stats = compiler.get_stats();
        assert_eq!(stats.functions_per_tier[&CompilationTier::Quick], 1);
        assert_eq!(stats.functions_per_tier[&CompilationTier::Optimized], 1);
    }

    #[test]
    fn test_cooldown_period() {
        let config = TieredConfig {
            recompilation_cooldown_ms: 1_000,
            quick_to_optimized_threshold: 10,
            ..TieredConfig::default()
        };

        let compiler = TieredCompiler::with_config(config);
        compiler.register_function("test_fn", CompilationTier::Quick);

        // Call enough times to exceed threshold
        for _ in 0..20 {
            compiler.record_call("test_fn");
        }

        // Should be promoted (cooldown should have passed in test)
        let info = compiler.get_function_info("test_fn").unwrap();
        assert!(info.call_count >= 10);
    }
}
