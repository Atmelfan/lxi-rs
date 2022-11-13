use http::server::lxi::{
    self,
    api::{common_configuration::UserInfo, prelude::*},
    identification::Identification,
};
use std::{collections::HashMap, sync::Arc};

// We define our user struct like so:
#[derive(Clone)]
pub(crate) struct User {
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) api_permissions: Option<Permission>,
}

#[derive(Clone)]
pub(crate) struct MyState {
    users: Arc<HashMap<String, User>>,
    apikeys: Arc<HashMap<String, Permission>>,
}

impl MyState {
    pub fn new(userlist: Vec<User>, apikeys: Vec<String>) -> Self {
        let mut users = HashMap::new();
        for user in userlist {
            users.insert(user.username.to_owned(), user);
        }

        let mut api = HashMap::new();
        for apikey in apikeys {
            api.insert(apikey, Permission::admin());
        }

        MyState {
            users: Arc::new(users),
            apikeys: Arc::new(api),
        }
    }
}

impl Identification for MyState {
    fn lxi_version() -> String {
        "1.6".to_string()
    }

    fn manufacturer(&self) -> String {
        "Cyberdyne systems".to_string()
    }

    fn model(&self) -> String {
        "T800 Model 101".to_string()
    }

    fn serial_number(&self) -> String {
        "A9012.C".to_string()
    }

    fn interfaces(&self) -> Vec<lxi::identification::Interface> {
        vec![]
    }

    fn user_description(&self) -> String {
        "Some description".to_string()
    }

    fn host(&self) -> String {
        "localhost".to_string()
    }
}

// User permission storage
#[async_trait::async_trait]
impl LxiApiAuthStorage for MyState {
    async fn get_user_permissions(
        &self,
        user: LxiBasicAuthRequest,
    ) -> Result<Option<Permission>, LxiAuthenticationError> {
        match self.users.get(&user.username) {
            Some(u) => {
                if u.password == user.password {
                    Ok(u.api_permissions.clone())
                } else {
                    Err(LxiAuthenticationError::InvalidCredentials)
                }
            }
            None => Err(LxiAuthenticationError::InvalidCredentials),
        }
    }

    async fn get_apikey_permissions(&self, apikey: LxiApiAuthRequest) -> Option<Permission> {
        self.apikeys.get(&apikey.token).copied()
    }
}

impl lxi::api::common_configuration::CommonConfiguration for MyState {
    fn lan_config_initialize(&self) {
        todo!()
    }

    fn get_users(&self) -> Vec<lxi::api::common_configuration::UserInfo> {
        self.users
            .values()
            .into_iter()
            .map(|user| UserInfo {
                username: user.username.clone(),
                api_access: user.api_permissions,
            })
            .collect()
    }
}

impl lxi::api::device_specific_configuration::DeviceSpecificConfiguration for MyState {
    fn lan_config_initialize(&self) {
        todo!()
    }
}
