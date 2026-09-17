# ecomrust

The Rust backend for the ecommerce admin app. It holds three main programs:

1. **cache_loader**: a one-shot tool that copies all orders and shipments out
   of PostgreSQL and writes them into Redis.
2. **orders_api**: a small web server on port 4001 that answers questions
   about orders, reading only from Redis.
3. **shipments_api**: the same thing for shipments, on port 4002.

There are also two optional programs (`grpc_server` and `grpc_client`) and an
optional Node.js gateway, described at the bottom of this file.

The idea behind the design: PostgreSQL is the permanent store, but reading
100,000-row tables page by page is slow. So we copy the two hot tables into
Redis once, and the APIs serve every request from memory. The web frontend
(in the sibling `ecom` repo) talks to these APIs through its own proxy, so
you normally start this repo's programs first.

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

Build everything (all five binaries) in debug mode:

```bash
cargo build
```

Or build just the one binary you need:

```bash
cargo build --bin cache_loader
cargo build --bin orders_api
cargo build --bin shipments_api
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

Order matters. Run `cache_loader` first and keep it a habit: the two APIs
read only from Redis, so if the cache is empty they return empty pages even
though PostgreSQL is full.

### 1. cache_loader

Copies every order and shipment from PostgreSQL into Redis.

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
cache_loader: done
```

It exits when finished. Re-run it any time the PostgreSQL data changes and
you want the cache refreshed.

### 2. orders API

Web server for orders, listening on `http://localhost:4001`. Runs forever
until you stop it with Ctrl+C. Give it its own terminal window.

Build:

```bash
cargo build --bin orders_api
```

Run:

```bash
cargo run --bin orders_api
```

Expected output:

```
orders API listening on http://0.0.0.0:4001 (Redis: redis://localhost:6379/0)
```

Try it from another terminal:

```bash
curl localhost:4001/health
curl localhost:4001/orders?page=1\&page_size=2
curl localhost:4001/order/1
```

### 3. shipments API

Web server for shipments, listening on `http://localhost:4002`. Give it its
own terminal window too.

Build:

```bash
cargo build --bin shipments_api
```

Run:

```bash
cargo run --bin shipments_api
```

Expected output:

```
shipments API listening on http://0.0.0.0:4002 (Redis: redis://localhost:6379/0)
```

Try it:

```bash
curl localhost:4002/health
curl localhost:4002/shipments?page=1\&page_size=2
curl localhost:4002/shipment/1
```

## API reference

Both APIs speak JSON and share the same rules.

### List endpoints

```
GET /orders?page=1&page_size=50&sort=id&order=asc
GET /shipments?page=1&page_size=50&sort=id&order=asc
```

| Parameter | Default | Allowed values |
| --- | --- | --- |
| `page` | 1 | whole number, 1 or higher |
| `page_size` | 50 | whole number, 1 to 200 |
| `sort` | `id` | orders: `id` or `order_date`; shipments: `id` or `shipment_date` |
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

Money fields like `total_amount` and `shipment_cost` are strings on purpose:
PostgreSQL `NUMERIC` values are converted to text so no decimal precision is
lost to floating-point rounding.

### Single-row endpoints

```
GET /order/{id}      -> one order, or 404 if that id is not cached
GET /shipment/{id}   -> one shipment, or 404 if that id is not cached
```

`id` must be 1 or higher.

### Health check

```
GET /health   -> {"service":"orders","status":"ok"}    (port 4001)
GET /health   -> {"service":"shipments","status":"ok"} (port 4002)
```

### Errors

Every error is JSON with one `error` field:

- `400` bad input, for example `{"error":"bad request: page must be >= 1"}`
  or `invalid sort 'xyz'; allowed: id, order_date`
- `404` missing row, for example `{"error":"not found: orders id 42 not in cache"}`
- `500` something broke on the server side (Redis down, bad JSON, and so on)

## How the cache works

For each entity, Redis holds three keys per row plus two index sets
(defined in `src/cache.rs`):

```
orders:{id}                    the full row as a JSON string
orders:index:id                sorted set: score = id, member = id
orders:index:order_date        sorted set: score = date in epoch ms, member = id
shipments:{id}                 the full row as a JSON string
shipments:index:id             sorted set: score = id, member = id
shipments:index:shipment_date  sorted set: score = date in epoch ms, member = id
```

A sorted set is a Redis structure that keeps its members ordered by a numeric
score, which makes "give me items 51 to 100, oldest first" a single cheap
operation. A list request works in two steps: pick the page's ids out of the
right sorted set (`ZRANGE` for ascending, `ZREVRANGE` for descending), then
fetch all the row JSON in one `MGET`. No matter the page size, that is two
round trips to Redis, never one trip per row.

`cache_loader` writes with pipelines of 5,000 rows at a time, so loading
100,000 rows takes a few seconds.

One spec note: the original spec asked for a Redis database called `ecomdb`.
Redis databases are only numbered (0 to 15), they have no names, so this
project uses database 0 and gives every key an `orders:` or `shipments:`
prefix to keep the namespaces separate.

## Configuration

Everything works with no configuration on a default local machine. To point
at other hosts, set these environment variables:

| Variable | Default | Used by | Meaning |
| --- | --- | --- | --- |
| `DATABASE_URL` | `postgresql://harir@localhost:5432/ecomdb` | cache_loader | where PostgreSQL lives |
| `REDIS_URL` | `redis://localhost:6379/0` | all Rust binaries | where Redis lives |

Example:

```bash
REDIS_URL=redis://192.168.1.10:6379/0 cargo run --bin orders_api
```

## Optional extras

These are not needed for the web app in the `ecom` repo, but they work and
are covered by the same build.

### gRPC server and smoke-test client

`grpc_server` serves the same orders and shipments over gRPC (Protocol
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
│   ├── cache_loader.rs    Postgres -> Redis copy tool
│   ├── orders_api.rs      REST server for orders, port 4001
│   ├── shipments_api.rs   REST server for shipments, port 4002
│   ├── grpc_server.rs     gRPC server, port 50051
│   └── grpc_client.rs     gRPC smoke test
├── cache.rs               Redis key layout, page reads, pipelined writes
├── db.rs                  PostgreSQL queries used by cache_loader
├── models.rs              row types and the page/sort parameter types
├── rest.rs                shared query validation and response envelope
├── grpc.rs                gRPC handlers (same cache logic as REST)
├── error.rs               one error type for REST and gRPC
└── lib.rs                 module wiring + generated protobuf stubs
proto/ecom.proto           the gRPC contract, shared with the Node gateway
gateway/server.js          optional Fastify gateway on port 4000
build.rs                   build script: compiles the proto with vendored protoc
specs.md                   the original requirements this repo was built from
claude.md                  architecture and coding rules
```

## Troubleshooting

**The APIs answer but every list is empty (`"total_records": 0`).**
Redis has not been loaded yet. Run `cargo run --bin cache_loader`.

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
