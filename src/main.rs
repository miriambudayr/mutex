use std::{
    cell::UnsafeCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

// =============================================================================
// =============================================================================
// ================================ Background =================================
// =============================================================================
// =============================================================================

// Investigative challenge: implement one or more mutexes from scratch

// My experience at Replay.io sparked a deep curiosity about concurrency, particularly in Rust.
// Coming from a purely JS background, I want to implement simple concurrency primitives from scratch
// to better understand them.
// I know time is of the essence, so if you're curious to learn more about why, you can read
// more at the very end of this file.

// =============================================================================
// =============================================================================
// ================================ Simplest Mutex =============================
// =============================================================================
// =============================================================================
pub struct SimplestMutex {
    // more than one thread can interact with an AtomicBool at the same time
    locked: AtomicBool,
}

impl SimplestMutex {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self, timeout: Duration) -> Result<bool, &str> {
        let start = Instant::now();

        // This pattern is called a "busy-wait-loop" (https://en.wikipedia.org/wiki/Busy_waiting).
        // If the lock is highly contended, I think busy-waiting could lead to high CPU usage. I wonder how else this could be done.
        // I am guessing that alternative techniques like sleeping the thread are only worth the overhead if the lock is highly contended,
        // but it seems difficult to predict if a lock will be highly contended without running it in production.
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            if start.elapsed() >= timeout {
                return Err("Timed out");
            }
        }

        return Ok(true);
    }

    // If the lock holder (thread A) fails to unlock the mutex when finished with it, it will block all other threads
    // that are trying to acquire the lock even if thread A doesn't have the mutex in scope or doesn't even exist anymore.
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        return self.locked.load(Ordering::SeqCst);
    }
}

#[test]
// SimplestMutex tests.
fn basic_test() {
    let mutex = SimplestMutex::new();
    mutex.lock(Duration::from_secs(1)).unwrap();
    assert_eq!(mutex.is_locked(), true);
    mutex.unlock();
    assert_eq!(mutex.is_locked(), false);
}

#[cfg(test)]
mod simple_mutex_tests {
    use std::sync::{atomic::AtomicUsize, Barrier};

    use super::*;

    #[test]
    fn test_basic_functionality() {
        let mutex = SimplestMutex::new();

        mutex.lock(Duration::from_secs(1)).unwrap();
        assert_eq!(mutex.is_locked(), true);

        mutex.unlock();
        assert_eq!(mutex.is_locked(), false);
    }

    #[test]
    fn test_infinite_blocking() {
        let mutex = SimplestMutex::new();

        {
            mutex.lock(Duration::from_secs(1)).unwrap();
            assert_eq!(mutex.is_locked(), true);
        }

        // This is expected because unlocking is completely manual.
        assert_eq!(mutex.is_locked(), true);

        mutex.unlock();
    }

    #[test]
    fn test_thread_blocking_behavior() {
        let mutex: Arc<SimplestMutex> = Arc::new(SimplestMutex::new());

        // Barrier to synchronize the start of the second thread
        let barrier = Arc::new(Barrier::new(2));
        let lock_holder = Arc::new(AtomicUsize::new(0)); // 0 = no holder; 1 = thread 1; 2 = thread 2

        // Spawn a thread that locks the mutex but does not unlock it.
        let handle = thread::spawn({
            let m = Arc::clone(&mutex);
            let c = Arc::clone(&barrier);
            let lh = Arc::clone(&lock_holder);

            move || {
                m.lock(Duration::from_secs(1)).unwrap();
                lh.store(1, Ordering::SeqCst);
                println!("Thread 1 acquired lock");
                // Do not unlock the mutex
                // Synchronize with the second thread
                c.wait();
            }
        });

        // Spawn another thread that will attempt to lock the mutex.
        let blocked_handle = thread::spawn({
            let m: Arc<SimplestMutex> = Arc::clone(&mutex);
            let c = Arc::clone(&barrier);
            let lh = Arc::clone(&lock_holder);

            move || {
                // Synchronize with the first thread
                c.wait();
                assert_eq!(m.is_locked(), true);
                assert_eq!(lh.load(Ordering::SeqCst), 1);
                // Attempt to lock the mutex
                let attempt_succeeded = !m.lock(Duration::from_secs(1)).is_err();
                assert_eq!(attempt_succeeded, false);

                if attempt_succeeded {
                    lh.store(2, Ordering::SeqCst);
                }
            }
        });

        // Wait for both threads to finish.
        handle.join().unwrap();
        blocked_handle.join().unwrap();

        // Check that the lock has not been acquired by the second thread.
        assert_eq!(lock_holder.load(Ordering::SeqCst), 1);
        assert_eq!(mutex.is_locked(), true);

        // Ensure the mutex is unlocked at the end.
        mutex.unlock();
        lock_holder.store(0, Ordering::SeqCst);
        assert_eq!(mutex.is_locked(), false);
    }
}

// =============================================================================
// =============================================================================
// ================================ Mutex with Data ============================
// =============================================================================
// =============================================================================

pub struct MutexWithData<T> {
    locked: AtomicBool,
    // Can't just use `data: T` because it can't be shared between threads.
    // MutexWithData lock-holders must have exclusive access to `T`. A more sophisticated mutex could allow for multiple shared accesses (reads) if none of the mutex users need exclusive access.
    data: UnsafeCell<T>,
}

