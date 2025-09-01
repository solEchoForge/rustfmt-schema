# rustfmt-schema

A Rust module for managing rustfmt-schema.

## Features

- Read variables from various file formats 
- Send data to backend servers via HTTP POST
- Optional authentication support
- Configurable timeouts
- Async/await support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rustfmt-schema = "0.1.0"
```

## API Reference

## CLI Usage

```bash
# Send a single .rustfmt file
rustfmt-schema send .rustfmt

# Send multiple files
rustfmt-schema send-multiple .rustfmt .rustfmt.local

# Test backend connection
rustfmt-schema test
```

## Testing

```bash
cargo test
```
