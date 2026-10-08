//! Payment infos REST API (specs.md #2): actix-web server on port 4006.
//!   GET /payment-infos?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /payment-info/{id}                            -> single payment info, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//!
//! Run: cargo run --bin payment_infos_api

use ecomrust::cache::{PAYMENT_INFOS_PREFIX, PAYMENT_INFOS_SORT_FIELDS};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "payment-infos",
        port: 4006,
        prefix: PAYMENT_INFOS_PREFIX,
        sort_fields: PAYMENT_INFOS_SORT_FIELDS,
        list_path: "/payment-infos",
        item_path: "/payment-info/{id}",
        fetch_all: db::fetch_all_payment_infos,
        fetch_one: db::fetch_payment_info,
    })
    .await
}
