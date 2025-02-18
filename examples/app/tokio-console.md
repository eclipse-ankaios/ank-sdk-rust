# Tokio Console

The Console is used for debugging Rust multi-threaded applications that use tokio.

To use it, add the following dependency in `Cargo.toml` file:
```toml
console-subscriber = "0.4.1"
```
And also, `tokio` should contain at least the following features: `full`, `tracing`.

To enable the console, add this line in the code:
```rs
console_subscriber::init();
```

For build, the application needs to be built with `tokio_unstable` flag. For this, you can either provide it in the build command:
```sh
$ RUSTFLAGS="--cfg tokio_unstable" cargo build
```
or directly in the `Cargo.toml` file:
```toml
[build]
rustflags = ["--cfg", "tokio_unstable"]
```

After starting the application, while it is still running, install the tokio console (if it's not installed):
```sh
cargo install --locked tokio-console
```

And then start it:
```
tokio-console
```

For more information, check the following page:
* https://github.com/tokio-rs/console