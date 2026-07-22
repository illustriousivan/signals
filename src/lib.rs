use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::IndexSet;

pub struct Callable<T>(Rc<dyn FnMut(T)>);

impl<T> Callable<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnMut(T) + 'static,
    {
        Callable(Rc::new(f))
    }
}

impl<T> Clone for Callable<T> {
    fn clone(&self) -> Self {
        Callable(self.0.clone())
    }
}

impl<T> PartialEq for Callable<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for Callable<T> {}

impl<T> Hash for Callable<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(&self.0) as *const ();
        ptr.hash(state);
    }
}

pub struct Signal<T> {
    callables: indexmap::IndexSet<Callable<T>>,
}

impl<T> Signal<T> {
    pub fn new() -> Self {
        Signal {
            callables: IndexSet::new(),
        }
    }

    pub fn connect(&mut self, callable: Callable<T>) -> Result<(), AlreadyConnected> {
        if self.callables.get_index_of(&callable).is_some() {
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

    #[test]
    fn connect_single() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|x: i32| {
            let _ = x;
        });
        assert!(signal.connect(callable).is_ok());
        assert_eq!(signal.len(), 1);
    }

    #[test]
    fn connect_multiple() {
        let mut signal = Signal::<i32>::new();
        for i in 0..3 {
            let callable = Callable::new(move |x: i32| {
                let _ = (i, x);
            });
            assert!(signal.connect(callable).is_ok());
        }
        assert_eq!(signal.len(), 3);
    }

    #[test]
    fn connect_same_callable_twice() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|x: i32| {
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
        let callable = Callable::new(|x: i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();
        assert_eq!(signal.connect(callable), Err(AlreadyConnected));
    }

    #[test]
    fn len_unchanged_after_duplicate_attempt() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|x: i32| {
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
            let callable = Callable::new(move |x: i32| {
                let _ = (i, x);
            });
            assert!(signal.connect(callable).is_ok());
            assert_eq!(signal.len(), i + 1);
        }
    }

    #[test]
    fn not_connected_by_default() {
        let signal = Signal::<i32>::new();
        let callable = Callable::new(|x: i32| {
            let _ = x;
        });
        assert!(!signal.is_connected(&callable));
    }

    #[test]
    fn connected_returns_true() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|x: i32| {
            let _ = x;
        });
        signal.connect(callable.clone()).ok();
        assert!(signal.is_connected(&callable));
    }

    #[test]
    fn different_callable_returns_false() {
        let mut signal = Signal::<i32>::new();
        let callable1 = Callable::new(|x: i32| {
            let _ = x;
        });
        let callable2 = Callable::new(|x: i32| {
            let _ = x + 1;
        });
        assert!(signal.connect(callable1).is_ok());
        assert!(!signal.is_connected(&callable2));
    }

    #[test]
    fn clone_is_same_connection() {
        let mut signal = Signal::<i32>::new();
        let callable = Callable::new(|x: i32| {
            let _ = x;
        });
        assert!(signal.connect(callable.clone()).is_ok());
        assert!(signal.is_connected(&callable));

        let cloned = callable.clone();
        assert!(signal.is_connected(&cloned));
    }
}
