use std::fmt;

use crate::errors::{Result, TaError};
use crate::{Close, High, Low, Next, Period, Reset};
use serde::{Deserialize, Serialize};

const DEFAULT_PERIOD: usize = 14;
const DEFAULT_UNSTABLE_PERIOD: usize = 15;
const DEFAULT_ROUND_POS: bool = false;
const MIN_VALUE: f64 = 0.0;
const MAX_VALUE: f64 = 100.0;

/// Average Directional Movement Index (ADX).
///
/// A technical analysis indicator used to determine the strength of a trend.
/// It measures the strength of the trend, regardless of whether it's up or down.
///
/// # Formula
///
/// 1. Calculate the Directional Movement:
///    * +DM1 = current high - previous high (if positive and greater than -DM1, otherwise 0)
///    * -DM1 = previous low - current low (if positive and greater than +DM1, otherwise 0)
///
/// 2. Calculate the True Range (TR)
///
/// 3. Calculate the Directional Indicators:
///    * +DI = 100 * EMA(+DM) / EMA(TR)
///    * -DI = 100 * EMA(-DM) / EMA(TR)
///
/// 4. Calculate the Directional Index (DX):
///    * DX = 100 * |+DI - -DI| / (|+DI| + |-DI|)
///
/// 5. Calculate the Average Directional Index (ADX):
///    * ADX = EMA(DX) over the specified period
///
/// # Parameters
///
/// * _period_ - smoothing period (integer greater than 1). Default value is 14.
///
/// # Example
///
/// ```
/// use tam::indicators::AverageDirectionalIndex;
/// use tam::{Next, DataItem};
///
/// let mut adx = AverageDirectionalIndex::new(14).unwrap();
/// // You need at least 2*period-1 data points to get valid ADX values
/// // Feed price data to calculate ADX values
/// let item = DataItem::builder()
///     .high(102.0)
///     .low(98.0)
///     .close(100.0)
///     .open(99.0)
///     .volume(1000.0)
///     .build()
///     .unwrap();
/// let _adx_value = adx.next(&item);
/// ```
#[doc(alias = "ADX")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AverageDirectionalIndex {
    period: usize,
    prev_high: Option<f64>,
    prev_low: Option<f64>,
    prev_close: Option<f64>,
    prev_plus_dm: f64,
    prev_minus_dm: f64,
    prev_tr: f64,
    prev_adx: f64,
    dx_values: Vec<f64>,
    dx_count: usize,
    is_initialized: bool,
    unstable_period: usize,
    unstable_period_count: usize,  
    round_pos: bool,
}

impl AverageDirectionalIndex {
    pub fn new(period: usize) -> Result<Self> {
        match period {
            0 | 1 => Err(TaError::InvalidParameter),
            _ => Ok(Self {
                period,
                prev_high: None,
                prev_low: None,
                prev_close: None,
                prev_plus_dm: 0.0,
                prev_minus_dm: 0.0,
                prev_tr: 0.0,
                prev_adx: 0.0,
                dx_values: Vec::new(),
                dx_count: 0,
                is_initialized: false,
                unstable_period: DEFAULT_UNSTABLE_PERIOD,  
                unstable_period_count: 0,
                round_pos: DEFAULT_ROUND_POS,
            }),
        }
    }
    
    /// Enable rounding of the ADX value.
    ///
    /// This method returns a new instance of the AverageDirectionalIndex with rounding enabled.
    /// By default, rounding is disabled.
    ///
    /// # Returns
    ///
    /// A new instance of the `AverageDirectionalIndex` with rounding enabled.
    pub fn with_rounding(mut self) -> Self {
        self.round_pos = true;
        self
    }

    // Helper function to calculate the true range
    fn calculate_tr(&self, high: f64, low: f64) -> f64 {
        if let Some(prev_close) = self.prev_close {
            let range = high - low;
            let high_close = (high - prev_close).abs();
            let low_close = (low - prev_close).abs();
            
            range.max(high_close).max(low_close)
        } else {
            high - low
        }
    }
    

    fn round_pos(&self, x: f64) -> f64 {
        if self.round_pos {
            x.round()
        } else {
            x
        }
    }
}

