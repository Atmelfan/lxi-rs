#[derive(Clone)]
pub struct ServerConfig {
    pub vendor_id: u16,
    /// Maximum server message size
    pub max_message_size: u64,
    /// Prefer overlapped data
    pub prefer_overlap: bool,
    /// Maximum allowed number of sessions
    pub max_num_sessions: usize,
    /// Force use of encryption and do do not allow clients to end encryption
    #[cfg(feature="secure-capability")]
    pub encryption_mandatory: bool,
    /// Clients must encrypt/authenticate after initializing the session
    #[cfg(feature="secure-capability")]
    pub initial_encryption: bool,
}

impl ServerConfig {
    pub fn vendor_id(mut self, vendor_id: u16) -> Self {
        self.vendor_id = vendor_id;
        self
    }

    pub fn max_message_size(mut self, max_message_size: u64) -> Self {
        self.max_message_size = max_message_size;
        self
    }

    pub fn max_num_sessions(mut self, max_num_sessions: usize) -> Self {
        self.max_num_sessions = max_num_sessions;
        self
    }

    pub fn prefer_overlap(mut self) -> Self {
        self.prefer_overlap = true;
        self
    }

    pub fn prefer_synchronized(mut self) -> Self {
        self.prefer_overlap = false;
        self
    }

    #[cfg(feature="secure-capability")]
    pub fn encryption_mandatory(mut self, encryption_mandatory: bool) -> Self {
        self.encryption_mandatory = encryption_mandatory;
        self
    }

    #[cfg(feature="secure-capability")]
    pub fn initial_encryption(mut self, initial_encryption: bool) -> Self {
        self.initial_encryption = initial_encryption;
        self
    }

    #[cfg(feature="secure-capability")]
    pub fn is_secure(&self) -> bool {
        return self.encryption_mandatory && self.initial_encryption;
    }

}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0xBEEF,
            max_message_size: 1024 * 1024,
            prefer_overlap: true,
            max_num_sessions: 64,
            #[cfg(feature="secure-capability")]
            encryption_mandatory: false,
            #[cfg(feature="secure-capability")]
            initial_encryption: false,
        }
    }
}
