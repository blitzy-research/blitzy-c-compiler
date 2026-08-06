//! Oracle discovery and the capability record.
//!
//! This module is the differential conformance suite's single point of contact with the
//! machine it is running on. It locates the compiler under test, the native reference
//! compiler, the three cross reference drivers and the three emulated-execution runners;
//! it reads the entire environment-variable catalogue; and it emits a capability record
//! that states exactly which arms of which oracles will run.
//!
//! # Availability is answered per arm, but not every cause is per-arm
//!
//! There is deliberately no "skip because unsupported" verdict anywhere in the suite. An arm that
//! cannot be attempted is recorded as [`Verdict::Unavailable`](super::Verdict::Unavailable) and
//! listed individually by [`Capabilities::unavailable_oracle_arms`], so a modest environment can
//! never masquerade as a passing run.
//!
//! [`Capabilities::oracle_a_available`], [`Capabilities::oracle_b_available`] and
//! [`Capabilities::oracle_c_available`] each take a target and answer for that target alone, which
//! is the finest granularity the report can carry: a single missing emulator costs one target's
//! arms and nothing more.
//!
//! Answering per arm is not the same as every *cause* being per-arm, and the distinction matters
//! because one cause genuinely is not. Some inputs an arm depends on are shared across all four
//! targets, and when one of those is missing, every arm that depends on it is unavailable — which
//! is a whole-oracle outcome reached one arm at a time rather than an exception to the per-arm
//! rule. The native reference compiler is exactly such an input: it is the driver both
//! undefined-behaviour audit gates run, so without it oracle (a) is unavailable on every target,
//! including targets whose own cross driver resolved perfectly well. The degradation table below
//! states each cause and its reach, and it is applied exactly as written.
//!
//! Reporting the surviving arms individually therefore loses no detail in either case, and it
//! cannot mask a gap, because every unattemptable arm is enumerated by name whether it fell alone
//! or alongside the rest of its oracle.
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
//! # What this module writes, and what it cannot confine
//!
//! This module writes nothing itself: it reads environment variables, walks `PATH`, calls
//! [`std::fs::metadata`], and spawns short, non-interactive, bounded probes — a tool's version
//! banner, a driver's answer to where it would find a static-link input, and the kernel
//! identification — none of which is asked to produce a file. It opens no socket. Everything the
//! wider harness writes lives beneath the Cargo build directory, under [`super::work_root`],
//! [`super::report_root`] and [`super::findings_root`].
//!
//! A probe target is an installed tool, executed from wherever it lives, and — like every other
//! child in this suite — under the fixed environment [`super::isolate_child_environment`] installs:
//! the environment is cleared, the search path is [`sanitized_search_path`] rather than the inherited
//! one, and `TMPDIR`, `TMP`, `TEMP` and `HOME` point beneath the build directory. Nothing here
//! isolates a syscall or the network or enters a namespace, so a probed tool that reads a
//! configuration file by absolute path or writes to a hard-coded directory goes on doing so — a
//! variable cannot bind a program that never reads it. The guarantee is about the paths *this* module
//! constructs, which is none, plus what every child is told; the bound below is what keeps a badly
//! behaved tool from stalling the run.
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
use std::iter::Peekable;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use super::execute::TIMEOUT_UTILITY_OUTER_MARGIN;
use super::manifest::Execution;
use super::{
    build_root, digest_hex, isolate_child_environment, own_process_group, reap_bounded,
    record_infrastructure_breach, redact_secrets, register_process_group, run_id,
    sanitize_text_for_report, shown_path, stable_digest, terminate_process_group, AreaSpec,
    HarnessError, HarnessResult, OptLevel, Oracle, ReapOutcome, RunGeneration, Target,
    BCC_TARGET_FLAG,
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
/// `gcc` is the reference compiler of record, and the suite passes **no** standard-selection flag to
/// either compiler, because bcc has none to match — so the reference driver's *default* language
/// mode is the one that has to be right. The mode the corpus is written against is gnu17, and that
/// is the default of the **GCC 13 series**, which is therefore what an environment has to provide
/// under one of these names. It is not a property of the name: a newer driver installed as bare
/// `gcc` defaults to a later mode — GCC 15 defaults to gnu23 — and the two differ in ways a corpus
/// program can observe. An environment pins the 13 series either by installing it under one of the
/// names probed here or by naming it explicitly through [`VAR_REF_CC`], and the version each
/// candidate reports is captured into the pre-flight report and every finding's fingerprint so the
/// mode actually used is auditable rather than assumed.
///
/// `clang` is the last name in the order so that an environment which installs it *instead of* a
/// GCC driver still has a native reference compiler, and so that naming it through [`VAR_REF_CC`]
/// — a second, genuinely independent oracle — takes no code change.
///
/// # What "probed in order" does and does not mean
///
/// The order decides which name is *looked for* first. The first candidate that **exists** becomes
/// the candidate, and that one candidate is then vetted; a candidate that vetting **refuses** is
/// reported as a refusal and the order is **not** continued past it. So on a host where `gcc` is
/// present but its default language mode is wrong, this arm is unavailable — loudly, in the
/// pre-flight report and in the run summary — even though a later name in the list might have been
/// acceptable. [`VAR_REF_CC`] is the remedy, and it is the *only* remedy by design.
///
/// That is deliberate rather than an oversight, because the two candidates in a probe list are not
/// interchangeable spares. A refused driver and the next name after it are typically different
/// compilers, and substituting one for the other silently would change three things at once that
/// every verdict in the run depends on: the language dialect the corpus is judged against, the
/// diagnostics the undefined-behaviour warning gate is calibrated on, and the sanitizer runtime that
/// gate's second half needs. Measured on the reference host, that last one is not hypothetical —
/// `clang` is installed and would satisfy the language-mode check, but its sanitizer runtime
/// archives are not, so `-fsanitize=undefined,address` cannot link. Falling through to it would turn
/// a correctly reported *missing oracle* into 108 false reports that the **corpus** is defective,
/// because `ubaudit.rs` attributes a sanitizer-gate failure to the test program. A missing arm that
/// says so is strictly better than an arm quietly backed by something else, which is the same
/// principle that keeps an explicit override single-candidate.
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

/// Reference-compiler arguments that make a driver state the language mode it compiles in by default.
///
/// `-dM -E` prints every macro the driver predefines and compiles nothing; `-x c -` gives it an empty
/// C translation unit on standard input, so the answer describes C rather than whatever language a
/// file name would have implied.
///
/// Standard input rather than a path, deliberately. [`run_bounded_probe`] already gives every probe an
/// empty standard input, so the empty translation unit costs nothing and the argument list names no
/// file at all — which keeps the probe independent of any path existing, and keeps its rendered command
/// line free of a path that the report's credential redaction could legitimately rewrite. (Measured:
/// on a host where an environment variable whose name marks it as credential-bearing happens to hold
/// the value `/dev/null`, that spelling is redacted out of every diagnostic, and a refusal quoting a
/// partly redacted command line is a refusal nobody can act on.)
///
/// Like [`DUMP_MACHINE_FLAG`], these are probe arguments against the reference compiler and appear in
/// no differential invocation, so the rule that only flags both compilers honour with the same meaning
/// may be passed to both compilers is not engaged by them — which matters here specifically, because
/// `-E` is one of the flags the shared set is asserted *not* to contain.
const LANGUAGE_MODE_ARGUMENTS: &[&str] = &["-dM", "-E", "-x", "c", "-"];

/// The macro whose value states which revision of C a driver compiles by default.
const STDC_VERSION_MACRO: &str = "__STDC_VERSION__";

/// The macro a driver predefines when its default mode rejects the GNU extensions.
const STRICT_ANSI_MACRO: &str = "__STRICT_ANSI__";

/// Oldest revision of C a reference driver may compile by default: C11.
///
/// The corpus is C11 material by construction — `_Static_assert`, `_Generic`, `_Alignof`, `_Alignas`,
/// `_Noreturn`, anonymous aggregates and the UTF-8 and 16- and 32-bit string literals all arrive with
/// that revision — and the compiler under test is a C11 compiler. A driver defaulting to an earlier
/// revision would reject much of the corpus, and every such rejection would be recorded as a
/// divergence attributable to the oracle rather than to the compiler.
const MIN_STDC_VERSION: u64 = 201_112;

/// Newest revision of C a reference driver may compile by default: C17.
///
/// C17 is a defect-fix revision of C11 with no new features, so C11 and C17 are the same language for
/// every construct this corpus contains — which is why the range spans both rather than pinning one.
///
/// C23 is not, and the difference is measurable **on this corpus**. Measured with the same driver,
/// changing only the mode: a `_Generic` selection over `u8"A"` chooses `char *` under gnu17 and
/// `unsigned char *` under gnu23, because C23 gives a UTF-8 literal the type `unsigned char[]`. The
/// corpus contains both constructs. C23 also makes `bool`, `true` and `false` keywords and redefines
/// an empty parameter list, each of which the corpus relies on the older meaning of. Since **no
/// standard-selection flag is ever passed** — the compiler under test has none to match, so passing
/// one to the reference compiler alone would break the shared-flag discipline the comparison rests
/// on — the mode is decided entirely by *which driver was selected*, and there is no way to correct a
/// wrong one after the fact. An oracle speaking a different language than the corpus was written in
/// is a false authority rather than a weaker one.
const MAX_STDC_VERSION: u64 = 201_710;

/// Architecture spellings that all denote the suite's 32-bit x86 target.
///
/// Distributions do not agree here: the same instruction set is reported as `i686`, `i586`, `i486`,
/// `i386`, or `x86` depending on how the driver was configured. Accepting the family keeps a
/// correctly configured machine from being rejected over a naming convention, while still refusing
/// a driver that reports a genuinely different architecture.
const I686_ARCHITECTURE_ALIASES: &[&str] = &["i686", "i586", "i486", "i386", "x86"];

/// The only operating-system component a reference driver for this suite may report.
///
/// Every one of the four supported targets is a Linux target, every test binary is a statically
/// linked Linux executable, and the three non-native arms execute under a Linux user-mode emulator.
/// A driver reporting anything else — `mingw32`, `elf`, `darwin`, `freebsd` — cannot build a program
/// this suite is able to run, let alone one whose output may be compared.
const LINUX_OPERATING_SYSTEM: &str = "linux";

/// The only environment component a reference driver for this suite may report when it states one.
///
/// The comparison is byte-exact against stdout, so the reference arm and the compiler under test
/// must agree on type widths, calling convention and the formatting of the C library they link. A
/// driver reporting `musl`, `android`, `uclibc` or `gnux32` satisfies none of that, and every one of
/// them says so in this component.
const GNU_ENVIRONMENT: &str = "gnu";

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

/// Most an engaged outer net may add to one invocation, in milliseconds.
///
/// The external `timeout` utility is an **outer net** and nothing more: `execute.rs`'s own watchdog
/// is the authoritative bound on every path, so in a healthy run the utility never fires and its
/// entire contribution is the cost of supervising a child that finished on time. That cost is
/// therefore the whole of what qualification measures, and this is the ceiling.
///
/// The number is set from measurement rather than taste, because the two implementation strategies
/// in circulation differ by more than an order of magnitude. One supervises with a tight loop or a
/// signal and costs single-digit milliseconds; the other polls its child on a 100 millisecond
/// granularity and charges roughly that *per invocation whatever the budget*. Twenty-five
/// milliseconds sits an order of magnitude above the former and a quarter of the way to the latter,
/// so the two are separated with margin on both sides rather than by a hair.
///
/// The scale is what makes this worth measuring at all. A full matrix spawns at least 5,508 bounded
/// invocations, so 103 milliseconds each is roughly 569 seconds of wall time added to a suite whose
/// own measured run is a 345–476 second band on the four-core machine recorded in
/// `tests/conformance/README.md` — an outer net that never fires costing more wall time than the
/// work it supervises.
const OUTER_NET_OVERHEAD_MAX: Duration = Duration::from_millis(25);

/// How many times the qualification probe is run before its cost is judged.
///
/// The estimator is the **minimum** of these samples rather than the mean, because the quantity
/// being estimated is the implementation's own floor and every source of noise on a shared machine —
/// scheduling, page cache, another test's compiler — adds to a sample and never subtracts from it.
/// Three samples cost a few milliseconds against a qualifying implementation and about a third of a
/// second against one that will be declined, which is paid once per run rather than once per cell.
const OUTER_NET_PROBE_SAMPLES: usize = 3;

/// The budget handed to the qualification probe, in seconds.
///
/// A whole second, because the utility's budget argument is a whole number of seconds and the probe
/// must not be measuring an expiry: the child it supervises exits immediately, so a one-second budget
/// is never reached and what is measured is purely the supervision overhead.
const OUTER_NET_PROBE_BUDGET_SECS: u64 = 1;

/// Static archives and start files whose presence indicates a usable C runtime for a
/// target.
///
/// These four are the inputs a **target's C library development package** supplies, and they are
/// what this probe exists to find, because they are the ones an unprovisioned target is missing. The
/// archive supplies the library, and the three start files are the C runtime's initialization
/// sequence: `crt1.o` carries the entry stub that calls `main`, while `crti.o` and `crtn.o` are the
/// prologue and epilogue halves that bracket the initialization and finalization sections. All four
/// are required, which is the linkage contract the project's own technical specification states for
/// every one of the four architectures.
///
/// They are **not** the whole of what a static link consumes. Measured with `gcc -static -v`, the
/// link line also carries `crtbeginT.o` and `crtend.o` and links `-lgcc -lgcc_eh` alongside `-lc`.
/// Those four ship with the **compiler driver** rather than with the target's C library, so they are
/// present whenever the driver that would use them is present, and looking for them would test the
/// wrong thing: the question this probe answers is whether the target's runtime package was
/// installed, not whether the driver is internally complete. A driver missing its own support files
/// fails with a stage-execution or missing-file diagnostic that `compile.rs` attributes separately.
///
/// Checking a subset of the four would be worse than checking nothing, because it would report a
/// runtime as usable while a link against it fails: a missing `crti.o` or `crtn.o` produces an
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

/// What a catalogued override variable's raw state is, for the provenance report.
///
/// Three states rather than two, because two of them behave identically and mean different things.
/// `Unset` and `Blank` both resolve to the catalogued default — [`present_var_os`] treats an empty
/// value as absent, and [`checked_var`] treats a whitespace-only one the same way — and that is the
/// documented behaviour rather than an accident: a variable an environment cleared with `VAR=` names
/// no tool, and probing the default is the only remaining answer.
///
/// What the two states must not share is the sentence that describes them. Reporting a value the
/// operator explicitly exported as "unset" contradicts what they did, and leaves them unable to
/// predict the outcome from the report they are reading. Distinguishing the two costs one enum and
/// changes no behaviour anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverrideState {
    /// The variable is not in the environment at all.
    Unset,
    /// The variable is in the environment but holds nothing usable — empty, or only whitespace.
    Blank,
    /// The variable names something, and that something is the only candidate.
    Set,
}

