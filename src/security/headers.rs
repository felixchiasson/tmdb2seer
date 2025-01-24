use axum::response::Response;
use futures::future::BoxFuture;
use http::{HeaderValue, Request};
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct SecurityHeadersLayer;

impl SecurityHeadersLayer {
    pub fn new() -> Self {
        SecurityHeadersLayer
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeaders<S>;

    fn layer(&self, service: S) -> Self::Service {
        SecurityHeaders { inner: service }
    }
}

#[derive(Clone)]
pub struct SecurityHeaders<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for SecurityHeaders<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let mut response = inner.call(request).await?;

            let headers = response.headers_mut();

            headers.insert(
                "X-Content-Type-Options",
                HeaderValue::from_static("nosniff"),
            );

            headers.insert("X-Frame-Options", HeaderValue::from_static("SAMEORIGIN"));

            headers.insert(
                "X-XSS-Protection",
                HeaderValue::from_static("1; mode=block"),
            );

            headers.insert(
                "Content-Security-Policy",
                HeaderValue::from_static(
                    "default-src 'self'; \
                     script-src 'self' 'unsafe-inline'; \
                     style-src 'self' 'unsafe-inline'; \
                     img-src 'self' https://image.tmdb.org https:; \
                     connect-src 'self' https:;",
                ),
            );

            headers.insert(
                "Referrer-Policy",
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("no-store, max-age=0"),
            );

            Ok(response)
        })
    }
}
