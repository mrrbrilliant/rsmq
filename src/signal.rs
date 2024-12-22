//! # rust-signals
//!
//! A reactive signal library for Rust that provides both synchronous and asynchronous reactive primitives.
//! This library implements a signal system similar to those found in modern frontend frameworks,
//! but designed specifically for Rust's concurrent and asynchronous ecosystem.
//!
//! ## Features
//!
//! - Synchronous and asynchronous signal operations
//! - Reactive computations (memos)
//! - Effect systems for side effects
//! - Thread-safe design using Arc
//! - Built on tokio for async support
//!
//! ## Example
//!
//! ```rust
//! use rust_signals::{create_signal, create_memo};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a signal with initial value
//!     let count = create_signal(0);
//!
//!     // Create a derived computation
//!     let doubled = create_memo(&count, |value| value * 2);
//!
//!     // Set up an effect
//!     count.effect(|value| {
//!         println!("Count changed to: {}", value);
//!     });
//!
//!     // Update the signal
//!     count.set(1);
//!     assert_eq!(count.get(), 1);
//!     assert_eq!(doubled.get(), 2);
//! }
//! ```

use std::future::Future;
use std::sync::Arc;
use tokio::sync::watch;

/// A reactive signal that can be shared across threads and react to changes.
///
/// Signals are the basic reactive primitive in this library. They wrap a value
/// that can change over time, and provide mechanisms to track and react to those
/// changes.
///
/// # Generic Parameters
///
/// * `T` - The type of value stored in the signal. Must implement `Clone + Send + Sync + 'static`
#[derive(Clone)]
pub struct Signal<T> {
    tx: watch::Sender<T>,
    rx: watch::Receiver<T>,
}

impl<T: Clone + Send + Sync + 'static> Signal<T> {
    /// Creates a new signal with the given initial value.
    ///
    /// Returns an Arc-wrapped Signal for thread-safe sharing.
    ///
    /// # Arguments
    ///
    /// * `initial` - The initial value for the signal
    ///
    /// # Example
    ///
    /// ```rust
    /// let signal = Signal::new(42);
    /// assert_eq!(signal.get(), 42);
    /// ```
    pub fn new(initial: T) -> Arc<Self> {
        let (tx, rx) = watch::channel(initial);
        Arc::new(Self { tx, rx })
    }

    /// Gets the current value of the signal.
    ///
    /// This is a synchronous operation that returns a clone of the current value.
    pub fn get(&self) -> T {
        self.rx.borrow().clone()
    }

    /// Sets a new value for the signal.
    ///
    /// This will notify all listeners of the change.
    ///
    /// # Arguments
    ///
    /// * `value` - The new value to set
    pub fn set(&self, value: T) {
        let _ = self.tx.send(value);
    }

    /// Registers a callback to be called whenever the signal's value changes.
    ///
    /// # Arguments
    ///
    /// * `f` - A function to be called with the new value whenever the signal changes
    ///
    /// # Example
    ///
    /// ```rust
    /// let count = create_signal(0);
    /// count.on_change(|value| {
    ///     println!("Count changed to: {}", value);
    /// });
    /// ```
    pub fn on_change<F>(&self, mut f: F)
    where
        F: FnMut(T) + Send + 'static,
    {
        let mut rx = self.rx.clone();

        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let value = rx.borrow().clone();
                drop(rx.borrow());
                f(value);
            }
        });
    }

    /// Creates an effect that runs immediately and on every change of the signal.
    ///
    /// This is similar to `on_change`, but it also runs once immediately with the
    /// current value.
    ///
    /// # Arguments
    ///
    /// * `f` - A function to be called with the current value and on every change
    pub fn effect<F>(&self, mut f: F)
    where
        F: FnMut(T) + Send + 'static,
    {
        let initial_value = self.get();
        f(initial_value);
        self.on_change(f);
    }

    /// Gets the current value of the signal asynchronously.
    pub async fn get_async(&self) -> T {
        self.rx.borrow().clone()
    }

    /// Sets a new value for the signal asynchronously.
    pub async fn set_async(&self, value: T) {
        let _ = self.tx.send(value);
    }

    /// Registers an async callback to be called whenever the signal's value changes.
    ///
    /// # Arguments
    ///
    /// * `f` - An async function to be called with the new value whenever the signal changes
    pub async fn on_change_async<F, Fut>(&self, f: F)
    where
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut rx = self.rx.clone();

        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let value = rx.borrow().clone();
                drop(rx.borrow());
                f(value).await;
            }
        });
    }

    /// Creates an async effect that runs immediately and on every change of the signal.
    ///
    /// # Arguments
    ///
    /// * `f` - An async function to be called with the current value and on every change
    pub async fn effect_async<F, Fut>(&self, f: F)
    where
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let initial_value = self.get();
        f(initial_value).await;
        self.on_change_async(f).await;
    }
}

