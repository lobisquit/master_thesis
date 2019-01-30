use std::iter::Iterator;
use std::collections::VecDeque;

#[derive(Default, Debug)]
pub struct DelayTracker {
    queue: VecDeque<f64>,
    n_max: usize
}

impl DelayTracker {
    pub fn new(n_max: usize) -> DelayTracker {
        DelayTracker {
            queue: VecDeque::new(),
            n_max: n_max
        }
    }

    pub fn push(&mut self, delay: f64) {
        self.queue.push_back(delay);
        if self.queue.len() == self.n_max {
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
