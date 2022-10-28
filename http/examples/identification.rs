use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    #[clap(short, long, default_value_t = 8080)]
    port: u16,
}

mod state {
    use http::server::lxi::identification::{
        Function, IVISoftwareModuleName, Identification, InstrumentAddressString, Interface,
        IpType, Subaddress,
    };

    #[derive(Clone)]
    pub(crate) struct DemoState;

    impl Identification for DemoState {
        fn lxi_version() -> String {
            "1.6".to_string()
        }

        fn manufacturer(&self) -> String {
            "My Company, Inc.".to_string()
        }

        fn model(&self) -> String {
            "EX1234".to_string()
        }

        fn serial_number(&self) -> String {
            "543210".to_string()
        }

        fn firmware_revision(&self) -> String {
            "1.2.3a".to_string()
        }

        fn manufacturer_description(&self) -> String {
            "Sample Device".to_string()
        }

        fn homepage_url(&self) -> String {
            "http://www.mycompany.com".to_string()
        }

        fn driver_url(&self) -> String {
            "http://www.mycompany.com".to_string()
        }

        fn ivisoftware_module_name(&self) -> Vec<IVISoftwareModuleName> {
            vec![IVISoftwareModuleName {
                comment: Some("A comment".to_string()),
                name: "Module name".to_string(),
            }]
        }

        fn extended_functions(&self) -> Vec<http::common::lxi::identification::Function> {
            vec![
                Function::Hislip {
                    version: "2.0".to_string(),
                    port: 4880,
                    subaddresses: vec![
                        Subaddress("hislip0".to_string()),
                        Subaddress("hislip1".to_string()),
                    ],
                },
                Function::Vxi11DiscoveryAndIdentification {
                    version: "1.1".to_string(),
                },
            ]
        }

        fn interfaces(&self) -> Vec<http::common::lxi::identification::Interface> {
            vec![
                Interface::NetworkInformation {
                    interface_type: "LXI".to_string(),
                    interface_name: Some("eth0".to_string()),
                    ip_type: IpType::Ipv4,
                    instrument_address_strings: vec![
                        InstrumentAddressString {
                            value: "TCPIP::10.1.2.32::INSTR".to_string(),
                        },
                        InstrumentAddressString {
                            value: "TCPIP::10.1.2.32::5000::SOCKET".to_string(),
                        },
                        InstrumentAddressString {
                            value: "TCPIP::10.1.2.32::hislip0::INSTR".to_string(),
                        },
                    ],
                    hostname: "10.1.2.32".to_string(),
                    ip_address: "10.1.2.32".to_string(),
                    subnet_mask: "255.255.255.0".to_string(),
                    mac_address: "00:3F:F8:6A:1A:3A".to_string(),
                    gateway: "10.1.2.1".to_string(),
                    dhcp_enabled: true,
                    auto_ip_enabled: true,
                },
                Interface::InterfaceInformation {
                    interface_type: "MyCompanyCustomNetworkInterface".to_string(),
                    interface_name: Some("MyCompany1".to_string()),
                    instrument_address_strings: vec![InstrumentAddressString {
                        value: "10.1.2.32:5025".to_string(),
                    }],
                },
            ]
        }

        fn user_description(&self) -> String {
            "Demo of Identification Schema".to_string()
        }

        fn domain(&self) -> Option<u8> {
            Some(1)
        }

        fn host(&self) -> String {
            "localhost".to_string()
        }

        fn connected_devices(&self) -> Vec<String> {
            vec!["device0".to_string(), "device2".to_string()]
        }
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let mut app = tide::with_state(state::DemoState);

    app.at("/lxi/identification")
        .get(http::server::lxi::identification::get);
    app.at("/lxi/schemas/LXIIdentification/1.0")
        .get(http::server::lxi::schemas::identification);

    app.listen((args.ip, args.port)).await?;

    Ok(())
}
