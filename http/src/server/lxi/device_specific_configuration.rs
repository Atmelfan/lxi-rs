use tide::Request;

pub async fn get<S>(req: Request<S>) -> tide::Result
where
    S: super::api::common_configuration::CommonConfiguration,
{
    todo!()
}
