use std::fmt;
use std::collections::VecDeque;

use crate::errors::Result;
use crate::{Close, Next, Period, Reset};
use serde::{Deserialize, Serialize};

/// The relative strength index (RSI).
///
/// It is a momentum oscillator,
/// that compares the magnitude of recent gains
/// and losses over a specified time period to measure speed and change of price
/// movements of a security. It is primarily used to attempt to identify
/// overbought or oversold conditions in the trading of an asset.
///
/// The oscillator returns output in the range of 0..100.
///
/// ![RSI](https://upload.wikimedia.org/wikipedia/commons/6/67/RSIwiki.gif)
///
/// # Formula
///
/// RSI<sub>t</sub> = 100 - (100 / (1 + RS<sub>t</sub>))
///
/// Where:
///
/// * RSI<sub>t</sub> - value of RSI indicator in a moment of time _t_
/// * RS<sub>t</sub> - Relative Strength value at time _t_
/// * RS<sub>t</sub> = AvgGain<sub>t</sub> / AvgLoss<sub>t</sub>
/// * AvgGain<sub>t</sub> - Average gain at time _t_, using Wilder's smoothing method
/// * AvgLoss<sub>t</sub> - Average loss at time _t_, using Wilder's smoothing method
///
/// If current period has value higher than previous period, than:
///
/// Gain = p<sub>t</sub> - p<sub>t-1</sub>
///
/// Loss = 0
///
/// Otherwise:
///
/// Gain = 0
///
/// Loss = p<sub>t-1</sub> - p<sub>t</sub>
///
/// For the initial period, simple average is used. Then for subsequent periods:
/// * AvgGain = ((PreviousAvgGain * (period-1)) + CurrentGain) / period
/// * AvgLoss = ((PreviousAvgLoss * (period-1)) + CurrentLoss) / period
///
/// # Parameters
///
/// * _period_ - number of periods (integer greater than 0). Default value is 14.
///
/// # Example
///
/// ```
/// use ta::indicators::RelativeStrengthIndex;
/// use ta::Next;
///
/// let mut rsi = RelativeStrengthIndex::new(3).unwrap();
/// // First period values are NaN as per TA-Lib behavior
/// assert!(rsi.next(10.0).is_nan());
/// assert!(rsi.next(10.5).is_nan());
/// assert!(rsi.next(10.0).is_nan());
/// assert_eq!(rsi.next(9.5).round(), 33.0);
/// ```
///
/// # Links
/// * [Relative strength index (Wikipedia)](https://en.wikipedia.org/wiki/Relative_strength_index)
/// * [RSI (Investopedia)](http://www.investopedia.com/terms/r/rsi.asp)
///
#[doc(alias = "RSI")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeStrengthIndex {
    period: usize,
    prev_val: f64,
    is_new: bool,
    price_changes: VecDeque<(f64, f64)>,
    avg_gain: f64,
    avg_loss: f64,
}

impl RelativeStrengthIndex {
    pub fn new(period: usize) -> Result<Self> {
        if period == 0 {
            return Err(crate::errors::TaError::InvalidParameter);
        }
        
        Ok(Self {
            period,
            prev_val: 0.0,
            is_new: true,
            price_changes: VecDeque::with_capacity(period),
            avg_gain: 0.0,
            avg_loss: 0.0,
        })
    }
}

impl Period for RelativeStrengthIndex {
    fn period(&self) -> usize {
        self.period
    }
}

impl Next<f64> for RelativeStrengthIndex {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        // Handle the first input
        if self.is_new {
            self.is_new = false;
            self.prev_val = input;
            return std::f64::NAN; // TA-Lib returns NaN for first values
        }
        
        // Calculate price change
        let change = input - self.prev_val;
        self.prev_val = input;
        
        // Split the change into gain and loss components
        let (gain, loss) = if change >= 0.0 {
            (change, 0.0)
        } else {
            (0.0, -change) // Make loss positive
        };
        
        // Store price change data
        self.price_changes.push_back((gain, loss));
        
        // If we don't have a full period of price changes yet, return NaN
        if self.price_changes.len() < self.period {
            return std::f64::NAN;
        }
        
        // Keep only the changes needed for the calculation
        while self.price_changes.len() > self.period {
            self.price_changes.pop_front();
        }
        
        // First time we have enough data - use simple average
        if self.price_changes.len() == self.period && self.avg_gain == 0.0 && self.avg_loss == 0.0 {
            // Calculate initial averages using simple average method
            let mut sum_gains = 0.0;
            let mut sum_losses = 0.0;
            
            for &(gain, loss) in self.price_changes.iter() {
                sum_gains += gain;
                sum_losses += loss;
            }
            
            self.avg_gain = sum_gains / self.period as f64;
            self.avg_loss = sum_losses / self.period as f64;
        } else {
            // For subsequent calculations, use Wilder's smoothing
            self.avg_gain = ((self.avg_gain * (self.period as f64 - 1.0)) + gain) / self.period as f64;
            self.avg_loss = ((self.avg_loss * (self.period as f64 - 1.0)) + loss) / self.period as f64;
        }
        