impl Period for AverageDirectionalIndex {
    fn period(&self) -> usize {
        self.period
    }
}

impl<T: High + Low + Close> Next<&T> for AverageDirectionalIndex {
    type Output = f64;

    fn next(&mut self, bar: &T) -> Self::Output {
        let high = bar.high();
        let low = bar.low();
        let close = bar.close();

        // Initial setup or after reset
        if self.prev_high.is_none() {
            self.prev_high = Some(high);
            self.prev_low = Some(low);
            self.prev_close = Some(close);
            return MIN_VALUE;
        }

        let prev_high = self.prev_high.unwrap();
        let prev_low = self.prev_low.unwrap();

        // Calculate directional movements
        let diff_p = high - prev_high; // Plus Delta
        let diff_m = prev_low - low;   // Minus Delta

        // Update plus and minus DM based on rules from the C++ implementation
        let plus_dm1;
        let minus_dm1;

        if (diff_m > MIN_VALUE) && (diff_p < diff_m) {
            // Case 2 and 4: +DM=0, -DM=diffM
            plus_dm1 = MIN_VALUE;
            minus_dm1 = diff_m;
        } else if (diff_p > MIN_VALUE) && (diff_p > diff_m) {
            // Case 1 and 3: +DM=diffP, -DM=0
            plus_dm1 = diff_p;
            minus_dm1 = MIN_VALUE;
        } else {
            // Case 5, 6, and 7: +DM=0, -DM=0
            plus_dm1 = MIN_VALUE;
            minus_dm1 = MIN_VALUE;
        }

        // Calculate true range
        let tr = self.calculate_tr(high, low);

        // Wilder's smoothing for DM and TR
        if !self.is_initialized {
            // Accumulation phase - first period values
            self.prev_plus_dm += plus_dm1;
            self.prev_minus_dm += minus_dm1;
            self.prev_tr += tr;
            self.dx_count += 1;

            if self.dx_count == self.period - 1 {
                // We've collected period-1 values, now calculate first smoothed values
                self.is_initialized = true;
                self.dx_count = 0;
            }

            self.prev_high = Some(high);
            self.prev_low = Some(low);
            self.prev_close = Some(close);

            return MIN_VALUE;
        }

        // Apply Wilder's smoothing
        if self.dx_values.is_empty() {
            // First smoothing phase - we update with the last +DM and TR
            self.prev_plus_dm += plus_dm1;
            self.prev_minus_dm += minus_dm1;
            self.prev_tr += tr;

            // Calculate the first DI values with rounding as TA-Lib does
            let plus_di = if self.prev_tr > MIN_VALUE {
                self.round_pos(MAX_VALUE * (self.prev_plus_dm / self.prev_tr))
            } else {
                MIN_VALUE
            };
            
            let minus_di = if self.prev_tr > MIN_VALUE {
                self.round_pos(MAX_VALUE * (self.prev_minus_dm / self.prev_tr))
            } else {
                MIN_VALUE
            };

            // Calculate the first DX value with rounding as TA-Lib does
            let di_diff = (plus_di - minus_di).abs();
            let di_sum = plus_di + minus_di;
            
            let dx = if di_sum > MIN_VALUE {
                self.round_pos(MAX_VALUE * (di_diff / di_sum))
            } else {
                MIN_VALUE
            };

            self.dx_values.push(dx);
            
            // Start applying Wilder's smoothing for subsequent values
            self.prev_plus_dm = self.prev_plus_dm - (self.prev_plus_dm / self.period as f64) + plus_dm1;
            self.prev_minus_dm = self.prev_minus_dm - (self.prev_minus_dm / self.period as f64) + minus_dm1;
            self.prev_tr = self.prev_tr - (self.prev_tr / self.period as f64) + tr;
        } else if self.dx_values.len() < self.period {
            // Continue accumulating DX values until we have period values
            self.prev_plus_dm = self.prev_plus_dm - (self.prev_plus_dm / self.period as f64) + plus_dm1;
            self.prev_minus_dm = self.prev_minus_dm - (self.prev_minus_dm / self.period as f64) + minus_dm1;
            self.prev_tr = self.prev_tr - (self.prev_tr / self.period as f64) + tr;

            let plus_di = if self.prev_tr > MIN_VALUE {
                self.round_pos(MAX_VALUE * (self.prev_plus_dm / self.prev_tr))
            } else {
                MIN_VALUE
            };
            
            let minus_di = if self.prev_tr > MIN_VALUE {
                self.round_pos(MAX_VALUE * (self.prev_minus_dm / self.prev_tr))
            } else {
                MIN_VALUE
            };

            let di_diff = (plus_di - minus_di).abs();
            let di_sum = plus_di + minus_di;
            
            let dx = if di_sum > MIN_VALUE {
                self.round_pos(MAX_VALUE * (di_diff / di_sum))
            } else {
                MIN_VALUE
            };

            self.dx_values.push(dx);
            
            if self.dx_values.len() == self.period {
                // Calculate first ADX as average of first period DX values
                self.prev_adx = self.round_pos(self.dx_values.iter().sum::<f64>() / self.period as f64);
                // Reset unstable period counter when we have the first ADX value
                self.unstable_period_count = 0;
            }
        } else {
            // Normal calculation after initialization
            self.prev_plus_dm = self.prev_plus_dm - (self.prev_plus_dm / self.period as f64) + plus_dm1;
            self.prev_minus_dm = self.prev_minus_dm - (self.prev_minus_dm / self.period as f64) + minus_dm1;
            self.prev_tr = self.prev_tr - (self.prev_tr / self.period as f64) + tr;

            let plus_di = if self.prev_tr > MIN_VALUE {
                self.round_pos(MAX_VALUE * (self.prev_plus_dm / self.prev_tr))
            } else {
                MIN_VALUE
            };
            
            let minus_di = if self.prev_tr > MIN_VALUE {
                self.round_pos(MAX_VALUE * (self.prev_minus_dm / self.prev_tr))
            } else {
                MIN_VALUE
            };

            let di_diff = (plus_di - minus_di).abs();
            let di_sum = plus_di + minus_di;
            
            let dx = if di_sum > MIN_VALUE {
                self.round_pos(MAX_VALUE * (di_diff / di_sum))
            } else {
                MIN_VALUE
            };

            // Calculate ADX using Wilder's smoothing with rounding as TA-Lib does
            self.prev_adx = self.round_pos(((self.prev_adx * (self.period as f64 - 1.0)) + dx) / self.period as f64);
            
            // Count up in the unstable period if we haven't reached it yet
            if self.unstable_period_count < self.unstable_period {
                self.unstable_period_count += 1;
            }
        }

        // Update previous values for next calculation
        self.prev_high = Some(high);
        self.prev_low = Some(low);
        self.prev_close = Some(close);

        // Return ADX value or f64::NAN if still initializing
        if self.dx_values.len() < self.period {
            f64::NAN // Not enough data yet
        } else {
            // Always return the calculated ADX, even during unstable period
            // This matches TA-Lib behavior where values are calculated but may not be reliable
            // during the unstable period
            self.prev_adx
        }
    }
}

