//! LXI-API end-points and 
//! 

use self::common_configuration::CommonConfiguration;

/// Authentication stuff + middleware
pub mod auth;
/// Miscellaneus middlewares
pub mod middleware;

/// Endpoints for `/common-configuration` 
pub mod common_configuration;
/// Endpoints for `/common-configuration` 
pub mod device_specific_configuration;
/// Endpoints for `/certificates`, `/certificates/:guid`, and `/certificates/:guid/enabled`
pub mod certificates;
/// Endpoints for `/get-csr` 
pub mod get_csr;
/// Endpoints for `/create-certificate` 
pub mod create_certificate;

trait LxiApi: CommonConfiguration {}