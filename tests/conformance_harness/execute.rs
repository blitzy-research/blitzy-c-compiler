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
//! # Two timeout mechanisms, one observable behaviour
//!
//! The standard library offers no timed wait on a child process, and the crate that does was
//! considered and rejected under the project's zero-dependency rule. Two substitutes are
//! implemented, and they are chosen in this order:
//!
//! 1. **The system `timeout` utility**, when discovery found one and the budget is a whole
//!    number of seconds. The invocation is the plain portable form `timeout <secs> <argv...>`,
//!    and expiry is recognised as exit status [`TIMEOUT_UTILITY_EXPIRY_STATUS`] corroborated
//!    by an elapsed time that reached the budget.
//! 2. **A watchdog owned by this module**, otherwise: the child is polled to completion and
//!    forcibly killed once the budget expires.
//!
//! The watchdog also runs as a **safety net underneath the utility**, at the budget plus
//! [`TIMEOUT_UTILITY_GRACE`]. That is not belt-and-braces decoration. The `timeout`
//! implementation installed in the environment this suite was developed against
//! (`timeout (uutils coreutils) 0.2.2`) was measured to be defective in two of its three
//! spellings:
//!
//! | invocation | observed status | observed elapsed | conclusion |
//! | --- | --- | --- | --- |
//! | `timeout 1 sleep 5` | 124 | 1.008 s | correct |
//! | `timeout -s KILL 1 sleep 5` | 124 | **5.008 s** | no signal is sent; it waits for the child and then reports expiry |
//! | `timeout -s KILL 1 <spin loop>` | — | **never returned** | hung indefinitely with the child still at 100% CPU |
//! | `timeout -k 1 1 <spin loop>` | **125** | 1.107 s | the child does die, but 125 means "the utility itself failed" and collides with the contract range |
//! | `timeout 1 <spin loop>` | 124 | 1.008 s | correct |
//!
//! So `--signal=KILL` and `--kill-after` are deliberately **not** passed, despite being the
//! obvious way to guarantee an uncatchable kill: on the measured implementation the first
//! sends no signal at all and the second reports a status inside the contract range.
//! Uncatchable termination is guaranteed instead by the safety net, which sends `SIGKILL`
//! through [`Child::kill`]. A net that fires is recorded in the outcome's notes, so a
//! maintainer can see that the external utility failed to enforce its own budget.
//!
//! A timeout is a **first-class divergence class**, never an infrastructure error: a program
//! that finishes promptly under one compiler and hangs under another is exactly the kind of
//! defect this suite exists to surface, so [`Termination::TimedOut`] maps to
//! [`DivergenceClass::Timeout`] and the verdict is left to `classify.rs`.
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
//! # Hermeticity and parallel safety
//!
//! Every child is launched with its current directory set to the cell's own workspace through
//! [`Command::current_dir`]. The process-wide working directory is never changed: the feature
//! area tests run concurrently in one process, so a `chdir` would be a data race rather than a
//! confinement. The artifact must be an absolute path to a regular file **inside** that
//! workspace, which is checked rather than assumed, so nothing outside the build directory can
//! be executed.
//!
//! Nothing is added to the child's environment and no argument is passed to the program: a
//! corpus program takes all of its input from literals in its own source, and injecting
//! anything would make the compared behaviour depend on the harness. Standard input is
//! redirected from the null device so a program can never block waiting for input, and neither
//! output stream is ever inherited, which would both pollute the runner's own output and
//! destroy byte-exactness.
//!
//! One thing this module cannot confine is named here rather than left as a surprise: when a
//! child dies from a signal, whether a core image is written and where it lands are decided by
//! the host's own `kernel.core_pattern` and core-size limit, not by the harness. An absolute
//! pattern therefore writes outside the workspace on a crash. Changing that would mean lowering
//! the child's core limit between the fork and the exec, which no safe standard-library API
//! offers, so the honest position is that this module itself writes nothing outside the
//! workspace and that a crash dump is a property of the machine the suite runs on.
//!
//! This module holds no global mutable state, registers nothing, and shares no path between
//! cells, so it needs no lock under `cargo test`'s default concurrency.
//!
//! Edition 2021, minimum supported Rust 1.70. Only the standard library is used, as the
//! project permits no third-party crate; this module is what stands in for a
//! timed-child-wait crate.

