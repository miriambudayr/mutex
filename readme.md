# Rust Mutex Implementation

This project implements custom mutexes in Rust to explore concurrency and synchronization primitives. The goal is to understand how basic synchronization mechanisms work, particularly for shared state management across multiple threads. The mutexes are implemented from scratch, allowing deeper insights into how they function at a low level.

## Background

The idea for this project stemmed from my experience at Replay.io, where concurrency and managing shared state presented significant challenges. Having come from a purely JavaScript background, I wanted to implement simple concurrency primitives in Rust to understand their inner workings better.

### Why Mutexes?

Mutexes are a fundamental concurrency primitive that ensures mutual exclusion when accessing shared data. This project aims to demonstrate how mutexes work, starting with a basic implementation and progressing to a more sophisticated one that can protect data.

## Project Structure

- **SimplestMutex**: A basic implementation of a mutex that uses an `AtomicBool` to indicate whether the lock is held. It provides basic lock/unlock functionality and handles timeout when attempting to acquire the lock.
- **MutexWithData**: An extension of `SimplestMutex` that holds a piece of data protected by the mutex. This implementation uses `UnsafeCell` to allow mutable access to the data while ensuring exclusive access to it through the lock.

## Implementation Overview

### SimplestMutex

The `SimplestMutex` is the simplest form of a mutex. It uses `AtomicBool` to represent whether the mutex is locked or not. The `lock` method uses a busy-wait loop to acquire the lock, and the `unlock` method releases it.

### MutexWithData

The `MutexWithData` extends the functionality of the `SimplestMutex` by adding a piece of data, which can be modified while the mutex is locked. This data is wrapped in an `UnsafeCell` to allow interior mutability. `UnsafeCell` allows for safe mutable access to the data while maintaining the constraints of the mutex.

### Locking Mechanism

The `lock` method attempts to acquire the lock. If unsuccessful, it will retry until the lock is acquired or a timeout occurs. The `unlock` method releases the lock, and all threads trying to acquire it will be able to do so once it's unlocked.

### Tests

- **Basic Functionality**: Tests that the lock can be acquired and released properly.
- **Infinite Blocking**: Tests the behavior when a thread locks the mutex and doesn't release it.
- **Thread Blocking**: Tests that a second thread is blocked from acquiring the mutex when it's already locked.
- **Data Modification**: Tests that data protected by the mutex can be safely modified across threads.

## Features

- Simple and understandable mutex implementation.
- Thread synchronization with timeouts.
- Protection of shared data with `UnsafeCell` for interior mutability.
- Concurrent thread testing to verify correct locking behavior.

## Usage

To run the tests:

```bash
cargo test
```
