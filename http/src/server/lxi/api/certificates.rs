use tide::Request;
use uuid::Uuid;

use super::middleware::ProblemDetails;

pub struct Certificate;
pub struct Csr;

pub enum StoredCertificate {
    Cert { data: Certificate, enabled: bool },
    Csr { data: Csr },
}

pub trait CertificateStore {
    fn get_idevid(&self) -> Certificate;

    fn list_certificates(&self) -> Vec<(Uuid, StoredCertificate, bool)>;
    fn get_certificate(&self, guid: &uuid::Uuid) -> StoredCertificate;
    fn add_certificate(&self, guid: &uuid::Uuid, cert: StoredCertificate);
    fn remove_certificate(&self, guid: &uuid::Uuid);
    fn enable_certificate(&self, guid: &uuid::Uuid, enable: bool);
}

pub async fn all<S>(_req: Request<S>) -> tide::Result {
    let mut response: tide::Response = tide::http::StatusCode::NotImplemented.into();
    response.insert_ext(ProblemDetails::with_detail(
        "Certificate management not yet implemented",
        None,
    ));
    return Ok(response);
}
