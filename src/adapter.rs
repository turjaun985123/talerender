/*
 * DELIVERY NOTE — TALERENDER V2, Batch 2, Seat 3: WP3 adapter
 * policies + device-loss survey/implementation design
 * (contract C7, docs/integration/V2-CONTRACTS.md §3; master
 * prompt §7; folds in Stage 1 item 3, STAGE1-SCOPE §Item 3).
 *
 * FILES CREATED (one, new files only — no existing file was
 * edited by this seat, no git was run, no dependency added):
 *   src/adapter.rs  (this file; module + in-module tests)
 *
 * CONTRACT POINTS IMPLEMENTED (C7, exactly as specified):
 *   - AdapterPolicy { Auto, HardwareOnly, SoftwareOk,
 *     Named(String) } and AdapterReport { name, backend,
 *     device_type, driver, is_software } with the exact C7
 *     field/variant names; policy_from(arg, env), describe(
 *     info), report_line(report) with the exact C7 signatures.
 *   - policy_from: arg beats env. Values (whitespace-trimmed,
 *     case-sensitive keywords — the same spellings config.rs
 *     C8 validates): "auto" | "hardware-only" | "software-ok"
 *     | "named:<name>" (name = the trimmed remainder, must be
 *     non-empty; its case is preserved — matching at select
 *     time is case-insensitive). A present-but-invalid value
 *     never selects a policy by itself: resolution falls
 *     through to the next source (the house pattern — cf.
 *     config.rs layer fall-through, mempressure's
 *     resolve_profile), and if NO source yields a valid value
 *     the result is Auto. For a single source that is exactly
 *     the charter's "invalid -> Auto".
 *   - describe(info: &wgpu::AdapterInfo) -> AdapterReport:
 *     backend / device_type are the Debug spellings the
 *     codebase already prints with {:?} ("Vulkan",
 *     "IntegratedGpu", "Cpu", ...); driver is the existing
 *     sites' "{} {}" (driver, driver_info) pair collapsed to
 *     one string — joined with a single space when both are
 *     non-empty, the non-empty one alone otherwise (so no
 *     leading/trailing space; byte-identical to the existing
 *     rendering whenever driver_info is non-empty, which is
 *     the lavapipe and RADV reality). is_software is
 *     device_type == Cpu (via matches!, C7's definition).
 *     AdapterInfo field names verified against docs.rs wgpu
 *     30.0.2 (struct.AdapterInfo): name, vendor, device,
 *     device_type, device_pci_bus_id, driver, driver_info,
 *     backend, subgroup_min_size, subgroup_max_size,
 *     transient_saves_memory, limit_bucket — all pub.
 *   - report_line(r) -> the canonical line the codebase
 *     already prints at main.rs / ab1.rs / scenefile.rs:
 *       adapter: <name> | backend: <b> | device type: <t> | driver: <d>
 *     with " (software)" appended iff r.is_software (master
 *     prompt §7: a software adapter is ALWAYS labelled
 *     software; hardware never is).
 *   - select(adapters: &[wgpu::AdapterInfo], policy:
 *     &AdapterPolicy) -> Option<usize> (seat-charter addition,
 *     not in C7's listing): PURE selection over info structs.
 *     Auto = first non-Cpu if any, else the first adapter;
 *     HardwareOnly = first non-Cpu, else None; SoftwareOk =
 *     the first adapter; Named = first whose name contains
 *     the given string, case-insensitive (substring, not
 *     prefix), else None — Named imposes NO device-type
 *     restriction (it may select a Cpu adapter; report_line
 *     still labels it software). "Non-Cpu" means any of
 *     Other / IntegratedGpu / DiscreteGpu / VirtualGpu:
 *     VirtualGpu counts as non-software because C7 defines
 *     software strictly as device_type == Cpu. Empty slice ->
 *     None under every policy. Degenerate Named("") matches
 *     the first adapter (str::contains("") is true) —
 *     policy_from can never produce it (empty name is
 *     invalid there); the behaviour is pinned by a test so
 *     it is defined, not accidental.
 *
 * WIRING THE LEAD MUST DO (this seat edited nothing else):
 *   1. src/main.rs — mod line: insert `mod adapter;`
 *      immediately after `mod ab1;`, before `mod anim;`
 *      (alphabetical slot in the mod block). That line alone
 *      makes this module and its tests compile under CI.
 *   2. Adapter REPORTING — replace the inline println! at
 *      each get_info() site with
 *          println!("{}", crate::adapter::report_line(
 *              &crate::adapter::describe(&info)));
 *      Sites (anchor: `let info = adapter.get_info();`):
 *        - src/main.rs, async fn run — the `adapter: ...`
 *          println! right after that anchor.
 *        - src/ab1.rs, pub async fn run_ab1 — same shape.
 *        - src/scenefile.rs, async fn make_device — same.
 *        - src/film.rs, Renderer::new — the site prints the
 *          same fields under a `renderer:` prefix today.
 *          DECISION FOR THE LEAD: report_line always says
 *          `adapter:`. Adopting it verbatim at film changes
 *          that one prefix (log-only unification); keeping
 *          the historic prefix means formatting the
 *          AdapterReport fields with `renderer:` at that
 *          site instead. Do not let the choice happen
 *          silently — the still/film log grammars are
 *          compared in A/B work.
 *        - src/dof.rs, pub async fn run_dof — THE Stage 1
 *          item 3 fix. Today's line is hardcoded:
 *            "adapter: {} ({:?}, driver {}) — software
 *             rasteriser, no performance claim"
 *          which mislabels any hardware adapter as software.
 *          Replace the whole println! with the report_line
 *          form above: the canonical fields appear, and the
 *          software note (" (software)") appears ONLY when
 *          is_software. On lavapipe the software claim is
 *          preserved (Cpu branch); on RADV the line becomes
 *          truthful. The "no performance claim" prose is
 *          dropped — it was part of the hardcoded claim, and
 *          the canonical line carries no such prose at any
 *          other site.
 *      NOTE on line drift: on lavapipe (Cpu) the canonical
 *      sites (main/ab1/scenefile) GAIN the " (software)"
 *      suffix versus today's bytes. That is the C7 /
 *      master-prompt-§7 mandate ("software rendering is
 *      never reported as hardware rendering" — and, by the
 *      same token, always labelled), and it supersedes
 *      STAGE1-SCOPE item 3's "lines unchanged on lavapipe"
 *      phrasing for those lanes; dof's software claim is
 *      the one that stays. Flagged here so the lead lands
 *      it consciously; it is log-only, byte probes are
 *      unaffected.
 *   3. Adapter SELECTION (policy-aware request_adapter) —
 *      gpucore.rs. Keep request_adapter's existing two-step
 *      verbatim as the Auto/no-enumeration fallback; add
 *      beside it (lead's exact factoring is free):
 *          pub(crate) async fn request_adapter_with_policy(
 *              instance: &wgpu::Instance,
 *              policy: &crate::adapter::AdapterPolicy,
 *          ) -> TaleResult<wgpu::Adapter>
 *      Body sketch: enumerate the instance's adapters over
 *      Backends::VULKAN (matching new_instance's restriction;
 *      lead verifies the enumeration call's exact 30.0.2
 *      shape at integration — select() is written over
 *      &[AdapterInfo] precisely so sync-vs-awaited
 *      enumeration cannot affect this module), map each
 *      Adapter::get_info into a Vec<AdapterInfo> IN THE SAME
 *      ORDER as the Vec<Adapter>, then
 *      adapter::select(&infos, policy):
 *        - Some(i)  -> return adapters[i]; optionally emit
 *          observability::adapter_line(...) / the
 *          adapter.selected event from describe(&infos[i]).
 *        - None + non-empty list (HardwareOnly with only
 *          Cpu adapters, or Named with no match) -> FAIL
 *          with TaleError::gpu_init (exit 3) naming the
 *          policy, e.g. source "no adapter satisfies the
 *          requested adapter policy". Do NOT reuse the
 *          existing "no wgpu adapter found — ..." verbatim
 *          string: gpucore documents it as load-bearing for
 *          the no-adapter-at-all row.
 *        - Empty list -> fall through to today's two-step
 *          (HighPerformance, then force-fallback), which
 *          produces the established GpuInit failure if the
 *          machine truly has no adapter.
 *      Policy source at the call sites: the resolved C8
 *      string — adapter::policy_from(Some(cfg.
 *      adapter_policy.as_str()), None) after config
 *      resolution, or policy_from(flag_value,
 *      env TALERENDER_ADAPTER_POLICY) where config is not
 *      consulted. Until the lead wires a policy through,
 *      every site keeps today's behaviour exactly (Auto is
 *      outcome-identical to the current preference order on
 *      any realistic adapter list: first hardware adapter,
 *      else the software one).
 *   4. DEVICE LOSS — gpucore::request_device, the single
 *      chokepoint every render entry point already funnels
 *      through (film/ab1/scenefile/dof via
 *      request_device_default, main.rs via request_device),
 *      so ONE wiring covers all five sites. Restructure the
 *      success path and install the closure IMMEDIATELY
 *      after the device is created, before returning:
 *          let (device, queue) = adapter
 *              .request_device(&wgpu::DeviceDescriptor { ... })
 *              .await
 *              .map_err(|e| TaleError::device_request(...))?;
 *          device.set_device_lost_callback(|reason, message| {
 *              // Destroyed is deliberate teardown
 *              // (Device::destroy); never a failure. No
 *              // site calls destroy() today, so this guard
 *              // is future-proofing, not load-bearing.
 *              if matches!(reason,
 *                          wgpu::DeviceLostReason::Destroyed) {
 *                  return;
 *              }
 *              let detail = if message.is_empty() {
 *                  format!("{reason:?}")
 *              } else {
 *                  format!("{reason:?}: {message}")
 *              };
 *              crate::observability::event(
 *                  "device.lost", &[("reason", detail)]);
 *              std::process::exit(3);
 *          });
 *          Ok((device, queue))
 *      Why process::exit inside the closure: the closure
 *      fires asynchronously from inside Device::poll
 *      (maintain), with no caller on the stack able to
 *      receive a TaleError — exit 3 IS the GpuInit-class
 *      exit code (errors.rs exit_code(): GpuInit /
 *      DeviceRequest / BufferMap / Render -> 3), matching
 *      C7's prescription exactly. observability::event
 *      prints via println! (newline-terminated; std stdout
 *      is line-buffered), so the event line is flushed
 *      before exit runs. Do not substitute process::abort
 *      (wrong code, no flush guarantee).
 *
 * DEVICE-LOSS SURVEY (the heart of this seat; sources:
 * docs.rs wgpu 30.0.2 pages for Device, DeviceLostReason,
 * DeviceType, Backend, AdapterInfo + the wgpu-core
 * DeviceLostClosure type, all fetched 2026-11-15):
 *   - Detection API EXISTS and is exactly what C7 assumes:
 *     Device::set_device_lost_callback — docs.rs 30.0.2
 *     Device page: "Set a DeviceLostCallback on this
 *     device." The closure type (wgpu-core) is
 *         DeviceLostClosure =
 *             Box<dyn FnOnce(DeviceLostReason, String)
 *                     + Send + 'static>
 *     i.e. the facade takes
 *     impl FnOnce(DeviceLostReason, String) + Send + 'static:
 *     called ONCE with the reason and a human-readable,
 *     implementation-specific message String (diagnostic
 *     only — never parse it).
 *   - DeviceLostReason in the PUBLIC wgpu 30.0.2 enum has
 *     exactly two variants: Unknown = 0 ("The device was
 *     lost for an unspecific reason, including driver
 *     errors") and Destroyed = 1 ("The device's destroy
 *     method was called"). (Older wgpu-types generations
 *     exposed Dropped / ReplacedCallback / DeviceInvalid as
 *     well; 30.0.2's public surface does not — the wiring
 *     above therefore discriminates with matches! on
 *     Destroyed only and treats every other reason as an
 *     unexpected loss, which stays correct if a future
 *     wgpu re-adds variants.)
 *   - Delivery is poll-driven: wgpu-core invokes the
 *     closure from its lose-the-device path via the
 *     UserCallbacks produced by maintain — in facade terms,
 *     during Device::poll ("Check for resource cleanups
 *     and mapping callbacks. Will block if PollType::Wait
 *     is passed", docs.rs 30.0.2). Core's own field doc
 *     warns the closure "might never be called" (e.g. the
 *     device is dropped without a further maintain). For
 *     TaleRender this is sufficient and timely: gpucore's
 *     Readback::collect polls with PollType::Wait on EVERY
 *     frame readback, so a mid-render loss surfaces at the
 *     next frame boundary at the latest; a loss after the
 *     final poll of a run may never fire — harmless, the
 *     run's work is already on disk. There is no
 *     polling-style is_lost query and no lost-future in
 *     the native 30.0.2 Device surface surveyed; the
 *     closure is the notification mechanism.
 *   - on_uncaptured_error EXISTS but is NOT a device-loss
 *     API: Device::on_uncaptured_error(handler: Box<dyn
 *     UncapturedErrorHandler>) — "Set a callback which will
 *     be called for all errors that are not handled in
 *     error scopes" (docs.rs 30.0.2); the handler receives
 *     a Box<dyn Error + Send + Sync>. A loss may coincide
 *     with uncaptured internal errors on some backends, but
 *     the callback carries no reason code and no loss
 *     guarantee, so it cannot substitute for the lost
 *     closure. This seat does NOT wire it; a logging-only
 *     handler is optional future hardening for the lead.
 *   - VERDICT 1 — detection + diagnosed failure:
 *     IMPLEMENTABLE, exactly as wired in point 4 above:
 *     event=device.lost reason=<...> via
 *     observability::event, then exit 3 (GpuInit class).
 *     One integration risk the lead MUST confirm on the
 *     first CI run with the closure wired: a clean lavapipe
 *     run still exits 0. The Destroyed guard covers the
 *     documented teardown path, and no code in the tree
 *     calls Device::destroy today; if CI ever shows a
 *     teardown firing with reason Unknown, the handler
 *     needs a run-finished flag (set after the last frame)
 *     instead of reason-only discrimination. Do not land
 *     the wiring without that CI observation.
 *   - VERDICT 2 — recreate-and-resume: NOT honestly
 *     supportable in this wave, and NOT implemented. Why,
 *     precisely: (a) wgpu permits the prerequisite — after
 *     a loss one may request a fresh adapter + device —
 *     but offers NO migration, checkpoint, or revalidation
 *     API: every texture, buffer, pipeline, bind group and
 *     sampler created from the lost device is invalid with
 *     it, and after a driver reset the ADAPTER itself may
 *     be invalid, so even re-request must start from the
 *     instance. (b) Resuming therefore means rebuilding
 *     the renderer's ENTIRE GPU state (film's Renderer::new
 *     construction: pipelines, PMREM, paint uploads,
 *     composer chains, standing targets) and re-rendering
 *     the in-flight frame from scratch; frames already
 *     written to disk are safe, so the honest unit of
 *     recovery is "restart the current frame on a new
 *     device", never "continue where it stopped".
 *     (c) TaleRender has no seam for that today: devices
 *     are created inside five entry points' constructors
 *     and owned deep inside long-lived renderer structs,
 *     with no device factory, no frame-level retry loop,
 *     and no replayable inventory of GPU resources (the
 *     VRAM ledger counts bytes; it cannot recreate
 *     objects). Building resume is a renderer-lifetime
 *     refactor far outside WP3's new-file charter, and C7
 *     sanctions it ONLY if the survey shows wgpu 30
 *     supports it without fabricating state — it does not.
 *     The limitation is documented here and belongs in
 *     STAGE1-SCOPE §6 as the lead records this landing.
 *
 * TESTS INCLUDED (#[cfg(test)], pure CPU — no GPU, no env,
 * no filesystem; CI runs them at integration):
 *   - policy_from: arg-beats-env, env fallback, both-None,
 *     each keyword, named with spaces/case preserved, named
 *     name trimming, empty named name invalid, invalid arg
 *     falling through to a valid env, invalid everywhere ->
 *     Auto, keyword case-sensitivity, whitespace trimming.
 *   - describe + report_line: hardware report fields and
 *     the exact canonical line, software report with the
 *     exact " (software)" suffix, driver_info empty (no
 *     trailing space), driver empty (no leading space),
 *     report_line over a hand-built AdapterReport.
 *   - select: fabricated wgpu::AdapterInfo values built
 *     literally (all 12 fields; the struct is all-pub with
 *     no #[non_exhaustive] on its docs.rs 30.0.2 page, so
 *     literal construction is available to tests) — the
 *     full policy matrix: empty slice, Auto preference and
 *     all-Cpu fallback, HardwareOnly None on all-Cpu,
 *     SoftwareOk first-always, Named substring +
 *     case-insensitive + no-match + software-matchable,
 *     VirtualGpu as non-Cpu, degenerate Named("").
 *
 * KNOWN LIMITATIONS:
 *   - This module selects and describes; it does not
 *     enumerate adapters or create devices — enumeration
 *     and both gpucore wirings are the lead's (points 3–4),
 *     including verifying Instance enumeration's exact
 *     30.0.2 call shape and the clean-run exit-0 check for
 *     the lost closure.
 *   - select's "hardware" is strictly non-Cpu per C7: a
 *     VirtualGpu (hosted/VM) adapter counts as hardware
 *     here. If the lead ever wants VirtualGpu treated as
 *     second-class, that is a policy change, not a bug in
 *     this module.
 *   - policy keywords are case-sensitive (config.rs C8
 *     parity): "AUTO" is invalid and resolves to Auto by
 *     fall-through — same result, different route; the
 *     named:<name> NAME keeps its case and matches
 *     case-insensitively.
 *   - If CI proves AdapterInfo literal construction is
 *     blocked after all (e.g. a non_exhaustive the docs
 *     render did not show), the fallback is mechanical:
 *     route select through an internal core over
 *     (&str, DeviceType) pairs and test that — select's
 *     semantics are unchanged. Flagged, not expected.
 *   - Device-loss resume is documented-impossible for this
 *     wave (survey verdict 2); the film-s03 pin, RADV
 *     numbers and skill-tree adoption are out of scope per
 *     V2-CONTRACTS §6.
 */

