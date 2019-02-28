use std::iter::Iterator;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct DelayTracker {
    queue: VecDeque<f64>,
    n_max: usize
}

impl Default for DelayTracker {
    fn default() -> DelayTracker {
        DelayTracker {
            queue: VecDeque::new(),
            n_max: 100
        }
    }
}

impl DelayTracker {
    pub fn push(&mut self, delay: f64) {
        self.queue.push_back(delay);
        if self.queue.len() >= self.n_max {
            self.queue.pop_front();
        }
    }

    pub fn median(&self) -> Option<f64> {
        let mut data_vec: Vec<f64> = self.queue.iter().map(|x| *x).collect();
        data_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mid = data_vec.len() / 2;

        if mid == 0 {
            None
        }
        else if data_vec.len() % 2 == 0 {
            Some((data_vec[mid - 1] + data_vec[mid]) / 2.0)
        } else {
            Some(data_vec[mid])
        }
    }
}

pub fn variance(data: &Vec<f64>) -> f64 {
    let average = mean(data);

    let power = data
        .iter()
        .map(|x| x.powf(2.0))
        .sum::<f64>() / data.len() as f64;

    power - average.powf(2.0)
}

pub fn mean(data: &Vec<f64>) -> f64 {
    let n_data = data.len() as f64;

    data.iter().sum::<f64>() / n_data
}

pub fn utility(value: f64,
               critic_value: f64,
               tolerance: f64,
               margin: f64) -> f64 {

    // Utility function is a sigmoid guaranteed to cross (critic_value, 0.5) and
    // (critic_value + tolerance, margin)
    1./(1. + ((1. - margin)/margin).powf((value - critic_value)/tolerance))
}
