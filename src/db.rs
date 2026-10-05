//! PostgreSQL access (primary storage). Used by the cache loader and by the
//! REST services on cache miss (cache-aside: Redis first, database on miss).
//! NUMERIC -> text casts preserve exact decimal precision.
//!
//! Each entity exposes two adapters returning boxed futures so the generic
//! REST tier (rest.rs) can hold them behind plain fn pointers:
//!   fetch_all_{entity}  -> every row as CacheDocs   (cold-cache list load)
//!   fetch_{entity}      -> one row by id as CacheDoc (single-doc cache miss)

use std::future::Future;
use std::pin::Pin;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::AppResult;
use crate::models::{
    CacheDoc, LoginRow, OrderRow, PaymentInfoRow, PaymentRow, ShipmentRow, ShipmentTrackingRow,
    ShoppingCartRow, UserRow,
};

pub const DEFAULT_DATABASE_URL: &str = "postgresql://harir@localhost:5432/ecomdb";

/// Boxed future returned by the entity fetchers below.
pub type DbFut<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

/// Fetch every row of an entity (cold-cache load).
pub type FetchAll = for<'a> fn(&'a PgPool) -> DbFut<'a, Vec<CacheDoc>>;
/// Fetch one row by id (single-doc cache miss).
pub type FetchOne = for<'a> fn(&'a PgPool, i64) -> DbFut<'a, Option<CacheDoc>>;

pub async fn connect(database_url: &str) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Lazy pool for the REST services: they must start (and keep serving from
/// Redis) even when PostgreSQL is briefly unreachable; a connection is only
/// established on the first actual cache miss.
pub fn connect_lazy(database_url: &str) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(database_url)?;
    Ok(pool)
}

// ------------------------------------------------------------- orders -----

