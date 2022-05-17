use std::net::Ipv4Addr;

use lxi_vxi11::client::portmapper::prelude::*;

#[async_std::test]
async fn portmap_tcp_null() {
    let mut client = PortMapperClient::connect_tcp((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT))
        .await
        .unwrap();
    let _ = client.null().await.unwrap();
}

#[async_std::test]
async fn portmap_tcp_getport() {
    let mut client = PortMapperClient::connect_tcp((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT))
        .await
        .unwrap();
    let port = client
        .getport(Mapping::new(
            PORTMAPPER_PROG,
            PORTMAPPER_VERS,
            PORTMAPPER_PROT_TCP,
            0,
        ))
        .await
        .unwrap();

    assert_eq!(port, 111);
}

#[async_std::test]
async fn portmap_tcp_set_unset() {
    let mut client = PortMapperClient::connect_tcp((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT))
        .await
        .unwrap();

    let success = client
        .set(Mapping::new(0xDEADBEEF, 1, PORTMAPPER_PROT_TCP, 12345))
        .await
        .unwrap();
    assert_eq!(success, true);

    let success = client
        .unset(Mapping::new(0xDEADBEEF, 1, PORTMAPPER_PROT_TCP, 0))
        .await
        .unwrap();
    assert_eq!(success, true);
}

#[async_std::test]
async fn portmap_udp_null() {
    let mut client = PortMapperClient::connect_udp((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT))
        .await
        .unwrap();
    let _ = client.null().await.unwrap();
}
