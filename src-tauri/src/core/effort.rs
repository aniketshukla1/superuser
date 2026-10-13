//! How hard the model is asked to think, as one named scale the whole app agrees on.
//!
//! The composer shows this as a slider from "Faster" to "Smarter". It is not a
//! cosmetic preference: each level resolves to an extended-thinking budget on the
//! request, so moving the slider changes what the model actually does.
//!
//! The scale is five named steps rather than a raw token count because a token
//! count is not a thing anyone has an opinion about. "How much should it think
//! about this" is.

#[cfg(test)]
mod tests;

mod budget;
mod what_each_model;

pub use budget::*;
pub use what_each_model::*;
