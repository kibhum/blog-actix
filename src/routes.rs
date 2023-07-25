use crate::errors::AppError;
use actix_web::HttpResponse;

// If you want to restrict the visibility of an item to a specific scope then you can use one of pub(in path),pub(crate), pub(super), or pub(self),

pub(super) mod users;

fn convert<T, E>(res: Result<T, E>) -> Result<HttpResponse, AppError>
where
    T: serde::Serialize,
    AppError: From<E>,
{
    res.map(|d| HttpResponse::Ok().json(d)).map_err(Into::into)
}
