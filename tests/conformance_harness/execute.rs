//! Runs one built artifact and records exactly what it did.
//!
//! This module is the execution half of every cell of the build matrix. It launches an
//! artifact — natively, or under the target's emulator — and returns the three facts the
//! three oracles are decided on: the **raw stdout bytes**, the **termination**, and the
//! **raw wait status**. It renders no verdict and performs no comparison; that is
//! `compare.rs` and `classify.rs`. Keeping judgement out of here is what lets the same
//! function serve a differential comparison, a golden-record assertion and a finding
//! artifact without any of them influencing what was observed.
//!
//! # Native versus runner dispatch
//!
//! A target the host executes directly is run with **no runner prefix at all**: the argument
//! vector is the artifact and nothing else. That is precisely why x86-64 is oracle (b)'s
//! baseline — a baseline cell involves no emulator and therefore no emulation-related
//! variable. Every other target is run as `[<runner>, <artifact>]`, which is the shape the
//! repository documents (`qemu-<arch>-static ./binary`, and the per-target lines
//! `qemu-i386-static ./hello_i686`, `qemu-aarch64-static ./hello_arm64`,
//! `qemu-riscv64-static ./hello_riscv64`).
//!
//! The runner is **always** obtained from `env.rs` through [`Capabilities::runner_for`]. No
//! emulator name is spelled anywhere in this file, deliberately: the i686 target is executed
//! by `qemu-i386` rather than by anything containing `i686`, and both the plain and the
//! statically linked spellings are legitimate, so a name written here would eventually
//! disagree with the one discovery actually vetted. It is also why the artifacts are linked
//! statically — a static binary is self-contained, so emulated execution needs no sysroot and
//! no dynamic loader.
//!
//! A target whose runner is absent is **never a silent skip**. [`run`] returns
//! [`RunAttempt::RunnerUnavailable`], a condition distinct in the type from any program
//! behaviour, which `classify.rs` turns into `Verdict::Unavailable` — reported loudly and in
//! the summary, and escalated to a failure under the strict setting intended for continuous
//! integration.
//!
//! # Why the raw wait status is recorded, and signal death kept separate
//!
//! Exit status is captured as a raw wait status and modelled by [`Termination`], which
//! distinguishes an ordinary exit from death by signal. Conflating the two would hide exactly
//! the class of defect oracle (b) exists to find: a program that crashes on one backend and
//! returns a value on another would compare as "different numbers" rather than as "one of
//! them crashed", and a crash whose wait status happened to equal another cell's exit code
//! would compare as agreement.
//!
//! Both facts are read through the safe standard-library accessors —
//! [`ExitStatus::code`], [`ExitStatusExt::signal`] and [`ExitStatusExt::into_raw`] — so signal
//! extraction needs no escape from the compiler's safety guarantees, no raw pointer arithmetic
//! and no C binding. The keyword that would opt out of those guarantees appears nowhere in this
//! module, and neither does any reference to a foreign-function crate.
//!
//! # The 0-125 exit-code contract
//!
//! The operating system truncates a returned value into a byte, so an expected exit code is
//! only meaningful in the range 0 to [`MAX_CONTRACT_EXIT_CODE`]. This is measured rather than
//! theoretical: a program whose `main` ends in `return 300` was observed to produce wait
//! status **44**. Corpus programs therefore stay inside that range.
//!
//! This module records what it observed and **never clamps, corrects or reinterprets** a
//! status. A value outside the contract is a defect in the program or in the compiler that
//! built it, and masking it here would destroy the evidence; [`compare.rs`][super] compares,
//! and the expectation record states what was expected.
//!
//! # One authoritative bound, and an outer net that is never interpreted
//!
//! The standard library offers no timed wait on a child process, and the crate that does was
//! considered and rejected under the project's zero-dependency rule. This module implements the
//! substitute, and it is the **authoritative** mechanism on every path:
//!
//! 1. **A watchdog owned by this module** polls the child to completion and, at the budget,
//!    terminates it and then sweeps its whole process group with an uncatchable signal. A
//!    timeout is therefore a *fact this module established*, never a value read back out of
//!    somebody else's exit status.
//! 2. **The system `timeout` utility**, when discovery found one and the budget is a whole
//!    number of seconds, wraps the launch as an **outer net** at the budget plus
//!    [`TIMEOUT_UTILITY_OUTER_MARGIN`]. It exists only for the case where this module's own
//!    thread is itself starved, and in a healthy run it never fires.
//!
//! That ordering is the reverse of the obvious one, and it is deliberate. Putting the utility in
//! front made a cell's verdict depend on interpreting the utility's exit status, and that status
//! is not interpretable: expiry is reported as 124, which lies inside the corpus's own 0 to
//! [`MAX_CONTRACT_EXIT_CODE`] contract, so a program that legitimately returned 124 and a
//! program the utility killed were told apart only by a clock reading. **No exit status is
//! interpreted here any more.** A timeout is reported when, and only when, this module killed
//! the child itself.
//!
//! The measurements that motivated distrusting the utility are kept, because they are the reason
//! the arrangement is shaped this way. The implementation installed in the environment this suite
//! was developed against (`timeout (uutils coreutils) 0.2.2`) was measured to be defective in two
//! of its three spellings:
//!
//! | invocation | observed status | observed elapsed | conclusion |
//! | --- | --- | --- | --- |
//! | `timeout 1 sleep 5` | 124 | 1.008 s | correct |
//! | `timeout -s KILL 1 sleep 5` | 124 | **5.008 s** | no signal is sent; it waits for the child and then reports expiry |
//! | `timeout -s KILL 1 <spin loop>` | — | **never returned** | hung indefinitely with the child still at 100% CPU |
//! | `timeout -k 1 1 <spin loop>` | **125** | 1.107 s | the child does die, but 125 means "the utility itself failed" and collides with the contract range |
//! | `timeout 1 <spin loop>` | 124 | 1.008 s | correct |
//!
//! So `--signal=KILL` and `--kill-after` are deliberately **not** passed. Uncatchable
//! termination is guaranteed by this module instead.
//!
//! One further measured property of that implementation is recorded here rather than discovered
//! again later: it charges roughly **103 ms per invocation whatever the budget**, because it polls
//! its child on a 100 ms granularity. A trivial artifact costs 0.5 ms bare, 103 ms under the
//! utility, and 108 ms through this module — so the supervision added here is under 5 ms and the
//! rest is the utility. The wrapper is kept regardless, because the project plan names the system
//! `timeout` utility as the per-cell bound and this module's watchdog as the fallback when it is
//! absent; declining to use a tool the plan mandates is not a decision this module gets to make on
//! performance grounds. A maintainer who wants the cost back has one supported lever: point the
//! timeout-tool discovery at an implementation that signals promptly, and the watchdog behaviour
//! does not change either way.
//!
//! A timeout is a **first-class divergence class**, never an infrastructure error: a program
//! that finishes promptly under one compiler and hangs under another is exactly the kind of
//! defect this suite exists to surface, so [`Termination::TimedOut`] maps to
//! [`DivergenceClass::Timeout`] and the verdict is left to `classify.rs`.
//!
//! # Every child owns a process group, and the whole group is swept
//!
//! A child is placed in a **process group of its own** before it is spawned, and that group is
//! swept with an uncatchable signal and then verified empty on **every** exit path, the ordinary
//! one included. Terminating the process this module launched is not enough, and the outer-net
//! path shows why: when the utility expires it is the *utility* that this module holds a handle
//! to, so a program that ignored a catchable signal would outlive it, keep the capture pipes
//! open, and accumulate one leaked process per cell across a matrix of thousands. A compiler
//! driver that has already replaced itself with a stage, and an emulator whose guest forked, can
//! leave a descendant behind on a perfectly ordinary exit for the same reason.
//!
//! Sweeping needs a signal sent to a *group*, which the standard library cannot express, so it
//! is delivered by the vetted `kill` utility with an argument vector and no shell. The result is
//! never discarded: a group that still had a member after an uncatchable kill, and an
//! environment with no `kill` utility at all, are both recorded in the outcome notes. This
//! module reports that condition rather than swallowing it, because unlike a `--version` probe a
//! survivor here holds the pipes a verdict is computed from.
//!
//! # Capture ceilings
//!
//! Each stream is captured into a buffer bounded at [`CAPTURE_STREAM_BYTES_MAX`]. A corpus
//! program prints one short line per property it asserts, and a compiler diagnostic runs to
//! kilobytes at most, so the ceiling sits orders of magnitude above anything legitimate. A
//! miscompiled loop, a runaway emulator or a hostile artifact, however, can print without end,
//! and an unbounded buffer would exhaust the harness before any clock expired.
//!
//! Reaching the ceiling is **classified explicitly** rather than silently truncated: the reader
//! stops, the child and its whole group are terminated and reaped, and the execution is refused
//! with a diagnostic naming the stream, the ceiling and the byte counts. Refusal rather than
//! comparison is the same judgement this module already makes about an undrainable pipe, and for
//! the same reason. Two flooding sides truncated at the same ceiling would compare **equal**, so
//! a silent truncation is the one failure mode capable of turning a real divergence into a pass.
//!
//! # Capture, without the classic deadlock
//!
//! Standard output and standard error are both piped and drained by a reader thread each,
//! into a buffer the caller's thread can inspect at any moment. A child that filled a pipe
//! buffer while the parent sat in `wait()` would hang forever, and corpus programs print one
//! line per asserted property, so the hazard is real rather than hypothetical. Draining on
//! separate threads also means a partial capture survives a forcible kill.
//!
//! Standard error is captured because diagnostic text is often the fastest route to a
//! diagnosis in a finding artifact. It is **never compared**: wording legitimately differs
//! between compilers, so comparing it would flood a run with divergences that say nothing
//! about code correctness. No function here offers to compare it.
//!
//! # Workspace discipline and parallel safety
//!
//! Every child is launched with its current directory set to the cell's own workspace through
//! [`Command::current_dir`]. The process-wide working directory is never changed: the feature
//! area tests run concurrently in one process, so a `chdir` would be a data race rather than a
//! confinement. The artifact must be an absolute path to a regular file **inside** that
//! workspace, which is checked rather than assumed, so this module executes nothing outside the
//! build directory.
//!
//! The artifact is also **identity-bound**. The file the checks were made against is recorded by
//! device and inode number, the recording is verified again in the instant before the spawn, and
//! it is verified a third time once the child has been reaped. A path check performed and then
//! left behind while a runner is resolved and a command assembled proves nothing about the file
//! that eventually runs — a surviving helper could replace the leaf in between — so what is
//! executed is tied to the file that was inspected rather than merely to the name it had. A
//! second link to the file is refused for the same reason, since a link elsewhere is a second
//! name by which the contents could be exchanged. The residual window between the final check
//! and the kernel's own `execve` cannot be closed by any safe standard-library API, and the
//! post-execution check is what narrows it: an exchange would have to be reverted inside the
//! execution itself to go unnoticed.
//!
//! The child's environment is **replaced, not inherited**. Every variable is cleared and a
//! documented minimal set is restored: the search path, a fixed C locale and time zone, a dumb
//! terminal, and a private home and temporary directory pointing at the cell's own workspace.
//! Inheritance was worse in both directions. Outward, a continuous-integration token in the test
//! runner's environment reached every compiler, emulator and generated artifact this suite
//! launches, and any of them can print it. Inward, a loader or locale variable set on the
//! machine changes how a program formats its output, which byte-exact comparison reads as a
//! compiler divergence. A fixed locale is also what the corpus rules already require of the
//! programs themselves.
//!
//! No argument is passed to the program: a corpus program takes all of its input from literals
//! in its own source, and injecting anything would make the compared behaviour depend on the
//! harness. Standard input is redirected from the null device so a program can never block
//! waiting for input, and neither output stream is ever inherited, which would both pollute the
//! runner's own output and destroy byte-exactness.
//!
//! Two things this module does **not** do are named here rather than left as a surprise, since
//! "the artifact is inside the workspace" is easily over-read as confinement:
//!
//! - **It applies no operating-system isolation.** There is no namespace, no `chroot`, no
//!   syscall filter and no network restriction, so a program that opened a socket or an absolute
//!   path would succeed. What keeps a corpus program from doing either is authoring policy — every
//!   input is a literal in its own source — which is auditable because the corpus is committed. The
//!   execution runner itself is likewise an installed tool executed from outside the build tree;
//!   only the artifact it is pointed at is required to be inside a workspace. The **environment** is
//!   the one thing that *is* enforced: it is cleared and replaced, and `TMPDIR`, `TMP`, `TEMP` and
//!   `HOME` all point at the cell's workspace, so a program that consults them writes there. That is
//!   a guarantee about what the child is told, not a namespace — a program that hard-codes a path
//!   ignores it.
//! - **It does not decide what happens to a crash dump.** When a child dies from a signal,
//!   whether a core image is written and where it lands are decided by the host's own
//!   `kernel.core_pattern` and core-size limit. An absolute pattern therefore writes outside the
//!   workspace on a crash. Changing that would mean lowering the child's core limit between the
//!   fork and the exec, which no safe standard-library API offers, so the honest position is that
//!   this module itself writes nothing outside the workspace and that a crash dump is a property
//!   of the machine the suite runs on.
//!
//! This module holds no global mutable state, registers nothing, and shares no path between
//! cells, so it needs no lock under `cargo test`'s default concurrency.
//!
//! Edition 2021, minimum supported Rust 1.70. Only the standard library is used, as the
//! project permits no third-party crate; this module is what stands in for a
//! timed-child-wait crate.

