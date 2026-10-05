# ecomrust

The Rust backend for the ecommerce admin app. It holds nine main programs:

1. **cache_loader**: a one-shot tool that copies all eight ecommerce tables
   (orders, shipments, users, logins, shopping carts, payment infos,
   payments, shipment trackings) out of PostgreSQL and writes them into
   Redis.
2. **eight small web servers**, one per entity, on ports 4001-4008. Each one
   answers questions about its table, reading from Redis first and falling
   back to PostgreSQL on a cache miss.

There are also two optional programs (`grpc_server` and `grpc_client`) and an
optional Node.js gateway, described at the bottom of this file.

The idea behind the design: PostgreSQL is the permanent store, but reading
100,000-row tables page by page is slow. So every API serves from Redis
whenever it can. A read that misses the cache — a single row that was never
loaded, or a whole table on a cold start — is fetched from PostgreSQL,
written back into Redis, and only then returned (this is called
*cache-aside*). `cache_loader` pre-warms everything so the first request is
never a miss. The web frontend (in the sibling `ecom` repo) talks to these
APIs through its own proxy, so you normally start this repo's programs first.

## What you need installed

- **Rust with cargo**. Check with `cargo --version`. If you do not have it,
  install it from https://rustup.rs.
- **Redis** running on `localhost:6379`. Check with `redis-cli ping`; it
  should answer `PONG`.
- **PostgreSQL** running on `localhost:5432`, with the `ecomdb` database
  already created and filled. If you have not done that yet, do it first in
  the sibling repo:

  ```bash
  cd ../ecom
  createdb ecomdb
  psql -d ecomdb -f DDL.sql
  python3 load_data.py
  ```

You do **not** need to install protoc (the Protocol Buffers compiler). The
build script uses a vendored copy that ships with the project, so
`cargo build` works out of the box.

## Setup

Clone this repo next to the `ecom` repo, so both live in the same folder:

```
workspace/
├── ecom/        <- web app + legacy API
└── ecomrust/    <- this repo
```

There is nothing else to install for the Rust programs. `cargo build`
downloads all dependencies by itself.

## Build

Build everything (all eleven binaries) in debug mode:

```bash
cargo build
```

Or build just the one binary you need:

```bash
cargo build --bin cache_loader
cargo build --bin orders_api
cargo build --bin users_api
```

For an optimized production build, add `--release`:

```bash
cargo build --release
```

The compiled programs land in `target/debug/` (or `target/release/`). A
release binary can also be started directly, without cargo:

```bash
./target/release/orders_api
```

## Run

### 1. cache_loader

Copies every row of the eight exposed tables from PostgreSQL into Redis.

Build:

```bash
cargo build --bin cache_loader
```

Run:

```bash
cargo run --bin cache_loader
```

Expected output:

```
cache_loader: connecting to PostgreSQL (postgresql://harir@localhost:5432/ecomdb)
cache_loader: connecting to Redis (redis://localhost:6379/0)
cache_loader: fetched 100000 orders from PostgreSQL
cache_loader: loaded 100000 orders into Redis (orders:*)
cache_loader: fetched 100000 shipments from PostgreSQL
cache_loader: loaded 100000 shipments into Redis (shipments:*)
... (users, logins, shopping-carts, payment-infos, payments, shipment-trackings)
cache_loader: done
```

It exits when finished. Re-run it any time the PostgreSQL data changes and
you want the cache refreshed.

Running it is recommended but no longer strictly required: the APIs load
their table from PostgreSQL automatically the first time they notice an
empty cache (see "Cache-aside reads" below). Pre-loading just keeps that
first request fast.

### 2. The eight APIs

One web server per entity.

