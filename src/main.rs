//! Binary entry point for the payment processor.

use anyhow::Result;

fn main() -> Result<()> {
    smaugs_treasure::app::run()
}