use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, Metadata};
use std::io::{self, Read};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::env::{confirm_vetted_tools_unchanged, kill_tool, Capabilities};
use super::sandbox::{
    workspace_path, Workspace, BCC_EXIT_NAME, BCC_STDERR_NAME, BCC_STDOUT_NAME,
    REFERENCE_EXIT_NAME, REFERENCE_STDERR_NAME, REFERENCE_STDOUT_NAME,
};
use super::{
    ensure_within, isolate_child_environment, own_process_group, posix_command_line,
    redact_secrets, require_regular_file, sanitize_text_for_report, shown_path,
    terminate_process_group, CaptureIntegrity, CellKey, DivergenceClass, GroupTermination,
    HarnessError, HarnessResult, Target, CAPTURE_CHUNK_BYTES, CAPTURE_RETAINED_BYTES_MAX,
};

/// The highest exit code a corpus program may be expected to return.
///
/// The operating system truncates a returned value into a byte, so a larger value does not
/// survive: `return 300` was measured to produce wait status 44. Values above this bound are
/// out of contract for the corpus — and are still recorded faithfully if one appears, because
/// masking an out-of-contract status would destroy the evidence that something produced it.
pub const MAX_CONTRACT_EXIT_CODE: i32 = 125;

/// How far beyond the budget the external `timeout` utility's outer net is set.
///
/// This module's own watchdog is the authoritative bound and fires at the budget exactly. The
/// utility sits five seconds behind it, which is long enough that the watchdog always wins the
/// race in a healthy run and short enough that a starved harness thread cannot stall a run
/// indefinitely. The utility's own exit status is never interpreted, so a net that does fire is
/// visible only as an ordinary exit of whatever the utility reported, alongside the recorded fact
/// that the launch was wrapped.
pub const TIMEOUT_UTILITY_OUTER_MARGIN: Duration = Duration::from_secs(5);

/// The most bytes either captured stream may hold before the execution is stopped and refused.
///
/// Four mebibytes per stream, so an execution can hold at most eight. The arithmetic behind the
/// choice: the widest corpus program prints a few hundred bytes, the widest compiler diagnostic
/// this suite provokes runs to a few kilobytes, and the concurrency is bounded by the built-in
/// harness's thread pool over fourteen feature areas — so the worst case a healthy run can reach
/// is a few hundred kilobytes in total, while the worst case a pathological one can reach is
/// bounded rather than open-ended.
///
/// The ceiling is deliberately generous. Its purpose is to bound catastrophe, not to police
/// output: a program that legitimately printed a megabyte would already have broken the corpus's
/// one-line-per-property rule long before reaching this.
pub const CAPTURE_STREAM_BYTES_MAX: usize = CAPTURE_RETAINED_BYTES_MAX as usize;

/// How long the reader threads are given to finish once the child has been reaped.
///
/// They normally finish immediately, because a reaped child's pipe write ends are closed and
/// the reads return end-of-file. A reader still blocked after this long means some process
/// that inherited the pipes is holding them open, which is handled explicitly rather than
/// waited on forever.
const CAPTURE_DRAIN_GRACE: Duration = Duration::from_secs(2);

/// The first interval between completion polls, kept short so that a cell costing a few
/// milliseconds is not delayed by the wait itself.
const POLL_INTERVAL_INITIAL: Duration = Duration::from_micros(250);

/// The longest interval between completion polls. Reached by doubling, so a long-running cell
/// costs a negligible number of wake-ups without letting a short one wait.
const POLL_INTERVAL_MAX: Duration = Duration::from_millis(10);

/// Note recorded when a reader thread finds the shared capture buffer poisoned.
///
/// A poisoned lock means another thread panicked while holding it, so the bytes already gathered
/// cannot be trusted to be whole. The reader stops and says so rather than continuing against a
/// buffer of unknown content, and the caller turns an incomplete capture into a hard error unless
/// the child was killed at the budget.
const POISONED_CAPTURE_BUFFER: &str =
    "the shared capture buffer was poisoned by a panic in another thread";

/// How a child process ended.
///
/// The three cases are kept distinct because they mean entirely different things about the
/// program, and because collapsing any two of them would hide a defect: a crash reported as an
/// exit could compare equal to a genuine exit code, and a timeout reported as a crash would
/// lose the one fact that distinguishes a hang from a fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Termination {
    /// The program ran to completion and returned this exit code.
    ///
    /// Recorded exactly as observed, including a value outside the corpus's 0 to
    /// [`MAX_CONTRACT_EXIT_CODE`] contract — see [`Termination::within_exit_code_contract`].
    Exited(i32),
    /// The program was terminated by this signal number rather than exiting.
    ///
    /// The shape of a crash, and the reason exit status is carried as a raw wait status:
    /// nothing here can be mistaken for an ordinary exit.
    Signalled(i32),
    /// Execution exceeded its per-cell budget and was terminated.
    ///
    /// A divergence class rather than an infrastructure error. Whether it was the external
    /// utility or this module's own watchdog that enforced the budget is recorded separately, and
    /// both facts reach the reader through [`RunOutcome::status_record`] and
    /// [`RunOutcome::describe`] rather than through a predicate a caller has to remember to ask.
    TimedOut,
}

impl Termination {
    /// The token used in a status record, a report row and a diagnostic.
    pub fn label(self) -> &'static str {
        match self {
            Termination::Exited(_) => "exited",
            Termination::Signalled(_) => "signalled",
            Termination::TimedOut => "timed_out",
        }
    }

    /// The exit code, present only for an ordinary exit.
    ///
    /// `None` for a signal and for a timeout, so a caller cannot accidentally compare a crash
    /// against a number.
    pub fn exit_code(self) -> Option<i32> {
        match self {
            Termination::Exited(code) => Some(code),
            Termination::Signalled(_) | Termination::TimedOut => None,
        }
    }

    /// The terminating signal number, present only for a signalled termination.
    pub fn signal(self) -> Option<i32> {
        match self {
            Termination::Signalled(signal) => Some(signal),
            Termination::Exited(_) | Termination::TimedOut => None,
        }
    }

    /// True only for an exit code of zero.
    ///
    /// Deliberately not named `is_ok`: a corpus program may be expected to return a non-zero
    /// code, so this answers "did it succeed", never "did it meet its expectation".
    pub fn succeeded(self) -> bool {
        matches!(self, Termination::Exited(0))
    }

    /// True when the budget was exceeded.
    pub fn timed_out(self) -> bool {
        matches!(self, Termination::TimedOut)
    }

    /// Whether an ordinary exit code lies inside the corpus's 0 to
    /// [`MAX_CONTRACT_EXIT_CODE`] contract.
    ///
    /// `false` for a signal and for a timeout, neither of which carries an exit code at all.
    /// This reports a fact; it never changes one.
    pub fn within_exit_code_contract(self) -> bool {
        match self {
            Termination::Exited(code) => (0..=MAX_CONTRACT_EXIT_CODE).contains(&code),
            Termination::Signalled(_) | Termination::TimedOut => false,
        }
    }

    /// The **shape** of the divergence this termination represents, if it is one by itself.
    ///
    /// A crash and a timeout are divergent however they are compared, so each maps to its
    /// class. An ordinary exit is not: whether its code and its output agree with an oracle is
    /// decided elsewhere, which is why this returns `None` for every exit code including a
    /// non-zero one.
    ///
    /// This reports a shape and never a verdict — `classify.rs` alone maps a shape onto a
    /// verdict.
    pub fn divergence_class(self) -> Option<DivergenceClass> {
        match self {
            Termination::Exited(_) => None,
            Termination::Signalled(_) => Some(DivergenceClass::RunCrash),
            Termination::TimedOut => Some(DivergenceClass::Timeout),
        }
    }
}

impl fmt::Display for Termination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Termination::Exited(code) => write!(f, "exited with code {code}"),
            Termination::Signalled(signal) => write!(f, "terminated by signal {signal}"),
            Termination::TimedOut => f.write_str("exceeded its execution budget"),
        }
    }
}

/// How this execution was bounded.
///
/// Both arrangements are authoritative in the same place — this module's own watchdog — so the
/// distinction records what stood *behind* it, never who decided the verdict. That is the whole
/// point of the arrangement: a reader of a status record never has to wonder whether an exit
/// status was a program's answer or an external utility's report of expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeoutEnforcement {
    /// This module's watchdog was the only bound, because no `timeout` utility was discovered or
    /// the budget is not a whole number of seconds.
    Watchdog,
    /// This module's watchdog was the bound, with the external `timeout` utility wrapped around
    /// the launch as an outer net at the budget plus [`TIMEOUT_UTILITY_OUTER_MARGIN`].
    WatchdogWithOuterNet,
}

impl TimeoutEnforcement {
    /// The token used in a status record and a report row.
    pub fn label(self) -> &'static str {
        match self {
            TimeoutEnforcement::Watchdog => "watchdog",
            TimeoutEnforcement::WatchdogWithOuterNet => "watchdog+outer-net",
        }
    }
}

