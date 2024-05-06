Webhook faucet
==============
Buffer high-load webhook request and push to downstream webhook handler with adaptive rate control
# Motivation and design
The downstream backend this project was first developed for is a Shopify application that subscribe to webhook topics from the
stores using the app. One occasional issue is, there are some stores with a few millions products that sometimes do bulk update
and thus generate that same few millions of webhook events under the products/update topic, which (1) is a spike load of requests at a scale the downstream backend doesn't suit to handle and (2) overloads the jobs queue of the downstream backend way past the throughput it could reach.

This project is originally developed to solve the two problems above by (1) a performant, high-throughput and scalable
*`request-receiver`* service to efficiently receive webhook calls and enqueue those requests and (2) a *`downstreamer`*
worker gradually deques those requests and adaptively push them to the downstream backend in the same order they were received, at
a rate that wouldn't overload the downstream.

An alternative and more familiar approach would be push those requests to a backpressure queue with Nats or Kafka and have the downstream jobs workers proactively pull them at their own pace instead, but that requires modification to the existing webhook handling logic and complicates deployment topology, which would definitely be worth it when the system grew past a certain scale, but right now it asks for more problems than it solves. When we eventually need such a backpressure queue, a performant `request-receiver` service is still needed to quickly handle the spike load *and* to put more guarantee on the order of webhook events the downstream receives, and we can simply swap the `downstreamer` with an equivalent worker that instead acts as a producer for the backpressure queue. But the current design should be sufficient for now.

This design could also be adapted into a more generic congestion control system at the application layer and thus can be applied
to usecases beyond its original purpose.


 * Update: `kafka_producer` added. This background worker regularly pulls webhook events stored in redis and sends to a kafka cluster
   under a single topic (configured by the env variable `KAFKA_TOPIC`). There's no explicit partition config at the moment, but
   each webhook event is sent to kafka with its shopify webhook's topic as the key, so partition config in the future should also
   enqueue message with the same shopify webhook's topic under the same partition to respect that.
   `kafka_producer` uses the same lockfile as `downstreamer`, since the intention is they are mutually exclusive and only one
   should be running at the same time.
 * Messages sent to Kafka are in the same format as the following example. The gist of this is, each message is the whole HTTP
   request shopify sent to the webhook endpoint, and the `payload` field is the body of that request:
