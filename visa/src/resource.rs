enum ResourceIdentifier {
    Gpib,
    Serial,
    TcpIp,
    Usb,
}

impl ResourceIdentifier {
    fn to_resource_string(&self) -> &str {
        match self {
            ResourceIdentifier::Gpib => "GPIB",
            ResourceIdentifier::Serial => "ASRL",
            ResourceIdentifier::TcpIp => "TCPIP",
            ResourceIdentifier::Usb => "USB",
        }
    }
}