impl fmt::Display for TimeoutEnforcement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Everything one execution produced.
///
/// The three fields the oracles read are the raw stdout bytes, the termination and the raw
/// wait status. The rest is what makes a report row and a finding artifact self-sufficient:
/// the full argument vector, the working directory, the runner, the measured duration, the
/// budget it was measured against, which mechanism enforced that budget, and any note the
/// execution needs to carry with it.
///
/// # Why every field is private
///
/// This is the evidence a verdict is derived from, and every accessor below hands out a
/// borrow or a copy of something that was *observed*. A writable field would let a caller
/// change the evidence after the fact — and the whole value of a differential oracle is that
/// nothing between the program and the comparison may adjust what the program did. Two
/// specific guarantees follow from construction and could not survive assignment: the stdout
/// bytes are exactly the bytes the child wrote, with no normalization of any kind, and the
/// termination agrees with the raw wait status it was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    /// The program's own argument vector: the runner and the artifact for a corpus execution,
    /// or exactly what the caller assembled for a shared-helper invocation.
    ///
    /// Deliberately free of this module's timeout scaffolding. A finding artifact's commands
    /// file must reproduce a cell *without* the harness, and an expectation record's run
    /// template is `<runner> <artifact>`, so the line a reader is handed has to be the program's
    /// line and nothing else. What was literally spawned is kept separately in the
    /// `launch_argv` field, published as [`RunOutcome::launch_command_line`], so nothing is hidden
    /// either.
    argv: Vec<String>,
    /// The argument vector as literally spawned: [`RunOutcome::argv`] when this module enforced
    /// the budget itself, and the external `timeout` utility wrapping it when that utility did.
    ///
    /// Every diagnostic about the spawn, the wait and the kill is phrased in these terms,
    /// because a failure to launch belongs to the command that was actually launched.
    launch_argv: Vec<String>,
    /// The working directory the child was given — a cell's own workspace for a corpus
    /// execution, and whatever the caller set for a shared-helper invocation.
    working_dir: Option<PathBuf>,
    /// The emulator the artifact was run under, or `None` when it ran directly.
    runner: Option<PathBuf>,
    termination: Termination,
    /// The wait status exactly as the operating system reported it.
    ///
    /// Kept alongside [`RunOutcome::termination`] rather than replaced by it, because a
    /// comparison of raw wait statuses is what keeps a signal death from being conflated with
    /// a numerically equal ordinary exit.
    raw_wait_status: i32,
    /// Standard output, byte for byte as the program wrote it: no lossy conversion, no
    /// line-ending normalization, no trimming. Byte-exact comparison is the point of the
    /// suite, and any normalization here would silently mask a real divergence.
    stdout: Vec<u8>,
    /// Standard error, captured for finding artifacts and never compared.
    stderr: Vec<u8>,
    /// How faithfully [`RunOutcome::stdout`] represents what the program wrote.
    ///
    /// Recorded rather than assumed, because the retained bytes are bounded and the drain is
    /// bounded, so "these are the bytes" is a claim that has to be substantiated. A construction
    /// that is not complete is only reachable for a cell that timed out, whose stdout no oracle
    /// compares — [`execute_bounded`] refuses every other combination outright.
    stdout_integrity: CaptureIntegrity,
    /// How faithfully [`RunOutcome::stderr`] represents what the program wrote.
    ///
    /// Standard error is never compared, so a quota-truncated one is recorded and reported rather
    /// than refused; a reader that never finished is refused, because that is a statement about a
    /// process still holding the pipes rather than about one stream.
    stderr_integrity: CaptureIntegrity,
    duration: Duration,
    budget: Duration,
    enforcement: TimeoutEnforcement,
    /// Anything unusual about the execution that a reader must be told: an external utility
    /// that failed to enforce its own budget, a stream that could not be drained cleanly, a
    /// read error on a pipe. Each note is already safe to render on one line.
    notes: Vec<String>,
}

impl RunOutcome {
    /// The program's own argument vector, free of this module's timeout scaffolding.
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// The argument vector as a single POSIX shell line, quoted so it can be pasted into a
    /// terminal and reproduce this execution exactly.
    ///
    /// This is the line a finding artifact publishes and a report row names, and it is
    /// reproducible on its own: no wrapper of this module's appears in it.
    pub fn command_line(&self) -> String {
        posix_command_line(&self.argv)
    }

    /// The argument vector as literally spawned — including the external `timeout` utility when
    /// it wrapped the launch as an outer net — as a single POSIX shell line.
    ///
    /// Equal to [`RunOutcome::command_line`] whenever nothing wrapped the launch. Present so that
    /// the scaffolding is auditable rather than invisible: a reader comparing this against
    /// [`RunOutcome::command_line`] can see exactly what the harness added. The vector itself is
    /// not published, because the quoted line is the form every consumer needs and an unquoted
    /// vector invites a caller to re-assemble it differently.
    pub fn launch_command_line(&self) -> String {
        posix_command_line(&self.launch_argv)
    }

    /// Whether the external utility wrapped the program, making the two command lines differ.
    pub fn launch_was_wrapped(&self) -> bool {
        self.launch_argv != self.argv
    }

    /// The emulator the artifact ran under, or `None` when it ran directly on the host.
    pub fn runner(&self) -> Option<&Path> {
        self.runner.as_deref()
    }

    /// How the child ended.
    pub fn termination(&self) -> Termination {
        self.termination
    }

    /// The wait status exactly as the operating system reported it.
    pub fn raw_wait_status(&self) -> i32 {
        self.raw_wait_status
    }

    /// Standard output, byte for byte as the program wrote it.
    ///
    /// This is what oracles (a), (b) and (c) compare. It is returned as bytes rather than text
    /// on purpose: a program that emitted a byte sequence which is not valid text must compare
    /// unequal to one that did not, and any string conversion would erase that difference.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Standard error, byte for byte.
    ///
    /// For finding artifacts and diagnostics only. Diagnostic wording legitimately differs
    /// between compilers, so this is **never** compared — no oracle reads it, and no helper
    /// here offers to compare it.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// The budget the execution was measured against.
    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// The text written to the status entry of a workspace.
    ///
    /// A line-oriented `key = value` record, in the same spelling an expectation record uses,
    /// so that a retained workspace can be read by eye and by a script without either needing
    /// this module. Every value is either a number, a fixed token, or text already made safe
    /// for a single line; the argument vector is rendered in its shell-quoted form because its
    /// purpose is to be re-run rather than to be parsed.
    pub fn status_record(&self) -> String {
        let mut record = String::new();
        record.push_str(&format!("termination = {}\n", self.termination.label()));
        if let Some(code) = self.termination.exit_code() {
            record.push_str(&format!("exit_code = {code}\n"));
            record.push_str(&format!(
                "exit_code_within_contract = {}\n",
                self.termination.within_exit_code_contract()
            ));
        }
        if let Some(signal) = self.termination.signal() {
            record.push_str(&format!("signal = {signal}\n"));
        }
        record.push_str(&format!("raw_wait_status = {}\n", self.raw_wait_status));
        record.push_str(&format!("succeeded = {}\n", self.termination.succeeded()));
        record.push_str(&format!("timed_out = {}\n", self.timed_out()));
        // Stated only when the termination is divergent by its own shape, which is the fact a
        // reader of a retained failure directory needs first: a crash and a hang are divergences
        // however they are compared, so no byte comparison could have rescued the cell.
        if let Some(shape) = self.divergence_class() {
            record.push_str(&format!("divergence_by_shape = {}\n", shape.label()));
        }
        record.push_str(&format!("duration_ms = {}\n", self.duration().as_millis()));
        record.push_str(&format!("budget_ms = {}\n", self.budget().as_millis()));
        record.push_str(&format!("timeout_enforcement = {}\n", self.enforcement()));
        record.push_str(&format!(
            "runner = {}\n",
            match self.runner() {
                Some(runner) => shown_path(runner),
                None => String::from("(none: executed directly on the host)"),
            }
        ));
        record.push_str(&format!(
            "working_dir = {}\n",
            describe_working_dir(self.working_dir())
        ));
        record.push_str(&format!("stdout_bytes = {}\n", self.stdout.len()));
        record.push_str(&format!(
            "stdout_capture_ceiling_bytes = {CAPTURE_STREAM_BYTES_MAX}\n"
        ));
        record.push_str(&format!("stderr_bytes = {}\n", self.stderr.len()));
        // The fidelity of each capture is recorded unconditionally, so a reader can tell a complete
        // capture from a record that simply predates the accounting.
        record.push_str(&self.stdout_integrity.record_lines("stdout"));
        record.push_str(&self.stderr_integrity.record_lines("stderr"));
        record.push_str(&format!("argv = {}\n", self.command_line()));
        // Only when it differs, so an ordinary record stays free of a line that repeats the one
        // above it — and so a record that does carry one is worth reading.
        if self.launch_was_wrapped() {
            record.push_str(&format!("launch = {}\n", self.launch_command_line()));
        }
        for note in self.notes() {
            record.push_str(&format!("note = {note}\n"));
        }
        record
    }

    /// Write the captured streams and the status record into a workspace, and return the three
    /// paths in the order stdout, stderr, status.
    ///
    /// This is what makes a retained failure directory self-sufficient: the bytes that were
    /// compared, the diagnostics that were not, and the termination that produced them all sit
    /// beside the artifact they came from.
    ///
    /// The caller chooses the entry names, because one workspace holds both sides of a
    /// differential comparison — see [`CaptureNames::bcc`] and [`CaptureNames::reference`].
    ///
    /// # Errors
    ///
    /// Propagates the workspace's own refusal of a name that is not a plain entry of the
    /// directory, and any failure to write. Both are hard failures: a cell whose evidence
    /// could not be recorded cannot be investigated afterwards.
    pub fn persist(
        &self,
        workspace: &Workspace,
        names: &CaptureNames,
    ) -> HarnessResult<(PathBuf, PathBuf, PathBuf)> {
        let stdout_path = workspace.write(names.stdout(), &self.stdout)?;
        let stderr_path = workspace.write(names.stderr(), &self.stderr)?;
        let status_path = workspace.write_text(names.exit(), &self.status_record())?;
        Ok((stdout_path, stderr_path, status_path))
    }

    /// A one-line description for a report row or an outcome detail.
    ///
    /// Carries the termination, the raw wait status, the **budget**, the byte counts and the command
    /// line, which is everything needed to understand the row without re-running it. Passed through
    /// the report-safe rendering, so no captured path or note can forge a column or hide a line.
    ///
    /// # Why the measured duration is deliberately absent
    ///
    /// This line is what the comparator puts into a divergence's detail, and that detail is rendered
    /// verbatim into a report row, into the summary and into a finding's `diff.txt`. A wall-clock
    /// millisecond count in it would make every one of those bytes differ between two runs of the
    /// same inputs, which would destroy the property those artifacts exist to have: diff two runs and
    /// anything that differs is something the run genuinely found. The budget is kept because it is a
    /// pure function of the configuration and is the fact a timeout has to be read against.
    ///
    /// The measured duration is not discarded — it is recorded, once, in the `duration_ms` field of
    /// this execution's own status record, which is where a retained workspace and a finding's
    /// `.exit` entry both read it from. Timing telemetry lives there and nowhere a diff looks.
    /// [`RunOutcome::describe_with_timing`] is the terminal-progress variant, mirroring
    /// `CompileOutcome`'s split for exactly the same reason.
    pub fn describe(&self) -> String {
        let mut described = format!(
            "{} (raw wait status {}) against a {} ms budget, {} stdout bytes, {} stderr \
             bytes, bounded by the {} mechanism, command: {}",
            self.termination,
            self.raw_wait_status,
            self.budget().as_millis(),
            self.stdout.len(),
            self.stderr.len(),
            self.enforcement(),
            self.command_line()
        );
        described.push_str(&self.stdout_integrity.describe("stdout"));
        described.push_str(&self.stderr_integrity.describe("stderr"));
        for note in self.notes() {
            described.push_str("; note: ");
            described.push_str(note);
        }
        sanitize_text_for_report(&described)
    }

    /// The same line with the measured duration appended, for progress output only.
    ///
    /// Separated from [`RunOutcome::describe`] rather than offered as an option, because the
    /// distinction being enforced is *where the text may go*: this variant is safe on a terminal and
    /// never safe in a file whose bytes are compared. Keeping them as two named methods makes the
    /// wrong choice visible at the call site instead of hiding it in an argument — the same split
    /// `CompileOutcome` makes, for the same reason.
    pub fn describe_with_timing(&self) -> String {
        format!("{} ({} ms)", self.describe(), self.duration().as_millis())
    }

    /// Notes a reader must be told about, each already safe to render on one line.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Which mechanism was configured to bound this execution.
    pub fn enforcement(&self) -> TimeoutEnforcement {
        self.enforcement
    }