// `UnsafeCell` does not implement `Sync`, so I am implementing it here.
// This is unsafe because usually Rust's borrow-checker does not allow you to mutate data through a shared reference. This relates to the concept of "interior mutability" -- it is a technique you can use to get around the normal borrow-checking rules.
unsafe impl<T> Sync for MutexWithData<T> where T: Send {}

impl<T> MutexWithData<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self, timeout: Duration) -> Result<&mut T, &str> {
        let start = Instant::now();

        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            if start.elapsed() >= timeout {
                return Err("Timed out");
            }
        }

        return Ok(unsafe { &mut *self.data.get() });
    }

    // Only the thread that acquired the lock should ever unlock it.
    // Otherwise, there could be data races or the acquiring thread could assume it has exclusive
    // access to the data.
    // Because `UnsafeCell` allows for interior mutability, the borrow-checker cannot enforce its normal rules and the onus
    // falls on the programmer to properly unlock the mutex.
    pub unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        return self.locked.load(Ordering::SeqCst);
    }
}

#[cfg(test)]
mod mutex_with_data_tests {
    use std::sync::{atomic::AtomicUsize, Barrier};

    use super::*;

    #[test]
    fn basic_test_with_data() {
        let mutex = MutexWithData::new(5);
        {
            let data = mutex.lock(Duration::from_secs(1)).unwrap();
            *data += 1;
            unsafe {
                mutex.unlock();
            }
        }

        let new_data = mutex.lock(Duration::from_secs(1)).unwrap();
        assert_eq!(*new_data, 6);

        unsafe {
            mutex.unlock();
        }
    }

    #[test]
    fn test_infinite_blocking_with_data() {
        let mutex: MutexWithData<i32> = MutexWithData::new(5);

        {
            mutex.lock(Duration::from_secs(1)).unwrap();
        }

        assert_eq!(mutex.is_locked(), true);

        unsafe {
            mutex.unlock();
        }
    }

    #[test]
    fn test_thread_blocking_behavior_with_data() {
        let mutex = Arc::new(MutexWithData::new(5));

        // Barrier to synchronize the start of the second thread
        let barrier = Arc::new(Barrier::new(2));
        let lock_holder = Arc::new(AtomicUsize::new(0)); // 0 = no holder; 1 = thread 1; 2 = thread 2

        // Spawn a thread that locks the mutex but does not unlock it.
        let handle = thread::spawn({
            let m = Arc::clone(&mutex);
            let c = Arc::clone(&barrier);
            let lh = Arc::clone(&lock_holder);

            move || {
                m.lock(Duration::from_secs(1)).unwrap();
                lh.store(1, Ordering::SeqCst);
                println!("Thread 1 acquired lock");
                // Do not unlock the mutex
                // Synchronize with the second thread
                c.wait();
            }
        });

        // Spawn another thread that will attempt to lock the mutex.
        let blocked_handle = thread::spawn({
            let m = Arc::clone(&mutex);
            let c = Arc::clone(&barrier);
            let lh = Arc::clone(&lock_holder);

            move || {
                // Synchronize with the first thread
                c.wait();
                assert_eq!(m.is_locked(), true);
                assert_eq!(lh.load(Ordering::SeqCst), 1);
                // Attempt to lock the mutex
                let attempt_succeeded = !m.lock(Duration::from_secs(1)).is_err();
                assert_eq!(attempt_succeeded, false);

                if attempt_succeeded {
                    lh.store(2, Ordering::SeqCst);
                }
            }
        });

        // Wait for both threads to finish.
        handle.join().unwrap();
        blocked_handle.join().unwrap();

        // Check that the lock has not been acquired by the second thread.
        assert_eq!(lock_holder.load(Ordering::SeqCst), 1);
        assert_eq!(mutex.is_locked(), true);

        // Ensure the mutex is unlocked at the end.
        unsafe {
            mutex.unlock();
        }
        lock_holder.store(0, Ordering::SeqCst);
        assert_eq!(mutex.is_locked(), false);
    }

    #[test]
    fn test_data_modification_across_threads() {
        let mutex = Arc::new(MutexWithData::new(0));

        let handle = thread::spawn({
            let m = Arc::clone(&mutex);
            move || {
                let data = m.lock(Duration::from_secs(1)).unwrap();
                *data += 1;
                unsafe {
                    m.unlock();
                }
            }
        });

        handle.join().unwrap();
        let data = mutex.lock(Duration::from_secs(1)).unwrap();
        assert_eq!(*data, 1);
    }
}

fn main() {
    // intentionally empty
}

// =============================================================================
// =============================================================================
// ================================ Background Continued =======================
// =============================================================================
// =============================================================================

// While I was at Replay.io, we experienced significant difficulty related to concurrency and shared state.
// Our backend (built with Node.js) served users data about their program's Javascript execution which the backend
// requested from the modified Chromium browser runtime. The backend also managed representations of browser/linker
// processes, which helped me understand domain-specific challenges with process management.
// If I could redesign the system from scratch, I would significantly minimize the amount of shared state and
// asynchronous access to it because it led to intractable bugs and generally unsafe programs.
// Node.js lacks robust concurrency primitives for ordering and defining dependencies.
// Some areas of the code were so fragile that even small changes would introduce new, unexpected race conditions.
// We would often fall back to writing all the code synchronously to prevent event loop interruptions, but that was
// still error-prone.
