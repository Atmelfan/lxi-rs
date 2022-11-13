use tide::Request;

/// Just an alias for [super::api::common_configuration::get]
pub async fn get<S>(req: Request<S>) -> tide::Result
where
    S: super::api::common_configuration::CommonConfiguration,
{
    super::api::common_configuration::get(req).await
}
