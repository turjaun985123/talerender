//! GPUFIRST Batch 3, seat FILMPATH — the ONE backend-selection
//! contract for character painting.
//!
//! THE CONTRACT (single source of truth; the telemetry seat and
//! any future caller consume THIS helper, never their own env
//! read):
//!
//!   * The film path paints characters with the GPU painter
//!     (`crate::gpupaint`) if and only if BOTH hold:
//!       1. the binary was built `--features gpupaint`, AND
//!       2. the environment variable `TALERENDER_GPUPAINT` is
//!          exactly `"1"` at selection time.
//!   * In every other configuration — default build (no
//!     feature), feature build with the env unset, or the env
//!     set to anything other than `"1"` (`"0"`, `"true"`, `""`,
//!     …) — the CPU painter (`toonfull::paint_toon_full`) is
//!     used, byte-for-byte as before Batch 3.
//!   * No default is flipped by this module: the feature stays
//!     off by default in Cargo.toml and the env stays opt-in.
//!
//! Selection timing: `paint_backend()` is a pure read of the
//! process env + the compile-time feature. `film::PaintCache`
//! calls it ONCE per cache construction (`new_for_film`) and
//! pins the result for the cache's lifetime, so a film run can
//! never switch painters mid-film if the env were mutated.
//! `PaintCache::new` never selects the GPU backend. The
//! telemetry leg has its own constructor, `new_for_telemetry`
//! (Batch 3, seat TELEM), which DOES consult this contract:
//! under the flag, check/telemetry paint comes from gpupaint
//! with verdict semantics unchanged — the user's 2026-11-18
//! scope note supersedes the W84 ruling D1 "telemetry stays
//! CPU-only" posture for the flagged case. CPU remains the
//! telemetry default in every unflagged configuration.
//!
//! This module is NOT feature-gated, so callers in any build
//! can ask `paint_backend()` / `gpupaint_enabled()` without a
//! `#[cfg]` at the call site; without the feature the answer
//! is always `PaintBackend::Cpu`.

/// Which painter the film path uses for character cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaintBackend {
    /// `toonfull::paint_toon_full` — the production painter.
    Cpu,
    /// `gpupaint::paint_state_gpu` — trace + GPU replay of the
    /// same paint (feature `gpupaint` builds only).
    GpuPaint,
}

/// Pure selection rule, split out for unit tests (no env or
/// cfg access): GPU only when the feature is compiled in AND
/// the env value is exactly `"1"`.
fn backend_for(feature_compiled: bool, env_value: Option<&str>) -> PaintBackend {
    if feature_compiled && env_value == Some("1") {
        PaintBackend::GpuPaint
    } else {
        PaintBackend::Cpu
    }
}

/// THE selection helper. Returns the backend for this process
/// per the contract in the module header.
pub(crate) fn paint_backend() -> PaintBackend {
    backend_for(
        cfg!(feature = "gpupaint"),
        std::env::var("TALERENDER_GPUPAINT").ok().as_deref(),
    )
}

/// Convenience predicate over `paint_backend()` — same
/// contract, boolean shape (also re-exported as
/// `gpupaint::enabled()` in feature builds for callers that
/// think in the gpupaint module's namespace). Kept even when
/// no in-tree caller uses it yet: it is the telemetry seat's
/// contracted entry point (Batch 3 coordination).
#[allow(dead_code)]
pub(crate) fn gpupaint_enabled() -> bool {
    paint_backend() == PaintBackend::GpuPaint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_requires_feature_and_exact_env_1() {
        assert_eq!(backend_for(true, Some("1")), PaintBackend::GpuPaint);
        // Feature on, env missing or not exactly "1" -> CPU.
        assert_eq!(backend_for(true, None), PaintBackend::Cpu);
        assert_eq!(backend_for(true, Some("0")), PaintBackend::Cpu);
        assert_eq!(backend_for(true, Some("true")), PaintBackend::Cpu);
        assert_eq!(backend_for(true, Some("")), PaintBackend::Cpu);
        assert_eq!(backend_for(true, Some(" 1")), PaintBackend::Cpu);
        // Env set but feature NOT compiled in -> CPU, always:
        // a default build ignores TALERENDER_GPUPAINT entirely.
        assert_eq!(backend_for(false, Some("1")), PaintBackend::Cpu);
        assert_eq!(backend_for(false, None), PaintBackend::Cpu);
    }

    #[test]
    fn default_build_never_selects_gpu() {
        // In a build without the feature (the `cargo test`
        // default), the live helper must answer CPU whatever
        // the ambient env says — this test does not mutate the
        // env, it only pins the cfg half of the contract.
        #[cfg(not(feature = "gpupaint"))]
        assert_eq!(paint_backend(), PaintBackend::Cpu);
    }
}
