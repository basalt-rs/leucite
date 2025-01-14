<!-- Readme generated with `cargo-readme`: https://github.com/webern/cargo-readme -->

# leucite

[![Crates.io](https://img.shields.io/crates/v/leucite.svg)](https://crates.io/crates/leucite)
[![Documentation](https://docs.rs/leucite/badge.svg)](https://docs.rs/leucite/)
[![Dependency status](https://deps.rs/repo/github/basalt-rs/leucite/status.svg)](https://deps.rs/repo/github/basalt-rs/leucite)

A wrapper crate around [`rust-landlock`](https://docs.rs/landlock) that provides useful
abstractions and utilities

### Example

```rust
let rules = Rules::new()
    .add_read_only("/usr")
    .add_read_only("/etc")
    .add_read_only("/dev")
    .add_read_only("/bin")
    .add_read_write("/tmp/foo");

// Execute `bash -i` in the `/tmp/foo` directory using the provided rules
Command::new("bash")
    .arg("-i")
    .current_dir("/tmp/foo")
    .env_clear()
    .restrict(rules.into())
    .max_memory(MemorySize::from_mb(100))
    .spawn()?
    .wait()?;
```
