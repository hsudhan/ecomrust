//! Redis cache layer (specs.md #1, #5).
//!
//! Key layout per entity (prefix = "orders" | "shipments"):
//!   {prefix}:{id}                -> JSON string of the row      (GET by id)
//!   {prefix}:index:id            -> ZSET score=id,   member=id  (sort by id)
//!   {prefix}:index:{date_field}  -> ZSET score=epoch ms of the
//!                                   date column,    member=id  (sort by date)
//!
//! List pages are resolved with ZRANGE/ZREVRANGE against an index, then a
//! single MGET fetches the row documents — no per-row round trips (claude.md
//! rule #5: avoid N+1 patterns).
//!
//! Note: Redis logical databases are numeric (0-15); "Database: ecomdb" from
//! specs.md is satisfied by connecting to DB 0 and namespacing every key with
//! the required prefixes (`orders:`, `shipments:`).

use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::{OrderJson, OrderRow, PageParams, ShipmentJson, ShipmentRow, SortDir};

pub const DEFAULT_REDIS_URL: &str = "redis://localhost:6379/0";
pub const ORDERS_PREFIX: &str = "orders";
pub const SHIPMENTS_PREFIX: &str = "shipments";
pub const ORDERS_DATE_FIELD: &str = "order_date";
pub const SHIPMENTS_DATE_FIELD: &str = "shipment_date";

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

    pub async fn load_orders(&self, rows: Vec<OrderRow>) -> AppResult<usize> {
        let mut conn = self.conn.clone();
        let count = rows.len();
        for chunk in rows.chunks(PIPELINE_BATCH) {
            let mut pipe = redis::pipe();
            for row in chunk {
                let json = serde_json::to_string(&OrderJson::from(row.clone()))?;
                let id = row.id.to_string();
                let epoch_ms = row.order_date.timestamp_millis();
                pipe.set(format!("{ORDERS_PREFIX}:{id}"), json).ignore();
                pipe.zadd(format!("{ORDERS_PREFIX}:index:id"), id.clone(), row.id as f64)
                    .ignore();
                pipe.zadd(
                    format!("{ORDERS_PREFIX}:index:{ORDERS_DATE_FIELD}"),
                    id,
                    epoch_ms as f64,
                )
                .ignore();
            }
            pipe.query_async::<()>(&mut conn).await?;
        }
        Ok(count)
    }

    pub async fn load_shipments(&self, rows: Vec<ShipmentRow>) -> AppResult<usize> {
        let mut conn = self.conn.clone();
        let count = rows.len();
        for chunk in rows.chunks(PIPELINE_BATCH) {
            let mut pipe = redis::pipe();
            for row in chunk {
                let json = serde_json::to_string(&ShipmentJson::from(row.clone()))?;
                let id = row.id.to_string();
                let epoch_ms = row.shipment_date.timestamp_millis();
                pipe.set(format!("{SHIPMENTS_PREFIX}:{id}"), json).ignore();
                pipe.zadd(format!("{SHIPMENTS_PREFIX}:index:id"), id.clone(), row.id as f64)
                    .ignore();
                pipe.zadd(
                    format!("{SHIPMENTS_PREFIX}:index:{SHIPMENTS_DATE_FIELD}"),
                    id,
                    epoch_ms as f64,
                )
                .ignore();
            }
            pipe.query_async::<()>(&mut conn).await?;
        }
        Ok(count)
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

    /// Single row by id from the cache. 404s when absent.
    pub async fn get<T: DeserializeOwned>(&self, prefix: &str, id: i64) -> AppResult<T> {
        let mut conn = self.conn.clone();
        let doc: Option<String> = conn.get(format!("{prefix}:{id}")).await?;
        match doc {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Err(AppError::NotFound(format!("{prefix} id {id} not in cache"))),
        }
    }
}
