//! Normal crate documentation goes here.
//!
//! ## Feature flags
#![doc = document_features::document_features!()]

pub mod resources;

enum LxiError {
    /// Opertion not supported by backend or protocol
    OperationNotSupported,
}

trait LxiProtocol {
    fn protocol(&self) -> String;
}

enum Termination {
    /// No line termination
    None,
    /// Null byte `\0`
    Null,
    /// Linefeed or newline character
    Lf,
    /// Control character
    Cr,
    /// Control + linefeed character
    CrLf
}