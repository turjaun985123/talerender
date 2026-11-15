/* DELIVERY NOTE — TaleRender V2, Batch 1, Seat 10 (WP13: CLI + configuration)
 *
 * FILES CREATED (new files only; no existing file was edited):
 *   src/config.rs — this file, the ONLY deliverable of this seat.
 *
 * CONTRACT POINTS IMPLEMENTED:
 *   - V2-CONTRACTS.md §3 C8: one resolved `Config` with precedence
 *     CLI arg > env (TALERENDER_*) > config file (talerender.toml,
 *     hand-parsed `key = value`, no new deps) > default.
 *   - V2-CONTRACTS.md §1: seat writes a NEW file only; std only;
 *     the lead owns src/main.rs and performs the wiring below.
 *   - V2 master prompt §17: the v1.0.1 CLI contract is NOT broken —
 *     this module feeds main.rs, never replaces it. -h/--help,
 *     -V/--version, panic-free errors and the knob flags with
 *     [DRAFT] tagging all keep their existing behaviour; nothing
 *     here parses argv and nothing here exits the process.
 *   - Seat charter field set (authoritative for this seat):
 *       CliOverrides { mem_profile, adapter_policy, output_mode,
 *                      perf_record_dir: Option<PathBuf> }
 *       Config       { mem_profile: String, adapter_policy: String,
 *                      output_mode: String,
 *                      perf_record_dir: Option<PathBuf> }
 *     Defaults: mem_profile "balanced", adapter_policy "auto",
 *     output_mode "png", perf_record_dir None.
 *   - Env names: TALERENDER_MEM_PROFILE, TALERENDER_ADAPTER_POLICY,
 *     TALERENDER_OUTPUT_MODE, TALERENDER_PERF_RECORD_DIR.
 *   - Config-file keys: mem_profile, adapter_policy, output_mode,
 *     perf_record_dir (exactly the field names).
 *   - Validation (an invalid value at ANY layer is ignored, with a
 *     warning, and resolution falls through to the next layer):
 *       mem_profile    ∈ {low-memory, balanced, high-performance}
 *       adapter_policy ∈ {auto, hardware-only, software-ok}
 *                        or named:<name> (name non-empty)
 *       output_mode    ∈ {png, stream, auto}
 *       perf_record_dir: any non-empty path
 *     Values are case-sensitive and whitespace-trimmed per layer.
 *   - `resolve(cli, env, file_text)` takes the environment as an
 *     injected closure so tests never touch the process env.
 *     `resolve_with_report` additionally returns the Vec<String> of
 *     ignored-invalid warnings. `resolve_from_process(cli,
 *     config_path)` wires std::env + the file on disk (see its doc
 *     comment for the missing/unreadable-file behaviour), and
 *     `resolve_from_process_with_report` is the same with warnings.
 *
 * WIRING THE LEAD MUST DO IN src/main.rs (exact):
 *   1. Module line — insert after `mod composer;` (alphabetical
 *      slot in the existing mod list):
 *          mod config;
 *   2. Declarations — with the other parse state, before the
 *      `while i < args.len()` loop:
 *          let mut cli_overrides = config::CliOverrides::default();
 *          let mut config_path: Option<String> = None;
 *   3. Arg-loop arms — add alongside the existing arms (same
 *      `value()` pattern; shown for the "--draft" anchor, insert
 *      before or after it):
 *          "--mem-profile" => {
 *              i += 1;
 *              cli_overrides.mem_profile =
 *                  Some(value(&args, i, "--mem-profile"));
 *          }
 *          "--adapter-policy" => {
 *              i += 1;
 *              cli_overrides.adapter_policy =
 *                  Some(value(&args, i, "--adapter-policy"));
 *          }
 *          "--output-mode" => {
 *              i += 1;
 *              cli_overrides.output_mode =
 *                  Some(value(&args, i, "--output-mode"));
 *          }
 *          "--perf-record-dir" => {
 *              i += 1;
 *              cli_overrides.perf_record_dir =
 *                  Some(std::path::PathBuf::from(
 *                      value(&args, i, "--perf-record-dir")));
 *          }
 *          "--config" => {
 *              i += 1;
 *              config_path = Some(value(&args, i, "--config"));
 *          }
 *      DELIBERATE DIVERGENCE the lead must keep: an invalid VALUE
 *      for these four new flags must NOT go through cli_error /
 *      exit 2 the way --res/--aa/--shadow do. C8 fallthrough
 *      semantics require resolve() to see the raw string, ignore
 *      it with a warning, and fall through. Only a MISSING value
 *      errors, via the existing value() helper, as today.
 *   4. Resolution — immediately after the parse loop (the -h/-V
 *      early returns above the loop still win, untouched):
 *          let (cfg, config_warnings) =
 *              config::resolve_from_process_with_report(
 *                  &cli_overrides,
 *                  config_path.as_deref().map(std::path::Path::new),
 *              );
 *          for w in &config_warnings {
 *              eprintln!("{w}");
 *          }
 *      Consumers (lead wires as those seats land; until then name
 *      it `_cfg` or print it on the startup line so it is not an
 *      unused binding):
 *        cfg.mem_profile     -> WP1.3 mempressure (C1): the string
 *                               is the profile name that module's
 *                               selection already speaks.
 *        cfg.adapter_policy  -> WP3 adapter (C7): raw string form
 *                               "auto" | "hardware-only" |
 *                               "software-ok" | "named:<name>",
 *                               converted by that module.
 *        cfg.output_mode     -> WP4 streammodes (C3, Batch 2):
 *                               "png" | "stream" | "auto".
 *        cfg.perf_record_dir -> WP6 perf lab record directory.
 *      This module deliberately stores STRINGS, not the consumer
 *      modules' enums: config is the interchange layer and no
 *      cross-seat dependency is created.
 *   5. HELP text — add to const HELP, OPTIONS section:
 *          --mem-profile <NAME>    Memory profile: low-memory |
 *                                  balanced | high-performance
 *                                  (default: balanced). Bounds
 *                                  caches/queues only — never
 *                                  quality, resolution or fps.
 *          --adapter-policy <POL>  Adapter policy: auto |
 *                                  hardware-only | software-ok |
 *                                  named:<NAME> (default: auto).
 *          --output-mode <MODE>    Output mode: png | stream |
 *                                  auto (default: png).
 *          --perf-record-dir <DIR> Directory for performance
 *                                  records (default: none).
 *          --config <PATH>         Config file (default:
 *                                  ./talerender.toml).
 *      and to the ENVIRONMENT section:
 *          TALERENDER_MEM_PROFILE / TALERENDER_ADAPTER_POLICY /
 *          TALERENDER_OUTPUT_MODE / TALERENDER_PERF_RECORD_DIR
 *                                  Environment overrides for the
 *                                  four settings above. Precedence:
 *                                  CLI flag > environment > config
 *                                  file > default. An invalid value
 *                                  at any layer is ignored (with a
 *                                  warning on stderr) and
 *                                  resolution falls through to the
 *                                  next layer.
 *
 * TESTS INCLUDED (#[cfg(test)] in this file, 27 tests):
 *   defaults; per-field precedence (CLI > env > file > default)
 *   incl. an 8-case mem_profile matrix; invalid-value fallthrough
 *   at one / two / all three layers with warning counts, fields
 *   and sources asserted; valid-CLI short-circuit (invalid lower
 *   layers never examined, no warnings); the exact env names
 *   asked, in field order; valid value sets for all three enums;
 *   case sensitivity; named:<name> acceptance, whitespace
 *   normalisation and empty-name rejection; perf_record_dir
 *   precedence + empty-env fallthrough; file-parser edge cases
 *   (comments, blank lines, whitespace/tabs, CRLF, unknown keys
 *   preserved by the parser but ignored by resolve, lines without
 *   '=', empty keys, empty values, first-'=' split, duplicate
 *   keys last-wins, inline '#' kept in the value); resolve() ==
 *   resolve_with_report().0.
 *
 * KNOWN LIMITATIONS:
 *   - C8 also names a "stream_fps override (none by default)"
 *     field. This seat's charter fixes the field list to the four
 *     fields above, so stream_fps is NOT in this module. If the
 *     lead rules it in, it is a purely additive fifth field
 *     (Option<u32> on CliOverrides, Option<u32> on Config,
 *     TALERENDER_STREAM_FPS, key stream_fps) — flagging for the
 *     lead's decision rather than silently widening the charter.
 *   - The config file is hand-parsed `key = value` ONLY (per C8:
 *     no new deps, no TOML parser). No sections, no quoting, no
 *     escapes. Inline comments are NOT stripped: '#' starts a
 *     comment only at the start of a (trimmed) line; a '#' inside
 *     a value is part of the value.
 *   - resolve_from_process: a missing, unreadable or non-UTF-8
 *     config file is indistinguishable from "no config file"
 *     (file_text = None) and produces no warning — resolution
 *     simply proceeds to defaults. The path itself is the only
 *     diagnostic surface; documented on the fn.
 *   - perf_record_dir is not checked for existence or writability:
 *     the directory may legitimately be created later by the perf
 *     tooling that consumes it.
 *   - No low-RAM auto-default: C1's "≤ 8 GiB ⇒ LowMemory when
 *     unset" rule belongs to WP1.3's mempressure module, which
 *     owns /proc/meminfo detection. This module always defaults
 *     mem_profile to "balanced" exactly as chartered; the lead
 *     should pass cfg.mem_profile into that module's selection
 *     rather than re-implementing the rule here.
 */

