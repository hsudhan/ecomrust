//! Cache loader (specs.md #1): fetches all eight ecommerce tables
//! (orders, shipments, users, logins, shopping carts, payment infos,
//! payments, shipment trackings) from PostgreSQL and loads them into Redis
//! under their key prefixes (`orders:*`, `shipments:*`, `users:*`, ...).
//!
//! Run: cargo run --bin cache_loader

use ecomrust::cache::{
    Cache, DEFAULT_REDIS_URL, LOGINS_DATE_FIELD, LOGINS_PREFIX, ORDERS_DATE_FIELD, ORDERS_PREFIX,
    PAYMENT_INFOS_DATE_FIELD, PAYMENT_INFOS_PREFIX, PAYMENTS_DATE_FIELD, PAYMENTS_PREFIX,
    SHIPMENT_TRACKINGS_DATE_FIELD, SHIPMENT_TRACKINGS_PREFIX, SHIPMENTS_DATE_FIELD,
    SHIPMENTS_PREFIX, SHOPPING_CARTS_DATE_FIELD, SHOPPING_CARTS_PREFIX, USERS_DATE_FIELD,
    USERS_PREFIX,
};
use ecomrust::db::{self, FetchAll, DEFAULT_DATABASE_URL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());

    println!("cache_loader: connecting to PostgreSQL ({db_url})");
    let pool = db::connect(&db_url).await?;
    println!("cache_loader: connecting to Redis ({redis_url})");
    let cache = Cache::connect(&redis_url).await?;

    let entities: [(&str, &str, FetchAll); 8] = [
        (ORDERS_PREFIX, ORDERS_DATE_FIELD, db::fetch_all_orders),
        (SHIPMENTS_PREFIX, SHIPMENTS_DATE_FIELD, db::fetch_all_shipments),
        (USERS_PREFIX, USERS_DATE_FIELD, db::fetch_all_users),
        (LOGINS_PREFIX, LOGINS_DATE_FIELD, db::fetch_all_logins),
        (
            SHOPPING_CARTS_PREFIX,
            SHOPPING_CARTS_DATE_FIELD,
            db::fetch_all_shopping_carts,
        ),
        (
            PAYMENT_INFOS_PREFIX,
            PAYMENT_INFOS_DATE_FIELD,
            db::fetch_all_payment_infos,
        ),
        (PAYMENTS_PREFIX, PAYMENTS_DATE_FIELD, db::fetch_all_payments),
        (
            SHIPMENT_TRACKINGS_PREFIX,
            SHIPMENT_TRACKINGS_DATE_FIELD,
            db::fetch_all_shipment_trackings,
        ),
    ];

    for (prefix, date_field, fetch) in entities {
        let docs = fetch(&pool).await?;
        println!(
            "cache_loader: fetched {} {prefix} from PostgreSQL",
            docs.len()
        );
        let n = cache.load(prefix, date_field, &docs).await?;
        println!("cache_loader: loaded {n} {prefix} into Redis ({prefix}:*)");
    }

    println!("cache_loader: done");
    Ok(())
}