//! Adapter policy, reporting and selection — V2 WP3
//! (contract C7).
//!
//! One canonical place answers three questions every render
//! entry point used to answer inline: WHICH adapter may be
//! used (`AdapterPolicy` + `select`), WHAT was used
//! (`describe` + `report_line` — a software adapter is
//! always labelled software, master prompt §7), and — via
//! the delivery note at the top of this file — what happens
//! when the device is lost (gpucore wiring; detection +
//! diagnosed exit 3, no fabricated resume).
//!
//! Pure CPU logic over `wgpu::AdapterInfo`: this module
//! never creates an instance, adapter or device, so every
//! function is unit-testable without a GPU. std + wgpu
//! types only; no new dependencies.

// ---------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------

/// Which adapters are acceptable, resolved from the CLI /
/// config string by `policy_from` and consumed by `select`
/// (and, at integration, by gpucore's adapter request).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AdapterPolicy {
    /// First hardware (non-Cpu) adapter if any, else the
    /// first adapter — the outcome of today's two-step
    /// request_adapter preference order.
    Auto,
    /// First hardware (non-Cpu) adapter; software is a
    /// failure, never a fallback.
    HardwareOnly,
    /// The first adapter, whatever it is (software welcome).
    SoftwareOk,
    /// First adapter whose name contains the given string
    /// (case-insensitive substring match).
    Named(String),
}

