//! Oracle discovery and the capability record.
//!
//! This module is the differential conformance suite's single point of contact with the
//! machine it is running on. It locates the compiler under test, the native reference
//! compiler, the three cross reference drivers and the three emulated-execution runners;
//! it reads the entire environment-variable catalogue; and it emits a capability record
//! that states exactly which arms of which oracles will run.
//!
//! # Availability is decided per target
//!
//! There is deliberately no "skip because unsupported" verdict anywhere in the suite. An arm that
//! cannot be attempted is recorded as [`Verdict::Unavailable`](super::Verdict::Unavailable) and
//! listed individually by [`Capabilities::unavailable_oracle_arms`], so a modest environment can
//! never masquerade as a passing run.
//!
//! [`Capabilities::oracle_a_available`], [`Capabilities::oracle_b_available`] and
//! [`Capabilities::oracle_c_available`] each take a target and answer for that target alone, so no
//! oracle is ever declared dead as a whole. Reporting the surviving arms is strictly more truthful
//! than declaring the oracle gone, and it cannot mask a gap, because every unattemptable arm is
//! enumerated.
//!
//! # Environment-variable catalogue
//!
//! Every variable has a safe default, so the suite runs correctly with none of them set. All
//! configuration reads live in this module so that no two modules can drift on a default.
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
//! | `BCC_CONFORMANCE_ONLY` | unset | Restrict the run to one `<area>/<program>`, validated into a [`ProgramFilter`] |
//! | `BCC_CONFORMANCE_STRICT` | unset | Treat an unavailable oracle as a failure; the intended continuous-integration setting |
//! | `BCC_CONFORMANCE_ALLOW_XPASS` | unset | Downgrade unexpected success from a failure to a warning during a marker-retirement window |
//! | `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` | unset | Explicitly acknowledge a reduced-oracle environment; the gap is still reported, and under [`RunConfig::strict`] it is still a failure |
//! | `BCC_CONFORMANCE_TIMEOUT_SECS` | `30` | Per-cell execution budget |
//! | `BCC_CONFORMANCE_KEEP_WORK` | unset | Retain every cell workspace instead of removing it on success |
//!
//! A boolean variable is true when it is set, non-empty, and not the single character `0`, so a
//! maintainer who exports a flag at all gets the behaviour they were reaching for.
//!
//! # One validated configuration snapshot, read once
//!
//! [`RunConfig`] is the only interpreter of that catalogue and [`RunConfig::read`] the only
//! function that consults the environment for it. Everything downstream — the effective matrix,
//! whether coverage is reduced, whether unexpected success fails, whether an unavailable oracle
//! fails, the per-cell budget and the active program filter — is a method on the snapshot rather
//! than a fresh environment read. Three properties follow:
//!
//! - Every value is validated before any cell runs. Reading configuration is fallible and says so
//!   in its signature: a malformed budget, an unparsable program filter or a value that is not
//!   valid Unicode is a hard error carrying the variable's name, raised by [`discover`] during the
//!   pre-flight check. No configuration path panics and none silently substitutes a default for a
//!   value a maintainer deliberately set.
//! - One run cannot disagree with itself. The snapshot is taken once and cloned, so two
//!   concurrently executing area tests cannot observe different policies, and the report cannot
//!   describe a policy other than the one applied.
//! - Rendering stays infallible. [`Capabilities::render_report`] and
//!   [`Capabilities::render_fingerprint`] read the snapshot they were built with.
//!
//! An invalid value is never treated as absence: a variable holding non-Unicode bytes is rejected
//! by name rather than read as unset, because falling back to a probed default there would test a
//! different tool than the one that was named.
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
//! The table is applied exactly as written, including its second row: an absent native reference
//! compiler takes oracle (a) out on every target, not merely on the native arm, and a surviving
//! cross driver does not rescue a cross arm. The native compiler is the driver both
//! undefined-behaviour audit gates run, and those gates are what establish that a program contains
//! no undefined or unspecified behaviour — the precondition that makes an oracle (a) divergence
//! mean anything at all. Reporting such an arm as available would offer a comparison whose verdict
//! could not be interpreted. Oracles (b) and (c) are untouched, since (b) compares bcc against bcc
//! and (c) against the recorded golden stdout. Availability is still reported per arm by
//! [`Capabilities::unavailable_oracle_arms`], so the whole-oracle rule loses no detail.
//!
//! Under [`RunConfig::strict`] every unavailable oracle becomes a failure. That is the intended
//! continuous-integration setting, where the toolchain is installed deliberately and a missing
//! oracle indicates a broken workflow. Strict mode is therefore dominant: no other variable can
//! lower it. `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` is an acknowledgement, recorded in the report
//! so a deliberately reduced environment says so out loud; it never suppresses the strict
//! escalation, because a job that both demanded strictness and excused missing oracles would
//! report a green run over a matrix it never executed. The policy is exposed through
//! [`RunConfig::unavailable_fails_run`] and [`RunConfig::xpass_fails_run`] and applied by
//! `classify.rs`, so there is exactly one place it can be changed.
//!
//! # Hermeticity
//!
//! This module writes nothing: it reads environment variables, walks `PATH`, calls
//! [`std::fs::metadata`], and spawns short, non-interactive, bounded probes — a tool's version
//! banner, a driver's answer to where it would find a static-link input, and the kernel
//! identification — none of which produces a file. It opens no socket. Everything the wider
//! harness writes lives beneath the Cargo build directory, under [`super::work_root`],
//! [`super::report_root`] and [`super::findings_root`].
//!
//! # Every pre-flight subprocess is bounded
//!
//! Discovery spawns external programs it did not write and cannot vouch for. Any of them may be a
//! wrapper script, a network-mounted binary or a misconfigured driver, and an unbounded wait would
//! hang the pre-flight check itself — the one step whose purpose is to diagnose a misconfigured
//! environment before the matrix runs. Every external process here therefore goes through
//! `run_bounded_probe`, which applies four bounds: standard input is the null device, so a probe
//! that reads sees end of file at once; a fixed wall-clock deadline, after which the child is
//! killed and reaped, deliberately not configurable by the per-cell budget, since identifying a
//! tool is not running a test; a byte cap on each captured stream; and a bounded drain, because
//! both streams are read concurrently on their own threads and collected with a timed receive —
//! reading concurrently is what stops a child that fills a pipe buffer from deadlocking the wait
//! that is supposed to bound it, and the timed receive is what stops a stream held open by
//! something the child left behind from blocking collection.
//!
//! A terminated probe is recorded, not hidden: the tool's reported version carries a stable marker
//! saying its banner probe was terminated, appended to whatever banner it did manage to print, so
//! neither the identification nor the hang is lost; and a runtime-artifact answer from a terminated
//! probe is discarded rather than trusted. The number of probes is fixed by the catalogue rather
//! than by the corpus, so the aggregate bound is a constant.
//!
//! `readelf`, `objdump` and `nm` are deliberately not probed and nothing is conditional on them.
//! The flag probe reads ELF identification bytes directly with the standard library instead, so the
//! suite does not depend on a binary-inspection package being installed.
//!
//! # Compatibility
//!
//! Edition 2021, minimum supported Rust 1.70. [`std::sync::OnceLock`] is the newest
//! standard-library item used here and it stabilized in 1.70; every primitive the bounded probe
//! rests on — [`std::process::Child::try_wait`], [`std::sync::mpsc::Receiver::recv_timeout`],
//! [`std::io::Read::take`] and [`std::thread::spawn`] — has been stable far longer. Every
//! operation is a checked standard-library call, no expression here can panic, and no third-party
//! crate is used.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use super::{
    sanitize_text_for_report, AreaSpec, HarnessError, HarnessResult, OptLevel, Oracle, Target,
};

// Every variable the harness reads is a named constant so that a diagnostic can quote the exact
// spelling a maintainer must export. A "not found" message that does not name its override variable
// forces the reader back into the source; every message below names it.

pub const VAR_BCC_BIN: &str = "BCC_BIN";

pub const VAR_REF_CC: &str = "BCC_REF_CC";

pub const VAR_REF_CC_I686: &str = "BCC_REF_CC_I686";

pub const VAR_REF_CC_AARCH64: &str = "BCC_REF_CC_AARCH64";

pub const VAR_REF_CC_RISCV64: &str = "BCC_REF_CC_RISCV64";

pub const VAR_QEMU_I386: &str = "BCC_QEMU_I386";

pub const VAR_QEMU_AARCH64: &str = "BCC_QEMU_AARCH64";

pub const VAR_QEMU_RISCV64: &str = "BCC_QEMU_RISCV64";

pub const VAR_QUICK: &str = "BCC_CONFORMANCE_QUICK";

pub const VAR_ONLY: &str = "BCC_CONFORMANCE_ONLY";

pub const VAR_STRICT: &str = "BCC_CONFORMANCE_STRICT";

pub const VAR_ALLOW_XPASS: &str = "BCC_CONFORMANCE_ALLOW_XPASS";

pub const VAR_ALLOW_MISSING_ORACLES: &str = "BCC_CONFORMANCE_ALLOW_MISSING_ORACLES";

pub const VAR_TIMEOUT_SECS: &str = "BCC_CONFORMANCE_TIMEOUT_SECS";

pub const VAR_KEEP_WORK: &str = "BCC_CONFORMANCE_KEEP_WORK";

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
/// A dedicated cross driver, not a word-size flag on the native driver: `-m32` was measured to
/// fail on the reference host because the multilib start files are absent, and the GCC drivers this
/// harness uses reject the `--target=` spelling that belongs to a different compiler family. Using
/// the per-target driver also matches how the AArch64 and RISC-V 64 arms work, so all three cross
/// arms resolve the same way.
pub const DEFAULT_REF_CC_I686: &[&str] = &["i686-linux-gnu-gcc"];

pub const DEFAULT_REF_CC_AARCH64: &[&str] = &["aarch64-linux-gnu-gcc"];

pub const DEFAULT_REF_CC_RISCV64: &[&str] = &["riscv64-linux-gnu-gcc"];

/// i686 execution runners, probed in order.
///
/// The runner is named `i386`, never after the target slug `i686`: no emulator carries the slug's
/// spelling. Both the plain and the statically linked form are probed because packagings differ on
/// which they ship, so the harness works against either with no configuration.
pub const DEFAULT_QEMU_I386: &[&str] = &["qemu-i386", "qemu-i386-static"];

pub const DEFAULT_QEMU_AARCH64: &[&str] = &["qemu-aarch64", "qemu-aarch64-static"];

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

const DEFAULT_UNAME: &[&str] = &["uname"];

/// Banner arguments tried in order when capturing a tool's version.
///
/// The usage banner is accepted as a fallback because the compiler under test documents
/// `--help` while not documenting `--version`, and a banner is only ever used to fingerprint
/// the environment. Failing to capture one is recorded as unknown and never fails a run.
const VERSION_ARGUMENTS: &[&str] = &["--version", "--help"];

/// Reference-compiler flag that makes a driver state the target it builds for.
///
/// Used only to verify that the driver selected for an architecture's arm actually serves that
/// architecture. It is a probe flag against the reference compiler, exactly like the flag that asks
/// where a runtime file would be found, and it appears in no differential invocation — so the rule
/// that only flags both compilers honour with the same meaning may be passed to both compilers is
/// not engaged by it.
const DUMP_MACHINE_FLAG: &str = "-dumpmachine";

/// Architecture spellings that all denote the suite's 32-bit x86 target.
///
/// Distributions do not agree here: the same instruction set is reported as `i686`, `i586`, `i486`,
/// `i386`, or `x86` depending on how the driver was configured. Accepting the family keeps a
/// correctly configured machine from being rejected over a naming convention, while still refusing
/// a driver that reports a genuinely different architecture.
const I686_ARCHITECTURE_ALIASES: &[&str] = &["i686", "i586", "i486", "i386", "x86"];

/// Default per-cell execution budget in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Largest per-cell execution budget an interactive run may request, in seconds.
///
/// One hour. Deliberately generous, because a maintainer investigating a single cell by hand has a
/// real reason to raise the budget, and deliberately finite, because a budget that can outlive the
/// job is not a bound at all.
pub const TIMEOUT_SECS_MAX: u64 = 3_600;

