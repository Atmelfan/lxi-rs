use std::ops::{Deref, DerefMut};

use tide::Route;

use self::identification::Identification;

pub mod identification;
pub mod schemas;

#[cfg(feature = "lxi-api")]
pub mod api;
#[cfg(feature = "lxi-api")]
pub mod common_configuration;
#[cfg(feature = "lxi-api")]
pub mod device_specific_configuration;

pub struct LxiService<S>(pub tide::Server<S>);

impl<S> LxiService<S>
where
    S: Clone + Send + Sync + 'static,
    // Identification is required
    S: Identification,
{
    pub fn new(inner: S) -> Self {
        let mut inner = tide::Server::with_state(inner);
        // Identification Endpoint
        inner.at("/identification").get(identification::get);
        // Identification schema
        let mut schemas = inner.at("/schemas");
        register_identification_schemas(&mut schemas);

        Self(inner)
    }

    #[cfg(feature = "lxi-api")]
    pub fn new_http_api(inner: S) -> Self
    where
        S: api::common_configuration::CommonConfiguration
            + api::device_specific_configuration::DeviceSpecificConfiguration,
    {
        let mut lxi = tide::Server::with_state(inner);
        lxi.with(api::middleware::LxiProblemDetailsMiddleware);

        // LXI-API endpoints
        lxi.at("/identification").get(identification::get);
        lxi.at("/common-configuration")
            .get(common_configuration::get);
        lxi.at("/device-specific-configuration")
            .get(device_specific_configuration::get);

        let mut api = lxi.at("/api");
        api.at("*").all(|_| async {
            //
            let res: tide::Response = tide::http::StatusCode::ImATeapot.into();
            Ok(res)
        });

        // LXI Schemas
        let mut schemas = lxi.at("/schemas");
        register_identification_schemas(&mut schemas);
        register_api_schemas(&mut schemas);

        Self(lxi)
    }

    #[cfg(feature = "lxi-api")]
    pub fn new_https_api(inner: S) -> Self
    where
        S: api::common_configuration::CommonConfiguration
            + api::device_specific_configuration::DeviceSpecificConfiguration
            + api::auth::LxiApiAuthStorage,
    {
        let mut lxi = tide::Server::with_state(inner);

        // Report errors using LxiProblemDetails XML
        lxi.with(api::middleware::LxiProblemDetailsMiddleware);

        // LXI-API endpoints
        lxi.at("/identification").get(identification::get);
        lxi.at("/common-configuration")
            .get(common_configuration::get);
        lxi.at("/device-specific-configuration")
            .get(device_specific_configuration::get);

        // Requires authentication
        let mut api = lxi.at("/api");
        let api = api.with(api::auth::LxiApiAuthentication);
        api.at("/common-configuration")
            .get(api::common_configuration::get)
            .put(api::common_configuration::put);
        api.at("/device-specific-configuration")
            .get(api::device_specific_configuration::get)
            .put(api::device_specific_configuration::put);
        api.at("/certificates").all(api::certificates::all);
        //     .get(lxi::api::certificates::get)
        //     .post(lxi::api::certificates::post);
        api.at("/certificates/:guid").all(api::certificates::all);
        //     .get(lxi::api::certificates::get_guid)
        //     .delete(lxi::api::certificates::delete_guid);
        api.at("/certificates/:guid/enabled")
            .all(api::certificates::all);
        //     .get(lxi::api::certificates::get_enabled)
        //     .put(lxi::api::certificates::put_enabled);
        api.at("/get-csr").all(api::get_csr::all);
        //     .get(lxi::api::get_csr::get);
        api.at("/create-certificate")
            .all(api::create_certificate::all);
        //     .get(lxi::api::create_certificate::get);

        // LXI Schemas
        let mut schemas = lxi.at("/schemas");
        register_identification_schemas(&mut schemas);
        register_api_schemas(&mut schemas);

        Self(lxi)
    }
}

impl<S> Deref for LxiService<S> {
    type Target = tide::Server<S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for LxiService<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn register_identification_schemas<S>(schemas: &mut Route<S>)
where
    S: Clone + Send + Sync + 'static,
{
    // LXI Schemas
    schemas
        .at("LXIIdentification/1.0")
        .get(schemas::identification);
}

fn register_api_schemas<S>(schemas: &mut Route<S>)
where
    S: Clone + Send + Sync + 'static,
{
    // LXI Schemas
    schemas
        .at("LXIIdentification/1.0")
        .get(schemas::identification);
    schemas
        .at("LXICertificateList/1.0")
        .get(schemas::certificate_list);
    schemas
        .at("LXICertificateRef/1.0")
        .get(schemas::certificate_reference);
    schemas
        .at("LXICertificateRequest/1.0")
        .get(schemas::certificate_request);
    schemas
        .at("LXICommonConfiguration/1.0")
        .get(schemas::common_configuration);
    schemas
        .at("LXIDeviceSpecificConfiguration/1.0")
        .get(schemas::device_specific_configuration);
    schemas.at("LXILiterals/1.0").get(schemas::literals);
    schemas
        .at("LXIPendingDetails/1.0")
        .get(schemas::pending_details);
    schemas
        .at("LXIProblemDetails/1.0")
        .get(schemas::problem_details);
}
