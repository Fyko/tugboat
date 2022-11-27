/// Make our own error that wraps `anyhow::Error`.
use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};

enum MyError {
    SomethingWentWrong,
    SomethingElseWentWrong,
}

impl IntoResponse for MyError {
    type Body = Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        let body = match self {
            MyError::SomethingWentWrong => Body::from("something went wrong"),
            MyError::SomethingElseWentWrong => Body::from("something else went wrong"),
        };

        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body)
            .unwrap()
    }
}
