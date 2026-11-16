//! TaleRender — native Rust + wgpu renderer for Tall Tale Shorts.
//!
//! A native renderer for the Tall Tale hybrid format: expressive
//! camera-facing 2D characters inside fully dimensional 3D
//! environments with cinematic 3D cameras. No browser, no window,
//! no display server — headless, offscreen, PNG out.
//!
//! Modes: built-in scenes, scene-format JSON files (v1/v2), whole
//! films (scene format v2 + the film driver), and the M6 QA
//! telemetry gates (pure CPU). The full command surface is the
//! `--help` text below (const HELP) — keep the two in step.
//!
//! The adapter actually used is printed on every run: on GPU-less
//! machines wgpu falls back to a software Vulkan driver
//! (lavapipe/llvmpipe) and the log says so. Nothing here claims GPU
//! acceleration it cannot prove.

mod ab1;
mod adapter;
mod anim;
mod assetstream;
mod assetwire;
mod atlas;
mod battery;
mod bufferpool;
mod charanim;
mod charstate;
mod composer;
mod config;
mod dirtyrect;
mod dof;
mod dyntex;
mod errors;
mod eval15;
mod gpucore;
#[cfg(feature = "gpupaint")]
mod gpupaint_states;
mod gpupaint_time;
mod lifetimes;
mod math;
mod mempressure;
mod memreport;
mod memreport_print;
mod observability;
mod outputmgr;
mod outwire;
mod locomotion;
mod paintbudget;
mod rendergraph;
mod runlog;
mod scheduler;
mod schedwire;
mod scene;
mod projectfile;
mod scenefile;
mod streamq;
mod streammodes;
mod streamsession;
mod telemetry;
mod ffstream;
mod film;
mod filmseq;
mod filmtelemetry;
mod toonfull;
#[cfg(feature = "gpupaint")]
pub(crate) mod gpupaint;
#[cfg(feature = "gpupaint")]
pub(crate) mod gputrace;

use math::Mat4;
use std::borrow::Cow;

const HELP: &str = "\
TaleRender — native Rust + wgpu renderer for Tall Tale Shorts.
No Chrome, no browser: expressive 2D characters inside dimensional
3D environments, rendered headless to PNG (clean frames).

USAGE:
    talerender [OPTIONS]
    talerender --scene-file <scene.json> [--out frame.png]
    talerender --film <DIR> --out <FRAMES_DIR> [--from N] [--to N]
    talerender --film-telemetry <DIR>
    talerender --telemetry <MODE> [--scene-file <scene.json>]

MODES (choose one; the default is a built-in scene render):
    --scene <NAME>          Built-in scene: m1 | ab1 | dof-chart |
                            stroke-battery (default: m1).
                            ab1 = the §28 Onoda reference scene;
                            dof-chart = the M3 Bokeh chart;
                            stroke-battery writes the M4 battery
                            cards into --out (a directory).
    --scene-file <PATH>     Render a scene-format JSON file (v1/v2;
                            see docs/scene-format.md). Wins over
                            --scene when both are given.
    --film <DIR>            Render a whole film: DIR holds the
                            film data (film.json — the generic
                            entry; onoda.json accepted for this
                            film) + an assets/ dir; the segment
                            shots are staged by the driver in
                            code (scenes/film/ is the tools'
                            manifest copy — --film never reads
                            it). Frames are written to --out as
                            frame_%05d.png; existing frames are
                            skipped (resume).
    --film-telemetry <DIR>  Native telemetry gate over a film
                            directory (CPU only, no GPU). The
                            verdict is the printed line
                            \"FILM TELEMETRY: CHECK OK\" /
                            \"CHECK FAILED (...)\" — read the line.
    --tables                With --film-telemetry: also print the
                            shot, caption and cast tables after the
                            verdict line. Default output is unchanged.
    --telemetry <MODE>      M6 QA telemetry (CPU only):
                            ab1 | ab1-file | ab1-variant.
                            Pair ab1-file with --scene-file.