    /// How long the execution took, measured across the spawn and the wait.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// The working directory the child was given.
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_deref()
    }

    /// True when the budget was exceeded.
    pub fn timed_out(&self) -> bool {
        self.termination.timed_out()
    }

    /// The argument vector as literally spawned, including the external `timeout` utility when
    /// that utility enforced the budget.
    ///
    /// Equal to [`RunOutcome::argv`] whenever this module's own watchdog did the enforcing.
    pub fn launch_argv(&self) -> &[String] {
        &self.launch_argv
    }

    /// The shape of the divergence this execution represents by itself, if any.
    ///
    /// Delegates to [`Termination::divergence_class`]: a crash or a timeout is divergent
    /// however it is compared, while an ordinary exit is judged by the oracles.
    pub fn divergence_class(&self) -> Option<DivergenceClass> {
        self.termination.divergence_class()
    }
}

impl fmt::Display for RunOutcome {
    /// One line, identical to [`RunOutcome::describe`], so a diagnostic and a report row read
    /// the same.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

/// The three workspace entry names one side of a comparison writes its capture into.
///
/// A cell workspace holds both sides — the artifact built by the compiler under test and the
/// one built by the reference compiler — so the names cannot be fixed by this module. The two
/// constructors below use the workspace's own published spellings, and
/// [`CaptureNames::new`] exists for a caller that needs a third set.
///
/// Note that the workspace's standard-error entry is shared by a cell's compile step and its
/// run step. A caller that needs to keep both must give the run a distinct name through
/// [`CaptureNames::new`]; otherwise the later write wins.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaptureNames {
    stdout: String,
    stderr: String,
    exit: String,
}

impl CaptureNames {
    /// The entry names for the artifact built by the compiler under test.
    pub fn bcc() -> CaptureNames {
        CaptureNames::new(BCC_STDOUT_NAME, BCC_STDERR_NAME, BCC_EXIT_NAME)
    }

    /// The entry names for the artifact built by the reference compiler.
    pub fn reference() -> CaptureNames {
        CaptureNames::new(
            REFERENCE_STDOUT_NAME,
            REFERENCE_STDERR_NAME,
            REFERENCE_EXIT_NAME,
        )
    }

    /// Entry names chosen by the caller.
    ///
    /// The names are not validated here on purpose: the workspace is the single authority on
    /// what may be written into it, and it refuses an empty name, a name carrying a path
    /// separator and a name carrying a character that cannot appear in a report at the moment
    /// of the write. Validating in two places would invite the two rules to drift apart.
    pub fn new(
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        exit: impl Into<String>,
    ) -> CaptureNames {
        CaptureNames {
            stdout: stdout.into(),
            stderr: stderr.into(),
            exit: exit.into(),
        }
    }

    /// Entry name for the captured standard output.
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Entry name for the captured standard error.
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Entry name for the recorded termination status.
    pub fn exit(&self) -> &str {
        &self.exit
    }
}

/// A target that cannot be executed here, because the emulator it needs is absent.
///
/// This is not an error and not a skip. It is the distinct condition `classify.rs` turns into
/// `Verdict::Unavailable`: reported loudly, listed in the summary, and escalated to a failure
/// under the strict setting intended for continuous integration, where a missing runner means a
/// broken workflow rather than a modest machine.
/// Both halves are carried, because they are read separately. [`RunnerUnavailable::detail`] is the
/// reason on its own, for a caller that has already grouped its rows by target, and
/// [`RunnerUnavailable::describe`] composes the target with it for a caller that has not. The detail
/// deliberately does not name the target itself: a reason that repeated it would drift out of step
/// with the composed line, and the target is the one field a report row cannot be rendered without.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunnerUnavailable {
    target: Target,
    detail: String,
}

impl RunnerUnavailable {
    /// Why it cannot be executed, phrased for a report row and already safe on one line.
    ///
    /// Does **not** name the target: the target is carried by the value itself, and a reason that
    /// repeated it would either duplicate the rendering below or drift out of step with it.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// The complete one-line report row: which target, and why nothing ran for it.
    ///
    /// This is the form every consumer wants, and composing it here rather than baking the target
    /// into [`RunnerUnavailable::detail`] keeps the two facts separable — a caller that already
    /// groups its rows by target can render the reason alone.
    pub fn describe(&self) -> String {
        sanitize_text_for_report(&format!("{} {}", self.target(), self.detail()))
    }
    /// The target whose artifacts cannot be executed.
    pub fn target(&self) -> Target {
        self.target
    }
}

impl fmt::Display for RunnerUnavailable {
    /// One line, identical to [`RunnerUnavailable::describe`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

/// The result of asking this module to run an artifact: either it ran, or its runner is absent.
///
/// # Why this is not simply a [`RunOutcome`]
///
/// Execution has three situations and only two of them are program behaviour. A native target
/// runs the binary directly, a foreign target runs it under its emulator, and a foreign target
/// with no emulator cannot be run at all. Folding the third into the second — by returning an
/// outcome that merely happens to look like a failure — puts the most dangerous default on the
/// most likely mistake: a foreign binary handed to the host produces a non-zero status that a
/// comparison reading only status and stdout cannot tell apart from the compiler under test
/// having produced a program that fails, so a missing emulator would be reported as a compiler
/// defect.
///
/// Making the two structurally distinct means a caller cannot read one as the other. It is the
/// same argument `env.rs` makes for returning an execution mode rather than a bare runner path,
/// and it is what keeps "a missing oracle is never a silent pass" a property of the type rather
/// than a rule every caller must remember.
/// # Why the ran case is boxed
///
/// A run outcome carries both captured streams, both capture-integrity records, the raw status and
/// the collected notes, and is an order of magnitude larger than the unavailability beside it. An
/// unboxed enumeration is sized for its largest variant, so every value of this type — including
/// every one of the far more numerous unavailable ones on a machine missing an emulator — would
/// carry that footprint. Boxing the large case is the same choice the driver makes for its own
/// authority values, for the same reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAttempt {
    /// The artifact was launched and reached a termination.
    Ran(Box<RunOutcome>),
    /// Nothing was launched, because the target's emulator is absent.
    RunnerUnavailable(RunnerUnavailable),
}

impl RunAttempt {
    /// A one-line description of either case, for a report row.
    ///
    /// The only inherent method, deliberately. Every other consumer matches the two variants, which
    /// is what the type exists to force: the compiler then checks that match for exhaustiveness,
    /// where an `Option`-returning accessor beside them would hand a caller back exactly the "read
    /// one as the other" affordance the distinction was introduced to remove. A caller that took the
    /// `None` for a failed run would report a missing emulator as a compiler defect, which is the
    /// "missing oracle became a silent pass" failure this type exists to prevent.
    pub fn describe(&self) -> String {
        match self {
            RunAttempt::Ran(outcome) => outcome.describe(),
            RunAttempt::RunnerUnavailable(reason) => reason.describe(),
        }
    }
}

impl fmt::Display for RunAttempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

/// The per-cell execution budget this run operates under.
///
/// Read from the validated configuration snapshot `env.rs` holds rather than from the
/// environment, so two concurrently executing feature areas cannot be bounded differently. The
/// snapshot guarantees a positive whole number of seconds within a ceiling, which is what lets
/// the external utility be used for a cell without any rounding.
pub fn budget_for(caps: &Capabilities) -> Duration {
    Duration::from_secs(caps.config().timeout_secs())
}

/// Run one built artifact for one cell and record exactly what it did.
///
/// The cell's identity supplies both facts this function needs beyond the artifact itself: the
/// target, which decides how the artifact is launched, and — through
/// [`workspace_path`] — the one directory the child is permitted to run in. The artifact is
/// launched directly when the host executes that target natively, and under the emulator `env.rs`
/// vetted for it otherwise, with no arguments and the null device as its standard input; both output
/// streams are captured and neither is inherited. The environment is neither inherited nor added to:
/// it is **replaced** with the suite's fixed set, so a program that consulted one would see the same
/// values on every machine.
///
/// Returns [`RunAttempt::RunnerUnavailable`] — never an error and never a silent skip — when a
/// non-native target has no emulator here.
///
/// # Errors
///
/// A hard failure is reserved for a broken invariant rather than a property of the machine:
///
/// - the workspace supplied is not the directory the cell's identity derives, which would mean
///   the build wrote its artifact somewhere this execution would never look;
/// - the artifact path is not absolute, so what gets executed would depend on a working
///   directory the test harness makes no guarantee about;
/// - the artifact is not a regular file, or is a symbolic link, which
///   [`require_regular_file`] refuses rather than follows — a link would let something outside
///   the build directory be executed while every report still showed the workspace path;
/// - the artifact does not resolve to a path strictly inside the cell's workspace, which is the
///   mechanical half of the suite's path discipline;
/// - the artifact has a second hard link, or is not the same file at the moment of the spawn, or
///   is no longer the same file once the child has been reaped, all of which mean the program
///   that ran cannot be shown to be the program that was inspected;
/// - the child could not be spawned, could not be reaped, or ended in a way the operating
///   system reported as neither an exit nor a signal;
/// - a stream reached its capture ceiling, since the bytes held are then a prefix of what the
///   program wrote rather than what it wrote;
/// - a stream could not be drained after a child that ended normally, since a silently
///   truncated capture would be compared and reported as a compiler divergence.
pub fn run(
    artifact: &Path,
    key: &CellKey,
    workspace: &Workspace,
    caps: &Capabilities,
) -> HarnessResult<RunAttempt> {
    let target = key.target();
    let context = format!(
        "running the {target} artifact {} in the workspace {}",
        shown_path(artifact),
        shown_path(workspace.root())
    );

    // The child's current directory is derived from the cell's identity through
    // [`workspace_path`] rather than taken from the workspace value handed in, and then the two
    // are required to agree. That is not ceremony: the build step writes its artifact into the
    // path the workspace names while this step runs in the directory the identity derives, so a
    // disagreement would leave the artifact somewhere the execution never looks — and the cell
    // would fail for a reason that has nothing to do with any compiler. `sandbox` derives both,
    // so they agree by construction; requiring it here means a future change to either
    // derivation is reported as the harness defect it is instead of surfacing as a divergence.
    let working_dir = workspace_path(key);
    if working_dir != workspace.root() {
        return Err(HarnessError::new(
            context,
            format!(
                "the workspace supplied for this execution is {} while the cell's identity \
                 derives {}; a child launched in one directory cannot execute an artifact written \
                 into the other, and running anyway would report a missing artifact as a compiler \
                 divergence",
                workspace.root().display(),
                working_dir.display()
            ),
        ));
    }

    if !artifact.is_absolute() {
        return Err(HarnessError::new(
            context,
            format!(
                "the artifact path {} is not absolute; an artifact is named from its cell's own \
                 workspace so that what gets executed never depends on the working directory, \
                 about which the test harness makes no guarantee",
                shown_path(artifact)
            ),
        ));
    }
    require_regular_file(&context, artifact)?;
    ensure_within(&context, workspace.root(), artifact)?;
    // Recorded here, checked again in the instant before the spawn and once more after the child
    // has been reaped. The checks above establish facts about a *name*; runner resolution and
    // command assembly happen after them, and a name proves nothing about the file that
    // eventually runs.
    let identity = ArtifactIdentity::of(&context, artifact)?;

    let runner = match resolve_runner(target, caps) {
        Ok(runner) => runner,
        Err(unavailable) => return Ok(RunAttempt::RunnerUnavailable(unavailable)),
    };

    // The argument vector is exactly the documented shape: the artifact alone when it runs
    // natively, and `<runner> <artifact>` when it does not. Nothing else is appended, because a
    // corpus program reads all of its input from literals in its own source.
    let mut command = match runner.as_deref() {
        Some(emulator) => {
            let mut command = Command::new(emulator);
            command.arg(artifact);
            command
        }
        None => Command::new(artifact),
    };
    // Per-child, never process-wide: the feature area tests run concurrently in one process, so
    // altering the shared working directory would be a data race rather than a confinement.
    command.current_dir(&working_dir);

    // The environment is replaced rather than inherited, and the private home and temporary
    // directory are the cell's own workspace, so a program that consults either stays inside the
    // build directory. A fixed C locale is also what byte-exact comparison requires of the
    // formatting the program performs. It is requested here and performed inside
    // [`execute_bounded`], because only that function knows whether the launch ends up wrapped.
    let outcome = execute_bounded(
        command,
        budget_for(caps),
        runner,
        caps.timeout_tool().path(),
        Some(BoundArtifact {
            path: artifact,
            identity: &identity,
        }),
        Some(workspace.root()),
    )?;

    // The third check. An exchange performed inside the window the pre-spawn check leaves open
    // would have to be reverted before the child was reaped to escape notice, which is a far
    // narrower opportunity than replacing a validated leaf at leisure.
    if let Some(changed) = identity.changed_since(artifact) {
        return Err(HarnessError::new(
            context,
            format!(
                "the artifact {} is no longer the file that was executed: {changed}. The capture \
                 is discarded rather than compared, because the bytes cannot be attributed to a \
                 program this run can name",
                shown_path(artifact)
            ),
        ));
    }
    Ok(RunAttempt::Ran(Box::new(outcome)))
}

