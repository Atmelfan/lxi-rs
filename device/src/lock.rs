use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use futures::{
    channel::oneshot::{channel, Receiver, Sender},
    lock::{Mutex, MutexGuard},
};

pub use spin::Mutex as SpinMutex;

#[derive(Debug)]
pub enum SharedLockError {
    /// Already locked
    AlreadyLocked,
    /// Already unlocked
    AlreadyUnlocked,
    /// Cannot acquire shared lock due to other shared lock
    LockedByShared,
    /// Cannot aquire exclusive lock due to other exclusive lock
    LockedByExclusive,
    /// Device is used by other session but not locked
    Busy,
    /// Timed out
    Timeout,
}

#[derive(Debug)]
pub enum SharedLockMode {
    Shared,
    Exclusive,
}

pub struct SharedLock {
    shared_lock: Option<String>,
    num_shared_locks: u32,
    exclusive_lock: bool,
    event: Vec<Sender<()>>,
}

impl SharedLock {
    pub fn new() -> Arc<SpinMutex<SharedLock>> {
        Arc::new(SpinMutex::new(SharedLock {
            shared_lock: None,
            num_shared_locks: 0,
            exclusive_lock: false,
            event: Vec::new(),
        }))
    }

    /// Get the shared lock's num shared locks.
    #[must_use]
    pub fn num_shared_locks(&self) -> u32 {
        self.num_shared_locks
    }

    /// Get the shared lock's exclusive lock.
    #[must_use]
    pub fn exclusive_lock(&self) -> bool {
        self.exclusive_lock
    }

    fn notify_release(&mut self) {
        for sender in self.event.drain(..) {
            let _ = sender.send(());
        }
    }

    fn notify_acquired(&mut self) {
        for sender in self.event.drain(..) {
            let _ = sender.send(());
        }
    }

    fn listen(&mut self) -> Receiver<()> {
        let (sender, receiver) = channel();
        self.event.push(sender);
        receiver
    }
}

/// A handle to a locked resource.
///
/// You **MUST** call [LockHandle::force_release] when a connection or handle is no
/// longer needed!
pub struct LockHandle<DEV> {
    parent: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
    has_shared: bool,
    has_exclusive: bool,
}

impl<DEV> LockHandle<DEV> {
    pub fn new(parent: Arc<SpinMutex<SharedLock>>, device: Arc<Mutex<DEV>>) -> Self {
        LockHandle {
            parent,
            device,
            has_shared: false,
            has_exclusive: false,
        }
    }

    /// Checks if the device is available to try and lock. I.e. this handle holds a lock, no other session holds an exclusive lock or no locks are active.
    /// Another session may still be using the device if no locks are active.
    pub fn can_lock(&self) -> Result<(), SharedLockError> {
        let shared = self.parent.lock();
        if self.has_exclusive {
            // I have an exclusive lock
            Ok(())
        } else if self.has_shared {
            // I have a shared lock
            if shared.exclusive_lock {
                // Someone else have acquired an exclusive
                Err(SharedLockError::LockedByExclusive)
            } else {
                Ok(())
            }
        } else {
            // I do not have any locks
            // Check if anyone else have one?
            if shared.exclusive_lock {
                Err(SharedLockError::LockedByExclusive)
            } else if shared.num_shared_locks > 0 {
                Err(SharedLockError::LockedByShared)
            } else {
                Ok(())
            }
        }
    }

    /// Try to acquire an exclusive lock
    /// Returns immediately once it ha polled the lock with success or error
    pub fn try_acquire_exclusive(&mut self) -> Result<(), SharedLockError> {
        if self.has_exclusive {
            return Err(SharedLockError::AlreadyLocked);
        }

        let mut shared = self.parent.lock();

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
                shared.exclusive_lock = true;
                self.has_exclusive = true;

                //
                shared.notify_acquired();
                Ok(())
            }
            // Current state: Exclusively locked
            (true, None) => Err(SharedLockError::LockedByExclusive),
            // Current state: Shared lock
            (false, Some(_)) => {
                if self.has_shared {
                    self.has_exclusive = true;
                    shared.exclusive_lock = true;

                    //
                    shared.notify_acquired();
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
            // Current state: Both locks
            (true, Some(_)) => Err(SharedLockError::LockedByExclusive),
        }
    }

