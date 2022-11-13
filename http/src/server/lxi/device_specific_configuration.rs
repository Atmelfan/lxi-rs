use tide::Request;

/// Just an alias for [super::api::common_configuration::get]
pub async fn get<S>(req: Request<S>) -> tide::Result
where
    S: super::api::device_specific_configuration::DeviceSpecificConfiguration,
{
    super::api::device_specific_configuration::get(req).await
}
