use std::net::SocketAddr;

use axum::routing::get;
use axum::Router;

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_config = config::load()?;

    let app = app();

    let addr = SocketAddr::from(([0, 0, 0, 0], app_config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn app() -> Router {
    Router::new().route("/", get(root))
}

async fn root() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    /// 2.1 - GET / returns 200 with "Hello, world!"
    #[tokio::test]
    async fn root_returns_hello_world() {
        let response = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello, world!");
    }

    /// 2.2 - unknown path returns 404
    #[tokio::test]
    async fn unknown_path_returns_404() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// 2.3 - unsupported method on / returns 405
    #[tokio::test]
    async fn unsupported_method_returns_405() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// 2.4 - concurrent requests are handled independently
    #[tokio::test]
    async fn concurrent_requests_all_succeed() {
        let mut handles = Vec::new();
        for _ in 0..10 {
            let app = app();
            handles.push(tokio::spawn(async move {
                let response = app
                    .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
                    .await
                    .unwrap();

                assert_eq!(response.status(), StatusCode::OK);
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                assert_eq!(&body[..], b"Hello, world!");
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
}
