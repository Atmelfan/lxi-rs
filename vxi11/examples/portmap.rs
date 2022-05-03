

#[async_std::main]
async fn main() -> io::Result<()> {
    let portmap = StaticPortMapBuilder::new()
        .set(Mapping::new(
            100079,
            1,
            PORTMAPPER_PROT_TCP,
            12345,
        ))
        .set(Mapping::new(
            100079,
            1,
            PORTMAPPER_PROT_UDP,
            12345,
        ))
        .build();
    portmap.serve(IpAddrV4::UNSPECIFIED).await
}