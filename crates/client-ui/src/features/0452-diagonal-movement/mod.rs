mod input;
mod view;
pub use input::step;
pub use view::facing;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
