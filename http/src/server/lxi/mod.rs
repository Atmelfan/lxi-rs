pub mod identification;
pub mod schemas;

#[cfg(feature = "lxi-api")]
pub mod api;
#[cfg(feature = "lxi-api")]
pub mod common_configuration;
#[cfg(feature = "lxi-api")]
pub mod device_specific_configuration;
