# Signals

A Rust study case implementing a Signal system inspired by [Godot Engine's
Signal architecture](https://docs.godotengine.org/en/stable/tutorials/misc/signal_system.html).

**Goal:** Learn what is necessary to adapt the Signal pattern to Rust — ownership,
lifetimes, interior mutability, and trait design.

## Features

- Generic `Signal<T>` with type-safe callbacks
- Deduplicated connections via pointer identity (`Rc`)
- Insertion-order preservation for callback invocation
- Panic isolation — callbacks that panic don't prevent others from executing
- Single-thread only (uses `Rc`, not `Arc`)

## Quick Start

```rust
use signals::{Signal, Callable};

let mut signal = Signal::<i32>::new();
signal.connect(Callable::new(|&x: &i32| {
    println!("Got: {}", x);
}));

signal.emit(&42); // prints "Got: 42"
```

## Status

Early stage — API surface is stable but not yet published to crates.io.