/// An artifact an execution is bound to: the path it is named by, and the file it must still be.
///
/// Passed to [`execute_bounded`] so the identity can be re-checked in the instant before the
/// spawn. `None` for a prepared command, where the program is a tool `env.rs` already vetted and
/// bound to a device and inode of its own.
struct BoundArtifact<'a> {
    path: &'a Path,
    identity: &'a ArtifactIdentity,
}

/// The identity of a file, recorded so that what executes can be tied to what was inspected.
///
/// Device and inode name the file itself rather than a path to it, which is what makes a
/// replacement of the leaf visible. The link count is recorded because a second link is a second
/// name by which the contents could be exchanged, and the change time because it moves whenever
/// the inode is written — so an in-place rewrite that preserved the device, the inode and even the
/// length is still caught.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactIdentity {
    device: u64,
    inode: u64,
    links: u64,
    length: u64,
    changed: (i64, i64),
}

impl ArtifactIdentity {
    /// Record the identity of `path`, refusing anything that cannot be bound to.
    ///
    /// # Errors
    ///
    /// The file could not be inspected without following a link, is not a regular file, or has
    /// more than one link to it.
    fn of(context: &str, path: &Path) -> HarnessResult<ArtifactIdentity> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            HarnessError::new(
                String::from(context),
                format!(
                    "the identity of {} could not be recorded: {error}; an execution is bound to \
                     the file that was inspected, so a file whose identity cannot be read cannot \
                     be executed",
                    shown_path(path)
                ),
            )
        })?;
        if !metadata.file_type().is_file() {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "{} is not a regular file at the moment its identity was recorded; only a \
                     regular file can be bound to and executed",
                    shown_path(path)
                ),
            ));
        }
        let identity = ArtifactIdentity::from_metadata(&metadata);
        if identity.links != 1 {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "{} has {} hard links, so the same contents are reachable under another name \
                     that this run does not control; a second name is a second opportunity to \
                     exchange the file between the check and the execution",
                    shown_path(path),
                    identity.links
                ),
            ));
        }
        Ok(identity)
    }

    /// Read an identity out of metadata already gathered without following a link.
    fn from_metadata(metadata: &Metadata) -> ArtifactIdentity {
        ArtifactIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            length: metadata.len(),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }

    /// How `path` now differs from what was recorded, in words, or `None` when it is unchanged.
    ///
    /// A path that cannot be inspected at all is a difference rather than an absence of one: a
    /// file that vanished is precisely the case this check exists to catch.
    fn changed_since(&self, path: &Path) -> Option<String> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Some(format!(
                    "it can no longer be inspected without following a link ({error})"
                ))
            }
        };
        if !metadata.file_type().is_file() {
            return Some(String::from("it is no longer a regular file"));
        }
        let now = ArtifactIdentity::from_metadata(&metadata);
        if now == *self {
            return None;
        }
        Some(format!(
            "the file recorded as device {} inode {} with {} link, {} bytes and change time {}.{} \
             now reads as device {} inode {} with {} link(s), {} bytes and change time {}.{}",
            self.device,
            self.inode,
            self.links,
            self.length,
            self.changed.0,
            self.changed.1,
            now.device,
            now.inode,
            now.links,
            now.length,
            now.changed.0,
            now.changed.1
        ))
    }
}

/// [`run`], and then write the capture into the workspace under `names`.
///
/// The convenience that makes a retained failure directory self-sufficient without every caller
/// having to remember the second step. Nothing is written for an unavailable runner, because
/// nothing was captured; that condition belongs to the run summary, which reports it explicitly.
///
/// # Errors
///
/// Everything [`run`] can fail on, plus a workspace write that could not be performed.
pub fn run_and_record(
    artifact: &Path,
    key: &CellKey,
    workspace: &Workspace,
    caps: &Capabilities,
    names: &CaptureNames,
) -> HarnessResult<RunAttempt> {
    let attempt = run(artifact, key, workspace, caps)?;
    if let RunAttempt::Ran(outcome) = &attempt {
        outcome.persist(workspace, names)?;
    }
    Ok(attempt)
}

/// Run any prepared command under a bounded wait, capturing both streams.
///
/// The shared timed-wait facility, and the only one. `flagprobe.rs` and `ubaudit.rs` both need to
/// spawn a process, bound it, capture its streams and read its status, and both go through this
/// single implementation: process handling written once cannot acquire two different timeout
/// bugs. `compile.rs` keeps its own capped variant deliberately, because a compiler driver's
/// diagnostics are bounded in size while a corpus program's stdout must be captured byte for byte
/// with no cap at all — the two requirements are opposite, and one function cannot honour both.
///
/// The caller prepares the command — program, arguments, working directory, and any environment
/// change it needs — and this function supplies only the parts that must not vary: standard
/// input from the null device, both output streams piped and drained concurrently, a bounded
/// wait, and a forcible kill on expiry. Any standard-stream configuration the caller set is
/// replaced, because an inherited stream would both pollute the runner's own output and destroy
/// byte-exactness.
///
/// Capture is bounded in both dimensions, which is what makes this safe to point at a program the
/// compiler under test may have miscompiled. Time is bounded by the budget; bytes are bounded by
/// [`CAPTURE_STREAM_BYTES_MAX`] per stream, past which the reader keeps draining and discards the
/// excess
/// so that neither memory growth nor a full pipe can hang the run. A stream that exceeds the cap
/// without the cell timing out is a hard error rather than a comparison, because the bytes left
/// are a prefix of the program's output and comparing a prefix would be reported as a divergence
/// in the compiler.
///
/// This function knows nothing about oracles, targets or verdicts. It spawns, bounds, captures
/// and returns.
///
/// # The timeout utility is a parameter, not a second entry point
///
/// `timeout_tool` is the path discovery vetted, normally `caps.timeout_tool().path()`. When it is
/// `Some` and the budget is a whole number of seconds it wraps the launch as an outer net beyond
/// the budget; when it is `None` nothing wraps the launch. Either way this module's own watchdog
/// is what enforces the budget, so the observable behaviour is identical and the utility is
/// genuinely optional.
///
/// There is deliberately no shorter spelling that omits the argument. A caller that passed nothing
/// would silently lose the outer net while the capability report stated that the utility was
/// discovered and is in use, so requiring the argument makes that choice visible at every call
/// site, and a caller with genuinely nothing to pass says so by passing `None`.
///
/// `private_dir` asks for an **isolated environment**: every variable cleared, a documented
/// minimal set restored, and that directory used as the child's private home and temporary
/// directory. `None` leaves the environment exactly as the caller prepared it.
///
/// A caller must **not** clear the environment itself and then pass `None`. The request has to be
/// made here because a clear is the one property of a prepared command that
/// [`Command::get_envs`] cannot report, so the outer-net wrapper — which rebuilds the command
/// around the utility — has no way to reproduce it. A command cleared by its caller and then
/// wrapped would silently inherit the whole test-process environment again, which is precisely
/// the exposure the isolation exists to prevent.
///
/// # Errors
///
/// The child could not be spawned or reaped, an argument could not be represented as text, the
/// termination was reported as neither an exit nor a signal, a stream reached its retention quota
/// on a cell that did not time out, or a stream could not be drained after a child that ended
/// normally.
pub fn run_command_captured_with(
    command: Command,
    budget: Duration,
    timeout_tool: Option<&Path>,
    private_dir: Option<&Path>,
) -> HarnessResult<RunOutcome> {
    execute_bounded(command, budget, None, timeout_tool, None, private_dir)
}

/// Resolve how an artifact for `target` must be launched.
///
/// `Ok(None)` means it runs directly, `Ok(Some(runner))` means it runs under that emulator, and
/// `Err` means it cannot be run here at all. The three cases are kept distinct so that "no
/// runner needed" and "no runner available" can never be confused — handing a foreign binary to
/// the host would produce a non-zero status indistinguishable, to a comparison that reads only
/// status and stdout, from a program the compiler under test miscompiled.
///
/// The emulator is always the one `env.rs` discovered and vetted. No emulator name is spelled
/// here: the i686 target's runner is named after `i386`, both plain and statically linked
/// spellings are legitimate, and an override may point at either, so a name written into this
/// module would eventually disagree with the one that was actually checked.
fn resolve_runner(
    target: Target,
    caps: &Capabilities,
) -> Result<Option<PathBuf>, RunnerUnavailable> {
    if target.is_native() {
        return Ok(None);
    }
    match caps.runner_for(target) {
        Some(runner) => Ok(Some(runner.to_path_buf())),
        None => Err(RunnerUnavailable {
            target,
            detail: sanitize_text_for_report(&format!(
                "artifacts cannot be executed on this {host} host because no execution \
                 runner for the target was discovered, so nothing was launched. This is reported \
                 as an unavailable oracle arm rather than as a passing or failing cell: every \
                 oracle compares the behaviour of a program that ran, so a target that cannot run \
                 has nothing to compare. Install the emulator for this architecture, or point the \
                 target's runner override at one; the capability report names the variable and the \
                 candidates that were tried. Under the strict setting intended for continuous \
                 integration, where the toolchain is installed deliberately, this escalates to a \
                 failure",
                host = caps.host_arch()
            )),
        }),
    }
}