OPTIONS:
    --width <PX>            Frame width (default: 1080).
    --height <PX>           Frame height (default: 1920).
    --out <PATH>            Output PNG file — or, for --film and
                            stroke-battery, the output directory
                            (default: frame.png). For --film in
                            stream output mode, the output MP4
                            file instead.
    --caption <PATH>        Caption strip PNG composited by
                            --scene ab1 (default: the §28 strip
                            under docs/ab/ — a repo-relative path,
                            so run from the repo root or pass
                            --caption explicitly).
    --post-stage <STAGE>    Composer ablation for --scene ab1:
                            lighting | bloom | grade
                            (default: grade = the full chain).
    --captions <PATH>       Caption metrics JSON (for --telemetry).
    --cutouts-out <PATH>    Write cutouts() JSON (with --telemetry).
    --from <N>              First film frame to render, inclusive
                            (default: 0).
    --to <N>                Film frame end, exclusive; 0 = through
                            the end (default: 0).
    --res <SCALE>           Render scale for drafts: 0 < SCALE <= 1,
                            applied to --width/--height (default: 1
                            = full size). The whole frame scales,
                            captions included. Draft only.
    --aa <0|2|4|8>          MSAA samples for the HDR scene pass
                            (default: 4 = full quality; 0 =
                            single-sample). Draft only below 4.
    --shadow <SIZE>         Key shadow map size: 512 | 1024 | 2048 |
                            4096 (default: 2048 = full quality).
    --draft                 Draft preset: --res 0.5 --aa 0
                            --shadow 512. Any of the three flags
                            given after --draft overrides it.
                            Draft runs label every output line
                            DRAFT — never use for delivery.
    --mem-profile <NAME>    Memory profile: low-memory |
                            balanced | high-performance
                            (default: balanced, or low-memory
                            on machines with <= 8 GiB RAM).
                            Profiles change budgets and queue
                            sizes only — never quality.
    --adapter-policy <P>    Adapter selection: auto |
                            hardware-only | software-ok |
                            named:<substring> (default: auto).
    --output-mode <MODE>    Film output: png | stream | auto
                            (default: png). The resolved mode
                            is printed on every --film run;
                            stream encodes straight to MP4
                            through the streaming session
                            (no PNG frames are written).
    --perf-record-dir <DIR> Directory for performance records
                            (bench tooling; see docs/v2/
                            PERF-LAB.md).
    --config <PATH>         Config file (default: ./
                            talerender.toml when present).
                            Precedence: CLI > TALERENDER_*
                            env > config file > defaults;
                            invalid values fall through with a
                            warning, never a hard error.

    -h, --help              Print this help and exit.
    -V, --version           Print the version and exit.

ENVIRONMENT:
    TALERENDER_PAINT_CACHE <N>
                            Cap the paint cache at N entries
                            (default: 160). Lower it on low-RAM
                            machines; telemetry runs often use 24.
    TALERENDER_PAINT_BUDGET_MB <N>
                            Cap the paint cache at N MiB of
                            painted canvases (default: derived —
                            the entry cap's bytes, at most
                            2048 MiB). 0 disables the byte budget
                            (entry cap alone). Eviction under
                            either cap costs time, never pixels.

EXIT CODES:
    0   Success.
    1   --film-telemetry QA verdict: CHECK FAILED (the printed
        line carries the counts and stays authoritative).
    2   Command-line error (unknown flag, bad or missing value);
        also film or scene data that fails to load.
    3   --film-telemetry could not run (film data unreadable;
        the reason is printed on stderr, no verdict line);
        also GPU initialisation or render failure in any mode.
    4   Image I/O failure (reading or writing a PNG).

The renderer actually used is printed on every run — a hardware
GPU or a software Vulkan driver such as lavapipe. Nothing claims
GPU acceleration it cannot prove.
";

/// Print a command-line error and exit 2, the way a CLI should:
/// no panic, a hint at the help text.
fn cli_error(msg: &str) -> ! {
    eprintln!("error: {msg}");
    eprintln!("Try 'talerender --help' for usage.");
    std::process::exit(2);
}

/// The value following a flag, or a clean error if it is missing.
fn value(args: &[String], i: usize, flag: &str) -> String {
    args.get(i)
        .cloned()
        .unwrap_or_else(|| cli_error(&format!("missing value for {flag}")))
}

/// The value following a numeric flag, parsed, or a clean error.
fn parse_u32(args: &[String], i: usize, flag: &str) -> u32 {
    let raw = value(args, i, flag);
    raw.parse().unwrap_or_else(|_| {
        cli_error(&format!(
            "invalid value for {flag}: '{raw}' (expected a non-negative integer)"
        ))
    })
}

/// Draft/preview quality knobs (KNOBS-W43; patch pack
/// KNOBS-PATCH-W54). Defaults are full quality: a run at
/// `Knobs::FULL` must be byte-identical to the pre-knob binary,
/// frames and stdout alike. Knobs exist for drafts only.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Knobs {
    /// Render scale, 0 < res <= 1 (1 = full size).
    pub res: f32,
    /// MSAA in the user's spelling: 0 | 2 | 4 | 8 (0 = single).
    pub aa: u32,
    /// Key shadow map size: 512 | 1024 | 2048 | 4096.
    pub shadow: u32,
}