use std::ffi::OsStr;
use std::fmt;
use std::io::{self, Read};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::env::Capabilities;
use super::sandbox::{
    Workspace, BCC_EXIT_NAME, BCC_STDERR_NAME, BCC_STDOUT_NAME, REFERENCE_EXIT_NAME,
    REFERENCE_STDERR_NAME, REFERENCE_STDOUT_NAME,
};
use super::{
    ensure_within, posix_command_line, require_regular_file, sanitize_text_for_report,
    DivergenceClass, HarnessError, HarnessResult, Target,
};

/// The highest exit code a corpus program may be expected to return.
///
/// The operating system truncates a returned value into a byte, so a larger value does not
/// survive: `return 300` was measured to produce wait status 44. Values above this bound are
/// out of contract for the corpus — and are still recorded faithfully if one appears, because
/// masking an out-of-contract status would destroy the evidence that something produced it.
pub const MAX_CONTRACT_EXIT_CODE: i32 = 125;

/// The exit status the `timeout` utility reports when it expires.
///
/// A corpus program could legally return this value too, since it lies inside the contract
/// range, so expiry is never concluded from the status alone: it must be corroborated by an
/// elapsed time that reached the budget. See [`TIMEOUT_CORROBORATION_SLACK`].
pub const TIMEOUT_UTILITY_EXPIRY_STATUS: i32 = 124;

/// How long the module's own watchdog waits beyond the budget before it stops trusting the
/// external `timeout` utility and kills the child itself.
///
/// The net exists because the utility was measured to be defective in this environment — see
/// the module documentation. Five seconds is long enough that a correctly working utility
/// always wins the race, and short enough that a defective one cannot stall a run.
///
/// One consequence is recorded rather than glossed over: the net terminates the process this
/// module launched, which on the utility path is the utility itself, so a program the utility had
/// launched in turn can briefly outlive it. That condition is never silent — the reader threads
/// find the pipes still held open, the drain reports the stream as incomplete, and the outcome
/// carries a note naming both the defective utility and the undrained capture. On the watchdog
/// path the program is the process this module launched, so an uncatchable signal reaches it
/// directly and nothing can be left behind.
pub const TIMEOUT_UTILITY_GRACE: Duration = Duration::from_secs(5);

/// How far short of the budget an elapsed time may fall and still corroborate expiry reported
/// by the external `timeout` utility.
///
/// The utility waits the whole budget before signalling, and the measured elapsed time
/// therefore exceeds it. The slack exists only so that a coarse clock cannot turn a genuine
/// expiry into an ordinary exit of [`TIMEOUT_UTILITY_EXPIRY_STATUS`].
pub const TIMEOUT_CORROBORATION_SLACK: Duration = Duration::from_millis(100);

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

/// Size of the buffer each reader thread reads a pipe into.
const CAPTURE_CHUNK_BYTES: usize = 16 * 1024;

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
    /// utility or this module's own watchdog that enforced the budget is recorded separately,
    /// in [`RunOutcome::enforcement`] and [`RunOutcome::notes`].
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

/// Which of the two mandated mechanisms bounded this execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeoutEnforcement {
    /// The external `timeout` utility, wrapped around the argument vector.
    Utility,
    /// This module's own watchdog, used when no utility was discovered or when the budget is
    /// not a whole number of seconds. Also the safety net underneath the utility.
    Watchdog,
}

