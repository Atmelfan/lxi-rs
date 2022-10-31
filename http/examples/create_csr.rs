use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectAlternativeName,
    SubjectKeyIdentifier,
};
use openssl::x509::{X509NameBuilder, X509Ref, X509Req, X509ReqBuilder, X509VerifyResult, X509, X509ReqRef};

/// Make a CA certificate and private key
fn mk_ca_cert(serial: u32) -> Result<(X509, PKey<Private>), ErrorStack> {
    let rsa = Rsa::generate(4096)?;
    let key_pair = PKey::from_rsa(rsa)?;

    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", "US")?;
    x509_name.append_entry_by_text("ST", "TX")?;
    x509_name.append_entry_by_text("O", "Some CA organization")?;
    x509_name.append_entry_by_text("CN", "ca test")?;
    let x509_name = x509_name.build();

    let mut cert_builder = X509::builder()?;
    cert_builder.set_version(2)?;
    let serial_number = {
        let serial = BigNum::from_u32(serial)?;
        serial.to_asn1_integer()?
    };
    cert_builder.set_serial_number(&serial_number)?;
    cert_builder.set_subject_name(&x509_name)?;
    cert_builder.set_issuer_name(&x509_name)?;
    cert_builder.set_pubkey(&key_pair)?;
    let not_before = Asn1Time::days_from_now(0)?;
    cert_builder.set_not_before(&not_before)?;
    let not_after = Asn1Time::days_from_now(365)?;
    cert_builder.set_not_after(&not_after)?;

    cert_builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
    cert_builder.append_extension(
        KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()?,
    )?;

    let subject_key_identifier =
        SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(None, None))?;
    cert_builder.append_extension(subject_key_identifier)?;

    cert_builder.sign(&key_pair, MessageDigest::sha256())?;
    let cert = cert_builder.build();

    Ok((cert, key_pair))
}

/// Make a X509 request with the given private key
fn mk_request(key_pair: &PKey<Private>) -> Result<X509Req, ErrorStack> {
    let mut req_builder = X509ReqBuilder::new()?;
    req_builder.set_pubkey(key_pair)?;

    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", "US")?;
    x509_name.append_entry_by_text("ST", "TX")?;
    x509_name.append_entry_by_text("O", "Some organization")?;
    x509_name.append_entry_by_text("CN", "www.example.com")?;
    let x509_name = x509_name.build();
    req_builder.set_subject_name(&x509_name)?;

    req_builder.sign(key_pair, MessageDigest::sha256())?;
    let req = req_builder.build();
    Ok(req)
}

/// Make a certificate and private key signed by the given CA cert and private key
fn mk_ca_signed_cert(
    ca_cert: &X509Ref,
    ca_key_pair: &PKeyRef<Private>,
    req: &X509ReqRef
) -> Result<(X509, PKey<Private>), ErrorStack> {
    let rsa = Rsa::generate(4096)?;
    let key_pair = PKey::from_rsa(rsa)?;

    let mut cert_builder = X509::builder()?;
    cert_builder.set_version(2)?;
    let serial_number = {
        let mut serial = BigNum::new()?;
        serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
        serial.to_asn1_integer()?
    };
    cert_builder.set_serial_number(&serial_number)?;
    cert_builder.set_subject_name(req.subject_name())?;
    cert_builder.set_issuer_name(ca_cert.subject_name())?;
    let pubkey = req.public_key()?;
    cert_builder.set_pubkey(&pubkey)?;
    let not_before = Asn1Time::days_from_now(0)?;
    cert_builder.set_not_before(&not_before)?;
    let not_after = Asn1Time::days_from_now(365)?;
    cert_builder.set_not_after(&not_after)?;

    cert_builder.append_extension(BasicConstraints::new().build()?)?;

    cert_builder.append_extension(
        KeyUsage::new()
            .critical()
            .non_repudiation()
            .digital_signature()
            .key_encipherment()
            .build()?,
    )?;

    let subject_key_identifier =
        SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
    cert_builder.append_extension(subject_key_identifier)?;

    let auth_key_identifier = AuthorityKeyIdentifier::new()
        .keyid(false)
        .issuer(false)
        .build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
    cert_builder.append_extension(auth_key_identifier)?;

    let subject_alt_name = SubjectAlternativeName::new()
        .dns("*.example.com")
        .dns("hello.com")
        .build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
    cert_builder.append_extension(subject_alt_name)?;

    cert_builder.sign(ca_key_pair, MessageDigest::sha256())?;
    let cert = cert_builder.build();

    Ok((cert, key_pair))
}

fn pkcs7() {

}

fn _main() -> Result<(), ErrorStack> {

    // certificates/create-certificate
    let (ldevid, ldevid_key) = mk_ca_cert(1).expect("Failed to create LDevID");
    let ldevid_uuid = uuid::Uuid::new_v4();
    std::fs::write(format!(".certificates/{}.crt", ldevid_uuid), ldevid.to_pem()?).expect("Failed to write");
    std::fs::write(format!(".certificates/{}.key", ldevid_uuid), ldevid_key.private_key_to_pem_pkcs8()?).expect("Failed to write");
    println!("Created LDevID {}", ldevid_uuid);

    // certificates/get-csr
    // Generate and store a new CSR
    let rsa = Rsa::generate(4096)?;
    let key_pair = PKey::from_rsa(rsa)?;
    let csr = mk_request(&key_pair).expect("Failed to create CSR");
    let csr_uuid = uuid::Uuid::new_v4();
    std::fs::write(format!(".certificates/{}.key", csr_uuid), key_pair.private_key_to_pem_pkcs8()?).expect("Failed to write");
    std::fs::write(format!(".certificates/{}.csr", csr_uuid), csr.to_pem()?).expect("Failed to write");
    println!("Created CSR {}", csr_uuid);
    
    // CLIENT SIDE
    let (ca, ca_key) = mk_ca_cert(1).expect("Failed to create LDevID");
    let (signed_cert, _) = mk_ca_signed_cert(&ca, &ca_key, &csr)?;

    // POST certificates
    let csr_pub = csr.public_key()?;
    let cert_pub = signed_cert.public_key()?;
    
    
    let csr_eq_cert = csr_pub.public_eq(&cert_pub);
    println!("CSR == CERT = {csr_eq_cert}");
    println!("CSR.n = {}", csr_pub.rsa()?.n());
    println!("CERT.n = {}", cert_pub.rsa()?.n());

    Ok(())
}

fn main() {
    match _main() {
        Ok(()) => println!("Success!"),
        Err(err) => println!("Error: {err:?}"),
    }
}