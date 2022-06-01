use sasl::server::{
    mechanisms::{Anonymous, Plain},
    Mechanism, MechanismError, Validator,
};

pub(crate) use sasl::server::Response as SaslResponse;
pub(crate) use sasl::Error as SaslError;

pub use sasl::secret;

use crate::common::errors::{Error, NonFatalErrorCode};

impl From<SaslError> for Error {
    fn from(err: SaslError) -> Self {
        match err {
            #[cfg(feature = "scram")]
            SaslError::RngError(_rng) => Error::NonFatal(
                NonFatalErrorCode::AuthenticationFailed,
                "Random generator failure".to_string(),
            ),
            SaslError::SaslError(s) => Error::NonFatal(NonFatalErrorCode::AuthenticationFailed, s),
        }
    }
}

impl From<MechanismError> for Error {
    fn from(err: MechanismError) -> Self {
        Error::NonFatal(
            NonFatalErrorCode::AuthenticationFailed,
            format!("Mechanism error: {}", err),
        )
    }
}

pub trait Auth {
    /// Return a list of supported mechanims
    fn list_mechanisms(&self) -> Vec<&str>;

    fn start_exchange(&self, mechanism: &str) -> Result<Box<dyn Mechanism + Send>, SaslError>;
}

#[derive(Debug, Clone)]
pub struct PlainAuth<V>
where
    V: Clone,
{
    allow_anonymous: bool,
    validator: V,
}

impl<V> PlainAuth<V>
where
    V: Clone,
{
    pub fn new(allow_anonymous: bool, validator: V) -> Self {
        Self {
            allow_anonymous,
            validator,
        }
    }
}

impl<V> Auth for PlainAuth<V>
where
    V: Clone + 'static,
    V: Validator<secret::Plain> + Send,
{
    fn list_mechanisms(&self) -> Vec<&str> {
        let mut vec = Vec::new();
        // Prefer PLAIN
        vec.push("PLAIN");
        // Optionally support anonymous
        if self.allow_anonymous {
            vec.push("ANONYMOUS");
        }
        vec
    }

    fn start_exchange(&self, mechanism: &str) -> Result<Box<dyn Mechanism + Send>, SaslError> {
        match mechanism {
            "ANONYMOUS" if self.allow_anonymous => Ok(Box::new(Anonymous::new())),
            "PLAIN" => Ok(Box::new(Plain::new(self.validator.clone()))),
            _ => Err(SaslError::SaslError("Mechanism not available".to_string())),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AnonymousAuth;

impl Auth for AnonymousAuth {
    fn list_mechanisms(&self) -> Vec<&str> {
        vec!["ANONYMOUS"]
    }

    fn start_exchange(&self, mechanism: &str) -> Result<Box<dyn Mechanism + Send>, SaslError> {
        if mechanism.eq_ignore_ascii_case("ANONYMOUS") {
            Ok(Box::new(Anonymous))
        } else {
            Err(SaslError::SaslError("Mechanism not available".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {

    use sasl::{
        client::{
            mechanisms::{Anonymous as ClientAnonymous, Plain as ClientPlain},
            Mechanism as ClientMechanism,
        },
        common::{Credentials, Identity},
        secret::Plain as PlainSecret,
        server::{Response, Validator, ValidatorError},
    };

    use super::{Auth, PlainAuth};

    #[derive(Clone)]
    struct DummyValidator;

    impl Validator<PlainSecret> for DummyValidator {
        fn validate(&self, identity: &Identity, value: &PlainSecret) -> Result<(), ValidatorError> {
            match identity {
                Identity::None => Err(ValidatorError::AuthenticationFailed),
                Identity::Username(username) => {
                    if username.eq_ignore_ascii_case("user") && value.0 == "pencil" {
                        Ok(())
                    } else {
                        Err(ValidatorError::AuthenticationFailed)
                    }
                }
            }
        }
    }

    #[test]
    fn plain_auth() {
        //Server
        let validator = DummyValidator;
        let auth = PlainAuth::new(true, validator);

        // Client
        let credentials = Credentials::default()
            .with_username("user")
            .with_password("pencil");
        let mut client = ClientPlain::from_credentials(credentials).unwrap();

        // Select PLAIN
        let mut server = auth.start_exchange("PLAIN").unwrap();

        // C -> S
        let exchange1 = client.initial();

        // S -> C OK
        let exchange2 = server.respond(&exchange1).unwrap();
        assert_eq!(
            exchange2,
            Response::Success(Identity::Username("user".to_string()), vec![])
        )
    }

    #[test]
    fn anonymous_auth() {
        //Server
        let validator = DummyValidator;
        let auth = PlainAuth::new(true, validator);

        // Client
        let mut client = ClientAnonymous::new();

        // Select PLAIN
        let mut server = auth.start_exchange("ANONYMOUS").unwrap();

        // C -> S
        let exchange1 = client.initial();
        println!("C: {:?}", exchange1);

        // S -> C OK
        let exchange2 = server.respond(&exchange1).unwrap();

        assert!(matches!(
            exchange2,
            Response::Success(Identity::Username(..), _)
        ))
    }
}
