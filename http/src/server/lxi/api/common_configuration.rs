use tide::{Request, Response};

use crate::common::lxi::api::common_configuration::*;

use super::{auth::Permission, middleware::ProblemDetails};

pub trait CommonConfiguration {
    /// Something triggered an 
    fn lan_config_initialize(&mut self) {

    }

    fn hsm_present(&self) -> bool {
        false
    }
}

pub async fn get<S>(req: Request<S>) -> tide::Result
where
    S: CommonConfiguration,
{
    let _guard = req
        .ext::<Permission>()
        .ok_or_else(|| tide::http::format_err!("No api permissions set"))?;
    get_common(req, true).await
}

pub async fn get_common<S>(req: Request<S>, list_users: bool) -> tide::Result
where
    S: CommonConfiguration,
{
    let mut schema = req.url().clone();
    schema.set_path("lxi/schemas/LXICommonConfiguration/1.0");
    let mut response: tide::Response = LxiCommonConfiguration {
        xmlns: "http://lxistandard.org/schemas/LXICommonConfiguration/1.0".to_string(),
        xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
        xsi_schema_location: format!(
            "http://lxistandard.org/schemas/LXICommonConfiguration/1.0 {}",
            schema.as_str()
        ),

        strict: None, //write-only
        hsm_present: Some(req.state().hsm_present()),
        interfaces: vec![Interface {
            name: Some("eth0".to_string()),
            lxi_conformant: Some("LXI Device Specification 1.6".to_string()),
            enabled: Some(true),
            unsecure_mode: Some(false),
            other_unsecure_protocols_enabled: Some(false),
            network: Some(Network {
                ipv4: Some(NetworkIpv4 {
                    enabled: Some(true),
                    auto_ip_enabled: Some(false),
                    dhcp_enabled: Some(false),
                    mdns_enabled: Some(false),
                    dynamic_dns_enabled: Some(false),
                    ping_enabled: Some(true),
                }),
                ipv6: Some(NetworkIpv6 {
                    enabled: Some(true),
                    dhcp_enabled: Some(true),
                    ra_enabled: Some(false),
                    static_address_enabled: Some(false),
                    privacy_mode_enabled: Some(false),
                    mdns_enabled: Some(false),
                    dynamic_dns_enabled: Some(false),
                    ping_enabled: Some(true),
                }),
            }),
            http: Some(vec![Http {
                operation: Some(HttpOperation::Enable),
                port: Some(8080),
                services: Some(vec![Service {
                    name: "Human-Interface".to_string(),
                    enabled: true,
                    basic: None,
                    digest: None,
                }]),
            }]),
            https: Some(vec![Https {
                port: Some(4433),
                client_authentication_required: Some(true),
                services: Some(vec![Service {
                    name: "Human-Interface".to_string(),
                    enabled: true,
                    basic: Some(AuthenticationMechanism {
                        enabled: Some(true),
                    }),
                    digest: None,
                }]),
            }]),
            scpi_raw: Some(vec![ScpiRaw {
                enabled: Some(true),
                port: Some(5025),
                capability: Some(64),
            }]),
            telnet: Some(vec![Telnet {
                enabled: Some(true),
                port: Some(5024),
                tls_required: Some(false),
                client_authentication_required: Some(false),
                capability: Some(64),
            }]),
            scpi_tls: Some(vec![ScpiTls {
                enabled: Some(true),
                port: 5026,
                client_authentication_required: Some(false),
                capability: Some(64),
            }]),
            hislip: Some(Hislip {
                enabled: Some(true),
                port: Some(4880),
                must_start_encrypted: Some(false),
                encryption_mandatory: Some(false),
                client_authentication_mechanisms: Some(ClientAuthenticationMechanisms {
                    anonymous: Some(AuthenticationMechanism { enabled: Some(true) }),
                    plain: Some(AuthenticationMechanism { enabled: Some(true) }),
                    scram: Some(AuthenticationMechanism { enabled: Some(true) }),
                    mtls: Some(AuthenticationMechanism { enabled: Some(true) }),
                }),
            }),
            vxi11: Some(Vxi11 {
                enabled: Some(true),
            }),
        }],
        client_authenticaton: if list_users {
            // Provide userinformation without password/api_access
            Some(ClientAuthentication {
                scram_hash_iteration_count: None,
                scram_channel_binding_required: None,
                client_credential: Some(vec![ClientCredential {
                                    user: "Basil".to_string(),
                                    password: None,
                                    api_access: None,
                                }]),
                client_cert_authentication: None,
            })
        } else {
            // Elided if connectin is insecure
            None
        },
    }
    .to_xml()?
    .into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn put<S>(mut req: Request<S>) -> tide::Result
where
    S: CommonConfiguration,
{
    let permissions = req
        .ext::<Permission>().cloned()
        .ok_or_else(|| tide::http::format_err!("No api permissions set"))?;

    // Deserialize xml
    let xml = req.body_string().await?;
    let config = match LxiCommonConfiguration::from_xml(&xml) {
        Ok(c) => c,
        Err(err) =>  {
            let mut response: tide::Response = tide::http::StatusCode::BadRequest.into();
            response.insert_ext(ProblemDetails::with_detail(err, None));
            return Ok(response);
        },
    };

    // Apply xml
    log::info!("PUT config = {config:?}");
    if matches!(config.client_authenticaton, Some(ClientAuthentication {client_credential: Some(_), .. })) && !permissions.user_management {
        let mut response: tide::Response = tide::http::StatusCode::BadRequest.into();
        response.insert_ext(ProblemDetails::with_detail("User/API-Key is not permitted to modify user credentials", None));
        return Ok(response);
    }

    return Ok(tide::http::StatusCode::Ok.into());
}
