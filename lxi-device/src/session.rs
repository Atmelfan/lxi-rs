
pub trait Session {
    /// Return a name or identifying string (session id, remote addr, etc) for this session
    fn session_name(&self) -> String;
} 