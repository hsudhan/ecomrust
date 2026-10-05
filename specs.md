    # Specs for E-commerce Application with Rust API

## Tech Stack Specifications

### Programming Language
Rust

### Database
Postgres (via SQL dialect)

### Cache Layer
Redis

### API Framework
[Your preferred API framework (e.g., actix-web, async-std, etc.)]

### Web Application
Node.js (using Fastify)

### gRPC
[Your preferred gRPC library (e.g., tonic, etc.)]

## Design Requirements

### 1. Cache Loading

* Fetch data from all the below tables from the Postgres database.
* **ORDERS**
* **SHIPMENT**
* **USERS**
* **LOGIN**
* **SHOPPING CART**
* **PAYMENT INFO**
* **PAYMENT**
* **SHIPMENT TRACKING**

* Load the data into Redis using a cache layer.
* Store the data in Redis using a key-value store (e.g., `orders:123` or `shipments:123`).

### 2. API Endpoints

* Expose the following APIs:
	+ `GET /orders`: Fetch a list of orders from Redis cache.
	+ `GET /shipments`: Fetch a list of shipments from Redis cache.
	+ `GET /order/:id`: Fetch a single order from Redis cache by ID.
	+ `GET /shipment/:id`: Fetch a single shipment from Redis cache by ID.
* APIs should handle pagination and sorting for `GET /orders` and `GET /shipments` endpoints.
orders API should listen on 4001 port
shipments API should listen on 4002 port 

similarly expose URLs for other tabs also as mentioned below. 
users API should listen on 4003 port 
login API should listen on 4004 port 
shopping_cart API should listen on 4005 port 
payment_info API should listen on 4006 port 
payment API should listen on 4007 port 
shipment_tracking API should listen on 4008 port 

they should be built and run using cargo commands.

### 3. Integration with Node.js Application

* Use Fastify to create a Node.js application that will call the Rust API endpoints.
* Use the gRPC library to invoke the Rust API endpoints from the Node.js application.
* Handle API responses from Rust in the Node.js application.

### 4. Database Connection

* Use the `postgresql://harir@localhost:5432/ecomdb` connection string to connect to the Postgres database.

### 5. Redis Configuration

* Use a Redis instance with the following configuration:
	+ Host: `localhost`
	+ Port: `6379`
	+ Database: `ecomdb`
* Store data in a key-value store using the following prefix:
	+ `orders:`
	+ `shipments:`

### 6. Rust Application

* Create a Rust crate that will handle the database queries and Redis caching.
* Use the `actix-web` framework to create a web server that exposes the APIs.
* Handle API requests and responses using the `async` and `await` keywords.
* Use the `tonic` library to create gRPC server that exposes the APIs.
* Implement API validation and error handling for API requests.

### 7. Node.js Application

* Create a Fastify application that will call the Rust API endpoints.
* Use the gRPC library to invoke the Rust API endpoints from the Node.js application.
* Handle API responses from the Rust application.
* Integrate the response data with the e-commerce web application.