impl TimeoutEnforcement {
    /// The token used in a status record and a report row.
    pub fn label(self) -> &'static str {
        match self {
            TimeoutEnforcement::Utility => "utility",
            TimeoutEnforcement::Watchdog => "watchdog",
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
    /// line and nothing else. What was literally spawned is kept separately in
    /// [`RunOutcome::launch_argv`], so nothing is hidden either.
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

    /// The argument vector as literally spawned, including the external `timeout` utility when
    /// that utility enforced the budget.
    ///
    /// Equal to [`RunOutcome::argv`] whenever this module's own watchdog did the enforcing.
    pub fn launch_argv(&self) -> &[String] {
        &self.launch_argv
    }

    /// [`RunOutcome::launch_argv`] as a single POSIX shell line.
    ///
    /// Present so that the scaffolding is auditable rather than invisible: a reader comparing
    /// this against [`RunOutcome::command_line`] can see exactly what the harness added.
    pub fn launch_command_line(&self) -> String {
        posix_command_line(&self.launch_argv)
    }

    /// Whether the external utility wrapped the program, making the two command lines differ.
    pub fn launch_was_wrapped(&self) -> bool {
        self.launch_argv != self.argv
    }

    /// The working directory the child was given.
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_deref()
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

    /// How long the execution took, measured across the spawn and the wait.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// The budget the execution was measured against.
    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// Which mechanism was configured to bound this execution.
    pub fn enforcement(&self) -> TimeoutEnforcement {
        self.enforcement
    }

    /// Notes a reader must be told about, each already safe to render on one line.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// True when the budget was exceeded.
    pub fn timed_out(&self) -> bool {
        self.termination.timed_out()
    }

    /// The shape of the divergence this execution represents by itself, if any.
    ///
    /// Delegates to [`Termination::divergence_class`]: a crash or a timeout is divergent
    /// however it is compared, while an ordinary exit is judged by the oracles.
    pub fn divergence_class(&self) -> Option<DivergenceClass> {
        self.termination.divergence_class()
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
        record.push_str(&format!("duration_ms = {}\n", self.duration.as_millis()));
        record.push_str(&format!("budget_ms = {}\n", self.budget.as_millis()));
        record.push_str(&format!("timeout_enforcement = {}\n", self.enforcement));
        record.push_str(&format!(
            "runner = {}\n",
            match &self.runner {
                Some(runner) => runner.display().to_string(),
                None => String::from("(none: executed directly on the host)"),
            }
        ));
        record.push_str(&format!(
            "working_dir = {}\n",
            match &self.working_dir {
                Some(directory) => directory.display().to_string(),
                None => String::from("(inherited from the test process)"),
            }
        ));
        record.push_str(&format!("stdout_bytes = {}\n", self.stdout.len()));
        record.push_str(&format!("stderr_bytes = {}\n", self.stderr.len()));
        record.push_str(&format!("argv = {}\n", self.command_line()));
        // Only when it differs, so an ordinary record stays free of a line that repeats the one
        // above it — and so a record that does carry one is worth reading.
        if self.launch_was_wrapped() {
            record.push_str(&format!("launch = {}\n", self.launch_command_line()));
        }
        for note in &self.notes {
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
    /// Carries the termination, the raw wait status, the byte counts, the duration and the
    /// command line, which is everything needed to understand the row without re-running it.
    /// Passed through the report-safe rendering, so no captured path or note can forge a
    /// column or hide a line.
    pub fn describe(&self) -> String {
        let mut described = format!(
            "{} (raw wait status {}) in {} ms of a {} ms budget, {} stdout bytes, {} stderr \
             bytes, bounded by the {} mechanism, command: {}",
            self.termination,
            self.raw_wait_status,
            self.duration.as_millis(),
            self.budget.as_millis(),
            self.stdout.len(),
            self.stderr.len(),
            self.enforcement,
            self.command_line()
        );
        for note in &self.notes {
            described.push_str("; note: ");
            described.push_str(note);
        }
        sanitize_text_for_report(&described)
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunnerUnavailable {
    target: Target,
    detail: String,
}

impl RunnerUnavailable {
    /// The target whose artifacts cannot be executed.
    pub fn target(&self) -> Target {
        self.target
    }

    /// Why it cannot be executed, phrased for a report row and already safe on one line.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RunnerUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.detail)
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAttempt {
    /// The artifact was launched and reached a termination.
    Ran(RunOutcome),
    /// Nothing was launched, because the target's emulator is absent.
    RunnerUnavailable(RunnerUnavailable),
}

impl RunAttempt {
    /// The outcome, when the artifact ran.
    pub fn outcome(&self) -> Option<&RunOutcome> {
        match self {
            RunAttempt::Ran(outcome) => Some(outcome),
            RunAttempt::RunnerUnavailable(_) => None,
        }
    }

    /// The unavailability, when nothing ran.
    pub fn unavailable(&self) -> Option<&RunnerUnavailable> {
        match self {
            RunAttempt::Ran(_) => None,
            RunAttempt::RunnerUnavailable(reason) => Some(reason),
        }
    }

    /// True when nothing ran because the target's emulator is absent.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, RunAttempt::RunnerUnavailable(_))
    }

    /// A one-line description of either case, for a report row.
    pub fn describe(&self) -> String {
        match self {
            RunAttempt::Ran(outcome) => outcome.describe(),
            RunAttempt::RunnerUnavailable(reason) => sanitize_text_for_report(reason.detail()),
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

/// Run one built artifact for one target and record exactly what it did.
///
/// The artifact is launched directly when the host executes `target` natively, and under the
/// emulator `env.rs` vetted for `target` otherwise. It is given the cell's workspace as its
/// working directory, no arguments, no environment additions, and the null device as its
/// standard input; both output streams are captured and neither is inherited.
///
/// Returns [`RunAttempt::RunnerUnavailable`] — never an error and never a silent skip — when a
/// non-native target has no emulator here.
///
/// # Errors
///
/// A hard failure is reserved for a broken invariant rather than a property of the machine:
///
/// - the artifact path is not absolute, so what gets executed would depend on a working
///   directory the test harness makes no guarantee about;
/// - the artifact is not a regular file, or is a symbolic link, which
///   [`require_regular_file`] refuses rather than follows — a link would let something outside
///   the build directory be executed while every report still showed the workspace path;
/// - the artifact does not resolve to a path strictly inside the cell's workspace, which is the
///   mechanical half of the suite's hermeticity contract;
/// - the child could not be spawned, could not be reaped, or ended in a way the operating
///   system reported as neither an exit nor a signal;
/// - a stream could not be drained after a child that ended normally, since a silently
///   truncated capture would be compared and reported as a compiler divergence.
pub fn run(
    artifact: &Path,
    target: Target,
    workspace: &Workspace,
    caps: &Capabilities,
) -> HarnessResult<RunAttempt> {
    let context = format!(
        "running the {target} artifact {} in the workspace {}",
        artifact.display(),
        workspace.root().display()
    );

    if !artifact.is_absolute() {
        return Err(HarnessError::new(
            context,
            format!(
                "the artifact path {} is not absolute; an artifact is named from its cell's own \
                 workspace so that what gets executed never depends on the working directory, \
                 about which the test harness makes no guarantee",
                artifact.display()
            ),
        ));
    }
    require_regular_file(&context, artifact)?;
    ensure_within(&context, workspace.root(), artifact)?;

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
    command.current_dir(workspace.root());

    let outcome = execute_bounded(
        command,
        budget_for(caps),
        runner,
        caps.timeout_tool().path(),
    )?;
    Ok(RunAttempt::Ran(outcome))
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
    target: Target,
    workspace: &Workspace,
    caps: &Capabilities,
    names: &CaptureNames,
) -> HarnessResult<RunAttempt> {
    let attempt = run(artifact, target, workspace, caps)?;
    if let RunAttempt::Ran(outcome) = &attempt {
        outcome.persist(workspace, names)?;
    }
    Ok(attempt)
}

/// Run any prepared command under a bounded wait, capturing both streams.
///
/// The shared timed-wait facility. `compile.rs`, `flagprobe.rs` and `ubaudit.rs` all need to
/// spawn a process, bound it, capture its streams and read its status, and all four users go
/// through this one implementation: process handling written four times is how a suite acquires
/// four different timeout bugs.
///
/// The caller prepares the command — program, arguments, working directory, and any environment
/// change it needs — and this function supplies only the parts that must not vary: standard
/// input from the null device, both output streams piped and drained concurrently, a bounded
/// wait, and a forcible kill on expiry. Any standard-stream configuration the caller set is
/// replaced, because an inherited stream would both pollute the runner's own output and destroy
/// byte-exactness.
///
/// This function knows nothing about oracles, targets or verdicts. It spawns, bounds, captures
/// and returns.
///
/// The budget is enforced by this module's watchdog. A caller that holds a [`Capabilities`] and
/// wants the mandated primary mechanism should call [`run_command_captured_with`] instead,
/// passing the discovered `timeout` utility.
///
/// # Errors
///
/// The child could not be spawned or reaped, an argument could not be represented as text, the
/// termination was reported as neither an exit nor a signal, or a stream could not be drained
/// after a child that ended normally.
pub fn run_command_captured(command: Command, budget: Duration) -> HarnessResult<RunOutcome> {
    run_command_captured_with(command, budget, None)
}

/// [`run_command_captured`], with the external `timeout` utility used when one is available.
///
/// `timeout_tool` is the path discovery vetted, normally `caps.timeout_tool().path()`. When it
/// is `Some` and the budget is a whole number of seconds, the utility becomes the primary
/// enforcement and this module's watchdog stays behind it as a safety net; when it is `None`,
/// the watchdog alone enforces the budget. The observable behaviour is the same either way,
/// which is what makes the utility genuinely optional.
///
/// # Errors
///
/// As [`run_command_captured`].
pub fn run_command_captured_with(
    command: Command,
    budget: Duration,
    timeout_tool: Option<&Path>,
) -> HarnessResult<RunOutcome> {
    execute_bounded(command, budget, None, timeout_tool)
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
                "{target} artifacts cannot be executed on this {host} host because no execution \
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
/// suite has one timeout mechanism pair rather than four.
///
/// The sequence is deliberate. The streams are drained by threads that start before the wait
/// does, so a child that fills a pipe buffer cannot deadlock the harness — a real hazard here,
/// since a corpus program prints one line per property it asserts. The child is then polled to
/// completion, and killed if the bound is reached. Only after it has been reaped are the readers
/// awaited, because a reaped child's pipe write ends are closed and the reads finish at once.
fn execute_bounded(
    command: Command,
    budget: Duration,
    runner: Option<PathBuf>,
    timeout_tool: Option<&Path>,
) -> HarnessResult<RunOutcome> {
    // Mechanism selection, in the mandated order: the external utility first when it is present
    // and the budget is a whole number of seconds, this module's watchdog otherwise. When the
    // utility is in front, the watchdog stays behind it as a safety net a few seconds later, so a
    // utility that fails to enforce its own budget cannot stall the run.
    // The program's own line is taken before any wrapping, because that is the line a finding
    // artifact publishes and a maintainer re-runs. The wrapper, when there is one, is recorded
    // separately as the launch line rather than folded into it.
    let argv = argv_of(&command)?;
    let working_dir = command.get_current_dir().map(Path::to_path_buf);

    let (mut command, enforcement, net_budget) = match (timeout_tool, whole_second_budget(budget)) {
        (Some(tool), Some(seconds)) => (
            wrap_in_timeout_utility(&command, tool, seconds),
            TimeoutEnforcement::Utility,
            budget.saturating_add(TIMEOUT_UTILITY_GRACE),
        ),
        _ => (command, TimeoutEnforcement::Watchdog, budget),
    };

    // Every diagnostic below is phrased in terms of what was literally spawned, so a failure to
    // launch names the command that failed rather than the one it was standing in for.
    let launch_argv = argv_of(&command)?;

    // Fixed regardless of what the caller configured. Standard input comes from the null device
    // so a program can never block waiting for input; both output streams are piped so that
    // nothing is inherited, which would pollute the runner's own output and destroy the
    // byte-exactness every oracle depends on.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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

    let stdout_capture = match child.stdout.take() {
        Some(pipe) => spawn_reader(pipe),
        None => return Err(missing_pipe(&mut child, &launch_argv, "standard output")),
    };
    let stderr_capture = match child.stderr.take() {
        Some(pipe) => spawn_reader(pipe),
        None => return Err(missing_pipe(&mut child, &launch_argv, "standard error")),
    };

    let waited = wait_bounded(&mut child, net_budget, started, &launch_argv)?;
    let duration = started.elapsed();

    let drain_deadline = Instant::now() + CAPTURE_DRAIN_GRACE;
    let stdout_stream = drain(stdout_capture, "standard output", drain_deadline);
    let stderr_stream = drain(stderr_capture, "standard error", drain_deadline);

    let termination = termination_of(
        waited.status,
        waited.killed_by_net,
        enforcement,
        duration,
        budget,
        &launch_argv,
    )?;

    let mut notes = Vec::new();
    if waited.killed_by_net && enforcement == TimeoutEnforcement::Utility {
        push_note(
            &mut notes,
            format!(
                "the external timeout utility did not enforce its own budget of {} ms, so the \
                 harness's safety net terminated the child after {} ms; the installed utility may \
                 be defective, and the cell is still reported as a timeout",
                budget.as_millis(),
                net_budget.as_millis()
            ),
        );
    }
    for note in [stdout_stream.note.clone(), stderr_stream.note.clone()]
        .into_iter()
        .flatten()
    {
        push_note(&mut notes, note);
    }

    // A capture that could not be drained is tolerable only for a cell already known to have
    // timed out, whose stdout no oracle compares. After a child that ended on its own, a
    // truncated capture would be compared byte for byte and reported as a compiler divergence,
    // which is the one failure that would leave no trace of its real cause — so it is a hard
    // failure instead.
    let capture_complete = stdout_stream.completed && stderr_stream.completed;
    let truncation_tolerable = termination.timed_out() || capture_complete;
    if !truncation_tolerable {
        return Err(HarnessError::new(
            format!(
                "capturing the output of {} in {}",
                posix_command_line(&launch_argv),
                describe_working_dir(working_dir.as_deref())
            ),
            format!(
                "the child {termination} but its captured streams could not be drained within {} \
                 ms, so the capture is incomplete: some process that inherited the pipes is still \
                 holding them open. A truncated capture is refused rather than compared, because \
                 it would be reported as a divergence in the compiler under test",
                CAPTURE_DRAIN_GRACE.as_millis()
            ),
        ));
    }

    Ok(RunOutcome {
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
        duration,
        budget,
        enforcement,
        notes,
    })
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
/// positive integer within a ceiling, so every corpus execution takes the utility path when one
/// was discovered.
fn whole_second_budget(budget: Duration) -> Option<u64> {
    if budget.as_secs() == 0 || budget.subsec_nanos() != 0 {
        return None;
    }
    Some(budget.as_secs())
}

/// Rebuild `command` wrapped in the external timeout utility.
///
/// The invocation is the plain portable form `timeout <secs> <program> <args...>`. Neither
/// `--signal=KILL` nor `--kill-after` is passed, even though an uncatchable kill is exactly what
/// one would want: on the implementation measured in this environment the first sends no signal
/// at all — it merely waits for the child and then reports expiry, and hung indefinitely against
/// a compute-bound child — and the second reports status 125, which lies inside the corpus's own
/// expected-exit-code range and would therefore be indistinguishable from a program's own return
/// value. Uncatchable termination is guaranteed by this module's safety net instead, which sends
/// `SIGKILL` through [`Child::kill`].
///
/// The working directory and every explicit environment change the caller made are carried over,
/// so wrapping changes how the child is bounded and nothing else about it. The standard streams
/// are not carried over because [`execute_bounded`] fixes them for every path.
fn wrap_in_timeout_utility(command: &Command, tool: &Path, seconds: u64) -> Command {
    let mut wrapped = Command::new(tool);
    wrapped.arg(seconds.to_string());
    wrapped.arg(command.get_program());
    wrapped.args(command.get_args());
    if let Some(directory) = command.get_current_dir() {
        wrapped.current_dir(directory);
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
        Some(directory) => directory.display().to_string(),
        None => String::from("(inherited from the test process)"),
    }
}

/// Terminate and reap a child that cannot be used, and describe why.
///
/// Called only when a pipe the command asked for is absent, which the standard library does not
/// do for a piped stream. The child is still killed and reaped rather than abandoned: 1,296
/// cells run concurrently, and a process left behind by each would accumulate.
fn missing_pipe(child: &mut Child, argv: &[String], stream: &str) -> HarnessError {
    abandon(child);
    HarnessError::new(
        format!("capturing the output of {}", posix_command_line(argv)),
        format!(
            "the child's {stream} pipe was not available even though the stream was configured as \
             a pipe, so the capture could not be started; the child was terminated rather than \
             left running"
        ),
    )
}

/// Kill and reap a child whose outcome is no longer wanted, ignoring both failures.
///
/// Used only on an error path that is already returning an explanatory failure, where a
/// secondary failure to clean up would replace a diagnosable message with a less useful one.
fn abandon(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Append a note, made safe to render on a single line first.
///
/// Notes reach a status record, where each occupies one `note = ...` line, and a report row,
/// where an embedded line feed would split one row into two and a tab would forge a column. The
/// text they are built from includes messages from the operating system, so escaping at the point
/// of collection is what makes the guarantee hold for every consumer.
fn push_note(notes: &mut Vec<String>, note: String) {
    notes.push(sanitize_text_for_report(&note));
}

/// A pipe being drained on its own thread.
///
/// The buffer is shared rather than owned by the thread so that whatever has arrived can be
/// taken at any moment — including from a reader that is still blocked because some process
/// inherited the pipe. That is what turns a forcibly killed child's partial output into evidence
/// instead of nothing.
struct StreamCapture {
    /// Everything read so far.
    buffer: Arc<Mutex<Vec<u8>>>,
    /// Sends exactly once when the reader stops: `None` for end-of-file, `Some(message)` for a
    /// read error. A bounded receive on this is what keeps a stuck pipe from blocking the harness
    /// forever, which a plain thread join could not.
    finished: Receiver<Option<String>>,
}

/// Start draining one pipe on its own thread.
///
/// Both streams are drained concurrently and from the moment the child starts, because a child
/// that fills a pipe buffer while its parent waits for it to exit would deadlock — and every
/// corpus program prints one line per property it asserts, so the buffer genuinely does fill.
///
/// Bytes are appended exactly as they arrive. Nothing decodes them, normalizes a line ending or
/// trims anything: comparison is byte-exact, so a conversion here would silently mask a real
/// divergence.
fn spawn_reader<R: Read + Send + 'static>(mut source: R) -> StreamCapture {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buffer);
    let (report, finished) = mpsc::channel();
    thread::spawn(move || {
        let stopped_because = pump(&mut source, &sink);
        // A closed receiver means the execution already concluded without this reader, which the
        // drain records as an incomplete stream. There is nothing further to do about it here.
        let _ = report.send(stopped_because);
    });
    StreamCapture { buffer, finished }
}

/// Copy every byte one stream produces into the shared buffer, reporting why the copy stopped.
///
/// `None` means the stream reached end of file, which is the ordinary ending and the only one
/// that proves a capture is complete. `Some(reason)` carries a human-readable explanation that
/// travels into the outcome's notes, so a truncated capture is always visible rather than being
/// mistaken for a program that simply printed less.
///
/// Reads are retried on interruption, because the standard library leaves that retry to the
/// caller and abandoning the stream there would silently truncate the very bytes under
/// comparison. Nothing else about the bytes is touched.
fn pump<R: Read>(source: &mut R, sink: &Mutex<Vec<u8>>) -> Option<String> {
    let mut chunk = [0_u8; CAPTURE_CHUNK_BYTES];
    loop {
        match source.read(&mut chunk) {
            Ok(0) => return None,
            Ok(count) => match sink.lock() {
                Ok(mut held) => held.extend_from_slice(&chunk[..count]),
                Err(_) => return Some(String::from(POISONED_CAPTURE_BUFFER)),
            },
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Some(error.to_string()),
        }
    }
}

/// One drained stream: the bytes, whether the reader finished, and any note about it.
struct DrainedStream {
    bytes: Vec<u8>,
    /// True when the reader reached end-of-file or reported a read error — in both cases it is
    /// done. False only when it was still blocked at the deadline, which means the capture may be
    /// short of what the program actually wrote.
    completed: bool,
    note: Option<String>,
}

/// Collect one stream, waiting no longer than `deadline` for its reader to finish.
///
/// The reader normally finishes immediately: the child has already been reaped by the time this
/// is called, so the pipe's write ends are closed and the read returns end-of-file. The bound
/// exists for the one case where they are not — a process that inherited the pipes and outlived
/// the child — where waiting forever would hang the whole run.
fn drain(capture: StreamCapture, role: &str, deadline: Instant) -> DrainedStream {
    let remaining = deadline.saturating_duration_since(Instant::now());
    let (completed, note) = match capture.finished.recv_timeout(remaining) {
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
                "the {role} pipe was still open {} ms after the child was reaped, so the capture \
                 holds only what had arrived by then; a process that inherited the pipe is still \
                 running",
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
    DrainedStream {
        bytes: take_buffer(&capture.buffer),
        completed,
        note,
    }
}

/// Take everything a capture buffer holds, recovering the bytes even from a poisoned lock.
///
/// A poisoned lock means a reader thread panicked, which loses nothing already appended — so the
/// bytes are recovered rather than discarded, and rather than being turned into a panic of this
/// thread by unwrapping. Losing a capture would turn a diagnosable divergence into an
/// unexplained one.
fn take_buffer(buffer: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    match buffer.lock() {
        Ok(mut held) => std::mem::take(&mut *held),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    }
}

/// The result of the bounded wait: the status, and whether this module had to force it.
struct BoundedWait {
    status: ExitStatus,
    /// True when the budget was reached and the child was killed here, which is what makes a
    /// timeout a fact rather than an inference from an exit status.
    killed_by_net: bool,
}

/// Wait for a child, for no longer than `net_budget` measured from `started`.
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
fn wait_bounded(
    child: &mut Child,
    net_budget: Duration,
    started: Instant,
    argv: &[String],
) -> HarnessResult<BoundedWait> {
    let mut interval = POLL_INTERVAL_INITIAL;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(BoundedWait {
                    status,
                    killed_by_net: false,
                })
            }
            Ok(None) => {}
            Err(error) => {
                return Err(HarnessError::new(
                    format!("waiting for {}", posix_command_line(argv)),
                    format!(
                        "the child's status could not be read: {error}; the harness cannot report \
                         what a program did if it cannot learn how the program ended"
                    ),
                ))
            }
        }

        if started.elapsed() >= net_budget {
            let status = kill_and_reap(child, argv, net_budget)?;
            return Ok(BoundedWait {
                status,
                killed_by_net: true,
            });
        }

        thread::sleep(interval);
        interval = (interval * 2).min(POLL_INTERVAL_MAX);
    }
}

/// Terminate a child that exceeded its budget and reap it.
///
/// [`Child::kill`] sends an uncatchable signal, so the subsequent wait returns promptly and no
/// process and no zombie is left behind — which matters at the scale of the full matrix, where
/// one leaked process per cell would accumulate into thousands.
///
/// A refused kill is only fatal if the child is demonstrably still running: a child that has
/// already exited but not yet been reaped is reported differently across platform versions, and
/// treating that as a failure would turn an ordinary race into a harness error.
fn kill_and_reap(
    child: &mut Child,
    argv: &[String],
    net_budget: Duration,
) -> HarnessResult<ExitStatus> {
    if let Err(kill_error) = child.kill() {
        if let Ok(Some(status)) = child.try_wait() {
            return Ok(status);
        }
        return Err(HarnessError::new(
            format!(
                "terminating {} after its {} ms bound",
                posix_command_line(argv),
                net_budget.as_millis()
            ),
            format!(
                "the child could not be terminated: {kill_error}; it is still running, so the \
                 harness refuses to report a result it cannot bound"
            ),
        ));
    }
    child.wait().map_err(|error| {
        HarnessError::new(
            format!(
                "reaping {} after terminating it at its {} ms bound",
                posix_command_line(argv),
                net_budget.as_millis()
            ),
            format!(
                "the terminated child could not be reaped: {error}; a child left unreaped would \
                 accumulate across the matrix"
            ),
        )
    })
}

/// Decide how the child ended, from the wait status and from what enforced the budget.
///
/// The order is the order of certainty. A kill by this module's own net is a fact, so it settles
/// the question first. A signal comes next, read through the safe platform accessor, because a
/// signalled child has no exit code at all and must never be reported as though it had one.
/// Only then is an exit code considered, and only there can the external utility's expiry status
/// arise.
///
/// That expiry status is ambiguous on its own — it lies inside the corpus's own expected range,
/// so a program could legitimately return it — and is therefore accepted only when the elapsed
/// time also reached the budget. The slack absorbs a coarse clock and nothing more: a program
/// that returns the same value promptly is recorded as having exited with it, exactly as
/// observed.
fn termination_of(
    status: ExitStatus,
    killed_by_net: bool,
    enforcement: TimeoutEnforcement,
    elapsed: Duration,
    budget: Duration,
    argv: &[String],
) -> HarnessResult<Termination> {
    if killed_by_net {
        return Ok(Termination::TimedOut);
    }
    if let Some(signal) = status.signal() {
        return Ok(Termination::Signalled(signal));
    }
    if let Some(code) = status.code() {
        let reported_expiry =
            enforcement == TimeoutEnforcement::Utility && code == TIMEOUT_UTILITY_EXPIRY_STATUS;
        let reached_budget = elapsed.saturating_add(TIMEOUT_CORROBORATION_SLACK) >= budget;
        if reported_expiry && reached_budget {
            return Ok(Termination::TimedOut);
        }
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
