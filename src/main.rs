#![feature(type_alias_impl_trait)]

pub mod command;
pub mod error;
pub mod responder;
pub mod router;

use std::net::SocketAddr;

use axum::{
    async_trait,
    body::{self, BoxBody, Bytes, Full, Body},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Router, Server, extract::FromRequest,
};
use responder::Responder;
use router::InteractionRouter;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::InteractionResponseDataBuilder;

fn ping() -> impl Responder {
    "Pong!"
}

fn create_interaction_router() -> InteractionRouter {
    let router = InteractionRouter::new();
    router.command("ping", ping);

    router
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let router = create_interaction_router();

    let app = Router::new()
        .route("/api/interactions", post(router.handle_request))
        .layer(
            ServiceBuilder::new()
                .map_request(body::boxed)
                .layer(middleware::from_fn(print_request_body)),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    if let Err(e) = Server::bind(&addr).serve(app.into_make_service()).await {
        tracing::error!("server error: {}", e);
    }
}

async fn print_request_body(
    request: Request<BoxBody>,
    next: Next<BoxBody>,
) -> Result<impl IntoResponse, Response> {
    let request = buffer_request_body(request).await?;

    Ok(next.run(request).await)
}

// the trick is to take the request apart, buffer the body, do what you need to do, then put
// the request back together
async fn buffer_request_body(request: Request<BoxBody>) -> Result<Request<BoxBody>, Response> {
    let (parts, body) = request.into_parts();

    // this wont work if the body is an long running stream
    let bytes = hyper::body::to_bytes(body)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    do_thing_with_request_body(bytes.clone());

    Ok(Request::from_parts(parts, body::boxed(Full::from(bytes))))
}

fn do_thing_with_request(req: Body) {
    tracing::debug!(body = ?bytes);
}

// extractor that shows how to consume the request body upfront
enum InteractionData{
    Command(CommandData),
}

// we must implement `FromRequest` (and not `FromRequestParts`) to consume the body
#[async_trait]
impl<S> FromRequest<S, BoxBody> for InteractionData
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request<BoxBody>, state: &S) -> Result<Self, Self::Rejection> {
        let body = Bytes::from_request(req, state)
            .await
            .map_err(|err| err.into_response())?;

        do_thing_with_request(req.clone());

        Ok(Self(body))
    }
}
