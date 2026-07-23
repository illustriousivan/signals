use std::hash::{Hash, Hasher};
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};

use indexmap::IndexSet;

type Consumer<T> = Arc<Mutex<dyn FnMut(&T) + Send>>;

pub struct Callable<T>(Consumer<T>);

impl<T> Callable<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnMut(&T) + 'static + Send,
    {
        Callable(Arc::new(Mutex::new(f)))
    }
}

impl<T> Clone for Callable<T> {
    fn clone(&self) -> Self {
        Callable(self.0.clone())
    }
}

impl<T> PartialEq for Callable<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for Callable<T> {}

impl<T> Hash for Callable<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as *const ();
        ptr.hash(state);
    }
}

pub struct Signal<T> {
    callables: IndexSet<Callable<T>>,
}

impl<T> Signal<T> {
    pub fn new() -> Self {
        Signal {
            callables: IndexSet::new(),
        }
    }

    pub fn connect(&mut self, callable: Callable<T>) -> Result<(), AlreadyConnected> {
        if self.is_connected(&callable) {
            return Err(AlreadyConnected);
        }
        self.callables.insert(callable);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.callables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.callables.is_empty()
    }

    pub fn is_connected(&self, callable: &Callable<T>) -> bool {
        self.callables.contains(callable)
    }

    pub fn emit(&mut self, value: &T) {
        for callable in self.callables.iter() {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                (callable.0.lock().unwrap())(value);
            }));
        }
    }

    pub fn disconnect(&mut self, callable: &Callable<T>) -> bool {
        self.callables.shift_remove(callable)
    }
}

impl<T> Default for Signal<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlreadyConnected;

impl std::fmt::Display for AlreadyConnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "callable already connected")
    }
}

