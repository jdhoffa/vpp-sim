/// External demand response event requesting temporary load reduction.
#[derive(Debug, Clone, Copy)]
pub struct DemandResponseEvent {
    /// Start timestep (inclusive).
    pub start_step: usize,
    /// End timestep (exclusive).
    pub end_step: usize,
    /// Requested reduction while event is active.
    pub requested_reduction_kw: f32,
}

impl DemandResponseEvent {
    /// Creates a new demand-response event spanning `[start_step, end_step)`.
    ///
    /// # Panics
    ///
    /// Panics if `start_step >= end_step` or `requested_reduction_kw < 0.0`.
    pub fn new(start_step: usize, end_step: usize, requested_reduction_kw: f32) -> Self {
        assert!(start_step < end_step);
        assert!(requested_reduction_kw >= 0.0);

        Self {
            start_step,
            end_step,
            requested_reduction_kw,
        }
    }

    /// Returns `true` when `timestep` falls within the active window.
    pub fn is_active(&self, timestep: usize) -> bool {
        timestep >= self.start_step && timestep < self.end_step
    }

    /// Returns the requested reduction in kW if the event is active, otherwise `0.0`.
    pub fn requested_reduction_at_kw(&self, timestep: usize) -> f32 {
        if self.is_active(timestep) {
            self.requested_reduction_kw
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DemandResponseEvent;

    #[test]
    fn active_only_inside_window() {
        let event = DemandResponseEvent::new(5, 8, 2.0);
        assert!(!event.is_active(4));
        assert!(event.is_active(5));
        assert!(event.is_active(7));
        assert!(!event.is_active(8));
    }

    #[test]
    fn reduction_is_zero_outside_window() {
        let event = DemandResponseEvent::new(10, 12, 1.5);
        assert_eq!(event.requested_reduction_at_kw(9), 0.0);
        assert_eq!(event.requested_reduction_at_kw(10), 1.5);
        assert_eq!(event.requested_reduction_at_kw(11), 1.5);
        assert_eq!(event.requested_reduction_at_kw(12), 0.0);
    }
}