/// Largest per-cell execution budget a run under [`RunConfig::strict`] may request, in seconds.
///
/// Five minutes, and not raisable by any variable. Under strict mode the suite is running
/// unattended with a deliberately installed toolchain, so a cell still executing after five
/// minutes — against a measured cost of roughly seventy milliseconds per compile-and-run pair —
/// is hung rather than slow, and detecting that is the timeout's entire purpose.
pub const TIMEOUT_SECS_MAX_STRICT: u64 = 300;

/// Wall-clock deadline for a single environment probe.
///
/// A probe asks a tool to identify itself or to print where it would find a file. Neither is
/// computation, so a tool that has not answered within this window is not slow but stuck, and the
/// probe is abandoned rather than waited on. This bound is independent of the per-cell execution
/// budget above, because probing happens during discovery — before any cell exists to have a
/// budget — which is precisely why an unbounded probe could hang a run before the per-cell
/// timeout had anything to govern.
const PROBE_DEADLINE: Duration = Duration::from_secs(5);

/// Most output a single probe will retain from one stream, in bytes.
///
/// A banner is one line and a printed file name is one path. Sixty-four kilobytes is therefore
/// four orders of magnitude of margin, and it is a hard cap rather than a hint: the reader stops
/// at it, which closes the pipe and causes a tool printing without end to be terminated by the
/// operating system rather than filling this process's memory.
const PROBE_OUTPUT_BYTES_MAX: u64 = 64 * 1024;

/// How often a probe checks whether its child has finished.
///
/// The standard library offers no timed wait on a child process, so the deadline is enforced by
/// polling [`std::process::Child::try_wait`]. Ten milliseconds is short enough that a probe
/// finishing immediately — which every probe in a healthy environment does — is not measurably
/// delayed, and long enough that polling costs nothing.
const PROBE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Static archives and start files whose presence indicates a usable C runtime for a
/// target.
///
/// This is the complete set the suite's static links consume, and it is complete on purpose.
/// The archive supplies the library, and the three start files are the C runtime's
/// initialization sequence: `crt1.o` carries the entry stub that calls `main`, while `crti.o`
/// and `crtn.o` are the prologue and epilogue halves that bracket the initialization and
/// finalization sections. A link is satisfied only when all four are present, which is the
/// linkage contract the project's own technical specification states for every one of the four
/// architectures.
///
/// Checking a subset would be worse than checking nothing, because it would report a runtime as
/// usable while a link against it fails: a missing `crti.o` or `crtn.o` produces an
/// unresolved-symbol or malformed-initialization failure at link time, and a capability report
/// that had already called the runtime healthy would send the reader looking for a compiler
/// defect instead of an absent package. Reporting a gap is cheap; misreporting one is expensive.
///
/// The files are located by asking the target's own reference driver where it would find each
/// of them, which avoids hard-coding a distribution's directory layout — the documented cause of
/// a whole class of cross-compilation surprises.
const C_RUNTIME_ARTIFACTS: &[&str] = &["libc.a", "crt1.o", "crti.o", "crtn.o"];

/// Every variable the catalogue defines, in documentation order.
///
/// Enumerated so that [`validate_environment`] can prove **every** override is readable before
/// a single tool is resolved, rather than discovering an unreadable one at the moment it happens
/// to be consulted. A variable absent from this list would escape that proof, so the list and
/// the catalogue table in the module documentation are kept in step deliberately.
const ALL_VARIABLES: &[&str] = &[
    VAR_BCC_BIN,
    VAR_REF_CC,
    VAR_REF_CC_I686,
    VAR_REF_CC_AARCH64,
    VAR_REF_CC_RISCV64,
    VAR_QEMU_I386,
    VAR_QEMU_AARCH64,
    VAR_QEMU_RISCV64,
    VAR_QUICK,
    VAR_ONLY,
    VAR_STRICT,
    VAR_ALLOW_XPASS,
    VAR_ALLOW_MISSING_ORACLES,
    VAR_TIMEOUT_SECS,
    VAR_KEEP_WORK,
];

/// Read a variable as raw operating-system bytes, treating unset and empty as absent.
///
/// [`std::env::var_os`] rather than [`std::env::var`], and that choice is the whole of this
/// function's purpose. The fallible form returns `Err` for a value that is not valid UTF-8, and
/// discarding that error — which is what `.ok()` does — makes a **set** variable indistinguishable
/// from an **unset** one. The consequences were not cosmetic: an override that named a tool in
/// bytes the platform accepts but Rust cannot decode would silently fall back to the probed
/// defaults, the run would proceed against a tool the maintainer never chose, and the provenance
/// report would state "probed default" with no mention that an override existed at all. Reading
/// the bytes first means the value is always available to be reported even when it cannot be used.
fn present_var_os(name: &str) -> Option<OsString> {
    let raw = env::var_os(name)?;
    if raw.is_empty() {
        return None;
    }
    Some(raw)
}

/// Build the hard failure for a variable whose value is not valid UTF-8.
///
/// The offending value is shown rather than withheld: it is rendered lossily so the maintainer
/// can see which value is at fault, then passed through the report-safety escape so that a
/// variable carrying a terminal escape sequence cannot repaint the diagnostic that rejects it.
/// The byte length is stated too, because the lossy rendering necessarily hides how much of the
/// value was undecodable.
fn non_utf8_error(name: &str, raw: &OsStr) -> HarnessError {
    let shown = sanitize_text_for_report(&raw.to_string_lossy());
    HarnessError::new(
        format!("reading the environment variable {name}"),
        format!(
            "the value is {} bytes that are not valid UTF-8; rendered lossily it reads {shown:?}. \
             It is refused rather than ignored, because ignoring it would make a variable that is \
             set indistinguishable from one that is not: the suite would fall back to its default, \
             run against a tool nobody chose, and report the provenance of that tool as a probed \
             default with no mention of the override. Every value in this catalogue is written \
             into the capability report, the environment fingerprint and the reproduction commands \
             of any finding, all of which are text, so a value that cannot be decoded cannot be \
             recorded faithfully either. Re-export {name} with a value the platform and the report \
             can both represent",
            raw.len()
        ),
    )
}

/// Read a variable as trimmed text, refusing a value that is not valid UTF-8.
///
/// Trimming matters because these variables are frequently set by a shell profile or a
/// continuous-integration expression, where a stray space is easy to introduce and would
/// otherwise turn into an unresolvable tool name. An all-whitespace value is absent.
fn checked_var(name: &str) -> HarnessResult<Option<String>> {
    let Some(raw) = present_var_os(name) else {
        return Ok(None);
    };
    let Some(text) = raw.to_str() else {
        return Err(non_utf8_error(name, &raw));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(String::from(trimmed)))
}

/// Prove every catalogued variable is readable, before anything is resolved or spawned.
///
/// Called as the very first act of discovery. Validating the whole catalogue in one place is what
/// lets the infallible accessors below keep their signatures: by the time any of them is
/// consulted in normal operation, every value has already been proved decodable, so their
/// unreadable branch is unreachable rather than merely unlikely.
fn validate_environment() -> HarnessResult<()> {
    for name in ALL_VARIABLES {
        checked_var(name)?;
    }
    Ok(())
}

/// Infallible view of a text variable, for the accessors whose signature cannot fail.
///
/// An unreadable value yields `None`, exactly as an unset one does. That is safe here and only
/// here, for two reasons that must both hold: [`validate_environment`] has already refused an
/// unreadable value with a precise diagnostic before any cell runs, so this path is unreachable in
/// a real run; and every caller's `None` branch is the documented default, so even on the
/// unreachable path the suite behaves as though the variable had never been set rather than
/// inventing a third behaviour.
fn readable_var(name: &str) -> Option<String> {
    checked_var(name).ok().flatten()
}

/// The boolean convention for every flag in the catalogue: set, non-empty, and not `0`.
///
/// Deliberately permissive about which affirmative spelling is used, and deliberately strict
/// about the one negative spelling that matters, so that exporting `VAR=0` disables a flag
/// rather than enabling it because it happened to be set.
///
/// # Errors
///
/// Propagates the undecodable-value rejection from [`checked_var`]. A flag whose value cannot be
/// read cannot be judged, and guessing at it would defeat the point of setting it.
fn checked_flag(name: &str) -> HarnessResult<bool> {
    Ok(match checked_var(name)? {
        Some(value) => value != "0",
        None => false,
    })
}

/// Parse the per-cell execution budget, rejecting a malformed or unbounded value.
///
/// A malformed value is an error rather than a silent fall back to the default, because
/// silently ignoring a misconfigured timeout would hide exactly the failure the timeout
/// exists to catch: a program that finishes promptly under one compiler and hangs under
/// another. Zero is rejected for the same reason — a zero budget would fail every cell
/// instantly and look like a compiler defect.
///
/// # Why an upper bound is needed at all
///
/// Accepting any positive value is not a bound. A budget of `u64::MAX` seconds is not a long
/// timeout, it is the **absence** of one: the runaway-process protection is gone while the
/// configuration still reads as though it were in force, so a cell that hangs occupies a worker
/// until the job itself is killed by something outside the suite — and a hang is one of the six
/// divergence classes this suite exists to detect, so losing the ability to detect it silently
/// removes a verdict from the taxonomy.
///
/// Two ceilings therefore apply, and they differ because the two situations differ:
///
/// - [`TIMEOUT_SECS_MAX`] bounds an interactive run. It is generous — far beyond any legitimate
///   cell, whose measured cost is a small fraction of a second — because a maintainer stepping
///   through a cell under a debugger or a heavily loaded machine has a real reason to raise it.
/// - [`TIMEOUT_SECS_MAX_STRICT`] bounds a run under [`RunConfig::strict`], which is the intended
///   continuous-integration setting. There the toolchain is installed deliberately and nobody is
///   watching, so a value above this ceiling is a misconfiguration rather than a choice. The
///   ceiling is a constant of this module and cannot be raised by any variable, which is what
///   makes it a bound rather than another default.
fn parse_timeout_secs(strict: bool) -> HarnessResult<u64> {
    let Some(raw) = checked_var(VAR_TIMEOUT_SECS)? else {
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
    let (ceiling, scope) = if strict {
        (
            TIMEOUT_SECS_MAX_STRICT,
            format!(
                "{VAR_STRICT} is set, so the stricter continuous-integration ceiling applies; it \
                 is a constant of the suite and cannot be raised by any variable"
            ),
        )
    } else {
        (
            TIMEOUT_SECS_MAX,
            String::from("this is the ceiling for an interactive run"),
        )
    };
    if parsed > ceiling {
        return Err(HarnessError::new(
            format!("reading the per-cell execution budget from {VAR_TIMEOUT_SECS}"),
            format!(
                "a budget of {parsed} seconds is above the maximum of {ceiling} seconds — {scope}. \
                 A budget large enough to outlive the run is not a long timeout but the absence of \
                 one: the runaway-process bound would be gone while the configuration still read \
                 as though it were in force, and a program that terminates promptly under one \
                 compiler and hangs under another is one of the divergences this suite exists to \
                 detect. A legitimate cell costs a small fraction of a second, so the default of \
                 {DEFAULT_TIMEOUT_SECS} seconds already carries three orders of magnitude of margin"
            ),
        ));
    }
    Ok(parsed)
}

/// The bytes a feature-area directory name and a program stem may contain.
///
/// This is the alphabet the corpus actually uses — `01_integer_conversions`,
/// `004_narrowing_conversions` — widened only by the hyphen, which costs nothing and spares a
/// maintainer a rejection over a spelling the corpus could legitimately adopt.
fn is_filter_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// The one separator between the two halves of a program filter.
const FILTER_SEPARATOR: char = '/';

/// The corpus's feature-area directory names, quoted, for a filter diagnostic.
///
/// Built from the area table rather than written out here, so a corpus that gains or renames an
/// area cannot leave this message describing the corpus it used to be.
fn corpus_area_list() -> String {
    let directories = super::AREAS
        .iter()
        .map(|area| area.directory)
        .collect::<Vec<_>>();
    join_quoted_static(&directories)
}

/// A validated restriction of the run to a single program of a single feature area.
///
/// Parsed from `BCC_CONFORMANCE_ONLY`, whose documented spelling is `<area>/<program>`, for
/// example `04_bitfields/003_compound_assignment`. The value becomes a type rather than
/// travelling as a string so that its shape is established once, where the environment is read,
/// instead of being re-guessed by every consumer that has to act on it.
///
/// # What is validated, and why each rule is here
///
/// - **Exactly one forward slash.** None means no area was named; more than one means a path was
///   supplied where a two-part identifier belongs. Either value fails to denote a program, and
///   accepting it would produce a run that matched nothing while still reporting itself as a run.
/// - **A feature area the corpus actually has.** The area half must appear in
///   [`AREAS`](super::AREAS), and the canonical spelling from that table is what is stored, so
///   the filter echoed into the report is the same string the corpus uses for the directory.
/// - **A canonical identifier on both sides.** Every byte must satisfy
///   [`is_filter_identifier_byte`]. That single rule also disposes of every traversal-shaped
///   value without a special case for any of them: `.` is not in the alphabet, so neither `.`
///   nor `..` can appear; the platform separator is not in the alphabet either, on any host; and
///   no leading, trailing or embedded whitespace survives. A value read from the environment
///   therefore cannot be bent into a path that leaves the corpus.
///
/// # What is deliberately not validated here, and what happens instead
///
/// Whether a program of that name exists. Answering that means reading the corpus, which is
/// `manifest.rs`'s responsibility; reaching into it from the environment module would invert the
/// layering for no gain. A filter naming a program that does not exist yields a run that covers
/// nothing, and that outcome is loud rather than silent: [`RunConfig::is_reduced_run`] holds, so
/// the report is stamped partial, and the driver states the covered-program count — which is
/// zero — in the summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramFilter {
    /// Canonical feature-area directory name, borrowed from the corpus area table so that the
    /// stored spelling can only ever be one the corpus recognises.
    area: &'static str,
    /// Program stem, without the `.c` extension and without any directory component.
    program: String,
}