/// Parse one policy string (see the delivery note for the
/// grammar). Returns None for anything invalid, including
/// `named:` with an empty name; keywords are case-sensitive
/// and the whole value is whitespace-trimmed first (the
/// `named:` name is trimmed too, its interior case kept).
fn parse_policy(raw: &str) -> Option<AdapterPolicy> {
    let v = raw.trim();
    match v {
        "auto" => Some(AdapterPolicy::Auto),
        "hardware-only" => Some(AdapterPolicy::HardwareOnly),
        "software-ok" => Some(AdapterPolicy::SoftwareOk),
        _ => {
            let name = v.strip_prefix("named:")?.trim();
            if name.is_empty() {
                None
            } else {
                Some(AdapterPolicy::Named(name.to_string()))
            }
        }
    }
}

/// Resolve the adapter policy: `arg` beats `env`; the first
/// source yielding a VALID value wins, an invalid value
/// falls through to the next source, and if neither source
/// yields a valid value the policy is `Auto` (so a single
/// invalid value resolves to `Auto`). Never panics.
pub(crate) fn policy_from(arg: Option<&str>, env: Option<&str>) -> AdapterPolicy {
    for candidate in [arg, env].into_iter().flatten() {
        if let Some(policy) = parse_policy(candidate) {
            return policy;
        }
    }
    AdapterPolicy::Auto
}

