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

//! A thread-safe queue testing and benchmarking library.

#![warn(missing_copy_implementations, missing_debug_implementations, missing_docs)]

use std::ops::{Range};
use std::time::{Duration};

//================================================
// Macros
//================================================

// queuecheck_bench_latency! _____________________

/// Benchmarks the latency of the supplied queue.
///
/// # Example
///
/// The below benchmarks the latency of the unbounded MPMC queue from the standard library by
/// producing 100,000 items using two producer threads which are then consumed by one consumer
/// thread.
///
/// ```
/// # #[macro_use] extern crate queuecheck;
/// # fn main() {
/// use std::sync::mpsc::{self, Receiver, Sender};
///
/// let (producer, consumer) = mpsc::channel();
///
/// let latency = queuecheck_bench_latency!(
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
/// latency.report("mpmc", &[50.0, 70.0, 90.0, 95.0, 99.09]);
/// # }
/// ```
///
/// ## Sample Output
///
/// ```console
/// mpmc
///   produce
///     50%:       239.00ns
///     70%:       253.00ns
///     90%:       278.00ns
///     95%:       294.00ns
///     99%:       970.00ns
///   consume
///     50%:       178.00ns
///     70%:       249.00ns
///     90%:       279.00ns
///     95%:       295.00ns
///     99%:       1_578.00ns
/// ```
#[macro_export]
macro_rules! queuecheck_bench_latency {
    ($pairs:expr, $producers:expr, $consumers:expr, $produce:expr, $consume:expr) => ({
        use std::thread;
        use std::sync::{Arc, Barrier};
        use std::time::{Instant};

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
                m.map(|i| {
                    let start = Instant::now();
                    $produce(&p, i);
                    Instant::now() - start
                }).collect::<Vec<_>>().into_iter()
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
                m.map(|_| {
                    let start = Instant::now();
                    while $consume(&c).is_none() { }
                    Instant::now() - start
                }).collect::<Vec<_>>().into_iter()
            })
        }).collect::<Vec<_>>().into_iter();

        let produce = pthreads.flat_map(|t| t.join().unwrap().map($crate::nanoseconds)).collect();
        let consume = cthreads.flat_map(|t| t.join().unwrap().map($crate::nanoseconds)).collect();
        $crate::Latency::new(produce, consume)
    });
}

// queuecheck_bench_throughput! __________________

/// Benchmarks the throughput of the supplied queue.
///
/// # Example
///
/// The below benchmarks the throughput of the unbounded MPMC queue from the standard library by
/// producing 100,000 items using two producer threads which are then consumed by one consumer
/// thread.
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
/// The below tests the unbounded MPMC queue from the standard library by producing 100,000 items
/// using two producer threads which are then consumed by one consumer thread.
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
// Structs
//================================================

// Data __________________________________________

/// A collection of data.
#[derive(Clone, Debug)]
pub struct Data(Vec<f64>);

impl Data {
    //- Accessors --------------------------------

    /// Returns the percentile with the supplied rank.
    pub fn percentile(&self, rank: f64) -> f64 {
        assert!(rank >= 0.0 && rank <= 100.0, "`rank` must be in the range [0.0, 100.0]");
        self.0[((self.0.len() - 1) as f64 * (rank / 100.0)) as usize]
    }

    //- Accessors --------------------------------

    /// Prints a data report to the console for the percentiles with the supplied ranks.
    fn report(&self, name: &str, ranks: &[f64]) {
        println!("  {}", name);
        for rank in ranks {
            let name = format!("{}%:", rank);
            println!("    {:<10} {}ns", name, thousands(self.percentile(*rank), 2));
        }
    }
}

// Latency _______________________________________

/// A measurement of the latency of a queue.
#[derive(Clone, Debug)]
pub struct Latency {
    /// The enqueue operation latencies in nanoseconds.
    pub produce: Data,
    /// The dequeue operation latencies in nanoseconds.
    pub consume: Data,
}

impl Latency {
    //- Constructors -----------------------------

    /// Constructs a new `Latency`.
    pub fn new(mut produce: Vec<f64>, mut consume: Vec<f64>) -> Self {
        produce.sort_by(|a, b| a.partial_cmp(b).unwrap());
        consume.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Latency { produce: Data(produce), consume: Data(consume) }
    }

    //- Accessors --------------------------------

    /// Prints a latency report to the console for the percentiles with the supplied ranks.
    pub fn report(&self, name: &str, ranks: &[f64]) {
        println!("{}", name);
        self.produce.report("produce", ranks);
        self.consume.report("consume", ranks);
    }
}

//================================================
// Functions
//================================================

/// Returns the supplied number formatted with thousands separators.
fn thousands(number: f64, precision: usize) -> String {
    let mut string = format!("{:.*}", precision, number);
    let mut index = string.find('.').unwrap();
    while index > 3 {
        index -= 3;
        string.insert(index, '_');
    }
    string
}

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