/// Read a catalogued override variable's raw state.
///
/// Reads the same two predicates the resolution path reads, in the same order, so the state reported
/// can never disagree with the state acted on. A value that is not valid text is reported as `Set`,
/// which is correct in the only sense that matters here — it is present and it names something —
/// and is unreachable in a real run regardless, because [`validate_environment`] refuses such a
/// value with a precise diagnostic before any tool is resolved.
fn override_state(name: &str) -> OverrideState {
    match env::var_os(name) {
        None => OverrideState::Unset,
        Some(raw) => match raw.to_str() {
            Some(text) if text.trim().is_empty() => OverrideState::Blank,
            None if raw.is_empty() => OverrideState::Blank,
            _ => OverrideState::Set,
        },
    }
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
///
/// # Return value
///
/// The validated budget, paired with whether the variable supplied it. The second half exists so
/// that [`Capabilities::render_report`] can state the budget without also advising a reader to set
/// the very variable that produced it: the number alone cannot distinguish a default of 30 from an
/// override that happens to say 30, and a report that offered the override as an available action
/// while already obeying it would be describing a decision nobody made.
fn parse_timeout_secs(strict: bool) -> HarnessResult<(u64, bool)> {
    let Some(raw) = checked_var(VAR_TIMEOUT_SECS)? else {
        return Ok((DEFAULT_TIMEOUT_SECS, false));
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
    Ok((parsed, true))
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
        .map(|area| area.directory())
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
/// # Existence is checked, but not here
///
/// Whether a program of that name exists is deliberately *not* answered by this constructor.
/// Answering it means reading the corpus, which is `manifest.rs`'s responsibility, and reaching
/// into the corpus from the environment module would invert the layering for no gain.
///
/// It is answered nonetheless, and as a hard failure, by [`ProgramFilter::require_match`]: the
/// consumer that has just performed discovery reports how many programs this filter selected, and
/// a count of zero fails the run. Splitting the question that way keeps each half where it belongs
/// — shape is validated where the environment is read, existence where the corpus is read — while
/// leaving no gap between them.
///
/// Being merely "loud" is not sufficient here, and it is worth saying why, because reporting
/// without failing is the tempting alternative. A mistyped filter selects nothing; every selected
/// program then passes, because there are none to fail; and the run exits successfully. The report
/// is stamped partial and states a covered-program count of zero, both of which are true and
/// neither of which changes the exit status — so in continuous integration, where nobody reads a
/// passing job's report, a single typo turns the entire suite into a no-op that reports success. A
/// filter is an instruction to test one program, so selecting no program is a failure to carry out
/// that instruction, not a smaller run.
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
            area: spec.directory(),
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

    /// Confirm this filter actually selected a program, given how many it matched during discovery.
    ///
    /// The count is supplied by the caller rather than computed here, because computing it means
    /// reading the corpus and that belongs to `manifest.rs`. This function owns the *policy*: a
    /// filter that selected nothing is a hard failure, and a filter that somehow selected more than
    /// one program is too.
    ///
    /// # Errors
    ///
    /// Zero matches fails the run, naming the filter, the variable that set it and the two mistakes
    /// that actually cause it — a mistyped stem, and a stem that is correct but lives in a different
    /// area. Nothing about a zero-match run is worth continuing: every remaining check would pass
    /// for want of anything to check.
    ///
    /// More than one match also fails. It cannot arise from a well-formed corpus, because
    /// [`ProgramFilter::matches`] compares both halves exactly and a feature area cannot hold two
    /// programs of one stem — so if it ever happens, the corpus has acquired a duplicate and the
    /// cells of two different programs are about to be reported under one identity.
    pub fn require_match(&self, matched: usize) -> HarnessResult<()> {
        let context = format!("applying the program filter {self} from {VAR_ONLY}");
        if matched == 1 {
            return Ok(());
        }
        if matched == 0 {
            return Err(HarnessError::new(
                context,
                format!(
                    "the filter selected no program: no `{}.c` exists in the feature area {:?}. \
                     This fails the run rather than shrinking it, because a filter that matches \
                     nothing produces a run in which every selected program passes for want of any \
                     program to fail — a successful exit over a matrix of zero cells, which in \
                     continuous integration is indistinguishable from a suite that works. The two \
                     causes are a mistyped program stem and a correct stem named under the wrong \
                     area; unset {} to run the whole matrix, or correct the half that is wrong",
                    self.program, self.area, VAR_ONLY
                ),
            ));
        }
        Err(HarnessError::new(
            context,
            format!(
                "the filter selected {matched} programs, and it can only ever name one: both halves \
                 are compared exactly, so this means the feature area {:?} contains more than one \
                 program with the stem {:?}. That is a corpus defect rather than a filter defect — \
                 two programs sharing one identity would have their cells reported under the same \
                 name, making every verdict about either of them ambiguous",
                self.area, self.program
            ),
        ))
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
    /// True when [`VAR_TIMEOUT_SECS`] supplied the budget rather than the catalogued default.
    ///
    /// Kept because the number cannot answer the question on its own: an override of exactly
    /// [`DEFAULT_TIMEOUT_SECS`] is indistinguishable from no override at all, and the pre-flight
    /// report has to tell a reader which of the two it is looking at before it offers the override
    /// as something still to do.
    timeout_from_variable: bool,
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
        let (timeout_secs, timeout_from_variable) = parse_timeout_secs(strict)?;
        let keep_work = checked_flag(VAR_KEEP_WORK)?;
        Ok(RunConfig {
            quick,
            only,
            strict,
            allow_xpass,
            missing_oracles_acknowledged,
            keep_work,
            timeout_secs,
            timeout_from_variable,
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

    /// True when [`VAR_TIMEOUT_SECS`] supplied the budget rather than the catalogued default.
    ///
    /// Exists for the report rather than for any decision: no behaviour depends on where a valid
    /// budget came from, but the sentence that states it does, because a reader who has already
    /// exported the variable must not be told to export it.
    pub fn timeout_secs_from_variable(&self) -> bool {
        self.timeout_from_variable
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
/// # A launcher is not an implementation either
///
/// The identity of the file a name resolves to is still not always the identity of the compiler
/// that runs. A compiler is very often shipped as a small shell script that `exec`s the real
/// driver, and this suite's own reference toolchain is installed exactly that way: several
/// distinct wrapper scripts, each its own file with its own inode, every one of them handing the
/// work to the same driver binary.
///
/// Comparing launchers alone would therefore answer the independence question wrongly in the one
/// direction that matters. Two different wrapper scripts are two different files, so a launcher
/// comparison calls them independent — while the compiler that actually runs is the same compiler
/// on both sides. Oracle (a) would then compare an implementation with itself and agree
/// everywhere, and the arm would report perfect agreement *precisely because* it had stopped being
/// an oracle. That is a silent false authority, which is worse than a missing arm.
///
/// So a resolved wrapper is attested: [`read_wrapper_target`] resolves the program its `exec` line
/// names, and that program's identity is recorded here as the *implementation*. Independence is
/// then decided on implementations by [`ToolIdentity::is_same_implementation`], while the launcher
/// spelling is kept intact for invocation — which is not a nicety, because a GCC driver derives its
/// own exec prefix from `argv[0]` and invoking the implementation directly can make it fail to find
/// its own components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    /// The fully resolved path, with every symbolic link followed.
    canonical: PathBuf,
    /// Device and inode numbers, where the platform exposes them.
    ///
    /// `None` on a platform with no such notion, in which case [`ToolIdentity::is_same_file`]
    /// falls back to comparing canonical paths. The distinction is kept explicit rather than
    /// defaulted to zero, because two absent identities must never be reported as equal.
    file_id: Option<(u64, u64)>,
    /// True when the name that resolved was a symbolic link.
    ///
    /// Recorded rather than resolved away, because it is a fact the pre-flight report needs: this
    /// environment supplies its emulators under two spellings, one of which is a link, and which
    /// one was selected is exactly what the report exists to state.
    via_symlink: bool,
    /// The identity of the program a wrapper script hands the work to, when this file is one.
    ///
    /// Boxed because the relation nests: a wrapper may `exec` another wrapper, and the chain is
    /// followed to its end within [`WRAPPER_ATTESTATION_DEPTH_MAX`].
    implementation: Option<Box<ToolIdentity>>,
    /// The words this file's own `exec` line passes to the program it runs, when it is a wrapper.
    ///
    /// Empty for a file that is not a wrapper, and for one whose `exec` line names a program and
    /// nothing else — which is a real shape and a consequential one, since such a launcher drops
    /// every argument the suite assembled.
    ///
    /// # Why they are recorded rather than only counted
    ///
    /// The attestation exists to answer which implementation a name denotes, and the program word
    /// answers that. It does not answer what the launcher *does with the command line*, and a
    /// wrapper that inserts a flag changes what a differential invocation actually compiled — the one
    /// property requirement 3's shared-flag discipline is about. The suite prescribes wrapper scripts
    /// as the way to pin a driver under a plain name, so this is a mechanism it recommends; leaving
    /// its effect out of the report would mean the report could not describe a configuration the
    /// documentation asks for.
    ///
    /// Recorded and rendered, never acted on. A wrapper is not refused for carrying arguments: a
    /// provisioned toolchain may legitimately need a sysroot or a machine flag to work at all, and
    /// refusing it would break the environments the wrapper guidance was written for. What changes is
    /// that the words appear in the capability report and in every finding's environment fingerprint,
    /// so a divergence can be read against the command line that was really run.
    implementation_arguments: Vec<String>,
    /// True when this file looks like a wrapper script but the program it runs could not be
    /// determined.
    ///
    /// Kept distinct from "not a wrapper", because the two call for opposite treatment. A compiled
    /// binary is its own implementation and needs no attestation. A script whose `exec` target
    /// could not be read might be a wrapper for the very compiler it is being compared against,
    /// and there is no way to tell — so under strict mode it is refused rather than trusted.
    unattested_wrapper: bool,
}

impl ToolIdentity {
    /// Capture the identity of the file `path` currently denotes.
    ///
    /// Returns `None` when the path does not denote a regular file at this instant — which includes
    /// the case where it did a moment ago and no longer does. That is not a failure to be papered
    /// over: a tool that vanished between being resolved and being identified cannot be used, and
    /// saying so is better than carrying a path that will fail at the first spawn.
    fn of(path: &Path) -> Option<ToolIdentity> {
        ToolIdentity::of_within(path, WRAPPER_ATTESTATION_DEPTH_MAX)
    }

    /// Capture an identity, following a wrapper chain no deeper than `depth`.
    ///
    /// The depth bound is what makes the recursion total. A wrapper that `exec`s a second wrapper is
    /// ordinary, but a pair that `exec` each other is possible to write and would otherwise recurse
    /// without end — turning a misconfiguration into a stack overflow during pre-flight, which is
    /// the least diagnosable failure this module could produce. On exhausting the bound the file is
    /// recorded as an unattested wrapper, which is the honest answer: it *is* a wrapper and its
    /// implementation was *not* established.
    fn of_within(path: &Path, depth: usize) -> Option<ToolIdentity> {
        let link = fs::symlink_metadata(path).ok()?;
        let via_symlink = link.file_type().is_symlink();
        // Followed deliberately. A link is a legitimate way to ship a tool — this environment
        // supplies its emulators that way — so the question is not whether a link was used but
        // whether what it points at is a regular executable file.
        let target = fs::metadata(path).ok()?;
        if !target.is_file() {
            return None;
        }
        let canonical = fs::canonicalize(path).ok()?;
        let (implementation, implementation_arguments, unattested_wrapper) =
            match read_wrapper_target(&canonical) {
                // Not a script at all: a compiled binary is its own implementation, so there is
                // nothing to attest and nothing missing.
                WrapperTarget::NotAWrapper => (None, Vec::new(), false),
                WrapperTarget::Unreadable => (None, Vec::new(), true),
                WrapperTarget::Execs { program, arguments } => {
                    if depth == 0 {
                        // The chain is longer than the bound allows, so the implementation was never
                        // established. The arguments are dropped with it: reporting what an
                        // unattested launcher passes would dress an unknown chain up as a known one.
                        (None, Vec::new(), true)
                    } else {
                        match ToolIdentity::of_within(&program, depth - 1) {
                            Some(inner) => (Some(Box::new(inner)), arguments, false),
                            // The `exec` line names a program that cannot be identified. The wrapper
                            // will fail at the first spawn, but more importantly its implementation is
                            // unknown, so it cannot be compared for independence.
                            None => (None, Vec::new(), true),
                        }
                    }
                }
            };
        Some(ToolIdentity {
            canonical,
            file_id: identity_numbers(&target),
            via_symlink,
            implementation,
            implementation_arguments,
            unattested_wrapper,
        })
    }

    /// True when this file is a wrapper script whose implementation could not be established.
    pub fn is_unattested_wrapper(&self) -> bool {
        self.unattested_wrapper
    }

    /// Confirm this identity still describes the file at `path`, or say how it has changed.
    ///
    /// Discovery and use are separated by the whole of a run: a tool is identified once during
    /// pre-flight and then invoked in up to 1,296 cells. Nothing in between guarantees the file has
    /// not been replaced, and a substitution after vetting would make every vetting decision — the
    /// location check, the wrapper attestation, the declared target — a statement about a file that
    /// no longer exists.
    ///
    /// # Where this is called from, so the claim can be checked
    ///
    /// Immediately before a tool is spawned, at every place in the suite that spawns one, and as the
    /// last statement before the launch in each:
    ///
    /// - [`confirm_vetted_tools_unchanged`], reached from `compile`'s own bounded spawn and from
    ///   `execute`'s single bounded launch. Those two cover every build, every cell execution, both
    ///   audit-gate invocations and every flag-probe invocation, because the audit and the probe
    ///   both launch through `execute`.
    /// - [`confirm_vetted_tool_unchanged`], reached from the process-group sweep, which spawns the
    ///   `kill` utility with a signal and a group identifier and therefore has the same substitution
    ///   concern as any other tool.
    /// - [`attest_runners`], which asks the question a second time for an emulator, because
    ///   attestation is the first thing that ever runs one and is therefore the earliest point at
    ///   which the answer is useful.
    ///
    /// Discovery's own subprocesses — the banner reads and the runner attestation's build-and-run —
    /// are the one place a spawn is *not* preceded by this check, and necessarily so: they run while
    /// the tools are being vetted, and a tool with no earlier vetting has no earlier identity to be
    /// compared against.
    ///
    /// # Two residuals, stated rather than implied
    ///
    /// **The interval to `execve` is narrowed, not closed.** Closing it needs an open handle spawned
    /// through, which the standard library offers no way to do. Narrowing it to microseconds and
    /// *reporting* a change that did happen is what is achievable here, and it converts a silent
    /// substitution into a named refusal — which is the difference that matters, because a
    /// substitution reported as a refusal cannot be mistaken for evidence about a compiler.
    ///
    /// **An inode number can be reused, so a same-path delete-and-recreate can compare equal.**
    /// Measured on the reference host rather than assumed: writing a file, removing it and writing a
    /// different one at the same name returned the identical device and inode pair, while renaming a
    /// second file over the first returned a different one. What this comparison therefore catches is
    /// a tool that was removed, replaced by something that is not a regular file, left dangling, or
    /// renamed over — and what it can miss is an in-place recreate that the allocator happened to
    /// give the same inode. Closing that too would mean stamping content, which at the size of a
    /// compiler driver would be paid on every one of the spawns this check exists to guard and would
    /// buy a narrower improvement than its cost. The limit is recorded here so a reader takes the
    /// guarantee for what it is rather than for what its name suggests.
    ///
    /// Device and inode numbers are compared where the platform exposes them, because they see
    /// through every spelling. When it does not, the canonical path is compared instead and the two
    /// answers are never mixed, so an absent identity cannot be mistaken for a match.
    ///
    /// Returns [`None`] when the file is unchanged.
    pub fn changed_since_vetting(&self, path: &Path) -> Option<String> {
        let Some(current) = ToolIdentity::of(path) else {
            return Some(format!(
                "{} could no longer be identified as a regular file, so it was deleted, replaced by \
                 something that is not a file, or is now a dangling symbolic link",
                shown_path(path)
            ));
        };
        match (self.file_id, current.file_id) {
            (Some(vetted), Some(now)) if vetted != now => Some(format!(
                "{} is now device {}, inode {}, where vetting recorded device {}, inode {}: the file \
                 was replaced after it was checked, so every judgement made about it — its location, \
                 its wrapper attestation and the target it declared — describes a file that is no \
                 longer there",
                shown_path(path),
                now.0,
                now.1,
                vetted.0,
                vetted.1
            )),
            (None, _) | (_, None) if self.canonical != current.canonical => Some(format!(
                "{} now resolves to {}, where vetting recorded {}",
                shown_path(path),
                shown_path(&current.canonical),
                shown_path(&self.canonical)
            )),
            _ => None,
        }
    }

    /// The identity that actually does the compiling: the end of the wrapper chain, or this file
    /// when it is not a wrapper.
    ///
    /// This is the identity every independence question must be asked about, because it is the one
    /// that describes the program that runs rather than the name it was reached by.
    pub fn implementation(&self) -> &ToolIdentity {
        match &self.implementation {
            Some(inner) => inner.implementation(),
            None => self,
        }
    }

    /// Canonical path of the implementation, for reporting and for trust judgements.
    ///
    /// # Never invoke this path
    ///
    /// It is deliberately *not* the path any tool is spawned with, and substituting it would break
    /// the very toolchain this attestation was written for. A GCC driver derives its own exec prefix
    /// from `argv[0]` and then looks for its compiler proper — `cc1` — relative to that prefix. The
    /// launcher spelling is what the installation was built to be invoked as; running the driver
    /// under a different name can make it search a prefix where `cc1` does not exist and fail with
    /// nothing but "cannot execute `cc1`". Measured on the reference host, which is exactly why its
    /// five compiler entries are `exec`-ing scripts rather than symbolic links.
    ///
    /// The two paths therefore have two distinct jobs, and both are kept: [`ToolRecord::path`] is
    /// the spelling every invocation and every recorded reproduction command uses, while this is the
    /// file whose identity independence is decided on and whose location strict mode vets.
    pub fn implementation_path(&self) -> &Path {
        &self.implementation().canonical
    }

    /// True when both names ultimately run the same program.
    ///
    /// This — not [`ToolIdentity::is_same_file`] — is the question oracle independence turns on.
    /// Two distinct wrapper scripts are two distinct files, so a launcher comparison would call
    /// them independent while the compiler doing the work is the same on both sides; this
    /// comparison sees through the wrappers to the program itself.
    pub fn is_same_implementation(&self, other: &ToolIdentity) -> bool {
        self.implementation().is_same_file(other.implementation())
    }

    /// True when both identities denote the same file on disk.
    ///
    /// Device and inode numbers are authoritative where the platform provides them, because they
    /// see through every spelling: a relative path, a symbolic link and a hard link all reduce to
    /// the same pair. The canonical-path comparison is the fallback for a platform that exposes no
    /// such numbers, and it is never mixed with the numeric answer, so an absent identity can never
    /// be mistaken for a match.
    pub fn is_same_file(&self, other: &ToolIdentity) -> bool {
        match (self.file_id(), other.file_id()) {
            (Some(mine), Some(theirs)) => mine == theirs,
            _ => self.canonical() == other.canonical(),
        }
    }

    /// A one-line description for the pre-flight report and the environment fingerprint.
    ///
    /// The implementation is named whenever it differs from the launcher, because "which compiler
    /// actually ran" is the question a fingerprint exists to answer, and behind a wrapper the
    /// launcher path does not answer it. An unattested wrapper says so, so a reader can see that
    /// the independence guarantee rests on a chain that could not be followed.
    pub fn describe(&self) -> String {
        // Read through this type's own accessors rather than its fields. The distinction is not
        // stylistic: every consumer outside this module can only use the accessors, so a renderer
        // that reads fields directly can silently disagree with what everyone else sees.
        let mut text = shown_path(self.canonical());
        if let Some((device, inode)) = self.file_id() {
            text.push_str(&format!(" (device {device}, inode {inode})"));
        }
        if self.via_symlink() {
            text.push_str(" via symbolic link");
        }
        let implementation = self.implementation();
        if !std::ptr::eq(implementation, self) {
            text.push_str(&format!(
                ", a wrapper for {}",
                shown_path(&implementation.canonical)
            ));
            if let Some((device, inode)) = implementation.file_id {
                text.push_str(&format!(" (device {device}, inode {inode})"));
            }
            // What the chain does to the command line, stated beside what it runs. Naming only the
            // target program would describe a launcher that hands this suite's arguments through
            // unchanged and one that rewrites them in exactly the same words.
            text.push_str(&describe_wrapper_arguments(&self.chain_arguments()));
        }
        if self.unattested_wrapper {
            text.push_str(", a script whose exec target could not be determined");
        }
        text
    }

    /// Every word each hop of this wrapper chain adds, outermost hop first.
    ///
    /// Flattened rather than reported per hop because the chain is an implementation detail of how a
    /// name was pinned, while what reaches the compiler is the accumulation of every hop's words. A
    /// chain whose injection sits two levels down is exactly the case a per-hop-first reading would
    /// let a reader skim past, so the words are gathered in the order they are applied and rendered
    /// as one list.
    fn chain_arguments(&self) -> Vec<String> {
        let mut collected = self.implementation_arguments.clone();
        let mut hop = self.implementation.as_deref();
        while let Some(inner) = hop {
            collected.extend(inner.implementation_arguments.iter().cloned());
            hop = inner.implementation.as_deref();
        }
        collected
    }
    /// Device and inode numbers of the launcher, where the platform exposes them.
    pub fn file_id(&self) -> Option<(u64, u64)> {
        self.file_id
    }

    /// The fully resolved path of the file this name denotes, with every symbolic link followed.
    ///
    /// This is the *launcher*: the file the suite will actually execute. See
    /// [`ToolIdentity::implementation_path`] for the compiler behind a wrapper.
    pub fn canonical(&self) -> &Path {
        &self.canonical
    }

    /// True when the name that resolved was a symbolic link.
    pub fn via_symlink(&self) -> bool {
        self.via_symlink
    }
}

/// The spellings of the shell's argument-forwarding placeholder.
///
/// A launcher whose `exec` line ends in exactly this and nothing else passes the suite's own command
/// line through untouched, which is the canonical shape and the one the wrapper guidance describes.
/// Both spellings are listed because the quotes are what make the expansion correct — an unquoted
/// `$@` re-splits an argument containing a space — and a report must recognise the shape whichever
/// way it was written rather than describing a correct launcher as an unusual one.
const EXEC_FORWARD_PLACEHOLDERS: &[&str] = &["\"$@\"", "$@"];

/// State what a wrapper chain's `exec` lines add to the command line, or say nothing when they add
/// nothing worth saying.
///
/// Three outcomes, because there are three materially different things a launcher can do:
///
/// * **Forwards the command line untouched.** The single placeholder and nothing else. This is the
///   documented shape and the overwhelmingly common one, so it earns no words: annotating every
///   correctly written launcher would bury the two cases that matter under noise.
/// * **Forwards nothing.** An `exec` line naming a program and no arguments at all. Easy to write by
///   accident, and it silently discards every flag, source and output path the cell assembled, so it
///   is stated plainly.
/// * **Adds, reorders or drops arguments.** Anything else. The words are quoted verbatim so a reader
///   can see precisely what was inserted and where it sits relative to the placeholder.
///
/// Every word is escaped for reporting before it is quoted. The words come from a file on disk that
/// an override pointed at, so they are as untrusted as any other value this report renders, and a
/// control character or a directional override in a wrapper script must not be able to repaint the
/// line that discloses it.
fn describe_wrapper_arguments(arguments: &[String]) -> String {
    if arguments.is_empty() {
        return String::from(
            ", whose exec line passes it no arguments at all, so nothing this suite puts on the \
             command line reaches it",
        );
    }
    if arguments.len() == 1 && EXEC_FORWARD_PLACEHOLDERS.contains(&arguments[0].as_str()) {
        return String::new();
    }
    let escaped: Vec<String> = arguments
        .iter()
        .map(|word| sanitize_text_for_report(word))
        .collect();
    format!(
        ", whose exec line also passes {} — these words reach the compiler on every invocation, so \
         the flags a comparison ran are these as well as the ones the suite assembled",
        join_quoted(&escaped)
    )
}

/// What inspecting a resolved tool for wrapper-hood established.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WrapperTarget {
    /// The file carries no shebang, so it is a compiled program and is its own implementation.
    NotAWrapper,
    /// The file begins with a shebang but no program could be extracted from it.
    Unreadable,
    /// The file is a script consisting of a shebang, comments and one unconditional `exec` of this
    /// absolute path — the only shape whose implementation is knowable without interpreting shell.
    Execs {
        /// The absolute path the `exec` line runs.
        program: PathBuf,
        /// Every word the `exec` line passes after the program, in order and verbatim.
        ///
        /// Kept because they are part of what the wrapper *does*, not decoration. The canonical
        /// launcher shape passes exactly `"$@"` and therefore hands the suite's own command line
        /// through untouched; a line that passes anything else is adding to, reordering or dropping
        /// the arguments every differential invocation was assembled from, which is a claim about the
        /// flags the comparison actually ran. These words are never interpreted or acted on — the
        /// grammar deliberately stops short of understanding shell — but discarding them would leave
        /// the report unable to say that they exist.
        arguments: Vec<String>,
    },
}

/// How many wrapper links are followed before the chain is declared unattested.
///
/// A wrapper that wraps a wrapper is ordinary; four levels is far beyond any real installation, and
/// the bound is what stops a mutually-`exec`ing pair from recursing forever.
const WRAPPER_ATTESTATION_DEPTH_MAX: usize = 4;

/// How much of a candidate is read while looking for a wrapper's `exec` line.
///
/// Generous for a shell script and negligible for a compiled binary, which is rejected by its
/// missing shebang after the first two bytes anyway. Bounded so that pointing an override at a
/// pipe, a device or a multi-gigabyte file cannot stall or exhaust pre-flight.
const WRAPPER_SCRIPT_BYTES_MAX: usize = 8 * 1024;

/// The two bytes that introduce an interpreted script.
const SHEBANG: &[u8] = b"#!";

/// Determine whether a resolved tool is a wrapper script and, if so, which program it runs.
///
/// # Why this is worth doing at all
///
/// Oracle (a) is only an oracle while the two compilers are different implementations, and a
/// wrapper hides which implementation a name denotes. A provisioned reference toolchain routinely
/// reaches this case, because pinning a particular driver series under a plain name — `gcc`, `cc` or
/// a cross-driver name — is most simply done with a one-line `exec` script, and two such names can
/// perfectly well `exec` the same driver. How many of the names on a given machine are scripts and
/// how many are real binaries is a property of that machine, not of this suite; both are accepted.
/// What matters is that a launcher-only comparison would declare a pair independent whenever their
/// launchers differ, even when one driver stands behind both. Reading the `exec` line is what turns
/// "different file" into "different compiler".
///
/// # What is accepted as an answer
///
/// Deliberately narrow, because a wrong answer here is worse than no answer. A candidate qualifies
/// only when it begins with a shebang; only lines whose first word is exactly `exec` are considered;
/// the builtin's own options are accounted for by [`exec_program_word`], including `-a NAME`, whose
/// argument would otherwise be mistaken for the program; and the program word must be an
/// **absolute** path. A relative program word is not resolved, because resolving it would mean
/// guessing the working directory the wrapper will eventually run in, and a guess is precisely what
/// must not enter an independence decision — it is reported as unreadable instead, which refuses the
/// tool in every mode.
///
/// # The grammar, and why it is exactly this narrow
///
/// A script is attested only when it matches **one unconditional `exec`** in full:
///
/// 1. the first line is a shebang;
/// 2. every other line is blank, or a comment, or *the* `exec` line — there is exactly one line
///    whose first word is `exec`, and no other executable statement of any kind;
/// 3. that `exec` line is the **last** non-blank, non-comment line in the file, so nothing can run
///    after it and nothing it does can be conditional on anything;
/// 4. its program word is an absolute path.
///
/// Requiring *the* `exec` line to be the only executable statement, rather than reading whichever
/// `exec` comes last, is what makes the answer trustworthy. A script can `exec` a decoy on its final
/// line while an earlier conditional — `if [ -n "$SOMETHING" ]; then exec /other/cc "$@"; fi` — is
/// the branch that actually runs. Reading the trailing line would then report a compiler that never
/// executed, and oracle independence is decided on that report: two names could hand their work to
/// one driver and be recorded as distinct, in which case a differential comparison compares a
/// compiler with itself and agrees by construction.
///
/// The grammar is *deliberately* unable to describe such a script. Comment and blank lines carry no
/// statement and are not counted — any number of them is accepted, up to
/// [`WRAPPER_SCRIPT_BYTES_MAX`] for the file as a whole — so an ordinary pinning wrapper, a shebang
/// plus however many lines explain which driver series it selects and why plus one trailing
/// unconditional `exec` of an absolute path, satisfies it as written. Anything more complicated yields [`WrapperTarget::Unreadable`]: not a
/// claim that the file runs itself, but a refusal to guess, and that refusal rejects the tool in
/// **every** mode rather than only under strict.
///
/// Following conditionals, expanding variables or honouring `$@` placement would mean interpreting
/// shell, and a partial shell interpreter reaching a confident wrong conclusion is precisely the
/// failure this function exists to avoid.
fn read_wrapper_target(path: &Path) -> WrapperTarget {
    let Ok(bytes) = read_file_prefix(path, WRAPPER_SCRIPT_BYTES_MAX) else {
        return WrapperTarget::Unreadable;
    };
    if !bytes.starts_with(SHEBANG) {
        return WrapperTarget::NotAWrapper;
    }
    // A script whose bytes are not text cannot be read for an `exec` line, and guessing is not an
    // option, so it is unattested rather than assumed to run itself.
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return WrapperTarget::Unreadable;
    };
    // A file longer than the read bound cannot be attested at all: the statements this function
    // never saw could include another `exec`, and "the part I read looked fine" is not attestation.
    if bytes.len() >= WRAPPER_SCRIPT_BYTES_MAX {
        return WrapperTarget::Unreadable;
    }

    let mut program: Option<PathBuf> = None;
    let mut arguments: Vec<String> = Vec::new();
    let mut exec_lines = 0usize;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        // The shebang itself, and any blank or comment line, carry no statement.
        if index == 0 || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut words = trimmed.split_whitespace();
        if words.next() != Some("exec") {
            // Any other statement — a conditional, an assignment, a call, a redirection — means the
            // file's behaviour is not determined by its `exec` line alone.
            return WrapperTarget::Unreadable;
        }
        if exec_lines > 0 {
            // A second `exec` means at most one of them runs, and which one is a shell decision.
            return WrapperTarget::Unreadable;
        }
        exec_lines += 1;
        // Split rather than consumed, so the words after the program word survive the scan. The
        // program word decides the independence question; the rest decide what the launcher does to
        // the command line, and the report states both.
        let mut remainder = words.peekable();
        let Some(word) = exec_program_word(&mut remainder) else {
            return WrapperTarget::Unreadable;
        };
        let candidate = Path::new(word);
        if !candidate.is_absolute() {
            // A relative program word would have to be resolved against a working directory this
            // function would be guessing at, and a guess must not enter an independence decision.
            return WrapperTarget::Unreadable;
        }
        program = Some(candidate.to_path_buf());
        arguments = remainder.map(String::from).collect();
    }
    match program {
        // Exactly one `exec`, nothing else executable, and it was necessarily the last statement —
        // the loop returns `Unreadable` for any statement it meets, whether before or after.
        Some(program) if exec_lines == 1 => WrapperTarget::Execs { program, arguments },
        _ => WrapperTarget::Unreadable,
    }
}

/// The option of the `exec` builtin that takes a separate argument.
///
/// `exec -a NAME PROGRAM` runs `PROGRAM` while presenting `NAME` as its `argv[0]`. It is the one
/// `exec` option whose argument is a separate word, and skipping the option without skipping its
/// argument would mistake `NAME` for the program — which matters here more than anywhere, because
/// controlling `argv[0]` is exactly what a GCC launcher is written to do.
const EXEC_ARGV0_OPTION: &str = "-a";

/// Marks the end of the `exec` builtin's options.
const EXEC_END_OF_OPTIONS: &str = "--";

/// Pick the program word out of the remainder of an `exec` line.
///
/// The words supplied are everything after `exec` itself. The builtin's option syntax is small and
/// documented — `-c` and `-l` take nothing, `-a NAME` takes one word, `--` ends the options — so this
/// is parsing one builtin's grammar rather than interpreting shell, and it stops at the first word
/// that is not an option or an option's argument.
///
/// Anything it cannot account for yields the word it found anyway, which the caller then requires to
/// be an absolute path. That is the safe direction: an unrecognised spelling produces a relative or
/// nonsensical word, the caller declines it, and the tool is reported unattested — refused under
/// strict mode rather than guessed at.
///
/// The iterator is borrowed rather than consumed so the caller keeps everything after the program
/// word. Those words are the wrapper's effect on the command line, and the caller records them for
/// the report; taking the iterator by value would have discarded them at exactly the point they
/// became identifiable.
fn exec_program_word<'a, I>(words: &mut Peekable<I>) -> Option<&'a str>
where
    I: Iterator<Item = &'a str>,
{
    while let Some(word) = words.peek().copied() {
        if word == EXEC_END_OF_OPTIONS {
            words.next();
            break;
        }
        if !word.starts_with('-') {
            break;
        }
        words.next();
        if word == EXEC_ARGV0_OPTION {
            // Consume the replacement `argv[0]`, which is a word of its own and is not the program.
            words.next();
        }
    }
    words.next()
}

/// Read at most `limit` bytes from the beginning of `path`.
///
/// Bounded rather than whole-file, so a candidate that is a pipe, a device or an enormous file
/// cannot stall pre-flight or exhaust memory. A short read is not an error: a wrapper script is a
/// few hundred bytes and the prefix is all that is wanted.
fn read_file_prefix(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
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

/// The effective user identifier of this process, read without a foreign-function call.
///
/// `/proc/self/status` carries a `Uid:` line whose four fields are the real, effective, saved-set
/// and filesystem identifiers in that order; the effective one is what decides whether a file this
/// process owns could be rewritten by this process, and therefore which owners count as trusted.
///
/// [`None`] when the file cannot be read or parsed, which is the honest answer on a system with no
/// `/proc`. Callers treat that as "only the superuser is a trusted owner", which is the
/// conservative reading: it can refuse a tool, never accept one it should not.
#[cfg(unix)]
fn effective_uid() -> Option<u32> {
    static EFFECTIVE_UID: OnceLock<Option<u32>> = OnceLock::new();
    *EFFECTIVE_UID.get_or_init(|| {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        let line = status
            .lines()
            .find(|line| line.starts_with("Uid:"))?
            .trim_start_matches("Uid:");
        line.split_whitespace().nth(1)?.parse::<u32>().ok()
    })
}

/// Fallback for a platform that exposes no such identifier here.
#[cfg(not(unix))]
fn effective_uid() -> Option<u32> {
    None
}

/// Owners a tool or one of its ancestor directories may have and still be trusted.
///
/// The superuser is always trusted, because a machine whose superuser is hostile has no security
/// boundary left for this suite to enforce. This process's own effective identifier is trusted
/// because a file it could rewrite itself grants an attacker nothing it does not already have.
/// Every other owner is refused: a tool owned by an unrelated account can be rewritten by that
/// account between this check and the moment the suite runs it, which is precisely the substitution
/// an oracle must not be subject to.
#[cfg(unix)]
fn owner_is_trusted(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    let owner = metadata.uid();
    owner == 0 || effective_uid().is_some_and(|effective| owner == effective)
}

/// Fallback for a platform with no ownership notion, where the question cannot be asked.
#[cfg(not(unix))]
fn owner_is_trusted(_metadata: &fs::Metadata) -> bool {
    true
}

/// Why this path is writable by an account the suite does not trust, or [`None`] when it is not.
///
/// Three conditions are refused, and the sticky bit exempts **none** of them:
///
/// - **other-writable**, which lets any account on the machine write here;
/// - **group-writable**, which lets every member of the owning group write here. A group is not a
///   single trusted principal — on a developer machine or a shared build host it routinely contains
///   accounts that have nothing to do with this suite — and this condition was entirely unhandled
///   before;
/// - **an owner that is neither the superuser nor this process**, which lets that account write
///   here whatever the mode bits say, because an owner may always change them.
///
/// The sticky bit is deliberately not an exemption, and this is a change from a narrower earlier
/// rule that exempted it. The bit restrains *deletion and renaming* of an entry by an account that
/// does not own it; it does not restrain **creating a new entry**, which is the whole of the attack
/// on a search path — dropping a file named `gcc` into a world-writable sticky directory that
/// appears in `PATH` is enough to be selected as the reference compiler. It also does not restrain
/// the directory's own owner. Exempting it therefore left the case that matters open in exchange
/// for accepting shared temporary directories, and no legitimate tool in this environment lives in
/// one: every reference driver, every emulator and every ancestor of each was measured to be owned
/// by the superuser with mode 755, so the strict rule refuses nothing the suite needs.
#[cfg(unix)]
fn untrusted_writer_reason(metadata: &fs::Metadata) -> Option<&'static str> {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode();
    if mode & 0o002 != 0 {
        return Some(
            "it is writable by any account on this machine (the sticky bit is not treated as an \
             exemption, because it restrains deleting an entry, not creating one)",
        );
    }
    if mode & 0o020 != 0 {
        return Some(
            "it is writable by every member of its owning group, which is not a single trusted \
             principal",
        );
    }
    if !owner_is_trusted(metadata) {
        return Some(
            "it is owned by an account that is neither the superuser nor this process, and an \
             owner may change its own permissions at any time",
        );
    }
    let _ = metadata.uid();
    None
}

/// Fallback for a platform with no POSIX permission model, where no such judgement can be made.
#[cfg(not(unix))]
fn untrusted_writer_reason(_metadata: &fs::Metadata) -> Option<&'static str> {
    None
}

/// Why a resolved tool's location cannot be trusted, or `None` when it can.
///
/// The file and **every directory from it up to the root** are examined, because any one of them is
/// enough to substitute the tool: a world-writable **file** can have its contents replaced, the
/// **directory holding it** can have the file replaced beneath an unchanged name, and any
/// **directory further up** can be renamed so that a different subtree answers to the same path.
/// The second and third are the substitutions a name-based check cannot see, and the third is the
/// one a parent-only check misses: a mode-755 directory sitting beneath a world-writable one leaves
/// the entire subtree replaceable while both the file and its own directory look correct, which is
/// why the walk does not stop at the immediate parent.
///
/// Metadata is taken with the link-following call on purpose. On Linux a symbolic link's own
/// permission bits are always `rwxrwxrwx` and carry no meaning, so inspecting the link rather than
/// its target would report every link as world-writable and reject legitimate tools. Search-path
/// directories are routinely links, and so is one of the two spellings under which the emulators are
/// commonly installed, so a link-rejecting check would fail on an ordinary installation.
fn untrusted_reason(path: &Path) -> Option<String> {
    if let Ok(metadata) = fs::metadata(path) {
        if let Some(reason) = untrusted_writer_reason(&metadata) {
            return Some(format!(
                "the file itself can be written by an account this suite does not trust: {reason}, \
                 so its contents can be replaced between this check and the moment it runs"
            ));
        }
    }
    // `ancestors` yields the path itself first, which the file check above has already covered, so
    // the walk starts at the parent and continues to the root.
    for ancestor in path.ancestors().skip(1) {
        let Ok(metadata) = fs::metadata(ancestor) else {
            continue;
        };
        if let Some(reason) = untrusted_writer_reason(&metadata) {
            let shown = shown_path(ancestor);
            return Some(if Some(ancestor) == path.parent() {
                format!(
                    "its directory {shown} can be written by an account this suite does not trust: \
                     {reason}, so the file can be replaced beneath an unchanged name"
                )
            } else {
                format!(
                    "the directory {shown} on its path can be written by an account this suite does \
                     not trust: {reason}, so any directory beneath it can be renamed and \
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
/// - **An entry writable by an account this suite does not trust** — other-writable,
///   group-writable, or owned by neither the superuser nor this process — where that account can put
///   a file in place of the tool that was found there. The sticky bit is not an exemption: it
///   restrains deleting an entry, not creating one, and creating one is the whole of this attack.
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
                Ok(metadata) if untrusted_writer_reason(&metadata).is_some() => {
                    let reason = untrusted_writer_reason(&metadata).unwrap_or_default();
                    refused.push(format!(
                        "the PATH entry {shown:?} was skipped: {reason}, so an account other than \
                         this one could place a file there to be selected as an oracle"
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

/// What a child's `PATH` is, spelled for a report rather than for a child.
///
/// Kept beside [`sanitized_search_path`] so the two cannot describe different values, and returning a
/// sentence rather than an empty string in the `None` case, because an empty field in a fingerprint
/// reads as "not recorded" and this state — every entry refused — is a fact a reader must not
/// mistake for a gap in the record.
fn describe_child_search_path() -> String {
    match sanitized_search_path() {
        Some(path) => path.to_string_lossy().into_owned(),
        None => String::from(
            "(unset: no PATH entry survived the trust filter, so every child is given none)",
        ),
    }
}

/// The value of `PATH` every child of this suite receives: the trustworthy entries and no others.
///
/// # Why the child cannot be given the process's own `PATH`
///
/// [`search_path`] refuses an untrusted entry for tool *resolution*, so the reference compiler this
/// suite selects is never a file an untrusted account planted. That protects the driver and stops
/// there. A compiler driver is not one program: it locates and executes its own stages — `cc1`,
/// `cc1plus`, `as`, `ld`, `collect2` — and for the stages it does not find beside itself it searches
/// `PATH`. Handing the child the raw inherited `PATH` therefore reinstates, one level down, exactly
/// the substitution the resolution filter refused one level up: a `cc1` or an `as` planted in a
/// world-writable directory would be executed by a driver this suite had vetted, and every artefact
/// would still name the vetted driver. The same applies to each emulator, which reads `PATH` to
/// resolve helpers, and to the optional timeout utility.
///
/// The filter therefore has to hold for the whole process tree, not merely for the first process in
/// it, which is why the sanitized value is computed here and installed by
/// [`super::isolate_child_environment`] at every spawn site rather than at each call site's
/// discretion.
///
/// # Why it is memoized, and why it is exactly `search_path`'s accepted list
///
/// `PATH` belongs to the process and cannot change between two spawns within one run, so the value
/// is computed once. It is deliberately assembled from [`search_path`]'s accepted entries, in their
/// original order, rather than filtered again here: two filters that answer the same question are
/// two chances for the answers to differ, and a reader comparing the pre-flight report's list of
/// skipped entries against what a child actually received must find them consistent by construction.
///
/// # Why `None` is a value and not an error
///
/// `None` means no entry survived the filter — either `PATH` was unset, or every entry was relative
/// or writable by an account this suite does not trust. `PATH` is then left **unset** for the child
/// rather than set to the empty string, because an empty `PATH` is read as the current directory by
/// the same conventions that make an empty entry hazardous, so setting it would reintroduce the
/// hazard the filter exists to remove. A reference compilation then fails loudly with a driver that
/// cannot find its own stages, which is the correct outcome: on a machine whose entire search path is
/// writable by anyone, there is no honest oracle to be had, and failing to compile states that far
/// better than compiling with whatever was planted.
///
/// [`env::join_paths`] can only fail on an entry containing the separator, which an entry produced by
/// [`env::split_paths`] cannot contain. It is handled rather than unwrapped anyway, and it fails
/// closed to `None` for the reason above, because a control that panics is a control a maintainer
/// disables.
pub fn sanitized_search_path() -> Option<&'static OsString> {
    static SANITIZED: OnceLock<Option<OsString>> = OnceLock::new();
    SANITIZED
        .get_or_init(|| {
            let (accepted, _) = search_path();
            if accepted.is_empty() {
                return None;
            }
            env::join_paths(accepted.iter()).ok()
        })
        .as_ref()
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
        // Redaction before sanitization, and both before the text is retained. This is the single
        // funnel through which a tool's own output becomes a recorded string in this module — a
        // version banner and the kernel identification both come through here — and both of those
        // strings are written into the pre-flight report and into every finding's committed
        // environment fingerprint. `isolate_child_environment` stops a credential reaching a tool at
        // all; this closes the remaining path, where a tool that read one from a file echoes it back
        // in its banner and the suite would otherwise commit it to the repository.
        .map(|line| sanitize_text_for_report(&redact_secrets(line)))
}

/// What one bounded probe captured.
struct ProbeCapture {
    /// Standard output, truncated at [`PROBE_OUTPUT_BYTES_MAX`].
    stdout: Vec<u8>,
    /// Standard error, truncated at [`PROBE_OUTPUT_BYTES_MAX`].
    stderr: Vec<u8>,
    /// True when the child had to be terminated because it outlived [`PROBE_DEADLINE`].
    timed_out: bool,
    /// How the child ended, or `None` when it timed out or could not be waited on.
    ///
    /// Recorded rather than discarded because two probes in this module want opposite things from
    /// it. A banner probe deliberately ignores it — a usage message may exit non-zero while
    /// printing exactly the identification wanted. A target-reporting probe must not ignore it: a
    /// program that rejected the flag, or crashed on it, has said nothing about its target, and
    /// treating its silence as an answer is what made that check fail open.
    status: Option<ExitStatus>,
}

impl ProbeCapture {
    /// True when the child ran to completion and reported success.
    ///
    /// Both halves are required. A timeout leaves no status to consult, and a status that exists
    /// but is not success means the program declined the question rather than answered it.
    fn exited_successfully(&self) -> bool {
        !self.timed_out && self.status.is_some_and(|status| status.success())
    }

    /// How the child ended, phrased for a diagnostic.
    ///
    /// The text comes from the standard library's own rendering of an exit status, so it is
    /// generated rather than external and cannot carry hostile bytes into a report.
    fn completion_summary(&self) -> String {
        if self.timed_out {
            return format!(
                "it did not terminate within {} seconds and was killed",
                PROBE_DEADLINE.as_secs()
            );
        }
        match self.status {
            Some(status) if status.success() => String::from("it exited successfully"),
            Some(status) => format!("it ended with {status}"),
            None => String::from("it could not be waited on"),
        }
    }
}

/// Run one probe with a deadline, a capped output and a reaped direct child.
///
/// Every probe in this module goes through this function, and none of them may spawn a child any
/// other way. That single funnel is the fix for a real hazard: a probe target is not always a
/// trusted tool. Three of them are named by environment variables a maintainer or a continuous-
/// integration expression can set, so the executable being asked `--version` may be any program on
/// the machine — including one that loops without printing, prints without end, or waits for input
/// forever. Such a program would otherwise hang or exhaust discovery, and it would do so during
/// pre-flight — before any cell exists for the per-cell execution budget to govern — so the suite's
/// own runaway protection cannot engage on its behalf. This bound is the only thing standing there.
///
/// Six properties hold together, and each closes a distinct way a probe can fail to return or a
/// process can be left behind:
///
/// - **Standard input is the null device**, so any read of it sees end of file immediately and a
///   tool that waits for a maintainer to type cannot block.
/// - **Both output streams are read by their own thread, each bounded with
///   [`std::io::Read::take`]** at [`PROBE_OUTPUT_BYTES_MAX`]. This is what makes an endless printer
///   harmless: once a reader reaches its cap it stops reading and drops its end of the pipe, so
///   further output has nowhere to go. A writer that keeps writing then sees `EPIPE` or `SIGPIPE` —
///   which ends it only if it does not handle or ignore them, so this bounds the *harness's* memory
///   rather than guaranteeing the writer's death. Reading on separate threads is also what prevents
///   the classic deadlock in which a parent waits for a child that is itself blocked writing into a
///   full pipe.
/// - **The wait is bounded** by polling [`std::process::Child::try_wait`] until the deadline,
///   because the standard library offers no timed wait.
/// - **The direct child is always reaped.** On the deadline it is killed and then waited on, so no
///   zombie of it is left behind. That much is unconditional.
/// - **The child's process group is swept, best effort.** Each probe is placed in a group of its own
///   before it is spawned, and [`terminate_group_of`] sweeps that group on every exit path — the
///   ordinary one included — so a descendant the probe started of its own accord, a wrapper script's
///   `sleep` being the ordinary case, is cleaned up rather than left running. It is best effort for a
///   concrete reason: the standard library cannot signal a group, so the sweep needs a vetted `kill`
///   utility, and on a machine where none can be trusted it degrades to terminating the immediate
///   child. Its result is deliberately **discarded at this layer** rather than reported: a leaked
///   descendant of a `--version` invocation is untidy, not a fact about any compiler, and failing
///   pre-flight over cleanup would be the wrong trade. `execute.rs` does report the same condition,
///   because there a survivor holds the pipes a verdict is computed from. So a descendant can outlive
///   a probe on a degraded machine, which is exactly the situation the next property exists to
///   survive.
/// - **Collecting the output is bounded by the same deadline as the wait**, and this is the
///   property that is easy to omit and fatal to omit. Terminating a child does not close the pipe
///   it was writing to: any process that inherited the write end still holds it open, and a shell
///   script is the ordinary case — killing the shell leaves the `sleep` it had started alive, still
///   holding the pipe. A reader draining that pipe to end of file therefore blocks on the
///   *grandchild*, not on the child, so joining the reader threads would make the probe hostage to
///   a process it never started and cannot see. The symptom is measured and severe: without this
///   deadline a five-second probe budget produces a sixty-second probe. The readers are therefore
///   harvested through a channel with the remaining time as its timeout, and a reader that has not
///   delivered by then is abandoned rather than waited on. Abandoning it is safe and bounded: its
///   buffer cannot exceed the cap, it holds no lock, and it ends by itself when the last writer
///   finally closes the pipe.
///
/// Returns `None` only when the child could not be spawned at all. A probe that ran and timed out
/// returns its capture with [`ProbeCapture::timed_out`] set, because "the tool did not terminate"
/// is a fact about the environment worth recording rather than an absence to be silently ignored.
fn run_bounded_probe(command: &mut Command) -> Option<ProbeCapture> {
    // A probe is a program this suite did not write, so it gets the suite's fixed environment and
    // nothing else. Two things follow. A credential in the ambient environment cannot reach it, so
    // it cannot print one into a banner this module captures and writes into the pre-flight report
    // and into every finding's environment fingerprint. And a variable cannot change what it does:
    // a compiler driver reads a dozen include-path and option variables that alter the language it
    // accepts, so an inherited one would make the version and target this probe records describe a
    // configuration no cell reproduces.
    //
    // The private directory is the build root rather than a cell workspace, because a probe belongs
    // to no cell; it is where a tool that insists on writing a cache puts it.
    isolate_child_environment(command, &build_root());
    // Its own process group, so the whole tree can be terminated together. A compiler driver spawns
    // `cc1`; an emulator may fork. Killing only the process this module holds a handle to leaves
    // those descendants running, and a surviving descendant keeps the capture pipes open, so the
    // reader threads below never see end of file and the harvest waits out its full deadline for
    // bytes that will never arrive.
    own_process_group(command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // One deadline governs the wait and both harvests, so the whole probe is bounded rather than
    // each of its three phases being bounded separately and summing to three times the budget.
    let deadline = Instant::now() + PROBE_DEADLINE;
    let group = child.id();
    // Owned for the whole probe. A probe is short and ordinarily well behaved, which is exactly why an
    // unswept one is easy to overlook: the pre-flight runs dozens of them, and a probe of a tool that
    // forks would otherwise leave a descendant behind for every one. The guard covers an ending this
    // function unwinds through, and enrols the group with the external supervisor for the rest.
    let guard = register_process_group(group);
    let stdout_reader = child.stdout.take().map(spawn_capped_reader);
    let stderr_reader = child.stderr.take().map(spawn_capped_reader);
    let (timed_out, status) = await_child_within_deadline(&mut child, group, deadline);
    // Released after the wait, which is where this module performs its own sweep and reap.
    guard.release();
    let stdout = harvest_within_deadline(stdout_reader, deadline);
    let stderr = harvest_within_deadline(stderr_reader, deadline);
    Some(ProbeCapture {
        stdout,
        stderr,
        timed_out,
        status,
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
/// Returns `(timed_out, status)`. The status is `Some` on exactly one path — the child was observed
/// to finish on its own — and `None` on the two paths where no status exists to report: the deadline
/// was reached, or the status can no longer be observed at all. Keeping those three outcomes
/// distinct is what lets a caller that must not fail open tell "answered" from "did not answer",
/// while a caller that only wants a banner can go on ignoring the status entirely.
///
/// An error from [`std::process::Child::try_wait`] means the child's status can no longer be
/// observed, so waiting longer cannot help; the child is terminated and reaped on that path too,
/// which is what guarantees this function never leaves a process behind however it exits. That path
/// is deliberately *not* reported as a timeout, because the child did not outlive its budget — it
/// became unobservable, which is a different fact and yields a different diagnostic.
fn await_child_within_deadline(
    child: &mut Child,
    group: u32,
    deadline: Instant,
) -> (bool, Option<ExitStatus>) {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // The immediate child finished, which says nothing about what it started. A driver
                // that has already `exec`ed its stages, or an emulator whose guest forked, can leave
                // descendants holding the capture pipes; sweeping the group here is what makes the
                // harvest below terminate on its own rather than on its deadline.
                terminate_group_of(group);
                return (false, Some(status));
            }
            Ok(None) => {}
            Err(_) => {
                terminate_and_reap(child, group);
                return (false, None);
            }
        }
        if Instant::now() >= deadline {
            terminate_and_reap(child, group);
            return (true, None);
        }
        thread::sleep(PROBE_POLL_INTERVAL);
    }
}

/// Kill a child, reap it inside a deadline, and sweep its group.
///
/// A failed kill is still ignored on purpose: it is what a child that has already exited reports,
/// and the reap that follows is the postcondition rather than the kill.
///
/// # Why the reap is bounded rather than a plain wait
///
/// This function is the whole of what makes [`PROBE_DEADLINE`] a bound on the *probe* rather than
/// merely on the polling loop above it. A plain [`std::process::Child::wait`] here has no deadline of
/// its own, so a child that could not be torn down promptly would hold the pre-flight for as long as
/// it liked — and the pre-flight is what a run performs before it does anything else, with nothing
/// outside it to notice. [`reap_bounded`] kills and then polls to a stated deadline, so the probe's
/// five seconds is an end-to-end figure.
///
/// A child that could not be reaped even then is recorded as a run-level breach. That is deliberately
/// louder than the group sweep below, which stays unreported for the reason
/// [`terminate_group_of`] documents: a leaked descendant of a `--version` invocation is untidy,
/// whereas an unreaped child of this run's own is a process nothing can account for, and the
/// condition that produced it will produce it again on the thousands of cells that follow.
fn terminate_and_reap(child: &mut Child, group: u32) {
    if let ReapOutcome::Unreaped(detail) = reap_bounded(child) {
        record_infrastructure_breach(format!(
            "a pre-flight probe's child could not be accounted for: {detail}"
        ));
    }
    terminate_group_of(group);
}

/// Sweep the process group a probe was spawned into, so no descendant of it survives.
///
/// The standard library can place a child in its own group but cannot signal one, so the signal is
/// delivered by the `kill` utility with an argument vector and no shell. The utility is resolved
/// through the same trusted search every other tool goes through, and memoized, because this runs on
/// every probe and on both of a probe's exit paths.
///
/// The outcome is deliberately not reported here. A probe's product is an identification string, and
/// a leaked descendant of a `--version` invocation is untidy rather than a fact about the compiler
/// under test — turning it into a pre-flight refusal would fail a run over cleanup. The *cell*
/// execution path in `execute.rs` does report it, because there a surviving process holds the pipes
/// a verdict is computed from.
fn terminate_group_of(group: u32) {
    let _ = terminate_process_group(group, kill_tool());
}

/// The `kill` utility this run uses to signal a process group, if one could be trusted.
///
/// Resolved through the ordinary trusted search — an untrusted `kill` is a program that would be
/// handed a signal and a group identifier, so it is subject to the same substitution concern as any
/// other tool — and memoized for the life of the run. [`None`] means no such utility was found, in
/// which case group termination degrades to terminating the immediate child and says so.
///
/// Visible to the rest of the harness because `execute.rs` supervises cell executions the same way
/// and must not resolve, vet and memoize a second copy: two independently discovered `kill`
/// utilities could disagree about which file was trusted.
pub(super) fn kill_tool() -> Option<&'static Path> {
    signalling_tool().map(|(path, _)| path)
}

/// The signalling utility together with the identity recorded when it was resolved.
///
/// One memo rather than two, so the path every caller spawns and the identity
/// [`confirm_signalling_tool_unchanged`] compares can never describe different files. The identity is
/// itself optional: a utility whose identity could not be captured stays usable, because group
/// termination degrading to a single child is a worse outcome than an unconfirmed sweep, and the
/// absence is then simply nothing to confirm rather than a refusal.
fn signalling_tool() -> Option<(&'static Path, Option<&'static ToolIdentity>)> {
    static KILL_TOOL: OnceLock<Option<(PathBuf, Option<ToolIdentity>)>> = OnceLock::new();
    KILL_TOOL
        .get_or_init(|| {
            let resolved = resolve_tool(KILL_TOOL_NAME)?;
            if untrusted_reason(&resolved).is_some() {
                return None;
            }
            let identity = ToolIdentity::of(&resolved);
            Some((resolved, identity))
        })
        .as_ref()
        .map(|(path, identity)| (path.as_path(), identity.as_ref()))
}

/// Confirm `program` is still the signalling utility that was resolved, or say how it has changed.
///
/// Kept apart from [`Capabilities::confirm_tool_unchanged`] because the two tables have different
/// lifetimes: the capability record is built once, at pre-flight, while this utility is resolved
/// lazily the first time a process group has to be swept — which happens *during* pre-flight, on the
/// cleanup path of the very probes that build the capability record. Folding it into that record
/// would make its confirmation unavailable at exactly the moments it is first used.
///
/// Returns [`None`] when `program` is not that utility, when it was never resolved, when its identity
/// could not be captured, and when it is unchanged.
fn confirm_signalling_tool_unchanged(program: &Path) -> Option<String> {
    let (path, identity) = signalling_tool()?;
    if path != program {
        return None;
    }
    identity?
        .changed_since_vetting(program)
        .map(|change| format!("process-group signalling utility: {change}"))
}

/// The name under which the process-group signalling utility is looked up.
///
/// POSIX requires a `kill` utility, and every supported host provides one on the search path. It is
/// looked up by name rather than hard-coded to a directory so that a host which installs it in
/// `/bin` rather than `/usr/bin` still works.
const KILL_TOOL_NAME: &str = "kill";

/// The name under which the POSIX shell is looked up.
///
/// Needed for exactly one purpose: running the small program that supervises this run's process
/// groups from outside the process, so that a signal this process cannot catch still ends the
/// children it started. Looked up by name for the same reason `kill` is.
const SHELL_TOOL_NAME: &str = "sh";

/// The POSIX shell this run uses to host its process-group supervisor, if one could be trusted.
///
/// Resolved through the ordinary trusted search and memoized for the life of the run, exactly like
/// [`kill_tool`] — the two are used together, and a second discovery path would be a second chance to
/// disagree about which file on this machine was trusted. Trust matters more here than for most tools,
/// not less: this one is handed a program to execute.
///
/// [`None`] means no trusted shell was found, in which case the external supervisor is simply not
/// started. That is a disclosed degradation rather than a failure — the in-process guard that sweeps a
/// group when a cell unwinds is unaffected, and the pre-flight capability report states whether
/// external supervision is in force — for the same reason a missing `kill` utility does not fail a run:
/// refusing a whole matrix over a capability gap would be a worse answer than performing it with the
/// gap stated.
pub(super) fn shell_tool() -> Option<&'static Path> {
    static SHELL_TOOL: OnceLock<Option<PathBuf>> = OnceLock::new();
    SHELL_TOOL
        .get_or_init(|| {
            let resolved = resolve_tool(SHELL_TOOL_NAME)?;
            if untrusted_reason(&resolved).is_some() {
                return None;
            }
            Some(resolved)
        })
        .as_deref()
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

/// Whether the external `timeout` utility is wrapped around a launch, and why.
///
/// # What this decides, and what it deliberately does not
///
/// It decides one thing: whether the discovered utility is wrapped around a spawned command as an
/// **outer net**, a fixed margin beyond the per-cell budget. It decides nothing about how a timeout
/// is *established*. `execute.rs`'s own watchdog remains the authoritative bound on every path, a
/// timeout remains a fact this process observed rather than a status read back out of somebody
/// else's exit code, and every child remains in a process group of its own that is swept and
/// verified empty on every exit path. Declining the outer net therefore changes no verdict, which is
/// exactly why it can be decided on cost.
///
/// # Why the decision is behavioural rather than a bare presence test
///
/// An outer net that never fires still charges for supervising every child it wraps, and that charge
/// is the whole of what it costs a healthy run. Measured: the implementation installed in the
/// environment this suite was developed against polls its child on a 100 millisecond granularity and
/// costs roughly 103 milliseconds per invocation whatever the budget, which across a matrix of at
/// least 5,508 bounded invocations is about 569 seconds — roughly three times the suite's own
/// measured work. An implementation that supervises tightly costs single-digit milliseconds and is
/// worth engaging. Presence cannot tell those two apart; a measurement can, so
/// [`qualify_outer_net`] measures.
///
/// Either way the outcome is **reported**, never silent: the pre-flight capability report and the
/// environment fingerprint both state which decision was taken and the evidence for it, because a
/// reader comparing two runs' timings needs to know which of them was wrapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OuterNet {
    /// The utility is wrapped around every bounded launch, with the evidence that qualified it.
    Engaged {
        /// Absolute path of the vetted utility, as `execute.rs` and `compile.rs` will spawn it.
        path: PathBuf,
        /// Why it was engaged: the measured per-invocation cost, or the explicit request.
        evidence: String,
    },
    /// No wrapping happens, with the reason. The watchdog is the whole bound, as it always is.
    Declined {
        /// Why: no utility, an explicit `off`, or a measured cost above the ceiling.
        reason: String,
    },
}

impl OuterNet {
    /// The utility to wrap with, or `None` when nothing is wrapped.
    ///
    /// This is the single value every spawning site reads, which is what keeps the decision in one
    /// place: a caller cannot reach past it to the raw discovery result, because
    /// [`Capabilities::timeout_tool`] hands out the *record* — for reporting and fingerprinting —
    /// while only this returns a path to spawn.
    pub fn path(&self) -> Option<&Path> {
        match self {
            OuterNet::Engaged { path, .. } => Some(path.as_path()),
            OuterNet::Declined { .. } => None,
        }
    }

    /// One line stating the decision and its evidence, for a report or a fingerprint.
    pub fn summary(&self) -> String {
        let text = match self {
            OuterNet::Engaged { path, evidence } => format!(
                "engaged: {} wraps every bounded launch as an outer net {} second(s) beyond the \
                 per-cell budget ({evidence}). The watchdog in execute.rs remains the authoritative \
                 bound and no exit status of this utility is ever interpreted",
                shown_path(path),
                TIMEOUT_UTILITY_OUTER_MARGIN.as_secs(),
            ),
            OuterNet::Declined { reason } => format!(
                "declined: no external wrapper is spawned ({reason}). The watchdog in execute.rs is \
                 the whole bound, which it is authoritatively in either case, so no verdict changes \
                 — the decision is measured rather than configured, so there is nothing to override"
            ),
        };
        sanitize_text_for_report(&text)
    }
}

/// Decide whether the discovered `timeout` utility is engaged as an outer net.
///
/// Runs during discovery, once per process, so its cost is paid once rather than per cell.
///
/// # Measured, never configured
///
/// There is no override, and that is the point. The engagement is a *cost* decision — the watchdog in
/// `execute.rs` is the authoritative bound in either case — so the only question is what this machine's
/// implementation charges, and that is something to measure rather than to be told. An environment
/// variable able to force the answer either way would be declared in no document, would let a
/// malformed value silently become the default, and would not reach the environment fingerprint, so
/// one run's command topology could differ from another's with nothing in either run's artifacts to
/// show it. Making the decision a function of the machine avoids all three, and
/// [`OuterNet::summary`] records which way it went in the pre-flight report and in every finding.
///
/// # The measurement
///
/// The utility is asked to supervise a child that exits immediately — itself, printing its own
/// banner — under a one-second budget it therefore never reaches. What is measured is consequently
/// the supervision overhead alone and nothing else: no expiry is involved, no signal is sent, and no
/// exit status is interpreted. The child is the utility itself because that is the one program
/// qualification is already guaranteed to have: it is vetted, absolute, and known to answer a banner
/// argument promptly, so the probe introduces no dependency on any tool discovery has not cleared.
///
/// [`OUTER_NET_PROBE_SAMPLES`] samples are taken and the **minimum** is judged against
/// [`OUTER_NET_OVERHEAD_MAX`], because the estimator wanted is the implementation's floor and noise
/// on a shared machine only ever adds.
///
/// # What is not measured, and why that is sound
///
/// Whether the utility actually terminates a child that *does* outlive its budget is not probed.
/// Three reasons, in order of weight. The watchdog is authoritative, so a net that failed to fire
/// would cost the run nothing that the watchdog does not already cover. Probing expiry needs a child
/// that outlives a whole second, which needs a sleeping program this module has not vetted, and
/// vetting one to test a net would be a larger dependency than the net is worth. And the expiry
/// defects actually measured in the installed implementation — a signal that is never sent under
/// `--signal=KILL`, and an indefinite hang under `--kill-after` — are in spellings this suite never
/// passes; the plain spelling it does pass measured correct.
fn qualify_outer_net(tool: &ToolRecord) -> OuterNet {
    let Some(path) = tool.path() else {
        return OuterNet::Declined {
            reason: format!(
                "no per-cell timeout utility was discovered ({})",
                tool.diagnosis()
            ),
        };
    };
    match measure_outer_net_overhead(path) {
        None => OuterNet::Declined {
            reason: format!(
                "{} could not be measured supervising a child that exits immediately, so its \
                 per-invocation cost is unknown and it is not engaged",
                shown_path(path)
            ),
        },
        Some(floor) if floor > OUTER_NET_OVERHEAD_MAX => OuterNet::Declined {
            reason: format!(
                "{} charges at least {} ms per invocation supervising a child that exits \
                 immediately, above the {} ms ceiling; an outer net that never fires in a healthy \
                 run would add roughly {} s across the matrix's 5,508 bounded invocations",
                shown_path(path),
                floor.as_millis(),
                OUTER_NET_OVERHEAD_MAX.as_millis(),
                floor.as_millis().saturating_mul(5_508) / 1_000,
            ),
        },
        Some(floor) => OuterNet::Engaged {
            path: path.to_path_buf(),
            evidence: format!(
                "measured at {} ms per invocation supervising a child that exits immediately, \
                 within the {} ms ceiling",
                floor.as_millis(),
                OUTER_NET_OVERHEAD_MAX.as_millis(),
            ),
        },
    }
}

/// The floor of what this utility costs to supervise a child that exits immediately.
///
/// `None` when the probe could not be spawned at all, or when the utility did not report success
/// supervising a trivially successful child — either way its behaviour has not been established, and
/// an unestablished implementation is not engaged.
fn measure_outer_net_overhead(tool: &Path) -> Option<Duration> {
    let mut floor: Option<Duration> = None;
    for _ in 0..OUTER_NET_PROBE_SAMPLES {
        let started = Instant::now();
        let capture = run_bounded_probe(
            Command::new(tool)
                .arg(OUTER_NET_PROBE_BUDGET_SECS.to_string())
                .arg(tool)
                .arg(VERSION_ARGUMENTS[0]),
        )?;
        let elapsed = started.elapsed();
        if !capture.exited_successfully() {
            return None;
        }
        floor = Some(match floor {
            Some(previous) if previous <= elapsed => previous,
            _ => elapsed,
        });
    }
    floor
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

/// Whether a tool's identity participates in the oracle-independence decision.
///
/// The distinction exists because one vetting rule — refusing a launcher whose implementation cannot
/// be established — is sound for a compiler and a false positive for anything else. Oracle
/// independence is a property of *compilers*: two names that hand their work to one driver make
/// oracle (a) compare a compiler with itself. No such hazard exists for an emulator, a timeout
/// utility or a test-case reducer, none of which produces or compares a program's output.
///
/// Measured on the reference host, which is why this is a scoped rule rather than a blanket one: the
/// installed reducer is a Perl script, so it begins with a shebang and names no absolute program on
/// an `exec` line. Refusing it under strict mode would remove a working tool for no benefit — it
/// would make finding minimization manual on precisely the unattended runs that produce findings —
/// while protecting nothing, because a reducer's identity is not an input to any comparison.
///
/// Every other check in [`vet_resolved_tool`] applies to both kinds, and deliberately so: an
/// emulator that anyone on the machine could substitute is as much a hazard as a compiler that could
/// be, since it is what actually runs the program whose output becomes a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    /// The compiler under test, or a reference driver. Its identity decides oracle independence.
    Compiler,
    /// A supporting utility: an emulator, the timeout utility, the reducer.
    Support,
}

/// One discovered — or genuinely absent — tool.
///
/// A record is kept for a tool that could not be found, rather than the tool simply being
/// missing from a collection, because the report has to be able to say what was looked for
/// and how to supply it. An absent tool with no record would be an absent tool with no
/// diagnosis.
///
/// # Why every field is private
///
/// This record is the *outcome of vetting*, and a writable field would let a caller reinstate
/// exactly what vetting rejected. Three fields make that concrete, and they are the three a
/// well-meaning caller is most likely to reach for:
///
/// * **`path` and `rejection` are a matched pair.** A refusal clears the path, and every
///   availability question in the module — which arms run, which cells are unavailable, whether
///   strict mode escalates to a failure — is answered from the path being empty. Writing a path
///   back would restore a refused tool to service while leaving its own refusal recorded beside it,
///   so the report would explain why the tool was declined and the run would use it anyway.
/// * **`identity` is what oracle independence is decided on.** Substituting one would let two arms
///   backed by the same compiler pass the distinctness check, and oracle (a) would then compare a
///   compiler with itself and agree by construction. That is the one failure this whole module
///   exists to prevent, and it would leave no trace in any report.
/// * **`source` and `overridden` are how a diagnosis chooses its remedy.** They decide whether a
///   maintainer is told to edit a variable or install a package, and a wrong answer there sends
///   them to fix something that is not broken.
///
/// The accessors below return borrows and copies, so a caller can report anything and change
/// nothing. Construction goes through [`ToolRecord::discover`] or
/// [`ToolRecord::from_build_system`], each of which runs [`vet_resolved_tool`] and is therefore
/// unable to produce a record whose path survived a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRecord {
    /// What the tool does in this suite, for example `reference compiler (aarch64 arm of
    /// oracle (a))`.
    role: String,
    /// The environment variable that overrides this tool, when the catalogue defines one.
    ///
    /// `None` for the two optional auxiliary tools, which the catalogue deliberately gives no
    /// override: inventing a variable for them would add configuration the specification does not
    /// define, and neither tool changes a verdict, so probing the standard name is sufficient.
    override_variable: Option<&'static str>,
    defaults: &'static [&'static str],
    /// The candidates actually tried, in order. Equal to the override alone when one was set,
    /// and to the catalogued defaults otherwise, which is what lets the report distinguish a
    /// bad override from a genuinely missing package.
    candidates: Vec<String>,
    /// True when the override variable supplied the candidate list, whether or not the
    /// candidate resolved.
    ///
    /// Kept separately from the `source` field, which records only how a *successful*
    /// resolution happened. The distinction is what lets an unresolved tool be diagnosed
    /// correctly: a bad override is fixed by editing a variable, while a missing default is
    /// fixed by installing a package, and the two must not be described as each other.
    overridden: bool,
    /// The raw state of the override variable, for the provenance report only.
    ///
    /// `overridden` answers "did the override supply the candidate list", which is the question every
    /// decision turns on. This answers the narrower one the report needs: an override that was
    /// exported empty did not supply a candidate list either, and calling it unset in the sentence a
    /// reader checks their own configuration against would contradict what they did.
    override_state: OverrideState,
    source: ToolSource,
    /// Absolute or `PATH`-resolved path to the executable, when one was found.
    path: Option<PathBuf>,
    /// First line of the tool's banner, when one could be captured. `None` means the tool
    /// exists but declined to identify itself, which is recorded rather than treated as an
    /// error.
    version: Option<String>,
    /// Which file the resolved path actually denotes, captured once at discovery.
    ///
    /// Present exactly when [`ToolRecord::path`] is, and it is what makes oracle independence
    /// checkable: a name says what a tool is called, an identity says which file it is.
    identity: Option<ToolIdentity>,
    /// Why an otherwise-executable candidate was refused, when one was.
    ///
    /// A refusal is recorded rather than being allowed to look like an absent package, and it
    /// leaves [`ToolRecord::path`] empty so the affected oracle arm is reported unavailable — which
    /// under strict mode is escalated to a failure. That composition is deliberate: a tool the
    /// suite declines to trust must not quietly become a tool the suite silently did without.
    rejection: Option<String>,
    /// What the tool says it targets, for a driver that can be asked.
    ///
    /// Recorded for the report and the fingerprint. A driver whose stated target contradicts the
    /// arm it was selected for is refused through the `rejection` field, because a reference
    /// compiler that builds for the wrong architecture is not a weaker oracle but a false one.
    provenance: Option<String>,
}

impl ToolRecord {
    /// Discover one tool from its override variable and its probe list.
    ///
    /// When the override is set, it is the only candidate: a maintainer who names a tool
    /// explicitly must be told that that tool is missing, not silently given a different one
    /// that happened to be on `PATH`. When it is unset, the catalogued defaults supply the search
    /// order, and the **first of them that exists** becomes the candidate.
    ///
    /// Exactly one candidate is vetted, and a refusal ends the search rather than advancing it. The
    /// asymmetry with the sentence above is only apparent: in both cases the record reports the
    /// candidate it actually had, and in neither case is a tool the suite declined to trust replaced
    /// by a different implementation behind the reader's back. [`DEFAULT_REF_CC`] carries the full
    /// argument, including the measurement that decided it.
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
        kind: ToolKind,
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
            override_variable,
            kind,
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
            override_state: override_variable.map_or(OverrideState::Unset, override_state),
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
    /// build system rather than from a search, so there is nothing to probe on `PATH`.
    ///
    /// # Provenance is not a substitute for vetting
    ///
    /// It is tempting to treat a build-system path as trusted because no environment variable
    /// chose it, but the two checks this shares with a discovered tool do not depend on who named
    /// the path — they depend on what is at the end of it:
    ///
    /// * **Identity must be capturable.** [`ToolIdentity::of`] can fail while the path still passes
    ///   an executable-file test, because the two happen at different instants: a build directory
    ///   being rewritten by a concurrent build is the ordinary case, and it produces exactly this
    ///   window. Accepting a path with no captured identity would leave the compiler under test as
    ///   the one tool in the run whose file cannot be named — and it is the tool every oracle
    ///   independence check is made *against*, so its identity is the one that matters most.
    /// * **Under strict mode its location must be trustworthy.** A build directory is an ordinary
    ///   place for a compiler to be, and an interactive run is deliberately exempt for that reason.
    ///   An unattended job is not: a world-writable build tree, or a world-writable directory
    ///   anywhere above it, is a substitution opportunity for the binary whose output the entire
    ///   suite is about to attribute to this project.
    ///
    /// Only the fixed-target probe is omitted, and that omission is principled rather than
    /// convenient: a reference cross driver serves exactly one architecture and can be asked which,
    /// whereas the compiler under test is asked for a different target in every cell, so there is no
    /// single answer for it to give.
    ///
    /// A refusal is recorded in the `rejection` field and clears the path, exactly as it does
    /// for a discovered tool. For this record specifically, an empty path is a hard failure at the
    /// caller — there is no degraded mode in which the suite runs without the compiler it exists to
    /// test.
    fn from_build_system(
        role: impl Into<String>,
        override_variable: Option<&'static str>,
        path: PathBuf,
        strict: bool,
    ) -> ToolRecord {
        let identity = ToolIdentity::of(&path);
        let (rejection, provenance) = vet_resolved_tool(
            Some(&path),
            identity.as_ref(),
            override_variable,
            ToolKind::Compiler,
            None,
            strict,
        );
        let accepted = match rejection {
            Some(_) => None,
            None => Some(path.clone()),
        };
        let source = match accepted {
            Some(_) => ToolSource::CargoBinary,
            None => ToolSource::Unresolved,
        };
        let version = accepted.as_deref().and_then(probe_version);
        ToolRecord {
            role: role.into(),
            override_variable,
            defaults: &[],
            candidates: vec![shown_path(&path)],
            overridden: false,
            override_state: override_variable.map_or(OverrideState::Unset, override_state),
            source,
            identity: accepted.as_ref().and(identity),
            rejection,
            provenance,
            path: accepted,
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

    /// The full provenance of one tool, as the pre-flight report states it.
    ///
    /// [`ToolRecord::summary`] is one line and answers "what will run"; this answers "why *this*
    /// file, and what is known about it", which is the question a maintainer actually has when an
    /// arm is unexpectedly missing or a divergence has to be attributed to toolchain drift. Every
    /// fact is read through this type's own accessors rather than its fields, so the detail block and
    /// every other consumer see the record the same way and no field can be read in a form the
    /// constructor would not have produced.
    ///
    /// Returned as lines rather than one string so the caller controls indentation, and so a report
    /// can interleave these with its own headings without re-splitting text.
    /// How this record's override variable failed to supply a candidate, in the report's own words.
    ///
    /// Called only on the two arms that reached the catalogued default, so the variable is known not
    /// to have named anything. Which of the two ways it did not name anything is what this
    /// distinguishes: absent from the environment, or present and holding nothing usable. The second
    /// is a deliberate rule of the catalogue rather than a quirk — an empty or whitespace-only value
    /// names no tool, and there is nothing to resolve — but a report that called it "unset" would
    /// contradict the operator who exported it and leave them unable to predict this outcome from the
    /// report they are reading.
    ///
    /// The third state is reachable only on a path a real run never takes, and is still given its own
    /// words rather than folded into either neighbour. A variable holding bytes that are not valid
    /// text is present and does name something, yet the resolution path reads it as absent — and
    /// [`validate_environment`] refuses exactly that value, with the bytes shown, before the first
    /// tool is resolved. Should it ever be reached, "unset" and "empty" would both be false, so
    /// neither is what it says.
    fn override_absence_phrase(&self) -> &'static str {
        match self.override_state {
            OverrideState::Unset => "is unset",
            OverrideState::Blank => "is set but empty, which this catalogue treats as absent",
            OverrideState::Set => {
                "is set to a value that could not be read as text, so nothing could be resolved \
                 from it"
            }
        }
    }

    pub fn provenance_detail(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "{} — located by {}",
            self.role(),
            self.source().label()
        )];
        match self.override_variable() {
            Some(variable) if self.overridden() => lines.push(format!(
                "  override {variable} is set and selected {}",
                join_quoted(self.candidates())
            )),
            Some(variable) if self.defaults().is_empty() => lines.push(format!(
                "  override {variable} {} and this tool has no name to probe; its path came from \
                 the build system",
                self.override_absence_phrase()
            )),
            // The list is a *probe order*, and saying only that it was "probed in order" would imply
            // every name in it was tried. One was: the first that exists on PATH becomes the
            // candidate, and it is the only one vetted. Stating that here is what keeps this block
            // an account of what happened rather than of what the catalogue contains — the REFUSED
            // line below, when there is one, then reads as the fate of a named candidate rather than
            // of the whole list.
            Some(variable) => lines.push(format!(
                "  override {variable} {}; probe order {}, of which the first found on PATH is the \
                 candidate vetted",
                self.override_absence_phrase(),
                join_quoted_static(self.defaults())
            )),
            None => lines.push(format!(
                "  no override variable; probe order {}, of which the first found on PATH is the \
                 candidate vetted",
                join_quoted_static(self.defaults())
            )),
        }
        match self.identity() {
            // The identity is the answer to "which file, exactly" — device and inode, the link it
            // resolved through, and the compiler behind a launcher — and it is what a later
            // divergence is attributed against.
            Some(identity) => lines.push(format!("  identity {}", identity.describe())),
            None => lines.push(String::from(
                "  no identity was captured, so this tool is not in service",
            )),
        }
        lines.push(format!(
            "  version {}",
            self.version().unwrap_or("not reported")
        ));
        if let Some(note) = self.provenance() {
            lines.push(format!("  note {note}"));
        }
        if let Some(reason) = self.rejection() {
            lines.push(format!("  REFUSED {reason}"));
            // Said only for a *probed* candidate, and only when one was refused, because this is
            // exactly where a reader would otherwise assume the remaining names were tried next. The
            // order deliberately stops: the next name is usually a different compiler, and swapping
            // one in silently would change the dialect the corpus is judged against, the diagnostics
            // the warning gate is calibrated on, and the sanitizer runtime its second half needs. So
            // the arm is reported unavailable and the override is the way to name the right tool.
            // A record whose path came from the build system has no probe list at all, so the note
            // is withheld there: it would describe an order that was never consulted.
            let probed = !self.overridden() && !self.defaults().is_empty();
            if let (true, Some(variable)) = (probed, self.override_variable()) {
                lines.push(format!(
                    "  the probe order stopped at that candidate: a refused tool is reported rather \
                     than replaced by the next name in the list, because the next name is a \
                     different implementation and substituting it would change what every \
                     comparison on this arm is judged against — set {variable} to the intended tool"
                ));
            }
        }
        lines
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
    /// The default names the catalogue probes for this tool, in order.
    pub fn defaults(&self) -> &'static [&'static str] {
        self.defaults
    }

    /// The captured banner, when one could be captured.
    ///
    /// Prefer [`ToolRecord::version_or_unknown`] for report text: it substitutes a stable marker for
    /// an absent banner, which is what makes two fingerprints taken on the same machine compare
    /// equal.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// How a successful resolution happened, or that none did.
    pub fn source(&self) -> ToolSource {
        self.source
    }

    /// What this tool does in the suite, for a report row or a diagnostic.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Why an otherwise-executable candidate was refused, when one was.
    ///
    /// A `Some` here always accompanies an empty [`ToolRecord::path`], so a refused tool reports as
    /// unavailable with an explanation rather than as an absent package.
    pub fn rejection(&self) -> Option<&str> {
        self.rejection.as_deref()
    }

    /// What the tool says it targets, for a driver that can be asked.
    pub fn provenance(&self) -> Option<&str> {
        self.provenance.as_deref()
    }

    /// The environment variable that overrides this tool, when the catalogue defines one.
    pub fn override_variable(&self) -> Option<&'static str> {
        self.override_variable
    }

    /// True when the override variable supplied the candidate list, resolved or not.
    pub fn overridden(&self) -> bool {
        self.overridden
    }

    /// Which file the resolved path denotes, captured once at discovery.
    ///
    /// Present exactly when [`ToolRecord::path`] is. This is the value oracle independence is
    /// decided on, through [`ToolIdentity::is_same_implementation`].
    pub fn identity(&self) -> Option<&ToolIdentity> {
        self.identity.as_ref()
    }

    /// The candidates actually tried, in order.
    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }
}

/// Decide whether a resolved candidate may be used, and record what it says it targets.
///
/// Returns `(rejection, provenance)`. A `Some` rejection means the candidate was found and then
/// declined, which leaves the tool unavailable with an explanation rather than silently trusted.
/// Four conditions are checked, in the order that produces the most useful diagnostic:
///
/// 0. **Its path must be expressible as text without loss.** Every command this suite records is
///    text, so a path that does not decode as UTF-8 cannot be written into a reproduction command
///    without substituting a replacement character for the bytes that do not — at which point the
///    recorded command names a different file from the one that ran, and the identity vetted below
///    is not the identity of what the recorded command would launch. The record parser already
///    refuses to render such a path; refusing it here instead converts a failure buried in one cell
///    into a single pre-flight line naming the tool, the reason and the remedy.
/// 1. **It must still be the file it was a moment ago.** Resolution and use are two separate
///    instants, and a path that resolved but can no longer be identified as a regular file has
///    changed underneath the search — a dangling link, a replaced entry, or a deletion. This is the
///    time-of-check-to-time-of-use window itself, and while no check can close it entirely, the
///    identity captured here is what the run is conducted and reported against.
/// 2. **A compiler must not be an unattested wrapper, in any mode.** This is not a strictness knob.
///    Oracle independence rests on the two compilers being different implementations, and a script
///    whose implementation cannot be determined may hand its work to the driver the other arm uses —
///    which produces a comparison of a compiler with itself, agreeing by construction and reporting
///    a *pass*. A false pass is exactly as damaging interactively as unattended.
/// 3. **Under `strict` its location must be trustworthy; otherwise the weakness is recorded.** The
///    location rule refuses other-writable and group-writable paths and paths owned by an untrusted
///    account, over the file, over the link it resolves through, and over the implementation behind a
///    launcher, with the sticky bit granting no exemption. Interactive runs are deliberately exempt
///    from the *refusal*: a maintainer building a compiler into a scratch directory is doing
///    something ordinary, and refusing it would make the suite unusable for the person most likely to
///    run it. They are no longer exempt from the *report* — the weaker guarantee is stated in the
///    tool's diagnosis, so an interactive run never implies a protection it does not have.
/// 4. **A driver must target the arm it was chosen for.** Asked with the reference compiler's own
///    target-reporting flag — a probe flag, never a differential one — a cross driver states its
///    target, and a driver that states the wrong one is refused. This closes the case a name cannot
///    catch: pointing an architecture's override at the native compiler would produce an oracle
///    that compiled for the host while claiming to compile for that architecture, and the arm would
///    then be a false authority rather than a missing one. A driver that declines to answer is
///    accepted with its silence recorded, because the flag is a convention rather than a guarantee.
fn vet_resolved_tool(
    resolved: Option<&Path>,
    identity: Option<&ToolIdentity>,
    override_variable: Option<&'static str>,
    kind: ToolKind,
    expected_target: Option<Target>,
    strict: bool,
) -> (Option<String>, Option<String>) {
    let Some(path) = resolved else {
        return (None, None);
    };
    let shown = shown_path(path);
    if path.to_str().is_none() {
        let remedy = match override_variable {
            Some(variable) => format!(
                "Set {variable} to a path whose every byte is valid UTF-8, or install the tool \
                 somewhere whose name is"
            ),
            None => String::from(
                "Install the tool somewhere whose name is entirely valid UTF-8, or remove the \
                 directory holding it from PATH",
            ),
        };
        return (
            Some(format!(
                "{shown} was found but its path is not valid UTF-8, so it is refused. Every \
                 reproduction command this suite records is text, and a path that is not text can \
                 only be written into one by substituting a replacement character for the bytes \
                 that do not decode. That would make two of the suite's own guarantees false at \
                 once: the command a maintainer copies out of a finding would name a *different* \
                 file from the one that ran, and the tool whose identity was vetted would not be \
                 the tool the recorded command launches. Refusing here rather than at render time \
                 is deliberate — it turns a failure buried in one cell of a 1,296-cell matrix into \
                 one pre-flight line that names the tool and the fix. {remedy}"
            )),
            None,
        );
    }
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
    // An unattested compiler wrapper is refused in **every** mode, not only under strict. This is
    // deliberately not a strictness knob: oracle independence is the property that makes a
    // divergence mean anything at all, and it rests entirely on the two compilers being different
    // implementations. A script whose implementation cannot be determined might hand its work to the
    // very driver the other arm uses, in which case the comparison compares a compiler with itself
    // and agrees by construction — a *false pass*, not a missing check. A false pass is exactly as
    // damaging in an interactive run as in an unattended one, so the interactive exemption that
    // applies to a tool's *location* cannot be extended to its *identity*.
    if kind == ToolKind::Compiler && identity.is_some_and(ToolIdentity::is_unattested_wrapper) {
        return (
            Some(format!(
                "{shown} was found but is refused: it is a script whose implementation could not be \
                 determined. An attested wrapper is a shebang, comment lines, and exactly one \
                 unconditional `exec` of an absolute program as its last statement; this file does \
                 not match that shape, so which compiler it ultimately runs is unknown. That is the \
                 one fact oracle independence rests on — two names that appear distinct may hand \
                 their work to a single driver, in which case a differential comparison would \
                 compare a compiler with itself and agree by construction, reporting a pass rather \
                 than a missing oracle. This refusal is unconditional and is not relaxed by unsetting \
                 {VAR_STRICT}: point this tool's override at the driver itself rather than at a \
                 launcher"
            )),
            None,
        );
    }
    // A location an untrusted account can write to is refused under strict mode and *reported* in
    // every mode. The interactive exemption is kept on purpose — a maintainer building a compiler
    // into a scratch directory is doing something ordinary, and refusing it would make the suite
    // unusable for the person most likely to run it — but it is no longer silent, so an interactive
    // run states the weaker guarantee it is operating under instead of implying the stronger one.
    if let Some(reason) = untrusted_location(path, identity) {
        if strict {
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
        let note = format!(
            "{shown} is used with a weaker guarantee than strict mode would allow, because \
             {reason}. It is accepted here since {VAR_STRICT} is unset and building a compiler into \
             a writable directory is ordinary interactive practice, but a divergence involving this \
             tool cannot be attributed to the tool with certainty. Set {VAR_STRICT} to refuse it \
             instead"
        );
        let Some(target) = expected_target else {
            return (None, Some(note));
        };
        let (refusal, target_note) = vet_reference_driver(path, &shown, target);
        return (
            refusal,
            Some(match target_note {
                Some(extra) => format!("{note}. {extra}"),
                None => note,
            }),
        );
    }
    let Some(target) = expected_target else {
        return (None, None);
    };
    vet_reference_driver(path, &shown, target)
}

/// Every question a reference driver must answer about itself, asked in one place.
///
/// Two properties, and each on its own is insufficient. [`vet_declared_target`] establishes *what the
/// driver builds for*, and [`vet_language_mode`] establishes *what language it compiles*. A driver can
/// satisfy either while failing the other: two releases of the same compiler report an identical
/// triple and default to different revisions of C, and a driver for the wrong architecture may default
/// to exactly the right revision. Both are asked, in that order — the target first, because a driver
/// serving the wrong architecture is the more fundamental mistake and its refusal reads more usefully
/// than a language complaint about a compiler that was never the right one.
///
/// The notes are joined rather than one replacing the other, so the capability report states both
/// facts about every accepted arm: the target it declared and the language mode that was proven for
/// it. A report that showed only one would leave the other looking unchecked.
fn vet_reference_driver(
    path: &Path,
    shown: &str,
    target: Target,
) -> (Option<String>, Option<String>) {
    let (target_refusal, target_note) = vet_declared_target(path, shown, target);
    if target_refusal.is_some() {
        return (target_refusal, target_note);
    }
    let (mode_refusal, mode_note) = vet_language_mode(path, shown, target);
    let note = match (target_note, mode_note) {
        (Some(first), Some(second)) => Some(format!("{first}, {second}")),
        (Some(only), None) | (None, Some(only)) => Some(only),
        (None, None) => None,
    };
    (mode_refusal, note)
}

/// Ask a cross driver which target it builds for, and refuse it if the answer is wrong or absent.
///
/// Split out of [`vet_resolved_tool`] so that the two paths which need it — a tool with a
/// trustworthy location, and a tool accepted interactively with a recorded weaker guarantee — run the
/// same check rather than one of them skipping it. A driver that is accepted with a caveat still has
/// to be the driver for the arm it was chosen for; the caveat is about *who could tamper with it*,
/// not about *what it is*.
fn vet_declared_target(
    path: &Path,
    shown: &str,
    target: Target,
) -> (Option<String>, Option<String>) {
    let unanswered = |cause: String| {
        (
            Some(format!(
                "{shown} was found but did not state which target it builds for: asked with \
                 {DUMP_MACHINE_FLAG}, {cause}. It is refused rather than trusted, because the only \
                 thing that distinguishes the driver for {target} from any other program on this \
                 machine is its own answer to that question. Accepting silence would let an \
                 override aimed at the wrong driver — or at something that is not a compiler at all \
                 — stand in as the authority for the {target} arm. Point {} at a driver that \
                 answers {DUMP_MACHINE_FLAG} with a {target} triple, or unset it to probe the \
                 documented defaults",
                reference_variable_for(target)
            )),
            Some(format!("target not stated ({cause})")),
        )
    };
    match probe_dumpmachine(path) {
        MachineAnswer::Named(machine) => match machine_mismatch(&machine, target) {
            None => (None, Some(format!("reports target {machine}"))),
            Some(mismatch) => (
                Some(format!(
                    "{shown} was found but reports that it targets {machine}, while it was \
                     selected as the driver for {target} ({}): {mismatch}. A reference compiler \
                     from a different platform family than the arm it serves is not a weaker \
                     oracle but a false one: every comparison on that arm would be against a \
                     program built for something else. Point {} at a driver that targets {target}, \
                     or unset it to probe the documented defaults",
                    target.triple(),
                    reference_variable_for(target)
                )),
                Some(format!("reports target {machine}")),
            ),
        },
        MachineAnswer::NotLaunched => unanswered(String::from("it could not be launched")),
        MachineAnswer::NoCompletion(completion) => unanswered(completion),
        MachineAnswer::Declined(completion) => unanswered(completion),
        MachineAnswer::Silent => {
            unanswered(String::from("it exited successfully but printed no target"))
        }
    }
}

/// Ask a reference driver which language it compiles by default, and refuse it if that is the wrong
/// language.
///
/// # Why the language mode has to be proved rather than assumed
///
/// The oracle's authority rests on it compiling *the same language* the corpus was written in. Nothing
/// else in discovery establishes that. A driver's name does not: `gcc` is whichever version the
/// distribution made the default, and distributions advance that default — measured on the reference
/// host, `/usr/bin/gcc` is a release whose default is C23 while the pinned `gcc-13` beside it defaults
/// to C17. Its `-dumpmachine` answer does not either: both report the same triple. And the suite
/// cannot repair a wrong answer afterwards, because it passes **no** standard-selection flag: the
/// compiler under test has none, so passing one to the reference compiler alone would put a flag in a
/// differential invocation that only one side honours — precisely the discipline requirement 3 sets.
///
/// So the mode is asked for, and a driver that answers wrongly is refused. Refused, rather than noted:
/// an oracle compiling a different revision of C produces verdicts that look exactly like ordinary
/// ones, and a comparison against a false authority is worse than a missing arm, because a missing arm
/// is reported and this would not be.
///
/// Two conditions, each the failure of a real defect:
///
/// 1. `__STDC_VERSION__` must be stated and lie between [`MIN_STDC_VERSION`] and
///    [`MAX_STDC_VERSION`]. Below the range the driver would reject the corpus's C11 constructs;
///    above it, C23 changes the meaning of constructs the corpus contains — the constant's own
///    documentation records the measurement.
/// 2. [`STRICT_ANSI_MACRO`] must be absent. Requirement 2 mandates GNU extensions — statement
///    expressions, `typeof`, computed goto, case ranges — and a driver whose default mode is strictly
///    conforming rejects them. Measured: `-std=c11` predefines that macro while the default mode of
///    every driver this suite selects does not, so the macro genuinely distinguishes the two.
///
/// Split out of [`vet_resolved_tool`] for the same reason [`vet_declared_target`] is, and called from
/// the same two places, so a driver accepted with a recorded weaker guarantee is still checked.
fn vet_language_mode(path: &Path, shown: &str, target: Target) -> (Option<String>, Option<String>) {
    let unanswered = |cause: String| {
        (
            Some(format!(
                "{shown} was found but did not state which language it compiles by default: asked \
                 with `{}`, {cause}. It is refused rather than trusted, because this suite passes no \
                 standard-selection flag — the compiler under test has none to match — so the \
                 driver's own default mode *is* the language every comparison on the {target} arm is \
                 judged against, and an unproven language makes each of those comparisons \
                 unattributable. Point {} at a driver whose default mode is C11 or C17 with the GNU \
                 extensions enabled, or unset it to probe the documented defaults",
                LANGUAGE_MODE_ARGUMENTS.join(" "),
                reference_variable_for(target)
            )),
            Some(format!("language mode not stated ({cause})")),
        )
    };
    match probe_language_mode(path) {
        LanguageModeAnswer::Stated { version, strict } => {
            let mut faults: Vec<String> = Vec::new();
            if version < MIN_STDC_VERSION {
                faults.push(format!(
                    "it compiles {STDC_VERSION_MACRO} {version}L by default, older than the \
                     C11 ({MIN_STDC_VERSION}L) this corpus is written in, so it would reject the \
                     corpus rather than judge it"
                ));
            }
            if version > MAX_STDC_VERSION {
                faults.push(format!(
                    "it compiles {STDC_VERSION_MACRO} {version}L by default, newer than the C17 \
                     ({MAX_STDC_VERSION}L) this corpus is written in, and the difference is \
                     observable in the corpus itself — a UTF-8 string literal changes type, and \
                     `bool`, `true` and `false` become keywords"
                ));
            }
            if strict {
                faults.push(format!(
                    "it predefines {STRICT_ANSI_MACRO}, so its default mode rejects the GNU \
                     extensions requirement 2 mandates — statement expressions, `typeof`, computed \
                     goto and case ranges are all exercised by this corpus"
                ));
            }
            if faults.is_empty() {
                return (
                    None,
                    Some(format!(
                        "compiles {STDC_VERSION_MACRO} {version}L by default, GNU extensions enabled"
                    )),
                );
            }
            (
                Some(format!(
                    "{shown} was found but does not compile the language this suite compares \
                     against: {}. No standard-selection flag can correct it, because passing one to \
                     the reference compiler alone would put a flag in a differential invocation that \
                     the compiler under test does not honour. Point {} at a driver whose default mode \
                     is C11 or C17 with the GNU extensions enabled — a version-suffixed driver such \
                     as `gcc-13` pins that, where the unsuffixed name follows the distribution's \
                     current default — or unset it to probe the documented defaults",
                    faults.join("; and "),
                    reference_variable_for(target)
                )),
                Some(format!("compiles {STDC_VERSION_MACRO} {version}L by default")),
            )
        }
        LanguageModeAnswer::NotLaunched => unanswered(String::from("it could not be launched")),
        LanguageModeAnswer::NoCompletion(completion) => unanswered(completion),
        LanguageModeAnswer::Declined(completion) => unanswered(completion),
        LanguageModeAnswer::Unstated => unanswered(format!(
            "it exited successfully but predefined no {STDC_VERSION_MACRO}, so either it is not a C \
             driver or its answer was longer than this probe retains"
        )),
        LanguageModeAnswer::Unreadable(value) => unanswered(format!(
            "it defined {STDC_VERSION_MACRO} as {value:?}, which is not a revision number this \
             suite can compare against a range"
        )),
    }
}

/// What asking a driver which language it compiles by default produced.
///
/// Six outcomes rather than an `Option`, on the same reasoning [`MachineAnswer`] records: a driver that
/// could not be launched, one that hung, one that rejected the arguments, one that answered nothing and
/// one that answered something unreadable are five different situations, and collapsing them into a
/// single absence is exactly how a check comes to fail open.
enum LanguageModeAnswer {
    /// The driver could not be launched at all.
    NotLaunched,
    /// The driver outlived the probe deadline and was terminated, or became unobservable.
    NoCompletion(String),
    /// The driver ran to completion and reported failure, so it declined the question.
    Declined(String),
    /// The driver reported success but predefined no revision macro.
    Unstated,
    /// The driver stated a revision this suite cannot read as a number. Carries the value as printed.
    Unreadable(String),
    /// The driver stated its default revision, and whether its default mode is strictly conforming.
    Stated { version: u64, strict: bool },
}

/// Ask a compiler driver which language it compiles by default.
///
/// Success is required on top of a readable answer, and the two are separate conditions for the same
/// reason [`probe_dumpmachine`] separates them: a program that exits zero having printed nothing is a
/// successful non-answer, and only a check that reads both the status and the text can tell it from a
/// driver that genuinely answered.
fn probe_language_mode(driver: &Path) -> LanguageModeAnswer {
    let Some(capture) = run_bounded_probe(Command::new(driver).args(LANGUAGE_MODE_ARGUMENTS))
    else {
        return LanguageModeAnswer::NotLaunched;
    };
    if capture.timed_out || capture.status.is_none() {
        return LanguageModeAnswer::NoCompletion(capture.completion_summary());
    }
    if !capture.exited_successfully() {
        return LanguageModeAnswer::Declined(capture.completion_summary());
    }
    let text = String::from_utf8_lossy(&capture.stdout);
    let strict = predefined_macro(&text, STRICT_ANSI_MACRO).is_some();
    let Some(raw) = predefined_macro(&text, STDC_VERSION_MACRO) else {
        return LanguageModeAnswer::Unstated;
    };
    match revision_number(&raw) {
        Some(version) => LanguageModeAnswer::Stated { version, strict },
        None => LanguageModeAnswer::Unreadable(raw),
    }
}

/// The value a `-dM` dump gives one macro, or `None` when the dump does not define it.
///
/// Matched on the whole `#define <name> ` prefix rather than by searching for the name anywhere, so a
/// macro whose *value* mentions another macro's name cannot be read as a definition of it — a real
/// hazard here, since a driver's dump contains hundreds of definitions and several quote others.
fn predefined_macro(dump: &str, name: &str) -> Option<String> {
    let prefix = format!("#define {name} ");
    dump.lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|value| String::from(value.trim()))
}

/// A C revision macro's value as a number, or `None` when it is not one.
///
/// The value is a `long` constant, so it carries an `L` suffix — `201710L`. The suffix is optional in
/// what is accepted, and anything else is refused rather than salvaged: reading `2017abc` as `2017`
/// would turn an unreadable answer into a confidently wrong one, which is the failure mode this whole
/// check exists to remove.
fn revision_number(raw: &str) -> Option<u64> {
    let digits = raw
        .strip_suffix('L')
        .or_else(|| raw.strip_suffix('l'))
        .unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Report why a tool's location cannot be trusted, examining every path that reaches its bytes.
///
/// Three chains are checked, because each one is independently sufficient to substitute the tool and
/// none of them implies the others:
///
/// * The **name that was resolved** is what the suite will spawn. If any directory on its chain is
///   world-writable, the entry itself can be replaced or repointed at another program.
/// * The **file the name leads to** is what those bytes actually are. A symbolic link in a perfectly
///   safe directory can point into a world-writable one, and checking only the name would accept it
///   — a link's own permissions are always `rwxrwxrwx` and say nothing about its target.
/// * The **implementation behind a launcher** is what ultimately does the work. This is the same
///   distinction that makes a launcher-only identity comparison unsound: a wrapper script in
///   `/usr/local/bin` is unwritable and canonicalizes to itself, so both chains above are clean,
///   while the driver it `exec`s may sit anywhere at all. Vetting the launcher and not the driver
///   would trust a script precisely because *it* cannot be tampered with, while the compiler it
///   hands every compilation to could be replaced freely.
///
/// Checking any subset would leave a complete substitution path open, so the tool is refused if any
/// chain is untrustworthy. The third check costs nothing when the tool is not a wrapper, because the
/// implementation is then the file itself and the comparison short-circuits.
fn untrusted_location(path: &Path, identity: Option<&ToolIdentity>) -> Option<String> {
    if let Some(reason) = untrusted_reason(path) {
        return Some(reason);
    }
    let identity = identity?;
    let canonical = identity.canonical();
    if canonical != path {
        if let Some(reason) = untrusted_reason(canonical) {
            return Some(format!(
                "it resolves through a symbolic link to {}, and {reason}",
                shown_path(canonical)
            ));
        }
    }
    let implementation = identity.implementation_path();
    if implementation == canonical {
        return None;
    }
    let reason = untrusted_reason(implementation)?;
    Some(format!(
        "it is a launcher that runs {}, and {reason}",
        shown_path(implementation)
    ))
}

/// What asking a driver which target it builds for produced.
///
/// Five outcomes rather than an `Option`, because the four ways of *not* answering are not
/// interchangeable and each earns a different sentence in the refusal. Collapsing them into one
/// `None` is precisely what let this check fail open: a program that could not be launched, one that
/// hung, one that rejected the flag and one that printed nothing all arrived as the same absence,
/// and that absence was then read as permission to proceed.
enum MachineAnswer {
    /// The driver could not be launched at all.
    NotLaunched,
    /// The driver outlived the probe deadline and was terminated, or became unobservable.
    ///
    /// Carries the rendered completion so the diagnostic can say which of the two happened.
    NoCompletion(String),
    /// The driver ran to completion and reported failure, so it declined the question.
    Declined(String),
    /// The driver reported success but printed nothing that could be read as a target.
    Silent,
    /// The driver named this target.
    Named(String),
}

/// Ask a compiler driver which target it builds for.
///
/// The flag is a reference-compiler probe flag and appears in no differential invocation, so the
/// discipline that only flags both compilers honour identically may be passed to both compilers is
/// untouched — the same justification that already applies to the static-link input probe.
///
/// Success is required on top of a non-empty answer, and the two are separate conditions. Measured
/// here: `/bin/true -dumpmachine` exits zero and prints nothing, while `/bin/echo -dumpmachine`
/// exits zero and prints the flag back. The first is a successful non-answer and the second is a
/// confident wrong one, and only a check that looks at both the status and the text can tell either
/// of them from a real driver reporting `x86_64-linux-gnu`.
fn probe_dumpmachine(driver: &Path) -> MachineAnswer {
    let Some(capture) = run_bounded_probe(Command::new(driver).arg(DUMP_MACHINE_FLAG)) else {
        return MachineAnswer::NotLaunched;
    };
    if capture.timed_out || capture.status.is_none() {
        return MachineAnswer::NoCompletion(capture.completion_summary());
    }
    if !capture.exited_successfully() {
        return MachineAnswer::Declined(capture.completion_summary());
    }
    match first_non_empty_line(&capture.stdout) {
        Some(machine) => MachineAnswer::Named(machine),
        None => MachineAnswer::Silent,
    }
}

/// Why a driver's reported machine does not describe `target`, or `None` when it does.
///
/// # The architecture alone is not the contract
///
/// Comparing only the leading component accepts an oracle from an entirely different platform. Every
/// one of `x86_64-w64-mingw32`, `riscv64-unknown-elf`, `aarch64-linux-musl` and `x86_64-linux-gnux32`
/// leads with an architecture this suite recognises, and not one of them can serve as a reference
/// arm: the first targets Windows, the second is bare metal with no operating system underneath it,
/// the third is a different C library whose formatted output need not agree byte for byte, and the
/// fourth is a distinct ABI with different type widths. An arm built by any of them would not be a
/// weaker authority but a false one, and a false authority is worse than a missing one, because a
/// missing arm is reported and a false one silently produces verdicts.
///
/// What is compared is therefore the whole family: the architecture, the operating system, and the C
/// library environment.
///
/// # Why the triple cannot be read positionally
///
/// A GNU triple has two to four components and the vendor is optional, so position does not identify
/// meaning. Measured on this machine: the four reference drivers report `x86_64-linux-gnu`,
/// `i686-linux-gnu`, `aarch64-linux-gnu` and `riscv64-linux-gnu`, where the *second* component is the
/// operating system; the alternate reference compiler reports `x86_64-pc-linux-gnu`, where the second
/// component is the vendor and the *third* is the operating system. Reading by position would have to
/// be wrong about one of them. The components are consequently searched for the operating system
/// rather than indexed, which makes vendor variation free — exactly the tolerance a triple's optional
/// vendor field is supposed to buy.
///
/// # Why an absent environment component is accepted
///
/// Requiring the word `gnu` to be present would refuse a correctly configured machine. Red Hat and
/// SUSE build their native driver to report `x86_64-redhat-linux` and `x86_64-suse-linux`: three
/// components, vendor in the middle, and no environment component at all — while being ordinary
/// glibc Linux systems. The rule is therefore that an environment component, *when stated*, must be
/// exactly `gnu`.
///
/// This is a deliberate trade of a little assurance for not refusing legitimate machines, and the
/// assurance given up should be named rather than glossed over. An unqualified `<arch>-linux` triple
/// does **not** prove GNU/glibc; it proves only that the driver did not say otherwise. The
/// incompatible families conventionally do name themselves — musl, android, uclibc and the x32 ABI
/// all appear in that component, and MinGW and bare-metal ELF are already excluded by the
/// operating-system check above — so in practice the omission is a packaging habit rather than a
/// disguise. But a driver that omitted the component *while* targeting a different C library would
/// be accepted here, and the consequence is a same-target reference arm whose formatted output need
/// not agree byte for byte with glibc's. Such a driver is therefore accepted with **reduced
/// assurance**, not with the certainty an explicit `gnu` carries: the rule rejects everything the
/// stricter rule rejects, refuses one thing fewer that is legitimate, and records the driver's exact
/// reported machine in the pre-flight report and every finding's fingerprint so a reader can see
/// which of the two they got.
fn machine_mismatch(machine: &str, target: Target) -> Option<String> {
    // The spelling the four measured drivers produce, checked first so the ordinary case costs one
    // comparison and cannot be affected by any of the tolerance below.
    if machine == target.triple() {
        return None;
    }
    let components: Vec<&str> = machine.split('-').filter(|part| !part.is_empty()).collect();
    let Some(architecture) = components.first() else {
        return Some(String::from("that names no architecture at all"));
    };
    if !architecture_matches(architecture, target) {
        return Some(format!(
            "its architecture {architecture} is not {}",
            target.short_name()
        ));
    }
    let Some(operating_system) = components
        .iter()
        .position(|part| *part == LINUX_OPERATING_SYSTEM)
    else {
        return Some(format!(
            "it does not name the {LINUX_OPERATING_SYSTEM} operating system, so it belongs to a \
             different platform family"
        ));
    };
    match components.get(operating_system + 1) {
        None => None,
        Some(environment) if *environment == GNU_ENVIRONMENT => None,
        Some(environment) => Some(format!(
            "its environment {environment} is not {GNU_ENVIRONMENT}, so it targets a different C \
             library or application binary interface"
        )),
    }
}

/// True when a reported architecture component names `target`'s instruction set.
///
/// The 32-bit x86 family is the one target with several accepted architecture spellings, and a
/// driver may legitimately report any of them for the same instruction set. No other target is given
/// aliases, because none has any in circulation that a GNU driver would report.
fn architecture_matches(architecture: &str, target: Target) -> bool {
    if architecture == target.short_name() {
        return true;
    }
    target == Target::I686 && I686_ARCHITECTURE_ALIASES.contains(&architecture)
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
///
/// Both fields are private for the same reason the enclosing status keeps its own: the pair is a
/// probe *result*, and a caller that could write `resolved` back would turn "this target's C runtime
/// is incomplete" into "it is fine", which is the difference between a link failure attributed to
/// the environment and the same failure attributed to the compiler under test.
///
/// Read through the two accessors below, and only for reporting. [`CRuntimeStatus`] answers the
/// questions that are about the set — which inputs are missing, and how that reads as a one-line
/// verdict — while the capability report's static-link block walks the entries themselves to say
/// which input was found *where*. That per-entry detail is the form a maintainer whose link step
/// failed can act on, and it is the same evidence the compile stage uses to decide whether a link
/// failure belongs to the environment or to the compiler under test, so stating it in the
/// pre-flight report is what keeps the two from ever disagreeing unnoticed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CRuntimeArtifact {
    name: &'static str,
    /// Where the target's reference driver said it would find the file, when it named an
    /// existing absolute path.
    resolved: Option<PathBuf>,
}

impl CRuntimeArtifact {
    /// The static-link input this entry is about, for example `crt1.o`.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Where the driver said it would find the file, when it named an existing absolute path.
    pub fn resolved(&self) -> Option<&Path> {
        self.resolved.as_deref()
    }
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
///
/// Its fields are private because the whole value is a measurement: rewriting the driver or the
/// artifact list would let a report claim a runtime was probed with a driver that never ran, and the
/// only purpose this record serves is to let a maintainer trust that claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CRuntimeStatus {
    target: Target,
    driver: Option<PathBuf>,
    artifacts: Vec<CRuntimeArtifact>,
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

    /// The reference driver that was asked, when one was available to ask.
    pub fn driver(&self) -> Option<&Path> {
        self.driver.as_deref()
    }

    /// Every required static-link input the driver could not locate, in catalogue order.
    ///
    /// Empty means every input was located, which — together with a driver having been available
    /// to ask — is what "appears usable" means. The two facts are reported rather than folded into
    /// a single boolean, because a runtime nobody could be asked about and one that is genuinely
    /// incomplete call for opposite responses: the first is not a failure at all, while the second
    /// predicts a link failure at environment scope. [`CRuntimeStatus::describe`] states which of
    /// the three cases holds.
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
    /// Which target this runtime serves.
    pub fn target(&self) -> Target {
        self.target
    }

    /// One entry per required static-link input, in catalogue order.
    pub fn artifacts(&self) -> &[CRuntimeArtifact] {
        &self.artifacts
    }

    pub fn appears_usable(&self) -> bool {
        self.driver.is_some() && self.artifacts.iter().all(|entry| entry.resolved.is_some())
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
///
/// # Why every field is private
///
/// This is the record every downstream question is answered from, and the questions are not
/// independent of each other. [`discover`] establishes three cross-field properties before it
/// returns, and each one is a property of the *combination* rather than of any single field:
///
/// * **The oracles are mutually independent.** [`require_distinct_oracles`] has proved that the
///   compiler under test and every reference arm run different implementations, and that no two
///   reference arms are the same driver. Writing one tool record could make two arms share a
///   compiler, at which point oracle (a) compares a compiler with itself and reports agreement by
///   construction — a green run over a comparison that was never made.
/// * **Every retained tool passed vetting.** A record whose path is present has been through
///   [`vet_resolved_tool`]: its identity was captured, its path is expressible as text, and — under
///   strict mode — its location is not world-writable and it is not an unattested launcher.
/// * **One configuration governs the whole run.** [`RunConfig`] is held here rather than re-read on
///   demand precisely so that the matrix, the per-cell budget, the strictness policy and the active
///   filter cannot differ between two concurrently executing area tests. A writable field would
///   reintroduce exactly the divergence holding it here removes.
///
/// A struct literal or a field assignment could establish none of those, so both are made
/// impossible: the accessors below hand out borrows and copies, and [`discover`] is the only way to
/// obtain the value at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// The compiler under test. Always present: discovery fails hard when it is not.
    bcc: ToolRecord,
    /// Reference compiler for the host architecture, and the driver both undefined-behaviour
    /// audit gates use.
    ref_cc_native: ToolRecord,
    /// Reference cross driver for the i686 arm of oracle (a).
    ref_cc_i686: ToolRecord,
    /// Reference cross driver for the AArch64 arm of oracle (a).
    ref_cc_aarch64: ToolRecord,
    /// Reference cross driver for the RISC-V 64 arm of oracle (a).
    ref_cc_riscv64: ToolRecord,
    /// Emulated-execution runner for the i686 target. Its name carries the suffix `i386`,
    /// deliberately not the target slug — see [`DEFAULT_QEMU_I386`].
    runner_i686: ToolRecord,
    runner_aarch64: ToolRecord,
    runner_riscv64: ToolRecord,
    /// External per-cell timeout utility. Optional: execution falls back to a watchdog thread.
    timeout_tool: ToolRecord,
    /// Whether that utility is wrapped around a launch as an outer net, and why.
    ///
    /// Held beside the record rather than derived at each spawning site, so the qualification probe
    /// runs once per process and every site — compile, execute, the flag probe, the audit gate —
    /// reads one decision. Two sites deciding independently could wrap differently within one run,
    /// which would make a timing comparison between two cells meaningless.
    outer_net: OuterNet,
    reducer: ToolRecord,
    c_runtimes: Vec<CRuntimeStatus>,
    /// Kernel identification, or an explanatory substitute when it could not be obtained.
    kernel: String,
    host_arch: &'static str,
    host_os: &'static str,
    /// The validated configuration snapshot this run is operating under.
    ///
    /// Held here rather than re-read on demand so that every question about policy — the effective
    /// matrix, whether coverage is reduced, whether unexpected success or an unavailable oracle
    /// fails the run, the per-cell budget, the active program filter — is answered from one
    /// validated value. Two consequences follow: the reporting functions cannot fail on a
    /// configuration problem, and two concurrently executing area tests cannot describe different
    /// policies.
    config: RunConfig,
    /// `PATH` entries that were skipped during tool resolution, each with the reason.
    ///
    /// Surfaced in the capability report rather than discarded, because a skipped entry changes
    /// which tool a name resolves to. Silently ignoring a directory the maintainer put on `PATH`
    /// would make an absent tool look like a missing package when it is actually installed
    /// somewhere the suite declined to search — a diagnosis that would send them looking in
    /// entirely the wrong place.
    path_rejections: Vec<String>,
}

impl Capabilities {
    /// The compiler under test. Always available: discovery fails hard when it is not.
    ///
    /// Every one of the 1,296 bcc cells is built by the path this record carries, so it is the
    /// single most consequential value in the whole capability record — and the reason the record's
    /// fields are read-only. A caller that could replace it would redirect the entire matrix at a
    /// different binary while every report still named the one that was vetted.
    pub fn bcc(&self) -> &ToolRecord {
        &self.bcc
    }

    /// The native reference compiler: oracle (a)'s host arm and both audit gates.
    ///
    /// Exposed as the whole record rather than a path because its absence gates oracle (a) on
    /// *every* target — see [`Capabilities::oracle_a_available`] — so a report needs its diagnosis,
    /// not merely whether it is there.
    pub fn ref_cc_native(&self) -> &ToolRecord {
        &self.ref_cc_native
    }

    /// The external per-cell timeout utility, which is optional.
    ///
    /// Its absence changes how execution enforces the budget — a watchdog thread instead of the
    /// utility — and changes no verdict, which is why it degrades silently where a missing oracle
    /// does not.
    ///
    /// This is the record, for reporting and fingerprinting. It is deliberately **not** what a
    /// spawning site reads: whether the utility is actually wrapped around a launch is
    /// [`Capabilities::outer_net`], because presence and engagement are different questions and a
    /// site that answered the first would reinstate the per-invocation cost qualification exists to
    /// avoid.
    pub fn timeout_tool(&self) -> &ToolRecord {
        &self.timeout_tool
    }

    /// The utility to wrap a bounded launch with, or `None` when nothing is wrapped.
    ///
    /// The single authority the four spawning sites consult — `compile.rs`, `execute.rs`, the flag
    /// probe and the audit gate — so the qualification probe runs once per process and no site can
    /// reach past the decision to the raw discovery result. See [`OuterNet`] for what the decision
    /// does and does not affect; in particular, declining it changes no verdict, because
    /// `execute.rs`'s watchdog is authoritative either way. The decision itself, with its evidence,
    /// is stated by the pre-flight capability report and by the environment fingerprint.
    pub fn outer_net_tool(&self) -> Option<&Path> {
        self.outer_net.path()
    }

    /// The test-case reducer used to minimize a finding, which is optional.
    ///
    /// Its absence makes minimization manual. A finding remains complete without it, because the
    /// reproducer, the exact commands, the per-cell outputs and the environment fingerprint do not
    /// depend on it.
    pub fn reducer(&self) -> &ToolRecord {
        &self.reducer
    }

    /// Kernel identification, or an explanatory substitute when it could not be obtained.
    pub fn kernel(&self) -> &str {
        &self.kernel
    }

    /// The host architecture as the standard library reports it.
    pub fn host_arch(&self) -> &'static str {
        self.host_arch
    }

    /// The host operating system as the standard library reports it.
    pub fn host_os(&self) -> &'static str {
        self.host_os
    }

    /// The validated configuration snapshot this run operates under.
    ///
    /// Read from here rather than from the environment a second time, so that two concurrently
    /// executing area tests cannot describe different policies and no reporting function can fail
    /// on a configuration problem.
    pub fn config(&self) -> &RunConfig {
        &self.config
    }

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

    /// The emulator that must be prefixed to an execution for `target`, or `None` when there is
    /// none to prefix.
    ///
    /// # This returns `None` for two very different reasons
    ///
    /// A native target needs no runner, and a non-native target whose runner is missing has none.
    /// Both answer `None`, and nothing in the answer says which. Prefer
    /// [`Capabilities::execution_for`], which is the same question asked in a form that cannot be
    /// misread: it distinguishes the two by returning a value only in the case where execution is
    /// actually possible. This accessor is retained for reporting, where the runner's path is wanted
    /// as text and the distinction does not arise.
    pub fn runner_for(&self, target: Target) -> Option<&Path> {
        self.runner_record_for(target)
            .and_then(|record| record.path())
    }

    /// Every tool this record vetted, in the order the pre-flight report names them.
    ///
    /// The array is the single enumeration of what "a vetted tool" means, so a tool added to the
    /// record is added to identity confirmation by appearing here and nowhere else. Records with no
    /// resolved path are included deliberately: they answer "not this one" in a single comparison,
    /// and omitting them would put a condition in the list that a future edit could get wrong.
    fn vetted_records(&self) -> [&ToolRecord; 10] {
        [
            &self.bcc,
            &self.ref_cc_native,
            &self.ref_cc_i686,
            &self.ref_cc_aarch64,
            &self.ref_cc_riscv64,
            &self.runner_i686,
            &self.runner_aarch64,
            &self.runner_riscv64,
            &self.timeout_tool,
            &self.reducer,
        ]
    }

    /// Confirm `program` is still the file this record vetted, or say how it has changed.
    ///
    /// Returns [`None`] both when `program` names no vetted tool — there is then nothing this record
    /// has anything to say about — and when it names one that is unchanged. The two are deliberately
    /// the same answer, because the question this asks is "has anything I vouched for been swapped",
    /// and a path I never vouched for cannot have been.
    ///
    /// # Why this lives on the record rather than beside the spawn
    ///
    /// The vetted identity and the vetting decisions that rest on it — the location check, the
    /// wrapper attestation, the declared target — are all held here, so this is the only place that
    /// can answer without a second, necessarily weaker, notion of what was checked. See
    /// [`ToolIdentity::changed_since_vetting`] for what "changed" is decided on and why the window
    /// it closes cannot be closed completely with the standard library alone.
    ///
    /// A path is compared exactly as discovery recorded it. That is sound rather than lax: every
    /// argument vector the suite launches takes its tool path *from* this record, so a spelling that
    /// differed would mean a caller had invented a path of its own, which is a defect this
    /// comparison should not paper over by canonicalizing until it matches.
    pub fn confirm_tool_unchanged(&self, program: &Path) -> Option<String> {
        self.vetted_records().into_iter().find_map(|record| {
            if record.path() != Some(program) {
                return None;
            }
            record
                .identity()?
                .changed_since_vetting(program)
                .map(|change| format!("{}: {change}", record.role()))
        })
    }

    /// How a binary built for `target` is to be executed, or `None` when it cannot be.
    ///
    /// # Why this exists rather than a bare runner path
    ///
    /// Execution has two shapes and three situations, and a bare `Option<&Path>` can only express
    /// two of the three. Native execution runs the binary directly; a foreign target runs it under
    /// its emulator; and a foreign target with no emulator cannot be run at all. Collapsing the
    /// first and third into one `None` puts the most dangerous possible default on the most likely
    /// mistake: a caller that reads "no runner" as "run it directly" would launch a foreign binary
    /// on the host, and the operating system's refusal to execute it would arrive as a non-zero exit
    /// status — indistinguishable, to a comparison that only reads status and stdout, from the
    /// compiler under test having produced a program that fails. A missing emulator would then be
    /// reported as a compiler defect.
    ///
    /// [`Execution`] makes that mistake unrepresentable in the type rather than forbidden by a
    /// comment: `Execution::Native` is refused for a non-native target by
    /// [`Execution::runner`]'s own consumer in the record parser, and this function never
    /// manufactures it for one. A caller that has an `Execution` in hand has already been told
    /// execution is possible; a caller that has `None` has been told it is not, and has no value it
    /// could accidentally use.
    pub fn execution_for(&self, target: Target) -> Option<Execution> {
        if target.is_native() {
            return Some(Execution::Native);
        }
        self.runner_for(target)
            .map(|runner| Execution::Emulated(runner.to_path_buf()))
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
    /// The rendering is a pure function of the record, its configuration snapshot and the
    /// once-per-process credential snapshot [`redact_secrets`] holds: no clock, no process
    /// identifier and no map iteration, so the report is unaffected by how many threads the test
    /// harness uses and two runs on the same machine differ only where the *record* differs. Under
    /// strict mode that is not nowhere: an attested runner's provenance names the run-specific exit
    /// status it was required to produce, so the runner lines move between runs even on an unchanged
    /// machine. That is evidence rather than noise — see [`Capabilities::render_fingerprint`].
    ///
    /// [`redact_secrets`] is applied to the whole of it on the way out, because this text is printed
    /// to the test runner's output and quoted into two assertion messages — a continuous-integration
    /// log, in other words — and it carries discovered tool paths and refusal reasons, which the
    /// per-banner redaction performed at capture time does not cover. Applying it once here rather
    /// than at each of the three consumers is what makes the property structural.
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
        // Stated whichever way the decision went, because a reader comparing two runs' timings has
        // to know which of them was wrapping, and because an engaged net is a cost worth seeing.
        lines.push(format!("    outer net {}", self.outer_net.summary()));
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

        if !self.path_rejections().is_empty() {
            lines.push(String::from(
                "Search-path entries skipped during tool resolution",
            ));
            for rejection in self.path_rejections() {
                lines.push(format!("  {rejection}"));
            }
            lines.push(String::from(
                "  consequence: a tool installed only in a skipped directory is reported as \
                 absent. If a tool below is unexpectedly missing, check whether it lives in one of \
                 these",
            ));
            lines.push(String::new());
        }

        // Stated unconditionally, unlike the skipped entries above, because the value a child
        // receives is a fact about every invocation in the run rather than an exception worth
        // mentioning only when it occurs. It is also the one line that lets a maintainer see that the
        // filter reaches the *whole* process tree: a compiler driver executes its own stages, and if
        // this line showed the inherited path, a stage could be substituted from a directory the list
        // above says was refused.
        lines.push(String::from(
            "Search path given to every child process — computed, never inherited",
        ));
        lines.push(format!(
            "  PATH={}",
            sanitize_text_for_report(&redact_secrets(&describe_child_search_path()))
        ));
        lines.push(String::from(
            "  consequence: a compiler driver, an emulator and the timeout utility resolve their own \
             helpers only within these directories, so a substituted `cc1`, `as` or `ld` cannot be \
             reached from an entry that tool resolution refused",
        ));
        lines.push(String::new());

        // The static-link runtimes, in detail. `render_target_block` states each target's one-line
        // verdict; this states *which* input was found *where*, which is the only form that helps a
        // maintainer whose link step failed — and it is the same evidence the compile stage uses to
        // decide whether a link failure belongs to the environment or to the compiler, so having it
        // in the pre-flight report means the two can never disagree without it being visible.
        lines.push(String::from(
            "Static-link runtimes — asked of each target's own driver",
        ));
        for runtime in self.c_runtimes() {
            lines.push(format!(
                "  {}: {}",
                runtime.target(),
                if runtime.appears_usable() {
                    "complete"
                } else {
                    "not established"
                }
            ));
            for artifact in runtime.artifacts() {
                lines.push(match artifact.resolved() {
                    Some(path) => format!("    {} -> {}", artifact.name(), shown_path(path)),
                    None => format!(
                        "    {} not located, so a static link for this target fails at environment \
                         scope rather than indicating a compiler defect",
                        artifact.name()
                    ),
                });
            }
        }
        lines.push(String::new());

        lines.push(String::from("Tool provenance — why each file was selected"));
        for record in [
            &self.bcc,
            &self.ref_cc_native,
            &self.ref_cc_i686,
            &self.ref_cc_aarch64,
            &self.ref_cc_riscv64,
            &self.runner_i686,
            &self.runner_aarch64,
            &self.runner_riscv64,
            &self.timeout_tool,
            &self.reducer,
        ] {
            for line in record.provenance_detail() {
                lines.push(format!("  {line}"));
            }
        }
        lines.push(String::new());

        lines.push(String::from("Host"));
        lines.push(format!(
            "  architecture {}, operating system {}",
            self.host_arch, self.host_os
        ));
        lines.push(format!("  kernel {}", self.kernel));
        // Every parenthetical in this report describes the variable's CURRENT state, never an action
        // the reader has already taken. Advising someone to set what they have already set is not
        // merely redundant: it reads as though the setting had not taken effect, which is the one
        // thing a configuration report must never suggest when it has.
        lines.push(format!(
            "  per-cell execution budget {} s ({})",
            self.config.timeout_secs(),
            if self.config.timeout_secs_from_variable() {
                format!("{VAR_TIMEOUT_SECS} is set, and this is the value it names")
            } else {
                format!("the default; override with {VAR_TIMEOUT_SECS}")
            }
        ));
        // Whether the second line of defence around this run's process groups is in force, stated
        // rather than assumed. Each spawned child is put in a group of its own so that terminating an
        // invocation terminates everything it started — and so that an interrupt aimed at the test
        // runner cannot kill a compiler mid-write. The cost of that isolation is that a signal ending
        // THIS process no longer ends the children, and no `std` facility can catch a signal to make up
        // for it. A guard value covers every ending this process unwinds through; the external
        // supervisor covers the ones it does not, and it needs a trusted shell to exist. A machine
        // without one is told so here, because a reader of a green run is entitled to know which of the
        // two guarantees was in force.
        lines.push(match super::process_group_supervision() {
            Some(detail) => format!("  process groups: {detail}"),
            None => String::from(
                "  process groups: NO EXTERNAL SUPERVISOR — a trusted POSIX shell and a trusted \
                 `kill` utility are both required to host one, and at least one was not found, so a \
                 group is swept only by this run's own cleanup and by the guard that fires when a \
                 cell unwinds. Should this process be killed outright, a compiler or emulator it had \
                 launched could outlive it. Installing both on the trusted search path restores the \
                 guarantee",
            ),
        });
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
                "  policy: an unavailable arm {} the run ({})",
                if self.config.unavailable_fails_run() {
                    "FAILS"
                } else {
                    "is reported but does not fail"
                },
                if self.config.unavailable_fails_run() {
                    format!("{VAR_STRICT} is set, which is what escalates it")
                } else {
                    format!("set {VAR_STRICT} to escalate it")
                }
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
        // Each of these three answers is decided by exactly one variable, so the state of that
        // variable is already implied by the answer — which is why the parenthetical states it
        // rather than advising it. `no` here means the variable IS set; offering it as the next step
        // would leave a reader looking for a setting they had already made.
        lines.push(format!(
            "  unexpected success (XPASS) fails the run: {} ({})",
            yes_or_no(self.config.xpass_fails_run()),
            if self.config.allow_xpass() {
                format!(
                    "{VAR_ALLOW_XPASS} is set, which is what downgrades it to a warning; it is \
                     still listed separately in the summary"
                )
            } else {
                format!("set {VAR_ALLOW_XPASS} to downgrade it to a warning")
            }
        ));
        lines.push(format!(
            "  an unavailable oracle fails the run: {} ({}; {} cannot lower it, because a job that \
             demanded strictness while excusing the arms it could not attempt would report a green \
             run over a matrix it never executed)",
            yes_or_no(self.config.unavailable_fails_run()),
            if self.config.unavailable_fails_run() {
                format!("{VAR_STRICT} is set, which is what escalates it")
            } else {
                format!("set {VAR_STRICT} to escalate it")
            },
            VAR_ALLOW_MISSING_ORACLES
        ));
        lines.push(format!(
            "  cell workspaces retained after success: {} ({})",
            yes_or_no(self.config.keep_work()),
            if self.config.keep_work() {
                format!("{VAR_KEEP_WORK} is set, which is what retains them")
            } else {
                format!("set {VAR_KEEP_WORK} to retain them")
            }
        ));
        lines.push(String::from(
            "  a missing oracle is never a silent pass: it is recorded as UNAVAILABLE and listed \
             in the run summary",
        ));

        redact_secrets(&lines.join("\n"))
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

    /// A short digest of the configuration this run was performed under.
    ///
    /// # What it covers, and why it covers that much
    ///
    /// Everything that could make two runs' numbers incomparable: the effective matrix and whether
    /// it was reduced, the resolved identity of every tool — including the file each name ultimately
    /// runs, so a package rebuilt at the same version reads as a different configuration — the
    /// per-cell budget, and the behavioural settings that decide which verdicts fail a run and which
    /// programs are attempted at all. Any difference in what was asked for or what answered changes
    /// it, which is what makes a reduced run's totals impossible to mistake for a full run's.
    ///
    /// # It is a function of the run, not a constant across runs
    ///
    /// The digest is computed deterministically from its inputs, but one of those inputs is not
    /// stable between two runs on one unchanged machine, and a reader comparing two summaries needs
    /// to know that. Tool identity includes how a tool was established, and under strict mode a
    /// runner is established by attestation: it must print a token derived from this run and exit
    /// with a status derived from it, so that a stand-in ignoring its arguments cannot satisfy the
    /// check by luck. That required status is named in the runner's provenance, so
    /// [`render_fingerprint`](Capabilities::render_fingerprint) — and therefore this digest — differs
    /// from run to run whenever any runner was attested.
    ///
    /// The variation is the evidence, not a defect: an attestation whose expected answer never
    /// changed could be replayed, and a silently absent emulator would then be indistinguishable
    /// from a working one. Requirement-wise this is the emulator half of the environment fingerprint
    /// a finding must carry, which is what lets a divergence be attributed to toolchain drift.
    ///
    /// # Why a digest rather than the text it digests
    ///
    /// [`render_fingerprint`](Capabilities::render_fingerprint) is many lines long, and this value is
    /// carried on a single line by the run manifest and by every report. Its purpose there is
    /// comparison, not description: a reader who needs the detail has the fingerprint section of the
    /// same report a few lines away. It carries no timestamp and no process identifier, which is
    /// what makes a difference between two digests attributable to the run's configuration and
    /// attested tools rather than to when or by which process it ran. The artifact whose bytes are
    /// promised not to move is an area report's Markdown, which renders neither this digest nor
    /// [`RunGeneration::token`](super::RunGeneration::token) — the token being confined more tightly
    /// still, to one comment field of one artifact and no rendered text at all.
    pub fn configuration_fingerprint(&self) -> String {
        let config = &self.config;
        let behaviour = format!(
            "quick={} only={} strict={} allow_xpass={} allow_missing_oracles={} keep_work={}",
            config.quick_mode(),
            match config.only() {
                Some(filter) => format!("{}/{}", filter.area(), filter.program()),
                None => String::from("none"),
            },
            config.strict(),
            config.allow_xpass(),
            config.missing_oracles_acknowledged(),
            config.keep_work(),
        );
        digest_hex(&[&self.render_fingerprint(), &behaviour])
    }

    /// The environment fingerprint written into every finding artifact.
    ///
    /// Records the compiler under test, every reference driver, every runner, the auxiliary tools
    /// and the kernel, so that a divergence can later be attributed to toolchain drift rather than
    /// to the compiler. That distinction is precisely what the repository's own risk register asks
    /// for when it notes that emulator version skew is a hazard for cross-architecture testing.
    ///
    /// Free of any timestamp and of any process identifier, so a difference between two fingerprints
    /// is always a difference in what was discovered rather than in when it was discovered.
    ///
    /// It is not, however, constant across runs. A runner attested under strict mode records the
    /// run-specific exit status it was required to produce, so its line differs between two runs on
    /// an unchanged machine. Two fingerprints from the same machine therefore compare equal in every
    /// tool's name, version and resolved file, and differ in the attestation evidence — which is the
    /// part that proves the runner actually executed for *this* run rather than at some point in the
    /// past. [`configuration_fingerprint`](Capabilities::configuration_fingerprint) records what that
    /// means for the digests computed over this text.
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
        // Part of the fingerprint because it is part of how a cell was bounded: a finding whose
        // divergence class is `timeout` was reproduced under one arrangement or the other, and a
        // maintainer comparing this machine with theirs needs to know which.
        lines.push(format!("outer-net: {}", self.outer_net.summary()));
        lines.push(fingerprint_line("reducer", &self.reducer));
        lines.push(format!("host-arch: {}", self.host_arch));
        lines.push(format!("host-os: {}", self.host_os));
        lines.push(format!("kernel: {}", self.kernel));
        // The search path the *children* received, which is not the search path this process has.
        // A divergence that turns out to have been produced by a substituted compiler stage is only
        // attributable if the fingerprint records which directories that stage could have come from,
        // and a maintainer comparing two machines needs to see that this one skipped an entry the
        // other kept. Redacted before sanitization for the same reason every other recorded value is:
        // a directory name is not a likely place for a credential, but the cost of assuming so is a
        // credential committed inside a finding.
        lines.push(format!(
            "child-search-path: {}",
            sanitize_text_for_report(&redact_secrets(&describe_child_search_path()))
        ));
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
    /// `PATH` entries skipped during resolution, each with the reason it was skipped.
    pub fn path_rejections(&self) -> &[String] {
        &self.path_rejections
    }

    /// One C-runtime probe result per target, in [`Target::ALL`] order.
    pub fn c_runtimes(&self) -> &[CRuntimeStatus] {
        &self.c_runtimes
    }
}

/// One `key: value` fingerprint line, stating the version and the path or a fixed absence marker.
fn fingerprint_line(key: &str, record: &ToolRecord) -> String {
    let Some(path) = record.path() else {
        // A refused tool and an absent one must not read alike. A divergence explained by a tool
        // the suite declined to trust is a different story from one explained by a package nobody
        // installed, and the fingerprint is often the only account of the run a reader has.
        return match record.rejection() {
            Some(reason) => format!("{key}: refused — {reason}"),
            None => format!("{key}: absent"),
        };
    };
    let mut line = format!(
        "{key}: {} ({})",
        record.version_or_unknown(),
        shown_path(path)
    );
    if let Some(identity) = record.identity() {
        // The file identity, not just the name. A package rebuilt at the same version reports the
        // same banner from a different file, and that is precisely the toolchain drift a
        // fingerprint exists to expose — a name alone would show two runs as identical.
        line.push_str(&format!(" [{}]", identity.describe()));
    }
    if let Some(provenance) = record.provenance() {
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
    CAPABILITY_CACHE.get_or_init(discover_once).clone()
}

/// The one discovery this process performs, kept so a spawn path can borrow it.
///
/// Hoisted out of [`discover`] rather than nested inside it for a single reason:
/// [`confirm_vetted_tools_unchanged`] has to read the vetted identities from the deepest frame in
/// the harness — the statement before a `spawn` — where no capability record is in scope and where
/// threading one down would mean an `Option` parameter through four modules whose `None` case would
/// be a silent hole in the check. Borrowing the record every caller already shares closes that hole
/// without changing a single signature.
static CAPABILITY_CACHE: OnceLock<HarnessResult<Capabilities>> = OnceLock::new();

/// Confirm that every vetted tool named anywhere in `argv` is still the file discovery inspected.
///
/// Returns [`None`] when nothing in `argv` names a vetted tool, or when each one that does is
/// unchanged. Returns the first change found otherwise, already phrased for a diagnostic.
///
/// # Why the whole vector rather than its first element
///
/// The program actually executed is not always the tool under test. When the external `timeout`
/// utility is present the launch becomes `<timeout> <seconds> <program> <arguments...>`, so the
/// compiler or emulator whose identity matters has moved to the middle of the vector while the
/// utility now occupies the front. Checking only the front would silently stop checking the
/// compiler on precisely the machines that have the utility installed. Scanning every element
/// checks both, in one pass, and needs no caller to say which shape it built.
///
/// Elements that name no vetted tool cost a handful of string comparisons and no system call, so a
/// flag, a source path or an output path is skipped without touching the file system. A path is
/// matched exactly as discovery recorded it, which is sound because every one of these vectors is
/// assembled *from* the capability record: nothing re-spells a tool path between resolution and
/// launch.
///
/// # Before discovery has completed this is honestly silent
///
/// [`CAPABILITY_CACHE`] is read with [`OnceLock::get`] and never initialized here. Discovery's own
/// banner probes spawn the very tools it is in the middle of vetting, and a tool being vetted right
/// now has no earlier vetting to be compared against; answering [`None`] states that, where
/// triggering discovery from inside a spawn path would be a re-entrant call on a lock that is
/// already held.
pub fn confirm_vetted_tools_unchanged(argv: &[String]) -> Option<String> {
    argv.iter()
        .find_map(|argument| confirm_vetted_tool_unchanged(Path::new(argument)))
}

/// The same question asked about one path, for a caller that spawns a tool rather than a vector.
///
/// Both vetting tables are consulted, in the order a diagnostic should prefer: the capability record
/// first, because a change there invalidates a comparison, and the process-group signalling utility
/// second, because a change there invalidates a *sweep*. A path in neither table is not a vetted tool
/// and produces [`None`].
pub fn confirm_vetted_tool_unchanged(program: &Path) -> Option<String> {
    CAPABILITY_CACHE
        .get()
        .and_then(|cached| cached.as_ref().ok())
        .and_then(|capabilities| capabilities.confirm_tool_unchanged(program))
        .or_else(|| confirm_signalling_tool_unchanged(program))
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
        ToolKind::Compiler,
        // Required to target the host when the host is one of the four supported targets, and
        // unconstrained otherwise: on an unsupported host there is no architecture to demand.
        native_target(),
        strict,
    );
    let ref_cc_i686 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::I686),
        Some(VAR_REF_CC_I686),
        DEFAULT_REF_CC_I686,
        ToolKind::Compiler,
        Some(Target::I686),
        strict,
    );
    let ref_cc_aarch64 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::Aarch64),
        Some(VAR_REF_CC_AARCH64),
        DEFAULT_REF_CC_AARCH64,
        ToolKind::Compiler,
        Some(Target::Aarch64),
        strict,
    );
    let ref_cc_riscv64 = ToolRecord::discover(
        format!("reference cross driver, oracle (a) {} arm", Target::Riscv64),
        Some(VAR_REF_CC_RISCV64),
        DEFAULT_REF_CC_RISCV64,
        ToolKind::Compiler,
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
        ToolKind::Support,
        // An emulator is not a compiler driver: it emits nothing and states no target, so there is
        // no provenance to verify. Its architecture is instead proved by the arm's cells running.
        None,
        strict,
    );
    let runner_aarch64 = ToolRecord::discover(
        format!("execution runner for {}", Target::Aarch64),
        Some(VAR_QEMU_AARCH64),
        DEFAULT_QEMU_AARCH64,
        ToolKind::Support,
        None,
        strict,
    );
    let runner_riscv64 = ToolRecord::discover(
        format!("execution runner for {}", Target::Riscv64),
        Some(VAR_QEMU_RISCV64),
        DEFAULT_QEMU_RISCV64,
        ToolKind::Support,
        None,
        strict,
    );

    let timeout_tool = ToolRecord::discover(
        "per-cell timeout utility (optional)",
        None,
        DEFAULT_TIMEOUT_TOOL,
        ToolKind::Support,
        None,
        strict,
    );
    let reducer = ToolRecord::discover(
        "test-case reducer for finding minimization (optional)",
        None,
        DEFAULT_REDUCER,
        ToolKind::Support,
        None,
        strict,
    );

    // Behaviour, not presence: an outer net that never fires still charges for every child it
    // wraps, so the discovered implementation is measured once here and the decision is carried in
    // the record. See `qualify_outer_net`.
    let outer_net = qualify_outer_net(&timeout_tool);

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
        outer_net,
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
    // Proved by execution, once the drivers and runners are both known. Resolution established that
    // a file with the expected *name* exists; this establishes that it actually executes the
    // architecture whose verdicts will be computed from its output.
    attest_runners(&mut capabilities);
    // Filled after construction because the probe asks each target's own driver, which the record
    // above is what resolves. Built in Target::ALL order so the report is deterministic.
    capabilities.c_runtimes = Target::ALL
        .iter()
        .copied()
        .map(|target| CRuntimeStatus::probe(target, capabilities.ref_cc_for(target)))
        .collect();
    // The run's configuration is fully resolved for the first time here, so this is where it is
    // recorded. Everything the suite writes afterwards can then state which configuration produced
    // it, which is what stops a reduced run's numbers from being read as a full run's.
    RunGeneration::adopt_configuration(&capabilities.configuration_fingerprint())?;
    Ok(capabilities)
}

/// Check behaviourally that each non-native runner executes the architecture it was selected for.
///
/// # Why a name is not enough
///
/// Without this check a runner would earn its place by resolving alone: a file called
/// `qemu-aarch64` found on a trusted search path, with nothing ever asking it to *do* anything. An emulator is the program that runs every binary whose stdout becomes a verdict on
/// that arm, so a file that merely answers to the name is enough to fabricate an entire arm — a
/// three-line script that prints the output a maintainer expects and exits zero would make every
/// cell on that arm pass, and the run would report agreement across four backends while three of
/// them had never executed at all. That is the failure mode that produces a **pass**, which makes it
/// strictly more dangerous than a runner that is simply absent.
///
/// # What is checked, and what the check is and is not
///
/// A program is compiled *for that architecture* and executed *under that runner*, and both its
/// standard output and its exit status must match exactly. Three properties give that check teeth:
///
/// - **The expected output is unique to this run.** It carries a token derived from [`run_id`], which
///   differs between every process, so no output recorded earlier and no value hard-coded in a stub
///   can satisfy it.
/// - **The expected exit status is unique to this run** for the same reason, and is kept inside
///   0–125 so the operating system does not truncate it and so it cannot collide with the statuses a
///   shell reserves for "could not execute".
/// - **The output states the architecture's own type widths**, taken from the target table rather
///   than from the program, so a runner that executed a *host* binary would report the host's widths
///   wherever they differ from the target's. On this host that separates i686, whose pointer and
///   `long` are 4 bytes, from the three LP64 targets; it does **not** separate x86-64 from AArch64 or
///   RISC-V 64, all three of which report 8 and 8.
///
/// This is a **sanity check against accidental substitution, not an attestation against an
/// adversary**, and the distinction is worth stating plainly. The token and the required exit
/// status are both written into the program the runner is handed, so
/// anything able to read that source, or to run the real emulator, can reproduce them: a determined
/// forgery is not what this defends against. What it does catch, reliably, is the realistic failure:
/// a name on the search path that is not an emulator at all, a stale or wrongly targeted runner, a
/// path pointing at the wrong architecture's emulator, and — where the widths differ — a wrapper that
/// quietly ran the binary natively. That failure mode produces a **pass** rather than an error, which
/// is why it is worth a positive check at all; the check is proportionate to it, and the environment
/// this suite runs in is trusted for everything beyond it.
///
/// # Which compiler builds the attestation program
///
/// That target's vetted reference driver, when there is one — it is the independent authority, and
/// using it keeps the attestation independent of the compiler under test. When the cross driver is
/// absent the compiler under test builds it instead, and the diagnosis records that it did. This is
/// deliberate rather than a compromise: oracle (b) is bcc against bcc and needs only a runner, so
/// refusing to attest a runner merely because the *reference* driver is missing would remove
/// cross-backend testing from every environment that ships emulators without cross drivers. If the
/// compiler under test miscompiles this program the attestation fails and the arm is reported
/// unavailable — loudly, in the pre-flight report — which is the correct outcome either way.
///
/// # What failure does
///
/// The runner's record gains a rejection and loses its path, exactly as a vetting refusal does, so
/// every downstream question — which arms run, which cells are unavailable, whether strict mode
/// escalates to a failure — answers correctly without knowing that attestation exists. Nothing is
/// silently skipped: an unattested runner is reported, and under strict mode it fails the run.
fn attest_runners(capabilities: &mut Capabilities) {
    for target in Target::ALL {
        if target.is_native() {
            // A native target is executed directly, so there is no runner to attest.
            continue;
        }
        let Some(runner) = capabilities.runner_for(target).map(Path::to_path_buf) else {
            continue;
        };
        // Re-confirm the runner is still the file vetting inspected, immediately before it is used.
        // Discovery and use are separated by everything that happened in between, and a runner
        // replaced after vetting would have been trusted on the strength of checks that describe a
        // file no longer there. Attestation is the first thing that actually runs it, so this is the
        // earliest point the question can be asked usefully.
        let vetted = capabilities
            .runner_record_for(target)
            .and_then(ToolRecord::identity);
        if let Some(change) = vetted.and_then(|identity| identity.changed_since_vetting(&runner)) {
            let record = match target {
                Target::I686 => &mut capabilities.runner_i686,
                Target::Aarch64 => &mut capabilities.runner_aarch64,
                Target::Riscv64 => &mut capabilities.runner_riscv64,
                Target::X86_64 => continue,
            };
            record.path = None;
            record.rejection = Some(format!(
                "it changed after it was vetted and before it was attested: {change}"
            ));
            continue;
        }
        let builder = match capabilities.ref_cc_for(target) {
            Some(driver) => AttestationBuilder::Reference(driver.to_path_buf()),
            None => match capabilities.bcc.path() {
                Some(compiler) => AttestationBuilder::UnderTest(compiler.to_path_buf()),
                None => continue,
            },
        };
        let outcome = attest_one_runner(target, &runner, &builder, capabilities.config());
        let record = match target {
            Target::I686 => &mut capabilities.runner_i686,
            Target::Aarch64 => &mut capabilities.runner_aarch64,
            Target::Riscv64 => &mut capabilities.runner_riscv64,
            // The baseline is the host and is filtered out above; matching it here keeps the match
            // exhaustive without a wildcard that would silently absorb a future target.
            Target::X86_64 => continue,
        };
        match outcome {
            Ok(note) => {
                record.provenance = Some(match record.provenance.take() {
                    Some(existing) => format!("{existing}; {note}"),
                    None => note,
                })
            }
            Err(refusal) => {
                record.path = None;
                record.rejection = Some(refusal);
            }
        }
    }
}

/// Which compiler builds an attestation program, and therefore how it is invoked.
enum AttestationBuilder {
    /// The target's own reference driver, which selects its architecture by being that driver.
    Reference(PathBuf),
    /// The compiler under test, which selects its architecture with its target flag.
    UnderTest(PathBuf),
}

impl AttestationBuilder {
    /// The executable to spawn.
    fn program(&self) -> &Path {
        match self {
            AttestationBuilder::Reference(path) | AttestationBuilder::UnderTest(path) => path,
        }
    }

    /// How the choice is described in the runner's recorded provenance.
    fn described(&self) -> String {
        match self {
            AttestationBuilder::Reference(path) => {
                format!("built by the reference driver {}", shown_path(path))
            }
            AttestationBuilder::UnderTest(path) => format!(
                "built by the compiler under test {} because no reference driver for this \
                 architecture was available; the attestation therefore says the runner executes \
                 this architecture, not that the compiler under test is correct",
                shown_path(path)
            ),
        }
    }
}

/// Build one attestation program for `target` and run it under `runner`.
///
/// Returns a provenance note on success, or the refusal sentence the runner's record will carry.
fn attest_one_runner(
    target: Target,
    runner: &Path,
    builder: &AttestationBuilder,
    config: &RunConfig,
) -> Result<String, String> {
    let token = attestation_token(target);
    let expected_status = attestation_exit_status(&token);
    let expected_stdout = format!(
        "{token} ptr={} long={}\n",
        target.pointer_width_bytes(),
        target.long_width_bytes()
    );

    let workspace = super::sandbox::attestation_workspace(target.triple(), config)
        .map_err(|error| format!("its attestation workspace could not be prepared: {error}"))?;
    let source = workspace
        .write_text(
            ATTESTATION_SOURCE_NAME,
            &attestation_program(&token, expected_status),
        )
        .map_err(|error| format!("its attestation program could not be written: {error}"))?;
    let artifact = workspace
        .path(ATTESTATION_ARTIFACT_NAME)
        .map_err(|error| format!("its attestation artifact path could not be resolved: {error}"))?;

    let mut build = Command::new(builder.program());
    if let AttestationBuilder::UnderTest(_) = builder {
        build.arg(BCC_TARGET_FLAG).arg(target.triple());
    }
    build
        .arg("-O0")
        .arg("-static")
        .arg(&source)
        .arg("-o")
        .arg(&artifact)
        .current_dir(workspace.root());
    let built = run_bounded_probe(&mut build).ok_or_else(|| {
        format!(
            "its attestation program could not be compiled: {} could not be launched",
            shown_path(builder.program())
        )
    })?;
    if !built.status.is_some_and(|status| status.success()) {
        return Err(format!(
            "its attestation program could not be compiled for {} by {}, so there is no way to \
             prove this runner executes that architecture. The compiler reported: {}",
            target.triple(),
            shown_path(builder.program()),
            first_reported_line(&built.stderr)
        ));
    }

    let mut run = Command::new(runner);
    run.arg(&artifact).current_dir(workspace.root());
    let ran = run_bounded_probe(&mut run)
        .ok_or_else(|| String::from("its attestation program could not be launched under it"))?;
    if ran.timed_out {
        return Err(String::from(
            "it did not finish running a single-statement attestation program within the \
             pre-flight budget, so it cannot be relied on to execute this architecture's cells",
        ));
    }
    let observed_status = ran.status.and_then(|status| status.code());
    if observed_status != Some(expected_status) {
        return Err(format!(
            "running its attestation program yielded exit status {}, where a program built for {} \
             and actually executed must yield {expected_status}. The status is derived from this \
             run's own identity, so no recorded or hard-coded answer can satisfy it: a runner that \
             did not execute the program cannot produce it",
            match observed_status {
                Some(code) => code.to_string(),
                None => String::from("a signal rather than a normal exit"),
            },
            target.triple()
        ));
    }
    if ran.stdout != expected_stdout.as_bytes() {
        return Err(format!(
            "running its attestation program printed {:?}, where a program built for {} and \
             actually executed must print {:?}. The token is derived from this run's own identity, \
             so no recorded output can satisfy it, and the widths are this architecture's own — a \
             runner that quietly executed a host binary instead would report the host's",
            sanitize_text_for_report(&String::from_utf8_lossy(&ran.stdout)),
            target.triple(),
            sanitize_text_for_report(&expected_stdout)
        ));
    }

    let note = format!(
        "attested by execution: printed its run-specific token and this architecture's type widths \
         and exited {expected_status}, {}",
        builder.described()
    );
    if let Some(problem) = workspace.discard_advisory() {
        return Ok(format!("{note} ({problem})"));
    }
    Ok(note)
}

/// Name of the attestation program inside its workspace.
const ATTESTATION_SOURCE_NAME: &str = "attest.c";

/// Name of the attestation artifact inside its workspace.
const ATTESTATION_ARTIFACT_NAME: &str = "attest.out";

/// The token an attestation program must print, unique to this run and this architecture.
///
/// Derived from [`run_id`] and the target triple through the shared digest, so it is stable within a
/// run — the same value is expected and embedded — and different in every other run. Rendered as
/// lower-case hexadecimal with a fixed prefix so it is safe in a C string literal, in a report line
/// and in a diagnostic without any escaping.
fn attestation_token(target: Target) -> String {
    format!(
        "attest-{}",
        stable_digest(&[run_id(), target.triple(), "runner-attestation"])
    )
}

/// The exit status an attestation program must produce, unique to this run.
///
/// Confined to 1–100. The upper bound keeps it below the statuses a shell reserves for "could not
/// execute" and well inside the range the operating system does not truncate; the lower bound keeps
/// it away from zero, which is the status a program that did nothing at all is most likely to
/// produce.
fn attestation_exit_status(token: &str) -> i32 {
    let digest = stable_digest(&[token, "exit"]);
    let value = u32::from_str_radix(&digest[digest.len() - 4..], 16).unwrap_or(0);
    i32::try_from(value % 100).unwrap_or(0) + 1
}

/// The attestation program's source text.
///
/// Deliberately the smallest program that can carry the check. It declares `printf` by hand
/// rather than including a header, exactly as every corpus program does and for the same reason: the
/// compiler under test ships no `stdio.h`, so an include would fail on one side of a comparison for
/// a reason that has nothing to do with the question being asked. It contains no loop, no branch and
/// no arithmetic beyond `sizeof`, so a failure here can only mean the toolchain or the runner, never
/// the program.
fn attestation_program(token: &str, exit_status: i32) -> String {
    format!(
        "/* Generated per run by the differential conformance harness to check that this\n\
         architecture's execution runner really executes this architecture. The token below is\n\
         derived from this run's own identity, so no output recorded by an earlier run and no\n\
         value hard-coded in a stub can satisfy it. A sanity check against a misconfigured or\n\
         substituted runner, not an attestation against an adversary: the token and the required\n\
         exit status are both visible here. Not part of the committed corpus. */\n\
         int printf(const char *, ...);\n\
         \n\
         int main(void)\n\
         {{\n\
         \x20   printf(\"{token} ptr=%d long=%d\\n\", (int)sizeof(void *), (int)sizeof(long));\n\
         \x20   return {exit_status};\n\
         }}\n"
    )
}

/// The first line a tool reported, sanitized, for a one-line diagnostic.
///
/// A compiler's diagnostic can be long and is never compared by this suite; the first line is what
/// identifies the problem, and it is sanitized because it is a tool's own text reaching a report.
fn first_reported_line(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    match text.lines().find(|line| !line.trim().is_empty()) {
        Some(line) => sanitize_text_for_report(&redact_secrets(line.trim())),
        None => String::from("nothing on standard error"),
    }
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
        // Implementations, not launchers. Either side may be a wrapper script, and two distinct
        // wrappers for one compiler are two distinct files — so a launcher comparison would call
        // them independent while the compiler doing the work was the same on both sides.
        if under_test.is_same_implementation(reference) {
            return Err(HarnessError::new(
                "verifying that the compiler under test and the reference compilers are \
                 independent implementations",
                format!(
                    "the compiler under test and the reference compiler for the {target} arm run \
                     the same program ({}). Oracle (a) would then compare the compiler under test \
                     with itself, which agrees by construction: every comparison on that arm would \
                     pass, and the arm would report perfect agreement precisely because it had \
                     stopped being an independent oracle. This is judged on the program each name \
                     ultimately runs rather than on the names themselves, so two different wrapper \
                     scripts that hand the work to one driver are correctly recognised as one \
                     implementation. Point {} at the compiler under test and {} at a genuinely \
                     different reference compiler",
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
            if first.is_same_implementation(second) {
                return Err(HarnessError::new(
                    "verifying that each arm of oracle (a) has its own reference driver",
                    format!(
                        "the reference compilers for the {first_target} and {second_target} arms \
                         of oracle (a) run the same program ({}). One driver cannot serve two \
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
        let record = ToolRecord::discover(
            role,
            Some(VAR_BCC_BIN),
            &[],
            ToolKind::Compiler,
            None,
            strict,
        );
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
    let record = ToolRecord::from_build_system(role, Some(VAR_BCC_BIN), candidate, strict);
    if record.is_available() {
        return Ok(record);
    }
    // Vetting refused it. The same hard failure as an absent binary, because a compiler under test
    // the suite declines to trust must not quietly become a compiler under test the suite silently
    // did without — that is precisely the composition every other tool's refusal path avoids, and
    // this tool has no degraded mode at all.
    Err(compiler_under_test_error(&record.diagnosis()))
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

/// Identify the kernel for the environment fingerprint, **without naming the host**.
///
/// # Why not `uname -a`
///
/// The full banner's second field is the node name, and this line is copied verbatim into every
/// finding's `environment.txt`. A generated finding lives under the git-ignored build directory, so a
/// node name there costs nothing — but a curated finding is *committed*, and a machine's name is an
/// infrastructure identifier that is never needed to reproduce a divergence. Worse, it is the one
/// private identifier a validator on another machine cannot recognise: an account directory has a
/// shape, and a host name does not.
///
/// So it is never collected. The selectors below request exactly the facts the fingerprint exists for
/// — the system, the kernel release and version, the machine architecture, and the operating system —
/// and none of the fields that identify the host, which makes this a substitution rather than a
/// redaction: no fact the artifact is read for is lost, and there is nothing left to elide during
/// curation. That is strictly better than auditing for the node name afterwards, because the value the
/// audit cannot see is a value that was never written.
///
/// `-o` is an extension rather than a standardized selector, so a host whose utility rejects it falls
/// back to the four standardized ones; both spellings are node-name-free, so the fallback weakens
/// nothing that matters here.
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
        for selectors in KERNEL_SELECTORS {
            let Some(capture) = run_bounded_probe(Command::new(&uname).args(*selectors)) else {
                continue;
            };
            // The status is consulted here, unlike in a banner probe: a utility that rejected an
            // extension selector has said nothing about the kernel, and reading its refusal as an
            // answer is what would make the fallback below unreachable.
            if capture.timed_out || !capture.status.is_some_and(|status| status.success()) {
                continue;
            }
            if let Some(line) = first_non_empty_line(&capture.stdout) {
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

/// Node-name-free identification selectors, tried in order.
///
/// The first asks for system, kernel release, kernel version, machine and operating system. The second
/// drops the operating-system selector, which is an extension a host's utility may reject; the four
/// that remain are standardized. Neither spelling requests the node name, the processor or the hardware
/// platform, so neither can carry a host identity into a committed artifact.
const KERNEL_SELECTORS: &[&[&str]] = &[&["-srvmo"], &["-srvm"]];
