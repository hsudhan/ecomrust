//! Cache loader (specs.md #1): fetches all eight ecommerce tables
//! (orders, shipments, users, logins, shopping carts, payment infos,
//! payments, shipment trackings) from PostgreSQL and loads them into Redis
//! under their key prefixes (`orders:*`, `shipments:*`, `users:*`, ...).
//!
//! Run: cargo run --bin cache_loader

use ecomrust::cache::{
    Cache, DEFAULT_REDIS_URL, LOGINS_PREFIX, LOGINS_SORT_FIELDS, ORDERS_PREFIX, ORDERS_SORT_FIELDS,
    PAYMENT_INFOS_PREFIX, PAYMENT_INFOS_SORT_FIELDS, PAYMENTS_PREFIX, PAYMENTS_SORT_FIELDS,
    SHIPMENT_TRACKINGS_PREFIX, SHIPMENT_TRACKINGS_SORT_FIELDS, SHIPMENTS_PREFIX,
    SHIPMENTS_SORT_FIELDS, SHOPPING_CARTS_PREFIX, SHOPPING_CARTS_SORT_FIELDS, USERS_PREFIX,
    USERS_SORT_FIELDS,
};
use ecomrust::db::{self, FetchAll, DEFAULT_DATABASE_URL};
use ecomrust::models::SortField;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());

    println!("cache_loader: connecting to PostgreSQL ({db_url})");
    let pool = db::connect(&db_url).await?;
    println!("cache_loader: connecting to Redis ({redis_url})");
    let cache = Cache::connect(&redis_url).await?;

    let entities: [(&str, &[SortField], FetchAll); 8] = [
        (ORDERS_PREFIX, ORDERS_SORT_FIELDS, db::fetch_all_orders),
        (SHIPMENTS_PREFIX, SHIPMENTS_SORT_FIELDS, db::fetch_all_shipments),
        (USERS_PREFIX, USERS_SORT_FIELDS, db::fetch_all_users),
        (LOGINS_PREFIX, LOGINS_SORT_FIELDS, db::fetch_all_logins),
        (
            SHOPPING_CARTS_PREFIX,
            SHOPPING_CARTS_SORT_FIELDS,
            db::fetch_all_shopping_carts,
        ),
        (
            PAYMENT_INFOS_PREFIX,
            PAYMENT_INFOS_SORT_FIELDS,
            db::fetch_all_payment_infos,
        ),
        (PAYMENTS_PREFIX, PAYMENTS_SORT_FIELDS, db::fetch_all_payments),
        (
            SHIPMENT_TRACKINGS_PREFIX,
            SHIPMENT_TRACKINGS_SORT_FIELDS,
            db::fetch_all_shipment_trackings,
        ),
    ];

    for (prefix, sort_fields, fetch) in entities {
        let docs = fetch(&pool).await?;
        println!(
            "cache_loader: fetched {} {prefix} from PostgreSQL",
            docs.len()
        );
        let n = cache.reload(prefix, sort_fields, &docs).await?;
        println!("cache_loader: loaded {n} {prefix} into Redis ({prefix}:*)");
    }

    println!("cache_loader: done");
    Ok(())
}
