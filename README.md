# dif

[![Crates.io](https://img.shields.io/crates/v/dif.svg)](https://crates.io/crates/dif)
[![Documentation](https://docs.rs/dif/badge.svg)](https://docs.rs/dif)

`dif` is a lightweight dependency-injection library for Rust. Register services with an injector, resolve them by type, and let constructors receive their dependencies automatically.

## Getting started

Add `dif` with its procedural macros enabled:

```toml
[dependencies]
dif = { version = "2", features = ["macros"] }
```

Define a service, register it, and resolve it:

```rust
use dif::{service, Injector};
use dif::sync::Mutex;

struct Logger;

#[service]
impl Logger {
    pub fn new() -> Self {
        Self
    }

    pub fn log(&self, message: &str) {
        println!("{message}");
    }
}

fn main() {
    let mut injector = Injector::<Mutex>::new();
    injector.singleton::<Logger>();

    let logger = injector.get::<Logger>().expect("Logger is registered");
    logger.lock().unwrap().log("Hello from dif!");
}
```

The `#[derive(Service)]` macro can build services from their fields. Fields whose types are registered with the injector are resolved automatically:

```rust
use dif::{Injector, Service};
use dif::sync::Mutex;

#[derive(Service)]
struct App {
    logger: std::sync::Arc<std::sync::Mutex<Logger>>,
}
```

Register services as singletons with `singleton`, or create a fresh instance for each resolution with `transient`:

```rust
injector.singleton::<Logger>();
injector.transient::<App>();
```

## Dynamic services

Use `#[dynamic_service]` to register implementations behind a trait and resolve one implementation with `get` or all implementations with `get_list`:

```rust
use dif::{dynamic_service, service, Injector};
use dif::sync::Mutex;

#[dynamic_service]
trait Logger: Send + Sync + 'static {
    fn log(&mut self, message: &str);
}

struct ConsoleLogger;

#[service]
impl ConsoleLogger {
    pub fn new() -> Self {
        Self
    }
}

#[service]
impl Logger for ConsoleLogger {
    fn log(&mut self, message: &str) {
        println!("{message}");
    }
}

let mut injector = Injector::<Mutex>::new();
injector.singleton_dyn::<ConsoleLogger, dyn Logger>();
let logger = injector.get::<dyn Logger>().unwrap();
```

## Features

- `macros`: enables `#[service]`, `#[dynamic_service]`, and `#[derive(Service)]`.
- `async`: adds Tokio-based async mutex and read-write lock implementations.
- `globals`: enables global injectors for supported lock types.

By default, `dif` uses thread-safe `Mutex` and `RwLock` implementations. `RefCell` is also available for single-threaded applications.

## Documentation

- [API documentation on docs.rs](https://docs.rs/dif)
- [Package on crates.io](https://crates.io/crates/dif)

## License

Licensed under the [MIT License](LICENSE).