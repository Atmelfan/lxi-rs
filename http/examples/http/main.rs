use std::time::Duration;

use async_std::future::timeout;
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = 8080)]
    port: u16,

    /// Kill server after timeout (useful for coverage testing)
    #[clap(short, long)]
    timeout: Option<u64>,

    #[clap(short, long, default_value = "User description")]
    user_description: String,
}

mod state {
    use http::lxi::identification::{
        functions::{Function, Subaddress},
        xml::{InstrumentAddressString, Interface, IpType},
        IdentityProvider,
    };

    #[derive(Clone)]
    pub(crate) struct DemoState {
        pub(crate) user_description: String,
    }

    impl IdentityProvider for DemoState {
        fn lxi_version() -> String {
            "1.6".to_string()
        }

        fn manufacturer(&self) -> String {
            "GPA-Robotics".to_string()
        }

        fn model(&self) -> String {
            "HttpDemo".to_string()
        }

        fn serial_number(&self) -> String {
            "0".to_string()
        }

        fn firmware_revision(&self) -> String {
            "0".to_string()
        }

        fn manufacturer_description(&self) -> String {
            "Http demo application".to_string()
        }

        fn homepage_url(&self) -> String {
            "https://github.com/Atmelfan/lxi-rs".to_string()
        }

        fn driver_url(&self) -> String {
            "https://github.com/Atmelfan/lxi-rs".to_string()
        }

        fn host(&self) -> String {
            "localhost".to_string()
        }

        fn extended_functions(&self) -> Vec<http::lxi::identification::functions::Function> {
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

        fn user_description(&self) -> String {
            "User description".to_string()
        }

        fn interfaces(&self) -> Vec<http::lxi::identification::xml::Interface> {
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
                    xsi_type: "NetworkInformation".to_string(),
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
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let mut app = tide::with_state(state::DemoState {
        user_description: args.user_description,
    });

    // lxi pages
    let mut lxi = app.at("/lxi");
    lxi.at("/identification")
        .get(http::lxi::identification::handler);

    let mut api = lxi.at("/api");
    // api.at("common-configuration").get(ep).put(ep);
    // api.at("device-specific-configuration").get(ep).put(ep);
    // api.at("certificates").get(ep);

    // api.at("certificates/:guid").get(ep).delete(ep);
    // api.at("certificates/:guid/enabled").get(ep).put(ep);
    // api.at("get-csr").get(ep);
    // api.at("create-certificate").get(ep);

    log::info!("Running server on {}:{}...", args.ip, args.port);
    if let Some(t) = args.timeout {
        timeout(Duration::from_millis(t), app.listen((args.ip, args.port))).await??;
    } else {
        app.listen((args.ip, args.port)).await?;
    }
    Ok(())
}