/// Launch a prepared command, drain both of its streams concurrently, bound its lifetime, and
/// record what happened.
///
/// The single implementation behind every public entry point of this module, which is why the
/// suite has one supervision arrangement rather than four.
///
/// The sequence is deliberate, and every step of it exists because of a way the previous
/// arrangement could be defeated:
///
/// 1. The child is given a **process group of its own** before it is spawned, so the whole tree
///    can be signalled later rather than only the process this module holds a handle to.
/// 2. When an artifact is bound to this execution, its **identity is re-checked immediately
///    before the spawn** so a replacement of the leaf during runner resolution and command
///    assembly cannot be executed, and **every vetted tool named in the launch vector is
///    re-confirmed** in the same breath — the last two statements before the spawn — so an
///    emulator or timeout utility exchanged since pre-flight is refused rather than run. The
///    vector is scanned rather than just its first element, because wrapping moves the emulator
///    out of the front position and puts the utility there.
/// 3. The streams are drained by threads that start before the wait does, so a child that fills a
///    pipe buffer cannot deadlock the harness. Each buffer is **bounded**, so a child that prints
///    without end cannot exhaust the harness either.
/// 4. The child is polled to completion and killed once the budget elapses or a capture reaches
///    its ceiling. The budget is enforced **here**, never inferred from an exit status.
/// 5. The whole group is swept and verified empty on **every** path, the ordinary one included.
/// 6. Only then are the readers awaited: a reaped child whose group is empty has no writer left,
///    so the pipes are closed and the reads finish at once.
/// 7. A capture that reached its ceiling, and a capture that could not be drained after a child
///    that ended on its own, are both **refused** rather than compared.
fn execute_bounded(
    command: Command,
    budget: Duration,
    runner: Option<PathBuf>,
    timeout_tool: Option<&Path>,
    artifact: Option<BoundArtifact<'_>>,
    private_dir: Option<&Path>,
) -> HarnessResult<RunOutcome> {
    // The program's own line is taken before any wrapping, because that is the line a finding
    // artifact publishes and a maintainer re-runs. The wrapper, when there is one, is recorded
    // separately as the launch line rather than folded into it.
    let mut command = command;
    let argv = argv_of(&command)?;
    let working_dir = command.get_current_dir().map(Path::to_path_buf);

    // Before the wrapping, so that the entries the isolation sets are visible to the wrapper as
    // ordinary explicit entries and are copied across with everything else. The *clear* is the one
    // thing the wrapper cannot see, which is why it is told about it separately below.
    if let Some(directory) = private_dir {
        isolate_child_environment(&mut command, directory);
    }

    // The watchdog below is the authoritative bound and fires at the budget exactly. The external
    // utility, when there is one and the budget is a whole number of seconds, is wrapped around
    // the launch as an outer net a fixed margin further out. Its exit status is never read as
    // evidence of anything, which is what removes the ambiguity the previous arrangement had: the
    // utility reports expiry as a status inside the corpus's own contract range.
    let (mut command, enforcement) = match (timeout_tool, whole_second_budget(budget)) {
        (Some(tool), Some(seconds)) => (
            wrap_in_timeout_utility(
                &command,
                tool,
                seconds.saturating_add(TIMEOUT_UTILITY_OUTER_MARGIN.as_secs()),
                private_dir.is_some(),
            ),
            TimeoutEnforcement::WatchdogWithOuterNet,
        ),
        _ => (command, TimeoutEnforcement::Watchdog),
    };

    // Every diagnostic below is phrased in terms of what was literally spawned, so a failure to
    // launch names the command that failed rather than the one it was standing in for.
    let launch_argv = argv_of(&command)?;

    // Fixed regardless of what the caller configured. Standard input comes from the null device
    // so a program can never block waiting for input; both output streams are piped so that
    // nothing is inherited, which would pollute the runner's own output and destroy the
    // byte-exactness every oracle depends on.
    //
    // The child is also placed in a process group of its own. That is what makes the whole subtree
    // terminable: an emulator, or the external `timeout` utility, forks the program whose budget is
    // being enforced, and a group of one's own means the negated child identifier addresses the
    // program too. It also isolates the child from any signal delivered to the test process's own
    // group, so a cell cannot be killed by something aimed at the runner.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A group of its own, so that terminating this execution can mean terminating everything it
    // started. On the outer-net path the process this module launches is the utility, so without
    // this a program that ignored a catchable signal would simply outlive it.
    own_process_group(&mut command);

    // The last two statements before the spawn, deliberately. Anything between either check and the
    // launch is a window in which a file could be exchanged, so there is nothing between them.
    //
    // They ask the same question of two different things. The artifact is a product of this run and
    // is compared against the identity taken when this function's caller inspected it; the tools are
    // the machine's, and are compared against the identities discovery vetted. Neither substitutes
    // for the other: a replaced artifact makes the *program* unattributable, while a replaced
    // emulator or timeout utility makes the whole *observation* unattributable.
    if let Some(bound) = &artifact {
        if let Some(changed) = bound.identity.changed_since(bound.path) {
            return Err(HarnessError::new(
                format!("launching {}", posix_command_line(&launch_argv)),
                format!(
                    "the artifact {} is not the file that was inspected: {changed}. Nothing was \
                     launched, because an execution this run cannot attribute to a known file \
                     cannot produce evidence about a compiler",
                    shown_path(bound.path)
                ),
            ));
        }
    }
    // The launch vector rather than the one the caller prepared, so the external timeout utility is
    // covered on the machines that have it: after wrapping it is the program at the front, and the
    // emulator it wraps is still in the vector and still checked in the same pass.
    if let Some(change) = confirm_vetted_tools_unchanged(&launch_argv) {
        return Err(HarnessError::new(
            format!("launching {}", posix_command_line(&launch_argv)),
            format!(
                "a tool this launch is about to execute is no longer the file discovery vetted — \
                 {change}. Nothing was launched. The vetting that admitted the tool describes a \
                 file that is no longer there, so neither a passing nor a failing cell could be \
                 attributed to the toolchain the run reports having used"
            ),
        ));
    }

    let started = Instant::now();
    let mut child = command.spawn().map_err(|error| {
        HarnessError::new(
            format!("launching {}", posix_command_line(&launch_argv)),
            format!(
                "the process could not be spawned: {error}; working directory {}",
                describe_working_dir(working_dir.as_deref())
            ),
        )
    })?;
    // The group identifier equals the leader's own process identifier, which is this child, so no
    // extra bookkeeping is needed to name the group on any later path.
    let group = child.id();

    let stdout_capture = match child.stdout.take() {
        Some(pipe) => spawn_reader(pipe),
        None => {
            return Err(missing_pipe(
                &mut child,
                group,
                &launch_argv,
                "standard output",
            ))
        }
    };
    let stderr_capture = match child.stderr.take() {
        Some(pipe) => spawn_reader(pipe),
        None => {
            return Err(missing_pipe(
                &mut child,
                group,
                &launch_argv,
                "standard error",
            ))
        }
    };

    let waited = wait_bounded(
        &mut child,
        group,
        budget,
        started,
        &launch_argv,
        &[
            (STDOUT_ROLE, &stdout_capture),
            (STDERR_ROLE, &stderr_capture),
        ],
    )?;
    let duration = started.elapsed();

    // On every path, including the ordinary one. The immediate child has been reaped by now, so a
    // member still in the group is a descendant that outlived it, and the verification inside is
    // what turns that from an invisible leak into a recorded fact.
    let swept = terminate_process_group(group, kill_tool());

    let drain_deadline = Instant::now() + CAPTURE_DRAIN_GRACE;
    let stdout_stream = drain(stdout_capture, STDOUT_ROLE, drain_deadline);
    let stderr_stream = drain(stderr_capture, STDERR_ROLE, drain_deadline);

    let termination = termination_of(waited.status, &waited.cause, &launch_argv)?;

    // Refused before anything is derived from the bytes, unless the cell already timed out — whose
    // stdout no oracle compares, so a capture cut short costs nothing there and is recorded as a
    // note rather than costing the run the timeout it observed. The check is on the drained streams
    // rather than on the stop cause, because a stream can reach its quota in the same instant the
    // child exits on its own, and the bytes are just as incomplete either way.
    if !termination.timed_out()
        && (stdout_stream.integrity.truncated() || stderr_stream.integrity.truncated())
    {
        return Err(output_ceiling_refusal(
            &launch_argv,
            working_dir.as_deref(),
            &waited.cause,
            &stdout_stream,
            &stderr_stream,
        ));
    }

    let mut notes = Vec::new();
    // A leaked descendant and an environment that cannot signal a group are both facts about this
    // execution, not tidiness: a survivor can hold the capture pipes a verdict is computed from.
    match &swept {
        GroupTermination::Cleared => {}
        GroupTermination::Survivors(detail) | GroupTermination::Unsupervised(detail) => {
            push_note(&mut notes, detail.clone())
        }
    }
    for note in [stdout_stream.note.clone(), stderr_stream.note.clone()]
        .into_iter()
        .flatten()
    {
        push_note(&mut notes, note);
    }

    // Recorded rather than assumed, because the retained bytes are bounded and the drain is
    // bounded, so "these are the bytes" is a claim that has to be substantiated. The two gates
    // below are what make an incomplete capture unusable rather than merely annotated: a stream
    // that reached the retention quota is refused unless the cell already timed out, and a reader
    // that never finished is refused on every path but a timeout.
    let stdout_integrity = stdout_stream.integrity;
    let stderr_integrity = stderr_stream.integrity;

    let outcome = RunOutcome {
        argv,
        launch_argv,
        working_dir,
        runner,
        termination,
        // The raw wait status travels with the outcome so that a comparison can distinguish a
        // signal death from a numerically equal ordinary exit.
        raw_wait_status: waited.status.into_raw(),
        stdout: stdout_stream.bytes,
        stderr: stderr_stream.bytes,
        stdout_integrity,
        stderr_integrity,
        duration,
        budget,
        enforcement,
        notes,
    };

    // A capture that could not be drained is tolerable only for a cell already known to have
    // timed out, whose stdout no oracle compares. After a child that ended on its own, a
    // truncated capture would be compared byte for byte and reported as a compiler divergence,
    // which is the one failure that would leave no trace of its real cause — so it is a hard
    // failure instead.
    let capture_complete = stdout_stream.integrity.drained() && stderr_stream.integrity.drained();
    if !(outcome.timed_out() || capture_complete) {
        return Err(HarnessError::new(
            format!(
                "capturing the output of {} in {}",
                outcome.launch_command_line(),
                describe_working_dir(outcome.working_dir())
            ),
            format!(
                "the child {} but its captured streams could not be drained within {} ms, so the \
                 capture is incomplete: some process that inherited the pipes is still holding \
                 them open. A truncated capture is refused rather than compared, because it would \
                 be reported as a divergence in the compiler under test{}",
                outcome.termination(),
                CAPTURE_DRAIN_GRACE.as_millis(),
                match outcome.notes() {
                    [] => String::new(),
                    notes => format!(". Recorded notes: {}", notes.join("; ")),
                }
            ),
        ));
    }

    Ok(outcome)
}

/// The refusal an execution earns by filling a capture buffer to its ceiling.
///
/// Named, and separate from every other failure this module can report, because the condition is
/// specific and the remedy is specific: a program printed more than any corpus program legitimately
/// prints, so the bytes held are a prefix rather than the output. Comparing a prefix is the one
/// mistake that could turn a genuine divergence into a pass, since two flooding sides truncated at
/// the same ceiling compare equal.
fn output_ceiling_refusal(
    launch_argv: &[String],
    working_dir: Option<&Path>,
    cause: &StopCause,
    stdout_stream: &DrainedStream,
    stderr_stream: &DrainedStream,
) -> HarnessError {
    let mut which: Vec<&str> = Vec::new();
    if stdout_stream.integrity.truncated() {
        which.push(STDOUT_ROLE);
    }
    if stderr_stream.integrity.truncated() {
        which.push(STDERR_ROLE);
    }
    let stopped = match cause {
        StopCause::OutputCeiling(role) => format!(
            "the child was terminated, with its whole process group, as soon as its {role} capture \
             reached the ceiling"
        ),
        StopCause::Budget => String::from(
            "the child had already been terminated at its budget when the ceiling was reached",
        ),
        StopCause::Exited => String::from(
            "the child ended on its own in the same moment, so it was not terminated here",
        ),
    };
    HarnessError::new(
        format!(
            "capturing the output of {} in {}",
            posix_command_line(launch_argv),
            describe_working_dir(working_dir)
        ),
        format!(
            "the {} capture reached the per-stream ceiling of {CAPTURE_STREAM_BYTES_MAX} bytes, so \
             what was captured is a prefix of what the program wrote and not the output itself: \
             {stdout_bytes} bytes of standard output and {stderr_bytes} bytes of standard error \
             were held. {stopped}. The execution is refused rather than compared, because two \
             sides both truncated at this ceiling would compare equal and a real divergence would \
             be reported as a pass. No corpus program prints anything approaching this, so a \
             program that reaches it is either miscompiled into an unbounded loop or is not the \
             program the record describes",
            which.join(" and "),
            stdout_bytes = stdout_stream.bytes.len(),
            stderr_bytes = stderr_stream.bytes.len()
        ),
    )
}

