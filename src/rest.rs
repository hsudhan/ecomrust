//! Shared actix-web machinery for the entity REST services: query validation
//! (specs.md #6: API validation and error handling), the paginated response
//! envelope, and a generic cache-aside server — every read hits the Redis
//! cache first and falls back to the PostgreSQL table on a cache miss,
//! writing the result back into Redis before responding.

use actix_web::{get, web, App, HttpResponse, HttpServer};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::cache::{Cache, DEFAULT_REDIS_URL};
use crate::db::{self, FetchAll, FetchOne, DEFAULT_DATABASE_URL};
use crate::error::{AppError, AppResult};
use crate::models::{PageParams, SortDir};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

pub fn parse_params(q: &ListQuery) -> AppResult<PageParams> {
    let page = q.page.unwrap_or(1);
    if page < 1 {
        return Err(AppError::BadRequest("page must be >= 1".to_string()));
    }
    let page_size = q.page_size.unwrap_or(50);
    if !(1..=200).contains(&page_size) {
        return Err(AppError::BadRequest(
            "page_size must be between 1 and 200".to_string(),
        ));
    }
    let dir = match q.order.as_deref() {
        None | Some("asc") => SortDir::Asc,
        Some("desc") => SortDir::Desc,
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "invalid order '{other}'; allowed: asc, desc"
            )))
        }
    };
    Ok(PageParams {
        page,
        page_size,
        sort: q.sort.clone().unwrap_or_else(|| "id".to_string()),
        dir,
    })
}

pub fn list_body(rows: Vec<Value>, total: i64, params: &PageParams) -> Value {
    json!({
        "data": rows,
        "page": params.page,
        "page_size": params.page_size,
        "total_records": total,
        "total_pages": ((total + params.page_size - 1) / params.page_size).max(1),
        "sort": params.sort,
        "order": match params.dir {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        },
    })
}

// ------------------------------------------------------- generic server ---

/// Everything a per-entity bin must supply; routes, pagination, and
/// cache-aside reads are identical across entities.
#[derive(Clone)]
pub struct ServiceConfig {
    pub service_name: &'static str, // "orders"   (logs + /health)
    pub port: u16,                  // 4001
    pub prefix: &'static str,       // Redis key prefix
    pub date_field: &'static str,   // sortable date column
    pub list_path: &'static str,    // "/orders"
    pub item_path: &'static str,    // "/order/{id}"
    pub fetch_all: FetchAll,        // cold-cache full-table load
    pub fetch_one: FetchOne,        // single-doc cache miss
}

struct AppState {
    config: ServiceConfig,
    cache: Cache,
    pool: PgPool,
}

async fn list_handler(
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let params = parse_params(&query)?;
    let cfg = &state.config;
    // Cold cache: entity never loaded -> seed it from the database table.
    if !state.cache.index_exists(cfg.prefix).await? {
        let docs = (cfg.fetch_all)(&state.pool).await?;
        let n = state.cache.load(cfg.prefix, cfg.date_field, &docs).await?;
        println!(
            "{} API: cold cache -> loaded {n} rows from PostgreSQL ({}:*)",
            cfg.service_name, cfg.prefix
        );
    }
    let (rows, total) = state.cache.list(cfg.prefix, cfg.date_field, &params).await?;
    Ok(HttpResponse::Ok().json(list_body(rows, total, &params)))
}

async fn item_handler(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    if id < 1 {
        return Err(AppError::BadRequest("id must be >= 1".to_string()));
    }
    let cfg = &state.config;
    let json = match state.cache.get_raw(cfg.prefix, id).await? {
        Some(doc) => doc,
        None => {
            // Cache miss -> read the database table and write back to Redis.
            let doc = (cfg.fetch_one)(&state.pool, id)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("{} id {id} not found in cache or database", cfg.prefix))
                })?;
            state
                .cache
                .load(cfg.prefix, cfg.date_field, std::slice::from_ref(&doc))
                .await?;
            doc.json
        }
    };
    // Serve the stored JSON verbatim (no re-serialization).
    Ok(HttpResponse::Ok().content_type("application/json").body(json))
}

#[get("/health")]
async fn health(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok", "service": state.config.service_name }))
}

/// Connect to Redis (eager, required) and PostgreSQL (lazy, only used on
/// cache misses), then serve the configured entity until shutdown.
pub async fn serve(config: ServiceConfig) -> std::io::Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());

    let cache = Cache::connect(&redis_url)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string()))?;
    let pool = db::connect_lazy(&db_url)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    let name = config.service_name;
    let port = config.port;
    let list_path = config.list_path;
    let item_path = config.item_path;
    let state = web::Data::new(AppState { config, cache, pool });

    println!("{name} API listening on http://0.0.0.0:{port} (Redis: {redis_url}, PostgreSQL: {db_url})");
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route(list_path, web::get().to(list_handler))
            .route(item_path, web::get().to(item_handler))
            .service(health)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
