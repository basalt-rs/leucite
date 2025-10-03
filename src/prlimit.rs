use std::{io, ptr};

use libc::rlimit;

/// Representation of some amount of digital space
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MemorySize(u64);

macro_rules! impl_memsz {
    ($from_fn: ident => $param: ident * $expr: expr) => {
        #[inline]
        pub fn $from_fn($param: u64) -> Self {
            Self($param * $expr)
        }

        #[inline]
        pub fn $param(self) -> u64 {
            self.0 / $expr
        }
    };
}

impl MemorySize {
    impl_memsz!(from_bytes => bytes * 1);

    impl_memsz!(from_kb => kilobytes * 1000);
    impl_memsz!(from_mb => megabytes * 1000 * 1000);
    impl_memsz!(from_gb => gigabytes * 1000 * 1000 * 1000);

    impl_memsz!(from_kib => kibibytes * 1024);
    impl_memsz!(from_mib => mebibytes * 1024 * 1024);
    impl_memsz!(from_gib => gibibytes * 1024 * 1024 * 1024);
}

pub(crate) fn read_errno() -> io::Error {
    io::Error::last_os_error()
}

fn into_rlimit(n: u64) -> libc::rlimit {
    rlimit {
        rlim_cur: n,
        rlim_max: n,
    }
}

#[repr(u32)]
#[allow(unused)]
pub(crate) enum Limit {
    Cpu = libc::RLIMIT_CPU,
    FileSize = libc::RLIMIT_FSIZE,
    Data = libc::RLIMIT_DATA,
    Stack = libc::RLIMIT_STACK,
    Core = libc::RLIMIT_CORE,
    ResidentSetSize = libc::RLIMIT_RSS,
    NumberProcesses = libc::RLIMIT_NPROC,
    NumberFiles = libc::RLIMIT_NOFILE,
    MemoryLock = libc::RLIMIT_MEMLOCK,
    AddressSpace = libc::RLIMIT_AS,
    Locks = libc::RLIMIT_LOCKS,
    SignalPending = libc::RLIMIT_SIGPENDING,
    MessageQueue = libc::RLIMIT_MSGQUEUE,
    Nice = libc::RLIMIT_NICE,
    RTPrio = libc::RLIMIT_RTPRIO,
    RTTime = libc::RLIMIT_RTTIME,
}

impl From<Limit> for libc::__rlimit_resource_t {
    fn from(value: Limit) -> Self {
        value as _
    }
}

impl Limit {
    /// # SAFETY
    ///
    /// Caller must ensure that any resources that are already allocated by the process fall below
    /// the limit set by this call.
    pub(crate) unsafe fn limit(self, size: u64) -> io::Result<()> {
        let limit = into_rlimit(size);
        // SAFETY: this function should never crash based on input.  Any error is returned through
        // `errno` and we are handling that properly.
        let ret = unsafe {
            libc::prlimit(
                0,
                libc::__rlimit_resource_t::from(self),
                &limit as *const rlimit,
                ptr::null_mut(),
            )
        };

        if ret == 0 {
            return Ok(());
        }

        Err(read_errno())
    }
}