/// The budget as a whole number of seconds, when it can be expressed as one.
///
/// `None` for a zero budget and for a fractional one, and in both cases the external utility is
/// not used. Passing a floor of zero would be actively wrong — in the utility family a limit of
/// zero seconds means "no limit at all", so a sub-second budget would silently become unbounded
/// — and rounding a fractional budget would enforce something other than what the caller asked
/// for. The watchdog honours any budget exactly, so declining the utility loses nothing.
///
/// A cell's budget always is a whole number of seconds: `env.rs` validates the setting as a
/// positive integer within a ceiling, so every corpus execution gets the outer net when a utility
/// was discovered. The seconds the utility is actually given are this value plus
/// [`TIMEOUT_UTILITY_OUTER_MARGIN`], because it stands behind this module's watchdog rather than
/// in front of it.
fn whole_second_budget(budget: Duration) -> Option<u64> {
    if budget.as_secs() == 0 || budget.subsec_nanos() != 0 {
        return None;
    }
    Some(budget.as_secs())
}

/// Rebuild `command` wrapped in the external timeout utility.
///
/// `seconds` is the caller's budget plus [`TIMEOUT_UTILITY_OUTER_MARGIN`]: the utility is the
/// **outer** net here, not the primary mechanism, so it is set to expire after this module's own
/// watchdog would already have acted.
///
/// The invocation is the plain portable form `timeout <secs> <program> <args...>`. Neither
/// `--signal=KILL` nor `--kill-after` is passed, even though an uncatchable kill is exactly what
/// one would want: on the implementation measured in this environment the first sends no signal
/// at all — it merely waits for the child and then reports expiry, and hung indefinitely against
/// a compute-bound child — and the second reports status 125, which lies inside the corpus's own
/// expected-exit-code range and would therefore be indistinguishable from a program's own return
/// value. Uncatchable termination is guaranteed by this module instead, which sends `SIGKILL`
/// through [`Child::kill`] and then sweeps the whole process group.
///
/// The working directory and every explicit environment change the caller made are carried over,
/// so wrapping changes how the child is bounded and nothing else about it. The standard streams
/// are not carried over because [`execute_bounded`] fixes them for every path.
///
/// `cleared` says whether the command being wrapped had its environment cleared. That cannot be
/// read back off a [`Command`] — [`Command::get_envs`] reports the explicit sets and removals and
/// nothing about a clear — so it has to be passed in. Getting this wrong is not a cosmetic
/// difference: a cleared command whose wrapper was not cleared inherits the entire test-process
/// environment, and every secret in it, behind a caller that believes it isolated the child. This
/// was observed rather than reasoned about, which is why the parameter exists.
fn wrap_in_timeout_utility(command: &Command, tool: &Path, seconds: u64, cleared: bool) -> Command {
    let mut wrapped = Command::new(tool);
    wrapped.arg(seconds.to_string());
    wrapped.arg(command.get_program());
    wrapped.args(command.get_args());
    if let Some(directory) = command.get_current_dir() {
        wrapped.current_dir(directory);
    }
    if cleared {
        wrapped.env_clear();
    }
    for (name, value) in command.get_envs() {
        match value {
            Some(value) => {
                wrapped.env(name, value);
            }
            None => {
                wrapped.env_remove(name);
            }
        }
    }
    wrapped
}

/// The argument vector of a prepared command, as text.
///
/// Recorded exactly rather than approximately: the vector is what a report row shows and what a
/// reproduction command re-runs, so an argument that cannot be represented as text is a hard
/// failure rather than something to render approximately and hope nobody re-runs.
fn argv_of(command: &Command) -> HarnessResult<Vec<String>> {
    let mut argv = Vec::with_capacity(1 + command.get_args().len());
    argv.push(command_text("the program", command.get_program())?);
    for argument in command.get_args() {
        argv.push(command_text("an argument", argument)?);
    }
    Ok(argv)
}

/// One element of an argument vector as text, or an explanatory failure.
///
/// The diagnostic renders the offending value approximately — the only lossy conversion in this
/// module, and confined to a human-readable message. Nothing that is compared, and nothing that
/// is re-run, is ever produced this way.
fn command_text(role: &str, value: &OsStr) -> HarnessResult<String> {
    value.to_str().map(String::from).ok_or_else(|| {
        HarnessError::new(
            format!("recording {role} of a command line"),
            format!(
                "the value {:?} is not valid text, so the command could not be recorded exactly; \
                 every path this suite executes is derived from the package manifest directory and \
                 the corpus, both of which are plain text",
                value.to_string_lossy()
            ),
        )
    })
}

/// Render a working directory for a diagnostic, naming the inherited case explicitly.
fn describe_working_dir(directory: Option<&Path>) -> String {
    match directory {
        Some(directory) => shown_path(directory),
        None => String::from("(inherited from the test process)"),
    }
}

/// Terminate and reap a child that cannot be used, and describe why.
///
/// Called only when a pipe the command asked for is absent, which the standard library does not
/// do for a piped stream. The child and its whole process group are still killed and reaped rather
/// than abandoned: 1,296 cells run concurrently, and a process left behind by each would accumulate.
fn missing_pipe(child: &mut Child, group: u32, argv: &[String], stream: &str) -> HarnessError {
    let cleanup = abandon(child, group);
    HarnessError::new(
        format!("capturing the output of {}", posix_command_line(argv)),
        format!(
            "the child's {stream} pipe was not available even though the stream was configured as \
             a pipe, so the capture could not be started; the child's process group was signalled \
             and the child was reaped rather than left running{}",
            match cleanup {
                Some(note) => format!(". {note}"),
                None => String::new(),
            }
        ),
    )
}

/// Kill and reap a child whose outcome is no longer wanted, sweep its group, and say so when
/// cleanup fell short.
///
/// Used on every error path that abandons a spawned child. Two properties hold unconditionally,
/// and both matter at the scale of the full matrix: the direct child is **always** reaped, so no
/// zombie accumulates across 1,296 cells, and its process group is then swept, so a compiler
/// driver's or an emulator's sub-processes go with it.
///
/// The group is swept **after** the child has been reaped rather than before, deliberately:
/// sweeping first would find this module's own child still in the group and report it as a leaked
/// descendant.
///
/// The returned note is `None` when every step worked. It is deliberately a return value rather
/// than a discarded result: a cleanup that did not work leaves a process running on the machine,
/// which is a fact about this run that a caller has to be able to put in its diagnostic. What it
/// must never do is *replace* the caller's diagnostic, which is why it is a note and not an error.
fn abandon(child: &mut Child, group: u32) -> Option<String> {
    // A kill that fails because the child has already exited is the ordinary case here, and the
    // reap that follows proves it: only a reap that also failed leaves anything behind.
    let _ = child.kill();
    let reaped = child.wait();
    let swept = terminate_process_group(group, kill_tool());

    let mut details = Vec::new();
    if let Err(error) = reaped {
        details.push(format!(
            "the child could not be reaped: {error}, so it may remain as a zombie"
        ));
    }
    match &swept {
        GroupTermination::Cleared => {}
        GroupTermination::Survivors(detail) | GroupTermination::Unsupervised(detail) => {
            details.push(detail.clone())
        }
    }
    if details.is_empty() {
        return None;
    }
    Some(sanitize_text_for_report(&format!(
        "cleanup after abandoning the child was incomplete; {}",
        details.join("; ")
    )))
}

/// Append a note, redacted and made safe to render on a single line first.
///
/// Notes reach a status record, where each occupies one `note = ...` line, and a report row,
/// where an embedded line feed would split one row into two and a tab would forge a column. The
/// text they are built from includes messages from the operating system, so escaping at the point
/// of collection is what makes the guarantee hold for every consumer.
///
/// Redaction precedes escaping for the same reason it does in `env.rs`: a note is persisted into a
/// workspace and rendered into a report, and an operating-system message can quote text it was
/// handed. Redacting at the single point of collection is what makes the guarantee hold without
/// every producer of a note having to remember it.
fn push_note(notes: &mut Vec<String>, note: String) {
    notes.push(sanitize_text_for_report(&redact_secrets(&note)));
}

/// The name this module uses for the standard output stream in a note and a diagnostic.
const STDOUT_ROLE: &str = "standard output";

/// The name this module uses for the standard error stream in a note and a diagnostic.
const STDERR_ROLE: &str = "standard error";

/// What one reader thread and its caller share.
///
/// The buffer is shared rather than owned by the thread so that whatever has arrived can be
/// taken at any moment — including from a reader that is still blocked because some process
/// inherited the pipe. That is what turns a forcibly killed child's partial output into evidence
/// instead of nothing.
///
/// The two counters beside it are what make the retention quota honest. Without them a caller
/// could see four megabytes and have no way to tell whether that was the whole stream or the
/// first four megabytes of forty, and a truncated standard output compared byte for byte would be
/// reported as a divergence in the compiler under test.
struct CaptureState {
    /// Everything retained so far, bounded by [`CAPTURE_RETAINED_BYTES_MAX`].
    buffer: Mutex<Vec<u8>>,
    /// Every byte the child wrote into the pipe, including any the quota discarded.
    produced: AtomicU64,
    /// Set once the quota has discarded anything at all.
    truncated: AtomicBool,
}

/// A pipe being drained on its own thread.
struct StreamCapture {
    state: Arc<CaptureState>,
    /// Sends exactly once when the reader stops: `None` for end-of-file, `Some(message)` for a
    /// read error or for the ceiling. A bounded receive on this is what keeps a stuck pipe from
    /// blocking the harness forever, which a plain thread join could not.
    finished: Receiver<Option<String>>,
}

impl StreamCapture {
    /// Whether the reader has dropped bytes because the ceiling was reached.
    /// Whether the retention quota has already been exceeded.
    ///
    /// Polled by [`wait_bounded`] on every wake-up, which is what makes the quota a bound on the
    /// *execution* and not merely on the buffer: a program that has already produced more than can
    /// be kept is terminated at that moment rather than left to print for the rest of its budget
    /// into bytes nothing may compare.
    fn reached_ceiling(&self) -> bool {
        self.state.truncated.load(Ordering::Relaxed)
    }
}

/// Start draining one pipe on its own thread, into a buffer bounded at
/// [`CAPTURE_STREAM_BYTES_MAX`].
///
/// Both streams are drained concurrently and from the moment the child starts, because a child
/// that fills a pipe buffer while its parent waits for it to exit would deadlock — and every
/// corpus program prints one line per property it asserts, so the buffer genuinely does fill.
///
/// Bytes are appended exactly as they arrive. Nothing decodes them, normalizes a line ending or
/// trims anything: comparison is byte-exact, so a conversion here would silently mask a real
/// divergence.
///
/// Retention is bounded at [`CAPTURE_RETAINED_BYTES_MAX`], and the pipe keeps being read past that
/// bound. Both halves matter and for different reasons: without the bound, a program that prints
/// without end exhausts memory across 1,296 concurrent cells; without the continued reading, the
/// child blocks forever on a full pipe and the bound would have converted a memory fault into a
/// hang. What the quota discards is counted, so the overflow is disclosed rather than absorbed.
fn spawn_reader<R: Read + Send + 'static>(mut source: R) -> StreamCapture {
    let state = Arc::new(CaptureState {
        buffer: Mutex::new(Vec::new()),
        produced: AtomicU64::new(0),
        truncated: AtomicBool::new(false),
    });
    let sink = Arc::clone(&state);
    let (report, finished) = mpsc::channel();
    thread::spawn(move || {
        let stopped_because = pump(&mut source, &sink);
        // A closed receiver means the execution already concluded without this reader, which the
        // drain records as an incomplete stream. There is nothing further to do about it here.
        let _ = report.send(stopped_because);
    });
    StreamCapture { state, finished }
}

