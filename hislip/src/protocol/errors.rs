use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone)]
pub enum Error {
    Fatal(FatalErrorCode, &'static [u8]),
    NonFatal(NonFatalErrorCode, &'static [u8]),
}

impl Error {
    fn is_fatal(&self) {
        matches!(self, Self::Fatal(...))
    }
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::Fatal(FatalErrorCode::UnidentifiedError, b"IO Error")
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Fatal(err, _msg) => write!(f, "Fatal {}", err.error_code()),
            Error::NonFatal(err, _msg) => write!(f, "NonFatal {}", err.error_code()),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FatalErrorCode {
    UnidentifiedError,
    PoorlyFormattedMessageHeader,
    AttemptUseWithoutBothChannels,
    InvalidInitialization,
    MaximumClientsExceeded,
    SecureConnectionFailed,
    Extension(u8),
    DeviceDefined(u8),
    // Library specific Device defined errors
    // These error codes shall only be sent by the server
    IoError,
    LockError
}

impl FatalErrorCode {
    pub fn error_code(&self) -> u8 {
        match self {
            FatalErrorCode::UnidentifiedError => 0,
            FatalErrorCode::PoorlyFormattedMessageHeader => 1,
            FatalErrorCode::AttemptUseWithoutBothChannels => 2,
            FatalErrorCode::InvalidInitialization => 3,
            FatalErrorCode::MaximumClientsExceeded => 4,
            FatalErrorCode::SecureConnectionFailed => 5,
            FatalErrorCode::Extension(x) => *x,
            FatalErrorCode::DeviceDefined(x) => *x,

            FatalErrorCode::IoError => 128,
            FatalErrorCode::LockError => 129,
        }
    }

    pub fn from_error_code(code: u8) -> Self {
        match code {
            0 => FatalErrorCode::UnidentifiedError,
            1 => FatalErrorCode::PoorlyFormattedMessageHeader,
            2 => FatalErrorCode::AttemptUseWithoutBothChannels,
            3 => FatalErrorCode::InvalidInitialization,
            4 => FatalErrorCode::MaximumClientsExceeded,
            5 => FatalErrorCode::SecureConnectionFailed,
            6..=127 => FatalErrorCode::Extension(code),
            _ => FatalErrorCode::DeviceDefined(code),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum NonFatalErrorCode {
    UnidentifiedError,
    UnrecognizedMessageType,
    UnrecognizedControlCode,
    UnrecognizedVendorDefinedMessage,
    MessageTooLarge,
    AuthenticationFailed,
    Extension(u8),
    DeviceDefined(u8),
}

impl NonFatalErrorCode {
    pub fn error_code(&self) -> u8 {
        match self {
            NonFatalErrorCode::UnidentifiedError => 0,
            NonFatalErrorCode::UnrecognizedMessageType => 1,
            NonFatalErrorCode::UnrecognizedControlCode => 2,
            NonFatalErrorCode::UnrecognizedVendorDefinedMessage => 3,
            NonFatalErrorCode::MessageTooLarge => 4,
            NonFatalErrorCode::AuthenticationFailed => 5,
            NonFatalErrorCode::Extension(x) => *x,
            NonFatalErrorCode::DeviceDefined(x) => *x,
        }
    }

    pub fn from_error_code(code: u8) -> Self {
        match code {
            0 => NonFatalErrorCode::UnidentifiedError,
            1 => NonFatalErrorCode::UnrecognizedMessageType,
            2 => NonFatalErrorCode::UnrecognizedControlCode,
            3 => NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
            4 => NonFatalErrorCode::MessageTooLarge,
            5 => NonFatalErrorCode::AuthenticationFailed,
            6..=127 => NonFatalErrorCode::Extension(code),
            _ => NonFatalErrorCode::DeviceDefined(code),
        }
    }
}
