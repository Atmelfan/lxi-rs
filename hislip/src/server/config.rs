#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub vendor_id: u16,
    /// Maximum server message size
    pub max_message_size: u64,
    /// Prefer overlapped data
    pub prefer_overlap: bool,
    /// Maximum allowed number of sessions
    pub max_num_sessions: usize,
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0xBEEF,
            max_message_size: 1024 * 1024,
            prefer_overlap: true,
            max_num_sessions: 64,
        }
    }
}