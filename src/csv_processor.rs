//! CSV streaming processor for reading transactions.

use crate::error::{ParseError, Result};
use crate::types::Transaction;
use csv::{ReaderBuilder, Trim};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::{debug, warn};

/// Streaming CSV processor that reads transactions line-by-line.
pub struct CsvProcessor {
    reader: csv::Reader<BufReader<File>>,
    line_number: usize,
}

impl CsvProcessor {
    /// Creates a new CSV processor from a file path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let csv_reader = ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(true) // Allow missing fields (e.g., amount for disputes)
            .from_reader(reader);

        Ok(Self {
            reader: csv_reader,
            line_number: 0,
        })
    }

    /// Processes the CSV stream line-by-line, calling the handler for each transaction.
    pub fn process_stream<F>(&mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(Result<Transaction>),
    {
        debug!("Starting CSV stream processing");

        for result in self.reader.deserialize::<Transaction>() {
            self.line_number += 1;

            match result {
                Ok(tx) => {
                    debug!("Read transaction at line {}: {:?}", self.line_number, tx);
                    handler(Ok(tx));
                }
                Err(e) => {
                    warn!("CSV error at line {}: {}", self.line_number, e);
                    handler(Err(ParseError::InvalidCsvRow(format!(
                        "line {}: {}",
                        self.line_number, e
                    ))
                    .into()));
                }
            }
        }

        debug!("Finished processing {} lines", self.line_number);
        Ok(())
    }
}
