//! Device simulation components for power system modeling.

/// Residential base-load profile generator.
pub mod baseload;
/// Stationary battery storage model.
pub mod battery;
/// Electric vehicle charger model.
pub mod ev_charger;
/// Solar photovoltaic generation model.
pub mod solar;
/// Solar PV with AR(1) cloud variability model.
pub mod solar_ar1;
pub mod types;

// Re-export the main types for convenience
pub use baseload::BaseLoad;
pub use battery::Battery;
pub use ev_charger::EvCharger;
pub use solar::SolarPv;
pub use solar_ar1::SolarPvAr1;
pub use types::Device;
pub use types::DeviceContext;

/// Solar model selector wrapping independent noise and AR(1) cloud models.
#[derive(Debug, Clone)]
pub enum Solar {
    /// Independent Gaussian noise per timestep.
    Simple(SolarPv),
    /// AR(1) temporally correlated cloud front model.
    Ar1(SolarPvAr1),
}

impl Device for Solar {
    fn power_kw(&mut self, context: &DeviceContext) -> f32 {
        match self {
            Self::Simple(pv) => pv.power_kw(context),
            Self::Ar1(pv) => pv.power_kw(context),
        }
    }

    fn device_type(&self) -> &'static str {
        match self {
            Self::Simple(pv) => pv.device_type(),
            Self::Ar1(pv) => pv.device_type(),
        }
    }
}
