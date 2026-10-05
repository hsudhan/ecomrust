//! Users REST API (specs.md #2): actix-web server on port 4003.
//!   GET /users?page=&page_size=&sort=&order=  -> paginated list, Redis first
//!   GET /user/{id}                            -> single user, Redis first
//! Cache misses fall back to the PostgreSQL table and write back to Redis.
//! The password column never leaves the database.
//!
//! Run: cargo run --bin users_api

use ecomrust::cache::{USERS_DATE_FIELD, USERS_PREFIX};
use ecomrust::db;
use ecomrust::rest::{serve, ServiceConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    serve(ServiceConfig {
        service_name: "users",
        port: 4003,
        prefix: USERS_PREFIX,
        date_field: USERS_DATE_FIELD,
        list_path: "/users",
        item_path: "/user/{id}",
        fetch_all: db::fetch_all_users,
        fetch_one: db::fetch_user,
    })
    .await
}
