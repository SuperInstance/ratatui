# Contributing to ratatui-guardian

Thanks for your interest! Here's how to contribute.

## Development

```bash
# Build
cargo build

# Run all tests
cargo test

# Check formatting
cargo fmt --all -- --check

# Run linter
cargo clippy --all-targets -- -D warnings

# Run examples
cargo run --example minimal
cargo run --example full_demo
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch from `guardian`
3. Add tests for any new functionality
4. Ensure `cargo test`, `cargo fmt --check`, and `cargo clippy` all pass
5. Update CHANGELOG.md with your changes
6. Submit the PR

## Code Style

- Follow standard Rust conventions (`cargo fmt`)
- Every public method gets a doc comment with a short example
- Error paths return `Result<_, GuardianError>`, never panic
- Tests cover happy path, edge cases, and error cases

## Reporting Issues

- Include Rust version (`rustc --version`)
- Include a minimal reproducible example
- Include the output of `cargo test` if relevant

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
