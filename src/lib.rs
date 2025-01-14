//! # Leucite
//!
//! A wrapper crate around [`rust-landlock`](https://docs.rs/landlock) that provides useful
//! abstractions and utilities
//!
//! ## Example
//!
//! ```no_run
//! # use leucite::{Rules, CommandExt, MemorySize};
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
//!     .restrict(rules)
//!     .max_memory(MemorySize::from_mb(100))
//!     .spawn()?
//!     .wait()?;
//! # std::io::Result::Ok(())
//! ```
use anyhow::Context;
use landlock::{
    path_beneath_rules, Access, AccessFs, AccessNet, NetPort, Ruleset, RulesetAttr,
    RulesetCreatedAttr, RulesetStatus, ABI,
};
use prlimit::Limit;
use std::{io, os::unix::process::CommandExt as _, path::PathBuf, process::Command};
#[cfg(feature = "tokio")]
use tokio::process::Command as TokioCommand;

mod prlimit;
pub use prlimit::MemorySize;

#[cfg(not(target_os = "linux"))]
compile_error!("`leucite` must be run on linux.");

/// Struct which holds the rules for restrictions.  For more information, see [`Ruleset`].
///
/// Example
/// ```
/// # use leucite::Rules;
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
    pub fn restrict(&self) -> anyhow::Result<()> {
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

fn anyhow_to_io<T>(res: anyhow::Result<T>) -> io::Result<T> {
    res.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Extension for [`Command`] or [`tokio::process::Command`] that restricts a command once it is
/// spawned to be limited in its environment
pub trait CommandExt {
    /// Restrict the filesystem access for this command based on the provided rules
    fn restrict(&mut self, rules: Rules) -> &mut Self;

    /// Restrict the maxmimum memory usage for the command
    ///
    /// See [`getrlimit(2)`](https://www.man7.org/linux/man-pages/man2/prlimit.2.html) and `RLIMIT_DATA`
    fn max_memory(&mut self, max_memory: MemorySize) -> &mut Self;

    /// Restrict the maximum file size that the command may create
    ///
    /// See [`getrlimit(2)`](https://www.man7.org/linux/man-pages/man2/prlimit.2.html) and `RLIMIT_FSIZE`
    fn max_file_size(&mut self, max_file_size: MemorySize) -> &mut Self;
}

// This is okay since all of the functions have idential implementations for both StdCommand and
// TokioCommand, if that ever changes, this will need to change.
macro_rules! impl_cmd {
    ($($t: tt)+) => {
        impl CommandExt for Command {
            $($t)+
        }

        #[cfg(feature = "tokio")]
        impl CommandExt for TokioCommand {
            $($t)+
        }
    }
}

impl_cmd! {
    fn restrict(&mut self, rules: Rules) -> &mut Self {
        unsafe {
            self.pre_exec(move || anyhow_to_io(rules.restrict().context("creating restrictions")))
        }
    }

    fn max_memory(&mut self, max_memory: MemorySize) -> &mut Self {
        unsafe {
            self.pre_exec(move || Limit::Data.limit(*max_memory))
        }
    }

    fn max_file_size(&mut self, max_file_size: MemorySize) -> &mut Self {
        unsafe {
            self.pre_exec(move || Limit::FileSize.limit(*max_file_size))
        }
    }
}
