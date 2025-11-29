//! Text output and buffer management for the kernel.

use core::{
    cell::UnsafeCell,
    fmt::{Debug, Write},
    sync::atomic::{AtomicI64, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};

use cake::{OnceMutex, OnceRwLock, RwLock, core_id};

/// A lock-free output buffer for kernel logs and output.
/// More specifically, this acts as a ring buffer that wraps a `Write` implementor. This will provide a lock on the writer, but the buffer itself is lock-free.
#[derive(Debug)]
pub struct OutputBuffer<W, const CAP: usize>
where
    W: Write,
{
    /// The underlying buffer. Access must be done carefully to avoid data races.
    buf: UnsafeCell<[u8; CAP]>,
    /// The write cursor. Indicates where the next write will occur. Lock-free.
    write: Cursor<CAP>,
    /// The read cursor. Indicates where the next read will occur. Lock-free.
    read: Cursor<CAP>,
    /// The commit cursor. Indicates up to where data has been committed for reading.
    ///
    /// This should only be updated when `commit == write`.
    commit: Cursor<CAP>,
    /// The thread that currently holds the lock on this output buffer. -1 if unlocked.
    /// This is used to prevent deadlocks when flushing the buffer.
    ///
    /// Just because a thread holds the lock does not mean it has exclusive access to write to the buffer.
    ///
    /// TODO: In the future, this can be updated to a `CorePool<const MAX_CONCURRENT_CORES: usize>` to allow multiple threads to hold the lock simultaneously.
    /// This can very quickly remove almost any time spent waiting for the lock.
    thread: AtomicI64,
    /// An addend to be added to the commit cursor when the lock is released.
    /// This is used to handle re-entrant writes to the buffer.
    addend: AtomicUsize,
    /// The underlying writer. Protected by a OnceMutex.
    writer: OnceMutex<W>,
}

impl<W, const CAP: usize> OutputBuffer<W, CAP>
where
    W: Write,
{
    /// Creates a new, empty OutputBuffer.
    pub const fn new(writer: W) -> Self {
        Self {
            buf: UnsafeCell::new([0; CAP]),
            write: Cursor::new(),
            read: Cursor::new(),
            commit: Cursor::new(),
            thread: AtomicI64::new(-1),
            addend: AtomicUsize::new(0),
            writer: OnceMutex::new_with(writer),
        }
    }

    /// Creates a new, uninitialized OutputBuffer.
    pub const fn uninitialized() -> Self {
        Self {
            buf: UnsafeCell::new([0; CAP]),
            write: Cursor::new(),
            read: Cursor::new(),
            commit: Cursor::new(),
            thread: AtomicI64::new(-1),
            addend: AtomicUsize::new(0),
            writer: OnceMutex::uninitialized(),
        }
    }

    unsafe fn buf(&self) -> &[u8] {
        unsafe { &*self.buf.get() }
    }

    unsafe fn buf_mut(&self) -> &mut [u8] {
        unsafe { &mut *self.buf.get() }
    }

    /// Attempts to acquire the thread lock for this output buffer.
    fn acquire_thread_lock(&self) -> bool {
        let tid = core_id() as i64;

        while let Err(v) =
            self.thread
                .compare_exchange(-1, tid, Ordering::AcqRel, Ordering::Acquire)
        {
            if v == tid {
                // Already locked on this thread.
                return false;
            }
            core::hint::spin_loop();
        }

        true
    }

    fn release_thread_lock(&self) {
        let tid = core_id() as i64;
        let _ = self
            .thread
            .compare_exchange(tid, -1, Ordering::AcqRel, Ordering::Acquire);
    }

    /// Pushes data into the output buffer. This function is lock-free.
    /// If the buffer is full, it will overwrite the oldest data.
    pub fn push(&self, data: &str) {
        let lock_success = self.acquire_thread_lock();

        let w = self.write.advance(data.len());

        let buf = unsafe { self.buf_mut() };
        for (i, byte) in data.bytes().enumerate() {
            buf[(w + i) % CAP] = byte;
        }

        if !lock_success {
            // We failed to acquire the lock because we are already locked on this thread.
            // This probably means that there was an interrupt or another sporadic event that caused us to re-enter.
            self.addend.fetch_add(data.len(), Ordering::AcqRel);
            return;
        }

        // Wait for previous commit to finish.
        while self.commit.load() != w {
            core::hint::spin_loop();
        }

        self.release_thread_lock();
        self.commit
            .advance(data.len() + self.addend.swap(0, Ordering::AcqRel));
    }

    /// Flushes the output buffer to the underlying writer.
    /// This function acquires a lock on the writer, so it may block.
    /// If the writer is already locked on the current thread, this function will return a `FlushError::Deadlocked` error.
    pub fn flush(&self) -> Result<(), FlushError> {
        // First, acquire both indices. r needs to be updated to w so we use a compare_exchange loop.
        let mut read = self.read.load();
        let mut commit = self.commit.load();
        while let Err(e) = self.read.compare_exchange(read, commit) {
            read = e;
            commit = self.commit.load();
        }

        let buf = unsafe { self.buf() };
        let slice_end: &[u8];
        let slice_start: &[u8];
        if commit >= read {
            // Simple case, no wrap
            slice_start = &buf[read..commit];
            slice_end = &[];
        } else {
            // Wrapped case. Do r..CAP and 0..w
            slice_start = &buf[read..CAP];
            slice_end = &buf[0..commit];
        }

        let mut writer = self.writer.try_get().ok_or(FlushError::Deadlocked)?;

        // This initially seemed really easy to just from_utf8_unchecked, because there is no
        // public api to push non-utf8 data into the buffer. However, it is possible that
        // we got called in the middle of a multi-byte utf8 character being written.
        // To handle this,
        writer
            .write_str(core::str::from_utf8(slice_start).map_err(|_| FlushError::InsufficientData)?)
            .map_err(|_| FlushError::WriteError)?;
        writer
            .write_str(core::str::from_utf8(slice_end).map_err(|_| FlushError::InsufficientData)?)
            .map_err(|_| FlushError::WriteError)?;

        Ok(())
    }
}

unsafe impl<W, const CAP: usize> Sync for OutputBuffer<W, CAP> where W: Write + Send {}
unsafe impl<W, const CAP: usize> Send for OutputBuffer<W, CAP> where W: Write + Send {}

/// An error that can occur when flushing the output buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FlushError {
    /// Occurs when a write error happens while flushing the output buffer.
    #[error("Write error occurred while flushing output buffer")]
    WriteError,
    /// Occurs when attempting to flush the output buffer would cause a deadlock.
    /// More specifically, this condition occurs when the output buffer is locked a second time on the same thread.
    /// Waiting to flush the buffer would cause a deadlock.
    #[error("Attempting to flush would cause a deadlock")]
    Deadlocked,
    /// Occurs when there is not enough data in the buffer to form valid UTF-8.
    #[error("Not enough data in buffer to form valid UTF-8")]
    InsufficientData,
}

/// A lock-free cursor for the output buffer. All reads and writes are Acquire/Release.
#[repr(transparent)]
struct Cursor<const SIZE: usize>(AtomicUsize);

impl<const SIZE: usize> Cursor<SIZE> {
    const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    fn advance(&self, count: usize) -> usize {
        self.0
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pos| {
                Some((pos + count) % SIZE)
            })
            .unwrap()
    }

    fn load(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }

    /// Compare eXchange the cursor position.
    fn compare_exchange(&self, val: usize, new: usize) -> Result<usize, usize> {
        self.0
            .compare_exchange(val, new, Ordering::AcqRel, Ordering::Acquire)
    }
}

impl<const SIZE: usize> Debug for Cursor<SIZE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cursor")
            .field("position", &self.load())
            .field("len", &SIZE)
            .finish()
    }
}
