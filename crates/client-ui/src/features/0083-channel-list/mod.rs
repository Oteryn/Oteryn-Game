pub mod protocol;
mod view;
pub use view::channel_text;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