```json
{
  "endpoint": "/webhook/products/update",
  "method": "POST",
  "headers": {
    "host": "rnrcw-222-254-3-213.a.free.pinggy.link",
    "user-agent": "Shopify-Captain-Hook",
    "content-length": "4388",
    "accept": "*/*",
    "accept-encoding": "gzip;q=1.0,deflate;q=0.6,identity;q=0.3",
    "content-type": "application/json",
    "x-shopify-api-version": "2024-01",
    "x-shopify-hmac-sha256": "IYSq0SDPgr4Qu3JfMWl2vctQ5ELGrLoJcBTfZxahzEs=",
    "x-shopify-product-id": "9079211262258",
    "x-shopify-shop-domain": "feeder8.myshopify.com",
    "x-shopify-topic": "products/update",
    "x-shopify-triggered-at": "2024-05-06T10:45:17.912552336Z",
    "x-shopify-webhook-id": "12815508-f11e-41d0-af97-dba11f47b294"
  },
  "queries": {},
  "payload": "{\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/Product\\/9079211262258\",\"body_html\":null,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"handle\":\"selling-plans-ski-wax\",\"id\":9079211262258,\"product_type\":\"\",\"published_at\":\"2024-03-05T14:52:06+07:00\",\"template_suffix\":null,\"title\":\"Selling Plans Ski Waxx\",\"updated_at\":\"2024-05-06T17:45:18+07:00\",\"vendor\":\"feeder8\",\"status\":\"active\",\"published_scope\":\"web\",\"tags\":\"Accessory, Sport, Winter\",\"variants\":[{\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductVariant\\/47932212183346\",\"barcode\":null,\"compare_at_price\":null,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"fulfillment_service\":\"manual\",\"id\":47932212183346,\"inventory_management\":\"shopify\",\"inventory_policy\":\"deny\",\"position\":1,\"price\":\"25\",\"product_id\":9079211262258,\"sku\":\"\",\"taxable\":true,\"title\":\"Selling Plans Ski Wax\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"option1\":\"Selling Plans Ski Wax\",\"option2\":null,\"option3\":null,\"grams\":57,\"image_id\":44574175396146,\"weight\":2.0,\"weight_unit\":\"oz\",\"inventory_item_id\":49985193574706,\"inventory_quantity\":10,\"old_inventory_quantity\":10,\"requires_shipping\":true},{\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductVariant\\/47932212216114\",\"barcode\":null,\"compare_at_price\":null,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"fulfillment_service\":\"manual\",\"id\":47932212216114,\"inventory_management\":\"shopify\",\"inventory_policy\":\"deny\",\"position\":2,\"price\":\"50\",\"product_id\":9079211262258,\"sku\":\"\",\"taxable\":true,\"title\":\"Special Selling Plans Ski Wax\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"option1\":\"Special Selling Plans Ski Wax\",\"option2\":null,\"option3\":null,\"grams\":71,\"image_id\":44574175428914,\"weight\":2.5,\"weight_unit\":\"oz\",\"inventory_item_id\":49985193607474,\"inventory_quantity\":10,\"old_inventory_quantity\":10,\"requires_shipping\":true},{\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductVariant\\/47932212248882\",\"barcode\":null,\"compare_at_price\":null,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"fulfillment_service\":\"manual\",\"id\":47932212248882,\"inventory_management\":\"shopify\",\"inventory_policy\":\"deny\",\"position\":3,\"price\":\"10\",\"product_id\":9079211262258,\"sku\":\"\",\"taxable\":true,\"title\":\"Sample Selling Plans Ski Wax\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"option1\":\"Sample Selling Plans Ski Wax\",\"option2\":null,\"option3\":null,\"grams\":14,\"image_id\":44574175494450,\"weight\":0.5,\"weight_unit\":\"oz\",\"inventory_item_id\":49985193640242,\"inventory_quantity\":10,\"old_inventory_quantity\":10,\"requires_shipping\":true}],\"options\":[{\"name\":\"Title\",\"id\":11431955824946,\"product_id\":9079211262258,\"position\":1,\"values\":[\"Selling Plans Ski Wax\",\"Special Selling Plans Ski Wax\",\"Sample Selling Plans Ski Wax\"]}],\"images\":[{\"id\":44574175396146,\"product_id\":9079211262258,\"position\":1,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"alt\":\"A bar of golden yellow wax\",\"width\":2881,\"height\":2881,\"src\":\"https:\\/\\/cdn.shopify.com\\/s\\/files\\/1\\/0864\\/9808\\/3122\\/products\\/snowboard_wax.png?v=1709625126\",\"variant_ids\":[47932212183346],\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductImage\\/44574175396146\"},{\"id\":44574175428914,\"product_id\":9079211262258,\"position\":2,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"alt\":\"A bar of purple wax\",\"width\":2881,\"height\":2881,\"src\":\"https:\\/\\/cdn.shopify.com\\/s\\/files\\/1\\/0864\\/9808\\/3122\\/products\\/wax-special.png?v=1709625126\",\"variant_ids\":[47932212216114],\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductImage\\/44574175428914\"},{\"id\":44574175494450,\"product_id\":9079211262258,\"position\":3,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"alt\":\"a small cube of wax\",\"width\":2881,\"height\":2881,\"src\":\"https:\\/\\/cdn.shopify.com\\/s\\/files\\/1\\/0864\\/9808\\/3122\\/products\\/sample-normal-wax.png?v=1709625126\",\"variant_ids\":[47932212248882],\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductImage\\/44574175494450\"}],\"image\":{\"id\":44574175396146,\"product_id\":9079211262258,\"position\":1,\"created_at\":\"2024-03-05T14:52:06+07:00\",\"updated_at\":\"2024-03-05T14:52:06+07:00\",\"alt\":\"A bar of golden yellow wax\",\"width\":2881,\"height\":2881,\"src\":\"https:\\/\\/cdn.shopify.com\\/s\\/files\\/1\\/0864\\/9808\\/3122\\/products\\/snowboard_wax.png?v=1709625126\",\"variant_ids\":[47932212183346],\"admin_graphql_api_id\":\"gid:\\/\\/shopify\\/ProductImage\\/44574175396146\"},\"variant_ids\":[{\"id\":47932212183346},{\"id\":47932212216114},{\"id\":47932212248882}]}"
}
```

