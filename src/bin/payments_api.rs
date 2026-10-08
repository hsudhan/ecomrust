//! Payments REST API (specs.md #2): actix-web server on port 4007.
//!   GET /payments?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /payment/{id}                            -> single payment, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin payments_api

use ecomrust::cache::{PAYMENTS_PREFIX, PAYMENTS_SORT_FIELDS};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "payments",
        port: 4007,
        prefix: PAYMENTS_PREFIX,
        sort_fields: PAYMENTS_SORT_FIELDS,
        list_path: "/payments",
        item_path: "/payment/{id}",
        fetch_all: db::fetch_all_payments,
        fetch_one: db::fetch_payment,
    })
    .await
}
