# dil

[![Crates.io](https://img.shields.io/crates/v/dil.svg)](https://crates.io/crates/dil)
[![Documentation](https://docs.rs/dil/badge.svg)](https://docs.rs/dil)

`dil` is a lightweight dependency-injection library for Rust. Register services with an injector, resolve them by type, and let constructors receive their dependencies automatically.

## Getting started

Add `dil` with its procedural macros enabled:

```toml
[dependencies]
dil = { version = "2", features = ["macros"] }
```

Define a service, register it, and resolve it:

```rust
use dil::{service, Injector};
use dil::sync::Mutex;

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
    logger.lock().unwrap().log("Hello from dil!");
}
```

The `#[derive(Service)]` macro can build services from their fields. Fields whose types are registered with the injector are resolved automatically:

```rust
use dil::{Injector, Service};
use dil::sync::Mutex;

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
use dil::{dynamic_service, service, Injector};
use dil::sync::Mutex;

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

By default, `dil` uses thread-safe `Mutex` and `RwLock` implementations. `RefCell` is also available for single-threaded applications.

## Documentation

- [API documentation on docs.rs](https://docs.rs/dil)
- [Package on crates.io](https://crates.io/crates/dil)

## License

Licensed under the [MIT License](LICENSE).