/// Day-ahead schedule generation utilities.
#[derive(Debug, Default, Clone, Copy)]
pub struct DayAheadSchedule;

impl DayAheadSchedule {
    /// Generate a flat target schedule equal to the average of the forecast.
    pub fn flat_target(forecast: &[f32]) -> Vec<f32> {
        if forecast.is_empty() {
            return Vec::new();
        }

        let sum: f32 = forecast.iter().sum();
        let avg = sum / forecast.len() as f32;
        vec![avg; forecast.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::DayAheadSchedule;

    #[test]
    fn flat_target_matches_length() {
        let forecast = vec![1.0, 2.0, 3.0, 4.0];
        let schedule = DayAheadSchedule::flat_target(&forecast);
        assert_eq!(schedule.len(), forecast.len());
    }

    #[test]
    fn flat_target_is_average() {
        let forecast = vec![1.0, 2.0, 3.0];
        let schedule = DayAheadSchedule::flat_target(&forecast);
        assert_eq!(schedule, vec![2.0, 2.0, 2.0]);
    }
}
