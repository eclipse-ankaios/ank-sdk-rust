# Architecture Notes

Non-obvious facts about the SDK's internals — useful when navigating or modifying core code.

- The main API is the `Ankaios` struct in `src/ankaios.rs`. All methods are async and return `Result<_, AnkaiosError>`.
- All error types are variants of `AnkaiosError` in `src/errors.rs`.
- Communication uses Unix FIFO pipes at `/run/ankaios/control_interface/{input,output}` (length-prefixed protobuf).
- For the pipes to be mounted, the workload manifest must declare `controlInterfaceAccess.allowRules` (add a `LogRule` entry when using `request_logs`).
- `src/ankaios_api/` contains generated protobuf bindings — do not edit directly.
- Use `mockall` / `mockall_double` for mocking in tests; see `src/components/control_interface.rs` for the established pattern.
- Test utilities are gated behind the `test_utils` feature flag.
- SDK version mirrors the Ankaios version it targets (e.g., `1.0.x` ↔ Ankaios `1.0.x`). To update versions: `tools/update_version.sh --help`.