| Binary | Port | List endpoint | Single-row endpoint |
| --- | --- | --- | --- |
| `orders_api` | 4001 | `GET /orders` | `GET /order/{id}` |
| `shipments_api` | 4002 | `GET /shipments` | `GET /shipment/{id}` |
| `users_api` | 4003 | `GET /users` | `GET /user/{id}` |
| `logins_api` | 4004 | `GET /logins` | `GET /login/{id}` |
| `shopping_carts_api` | 4005 | `GET /shopping-carts` | `GET /shopping-cart/{id}` |
| `payment_infos_api` | 4006 | `GET /payment-infos` | `GET /payment-info/{id}` |
| `payments_api` | 4007 | `GET /payments` | `GET /payment/{id}` |
| `shipment_trackings_api` | 4008 | `GET /shipment-trackings` | `GET /shipment-tracking/{id}` |

#### Start them all at once (recommended)

```bash
./start_apis.sh
```

One command builds the binaries, launches all eight services in the
background with `nohup`, health-checks each one, and prints a summary:

```
SERVICE                    PORT   HEALTH    LOG
orders_api                 4001   ok        .../ecomrust-apis/orders_api.log
shipments_api              4002   ok        .../ecomrust-apis/shipments_api.log
users_api                  4003   ok        .../ecomrust-apis/users_api.log
logins_api                 4004   ok        .../ecomrust-apis/logins_api.log
shopping_carts_api         4005   ok        .../ecomrust-apis/shopping_carts_api.log
payment_infos_api          4006   ok        .../ecomrust-apis/payment_infos_api.log
payments_api               4007   ok        .../ecomrust-apis/payments_api.log
shipment_trackings_api     4008   ok        .../ecomrust-apis/shipment_trackings_api.log
```

Logs land in `${TMPDIR:-/tmp}/ecomrust-apis/` (override with `LOG_DIR`).
Re-running the script skips ports that are already listening. Stop
everything with:

```bash
pkill -f 'target/debug/.*_api'
```

#### Or run each in its own terminal

```bash
cargo run --bin orders_api              # listens on :4001
cargo run --bin shipments_api           # listens on :4002
cargo run --bin users_api               # listens on :4003
cargo run --bin logins_api              # listens on :4004
cargo run --bin shopping_carts_api      # listens on :4005
cargo run --bin payment_infos_api       # listens on :4006
cargo run --bin payments_api            # listens on :4007
cargo run --bin shipment_trackings_api  # listens on :4008
```

Expected output (users example):

```
users API listening on http://0.0.0.0:4003 (Redis: redis://localhost:6379/0, PostgreSQL: postgresql://harir@localhost:5432/ecomdb)
```

Try it from another terminal:

```bash
curl localhost:4003/health
curl localhost:4003/users?page=1\&page_size=2
curl localhost:4003/user/1
```

All eight binaries are thin wrappers around one shared server
(`src/rest.rs`): they differ only in port, paths, Redis prefix, sortable date
column, and which database query to run on a cache miss.

## API reference

All eight APIs speak JSON and share the same rules.

### List endpoints

```
GET /orders?page=1&page_size=50&sort=id&order=asc            (port 4001)
GET /shipments?page=1&page_size=50&sort=id&order=asc         (port 4002)
GET /users?page=1&page_size=50&sort=id&order=asc             (port 4003)
GET /logins?page=1&page_size=50&sort=id&order=asc            (port 4004)
GET /shopping-carts?page=1&page_size=50&sort=id&order=asc    (port 4005)
GET /payment-infos?page=1&page_size=50&sort=id&order=asc     (port 4006)
GET /payments?page=1&page_size=50&sort=id&order=asc          (port 4007)
GET /shipment-trackings?page=1&page_size=50&sort=id&order=asc (port 4008)
```

| Parameter | Default | Allowed values |
| --- | --- | --- |
| `page` | 1 | whole number, 1 or higher |
| `page_size` | 50 | whole number, 1 to 200 |
| `sort` | `id` | `id` or the entity's date column: `order_date` (orders), `shipment_date` (shipments), `created_at` (users, shopping-carts), `login_date` (logins), `payment_date` (payment-infos, payments), `updated_at` (shipment-trackings) |
| `order` | `asc` | `asc` (smallest/oldest first) or `desc` (newest first) |

The answer looks like this:

