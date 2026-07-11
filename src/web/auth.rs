// HTTP Basic Auth middleware buat dashboard. Aktif kalau
// `DASHBOARD_PASSWORD` di-set (lihat config). Bandingkan credential
// pakai timing-safe compare biar aman dari timing attack sederhana.

use crate::config::DashboardAuth;
use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::Arc;

/// Middleware factory — return closure yang bisa dipasang lewat `from_fn`.
pub fn require(creds: Arc<DashboardAuth>) -> impl Fn(Request, Next) -> BoxFuture + Clone {
    move |req, next| {
        let creds = creds.clone();
        Box::pin(async move { check(creds, req, next).await })
    }
}

type BoxFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>;

async fn check(creds: Arc<DashboardAuth>, req: Request, next: Next) -> Response {
    if authorized(&creds, &req) {
        return next.run(req).await;
    }
    let mut resp = (StatusCode::UNAUTHORIZED, "Butuh login dulu 🔒").into_response();
    resp.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static(r#"Basic realm="Kewarasan", charset="UTF-8""#),
    );
    resp
}

fn authorized(creds: &DashboardAuth, req: &Request) -> bool {
    let Some(header) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(b64) = header.strip_prefix("Basic ").map(str::trim) else {
        return false;
    };
    let Ok(bytes) = STANDARD.decode(b64) else {
        return false;
    };
    let Ok(pair) = std::str::from_utf8(&bytes) else {
        return false;
    };
    let Some((user, pass)) = pair.split_once(':') else {
        return false;
    };
    constant_eq(user.as_bytes(), creds.user.as_bytes())
        && constant_eq(pass.as_bytes(), creds.password.as_bytes())
}

/// Bandingkan byte-by-byte tanpa short-circuit — hindarin timing leak.
fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}