impl ProgramFilter {
    /// Validate one `<area>/<program>` value.
    ///
    /// # Errors
    ///
    /// Returns an error naming `BCC_CONFORMANCE_ONLY`, quoting the offending value and stating
    /// the expected form, for a value with the wrong number of separators, an empty half, a byte
    /// outside the canonical alphabet, or an area the corpus does not contain. The area error
    /// lists the corpus areas, because "which areas are there" is the question a maintainer who
    /// mistyped one is about to ask.
    pub fn parse(raw: &str) -> HarnessResult<ProgramFilter> {
        let context = format!("reading the program filter from {VAR_ONLY}");
        let separators = raw.matches(FILTER_SEPARATOR).count();
        if separators != 1 {
            return Err(HarnessError::new(
                context,
                format!(
                    "{raw:?} contains {separators} {FILTER_SEPARATOR:?} separators; the filter \
                     names exactly one program as <area>{FILTER_SEPARATOR}<program>, for example \
                     {:?}",
                    "04_bitfields/003_compound_assignment"
                ),
            ));
        }
        let Some((area, program)) = raw.split_once(FILTER_SEPARATOR) else {
            // Unreachable: the count above already established one separator. Handled as an
            // error rather than an unwrap so that this module keeps its property of containing
            // no panicking path at all.
            return Err(HarnessError::new(
                context,
                format!("{raw:?} could not be split at its {FILTER_SEPARATOR:?} separator"),
            ));
        };
        ProgramFilter::require_identifier(&context, raw, "feature area", area)?;
        ProgramFilter::require_identifier(&context, raw, "program", program)?;
        let Some(spec) = AreaSpec::lookup(area) else {
            return Err(HarnessError::new(
                context,
                format!(
                    "{area:?} is not a feature area of this corpus; the areas are {}",
                    corpus_area_list()
                ),
            ));
        };
        Ok(ProgramFilter {
            area: spec.directory,
            program: String::from(program),
        })
    }

    /// Reject a half that is empty or carries a byte outside the canonical alphabet.
    ///
    /// The diagnostic quotes the whole filter as well as the offending half, because a filter is
    /// usually set as one shell word and seeing only half of it back is a poor clue.
    fn require_identifier(context: &str, raw: &str, role: &str, value: &str) -> HarnessResult<()> {
        if value.is_empty() {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the {role} half of {raw:?} is empty; the filter names exactly one program as \
                     <area>{FILTER_SEPARATOR}<program>"
                ),
            ));
        }
        if let Some(byte) = value.bytes().find(|byte| !is_filter_identifier_byte(*byte)) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the {role} half of {raw:?} is {value:?}, which contains the byte {:?}; a \
                     {role} name may contain only letters, digits, {:?} and {:?}, so no filter \
                     can name a path component outside the corpus",
                    char::from(byte),
                    '_',
                    '-'
                ),
            ));
        }
        Ok(())
    }

    /// The canonical feature-area directory name this filter selects.
    pub fn area(&self) -> &'static str {
        self.area
    }

    /// The program stem this filter selects, without extension.
    pub fn program(&self) -> &str {
        self.program.as_str()
    }

    /// True when a discovered program is the one this filter selects.
    ///
    /// Both halves must match exactly. A prefix or substring match would silently widen the
    /// restriction — `001_layout` would select `001_layout_and_size` — and a filter that selects
    /// more than the maintainer asked for is as misleading as one that selects less.
    pub fn matches(&self, area: &str, program: &str) -> bool {
        self.area == area && self.program == program
    }
}

impl fmt::Display for ProgramFilter {
    /// Renders in the same `<area>/<program>` spelling the variable accepts, so a report line
    /// can be copied straight back into an environment assignment.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}{}",
            self.area, FILTER_SEPARATOR, self.program
        )
    }
}

/// Every behavioural setting of one run, read once and validated.
///
/// This is the single interpreter of the environment-variable catalogue. Nothing else in the
/// harness reads a `BCC_CONFORMANCE_*` variable, and no consumer re-derives a policy from one:
/// each question — which matrix, whether coverage is reduced, whether unexpected success fails,
/// whether an unavailable oracle fails, how long a cell may run, which program is selected — is
/// a method on this snapshot.
///
/// The fields are private and every value arrives through [`RunConfig::read`], so an instance
/// cannot exist in an invalid state: the budget is always a positive whole number of seconds and
/// the filter, when present, always names a feature area the corpus contains.
///
/// Cloned into [`Capabilities`] rather than consulted again later. That is what lets the
/// reporting functions stay infallible, and it is what stops two concurrently executing area
/// tests from observing different policies because a variable changed between their reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    quick: bool,
    only: Option<ProgramFilter>,
    strict: bool,
    allow_xpass: bool,
    missing_oracles_acknowledged: bool,
    keep_work: bool,
    timeout_secs: u64,
}

impl RunConfig {
    /// Read and validate the whole catalogue.
    ///
    /// # Errors
    ///
    /// Returns the first configuration defect found, in catalogue order so that a machine with
    /// two misconfigured variables reports the same one first every time: an undecodable value,
    /// a malformed program filter, or a budget that is not a positive whole number of seconds
    /// within the documented ceiling. Every message names the variable that must be corrected.
    ///
    /// Every value arrives through [`checked_var`] or [`checked_flag`], so a variable holding bytes
    /// that are not valid UTF-8 is refused here with the offending value shown rather than being
    /// read as absent — which would silently substitute a default for the setting the maintainer
    /// named.
    pub fn read() -> HarnessResult<RunConfig> {
        let quick = checked_flag(VAR_QUICK)?;
        let only = match checked_var(VAR_ONLY)? {
            Some(raw) => Some(ProgramFilter::parse(&raw)?),
            None => None,
        };
        let strict = checked_flag(VAR_STRICT)?;
        let allow_xpass = checked_flag(VAR_ALLOW_XPASS)?;
        let missing_oracles_acknowledged = checked_flag(VAR_ALLOW_MISSING_ORACLES)?;
        // The strictness is read before the budget because the budget's ceiling depends on it, and
        // reading it here rather than inside the parser keeps this the only interpreter of the
        // catalogue.
        let timeout_secs = parse_timeout_secs(strict)?;
        let keep_work = checked_flag(VAR_KEEP_WORK)?;
        Ok(RunConfig {
            quick,
            only,
            strict,
            allow_xpass,
            missing_oracles_acknowledged,
            keep_work,
            timeout_secs,
        })
    }

    /// True when the matrix is reduced to the native target at `-O0` and `-O2` for fast local
    /// iteration.
    ///
    /// Never the default, and every report produced under it is stamped as reduced coverage, so
    /// a quick run can never be mistaken for a full one.
    pub fn quick_mode(&self) -> bool {
        self.quick
    }

    /// The active program restriction, when one was set.
    pub fn only(&self) -> Option<&ProgramFilter> {
        self.only.as_ref()
    }

    /// True when an unavailable oracle must be escalated to a failure.
    ///
    /// The intended continuous-integration setting: there the toolchain is installed
    /// deliberately, so a missing oracle indicates a broken workflow rather than a modest
    /// machine.
    pub fn strict(&self) -> bool {
        self.strict
    }

    /// True when unexpected success is downgraded from a failure to a warning.
    ///
    /// Exists for a marker-retirement window only. Unexpected success is listed separately and
    /// prominently in the summary either way, so it can never pass unnoticed.
    pub fn allow_xpass(&self) -> bool {
        self.allow_xpass
    }

    /// True when a reduced-oracle environment has been acknowledged explicitly.
    ///
    /// An acknowledgement is a **statement in the report**, not a change of policy. It records
    /// that the operator knows this machine cannot attempt every arm, so a reviewer reading the
    /// summary can tell a deliberately modest environment from an accidentally broken one. It
    /// does not suppress the report of a gap, and — see [`RunConfig::unavailable_fails_run`] —
    /// it does not suppress the strict escalation of one either.
    pub fn missing_oracles_acknowledged(&self) -> bool {
        self.missing_oracles_acknowledged
    }

    /// True when every cell workspace is retained instead of being removed on success.
    pub fn keep_work(&self) -> bool {
        self.keep_work
    }

    /// The validated per-cell execution budget in seconds.
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    /// Whether unexpected success fails the run.
    ///
    /// Fails by default, following the convention that unexpected success is a failure. The
    /// justification is asymmetric cost: a stale marker is stale documented knowledge that will
    /// mislead the next reader, whereas retiring it is a trivial test-only edit.
    pub fn xpass_fails_run(&self) -> bool {
        !self.allow_xpass
    }

