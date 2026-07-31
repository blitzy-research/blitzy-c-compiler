//! Oracle discovery and the capability record.
//!
//! This module is the differential conformance suite's single point of contact with the
//! machine it is running on. It locates the compiler under test, the native reference
//! compiler, the three cross reference drivers and the three emulated-execution runners;
//! it reads the entire environment-variable catalogue; and it emits a capability record
//! that states exactly which arms of which oracles will run.
//!
//! Two properties of that record matter more than any other:
//!
//! - **An absent oracle is reported, never silently skipped.** There is deliberately no
//!   "skip because unsupported" verdict anywhere in the suite. A tool the environment
//!   genuinely lacks degrades one specific oracle arm, that arm's cells are recorded as
//!   [`Verdict::Unavailable`](super::Verdict::Unavailable), and the gap appears loudly in
//!   the run summary. A modest environment can therefore never masquerade as a passing
//!   run.
//! - **A missing compiler under test is a hard failure, immediately.** Every other missing
//!   tool degrades gracefully; this one cannot, because without it there is nothing to
//!   test and a green result would be a lie.
//!
//! # Environment-variable catalogue
//!
//! Every variable has a safe default, so the suite runs correctly with none of them set.
//! All configuration reads live in this module so that no other part of the harness
//! consults the environment directly and no two modules can drift on a default.
//!
//! | Variable | Default | Purpose |
//! | --- | --- | --- |
//! | `BCC_BIN` | the Cargo-provided `CARGO_BIN_EXE_bcc` path | Override the compiler under test, for validating an externally built binary |
//! | `BCC_REF_CC` | probe `gcc`, then `cc`, then `clang` | Native reference compiler for oracle (a) and for both undefined-behaviour audit gates |
//! | `BCC_REF_CC_I686` | `i686-linux-gnu-gcc` | Oracle (a)'s i686 arm |
//! | `BCC_REF_CC_AARCH64` | `aarch64-linux-gnu-gcc` | Oracle (a)'s AArch64 arm |
//! | `BCC_REF_CC_RISCV64` | `riscv64-linux-gnu-gcc` | Oracle (a)'s RISC-V 64 arm |
//! | `BCC_QEMU_I386` | probe `qemu-i386`, then `qemu-i386-static` | i686 execution runner |
//! | `BCC_QEMU_AARCH64` | probe `qemu-aarch64`, then `qemu-aarch64-static` | AArch64 execution runner |
//! | `BCC_QEMU_RISCV64` | probe `qemu-riscv64`, then `qemu-riscv64-static` | RISC-V 64 execution runner |
//! | `BCC_CONFORMANCE_QUICK` | unset | Reduce the matrix to the native target at `-O0` and `-O2` only; always reported as reduced coverage |
//! | `BCC_CONFORMANCE_ONLY` | unset | Restrict the run to one `<area>/<program>` |
//! | `BCC_CONFORMANCE_STRICT` | unset | Treat an unavailable oracle as a failure; the intended continuous-integration setting |
//! | `BCC_CONFORMANCE_ALLOW_XPASS` | unset | Downgrade unexpected success from a failure to a warning during a marker-retirement window |
//! | `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` | unset | Explicitly acknowledge a reduced-oracle environment; the gap is still reported |
//! | `BCC_CONFORMANCE_TIMEOUT_SECS` | `30` | Per-cell execution budget |
//! | `BCC_CONFORMANCE_KEEP_WORK` | unset | Retain every cell workspace instead of removing it on success |
//!
//! A boolean variable is true when it is set, non-empty, and not the single character `0`.
//! Any other value — `1`, `true`, `yes`, `on`, or anything else — enables the setting, so a
//! maintainer who exports a flag at all gets the behaviour they were reaching for.
//!
//! # Graceful degradation
//!
//! | Missing component | Consequence |
//! | --- | --- |
//! | The compiler under test | Hard failure, immediately and loudly |
//! | Native reference compiler | Oracle (a) unavailable entirely; oracles (b) and (c) still run; affected cells are unavailable |
//! | One cross reference driver | Only that architecture's oracle (a) arm is unavailable |
//! | One emulator | That target drops out of oracle (b) *and* oracle (a)'s cross arm; the other three targets continue |
//! | Cross C runtime for one architecture | That architecture's link step fails and is reported as a link failure at environment scope, never as a compiler defect |
//! | `timeout` utility | Execution falls back to a standard-library watchdog thread; no behavioural change |
//! | `creduce` | Finding minimization becomes manual; findings remain complete |
//!
//! Availability is decided **per arm**, which is finer than the table's first two rows can
//! express, and never coarser. Each arm of oracle (a) is judged on the driver that targets it,
//! so an absent native compiler removes the native arm — and with it both audit gates, which use
//! that compiler — while a cross arm whose own driver is present continues to run. Reporting
//! those surviving arms as available is strictly more truthful than declaring the whole oracle
//! dead, and it can never mask a gap, because every arm that cannot be attempted is listed
//! individually by [`Capabilities::unavailable_oracle_arms`].
//!
//! Under [`strict`] every unavailable oracle becomes a failure. That is the intended
//! continuous-integration setting, because in continuous integration the toolchain is
//! installed deliberately, so a missing oracle indicates a broken workflow rather than a
//! modest machine. The policy is exposed here through [`unavailable_fails_run`] and
//! [`xpass_fails_run`] and applied by `classify.rs`, so there is exactly one place where it
//! can be changed.
//!
//! # Hermeticity
//!
//! This module **writes nothing**. It reads environment variables, walks `PATH`, calls
//! [`std::fs::metadata`] and spawns short, non-interactive banner probes that produce no
//! files. It opens no socket. Everything the wider harness writes lives beneath the Cargo
//! build directory, under the roots [`super::work_root`], [`super::report_root`] and
//! [`super::findings_root`].
//!
//! # Why binary-inspection tools are absent from the probe list
//!
//! `readelf`, `objdump` and `nm` are deliberately not probed and nothing is conditional on
//! them. The flag probe verifies object-versus-executable output, static linkage and debug
//! sections by reading ELF identification bytes directly with the standard library, so the
//! suite does not depend on a binary-inspection package being installed.
//!
//! # Compatibility
//!
//! Edition 2021, minimum supported Rust 1.70. [`std::sync::OnceLock`] is the newest
//! standard-library item used here, and it stabilized in 1.70. Every operation in this
//! module is a checked standard-library call — the language's escape hatch for
//! unchecked code is never reached for, and the keyword that opens it appears nowhere in
//! this file — and no third-party crate is used: the project forbids them absolutely and
//! admits no exceptions.

use std::env;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::{HarnessError, HarnessResult, OptLevel, Oracle, Target};

// ---------------------------------------------------------------------------
// Environment-variable names
// ---------------------------------------------------------------------------
//
// Every name the harness reads is a named constant so that a variable can be renamed in
// one edit and, more importantly, so that a diagnostic can quote the exact spelling a
// maintainer must export. A "not found" message that does not name its override variable
// forces the reader back into the source; every message below names it.

/// Overrides the compiler under test.
pub const VAR_BCC_BIN: &str = "BCC_BIN";

/// Overrides the native reference compiler used by oracle (a) and by both audit gates.
pub const VAR_REF_CC: &str = "BCC_REF_CC";

/// Overrides the i686 reference cross driver.
pub const VAR_REF_CC_I686: &str = "BCC_REF_CC_I686";

/// Overrides the AArch64 reference cross driver.
pub const VAR_REF_CC_AARCH64: &str = "BCC_REF_CC_AARCH64";

/// Overrides the RISC-V 64 reference cross driver.
pub const VAR_REF_CC_RISCV64: &str = "BCC_REF_CC_RISCV64";

/// Overrides the i686 execution runner.
pub const VAR_QEMU_I386: &str = "BCC_QEMU_I386";

/// Overrides the AArch64 execution runner.
pub const VAR_QEMU_AARCH64: &str = "BCC_QEMU_AARCH64";

/// Overrides the RISC-V 64 execution runner.
pub const VAR_QEMU_RISCV64: &str = "BCC_QEMU_RISCV64";

/// Reduces the matrix to the native target at `-O0` and `-O2`.
pub const VAR_QUICK: &str = "BCC_CONFORMANCE_QUICK";

/// Restricts the run to a single `<area>/<program>`.
pub const VAR_ONLY: &str = "BCC_CONFORMANCE_ONLY";