// ---------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------

/// The selected adapter, flattened to the strings the
/// canonical report line prints (contract C7's shape).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdapterReport {
    pub name: String,
    /// Debug spelling of `wgpu::Backend` (e.g. "Vulkan") —
    /// the spelling the existing print sites produce with
    /// `{:?}`.
    pub backend: String,
    /// Debug spelling of `wgpu::DeviceType` (e.g.
    /// "IntegratedGpu", "Cpu").
    pub device_type: String,
    /// Driver name and driver info as one string: the
    /// existing sites' `"{} {}"` (driver, driver_info) pair,
    /// space-joined when both are present, the present one
    /// alone otherwise.
    pub driver: String,
    /// True iff the adapter's device type is Cpu — a
    /// software adapter, always labelled as such.
    pub is_software: bool,
}

/// Flatten a `wgpu::AdapterInfo` into an `AdapterReport`.
/// `is_software` is `device_type == Cpu` (contract C7).
pub(crate) fn describe(info: &wgpu::AdapterInfo) -> AdapterReport {
    let driver = match (info.driver.is_empty(), info.driver_info.is_empty()) {
        (true, true) => String::new(),
        (false, true) => info.driver.clone(),
        (true, false) => info.driver_info.clone(),
        (false, false) => format!("{} {}", info.driver, info.driver_info),
    };
    AdapterReport {
        name: info.name.clone(),
        backend: format!("{:?}", info.backend),
        device_type: format!("{:?}", info.device_type),
        driver,
        is_software: matches!(info.device_type, wgpu::DeviceType::Cpu),
    }
}

