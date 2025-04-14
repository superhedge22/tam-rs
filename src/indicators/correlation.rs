use std::fmt;

use crate::errors::{Result, TaError};
use crate::{Next, Period, Reset};
use serde::{Deserialize, Serialize};

/// Pearson's Correlation Coefficient (r).
///
/// A statistical measure of the strength of a linear relationship between two variables.
/// The correlation coefficient has a value between -1 and 1, where:
/// * -1 indicates a perfect negative correlation
/// * 0 indicates no correlation
/// * 1 indicates a perfect positive correlation
///
/// # Formula
///
/// Correlation = (sum(x*y) - sum(x)*sum(y)/n) / sqrt((sum(x²) - sum(x)²/n) * (sum(y²) - sum(y)²/n))
///
/// Where:
/// * x and y are the two input series
/// * n is the number of points (period)
///
/// # Parameters
///
/// * _period_ - number of periods (integer greater than 0). Default value is 30.
///
/// # Example
///
/// ```
/// use tam::indicators::Correlation;
/// use tam::Next;
///
/// let mut corr = Correlation::new(3).unwrap();
/// assert_eq!(corr.next((2.0, 3.0)), 0.0);  // First point doesn't have correlation
/// assert_eq!(corr.next((3.0, 2.0)), -1.0); // Perfect negative correlation with 2 points
/// assert_eq!(corr.next((6.0, 1.0)), -0.9607689228305228); // Strong negative correlation
/// ```
#[doc(alias = "CORREL")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correlation {
    period: usize,
    index: usize,
    count: usize,
    sum_x: f64,
    sum_y: f64,
    sum_xy: f64,
    sum_x2: f64,
    sum_y2: f64,
    values_x: Box<[f64]>,
    values_y: Box<[f64]>,
}

impl Correlation {
    pub fn new(period: usize) -> Result<Self> {
        match period {
            0 => Err(TaError::InvalidParameter),
            _ => Ok(Self {
                period,
                index: 0,
                count: 0,
                sum_x: 0.0,
                sum_y: 0.0,
                sum_xy: 0.0,
                sum_x2: 0.0,
                sum_y2: 0.0,
                values_x: vec![0.0; period].into_boxed_slice(),
                values_y: vec![0.0; period].into_boxed_slice(),
            }),
        }
    }
}

impl Period for Correlation {
    fn period(&self) -> usize {
        self.period
    }
}

impl Next<(f64, f64)> for Correlation {
    type Output = f64;

    fn next(&mut self, input: (f64, f64)) -> Self::Output {
        let (input_x, input_y) = input;
        
        // Store the trailing values before we overwrite them
        let trailing_x = self.values_x[self.index];
        let trailing_y = self.values_y[self.index];
        
        // Add new values to the buffers
        self.values_x[self.index] = input_x;
        self.values_y[self.index] = input_y;
        
        // Update index for next iteration
        self.index = if self.index + 1 < self.period {
            self.index + 1
        } else {
            0
        };
        
        // Update count of points (up to period)
        if self.count < self.period {
            self.count += 1;
            
            // Update sums with new values
            self.sum_x += input_x;
            self.sum_y += input_y;
            self.sum_xy += input_x * input_y;
            self.sum_x2 += input_x * input_x;
            self.sum_y2 += input_y * input_y;
        } else {
            // We have a full period, so remove trailing values and add new ones
            self.sum_x = self.sum_x - trailing_x + input_x;
            self.sum_y = self.sum_y - trailing_y + input_y;
            self.sum_xy = self.sum_xy - (trailing_x * trailing_y) + (input_x * input_y);
            self.sum_x2 = self.sum_x2 - (trailing_x * trailing_x) + (input_x * input_x);
            self.sum_y2 = self.sum_y2 - (trailing_y * trailing_y) + (input_y * input_y);
        }
        
        // Calculate correlation coefficient
        if self.count < 2 {
            // Need at least 2 points for correlation
            return 0.0;
        }
        
        let n = self.count as f64;
        let numerator = self.sum_xy - ((self.sum_x * self.sum_y) / n);
        let denominator_x = self.sum_x2 - ((self.sum_x * self.sum_x) / n);
        let denominator_y = self.sum_y2 - ((self.sum_y * self.sum_y) / n);
        let denominator = denominator_x * denominator_y;
        
        // Check for division by zero or negative under sqrt
        if denominator <= 0.0 {
            return 0.0;
        }
        
        numerator / denominator.sqrt()
    }
}

impl Reset for Correlation {
    fn reset(&mut self) {
        self.index = 0;
        self.count = 0;
        self.sum_x = 0.0;
        self.sum_y = 0.0;
        self.sum_xy = 0.0;
        self.sum_x2 = 0.0;
        self.sum_y2 = 0.0;
        
        for i in 0..self.period {
            self.values_x[i] = 0.0;
            self.values_y[i] = 0.0;
        }
    }
}

impl Default for Correlation {
    fn default() -> Self {
        Self::new(30).unwrap()
    }
}

impl fmt::Display for Correlation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CORREL({})", self.period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;
    
    #[test]
    fn test_new() {
        assert!(Correlation::new(0).is_err());
        assert!(Correlation::new(1).is_ok());
    }
    
    #[test]
    /// Running Correlation Test Cases
    /// ======================================================================
    /// Test Case Results:
    /// ----------------------------------------------------------------------
    /// Step  | Input (x, y) |        Expected        |      Python Impl       | Match
    /// ----------------------------------------------------------------------
    ///   1   | (2.0, 3.0) |     0.0000000000000000 |     0.0000000000000000 | ✓
    ///   2   | (3.0, 2.0) |    -1.0000000000000000 |    -1.0000000000000000 | ✓
    ///   3   | (6.0, 1.0) |    -0.9607689228305228 |    -0.9607689228305226 | ✓
    ///   4   | (5.0, 2.0) |    -0.7559289460184537 |    -0.7559289460184546 | ✓

    fn test_next() {
        let mut corr = Correlation::new(3).unwrap();
        
        // First point has no correlation yet
        assert_eq!(corr.next((2.0, 3.0)), 0.0);
        
        // Perfect negative correlation with 2 points
        assert_eq!(corr.next((3.0, 2.0)), -1.0);
        
        // Strong negative correlation with 3 points
        assert_eq!(corr.next((6.0, 1.0)), -0.9607689228305228);
        
        // Sliding window, removing the first point
        assert_eq!(corr.next((5.0, 2.0)), -0.7559289460184537);
    }
    
    #[test]
    fn test_reset() {
        let mut corr = Correlation::new(3).unwrap();
        
        corr.next((2.0, 3.0));
        corr.next((3.0, 2.0));
        
        corr.reset();
        assert_eq!(corr.next((8.0, 9.0)), 0.0);
    }
    
    #[test]
    fn test_default() {
        Correlation::default();
    }
    
    #[test]
    fn test_display() {
        let indicator = Correlation::new(10).unwrap();
        assert_eq!(format!("{}", indicator), "CORREL(10)");
    }
} 