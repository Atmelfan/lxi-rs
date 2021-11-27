enum LxiError {
    /// Opertion not suppoerted by backend or protocol
    OperationNotSupported,
}

trait LxiProtocol {
    fn protocol(&self) -> String;
}
