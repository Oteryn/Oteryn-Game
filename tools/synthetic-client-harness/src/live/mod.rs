//! Live mode of the synthetic client harness (D93): the graphical dev client lives here, in the
//! non-production harness, on top of `oteryn-dev-client`. `apps/client` stays fail-closed
//! (ADR-0011).
//!
//! Kept thin: `model` and `input` are pure mapping (snapshot/delta -> render model, click ->
//! command), `controller` is the loop body over a `DevClientSession`, `cli` is the terminal front
//! end. The renderer crate only owns surface state on this platform, so the frame is text drawn
//! from the render model; a GPU front end would consume the same `RenderModel`.

pub mod cli;
pub mod controller;
pub mod input;
pub mod model;

#[cfg(test)]
mod tests;
