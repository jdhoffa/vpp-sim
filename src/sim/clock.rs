/// A simulation clock that tracks steps over a fixed duration.
///
/// The `Clock` provides methods to advance time step-by-step or run
/// a function at each time step until completion.
///
/// # Examples
///
/// ```
/// use vpp_sim::sim::clock::Clock;
///
/// let mut clock = Clock::new(3);
/// let mut steps = Vec::new();
///
/// clock.run(|step| steps.push(step));
/// assert_eq!(steps, vec![0, 1, 2]);
/// ```
pub struct Clock {
    /// Current step of the simulation
    current: usize,
    /// Total steps to run in the simulation
    total: usize,
}

impl Clock {
    /// Creates a new clock with a specified total number of steps.
    ///
    /// # Arguments
    ///
    /// * `total` - The total number of steps the clock will run
    pub fn new(total: usize) -> Self {
        Self { current: 0, total }
    }

    /// Advances the clock by one step.
    ///
    /// # Returns
    ///
    /// * `Some(step)` - The current step number (starting from 0) before advancing
    /// * `None` - If the clock has reached its total steps
    pub fn tick(&mut self) -> Option<usize> {
        if self.current < self.total {
            let step = self.current;
            self.current += 1;
            Some(step)
        } else {
            None
        }
    }

    /// Runs a function for each remaining step in the clock.
    ///
    /// This method will call the provided function with the current step
    /// number for each step until the clock completes all steps.
    ///
    /// # Arguments
    ///
    /// * `f` - A function that takes the current step number as an argument
    pub fn run(&mut self, mut f: impl FnMut(usize)) {
        while let Some(step) = self.tick() {
            f(step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_clock() {
        let clock = Clock::new(5);
        assert_eq!(clock.current, 0);
        assert_eq!(clock.total, 5);
    }

    #[test]
    fn test_tick() {
        let mut clock = Clock::new(2);
        assert_eq!(clock.tick(), Some(0));
        assert_eq!(clock.tick(), Some(1));
        assert_eq!(clock.tick(), None);
    }

    #[test]
    fn test_run() {
        let mut clock = Clock::new(3);
        let mut steps = Vec::new();

        clock.run(|step| steps.push(step));

        assert_eq!(steps, vec![0, 1, 2]);
    }

    #[test]
    fn test_empty_clock() {
        let mut clock = Clock::new(0);
        assert_eq!(clock.tick(), None);

        let mut was_called = false;
        clock.run(|_| was_called = true);
        assert!(!was_called);
    }
}