# Build
This project is written using the Rust 1.79.0 nightly, so at least you'll need rustc and cargo at least at that version to build the
project. If your OS' package manager didn't have the right version, you can either use
[rustup](!https://rust-lang.github.io/rustup/) (analoguous to nvm in Javascript
world) to install and manage the toolchain, or use `nix` (the actual way this project was developed) to spin up a development
shell with the toolchain provided with the exact same version of the author's.
## Using rustup
1. Install rustup: https://rust-lang.github.io/rustup/installation/index.html
2. Install the nightly toolchain: `rustup toolchain install nightly-2024-04-11`
3. Build the project: `cargo build --release`
## Using nix
1. Install nix: https://nixos.org/manual/nix/stable/installation/installing-binary
2. Edit nix config (~/.config/nix/nix.conf) to enable flakes:
```
experimental-features = nix-command flakes
```
3. Spin up the development shell: `nix develop`
4. Build the project: `cargo build --release`

Either ways, the build products are at `./target/release/request-receiver` and `./target/release/downstreamer`.

# Operation
The only required runtime dependency is Redis (or another drop-in replacement like Valkey or Keydb). For better performance it's
recommended to deploy the redis instance on the same host, and connect to it using unix socket. An example configuration for that
can be found at `./ops/redis.conf`.

You can use supervisord to start and manage `request-receiver` and `downstreamer` using the config at `./ops/supervisor.conf`. Or
you can manually execute them yourself. No containerization yet since deployment is still pretty simple. Both redis and supervisor
are available as parts of the nix development shell. Operator can edit the config to use `kafka_producer` instead of
`downstreamer`, but only of of those two should be used at the same time.

Multiple `request-receiver` instances can run at the same time since the sockets use `SO_REUSEPORT`. Multiple running instances
can increase throughput if the server is configured properly (cpu load balancing from the rps_cpu parameter of NIC's rx queue, cpu
affinity of each replica `request-receiver` process), which could help to scale up/down on demand. The `numprocs` config for
supervisor can be used for this, which should not exceed the number of cpu cores. This is hopefully never necessary since current
`tokio` runtime is already supremely efficient at async network IO and the throughput bottleneck would probably be somewhere else
and not the `request-receiver` itself.

There should only be a single `downstreamer`/`kafka_producer` running at a time, which is safeguarded by a lockfile, to make sure
the order of requests pushed to downstream/kafka is preserved, and there's no point to increase concurrency here anyway, and for
`downstreamer` we even intend to *slow* the traffic rate down.

Both `request-receiver` and `downstreamer`/`kafka_producer` try to gracefully shutdown when they receive SIGTERM. Sometimes you'd
want either of them to terminate immediately instead, which requires SIGKILL. Even with ungraceful shutdown like that it's still
improbable to leave anything in inconsistent state thanks to how Redis works.

The executables source their config from environment variables as defined in `.env.example`, if there's an `.env` file as you'd
expect they would populate their processes' with the environment variables defined there in the familiar `dotenv` manner. Some
important variables:
* `DOWNSTREAM_APP_URL`: base url of the downstream backend that the worker would push request to
* `SHOPIFY_CLIENT_SECRET`: shopify application's client secret, used to verify the webhook request's hmac signature
* `BASE_DELAY_MS`: the base delay between downstream pushes in milliseconds, used for rate control. `downstreamer` would adapt the
  actual delay with this as a base, and with recent push latencies as an estimate of the downstream's load status.
* `WORKER_REST`: `downstreamer` and `kafka_producer`'s rest between Redis work queue check during idle periods, in second.
* `WORKER_BATCH`: the number of requests the worker pulls from the redis work queue.
* `DOWNSTREAM_LOCKFILE`: path to `downstreamer` and `kafka_producer`'s lockfile. Necessary to make sure there's only one instance of either `downstreamer` or `kafka_producer` worker process running at a given time.
* `REDIS_URL`: Redis connection url
* `KAFKA_URL`: Kafka cluster url
* `KAFKA_TOPIC`: Kafka topic to send webhook events to
* `KAFKA_TX_ID`: Kafka producer's transactional.id to enforce Kafka's transactional guarantee across different runs of the
  producer. Avoid changing this value too often after set, if you must change it, ensure that there's no message in Kafka before
  and during the period the change is being made.

Rust's specific variables to control logging and backtrace:
* `RUST_LOG`: log level, `trace` < `debug` < `info` < `warn` < `error`. Restrict to level equal and above of current value.
* `RUST_BACKTRACE`: 1 = enable stack trace of application error.
* `RUST_LIB_BACKTRACE`: 1 = enable stack trace of library error.

The system's gateway (i.e. probably the internet-facing nginx server) should be configured to route the webhook path that could be
under heavy load (probably just `/webhook/products/update` for now) to the server hosting this, and the downstreamer worker would
gradually push received webhook event to the old endpoint at a rate it can handle. The exact steps to do this are up to the
operator/administrator, since it depends on how the current system is deployed.