impl Knobs {
    pub(crate) const FULL: Knobs = Knobs { res: 1.0, aa: 4, shadow: 2048 };

    /// The wgpu sample count: the user's 0 is single-sample.
    pub(crate) fn samples(self) -> u32 {
        if self.aa == 0 { 1 } else { self.aa }
    }

    /// Any knob off its default makes the run a draft. The
    /// condition names all three knobs from birth (W73's
    /// warning): a later phase that plumbs aa/shadow must not
    /// be able to produce an untagged draft.
    pub(crate) fn is_draft(self) -> bool {
        self.res != 1.0 || self.aa != 4 || self.shadow != 2048
    }

    /// The in-band draft tag (KNOBS-W43 §3.4), with the
    /// effective values so any log snippet carries provenance.
    pub(crate) fn tag(self) -> String {
        format!("[DRAFT res={} aa={} shadow={} — NOT FOR DELIVERY]",
                self.res, self.aa, self.shadow)
    }
}

fn parse_res(args: &[String], i: usize) -> f32 {
    let raw = value(args, i, "--res");
    let v: f32 = raw.parse().unwrap_or_else(|_| {
        cli_error(&format!(
            "invalid value for --res: '{raw}' (expected 0 < scale <= 1)"))
    });
    // Negated conjunction on purpose: it also rejects NaN,
    // which parses as an f32 but satisfies no ordering.
    if !(v > 0.0 && v <= 1.0) {
        cli_error(&format!(
            "invalid value for --res: '{raw}' (expected 0 < scale <= 1)"));
    }
    v
}

fn parse_aa(args: &[String], i: usize) -> u32 {
    let v = parse_u32(args, i, "--aa");
    if !matches!(v, 0 | 2 | 4 | 8) {
        cli_error(&format!(
            "invalid value for --aa: '{}' (expected 0, 2, 4 or 8)",
            value(args, i, "--aa")));
    }
    v
}

fn parse_shadow(args: &[String], i: usize) -> u32 {
    let v = parse_u32(args, i, "--shadow");
    if !matches!(v, 512 | 1024 | 2048 | 4096) {
        cli_error(&format!(
            "invalid value for --shadow: '{}' (expected 512, 1024, 2048 or 4096)",
            value(args, i, "--shadow")));
    }
    v
}

/// Effective frame size: --width/--height x --res, computed
/// once in main() before dispatch. At res == 1 the base size is
/// returned exactly (the defaults identity law); below 1 the
/// size is rounded to even — the skill's draft encode is
/// yuv420 and needs even dimensions — with a floor of 2.
fn effective_size(width: u32, height: u32, res: f32) -> (u32, u32) {
    if res == 1.0 {
        return (width, height);
    }
    let scale = |v: u32| -> u32 {
        let raw = (v as f64 * res as f64).round() as u32;
        (raw.max(2) & !1).max(2)
    };
    (scale(width), scale(height))
}