/// Creates a new signal with the given initial value.
///
/// This is a convenience function that wraps `Signal::new`.
///
/// # Arguments
///
/// * `initial` - The initial value for the signal
///
/// # Example
///
/// ```rust
/// let count = create_signal(0);
/// assert_eq!(count.get(), 0);
/// ```
pub fn create_signal<T: Clone + Send + Sync + 'static>(initial: T) -> Arc<Signal<T>> {
    Signal::new(initial)
}

/// Creates a new memo that derives its value from a signal.
///
/// A memo is a cached computation that automatically updates when its dependencies change.
///
/// # Arguments
///
/// * `signal` - The signal to derive from
/// * `f` - A function that computes the derived value
///
/// # Example
///
/// ```rust
/// let count = create_signal(0);
/// let doubled = create_memo(&count, |value| value * 2);
/// assert_eq!(doubled.get(), 0);
///
/// count.set(1);
/// assert_eq!(doubled.get(), 2);
/// ```
pub fn create_memo<T, U, F>(signal: &Signal<T>, f: F) -> Arc<Signal<U>>
where
    T: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
    F: Fn(T) -> U + Clone + Send + Sync + 'static,
{
    let memo = Signal::new(f(signal.get()));
    let memo_clone = memo.clone();

    signal.on_change(move |value| {
        memo_clone.set(f(value));
    });

    memo
}

/// Creates a new async memo that derives its value from a signal.
///
/// Similar to `create_memo`, but works with async computations.
///
/// # Arguments
///
/// * `signal` - The signal to derive from
/// * `f` - An async function that computes the derived value
///
/// # Example
///
/// ```rust
/// let count = create_signal(0);
/// let doubled = create_memo_async(&count, |value| async move {
///     tokio::time::sleep(std::time::Duration::from_millis(1)).await;
///     value * 2
/// }).await;
/// ```
pub async fn create_memo_async<T, U, F, Fut>(signal: &Signal<T>, f: F) -> Arc<Signal<U>>
where
    T: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = U> + Send + 'static,
{
    let initial = f(signal.get()).await;
    let memo = Signal::new(initial);
    let memo_clone = memo.clone();

    signal
        .on_change_async(move |value| {
            let memo = memo_clone.clone();
            let f = f.clone();
            async move {
                let new_value = f(value).await;
                memo.set(new_value);
            }
        })
        .await;

    memo
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_sync_and_async() {
        // Test sync methods
        let count = create_signal(0);

        count.effect(|value| {
            println!("Sync effect: {}", value);
        });

        let doubled = create_memo(&count, |value| value * 2);

        count.set(1);
        assert_eq!(count.get(), 1);
        assert_eq!(doubled.get(), 2);

        // Test async methods
        let async_count = create_signal(0);

        async_count
            .effect_async(|value| async move {
                sleep(Duration::from_millis(1)).await;
                println!("Async effect: {}", value);
            })
            .await;

        let async_doubled = create_memo_async(&async_count, |value| async move {
            sleep(Duration::from_millis(1)).await;
            value * 2
        })
        .await;

        async_count.set_async(1).await;
        sleep(Duration::from_millis(10)).await;
        assert_eq!(async_count.get_async().await, 1);
        assert_eq!(async_doubled.get_async().await, 2);
    }
}
