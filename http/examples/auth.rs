use http::server::lxi::api::auth::{
    BasicAuthRequest, LxiApiAuthRequest, LxiApiAuthScheme, Storage,
};
use tide::{Middleware, Redirect};
use std::collections::HashMap;
use std::env;
use tide_rustls::TlsListener;

// We define our user struct like so:
#[derive(Clone)]
struct User {
    username: String,
    favorite_food: String,

    // We include the password here, which is not very secure. This is for
    // illustrative purposes only.
    password: String,

    api_permission: Option<Permission>,
}

#[derive(Clone, Default)]
struct Permission {
    do_stuff: bool,
}

// We're creating an in-memory map of usernames to users.
#[derive(Clone)]
struct ExampleState {
    users: HashMap<String, User>,
    api_permissions: HashMap<String, Permission>,
}

impl ExampleState {
    pub fn new(userlist: Vec<User>, apikeys: Vec<String>) -> Self {
        let mut users = HashMap::new();
        for user in userlist {
            users.insert(user.username.to_owned(), user);
        }

        let mut api_permissions = HashMap::new();
        for key in apikeys {
            api_permissions.insert(key, Permission::default());
        }

        ExampleState {
            users,
            api_permissions,
        }
    }
}

// User credentials storage
#[async_trait::async_trait]
impl Storage<User, BasicAuthRequest> for ExampleState {
    async fn get_user(&self, request: BasicAuthRequest) -> tide::Result<Option<User>> {
        match self.users.get(&request.username) {
            Some(user) => {
                // Again, this is just an example. In practice you'd want to use something called a
                // "constant time comparison function" to check if the passwords are equivalent to
                // avoid a timing attack.
                if user.password != request.password {
                    return Ok(None);
                }

                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }
}

// User permission storage
#[async_trait::async_trait]
impl Storage<Permission, BasicAuthRequest> for ExampleState {
    async fn get_user(&self, request: BasicAuthRequest) -> tide::Result<Option<Permission>> {
        match self.users.get(&request.username) {
            Some(user) => {
                // Again, this is just an example. In practice you'd want to use something called a
                // "constant time comparison function" to check if the passwords are equivalent to
                // avoid a timing attack.
                if user.password != request.password {
                    return Ok(None);
                }

                Ok(user.api_permission.clone())
            }
            None => Ok(None),
        }
    }
}

// LXI-API key permission storage
#[async_trait::async_trait]
impl Storage<Permission, LxiApiAuthRequest> for ExampleState {
    async fn get_user(&self, request: LxiApiAuthRequest) -> tide::Result<Option<Permission>> {
        // Token should be stored with some kind of hashing
        match self.api_permissions.get(&request.token) {
            Some(perms) => Ok(Some(perms.clone())),
            None => Ok(None),
        }
    }
}


enum HttpMode {
    Enable,
    RedirectAll,
    Disable,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    femme::with_level(log::LevelFilter::Debug);

    let port = env::var("PORT").ok().unwrap_or_else(|| "8080".to_string());
    let host = env::var("HOST")
        .ok()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let addr = format!("{}:{}", host, port);
    let mode = HttpMode::Enable;

    let users = vec![
        User {
            username: "Basil".to_string(),
            favorite_food: "Cat food".to_string(),
            password: "cool meow time".to_string(),
            api_permission: Default::default(),
        },
        User {
            username: "Fern".to_string(),
            favorite_food: "Human food".to_string(),
            password: "hunter2 am I doing this right".to_string(),
            api_permission: Default::default(),
        },
    ];


    let mut insecure_app = tide::new();
    match mode {
        HttpMode::Enable => {
            
        },
        HttpMode::RedirectAll => {
            insecure_app.at("*").all(redirect);
            insecure_app.at("/").all(redirect);
        },
        HttpMode::Disable => {
            
        },
    }
    

    let mut secure_app = tide::with_state(ExampleState::new(users, vec!["ABCTOKEN".to_string()]));
    secure_app.with(tide_http_auth::Authentication::<User, _>::new(
        tide_http_auth::BasicAuthScheme::default(),
    ));
    secure_app.with(tide_http_auth::Authentication::<Permission, _>::new(
        tide_http_auth::BasicAuthScheme::default(),
    ));
    secure_app.with(tide_http_auth::Authentication::<Permission, _>::new(
        LxiApiAuthScheme::default(),
    ));
    secure_app.at("/").get(secure);
    secure_app.at("/secure").get(secure);
    secure_app.listen(
        TlsListener::build()
            .addrs("localhost:4433")
            .cert(std::env::var("TIDE_CERT_PATH").unwrap())
            .key(std::env::var("TIDE_KEY_PATH").unwrap()),
    )
    .await?;

    Ok(())
}

async fn hello<State>(_req: tide::Request<State>) -> tide::Result<tide::Response> {
    let response: tide::Response = "howdy stranger".to_string().into();
    Ok(response)
}

async fn redirect<State>(req: tide::Request<State>) -> tide::Result<tide::Response> {
    let mut url = req.url().clone();
    url.set_scheme("https")
                .map_err(|_| tide::http::format_err!("could not set scheme of url {}", url))?;
    Ok(Redirect::new(url).into())
}

async fn secure<State>(req: tide::Request<State>) -> tide::Result<tide::Response> {
    if let Some(user) = req.ext::<User>() {
        Ok(format!(
            "hi {}! your favorite food is {}.",
            user.username, user.favorite_food
        )
        .into())
    } else if let Some(perms) = req.ext::<Permission>() {
        Ok(format!("API key do_stuf={}.", perms.do_stuff).into())
    } else {
        let mut response: tide::Response = "howdy stranger".to_string().into();
        response.set_status(tide::http::StatusCode::Unauthorized);
        response.insert_header("WWW-Authenticate", "Basic");
        Ok(response)
    }
}
