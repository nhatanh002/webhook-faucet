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
The only required runtime dependency needed is Redis (or another dropin replacement like Valkey or Keydb). It's recommended to deploy the redis instance on the same host, and connect to it using unix socket. An example configuration for that can be found at
`./ops/redis.conf`.

You can use supervisord to start and manage `request-receiver` and `downstreamer` using the config at `./ops/supervisor.conf`. Or you
can manually execute them yourself. No containerization yet since deployment is still pretty simple. Both redis and supervisor are
available as a part of the nix development shell.

There can be multiple `request-receiver` instances running at the same time, since the sockets use `SO_PORTREUSE` multiple running
instances can increase throughput, which could help to scale up/down on demand. The `numprocs` config for supervisor can be used
for this. Keep that number below the number of cpu cores. Operator can further tune performance with cpu load balancing and cpu
affinity, but those are probably not necessary. There should only be a single `downstreamer` running at a time, which is
safeguarded by a pid file, to make sure the order of requests pushed to downstream is preserved, and there's no point to increase
concurrency here anyway: we want to *slow* the traffic rate down.

Both `request-receiver` and `downstreamer` try to gracefully shutdown when they receive SIGTERM. Sometimes you'd want either of
them to terminate immediately instead, which requires SIGKILL. Even with ungraceful shutdown like that it's still improbable to
leave anything in inconsistent state thanks to how Redis works.

Both executables source their config from environment variables as defined in `.env.example`, if there's an `.env` file as you'd
expect they would populate their processes' with the environment variables defined there in the familiar `dotenv` manner. Some
important variables:
* `DOWNSTREAM_APP_URL`: base url of the downstream backend that the worker would push request to
* `SHOPIFY_CLIENT_SECRET`: shopify application's client secret, used to verify the webhook request's hmac signature
* `BASE_DELAY_MS`: the base delay between downstream pushes in milliseconds, used for rate control. `downstreamer` would adapt the
  actual delay with this as a base, and with recent push latencies as an estimate of the downstream's load status.
* `WORKER_REST`: `downstreamer`'s rest between Redis work queue check during idle periods, in second.
* `WORKER_BATCH`: the number of requests the worker pulls from the redis work queue.
* `DOWNSTREAM_LOCKFILE`: path to `downstreamer`'s pid file. Necessary to make sure there's only one `downstreamer` worker process running at
  a given time.

Rust's specific variables to control logging and backtrace:
* `RUST_LOG`: log level, `trace` < `debug` < `info` < `warn` < `error`. Restrict to level equal and above of current value.
* `RUST_BACKTRACE`: 1 = enable stack trace of application error.
* `RUST_LIB_BACKTRACE`: 1 = enable stack trace of library error.

The system's gateway (i.e. probably the internet-facing nginx server) should be configured to route the webhook path that could be
under heavy load (probably just `/webhook/products/update` for now) to the server hosting this, and the downstreamer worker would
gradually push received webhook event to the old endpoint at a rate it can handle. The exact steps to do this are up to the
operator/administrator, since it depends on how the current system is deployed.