/// Escalates an unavailable oracle to a failure.
pub const VAR_STRICT: &str = "BCC_CONFORMANCE_STRICT";

/// Downgrades unexpected success from a failure to a warning.
pub const VAR_ALLOW_XPASS: &str = "BCC_CONFORMANCE_ALLOW_XPASS";

/// Acknowledges a reduced-oracle environment explicitly.
pub const VAR_ALLOW_MISSING_ORACLES: &str = "BCC_CONFORMANCE_ALLOW_MISSING_ORACLES";

/// Sets the per-cell execution budget in seconds.
pub const VAR_TIMEOUT_SECS: &str = "BCC_CONFORMANCE_TIMEOUT_SECS";

/// Retains every cell workspace instead of removing it on success.
pub const VAR_KEEP_WORK: &str = "BCC_CONFORMANCE_KEEP_WORK";

// ---------------------------------------------------------------------------
// Default tool names
// ---------------------------------------------------------------------------

/// Native reference compilers, probed in order.
///
/// `gcc` is the reference compiler of record. Its default language mode was measured as
/// gnu17, which already enables the GNU extensions the corpus exercises, and that is why no
/// standard-selection flag is ever passed to either compiler — bcc has none to match.
/// `clang` is probed last so that a second, genuinely independent oracle is available
/// wherever it is installed without a single line of code changing.
pub const DEFAULT_REF_CC: &[&str] = &["gcc", "cc", "clang"];

/// i686 reference cross drivers, probed in order.
///
/// A dedicated cross driver, not a word-size flag on the native driver: `-m32` was measured
/// to fail on the reference host because the multilib start files are absent, and the
/// reference compiler has no target-selection flag at all. Using the per-target driver here
/// also matches how the AArch64 and RISC-V 64 arms work, so all three cross arms are
/// resolved the same way.
pub const DEFAULT_REF_CC_I686: &[&str] = &["i686-linux-gnu-gcc"];

/// AArch64 reference cross drivers, probed in order.
pub const DEFAULT_REF_CC_AARCH64: &[&str] = &["aarch64-linux-gnu-gcc"];

/// RISC-V 64 reference cross drivers, probed in order.
pub const DEFAULT_REF_CC_RISCV64: &[&str] = &["riscv64-linux-gnu-gcc"];

/// i686 execution runners, probed in order.
///
/// The runner name carries the suffix `i386`, and **never** the target slug: the slug and
/// the emulator name genuinely differ, no emulator named after the slug exists, and the
/// repository's own cross-compilation example invokes `qemu-i386-static` on an i686
/// binary. Conflating the two is the single most likely defect in this module, so the
/// distinction is stated here, at the only place the emulator name is written down.
///
/// Both spellings are probed because the requirements name the plain form while some
/// packagings ship only the statically linked form. The harness works against either with
/// no configuration.
pub const DEFAULT_QEMU_I386: &[&str] = &["qemu-i386", "qemu-i386-static"];

/// AArch64 execution runners, probed in order: plain spelling first, then the statically
/// linked spelling.
pub const DEFAULT_QEMU_AARCH64: &[&str] = &["qemu-aarch64", "qemu-aarch64-static"];

/// RISC-V 64 execution runners, probed in order: plain spelling first, then the statically
/// linked spelling.
pub const DEFAULT_QEMU_RISCV64: &[&str] = &["qemu-riscv64", "qemu-riscv64-static"];

/// External per-cell timeout utility.
///
/// The standard library offers no timed wait on a child process, so `execute.rs` wraps an
/// execution in this utility when it is present and falls back to a watchdog thread when it
/// is not. Its absence changes no behaviour and is never an unavailable oracle.
pub const DEFAULT_TIMEOUT_TOOL: &[&str] = &["timeout"];

/// Optional test-case reducer, used by finding minimization only when it is present.
///
/// Never required: adding a crate is forbidden, so minimization is otherwise manual or
/// scripted, and a finding is complete without it — the reproducer, the exact commands, the
/// captured outputs and the environment fingerprint do not depend on this tool.
pub const DEFAULT_REDUCER: &[&str] = &["creduce"];

/// Kernel identification utility, used for the environment fingerprint only.
const DEFAULT_UNAME: &[&str] = &["uname"];

/// Banner arguments tried in order when capturing a tool's version.
///
/// The usage banner is accepted as a fallback because the compiler under test documents
/// `--help` while not documenting `--version`, and a banner is only ever used to fingerprint
/// the environment. Failing to capture one is recorded as unknown and never fails a run.
const VERSION_ARGUMENTS: &[&str] = &["--version", "--help"];

/// Default per-cell execution budget in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Static archives and start files whose presence indicates a usable C runtime for a
/// target.
///
/// Both are required by the suite's static links: the archive supplies the library and the
/// start file supplies the entry stub. They are located by asking the target's own reference
/// driver where it would find them, which avoids hard-coding a distribution's directory
/// layout.
const C_RUNTIME_ARTIFACTS: &[&str] = &["libc.a", "crt1.o"];

// ---------------------------------------------------------------------------
// Environment-variable accessors
// ---------------------------------------------------------------------------

/// Read a variable and trim it, treating an unset or all-whitespace value as absent.
///
/// Trimming matters because these variables are frequently set by a shell profile or a
/// continuous-integration expression, where a stray space is easy to introduce and would
/// otherwise turn into an unresolvable tool name.
fn trimmed_var(name: &str) -> Option<String> {
    let raw = env::var(name).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(String::from(trimmed))
}

/// The boolean convention for every flag in the catalogue: set, non-empty, and not `0`.
///
/// Deliberately permissive about which affirmative spelling is used, and deliberately
/// strict about the one negative spelling that matters, so that exporting `VAR=0` disables
/// a flag rather than enabling it because it happened to be set.
fn flag_enabled(name: &str) -> bool {
    match trimmed_var(name) {
        Some(value) => value != "0",
        None => false,
    }
}

/// True when the matrix is reduced to the native target at `-O0` and `-O2` for fast local
/// iteration.
///
/// Never the default, and every report produced under it is stamped as reduced coverage, so
/// a quick run can never be mistaken for a full one.
pub fn quick_mode() -> bool {
    flag_enabled(VAR_QUICK)
}

/// True when an unavailable oracle must be escalated to a failure.
///
/// The intended continuous-integration setting: there the toolchain is installed
/// deliberately, so a missing oracle indicates a broken workflow rather than a modest
/// machine.
pub fn strict() -> bool {
    flag_enabled(VAR_STRICT)
}

/// True when unexpected success is downgraded from a failure to a warning.
///
/// Exists for a marker-retirement window only. Unexpected success is listed separately and
/// prominently in the summary either way, so it can never pass unnoticed.
pub fn allow_xpass() -> bool {
    flag_enabled(VAR_ALLOW_XPASS)
}

/// True when a reduced-oracle environment has been acknowledged explicitly.
///
/// This acknowledgement suppresses the strict escalation of an unavailable oracle; it never
/// suppresses the report of one.
pub fn allow_missing_oracles() -> bool {
    flag_enabled(VAR_ALLOW_MISSING_ORACLES)
}

/// True when every cell workspace is retained instead of being removed on success.
pub fn keep_work() -> bool {
    flag_enabled(VAR_KEEP_WORK)
}

/// The active `<area>/<program>` restriction, if any.
///
/// Returned trimmed. A restriction makes the run partial, which [`is_reduced_run`] reports
/// so the summary can be stamped accordingly.
pub fn only_filter() -> Option<String> {
    trimmed_var(VAR_ONLY)
}

/// Whether unexpected success fails the run.
///
/// Fails by default, following the convention that unexpected success is a failure. The
/// justification is asymmetric cost: a stale marker is stale documented knowledge that will
/// mislead the next reader, whereas retiring it is a trivial test-only edit.
pub fn xpass_fails_run() -> bool {
    !allow_xpass()
}

/// Whether an unavailable oracle fails the run.
///
/// False by default, because it is the environment rather than the compiler that is
/// incomplete, and the gap is reported in full either way. True under [`strict`], unless a
/// reduced-oracle environment has been acknowledged with
/// [`allow_missing_oracles`] — the two settings compose so that a deliberately reduced
/// continuous-integration job can still opt out of the escalation without also losing the
/// strict treatment of everything else.
pub fn unavailable_fails_run() -> bool {
    strict() && !allow_missing_oracles()
}