//! Configuration resolution (WP13; contract C8).
//!
//! One [`Config`] resolved from four layers, highest first:
//!
//! 1. CLI overrides ([`CliOverrides`], filled by `main.rs`),
//! 2. environment (`TALERENDER_*`, injected as a closure),
//! 3. config file (`talerender.toml`, hand-parsed `key = value`),
//! 4. built-in defaults.
//!
//! Validation is per-field and per-layer: a value that fails its
//! field's validation is *ignored* — resolution falls through to
//! the next layer for that field only, and
//! [`resolve_with_report`] records a warning naming the field,
//! the offending value and the layer it came from. Fields resolve
//! independently: an invalid `mem_profile` on the CLI does not
//! stop a valid CLI `output_mode` from winning.
//!
//! This module never reads argv, never exits and never touches
//! the process environment except inside
//! [`resolve_from_process`]. It stores plain strings: converting
//! `adapter_policy` / `output_mode` / `mem_profile` into the
//! consuming modules' enums is those modules' job (C1/C3/C7), so
//! config stays dependency-free within the crate.

use std::path::{Path, PathBuf};

/// Default config file name, looked up in the working directory
/// when [`resolve_from_process`] is given no explicit path (C8).
pub(crate) const DEFAULT_CONFIG_FILE: &str = "talerender.toml";

