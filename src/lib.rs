// Copyright 2017 Kyle Mayes
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A queue testing and benchmarking library.

#![warn(missing_copy_implementations, missing_debug_implementations, missing_docs)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", warn(clippy))]

use std::ops::{Range};
use std::time::{Duration};

//================================================
// Macros
//================================================

// queuecheck_bench_throughput! __________________

/// Benchmarks the throughput of the supplied queue.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate queuecheck;
/// # fn main() {
/// use std::sync::mpsc::{self, Receiver, Sender};
///
/// let (producer, consumer) = mpsc::channel();
///
/// let ops = queuecheck_bench_throughput!(
///     // warmup and measurement enqueue/dequeue operation pairs
///     (1_000, 100_000),
///     // producer threads
///     vec![producer.clone(), producer],
///     // consumer threads
///     vec![consumer],
///     // produce operation
///     |p: &Sender<i32>, i: i32| p.send(i).unwrap(),
///     // consume operation
///     |c: &Receiver<i32>| c.try_recv().ok()
/// );
///
/// println!("{:.3} operation/second", ops);
/// # }
/// ```
#[macro_export]
macro_rules! queuecheck_bench_throughput {
    ($pairs:expr, $producers:expr, $consumers:expr, $produce:expr, $consume:expr) => ({
        use std::thread;
        use std::sync::{Arc, Barrier};
        use std::time::{Duration, Instant};

        let (warmup, measurement) = $pairs;
        let producers = $producers;
        let consumers = $consumers;
        let plength = producers.len();
        let clength = consumers.len();

        let barrier = Arc::new(Barrier::new(plength + clength));

        let pwranges = $crate::partition(plength, warmup).into_iter();
        let pmranges = $crate::partition(plength, measurement).into_iter();
        let pthreads = producers.into_iter().zip(pwranges).zip(pmranges).map(|((p, w), m)| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                // Warmup
                for index in w { $produce(&p, index); }
                // Measurement
                let start = Instant::now();
                for index in m { $produce(&p, index); }
                Instant::now() - start
            })
        }).collect::<Vec<_>>().into_iter();

        let cwranges = $crate::partition(clength, warmup).into_iter();
        let cmranges = $crate::partition(clength, measurement).into_iter();
        let cthreads = consumers.into_iter().zip(cwranges).zip(cmranges).map(|((c, w), m)| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                // Warmup
                for _ in w { while $consume(&c).is_none() { } }
                // Measurement
                let start = Instant::now();
                for _ in m { while $consume(&c).is_none() { } }
                Instant::now() - start
            })
        }).collect::<Vec<_>>().into_iter();

        let mut duration = Duration::default();
        duration += pthreads.map(|t| t.join().unwrap()).sum();
        duration += cthreads.map(|t| t.join().unwrap()).sum();
        duration /= (clength + plength) as u32;
        (measurement as f64 / $crate::nanoseconds(duration)) * 1_000_000_000.0
    });
}

// queuecheck_test! ______________________________

/// Tests the supplied queue.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate queuecheck;
/// # fn main() {
/// use std::sync::mpsc::{self, Receiver, Sender};
///
/// let (producer, consumer) = mpsc::channel();
///
/// queuecheck_test!(
///     // enqueue/dequeue operation pairs
///     100_000,
///     // producer threads
///     vec![producer.clone(), producer],
///     // consumer threads
///     vec![consumer],
///     // produce operation
///     |p: &Sender<String>, i: String| p.send(i).unwrap(),
///     // consume operation
///     |c: &Receiver<String>| c.try_recv().ok()
/// );
/// # }
/// ```
#[macro_export]
macro_rules! queuecheck_test {
    ($pairs:expr, $producers:expr, $consumers:expr, $produce:expr, $consume:expr) => ({
        use std::thread;
        use std::sync::{Arc, Barrier};

        let pairs = $pairs;
        let producers = $producers;
        let consumers = $consumers;

        let barrier = Arc::new(Barrier::new(producers.len() + consumers.len()));

        let pranges = $crate::partition(producers.len(), pairs).into_iter();
        let pthreads = producers.into_iter().zip(pranges).map(|(p, r)| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                for index in r { $produce(&p, index.to_string()); }
            })
        }).collect::<Vec<_>>();

        let cranges = $crate::partition(consumers.len(), pairs).into_iter();
        let cthreads = consumers.into_iter().zip(cranges).map(|(c, r)| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                let mut indices = Vec::with_capacity(r.len());
                while indices.len() < r.len() {
                    if let Some(index) = $consume(&c) {
                        match index.parse::<usize>() {
                            Ok(index) => indices.push(index),
                            _ => panic!("invalid index string: {:?}", index),
                        }
                    }
                }
                indices
            })
        }).collect::<Vec<_>>();

        for thread in pthreads { thread.join().unwrap(); }
        let mut indices = Vec::with_capacity(pairs);
        for thread in cthreads { indices.extend(thread.join().unwrap()); }
        indices.sort();

        let expected = (0..pairs).filter(|i| indices.binary_search(i).is_err()).collect::<Vec<_>>();
        let unexpected = indices.iter().cloned().filter(|i| *i >= pairs).collect::<Vec<_>>();
        if !expected.is_empty() || !unexpected.is_empty() {
            panic!("dropped: {:?}, invalid: {:?}", expected, unexpected);
        }
    });
}

//================================================
// Functions
//================================================

/// Returns the supplied duration converted to nanoseconds.
#[doc(hidden)]
pub fn nanoseconds(duration: Duration) -> f64 {
    (duration.as_secs() * 1_000_000_000) as f64 + duration.subsec_nanos() as f64
}

/// Partitions the supplied number of operations into ranges.
#[doc(hidden)]
pub fn partition(threads: usize, operations: usize) -> Vec<Range<i32>> {
    let factor = operations / threads;
    (0..threads).map(|t| {
        let end = if t + 1 == threads { operations } else { factor * (t + 1) };
        ((factor * t) as i32)..(end as i32)
    }).collect()
}