/// The canonical adapter line, byte-shaped like the lines
/// main.rs / ab1.rs / scenefile.rs already print:
/// `adapter: <name> | backend: <b> | device type: <t> | driver: <d>`,
/// with ` (software)` appended iff the report is software.
pub(crate) fn report_line(r: &AdapterReport) -> String {
    let mut line = format!(
        "adapter: {} | backend: {} | device type: {} | driver: {}",
        r.name, r.backend, r.device_type, r.driver
    );
    if r.is_software {
        line.push_str(" (software)");
    }
    line
}

// ---------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------

/// Pure selection over adapter infos: returns the index of
/// the adapter `policy` chooses, or None when no listed
/// adapter satisfies it. Semantics per variant are defined
/// on `AdapterPolicy` and in the delivery note; an empty
/// slice yields None under every policy.
pub(crate) fn select(adapters: &[wgpu::AdapterInfo], policy: &AdapterPolicy) -> Option<usize> {
    fn is_cpu(info: &wgpu::AdapterInfo) -> bool {
        matches!(info.device_type, wgpu::DeviceType::Cpu)
    }
    match policy {
        AdapterPolicy::Auto => adapters
            .iter()
            .position(|info| !is_cpu(info))
            .or_else(|| (!adapters.is_empty()).then_some(0)),
        AdapterPolicy::HardwareOnly => adapters.iter().position(|info| !is_cpu(info)),
        AdapterPolicy::SoftwareOk => (!adapters.is_empty()).then_some(0),
        AdapterPolicy::Named(name) => {
            let needle = name.to_lowercase();
            adapters
                .iter()
                .position(|info| info.name.to_lowercase().contains(&needle))
        }
    }
}