        // Calculate RSI
        if self.avg_loss == 0.0 {
            if self.avg_gain == 0.0 {
                return 50.0; // No movement
            }
            return 100.0; // Only gains
        }
        
        // RSI = 100 - (100 / (1 + RS))
        let rs = self.avg_gain / self.avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}

impl<T: Close> Next<&T> for RelativeStrengthIndex {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        self.next(input.close())
    }
}

impl Reset for RelativeStrengthIndex {
    fn reset(&mut self) {
        self.is_new = true;
        self.prev_val = 0.0;
        self.price_changes.clear();
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
    }
}

impl Default for RelativeStrengthIndex {
    fn default() -> Self {
        Self::new(14).unwrap()
    }
}

impl fmt::Display for RelativeStrengthIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RSI({})", self.period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;

    // Custom version of test_indicator that works with RSI's NaN values
    #[test]
    fn test_indicator() {
        let bar = Bar::new();

        // ensure Default trait is implemented
        let mut indicator = RelativeStrengthIndex::default();

        // ensure Next<f64> is implemented - accept NaN for RSI first value
        let first_output = indicator.next(12.3);
        assert!(first_output.is_nan());

        // ensure next accepts &DataItem as well
        indicator.next(&bar);

        // ensure Reset is implemented and works correctly
        indicator.reset();
        let reset_output = indicator.next(12.3);
        assert!(reset_output.is_nan());
        assert!(first_output.is_nan());

        // ensure Display is implemented
        format!("{}", indicator);
    }

    #[test]
    fn test_new() {
        assert!(RelativeStrengthIndex::new(0).is_err());
        assert!(RelativeStrengthIndex::new(1).is_ok());
    }

    /// TA-Lib RSI Values for Test Cases (period=3)
    /// ============================================================
    /// Step  Price  TA-Lib RSI  Rounded
    ///     1   10.0         NaN      NaN
    ///     2   10.5         NaN      NaN
    ///     3   10.0         NaN      NaN
    ///     4    9.5   33.333333     33.0
    ///     5    9.0   22.222222     22.0
    ///     6   10.0   61.111111     61.0
    ///     7   10.5   71.717172     72.0
    ///     8   17.2   95.636590     96.0
    #[test]
    fn test_next() {
        let mut rsi = RelativeStrengthIndex::new(3).unwrap();
        
        // First value: TA-Lib returns NaN for the first data point
        let first = rsi.next(10.0);
        assert!(first.is_nan());
        
        // Second value: TA-Lib returns NaN for the second data point
        let second = rsi.next(10.5);
        assert!(second.is_nan());
        
        // Third value: TA-Lib returns NaN for the third data point
        let third = rsi.next(10.0);
        assert!(third.is_nan());
        
        // Fourth value: Now we have enough data for a real RSI calculation
        let fourth = rsi.next(9.5);
        
        // Matches TA-Lib value of 33.333 (rounds to 33)
        assert_eq!(fourth.round(), 33.0);
        
        // Fifth value: Continues with valid RSI values
        let fifth = rsi.next(9.0);
        assert_eq!(fifth.round(), 22.0); // TA-Lib: 22.222 -> 22
        
        // Sixth value
        let sixth = rsi.next(10.0);
        assert_eq!(sixth.round(), 61.0); // TA-Lib: 61.111 -> 61
        
        // Seventh value
        let seventh = rsi.next(10.5);
        assert_eq!(seventh.round(), 72.0); // TA-Lib: 71.717 -> 72

        let eighth = rsi.next(17.2);
        assert!((eighth - 95.6365903070).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reset() {
        let mut rsi = RelativeStrengthIndex::new(3).unwrap();
        
        // First value after initialization is NaN
        let first = rsi.next(10.0);
        assert!(first.is_nan());
        
        // Second value is NaN
        let second = rsi.next(10.5);
        assert!(second.is_nan());

        // After reset, behavior should repeat
        rsi.reset();
        
        // First value after reset is NaN
        let first_after_reset = rsi.next(10.0);
        assert!(first_after_reset.is_nan());
        
        // Second value after reset is NaN
        let second_after_reset = rsi.next(10.5);
        assert!(second_after_reset.is_nan());
    }

    #[test]
    fn test_default() {
        RelativeStrengthIndex::default();
    }

    #[test]
    fn test_display() {
        let rsi = RelativeStrengthIndex::new(16).unwrap();
        assert_eq!(format!("{}", rsi), "RSI(16)");
    }
}

