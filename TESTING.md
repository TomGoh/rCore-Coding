# Testing Guide

## Test Mode

The OS kernel supports a special test mode for running unit tests in CI without hanging.

### Running Tests

To run the kernel in test mode:

```bash
cd os
make run-test
```

This will:
1. Build the kernel with the `test-mode` feature flag
2. Run the following tests:
   - `heap_test` - Tests heap allocation functionality
   - `frame_allocator_test` - Tests frame allocator
   - `remap_test` - Tests memory remapping
3. Cleanly shutdown via SBI after all tests pass

### Normal Mode

To run the kernel in normal mode (for interactive use):

```bash
cd os
make run
```

This will run the full kernel with task scheduling and user programs.

### How It Works

The test mode uses conditional compilation with Rust's feature flags:

- **Cargo.toml**: Defines the `test-mode` feature
- **main.rs**: Uses `#[cfg(feature = "test-mode")]` to compile different code paths
- **Makefile**: Provides `run-test` target that builds with `--features test-mode`

When `test-mode` is enabled:
- Runs specific test functions after initialization
- Calls `sbi::shutdown(false)` to exit cleanly
- Skips task scheduling and user program execution

When `test-mode` is disabled (default):
- Runs the full kernel with task scheduling
- Loads and executes user programs
- Never exits (runs until manually stopped)

### CI Integration

The GitHub Actions workflow uses `make run-test` to verify the kernel builds and runs correctly without hanging the CI pipeline.