/// Parse the per-cell execution budget, rejecting a malformed value.
///
/// A malformed value is an error rather than a silent fall back to the default, because
/// silently ignoring a misconfigured timeout would hide exactly the failure the timeout
/// exists to catch: a program that finishes promptly under one compiler and hangs under
/// another. Zero is rejected for the same reason — a zero budget would fail every cell
/// instantly and look like a compiler defect.
fn parse_timeout_secs() -> HarnessResult<u64> {
    let Some(raw) = trimmed_var(VAR_TIMEOUT_SECS) else {
        return Ok(DEFAULT_TIMEOUT_SECS);
    };
    let parsed = raw.parse::<u64>().map_err(|error| {
        HarnessError::new(
            format!("reading the per-cell execution budget from {VAR_TIMEOUT_SECS}"),
            format!(
                "{raw:?} is not a whole number of seconds ({error}); unset {VAR_TIMEOUT_SECS} to \
                 use the default of {DEFAULT_TIMEOUT_SECS} seconds, or set it to a positive \
                 whole number"
            ),
        )
    })?;
    if parsed == 0 {
        return Err(HarnessError::new(
            format!("reading the per-cell execution budget from {VAR_TIMEOUT_SECS}"),
            format!(
                "a budget of zero seconds would time out every cell before it could run; unset \
                 {VAR_TIMEOUT_SECS} to use the default of {DEFAULT_TIMEOUT_SECS} seconds, or set \
                 it to a positive whole number"
            ),
        ));
    }
    Ok(parsed)
}

/// The per-cell execution budget in seconds, defaulting to [`DEFAULT_TIMEOUT_SECS`].
///
/// # Panics
///
/// Panics with the full explanatory message from [`parse_timeout_secs`] when the variable
/// holds a value that is not a positive whole number of seconds. That is deliberate and it
/// is not a silent fallback: a misconfigured budget must stop the run rather than quietly
/// become thirty seconds, since the difference between the two is the difference between
/// catching a hang and missing one.
///
/// In normal operation this path is unreachable, because [`discover`] validates the budget
/// before any cell executes and reports a malformed value as an ordinary hard error with no
/// panic at all. The panic here is the belt-and-braces path for a caller that reads the
/// budget without having gone through discovery first.
pub fn timeout_secs() -> u64 {
    match parse_timeout_secs() {
        Ok(seconds) => seconds,
        Err(error) => panic!("{error}"),
    }
}

/// The target the host executes without an emulator, or the oracle (b) baseline when the
/// host architecture matches no supported target.
///
/// The fallback is reported rather than hidden: [`Capabilities::render_report`] states which
/// target quick mode selected and why, so a reduced run on an unexpected host is still
/// self-describing.
fn native_or_baseline_target() -> Target {
    Target::ALL
        .iter()
        .copied()
        .find(|candidate| candidate.is_native())
        .unwrap_or(Target::BASELINE)
}

/// The target and optimization-level sweep this run will perform.
///
/// Every caller reduces identically because reduction happens here and nowhere else. The
/// full matrix is every target at every optimization level; quick mode narrows it to the
/// native target at `-O0` and `-O2`, which keeps both ends of the optimization range while
/// dropping the middle level and every emulated target.
///
/// A program may narrow this further through its own expectation record, but only with a
/// recorded reason: a restriction without an explanation is a defect in the test rather than
/// a legitimate exclusion.
pub fn effective_matrix() -> (Vec<Target>, Vec<OptLevel>) {
    if quick_mode() {
        return (
            vec![native_or_baseline_target()],
            vec![OptLevel::O0, OptLevel::O2],
        );
    }
    (Target::ALL.to_vec(), OptLevel::ALL.to_vec())
}

/// True when this run covers less than the full matrix, because quick mode is active or a
/// program filter is in force.
///
/// `report.rs` stamps its output partial when this holds, which is what stops a reduced run
/// from ever being mistaken for a full one.
pub fn is_reduced_run() -> bool {
    quick_mode() || only_filter().is_some()
}

// ---------------------------------------------------------------------------
// Tool resolution
// ---------------------------------------------------------------------------

/// True when a tool name should be treated as an explicit path rather than looked up on
/// `PATH`.
///
/// Both the platform separator and the forward slash are tested so that a path written in
/// the portable spelling is recognised on any host.
fn looks_like_path(name: &str) -> bool {
    name.contains(MAIN_SEPARATOR) || name.contains('/')
}

/// True when `path` names an existing file that carries an execute permission.
///
/// Symbolic links are followed, and a wrapper shell script is accepted exactly like a
/// compiled binary, because a reference-compiler installation may legitimately be either. A
/// directory is rejected even when it is executable, since a directory can never be spawned.
fn is_executable_file(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file() && has_execute_permission(&metadata),
        Err(_) => false,
    }
}

/// Whether the POSIX execute bit is set for anyone.
///
/// Checked for owner, group and other together rather than for the current user alone: the
/// question this answers is whether the file is a program at all, and a spawn attempt
/// reports a genuine permission problem far more precisely than a permission calculation
/// here could.
#[cfg(unix)]
fn has_execute_permission(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

/// Fallback for hosts with no POSIX permission model, where the existence of the file is the
/// only signal available.
///
/// The supported hosts for this suite are Linux, so this arm is never taken there; it exists
/// so the module remains compilable rather than as a behavioural claim about other systems.
#[cfg(not(unix))]
fn has_execute_permission(_metadata: &fs::Metadata) -> bool {
    true
}

/// Resolve one tool name to an executable file.
///
/// A name containing a separator is treated as an explicit path and checked directly, so an
/// override may point anywhere. A bare name is resolved by walking `PATH` in order and
/// returning the first executable match, which is what a shell would do — without spawning a
/// shell, because no shell is needed here and spawning one only adds a failure mode.
fn resolve_tool(name: &str) -> Option<PathBuf> {
    if looks_like_path(name) {
        let candidate = PathBuf::from(name);
        return is_executable_file(&candidate).then(|| absolutize(candidate));
    }
    let search_path = env::var_os("PATH")?;
    env::split_paths(&search_path)
        .filter(|directory| !directory.as_os_str().is_empty())
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable_file(candidate))
        .map(absolutize)
}

/// Make a resolved tool path absolute, without resolving symbolic links.
///
/// Absolute matters for reproducibility: a finding must carry reproduction commands that work
/// from anywhere, and the test harness makes no guarantee about a test process's working
/// directory, so a relative override or a relative `PATH` entry would otherwise be recorded as a
/// path that means nothing outside the directory discovery happened to run in.
///
/// Symbolic links are deliberately **not** resolved, for two independent reasons. A compiler
/// driver derives its own internal search prefix from the name it was invoked with, so rewriting
/// that name can stop the driver finding its own components; and resolving links would erase
/// which of an emulator's two spellings was selected, which is exactly the fact the pre-flight
/// report exists to state.
fn absolutize(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    match env::current_dir() {
        Ok(directory) => directory.join(path),
        Err(_) => path,
    }
}

/// The first non-empty line of a captured byte stream, trimmed.
///
/// Bytes are converted lossily because a banner is only ever displayed, never parsed, and a
/// tool that emits a stray non-UTF-8 byte should still be identifiable rather than reported
/// as unknown.
fn first_non_empty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(String::from)
}