/// Run a mode that returns a structured error: print it the CLI
/// way and exit with the error's audit-table code (ERRORS-W05).
fn exit_on_err(r: errors::TaleResult<()>) {
    if let Err(e) = r {
        eprintln!("error: {e}");
        std::process::exit(e.exit_code());
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Help and version win anywhere on the command line.
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        #[cfg(feature = "gpupaint")]
        println!("\n    --gpu-paint <SUB>       GPU-first Phase 0 trace harness (feature gpupaint): record-battery | record-states | replay | compare | time. Run '--gpu-paint' with no subcommand for its usage.");
        return;
    }
    // GPU-first Phase 0 harness (feature gpupaint only): handled
    // before the production argument loop; with the feature off
    // the flag falls through to the loop's unknown-argument error,
    // exactly as before.
    #[cfg(feature = "gpupaint")]
    if let Some(pos) = args.iter().position(|a| a == "--gpu-paint") {
        std::process::exit(crate::gpupaint::run(&args[pos + 1..]));
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("talerender {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let mut width: u32 = 1080;
    let mut height: u32 = 1920;
    let mut knobs = Knobs::FULL;
    let mut out = String::from("frame.png");
    let mut scene = String::from("m1");
    let mut caption = String::from("docs/ab/onoda-s01-t150/assets/caption.png");
    let mut scene_file: Option<String> = None;
    // M5 ablation control for --scene ab1: how much of the composer
    // chain runs (lighting | bloom | grade). Default is the full
    // chain, matching production.
    let mut post_stage = String::from("grade");
    let mut telemetry_mode: Option<String> = None;
    let mut captions_path: Option<String> = None;
    let mut cutouts_out: Option<String> = None;
    let mut film_dir: Option<String> = None;
    let mut film_telemetry_dir: Option<String> = None;
    let mut tables = false;
    let mut gpupaint_time_flag = false;
    let mut gpupaint_no_gpu = false;
    let mut film_from: u32 = 0;
    let mut film_to: u32 = 0;
    // V2 C8 configuration layers: raw CLI values only —
    // config::resolve validates them after the loop (invalid
    // values fall through with warnings, never cli_error).
    let mut cli_overrides = config::CliOverrides::default();
    let mut config_path: Option<std::path::PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--width" => {
                i += 1;
                width = parse_u32(&args, i, "--width");
            }
            "--height" => {
                i += 1;
                height = parse_u32(&args, i, "--height");
            }
            "--out" => {
                i += 1;
                out = value(&args, i, "--out");
            }
            "--scene" => {
                i += 1;
                scene = value(&args, i, "--scene");
            }
            "--scene-file" => {
                i += 1;
                scene_file = Some(value(&args, i, "--scene-file"));
            }
            "--caption" => {
                i += 1;
                caption = value(&args, i, "--caption");
            }
            "--post-stage" => {
                i += 1;
                post_stage = value(&args, i, "--post-stage");
            }
            "--telemetry" => {
                i += 1;
                telemetry_mode = Some(value(&args, i, "--telemetry"));
            }
            "--captions" => {
                i += 1;
                captions_path = Some(value(&args, i, "--captions"));
            }
            "--cutouts-out" => {
                i += 1;
                cutouts_out = Some(value(&args, i, "--cutouts-out"));
            }
            "--gpupaint-time" => {
                gpupaint_time_flag = true;
            }
            "--gpupaint-no-gpu" => {
                gpupaint_no_gpu = true;
            }
            "--gpupaint-gpu-only" => {
                // Modifier for --gpupaint-time; consumed at dispatch.
            }
            "--film-telemetry" => {
                i += 1;
                film_telemetry_dir = Some(value(&args, i, "--film-telemetry"));
            }
            "--tables" => {
                tables = true;
            }
            "--film" => {
                i += 1;
                film_dir = Some(value(&args, i, "--film"));
            }
            "--from" => {
                i += 1;
                film_from = parse_u32(&args, i, "--from");
            }
            "--to" => {
                i += 1;
                film_to = parse_u32(&args, i, "--to");
            }
            "--res" => {
                i += 1;
                knobs.res = parse_res(&args, i);
            }
            "--aa" => {
                i += 1;
                knobs.aa = parse_aa(&args, i);
            }
            "--shadow" => {
                i += 1;
                knobs.shadow = parse_shadow(&args, i);
            }
            "--mem-profile" => {
                i += 1;
                cli_overrides.mem_profile = Some(value(&args, i, "--mem-profile"));
            }
            "--adapter-policy" => {
                i += 1;
                cli_overrides.adapter_policy = Some(value(&args, i, "--adapter-policy"));
            }
            "--output-mode" => {
                i += 1;
                cli_overrides.output_mode = Some(value(&args, i, "--output-mode"));
            }
            "--perf-record-dir" => {
                i += 1;
                cli_overrides.perf_record_dir =
                    Some(std::path::PathBuf::from(value(&args, i, "--perf-record-dir")));
            }
            "--config" => {
                i += 1;
                config_path = Some(std::path::PathBuf::from(value(&args, i, "--config")));
            }
            "--draft" => {
                // A writer of all three fields; the loop is
                // last-writer-wins in command-line order, in
                // both directions (lead-confirmed symmetric
                // rule, KNOBS-PATCH-W54 §9.2 item 1).
                knobs.res = 0.5;
                knobs.aa = 0;
                knobs.shadow = 512;
            }
            other => {
                cli_error(&format!("unknown argument: {other}"));
            }
        }
        i += 1;
    }
    // C8 resolution (CLI > TALERENDER_* env > talerender.toml
    // > default). Warnings for ignored invalid values go to
    // stderr; the resolved values are reported by the modes
    // that consume them (film branch below, run()).
    let (cfg, cfg_warnings) =
        config::resolve_from_process_with_report(&cli_overrides, config_path.as_deref());
    for w in &cfg_warnings {
        eprintln!("warning: {w}");
    }
    if gpupaint_time_flag {
        // GPUFIRST Phase 0 PERF harness: CPU paint timings for the
        // states spread + battery cards, then (unless
        // --gpupaint-no-gpu) the GPU texture-replay proxy on the
        // selected adapter. Measurement only; renders nothing.
        let gpu_only = args.iter().any(|a| a == "--gpupaint-gpu-only");
        std::process::exit(gpupaint_time::run_mode(!gpupaint_no_gpu, gpu_only));
    }
    if let Some(mode) = &telemetry_mode {
        // M6 QA telemetry (contract §3): pure CPU — the painter
        // canvas + scene-graph projection; no GPU device is created.
        std::process::exit(telemetry::run(
            mode,
            scene_file.as_deref(),
            captions_path.as_deref(),
            cutouts_out.as_deref(),
        ));
    }
    if let Some(dir) = &film_telemetry_dir {
        // Native telemetry gate over the whole film (M6 semantics,
        // 0.25 s sampling) — no GPU needed. The verdict line printed
        // by run_film_telemetry stays authoritative; the exit code
        // agrees with it (EXIT-CODES-W02): 0 OK, 1 FAILED, 3 could
        // not run.
        match film::run_film_telemetry(std::path::Path::new(dir), tables) {
            Ok(true) => {}
            Ok(false) => std::process::exit(1),
            Err(e) => {
                eprintln!("film telemetry failed: {e}");
                std::process::exit(3);
            }
        }
        return;
    }
    // F4 knobs (KNOBS-W43 §3.1): the effective size is computed
    // once, here, before any GPU dispatch. Every entry point
    // below receives (ew, eh) in place of (width, height), so
    // film's Renderer::new is built at the effective size — the
    // substitution cannot land after it. The telemetry
    // early-returns above are CPU-only and never see it.
    let (ew, eh) = effective_size(width, height, knobs.res);
    if let Some(dir) = &film_dir {
        // Production integration: the complete film, frame range
        // [--from, --to) into the --out frames directory (resume:
        // existing frames are skipped). --to 0 = through the end.
        println!(
            "config: mem_profile={} adapter_policy={} output_mode={}",
            cfg.mem_profile, cfg.adapter_policy, cfg.output_mode
        );
        // C3 mode resolution: the choice is printed, never
        // silent. A resolved Stream is dispatched inside
        // run_film (streamsession); PNG remains the default
        // and its path is unchanged.
        let requested = streammodes::mode_from_config(&cfg.output_mode);
        let resolved = streammodes::resolve_mode(requested, ffstream::ffmpeg_available());
        println!("output-mode: {} — {}", resolved.mode.as_str(), resolved.reason);
        // C1: the resolved memory profile, reported with its
        // derived values. Consumers (paint-cache budget feed,
        // frame-queue caps) wire incrementally; the profile
        // itself never changes quality.
        let mp = mempressure::MemPressure::from_env_and_arg(Some(cfg.mem_profile.as_str()));
        println!(
            "mem-profile: {} (profile paint budget {} bytes, frame queue cap {})",
            mp.profile().name(),
            mp.paint_budget_bytes(),
            mp.frame_queue_capacity()
        );
        exit_on_err(pollster::block_on(film::run_film(
            ew,
            eh,
            dir,
            &out,
            film_from,
            film_to,
            knobs,
            &cfg.adapter_policy,
            resolved.mode,
            mp.frame_queue_capacity(),
        )));
        return;
    }
    if let Some(path) = &scene_file {
        // A scene file wins over --scene: the two are alternative
        // scene sources, and the file is the more specific request.
        exit_on_err(pollster::block_on(scenefile::run_scene_file(ew, eh, &out, path, knobs)));
        return;
    }
    match scene.as_str() {
        "m1" => exit_on_err(pollster::block_on(run(ew, eh, &out, knobs, &cfg.adapter_policy))),
        "ab1" => {
            let stage = match post_stage.as_str() {
                "lighting" => 1,
                "bloom" => 2,
                "grade" => 3,
                other => {
                    cli_error(&format!(
                        "unknown --post-stage: {other} (expected lighting, bloom or grade)"
                    ));
                }
            };
            exit_on_err(pollster::block_on(ab1::run_ab1(ew, eh, &out, &caption, stage, knobs)))
        }
        "dof-chart" => exit_on_err(pollster::block_on(dof::run_dof(ew, eh, &out, knobs))),
        "stroke-battery" => exit_on_err(battery::run_battery(&out)),
        other => {
            cli_error(&format!(
                "unknown scene: {other} (expected m1, ab1, dof-chart or stroke-battery)"
            ));
        }
    }
}

