# Ankaios API

This folder is needed as a workaround for the not yet published Ankaios `api`. Using the `api` directly from Github:

```toml
[dependencies]
api = { git = "https://github.com/eclipse-ankaios/ankaios.git", tag = "v0.0.0", subdir = "api", version = "0.0.0" }
```

does not work because the `crates.io` solver looks for a crate with that name. Vendoring the `api` does not work either.

## Steps after publishing

1. Remove the `src/ankaios_api` and `proto` folders.
2. Modify `build.rs` by removing the `tonic_build::configure` part that prepares the proto files.
3. Remove the `mod ankaios_api;` from `src/lib.rs`.
4. Remove all the `use crate::ankaios_api;` lines from the sdk. This help narrow down the dependency to the crate itself.
5. Add the `ankaios_api` crate properly in the `Cargo.toml` file.