/// Capture a tool's banner for the environment fingerprint.
///
/// Tries `--version` and then `--help`, accepting the first argument that produces any
/// output on either stream. Three deliberate decisions:
///
/// - **Standard error is accepted as well as standard output**, because tools disagree about
///   where a banner belongs and the fingerprint only needs the text.
/// - **The exit status is ignored.** A usage banner may legitimately exit non-zero while
///   still printing exactly the identification wanted, and the compiler under test documents
///   `--help` rather than `--version`.
/// - **Standard input is redirected from the null device.** The standard library offers no
///   timed wait on a child process, and the suite's timeout facility deliberately lives in
///   the execution module rather than here, so a probe is kept bounded by making it
///   non-interactive: any read of standard input sees end of file immediately and cannot
///   block waiting for a maintainer to type.
///
/// Returns `None` when no argument yields output, which is recorded as an unknown version and
/// never fails a run.
fn probe_version(path: &Path) -> Option<String> {
    for argument in VERSION_ARGUMENTS {
        let Ok(output) = Command::new(path)
            .arg(argument)
            .stdin(Stdio::null())
            .output()
        else {
            continue;
        };
        if let Some(line) = first_non_empty_line(&output.stdout) {
            return Some(line);
        }
        if let Some(line) = first_non_empty_line(&output.stderr) {
            return Some(line);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tool records
// ---------------------------------------------------------------------------

/// How a tool came to be located, or why it could not be.
///
/// Recorded so that the pre-flight report can state the mechanism rather than only the
/// result: "the override said so" and "the second probed default matched" are very different
/// facts when an environment behaves unexpectedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolSource {
    /// An environment variable named the tool explicitly.
    EnvironmentOverride,
    /// The build system supplied the path to the binary it had just built. Applies to the
    /// compiler under test alone, and it is what guarantees the suite never validates a stale
    /// build.
    CargoBinary,
    /// One of the probed default names resolved on `PATH`.
    ProbedDefault,
    /// Nothing resolved.
    Unresolved,
}

impl ToolSource {
    /// The phrase used in the pre-flight report and in diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            ToolSource::EnvironmentOverride => "environment override",
            ToolSource::CargoBinary => "build-system binary path",
            ToolSource::ProbedDefault => "probed default",
            ToolSource::Unresolved => "unresolved",
        }
    }
}

/// One discovered — or genuinely absent — tool.
///
/// A record is kept for a tool that could not be found, rather than the tool simply being
/// missing from a collection, because the report has to be able to say what was looked for
/// and how to supply it. An absent tool with no record would be an absent tool with no
/// diagnosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRecord {
    /// What the tool does in this suite, for example `reference compiler (aarch64 arm of
    /// oracle (a))`.
    pub role: String,
    /// The environment variable that overrides this tool, when the catalogue defines one.
    ///
    /// `None` for the two optional auxiliary tools, which the catalogue deliberately gives no
    /// override: inventing a variable for them would add configuration the specification does not
    /// define, and neither tool changes a verdict, so probing the standard name is sufficient.
    pub override_variable: Option<&'static str>,
    /// The default names probed when the override is unset, in probe order.
    pub defaults: &'static [&'static str],
    /// The candidates actually tried, in order. Equal to the override alone when one was set,
    /// and to [`ToolRecord::defaults`] otherwise, which is what lets the report distinguish a
    /// bad override from a genuinely missing package.
    pub candidates: Vec<String>,
    /// True when the override variable supplied the candidate list, whether or not the
    /// candidate resolved.
    ///
    /// Kept separately from [`ToolRecord::source`], which records only how a *successful*
    /// resolution happened. The distinction is what lets an unresolved tool be diagnosed
    /// correctly: a bad override is fixed by editing a variable, while a missing default is
    /// fixed by installing a package, and the two must not be described as each other.
    pub overridden: bool,
    /// How the tool was located, or [`ToolSource::Unresolved`].
    pub source: ToolSource,
    /// Absolute or `PATH`-resolved path to the executable, when one was found.
    pub path: Option<PathBuf>,
    /// First line of the tool's banner, when one could be captured. `None` means the tool
    /// exists but declined to identify itself, which is recorded rather than treated as an
    /// error.
    pub version: Option<String>,
}

impl ToolRecord {
    /// Discover one tool from its override variable and its probe list.
    ///
    /// When the override is set, it is the only candidate: a maintainer who names a tool
    /// explicitly must be told that that tool is missing, not silently given a different one
    /// that happened to be on `PATH`. When it is unset, the defaults are tried in order.
    fn discover(
        role: impl Into<String>,
        override_variable: Option<&'static str>,
        defaults: &'static [&'static str],
    ) -> ToolRecord {
        let (candidates, overridden) = match override_variable.and_then(trimmed_var) {
            Some(value) => (vec![value], true),
            None => (
                defaults
                    .iter()
                    .copied()
                    .map(String::from)
                    .collect::<Vec<_>>(),
                false,
            ),
        };
        let resolved = candidates
            .iter()
            .find_map(|candidate| resolve_tool(candidate));
        let source = match (&resolved, overridden) {
            (Some(_), true) => ToolSource::EnvironmentOverride,
            (Some(_), false) => ToolSource::ProbedDefault,
            (None, _) => ToolSource::Unresolved,
        };
        let version = resolved.as_deref().and_then(probe_version);
        ToolRecord {
            role: role.into(),
            override_variable,
            defaults,
            candidates,
            overridden,
            source,
            path: resolved,
            version,
        }
    }

    /// Record a tool that the build system located for us.
    ///
    /// Used for the compiler under test when no override is set, where the path comes from the
    /// build system rather than from a search, so there is nothing to probe and nothing that
    /// could have gone wrong on `PATH`.
    fn from_build_system(
        role: impl Into<String>,
        override_variable: Option<&'static str>,
        path: PathBuf,
    ) -> ToolRecord {
        let version = probe_version(&path);
        ToolRecord {
            role: role.into(),
            override_variable,
            defaults: &[],
            candidates: vec![path.display().to_string()],
            overridden: false,
            source: ToolSource::CargoBinary,
            path: Some(path),
            version,
        }
    }

    /// True when this tool is available for use.
    pub fn is_available(&self) -> bool {
        self.path.is_some()
    }

    /// The resolved executable path, when the tool is available.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The captured banner, or a fixed absence marker when none could be captured.
    ///
    /// The marker is a stable string so that two fingerprints taken on the same machine
    /// compare equal, which is what makes a fingerprint useful for attributing a divergence to
    /// toolchain drift.
    pub fn version_or_unknown(&self) -> &str {
        match &self.version {
            Some(version) => version.as_str(),
            None => "version unknown (tool present but reported no banner)",
        }
    }

    /// A one-line summary for the pre-flight report.
    pub fn summary(&self) -> String {
        match &self.path {
            Some(path) => format!(
                "{} via {} -> {} [{}]",
                self.role,
                self.source.label(),
                path.display(),
                self.version_or_unknown()
            ),
            None => format!("{} NOT FOUND: {}", self.role, self.diagnosis()),
        }
    }

    /// A self-diagnosing explanation of an unresolved tool.
    ///
    /// Always names the override variable and always says what was actually tried, so a
    /// misconfigured environment explains itself without the reader opening this file. The two
    /// cases are kept distinct because they call for different fixes: a bad override is
    /// corrected by editing the variable, while a missing default is corrected by installing a
    /// package.
    pub fn diagnosis(&self) -> String {
        let named = join_quoted(&self.candidates);
        match self.override_variable {
            Some(variable) if self.overridden && self.defaults.is_empty() => format!(
                "{variable} names {named}, which is not an executable file and was not found on \
                 PATH; set {variable} to an executable path"
            ),
            Some(variable) if self.overridden => format!(
                "{variable} names {named}, which is not an executable file and was not found on \
                 PATH; correct {variable}, or unset it to probe the defaults {}",
                join_quoted_static(self.defaults)
            ),
            Some(variable) if self.candidates.is_empty() => format!(
                "this tool has no default name to probe; set {variable} to an executable path"
            ),
            Some(variable) => format!(
                "none of the probed defaults {named} was found on PATH; install one of them, or \
                 set {variable} to an executable path"
            ),
            None if self.candidates.is_empty() => String::from(
                "this tool has neither a default name to probe nor an environment override",
            ),
            None => format!(
                "none of the probed defaults {named} was found on PATH; the catalogue defines no \
                 override for this optional tool, so install one of them to enable it"
            ),
        }
    }
}

/// Render owned strings as a quoted, comma-separated list for diagnostics.
fn join_quoted(values: &[String]) -> String {
    let quoted = values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>();
    quoted.join(", ")
}

/// Render static names as a quoted, comma-separated list for diagnostics.
fn join_quoted_static(values: &[&str]) -> String {
    let quoted = values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>();
    quoted.join(", ")
}

// ---------------------------------------------------------------------------
// C runtime availability
// ---------------------------------------------------------------------------

/// Whether one static-link input could be located for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CRuntimeArtifact {
    /// The file the static link needs, for example `libc.a`.
    pub name: &'static str,
    /// Where the target's reference driver said it would find the file, when it named an
    /// existing absolute path.
    pub resolved: Option<PathBuf>,
}

