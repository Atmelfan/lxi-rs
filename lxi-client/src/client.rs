//! VISA

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
enum LockError {
    NotSupported
}

#[derive(Debug, Clone, Copy)]
enum LockStatus {
    Exclusive,
    Shared(String),
    Unlocked
}

/// Generic VISA resource
pub trait Resource {
    fn lock(&'mut self) -> Result<LockStatus, LockError>;
    fn unlock(&'mut self) -> Result<LockStatus, UnlockError>;
}

/// A message-based resource
pub trait MessageResource: Resource {

}

/// TODO: A register-based resource
trait RegisterResource: Resource {
    
}