    /// Whether an unavailable oracle fails the run.
    ///
    /// Exactly [`RunConfig::strict`], with no other input. Strict mode is **dominant**: it is the
    /// specified continuous-integration setting, where every unavailable oracle becomes a
    /// failure, and nothing may lower it.
    ///
    /// In particular an acknowledgement through `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` does not
    /// enter this decision. Letting it do so would compose the two settings into the one
    /// combination that must not exist: a job that claims strictness while excusing the arms it
    /// could not attempt, reporting a green run over a matrix it never executed. Outside strict
    /// mode an unavailable arm is reported and does not fail, so the acknowledgement has nothing
    /// left to suppress there either — it annotates the report and changes no verdict anywhere.
    pub fn unavailable_fails_run(&self) -> bool {
        self.strict
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
    pub fn effective_matrix(&self) -> (Vec<Target>, Vec<OptLevel>) {
        if self.quick {
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
    pub fn is_reduced_run(&self) -> bool {
        self.quick || self.only.is_some()
    }
}

/// The target the host executes without an emulator, or the oracle (b) baseline when the
/// host architecture matches no supported target.
///
/// The fallback is reported rather than hidden: [`Capabilities::render_report`] states which
/// target quick mode selected and why, so a reduced run on an unexpected host is still
/// self-describing.
fn native_or_baseline_target() -> Target {
    native_target().unwrap_or(Target::BASELINE)
}

/// The supported target the host executes without an emulator, or `None` when it matches none.
///
/// Distinguished from [`native_or_baseline_target`] because the two callers need different
/// answers. Matrix selection needs *a* target and is content with the baseline as a documented
/// fallback. Verifying that the native reference driver targets the host needs the truth: on a
/// host matching no supported target there is no architecture to require, and substituting the
/// baseline would reject a perfectly good driver for not being something it was never asked to be.
fn native_target() -> Option<Target> {
    Target::ALL
        .iter()
        .copied()
        .find(|candidate| candidate.is_native())
}

/// True when a tool name should be treated as an explicit path rather than looked up on
/// `PATH`.
///
/// Both the platform separator and the forward slash are tested so that a path written in
/// the portable spelling is recognised on any host.
fn looks_like_path(name: &str) -> bool {
    name.contains(MAIN_SEPARATOR) || name.contains('/')
}

/// True when `path` names an existing file that carries an execute permission for anyone.
///
/// A coarse heuristic for "this is a program", not an answer about the current user: see
/// [`has_execute_permission`]. Symbolic links are followed, and a wrapper shell script is accepted
/// exactly like a compiled binary, because a reference-compiler installation may legitimately be
/// either. A directory is rejected even when it is executable, since a directory cannot be spawned.
fn is_executable_file(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file() && has_execute_permission(&metadata),
        Err(_) => false,
    }
}

/// Whether any POSIX execute bit — owner, group or other — is set.
///
/// This is deliberately a coarse heuristic for "the file is a program" and **not** a claim that
/// the current user may spawn it: effective-user permission depends on the process's user and
/// groups, on access-control lists and on mount options, none of which are consulted here. A file
/// this accepts may still fail to spawn, and the spawn attempt reports such a failure far more
/// precisely than a permission calculation here could.
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

/// The identity of a resolved tool: which file it actually is, not merely what it is called.
///
/// A name is not an identity. `PATH` order, a symbolic link, or a variable naming one binary in
/// place of another can all make two different names denote the same file, and can make the same
/// name denote a different file a moment later. Recording the file's own identity is what lets the
/// suite answer the two questions a name cannot:
///
/// - **Are these two oracles actually independent?** Oracle (a)'s whole value is that the reference
///   compiler is a *different* implementation. If a variable pointed it at the compiler under test,
///   every comparison would be that compiler against itself — agreement everywhere, no defect ever
///   reported, and nothing in a name-based check would notice. Comparing device and inode numbers
///   settles it regardless of spelling, link, hard link or relative path.
/// - **Which file was the run actually conducted against?** The canonical target is recorded in the
///   environment fingerprint, so a divergence can be attributed to a toolchain that changed
///   underneath a stable name — the case a name alone hides completely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    /// The fully resolved path, with every symbolic link followed.
    pub canonical: PathBuf,
    /// Device and inode numbers, where the platform exposes them.
    ///
    /// `None` on a platform with no such notion, in which case [`ToolIdentity::is_same_file`]
    /// falls back to comparing canonical paths. The distinction is kept explicit rather than
    /// defaulted to zero, because two absent identities must never be reported as equal.
    pub file_id: Option<(u64, u64)>,
    /// True when the name that resolved was a symbolic link.
    ///
    /// Recorded rather than resolved away, because it is a fact the pre-flight report needs: this
    /// environment supplies its emulators under two spellings, one of which is a link, and which
    /// one was selected is exactly what the report exists to state.
    pub via_symlink: bool,
}

impl ToolIdentity {
    /// Capture the identity of the file `path` currently denotes.
    ///
    /// Returns `None` when the path does not denote a regular file at this instant — which includes
    /// the case where it did a moment ago and no longer does. That is not a failure to be papered
    /// over: a tool that vanished between being resolved and being identified cannot be used, and
    /// saying so is better than carrying a path that will fail at the first spawn.
    fn of(path: &Path) -> Option<ToolIdentity> {
        let link = fs::symlink_metadata(path).ok()?;
        let via_symlink = link.file_type().is_symlink();
        // Followed deliberately. A link is a legitimate way to ship a tool — this environment
        // supplies its emulators that way — so the question is not whether a link was used but
        // whether what it points at is a regular executable file.
        let target = fs::metadata(path).ok()?;
        if !target.is_file() {
            return None;
        }
        Some(ToolIdentity {
            canonical: fs::canonicalize(path).ok()?,
            file_id: identity_numbers(&target),
            via_symlink,
        })
    }

    /// True when both identities denote the same file on disk.
    ///
    /// Device and inode numbers are authoritative where the platform provides them, because they
    /// see through every spelling: a relative path, a symbolic link and a hard link all reduce to
    /// the same pair. The canonical-path comparison is the fallback for a platform that exposes no
    /// such numbers, and it is never mixed with the numeric answer, so an absent identity can never
    /// be mistaken for a match.
    pub fn is_same_file(&self, other: &ToolIdentity) -> bool {
        match (self.file_id, other.file_id) {
            (Some(mine), Some(theirs)) => mine == theirs,
            _ => self.canonical == other.canonical,
        }
    }

    /// A one-line description for the pre-flight report and the environment fingerprint.
    pub fn describe(&self) -> String {
        let mut text = shown_path(&self.canonical);
        if let Some((device, inode)) = self.file_id {
            text.push_str(&format!(" (device {device}, inode {inode})"));
        }
        if self.via_symlink {
            text.push_str(" via symbolic link");
        }
        text
    }
}

/// Device and inode numbers of a file, where the platform exposes them.
#[cfg(unix)]
fn identity_numbers(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((metadata.dev(), metadata.ino()))
}

/// Fallback for a platform with no device and inode notion.
///
/// The supported hosts for this suite are Linux, so this arm is never taken there; it exists so the
/// module remains compilable rather than as a behavioural claim about other systems.
#[cfg(not(unix))]
fn identity_numbers(_metadata: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

/// True when anyone may write to this path and the sticky bit does not restrain them.
///
/// The sticky-bit exemption matters: a shared temporary directory is world-writable by design, and
/// the sticky bit is precisely what stops one user replacing another's file there. Treating such a
/// directory as untrusted anyway would be a false positive, while treating a plain world-writable
/// directory as trusted would miss the case that matters — anyone on the machine being able to
/// substitute the compiler the suite is about to run.
#[cfg(unix)]
fn world_writable_without_sticky(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode();
    mode & 0o002 != 0 && mode & 0o1000 == 0
}

/// Fallback for a platform with no POSIX permission model, where no such judgement can be made.
#[cfg(not(unix))]
fn world_writable_without_sticky(_metadata: &fs::Metadata) -> bool {
    false
}

/// Why a resolved tool's location cannot be trusted, or `None` when it can.
///
/// The file and **every directory from it up to the root** are examined, because any one of them is
/// enough to substitute the tool: a world-writable **file** can have its contents replaced, the
/// **directory holding it** can have the file replaced beneath an unchanged name, and any
/// **directory further up** can be renamed so that a different subtree answers to the same path.
/// The second and third are the substitutions a name-based check cannot see, and the third is the
/// one a parent-only check misses — measured here, where a mode-755 directory sat beneath a
/// mode-2777 one and the entire subtree was replaceable while both the file and its own directory
/// looked correct.
///
/// Metadata is taken with the link-following call on purpose. On Linux a symbolic link's own
/// permission bits are always `rwxrwxrwx` and carry no meaning, so inspecting the link rather than
/// its target would report every link as world-writable and reject legitimate tools: two of the
/// directories on this machine's search path are links, as is one of the two spellings under which
/// its emulators are installed.
fn untrusted_reason(path: &Path) -> Option<String> {
    if let Ok(metadata) = fs::metadata(path) {
        if world_writable_without_sticky(&metadata) {
            return Some(String::from(
                "the file itself is writable by any user on this machine and carries no sticky \
                 bit, so its contents can be replaced between this check and the moment it runs",
            ));
        }
    }
    // `ancestors` yields the path itself first, which the file check above has already covered, so
    // the walk starts at the parent and continues to the root.
    for ancestor in path.ancestors().skip(1) {
        let Ok(metadata) = fs::metadata(ancestor) else {
            continue;
        };
        if world_writable_without_sticky(&metadata) {
            let shown = shown_path(ancestor);
            return Some(if Some(ancestor) == path.parent() {
                format!(
                    "its directory {shown} is writable by any user on this machine and carries no \
                     sticky bit, so the file can be replaced beneath an unchanged name"
                )
            } else {
                format!(
                    "the directory {shown} on its path is writable by any user on this machine and \
                     carries no sticky bit, so any directory beneath it can be renamed and \
                     substituted, which replaces the file without the file or its own directory \
                     ever being written to"
                )
            });
        }
    }
    None
}

/// The trustworthy entries of `PATH`, and a note for every entry that was refused.
///
/// Computed once and memoized, because `PATH` belongs to the process and cannot change between two
/// tool lookups within one run; computing it once also means the refusals are reported once rather
/// than once per tool.
///
/// Two kinds of entry are refused, and neither can be a legitimate source for an oracle:
///
/// - **A relative entry**, including the empty entry that a leading, trailing or doubled separator
///   produces and which a shell reads as the current directory. `PATH=.:/usr/bin` makes the tool
///   that runs depend on where the process happens to have been started, so a file named `gcc`
///   dropped into a checkout would be selected as the reference compiler. The suite's own working
///   directory is not guaranteed by the test harness, so such an entry is not merely a hazard but
///   meaningless.
/// - **A world-writable entry with no sticky bit**, where any user on the machine can put a file in
///   place of the tool that was found there.
///
/// An explicit override is deliberately *not* subject to this filter: naming a path outright is a
/// maintainer's stated choice, the catalogue documents a relative override as a supported spelling,
/// and it is made absolute before use. This filter governs only the implicit search.
///
/// Only the entry's own permissions are examined, deliberately, and this is narrower than the
/// ancestor walk [`untrusted_reason`] performs. The two answer different questions and the
/// difference is not an inconsistency:
///
/// * This filter asks *can anyone plant a new file that this search would find?* — which is decided
///   by whether the entry directory itself is writable. A writable ancestor does not let anyone add
///   a file to this directory.
/// * [`untrusted_reason`] asks *can anyone substitute this particular resolved tool?* — and a
///   writable ancestor does exactly that, by renaming a directory and putting another subtree in
///   its place. That question needs the whole chain.
///
/// This filter is also unconditional while the ancestor walk applies under strict mode only, so the
/// narrower rule is the one that always runs and the broader one is reserved for the unattended
/// setting. Between them the coverage is complete: under strict mode a tool resolved from an entry
/// whose ancestor is writable is still refused, because vetting inspects the resolved path.
fn search_path() -> &'static (Vec<PathBuf>, Vec<String>) {
    static SEARCH_PATH: OnceLock<(Vec<PathBuf>, Vec<String>)> = OnceLock::new();
    SEARCH_PATH.get_or_init(|| {
        let mut accepted: Vec<PathBuf> = Vec::new();
        let mut refused: Vec<String> = Vec::new();
        let Some(raw) = env::var_os("PATH") else {
            refused.push(String::from(
                "PATH is not set at all, so no tool can be found by a bare name; every tool must \
                 be named by its environment override",
            ));
            return (accepted, refused);
        };
        for entry in env::split_paths(&raw) {
            let shown = shown_path(&entry);
            if entry.as_os_str().is_empty() {
                refused.push(String::from(
                    "an empty PATH entry (a leading, trailing or doubled separator) was skipped: a \
                     shell reads it as the current directory, which would make the selected tool \
                     depend on where this process was started",
                ));
                continue;
            }
            if !entry.is_absolute() {
                refused.push(format!(
                    "the relative PATH entry {shown:?} was skipped: it resolves against a working \
                     directory the test harness does not guarantee, so a file placed in a checkout \
                     could be selected as an oracle"
                ));
                continue;
            }
            match fs::metadata(&entry) {
                Ok(metadata) if world_writable_without_sticky(&metadata) => {
                    refused.push(format!(
                        "the PATH entry {shown:?} was skipped: it is writable by any user on this \
                         machine and carries no sticky bit, so anyone could place a file there to \
                         be selected as an oracle"
                    ));
                }
                Ok(_) => accepted.push(entry),
                // An unreadable or absent entry is skipped silently: it contributes no candidate
                // and its absence is ordinary rather than suspicious.
                Err(_) => {}
            }
        }
        (accepted, refused)
    })
}

/// Resolve one tool name to an executable file.
///
/// A name containing a separator is treated as an explicit path and checked directly, so an
/// override may point anywhere. A bare name is resolved by walking the trustworthy entries of
/// `PATH` in order and returning the first executable match, which is what a shell would do —
/// without spawning a shell, because no shell is needed here and spawning one only adds a failure
/// mode, and without the entries [`search_path`] refuses.
fn resolve_tool(name: &str) -> Option<PathBuf> {
    if looks_like_path(name) {
        let candidate = PathBuf::from(name);
        return is_executable_file(&candidate).then(|| absolutize(candidate));
    }
    let (accepted, _) = search_path();
    accepted
        .iter()
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

/// Appended to a tool's recorded version when its banner probe had to be terminated.
///
/// A suffix rather than a replacement, so that a tool which printed its banner and *then* hung is
/// recorded as both: the identification is kept, because it is genuinely useful for attributing a
/// later divergence to toolchain drift, and the hang is stated, because a driver that cannot
/// answer promptly is a fact about this machine that a reviewer must see. Replacing the banner
/// would throw away real information; omitting the suffix would let a fingerprint read exactly
/// like a healthy machine's while the driver was in fact unresponsive.
///
/// Fixed text, for the same reason every other marker in this module is fixed: two fingerprints
/// taken on the same machine must compare equal, or a fingerprint is useless for the comparison it
/// exists to support.
const PROBE_TERMINATED_SUFFIX: &str =
    " [banner probe terminated after exceeding the pre-flight budget]";

/// Stands in for the banner text when a terminated probe printed nothing at all.
///
/// Kept separate from [`ToolRecord::version_or_unknown`]'s marker on purpose: that one means "the
/// tool answered and said nothing", this one means "the tool never answered", and the two call for
/// entirely different investigations.
const PROBE_NO_BANNER: &str = "version unknown";

/// The first non-empty line of a captured byte stream, trimmed and made safe to print.
///
/// Bytes are converted lossily because a banner is only ever displayed, never parsed, and a
/// tool that emits a stray non-UTF-8 byte should still be identifiable rather than reported
/// as unknown.
///
/// The line is then passed through [`sanitize_text_for_report`], **at capture time rather than at
/// emit time**, and that placement is the point. This text is the one value in the whole capability
/// record that an arbitrary external executable chooses: a banner is whatever the selected tool
/// prints. It flows into the pre-flight report, the environment fingerprint of every finding, and
/// the tab-separated summary. A tab in it forges a summary column and can therefore relabel a
/// verdict; a carriage return erases the line it ends; an escape introducer opens a terminal
/// sequence that can hide a FINDING row or repaint it as a PASS; a directional override can make a
/// line render as something other than what it says. Escaping once, here, means every consumer is
/// safe without having to remember to be — and there is no second path by which raw banner bytes
/// can reach a report.
fn first_non_empty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(sanitize_text_for_report)
}

/// What one bounded probe captured.
struct ProbeCapture {
    /// Standard output, truncated at [`PROBE_OUTPUT_BYTES_MAX`].
    stdout: Vec<u8>,
    /// Standard error, truncated at [`PROBE_OUTPUT_BYTES_MAX`].
    stderr: Vec<u8>,
    /// True when the child had to be terminated because it outlived [`PROBE_DEADLINE`].
    timed_out: bool,
}

/// Run one probe with a deadline, a capped output and guaranteed cleanup.
///
/// Every probe in this module goes through this function, and none of them may spawn a child any
/// other way. That single funnel is the fix for a real hazard: a probe target is not always a
/// trusted tool. Three of them are named by environment variables a maintainer or a continuous-
/// integration expression can set, so the executable being asked `--version` may be any program on
/// the machine — including one that loops without printing, prints without end, or waits for input
/// forever. Before this bound existed such a program could hang or exhaust discovery, and it would
/// do so *before* the per-cell execution budget existed to govern anything, so the suite's own
/// runaway protection could never engage.
///
/// Five properties hold together, and each closes a distinct way a probe can fail to return:
///
/// - **Standard input is the null device**, so any read of it sees end of file immediately and a
///   tool that waits for a maintainer to type cannot block.
/// - **Both output streams are read by their own thread, each bounded with
///   [`std::io::Read::take`]** at [`PROBE_OUTPUT_BYTES_MAX`]. This is what makes an endless printer
///   harmless, and it is self-limiting rather than merely truncating: when a reader reaches its cap
///   it drops the pipe, the pipe fills, and the operating system terminates the writer. Reading on
///   separate threads is also what prevents the classic deadlock in which a parent waits for a
///   child that is itself blocked writing into a full pipe.
/// - **The wait is bounded** by polling [`std::process::Child::try_wait`] until the deadline,
///   because the standard library offers no timed wait.
/// - **The child is always reaped.** On the deadline it is killed and then waited on, so no zombie
///   is left behind.
/// - **Collecting the output is bounded by the same deadline as the wait**, and this is the
///   property that is easy to omit and fatal to omit. Terminating a child does not close the pipe
///   it was writing to: any process that inherited the write end still holds it open, and a shell
///   script is the ordinary case — killing the shell leaves the `sleep` it had started alive, still
///   holding the pipe. A reader draining that pipe to end of file therefore blocks on the
///   *grandchild*, not on the child, so joining the reader threads would make the probe hostage to
///   a process it never started and cannot see. Measured before this bound existed: a five-second
///   deadline produced a sixty-second probe. The readers are consequently harvested through a
///   channel with the remaining time as its timeout, and a reader that has not delivered by then is
///   abandoned rather than waited on. Abandoning it is safe and bounded: its buffer cannot exceed
///   the cap, it holds no lock, and it ends by itself when the last writer finally closes the pipe.
///
/// Returns `None` only when the child could not be spawned at all. A probe that ran and timed out
/// returns its capture with [`ProbeCapture::timed_out`] set, because "the tool did not terminate"
/// is a fact about the environment worth recording rather than an absence to be silently ignored.
fn run_bounded_probe(command: &mut Command) -> Option<ProbeCapture> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // One deadline governs the wait and both harvests, so the whole probe is bounded rather than
    // each of its three phases being bounded separately and summing to three times the budget.
    let deadline = Instant::now() + PROBE_DEADLINE;
    let stdout_reader = child.stdout.take().map(spawn_capped_reader);
    let stderr_reader = child.stderr.take().map(spawn_capped_reader);
    let timed_out = await_child_within_deadline(&mut child, deadline);
    let stdout = harvest_within_deadline(stdout_reader, deadline);
    let stderr = harvest_within_deadline(stderr_reader, deadline);
    Some(ProbeCapture {
        stdout,
        stderr,
        timed_out,
    })
}