/// Whether a target's C runtime appears usable for the static links the suite performs.
///
/// Determined by asking the target's own reference driver where it would find each required
/// input, rather than by hard-coding a distribution's directory layout, which differs between
/// distributions and is the documented cause of a whole class of cross-compilation surprises.
/// The probe writes nothing and compiles nothing.
///
/// This is a report-quality signal, not a gate. A runtime that is genuinely broken surfaces as
/// a link failure at environment scope, which is reported as such rather than as a compiler
/// defect — that distinction is the whole point of recording it here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CRuntimeStatus {
    /// The target this status describes.
    pub target: Target,
    /// The driver that was asked, when one was available for this target.
    pub driver: Option<PathBuf>,
    /// One entry per required input, in [`C_RUNTIME_ARTIFACTS`] order.
    pub artifacts: Vec<CRuntimeArtifact>,
}

impl CRuntimeStatus {
    /// Probe one target's runtime through the driver that targets it.
    fn probe(target: Target, driver: Option<&Path>) -> CRuntimeStatus {
        let artifacts = C_RUNTIME_ARTIFACTS
            .iter()
            .map(|name| CRuntimeArtifact {
                name,
                resolved: driver.and_then(|driver| probe_runtime_artifact(driver, name)),
            })
            .collect::<Vec<_>>();
        CRuntimeStatus {
            target,
            driver: driver.map(Path::to_path_buf),
            artifacts,
        }
    }

    /// True when every required input was located.
    pub fn appears_usable(&self) -> bool {
        self.driver.is_some() && self.artifacts.iter().all(|entry| entry.resolved.is_some())
    }

    /// The names of the inputs that could not be located, in probe order.
    pub fn missing(&self) -> Vec<&'static str> {
        self.artifacts
            .iter()
            .filter(|entry| entry.resolved.is_none())
            .map(|entry| entry.name)
            .collect()
    }

    /// A one-line description for the pre-flight report.
    ///
    /// Three outcomes are distinguished, because they mean three different things: the runtime
    /// looks fine, the runtime is missing pieces, or nobody could be asked. The third is not a
    /// failure — the compiler under test carries its own integrated linker and searches system
    /// paths itself, so a target with no reference driver may still link perfectly well.
    pub fn describe(&self) -> String {
        if self.driver.is_none() {
            return String::from(
                "not determined (no reference driver for this target to ask; the compiler under \
                 test links through its own integrated linker)",
            );
        }
        let missing = self.missing();
        if missing.is_empty() {
            return format!(
                "appears usable ({} located)",
                join_quoted_static(C_RUNTIME_ARTIFACTS)
            );
        }
        format!(
            "INCOMPLETE: {} not located; a static link for this target will fail at environment \
             scope rather than indicating a compiler defect",
            join_quoted_static(&missing)
        )
    }
}

/// Ask a driver where it would find one static-link input.
///
/// The driver echoes the bare name back when it cannot locate the file, so only an absolute
/// path that exists as a file is accepted as a positive answer. The exit status is not
/// consulted, because a driver reports this condition through the printed value rather than
/// through its status.
///
/// This flag is used against a reference driver in a probe only. It never appears in a
/// differential invocation, so the discipline that only flags both compilers honour identically
/// may be passed to both compilers is untouched.
fn probe_runtime_artifact(driver: &Path, artifact: &str) -> Option<PathBuf> {
    let output = Command::new(driver)
        .arg(format!("-print-file-name={artifact}"))
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let printed = first_non_empty_line(&output.stdout)?;
    let candidate = PathBuf::from(printed);
    if !candidate.is_absolute() {
        return None;
    }
    match fs::metadata(&candidate) {
        Ok(metadata) if metadata.is_file() => Some(candidate),
        Ok(_) | Err(_) => None,
    }
}

// ---------------------------------------------------------------------------
// The capability record
// ---------------------------------------------------------------------------

/// The path the build system recorded for the compiler under test, when this test was built in
/// a package that declares a binary target named `bcc`.
///
/// Read through [`option_env`] rather than [`env`](macro@std::env) deliberately. The
/// non-optional form is a compile-time assertion that such a target exists, which would make
/// this module fail to build in any package that does not declare one; the optional form lets
/// the same source serve both layouts and turns the question into a runtime one, answered with
/// an explanatory hard failure instead of a build error. When the value is present it points at
/// the binary the build system has just finished producing, which is what guarantees the suite
/// never silently validates a stale build.
const CARGO_BIN_EXE_BCC: Option<&str> = option_env!("CARGO_BIN_EXE_bcc");

/// Everything discovery learned about this machine.
///
/// Cloneable and free of interior mutability, so the memoized instance can be handed to every
/// concurrently executing test without a lock and without any test being able to disturb
/// another's view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// The compiler under test. Always present: discovery fails hard when it is not.
    pub bcc: ToolRecord,
    /// Reference compiler for the host architecture, and the driver both undefined-behaviour
    /// audit gates use.
    pub ref_cc_native: ToolRecord,
    /// Reference cross driver for the i686 arm of oracle (a).
    pub ref_cc_i686: ToolRecord,
    /// Reference cross driver for the AArch64 arm of oracle (a).
    pub ref_cc_aarch64: ToolRecord,
    /// Reference cross driver for the RISC-V 64 arm of oracle (a).
    pub ref_cc_riscv64: ToolRecord,
    /// Emulated-execution runner for the i686 target. Its name carries the suffix `i386`,
    /// deliberately not the target slug — see [`DEFAULT_QEMU_I386`].
    pub runner_i686: ToolRecord,
    /// Emulated-execution runner for AArch64.
    pub runner_aarch64: ToolRecord,
    /// Emulated-execution runner for RISC-V 64.
    pub runner_riscv64: ToolRecord,
    /// External per-cell timeout utility. Optional: execution falls back to a watchdog thread.
    pub timeout_tool: ToolRecord,
    /// Optional test-case reducer for finding minimization.
    pub reducer: ToolRecord,
    /// One C runtime status per target, in [`Target::ALL`] order.
    pub c_runtimes: Vec<CRuntimeStatus>,
    /// Kernel identification, or an explanatory substitute when it could not be obtained.
    pub kernel: String,
    /// Host architecture as the standard library reports it.
    pub host_arch: &'static str,
    /// Host operating system as the standard library reports it.
    pub host_os: &'static str,
    /// The validated per-cell execution budget in seconds.
    pub timeout_secs: u64,
}

impl Capabilities {
    /// The reference-compiler record that targets `target`, when one is configured.
    ///
    /// The native driver serves whichever target the host is, and each non-native target is
    /// served by its own cross driver. The single `None` case is a non-native x86-64 target,
    /// which arises only when the suite runs on a host that is not x86-64: the catalogue defines
    /// cross drivers for i686, AArch64 and RISC-V 64 only, so on such a host oracle (a)'s x86-64
    /// arm has no driver and is reported unavailable rather than being silently served by a
    /// driver that targets something else.
    fn ref_cc_record_for(&self, target: Target) -> Option<&ToolRecord> {
        if target.is_native() {
            return Some(&self.ref_cc_native);
        }
        match target {
            Target::I686 => Some(&self.ref_cc_i686),
            Target::Aarch64 => Some(&self.ref_cc_aarch64),
            Target::Riscv64 => Some(&self.ref_cc_riscv64),
            Target::X86_64 => None,
        }
    }

    /// The reference compiler that oracle (a) must use for `target`, when it is available.
    ///
    /// Never accompanied by a word-size or target-selection flag: the reference compiler has no
    /// target-selection flag, and the word-size flag was measured to fail on the reference host
    /// because the multilib start files are absent. Selecting a target on the reference side means
    /// selecting a different driver binary, which is exactly what this returns.
    pub fn ref_cc_for(&self, target: Target) -> Option<&Path> {
        self.ref_cc_record_for(target)
            .and_then(|record| record.path())
    }

    /// The runner record for a target that requires emulation, when one is configured.
    ///
    /// `None` for a target the host executes directly and for x86-64, which the catalogue defines
    /// no runner for.
    fn runner_record_for(&self, target: Target) -> Option<&ToolRecord> {
        if target.is_native() {
            return None;
        }
        match target {
            Target::I686 => Some(&self.runner_i686),
            Target::Aarch64 => Some(&self.runner_aarch64),
            Target::Riscv64 => Some(&self.runner_riscv64),
            Target::X86_64 => None,
        }
    }

