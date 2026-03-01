//! Payment processing engine.

mod processor;

pub use processor::{IgnoredReason, PaymentProcessor, ProcessingOutcome};
#[cfg(test)]
mod tests;
