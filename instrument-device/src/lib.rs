use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

pub trait Instrument {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LockError {
    /// Cannot lock as handle already have this lock
    AlreadyHaveLock,
    /// Already locked by a shared lock
    LockedByShared,
    /// Already locked by a exclusive lock
    LockedByExclusive,
    /// Attempted to release unlocked handle
    NotLocked,
}

#[derive(Debug)]
pub enum InstrLockError<GUARD> {
    LockedByOther,
    Poisoned(GUARD),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LockInfo {
    num_exclusive_locks: usize,
    num_locks: usize,
}

/// Locking mechanism around a instrument.
/// Allows multiple handles to acquire a shared lock or
/// one an exclusive lock.
///
pub struct InstrumentLock<INSTR> {
    instr: Mutex<INSTR>,
    shared_locks: AtomicUsize,
    shared_lock_str: Mutex<String>,
    exclusive_lock: AtomicUsize,
}

impl<INSTR> InstrumentLock<INSTR>
where
    INSTR: Instrument,
{
    /// Wrap instrument
    pub fn new(instr: INSTR) -> Self {
        InstrumentLock {
            instr: Mutex::new(instr),
            shared_locks: AtomicUsize::new(0),
            shared_lock_str: Mutex::new("".to_string()),
            exclusive_lock: AtomicUsize::new(0),
        }
    }

    /// Get a handle to this lock to operate on
    pub fn get_handle(&self) -> InstrumentHandle<INSTR> {
        InstrumentHandle {
            lock: &self,
            has_shared_lock: false,
            has_exclusive_lock: false,
        }
    }

    pub fn get_lock_info(&self) -> LockInfo {
        LockInfo {
            num_exclusive_locks: self.exclusive_lock.load(Ordering::Relaxed),
            num_locks: self.shared_locks.load(Ordering::Relaxed),
        }
    }
}

pub struct InstrumentHandle<'a, INSTR> {
    lock: &'a InstrumentLock<INSTR>,
    has_shared_lock: bool,
    has_exclusive_lock: bool,
}

impl<'a, INSTR> InstrumentHandle<'a, INSTR>
where
    INSTR: Instrument,
{
    /// Acquire an exclusive lock
    ///
    ///
    pub fn acquire_shared(&mut self, lstr: String) -> Result<(), LockError> {
        if self.has_shared_lock {
            // Already have this lock
            Err(LockError::AlreadyHaveLock)
        } else {
            if !self.has_exclusive_lock && self.lock.exclusive_lock.load(Ordering::Relaxed) != 0 {
                // Locked by an exclusive
                Err(LockError::LockedByExclusive)
            } else if self.lock.shared_locks.load(Ordering::Relaxed) != 0 {
                // Locked by a shared
                if *self.lock.shared_lock_str.lock().unwrap() == lstr {
                    self.lock.shared_locks.fetch_add(1, Ordering::Relaxed);
                    self.has_shared_lock = true;
                    Ok(())
                } else {
                    Err(LockError::LockedByShared)
                }
            } else {
                // Not locked
                let mut str = self.lock.shared_lock_str.lock().unwrap();
                *str = lstr;
                self.lock.shared_locks.fetch_add(1, Ordering::Relaxed);
                self.has_shared_lock = true;
                Ok(())
            }
        }
    }

    pub fn release_shared(&mut self) -> Result<(), LockError> {
        if self.has_shared_lock {
            self.lock.shared_locks.fetch_sub(1, Ordering::Relaxed);
            self.has_shared_lock = false;
            Ok(())
        } else {
            Err(LockError::NotLocked)
        }
    }

    pub fn acquire_exclusive(&mut self) -> Result<(), LockError> {
        if self.has_exclusive_lock {
            // Already have this lock
            Err(LockError::AlreadyHaveLock)
        } else {
            if self.lock.exclusive_lock.load(Ordering::Relaxed) != 0 {
                // Locked by an exclusive
                Err(LockError::LockedByExclusive)
            } else if self.lock.shared_locks.load(Ordering::Relaxed) != 0 {
                // Locked by a shared
                if self.has_shared_lock {
                    // Promote lock
                    self.lock.exclusive_lock.fetch_add(1, Ordering::Relaxed);
                    self.has_exclusive_lock = true;
                    Ok(())
                } else {
                    Err(LockError::LockedByShared)
                }
            } else {
                // Not locked
                self.lock.exclusive_lock.fetch_add(1, Ordering::Relaxed);
                self.has_exclusive_lock = true;
                Ok(())
            }
        }
    }

    pub fn release_exclusive(&mut self) -> Result<(), LockError> {
        if self.has_exclusive_lock {
            self.lock.exclusive_lock.fetch_sub(1, Ordering::Relaxed);
            self.has_exclusive_lock = false;
            Ok(())
        } else {
            Err(LockError::NotLocked)
        }
    }

    pub fn can_lock(&self) -> bool {
        self.has_exclusive_lock
            || (self.has_shared_lock && !self.lock.exclusive_lock.load(Ordering::Relaxed) != 0)
            || self.lock.shared_locks.load(Ordering::Relaxed) == 0
                && self.lock.exclusive_lock.load(Ordering::Relaxed) == 0
    }

    pub fn lock(&self) -> Result<MutexGuard<INSTR>, InstrLockError<MutexGuard<INSTR>>> {
        if self.can_lock() {
            self.lock
                .instr
                .lock()
                .map_err(|e| InstrLockError::Poisoned(e.into_inner()))
        } else {
            Err(InstrLockError::LockedByOther)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Instrument, InstrumentLock, LockError};

    #[derive(Debug)]
    struct MyInstrument;

    impl Instrument for MyInstrument {}

    #[test]
    fn no_lock() {
        let instrument_lock = InstrumentLock::new(MyInstrument);

        let mut handle_1 = instrument_lock.get_handle();
        let mut handle_2 = instrument_lock.get_handle();

        // Can lock
        {
            let _instr = handle_2.lock().unwrap();
        }
        // Can lock
        {
            let _instr = handle_1.lock().unwrap();
        }
    }

    #[test]
    fn exclusive_lock() {
        let instrument_lock = InstrumentLock::new(MyInstrument);

        let mut handle_1 = instrument_lock.get_handle();
        let mut handle_2 = instrument_lock.get_handle();

        assert_eq!(handle_1.acquire_exclusive(), Ok(()));
        assert_eq!(
            handle_2.acquire_exclusive(),
            Err(LockError::LockedByExclusive)
        );
        assert_eq!(
            handle_1.acquire_exclusive(),
            Err(LockError::AlreadyHaveLock)
        );
        assert_eq!(handle_1.release_exclusive(), Ok(()));
        assert_eq!(handle_1.release_exclusive(), Err(LockError::NotLocked));
        assert_eq!(handle_2.acquire_exclusive(), Ok(()));
        // Can lock
        {
            let _instr = handle_2.lock().unwrap();
        }
        // Cannot lock
        {
            let _instr = handle_1.lock().unwrap_err();
        }
        assert_eq!(handle_2.acquire_shared("POTATO".to_string()), Ok(()));
    }

    #[test]
    fn shared_lock() {
        let instrument_lock = InstrumentLock::new(MyInstrument);

        let mut handle_1 = instrument_lock.get_handle();
        let mut handle_2 = instrument_lock.get_handle();
        let mut handle_3 = instrument_lock.get_handle();

        // Lock with shared key "CARROT"
        assert_eq!(handle_1.acquire_shared("CARROT".to_string()), Ok(()));
        assert_eq!(handle_2.acquire_shared("CARROT".to_string()), Ok(()));
        // Try to acquire other locks
        assert_eq!(
            handle_3.acquire_shared("POTATO".to_string()),
            Err(LockError::LockedByShared)
        );
        assert_eq!(handle_3.acquire_exclusive(), Err(LockError::LockedByShared));
        // Try promote lock
        assert_eq!(handle_1.acquire_exclusive(), Ok(()));
        // Only one promote allowed
        assert_eq!(
            handle_2.acquire_exclusive(),
            Err(LockError::LockedByExclusive)
        );
        // Unlock
        assert_eq!(handle_1.release_exclusive(), Ok(()));
        assert_eq!(handle_1.release_shared(), Ok(()));
        assert_eq!(handle_2.release_shared(), Ok(()));
        // New lock
        assert_eq!(handle_3.acquire_shared("POTATO".to_string()), Ok(()));
        assert_eq!(
            handle_2.acquire_shared("CARROT".to_string()),
            Err(LockError::LockedByShared)
        );

        assert_eq!(handle_1.can_lock(), false);
        assert_eq!(handle_2.can_lock(), false);
        assert_eq!(handle_3.can_lock(), true);
    }
}