```json
{
  "data": [ { "id": 1, "customer_id": 8123, "order_date": "2026-09-10T04:12:55+00:00", "status": "SHIPPED", "total_amount": "2841.33", "created_at": "...", "updated_at": "..." } ],
  "page": 1,
  "page_size": 50,
  "total_records": 100000,
  "total_pages": 2000,
  "sort": "id",
  "order": "asc"
}
```

Money fields like `total_amount`, `shipment_cost`, and `payment_amount` are
strings on purpose: PostgreSQL `NUMERIC` values are converted to text so no
decimal precision is lost to floating-point rounding.

### Single-row endpoints

```
GET /order/{id}              -> one order, or 404
GET /shipment/{id}           -> one shipment, or 404
GET /user/{id}               -> one user (never includes the password column), or 404
GET /login/{id}              -> one login row, or 404
GET /shopping-cart/{id}      -> one cart row, or 404
GET /payment-info/{id}       -> one payment info row, or 404
GET /payment/{id}            -> one payment, or 404
GET /shipment-tracking/{id}  -> one tracking row, or 404
```

`id` must be 1 or higher.

### Health check

```
GET /health   -> {"service":"orders","status":"ok"}            (port 4001)
GET /health   -> {"service":"shipment-trackings","status":"ok"} (port 4008)
... and so on for each service
```

### Errors

Every error is JSON with one `error` field:

- `400` bad input, for example `{"error":"bad request: page must be >= 1"}`
  or `invalid sort 'xyz'; allowed: id, created_at`
- `404` missing row, for example
  `{"error":"not found: users id 42 not found in cache or database"}`
- `500` something broke on the server side (Redis down, bad JSON, and so on)

## Cache-aside reads (Redis first, database on miss)

Every request is answered from Redis if possible. On a miss, the service
reads the PostgreSQL table, writes the result back into Redis, and then
responds — so the next request for the same data is fast again.

- **Single row**: a missing `{prefix}:{id}` key triggers
  `SELECT ... WHERE id = $1`. A row missing from the database too gets a 404.
- **List**: a missing `{prefix}:index:id` sorted set (the entity was never
  loaded) triggers a full-table load before the page is served. The service
  log shows a line like
  `users API: cold cache -> loaded 10000 rows from PostgreSQL (users:*)`.

The database connection is lazy: a service starts and keeps serving from
Redis even if PostgreSQL is down, as long as the cache is warm.

## How the cache works

Each entity uses the same three-key layout under its own prefix
(`orders:`, `shipments:`, `users:`, `logins:`, `shopping-carts:`,
`payment-infos:`, `payments:`, `shipment-trackings:`), defined in
`src/cache.rs`:

```
orders:{id}                    the full row as a JSON string
orders:index:id                sorted set: score = id, member = id
orders:index:order_date        sorted set: score = date in epoch ms, member = id
```

A sorted set is a Redis structure that keeps its members ordered by a numeric
score, which makes "give me items 51 to 100, oldest first" a single cheap
operation. A list request works in two steps: pick the page's ids out of the
right sorted set (`ZRANGE` for ascending, `ZREVRANGE` for descending), then
fetch all the row JSON in one `MGET`. No matter the page size, that is two
round trips to Redis, never one trip per row.

Writes (`cache_loader` and cache-aside write-backs) use pipelines of 5,000
rows at a time, so loading 100,000 rows takes a few seconds.

One spec note: the original spec asked for a Redis database called `ecomdb`.
Redis databases are only numbered (0 to 15), they have no names, so this
project uses database 0 and gives every key an entity prefix to keep the
namespaces separate.

## Configuration

Everything works with no configuration on a default local machine. To point
at other hosts, set these environment variables:

| Variable | Default | Used by | Meaning |
| --- | --- | --- | --- |
| `DATABASE_URL` | `postgresql://harir@localhost:5432/ecomdb` | cache_loader, all eight APIs (cache misses only) | where PostgreSQL lives |
| `REDIS_URL` | `redis://localhost:6379/0` | all Rust binaries | where Redis lives |

Example:

```bash
REDIS_URL=redis://192.168.1.10:6379/0 cargo run --bin orders_api
```

## Optional extras

These are not needed for the web app in the `ecom` repo, but they work and
are covered by the same build.

### gRPC server and smoke-test client

`grpc_server` serves orders and shipments over gRPC (Protocol
Buffers over HTTP/2) on port 50051, using the contract in
`proto/ecom.proto`. `grpc_client` is a tiny smoke test that calls it.

```bash
cargo run --bin grpc_server    # terminal A
cargo run --bin grpc_client    # terminal B, prints a few rows and "OK"
```

### Node.js gateway

`gateway/server.js` is a Fastify server on port 4000. It forwards
orders/shipments requests to the gRPC server and answers six other entity
types (users, logins, shopping-carts, payment-infos, payments,
shipment-trackings) directly from PostgreSQL. It needs Node.js >= 18:

```bash
cd gateway
npm install
node server.js
```

Useful gateway environment variables: `GATEWAY_PORT` (default 4000),
`GRPC_ADDR` (default `localhost:50051`), `DATABASE_URL` (same default as
above).

## Project layout

```
src/
├── bin/
│   ├── cache_loader.rs            Postgres -> Redis copy tool (all 8 tables)
│   ├── orders_api.rs              REST server for orders, port 4001
│   ├── shipments_api.rs           REST server for shipments, port 4002
│   ├── users_api.rs               REST server for users, port 4003
│   ├── logins_api.rs              REST server for logins, port 4004
│   ├── shopping_carts_api.rs      REST server for shopping carts, port 4005
│   ├── payment_infos_api.rs       REST server for payment infos, port 4006
│   ├── payments_api.rs            REST server for payments, port 4007
│   ├── shipment_trackings_api.rs  REST server for shipment trackings, port 4008
│   ├── grpc_server.rs             gRPC server, port 50051
│   └── grpc_client.rs             gRPC smoke test
├── cache.rs               Redis key layout, page reads, pipelined writes
├── db.rs                  PostgreSQL queries (loader + cache-miss fallback)
├── models.rs              row types, cached-document type, page/sort params
├── rest.rs                the one generic server every API bin configures
├── grpc.rs                gRPC handlers (same cache logic as REST)
├── error.rs               one error type for REST and gRPC
└── lib.rs                 module wiring + generated protobuf stubs
proto/ecom.proto           the gRPC contract, shared with the Node gateway
gateway/server.js          optional Fastify gateway on port 4000
start_apis.sh              one command to start all eight APIs in the background
build.rs                   build script: compiles the proto with vendored protoc
specs.md                   the original requirements this repo was built from
claude.md                  architecture and coding rules
```

## Troubleshooting

**The first request for an entity is slow, then everything is fast.**
That entity's cache was cold; the service loaded the whole table from
PostgreSQL before answering (check the log for `cold cache -> loaded N
rows`). Pre-warm with `cargo run --bin cache_loader`.

**A single-row request returns 404 but the row exists in PostgreSQL.**
Should not happen any more: misses fall back to the database. If you still
see it, the row was inserted with a non-positive id, or you are talking to
the wrong Redis/Postgres pair — check `REDIS_URL` and `DATABASE_URL`.

**`cargo run --bin orders_api` fails immediately with a connection error.**
Redis is not running or not reachable at `redis://localhost:6379/0`. Start
Redis, check `redis-cli ping`, or set `REDIS_URL` to the right address.

**`cache_loader` fails to connect to PostgreSQL.**
PostgreSQL is not running, or the default connection string does not match
your machine. Fix the string with `DATABASE_URL`, and make sure `ecomdb`
exists and was seeded (see "What you need installed").

**`curl: (7) Failed to connect to localhost port 4001`.**
The API is not running. Start it with `cargo run --bin orders_api` and look
for the "listening on http://0.0.0.0:4001" line.

**Port already in use.**
Something else holds the port (often a previous run of the same binary).
Stop it with Ctrl+C in its terminal, or find and kill the process:
`lsof -ti :4001 | xargs kill`.