// The shared GPU primitives (instance / adapter / device /
// targets / readback) live in crate::gpucore since the Wave-2
// migration — every render entry point consumes them; there is
// no per-mode copy left in this file.

fn pack_f32(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn pack_u32(data: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn mat_bytes(m: &Mat4) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut k = 0;
    for col in 0..4 {
        for row in 0..4 {
            out[k..k + 4].copy_from_slice(&m[col][row].to_ne_bytes());
            k += 4;
        }
    }
    out
}

struct Uniforms {
    mvp: Mat4,
    model: Mat4,
}

fn uniform_bytes(u: &Uniforms) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mat_bytes(&u.mvp));
    out.extend_from_slice(&mat_bytes(&u.model));
    out
}

/// Vertex for the colour pipeline: position, normal, colour.
fn push_vert(v: &mut Vec<f32>, pos: [f32; 3], nrm: [f32; 3], col: [f32; 3]) {
    v.extend_from_slice(&pos);
    v.extend_from_slice(&nrm);
    v.extend_from_slice(&col);
}

/// A cube of half-extent `h` centred on the origin, 24 verts / 36 indices,
/// per-face colours. Windings verified so each triangle's right-hand-rule
/// normal matches its face normal (pipeline culls back faces).
fn cube(h: f32) -> (Vec<f32>, Vec<u32>) {
    // (normal, colour, four corners CCW seen from outside)
    let faces: [([f32; 3], [f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [0.85, 0.33, 0.30], [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]]), // +Z front
        ([0.0, 0.0, -1.0], [0.30, 0.45, 0.85], [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]]), // -Z back
        ([1.0, 0.0, 0.0], [0.40, 0.75, 0.45], [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]]), // +X right
        ([-1.0, 0.0, 0.0], [0.65, 0.45, 0.80], [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]]), // -X left
        ([0.0, 1.0, 0.0], [0.95, 0.80, 0.35], [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]]), // +Y top
        ([0.0, -1.0, 0.0], [0.25, 0.25, 0.28], [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]]), // -Y bottom
    ];
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for (nrm, col, corners) in faces {
        let base = (verts.len() / 9) as u32;
        for c in corners {
            push_vert(&mut verts, c, nrm, col);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

/// Ground plane at y = 0 spanning [-s, s] in x/z, one colour.
fn ground(s: f32, col: [f32; 3]) -> (Vec<f32>, Vec<u32>) {
    let n = [0.0, 1.0, 0.0];
    let corners = [[-s, 0.0, s], [s, 0.0, s], [s, 0.0, -s], [-s, 0.0, -s]];
    let mut verts = Vec::new();
    for c in corners {
        push_vert(&mut verts, c, n, col);
    }
    (verts, vec![0, 1, 2, 0, 2, 3])
}

/// The character-card texture, painted procedurally on the CPU (the M1
/// stand-in for a CanvasTexture): cream card, ink border, eyes, smile.
fn character_texture(size: u32) -> Vec<u8> {
    let mut px = vec![0u8; (size * size * 4) as usize];
    let s = size as i32;
    let set = |px: &mut Vec<u8>, x: i32, y: i32, rgba: [u8; 4]| {
        if x >= 0 && y >= 0 && x < s && y < s {
            let k = ((y * s + x) * 4) as usize;
            px[k..k + 4].copy_from_slice(&rgba);
        }
    };
    let disc = |cx: i32, cy: i32, r: i32| -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                    out.push((x, y));
                }
            }
        }
        out
    };
    let ink = [35, 35, 40, 255];
    for y in 0..s {
        for x in 0..s {
            set(&mut px, x, y, [245, 238, 220, 255]);
            if x < 8 || y < 8 || x >= s - 8 || y >= s - 8 {
                set(&mut px, x, y, ink);
            }
        }
    }
    for (x, y) in disc(88, 100, 15) {
        set(&mut px, x, y, ink);
    }
    for (x, y) in disc(168, 100, 15) {
        set(&mut px, x, y, ink);
    }
    for (x, y) in disc(70, 142, 13) {
        set(&mut px, x, y, [232, 178, 178, 255]);
    }
    for (x, y) in disc(186, 142, 13) {
        set(&mut px, x, y, [232, 178, 178, 255]);
    }
    // Smile: lower arc of a ring centred (128, 148), radius ~36, width ~7.
    for y in 0..s {
        for x in 0..s {
            let d = ((x - 128) as f32).powi(2) + ((y - 148) as f32).powi(2);
            let r = d.sqrt();
            if (32.0..=39.0).contains(&r) && y > 148 {
                set(&mut px, x, y, ink);
            }
        }
    }
    px
}

