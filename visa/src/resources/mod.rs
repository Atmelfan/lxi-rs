#[cfg(feature = "tcpip-socket")]
pub mod tcpip_socket;
#[cfg(feature = "tcpip-vxi11")]
pub mod tcpip_vxi11;
#[cfg(feature = "tcpip-hislip")]
pub mod tcpip_hislip;
#[cfg(feature = "serial")]
pub mod serial;
#[cfg(feature = "usb")]
pub mod usbtmc;

#[derive(Debug)]
#[non_exhaustive]
pub enum ResourceIdentifier {
    /// `TCPIP[board]::host::port::SOCKET`
    #[cfg(feature = "tcpip-socket")]
    TcpIpSocket {
        board: Option<u8>,
        host: String,
        port: u16,
    },
    /// `TCPIP[board]::host[::device_name][::INSTR]`
    #[cfg(any(feature = "tcpip-vxi11", feature = "tcpip-hislip"))]
    TcpipInstr {
        board: Option<u8>,
        host: String,
        device_name: Option<String>,
    },
    /// `ASRLboard[::INSTR]`
    #[cfg(feature = "serial")]
    Serial {
        board: u8
    },
    /// `USB[board]::manufacturer_id::model_code::serial_number[::usb_interface_number][::INSTR]`
    #[cfg(feature = "usb")]
    UsbInstr {
        board: Option<u8>,
        manufacturer_id: u16,
        model_code: u16,
        serial_number: String,
        usb_interface_number: Option<u8>,
    },
    /// `USB[board]::manufacturer_id::model_code::serial_number[::usb_interface_number][::RAW]`
    #[cfg(feature = "usb")]
    UsbRaw {
        board: Option<u8>,
        manufacturer_id: u16,
        model_code: u16,
        serial_number: String,
        usb_interface_number: Option<u8>,
    },
    /// `GPIB[board]::primary_address[::secondary_address][::INSTR]`
    #[cfg(feature = "gpib")]
    Gpib {
        board: u8,
        primary_address: u8,
        secondary_address: Option<u8>
    },
}

impl ResourceIdentifier {
    fn parse_str(name: String) -> Result<Self, ResourceError> {
        let mut toks = name.split("::");
        let dev = toks.next().ok_or(ResourceError::InvalidResourceName)?;
        match dev.to_lowercase().as_str() {
            "tcpip" => {

            },
            "asrl" => {

            },
            "usb" => {

            },
            "gpib" => {

            },
            _ => return Err(ResourceError::NotSupported)
        }
        Err(ResourceError::NotSupported)
    }
}

enum ResourceError {
    /// Resource name is not valid
    InvalidResourceName,
    /// The specified interface is not supported
    NotSupported
}



#[async_trait::async_trait]
pub trait Resource {
    async fn open(&mut self);
    async fn close(&mut self);
}