//! Logins REST API (specs.md #2): actix-web server on port 4004.
//!   GET /logins?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /login/{id}                            -> single login, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin logins_api

use ecomrust::cache::{LOGINS_DATE_FIELD, LOGINS_PREFIX};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "logins",
        port: 4004,
        prefix: LOGINS_PREFIX,
        date_field: LOGINS_DATE_FIELD,
        list_path: "/logins",
        item_path: "/login/{id}",
        fetch_all: db::fetch_all_logins,
        fetch_one: db::fetch_login,
    })
    .await
}
