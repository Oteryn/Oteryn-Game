//! Monotonic measurement time and bounded, zero-filled minute charts.
use std::{collections::VecDeque, time::Instant};

#[derive(Debug, Clone)]
pub struct Clock(Instant);
impl Default for Clock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock {
    pub fn seconds(&self) -> u64 {
        self.0.elapsed().as_secs()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Series(VecDeque<(u64, u64)>);
impl Series {
    pub fn add(&mut self, seconds: u64, value: u64) {
        let minute = seconds / 60;
        if let Some((last, total)) = self.0.back_mut() {
            if *last == minute {
                *total = total.saturating_add(value);
                return;
            }
            if *last > minute {
                return;
            }
        }
        self.0.push_back((minute, value));
        while self.0.len() > 60 {
            self.0.pop_front();
        }
    }
    pub fn values(&self, seconds: u64) -> Vec<u64> {
        let end = seconds / 60;
        let start = end.saturating_sub(59);
        let mut values = vec![0; (end - start + 1) as usize];
        for (minute, value) in &self.0 {
            if (start..=end).contains(minute) {
                values[(*minute - start) as usize] = *value;
            }
        }
        values
    }
    pub fn chart(&self, seconds: u64) -> String {
        chart_values(&self.values(seconds))
    }
}
pub fn chart_values(values: &[u64]) -> String {
    let max = values.iter().copied().max().unwrap_or(0);
    let bars: String = values
        .iter()
        .map(|value| {
            let index = (u128::from(*value) * 7 / u128::from(max.max(1))) as usize;
            ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][index]
        })
        .collect();
    lfmt!(
        "{bars}\r\nPer minute · oldest → newest · peak {max} · up to 60 minutes",
        bars = bars,
        max = max
    )
}

pub fn per_hour(value: u64, seconds: u64) -> u64 {
    if seconds == 0 {
        return 0;
    }
    u64::try_from(u128::from(value) * 3600 / u128::from(seconds)).unwrap_or(u64::MAX)
}
pub fn eta_seconds(remaining: u64, gained: u64, seconds: u64) -> Option<u64> {
    if remaining == 0 {
        return Some(0);
    }
    if gained == 0 || seconds == 0 {
        return None;
    }
    u64::try_from((u128::from(remaining) * u128::from(seconds)).div_ceil(u128::from(gained))).ok()
}