impl std::error::Error for AlreadyConnected {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn connect_single() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        assert!(signal.connect(callable).is_ok());
        assert_eq!(signal.len(), 1);
    }

    #[test]
    fn connect_multiple() {
        let mut signal = Signal::<i32>::new();
        for i in 0..3 {
            let callable = Callable::new(move |&x: &i32| {
                let _ = (i, x);
            });
            assert!(signal.connect(callable).is_ok());
        }
        assert_eq!(signal.len(), 3);
    }

    #[test]
    fn connect_same_callable_twice() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        assert!(signal.connect(callable.clone()).is_ok());
        assert_eq!(signal.connect(callable), Err(AlreadyConnected));
        assert_eq!(signal.len(), 1);
    }

    #[test]
    fn empty_returns_true_on_new_signal() {
        let signal = Signal::<i32>::new();
        assert!(signal.is_empty());
    }

    #[test]
    fn duplicate_returns_err() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();
        assert_eq!(signal.connect(callable), Err(AlreadyConnected));
    }

    #[test]
    fn len_unchanged_after_duplicate_attempt() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();
        assert!(!signal.is_empty());

        signal.connect(Callable::new(|_| {})).ok();
        assert!(!signal.is_empty());

        assert_eq!(signal.connect(callable), Err(AlreadyConnected));
        assert!(!signal.is_empty());
    }

    #[test]
    fn len_after_connects() {
        let mut signal = Signal::<i32>::new();
        for i in 0..5 {
            let callable = Callable::new(move |&x: &i32| {
                let _ = (i, x);
            });
            assert!(signal.connect(callable).is_ok());
            assert_eq!(signal.len(), i + 1);
        }
    }

    #[test]
    fn not_connected_by_default() {
        let signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        assert!(!signal.is_connected(&callable));
    }

    #[test]
    fn connected_returns_true() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();
        assert!(signal.is_connected(&callable));
    }

    #[test]
    fn different_callable_returns_false() {
        let mut signal = Signal::<i32>::new();
        let callable1 = Callable::new(|&x: &i32| {
            let _ = x;
        });
        let callable2 = Callable::new(|&x: &i32| {
            let _ = x + 1;
        });
        assert!(signal.connect(callable1).is_ok());
        assert!(!signal.is_connected(&callable2));
    }

    #[test]
    fn clone_is_same_connection() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        assert!(signal.connect(callable.clone()).is_ok());
        assert!(signal.is_connected(&callable));

        let cloned = callable.clone();
        assert!(signal.is_connected(&cloned));
    }

    #[test]
    fn emit_single_callback_receives_value() {
        let mut signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(0));
        let received_clone = Arc::clone(&received);
        let callable = Callable::new(move |x: &i32| {
            *received_clone.lock().unwrap() = *x;
        });
        signal.connect(callable).ok();

        let value = 42;
        signal.emit(&value);

        assert_eq!(*received.lock().unwrap(), 42);
    }

    #[test]
    fn emit_multiple_callbacks_all_receive_value() {
        let mut signal = Signal::<i32>::new();
        let received1 = Arc::new(Mutex::new(0));
        let r1 = Arc::clone(&received1);
        let received2 = Arc::new(Mutex::new(0));
        let r2 = Arc::clone(&received2);
        let received3 = Arc::new(Mutex::new(0));
        let r3 = Arc::clone(&received3);

        signal
            .connect(Callable::new(move |x: &i32| {
                *r1.lock().unwrap() = *x;
            }))
            .ok();
        signal
            .connect(Callable::new(move |x: &i32| {
                *r2.lock().unwrap() = *x;
            }))
            .ok();
        signal
            .connect(Callable::new(move |x: &i32| {
                *r3.lock().unwrap() = *x;
            }))
            .ok();

        let value = 99;
        signal.emit(&value);

        assert_eq!(*received1.lock().unwrap(), 99);
        assert_eq!(*received2.lock().unwrap(), 99);
        assert_eq!(*received3.lock().unwrap(), 99);
    }

    #[test]
    fn emit_empty_signal_is_safe() {
        let mut signal = Signal::<i32>::new();
        let value = 10;
        signal.emit(&value); // should not panic
    }

    #[test]
    fn emit_correct_value_passed_to_callback() {
        let mut signal = Signal::<String>::new();
        let received = Arc::new(Mutex::new(String::from("")));
        let received_clone = Arc::clone(&received);
        signal
            .connect(Callable::new(move |x: &String| {
                *received_clone.lock().unwrap() = x.clone();
            }))
            .ok();

        let value = String::from("hello");
        signal.emit(&value);

        assert_eq!(*received.lock().unwrap(), "hello");
    }

    #[test]
    fn emit_callbacks_fire_in_insertion_order() {
        let mut signal = Signal::<i32>::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        for i in 0..5 {
            let idx = i;
            let o = Arc::clone(&order);
            signal
                .connect(Callable::new(move |x: &i32| {
                    o.lock().unwrap().push((idx, *x));
                }))
                .ok();
        }

        signal.emit(&100);

        assert_eq!(
            *order.lock().unwrap(),
            vec![(0, 100), (1, 100), (2, 100), (3, 100), (4, 100)]
        );
    }

    #[test]
    fn emit_panic_in_callback_does_not_stop_others() {
        let mut signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = Arc::clone(&received);

        // First callback will panic
        signal
            .connect(Callable::new(|x: &i32| {
                let _ = x;
                panic!("boom");
            }))
            .ok();

        // Second callback should still be called
        signal
            .connect(Callable::new(move |x: &i32| {
                received_clone.lock().unwrap().push(*x);
            }))
            .ok();

        let value = 7;
        signal.emit(&value);

        assert_eq!(*received.lock().unwrap(), vec![7]);
    }

    #[test]
    fn disconnect_existing_returns_true() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();

        assert!(signal.disconnect(&callable));
    }

    #[test]
    fn disconnect_non_connected_returns_false() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });

        assert!(!signal.disconnect(&callable));
    }

    #[test]
    fn disconnect_decreases_len() {
        let mut signal = Signal::<i32>::new();
        let callable1 = Callable::new(|&x: &i32| {
            let _ = x;
        });
        let callable2 = Callable::new(|&x: &i32| {
            let _ = x;
        });

        signal.connect(callable1.clone()).ok();
        signal.connect(callable2).ok();
        assert_eq!(signal.len(), 2);

        signal.disconnect(&callable1);
        assert_eq!(signal.len(), 1);
    }

    #[test]
    fn disconnect_callable_stops_receiving_values() {
        let mut signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = Arc::clone(&received);

        signal
            .connect(Callable::new(move |x: &i32| {
                received_clone.lock().unwrap().push(*x);
            }))
            .ok();

        signal.emit(&10);
        assert_eq!(*received.lock().unwrap(), vec![10]);

        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();

        signal.disconnect(&callable);
        signal.emit(&20);

        assert_eq!(*received.lock().unwrap(), vec![10, 20]);
    }

    #[test]
    fn disconnect_cloned_callable_removes_original() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|&x: &i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();

        assert!(signal.disconnect(&callable));
        assert_eq!(signal.len(), 0);
    }

    #[test]
    fn multi_thread_emit_across_threads() {
        use std::thread;

        let mut signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let r1 = Arc::clone(&received);

        signal.connect(Callable::new(move |x: &i32| {
            r1.lock().unwrap().push(*x);
        })).ok();

        let handle = thread::spawn(move || {
            signal.emit(&42);
        });

        handle.join().unwrap();

        assert_eq!(*received.lock().unwrap(), vec![42]);
    }

    #[test]
    fn multi_thread_concurrent_emits() {
        use std::thread;

        let signal: Arc<Mutex<Signal<i32>>> = Arc::new(Mutex::new(Signal::<i32>::new()));
        let received = Arc::new(Mutex::new(Vec::new()));
        let r1 = Arc::clone(&received);

        {
            let mut s = signal.lock().unwrap();
            s.connect(Callable::new(move |x: &i32| {
                r1.lock().unwrap().push(*x);
            })).ok();
        }

        let handles: Vec<_> = (0..5)
            .map(|i| {
                let sig = Arc::clone(&signal);
                thread::spawn(move || {
                    let mut s = sig.lock().unwrap();
                    s.emit(&(i * 10));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(*received.lock().unwrap(), vec![0, 10, 20, 30, 40]);
    }

    #[test]
    fn callable_is_send() {
        fn is_send<T: Send>() {}
        is_send::<Callable<i32>>();
    }

    #[test]
    fn disconnect_after_thread_safe_changes() {
        use std::thread;

        let signal: Arc<Mutex<Signal<i32>>> = Arc::new(Mutex::new(Signal::<i32>::new()));
        let callable1 = Callable::new(|&x: &i32| {
            let _ = x;
        });
        let callable2 = Callable::new(|&x: &i32| {
            let _ = x;
        });

        {
            let mut s = signal.lock().unwrap();
            s.connect(callable1.clone()).ok();
            s.connect(callable2).ok();
        }

        let sig_clone = Arc::clone(&signal);
        thread::spawn(move || {
            let mut s = sig_clone.lock().unwrap();
            s.emit(&42);
        }).join().unwrap();

        assert!(signal.lock().unwrap().disconnect(&callable1));
        assert_eq!(signal.lock().unwrap().len(), 1);
    }
}
