//! VISA

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
enum LockError<E> where E: VisaError {
    /// Cannot 
    NotSupported,
    /// Locked by another client
    AlreadyLocked,
    /// Cannot unlock a non-existant lock
    NoHeldLock,
    Other(E)
}


#[derive(Debug, Clone, Copy)]
enum LockStatus<E> where E: VisaError {
    Exclusive,
    Shared(String),
    Unlocked,
    Other(E)
}

trait VisaError: Debug {

}

/// Generic VISA resource
pub trait Resource {
    fn name(&self) -> String;

    fn lock(&'mut self, timeout: Duration, key: Option<String>) -> Result<LockStatus, LockError>;
    fn unlock(&'mut self) -> Result<LockStatus, LockError>;

    /// Close resource
    fn close(mut self) -> Result<(), IoError>;
}

/// A message-based resource
pub trait MessageResource: Resource {
    fn write_raw(&'mut self, data: &[u8]) -> Result<(), IoError>;

}