    /// The emulator that must be prefixed to an execution for `target`, or `None` when the binary
    /// is to be executed directly.
    ///
    /// # This returns `None` for two very different reasons
    ///
    /// A native target needs no runner, and a non-native target whose runner is missing has none.
    /// Both answer `None`, so a caller about to execute something must gate on
    /// [`Capabilities::can_execute`] first and treat a target it cannot execute as unavailable.
    /// Executing on the strength of `None` alone would run a foreign binary directly and read the
    /// resulting failure as a compiler defect.
    pub fn runner_for(&self, target: Target) -> Option<&Path> {
        self.runner_record_for(target)
            .and_then(|record| record.path())
    }

    /// True when a binary built for `target` can actually be run on this machine.
    ///
    /// True for a target the host executes natively, and for a non-native target whose runner
    /// resolved. This is the precondition for every oracle, because all three compare the
    /// behaviour of a program that ran.
    pub fn can_execute(&self, target: Target) -> bool {
        if target.is_native() {
            return true;
        }
        self.runner_for(target).is_some()
    }

    /// True when oracle (a) can be attempted for `target`.
    ///
    /// Requires both a reference compiler that targets it and a way to execute the result. A
    /// missing native driver therefore takes oracle (a) out entirely while leaving oracles (b) and
    /// (c) running, and a missing emulator takes the affected target out of oracle (a)'s cross arm
    /// as well as out of oracle (b) — exactly the degradation the module documentation tabulates.
    pub fn oracle_a_available(&self, target: Target) -> bool {
        self.ref_cc_for(target).is_some() && self.can_execute(target)
    }

    /// True when oracle (b) can be attempted for `target`.
    ///
    /// Requires that both this target and the baseline can be executed, because the comparison is
    /// against the baseline cell at the same optimization level. The baseline compared with itself
    /// is not a comparison, so the baseline target itself is excluded.
    pub fn oracle_b_available(&self, target: Target) -> bool {
        target != Target::BASELINE && self.can_execute(target) && self.can_execute(Target::BASELINE)
    }

    /// True when oracle (c) can be attempted for `target`.
    ///
    /// Needs only the compiler under test, which is always present, and a way to execute the
    /// result. This is why the golden record keeps working in an environment that has no reference
    /// compiler at all, and why a reduced environment still catches regression and toolchain
    /// drift.
    pub fn oracle_c_available(&self, target: Target) -> bool {
        self.can_execute(target)
    }

    /// Dispatch the availability question by oracle identity, so a caller that already holds an
    /// [`Oracle`] does not have to re-derive which predicate to ask.
    pub fn oracle_available(&self, oracle: Oracle, target: Target) -> bool {
        match oracle {
            Oracle::ReferenceCompiler => self.oracle_a_available(target),
            Oracle::CrossBackend => self.oracle_b_available(target),
            Oracle::GoldenRecord => self.oracle_c_available(target),
        }
    }

    /// The C runtime status recorded for `target`.
    pub fn c_runtime_for(&self, target: Target) -> Option<&CRuntimeStatus> {
        self.c_runtimes
            .iter()
            .find(|status| status.target == target)
    }

    /// Every oracle arm that cannot be attempted, phrased for the summary.
    ///
    /// Only arms inside the effective matrix are listed, so a quick run does not report the
    /// targets it deliberately excluded as gaps. Each entry names the oracle, the target and the
    /// diagnosis, which is what makes an unavailable arm actionable instead of merely visible.
    pub fn unavailable_oracle_arms(&self) -> Vec<String> {
        let (targets, _) = effective_matrix();
        let mut gaps = Vec::new();
        for target in targets {
            for oracle in Oracle::ALL {
                if self.oracle_available(oracle, target) {
                    continue;
                }
                if oracle == Oracle::CrossBackend && target == Target::BASELINE {
                    continue;
                }
                gaps.push(format!(
                    "UNAVAILABLE: {} for {}: {}",
                    oracle.label(),
                    target,
                    self.arm_diagnosis(oracle, target)
                ));
            }
        }
        gaps
    }

    /// Why one oracle arm cannot be attempted.
    ///
    /// Reports the execution obstacle first, because an inability to run the program blocks every
    /// oracle and is therefore the more useful fact when both apply.
    fn arm_diagnosis(&self, oracle: Oracle, target: Target) -> String {
        if !self.can_execute(target) {
            return match self.runner_record_for(target) {
                Some(record) => format!("cannot execute {target}: {}", record.diagnosis()),
                None => format!(
                    "cannot execute {target}: this host is {}, no runner is defined for this \
                     target, and it is not the host architecture",
                    self.host_arch
                ),
            };
        }
        match oracle {
            Oracle::ReferenceCompiler => match self.ref_cc_record_for(target) {
                Some(record) => record.diagnosis(),
                None => format!(
                    "no reference driver targets {target} on a {} host; the catalogue defines \
                     cross drivers for i686, AArch64 and RISC-V 64 only",
                    self.host_arch
                ),
            },
            Oracle::CrossBackend if target == Target::BASELINE => format!(
                "{target} is the baseline, so there is no cross-backend comparison to make: every \
                 other target is compared against it"
            ),
            Oracle::CrossBackend => format!(
                "the baseline {} cannot be executed, so there is nothing to compare against",
                Target::BASELINE
            ),
            Oracle::GoldenRecord => format!("{target} cannot be executed"),
        }
    }

    /// The human-readable oracle inventory printed by the suite's pre-flight check.
    ///
    /// This exists so that a misconfigured environment is diagnosed before the whole matrix
    /// executes rather than after. It states, per target, whether the compiler-under-test arm runs,
    /// whether oracle (a) runs and through which driver, whether execution is native or through
    /// which runner, and whether the C runtime appears usable; and for the run as a whole, the
    /// effective matrix, whether coverage is reduced, every unavailable arm, and the verdict
    /// policies in force.
    ///
    /// No coverage ratio appears anywhere in it. None could be measured without instrumentation
    /// that the project's dependency rule forbids, so publishing one would be fabricating a number
    /// nobody working in this repository could verify. The enumerable matrix below is the honest
    /// substitute.
    ///
    /// The rendering is a pure function of the record and the environment: no clock, no process
    /// identifier, no map iteration, so two runs on the same machine produce byte-identical text
    /// and the report is unaffected by how many threads the test harness uses.
    pub fn render_report(&self) -> String {
        let (targets, opt_levels) = effective_matrix();
        let mut lines = Vec::new();
        lines.push(String::from(
            "=== differential conformance suite: oracle capability report ===",
        ));
        lines.push(String::new());

        lines.push(String::from("Compiler under test"));
        lines.push(format!("  {}", self.bcc.summary()));
        lines.push(String::new());

        lines.push(String::from("Reference compilers — oracle (a)"));
        for record in [
            &self.ref_cc_native,
            &self.ref_cc_i686,
            &self.ref_cc_aarch64,
            &self.ref_cc_riscv64,
        ] {
            lines.push(format!("  {}", record.summary()));
        }
        lines.push(String::new());

        lines.push(String::from(
            "Execution runners — oracle (b), and oracle (a)'s cross arms",
        ));
        for record in [
            &self.runner_i686,
            &self.runner_aarch64,
            &self.runner_riscv64,
        ] {
            lines.push(format!("  {}", record.summary()));
        }
        lines.push(format!(
            "  {}",
            if Target::BASELINE.is_native() {
                format!(
                    "{} needs no runner: it is the host architecture and is executed directly, \
                     which is why it is the baseline",
                    Target::BASELINE
                )
            } else {
                format!(
                    "{} is the baseline but is NOT native on this {} host, and the catalogue \
                     defines no runner for it, so it cannot be executed here",
                    Target::BASELINE,
                    env::consts::ARCH
                )
            }
        ));
        lines.push(String::new());

        lines.push(String::from("Auxiliary tooling — optional by design"));
        lines.push(format!("  {}", self.timeout_tool.summary()));
        if !self.timeout_tool.is_available() {
            lines.push(String::from(
                "    consequence: execution falls back to a standard-library watchdog thread; no \
                 behavioural change, and this is not an unavailable oracle",
            ));
        }
        lines.push(format!("  {}", self.reducer.summary()));
        if !self.reducer.is_available() {
            lines.push(String::from(
                "    consequence: finding minimization is manual or scripted; a finding remains \
                 complete without it",
            ));
        }
        lines.push(String::from(
            "  binary-inspection tools are deliberately not probed: ELF identification bytes are \
             read with the standard library",
        ));
        lines.push(String::new());

        lines.push(String::from("Host"));
        lines.push(format!(
            "  architecture {}, operating system {}",
            self.host_arch, self.host_os
        ));
        lines.push(format!("  kernel {}", self.kernel));
        lines.push(format!(
            "  per-cell execution budget {} s (override with {})",
            self.timeout_secs, VAR_TIMEOUT_SECS
        ));
        lines.push(String::new());

        lines.push(String::from("Effective matrix"));
        lines.push(format!("  targets: {}", join_targets(&targets)));
        lines.push(format!(
            "  optimization levels: {}",
            join_opt_levels(&opt_levels)
        ));
        lines.push(format!(
            "  cells per program: {}",
            targets.len() * opt_levels.len()
        ));
        if is_reduced_run() {
            lines.push(String::from(
                "  coverage: REDUCED — this run does not cover the full matrix and its report is \
                 stamped partial",
            ));
            if quick_mode() {
                lines.push(format!(
                    "    {VAR_QUICK} is set: reduced to {} at {} and {}",
                    native_or_baseline_target(),
                    OptLevel::O0,
                    OptLevel::O2
                ));
                if !Target::BASELINE.is_native() {
                    lines.push(format!(
                        "    note: no supported target is native on this {} host, so the baseline \
                         {} was selected",
                        self.host_arch,
                        Target::BASELINE
                    ));
                }
            }
            if let Some(filter) = only_filter() {
                lines.push(format!("    {VAR_ONLY} is set: restricted to {filter:?}"));
            }
        } else {
            lines.push(String::from(
                "  coverage: FULL — every target at every optimization level in scope",
            ));
        }
        lines.push(String::new());

        lines.push(String::from("Per-target oracle arms"));
        for target in Target::ALL {
            lines.extend(self.render_target_block(target, &targets));
        }
        lines.push(String::new());

        lines.push(String::from("Unavailable oracle arms"));
        let gaps = self.unavailable_oracle_arms();
        if gaps.is_empty() {
            lines.push(String::from(
                "  none — every oracle arm in the effective matrix can be attempted",
            ));
        } else {
            for gap in &gaps {
                lines.push(format!("  {gap}"));
            }
            lines.push(format!(
                "  policy: an unavailable arm {} the run ({} to escalate, {} to acknowledge a \
                 deliberately reduced environment)",
                if unavailable_fails_run() {
                    "FAILS"
                } else {
                    "is reported but does not fail"
                },
                VAR_STRICT,
                VAR_ALLOW_MISSING_ORACLES
            ));
        }
        lines.push(String::new());

        lines.push(String::from("Verdict policy"));
        lines.push(format!(
            "  unexpected success (XPASS) fails the run: {} (set {} to downgrade it to a warning)",
            yes_or_no(xpass_fails_run()),
            VAR_ALLOW_XPASS
        ));
        lines.push(format!(
            "  an unavailable oracle fails the run: {} (set {} to escalate)",
            yes_or_no(unavailable_fails_run()),
            VAR_STRICT
        ));
        lines.push(format!(
            "  cell workspaces retained after success: {} (set {} to retain them)",
            yes_or_no(keep_work()),
            VAR_KEEP_WORK
        ));
        lines.push(String::from(
            "  a missing oracle is never a silent pass: it is recorded as UNAVAILABLE and listed \
             in the run summary",
        ));

        lines.join("\n")
    }