// ---------------------------------------------------------------------
// Tests — pure CPU only (header law: GPU paths are CI-verified).
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fabricate an AdapterInfo literally — all twelve
    /// fields, as listed on the docs.rs wgpu 30.0.2 page
    /// (all pub, no #[non_exhaustive]). Only name /
    /// device_type vary in most tests; driver fields start
    /// empty and describe tests set them explicitly.
    fn fake_info(name: &str, device_type: wgpu::DeviceType) -> wgpu::AdapterInfo {
        wgpu::AdapterInfo {
            name: name.to_string(),
            vendor: 0x1002,
            device: 0,
            device_type,
            device_pci_bus_id: String::new(),
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
            subgroup_min_size: 8,
            subgroup_max_size: 8,
            transient_saves_memory: None,
            limit_bucket: None,
        }
    }

    fn mixed() -> Vec<wgpu::AdapterInfo> {
        use wgpu::DeviceType as T;
        vec![
            fake_info("llvmpipe (LLVM 17.0.6, 256 bits)", T::Cpu),
            fake_info("AMD Radeon Vega 7 (RADV VEGA10)", T::IntegratedGpu),
            fake_info("NVIDIA GeForce RTX 4090", T::DiscreteGpu),
        ]
    }

    // ---------------- policy_from ----------------

    #[test]
    fn policy_arg_beats_env() {
        assert_eq!(
            policy_from(Some("software-ok"), Some("hardware-only")),
            AdapterPolicy::SoftwareOk
        );
        assert_eq!(
            policy_from(Some("hardware-only"), Some("software-ok")),
            AdapterPolicy::HardwareOnly
        );
    }

    #[test]
    fn policy_env_used_when_arg_absent_and_default_is_auto() {
        assert_eq!(policy_from(None, Some("hardware-only")), AdapterPolicy::HardwareOnly);
        assert_eq!(policy_from(None, None), AdapterPolicy::Auto);
        assert_eq!(policy_from(Some("auto"), None), AdapterPolicy::Auto);
    }

    #[test]
    fn policy_named_keeps_name_case_and_spaces() {
        assert_eq!(
            policy_from(Some("named:NVIDIA GeForce RTX"), None),
            AdapterPolicy::Named("NVIDIA GeForce RTX".to_string())
        );
        // The name is trimmed; interior spacing/case survive.
        assert_eq!(
            policy_from(None, Some("named:  radv vega  ")),
            AdapterPolicy::Named("radv vega".to_string())
        );
    }

    #[test]
    fn policy_invalid_falls_through_then_auto() {
        // Invalid arg, valid env: the env wins (house
        // fall-through pattern; a single invalid source is
        // the charter's "invalid -> Auto" case, next test).
        assert_eq!(
            policy_from(Some("bogus"), Some("hardware-only")),
            AdapterPolicy::HardwareOnly
        );
        assert_eq!(policy_from(Some("bogus"), None), AdapterPolicy::Auto);
        assert_eq!(policy_from(Some("bogus"), Some("also-bogus")), AdapterPolicy::Auto);
        assert_eq!(policy_from(Some(""), Some("")), AdapterPolicy::Auto);
    }

    #[test]
    fn policy_named_with_empty_name_is_invalid() {
        assert_eq!(policy_from(Some("named:"), None), AdapterPolicy::Auto);
        assert_eq!(policy_from(Some("named:   "), None), AdapterPolicy::Auto);
        // ...and falls through like any invalid value.
        assert_eq!(
            policy_from(Some("named:"), Some("software-ok")),
            AdapterPolicy::SoftwareOk
        );
    }

    #[test]
    fn policy_keywords_are_case_sensitive_and_trimmed() {
        // config.rs (C8) parity: "AUTO" is not a keyword.
        assert_eq!(policy_from(Some("AUTO"), None), AdapterPolicy::Auto);
        assert_eq!(policy_from(Some("Hardware-Only"), None), AdapterPolicy::Auto);
        assert_eq!(
            policy_from(Some("  hardware-only  "), None),
            AdapterPolicy::HardwareOnly
        );
    }

    // ---------------- describe + report_line ----------------

    #[test]
    fn describe_hardware_report_and_canonical_line() {
        let mut info = fake_info("AMD Radeon Vega 7 (RADV VEGA10)", wgpu::DeviceType::IntegratedGpu);
        info.driver = "radv".to_string();
        info.driver_info = "Mesa 26.0.8".to_string();
        let r = describe(&info);
        assert_eq!(r.name, "AMD Radeon Vega 7 (RADV VEGA10)");
        assert_eq!(r.backend, "Vulkan");
        assert_eq!(r.device_type, "IntegratedGpu");
        assert_eq!(r.driver, "radv Mesa 26.0.8");
        assert!(!r.is_software);
        assert_eq!(
            report_line(&r),
            "adapter: AMD Radeon Vega 7 (RADV VEGA10) | backend: Vulkan \
             | device type: IntegratedGpu | driver: radv Mesa 26.0.8"
        );
    }

    #[test]
    fn describe_software_report_is_labelled_software() {
        let mut info = fake_info("llvmpipe (LLVM 17.0.6, 256 bits)", wgpu::DeviceType::Cpu);
        info.driver = "llvmpipe".to_string();
        info.driver_info = "LLVM 17.0.6".to_string();
        let r = describe(&info);
        assert!(r.is_software);
        assert_eq!(r.device_type, "Cpu");
        assert_eq!(
            report_line(&r),
            "adapter: llvmpipe (LLVM 17.0.6, 256 bits) | backend: Vulkan \
             | device type: Cpu | driver: llvmpipe LLVM 17.0.6 (software)"
        );
    }

    #[test]
    fn describe_driver_joining_never_pads() {
        // driver_info empty: no trailing space in the line.
        let mut info = fake_info("Some GPU", wgpu::DeviceType::DiscreteGpu);
        info.driver = "radv".to_string();
        let r = describe(&info);
        assert_eq!(r.driver, "radv");
        assert!(report_line(&r).ends_with("| driver: radv"), "got: {}", report_line(&r));
        // driver empty, info present: no leading space.
        let mut info = fake_info("Some GPU", wgpu::DeviceType::DiscreteGpu);
        info.driver_info = "Mesa 26.0.8".to_string();
        assert_eq!(describe(&info).driver, "Mesa 26.0.8");
        // Both empty: empty driver, line still well-formed.
        let info = fake_info("Some GPU", wgpu::DeviceType::Other);
        assert_eq!(describe(&info).driver, "");
    }

    #[test]
    fn report_line_from_hand_built_report() {
        let hw = AdapterReport {
            name: "X".to_string(),
            backend: "Vulkan".to_string(),
            device_type: "DiscreteGpu".to_string(),
            driver: "nvidia 555".to_string(),
            is_software: false,
        };
        assert_eq!(
            report_line(&hw),
            "adapter: X | backend: Vulkan | device type: DiscreteGpu | driver: nvidia 555"
        );
        let sw = AdapterReport { is_software: true, ..hw };
        assert_eq!(
            report_line(&sw),
            "adapter: X | backend: Vulkan | device type: DiscreteGpu | driver: nvidia 555 (software)"
        );
    }

    // ---------------- select ----------------

    #[test]
    fn select_empty_slice_is_none_for_every_policy() {
        let none: &[wgpu::AdapterInfo] = &[];
        assert_eq!(select(none, &AdapterPolicy::Auto), None);
        assert_eq!(select(none, &AdapterPolicy::HardwareOnly), None);
        assert_eq!(select(none, &AdapterPolicy::SoftwareOk), None);
        assert_eq!(select(none, &AdapterPolicy::Named("radv".to_string())), None);
    }

    #[test]
    fn select_auto_prefers_first_hardware_else_first() {
        let v = mixed();
        assert_eq!(select(&v, &AdapterPolicy::Auto), Some(1));
        // All-Cpu: Auto falls back to the first adapter
        // (today's force-fallback outcome).
        let cpus = vec![
            fake_info("llvmpipe", wgpu::DeviceType::Cpu),
            fake_info("softpipe", wgpu::DeviceType::Cpu),
        ];
        assert_eq!(select(&cpus, &AdapterPolicy::Auto), Some(0));
    }

    #[test]
    fn select_hardware_only_never_takes_software() {
        let v = mixed();
        assert_eq!(select(&v, &AdapterPolicy::HardwareOnly), Some(1));
        let cpus = vec![fake_info("llvmpipe", wgpu::DeviceType::Cpu)];
        assert_eq!(select(&cpus, &AdapterPolicy::HardwareOnly), None);
    }

    #[test]
    fn select_software_ok_always_takes_first() {
        let v = mixed();
        assert_eq!(select(&v, &AdapterPolicy::SoftwareOk), Some(0));
        let hw_first = vec![
            fake_info("AMD RADV", wgpu::DeviceType::DiscreteGpu),
            fake_info("llvmpipe", wgpu::DeviceType::Cpu),
        ];
        assert_eq!(select(&hw_first, &AdapterPolicy::SoftwareOk), Some(0));
    }

    #[test]
    fn select_named_is_case_insensitive_substring() {
        let v = mixed();
        assert_eq!(select(&v, &AdapterPolicy::Named("radv".to_string())), Some(1));
        assert_eq!(select(&v, &AdapterPolicy::Named("GEFORCE".to_string())), Some(2));
        assert_eq!(select(&v, &AdapterPolicy::Named("ega 7 (RADV".to_string())), Some(1));
        assert_eq!(select(&v, &AdapterPolicy::Named("intel".to_string())), None);
        // Named imposes no device-type restriction: it may
        // select the software adapter (report_line still
        // labels it software).
        assert_eq!(select(&v, &AdapterPolicy::Named("llvm".to_string())), Some(0));
    }

    #[test]
    fn select_virtual_gpu_counts_as_non_cpu() {
        // C7 defines software strictly as device_type == Cpu.
        let v = vec![
            fake_info("llvmpipe", wgpu::DeviceType::Cpu),
            fake_info("virtio-gpu", wgpu::DeviceType::VirtualGpu),
        ];
        assert_eq!(select(&v, &AdapterPolicy::Auto), Some(1));
        assert_eq!(select(&v, &AdapterPolicy::HardwareOnly), Some(1));
    }

    #[test]
    fn select_named_empty_string_is_defined_as_first() {
        // Degenerate: policy_from can never produce Named(""),
        // but if one is constructed directly, contains("")
        // is true and the first adapter matches. Pinned so
        // the behaviour is a decision, not an accident.
        let v = mixed();
        assert_eq!(select(&v, &AdapterPolicy::Named(String::new())), Some(0));
    }
}
