use std::time::{Duration, Instant};

pub struct Variable {
    /// Maximal duration for taking the average.
    pub duration: Duration,

    /// Last recorded values.
    pub recent_values: Vec<(Instant, f32)>,

    /// Overall average.
    pub average: f32,

    /// Overall number of samples.
    pub num_samples: usize,
}

impl Variable {
    pub fn new(duration: Duration) -> Variable {
        Self {
            duration,
            recent_values: Vec::new(),
            average: 0.0,
            num_samples: 0,
        }
    }

    pub fn record(&mut self, value: f32) {
        let now = Instant::now();
        let duration = self.duration;

        self.recent_values.push((now, value));
        self.recent_values
            .retain(|&(time, _)| now.duration_since(time) < duration);

        // https://math.stackexchange.com/questions/106700/incremental-averageing
        self.average = self.average + (value - self.average) / self.num_samples as f32;
        self.num_samples += 1;
    }

    pub fn recent_average(&self) -> f32 {
        self.recent_values
            .iter()
            .map(|&(_, value)| value)
            .sum::<f32>()
            / self.recent_values.len() as f32
    }
}
