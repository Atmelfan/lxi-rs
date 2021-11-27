use async_std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockUpgradableReadGuard};
use core::panic;

pub trait Device {
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8>;
}

pub enum SharedLockError {
    /// Already locked
    AlreadyLocked,
    /// Already unlocked
    AlreadyUnlocked,
    /// Cannot acquire shared lock duw to other shared lock
    LockedByShared,
    /// Cannot aquire exclusive lock due to other exclusive lock
    LockedByExclusive,
    /// Device is used by other session but not locked
    Busy,
}

pub struct SharedLock {
    shared_lock: Option<String>,
    num_shared_locks: u32,
    exclusive_lock: bool,
}

impl SharedLock {
    pub fn new() -> Arc<RwLock<SharedLock>> {
        Arc::new(RwLock::new(SharedLock {
            shared_lock: None,
            num_shared_locks: 0,
            exclusive_lock: false,
        }))
    }
}

pub struct LockHandle {
    parent: Arc<RwLock<SharedLock>>,
    device: Arc<Mutex<dyn Device + Send>>,
    has_shared: bool,
    has_exclusive: bool,
}

impl LockHandle {
    pub fn new(shared: Arc<RwLock<SharedLock>>, device: Arc<Mutex<dyn Device + Send>>) -> Self {
        LockHandle {
            parent: shared,
            device: device,
            has_shared: false,
            has_exclusive: false,
        }
    }

    /// Checks if the device is available to try and lock. I.e. this handle holds a lock, no other session holds an exclusive lock or no locks are active.
    /// Another session may still be using the device if no locks are active.
    pub async fn can_lock(&self) -> bool {
        let shared = self.parent.read().await;
        self.has_exclusive
            || (self.has_shared && !shared.exclusive_lock)
            || (!shared.exclusive_lock && shared.shared_lock.is_none())
    }

    /// Try to acquire an exclusive lock
    /// Returns immediately once it ha polled the lock with success or error
    pub async fn try_acquire_exclusive(&mut self) -> Result<(), SharedLockError> {
        if self.has_exclusive {
            return Err(SharedLockError::AlreadyLocked);
        }

        let shared = self.parent.upgradable_read().await;

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
                let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                shared.exclusive_lock = true;
                self.has_exclusive = true;
                Ok(())
            }
            // Current state: Exclusively locked
            (true, None) => Err(SharedLockError::LockedByExclusive),
            // Current state: Shared lock
            (false, Some(_)) => {
                if self.has_shared {
                    self.has_exclusive = true;
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
            // Current state: Both locks
            (true, Some(_)) => Err(SharedLockError::LockedByExclusive),
        }
    }

    pub async fn try_acquire_shared(&mut self, lockstr: String) -> Result<(), SharedLockError> {
        if self.has_shared {
            return Err(SharedLockError::AlreadyLocked);
        }

        let shared = self.parent.upgradable_read().await;

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
                let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                shared.shared_lock = Some(lockstr);
                shared.num_shared_locks = 1;
                self.has_shared = true;
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
                    let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                    shared.num_shared_locks += 1;
                    self.has_shared = true;
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
        }
    }

    pub async fn try_release(&mut self) -> Result<(), SharedLockError> {
        if !self.has_shared && !self.has_exclusive {
            return Err(SharedLockError::AlreadyLocked);
        }

        let shared = self.parent.upgradable_read().await;

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => Err(SharedLockError::AlreadyUnlocked),
            // Current state: Exclusively locked
            (true, None) => {
                if self.has_exclusive {
                    let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                    shared.exclusive_lock = false;
                    self.has_exclusive = false;
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByExclusive)
                }
            }
            // Current state: Shared lock
            (false, Some(_)) => {
                if self.has_shared {
                    let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                    shared.num_shared_locks -= 1;
                    if shared.num_shared_locks == 0 {
                        shared.shared_lock = None;
                    }
                    self.has_shared = false;
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
            // Both locks
            (true, Some(_)) => {
                if self.has_exclusive {
                    let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                    shared.exclusive_lock = false;
                    self.has_exclusive = false;
                    Ok(())
                } else if self.has_shared {
                    let mut shared = RwLockUpgradableReadGuard::upgrade(shared).await;
                    shared.num_shared_locks -= 1;
                    assert!(
                        shared.num_shared_locks != 0,
                        "Both locks but no other shared?"
                    );
                    self.has_shared = false;
                    Ok(())
                } else {
                    Err(SharedLockError::LockedByShared)
                }
            }
        }
    }

    pub async fn try_lock<'a>(&'a self) -> Option<MutexGuard<'a, dyn Device + Send>> {
        if self.can_lock().await {
            Some(self.device.lock().await)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {}
