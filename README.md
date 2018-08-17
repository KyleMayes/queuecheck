# queuecheck

[![crates.io](https://img.shields.io/crates/v/queuecheck.svg)](https://crates.io/crates/queuecheck)
[![docs.rs](https://docs.rs/queuecheck/badge.svg)](https://docs.rs/queuecheck)
[![Travis CI](https://travis-ci.org/KyleMayes/queuecheck.svg?branch=master)](https://travis-ci.org/KyleMayes/queuecheck)

A thread-safe queue testing and benchmarking library.

Supported on the stable, beta, and nightly Rust channels.

Released under the Apache License 2.0.

## Examples

### Testing

The below tests the unbounded MPMC queue from the standard library by producing 100,000 items using two producer threads which are then consumed by one consumer thread.

```rust
use std::sync::mpsc::{self, Receiver, Sender};

let (producer, consumer) = mpsc::channel();

queuecheck_test!(
    // enqueue/dequeue operation pairs
    100_000,
    // producer threads
    vec![producer.clone(), producer],
    // consumer threads
    vec![consumer],
    // produce operation
    |p: &Sender<String>, i: String| p.send(i).unwrap(),
    // consume operation
    |c: &Receiver<String>| c.try_recv().ok()
);
```

### Benchmarking

#### Latency

The below benchmarks the latency of the unbounded MPMC queue from the standard library by producing 100,000 items using two producer threads which are then consumed by one consumer thread.

```rust
use std::sync::mpsc::{self, Receiver, Sender};

let (producer, consumer) = mpsc::channel();

let latency = queuecheck_bench_latency!(
    // warmup and measurement enqueue/dequeue operation pairs
    (1_000, 100_000),
    // producer threads
    vec![producer.clone(), producer],
    // consumer threads
    vec![consumer],
    // produce operation
    |p: &Sender<i32>, i: i32| p.send(i).unwrap(),
    // consume operation
    |c: &Receiver<i32>| c.try_recv().ok()
);

println!("produce");
println!("  50%: {:.3}ns", latency.produce.percentile(50.0));
println!("  70%: {:.3}ns", latency.produce.percentile(70.0));
println!("  90%: {:.3}ns", latency.produce.percentile(90.0));
println!("consume");
println!("  50%: {:.3}ns", latency.consume.percentile(50.0));
println!("  70%: {:.3}ns", latency.consume.percentile(70.0));
println!("  90%: {:.3}ns", latency.consume.percentile(90.0));
```

#### Throughput

The below benchmarks the throughput of the unbounded MPMC queue from the standard library by producing 100,000 items using two producer threads which are then consumed by one consumer thread.

```rust
use std::sync::mpsc::{self, Receiver, Sender};

let (producer, consumer) = mpsc::channel();

let ops = queuecheck_bench_throughput!(
    // warmup and measurement enqueue/dequeue operation pairs
    (1_000, 100_000),
    // producer threads
    vec![producer.clone(), producer],
    // consumer threads
    vec![consumer],
    // produce operation
    |p: &Sender<i32>, i: i32| p.send(i).unwrap(),
    // consume operation
    |c: &Receiver<i32>| c.try_recv().ok()
);

println!("{:.3} operation/second", ops);
```