async fn query_all_orders(pool: &PgPool) -> AppResult<Vec<OrderRow>> {
    let rows = sqlx::query_as::<_, OrderRow>(
        r#"SELECT id, customer_id, order_date, status,
                  total_amount::text AS total_amount,
                  created_at, updated_at
           FROM ecommerce."order"
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_order(pool: &PgPool, id: i64) -> AppResult<Option<OrderRow>> {
    let row = sqlx::query_as::<_, OrderRow>(
        r#"SELECT id, customer_id, order_date, status,
                  total_amount::text AS total_amount,
                  created_at, updated_at
           FROM ecommerce."order"
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_orders(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_orders(pool).await?;
        rows.iter().map(OrderRow::cache_doc).collect()
    })
}

pub fn fetch_order(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_order(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// ---------------------------------------------------------- shipments -----

async fn query_all_shipments(pool: &PgPool) -> AppResult<Vec<ShipmentRow>> {
    let rows = sqlx::query_as::<_, ShipmentRow>(
        r#"SELECT id, order_id, shipment_date, shipment_status,
                  shipment_cost::text AS shipment_cost,
                  created_at, updated_at
           FROM ecommerce.shipment
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_shipment(pool: &PgPool, id: i64) -> AppResult<Option<ShipmentRow>> {
    let row = sqlx::query_as::<_, ShipmentRow>(
        r#"SELECT id, order_id, shipment_date, shipment_status,
                  shipment_cost::text AS shipment_cost,
                  created_at, updated_at
           FROM ecommerce.shipment
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_shipments(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_shipments(pool).await?;
        rows.iter().map(ShipmentRow::cache_doc).collect()
    })
}

pub fn fetch_shipment(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_shipment(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// ------------------------------------------------------------- users ------
// NOTE: the password column is intentionally never selected.

async fn query_all_users(pool: &PgPool) -> AppResult<Vec<UserRow>> {
    let rows = sqlx::query_as::<_, UserRow>(
        r#"SELECT id, username, email, created_at, updated_at
           FROM ecommerce."user"
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_user(pool: &PgPool, id: i64) -> AppResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"SELECT id, username, email, created_at, updated_at
           FROM ecommerce."user"
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_users(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_users(pool).await?;
        rows.iter().map(UserRow::cache_doc).collect()
    })
}

pub fn fetch_user(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_user(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// ------------------------------------------------------------ logins ------

async fn query_all_logins(pool: &PgPool) -> AppResult<Vec<LoginRow>> {
    let rows = sqlx::query_as::<_, LoginRow>(
        r#"SELECT id, user_id, login_date, ip_address, login_type,
                  device_name, location
           FROM ecommerce.login
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_login(pool: &PgPool, id: i64) -> AppResult<Option<LoginRow>> {
    let row = sqlx::query_as::<_, LoginRow>(
        r#"SELECT id, user_id, login_date, ip_address, login_type,
                  device_name, location
           FROM ecommerce.login
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_logins(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_logins(pool).await?;
        rows.iter().map(LoginRow::cache_doc).collect()
    })
}

pub fn fetch_login(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_login(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// ----------------------------------------------------- shopping carts -----

async fn query_all_shopping_carts(pool: &PgPool) -> AppResult<Vec<ShoppingCartRow>> {
    let rows = sqlx::query_as::<_, ShoppingCartRow>(
        r#"SELECT id, order_id, customer_id, product_id, quantity,
                  created_at, updated_at
           FROM ecommerce.shopping_cart
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_shopping_cart(pool: &PgPool, id: i64) -> AppResult<Option<ShoppingCartRow>> {
    let row = sqlx::query_as::<_, ShoppingCartRow>(
        r#"SELECT id, order_id, customer_id, product_id, quantity,
                  created_at, updated_at
           FROM ecommerce.shopping_cart
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_shopping_carts(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_shopping_carts(pool).await?;
        rows.iter().map(ShoppingCartRow::cache_doc).collect()
    })
}

pub fn fetch_shopping_cart(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_shopping_cart(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// ------------------------------------------------------- payment infos ----

async fn query_all_payment_infos(pool: &PgPool) -> AppResult<Vec<PaymentInfoRow>> {
    let rows = sqlx::query_as::<_, PaymentInfoRow>(
        r#"SELECT id, order_id, payment_method,
                  payment_amount::text AS payment_amount,
                  payment_status, payment_date, created_at, updated_at
           FROM ecommerce.payment_info
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_payment_info(pool: &PgPool, id: i64) -> AppResult<Option<PaymentInfoRow>> {
    let row = sqlx::query_as::<_, PaymentInfoRow>(
        r#"SELECT id, order_id, payment_method,
                  payment_amount::text AS payment_amount,
                  payment_status, payment_date, created_at, updated_at
           FROM ecommerce.payment_info
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_payment_infos(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_payment_infos(pool).await?;
        rows.iter().map(PaymentInfoRow::cache_doc).collect()
    })
}

pub fn fetch_payment_info(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_payment_info(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// ------------------------------------------------------------ payments ----

async fn query_all_payments(pool: &PgPool) -> AppResult<Vec<PaymentRow>> {
    let rows = sqlx::query_as::<_, PaymentRow>(
        r#"SELECT id, order_id, payment_date,
                  payment_amount::text AS payment_amount,
                  payment_status, created_at, updated_at
           FROM ecommerce.payment
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_payment(pool: &PgPool, id: i64) -> AppResult<Option<PaymentRow>> {
    let row = sqlx::query_as::<_, PaymentRow>(
        r#"SELECT id, order_id, payment_date,
                  payment_amount::text AS payment_amount,
                  payment_status, created_at, updated_at
           FROM ecommerce.payment
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_payments(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_payments(pool).await?;
        rows.iter().map(PaymentRow::cache_doc).collect()
    })
}

pub fn fetch_payment(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_payment(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}

// --------------------------------------------------- shipment trackings ---

async fn query_all_shipment_trackings(pool: &PgPool) -> AppResult<Vec<ShipmentTrackingRow>> {
    let rows = sqlx::query_as::<_, ShipmentTrackingRow>(
        r#"SELECT id, shipment_id, tracking_number, status, updated_at
           FROM ecommerce.shipment_tracking
           ORDER BY id"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn query_shipment_tracking(
    pool: &PgPool,
    id: i64,
) -> AppResult<Option<ShipmentTrackingRow>> {
    let row = sqlx::query_as::<_, ShipmentTrackingRow>(
        r#"SELECT id, shipment_id, tracking_number, status, updated_at
           FROM ecommerce.shipment_tracking
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub fn fetch_all_shipment_trackings(pool: &PgPool) -> DbFut<'_, Vec<CacheDoc>> {
    Box::pin(async move {
        let rows = query_all_shipment_trackings(pool).await?;
        rows.iter().map(ShipmentTrackingRow::cache_doc).collect()
    })
}

pub fn fetch_shipment_tracking(pool: &PgPool, id: i64) -> DbFut<'_, Option<CacheDoc>> {
    Box::pin(async move {
        match query_shipment_tracking(pool, id).await? {
            Some(row) => Ok(Some(row.cache_doc()?)),
            None => Ok(None),
        }
    })
}