/// Copy every byte one stream produces into the shared state, up to the retention quota, reporting
/// why the copy stopped.
///
/// `None` means the stream reached end of file, which is the ordinary ending and the only one
/// that proves a capture is complete. `Some(reason)` carries an explanation that travels into the
/// outcome's notes, so a truncated capture is always visible rather than being mistaken for a
/// program that simply printed less.
///
/// The ceiling is applied to the *total held*, and the flag is raised only when bytes were
/// actually dropped. A stream that produced exactly [`CAPTURE_STREAM_BYTES_MAX`] bytes and then
/// ended is therefore captured whole and reported as complete, which keeps the bound from
/// manufacturing a refusal out of an exact fit.
///
/// Reads are retried on interruption, because the standard library leaves that retry to the
/// caller and abandoning the stream there would silently truncate the very bytes under
/// comparison. Nothing else about the bytes is touched.
fn pump<R: Read>(source: &mut R, sink: &CaptureState) -> Option<String> {
    let mut chunk = [0_u8; CAPTURE_CHUNK_BYTES];
    loop {
        match source.read(&mut chunk) {
            Ok(0) => return None,
            Ok(count) => {
                sink.produced.fetch_add(count as u64, Ordering::Relaxed);
                match sink.buffer.lock() {
                    Ok(mut held) => {
                        let room = CAPTURE_RETAINED_BYTES_MAX.saturating_sub(held.len() as u64);
                        // `room` is bounded by the quota, so the cast back cannot lose a value
                        // that matters: the minimum with `count` is at most one chunk.
                        let take = room.min(count as u64) as usize;
                        held.extend_from_slice(&chunk[..take]);
                        if take < count {
                            sink.truncated.store(true, Ordering::Relaxed);
                        }
                    }
                    Err(_) => return Some(String::from(POISONED_CAPTURE_BUFFER)),
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Some(error.to_string()),
        }
    }
}

/// One drained stream: the bytes, how faithful they are, and any note about it.
struct DrainedStream {
    bytes: Vec<u8>,
    integrity: CaptureIntegrity,
    note: Option<String>,
}

/// Collect one stream, waiting no longer than `deadline` for its reader to finish.
///
/// The reader normally finishes immediately: the child has already been reaped and its group
/// swept by the time this is called, so the pipe's write ends are closed and the read returns
/// end-of-file. The bound exists for the one case where they are not — a process that inherited
/// the pipes and survived an uncatchable signal — where waiting forever would hang the whole run.
///
/// Two distinct incompletenesses are reported separately, because they have different causes and
/// different remedies: a reader abandoned at the deadline means a process is still holding the
/// pipe, while a stream the quota truncated means the program produced more than any corpus
/// program should.
fn drain(capture: StreamCapture, role: &str, deadline: Instant) -> DrainedStream {
    let remaining = deadline.saturating_duration_since(Instant::now());
    let (drained, mut note) = match capture.finished.recv_timeout(remaining) {
        Ok(None) => (true, None),
        Ok(Some(message)) => (
            true,
            Some(format!(
                "the {role} pipe reported a read error and the capture may be short: {message}"
            )),
        ),
        Err(RecvTimeoutError::Timeout) => (
            false,
            Some(format!(
                "the {role} pipe was still open {} ms after the child was reaped and its group \
                 swept, so the capture holds only what had arrived by then; a process that \
                 inherited the pipe survived an uncatchable signal",
                CAPTURE_DRAIN_GRACE.as_millis()
            )),
        ),
        Err(RecvTimeoutError::Disconnected) => (
            true,
            Some(format!(
                "the {role} reader ended without reporting why, so the capture holds only what it \
                 had already delivered"
            )),
        ),
    };

    let bytes = take_buffer(&capture.state);
    let produced = capture.state.produced.load(Ordering::Relaxed);
    let truncated = capture.state.truncated.load(Ordering::Relaxed);
    if truncated {
        // Appended rather than substituted, so a stream that was both undrained and truncated
        // reports both facts instead of only the one that was detected first.
        let quota_note = format!(
            "the {role} capture reached the {CAPTURE_RETAINED_BYTES_MAX}-byte retention quota \
             after the program had produced {produced} byte(s), so {} byte(s) were retained and \
             the remainder was drained and discarded; the stream is not the whole stream and is \
             never compared as though it were",
            bytes.len()
        );
        note = Some(match note {
            Some(existing) => format!("{existing}; {quota_note}"),
            None => quota_note,
        });
    }

    DrainedStream {
        integrity: CaptureIntegrity::new(produced, bytes.len() as u64, truncated, drained),
        bytes,
        note,
    }
}

/// Take everything a capture buffer holds, recovering the bytes even from a poisoned lock.
///
/// A poisoned lock means a reader thread panicked, which loses nothing already appended — so the
/// bytes are recovered rather than discarded, and rather than being turned into a panic of this
/// thread by unwrapping. Losing a capture would turn a diagnosable divergence into an
/// unexplained one.
fn take_buffer(state: &Arc<CaptureState>) -> Vec<u8> {
    match state.buffer.lock() {
        Ok(mut held) => std::mem::take(&mut *held),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    }
}

/// Why the bounded wait stopped waiting.
///
/// Replaces the boolean it grew out of, because there are now two reasons this module terminates a
/// child and they must not be conflated: one is a divergence class the suite reports, the other is
/// a refusal to compare bytes that are only a prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StopCause {
    /// The child ended on its own and was reaped by the poll that observed it.
    Exited,
    /// The budget elapsed and this module terminated the child and swept its group.
    Budget,
    /// A capture reached its ceiling and this module terminated the child and swept its group.
    /// Carries the stream that triggered it, for the diagnostic.
    OutputCeiling(String),
}

/// The result of the bounded wait: the status, and why the waiting stopped.
struct BoundedWait {
    status: ExitStatus,
    cause: StopCause,
}

/// Wait for a child, for no longer than `budget` measured from `started`, stopping early if a
/// capture reaches its ceiling.
///
/// Implemented by polling rather than by a thread that sleeps for the budget and then kills the
/// child, and the difference matters. A killer thread would need the child handle, so the handle
/// would have to be shared behind a lock — and the waiting thread holds that lock for as long as
/// it waits, so the killer could never acquire it. Polling keeps the handle in one thread, needs
/// no lock, and cannot race the reap.
///
/// The interval starts at a quarter of a millisecond and doubles to a ceiling of ten, so a cell
/// costing a few milliseconds is not delayed by the wait itself while a long one costs a
/// negligible number of wake-ups.
///
/// `captures` is the pair of streams with the names they are reported under. They are consulted on
/// every poll: a reader that has stopped at its ceiling is no longer draining, so a child left
/// running would block on a full pipe until the budget elapsed, and every one of those seconds
/// would be spent on an execution whose bytes are already known to be unusable.
///
/// # Errors
///
/// The child's status could not be read, or it could not be terminated. Both sweep the group
/// before returning, because a failure to report is not a licence to leak.
fn wait_bounded(
    child: &mut Child,
    group: u32,
    budget: Duration,
    started: Instant,
    argv: &[String],
    captures: &[(&str, &StreamCapture)],
) -> HarnessResult<BoundedWait> {
    let mut interval = POLL_INTERVAL_INITIAL;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(BoundedWait {
                    status,
                    cause: StopCause::Exited,
                })
            }
            Ok(None) => {}
            Err(error) => {
                // The wait failed, so this module can no longer learn how the child ended — but it
                // is still this module's child, and returning here without terminating it would
                // leave a process and an unreaped entry behind for every cell that took this path.
                // The cleanup is unconditional and its own shortcomings travel in the message,
                // which is why it cannot be a bare discarded kill.
                let cleanup = abandon(child, group);
                return Err(HarnessError::new(
                    format!("waiting for {}", posix_command_line(argv)),
                    format!(
                        "the child's status could not be read: {error}; the harness cannot report \
                         what a program did if it cannot learn how the program ended. The child's \
                         process group was signalled and the child was reaped before this failure \
                         was returned, so nothing is left running on its account{}",
                        match cleanup {
                            Some(note) => format!(". {note}"),
                            None => String::new(),
                        }
                    ),
                ));
            }
        }

        if let Some((role, _)) = captures
            .iter()
            .find(|(_, capture)| capture.reached_ceiling())
        {
            let status = stop_and_reap(
                child,
                group,
                argv,
                &format!(
                    "its {role} capture reached the ceiling of {CAPTURE_STREAM_BYTES_MAX} bytes"
                ),
            )?;
            return Ok(BoundedWait {
                status,
                cause: StopCause::OutputCeiling(String::from(*role)),
            });
        }

        if started.elapsed() >= budget {
            let status = stop_and_reap(
                child,
                group,
                argv,
                &format!("its budget of {} ms elapsed", budget.as_millis()),
            )?;
            return Ok(BoundedWait {
                status,
                cause: StopCause::Budget,
            });
        }

        thread::sleep(interval);
        interval = (interval * 2).min(POLL_INTERVAL_MAX);
    }
}

/// Terminate a child this module has decided to stop, and reap it.
///
/// [`Child::kill`] sends an uncatchable signal, so the subsequent wait returns promptly and no
/// process and no zombie is left behind — which matters at the scale of the full matrix, where
/// one leaked process per cell would accumulate into thousands. It is also the half that does not
/// depend on an external utility being present. The child's *descendants* are swept by the caller
/// once this has returned, deliberately in that order: sweeping first would find this child itself
/// still in the group and report it as a survivor.
///
/// A refused kill is only fatal if the child is demonstrably still running: a child that has
/// already exited but not yet been reaped is reported differently across platform versions, and
/// treating that as a failure would turn an ordinary race into a harness error.
///
/// `because` completes the sentence "terminated ... because ...", so every diagnostic below names
/// the reason this module intervened rather than leaving a reader to guess it.
fn stop_and_reap(
    child: &mut Child,
    group: u32,
    argv: &[String],
    because: &str,
) -> HarnessResult<ExitStatus> {
    if let Err(kill_error) = child.kill() {
        if let Ok(Some(status)) = child.try_wait() {
            return Ok(status);
        }
        let _ = terminate_process_group(group, kill_tool());
        return Err(HarnessError::new(
            format!("terminating {} because {because}", posix_command_line(argv)),
            format!(
                "the child could not be terminated: {kill_error}; it is still running, so the \
                 harness refuses to report a result it cannot bound"
            ),
        ));
    }
    match child.wait() {
        Ok(status) => Ok(status),
        Err(error) => {
            let _ = terminate_process_group(group, kill_tool());
            Err(HarnessError::new(
                format!(
                    "reaping {} after terminating it because {because}",
                    posix_command_line(argv)
                ),
                format!(
                    "the terminated child could not be reaped: {error}; a child left unreaped \
                     would accumulate across the matrix"
                ),
            ))
        }
    }
}

/// Decide how the child ended, from what this module did and from the wait status.
///
/// The order is the order of certainty, and there is now nothing in it that is inferred. A
/// termination this module performed at the budget is a fact it established, so it settles the
/// question first. A signal comes next, read through the safe platform accessor, because a
/// signalled child has no exit code at all and must never be reported as though it had one. Only
/// then is an exit code considered, and it is recorded exactly as observed.
///
/// **No exit status is interpreted.** The previous arrangement had to decide whether an exit of
/// 124 was a program's own answer or the external utility's report of expiry, and could only do so
/// by consulting a clock — a judgement that was wrong whenever a corpus program legitimately
/// returned that value promptly. The utility is now an outer net beyond this module's own bound,
/// so the question no longer arises.
///
/// A [`StopCause::OutputCeiling`] does not appear here as a termination: the caller refuses such an
/// execution outright rather than reporting a termination for it, because a prefix of a program's
/// output is not evidence about a compiler. The status is still read faithfully for the diagnostic.
fn termination_of(
    status: ExitStatus,
    cause: &StopCause,
    argv: &[String],
) -> HarnessResult<Termination> {
    if *cause == StopCause::Budget {
        return Ok(Termination::TimedOut);
    }
    if let Some(signal) = status.signal() {
        return Ok(Termination::Signalled(signal));
    }
    if let Some(code) = status.code() {
        return Ok(Termination::Exited(code));
    }
    let context = format!(
        "interpreting the termination of {}",
        posix_command_line(argv)
    );
    Err(HarnessError::new(
        context,
        format!(
            "the operating system reported the raw wait status {} as neither an exit nor a signal, \
             so the harness cannot say how the program ended; reporting a guess would put an \
             unfounded value into a comparison",
            status.into_raw()
        ),
    ))
}