/// Environment variable names, one per field (C8: `TALERENDER_*`).
pub(crate) const ENV_MEM_PROFILE: &str = "TALERENDER_MEM_PROFILE";
pub(crate) const ENV_ADAPTER_POLICY: &str = "TALERENDER_ADAPTER_POLICY";
pub(crate) const ENV_OUTPUT_MODE: &str = "TALERENDER_OUTPUT_MODE";
pub(crate) const ENV_PERF_RECORD_DIR: &str = "TALERENDER_PERF_RECORD_DIR";

/// Built-in defaults (layer 4).
pub(crate) const DEFAULT_MEM_PROFILE: &str = "balanced";
pub(crate) const DEFAULT_ADAPTER_POLICY: &str = "auto";
pub(crate) const DEFAULT_OUTPUT_MODE: &str = "png";

/// The valid spellings for the three enumerated fields. Values
/// are case-sensitive: `"Balanced"` is invalid, not a synonym.
pub(crate) const VALID_MEM_PROFILES: [&str; 3] =
    ["low-memory", "balanced", "high-performance"];
pub(crate) const VALID_ADAPTER_POLICIES: [&str; 3] =
    ["auto", "hardware-only", "software-ok"];
pub(crate) const VALID_OUTPUT_MODES: [&str; 3] = ["png", "stream", "auto"];

const MEM_PROFILE_EXPECTATION: &str =
    "one of: low-memory, balanced, high-performance";
const ADAPTER_POLICY_EXPECTATION: &str =
    "one of: auto, hardware-only, software-ok, or named:<name>";
const OUTPUT_MODE_EXPECTATION: &str = "one of: png, stream, auto";
const PERF_RECORD_DIR_EXPECTATION: &str = "a non-empty path";

/// Layer 1: what the command line actually gave. `None` means
/// "not given" for every field — there is no way to express
/// "given but empty" distinctly, and an empty string is simply
/// an invalid value that falls through like any other.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CliOverrides {
    pub mem_profile: Option<String>,
    pub adapter_policy: Option<String>,
    pub output_mode: Option<String>,
    pub perf_record_dir: Option<PathBuf>,
}

/// The resolved configuration: every field final. String fields
/// are guaranteed members of their valid sets (or, for
/// `adapter_policy`, a normalised `named:<name>`); see the
/// validators below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub mem_profile: String,
    pub adapter_policy: String,
    pub output_mode: String,
    pub perf_record_dir: Option<PathBuf>,
}

impl Default for Config {
    /// Layer 4 alone: the defaults every field falls back to.
    fn default() -> Self {
        Config {
            mem_profile: DEFAULT_MEM_PROFILE.to_string(),
            adapter_policy: DEFAULT_ADAPTER_POLICY.to_string(),
            output_mode: DEFAULT_OUTPUT_MODE.to_string(),
            perf_record_dir: None,
        }
    }
}

/// Parse config-file text into `(key, value)` pairs, in file
/// order. The format is deliberately minimal (C8: hand-parsed,
/// no new deps):
///
/// - one `key = value` pair per line, split at the FIRST `=`
///   (a value may itself contain `=`);
/// - keys and values are whitespace-trimmed;
/// - blank lines and lines whose first non-whitespace character
///   is `#` are skipped — inline comments are NOT stripped, a
///   `#` inside a value is part of the value;
/// - lines without `=`, and lines with an empty key, are skipped;
/// - unknown keys are PRESERVED in the output (callers other
///   than [`resolve`] may care); `resolve` ignores them;
/// - duplicate keys are all preserved; [`lookup`] — and therefore
///   `resolve` — takes the LAST occurrence (later lines override
///   earlier ones within the file).
pub(crate) fn parse_config_file(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim();
        if key.is_empty() {
            continue;
        }
        let value = line[eq + 1..].trim();
        out.push((key.to_string(), value.to_string()));
    }
    out
}

/// Look up `key` in parsed config pairs: the LAST occurrence
/// wins, matching the "later lines override" rule of
/// [`parse_config_file`]. Returns the value exactly as parsed
/// (already whitespace-trimmed by the parser).
pub(crate) fn lookup<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .rev()
        .find(|(k, _)| k.as_str() == key)
        .map(|(_, v)| v.as_str())
}

/// Validate + normalise a `mem_profile` candidate: the trimmed
/// value itself if it is in the valid set, else `None`.
fn normalize_mem_profile(raw: &str) -> Option<String> {
    let v = raw.trim();
    VALID_MEM_PROFILES.contains(&v).then(|| v.to_string())
}

/// Validate + normalise an `output_mode` candidate.
fn normalize_output_mode(raw: &str) -> Option<String> {
    let v = raw.trim();
    VALID_OUTPUT_MODES.contains(&v).then(|| v.to_string())
}

