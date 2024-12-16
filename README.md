# Sandbox

This is a wrapper crate around [`rust-landlock`](https://docs.rs/landlock)
that provides some abstractions and utilities for running commands.

## Example

```rust
let rules = Rules::new()
    .add_read_only("/usr")
    .add_read_only("/etc")
    .add_read_only("/dev")
    .add_read_only("/bin")
    .add_read_write("/tmp/foo");

let mut child = Command::new("bash")
    .arg("-i")
    .current_dir("/tmp/foo")
    .env_clear()
    .spawn_restricted(rules)?;

child.wait()?;
```
