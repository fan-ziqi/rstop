//! Sample-aligned projection. Rendering never changes recorded measurements.
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug)]
pub struct Window<'a> {
    pub samples: usize,
    /// Exclusive sequence number, independent of ring-buffer eviction.
    pub end: u64,
    pub time: Option<TimeScale<'a>>,
}

#[derive(Clone, Copy, Debug)]
pub struct TimeScale<'a> {
    pub timestamps: &'a VecDeque<f64>,
    pub periods: &'a VecDeque<f64>,
    pub seconds: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub low: f64,
    pub high: f64,
    pub last: f64,
    pub gap: bool,
}

impl Window<'_> {
    fn stride(self, width: usize) -> usize {
        // Prefer a whole dot-column per refresh. A narrow viewport crops the oldest
        // data instead of squeezing the entire ring into fractional coordinates.
        (self.samples.saturating_add(width / 2) / width.max(1)).max(1)
    }
    pub fn capacity(self, width: usize) -> usize {
        self.samples.min(width.saturating_mul(self.stride(width)))
    }
    /// One sample per half-cell, or fixed, globally aligned buckets when zoomed out.
    /// Startup history is right aligned; adding a sample never stretches old data.
    pub fn project(self, values: &VecDeque<f64>, width: usize) -> Vec<Option<Range>> {
        if let Some(time) = self.time {
            return time.project(values, width);
        }
        let mut result = vec![None::<Range>; width];
        if width == 0 || values.is_empty() {
            return result;
        }
        let samples = self.samples.max(1);
        let stride = self.stride(width) as u64;
        let end = self.end.max(values.len() as u64);
        let last_bucket = (end - 1) / stride;
        let start = end - values.len() as u64;
        let skip = values.len().saturating_sub(samples);
        let mut gaps = vec![false; width];
        for (i, value) in values.iter().enumerate().skip(skip) {
            let offset = last_bucket - (start + i as u64) / stride;
            if offset >= width as u64 {
                continue;
            }
            let x = width - 1 - offset as usize;
            if !value.is_finite() {
                gaps[x] = true;
                if let Some(range) = &mut result[x] {
                    range.gap = true;
                }
            } else if let Some(range) = &mut result[x] {
                range.low = range.low.min(*value);
                range.high = range.high.max(*value);
                range.last = *value;
            } else {
                result[x] = Some(Range {
                    low: *value,
                    high: *value,
                    last: *value,
                    gap: gaps[x],
                });
            }
        }
        result
    }
}

impl TimeScale<'_> {
    fn project(self, values: &VecDeque<f64>, width: usize) -> Vec<Option<Range>> {
        let mut result = vec![None::<Range>; width];
        if width == 0 || values.is_empty() || self.timestamps.is_empty() {
            return result;
        }
        // Absolute integer time buckets retain their phase as history rolls out.
        let step = (self.seconds.max(1) * 1_000_000)
            .div_ceil(width as u64)
            .max(1);
        let bucket = |time: f64| (time.max(0.) * 1_000_000.).round() as u64 / step;
        let end = bucket(*self.timestamps.back().unwrap());
        let count = values.len().min(self.timestamps.len());
        let mut gaps = vec![false; width];
        let mut previous: Option<(i64, f64, f64, f64)> = None;
        for i in 0..count {
            let index = self.timestamps.len() - count + i;
            let time = self.timestamps[index];
            let period = self.periods.get(index).copied().unwrap_or(1.).max(0.001);
            let value = values[values.len() - count + i];
            let x = width as i64 - 1 - end.saturating_sub(bucket(time)) as i64;
            if !value.is_finite() {
                previous = None;
                if x >= 0 {
                    gaps[x as usize] = true;
                }
                if x >= 0
                    && let Some(range) = &mut result[x as usize]
                {
                    range.gap = true;
                }
                continue;
            }
            if let Some((px, pv, pt, pp)) = previous
                && x > px
                && time - pt <= 2.5 * period.max(pp)
            {
                for col in (px + 1).max(0)..x {
                    let interpolated = pv + (value - pv) * (col - px) as f64 / (x - px) as f64;
                    result[col as usize] = Some(Range {
                        low: interpolated,
                        high: interpolated,
                        last: interpolated,
                        gap: gaps[col as usize],
                    });
                }
            }
            if x >= 0 {
                if let Some(range) = &mut result[x as usize] {
                    range.low = range.low.min(value);
                    range.high = range.high.max(value);
                    range.last = value;
                } else {
                    result[x as usize] = Some(Range {
                        low: value,
                        high: value,
                        last: value,
                        gap: gaps[x as usize],
                    });
                }
            }
            previous = Some((x, value, time, period));
        }
        result
    }
}

/// Quantized automatic ranges prevent tiny peak changes from moving every point.
pub fn nice_max(value: f64) -> f64 {
    if !value.is_finite() || value <= 0. {
        return 1.;
    }
    let power = 10f64.powf(value.log10().floor());
    let fraction = value / power;
    let step = if fraction <= 1. {
        1.
    } else if fraction <= 2. {
        2.
    } else if fraction <= 5. {
        5.
    } else {
        10.
    };
    (power * step).max(1.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_and_full_rings_shift_existing_samples_without_stretching() {
        for length in [3, 30, 180] {
            let mut values: VecDeque<_> = (0..length).map(|v| v as f64).collect();
            let before = Window {
                samples: 180,
                end: length,
                time: None,
            }
            .project(&values, 200);
            values.push_back(999.);
            if values.len() > 180 {
                values.pop_front();
            }
            let after = Window {
                samples: 180,
                end: length + 1,
                time: None,
            }
            .project(&values, 200);
            for x in 22..198 {
                assert_eq!(before[x + 1].map(|v| v.last), after[x].map(|v| v.last));
            }
        }
    }
    #[test]
    fn compressed_buckets_keep_extrema_and_phase_after_ring_eviction() {
        let mut values: VecDeque<_> = (0..12).map(|v| v as f64).collect();
        values[8] = 100.;
        let before = Window {
            samples: 12,
            end: 12,
            time: None,
        }
        .project(&values, 4);
        assert_eq!(before[2].unwrap().high, 100.);
        for value in [12., 13., 14.] {
            values.pop_front();
            values.push_back(value);
        }
        let after = Window {
            samples: 12,
            end: 15,
            time: None,
        }
        .project(&values, 4);
        for x in 0..3 {
            assert_eq!(before[x + 1].unwrap().last, after[x].unwrap().last);
        }
    }
    #[test]
    fn a_narrow_viewport_crops_old_data_and_refreshes_from_the_right() {
        let values = (0..170).map(|v| v as f64).collect();
        let window = Window {
            samples: 180,
            end: 170,
            time: None,
        };
        let projected = window.project(&values, 150);
        assert_eq!(window.capacity(150), 150);
        assert_eq!(projected[0].unwrap().last, 20.);
        assert_eq!(projected[149].unwrap().last, 169.);
    }
    #[test]
    fn missing_data_stays_a_gap_and_ranges_do_not_follow_small_peak_changes() {
        let values = [10., f64::NAN, 30.].into();
        let result = Window {
            samples: 3,
            end: 3,
            time: None,
        }
        .project(&values, 3);
        assert!(result[1].is_none());
        let result = Window {
            samples: 3,
            end: 3,
            time: None,
        }
        .project(&values, 1);
        assert!(result[0].unwrap().gap);
        assert_eq!(result[0].unwrap().high, 30.);
        assert_eq!(nice_max(110.), nice_max(150.));
        assert_eq!(nice_max(f64::NAN), 1.);
    }
}
