mod sim;
use sim::clock::Clock;
fn main() {
    let mut clock = Clock::new(5);
    clock.run(|t| println!("Step {}", t));
}
