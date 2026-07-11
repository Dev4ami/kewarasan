// Router web + endpoint JSON buat dashboard Chart.js.
// Read-only, owner-scoped: default user = config::OWNER_ID, override `?user=<id>`.
// Rentang tanggal `?from=YYYY-MM-DD&to=YYYY-MM-DD` (default 30 hari terakhir).
// Auth optional (cookie session) — lihat web/auth.rs.

use crate::config::OWNER_ID;
use crate::db::queries::{self, DayAgg, HeatCell, TagAgg};
use crate::web::auth;
use crate::web::AppState;
use axum::{
    extract::{FromRef, Query, State},
    http::{header, StatusCode},
    middleware,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use sqlx::PgPool;

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// Router web: halaman dashboard + endpoint agregasi + PWA assets + login/logout.
/// Kalau `state.auth = Some`, halaman & API di-guard cookie session.
pub fn router(state: AppState) -> Router {
    let mut protected = Router::new()
        .route("/", get(index))
        .route("/api/trend", get(api_trend))
        .route("/api/tags", get(api_tags))
        .route("/api/heatmap", get(api_heatmap))
        .route("/logout", post(auth::logout));

    if let Some(a) = &state.auth {
        let mw = auth::require(a.sessions.clone());
        protected = protected.layer(middleware::from_fn(mw));
    }

    protected
        .route(
            "/login",
            get(auth::login_page).post(auth::login_submit),
        )
        .route("/icon.png", get(icon))
        .route("/apple-touch-icon.png", get(icon))
        .route("/favicon.ico", get(icon))
        .route("/manifest.webmanifest", get(manifest))
        .with_state(state)
}

/// HTML di-embed ke binary (include_str!) — gak perlu copy file saat Docker.
async fn index() -> Html<&'static str> {
    Html(include_str!("templates/index.html"))
}

/// Icon PNG untuk favicon, apple-touch-icon, dan manifest PWA — embed 512×512.
async fn icon() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=604800"),
        ],
        &include_bytes!("assets/kewarasan.png")[..],
    )
}

/// Web App Manifest — bikin "Add to Home Screen" muncul dengan nama & icon.
async fn manifest() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        include_str!("assets/manifest.webmanifest"),
    )
}

#[derive(Deserialize)]
struct RangeQuery {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    user: Option<i64>,
}

/// Konteks satu request: user + timezone + window UTC hasil resolve rentang.
struct Ctx {
    user_id: i64,
    tz: String,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

fn err500(e: anyhow::Error) -> (StatusCode, String) {
    tracing::error!("dashboard query gagal: {e:?}");
    (StatusCode::INTERNAL_SERVER_ERROR, "gagal ngambil data".into())
}

/// Resolve user (`?user=` else OWNER_ID) + hitung batas UTC dari rentang lokal.
/// `Ok(None)` = user belum pernah /start (endpoint balikin array kosong).
async fn resolve(pool: &PgPool, q: &RangeQuery) -> anyhow::Result<Option<Ctx>> {
    let telegram_id = q.user.unwrap_or(OWNER_ID);
    let Some(user) = queries::find_user_by_telegram(pool, telegram_id).await? else {
        return Ok(None);
    };
    let tz: Tz = user.timezone.parse().unwrap_or(chrono_tz::Asia::Jakarta);

    let today = Utc::now().with_timezone(&tz).date_naive();
    let to_date = q.to.unwrap_or(today);
    let from_date = q.from.unwrap_or(to_date - Duration::days(29));

    Ok(Some(Ctx {
        user_id: user.id,
        tz: user.timezone,
        from: local_midnight(&tz, from_date),
        to: local_midnight(&tz, to_date + Duration::days(1)), // batas atas eksklusif
    }))
}

/// Midnight tanggal lokal → UTC (fallback aman kalau jam midnight kena DST-gap).
fn local_midnight(tz: &Tz, date: NaiveDate) -> DateTime<Utc> {
    let naive = date.and_hms_opt(0, 0, 0).unwrap();
    tz.from_local_datetime(&naive)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&naive))
}

async fn api_trend(
    State(pool): State<PgPool>,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Vec<DayAgg>> {
    let Some(ctx) = resolve(&pool, &q).await.map_err(err500)? else {
        return Ok(Json(vec![]));
    };
    let rows = queries::trend_range(&pool, ctx.user_id, &ctx.tz, ctx.from, ctx.to)
        .await
        .map_err(err500)?;
    Ok(Json(rows))
}

async fn api_tags(
    State(pool): State<PgPool>,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Vec<TagAgg>> {
    let Some(ctx) = resolve(&pool, &q).await.map_err(err500)? else {
        return Ok(Json(vec![]));
    };
    let rows = queries::tag_correlation(&pool, ctx.user_id, ctx.from, ctx.to)
        .await
        .map_err(err500)?;
    Ok(Json(rows))
}

async fn api_heatmap(
    State(pool): State<PgPool>,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Vec<HeatCell>> {
    let Some(ctx) = resolve(&pool, &q).await.map_err(err500)? else {
        return Ok(Json(vec![]));
    };
    let rows = queries::heatmap(&pool, ctx.user_id, &ctx.tz, ctx.from, ctx.to)
        .await
        .map_err(err500)?;
    Ok(Json(rows))
}
