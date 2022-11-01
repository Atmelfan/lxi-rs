use alloc::{sync::Arc, vec::Vec};
use futures::channel::oneshot::{channel, Receiver, Sender};

pub use futures::lock::{Mutex, MutexGuard};
pub use spin::Mutex as SpinMutex;

/// An error returned by a locking operation
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
    /// Aborted
    Aborted,
}

/// Is the device exclusively locked or shared?
#[derive(Debug)]
pub enum SharedLockMode {
    /// Shared access by multiple clients
    Shared,
    /// Exclusive access by one client
    Exclusive,
}

/// A lock controlling a device which may be accessed by multiple users.
/// A user may acquire a shared or exclusive lock or try to access without any lock.
pub struct SharedLock {
    id_counter: u32,
    shared_lock: Option<Vec<u8>>,
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
            id_counter: 1,
        }))
    }

    /// Get the number of clients that share access to this lock.
    #[must_use]
    pub fn num_shared_locks(&self) -> u32 {
        self.num_shared_locks
    }

    /// Get if a client has exclusive access to this lock.
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

    pub fn next_id(&mut self) -> u32 {
        self.id_counter = self.id_counter.wrapping_add(1);
        self.id_counter
    }
}

/// A handle to a locked resource.
///
/// This will check if the shared lock is available for this handle before locking.
pub struct LockHandle<DEV> {
    id: u32,
    parent: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
    has_shared: bool,
    has_exclusive: bool,
}

impl<DEV> LockHandle<DEV> {
    /// Create a new lock handle for device using a sared lock
    pub fn new(parent: Arc<SpinMutex<SharedLock>>, device: Arc<Mutex<DEV>>) -> Self {
        let id = parent.lock().next_id();
        LockHandle {
            id,
            parent,
            device,
            has_shared: false,
            has_exclusive: false,
        }
    }

