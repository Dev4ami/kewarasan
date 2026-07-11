// Cookie-based login/logout untuk dashboard.
// - Middleware: cek cookie `session` di setiap request protected.
//   HTML request tanpa session → redirect ke /login.
//   API request tanpa session → 401 JSON.
// - Login handler: verify password (constant-time) → create session → set cookie.
// - Logout handler: hapus session di store + clear cookie.

use crate::config::DashboardAuth;
use crate::web::session::Sessions;
use crate::web::AppState;
use axum::{
    extract::{Form, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::sync::Arc;

const COOKIE_NAME: &str = "session";

/// State bareng buat login/logout handler.
#[derive(Clone)]
pub struct AuthState {
    pub creds: Arc<DashboardAuth>,
    pub sessions: Arc<Sessions>,
}

/// Middleware factory — kembalikan closure yang bisa dipasang lewat `from_fn`.
pub fn require(sessions: Arc<Sessions>) -> impl Fn(Request, Next) -> BoxFuture + Clone {
    move |req, next| {
        let sessions = sessions.clone();
        Box::pin(async move { check(sessions, req, next).await })
    }
}

type BoxFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>;

async fn check(sessions: Arc<Sessions>, req: Request, next: Next) -> Response {
    if let Some(token) = cookie_value(&req, COOKIE_NAME) {
        if sessions.is_valid(&token) {
            return next.run(req).await;
        }
    }
    // Belum login: API balas 401 JSON, halaman biasa redirect ke /login.
    if req.uri().path().starts_with("/api/") {
        (StatusCode::UNAUTHORIZED, "Butuh login dulu 🔒").into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}

/// Halaman /login. Kalau user sudah login, langsung lempar ke dashboard.
/// Kalau auth dinonaktifkan (dev), /login tidak berguna → redirect ke dashboard.
pub async fn login_page(State(state): State<AppState>, req: Request) -> Response {
    let Some(auth) = state.auth else {
        return Redirect::to("/").into_response();
    };
    if let Some(token) = cookie_value(&req, COOKIE_NAME) {
        if auth.sessions.is_valid(&token) {
            return Redirect::to("/").into_response();
        }
    }
    Html(include_str!("templates/login.html")).into_response()
}

#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
}

/// POST /login — verify password → set session cookie → redirect ke /.
pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    let Some(auth) = state.auth else {
        return Redirect::to("/").into_response();
    };
    if !constant_eq(form.password.as_bytes(), auth.creds.password.as_bytes()) {
        // Password salah — kirim ulang form dengan flag error di URL.
        return Redirect::to("/login?err=1").into_response();
    }
    let token = auth.sessions.create();
    let ttl_secs = auth.sessions.ttl().as_secs();
    let cookie =
        format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={ttl_secs}");
    (
        [(header::SET_COOKIE, HeaderValue::from_str(&cookie).unwrap())],
        Redirect::to("/"),
    )
        .into_response()
}

/// POST /logout — hapus session server-side + clear cookie di browser.
pub async fn logout(State(state): State<AppState>, req: Request) -> Response {
    if let Some(auth) = &state.auth {
        if let Some(token) = cookie_value(&req, COOKIE_NAME) {
            auth.sessions.remove(&token);
        }
    }
    let cookie = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    (
        [(header::SET_COOKIE, HeaderValue::from_str(&cookie).unwrap())],
        Redirect::to("/login"),
    )
        .into_response()
}

/// Ambil satu cookie value dari header `Cookie: a=1; b=2; ...`.
fn cookie_value(req: &Request, name: &str) -> Option<String> {
    req.headers()
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.to_string())
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
