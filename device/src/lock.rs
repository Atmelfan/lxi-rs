use alloc::{
    string::String,
    vec::Vec,
    sync::Arc
};
use futures::lock::{Mutex, MutexGuard};

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
    exclusive_lock: bool
}

impl SharedLock {
    pub fn new() -> Arc<Mutex<SharedLock>> {
        Arc::new(Mutex::new(SharedLock {
            shared_lock: None,
            num_shared_locks: 0,
            exclusive_lock: false,
        }))
    }
}

/// A handle to a locked resource.
/// 
/// You **MUST** call [LockHandle::force_release] when a connection or handle is no
/// longer needed!
pub struct LockHandle<DEV> {
    parent: Arc<Mutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
    has_shared: bool,
    has_exclusive: bool,
}

impl<DEV> LockHandle<DEV> {
    pub fn new(shared: Arc<Mutex<SharedLock>>, device: Arc<Mutex<DEV>>) -> Self {
        LockHandle {
            parent: shared,
            device: device,
            has_shared: false,
            has_exclusive: false,
        }
    }

    /// Checks if the device is available to try and lock. I.e. this handle holds a lock, no other session holds an exclusive lock or no locks are active.
    /// Another session may still be using the device if no locks are active.
    pub async fn can_lock(&self) -> Result<(), SharedLockError> {
        let shared = self.parent.lock().await;
        if self.has_exclusive {
            // I have an exclusive lock
            Ok(())
        }else if self.has_shared {
            // I have a shared lock
            if shared.exclusive_lock {
                // Someone else have acquired a exclusive
                Err(SharedLockError::LockedByExclusive)
            } else {
                Ok(())
            }
        }else{
            // I do not have any locks,
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
    pub async fn try_acquire_exclusive(&mut self) -> Result<(), SharedLockError> {
        if self.has_exclusive {
            return Err(SharedLockError::AlreadyLocked);
        }

        let mut shared = self.parent.lock().await;

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
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
                    shared.exclusive_lock = true;
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

        let mut shared = self.parent.lock().await;

        match (shared.exclusive_lock, &shared.shared_lock) {
            // Current state: Unlocked
            (false, None) => {
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
            return Err(SharedLockError::AlreadyUnlocked);
        }

        let mut shared = self.parent.lock().await;

        if self.has_exclusive {
            shared.exclusive_lock = false;
            self.has_exclusive = false;
        }

        if self.has_shared {
            shared.num_shared_locks -= 1;
            if shared.num_shared_locks == 0 {
                shared.shared_lock = None;
            }
            self.has_shared = false;
        }

        Ok(())
    }

    /// 
    pub async fn try_lock<'a>(&'a self) -> Result<MutexGuard<'a, DEV>, SharedLockError> {
        // Check any active locks
        self.can_lock().await?;
        // Lock device and return a guard
        Ok(self.device.lock().await)
    }

    /// Force release both shared and exclusive locks
    /// Same as try_release but ignores any error
    pub async fn force_release(&mut self) {
        self.try_release().await.unwrap_or(())
    }
}

#[cfg(test)]
mod tests {

    use crate::util::EchoDevice;
    use super::{SharedLock, LockHandle};
    use alloc::{string::ToString, sync::Arc};
    use futures::lock::Mutex;
    
    #[async_std::test]
    async fn test_exclusive() {
        let mut shared = SharedLock::new();
        let mut device = Arc::new(Mutex::new(EchoDevice));

        let mut handle1 = LockHandle::new(shared.clone(), device.clone());
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());

        
        // Both handles can lock
        assert!(handle1.can_lock().await.is_ok());
        assert!(handle2.can_lock().await.is_ok());

        // Handle 1 acquires an exclusive lock 
        assert!(handle1.try_acquire_exclusive().await.is_ok());

        // Only handle1 can lock
        assert!(handle1.can_lock().await.is_ok());
        assert!(handle2.can_lock().await.is_err());

        // Handle2 cannot lock
        assert!(handle2.try_acquire_exclusive().await.is_err());
    }

    #[async_std::test]
    async fn test_shared() {
        let mut shared = SharedLock::new();
        let mut device = Arc::new(Mutex::new(EchoDevice));

        let mut handle1 = LockHandle::new(shared.clone(), device.clone());
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());
        let mut handle3 = LockHandle::new(shared.clone(), device.clone());


        // Multiple handles can acquire a shared lock "foo"
        assert!(handle1.try_acquire_shared("foo".to_string()).await.is_ok());
        assert!(handle2.try_acquire_shared("foo".to_string()).await.is_ok());

        // Cannot acquire a shared lock "bar" because "foo" is locked
        assert!(handle2.try_acquire_shared("bar".to_string()).await.is_err());

        // Only "foo" handles may lock
        assert!(handle1.can_lock().await.is_ok());
        assert!(handle2.can_lock().await.is_ok());
        assert!(handle3.can_lock().await.is_err());
    }

    #[async_std::test]
    async fn test_shared_upgrade() {
        let mut shared = SharedLock::new();
        let mut device = Arc::new(Mutex::new(EchoDevice));

        let mut handle1 = LockHandle::new(shared.clone(), device.clone());
        let mut handle2 = LockHandle::new(shared.clone(), device.clone());


        // Multiple handles can acquire a shared lock "foo"
        assert!(handle1.try_acquire_shared("foo".to_string()).await.is_ok());
        assert!(handle2.try_acquire_shared("foo".to_string()).await.is_ok());

        // Both "foo" handles may lock
        assert!(handle1.can_lock().await.is_ok());
        assert!(handle2.can_lock().await.is_ok());

        // Handle1 makes its shared lock exclusive
        assert!(handle1.try_acquire_exclusive().await.is_ok());
    
        // Only handle1 can lock using its exclusive
        assert!(handle1.can_lock().await.is_ok());
        assert!(handle2.can_lock().await.is_err());

        // Handle1 releases its locks
        assert!(handle1.try_release().await.is_ok());

        // Handle2 may lock, handle1 can no longer lock
        assert!(handle1.can_lock().await.is_err());
        assert!(handle2.can_lock().await.is_ok());

    }


}
