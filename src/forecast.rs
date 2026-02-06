//! Forecasting utilities for the simulator.

/// Naive "tomorrow is today" forecaster.
///
/// This forecast simply copies the provided baseline and repeats/truncates
/// it to match the requested simulation horizon.
#[derive(Debug, Default, Clone, Copy)]
pub struct NaiveForecast;

impl NaiveForecast {
    /// Produce a naive forecast for the given horizon.
    ///
    /// # Arguments
    ///
    /// * `baseline` - Historical or baseline values used as the forecast template
    /// * `horizon` - Number of steps to forecast
    ///
    /// # Returns
    ///
    /// A vector of forecast values with length equal to `horizon`.
    pub fn forecast(&self, baseline: &[f32], horizon: usize) -> Vec<f32> {
        if horizon == 0 {
            return Vec::new();
        }

        if baseline.is_empty() {
            return vec![0.0; horizon];
        }

        if baseline.len() == horizon {
            return baseline.to_vec();
        }

        if baseline.len() > horizon {
            return baseline[..horizon].to_vec();
        }

        let mut forecast = Vec::with_capacity(horizon);
        while forecast.len() < horizon {
            for value in baseline {
                if forecast.len() == horizon {
                    break;
                }
                forecast.push(*value);
            }
        }
        forecast
    }
}

#[cfg(test)]
mod tests {
    use super::NaiveForecast;

    #[test]
    fn forecast_matches_horizon_length() {
        let baseline = vec![1.0, 2.0, 3.0];
        let forecast = NaiveForecast.forecast(&baseline, 7);
        assert_eq!(forecast.len(), 7);
    }

    #[test]
    fn forecast_copies_baseline_when_equal() {
        let baseline = vec![0.5, 1.0, 1.5, 2.0];
        let forecast = NaiveForecast.forecast(&baseline, baseline.len());
        assert_eq!(forecast, baseline);
    }
}
