//! TEMPORARY STUB — GOV-PAR standalone-CI shim, NOT the governor.
//!
//! The real module is owned by seat GOV-CORE (Batch 3 resource
//! governance) and lands separately; base e2099c4 does not contain
//! it yet. This stub exists ONLY so the GOV-PAR branch compiles and
//! its CI runs standalone, and it implements ONLY the two accessors
//! GOV-PAR consumes, with the exact contracted signatures:
//!
//!   `crate::resource::governor()` -> `&'static Governor`
//!   `Governor::thread_cap()` — env `TALERENDER_THREADS`, default =
//!       half the logical cores, min 1
//!   `Governor::enabled()`
//!
//! AT INTEGRATION THE COORDINATOR DROPS THIS FILE (and the
//! `mod resource;` line) and GOV-PAR's call sites bind to GOV-CORE's
//! real module unchanged — the same shim pattern TELEM used for
//! `paint_backend`. No other config source is invented here: the
//! stub reads the one contracted env var, and `enabled()` is
//! unconditionally `true` in the stub (the governor is present);
//! GOV-CORE owns the real enable semantics.

use std::sync::OnceLock;

pub(crate) struct Governor {
    thread_cap: usize,
}

pub(crate) fn governor() -> &'static Governor {
    static GOVERNOR: OnceLock<Governor> = OnceLock::new();
    GOVERNOR.get_or_init(|| Governor { thread_cap: stub_thread_cap() })
}

fn stub_thread_cap() -> usize {
    if let Ok(raw) = std::env::var("TALERENDER_THREADS") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            return n.max(1);
        }
    }
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    (cores / 2).max(1)
}

impl Governor {
    pub(crate) fn thread_cap(&self) -> usize {
        self.thread_cap
    }

    pub(crate) fn enabled(&self) -> bool {
        true
    }
}