    /// Get status about the shared lock
    pub fn lock_info(&self) -> (bool, u32) {
        let shared = self.parent.lock();
        (shared.exclusive_lock(), shared.num_shared_locks())
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

    pub fn try_acquire(&mut self, lockstr: &[u8]) -> Result<(), SharedLockError> {
        if lockstr.is_empty() {
            self.try_acquire_exclusive()
        } else {
            self.try_acquire_shared(lockstr)
        }
    }

    pub async fn async_acquire(&mut self, lockstr: &[u8]) -> Result<(), SharedLockError> {
        if lockstr.is_empty() {
            self.async_acquire_exclusive().await
        } else {
            self.async_acquire_shared(lockstr).await
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
                log::trace!(id=self.id; "Acquired exclusive (previously unlocked)");
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
                    log::trace!(id=self.id; "Acquired exclusive (upgraded from shared)");
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
                            log::trace!("Waiting to acquire exclusive...");
                            let _ = l.await;
                        }
                    }
                }
                Err(err) => {
                    log::trace!("Failed to acquire shared: {err:?}");
                    break Err(err);
                }
            }
        }
    }

    pub fn try_acquire_shared(&mut self, lockstr: &[u8]) -> Result<(), SharedLockError> {
        if self.has_shared {
            return Err(SharedLockError::AlreadyLocked);
        }

        let mut shared = self.parent.lock();

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
                shared.shared_lock = Some(lockstr.to_vec());
                shared.num_shared_locks = 1;
                self.has_shared = true;

                //
                log::trace!(id=self.id; "Acquired shared (previouly unlocked)");
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
                if key == lockstr {
                    shared.num_shared_locks += 1;
                    self.has_shared = true;

                    //
                    log::trace!(id=self.id; "Acquired shared (previously shared)");
                    shared.notify_acquired();
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
        }
    }

    pub async fn async_acquire_shared(&mut self, lockstr: &[u8]) -> Result<(), SharedLockError> {
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
                            log::trace!("Waiting to acquire shared...");
                            let _ = l.await;
                        }
                    }
                }
                Err(err) => {
                    log::trace!("Failed to acquire shared: {err:?}");
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
        let mut notify = false;

        // Release my shared lock
        if self.has_shared {
            shared.num_shared_locks -= 1;
            if shared.num_shared_locks == 0 {
                shared.shared_lock = None;
                notify = true;
            }
            self.has_shared = false;
            log::trace!(id=self.id; "Released shared");
            res = Ok(SharedLockMode::Shared);
        }

        // Release my exclusive lock
        if self.has_exclusive {
            shared.exclusive_lock = false;
            self.has_exclusive = false;
            notify = true;
            log::trace!(id=self.id; "Released exclusive");
            res = Ok(SharedLockMode::Exclusive);
        }

        // Notify others waiting that lock might be available
        if notify {
            shared.notify_release();
        }

        res
    }

    /// Check if the shared lock is available and then lock
    pub fn try_lock(&self) -> Result<MutexGuard<DEV>, SharedLockError> {
        // Check any active locks
        self.can_lock()?;
        // Lock device and return a guard
        self.device.try_lock().ok_or(SharedLockError::Busy)
    }

    /// Lock device if allowed
    ///
    pub async fn async_lock(&self) -> Result<MutexGuard<DEV>, SharedLockError> {
        let mut listener = None;

        loop {
            match self.can_lock() {
                // Allowed to try and lock
                Ok(()) => {
                    log::trace!(id=self.id; "Can lock, trying...");
                    let mut shared = self.parent.lock();
                    let mut l = shared.listen();
                    drop(shared);

                    futures::select! {
                        // Device acquired
                        guard = self.device.lock() => {
                            log::trace!(id=self.id; "Locked!");
                            return Ok(guard)
                        },
                        // Interrupted by a new lock being granted/released
                        _event = l => {
                            log::trace!(id=self.id; "Lock interrupted, try again");
                            continue
                        }
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

    /// Lock device without checking shared/exclusive lock
    /// NOTE: This shuld ony be used for quick actions like reading status etc to avoid locking
    /// the device for handles holding a legitimate lock.
    pub async fn inner_lock(&self) -> MutexGuard<DEV> {
        self.device.lock().await
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

/// Lock using a remote [LockHandle]
/// Used to wait for a lock while also being able to request/release locks
/// using the remote handle.
pub struct RemoteLockHandle<DEV> {
    handle: Arc<SpinMutex<LockHandle<DEV>>>,
    device: Arc<Mutex<DEV>>,
}

impl<DEV> RemoteLockHandle<DEV> {
    pub fn new(handle: Arc<SpinMutex<LockHandle<DEV>>>) -> Self {
        let device = handle.lock().device.clone();
        Self { handle, device }
    }

    /// Check if the shared lock is available and then lock
    pub async fn try_lock(&self) -> Result<MutexGuard<DEV>, SharedLockError> {
        // Check any active locks
        self.can_lock()?;
        // Lock device and return a guard
        self.device.try_lock().ok_or(SharedLockError::Busy)
    }

    /// Wait for device becoming onlocked (or handle acquiring a lock) and available
    ///
    pub async fn async_lock(&self) -> Result<MutexGuard<DEV>, SharedLockError> {
        let mut listener = None;

        loop {
            match self.can_lock() {
                // Allowed to try and lock
                Ok(()) => {
                    let remote = self.handle.lock();
                    log::trace!(id=remote.id; "Can lock, trying...");

                    let mut shared = remote.parent.lock();
                    let mut l = shared.listen();
                    drop(shared);
                    drop(remote);

                    futures::select! {
                        // Device acquired
                        guard = self.device.lock() => {
                            log::trace!("Locked!");
                            return Ok(guard)
                        },
                        // Interrupted by a new lock being granted/released
                        _event = l => {
                            log::trace!("Lock interrupted, try again");
                            continue
                        }
                    }
                }
                // Currently locked by someone else
                Err(SharedLockError::LockedByShared) | Err(SharedLockError::LockedByExclusive) => {
                    match listener.take() {
                        None => {
                            // Start listening and then try locking again.
                            let remote = self.handle.lock();
                            log::trace!(id=remote.id; "Cannot lock, waiting...");

                            let mut shared = remote.parent.lock();
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

    pub fn can_lock(&self) -> Result<(), SharedLockError> {
        let remote = self.handle.lock();
        remote.can_lock()
    }

    /// Lock device without checking shared/exclusive lock
    /// NOTE: This shuld ony be used for quick actions like reading status etc to avoid locking
    /// the device for handles holding a legitimate lock.
    pub async fn inner_lock(&self) -> MutexGuard<DEV> {
        self.device.lock().await
    }

    pub fn try_acquire(&self, lockstr: &[u8]) -> Result<(), SharedLockError> {
        let mut remote = self.handle.lock();
        remote.try_acquire(lockstr)
    }
    pub async fn async_acquire(&self, lockstr: &[u8]) -> Result<(), SharedLockError> {
        let mut listener = None;

        loop {
            match self.try_acquire(lockstr) {
                Ok(()) => break Ok(()),
                Err(SharedLockError::LockedByShared) | Err(SharedLockError::LockedByExclusive) => {
                    match listener.take() {
                        None => {
                            let remote = self.handle.lock();

                            // Start listening and then try locking again.
                            let mut shared = remote.parent.lock();
                            listener = Some(shared.listen());
                        }
                        Some(l) => {
                            // Wait until a notification is received.
                            log::trace!("Waiting to acquire...");
                            let _ = l.await;
                        }
                    }
                }
                Err(err) => {
                    log::trace!("Failed to acquire: {err:?}");
                    break Err(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{LockHandle, SharedLock, SpinMutex};
    use crate::{lock::RemoteLockHandle, util::EchoDevice};
    use async_std::{sync::Arc, task::yield_now};
    use futures::{join, lock::Mutex};

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
        assert!(handle1.try_acquire_shared(b"foo").is_ok());
        assert!(handle2.try_acquire_shared(b"foo").is_ok());

        // Cannot acquire a shared lock "bar" because "foo" is locked
        assert!(handle2.try_acquire_shared(b"bar").is_err());

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
        assert!(handle1.try_acquire_shared(b"foo").is_ok());
        assert!(handle2.try_acquire_shared(b"foo").is_ok());

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

        // Handle 2 acquires an exclusive lock
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

    #[test]
    fn test_shared_handle() {
        let shared = SharedLock::new();
        let device = Arc::new(Mutex::new(EchoDevice));

        let handle1 = Arc::new(SpinMutex::new(LockHandle::new(
            shared.clone(),
            device.clone(),
        )));
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());

        let remote1 = RemoteLockHandle::new(handle1.clone());
        let remote2 = RemoteLockHandle::new(handle1.clone());

        // Both handles can lock
        assert!(handle1.lock().can_lock().is_ok());
        assert!(remote1.can_lock().is_ok());
        assert!(remote2.can_lock().is_ok());

        // Another handle has lock, none of our can lock
        assert!(handle2.try_acquire_exclusive().is_ok());
        assert!(handle1.lock().can_lock().is_err());
        assert!(remote1.can_lock().is_err());
        assert!(remote2.can_lock().is_err());
    }

    //#[cfg(std)]
    #[async_std::test]
    async fn test_shared_handle_async() {
        femme::with_level(log::LevelFilter::Trace);

        let shared = SharedLock::new();
        let device = Arc::new(Mutex::new(EchoDevice));

        let handle1 = Arc::new(SpinMutex::new(LockHandle::new(
            shared.clone(),
            device.clone(),
        )));
        let remote1 = RemoteLockHandle::new(handle1.clone());

        let mut handle2 = LockHandle::new(shared.clone(), device.clone());

        handle2.try_acquire_shared(b"foo").unwrap();

        //let barrier = Arc::new(Barrier::new(2));

        //let c = barrier.clone();
        let t1 = async_std::task::spawn(async move {
            // Wait until both tasks are running
            //c.wait().await;
            log::info!("t1: Running...");

            let d = remote1.async_lock().await;
            // Wait forever because handle2 has lock...
            assert!(d.is_ok());
            log::info!("t1: locked!");
        });

        //let c = barrier.clone();
        let t2 = async_std::task::spawn(async move {
            // Wait until both tasks are running
            //c.wait().await;
            log::info!("t2: Running...");
            // Let t1 try to get a lock first
            yield_now().await;
            log::info!("t2: Acquiring lock...");
            let d = handle1.lock().async_acquire_shared(b"foo").await;
            // Wait forever because handle2 has lock...
            assert!(d.is_ok());
            log::info!("t2: Lock acquired");
        });

        join!(t1, t2);
    }
}