    /// Acquire an exclusive lock asynchronously
    pub async fn async_acquire_exclusive(&mut self) -> Result<(), SharedLockError> {
        let mut listener = None;

        loop {
            match self.try_acquire_exclusive() {
                Ok(()) => break Ok(()),
                Err(SharedLockError::LockedByShared) | Err(SharedLockError::LockedByExclusive) => {
                    match listener.take() {
                        None => {
                            // Start listening and then try locking again.
                            let mut shared = self.parent.lock();
                            listener = Some(shared.listen());
                        }
                        Some(l) => {
                            // Wait until a notification is received.
                            let _ = l.await;
                        }
                    }
                }
                Err(err) => {
                    break Err(err);
                }
            }
        }
    }

    pub fn try_acquire_shared(&mut self, lockstr: &str) -> Result<(), SharedLockError> {
        if self.has_shared {
            return Err(SharedLockError::AlreadyLocked);
        }

        let mut shared = self.parent.lock();

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
                shared.shared_lock = Some(lockstr.to_string());
                shared.num_shared_locks = 1;
                self.has_shared = true;

                //
                shared.notify_acquired();
                Ok(())
            }
            // Current state: Exclusively locked
            (true, None) => {
                if self.has_exclusive {
                    Err(SharedLockError::AlreadyLocked)
                } else {
                    Err(SharedLockError::LockedByExclusive)
                }
            }
            // Current state: Shared lock or both locks
            (_, Some(key)) => {
                if key == &lockstr {
                    shared.num_shared_locks += 1;
                    self.has_shared = true;

                    //
                    shared.notify_acquired();
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
        }
    }

    pub async fn async_acquire_shared(&mut self, lockstr: &str) -> Result<(), SharedLockError> {
        let mut listener = None;

        loop {
            match self.try_acquire_shared(lockstr) {
                Ok(()) => break Ok(()),
                Err(SharedLockError::LockedByShared) | Err(SharedLockError::LockedByExclusive) => {
                    match listener.take() {
                        None => {
                            // Start listening and then try locking again.
                            let mut shared = self.parent.lock();
                            listener = Some(shared.listen());
                        }
                        Some(l) => {
                            // Wait until a notification is received.
                            let _ = l.await;
                        }
                    }
                }
                Err(err) => {
                    break Err(err);
                }
            }
        }
    }

    // Release any locks being held.
    // Returns an error if no locks are held by this handle
    pub fn try_release(&mut self) -> Result<SharedLockMode, SharedLockError> {
        let mut shared = self.parent.lock();
        let mut res = Err(SharedLockError::AlreadyUnlocked);

        // Release my shared lock
        if self.has_shared {
            shared.num_shared_locks -= 1;
            if shared.num_shared_locks == 0 {
                shared.shared_lock = None;
            }
            self.has_shared = false;
            res = Ok(SharedLockMode::Shared);
        }

        // Release my exclusive lock
        if self.has_exclusive {
            shared.exclusive_lock = false;
            self.has_exclusive = false;
            res = Ok(SharedLockMode::Exclusive);
        }

        // Notify others waiting that lock might be available
        if res.is_ok() {
            shared.notify_release();
        }

        res
    }

    /// Check if the shared lock is available and then lock
    pub fn try_lock<'a>(&'a self) -> Result<MutexGuard<'a, DEV>, SharedLockError> {
        // Check any active locks
        self.can_lock()?;
        // Lock device and return a guard
        self.device.try_lock().ok_or(SharedLockError::Busy)
    }

    /// Lock device if allowed
    ///
    pub async fn async_lock<'a>(&'a self) -> Result<MutexGuard<'a, DEV>, SharedLockError> {
        let mut listener = None;

        loop {
            match self.can_lock() {
                // Allowed to try and lock
                Ok(()) => {
                    let mut shared = self.parent.lock();
                    let mut l = shared.listen();
                    drop(shared);

                    futures::select! {
                        // Device acquired
                        guard = self.device.lock() => return Ok(guard),
                        // Interrupted by a new lock being granted/released
                        _event = l => continue
                    }
                }
                // Currently locked by someone else
                Err(SharedLockError::LockedByShared) | Err(SharedLockError::LockedByExclusive) => {
                    match listener.take() {
                        None => {
                            // Start listening and then try locking again.
                            let mut shared = self.parent.lock();
                            listener = Some(shared.listen());
                        }
                        Some(l) => {
                            // Wait until a notification is received.
                            let _ = l.await;
                        }
                    }
                }
                // Invalid attempt to lock
                Err(err) => return Err(err),
            }
        }
    }

    /// Force release both shared and exclusive locks
    /// Same as try_release but ignores any error
    pub fn force_release(&mut self) {
        let _res = self.try_release();
    }

    /// Get the lock handle's has shared.
    #[must_use]
    pub fn has_shared(&self) -> bool {
        self.has_shared
    }

    /// Get the lock handle's has exclusive.
    #[must_use]
    pub fn has_exclusive(&self) -> bool {
        self.has_exclusive
    }
}