/// Validate + normalise an `adapter_policy` candidate. Besides
/// the three fixed spellings, `named:<name>` selects an adapter
/// by name: everything after the prefix, trimmed, must be
/// non-empty (adapter names legitimately contain spaces, e.g.
/// `named:NVIDIA GeForce RTX`), and the stored form is the
/// normalised `named:<trimmed name>`.
fn normalize_adapter_policy(raw: &str) -> Option<String> {
    let v = raw.trim();
    if VALID_ADAPTER_POLICIES.contains(&v) {
        return Some(v.to_string());
    }
    let name = v.strip_prefix("named:")?.trim();
    if name.is_empty() {
        None
    } else {
        Some(format!("named:{name}"))
    }
}

/// One ignored-invalid warning. The wording is stable and
/// greppable (`config: ignoring invalid ...`) and names the
/// field, the offending value, the layer and what was expected.
fn invalid_warning(field: &str, value: &str, source: &str, expectation: &str) -> String {
    format!(
        "config: ignoring invalid {field} value '{value}' from {source} \
         (expected {expectation}); falling through to the next layer"
    )
}

/// Resolve one validated string field across the CLI / env /
/// file layers, in that order. The first layer yielding a valid
/// value wins and lower layers are never examined (so they can
/// produce no warnings); each examined-but-invalid value appends
/// exactly one warning and resolution falls through.
#[allow(clippy::too_many_arguments)]
fn pick_string(
    field: &str,
    cli_value: Option<&str>,
    env_name: &str,
    env: &dyn Fn(&str) -> Option<String>,
    file_pairs: &[(String, String)],
    default: &str,
    expectation: &str,
    normalize: fn(&str) -> Option<String>,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(raw) = cli_value {
        match normalize(raw) {
            Some(v) => return v,
            None => warnings.push(invalid_warning(
                field,
                raw,
                "cli override",
                expectation,
            )),
        }
    }
    if let Some(raw) = env(env_name) {
        match normalize(&raw) {
            Some(v) => return v,
            None => warnings.push(invalid_warning(
                field,
                &raw,
                &format!("env {env_name}"),
                expectation,
            )),
        }
    }
    if let Some(raw) = lookup(file_pairs, field) {
        match normalize(raw) {
            Some(v) => return v,
            None => warnings.push(invalid_warning(
                field,
                raw,
                "config file",
                expectation,
            )),
        }
    }
    default.to_string()
}

/// Resolve `perf_record_dir` across the layers. Any non-empty
/// path is valid; an empty value (empty `PathBuf` on the CLI, or
/// an empty/whitespace string from env or file) is invalid and
/// falls through with a warning, like the string fields. The
/// path is NOT checked for existence — see the delivery note.
fn pick_path(
    cli_value: Option<&PathBuf>,
    env_name: &str,
    env: &dyn Fn(&str) -> Option<String>,
    file_pairs: &[(String, String)],
    warnings: &mut Vec<String>,
) -> Option<PathBuf> {
    if let Some(p) = cli_value {
        if !p.as_os_str().is_empty() {
            return Some(p.clone());
        }
        warnings.push(invalid_warning(
            "perf_record_dir",
            "",
            "cli override",
            PERF_RECORD_DIR_EXPECTATION,
        ));
    }
    if let Some(raw) = env(env_name) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
        warnings.push(invalid_warning(
            "perf_record_dir",
            &raw,
            &format!("env {env_name}"),
            PERF_RECORD_DIR_EXPECTATION,
        ));
    }
    if let Some(raw) = lookup(file_pairs, "perf_record_dir") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
        warnings.push(invalid_warning(
            "perf_record_dir",
            raw,
            "config file",
            PERF_RECORD_DIR_EXPECTATION,
        ));
    }
    None
}

/// Resolve the configuration (precedence: CLI > env > config
/// file > default), discarding the ignored-invalid warnings.
/// Callers that can surface diagnostics should use
/// [`resolve_with_report`] instead.
///
/// `env` is the environment as a closure — `|k| std::env::var(k)
/// .ok()` for the real thing, a fixed map in tests — so no test
/// ever touches the process environment. `file_text` is the
/// config file's contents, or `None` when there is no file.
pub(crate) fn resolve(
    cli: &CliOverrides,
    env: &dyn Fn(&str) -> Option<String>,
    file_text: Option<&str>,
) -> Config {
    resolve_with_report(cli, env, file_text).0
}