    /// One target's block of the pre-flight report.
    fn render_target_block(&self, target: Target, in_matrix: &[Target]) -> Vec<String> {
        let mut lines = Vec::new();
        let mut role = Vec::new();
        if target == Target::BASELINE {
            role.push(String::from("oracle (b) baseline"));
        }
        role.push(String::from(if target.is_native() {
            "native execution"
        } else {
            "emulated execution"
        }));
        role.push(format!(
            "ELF{}, {}-byte pointers",
            target.elf_class(),
            target.pointer_width_bytes()
        ));
        lines.push(format!("  {} ({})", target, role.join(", ")));

        if !in_matrix.contains(&target) {
            lines.push(String::from(
                "    excluded from this run by the reduced matrix, not by a missing tool",
            ));
            return lines;
        }

        lines.push(String::from(
            "    compiler-under-test arm : RUNS (the compiler under test is always present)",
        ));
        lines.push(format!(
            "    oracle (a)              : {}",
            match self.ref_cc_for(target) {
                Some(driver) if self.can_execute(target) => {
                    format!("RUNS via {}", driver.display())
                }
                _ => format!(
                    "UNAVAILABLE — {}",
                    self.arm_diagnosis(Oracle::ReferenceCompiler, target)
                ),
            }
        ));
        lines.push(format!(
            "    oracle (b)              : {}",
            if target == Target::BASELINE {
                String::from("baseline — every other target is compared against this one")
            } else if self.oracle_b_available(target) {
                String::from("RUNS against the baseline at the same optimization level")
            } else {
                format!(
                    "UNAVAILABLE — {}",
                    self.arm_diagnosis(Oracle::CrossBackend, target)
                )
            }
        ));
        lines.push(format!(
            "    oracle (c)              : {}",
            if self.oracle_c_available(target) {
                String::from("RUNS against the recorded golden stdout")
            } else {
                format!(
                    "UNAVAILABLE — {}",
                    self.arm_diagnosis(Oracle::GoldenRecord, target)
                )
            }
        ));
        lines.push(format!(
            "    execution               : {}",
            match self.runner_for(target) {
                Some(runner) => format!("via {}", runner.display()),
                None if target.is_native() => String::from("native, no emulator involved"),
                None => String::from("NO RUNNER — this target cannot be executed"),
            }
        ));
        lines.push(format!(
            "    C runtime               : {}",
            match self.c_runtime_for(target) {
                Some(status) => status.describe(),
                None => String::from("not recorded"),
            }
        ));
        lines
    }

    /// The environment fingerprint written into every finding artifact.
    ///
    /// Records the compiler under test, every reference driver, every runner, the auxiliary tools
    /// and the kernel, so that a divergence can later be attributed to toolchain drift rather than
    /// to the compiler. That distinction is precisely what the repository's own risk register asks
    /// for when it notes that emulator version skew is a hazard for cross-architecture testing.
    ///
    /// Deterministic and free of any timestamp, so two fingerprints from the same machine compare
    /// equal and a difference between two fingerprints is always a real difference.
    pub fn render_fingerprint(&self) -> String {
        let mut lines = Vec::new();
        lines.push(String::from("# environment fingerprint"));
        lines.push(fingerprint_line("bcc", &self.bcc));
        lines.push(fingerprint_line("reference-cc-native", &self.ref_cc_native));
        lines.push(fingerprint_line("reference-cc-i686", &self.ref_cc_i686));
        lines.push(fingerprint_line(
            "reference-cc-aarch64",
            &self.ref_cc_aarch64,
        ));
        lines.push(fingerprint_line(
            "reference-cc-riscv64",
            &self.ref_cc_riscv64,
        ));
        lines.push(fingerprint_line("runner-i686", &self.runner_i686));
        lines.push(fingerprint_line("runner-aarch64", &self.runner_aarch64));
        lines.push(fingerprint_line("runner-riscv64", &self.runner_riscv64));
        lines.push(fingerprint_line("timeout-utility", &self.timeout_tool));
        lines.push(fingerprint_line("reducer", &self.reducer));
        lines.push(format!("host-arch: {}", self.host_arch));
        lines.push(format!("host-os: {}", self.host_os));
        lines.push(format!("kernel: {}", self.kernel));
        lines.push(format!("per-cell-timeout-secs: {}", self.timeout_secs));
        for target in Target::ALL {
            lines.push(format!(
                "c-runtime-{}: {}",
                target.short_name(),
                match self.c_runtime_for(target) {
                    Some(status) => status.describe(),
                    None => String::from("not recorded"),
                }
            ));
        }
        let (targets, opt_levels) = effective_matrix();
        lines.push(format!("matrix-targets: {}", join_targets(&targets)));
        lines.push(format!(
            "matrix-opt-levels: {}",
            join_opt_levels(&opt_levels)
        ));
        lines.push(format!(
            "matrix-coverage: {}",
            if is_reduced_run() { "reduced" } else { "full" }
        ));
        lines.join("\n")
    }
}

/// One `key: value` fingerprint line, stating the version and the path or a fixed absence marker.
fn fingerprint_line(key: &str, record: &ToolRecord) -> String {
    match record.path() {
        Some(path) => format!(
            "{key}: {} ({})",
            record.version_or_unknown(),
            path.display()
        ),
        None => format!("{key}: absent"),
    }
}