impl<DEV> Drop for LockHandle<DEV> {
    fn drop(&mut self) {
        self.force_release()
    }
}

#[cfg(test)]
mod tests {

    use super::{LockHandle, SharedLock};
    use crate::util::EchoDevice;
    use alloc::sync::Arc;
    use futures::lock::Mutex;

    #[test]
    fn test_exclusive() {
        let shared = SharedLock::new();
        let device = Arc::new(Mutex::new(EchoDevice));

        let mut handle1 = LockHandle::new(shared.clone(), device.clone());
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());

        // Both handles can lock
        assert!(handle1.can_lock().is_ok());
        assert!(handle2.can_lock().is_ok());

        // Handle 1 acquires an exclusive lock
        assert!(handle1.try_acquire_exclusive().is_ok());

        // Only handle1 can lock
        assert!(handle1.can_lock().is_ok());
        assert!(handle2.can_lock().is_err());

        // Handle2 cannot lock
        assert!(handle2.try_acquire_exclusive().is_err());
    }

    #[test]
    fn test_shared() {
        let shared = SharedLock::new();
        let device = Arc::new(Mutex::new(EchoDevice));

        let mut handle1 = LockHandle::new(shared.clone(), device.clone());
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());
        let handle3 = LockHandle::new(shared.clone(), device.clone());

        // Multiple handles can acquire a shared lock "foo"
        assert!(handle1.try_acquire_shared("foo").is_ok());
        assert!(handle2.try_acquire_shared("foo").is_ok());

        // Cannot acquire a shared lock "bar" because "foo" is locked
        assert!(handle2.try_acquire_shared("bar").is_err());

        // Only "foo" handles may lock
        assert!(handle1.can_lock().is_ok());
        assert!(handle2.can_lock().is_ok());
        assert!(handle3.can_lock().is_err());
    }

    #[test]
    fn test_shared_upgrade() {
        let shared = SharedLock::new();
        let device = Arc::new(Mutex::new(EchoDevice));

        let mut handle1 = LockHandle::new(shared.clone(), device.clone());
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());

        // Multiple handles can acquire a shared lock "foo"
        assert!(handle1.try_acquire_shared("foo").is_ok());
        assert!(handle2.try_acquire_shared("foo").is_ok());

        // Both "foo" handles may lock
        assert!(handle1.can_lock().is_ok());
        assert!(handle2.can_lock().is_ok());

        // Handle1 makes its shared lock exclusive
        assert!(handle1.try_acquire_exclusive().is_ok());

        // Only handle1 can lock using its exclusive
        assert!(handle1.can_lock().is_ok());
        assert!(handle2.can_lock().is_err());

        // Handle1 releases its locks
        assert!(handle1.try_release().is_ok());

        // Both handles have a shared lock
        assert!(handle1.can_lock().is_err());
        assert!(handle2.can_lock().is_ok());
    }

    #[test]
    fn test_drop_releases() {
        let shared = SharedLock::new();
        let device = Arc::new(Mutex::new(EchoDevice));

        let handle1 = LockHandle::new(shared.clone(), device.clone());

        // Both handles can lock
        assert!(handle1.can_lock().is_ok());

        // Handle 1 acquires an exclusive lock
        {
            let mut handle2 = LockHandle::new(shared.clone(), device.clone());
            assert!(handle1.can_lock().is_ok());
            assert!(handle2.can_lock().is_ok());

            assert!(handle2.try_acquire_exclusive().is_ok());

            assert!(handle1.can_lock().is_err());
            assert!(handle2.can_lock().is_ok());
        }

        // Handle1 can lock again
        assert!(handle1.can_lock().is_ok());
    }
}
