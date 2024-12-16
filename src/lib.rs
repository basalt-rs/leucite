//! # Sandbox
//!
//! A wrapper crate around [`rust-landlock`](https://docs.rs/landlock) that provides useful
//! abstractions and utilities
//!
//! ## Example
//!
//! ```no_run
//! # use sandbox::{Rules, CommandExt};
//! # use std::process::Command;
//! let rules = Rules::new()
//!     .add_read_only("/usr")
//!     .add_read_only("/etc")
//!     .add_read_only("/dev")
//!     .add_read_only("/bin")
//!     .add_read_write("/tmp/foo");
//!
//! // Execute `bash -i` in the `/tmp/foo` directory using the provided rules
//! Command::new("bash")
//!     .arg("-i")
//!     .current_dir("/tmp/foo")
//!     .env_clear()
//!     .spawn_restricted(rules)?
//!     .wait()?;
//! # std::io::Result::Ok(())
//! ```
use anyhow::Context;
use landlock::{
    path_beneath_rules, Access, AccessFs, AccessNet, NetPort, Ruleset, RulesetAttr,
    RulesetCreatedAttr, RulesetStatus, ABI,
};
use std::{
    io,
    os::unix::process::CommandExt as _,
    path::PathBuf,
    process::{Child, Command},
};

#[cfg(not(target_os = "linux"))]
compile_error!("`sandbox` must be run on linux.");

/// Struct which holds the rules for restrictions.  For more information, see [`Ruleset`].
///
/// Example
/// ```
/// # use sandbox::Rules;
/// let rules = Rules::new()
///     .add_read_only("/usr")
///     .add_read_only("/etc")
///     .add_read_only("/dev")
///     .add_read_only("/bin")
///     .add_read_write("/tmp/foo")
///     .add_bind_port(5050)
///     .add_connect_port(80)
///     .add_connect_port(443);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Rules {
    read_only: Vec<PathBuf>,
    read_write: Vec<PathBuf>,
    write_only: Vec<PathBuf>,
    bind_ports: Vec<u16>,
    connect_ports: Vec<u16>,
}

impl Rules {
    /// Create a new [`Rules`] with no permissions
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a read-only path to the rules
    pub fn add_read_only(mut self, p: impl Into<PathBuf>) -> Self {
        self.read_only.push(p.into());
        self
    }

    /// Add a read/write path to the rules
    pub fn add_read_write(mut self, p: impl Into<PathBuf>) -> Self {
        self.read_write.push(p.into());
        self
    }

    /// Add a write-only path to the rules
    pub fn add_write_only(mut self, p: impl Into<PathBuf>) -> Self {
        self.write_only.push(p.into());
        self
    }

    /// Add a port to which the command can connect port to the rules
    pub fn add_connect_port(mut self, p: u16) -> Self {
        self.connect_ports.push(p);
        self
    }

    /// Add a port to which the command can bind to the rules
    pub fn add_bind_port(mut self, p: u16) -> Self {
        self.bind_ports.push(p);
        self
    }

    /// Restrict the current thread using these rules
    fn restrict(&self) -> anyhow::Result<()> {
        let abi = ABI::V4;
        let rules = Ruleset::default()
            .handle_access(AccessFs::from_all(abi))
            .context("setting fs access")?
            .handle_access(AccessNet::from_all(abi))
            .context("setting net access")?
            .create()
            .context("creating ruleset")?;

        let rules = if self.bind_ports.is_empty() {
            rules.add_rule(NetPort::new(0, AccessNet::BindTcp))
        } else {
            rules.add_rules(
                self.bind_ports
                    .iter()
                    .map(|p| Ok(NetPort::new(*p, AccessNet::BindTcp))),
            )
        }
        .context("setting bind ports")?;

        let rules = if self.connect_ports.is_empty() {
            rules.add_rule(NetPort::new(0, AccessNet::ConnectTcp))
        } else {
            rules.add_rules(
                self.connect_ports
                    .iter()
                    .map(|p| Ok(NetPort::new(*p, AccessNet::ConnectTcp))),
            )
        }
        .context("setting connect ports")?;

        let status = rules
            .add_rules(path_beneath_rules(
                &self.read_only,
                AccessFs::from_read(abi),
            ))
            .context("setting RO paths")?
            .add_rules(path_beneath_rules(
                &self.write_only,
                AccessFs::from_write(abi),
            ))
            .context("setting WO paths")?
            .add_rules(path_beneath_rules(
                &self.read_write,
                AccessFs::from_all(abi),
            ))
            .context("setting RW paths")?
            .restrict_self()
            .context("creating restrictions")?;

        if let RulesetStatus::NotEnforced = status.ruleset {
            anyhow::bail!("Installed kernel does not support landlock.")
        }
        Ok(())
    }
}

/// Command extensions for `tokio`'s `Command` type
#[cfg(feature = "tokio")]
pub mod tokio {
    use anyhow::Context;
    use tokio::{
        io,
        process::{Child, Command},
    };

    use crate::Rules;

    /// Extension for `tokio`'s [`Command`] that grants the ability to spawn the command in a the
    /// restricted environment
    pub trait CommandExt {
        fn spawn_restricted(&mut self, rules: Rules) -> io::Result<Child>;
    }

    impl CommandExt for Command {
        fn spawn_restricted(&mut self, rules: Rules) -> io::Result<Child> {
            unsafe {
                self.pre_exec(move || {
                    rules
                        .restrict()
                        .context("creating restrictions")
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                })
            };
            Ok(self.spawn()?)
        }
    }
}

/// Extension for std library's [`Command`] that grants the ability to spawn the command in a the
/// restricted environment
pub trait CommandExt {
    fn spawn_restricted(&mut self, rules: Rules) -> io::Result<Child>;
}

impl CommandExt for Command {
    fn spawn_restricted(&mut self, rules: Rules) -> io::Result<Child> {
        unsafe {
            self.pre_exec(move || {
                rules
                    .restrict()
                    .context("creating restrictions")
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            })
        };
        Ok(self.spawn()?)
    }
}
