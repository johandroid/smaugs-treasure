//! Payment processing engine.

mod processor;

pub use processor::{PaymentProcessor, ProcessorStats};
#[cfg(test)]
mod tests;
