/// A simple feeder model that aggregates device power into net load.
///
/// Net load convention:
/// - Positive values increase feeder load (consumption)
/// - Negative values reduce feeder load (generation)
#[derive(Debug, Clone)]
pub struct Feeder {
    name: &'static str,
    net_kw: f32,
    max_import_kw: f32,
    max_export_kw: f32,
}

impl Feeder {
    /// Creates a new feeder with no power limits.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            net_kw: 0.0,
            max_import_kw: f32::INFINITY,
            max_export_kw: f32::INFINITY,
        }
    }

    /// Creates a new feeder with import and export power limits.
    ///
    /// # Panics
    ///
    /// Panics if `max_import_kw` or `max_export_kw` is negative.
    pub fn with_limits(name: &'static str, max_import_kw: f32, max_export_kw: f32) -> Self {
        assert!(max_import_kw >= 0.0);
        assert!(max_export_kw >= 0.0);

        Self {
            name,
            net_kw: 0.0,
            max_import_kw,
            max_export_kw,
        }
    }

    /// Resets accumulated net load to zero.
    pub fn reset(&mut self) {
        self.net_kw = 0.0;
    }

    /// Adds a signed contribution to feeder net load.
    pub fn add_net_kw(&mut self, kw: f32) {
        self.net_kw += kw;
    }

    /// Returns the current net load in kW.
    pub fn net_kw(&self) -> f32 {
        self.net_kw
    }

    /// Returns the maximum import (positive load) limit in kW.
    pub fn max_import_kw(&self) -> f32 {
        self.max_import_kw
    }

    /// Returns the maximum export (negative load) limit in kW.
    pub fn max_export_kw(&self) -> f32 {
        self.max_export_kw
    }

    /// Returns the minimum net load (negated export limit).
    pub fn min_net_kw(&self) -> f32 {
        -self.max_export_kw
    }

    /// Returns the maximum net load (import limit).
    pub fn max_net_kw(&self) -> f32 {
        self.max_import_kw
    }

    /// Returns `true` when net load is within import/export limits.
    pub fn within_limits(&self) -> bool {
        self.net_kw >= self.min_net_kw() && self.net_kw <= self.max_net_kw()
    }

    /// Returns the feeder name.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feeder_defaults() {
        let feeder = Feeder::new("FeederA");
        assert_eq!(feeder.name(), "FeederA");
        assert_eq!(feeder.net_kw(), 0.0);
        assert_eq!(feeder.max_import_kw(), f32::INFINITY);
        assert_eq!(feeder.max_export_kw(), f32::INFINITY);
    }

    #[test]
    fn test_with_limits() {
        let feeder = Feeder::with_limits("FeederA", 5.0, 3.0);
        assert_eq!(feeder.max_import_kw(), 5.0);
        assert_eq!(feeder.max_export_kw(), 3.0);
        assert_eq!(feeder.min_net_kw(), -3.0);
        assert_eq!(feeder.max_net_kw(), 5.0);
    }

    #[test]
    fn test_aggregate_net_kw() {
        let mut feeder = Feeder::new("FeederA");
        feeder.add_net_kw(3.5); // load
        feeder.add_net_kw(-1.0); // generation
        feeder.add_net_kw(0.5);
        assert!((feeder.net_kw() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_reset_clears_net_kw() {
        let mut feeder = Feeder::new("FeederA");
        feeder.add_net_kw(2.0);
        feeder.reset();
        assert_eq!(feeder.net_kw(), 0.0);
    }

    #[test]
    fn test_within_limits() {
        let mut feeder = Feeder::with_limits("FeederA", 4.0, 2.0);
        feeder.add_net_kw(3.5);
        assert!(feeder.within_limits());

        feeder.add_net_kw(1.0);
        assert!(!feeder.within_limits());

        feeder.reset();
        feeder.add_net_kw(-2.5);
        assert!(!feeder.within_limits());
    }
}
