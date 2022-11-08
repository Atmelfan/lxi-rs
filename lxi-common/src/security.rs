use digest::Digest;

struct ClientAuthentication {}


/// Algorithm used to calculate thumbprint
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum ThumbprintHash {
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
}

impl ThumbprintHash {
    pub fn from_str(hash: &str) -> Option<Self> {
        match hash {
            "sha-1" | "SHA-1" => Some(Self::Sha1),
            "sha-224" | "SHA-224" => Some(Self::Sha224),
            "sha-256" | "SHA-256" => Some(Self::Sha256),
            "sha-384" | "SHA-384" => Some(Self::Sha384),
            "sha-512" | "SHA-512" => Some(Self::Sha512),
            _ => None,
        }
    }

    pub fn digest(&self, data: &[u8]) -> Vec<u8> {
        match self {
            ThumbprintHash::Sha1 => sha1::Sha1::digest(data).to_vec(),
            ThumbprintHash::Sha224 => sha2::Sha224::digest(data).to_vec(),
            ThumbprintHash::Sha256 => sha2::Sha256::digest(data).to_vec(),
            ThumbprintHash::Sha384 => sha2::Sha384::digest(data).to_vec(),
            ThumbprintHash::Sha512 => sha2::Sha512::digest(data).to_vec(),
        }
    }
}

pub struct CertificateThumbprint {
    hash: ThumbprintHash,
    thumbprint: Vec<u8>,
}

impl CertificateThumbprint {
    pub fn new(hash: ThumbprintHash, thumbprint: Vec<u8>) -> Self {
        Self { hash, thumbprint }
    }

    pub fn new_from_hash_name(hash_name: &str, thumbprint: Vec<u8>) -> Option<Self> {
        Some(Self {
            hash: ThumbprintHash::from_str(hash_name)?,
            thumbprint,
        })
    }

    pub fn eq_certificate(&self, cert: &[u8]) -> bool {
        let cert_hash = self.hash.digest(cert);

        // Compare the calculated hash to our thumbprint
        cert_hash.as_slice() == self.thumbprint.as_slice()
    }
}
