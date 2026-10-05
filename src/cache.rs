//! Redis cache layer (specs.md #1, #5).
//!
//! Key layout per entity (prefix = "orders" | "shipments" | "users" | ...):
//!   {prefix}:{id}                -> JSON string of the row      (GET by id)
//!   {prefix}:index:id            -> ZSET score=id,   member=id  (sort by id)
//!   {prefix}:index:{date_field}  -> ZSET score=epoch ms of the
//!                                   date column,    member=id  (sort by date)
//!
//! List pages are resolved with ZRANGE/ZREVRANGE against an index, then a
//! single MGET fetches the row documents — no per-row round trips (claude.md
//! rule #5: avoid N+1 patterns).
//!
//! Reads are cache-aside: the REST tier serves from Redis first and, on a
//! miss (single doc absent or index never loaded), fetches from PostgreSQL
//! and writes the result back through `load` before responding.
//!
//! Note: Redis logical databases are numeric (0-15); "Database: ecomdb" from
//! specs.md is satisfied by connecting to DB 0 and namespacing every key with
//! the entity prefixes (`orders:`, `shipments:`, `users:`, ...).

use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::{CacheDoc, PageParams, SortDir};

pub const DEFAULT_REDIS_URL: &str = "redis://localhost:6379/0";

pub const ORDERS_PREFIX: &str = "orders";
pub const ORDERS_DATE_FIELD: &str = "order_date";
pub const SHIPMENTS_PREFIX: &str = "shipments";
pub const SHIPMENTS_DATE_FIELD: &str = "shipment_date";
pub const USERS_PREFIX: &str = "users";
pub const USERS_DATE_FIELD: &str = "created_at";
pub const LOGINS_PREFIX: &str = "logins";
pub const LOGINS_DATE_FIELD: &str = "login_date";
pub const SHOPPING_CARTS_PREFIX: &str = "shopping-carts";
pub const SHOPPING_CARTS_DATE_FIELD: &str = "created_at";
pub const PAYMENT_INFOS_PREFIX: &str = "payment-infos";
pub const PAYMENT_INFOS_DATE_FIELD: &str = "payment_date";
pub const PAYMENTS_PREFIX: &str = "payments";
pub const PAYMENTS_DATE_FIELD: &str = "payment_date";
pub const SHIPMENT_TRACKINGS_PREFIX: &str = "shipment-trackings";
pub const SHIPMENT_TRACKINGS_DATE_FIELD: &str = "updated_at";

const PIPELINE_BATCH: usize = 5_000;

#[derive(Clone)]
pub struct Cache {
    conn: MultiplexedConnection,
}

impl Cache {
    pub async fn connect(redis_url: &str) -> AppResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn })
    }

    // ---------------------------------------------------------- loading ---

    /// Load documents into Redis: one JSON key per row plus the id and date
    /// sort indexes, pipelined in batches. Used for the full seed
    /// (cache_loader / cold list fallback) and single-doc write-back.
    pub async fn load(
        &self,
        prefix: &str,
        date_field: &str,
        docs: &[CacheDoc],
    ) -> AppResult<usize> {
        let mut conn = self.conn.clone();
        for chunk in docs.chunks(PIPELINE_BATCH) {
            let mut pipe = redis::pipe();
            for doc in chunk {
                let id = doc.id.to_string();
                pipe.set(format!("{prefix}:{id}"), doc.json.clone()).ignore();
                pipe.zadd(format!("{prefix}:index:id"), id.clone(), doc.id as f64)
                    .ignore();
                pipe.zadd(
                    format!("{prefix}:index:{date_field}"),
                    id,
                    doc.date_epoch_ms as f64,
                )
                .ignore();
            }
            pipe.query_async::<()>(&mut conn).await?;
        }
        Ok(docs.len())
    }

    /// True when the entity's id sort index exists, i.e. the cache has been
    /// loaded for this entity at least once.
    pub async fn index_exists(&self, prefix: &str) -> AppResult<bool> {
        let mut conn = self.conn.clone();
        Ok(conn.exists(format!("{prefix}:index:id")).await?)
    }

    // ---------------------------------------------------------- reading ---

    /// Paginated + sorted list from the cache.
    /// Returns (rows for the page, total_records).
    pub async fn list(
        &self,
        prefix: &str,
        date_field: &str,
        params: &PageParams,
    ) -> AppResult<(Vec<Value>, i64)> {
        // API validation (specs.md #6): sort field restricted to known indexes.
        if params.sort != "id" && params.sort != date_field {
            return Err(AppError::BadRequest(format!(
                "invalid sort '{}'; allowed: id, {}",
                params.sort, date_field
            )));
        }
        if params.page < 1 {
            return Err(AppError::BadRequest("page must be >= 1".to_string()));
        }
        if !(1..=200).contains(&params.page_size) {
            return Err(AppError::BadRequest(
                "page_size must be between 1 and 200".to_string(),
            ));
        }

        let index_key = format!("{prefix}:index:{}", params.sort);
        let mut conn = self.conn.clone();

        let total: i64 = conn.zcard(&index_key).await?;
        let start = (params.page - 1) * params.page_size;
        let stop = start + params.page_size - 1;

        let ids: Vec<String> = match params.dir {
            SortDir::Asc => conn.zrange(&index_key, start as isize, stop as isize).await?,
            SortDir::Desc => {
                conn.zrevrange(&index_key, start as isize, stop as isize).await?
            }
        };

        if ids.is_empty() {
            return Ok((Vec::new(), total));
        }

        let keys: Vec<String> = ids.iter().map(|id| format!("{prefix}:{id}")).collect();
        let docs: Vec<Option<String>> = conn.mget(&keys).await?;
        let rows = docs
            .into_iter()
            .flatten()
            .map(|s| serde_json::from_str(&s))
            .collect::<Result<Vec<Value>, _>>()?;
        Ok((rows, total))
    }

    /// Raw cached JSON for one row; `None` on cache miss. The REST tier
    /// serves the stored string verbatim (no re-serialization).
    pub async fn get_raw(&self, prefix: &str, id: i64) -> AppResult<Option<String>> {
        let mut conn = self.conn.clone();
        let doc: Option<String> = conn.get(format!("{prefix}:{id}")).await?;
        Ok(doc)
    }

    /// Single row by id from the cache, deserialized. 404s when absent.
    /// Used by the gRPC tier.
    pub async fn get<T: DeserializeOwned>(&self, prefix: &str, id: i64) -> AppResult<T> {
        match self.get_raw(prefix, id).await? {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Err(AppError::NotFound(format!("{prefix} id {id} not in cache"))),
        }
    }
}