/// [`resolve`], plus the warnings for every invalid value that
/// was ignored along the way (in field order: mem_profile,
/// adapter_policy, output_mode, perf_record_dir; within a field,
/// in layer order CLI, env, file). An empty warning vec means
/// every examined value was valid or absent.
pub(crate) fn resolve_with_report(
    cli: &CliOverrides,
    env: &dyn Fn(&str) -> Option<String>,
    file_text: Option<&str>,
) -> (Config, Vec<String>) {
    let file_pairs = file_text.map(parse_config_file).unwrap_or_default();
    let mut warnings = Vec::new();
    let mem_profile = pick_string(
        "mem_profile",
        cli.mem_profile.as_deref(),
        ENV_MEM_PROFILE,
        env,
        &file_pairs,
        DEFAULT_MEM_PROFILE,
        MEM_PROFILE_EXPECTATION,
        normalize_mem_profile,
        &mut warnings,
    );
    let adapter_policy = pick_string(
        "adapter_policy",
        cli.adapter_policy.as_deref(),
        ENV_ADAPTER_POLICY,
        env,
        &file_pairs,
        DEFAULT_ADAPTER_POLICY,
        ADAPTER_POLICY_EXPECTATION,
        normalize_adapter_policy,
        &mut warnings,
    );
    let output_mode = pick_string(
        "output_mode",
        cli.output_mode.as_deref(),
        ENV_OUTPUT_MODE,
        env,
        &file_pairs,
        DEFAULT_OUTPUT_MODE,
        OUTPUT_MODE_EXPECTATION,
        normalize_output_mode,
        &mut warnings,
    );
    let perf_record_dir = pick_path(
        cli.perf_record_dir.as_ref(),
        ENV_PERF_RECORD_DIR,
        env,
        &file_pairs,
        &mut warnings,
    );
    (
        Config {
            mem_profile,
            adapter_policy,
            output_mode,
            perf_record_dir,
        },
        warnings,
    )
}

/// Read the config file for [`resolve_from_process_with_report`].
/// `None` path means `./talerender.toml` ([`DEFAULT_CONFIG_FILE`]).
/// A missing file, an unreadable file and a non-UTF-8 file all
/// yield `None` — i.e. "no config-file layer" — by design: the
/// config file is optional, and this helper is the one place
/// that behaviour lives. It is documented here because the
/// `Config`-only entry point below cannot distinguish "no file"
/// from "file could not be read"; neither produces a warning.
fn read_config_text(config_path: Option<&Path>) -> Option<String> {
    let path = config_path.unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE));
    std::fs::read_to_string(path).ok()
}

/// Resolve against the real process environment and the config
/// file on disk: the production entry point `main.rs` calls.
/// `config_path` is the `--config` value; `None` reads
/// `./talerender.toml` if it exists. A non-Unicode environment
/// value is treated as unset (`std::env::var` errors). Missing
/// or unreadable config file = no file layer (see
/// [`read_config_text`]). Warnings are discarded; use
/// [`resolve_from_process_with_report`] to surface them.
pub(crate) fn resolve_from_process(
    cli: &CliOverrides,
    config_path: Option<&Path>,
) -> Config {
    resolve_from_process_with_report(cli, config_path).0
}