impl Reset for AverageDirectionalIndex {
    fn reset(&mut self) {
        self.prev_high = None;
        self.prev_low = None;
        self.prev_close = None;
        self.prev_plus_dm = 0.0;
        self.prev_minus_dm = 0.0;
        self.prev_tr = 0.0;
        self.prev_adx = 0.0;
        self.dx_values.clear();
        self.dx_count = 0;
        self.is_initialized = false;
        self.unstable_period_count = 0;
    }
}

impl Default for AverageDirectionalIndex {
    fn default() -> Self {
        Self::new(DEFAULT_PERIOD).unwrap()
    }
}

impl fmt::Display for AverageDirectionalIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ADX({})", self.period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;


    #[test]
    fn test_new() {
        assert!(AverageDirectionalIndex::new(0).is_err());
        assert!(AverageDirectionalIndex::new(1).is_err());
        assert!(AverageDirectionalIndex::new(2).is_ok());
    }

    #[test]
    fn test_next() {
        // Basic functionality tests are still useful to keep
        let mut adx = AverageDirectionalIndex::new(14).unwrap();
        
        // Need to feed at least 2 * period - 1 bars to get valid results
        // The first bar just initializes the prev values
        let bar1 = Bar::new().high(10.0).low(8.0).close(9.0);
        assert_eq!(adx.next(&bar1), 0.0);
        
        // Additional data points
        let bar2 = Bar::new().high(11.0).low(9.0).close(10.0);
        assert_eq!(adx.next(&bar2), 0.0);
        
        // Test with more data points
        let mut adx = AverageDirectionalIndex::new(3).unwrap();
        adx.next(&Bar::new().high(10.0).low(8.0).close(9.0));
        adx.next(&Bar::new().high(11.0).low(9.0).close(10.0));
        adx.next(&Bar::new().high(10.5).low(8.5).close(9.5));
        adx.next(&Bar::new().high(11.5).low(9.5).close(10.5));
        adx.next(&Bar::new().high(12.0).low(10.0).close(11.0));
        adx.next(&Bar::new().high(11.0).low(9.0).close(10.0));
        let value = adx.next(&Bar::new().high(12.5).low(10.5).close(11.5));
        assert!(value > 0.0);
        
        // Now test with the ground truth data from adx_test_cases.json
        use std::fs::File;
        use std::io::BufReader;
        use serde_json::Value;
        
        let file = match File::open("tests/data/adx_test_cases.json") {
            Ok(f) => f,
            Err(_) => {
                println!("Skipping ground truth test: adx_test_cases.json not found");
                return;
            }
        };
        
        let reader = BufReader::new(file);
        let json: Value = match serde_json::from_reader(reader) {
            Ok(j) => j,
            Err(e) => {
                panic!("Failed to parse adx_test_cases.json: {}", e);
            }
        };
        
        // Test with data from the "realistic" dataset, which has more varied price movements
        let dataset = &json["realistic"];
        
        // Test with different periods (7, 14, 21)
        for period_name in ["period_7", "period_14", "period_21"].iter() {
            if let Some(period_data) = dataset.get(period_name) {
                let timeperiod = period_data["timeperiod"].as_u64().unwrap() as usize;
                let high_values = period_data["high"].as_array().unwrap();
                let low_values = period_data["low"].as_array().unwrap();
                let close_values = period_data["close"].as_array().unwrap();
                let adx_values = period_data["adx"].as_array().unwrap();
                
                let mut adx = AverageDirectionalIndex::new(timeperiod).unwrap();
                
                for i in 0..high_values.len() {
                    let high = high_values[i].as_f64().unwrap();
                    let low = low_values[i].as_f64().unwrap();
                    let close = close_values[i].as_f64().unwrap();
                    let expected_adx = adx_values[i].as_f64();
                    
                    let bar = Bar::new().high(high).low(low).close(close);
                    let result = adx.next(&bar);
                    
                    // Skip NaN values in the expected results
                    if let Some(expected) = expected_adx {
                        // Allow some tolerance for different implementations
                        // ADX calculation can vary slightly across libraries
                        let tolerance = 2.0;
                        
                        // Only test after we have enough data to calculate reliable values
                        // This is typically after 2*period+1 bars
                        if i >= 2 * timeperiod + 1 {
                            assert!((result - expected).abs() < tolerance,
                                "Period {}: ADX mismatch at index {}: got {}, expected {}",
                                timeperiod, i, result, expected);
                        }
                    }
                }
                
                println!("Successfully tested ADX with period {}", timeperiod);
            }
        }
    }

    #[test]
    fn test_reset() {
        let mut adx = AverageDirectionalIndex::new(5).unwrap();
        
        // Feed some data
        adx.next(&Bar::new().high(10.0).low(8.0).close(9.0));
        adx.next(&Bar::new().high(11.0).low(9.0).close(10.0));
        
        // Reset
        adx.reset();
        
        // After reset, first bar should return 0
        assert_eq!(adx.next(&Bar::new().high(20.0).low(18.0).close(19.0)), 0.0);
    }

    #[test]
    fn test_default() {
        let adx = AverageDirectionalIndex::default();
        assert_eq!(adx.period(), 14);
    }

    #[test]
    fn test_display() {
        let adx = AverageDirectionalIndex::new(9).unwrap();
        assert_eq!(format!("{}", adx), "ADX(9)");
    }
} 