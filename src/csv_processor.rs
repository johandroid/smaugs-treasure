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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_process_valid_csv() {
        let csv_data = "type,client,tx,amount\ndeposit,1,1,100.0\nwithdrawal,1,2,50.0\n";
        let file = create_temp_csv(csv_data);
        let mut processor = CsvProcessor::from_path(file.path()).unwrap();

        let mut transactions = Vec::new();
        processor
            .process_stream(|result| {
                if let Ok(tx) = result {
                    transactions.push(tx);
                }
            })
            .unwrap();

        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions[0].client, 1);
        assert_eq!(transactions[0].tx, 1);
    }

    #[test]
    fn test_process_with_whitespace() {
        let csv_data = "type,client,tx,amount\n  deposit  ,  1  ,  1  ,  100.0  \n";
        let file = create_temp_csv(csv_data);
        let mut processor = CsvProcessor::from_path(file.path()).unwrap();

        let mut transactions = Vec::new();
        processor
            .process_stream(|result| {
                if let Ok(tx) = result {
                    transactions.push(tx);
                }
            })
            .unwrap();

        assert_eq!(transactions.len(), 1);
    }

    #[test]
    fn test_process_without_amount() {
        let csv_data = "type,client,tx,amount\ndispute,1,1,\n";
        let file = create_temp_csv(csv_data);
        let mut processor = CsvProcessor::from_path(file.path()).unwrap();

        let mut transactions = Vec::new();
        processor
            .process_stream(|result| {
                if let Ok(tx) = result {
                    transactions.push(tx);
                }
            })
            .unwrap();

        assert_eq!(transactions.len(), 1);
        assert!(transactions[0].amount.is_none());
    }

    #[test]
    fn test_process_invalid_transaction_type() {
        let csv_data = "type,client,tx,amount\ninvalid,1,1,100.0\n";
        let file = create_temp_csv(csv_data);
        let mut processor = CsvProcessor::from_path(file.path()).unwrap();

        let mut errors = 0;
        processor
            .process_stream(|result| {
                if result.is_err() {
                    errors += 1;
                }
            })
            .unwrap();

        assert_eq!(errors, 1);
    }

    #[test]
    fn test_line_number_tracking() {
        let csv_data = "type,client,tx,amount\ndeposit,1,1,100.0\ndeposit,1,2,200.0\n";
        let file = create_temp_csv(csv_data);
        let mut processor = CsvProcessor::from_path(file.path()).unwrap();

        processor.process_stream(|_| {}).unwrap();
        assert_eq!(processor.line_number, 2);
    }
}