/// Drain one output stream on its own thread, retaining at most [`PROBE_OUTPUT_BYTES_MAX`] bytes.
///
/// Delivers through a channel rather than a join handle so that the caller can give up on it. A
/// join handle offers no timed wait, so holding one would force the caller to block until the
/// stream reached end of file — which, as [`run_bounded_probe`] explains, is an event a terminated
/// child does not guarantee.
///
/// A read error is discarded deliberately: the stream belongs to a probe whose only product is an
/// identification string, so a partial capture is worth exactly as much as the bytes it contains
/// and a failed read is indistinguishable from a tool that printed nothing. A send failure is
/// discarded for the same reason — it means the probe has already given up and moved on.
fn spawn_capped_reader<R>(stream: R) -> Receiver<Vec<u8>>
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer: Vec<u8> = Vec::new();
        let mut bounded = stream.take(PROBE_OUTPUT_BYTES_MAX);
        let _ = bounded.read_to_end(&mut buffer);
        let _ = sender.send(buffer);
    });
    receiver
}

/// Take a reader's bytes if they arrive by the deadline, and nothing if they do not.
///
/// The deadline may already have passed, in which case the remaining duration is zero and this
/// becomes a non-blocking poll — which is exactly right after a timeout: bytes already delivered
/// are kept, and nothing is waited for.
fn harvest_within_deadline(reader: Option<Receiver<Vec<u8>>>, deadline: Instant) -> Vec<u8> {
    let Some(receiver) = reader else {
        return Vec::new();
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    receiver.recv_timeout(remaining).unwrap_or_default()
}

/// Wait for a probe's child until the deadline, killing and reaping it if it outlives one.
///
/// Returns true when the deadline was reached and the child had to be terminated. An error from
/// [`std::process::Child::try_wait`] means the child's status can no longer be observed, so waiting
/// longer cannot help; the child is terminated and reaped on that path too, which is what
/// guarantees this function never leaves a process behind however it exits.
fn await_child_within_deadline(child: &mut Child, deadline: Instant) -> bool {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return false,
            Ok(None) => {}
            Err(_) => {
                terminate_and_reap(child);
                return false;
            }
        }
        if Instant::now() >= deadline {
            terminate_and_reap(child);
            return true;
        }
        thread::sleep(PROBE_POLL_INTERVAL);
    }
}

/// Kill a child and wait for it, ignoring both results.
///
/// Both results are ignored on purpose. A kill fails when the child has already exited, and a wait
/// fails when it has already been reaped; either way the postcondition this function exists to
/// establish — that no process of ours is still running and none is left unreaped — already holds.
fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Capture a tool's banner for the environment fingerprint.
///
/// Tries `--version` and then `--help`, accepting the first argument that produces any
/// output on either stream. Four deliberate decisions:
///
/// - **Standard error is accepted as well as standard output**, because tools disagree about
///   where a banner belongs and the fingerprint only needs the text.
/// - **The exit status is ignored.** A usage banner may legitimately exit non-zero while
///   still printing exactly the identification wanted, and the compiler under test documents
///   `--help` rather than `--version`.
/// - **Every invocation is bounded** by [`run_bounded_probe`], which supplies the null device on
///   standard input, caps each captured stream and terminates a child that outlives the probe
///   deadline. This matters more here than anywhere else in the module, because the executable being
///   asked to identify itself may have been named by an environment variable and is therefore not
///   necessarily a compiler at all: a wrapper script, a network-mounted driver or a misconfigured
///   override cannot hang the pre-flight check that exists to diagnose exactly such an environment.
/// - **A terminated attempt stops the sequence and is stated in the result.** A tool that hangs on
///   `--version` will almost certainly hang on `--help`, so trying the second argument would
///   double the delay to learn the same fact; stopping keeps the whole pre-flight cost bounded at
///   one budget per tool. Whatever the tool managed to print is kept and
///   [`PROBE_TERMINATED_SUFFIX`] is appended to it, so the hang is never flattened into "no
///   banner" and never hidden behind a banner that arrived before the tool stopped responding.
///   "This tool declined to identify itself" and "this tool had to be killed" describe different
///   environments, and a fingerprint that conflated them would mislead exactly when it mattered.
///
/// Returns `None` when the tool could be asked, answered promptly and produced nothing, which is
/// recorded as an unknown version and never fails a run.
fn probe_version(path: &Path) -> Option<String> {
    for argument in VERSION_ARGUMENTS {
        let Some(capture) = run_bounded_probe(Command::new(path).arg(argument)) else {
            continue;
        };
        let banner =
            first_non_empty_line(&capture.stdout).or_else(|| first_non_empty_line(&capture.stderr));
        if capture.timed_out {
            let identification = banner.unwrap_or_else(|| String::from(PROBE_NO_BANNER));
            return Some(format!("{identification}{PROBE_TERMINATED_SUFFIX}"));
        }
        if let Some(line) = banner {
            return Some(line);
        }
    }
    None
}

/// How a tool came to be located, or why it could not be.
///
/// Recorded so that the pre-flight report can state the mechanism rather than only the
/// result: "the override said so" and "the second probed default matched" are very different
/// facts when an environment behaves unexpectedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolSource {
    EnvironmentOverride,
    /// The build system supplied the path to the binary it had just built. Applies to the
    /// compiler under test alone, and it is what guarantees the suite never validates a stale
    /// build.
    CargoBinary,
    ProbedDefault,
    Unresolved,
}

