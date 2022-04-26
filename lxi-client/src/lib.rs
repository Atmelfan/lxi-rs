mod resource;

enum LxiError {
    /// Opertion not supported by backend or protocol
    OperationNotSupported,
}

trait LxiProtocol {
    fn protocol(&self) -> String;
}