async fn run(width: u32, height: u32, out: &str, knobs: Knobs, adapter_policy: &str) -> errors::TaleResult<()> {
    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
    let instance = crate::gpucore::new_instance();
    let policy = crate::adapter::policy_from(Some(adapter_policy), None);
    let adapter = crate::gpucore::request_adapter_with_policy(&instance, &policy).await?;
    let info = adapter.get_info();
    println!("{}", crate::adapter::report_line(&crate::adapter::describe(&info)));

    // Downlevel limits are this site's deliberate divergence
    // (GPU-CORE-W07 s1/s4): the M1 smoke scene's textures are
    // tiny; every other site takes standard limits because the
    // real character canvas (2560x2816) exceeds the 2048 cap.
    let (device, queue) = crate::gpucore::request_device(
        &adapter,
        "talerender device",
        wgpu::Limits::downlevel_defaults(),
    )
    .await?;

    // ---- camera & transforms -------------------------------------------
    let aspect = width as f32 / height as f32;
    let proj = math::perspective(36f32.to_radians(), aspect, 0.1, 100.0);
    // Portrait framing check: at 36 deg vertical FOV and aspect 0.5625,
    // the horizontal half-angle only fits ~0.183 x distance of half-width,
    // so the staging (cube left edge ~ -1.9, card right edge ~ +2.2 in
    // world units) needs ~11 m of camera distance. The first CI frame
    // (eye z 6.8) clipped both subjects — measured, not guessed.
    let eye = [3.4, 2.4, 11.0];
    let view = math::look_at(eye, [0.2, 0.9, 0.0], [0.0, 1.0, 0.0]);
    let vp = math::mul(&proj, &view);

    let cube_model = math::mul(
        &math::translate(-1.0, 0.75, 0.0),
        &math::rotate_y(0.6),
    );
    let char_pos = [1.6, 0.0, 0.2];
    let yaw = (eye[0] - char_pos[0]).atan2(eye[2] - char_pos[2]);
    let char_model = math::mul(&math::translate(char_pos[0], char_pos[1], char_pos[2]), &math::rotate_y(yaw));

    let make_uniform = |model: &Mat4| -> wgpu::Buffer {
        let u = Uniforms {
            mvp: math::mul(&vp, model),
            model: *model,
        };
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, 128); // VRAM ledger (W84 §5.2)
        buf.also_write(&queue, &u)
    };

    // ---- colour pipeline ------------------------------------------------
    let shader3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("geometry shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader3d.wgsl"))),
    });
    let bgl3d = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let layout3d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("colour pipeline layout"),
        bind_group_layouts: &[Some(&bgl3d)],
        immediate_size: 0,
    });
    let colour_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
    ];
    let pipeline3d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { cache: None,
        label: Some("colour pipeline"),
        layout: Some(&layout3d),
        vertex: wgpu::VertexState {
            module: &shader3d,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &colour_attrs })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader3d,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    });

    // ---- textured pipeline ----------------------------------------------
    let shader_tex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("textured shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_tex.wgsl"))),
    });
    let bgl_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("textured layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let layout_tex = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("textured pipeline layout"),
        bind_group_layouts: &[Some(&bgl_tex)],
        immediate_size: 0,
    });
    let tex_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
    ];
    let pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { cache: None,
        label: Some("textured pipeline"),
        layout: Some(&layout_tex),
        vertex: wgpu::VertexState {
            module: &shader_tex,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tex_attrs })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_tex,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    });

    // ---- geometry buffers -----------------------------------------------
    let make_vb = |verts: &[f32], idx: &[u32]| -> (wgpu::Buffer, wgpu::Buffer) {
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            size: (verts.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, (verts.len() * 4) as u64); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&vb, 0, &pack_f32(verts));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index buffer"),
            size: (idx.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        crate::gpucore::mem_alloc_buffer(crate::gpucore::MemClass::G, (idx.len() * 4) as u64); // VRAM ledger (W84 §5.2)
        queue.write_buffer(&ib, 0, &pack_u32(idx));
        (vb, ib)
    };
    let (cube_verts, cube_idx) = cube(0.75);
    let (cube_vb, cube_ib) = make_vb(&cube_verts, &cube_idx);
    let (gnd_verts, gnd_idx) = ground(9.0, [0.22, 0.38, 0.22]);
    let (gnd_vb, gnd_ib) = make_vb(&gnd_verts, &gnd_idx);

    // Character quad: local XY plane, 1.2 wide x 1.8 tall, base at origin.
    let quad_verts: [f32; 20] = [
        -0.6, 0.0, 0.0, 0.0, 1.0, // BL
        0.6, 0.0, 0.0, 1.0, 1.0, // BR
        0.6, 1.8, 0.0, 1.0, 0.0, // TR
        -0.6, 1.8, 0.0, 0.0, 0.0, // TL
    ];
    let quad_idx: [u32; 6] = [0, 1, 2, 0, 2, 3];
    let (quad_vb, quad_ib) = make_vb(&quad_verts, &quad_idx);

    // ---- character texture ----------------------------------------------
    let tex_size = 256u32;
    let char_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("character texture"),
        size: wgpu::Extent3d { width: tex_size, height: tex_size, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::E, crate::gpucore::tex_bytes(tex_size, tex_size, wgpu::TextureFormat::Rgba8UnormSrgb, 1)); // VRAM ledger (W84 §5.2)
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &char_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &character_texture(tex_size),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(tex_size * 4),
            rows_per_image: Some(tex_size),
        },
        wgpu::Extent3d { width: tex_size, height: tex_size, depth_or_array_layers: 1 },
    );
    let char_view = char_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("linear sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // ---- bind groups ------------------------------------------------------
    let bind3d = |buf: &wgpu::Buffer| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("colour bind group"),
            layout: &bgl3d,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        })
    };
    let ground_uniform = make_uniform(&math::identity());
    let cube_uniform = make_uniform(&cube_model);
    let char_uniform = make_uniform(&char_model);
    let ground_bg = bind3d(&ground_uniform);
    let cube_bg = bind3d(&cube_uniform);
    let char_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("character bind group"),
        layout: &bgl_tex,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: char_uniform.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&char_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
    });

    // ---- frame target + depth --------------------------------------------
    let (texture, view_tex) = crate::gpucore::make_target_viewed(
        &device,
        width,
        height,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        1,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        "frame target",
    );
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Rgba8UnormSrgb, 1)); // VRAM ledger (W84 §5.2)
    let (_depth, depth_view) = crate::gpucore::make_target_viewed(
        &device,
        width,
        height,
        wgpu::TextureFormat::Depth32Float,
        1,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        "depth target",
    );
    crate::gpucore::mem_alloc_texture(crate::gpucore::MemClass::A, crate::gpucore::tex_bytes(width, height, wgpu::TextureFormat::Depth32Float, 1)); // VRAM ledger (W84 §5.2)

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { depth_slice: None,
                view: &view_tex,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.10, g: 0.16, b: 0.30, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline3d);
        pass.set_bind_group(0, &ground_bg, &[]);
        pass.set_vertex_buffer(0, gnd_vb.slice(..));
        pass.set_index_buffer(gnd_ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..gnd_idx.len() as u32, 0, 0..1);
        pass.set_bind_group(0, &cube_bg, &[]);
        pass.set_vertex_buffer(0, cube_vb.slice(..));
        pass.set_index_buffer(cube_ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..cube_idx.len() as u32, 0, 0..1);
        pass.set_pipeline(&pipeline_tex);
        pass.set_bind_group(0, &char_bg, &[]);
        pass.set_vertex_buffer(0, quad_vb.slice(..));
        pass.set_index_buffer(quad_ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..quad_idx.len() as u32, 0, 0..1);
    }
    let rb = crate::gpucore::Readback::record(&device, &mut encoder, &texture, width, height, "readback buffer");
    queue.submit(Some(encoder.finish()));
    let pixels = rb.collect(&device)?;

    let img = image::RgbaImage::from_raw(width, height, pixels).expect("pixel buffer size mismatch");
    img.save(out).map_err(|e| errors::TaleError::image_io(out, e))?;
    println!("wrote {out} ({width}x{height}){tag}");
    // VRAM ledger (W84 §5.3): its own line, after the wrote line.
    // WP2 (CR-B1-08): the mem-report companion block follows it.
    crate::memreport_print::print_full_block_for_adapter(&adapter);
    Ok(())
}

// Small helper so `make_uniform` above reads linearly: creates the buffer
// and immediately queues the initial contents.
trait AlsoWrite {
    fn also_write(self, queue: &wgpu::Queue, u: &Uniforms) -> wgpu::Buffer;
}

impl AlsoWrite for wgpu::Buffer {
    fn also_write(self, queue: &wgpu::Queue, u: &Uniforms) -> wgpu::Buffer {
        queue.write_buffer(&self, 0, &uniform_bytes(u));
        self
    }
}
