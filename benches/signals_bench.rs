use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use signals::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

criterion_group!(
    benches,
    connect_benchmark,
    emit_benchmark,
    disconnect_middle_benchmark,
    disconnect_end_benchmark,
    dedup_at_scale_benchmark,
    multi_thread_emit_benchmark,
);
criterion_main!(benches);

fn connect_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("connect");

    for &n in &[1, 10, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut signal = Signal::<i32>::new();
                for i in 0..n {
                    let callable = Callable::new(move |&x: &i32| {
                        let _ = (i, x);
                    });
                    let _ = signal.connect(callable);
                }
            });
        });
    }

    group.finish();
}

fn emit_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("emit");

    for &n in &[1, 10, 100] {
        // Build callables outside the benchmark iteration
        let callables: Vec<_> = (0..n)
            .map(|i| {
                Callable::new(move |&x: &i32| {
                    let _ = x;
                    let _ = black_box(i);
                })
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bencher: &mut _, _: &_| {
            bencher.iter(|| {
                // Fresh signal each iteration to measure pure emit cost
                let mut signal = Signal::<i32>::new();
                for callable in &callables {
                    let _ = signal.connect(callable.clone());
                }

                let value = black_box(42);
                signal.emit(&value);
            });
        });
    }

    group.finish();
}

fn disconnect_middle_benchmark(c: &mut Criterion) {
    c.bench_with_input(
        BenchmarkId::new("disconnect", "middle"),
        &(),
        |b: &mut _, _: &()| {
            b.iter(|| {
                let mut signal = Signal::<i32>::new();
                let mut callables = Vec::new();

                for _i in 0..100 {
                    let callable = Callable::new(|&x: &i32| {
                        let _ = x;
                    });
                    callables.push(callable.clone());
                    let _ = signal.connect(callable);
                }

                signal.disconnect(&callables[50]);
            });
        },
    );
}

fn disconnect_end_benchmark(c: &mut Criterion) {
    c.bench_with_input(
        BenchmarkId::new("disconnect", "end"),
        &(),
        |b: &mut _, _: &()| {
            b.iter(|| {
                let mut signal = Signal::<i32>::new();
                let mut callables = Vec::new();

                for _i in 0..100 {
                    let callable = Callable::new(|&x: &i32| {
                        let _ = x;
                    });
                    callables.push(callable.clone());
                    let _ = signal.connect(callable);
                }

                signal.disconnect(&callables[99]);
            });
        },
    );
}

fn dedup_at_scale_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("dedup_at_scale");

    for &n in &[10, 50, 100, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut signal = Signal::<i32>::new();

                // Pre-connect N unique callables
                for _i in 0..n {
                    let callable = Callable::new(|&x: &i32| {
                        let _ = x;
                    });
                    let _ = signal.connect(callable);
                }

                // Now try to connect a clone of the first callable (duplicate)
                let first_callable = Callable::new(|&x: &i32| {
                    let _ = x;
                });
                let _ = signal.connect(first_callable.clone());
            });
        });
    }

    group.finish();
}

fn multi_thread_emit_benchmark(c: &mut Criterion) {
    let counter = Arc::new(AtomicUsize::new(0));

    // Pre-connect callables that increment the shared counter
    let callables: Vec<_> = (0..10)
        .map(|_| {
            let counter_clone = Arc::clone(&counter);
            Callable::new(move |&x: &i32| {
                let _ = x;
                counter_clone.fetch_add(1, Ordering::Relaxed);
            })
        })
        .collect();

    // Create a fresh signal and connect callables once (outside the measurement)
    let mut signal = Signal::<i32>::new();
    for callable in &callables {
        let _ = signal.connect(callable.clone());
    }

    let sig = Arc::new(std::sync::Mutex::new(signal));

    c.benchmark_group("multi_thread_emit")
        .bench_with_input(
            BenchmarkId::new("multi_thread", 4),
            &(),
            |b: &mut _, _: &()| {
                b.iter(|| {
                    counter.store(0, Ordering::Relaxed);

                    let handles: Vec<_> = (0..4)
                        .map(|_| {
                            let sig_clone = Arc::clone(&sig);
                            thread::spawn(move || {
                                for _ in 0..100 {
                                    let mut s = sig_clone.lock().unwrap();
                                    s.emit(&black_box(42));
                                }
                            })
                        })
                        .collect();

                    for h in handles {
                        h.join().unwrap();
                    }

                    let _ = counter.load(Ordering::Relaxed);
                });
            },
        );
}
