//! Fixed-point arithmetic module for handling monetary amounts with 4 decimal precision.
//!
//! This module provides a safe wrapper around i64 for fixed-point arithmetic,
//! where 1.0000 is represented as 10000 internally.

use crate::error::AmountError;
use serde::{Deserialize, Serialize};
use std::fmt;
// Removed unused imports
use std::str::FromStr;

const DECIMAL_PLACES: i64 = 4;
const SCALE_FACTOR: i64 = 10_000; // 10^4

/// Fixed-point decimal amount with 4 decimal places of precision.
///
/// Internally stored as i64 scaled by 10,000.
/// Example: 1.2345 is stored as 12345
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(i64);

impl Amount {
    /// Creates a new Amount from raw scaled value (internal representation).
    ///
    /// # Arguments
    /// * `raw` - The raw i64 value already scaled by 10,000
    ///
    /// # Examples
    /// ```
    /// # use smaugs_treasure::types::Amount;
    /// let amount = Amount::from_raw(12345); // Represents 1.2345
    /// ```
    pub const fn from_raw(raw: i64) -> Self {
        Amount(raw)
    }

    /// Returns the raw internal representation.
    pub const fn as_raw(&self) -> i64 {
        self.0
    }

    /// Creates an Amount representing zero.
    pub const fn zero() -> Self {
        Amount(0)
    }

    /// Checks if the amount is zero.
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Checks if the amount is positive.
    pub const fn is_positive(&self) -> bool {
        self.0 > 0
    }

    /// Checks if the amount is negative.
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }

    /// Converts to a floating-point number for display purposes only.
    ///
    /// Warning: Should only be used for display, not for calculations.
    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / SCALE_FACTOR as f64
    }

    /// Adds two amounts with overflow checking.
    pub fn add_checked(self, other: Self) -> std::result::Result<Self, AmountError> {
        self.0
            .checked_add(other.0)
            .map(Amount)
            .ok_or(AmountError::Overflow)
    }

    /// Subtracts two amounts with overflow checking.
    pub fn sub_checked(self, other: Self) -> std::result::Result<Self, AmountError> {
        self.0
            .checked_sub(other.0)
            .map(Amount)
            .ok_or(AmountError::Underflow)
    }

    /// Checks if this amount is greater than or equal to another.
    pub fn gte(&self, other: &Self) -> bool {
        self.0 >= other.0
    }
}

impl FromStr for Amount {
    type Err = AmountError;

    /// Parses a string representation of a decimal number into an Amount.
    ///
    /// Supports up to 4 decimal places. Extra decimal places are truncated.
    ///
    /// # Examples
    /// ```
    /// # use smaugs_treasure::types::Amount;
    /// # use std::str::FromStr;
    /// let amount = Amount::from_str("123.4567").unwrap();
    /// assert_eq!(amount.to_f64(), 123.4567);
    /// ```
    fn from_str(s: &str) -> std::result::Result<Self, AmountError> {
        let s = s.trim();

        if s.is_empty() {
            return Err(AmountError::ParseError("empty string".to_string()));
        }

        let (sign, unsigned) = match s.as_bytes().first() {
            Some(b'+') => (1_i64, &s[1..]),
            Some(b'-') => (-1_i64, &s[1..]),
            _ => (1_i64, s),
        };

        if unsigned.is_empty() {
            return Err(AmountError::ParseError("missing numeric value".to_string()));
        }

        let mut parts = unsigned.split('.');
        let integer_str = parts.next().unwrap_or_default();
        let decimal_str = parts.next();

        if parts.next().is_some() {
            return Err(AmountError::ParseError(
                "multiple decimal points".to_string(),
            ));
        }

        if integer_str.is_empty() || !integer_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(AmountError::ParseError(format!(
                "invalid integer part: {}",
                integer_str
            )));
        }

        let integer_part: i64 = integer_str.parse().map_err(|_| {
            AmountError::ParseError(format!("invalid integer part: {}", integer_str))
        })?;

        let decimal_part: i64 = if let Some(decimal_str) = decimal_str {
            if decimal_str.len() > DECIMAL_PLACES as usize {
                return Err(AmountError::ParseError(format!(
                    "too many decimal places (max {})",
                    DECIMAL_PLACES
                )));
            }

            if !decimal_str.chars().all(|c| c.is_ascii_digit()) {
                return Err(AmountError::ParseError(format!(
                    "invalid decimal part: {}",
                    decimal_str
                )));
            }

            let padded = format!("{:0<4}", decimal_str);
            padded.parse().map_err(|_| {
                AmountError::ParseError(format!("invalid decimal part: {}", decimal_str))
            })?
        } else {
            0
        };

        let raw_abs = integer_part
            .checked_mul(SCALE_FACTOR)
            .and_then(|v| v.checked_add(decimal_part))
            .ok_or(AmountError::Overflow)?;

        let raw = if sign < 0 {
            raw_abs.checked_neg().ok_or(AmountError::Overflow)?
        } else {
            raw_abs
        };

        Ok(Amount(raw))
    }
}

impl fmt::Display for Amount {
    /// Formats the amount with exactly 4 decimal places.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs_val = self.0.abs();
        let integer = abs_val / SCALE_FACTOR;
        let decimal = abs_val % SCALE_FACTOR;
        write!(f, "{}{}.{:04}", sign, integer, decimal)
    }
}

// Custom Serialize implementation for CSV output
impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Custom Deserialize implementation for CSV input
impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Amount::from_str(&s).map_err(serde::de::Error::custom)
    }
}
