use axum::response::IntoResponse;
use axum::{body::Body, extract::ConnectInfo, response::Response};
use http::{Request, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::Service;
use tracing::{debug, warn};

use super::rate_limiter::RateLimiter;

#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<RateLimiter>,
}

impl RateLimitLayer {
    pub fn new(rps: u32, burst_size: u32) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(rps, burst_size)),
        }
    }
}

// Define the middleware as a service
#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    layer: RateLimitLayer,
}

impl<S> RateLimitMiddleware<S> {
    pub fn new(inner: S, layer: RateLimitLayer) -> Self {
        Self { inner, layer }
    }
}

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let layer = self.layer.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let connect_info = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(addr)| addr.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if layer.limiter.is_allowed(connect_info.clone()).await {
                debug!("Request allowed from {}", connect_info);
                inner.call(request).await
            } else {
                warn!("Rate limit exceeded for IP: {}", connect_info);
                Ok(StatusCode::TOO_MANY_REQUESTS.into_response())
            }
        })
    }
}

// Layer implementation
#[derive(Clone)]
pub struct RateLimitServiceLayer {
    layer: RateLimitLayer,
}

impl RateLimitServiceLayer {
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            layer: RateLimitLayer::new(requests_per_second, burst_size),
        }
    }
}

impl<S> tower::Layer<S> for RateLimitServiceLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimitMiddleware::new(service, self.layer.clone())
    }
}