impl ToolSource {
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
    pub source: ToolSource,
    /// Absolute or `PATH`-resolved path to the executable, when one was found.
    pub path: Option<PathBuf>,
    /// First line of the tool's banner, when one could be captured. `None` means the tool
    /// exists but declined to identify itself, which is recorded rather than treated as an
    /// error.
    pub version: Option<String>,
    /// Which file the resolved path actually denotes, captured once at discovery.
    ///
    /// Present exactly when [`ToolRecord::path`] is, and it is what makes oracle independence
    /// checkable: a name says what a tool is called, an identity says which file it is.
    pub identity: Option<ToolIdentity>,
    /// Why an otherwise-executable candidate was refused, when one was.
    ///
    /// A refusal is recorded rather than being allowed to look like an absent package, and it
    /// leaves [`ToolRecord::path`] empty so the affected oracle arm is reported unavailable — which
    /// under strict mode is escalated to a failure. That composition is deliberate: a tool the
    /// suite declines to trust must not quietly become a tool the suite silently did without.
    pub rejection: Option<String>,
    /// What the tool says it targets, for a driver that can be asked.
    ///
    /// Recorded for the report and the fingerprint. A driver whose stated target contradicts the
    /// arm it was selected for is refused through [`ToolRecord::rejection`], because a reference
    /// compiler that builds for the wrong architecture is not a weaker oracle but a false one.
    pub provenance: Option<String>,
}

