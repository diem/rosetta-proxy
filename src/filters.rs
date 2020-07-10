use crate::{
    error::ApiError,
    options::Options,
};
use futures::future::BoxFuture;
use serde::{Serialize, Deserialize};
use std::{
    convert::Infallible,
    future::Future,
};
use warp::Filter;

pub fn with_options(options: Options) -> impl Filter<Extract=(Options,), Error=Infallible> + Clone {
    warp::any().map(move || options.clone())
}

#[derive(Deserialize, Serialize)]
pub struct EmptyRequest;

pub fn with_empty_request() -> impl Filter<Extract=(EmptyRequest,), Error=Infallible> + Clone {
    warp::any().map(move || EmptyRequest)
}

pub fn handle<'a, F, R, Req, Resp>(
    handler: F,
) -> impl Fn(Req, Options) -> BoxFuture<'static, Result<warp::reply::WithStatus<warp::reply::Json>, Infallible>>
       + Clone
where
    F: FnOnce(Req, Options) -> R + Clone + Copy + Send + 'static,
    R: Future<Output = Result<Resp, ApiError>> + Send,
    Req: Deserialize<'a> + Send + 'static,
    Resp: Serialize,
{
    move |request, options| {
        let fut = async move {
            match handler(request, options).await {
                Ok(response) => Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::OK,
                )),
                Err(api_error) => {
                    let status = api_error.status_code();
                    Ok(warp::reply::with_status(
                        warp::reply::json(&api_error.into_error()),
                        status,
                    ))
                }
            }
        };
        Box::pin(fut)
    }
}