/// [`resolve_from_process`], plus the ignored-invalid warnings
/// (which `main.rs` prints to stderr — see the delivery note).
pub(crate) fn resolve_from_process_with_report(
    cli: &CliOverrides,
    config_path: Option<&Path>,
) -> (Config, Vec<String>) {
    let file_text = read_config_text(config_path);
    resolve_with_report(
        cli,
        &|key: &str| std::env::var(key).ok(),
        file_text.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Build a CliOverrides from string slices (perf as a path).
    fn cli(
        mem: Option<&str>,
        adapter: Option<&str>,
        output: Option<&str>,
        perf: Option<&str>,
    ) -> CliOverrides {
        CliOverrides {
            mem_profile: mem.map(|s| s.to_string()),
            adapter_policy: adapter.map(|s| s.to_string()),
            output_mode: output.map(|s| s.to_string()),
            perf_record_dir: perf.map(PathBuf::from),
        }
    }

    /// An env closure backed by a fixed map: the tests never
    /// touch the process environment.
    fn env_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    // ---- defaults -------------------------------------------------

    #[test]
    fn defaults_when_nothing_is_given_anywhere() {
        let cfg = resolve(&CliOverrides::default(), &no_env, None);
        assert_eq!(cfg, Config::default());
        assert_eq!(cfg.mem_profile, "balanced");
        assert_eq!(cfg.adapter_policy, "auto");
        assert_eq!(cfg.output_mode, "png");
        assert_eq!(cfg.perf_record_dir, None);
        let (cfg2, warnings) =
            resolve_with_report(&CliOverrides::default(), &no_env, None);
        assert_eq!(cfg2, cfg);
        assert!(warnings.is_empty());
    }

    // ---- precedence ------------------------------------------------

    #[test]
    fn mem_profile_precedence_matrix() {
        // (cli, env, file, expected): every layer combination.
        let cases: [(Option<&str>, Option<&str>, Option<&str>, &str); 8] = [
            (Some("low-memory"), Some("balanced"), Some("high-performance"), "low-memory"),
            (Some("low-memory"), Some("balanced"), None, "low-memory"),
            (Some("low-memory"), None, Some("high-performance"), "low-memory"),
            (Some("balanced"), Some("low-memory"), Some("low-memory"), "balanced"),
            (None, Some("low-memory"), Some("high-performance"), "low-memory"),
            (None, Some("high-performance"), None, "high-performance"),
            (None, None, Some("low-memory"), "low-memory"),
            (None, None, None, "balanced"),
        ];
        for (cli_v, env_v, file_v, want) in cases {
            let c = cli(cli_v, None, None, None);
            let mut pairs = Vec::new();
            if let Some(v) = env_v {
                pairs.push((ENV_MEM_PROFILE, v));
            }
            let m = env_map(&pairs);
            let env = |k: &str| m.get(k).cloned();
            let file = file_v.map(|v| format!("mem_profile = {v}"));
            let got = resolve(&c, &env, file.as_deref());
            assert_eq!(
                got.mem_profile, want,
                "cli={cli_v:?} env={env_v:?} file={file_v:?}"
            );
        }
    }

    #[test]
    fn cli_beats_env_and_file_for_every_field() {
        let c = cli(
            Some("high-performance"),
            Some("hardware-only"),
            Some("stream"),
            Some("cli-records"),
        );
        let m = env_map(&[
            (ENV_MEM_PROFILE, "low-memory"),
            (ENV_ADAPTER_POLICY, "software-ok"),
            (ENV_OUTPUT_MODE, "auto"),
            (ENV_PERF_RECORD_DIR, "env-records"),
        ]);
        let env = |k: &str| m.get(k).cloned();
        let file = "mem_profile = low-memory\n\
                    adapter_policy = software-ok\n\
                    output_mode = auto\n\
                    perf_record_dir = file-records\n";
        let (cfg, warnings) = resolve_with_report(&c, &env, Some(file));
        assert_eq!(cfg.mem_profile, "high-performance");
        assert_eq!(cfg.adapter_policy, "hardware-only");
        assert_eq!(cfg.output_mode, "stream");
        assert_eq!(cfg.perf_record_dir, Some(PathBuf::from("cli-records")));
        assert!(warnings.is_empty());
    }

    #[test]
    fn env_beats_file_when_cli_absent() {
        let m = env_map(&[
            (ENV_MEM_PROFILE, "low-memory"),
            (ENV_ADAPTER_POLICY, "software-ok"),
            (ENV_OUTPUT_MODE, "auto"),
            (ENV_PERF_RECORD_DIR, "env-records"),
        ]);
        let env = |k: &str| m.get(k).cloned();
        let file = "mem_profile = high-performance\n\
                    adapter_policy = hardware-only\n\
                    output_mode = stream\n\
                    perf_record_dir = file-records\n";
        let cfg = resolve(&CliOverrides::default(), &env, Some(file));
        assert_eq!(cfg.mem_profile, "low-memory");
        assert_eq!(cfg.adapter_policy, "software-ok");
        assert_eq!(cfg.output_mode, "auto");
        assert_eq!(cfg.perf_record_dir, Some(PathBuf::from("env-records")));
    }

    #[test]
    fn file_beats_default_when_cli_and_env_absent() {
        let file = "mem_profile = high-performance\n\
                    adapter_policy = hardware-only\n\
                    output_mode = stream\n\
                    perf_record_dir = file-records\n";
        let cfg = resolve(&CliOverrides::default(), &no_env, Some(file));
        assert_eq!(cfg.mem_profile, "high-performance");
        assert_eq!(cfg.adapter_policy, "hardware-only");
        assert_eq!(cfg.output_mode, "stream");
        assert_eq!(cfg.perf_record_dir, Some(PathBuf::from("file-records")));
    }

    #[test]
    fn fields_resolve_independently() {
        // One field per layer: no layer "wins" wholesale.
        let c = cli(Some("low-memory"), None, None, None);
        let m = env_map(&[(ENV_OUTPUT_MODE, "stream")]);
        let env = |k: &str| m.get(k).cloned();
        let file = "adapter_policy = hardware-only\n";
        let cfg = resolve(&c, &env, Some(file));
        assert_eq!(cfg.mem_profile, "low-memory"); // cli
        assert_eq!(cfg.output_mode, "stream"); // env
        assert_eq!(cfg.adapter_policy, "hardware-only"); // file
        assert_eq!(cfg.perf_record_dir, None); // default
    }

    #[test]
    fn env_is_asked_for_exactly_the_documented_names_in_field_order() {
        use std::cell::RefCell;
        let asked: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let env = |k: &str| {
            asked.borrow_mut().push(k.to_string());
            None
        };
        let cfg = resolve(&CliOverrides::default(), &env, None);
        assert_eq!(cfg, Config::default());
        let asked_ref = asked.borrow();
        let names: Vec<&str> =
            asked_ref.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            names,
            vec![
                ENV_MEM_PROFILE,
                ENV_ADAPTER_POLICY,
                ENV_OUTPUT_MODE,
                ENV_PERF_RECORD_DIR
            ]
        );
    }

    // ---- invalid-value fallthrough ---------------------------------

    #[test]
    fn invalid_cli_value_falls_through_to_env() {
        let c = cli(Some("huge"), None, None, None);
        let m = env_map(&[(ENV_MEM_PROFILE, "low-memory")]);
        let env = |k: &str| m.get(k).cloned();
        let (cfg, warnings) = resolve_with_report(&c, &env, None);
        assert_eq!(cfg.mem_profile, "low-memory");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("mem_profile"));
        assert!(warnings[0].contains("'huge'"));
        assert!(warnings[0].contains("cli"));
    }

    #[test]
    fn invalid_cli_and_env_fall_through_to_file() {
        let c = cli(None, Some("gpu-please"), None, None);
        let m = env_map(&[(ENV_ADAPTER_POLICY, "any")]);
        let env = |k: &str| m.get(k).cloned();
        let (cfg, warnings) =
            resolve_with_report(&c, &env, Some("adapter_policy = software-ok"));
        assert_eq!(cfg.adapter_policy, "software-ok");
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("cli"));
        assert!(warnings[1].contains(ENV_ADAPTER_POLICY));
    }

    #[test]
    fn invalid_everywhere_falls_back_to_default_with_three_warnings() {
        let c = cli(None, None, Some("jpeg"), None);
        let m = env_map(&[(ENV_OUTPUT_MODE, "gif")]);
        let env = |k: &str| m.get(k).cloned();
        let (cfg, warnings) =
            resolve_with_report(&c, &env, Some("output_mode = bmp"));
        assert_eq!(cfg.output_mode, "png");
        assert_eq!(warnings.len(), 3);
        assert!(warnings.iter().all(|w| w.contains("output_mode")));
        assert!(warnings[0].contains("cli"));
        assert!(warnings[1].contains(ENV_OUTPUT_MODE));
        assert!(warnings[2].contains("config file"));
    }

    #[test]
    fn valid_cli_short_circuits_invalid_lower_layers_without_warnings() {
        // Lower layers are never examined once a layer wins, so
        // their invalid values cannot warn.
        let c = cli(Some("balanced"), None, None, None);
        let m = env_map(&[(ENV_MEM_PROFILE, "bogus")]);
        let env = |k: &str| m.get(k).cloned();
        let (cfg, warnings) =
            resolve_with_report(&c, &env, Some("mem_profile = also-bogus"));
        assert_eq!(cfg.mem_profile, "balanced");
        assert!(warnings.is_empty());
    }

    #[test]
    fn empty_string_values_are_invalid_and_fall_through() {
        let c = cli(Some(""), None, Some(""), None);
        let (cfg, warnings) = resolve_with_report(&c, &no_env, None);
        assert_eq!(cfg.mem_profile, "balanced");
        assert_eq!(cfg.output_mode, "png");
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn values_are_case_sensitive() {
        let c = cli(Some("Balanced"), Some("Auto"), Some("PNG"), None);
        let (cfg, warnings) = resolve_with_report(&c, &no_env, None);
        assert_eq!(cfg, Config::default());
        assert_eq!(warnings.len(), 3);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_at_every_layer() {
        let c = cli(Some("  low-memory  "), None, None, None);
        let m = env_map(&[(ENV_OUTPUT_MODE, " stream ")]);
        let env = |k: &str| m.get(k).cloned();
        let cfg = resolve(&c, &env, Some("adapter_policy =   hardware-only\t"));
        assert_eq!(cfg.mem_profile, "low-memory");
        assert_eq!(cfg.output_mode, "stream");
        assert_eq!(cfg.adapter_policy, "hardware-only");
    }

    // ---- valid value sets -------------------------------------------

    #[test]
    fn every_valid_mem_profile_and_output_mode_is_accepted() {
        for v in VALID_MEM_PROFILES {
            let cfg = resolve(&cli(Some(v), None, None, None), &no_env, None);
            assert_eq!(cfg.mem_profile, v);
        }
        for v in VALID_OUTPUT_MODES {
            let cfg = resolve(&cli(None, None, Some(v), None), &no_env, None);
            assert_eq!(cfg.output_mode, v);
        }
        for v in ["low", "performance", "medium"] {
            let (cfg, warnings) =
                resolve_with_report(&cli(Some(v), None, None, None), &no_env, None);
            assert_eq!(cfg.mem_profile, "balanced", "{v} must be invalid");
            assert_eq!(warnings.len(), 1);
        }
    }

    #[test]
    fn fixed_adapter_policies_are_accepted() {
        for v in VALID_ADAPTER_POLICIES {
            let cfg = resolve(&cli(None, Some(v), None, None), &no_env, None);
            assert_eq!(cfg.adapter_policy, v);
        }
        let (cfg, warnings) =
            resolve_with_report(&cli(None, Some("software"), None, None), &no_env, None);
        assert_eq!(cfg.adapter_policy, "auto");
        assert_eq!(warnings.len(), 1);
    }

    // ---- named: adapter policy ---------------------------------------

    #[test]
    fn named_adapter_policy_is_accepted_and_normalised() {
        let cfg = resolve(
            &cli(None, Some("named:NVIDIA GeForce RTX"), None, None),
            &no_env,
            None,
        );
        assert_eq!(cfg.adapter_policy, "named:NVIDIA GeForce RTX");
        // Whitespace around the name is normalised away.
        let cfg = resolve(
            &cli(None, Some("named:  Lavapipe  "), None, None),
            &no_env,
            None,
        );
        assert_eq!(cfg.adapter_policy, "named:Lavapipe");
        // Also reachable from env and file layers.
        let m = env_map(&[(ENV_ADAPTER_POLICY, "named:env-gpu")]);
        let env = |k: &str| m.get(k).cloned();
        let cfg = resolve(&CliOverrides::default(), &env, None);
        assert_eq!(cfg.adapter_policy, "named:env-gpu");
        let cfg = resolve(
            &CliOverrides::default(),
            &no_env,
            Some("adapter_policy = named:file-gpu"),
        );
        assert_eq!(cfg.adapter_policy, "named:file-gpu");
    }

    #[test]
    fn named_adapter_policy_with_empty_name_is_invalid() {
        for bad in ["named:", "named:   ", "named"] {
            let (cfg, warnings) = resolve_with_report(
                &cli(None, Some(bad), None, None),
                &no_env,
                None,
            );
            assert_eq!(cfg.adapter_policy, "auto", "{bad:?} must be invalid");
            assert_eq!(warnings.len(), 1, "{bad:?}");
        }
        // ... and an empty-named CLI value falls through to env.
        let m = env_map(&[(ENV_ADAPTER_POLICY, "hardware-only")]);
        let env = |k: &str| m.get(k).cloned();
        let (cfg, warnings) = resolve_with_report(
            &cli(None, Some("named:"), None, None),
            &env,
            None,
        );
        assert_eq!(cfg.adapter_policy, "hardware-only");
        assert_eq!(warnings.len(), 1);
    }

    // ---- perf_record_dir ----------------------------------------------

    #[test]
    fn perf_record_dir_precedence_and_empty_fallthrough() {
        // File only.
        let cfg = resolve(
            &CliOverrides::default(),
            &no_env,
            Some("perf_record_dir = records/run1"),
        );
        assert_eq!(cfg.perf_record_dir, Some(PathBuf::from("records/run1")));
        // Env beats file.
        let m = env_map(&[(ENV_PERF_RECORD_DIR, "env-records")]);
        let env = |k: &str| m.get(k).cloned();
        let cfg = resolve(
            &CliOverrides::default(),
            &env,
            Some("perf_record_dir = file-records"),
        );
        assert_eq!(cfg.perf_record_dir, Some(PathBuf::from("env-records")));
        // Empty env value is invalid: falls through to file, warns.
        let m = env_map(&[(ENV_PERF_RECORD_DIR, "   ")]);
        let env = |k: &str| m.get(k).cloned();
        let (cfg, warnings) = resolve_with_report(
            &CliOverrides::default(),
            &env,
            Some("perf_record_dir = file-records"),
        );
        assert_eq!(cfg.perf_record_dir, Some(PathBuf::from("file-records")));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("perf_record_dir"));
    }

    // ---- config-file parser -------------------------------------------

    #[test]
    fn parse_skips_comments_and_blank_lines() {
        let text = "# a comment\n\n   \n#mem_profile = low-memory\nmem_profile = balanced\n   # indented comment\n";
        assert_eq!(
            parse_config_file(text),
            vec![("mem_profile".to_string(), "balanced".to_string())]
        );
    }

    #[test]
    fn parse_trims_whitespace_and_handles_crlf_and_tabs() {
        let text = "  mem_profile\t=\tlow-memory  \r\noutput_mode = stream\r\n";
        assert_eq!(
            parse_config_file(text),
            vec![
                ("mem_profile".to_string(), "low-memory".to_string()),
                ("output_mode".to_string(), "stream".to_string()),
            ]
        );
    }

    #[test]
    fn parse_preserves_unknown_keys_but_resolve_ignores_them() {
        let text = "bogus_key = low-memory\nanother = 42\n";
        let pairs = parse_config_file(text);
        assert_eq!(pairs.len(), 2);
        assert_eq!(lookup(&pairs, "bogus_key"), Some("low-memory"));
        let (cfg, warnings) =
            resolve_with_report(&CliOverrides::default(), &no_env, Some(text));
        assert_eq!(cfg, Config::default());
        assert!(warnings.is_empty());
    }

    #[test]
    fn parse_skips_lines_without_equals_and_empty_keys() {
        let text = "just some words\n= orphan value\nmem_profile = balanced\n";
        assert_eq!(
            parse_config_file(text),
            vec![("mem_profile".to_string(), "balanced".to_string())]
        );
    }

    #[test]
    fn parse_splits_at_first_equals_and_keeps_hash_inside_values() {
        let pairs = parse_config_file("note = a=b # c\n");
        assert_eq!(
            pairs,
            vec![("note".to_string(), "a=b # c".to_string())]
        );
    }

    #[test]
    fn parse_duplicate_keys_last_one_wins() {
        let text = "mem_profile = low-memory\nmem_profile = high-performance\n";
        let pairs = parse_config_file(text);
        assert_eq!(pairs.len(), 2);
        assert_eq!(lookup(&pairs, "mem_profile"), Some("high-performance"));
        let cfg = resolve(&CliOverrides::default(), &no_env, Some(text));
        assert_eq!(cfg.mem_profile, "high-performance");
    }

    #[test]
    fn parse_empty_value_is_preserved_and_invalid_for_resolve() {
        let pairs = parse_config_file("mem_profile =\n");
        assert_eq!(pairs, vec![("mem_profile".to_string(), String::new())]);
        let (cfg, warnings) = resolve_with_report(
            &CliOverrides::default(),
            &no_env,
            Some("mem_profile =\n"),
        );
        assert_eq!(cfg.mem_profile, "balanced");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("config file"));
    }

    #[test]
    fn resolve_matches_resolve_with_report() {
        let c = cli(Some("nope"), None, Some("stream"), Some("rec"));
        let m = env_map(&[(ENV_MEM_PROFILE, "low-memory")]);
        let env = |k: &str| m.get(k).cloned();
        let file = Some("output_mode = auto");
        assert_eq!(
            resolve(&c, &env, file),
            resolve_with_report(&c, &env, file).0
        );
    }
}