impl ToolRecord {
    /// Discover one tool from its override variable and its probe list.
    ///
    /// When the override is set, it is the only candidate: a maintainer who names a tool
    /// explicitly must be told that that tool is missing, not silently given a different one
    /// that happened to be on `PATH`. When it is unset, the defaults are tried in order.
    ///
    /// Infallible by construction. A tool that simply could not be found is **not** an error: it
    /// produces an unresolved record carrying its own diagnosis, which is what lets the environment
    /// degrade and report rather than refuse to start. An undecodable override *is* a configuration
    /// defect — honouring it as absence would probe a default and then attribute the results to the
    /// tool the maintainer named — but it is refused earlier, by [`validate_environment`], which
    /// proves every catalogued variable readable before the first tool is resolved. By the time this
    /// function runs there is no unreadable value left to report, so it needs no failure path.
    ///
    /// `expected_target` is the architecture this tool is being selected to serve, when there is one
    /// to demand: a reference cross driver must state that target, whereas an emulator states no
    /// target at all. It is passed to [`vet_resolved_tool`], which refuses a driver that contradicts
    /// it. `strict` carries the run's strictness so that vetting can apply the location checks that
    /// only an unattended run needs, without this module reading the environment a second time.
    fn discover(
        role: impl Into<String>,
        override_variable: Option<&'static str>,
        defaults: &'static [&'static str],
        expected_target: Option<Target>,
        strict: bool,
    ) -> ToolRecord {
        let (candidates, overridden) = match override_variable.and_then(readable_var) {
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
        let identity = resolved.as_deref().and_then(ToolIdentity::of);
        let (rejection, provenance) = vet_resolved_tool(
            resolved.as_deref(),
            identity.as_ref(),
            expected_target,
            strict,
        );
        // A refused candidate is not carried as a usable path. Clearing it here, in one place, is
        // what makes every downstream availability question — which arms run, which cells are
        // unavailable, whether strict mode escalates — answer correctly without any of them having
        // to know that vetting exists.
        let accepted = match rejection {
            Some(_) => None,
            None => resolved,
        };
        let source = match (&accepted, overridden) {
            (Some(_), true) => ToolSource::EnvironmentOverride,
            (Some(_), false) => ToolSource::ProbedDefault,
            (None, _) => ToolSource::Unresolved,
        };
        let version = accepted.as_deref().and_then(probe_version);
        ToolRecord {
            role: role.into(),
            override_variable,
            defaults,
            candidates,
            overridden,
            source,
            identity: accepted.as_ref().and(identity),
            rejection,
            provenance,
            path: accepted,
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
        let identity = ToolIdentity::of(&path);
        ToolRecord {
            role: role.into(),
            override_variable,
            defaults: &[],
            candidates: vec![path.display().to_string()],
            overridden: false,
            source: ToolSource::CargoBinary,
            identity,
            // Nothing to vet: this path came from the build system rather than from a search or a
            // variable, and it is the compiler under test, whose target is chosen per cell rather
            // than fixed by the driver.
            rejection: None,
            provenance: None,
            path: Some(path),
            version,
        }
    }

    pub fn is_available(&self) -> bool {
        self.path.is_some()
    }

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

    pub fn summary(&self) -> String {
        match &self.path {
            Some(path) => format!(
                "{} via {} -> {} [{}]",
                self.role,
                self.source.label(),
                shown_path(path),
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
        // A refused tool was found and then declined. Reporting it as though nothing had been
        // located would send a maintainer to install a package that is already installed, so the
        // refusal speaks first and in its own words.
        if let Some(reason) = &self.rejection {
            return reason.clone();
        }
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

/// Decide whether a resolved candidate may be used, and record what it says it targets.
///
/// Returns `(rejection, provenance)`. A `Some` rejection means the candidate was found and then
/// declined, which leaves the tool unavailable with an explanation rather than silently trusted.
/// Three conditions are checked, in the order that produces the most useful diagnostic:
///
/// 1. **It must still be the file it was a moment ago.** Resolution and use are two separate
///    instants, and a path that resolved but can no longer be identified as a regular file has
///    changed underneath the search — a dangling link, a replaced entry, or a deletion. This is the
///    time-of-check-to-time-of-use window itself, and while no check can close it entirely, the
///    identity captured here is what the run is conducted and reported against.
/// 2. **Under `strict` its location must be trustworthy.** Strict mode is the unattended
///    continuous-integration setting, where a world-writable tool or directory is a real
///    substitution opportunity and nobody is watching. Interactive runs are deliberately exempt: a
///    maintainer building a compiler into a scratch directory is doing something ordinary, and
///    refusing it would make the suite unusable for the person most likely to run it.
/// 3. **A driver must target the arm it was chosen for.** Asked with the reference compiler's own
///    target-reporting flag — a probe flag, never a differential one — a cross driver states its
///    target, and a driver that states the wrong one is refused. This closes the case a name cannot
///    catch: pointing an architecture's override at the native compiler would produce an oracle
///    that compiled for the host while claiming to compile for that architecture, and the arm would
///    then be a false authority rather than a missing one. A driver that declines to answer is
///    accepted with its silence recorded, because the flag is a convention rather than a guarantee.
fn vet_resolved_tool(
    resolved: Option<&Path>,
    identity: Option<&ToolIdentity>,
    expected_target: Option<Target>,
    strict: bool,
) -> (Option<String>, Option<String>) {
    let Some(path) = resolved else {
        return (None, None);
    };
    let shown = shown_path(path);
    if identity.is_none() {
        return (
            Some(format!(
                "{shown} resolved but could not be identified as a regular file when its identity \
                 was captured, so it changed between being found and being inspected — a dangling \
                 symbolic link, a replaced entry, or a deletion. It is refused rather than used, \
                 because a tool that has already changed once cannot be relied on to be the same \
                 file at the moment it runs"
            )),
            None,
        );
    }
    if strict {
        if let Some(reason) = untrusted_location(path, identity) {
            return (
                Some(format!(
                    "{shown} was found but is refused because {VAR_STRICT} is set and {reason}. \
                     Strict mode is the unattended continuous-integration setting, where the \
                     toolchain is installed deliberately and a tool anyone could substitute is a \
                     misconfiguration rather than a choice. Install the tool in a location only \
                     privileged users can write, or unset {VAR_STRICT} for an interactive run"
                )),
                None,
            );
        }
    }
    let Some(target) = expected_target else {
        return (None, None);
    };
    match probe_dumpmachine(path) {
        Some(machine) if machine_targets(&machine, target) => {
            (None, Some(format!("reports target {machine}")))
        }
        Some(machine) => (
            Some(format!(
                "{shown} was found but reports that it targets {machine}, while it was selected as \
                 the driver for {target}. A reference compiler that builds for a different \
                 architecture than the arm it serves is not a weaker oracle but a false one: every \
                 comparison on that arm would be against a program built for something else. Point \
                 the override for this arm at a driver that targets {target}, or unset it to probe \
                 the documented defaults"
            )),
            Some(format!("reports target {machine}")),
        ),
        None => (
            None,
            Some(String::from(
                "target not stated (the driver did not answer the target-reporting flag)",
            )),
        ),
    }
}

/// Report why a tool's location cannot be trusted, examining every path that reaches its bytes.
///
/// Both chains are checked when a symbolic link is involved, because either one is sufficient to
/// substitute the tool and neither implies the other:
///
/// * The **name that was resolved** is what the suite will execute. If any directory on its chain
///   is world-writable, the link itself can be repointed at another program.
/// * The **file the name leads to** is what actually runs. A link in a perfectly safe directory can
///   point into a world-writable one, and checking only the name would accept it — the link's own
///   permissions are always `rwxrwxrwx` and say nothing about its target.
///
/// Checking one chain and not the other would leave a complete substitution path open, so the tool
/// is refused if either chain is untrustworthy.
fn untrusted_location(path: &Path, identity: Option<&ToolIdentity>) -> Option<String> {
    if let Some(reason) = untrusted_reason(path) {
        return Some(reason);
    }
    let canonical = &identity?.canonical;
    if canonical == path {
        return None;
    }
    let reason = untrusted_reason(canonical)?;
    Some(format!(
        "it resolves through a symbolic link to {}, and {reason}",
        shown_path(canonical)
    ))
}

/// Ask a compiler driver which target it builds for.
///
/// The flag is a reference-compiler probe flag and appears in no differential invocation, so the
/// discipline that only flags both compilers honour identically may be passed to both compilers is
/// untouched — the same justification that already applies to the static-link input probe.
fn probe_dumpmachine(driver: &Path) -> Option<String> {
    let capture = run_bounded_probe(Command::new(driver).arg(DUMP_MACHINE_FLAG))?;
    if capture.timed_out {
        return None;
    }
    first_non_empty_line(&capture.stdout)
}

/// True when a driver's reported machine describes `target`.
///
/// The architecture component is compared rather than the whole triple, because the remainder is
/// spelled differently by different distributions and vendors while the architecture is the part
/// that decides what the driver actually emits. The measured drivers on this machine report exactly
/// the suite's own triples, so this comparison is a tolerance rather than a necessity — but it is
/// the tolerance that keeps the check from failing on a correctly configured machine that spells
/// its triples another way.
fn machine_targets(machine: &str, target: Target) -> bool {
    if machine == target.triple() {
        return true;
    }
    let Some(architecture) = machine.split('-').next() else {
        return false;
    };
    if architecture == target.short_name() {
        return true;
    }
    // The 32-bit x86 family is the one target with several accepted architecture spellings, and a
    // driver may legitimately report any of them for the same instruction set.
    target == Target::I686 && I686_ARCHITECTURE_ALIASES.contains(&architecture)
}

/// Render a path for a report line, with every hostile character escaped.
///
/// The single place in this module where a path becomes report text. A path here is not trusted
/// input: it arrives from an environment override or from a `PATH` directory listing, and a file
/// name may legally contain any byte except the separator and NUL — including a line feed, an
/// escape sequence, or a bidirectional override. Rendered raw into a report, such a name could
/// forge a verdict line, repaint the terminal, or reverse the apparent reading order of the text
/// around it.
///
/// Escaping happens through the shared helper rather than a local rule, so this module, the record
/// parser and the module root can never disagree about which characters must not appear literally.
fn shown_path(path: &Path) -> String {
    sanitize_text_for_report(&path.display().to_string())
}

/// Render owned strings as a quoted, comma-separated list for diagnostics.
///
/// Quoting is by the standard library's debug formatting, which escapes every control character and
/// every bidirectional and separator format character — the same class [`shown_path`] escapes — so
/// a candidate name taken verbatim from an environment variable cannot forge a report line either.
fn join_quoted(values: &[String]) -> String {
    let quoted = values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>();
    quoted.join(", ")
}

fn join_quoted_static(values: &[&str]) -> String {
    let quoted = values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>();
    quoted.join(", ")
}

/// Whether one static-link input could be located for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CRuntimeArtifact {
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
    pub target: Target,
    pub driver: Option<PathBuf>,
    pub artifacts: Vec<CRuntimeArtifact>,
}

impl CRuntimeStatus {
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

    pub fn appears_usable(&self) -> bool {
        self.driver.is_some() && self.artifacts.iter().all(|entry| entry.resolved.is_some())
    }

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
/// The probe is bounded by [`run_bounded_probe`], and a **terminated** probe is treated as no
/// answer even if it printed something first. A driver that had to be killed mid-answer has not
/// told us where the file is; trusting a partial line from it could record a runtime as usable on
/// the strength of a truncated path, which is the one outcome this probe exists to prevent.
///
/// This flag is used against a reference driver in a probe only. It never appears in a
/// differential invocation, so the discipline that only flags both compilers honour identically
/// may be passed to both compilers is untouched.
/// The probe is bounded by [`run_bounded_probe`], and a driver that does not terminate is treated
/// as having failed to locate the file rather than being waited on: the answer is a report-quality
/// signal, so no answer degrades one line of the pre-flight report and nothing else.
fn probe_runtime_artifact(driver: &Path, artifact: &str) -> Option<PathBuf> {
    let capture =
        run_bounded_probe(Command::new(driver).arg(format!("-print-file-name={artifact}")))?;
    if capture.timed_out {
        return None;
    }
    let printed = first_non_empty_line(&capture.stdout)?;
    // The captured line has already been escaped for reporting. A path containing a character that
    // escaping would rewrite is therefore not this path, and treating it as one would name a file
    // that does not exist; the containment check below settles it either way, because an escaped
    // spelling cannot resolve to a regular file.
    let candidate = PathBuf::from(printed);
    if !candidate.is_absolute() {
        return None;
    }
    // `symlink_metadata` first, so that a dangling link is refused as such rather than through the
    // failure of a later open, and `metadata` afterwards, because a distribution may legitimately
    // ship a start file or an archive as a link to a versioned name and following it is correct.
    if fs::symlink_metadata(&candidate).is_err() {
        return None;
    }
    match fs::metadata(&candidate) {
        Ok(metadata) if metadata.is_file() => Some(candidate),
        Ok(_) | Err(_) => None,
    }
}

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
/// The value itself carries no interior mutability, so once [`discover`] has initialized it a
/// clone can be handed to every concurrently executing test without further synchronization and
/// with no test able to disturb another's view.
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
    pub runner_aarch64: ToolRecord,
    pub runner_riscv64: ToolRecord,
    /// External per-cell timeout utility. Optional: execution falls back to a watchdog thread.
    pub timeout_tool: ToolRecord,
    pub reducer: ToolRecord,
    pub c_runtimes: Vec<CRuntimeStatus>,
    /// Kernel identification, or an explanatory substitute when it could not be obtained.
    pub kernel: String,
    pub host_arch: &'static str,
    pub host_os: &'static str,
    /// The validated configuration snapshot this run is operating under.
    ///
    /// Held here rather than re-read on demand so that every question about policy — the effective
    /// matrix, whether coverage is reduced, whether unexpected success or an unavailable oracle
    /// fails the run, the per-cell budget, the active program filter — is answered from one
    /// validated value. Two consequences follow: the reporting functions cannot fail on a
    /// configuration problem, and two concurrently executing area tests cannot describe different
    /// policies.
    pub config: RunConfig,
    /// `PATH` entries that were skipped during tool resolution, each with the reason.
    ///
    /// Surfaced in the capability report rather than discarded, because a skipped entry changes
    /// which tool a name resolves to. Silently ignoring a directory the maintainer put on `PATH`
    /// would make an absent tool look like a missing package when it is actually installed
    /// somewhere the suite declined to search — a diagnosis that would send them looking in
    /// entirely the wrong place.
    pub path_rejections: Vec<String>,
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
    /// Never accompanied by a word-size or target-selection flag: the GCC drivers this harness uses
    /// reject the `--target=` spelling, and `-m32` was measured to fail on the reference host
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
    /// Three conditions, all required, in the order the degradation table imposes them:
    ///
    /// 1. The native reference compiler is present. This gates every arm, not only the native one,
    ///    because the native compiler is the driver both undefined-behaviour audit gates run, and
    ///    those gates are what establish that a program is free of undefined and unspecified
    ///    behaviour. Without that precondition a difference between two compilers proves nothing
    ///    about either, so a cross arm with its own driver present would produce a comparison whose
    ///    verdict could not be interpreted. An absent native compiler therefore takes oracle (a)
    ///    out entirely, while leaving oracles (b) and (c) running in full.
    /// 2. A reference compiler that targets `target` is present, since the reference side selects a
    ///    target by choosing a different driver binary.
    /// 3. The result can be executed here, because the comparison is of what the program did. A
    ///    missing emulator therefore takes the affected target out of oracle (a)'s cross arm as
    ///    well as out of oracle (b).
    pub fn oracle_a_available(&self, target: Target) -> bool {
        self.ref_cc_native.is_available()
            && self.ref_cc_for(target).is_some()
            && self.can_execute(target)
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
        let (targets, _) = self.config.effective_matrix();
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
            // The native driver is reported first and for every target, because its absence
            // removes the whole oracle rather than one arm: it is the driver both audit gates
            // run, so without it no program's freedom from undefined behaviour has been
            // established and no oracle (a) comparison could be interpreted. A reader told only
            // that "the aarch64 cross driver is present" would be left wondering why the arm is
            // unavailable.
            Oracle::ReferenceCompiler if !self.ref_cc_native.is_available() => format!(
                "the native reference compiler is unavailable, which takes oracle (a) out for \
                 every target including {target}: it is the driver both undefined-behaviour audit \
                 gates run, and without those gates a divergence could not be attributed to the \
                 compiler under test. {}. Oracles (b) and (c) are unaffected and still run",
                self.ref_cc_native.diagnosis()
            ),
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
    /// Coverage is reported as the enumerable matrix rather than as a ratio, because no ratio can
    /// be measured without instrumentation the project's dependency rule forbids.
    ///
    /// The rendering is a pure function of the record and its configuration snapshot: no clock,
    /// no process identifier, no map iteration and no fresh environment read, so two runs on the
    /// same machine produce byte-identical text and the report is unaffected by how many threads
    /// the test harness uses.
    pub fn render_report(&self) -> String {
        let (targets, opt_levels) = self.config.effective_matrix();
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

        if !self.path_rejections.is_empty() {
            lines.push(String::from(
                "Search-path entries skipped during tool resolution",
            ));
            for rejection in &self.path_rejections {
                lines.push(format!("  {rejection}"));
            }
            lines.push(String::from(
                "  consequence: a tool installed only in a skipped directory is reported as \
                 absent. If a tool below is unexpectedly missing, check whether it lives in one of \
                 these",
            ));
            lines.push(String::new());
        }

        lines.push(String::from("Host"));
        lines.push(format!(
            "  architecture {}, operating system {}",
            self.host_arch, self.host_os
        ));
        lines.push(format!("  kernel {}", self.kernel));
        lines.push(format!(
            "  per-cell execution budget {} s (override with {})",
            self.config.timeout_secs(),
            VAR_TIMEOUT_SECS
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
        if self.config.is_reduced_run() {
            lines.push(String::from(
                "  coverage: REDUCED — this run does not cover the full matrix and its report is \
                 stamped partial",
            ));
            if self.config.quick_mode() {
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
            if let Some(filter) = self.config.only() {
                lines.push(format!(
                    "    {VAR_ONLY} is set: restricted to {filter}, so this run covers one \
                     program of one area"
                ));
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
                "  policy: an unavailable arm {} the run (set {} to escalate)",
                if self.config.unavailable_fails_run() {
                    "FAILS"
                } else {
                    "is reported but does not fail"
                },
                VAR_STRICT
            ));
            lines.push(format!(
                "  acknowledgement ({}): {}",
                VAR_ALLOW_MISSING_ORACLES,
                match (
                    self.config.missing_oracles_acknowledged(),
                    self.config.strict()
                ) {
                    (true, true) => String::from(
                        "set, and it does NOT apply: strict mode is dominant and escalates every \
                         unavailable arm above regardless"
                    ),
                    (true, false) => String::from(
                        "set: this reduced-oracle environment is acknowledged deliberately, and \
                         every gap above is still reported"
                    ),
                    (false, true) => String::from(
                        "not set, and it would make no difference if it were: strict mode is \
                         dominant"
                    ),
                    (false, false) => String::from(
                        "not set: the gaps above are reported and do not fail the run"
                    ),
                }
            ));
        }
        lines.push(String::new());

        lines.push(String::from("Verdict policy"));
        lines.push(format!(
            "  unexpected success (XPASS) fails the run: {} (set {} to downgrade it to a warning)",
            yes_or_no(self.config.xpass_fails_run()),
            VAR_ALLOW_XPASS
        ));
        lines.push(format!(
            "  an unavailable oracle fails the run: {} (set {} to escalate; {} cannot lower it, \
             because a job that demanded strictness while excusing the arms it could not attempt \
             would report a green run over a matrix it never executed)",
            yes_or_no(self.config.unavailable_fails_run()),
            VAR_STRICT,
            VAR_ALLOW_MISSING_ORACLES
        ));
        lines.push(format!(
            "  cell workspaces retained after success: {} (set {} to retain them)",
            yes_or_no(self.config.keep_work()),
            VAR_KEEP_WORK
        ));
        lines.push(String::from(
            "  a missing oracle is never a silent pass: it is recorded as UNAVAILABLE and listed \
             in the run summary",
        ));

        lines.join("\n")
    }

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
                // Asked through the availability predicate rather than re-derived from the
                // driver, so this line cannot claim an arm runs that the predicate excludes —
                // which is exactly what would happen if it looked only at the driver that
                // targets `target` and ignored the native compiler the audit gates need.
                Some(driver) if self.oracle_a_available(target) => {
                    format!("RUNS via {}", shown_path(driver))
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
                Some(runner) => format!("via {}", shown_path(runner)),
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
        lines.push(format!(
            "per-cell-timeout-secs: {}",
            self.config.timeout_secs()
        ));
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
        let (targets, opt_levels) = self.config.effective_matrix();
        lines.push(format!("matrix-targets: {}", join_targets(&targets)));
        lines.push(format!(
            "matrix-opt-levels: {}",
            join_opt_levels(&opt_levels)
        ));
        lines.push(format!(
            "matrix-coverage: {}",
            if self.config.is_reduced_run() {
                "reduced"
            } else {
                "full"
            }
        ));
        lines.join("\n")
    }
}

/// One `key: value` fingerprint line, stating the version and the path or a fixed absence marker.
fn fingerprint_line(key: &str, record: &ToolRecord) -> String {
    let Some(path) = record.path() else {
        // A refused tool and an absent one must not read alike. A divergence explained by a tool
        // the suite declined to trust is a different story from one explained by a package nobody
        // installed, and the fingerprint is often the only account of the run a reader has.
        return match &record.rejection {
            Some(reason) => format!("{key}: refused — {reason}"),
            None => format!("{key}: absent"),
        };
    };
    let mut line = format!(
        "{key}: {} ({})",
        record.version_or_unknown(),
        shown_path(path)
    );
    if let Some(identity) = &record.identity {
        // The file identity, not just the name. A package rebuilt at the same version reports the
        // same banner from a different file, and that is precisely the toolchain drift a
        // fingerprint exists to expose — a name alone would show two runs as identical.
        line.push_str(&format!(" [{}]", identity.describe()));
    }
    if let Some(provenance) = &record.provenance {
        line.push_str(&format!(" [{provenance}]"));
    }
    line
}

fn yes_or_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn join_targets(targets: &[Target]) -> String {
    targets
        .iter()
        .map(|target| String::from(target.triple()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn join_opt_levels(levels: &[OptLevel]) -> String {
    levels
        .iter()
        .map(|level| String::from(level.flag()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Discover the oracles available on this machine.
///
/// Cheap and idempotent. The first call performs the probing — a handful of `PATH` lookups and
/// short banner spawns — and memoizes the outcome, including a failure, in a
/// [`OnceLock`]. Every subsequent call in the same process returns a clone of that outcome.
///
/// Memoization matters here for two reasons beyond speed. The suite's tests run concurrently by
/// default and each one needs the record, so probing once removes a few hundred redundant process
/// spawns; and because the record is computed exactly once, every test in a run sees the same
/// inventory, which is what makes the reports of two runs comparable. [`OnceLock`] provides that
/// as synchronized one-time interior initialization: concurrent callers race to initialize, one
/// wins, the rest block until it finishes, and the value is then shared immutably. That is what
/// makes the cache safe to reach for a shared reference from many threads, where a mutable global
/// would not be.
///
/// # Errors
///
/// Returns an error in exactly four situations. Every one of them is the suite's own configuration
/// rather than a modest environment, which is the line that decides an error from a degradation:
///
/// - a catalogued variable holds bytes that are not valid UTF-8, so an override the maintainer set
///   could neither be used nor reported faithfully;
/// - a configuration defect reported by [`RunConfig::read`] — a malformed program filter, or a
///   per-cell budget that is not a positive whole number of seconds within the documented ceiling
///   — which is rejected here so that the pre-flight check reports it before any cell runs and
///   before any process is spawned;
/// - the compiler under test could not be located, which no amount of degradation can work around;
/// - two tools that must be independent implementations turn out to be the same file, which would
///   make a comparison meaningless rather than merely absent — the one failure mode that otherwise
///   produces a *pass*, and therefore the one that must never be degraded around.
///
/// Every other missing tool is recorded, reported and degraded around. A tool an environment can
/// legitimately lack is never an error, and neither is a tool that had to be terminated during its
/// banner probe: both are recorded and both appear in the report.
pub fn discover() -> HarnessResult<Capabilities> {
    static CACHE: OnceLock<HarnessResult<Capabilities>> = OnceLock::new();
    CACHE.get_or_init(discover_once).clone()
}

fn discover_once() -> HarnessResult<Capabilities> {
    // The whole catalogue is proved readable before anything else happens. Doing this first, rather
    // than letting each variable be checked at the moment it is consulted, is what makes an
    // undecodable override a reported configuration defect instead of a silent fall back to a
    // default.
    validate_environment()?;
    // The behavioural settings are validated next, and deliberately before any process is spawned.
    // A misconfigured variable is a defect in the suite's own configuration rather than a property
    // of the machine, so reporting it up front is far clearer than letting probing succeed and then
    // failing at the first cell — and it means the snapshot every later decision reads has already
    // been checked.
    let config = RunConfig::read()?;
    let strict = config.strict();
    let bcc = resolve_compiler_under_test(strict)?;

    let ref_cc_native = ToolRecord::discover(
        format!(
            "reference compiler, native arm ({}) and undefined-behaviour audit gates",
            env::consts::ARCH
        ),
        Some(VAR_REF_CC),
        DEFAULT_REF_CC,
        // Required to target the host when the host is one of the four supported targets, and
        // unconstrained otherwise: on an unsupported host there is no architecture to demand.
        native_target(),
        strict,
    );
    let ref_cc_i686 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::I686),
        Some(VAR_REF_CC_I686),
        DEFAULT_REF_CC_I686,
        Some(Target::I686),
        strict,
    );
    let ref_cc_aarch64 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::Aarch64),
        Some(VAR_REF_CC_AARCH64),
        DEFAULT_REF_CC_AARCH64,
        Some(Target::Aarch64),
        strict,
    );
    let ref_cc_riscv64 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::Riscv64),
        Some(VAR_REF_CC_RISCV64),
        DEFAULT_REF_CC_RISCV64,
        Some(Target::Riscv64),
        strict,
    );

    // The runner names are the one place where the emulator spelling and the target slug diverge:
    // the i686 target is executed by qemu-i386. Both the plain and the statically linked spellings
    // are probed for each architecture, in that order.
    let runner_i686 = ToolRecord::discover(
        format!("execution runner for {}", Target::I686),
        Some(VAR_QEMU_I386),
        DEFAULT_QEMU_I386,
        // An emulator is not a compiler driver: it emits nothing and states no target, so there is
        // no provenance to verify. Its architecture is instead proved by the arm's cells running.
        None,
        strict,
    );
    let runner_aarch64 = ToolRecord::discover(
        format!("execution runner for {}", Target::Aarch64),
        Some(VAR_QEMU_AARCH64),
        DEFAULT_QEMU_AARCH64,
        None,
        strict,
    );
    let runner_riscv64 = ToolRecord::discover(
        format!("execution runner for {}", Target::Riscv64),
        Some(VAR_QEMU_RISCV64),
        DEFAULT_QEMU_RISCV64,
        None,
        strict,
    );

    let timeout_tool = ToolRecord::discover(
        "per-cell timeout utility (optional)",
        None,
        DEFAULT_TIMEOUT_TOOL,
        None,
        strict,
    );
    let reducer = ToolRecord::discover(
        "test-case reducer for finding minimization (optional)",
        None,
        DEFAULT_REDUCER,
        None,
        strict,
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
        config,
        path_rejections: search_path().1.clone(),
    };
    // Checked once every record exists, because independence is a property of the set rather than
    // of any one tool: no individual resolution can tell that it collided with another.
    require_distinct_oracles(&capabilities)?;
    // Filled after construction because the probe asks each target's own driver, which the record
    // above is what resolves. Built in Target::ALL order so the report is deterministic.
    capabilities.c_runtimes = Target::ALL
        .iter()
        .copied()
        .map(|target| CRuntimeStatus::probe(target, capabilities.ref_cc_for(target)))
        .collect();
    Ok(capabilities)
}

/// Fail when two tools that must be independent turn out to be the same file.
///
/// Differential testing rests on an assumption no individual resolution can check: that the things
/// being compared are genuinely different implementations. Names conceal this completely — an
/// override, a symbolic link, a wrapper script, or a package manager's alternatives mechanism can
/// all make two different names denote one binary, and the run would then look entirely healthy
/// while proving nothing. Two collisions are refused:
///
/// * **The compiler under test against any reference driver.** Oracle (a) would compare a compiler
///   with itself, which agrees by construction. Every one of its comparisons would pass, and the
///   oracle would report perfect agreement precisely because it had stopped being an oracle.
/// * **Two reference drivers against each other.** Each cross arm asserts that bcc's output for an
///   architecture matches a driver that builds for that architecture. Two arms served by one driver
///   means at least one is comparing against a driver built for something else, so the arm is a
///   false authority rather than a missing one.
///
/// Both are hard failures rather than reported unavailability, and deliberately so: unavailability
/// says a comparison did not happen, while this says a comparison happened and was meaningless.
/// The second is far more dangerous, because it produces a pass.
///
/// Comparison is by file identity rather than by path, so a symbolic link, a hard link, and a
/// second spelling of one directory are all caught. A record with no identity — an unresolved or
/// refused tool — takes part in no comparison, because there is nothing to collide.
///
/// The records compared are the ones each target's arm will *actually use*, obtained the same way
/// the arm itself obtains them, rather than the four reference fields taken as a list. The
/// distinction is not cosmetic. On a host that is itself one of the non-baseline targets, that
/// target's arm is served by the native driver and its cross field goes unused; comparing the
/// unused field would reject a machine whose native driver and matching cross driver are
/// legitimately the same binary — a perfectly ordinary arrangement on a native build of that
/// architecture. Asking the same question the arm asks means this check can only ever refuse a
/// collision that would really have affected a comparison.
fn require_distinct_oracles(capabilities: &Capabilities) -> HarnessResult<()> {
    // One entry per target whose oracle (a) arm has a driver at all, paired with that driver.
    let arms: Vec<(Target, &ToolRecord)> = Target::ALL
        .iter()
        .copied()
        .filter_map(|target| {
            capabilities
                .ref_cc_record_for(target)
                .map(|record| (target, record))
        })
        .collect();
    let compiler = capabilities.bcc.identity.as_ref();
    for (target, record) in &arms {
        let Some((under_test, reference)) = compiler.zip(record.identity.as_ref()) else {
            continue;
        };
        if under_test.is_same_file(reference) {
            return Err(HarnessError::new(
                "verifying that the compiler under test and the reference compilers are \
                 independent implementations",
                format!(
                    "the compiler under test and the reference compiler for the {target} arm are \
                     the same file ({}). Oracle (a) would then compare the compiler under test \
                     with itself, which agrees by construction: every comparison on that arm would \
                     pass, and the arm would report perfect agreement precisely because it had \
                     stopped being an independent oracle. Point {} at the compiler under test and \
                     {} at a genuinely different reference compiler",
                    under_test.describe(),
                    VAR_BCC_BIN,
                    reference_variable_for(*target),
                ),
            ));
        }
    }
    for (index, (first_target, first_record)) in arms.iter().enumerate() {
        for (second_target, second_record) in arms.iter().skip(index + 1) {
            // Two targets legitimately share a record when one of them is the host, because the
            // native driver serves the host's arm. That is one driver serving one architecture
            // under two names for it, not one driver serving two architectures.
            if std::ptr::eq(*first_record, *second_record) {
                continue;
            }
            let Some((first, second)) = first_record
                .identity
                .as_ref()
                .zip(second_record.identity.as_ref())
            else {
                continue;
            };
            if first.is_same_file(second) {
                return Err(HarnessError::new(
                    "verifying that each arm of oracle (a) has its own reference driver",
                    format!(
                        "the reference compilers for the {first_target} and {second_target} arms \
                         of oracle (a) are the same file ({}). One driver cannot serve two \
                         architectures here — the reference compiler has no target-selection flag, \
                         so selecting a target means selecting a different driver binary — which \
                         means at least one of these two arms would compare against a program \
                         built for the wrong architecture. That arm would be a false authority \
                         rather than a missing one, so it is refused. Point {} and {} at the \
                         drivers for their own architectures",
                        first.describe(),
                        reference_variable_for(*first_target),
                        reference_variable_for(*second_target),
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The override variable that selects the reference driver for a target's arm.
///
/// Mirrors [`Capabilities::ref_cc_record_for`], so the variable named is always the one that
/// actually chose the driver in question: the host's arm is served by the native override however
/// the host is spelled, and every other arm by its own. Used only to make a collision diagnostic
/// actionable — naming the variable the maintainer must change is the difference between a report
/// they can act on and one they must first decode.
fn reference_variable_for(target: Target) -> &'static str {
    if target.is_native() {
        return VAR_REF_CC;
    }
    match target {
        Target::I686 => VAR_REF_CC_I686,
        Target::Aarch64 => VAR_REF_CC_AARCH64,
        Target::Riscv64 => VAR_REF_CC_RISCV64,
        Target::X86_64 => VAR_REF_CC,
    }
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
fn resolve_compiler_under_test(strict: bool) -> HarnessResult<ToolRecord> {
    let role = "compiler under test";
    if checked_var(VAR_BCC_BIN)?.is_some() {
        let record = ToolRecord::discover(role, Some(VAR_BCC_BIN), &[], None, strict);
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
            shown_path(&candidate)
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
/// Never fatal, and bounded like every other pre-flight process: the utility is spawned through
/// [`run_bounded_probe`], so a host whose identification utility hangs degrades to the fallback
/// line below instead of stalling discovery, and its output is escaped for reporting at capture
/// time, so a kernel line cannot carry a control character into the fingerprint it identifies.
/// When the utility is absent, declines to answer or had to be terminated, the standard library's
/// own view of the host is recorded instead, so a fingerprint always carries a host identification
/// of some kind.
fn probe_kernel() -> String {
    let resolved = DEFAULT_UNAME
        .iter()
        .find_map(|candidate| resolve_tool(candidate));
    if let Some(uname) = resolved {
        if let Some(capture) = run_bounded_probe(Command::new(&uname).arg("-a")) {
            if !capture.timed_out {
                if let Some(line) = first_non_empty_line(&capture.stdout) {
                    return line;
                }
            }
        }
    }
    format!(
        "kernel identification unavailable; the standard library reports this host as {}/{}",
        env::consts::OS,
        env::consts::ARCH
    )
}