/// `yes` or `no`, for policy lines that must read unambiguously.
fn yes_or_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

/// Render targets as a comma-separated list of triples, in the order given.
fn join_targets(targets: &[Target]) -> String {
    targets
        .iter()
        .map(|target| String::from(target.triple()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render optimization levels as a comma-separated list of command-line spellings.
fn join_opt_levels(levels: &[OptLevel]) -> String {
    levels
        .iter()
        .map(|level| String::from(level.flag()))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Discover the oracles available on this machine.
///
/// Cheap and idempotent. The first call performs the probing — a handful of `PATH` lookups and
/// short banner spawns — and memoizes the outcome, including a failure, in a
/// [`OnceLock`]. Every subsequent call in the same process returns a clone of that outcome.
///
/// Memoization matters here for two reasons beyond speed. The suite's tests run concurrently by
/// default and each one needs the record, so probing once removes a few hundred redundant process
/// spawns; and because the record is computed exactly once, every test in a run sees the same
/// inventory, which is what makes the reports of two runs comparable. The cache is a
/// write-once cell rather than a mutable global: there is no `static mut`, no lock to acquire, no
/// interior mutability and no unchecked-code escape hatch anywhere in this module.
///
/// # Errors
///
/// Returns an error in exactly two situations, both of which are the suite's own configuration
/// rather than a modest environment:
///
/// - the compiler under test could not be located, which no amount of degradation can work around;
/// - the per-cell execution budget is set to something that is not a positive whole number of
///   seconds, which is rejected here so that the pre-flight check reports it before any cell runs.
///
/// Every other missing tool is recorded, reported and degraded around. A tool an environment can
/// legitimately lack is never an error.
pub fn discover() -> HarnessResult<Capabilities> {
    static CACHE: OnceLock<HarnessResult<Capabilities>> = OnceLock::new();
    CACHE.get_or_init(discover_once).clone()
}

/// Perform the probing exactly once on behalf of [`discover`].
fn discover_once() -> HarnessResult<Capabilities> {
    // Validated first, and deliberately before any process is spawned: a misconfigured budget is
    // a configuration defect, and reporting it up front is far clearer than letting probing
    // succeed and then failing at the first cell.
    let timeout_secs = parse_timeout_secs()?;
    let bcc = resolve_compiler_under_test()?;

    let ref_cc_native = ToolRecord::discover(
        format!(
            "reference compiler, native arm ({}) and undefined-behaviour audit gates",
            env::consts::ARCH
        ),
        Some(VAR_REF_CC),
        DEFAULT_REF_CC,
    );
    let ref_cc_i686 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::I686),
        Some(VAR_REF_CC_I686),
        DEFAULT_REF_CC_I686,
    );
    let ref_cc_aarch64 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::Aarch64),
        Some(VAR_REF_CC_AARCH64),
        DEFAULT_REF_CC_AARCH64,
    );
    let ref_cc_riscv64 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::Riscv64),
        Some(VAR_REF_CC_RISCV64),
        DEFAULT_REF_CC_RISCV64,
    );

    // The runner names are the one place where the emulator spelling and the target slug diverge:
    // the i686 target is executed by qemu-i386. Both the plain and the statically linked spellings
    // are probed for each architecture, in that order.
    let runner_i686 = ToolRecord::discover(
        format!("execution runner for {}", Target::I686),
        Some(VAR_QEMU_I386),
        DEFAULT_QEMU_I386,
    );
    let runner_aarch64 = ToolRecord::discover(
        format!("execution runner for {}", Target::Aarch64),
        Some(VAR_QEMU_AARCH64),
        DEFAULT_QEMU_AARCH64,
    );
    let runner_riscv64 = ToolRecord::discover(
        format!("execution runner for {}", Target::Riscv64),
        Some(VAR_QEMU_RISCV64),
        DEFAULT_QEMU_RISCV64,
    );

    let timeout_tool = ToolRecord::discover(
        "per-cell timeout utility (optional)",
        None,
        DEFAULT_TIMEOUT_TOOL,
    );
    let reducer = ToolRecord::discover(
        "test-case reducer for finding minimization (optional)",
        None,
        DEFAULT_REDUCER,
    );

    let mut capabilities = Capabilities {
        bcc,
        ref_cc_native,
        ref_cc_i686,
        ref_cc_aarch64,
        ref_cc_riscv64,
        runner_i686,
        runner_aarch64,
        runner_riscv64,
        timeout_tool,
        reducer,
        c_runtimes: Vec::new(),
        kernel: probe_kernel(),
        host_arch: env::consts::ARCH,
        host_os: env::consts::OS,
        timeout_secs,
    };
    // Filled after construction because the probe asks each target's own driver, which the record
    // above is what resolves. Built in Target::ALL order so the report is deterministic.
    capabilities.c_runtimes = Target::ALL
        .iter()
        .copied()
        .map(|target| CRuntimeStatus::probe(target, capabilities.ref_cc_for(target)))
        .collect();
    Ok(capabilities)
}

/// Locate the compiler under test, or fail hard.
///
/// The override takes precedence over the build-system path so that an externally built binary can
/// be validated without touching the build. When the override is set it is the only candidate: a
/// maintainer who names a binary explicitly must be told that binary is missing rather than be
/// silently given whichever `bcc` happened to be on `PATH`.
///
/// This is the one component whose absence is not degraded around, and the message says so, because
/// a suite that reported success while testing nothing would be worse than a suite that refused to
/// start.
fn resolve_compiler_under_test() -> HarnessResult<ToolRecord> {
    let role = "compiler under test";
    if trimmed_var(VAR_BCC_BIN).is_some() {
        let record = ToolRecord::discover(role, Some(VAR_BCC_BIN), &[]);
        if record.is_available() {
            return Ok(record);
        }
        return Err(compiler_under_test_error(&record.diagnosis()));
    }
    let Some(built) = CARGO_BIN_EXE_BCC
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Err(compiler_under_test_error(
            "the build system provided no CARGO_BIN_EXE_bcc path, which means this test was built \
             in a package that declares no binary target named `bcc`",
        ));
    };
    let candidate = PathBuf::from(built);
    if !is_executable_file(&candidate) {
        return Err(compiler_under_test_error(&format!(
            "the build system named {} as the compiler under test, but no executable file exists \
             at that path; rebuild the package, or point {VAR_BCC_BIN} at a binary that does exist",
            candidate.display()
        )));
    }
    Ok(ToolRecord::from_build_system(
        role,
        Some(VAR_BCC_BIN),
        candidate,
    ))
}

/// Build the hard-failure message for an unlocatable compiler under test.
///
/// Names both resolution mechanisms, states what to set, and explains why this failure is not
/// degraded around, so the message is actionable without opening this file.
fn compiler_under_test_error(reason: &str) -> HarnessError {
    HarnessError::new(
        "locating the compiler under test",
        format!(
            "{reason}. Two mechanisms resolve it, in this order: (1) the {VAR_BCC_BIN} environment \
             variable, which may name an executable directly or a bare name to find on PATH, for \
             example {VAR_BCC_BIN}=./target/release/bcc; (2) the build system's CARGO_BIN_EXE_bcc \
             path, which points at the binary just built and is what guarantees the suite never \
             validates a stale build. Unlike a missing reference compiler or emulator — each of \
             which degrades one oracle arm and is reported as UNAVAILABLE — this is a hard failure: \
             all three oracles compare the observable behaviour of a program the compiler under \
             test produced, so without it there is nothing to test and no result could be trusted"
        ),
    )
}

/// Identify the kernel for the environment fingerprint.
///
/// Never fatal. When the utility is absent or declines to answer, the standard library's own view
/// of the host is recorded instead, so a fingerprint always carries a host identification of some
/// kind.
fn probe_kernel() -> String {
    let resolved = DEFAULT_UNAME
        .iter()
        .find_map(|candidate| resolve_tool(candidate));
    if let Some(uname) = resolved {
        if let Ok(output) = Command::new(&uname).arg("-a").stdin(Stdio::null()).output() {
            if let Some(line) = first_non_empty_line(&output.stdout) {
                return line;
            }
        }
    }
    format!(
        "kernel identification unavailable; the standard library reports this host as {}/{}",
        env::consts::OS,
        env::consts::ARCH
    )
}
