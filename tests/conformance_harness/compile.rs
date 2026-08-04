//! Builds one cell: a single compiler invocation, assembled under the shared-flag
//! discipline, run inside the cell's own workspace, and reported as data rather than raised as
//! an error.
//!
//! # What building one cell means
//!
//! A cell is one program, on one target, at one optimization level. Building it is exactly
//! one process: no shell, no intermediate script, no second probe, and no retry. The result
//! is a [`CompileOutcome`] carrying the argument vector, the exact reproduction line, the
//! termination status, both captured byte streams, whether an artifact appeared and how
//! large it was, and how long it took.
//!
//! # The shared-flag discipline, enforced rather than intended
//!
//! Only flags both compilers honour with the same meaning may be passed to both compilers,
//! and the set this module actually passes is deliberately the smallest one that can produce
//! a runnable, comparable artifact:
//!
//! | Argument | Why it is in the set |
//! | --- | --- |
//! | [`FLAG_OUTPUT`] followed by the artifact path | Both compilers place the artifact where they are told, and the cell needs a known path to execute |
//! | [`OptLevel::flag`] — one of `-O0`, `-O1`, `-O2` | The whole sweep. Comparing output across the three is what makes the suite a semantic-preservation test |
//! | [`FLAG_STATIC`] | The one linkage mode both compilers spell identically, and what lets an emulated target run with no sysroot and no dynamic loader |
//!
//! Everything else is refused at the point of assembly by `enforce_flag_discipline`, which
//! applies two independent tests to every argument:
//!
//! - a **deny-list**, [`is_forbidden_for_side`], which rejects a flag only one compiler
//!   honours, or one both accept while disagreeing about its default scope;
//! - an **allow-list**, `is_permitted_flag`, which rejects anything outside the minimal set
//!   above even when the deny-list has never heard of it.
//!
//! The allow-list is the half that survives maintenance. A deny-list can only refuse what
//! somebody thought to name; the allow-list refuses everything nobody has justified, so a
//! flag added to a future invocation has to be argued for here rather than merely spelled
//! correctly. Neither test is a convention that a reviewer has to remember: both run on every
//! single invocation, and a violation is a hard failure that names the flag, the side and the
//! whole command line.
//!
//! Three groups of flags are worth naming explicitly, because each is refused for a different
//! reason. `-O3` and `-Os` are levels the compiler under test documents no support for, and
//! every `-std=` spelling is a flag it does not have at all. The warning gate and the
//! sanitizers — `-Wall`, `-Wextra`, `-pedantic`, `-Werror`, every `-fsanitize=` form — belong
//! exclusively to the undefined-behaviour audit, which drives the reference compiler alone and
//! never enters a differential invocation. And `-fcf-protection` is refused *although both
//! compilers accept it*, because their default scopes differ: it is the entry that proves flag
//! parity had to be verified semantically rather than syntactically, and the reason this module
//! trusts a table rather than an experiment in acceptance.
//!
//! # Oracle (a) versus oracle (b): why one compiler-only flag is legitimate
//!
//! The discipline governs the arguments passed to *both* compilers. Selecting a target is not
//! such an argument, because the two sides select one by different mechanisms and neither
//! mechanism exists on the other side:
//!
//! - The **reference compiler has no target-selection flag.** The `--target=` spelling belongs
//!   to a different compiler family and is rejected, and `-m32` fails for want of multilib
//!   start files. A target is selected there by choosing a different driver binary, which is
//!   what [`Capabilities::ref_cc_for`] returns. A reference invocation therefore carries **no**
//!   target argument at all, and `enforce_target_selection_discipline` proves it.
//! - The **compiler under test has no cross drivers.** [`BCC_TARGET_FLAG`] is the only way it
//!   can reach a non-native backend, so it is required for every non-native cell — under
//!   oracle (a)'s cross arm exactly as much as under oracle (b), where both sides are the
//!   compiler under test and a flag it alone honours trivially satisfies "the same meaning to
//!   both". Without that reading, cross-backend testing would be impossible, since selecting a
//!   target is precisely what oracle (b) requires.
//!
//! The selection is passed for the native target too. That is permitted — it names the target
//! the compiler would have chosen anyway — and it keeps one recorded command template correct
//! for every cell of a program.
//!
//! # The argument vector is cross-checked against the program's own record
//!
//! Every program carries an expectation record holding the literal command templates a
//! maintainer can reproduce the cell with by hand. `cross_check_against_template` renders
//! the relevant template and requires it to expand to **exactly** the vector this module is
//! about to spawn, element for element.
//!
//! That check exists because the recorded recipe is itself a deliverable. Without it, a change
//! here would silently make every `.expected` record a work of fiction: the suite would compile
//! one way and document another, and the first person to discover it would be a maintainer
//! whose hand-run reproduction disagreed with the report. A mismatch is therefore a hard
//! failure naming the differing element and both full lines, never a warning.
//!
//! # Workspace discipline and parallel safety
//!
//! Each child runs with its current directory set to the cell's own workspace, so a compiler
//! that writes a temporary file *beside its output* writes it inside the one directory the cell
//! owns. The process-wide working directory is **never** changed: the feature-area tests run
//! concurrently in one process, so that would be a data race rather than a confinement. The
//! artifact is required to be a direct child of that workspace, and it is required to be the
//! canonical name for its compiler — which is what stops two compilers in one cell from
//! writing over each other's binary and leaving a comparison to be made against a single file
//! twice.
//!
//! Setting a working directory is not confinement, and the difference is worth stating. The
//! environment *is* replaced rather than inherited — `isolate_child_environment` clears it and
//! points `TMPDIR`, `TMP`, `TEMP` and `HOME` at the cell's own workspace — so a driver that
//! consults those variables writes its intermediates inside the workspace. A driver that instead
//! hard-codes a path under the system temporary directory — `gcc -###` shows `/tmp/cc*` for the
//! assembler input, the object and the linker response file — keeps doing so, because a variable
//! cannot bind a program that never reads it, and nothing here enters a namespace or filters a
//! syscall. The guarantee is that *this module* constructs no path outside the cell's workspace,
//! requires the artifact it goes on to execute to be inside it, and tells every child to keep its
//! scratch state there.
//!
//! Standard input is the null device, both output streams are captured as raw bytes through
//! pipes, and every invocation is bounded in time twice over: by the system timeout utility
//! when the environment has one, and by this module's own watchdog regardless. Nothing here
//! holds a lock, touches a global, or writes outside the cell's workspace.
//!
//! # A rejected program is data, not an error
//!
//! A compiler that refuses a program has told the suite something, and one that refuses a
//! program the other compiles has told it something important. Such an outcome is returned as
//! a [`BuildFailure`] carried inside the [`CompileOutcome`], never as a [`HarnessError`], so a
//! program at the edge of what an implementation supports is still built, still reported, and
//! still classifiable as an expected divergence against a documented limitation. Nothing here
//! drops a cell for being difficult.
//!
//! [`BuildFailure`] separates two questions that a bare exit status conflates. Its
//! [`DivergenceClass`] says what shape the failure took, and its [`FailureScope`] says who is
//! answerable for it — the compiler, the machine, or this process. A target whose C runtime is not
//! installed produces a link failure at [`FailureScope::Environment`], which must be reported as a
//! gap in the environment rather than as a finding against a compiler that never had the inputs it
//! needed. A build whose result could not be established at all is [`FailureScope::Harness`], which
//! must be reported as a failure of this suite rather than as either of the other two: it is the one
//! answer that says nothing was observed, and filing it as an environment gap would let a cell that
//! was launched and lost read as one the run had no reason to fail for.
//!
//! # What this module deliberately does not do
//!
//! It does not compare anything, classify a verdict, or write a report. In particular it never
//! compares the two compilers' diagnostics: wording legitimately differs between
//! implementations, so comparing it would produce a flood of divergences that say nothing about
//! code correctness. Standard error is captured because it is the fastest route to a diagnosis
//! in a finding artifact, and for no other reason.
//!
//! Only the standard library is used, the module is entirely safe Rust, and this file contains
//! no test function: it is infrastructure reached through `mod conformance_harness`, so a
//! self-test here would move the suite's own test counts.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::env::{confirm_vetted_tools_unchanged, kill_tool, Capabilities};
use super::execute::TIMEOUT_UTILITY_OUTER_MARGIN;
use super::manifest::{CommandSubstitutions, Manifest};
use super::sandbox::{
    Workspace, BCC_ARTIFACT_NAME, BCC_COMPILE_STATUS_NAME, BCC_COMPILE_STDERR_NAME,
    BCC_COMPILE_STDOUT_NAME, REFERENCE_ARTIFACT_NAME, REFERENCE_COMPILE_STATUS_NAME,
    REFERENCE_COMPILE_STDERR_NAME, REFERENCE_COMPILE_STDOUT_NAME,
};
use super::{
    bcc_requires_explicit_target, bcc_target_arguments, corpus_root, ensure_within,
    infrastructure_breach, infrastructure_breach_refusal, is_bcc_target_selector,
    is_forbidden_for_side, isolate_child_environment, measure_tree, own_process_group,
    posix_command_line, public_text, reap_bounded, record_infrastructure_breach, redact_secrets,
    require_regular_file, sanitize_text_for_report, shown_path, terminate_process_group,
    CaptureIntegrity, CompilerSide, DivergenceClass, GroupTermination, HarnessError, HarnessResult,
    OptLevel, Provenance, SideRecord, Target, TreeLimits, BCC_TARGET_FLAG, CAPTURE_CHUNK_BYTES,
    CAPTURE_RETAINED_BYTES_MAX, DIFFERENTIAL_FLAGS_MINIMAL,
};

/// The flag that names the artifact, spelled once so the argument builder, the allow-list and
/// the recorded command template cannot disagree about it.
///
/// A member of [`DIFFERENTIAL_FLAGS_MINIMAL`], which
/// `require_minimal_flag_table_agreement` asserts on every invocation.
pub const FLAG_OUTPUT: &str = "-o";

/// The linkage mode every artifact is built with.
///
/// Static linkage is the one mode both compilers spell identically, and it is what makes an
/// emulated target executable with no sysroot and no dynamic-loader configuration. Also a
/// member of [`DIFFERENTIAL_FLAGS_MINIMAL`].
pub const FLAG_STATIC: &str = "-static";

/// Bytes retained from each of the compiler's output streams.
///
/// Generous for a diagnostic and finite by design. Spelled as an alias of the harness-wide
/// [`CAPTURE_RETAINED_BYTES_MAX`] rather than as its own number, so a compiler's diagnostics and
/// a program's output are bounded by one quota that cannot drift apart between the two modules
/// that enforce it — and so a reader of either module finds the same figure.
///
/// The bound alone would not make a diagnostic loop harmless, because a reader that stops at the
/// cap and abandons the pipe leaves the writer blocked rather than terminated. The pipe is
/// therefore drained past the cap and the surplus discarded, and how much was discarded is
/// carried on the outcome as a [`CaptureIntegrity`] instead of being silently absorbed.
const CAPTURE_BYTES_MAX: u64 = CAPTURE_RETAINED_BYTES_MAX;

/// Interval between checks on a child that has not yet finished.
///
/// Short enough that the measured duration of a compilation — roughly seventy milliseconds for
/// a whole compile-and-run pair — is not dominated by polling granularity, and long enough that
/// waiting costs no measurable processor time.
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Bytes one cell workspace may hold while a compilation is still in flight.
///
/// The captured pipes are already bounded in memory, and a workspace kept after a failing cell is
/// already bounded by the retention ceilings in `sandbox.rs`. Neither of those bounds the **disk**
/// while a compiler is running: a driver that writes without end fills the build volume during its
/// budget, and up to fourteen feature areas do that concurrently, so the volume is gone before any
/// post-cell accounting is reached.
///
/// The figure is deliberately far above anything legitimate. A cell workspace holds the copied
/// program and its record, up to two statically linked executables of roughly three quarters of a
/// megabyte each, four captured streams and two status records — call it a few megabytes at the very
/// most. Sixty-four is therefore about twenty times the honest worst case and still leaves the volume
/// intact when every concurrent worker reaches it at once, which is the property that matters: this
/// is a ceiling on a runaway, not a budget for a build.
///
/// The artifact is measured as part of the workspace rather than separately, because it is *in* the
/// workspace: one ceiling over the directory covers the artifact, the compiler's temporary files and
/// anything else the driver leaves behind, and cannot disagree with itself the way two would.
const WORKSPACE_LIVE_BYTES_MAX: u64 = 64 * 1024 * 1024;

/// Entries one cell workspace may hold while a compilation is still in flight.
///
/// A count ceiling as well as a byte ceiling, because the two failures look nothing alike: a million
/// empty files cost almost no bytes and still exhaust a filesystem's inodes and make the directory
/// unreadable. A legitimate cell workspace holds roughly a dozen entries, so this is two orders of
/// magnitude of headroom.
const WORKSPACE_LIVE_ENTRIES_MAX: u64 = 256;

/// How often the workspace is measured while a child is being watched.
///
/// Not on every poll: the wait polls every [`WAIT_POLL_INTERVAL`], and walking a directory that often
/// would cost more than the compilation it is protecting. Fifty milliseconds is the balance, and the
/// two sides of it are worth stating because they pull in opposite directions.
///
/// The interval is the **overshoot**: a runaway is stopped at the ceiling plus whatever it can write
/// before the next scan, and a fast volume can write a great deal in a quarter of a second. The cost
/// is twenty walks a second of a directory holding about a dozen entries — a readdir and a handful of
/// metadata reads, tens of microseconds — which is immaterial beside a compilation measured in tens
/// of milliseconds. Since a whole compile-and-run pair takes roughly seventy milliseconds, this also
/// means an ordinary cell is scanned at least once, so the mechanism is exercised by the matrix itself
/// rather than only by the pathological case it exists for.
const WORKSPACE_SCAN_INTERVAL: Duration = Duration::from_millis(50);

/// Ceilings on the live workspace measurement itself.
///
/// The measurement is a bound, so it needs bounds of its own: it runs while a child is being watched,
/// and a walk that took an unbounded amount of time would delay the very watchdog it serves.
///
/// The entry ceiling is deliberately well above [`WORKSPACE_LIVE_ENTRIES_MAX`], so that a workspace
/// over the live ceiling is *seen* to be over it rather than merely reported as unmeasurable. The
/// depth ceiling is generous against a compiler's temporary sub-directory and refuses the deep tree
/// a hostile one could build. Fifty milliseconds is far more than a walk of a dozen entries needs.
const WORKSPACE_TREE_LIMITS: TreeLimits = TreeLimits {
    depth_max: 8,
    entries_max: 4096,
    elapsed_max: Duration::from_millis(50),
};

/// Time allowed for the capture threads to deliver after the child has been waited on.
///
/// Without it, a compilation that finished just inside its budget could have its diagnostics
/// truncated by the same deadline that governed the wait — losing exactly the text a
/// divergence has to be read from. With it, the whole invocation is still bounded, at the
/// budget plus this grace.
const CAPTURE_GRACE: Duration = Duration::from_secs(2);

/// Time allowed for a cancelled capture thread to notice, once the grace above has already elapsed.
///
/// A second and much shorter window, because cancellation is cooperative: the flag is observed by the
/// drain between reads, so a reader that is between reads stops promptly while one blocked inside a
/// read cannot be interrupted by anything the standard library offers. Long enough to observe the
/// former, short enough not to extend the invocation's bound in any way that matters.
const CAPTURE_CANCEL_GRACE: Duration = Duration::from_millis(250);

/// Characters of a captured diagnostic quoted in a failure summary.
///
/// A summary is one line in a report, and the untruncated stream is always available on the
/// outcome, so the excerpt exists to identify the failure rather than to explain it in full.
const SUMMARY_EXCERPT_CHARS_MAX: usize = 240;

/// The marker that introduces a diagnostic in the documented GCC-compatible form
/// `file:line:col: error: description`.
const DIAGNOSTIC_ERROR_MARKER: &str = "error:";

/// Diagnostics that mean a target's C runtime is not installed, rather than that a compiler
/// produced something that could not be linked.
///
/// Getting this distinction wrong in the other direction would manufacture findings against a
/// compiler on any modestly provisioned machine, which is why it is checked before every other
/// signature: a cross target whose start files or C library are absent cannot be statically
/// linked by anybody, and saying so is a report about the environment.
///
/// Matched case-insensitively against the whole captured stream.
const ENVIRONMENT_LINK_SIGNATURES: &[&str] = &[
    "cannot find crt1.o",
    "cannot find crti.o",
    "cannot find crtn.o",
    "cannot find scrt1.o",
    "cannot find crtbegin",
    "cannot find crtend",
    "cannot find -lc",
    "cannot find -lm",
    "cannot find -lgcc",
    "cannot find libc.a",
    "unable to find library -lc",
    "library not found for -lc",
    "skipping incompatible",
];

/// Diagnostics that mean the driver's own installation is broken — a compiler stage it wanted
/// to run is missing or could not be executed.
///
/// The compilation never happened, so this is a compile-stage failure at
/// [`FailureScope::Environment`]: nothing about the program under test has been established.
const TOOLCHAIN_INSTALLATION_SIGNATURES: &[&str] = &[
    "cannot execute '",
    "error trying to exec",
    "installation problem, cannot exec",
];

/// Diagnostics that place a failure at the link stage rather than in translation.
///
/// Deliberately conservative. A compiler echoes source text into its diagnostics, so a
/// signature loose enough to match a fragment of a test program would misclassify a plain
/// compile error; every entry here is a phrase a linker produces and a C program is
/// vanishingly unlikely to contain. Anything unmatched falls through to a compile failure with
/// the whole stream attached, which is the documented conservative default — and it is the
/// right default for the compiler under test, whose integrated linker owes this suite no
/// particular wording.
const LINKER_STAGE_SIGNATURES: &[&str] = &[
    "undefined reference to",
    "undefined symbol",
    "multiple definition of",
    "relocation truncated",
    "final link failed",
    "linker input file",
    "ld returned",
    "collect2:",
    "ld: cannot",
    "ld: error",
];

/// Which compiler an invocation drives.
///
/// This is the only distinction the module branches on, and every difference between the two
/// sides follows from it: which binary is spawned, whether a target-selection flag is passed,
/// which artifact name the workspace expects, and which recorded command template the assembled
/// vector is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Compiler {
    /// The compiler under test, invoked with [`BCC_TARGET_FLAG`] because it has no cross
    /// drivers and that flag is its only route to a non-native backend.
    Bcc,
    /// The reference compiler for this cell's target: the native driver on the native arm, the
    /// matching cross driver on a cross arm, and never a target flag.
    Reference,
}

impl Compiler {
    /// Both compilers, in oracle (a) comparison order — the compiler under test first, because
    /// it is the subject of every comparison.
    ///
    /// Read on every invocation by `CompileRequest::validate_artifact`, which uses it to prove the
    /// compiler-to-artifact-name mapping is injective. That proof is what stops two builds of one
    /// cell from writing the same path and leaving oracle (a) comparing a binary against itself.
    pub const ALL: [Compiler; 2] = [Compiler::Bcc, Compiler::Reference];

    /// Which side of the shared-flag discipline this compiler is on.
    ///
    /// The reference side is judged by the whole forbidden table, because it is the side that
    /// decides whether a spelling can belong to a set both compilers honour identically. The
    /// side under test is judged by the same table minus the target selectors it alone can
    /// honour. That split lives in [`is_forbidden_for_side`] and is expressed nowhere else.
    pub fn side(self) -> CompilerSide {
        match self {
            Compiler::Bcc => CompilerSide::UnderTest,
            Compiler::Reference => CompilerSide::Reference,
        }
    }

    /// Whether this compiler selects a target with a flag rather than by driver binary.
    ///
    /// True for the compiler under test and false for the reference compiler, which makes
    /// [`BCC_TARGET_FLAG`] unreachable for [`Compiler::Reference`] by construction rather than
    /// by inspection of the call sites.
    pub fn selects_target_by_flag(self) -> bool {
        matches!(self, Compiler::Bcc)
    }

    /// The name this compiler's artifact takes inside a cell workspace.
    ///
    /// Taken from the workspace vocabulary rather than spelled again here, so the file this
    /// module writes is the file every other consumer of the workspace looks for.
    pub fn artifact_name(self) -> &'static str {
        match self {
            Compiler::Bcc => BCC_ARTIFACT_NAME,
            Compiler::Reference => REFERENCE_ARTIFACT_NAME,
        }
    }

    /// The three workspace names this compiler's *build* evidence is written under.
    ///
    /// Returned as a triple from the workspace vocabulary rather than assembled from a prefix, for
    /// the same reason [`Compiler::artifact_name`] is: the file this module writes must be the file
    /// every other consumer of the workspace looks for, and a name built by formatting is a name no
    /// other consumer can reference. Taking all three together also makes it impossible to write a
    /// build's standard output under one compiler's name and its status under the other's.
    pub fn compile_evidence_names(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Compiler::Bcc => (
                BCC_COMPILE_STDOUT_NAME,
                BCC_COMPILE_STDERR_NAME,
                BCC_COMPILE_STATUS_NAME,
            ),
            Compiler::Reference => (
                REFERENCE_COMPILE_STDOUT_NAME,
                REFERENCE_COMPILE_STDERR_NAME,
                REFERENCE_COMPILE_STATUS_NAME,
            ),
        }
    }

    /// The phrase used in diagnostics and report rows.
    pub fn label(self) -> &'static str {
        self.side().label()
    }
}

impl std::fmt::Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Who is answerable for a build failure.
///
/// Kept separate from the failure's shape because the two answer different questions and a
/// single value cannot carry both. A static link that fails because a target's C runtime was
/// never installed has exactly the shape of a link failure caused by a defect in code
/// generation, and the difference between them is the difference between a report about this
/// machine and a finding against a compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FailureScope {
    /// The failure is a property of the compiler and the program, and is therefore a candidate
    /// for a finding or for an expected divergence.
    Compiler,
    /// The failure is a property of this machine — a missing C runtime for a target, or a
    /// driver installation that cannot run its own stages. Reported as an environment gap, and
    /// never as a defect in a compiler that was never given the inputs it needed.
    Environment,
    /// Who is answerable could not be determined, because the evidence the determination rests on
    /// was not fully captured.
    ///
    /// # Why an honest third answer, rather than a default to one of the other two
    ///
    /// Most scopes are read out of the compiler's own diagnostics: a line naming a missing C
    /// runtime means the machine, a line naming a source location means the program. That reading
    /// is only as good as the text it was performed on, and a diagnostic stream truncated by the
    /// retention quota or abandoned by a reader that could not finish may be missing precisely the
    /// line that would have decided it.
    ///
    /// Defaulting such a case either way is wrong in a way that costs something real. Defaulting
    /// to [`FailureScope::Compiler`] manufactures a finding against a compiler on the strength of
    /// text nobody read — the worst outcome available, because requirement 6 makes a finding a
    /// deliverable a maintainer is expected to act on. Defaulting to
    /// [`FailureScope::Environment`] silently discards a real defect, and does so in the one
    /// category the suite exists to detect. This variant says what actually happened, and the
    /// classifier renders it as a reported gap that is neither a pass nor an accusation.
    ///
    /// Deliberately **not** used for a failure established by a fact rather than by text. A
    /// timeout, a signal death, an unobservable status and a missing artifact are all observed
    /// directly, so a truncated diagnostic stream does not weaken them and they keep the scope
    /// they were given.
    Indeterminate,
    /// The failure is a property of *this process*: a compilation was launched and its result
    /// could not be established, so nothing was observed about either the compiler or the machine.
    ///
    /// Held apart from [`FailureScope::Environment`] because the two lead to opposite verdicts and
    /// conflating them is how a defect escapes. An environment gap is a reported absence — the arm
    /// could not be attempted, and the run does not fail for it. A bookkeeping fault is the
    /// harness's own defect: it means a cell was launched and its outcome lost, which is a failure
    /// and must be reported as one. Filing it as an absence would let a suite that cannot observe
    /// its own children report a clean run over cells it never actually judged.
    Harness,
}

impl FailureScope {
    /// The token used in reports and summaries.
    pub fn label(self) -> &'static str {
        match self {
            FailureScope::Compiler => "compiler scope",
            FailureScope::Environment => "environment scope",
            FailureScope::Indeterminate => "indeterminate scope",
            FailureScope::Harness => "harness scope",
        }
    }
}

impl std::fmt::Display for FailureScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Why one build did not produce a usable artifact.
///
/// A value of this type is an observation, not an error: it is carried inside a
/// [`CompileOutcome`] so that the classifier can decide a verdict with the whole invocation in
/// front of it. The shape is one of the suite's closed set of divergence classes — a rejected
/// program is [`DivergenceClass::CompileFailure`], a program that translated but did not link
/// is [`DivergenceClass::LinkFailure`], and an invocation that outlived its budget is
/// [`DivergenceClass::Timeout`] — and the scope says who is answerable: the compiler, the machine,
/// or this process itself.
///
/// The fields are private because the three travel together as one judgement. A caller able to
/// rewrite the scope could turn an environment gap into a finding against a compiler, hide a real
/// defect behind a claim about the machine, or relabel a result this suite lost as an arm it merely
/// could not attempt, while every report still displayed the original summary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BuildFailure {
    class: DivergenceClass,
    scope: FailureScope,
    summary: String,
}

impl BuildFailure {
    /// Build a failure observation. Private so that every value originates in
    /// [`classify_failure`], which is the only place the evidence is all present at once.
    fn new(
        class: DivergenceClass,
        scope: FailureScope,
        summary: impl Into<String>,
    ) -> BuildFailure {
        BuildFailure {
            class,
            scope,
            summary: summary.into(),
        }
    }

    /// The shape of the failure, from the suite's closed set.
    pub fn class(&self) -> DivergenceClass {
        self.class
    }

    /// Who is answerable: the compiler, the machine, or this process.
    pub fn scope(&self) -> FailureScope {
        self.scope
    }

    /// A single line naming what happened, safe to write into a report.
    ///
    /// Any text quoted from a compiler's diagnostics has already been escaped and truncated, so
    /// no captured byte can forge a column, erase a line or repaint a verdict.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    // # Why there is no `is_environment` predicate here
    //
    // A boolean over three scopes cannot be read correctly: "is the machine answerable" is `false`
    // both for a compiler defect and for a failure whose answerable party is unknown, so every
    // caller of such a predicate silently treats the second as the first — which is a manufactured
    // finding against a compiler on the strength of diagnostics nobody read.
    //
    // [`BuildFailure::scope`] is the only accessor, and every consumer matches it exhaustively, so
    // a fourth scope added later cannot reach a report until each consumer says what it means. That
    // is a compiler-checked guarantee where a predicate would offer only a convention.

    /// The same failure with its scope declared unattributable, and the reason appended.
    ///
    /// Consumes and returns rather than mutating, so an indeterminate scope can only be produced
    /// where the original judgement is in hand and is being replaced wholesale. The summary keeps
    /// what was originally concluded and states that it could not be relied upon, because a
    /// maintainer reading the record needs both the reading and the reason it was withdrawn.
    fn into_indeterminate(self, reason: &str) -> BuildFailure {
        BuildFailure {
            class: self.class,
            scope: FailureScope::Indeterminate,
            summary: format!(
                "{} — who is answerable could not be determined because {reason}, so this is \
                 reported as an unattributable gap rather than as a defect in the compiler or in \
                 the machine",
                self.summary
            ),
        }
    }

    /// Whether this failure is a fault in this process's own bookkeeping.
    ///
    /// Asked *before* [`BuildFailure::is_environment`] by every caller that decides a verdict,
    /// because the two questions are not independent: a bookkeeping fault is neither the
    /// compiler's nor the machine's, and answering only the environment question would file it
    /// as an absence the run does not fail for.
    pub fn is_harness(&self) -> bool {
        matches!(self.scope, FailureScope::Harness)
    }
}

impl std::fmt::Display for BuildFailure {
    /// The one canonical rendering of a failure: class, scope, and the single-line summary.
    ///
    /// There is deliberately no separate accessor for the summary alone. Every consumer wants the
    /// scope beside it — a link failure at [`FailureScope::Environment`] and one at
    /// [`FailureScope::Compiler`] are the same sentence with opposite meanings — and an accessor
    /// that handed out the summary by itself would make it possible to render the words without
    /// the attribution.
    ///
    /// Any text quoted from a compiler's diagnostics has already been escaped and truncated, so no
    /// captured byte can forge a column, erase a line or repaint a verdict.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.class, self.scope, self.summary)
    }
}

/// One build to perform: which compiler, for which cell, writing into which workspace.
///
/// # Why the cell is carried as substitutions rather than as loose paths
///
/// The target, the optimization level, the program source and the artifact path all arrive
/// inside a [`CommandSubstitutions`], and they arrive that way for a reason that is the whole
/// point of this type. Those values have already been vetted by the layer that produced them —
/// a compiler path was checked for trust, a source path was proved to resolve inside the corpus,
/// an artifact path was derived from the cell's own workspace — and the worth of that vetting is
/// entirely that the *same* values are the ones that get executed. Carrying them in the object
/// the recorded command templates are rendered from means the vector this module spawns and the
/// line a maintainer reproduces it with are expansions of one set of values, with no second
/// copy to drift.
///
/// The caller has that object already: it is what renders the run command too, so nothing extra
/// is asked of it.
///
/// # Why the fields are private
///
/// Construction is the only place the combination is checked, and every check is about a
/// *combination* rather than a field: that the artifact is a direct child of this workspace and
/// carries the canonical name for this compiler, that the source is a real file inside the
/// corpus or inside the workspace, that a reference build actually knows which reference driver
/// it means, and that this cell is one the program's own record declares. Any of those could be
/// defeated by assigning a single field afterwards, so a value of this type cannot exist in a
/// state the constructor would have refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileRequest<'a> {
    compiler: Compiler,
    manifest: &'a Manifest,
    substitutions: &'a CommandSubstitutions,
    workspace: &'a Workspace,
}

impl<'a> CompileRequest<'a> {
    /// Describe one build, refusing a combination that could not be reproduced or could not be
    /// trusted.
    ///
    /// # Errors
    ///
    /// Every rejection below is a caller or corpus defect rather than something the environment
    /// can legitimately produce, so each is an explanatory hard failure rather than a build that
    /// goes ahead and reports a misleading result:
    ///
    /// - the artifact is not a direct child of `workspace`, which would let a build write
    ///   outside the one directory the cell owns;
    /// - the artifact is not named [`Compiler::artifact_name`] for this compiler. This is the
    ///   check that prevents the quietest possible bug in the suite: two compilers in one
    ///   workspace writing to one path, after which oracle (a) compares a binary against
    ///   itself and reports agreement;
    /// - the source is not an existing regular file, is a symbolic link, or resolves outside
    ///   both the corpus and this workspace — the only two places a program the suite compiles
    ///   and then *executes* may legitimately come from;
    /// - a reference build was described without a reference driver, so the recorded reference
    ///   template would have nothing to expand;
    /// - the cell's target or optimization level is not one the program's own record declares,
    ///   which would test a configuration the record does not describe and compare it against
    ///   an expectation recorded for a different one.
    pub fn new(
        compiler: Compiler,
        manifest: &'a Manifest,
        substitutions: &'a CommandSubstitutions,
        workspace: &'a Workspace,
    ) -> HarnessResult<CompileRequest<'a>> {
        let request = CompileRequest {
            compiler,
            manifest,
            substitutions,
            workspace,
        };
        let context = format!(
            "describing the {} build of {}",
            compiler.label(),
            request.cell_label()
        );
        request.validate_artifact(&context)?;
        request.validate_source(&context)?;
        request.validate_reference_driver(&context)?;
        request.validate_declared_matrix(&context)?;
        Ok(request)
    }

    /// Which compiler this build drives.
    pub fn compiler(&self) -> Compiler {
        self.compiler
    }

    /// Which side of the shared-flag discipline this build's arguments are judged by.
    pub fn side(&self) -> CompilerSide {
        self.compiler.side()
    }

    /// The expectation record that governs the program being built.
    pub fn manifest(&self) -> &'a Manifest {
        self.manifest
    }

    /// The vetted values this build's arguments and recorded command line are rendered from.
    pub fn substitutions(&self) -> &'a CommandSubstitutions {
        self.substitutions
    }

    /// The one directory this build may write into, and the working directory its child is
    /// given.
    pub fn workspace(&self) -> &'a Workspace {
        self.workspace
    }

    /// The target architecture this build emits for.
    pub fn target(&self) -> Target {
        self.substitutions.target()
    }

    /// The optimization level this build is performed at.
    pub fn opt(&self) -> OptLevel {
        self.substitutions.opt()
    }

    /// The program source handed to the compiler.
    pub fn source(&self) -> &'a Path {
        self.substitutions.source()
    }

    /// The artifact the compiler is asked to produce.
    pub fn output(&self) -> &'a Path {
        self.substitutions.output()
    }

    /// This cell named the way reports and diagnostics name it: area, program, target and level.
    pub fn cell_label(&self) -> String {
        format!(
            "{}/{} @ {} {}",
            self.manifest.area(),
            self.manifest.program(),
            self.target().triple(),
            self.opt().flag()
        )
    }

    /// Require the artifact to be a direct child of this workspace, under the canonical name for
    /// this compiler.
    ///
    /// Containment is decided on the derived spelling rather than by resolving the path, because
    /// the artifact does not exist yet and a path that does not exist cannot be resolved. The
    /// workspace was already proved to sit immediately beneath the run's work root when it was
    /// allocated, so a direct child of it is inside the build directory by construction.
    fn validate_artifact(&self, context: &str) -> HarnessResult<()> {
        let output = self.output();
        let root = self.workspace.root();
        if output.parent() != Some(root) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the artifact {} is not a direct child of the cell workspace {}; a build \
                     writes into the one directory its cell owns and nowhere else, and the \
                     artifact path must be derived from that workspace — for example with \
                     `Workspace::path({:?})`",
                    shown_path(output),
                    shown_path(root),
                    self.compiler.artifact_name()
                ),
            ));
        }
        let expected = self.compiler.artifact_name();
        // The name must identify *this* compiler, so the mapping from compiler to artifact name has
        // to be injective. Checking only that the artifact carries this compiler's own name would
        // pass unchanged if both compilers named the same file, and that is the one failure the
        // check below exists to prevent: two builds into one path leave oracle (a) comparing a
        // binary against itself and reporting agreement, which looks exactly like a pass. The
        // assertion is over the whole compiler set rather than over the two spellings, so a third
        // compiler added later is covered without anyone remembering to come back here.
        for other in Compiler::ALL {
            if other != self.compiler && other.artifact_name() == expected {
                return Err(HarnessError::new(
                    String::from(context),
                    format!(
                        "the {} and the {} both name their artifact {expected:?}, so the two \
                         builds of a cell would write the same path and oracle (a) would compare \
                         one binary against itself and report agreement — a false pass that \
                         leaves no trace in any output. The artifact-name mapping must be \
                         injective; correct it in `Compiler::artifact_name`",
                        self.compiler.label(),
                        other.label()
                    ),
                ));
            }
        }
        let actual = output.file_name().and_then(|name| name.to_str());
        if actual != Some(expected) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the artifact {} is not named {expected:?}, which is the canonical name for \
                     the {}; both compilers build into one workspace per cell, so a shared or \
                     swapped artifact name would leave oracle (a) comparing one binary against \
                     itself and reporting agreement — the one failure that leaves no trace",
                    shown_path(output),
                    self.compiler.label()
                ),
            ));
        }
        Ok(())
    }

    /// Require the program source to be a real file the suite is permitted to compile.
    ///
    /// Two locations are legitimate and no others: the corpus, which holds every committed
    /// program, and this cell's own workspace, which holds the copy that makes a retained
    /// workspace reproducible on its own. A file from anywhere else is arbitrary code that the
    /// suite would compile and then execute.
    fn validate_source(&self, context: &str) -> HarnessResult<()> {
        let source = self.source();
        require_regular_file(context, source)?;
        let workspace_root = self.workspace.root();
        if ensure_within(context, &corpus_root(), source).is_ok()
            || ensure_within(context, workspace_root, source).is_ok()
        {
            return Ok(());
        }
        Err(HarnessError::new(
            String::from(context),
            format!(
                "the program source {} resolves neither inside the corpus {} nor inside the cell \
                 workspace {}; those are the only two places a file the suite compiles and then \
                 executes may come from, so a source outside both is refused rather than built",
                shown_path(source),
                shown_path(&corpus_root()),
                shown_path(workspace_root)
            ),
        ))
    }

    /// Require a reference build to know which reference driver it means.
    ///
    /// Checked here rather than left to rendering so that the message names the omission and the
    /// remedy. A cell whose reference driver did not resolve is reported as an unavailable
    /// oracle arm by the discovery layer and never reaches this module.
    fn validate_reference_driver(&self, context: &str) -> HarnessResult<()> {
        if self.compiler.selects_target_by_flag() {
            return Ok(());
        }
        if self.substitutions.reference_compiler().is_some() {
            return Ok(());
        }
        Err(HarnessError::new(
            String::from(context),
            format!(
                "these substitutions carry no reference compiler driver, so neither the argument \
                 vector nor the recorded reference command can name one. The reference compiler \
                 has no target-selection flag: on that side the driver binary *is* the target, so \
                 the driver for {} must be resolved with `Capabilities::ref_cc_for` and supplied \
                 through `CommandSubstitutions::with_reference_compiler` before a reference build \
                 is described. Where oracle (a) has no driver for a target, that arm is reported \
                 unavailable and the cell's other oracles run as usual",
                self.target().triple()
            ),
        ))
    }

    /// Require this cell to be one the program's own record declares.
    ///
    /// A record may narrow its matrix, but only with a recorded reason — that is how a
    /// construct whose value legitimately differs between architectures stays under test while
    /// being excluded from the one comparison that cannot be made. Building a cell the record
    /// excludes would test a configuration nothing describes and compare it against an
    /// expectation recorded for a different one.
    fn validate_declared_matrix(&self, context: &str) -> HarnessResult<()> {
        let target = self.target();
        if !self.manifest.targets().contains(&target) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the record {} does not declare the target {}; it declares {}. A narrowed \
                     target list is a recorded exclusion with a stated reason, so a cell outside \
                     it must not be built",
                    shown_path(self.manifest.path()),
                    target.triple(),
                    joined_targets(self.manifest.targets())
                ),
            ));
        }
        let opt = self.opt();
        if !self.manifest.opt_levels().contains(&opt) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the record {} does not declare the optimization level {}; it declares {}",
                    shown_path(self.manifest.path()),
                    opt.flag(),
                    joined_opt_levels(self.manifest.opt_levels())
                ),
            ));
        }
        Ok(())
    }
}

/// Render a target list as one comma-separated line for a diagnostic.
fn joined_targets(targets: &[Target]) -> String {
    targets
        .iter()
        .map(|target| String::from(target.triple()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render an optimization-level list as one comma-separated line for a diagnostic.
fn joined_opt_levels(levels: &[OptLevel]) -> String {
    levels
        .iter()
        .map(|level| String::from(level.flag()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Everything one build produced, whether it succeeded or not.
///
/// Self-sufficient by design: a report row, a finding artifact and a reproduction script can all
/// be written from this value alone, with no second look at the environment and no re-run. That
/// is what [`CompileOutcome::command_line`] is for — the exact line, quoted so a shell
/// reconstructs the vector element for element, so that "the exact reproduction commands" is
/// literally true rather than approximately true.
///
/// The fields are private because several of them are only meaningful together. The judgement in
/// [`CompileOutcome::failure`] was reached from the status, the captured diagnostics, the
/// artifact and the probed C runtime of the target at once; a caller able to rewrite any one of
/// those inputs afterwards would leave a report whose evidence and whose conclusion no longer
/// matched. Read-only access costs a consumer nothing and keeps the record faithful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutcome {
    compiler: Compiler,
    target: Target,
    opt: OptLevel,
    argv: Vec<String>,
    command_line: String,
    spawned_argv: Vec<String>,
    timeout_tool_used: bool,
    budget: Duration,
    status: Option<ExitStatus>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_integrity: CaptureIntegrity,
    stderr_integrity: CaptureIntegrity,
    source: PathBuf,
    artifact: PathBuf,
    artifact_size: Option<u64>,
    duration: Duration,
    notes: Vec<String>,
    failure: Option<BuildFailure>,
}

impl CompileOutcome {
    /// Which compiler produced this outcome.
    pub fn compiler(&self) -> Compiler {
        self.compiler
    }

    /// The target this build emitted for.
    pub fn target(&self) -> Target {
        self.target
    }

    /// The optimization level this build was performed at.
    pub fn opt(&self) -> OptLevel {
        self.opt
    }

    /// The compiler invocation, one element per argument, with no wrapper.
    ///
    /// This is the vector the recorded command template expands to and the one a maintainer
    /// reproduces. Where a timeout utility was used it wraps this vector rather than appearing
    /// in it — see [`CompileOutcome::spawned_command_line`].
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// The compiler invocation as one copy-pasteable shell line.
    ///
    /// Each element is quoted so that pasting the line reconstructs [`CompileOutcome::argv`]
    /// exactly: a path containing a space becomes one argument rather than two, and a path
    /// containing shell grammar is passed as data rather than executed. This is the line written
    /// into a cell's recorded commands and into a finding's reproduction script.
    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    /// The vector actually spawned, as one shell line: [`CompileOutcome::argv`] prefixed with the
    /// system timeout utility when the environment provided one.
    ///
    /// Reported separately from [`CompileOutcome::command_line`], and never used as the
    /// reproduction line, because the wrapper is an implementation detail of how this run bounded
    /// the compilation rather than part of the command that reproduces the artifact. A finding
    /// artifact records both: the reproduction line so a maintainer can rebuild the artifact, and
    /// this line so the record states exactly what ran.
    ///
    /// Only the rendered line is exposed. The vector itself has no consumer that the line does not
    /// serve, and handing it out would invite a second, differently quoted rendering of the same
    /// facts.
    pub fn spawned_command_line(&self) -> String {
        posix_command_line(&self.spawned_argv)
    }

    /// Whether the system timeout utility bounded this invocation in addition to the harness's
    /// own watchdog.
    pub fn timeout_tool_used(&self) -> bool {
        self.timeout_tool_used
    }

    /// The timeout utility this build was wrapped in, when it was wrapped in one.
    ///
    /// The first element of the spawned vector, which is where the wrapper sits: a wrapped launch is
    /// `<utility> <seconds> <compiler> …`. Taken from the vector that was **spawned** rather than
    /// from the capability record, for the same reason every other tool path a finding publishes is:
    /// the artifact must state what ran, not what happened to be discovered.
    ///
    /// This exists because a finding whose compiler *refused* the program has no execution, and a
    /// reproduction script that derived its bound only from an execution would then bound nothing —
    /// leaving the one class of finding most likely to hang a reader's shell, a build that never
    /// returns, reproduced without a bound. `None` means the run had no utility available and the
    /// harness's own watchdog was the only bound, which no script can reproduce; the script says so
    /// rather than pretending otherwise.
    pub fn timeout_tool(&self) -> Option<&str> {
        if !self.timeout_tool_used {
            return None;
        }
        self.spawned_argv.first().map(String::as_str)
    }

    /// The per-invocation budget in force.
    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// How the compiler terminated, or `None` when it was killed for outliving its budget or its
    /// status could no longer be observed.
    pub fn status(&self) -> Option<ExitStatus> {
        self.status
    }

    /// The compiler's exit code, or `None` when it was terminated by a signal, timed out, or
    /// could not be waited on.
    ///
    /// A signal is deliberately not folded into a numeric code: the two are different events,
    /// and conflating them is how a crash comes to be reported as an ordinary failure.
    pub fn exit_code(&self) -> Option<i32> {
        self.status.and_then(|status| status.code())
    }

    /// Whether the compiler itself was terminated by a signal.
    ///
    /// True only when a status was observed and it carries no exit code, which on this platform
    /// is precisely signal termination. A compiler that crashes on a valid program is a defect
    /// worth surfacing, and it looks nothing like a compiler that rejected the program.
    pub fn terminated_by_signal(&self) -> bool {
        self.status
            .is_some_and(|status| status.code().is_none() && !self.timed_out)
    }

    /// Whether this invocation outlived its budget.
    pub fn timed_out(&self) -> bool {
        self.timed_out
    }

    /// The compiler's standard output, as raw bytes.
    ///
    /// Written verbatim into a finding artifact's `.compile.stdout` entry, and **never compared**:
    /// the streams that decide a verdict are those of the *program*, not of the compiler that built
    /// it. A compiler ordinarily prints nothing here, and the entry is written even when it is
    /// empty, because a reproduction script redirects this stream to a file of the same name and a
    /// maintainer comparing the two needs both sides to exist.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// The compiler's diagnostics, as raw bytes.
    ///
    /// Kept verbatim for finding artifacts and **never compared between compilers**: wording
    /// legitimately differs between implementations, so comparing it would produce a flood of
    /// divergences that say nothing about code correctness.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// The diagnostics as text, with invalid bytes replaced.
    ///
    /// For display and for the signature tests only. The raw bytes remain available through
    /// [`CompileOutcome::stderr`], and it is those that a finding artifact records.
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// How faithfully [`CompileOutcome::stdout`] represents what the compiler wrote there.
    pub fn stdout_integrity(&self) -> CaptureIntegrity {
        self.stdout_integrity
    }

    /// How faithfully [`CompileOutcome::stderr`] represents what the compiler wrote there.
    ///
    /// The stream every diagnostic arrives on, so this is the value that decides whether an
    /// attribution read out of that text is a fact or a guess. [`CompileOutcome::failure`] is
    /// reported at [`FailureScope::Indeterminate`] when a scope would otherwise have been derived
    /// from diagnostics this says were not fully read.
    pub fn stderr_integrity(&self) -> CaptureIntegrity {
        self.stderr_integrity
    }

    /// Whether the diagnostics were captured whole.
    ///
    /// The precondition for reading a compiler's intent out of its own words. False means the
    /// stream was truncated by the retention quota or its reader did not finish, either of which
    /// can hide the one line that would have named the real cause.
    pub fn diagnostics_complete(&self) -> bool {
        self.stderr_integrity.complete()
    }

    /// Everything that fell short of the ideal while this build was watched and collected.
    ///
    /// Each entry is a fact about the machine — a cleanup that could not be completed, a stream
    /// that could not be read to its end, a status that stopped being observable, a process group
    /// that could not be swept or still held processes after the sweep. Reported rather than
    /// absorbed, because each one changes how the evidence beside it should be read: a surviving
    /// sub-process holds a pipe and can outlive the run, and a suite that quietly accumulated them
    /// would slow down and eventually fail for reasons no report explained.
    ///
    /// Kept out of [`CompileOutcome::stderr`] deliberately, so the bytes a finding artifact records
    /// as the compiler's own diagnostics contain nothing this harness wrote.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// The program this build compiled.
    ///
    /// Carried as its own field rather than recovered from [`CompileOutcome::argv`], because
    /// recovering it means guessing: the only textual cue is the `.c` suffix, and the vector's first
    /// element is a compiler path that a maintainer is entitled to have named `…/cc-13.c` or to have
    /// installed in a directory ending that way. A finding's reproduction script substitutes this
    /// argument with a path beside the script, so guessing wrong replaces the *compiler* with the
    /// reproducer and the script silently reproduces nothing — while still reading, line for line,
    /// like the invocation the run performed.
    ///
    /// The request that produced this outcome already had the answer as a typed path, and it has been
    /// through the containment and regular-file checks [`CompileRequest`] performs on construction, so
    /// keeping it costs one `PathBuf` and removes the guess entirely.
    pub fn source(&self) -> &Path {
        &self.source
    }

    /// Where the artifact was asked to be written.
    pub fn artifact(&self) -> &Path {
        &self.artifact
    }

    /// The artifact's size in bytes, or `None` when no regular file was there.
    ///
    /// Recorded so that "it exited successfully and produced nothing" is visible as the distinct
    /// failure it is, rather than surfacing later as an execution that could not start. Every
    /// consumer needs that three-way answer — absent, present but empty, or present with content —
    /// so existence is read from this one value rather than probed again, and no separate
    /// existence predicate is offered that could disagree with it or change if something later
    /// removed the file.
    pub fn artifact_size(&self) -> Option<u64> {
        self.artifact_size
    }

    /// Wall-clock time the compilation took.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Whether this build produced a usable artifact.
    ///
    /// Exactly the absence of a failure, so the two answers cannot disagree: true when the
    /// compiler terminated successfully within its budget and left a non-empty regular file at
    /// the artifact path, and false in every other case.
    pub fn succeeded(&self) -> bool {
        self.failure.is_none()
    }

    /// Why the build did not produce a usable artifact, when it did not.
    ///
    /// A failure here is an observation for the classifier, not an error: a program one compiler
    /// rejects and the other accepts is precisely the asymmetry the suite exists to surface, and
    /// it may well be an expected divergence against a documented limitation.
    pub fn failure(&self) -> Option<&BuildFailure> {
        self.failure.as_ref()
    }

    /// The divergence class this outcome contributes, when it failed.
    ///
    /// A convenience for the classifier, which asks this question far more often than it asks
    /// for the whole failure.
    pub fn divergence_class(&self) -> Option<DivergenceClass> {
        self.failure.as_ref().map(BuildFailure::class)
    }

    // # Why there is no `is_environment_failure` predicate here
    //
    // For the reason given on [`BuildFailure`]: a boolean cannot carry a three-way judgement, and
    // the reading it forces on its callers is the damaging one. A caller deciding whether to raise
    // a finding reads [`CompileOutcome::failure`] and matches that failure's own
    // [`BuildFailure::scope`], which the compiler checks for exhaustiveness. The one predicate
    // this type does offer — [`CompileOutcome::is_harness_failure`] below — answers a different
    // question: whether this process's own bookkeeping failed, which every caller must ask first.

    /// Whether the failure, if any, is a fault in this process's own bookkeeping.
    ///
    /// False for a successful build. A caller deciding a verdict must consult this *first*: a
    /// compilation whose result could not be established says nothing about the compiler and
    /// nothing about the machine, so it is neither a candidate finding nor an environment gap. It
    /// is this suite failing to observe a cell it launched, and the only honest report of that is
    /// a failure.
    pub fn is_harness_failure(&self) -> bool {
        self.failure.as_ref().is_some_and(BuildFailure::is_harness)
    }

    /// The structured account of this build, as the fields a report column carries.
    ///
    /// # Why the build layer supplies this rather than the report layer deriving it
    ///
    /// Everything here is already held in this type — the exact argument vector, the termination, the
    /// entry names its captured diagnostics were written under — and none of it used to reach a
    /// report. A refused build was reduced to a single `summary` line at the classifier boundary, so a
    /// row reading `XFAIL` or `FINDING` for a compile refusal named the shape of the refusal and not
    /// one fact a maintainer could act on: not the command, not how the compiler ended, not where its
    /// diagnostics were kept. Deriving it here rather than at the sink keeps the knowledge of this
    /// type's shape inside this file, which is the only place that shape can change.
    ///
    /// A refused build has a subject and no authority: no artifact was produced, so nothing was judged
    /// against anything. That is why this returns a one-sided provenance rather than an empty pair.
    ///
    /// `command_line` is the vetted argument vector as it was assembled, not the spawned form: the
    /// spawned form may carry a timeout wrapper this suite added, and a maintainer reproducing the
    /// cell wants the compiler's own line. The wrapper is visible in the workspace's status record
    /// beside the captures this points at.
    pub fn provenance(&self) -> Provenance {
        let (_, stderr_name, status_name) = self.compiler.compile_evidence_names();
        Provenance::of_subject(SideRecord::new(
            format!(
                "{} building for {} at {}",
                self.compiler.label(),
                self.target.triple(),
                self.opt.flag()
            ),
            public_text(&self.command_line),
            self.describe(),
            format!("{stderr_name} and {status_name} in this cell's workspace"),
        ))
    }

    /// One line describing this outcome for a report or a failure message.
    ///
    /// **Deliberately free of any measured time.** This line reaches a report file and a finding
    /// manifest, both of which must be byte-identical across two runs of the same inputs so that a
    /// difference between them means a difference in behaviour. An elapsed-millisecond figure
    /// varies with machine load on every run, so including it would make every artifact differ
    /// from every other artifact and destroy exactly the comparability the records exist for.
    /// Timing is genuinely useful — for progress output, where a slow cell is worth noticing — and
    /// [`CompileOutcome::describe_with_timing`] is where it is available.
    pub fn describe(&self) -> String {
        let termination = match (&self.failure, self.status) {
            (Some(failure), _) => failure.to_string(),
            (None, Some(status)) => format!("succeeded ({status})"),
            (None, None) => String::from("succeeded"),
        };
        let size = match self.artifact_size {
            Some(bytes) => format!("{bytes} byte artifact"),
            None => String::from("no artifact"),
        };
        let mut line = format!(
            "{} {} {}: {termination}; {size}",
            self.compiler.label(),
            self.target.triple(),
            self.opt.flag(),
        );
        // Appended only when a capture fell short, so a complete capture — the ordinary case —
        // contributes nothing and the line stays stable.
        line.push_str(
            &self
                .stdout_integrity
                .describe("the compiler's standard output"),
        );
        line.push_str(&self.stderr_integrity.describe("the compiler's diagnostics"));
        for note in self.notes() {
            line.push_str("; ");
            line.push_str(note);
        }
        line.push_str("; ");
        line.push_str(&self.command_line);
        // The command line is the one part of this description that carries absolute paths — the
        // workspace, the program, the driver — and this line goes to the console as progress and into
        // report rows, so it is published rather than kept. `public_text` redacts any credential in it
        // and elides the checkout's own location; what a maintainer re-runs is rendered from the
        // argument vector by `commands.sh`, never from this description, so nothing reproducible is
        // lost.
        public_text(&line)
    }

    /// The same line with the measured duration appended, for progress output only.
    ///
    /// Separated from [`CompileOutcome::describe`] rather than offered as an option, because the
    /// distinction being enforced is *where the text may go*: this variant is safe on a terminal
    /// and never safe in a file whose bytes are compared. Keeping them as two named methods makes
    /// the wrong choice visible at the call site instead of hiding it in an argument.
    pub fn describe_with_timing(&self) -> String {
        format!("{} ({} ms)", self.describe(), self.duration.as_millis())
    }

    /// Write this build's evidence into the cell workspace, for a retained failure to be read from.
    ///
    /// A workspace kept after a failure is the maintainer's primary evidence, and until now it
    /// held the artifact and the command lines but not the build that produced them. The four
    /// files written here close that gap: the compiler's own two streams verbatim, the raw
    /// termination facts, and the fidelity of each capture, so a truncated diagnostic listing
    /// cannot be mistaken for a short one.
    ///
    /// Every write goes through the workspace's publisher, which refuses to follow a symbolic link
    /// planted at a destination name, so a compiler that emitted a link into its own working
    /// directory cannot redirect this evidence outside the cell.
    ///
    /// # Errors
    ///
    /// A name that escapes the workspace, a destination that is not a regular file, or an
    /// underlying write failure — each of which is a condition of the machine and is reported
    /// rather than absorbed, because evidence that was silently not written is worse than evidence
    /// that was never promised.
    pub fn persist(&self, workspace: &Workspace) -> HarnessResult<()> {
        let (stdout_name, stderr_name, status_name) = self.compiler.compile_evidence_names();
        workspace.write(stdout_name, self.stdout())?;
        workspace.write(stderr_name, self.stderr())?;
        workspace.write_text(status_name, &self.status_record())?;
        Ok(())
    }

    /// The build's termination and capture facts, as a line-oriented record.
    ///
    /// Line-oriented and free of measured time for the same reason [`CompileOutcome::describe`] is:
    /// a maintainer comparing two retained workspaces must see only the differences that mean
    /// something. Every value here is either a decision the compiler made or a fact about how
    /// faithfully it was observed.
    fn status_record(&self) -> String {
        let mut record = String::new();
        record.push_str(&format!("compiler = {}\n", self.compiler.label()));
        record.push_str(&format!("target = {}\n", self.target.triple()));
        record.push_str(&format!("opt = {}\n", self.opt.flag()));
        record.push_str(&format!("command = {}\n", self.command_line));
        record.push_str(&format!("spawned = {}\n", self.spawned_command_line()));
        record.push_str(&format!(
            "timeout_tool_used = {}\n",
            self.timeout_tool_used()
        ));
        record.push_str(&format!("budget_secs = {}\n", self.budget.as_secs()));
        record.push_str(&format!(
            "exit_code = {}\n",
            match self.exit_code() {
                Some(code) => code.to_string(),
                None => String::from("none"),
            }
        ));
        record.push_str(&format!(
            "terminated_by_signal = {}\n",
            self.terminated_by_signal()
        ));
        record.push_str(&format!("timed_out = {}\n", self.timed_out));
        record.push_str(&format!("artifact = {}\n", self.artifact.display()));
        record.push_str(&format!("artifact_exists = {}\n", self.artifact_exists()));
        record.push_str(&format!(
            "artifact_size = {}\n",
            match self.artifact_size {
                Some(bytes) => bytes.to_string(),
                None => String::from("none"),
            }
        ));
        record.push_str(&self.stdout_integrity().record_lines("stdout"));
        record.push_str(&self.stderr_integrity().record_lines("stderr"));
        record.push_str(&format!("succeeded = {}\n", self.succeeded()));
        match self.failure.as_ref() {
            Some(failure) => {
                record.push_str(&format!("failure_scope = {}\n", failure.scope()));
                record.push_str(&format!("failure_class = {:?}\n", failure.class()));
                record.push_str(&format!("failure = {}\n", failure.summary()));
            }
            None => record.push_str("failure = none\n"),
        }
        for note in self.notes() {
            record.push_str(&format!("note = {note}\n"));
        }
        record
    }
    /// Whether a regular file appeared at the artifact path.
    ///
    /// Derived from the recorded size rather than probed again, so this answer and
    /// [`CompileOutcome::artifact_size`] can never disagree, and neither changes if something
    /// later removes the file.
    pub fn artifact_exists(&self) -> bool {
        self.artifact_size.is_some()
    }
}

/// Build one cell.
///
/// Assembles the argument vector, proves it satisfies the shared-flag discipline, proves it is
/// exactly what the program's own expectation record says it should be, runs it once inside the
/// cell's workspace under a bounded wait, and reports what happened.
///
/// # Errors
///
/// Only for a defect in the suite, the corpus or the machine's basic ability to start a process:
/// an argument that the flag discipline forbids, an assembled vector that disagrees with the
/// recorded command template, a compiler the discovery layer could not resolve, a path that
/// cannot be rendered as text without loss, or a child that could not be spawned at all. Each
/// carries the full command line, so a failure is actionable without re-running.
///
/// A compiler that *ran* and rejected the program is **not** an error: it is reported through
/// [`CompileOutcome::failure`], because a program one implementation refuses and another accepts
/// is exactly the asymmetry this suite exists to surface.
pub fn build(request: &CompileRequest<'_>, caps: &Capabilities) -> HarnessResult<CompileOutcome> {
    let context = format!(
        "building the {} artifact for {}",
        request.compiler().label(),
        request.cell_label()
    );
    require_minimal_flag_table_agreement(&context)?;
    require_distinct_evidence_names(&context)?;

    let program = resolve_compiler(request, caps, &context)?;
    let argv = assemble_argv(request, &program, &context)?;
    enforce_flag_discipline(
        &argv,
        request.side(),
        &context,
        "the assembled argument vector",
    )?;
    enforce_target_selection_discipline(&argv, request, &context)?;
    cross_check_against_template(request, &argv, &context)?;

    let budget = Duration::from_secs(caps.config().timeout_secs());
    // This module's own watchdog is the authoritative bound, and it is set at exactly the
    // configured budget. The system utility, when present, is given the budget plus
    // [`TIMEOUT_UTILITY_OUTER_MARGIN`], so it can only ever act after the watchdog has already
    // acted and failed to stop the child.
    //
    // The earlier arrangement was the reverse — the utility first, the watchdog as a backstop —
    // and it had to be inverted. Reading a timeout off the utility means reading it off an exit
    // status, and an exit status is a number the compiler under test is equally entitled to
    // choose. The installed utility passes its child's status through verbatim, so a compiler
    // exiting 124, 125, 126 or 127 was indistinguishable from the utility reporting an expiry or
    // a failure of its own; the second of those became an `Environment` attribution, which passes
    // by default. With the watchdog authoritative, a timeout is a fact this process observed — it
    // reached its own deadline and terminated the child — and no exit status is interpreted
    // anywhere in this module.
    let (spawned_argv, timeout_tool_used) =
        wrap_with_timeout_tool(&argv, caps, budget + TIMEOUT_UTILITY_OUTER_MARGIN);
    let capture = spawn_bounded(&spawned_argv, request.workspace().root(), budget, &context)?;

    let artifact = request.output().to_path_buf();
    let artifact_size = regular_file_size(&artifact);
    // Solely the watchdog's own observation. Nothing about the child's exit status contributes.
    let timed_out = capture.timed_out;
    let mut outcome = CompileOutcome {
        compiler: request.compiler(),
        target: request.target(),
        opt: request.opt(),
        command_line: posix_command_line(&argv),
        argv,
        spawned_argv,
        timeout_tool_used,
        budget,
        status: capture.status,
        timed_out,
        stdout: capture.stdout,
        stderr: capture.stderr,
        stdout_integrity: capture.stdout_integrity,
        stderr_integrity: capture.stderr_integrity,
        source: request.source().to_path_buf(),
        artifact,
        artifact_size,
        duration: capture.duration,
        notes: capture.notes,
        failure: None,
    };
    // Classified once, with the status, the diagnostics, the artifact and the target's probed C
    // runtime all in front of it, so the judgement and the evidence recorded beside it can never
    // have been reached from different inputs.
    outcome.failure = classify_failure(&outcome, request, caps);
    Ok(outcome)
}

/// Assert that this module and the shared minimal flag table still describe the same set.
///
/// Both directions are checked, and the second is the one that earns its keep: a flag added to
/// the table would otherwise be silently ignored here, leaving the table claiming a discipline
/// the invocations do not implement. Two comparisons per invocation is nothing against a
/// disagreement nobody would notice.
fn require_minimal_flag_table_agreement(context: &str) -> HarnessResult<()> {
    let emitted = [FLAG_OUTPUT, FLAG_STATIC];
    for flag in DIFFERENTIAL_FLAGS_MINIMAL {
        if !emitted.contains(flag) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the minimal differential flag set names {flag:?}, which this module never \
                     passes; the table and the argument builder must describe one set, so either \
                     pass it here or remove it there"
                ),
            ));
        }
    }
    for flag in emitted {
        if !DIFFERENTIAL_FLAGS_MINIMAL.contains(&flag) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "this module passes {flag:?}, which the minimal differential flag set does \
                     not name; every flag reaching a differential invocation is verified against \
                     that set, so one written only here would bypass the verification"
                ),
            ));
        }
    }
    Ok(())
}

/// Assert that no two compilers write their build evidence under the same workspace name.
///
/// # Why this is checked rather than assumed
///
/// A cell workspace holds both compilers' evidence side by side, and the two name triples come from
/// two separate groups of constants in the workspace vocabulary. A single transposed prefix there —
/// one `bcc.` where a `ref.` belonged — would make the reference build's diagnostics overwrite the
/// subject's in every one of the 1,296 cells, and the failure would be invisible: each file would
/// exist, be non-empty and contain real compiler output, just the wrong compiler's. A maintainer
/// investigating a retained cell would read the reference compiler's reasoning and attribute it to
/// the compiler under test.
///
/// Every pairing is compared, over [`Compiler::ALL`], so a compiler added later is included without
/// this function being revisited. Nine string comparisons per invocation is nothing against a
/// mistake that would silently corrupt the evidence trail of an entire run.
fn require_distinct_evidence_names(context: &str) -> HarnessResult<()> {
    for (index, compiler) in Compiler::ALL.iter().enumerate() {
        let (stdout, stderr, status) = compiler.compile_evidence_names();
        for other in &Compiler::ALL[index + 1..] {
            let (other_stdout, other_stderr, other_status) = other.compile_evidence_names();
            for mine in [stdout, stderr, status] {
                for theirs in [other_stdout, other_stderr, other_status] {
                    if mine == theirs {
                        return Err(HarnessError::new(
                            String::from(context),
                            format!(
                                "the {} and the {} would both write build evidence to {mine:?} in \
                                 the same cell workspace, so one would overwrite the other and a \
                                 retained cell would attribute one compiler's diagnostics to the \
                                 other; the workspace vocabulary must give every compiler its own \
                                 three names",
                                compiler.label(),
                                other.label()
                            ),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Resolve the binary this build must spawn, and prove the recorded command names the same one.
///
/// The discovery layer is the authority on both paths: it vetted them, and it is what decides
/// whether an oracle arm can run at all. The substitutions the recorded command template expands
/// from must therefore name exactly the same binary, textually — not merely the same file. A
/// difference of spelling would be enough to make the reproduction line a maintainer copies out
/// of a report name a different path from the one that ran, which is the single guarantee the
/// recorded commands exist to provide.
fn resolve_compiler(
    request: &CompileRequest<'_>,
    caps: &Capabilities,
    context: &str,
) -> HarnessResult<PathBuf> {
    match request.compiler() {
        Compiler::Bcc => {
            let record = caps.bcc();
            let Some(resolved) = record.path() else {
                return Err(HarnessError::new(
                    String::from(context),
                    format!(
                        "the compiler under test is not available, so there is nothing to build \
                         with: {}",
                        record.diagnosis()
                    ),
                ));
            };
            require_same_path(
                context,
                "compiler under test",
                resolved,
                request.substitutions().bcc(),
            )?;
            Ok(resolved.to_path_buf())
        }
        Compiler::Reference => {
            let target = request.target();
            let Some(resolved) = caps.ref_cc_for(target) else {
                return Err(HarnessError::new(
                    String::from(context),
                    format!(
                        "no reference compiler driver is available for {}, so oracle (a) has no \
                         arm to run for this cell; that arm is reported unavailable by the \
                         discovery layer and must not be built. The reference compiler has no \
                         target-selection flag, so a target is selected there by choosing a \
                         different driver binary",
                        target.triple()
                    ),
                ));
            };
            let Some(declared) = request.substitutions().reference_compiler() else {
                return Err(HarnessError::new(
                    String::from(context),
                    String::from(
                        "these substitutions carry no reference compiler driver; supply it with \
                         `CommandSubstitutions::with_reference_compiler` before describing a \
                         reference build",
                    ),
                ));
            };
            require_same_path(context, "reference compiler", resolved, declared)?;
            Ok(resolved.to_path_buf())
        }
    }
}

/// Require two paths to be the same text, so that what runs and what is recorded cannot differ.
fn require_same_path(
    context: &str,
    role: &str,
    resolved: &Path,
    declared: &Path,
) -> HarnessResult<()> {
    if resolved == declared {
        return Ok(());
    }
    Err(HarnessError::new(
        String::from(context),
        format!(
            "the discovery layer resolved the {role} to {}, but the substitutions this cell's \
             recorded commands are rendered from name {}. One of them would run and the other \
             would be reported, so the difference is refused rather than reconciled: build the \
             substitutions from the discovery layer's own answer — `Capabilities::bcc().path()` \
             for the compiler under test, `Capabilities::ref_cc_for(target)` for the reference \
             compiler",
            shown_path(resolved),
            shown_path(declared)
        ),
    ))
}

/// Assemble the argument vector for one build.
///
/// The shape is fixed and matches the recorded command templates exactly:
///
/// - compiler under test — `<bcc> --target <triple> <opt> -static <src> -o <out>`
/// - reference compiler — `<driver> <opt> -static <src> -o <out>`
///
/// The target selection appears on the compiler-under-test path only. It is not conditional on
/// the target being non-native: passing it for the native target too is permitted, names the
/// target the compiler would have chosen anyway, and keeps one recorded template correct for
/// every cell of a program. `enforce_target_selection_discipline` then proves the flag is present
/// wherever it is *required*, and absent wherever it is forbidden.
fn assemble_argv(
    request: &CompileRequest<'_>,
    program: &Path,
    context: &str,
) -> HarnessResult<Vec<String>> {
    let compiler = request.compiler();
    let mut argv: Vec<String> = Vec::with_capacity(8);
    argv.push(argument_text(context, compiler.label(), program)?);
    if compiler.selects_target_by_flag() {
        argv.extend(bcc_target_arguments(request.target()));
    }
    argv.push(String::from(request.opt().flag()));
    argv.push(String::from(FLAG_STATIC));
    argv.push(argument_text(context, "program source", request.source())?);
    argv.push(String::from(FLAG_OUTPUT));
    argv.push(argument_text(context, "build artifact", request.output())?);
    Ok(argv)
}

/// Render one path as the exact text that becomes a command-line argument, refusing a path that
/// cannot be rendered without loss.
///
/// Refused rather than converted, because the lossy conversion would produce a *different* path
/// that merely prints plausibly: the recorded reproduction line would name a file that is not the
/// one that was compiled, and the vetting the discovery layer performed would have been performed
/// on a path that never ran.
fn argument_text(context: &str, role: &str, path: &Path) -> HarnessResult<String> {
    match path.to_str() {
        Some(text) => Ok(String::from(text)),
        None => Err(HarnessError::new(
            String::from(context),
            format!(
                "the {role} path {} is not valid UTF-8, so it cannot be written into a recorded \
                 command line without substituting a replacement character for the bytes that do \
                 not decode. It is refused rather than converted: a reproduction command that \
                 names a different file from the one that ran would be worse than no command at \
                 all",
                shown_path(path)
            ),
        )),
    }
}

/// Prove every argument in a vector is one this side is permitted to receive.
///
/// Two independent tests are applied to each argument that looks like a flag, and both must
/// pass. They are not redundant, and the second is the one that matters over time:
///
/// - the **deny-list** asks whether the shared table forbids this spelling on this side. It
///   catches every flag anyone has thought to forbid, in all three of the spellings the table
///   recognises, so `--target=aarch64-linux-gnu` cannot slip past a check written for
///   `--target aarch64-linux-gnu`;
/// - the **allow-list** asks whether this module is supposed to emit this flag at all. It
///   catches every flag nobody has thought to forbid — which is the whole difficulty, because a
///   flag whose meanings differ subtly between two implementations is exactly the flag that was
///   never suspected. A maintainer who adds an argument here has to add it to the allow-list
///   too, and that is the moment the question "do both compilers honour this identically?" gets
///   asked.
///
/// An argument that is a *value* rather than a flag is skipped, because it is data: the triple
/// after the target selector and the path after the output selector are not spellings this
/// discipline governs. [`flag_takes_separate_value`] is what identifies them, so a path that
/// happens to begin with a hyphen is treated as the path it is.
///
/// # Errors
///
/// A forbidden or unrecognised flag, naming the flag, the side, the reason and the full command
/// line. This is a suite defect rather than an environment condition, so it is refused before
/// anything is spawned rather than reported as a divergence: a comparison performed with
/// mismatched flags would be evidence about nothing at all.
fn enforce_flag_discipline(
    argv: &[String],
    side: CompilerSide,
    context: &str,
    description: &str,
) -> HarnessResult<()> {
    let mut expecting_value = false;
    for argument in argv.iter().skip(1) {
        if expecting_value {
            expecting_value = false;
            continue;
        }
        if !is_flag_argument(argument) {
            continue;
        }
        if is_forbidden_for_side(argument, side) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "{description} carries {argument:?}, which must never be passed to the \
                     {side}. Either both compilers do not accept it, or they accept it and do \
                     not mean the same thing by it, so a comparison performed with it would be \
                     evidence about the flag rather than about the code. The warning and \
                     sanitizer spellings are refused here because they belong to the \
                     undefined-behaviour audit alone, where only the reference compiler is \
                     driven and no comparison is made. The vector was: {}",
                    posix_command_line(argv)
                ),
            ));
        }
        if !is_permitted_flag(argument, side) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "{description} carries {argument:?}, which is not one of the flags a \
                     differential invocation is permitted to pass. The permitted set is \
                     deliberately minimal and closed: {:?} for the artifact, one of {:?} for the \
                     optimization level, {:?} for the linkage, and {:?} on the \
                     compiler-under-test side only. A flag absent from that set is refused even \
                     though nothing forbids it by name, because the flags that quietly differ \
                     between two implementations are precisely the ones nobody suspected: adding \
                     one here means first establishing that both compilers honour it with the \
                     same meaning. The vector was: {}",
                    FLAG_OUTPUT,
                    OptLevel::ALL.map(OptLevel::flag),
                    FLAG_STATIC,
                    BCC_TARGET_FLAG,
                    posix_command_line(argv)
                ),
            ));
        }
        expecting_value = flag_takes_separate_value(argument);
    }
    Ok(())
}

/// True when an argument is a flag rather than a value.
///
/// A lone hyphen is not a flag — it is the conventional name for standard input — and neither is
/// an empty argument. Everything else beginning with a hyphen is treated as a flag and is
/// therefore subject to the discipline, which is the safe direction to err in: a value mistaken
/// for a flag is refused loudly, whereas a flag mistaken for a value would pass unexamined.
fn is_flag_argument(argument: &str) -> bool {
    argument.len() > 1 && argument.starts_with('-')
}

/// True when a flag's value is a separate following argument rather than part of the flag.
///
/// Only the two spellings this module emits in that form are listed, because only they can
/// occur. The consequence is narrow and worth stating: the argument after one of these is
/// skipped by the flag guard, so a workspace or corpus path that begins with a hyphen is
/// carried through as the path it is instead of being rejected as an unrecognised flag.
fn flag_takes_separate_value(flag: &str) -> bool {
    flag == FLAG_OUTPUT || flag == BCC_TARGET_FLAG
}

/// True when this module is permitted to pass `flag` to `side`.
///
/// The closed set, and the reason each member is in it:
///
/// | Flag | Why it is admissible |
/// | --- | --- |
/// | [`FLAG_OUTPUT`] | Both compilers name the artifact this way, and the suite must control where it lands. |
/// | [`OptLevel::flag`] | `-O0`, `-O1` and `-O2` are the three levels both compilers honour. Nothing else is offered, so nothing else can be swept in by a wildcard. |
/// | [`FLAG_STATIC`] | The one linkage mode both compilers spell identically, and what makes an emulated cross-target binary self-contained. |
/// | [`BCC_TARGET_FLAG`] | Admissible on the compiler-under-test side alone, which has no cross drivers and no other route to a non-native backend. |
///
/// Nothing else is admissible from this module, including spellings that are perfectly
/// legitimate elsewhere in the suite: the sysroot selector is honoured by the compiler under
/// test and is never needed here, and the whole verified shared set is wider than this. Narrower
/// than necessary is the right choice for the one function that decides what a comparison is
/// performed with.
fn is_permitted_flag(flag: &str, side: CompilerSide) -> bool {
    if flag == FLAG_OUTPUT || flag == FLAG_STATIC {
        return true;
    }
    if OptLevel::ALL.iter().any(|level| level.flag() == flag) {
        return true;
    }
    flag == BCC_TARGET_FLAG && matches!(side, CompilerSide::UnderTest)
}

/// Prove the target was selected the way this compiler selects targets, and only that way.
///
/// The asymmetry enforced here is the one that makes cross-target testing possible at all, and
/// it is worth being explicit about why it is not a loophole. The requirement that both
/// compilers honour a flag identically constrains the comparison against an *independent*
/// implementation. A target selector reaches one implementation only:
///
/// - the **reference compiler** has no target-selection flag. Its cross arms are separate driver
///   binaries, so a selector appearing on that side would be rejected outright by the driver, or
///   — worse — accepted by a different implementation that spells it the same way and quietly
///   mean something else. It is refused here.
/// - the **compiler under test** is a single binary serving four backends, so the selector is its
///   only route to a non-native one. Absent it, a cell nominally testing a cross target would
///   compile for the host, and the comparison would report agreement between two host binaries
///   while the backend under test was never exercised. That is a false pass, and it is silent,
///   which is why the presence of the selector is *required* here rather than merely allowed
///   wherever the target demands it.
///
/// The selection is also proved to name **this cell's** target. A selector naming a different
/// triple would produce an artifact for the wrong architecture under a record and a report that
/// both described the right one.
///
/// # Errors
///
/// A selector on the reference side, a missing selector where one is required, a selector naming
/// the wrong triple, a selector with no triple after it, or more than one selector. Each names
/// the full command line.
fn enforce_target_selection_discipline(
    argv: &[String],
    request: &CompileRequest<'_>,
    context: &str,
) -> HarnessResult<()> {
    let selectors = argv
        .iter()
        .skip(1)
        .filter(|argument| is_bcc_target_selector(argument))
        .count();
    let target = request.target();
    if !request.compiler().selects_target_by_flag() {
        if selectors == 0 {
            return Ok(());
        }
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the reference compiler invocation carries a target-selection argument, which \
                 that compiler does not have: it selects a target by *being* a different driver \
                 binary, so the driver for {} must be resolved with `Capabilities::ref_cc_for` \
                 instead. A selector here would either be rejected by the driver or, if some \
                 other implementation accepted the same spelling, would silently mean something \
                 else. The vector was: {}",
                target.triple(),
                posix_command_line(argv)
            ),
        ));
    }
    if selectors > 1 {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the compiler-under-test invocation carries {selectors} target-selection \
                 arguments; exactly one selection may be in force, because which of several the \
                 compiler would honour is not something a report could state. The vector was: {}",
                posix_command_line(argv)
            ),
        ));
    }
    let position = argv.iter().position(|argument| argument == BCC_TARGET_FLAG);
    let Some(position) = position else {
        if !bcc_requires_explicit_target(target) {
            return Ok(());
        }
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the compiler-under-test invocation for {} carries no {BCC_TARGET_FLAG} \
                 argument, and that target is not the host. The compiler under test is one \
                 binary serving four backends, so without the selection it would emit for the \
                 host: the cell would compare a host binary against a cross-compiled one and \
                 report agreement, having never exercised the backend it names. The vector was: \
                 {}",
                target.triple(),
                posix_command_line(argv)
            ),
        ));
    };
    match argv.get(position + 1) {
        Some(triple) if triple == target.triple() => Ok(()),
        Some(triple) => Err(HarnessError::new(
            String::from(context),
            format!(
                "the compiler-under-test invocation selects the target {triple:?}, but this cell \
                 is {}. The artifact would be built for one architecture while the record it is \
                 checked against, and the report it appears in, both described another. The \
                 vector was: {}",
                target.triple(),
                posix_command_line(argv)
            ),
        )),
        None => Err(HarnessError::new(
            String::from(context),
            format!(
                "the compiler-under-test invocation ends with {BCC_TARGET_FLAG} and no triple \
                 after it, so nothing selects the target for {}. The vector was: {}",
                target.triple(),
                posix_command_line(argv)
            ),
        )),
    }
}

/// Prove the vector about to be spawned is exactly what the program's own record says it is.
///
/// This is the check that keeps a recorded reproduction command truthful. Each program ships an
/// expectation record carrying the literal command template for both compilers, and a maintainer
/// is promised that the program source plus that record reproduce any cell with no harness at
/// all. The promise is only worth something while the harness and the record agree — and nothing
/// about a divergence between them would be visible in a passing run, because the harness would
/// go on comparing artifacts built its own way and reporting them under commands nobody had run.
/// A maintenance edit to the argument builder would, silently, turn every record in the corpus
/// into a work of fiction.
///
/// So the two are compared element for element on every invocation. The cost is one template
/// expansion; the alternative is a deliverable that is wrong in a way no test can see.
///
/// Comparison is by argument rather than by rendered line deliberately: two different vectors can
/// render to the same line once quoting is applied, and it is the vector that is executed.
///
/// # Errors
///
/// A length difference or the first differing element, naming the index, both spellings and both
/// full command lines, so the correction is obvious from the message. Also any failure to expand
/// the template, which means the record names a placeholder the substitutions cannot satisfy.
fn cross_check_against_template(
    request: &CompileRequest<'_>,
    argv: &[String],
    context: &str,
) -> HarnessResult<()> {
    let manifest = request.manifest();
    let substitutions = request.substitutions();
    let (template, recorded) = match request.compiler() {
        Compiler::Bcc => (
            manifest.bcc_command(),
            manifest.render_bcc_argv(substitutions)?,
        ),
        Compiler::Reference => (
            manifest.ref_command(),
            manifest.render_ref_argv(substitutions)?,
        ),
    };
    for (index, (assembled, expected)) in argv.iter().zip(recorded.iter()).enumerate() {
        if assembled == expected {
            continue;
        }
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the assembled {} invocation differs from the one this program's record \
                 describes, at argument {index}: this module would pass {assembled:?} where the \
                 record's template expands to {expected:?}. The record is the reproduction \
                 recipe a maintainer is promised, so the two must be one command and the \
                 difference is refused rather than preferred one way or the other. Template in \
                 {}: {template:?}. Assembled: {}. Recorded: {}",
                request.compiler().label(),
                shown_path(manifest.path()),
                posix_command_line(argv),
                posix_command_line(&recorded)
            ),
        ));
    }
    if argv.len() == recorded.len() {
        return Ok(());
    }
    Err(HarnessError::new(
        String::from(context),
        format!(
            "the assembled {} invocation has {} arguments and the one this program's record \
             describes has {}, agreeing on every argument they share. The record is the \
             reproduction recipe a maintainer is promised, so a command that runs with arguments \
             the recipe omits — or omits arguments the recipe carries — would make the recipe \
             untrue in a way no comparison could reveal. Template in {}: {template:?}. \
             Assembled: {}. Recorded: {}",
            request.compiler().label(),
            argv.len(),
            recorded.len(),
            shown_path(manifest.path()),
            posix_command_line(argv),
            posix_command_line(&recorded)
        ),
    ))
}

/// Prefix an invocation with the system timeout utility when the run engaged it as an outer net.
///
/// Returns the vector to spawn and whether the utility is in it.
///
/// The decision is `caps.outer_net_tool()`, which `env.rs` reached once for the whole run by
/// **measuring** the discovered implementation rather than merely finding it: an outer net that never
/// fires still charges for supervising every child it wraps, and the implementation measured in this
/// environment charges about 103 ms per invocation — roughly 569 s across the matrix, which is more
/// wall time than the suite's own measured run takes (a 345–476 s band on the four-core machine
/// recorded in `tests/conformance/README.md`). An implementation inside the ceiling is engaged and one above it is
/// declined, with the decision and its evidence stated in the pre-flight report and in every
/// finding's environment fingerprint. This function therefore asks one question and never
/// re-litigates it.
///
/// The utility is worth engaging when it is cheap for one reason that matters: an
/// implementation that signals the process group can reach further than this module can. The
/// driver this module spawns starts sub-processes of its own — a preprocessor, a compiler proper,
/// an assembler, a linker — and it is one of *those* that hangs. Terminating only the driver
/// leaves the sub-process running, holding the pipe this module is reading, so a harness that
/// read that pipe to end of file would go on waiting for a process it never started and cannot
/// see.
///
/// That reach is the utility's to provide, and it is neither established nor relied on here.
/// Nothing in this module creates a process group or sends a group-wide signal, and the
/// implementation measured in this environment was defective in two of its three spellings, so a
/// descendant may well survive. What does not depend on the utility is stated exactly: the
/// watchdog in [`spawn_bounded`] remains in force regardless, set to fire later, and it
/// terminates and reaps the one child this module spawned, so the invocation returns within the
/// budget whether the utility acted or not. Reading the captures with a deadline rather than to
/// end of file is what makes a surviving descendant survivable.
///
/// Where the outer net was declined or is absent, or where its path cannot be rendered as text
/// without loss, the vector is returned untouched and the watchdog is the whole bound. That is a
/// complete substitute rather than a degradation of correctness — it bounds the invocation just as
/// surely, it forfeits only a reach the utility might have had, and the child's own process group is
/// swept on every exit path regardless — so it is taken without refusing anything. It is not taken
/// *silently*: the capability report states the decision either way. Reporting the untouched vector
/// is also what keeps [`CompileOutcome::spawned_command_line`] a faithful record of what ran.
fn wrap_with_timeout_tool(
    argv: &[String],
    caps: &Capabilities,
    budget: Duration,
) -> (Vec<String>, bool) {
    let Some(tool) = caps.outer_net_tool() else {
        return (argv.to_vec(), false);
    };
    let Some(program) = tool.to_str() else {
        return (argv.to_vec(), false);
    };
    // A relative or bare name would be resolved against this child's working directory, which is
    // the cell workspace, so only an absolute path is used. The discovery layer resolves the
    // utility to an absolute path; anything else is treated as absent.
    if !tool.is_absolute() {
        return (argv.to_vec(), false);
    }
    let mut wrapped = Vec::with_capacity(argv.len() + 2);
    wrapped.push(String::from(program));
    wrapped.push(budget.as_secs().to_string());
    wrapped.extend(argv.iter().cloned());
    (wrapped, true)
}

/// Everything one spawned child produced, before it is interpreted.
///
/// Deliberately free of judgement: it records what happened and nothing about what it means, so
/// that [`classify_failure`] is the single place an outcome acquires a meaning.
#[derive(Debug)]
struct Capture {
    /// Bytes the child wrote to standard output, capped at [`CAPTURE_BYTES_MAX`].
    stdout: Vec<u8>,
    /// Bytes the child wrote to standard error, capped at [`CAPTURE_BYTES_MAX`].
    stderr: Vec<u8>,
    /// How faithfully `stdout` represents what the child actually wrote there.
    stdout_integrity: CaptureIntegrity,
    /// How faithfully `stderr` represents what the child actually wrote there.
    ///
    /// The stream a compiler's diagnostics arrive on, and therefore the stream every attribution
    /// derived from diagnostic text depends upon. Recorded rather than assumed so that
    /// [`classify_failure`] can decline to attribute a failure it could not fully read.
    stderr_integrity: CaptureIntegrity,
    /// Whether the harness's watchdog killed the child for outliving its bound.
    timed_out: bool,
    /// How the child terminated, or `None` when it was killed or became unobservable.
    status: Option<ExitStatus>,
    /// Wall-clock time from spawn to termination, excluding the time spent collecting output.
    duration: Duration,
    /// Everything that fell short of the ideal while this child was watched and collected.
    ///
    /// A cleanup that could not be completed, a stream that could not be read to its end, a
    /// status that stopped being observable: each is a fact about the machine that would
    /// otherwise be discarded, and each can change how a maintainer reads the outcome beside it.
    notes: Vec<String>,
}

/// One output stream after it has been drained, with the fidelity of that drain recorded.
///
/// The bytes and the account of how they were obtained travel together on purpose. A truncated
/// stream and a complete one are indistinguishable once separated from their byte counts, and a
/// compiler attribution read out of a truncated diagnostic stream is a guess presented as a fact
/// — which is precisely what [`FailureScope::Indeterminate`] exists to prevent.
#[derive(Debug)]
struct StreamHarvest {
    /// The retained bytes, never more than [`CAPTURE_BYTES_MAX`].
    bytes: Vec<u8>,
    /// Total bytes the child wrote to this stream, including any the quota discarded.
    produced: u64,
    /// Whether the quota discarded anything.
    truncated: bool,
    /// Whether the stream was read all the way to its end.
    drained: bool,
    /// Why the drain fell short, when it did.
    note: Option<String>,
}

impl StreamHarvest {
    /// A stream that was never opened, which is not the same as one that was empty.
    ///
    /// Reported as undrained, because nothing was read: describing an absent pipe as a completely
    /// read empty one would let a missing stream masquerade as a silent compiler.
    fn absent(reason: &str) -> Self {
        Self {
            bytes: Vec::new(),
            produced: 0,
            truncated: false,
            drained: false,
            note: Some(String::from(reason)),
        }
    }

    /// The fidelity of this harvest, in the harness-wide form every capture is reported in.
    fn integrity(&self) -> CaptureIntegrity {
        CaptureIntegrity::new(
            self.produced,
            self.bytes.len() as u64,
            self.truncated,
            self.drained,
        )
    }
}

/// Run one invocation inside a cell workspace, bounded, isolated, supervised as a group, with
/// its output captured.
///
/// Seven properties hold together, and each closes a distinct way a compiler invocation can fail
/// to return, can reach outside the directory it was given, or can carry something out of this
/// process that should never have left it:
///
/// - **The environment is replaced, not inherited.** Every variable is cleared and a documented
///   minimal set restored, with the cell workspace as the child's private `HOME` and `TMPDIR`.
///   A compiler driver reads a great many variables — search paths, loader configuration, its own
///   options — and a continuous-integration environment carries credentials beside them. Neither
///   belongs in a hermetic comparison: an inherited search path makes the result depend on the
///   machine rather than on the compiler, and an inherited credential can be printed by a tool
///   and persisted into a finding artifact. Isolation is applied to the command that is actually
///   spawned. That ordering matters here only in the negative sense: this module wraps by
///   rebuilding the **argument vector** and constructs its `Command` once from the final vector,
///   so there is no second `Command` for a clear to be lost from — unlike a design that wraps by
///   rebuilding the command, where `Command::env_clear` is invisible to `Command::get_envs` and a
///   rebuild silently re-inherits everything.
/// - **The child owns its own process group, and the whole group is swept.** A compiler driver is
///   ordinarily a process group: it execs a preprocessor, a compiler proper, an assembler and a
///   linker, and it is one of *those* that hangs. Killing the driver alone leaves the sub-process
///   running, still holding the pipe this module is reading. Every exit path therefore sweeps the
///   whole group after the direct child has been reaped, and records the result: a group that
///   still holds survivors, or one that could not be swept at all, is stated in the outcome
///   rather than passed over.
/// - **The working directory is the cell's own workspace.** Set on the child rather than by
///   changing this process's directory, which would be a data race: the suite's area tests run
///   concurrently by default, and a process-wide change of directory made by one of them would
///   silently relocate every other test's relative paths. It is also what makes a compiler's
///   incidental output — a temporary file, a listing, an artifact written to a bare name — land
///   inside the one directory the cell owns.
/// - **Standard input is the null device**, so a driver that reads it sees end of file at once
///   rather than waiting for input that will never arrive.
/// - **Each output stream is drained by its own thread, bounded** at [`CAPTURE_BYTES_MAX`], and
///   drained *past* the bound so nothing is retained beyond it while the pipe keeps being
///   emptied. Two hazards are closed by this together: a driver that prints without end cannot
///   exhaust memory, and the classic deadlock in which a parent waits for a child that is itself
///   blocked writing into a pipe nobody is reading cannot occur. Stopping the reader at the cap
///   would close the first hazard and reopen the second, so the surplus is read and discarded
///   rather than left in the pipe, and the discarded quantity is reported on the outcome.
/// - **Every vetted tool named in the argument vector is re-confirmed as the last statement before
///   the launch**, so a compiler — or the timeout utility wrapping it — exchanged since pre-flight
///   is refused rather than run. The whole vector is scanned rather than just its first element,
///   because wrapping moves the compiler out of the front position and puts the utility there.
/// - **The child is spawned into its own process group**, and the group — not merely the direct
///   child — is what gets signalled when the watchdog fires. A compiler driver's sub-processes
///   are the ordinary case here, and killing only the driver leaves the assembler or the linker
///   it started running and holding the pipe open.
/// - **The wait is bounded** by polling, because the standard library offers no timed wait on a
///   child, and the child is **always reaped**, so no zombie is left behind on any path. The
///   bound is this module's own and is authoritative: the system utility, when present, is given
///   a strictly later deadline and no status it might report is ever interpreted.
/// - **Collecting the output is bounded too**, and separately from the wait. Terminating a child
///   does not close the pipe it was writing to: any process that inherited the write end still
///   holds it open, and a compiler driver's sub-processes are the ordinary case. Draining to end
///   of file would therefore block on a grandchild this module never started and cannot see. The
///   readers are harvested through a channel with a deadline, and one that has not delivered by
///   then is abandoned rather than waited on — safe and bounded, because its buffer cannot exceed
///   the cap, it holds no lock, and it ends by itself when the last writer closes the pipe.
///
/// The harvest deadline is at least [`CAPTURE_GRACE`] from now, even when the wait has already
/// consumed the whole budget. Without that floor, a compilation that finished legitimately but
/// slowly would have its diagnostics discarded — the one case where the captured text matters
/// most, since a slow build that then failed is exactly what a maintainer needs to read.
///
/// # Errors
///
/// An empty vector, which is a defect in the caller, and a child that could not be spawned at
/// all. The latter names the full command line and the operating system's own reason, which
/// distinguishes a binary that has been removed from one that cannot be executed.
fn spawn_bounded(
    argv: &[String],
    working_directory: &Path,
    bound: Duration,
    context: &str,
) -> HarnessResult<Capture> {
    let Some((program, arguments)) = argv.split_first() else {
        return Err(HarnessError::new(
            String::from(context),
            String::from(
                "an empty argument vector was assembled, so there is no program to spawn; this \
                 is a defect in the argument builder rather than a condition of the machine",
            ),
        ));
    };
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A group of the child's own, so terminating it reaches every sub-process a compiler driver
    // started. Zero requests a new group whose identifier is the child's own process identifier,
    // which is what makes the group addressable from here without a second lookup. Deliberately
    // not the harness's own group: signalling that would kill the test process itself along with
    // every other cell running concurrently beside it.
    //
    // Both are applied to the one command this function spawns, which is built from the
    // already-wrapped vector, so nothing downstream can rebuild it and lose the clear.
    isolate_child_environment(&mut command, working_directory);
    own_process_group(&mut command);
    // The last statement before the launch, deliberately, and it scans the whole vector rather than
    // its first element: when the external timeout utility wraps the build the compiler has moved to
    // the middle of the vector and the utility occupies the front, so both are vetted tools and both
    // are checked here. Anything placed between this and the spawn would reopen the window it exists
    // to narrow.
    if let Some(change) = confirm_vetted_tools_unchanged(argv) {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "a tool this invocation is about to execute is no longer the file discovery \
                 vetted — {change}. Nothing was launched. Every decision the pre-flight report \
                 records about that tool, and the whole comparison this cell would have \
                 contributed to, describes a file that is no longer there, so a build performed \
                 anyway could not be attributed to the compiler the run claims to be testing. The \
                 command was: {}",
                posix_command_line(argv)
            ),
        ));
    }
    // The last question before the launch after the tools have been confirmed, and it is about this
    // run rather than about the files: a run that has already leaked a process, or that holds a
    // capture pipe it cannot prove is closed, must not add another child to the pile. The fault
    // recurs per cell, and the matrix is thousands of cells across concurrent workers.
    if let Some(breach) = infrastructure_breach() {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "{}. The command would have been: {}",
                infrastructure_breach_refusal(breach),
                posix_command_line(argv)
            ),
        ));
    }
    let started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the invocation could not be started at all: {error}. This is the machine \
                     rather than the program under test — a binary that has been removed, or one \
                     that cannot be executed — and it is reported as a failure of the suite \
                     rather than as a divergence, because no compiler ran and therefore nothing \
                     was observed. The command was: {}",
                    posix_command_line(argv)
                ),
            ));
        }
    };
    // The child leads its own group because `own_process_group` was applied before the spawn, so
    // its identifier is also the group's. Read before anything can fail, so that every path below
    // has a group to sweep.
    let group = child.id();
    let deadline = started + bound;
    let stdout_reader = child.stdout.take().map(spawn_capped_reader);
    let stderr_reader = child.stderr.take().map(spawn_capped_reader);
    let watched = await_child_within_deadline(&mut child, deadline, working_directory);
    // Measured before the harvest, so the reported duration is how long the compilation took and
    // not how long its diagnostics took to arrive.
    let duration = started.elapsed();
    // Swept after the direct child has been waited on, never before: a reaped-but-unwaited child
    // is a zombie, a zombie still answers an existence probe, and sweeping first would therefore
    // report a survivor that is nothing of the kind. The sweep is unconditional — a driver that
    // exited normally can still have left a sub-process behind holding the pipe.
    let mut notes: Vec<String> = Vec::new();
    match terminate_process_group(group, kill_tool()) {
        GroupTermination::Cleared => {}
        GroupTermination::Survivors(detail) | GroupTermination::Unsupervised(detail) => {
            notes.push(sanitize_text_for_report(&redact_secrets(&detail)));
        }
    }
    let harvest_deadline = deadline.max(Instant::now() + CAPTURE_GRACE);
    let stdout = harvest_within_deadline(stdout_reader, harvest_deadline, "standard output");
    let stderr = harvest_within_deadline(stderr_reader, harvest_deadline, "standard error");
    notes.extend(watched.notes);
    if let Some(note) = stdout.note.as_ref() {
        notes.push(format!("the compiler's standard output {note}"));
    }
    if let Some(note) = stderr.note.as_ref() {
        notes.push(format!("the compiler's standard error {note}"));
    }
    // Raised only after the group has been swept and both readers harvested, so the refusal reports a
    // finished cleanup rather than causing one to be skipped. A workspace that crossed its live
    // ceiling is not an observation about the compiler's *output*: the child was stopped part-way
    // through writing, so its artifact is a fragment and its diagnostics are a prefix. Comparing
    // either would be reporting a divergence in a program that was never allowed to finish, which is
    // why this is a refusal in the shape `execute.rs` already uses for a capture that overflowed.
    if let Some(overflow) = watched.overflow {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the invocation was terminated because {overflow}. The child and its whole process \
                 group were signalled and reaped, so nothing of it is still writing. This is \
                 reported as a failure of the suite rather than as a divergence, because the \
                 compilation was stopped part-way and produced no result to compare: the artifact is \
                 a fragment and the diagnostics are a prefix. Fourteen feature areas conclude cells \
                 concurrently, so the ceiling exists to keep one runaway invocation from taking the \
                 build volume — and with it every other cell's evidence — before any post-cell \
                 accounting could run. The command was: {}{}",
                posix_command_line(argv),
                match notes.as_slice() {
                    [] => String::new(),
                    recorded => format!(". Recorded notes: {}", recorded.join("; ")),
                }
            ),
        ));
    }
    Ok(Capture {
        stdout_integrity: stdout.integrity(),
        stderr_integrity: stderr.integrity(),
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        timed_out: watched.timed_out,
        status: watched.status,
        duration,
        notes,
    })
}

/// Drain one output stream on its own thread, retaining at most [`CAPTURE_BYTES_MAX`] bytes.
///
/// Delivers through a channel rather than a join handle so the caller can give up on it. A join
/// handle offers no timed wait, so holding one would force the caller to block until the stream
/// reached end of file — which, as [`spawn_bounded`] explains, is an event a terminated compiler
/// driver does not guarantee.
///
/// **The quota bounds what is retained, not what is read.** A reader that stopped at the cap
/// would leave the writer blocked on a full pipe forever, converting a chatty compiler into a
/// hang — the exact deadlock the separate reader threads exist to prevent. So the stream is read
/// to its end in fixed-size chunks and the surplus is dropped, which keeps memory bounded by the
/// quota while keeping the pipe empty.
///
/// A read error no longer vanishes. It ends the drain, and the fact that the drain ended early is
/// carried back to the caller so an attribution built on this text can decline to be certain. The
/// send failure is still discarded, and only that: it means the caller reached its deadline and
/// moved on, which the caller already knows because it is the party that timed out.
///
/// The reader is also **cancellable**. The caller that gives up sets the flag, and the drain observes
/// it around every read, so a reader nobody is waiting for stops instead of following a runaway
/// writer for the rest of the run — releasing its thread and its end of the pipe. What that cannot do
/// is interrupt a read already blocked in the kernel; [`harvest_within_deadline`] says what closes
/// that consequence instead.
fn spawn_capped_reader<R>(stream: R) -> CappedReader
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&cancelled);
    thread::spawn(move || {
        let _ = sender.send(drain_capped(stream, &observed));
    });
    CappedReader {
        harvest: receiver,
        cancelled,
    }
}

/// One pipe being drained on its own thread, and the switch that asks that thread to stop.
///
/// A record rather than a bare receiver, because giving up on a reader and being able to *tell* it so
/// are two halves of one operation: a caller holding only the receiver can stop waiting but cannot
/// stop the reading, which is how a thread and a descriptor per cell accumulate across a matrix of
/// 1,296 of them.
struct CappedReader {
    /// Delivers the harvest exactly once, when the drain ends for any reason.
    harvest: Receiver<StreamHarvest>,
    /// Set by the caller when it has stopped waiting; observed by the drain around every read.
    cancelled: Arc<AtomicBool>,
}

/// Read one stream to its end, retaining a bounded prefix and counting everything.
///
/// Split out from the thread body so the retention rule is stated once and can be read without
/// the concurrency around it: every byte is read, a prefix up to [`CAPTURE_BYTES_MAX`] is kept,
/// and the count of what the child actually produced is exact regardless of how much was kept.
fn drain_capped<R>(mut stream: R, cancelled: &AtomicBool) -> StreamHarvest
where
    R: Read,
{
    let mut retained: Vec<u8> = Vec::new();
    let mut produced: u64 = 0;
    let mut truncated = false;
    let mut chunk = vec![0_u8; CAPTURE_CHUNK_BYTES];
    let mut drained = false;
    let mut note = None;
    loop {
        // Asked around every read, so a reader whose caller has given up stops at its next
        // opportunity — after the read already in flight returns, or before the next one begins —
        // rather than draining a stream nothing will ever look at.
        if cancelled.load(Ordering::Relaxed) {
            note = Some(format!(
                "was cancelled after the collection grace elapsed, so it was not read to its end; \
                 the {} byte(s) collected before the cancellation are reported",
                retained.len()
            ));
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) => {
                drained = true;
                break;
            }
            Ok(count) => {
                produced = produced.saturating_add(count as u64);
                let room = CAPTURE_BYTES_MAX.saturating_sub(retained.len() as u64);
                let keep = room.min(count as u64) as usize;
                if keep > 0 {
                    retained.extend_from_slice(&chunk[..keep]);
                }
                if keep < count {
                    truncated = true;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                note = Some(format!(
                    "could not be read to its end: {}; the {} byte(s) collected before the error \
                     are reported, and any diagnostic the compiler wrote after it is absent",
                    sanitize_text_for_report(&error.to_string()),
                    retained.len()
                ));
                break;
            }
        }
    }
    if truncated && note.is_none() {
        note = Some(format!(
            "produced {produced} byte(s), of which the {CAPTURE_BYTES_MAX}-byte quota retained \
             the first {}; the remainder was read from the pipe and discarded so the compiler \
             could not block on it",
            retained.len()
        ));
    }
    StreamHarvest {
        bytes: retained,
        produced,
        truncated,
        drained,
        note,
    }
}

/// Take a reader's harvest if it arrives by the deadline, and an accounted absence if it does not.
///
/// A deadline already in the past yields a zero wait, which makes this a non-blocking poll —
/// exactly right after a timeout, where bytes already delivered are kept and nothing is waited
/// for.
///
/// The two ways this can come back empty are no longer conflated with an empty stream. A reader
/// that never existed and a reader that did not deliver in time are both reported as undrained,
/// with the reason attached, because a compiler whose diagnostics were lost to a deadline must
/// never be mistaken for a compiler that printed nothing.
///
/// # Giving up is not the same as walking away
///
/// A reader that has not delivered by the deadline is **cancelled** and then given one short further
/// window, [`CAPTURE_CANCEL_GRACE`], to act on it. That is what releases the thread and the pipe
/// descriptor in the case where the reader is between reads. A reader wedged inside a read cannot be
/// released at all — the standard library offers nothing that interrupts one from another thread, and
/// this suite may add no dependency that does — so that case records a run-level breach and the run
/// stops scheduling cells behind it. One held descriptor reported loudly is diagnosable; 1,296
/// accumulating quietly are not.
fn harvest_within_deadline(
    reader: Option<CappedReader>,
    deadline: Instant,
    role: &str,
) -> StreamHarvest {
    let Some(reader) = reader else {
        return StreamHarvest::absent(&format!(
            "was never opened, so no {role} could be collected; this is a condition of the \
             machine rather than a silent compiler"
        ));
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    match reader.harvest.recv_timeout(remaining) {
        Ok(harvest) => harvest,
        Err(error) => {
            let waited = sanitize_text_for_report(&error.to_string());
            reader.cancelled.store(true, Ordering::Relaxed);
            match reader.harvest.recv_timeout(CAPTURE_CANCEL_GRACE) {
                // The cancellation was observed, so the reader has ended and its harvest is whatever
                // it had gathered. Reported as undrained through the note the drain itself attached.
                Ok(harvest) => harvest,
                Err(_) => {
                    let detail = format!(
                        "the compiler's {role} reader did not deliver within the collection grace \
                         ({waited}) and had still not stopped {} ms after it was cancelled, so its \
                         thread and its end of the pipe remain held by whatever survived the group \
                         sweep",
                        CAPTURE_CANCEL_GRACE.as_millis()
                    );
                    record_infrastructure_breach(detail.clone());
                    StreamHarvest::absent(&format!(
                        "did not arrive within the collection grace: {waited}; the {role} it may \
                         have carried is absent from this record rather than empty, and the reader \
                         could not be stopped"
                    ))
                }
            }
        }
    }
}

/// Everything learned while watching one child until it finished or was stopped.
///
/// Returned as a record rather than a tuple because the third element — what could not be
/// completed during cleanup — is the part a tuple invites a caller to drop.
#[derive(Debug)]
struct WatchedChild {
    /// Whether the child outlived its budget and was terminated for it.
    timed_out: bool,
    /// How the child terminated, or `None` when it was killed or became unobservable.
    status: Option<ExitStatus>,
    /// Cleanup and observation shortfalls, each already sanitized for a report.
    notes: Vec<String>,
    /// Set when the child was stopped because its workspace crossed a live disk ceiling.
    ///
    /// Held apart from `timed_out` because the two are different facts with different remedies: a
    /// child that outlived its budget is a timeout, which is one of the suite's divergence classes,
    /// while one stopped for filling the disk produced no observation at all and is refused.
    overflow: Option<String>,
}

/// Wait for a child until the deadline, killing and reaping it if it outlives one.
///
/// A status is present on exactly one path — the child was observed to finish on its own — and
/// absent on the two paths where none exists to report: the deadline was reached, or the status
/// can no longer be observed at all.
///
/// Those two absences are deliberately not conflated. A child that outlived its budget is a
/// timeout, which is one of the suite's divergence classes and a real finding about a compiler. A
/// child whose status became unobservable is a condition of the machine, and reporting it as a
/// timeout would attribute a fault in this process's bookkeeping to the compiler it was watching.
/// Both absences now carry a note saying which of the two occurred, so the distinction survives
/// into the record a maintainer reads.
fn await_child_within_deadline(
    child: &mut Child,
    deadline: Instant,
    workspace: &Path,
) -> WatchedChild {
    let mut scanned_at = Instant::now();
    // The first incomplete workspace measurement, if any, and only the first: the scan repeats four
    // times a second, and one sentence per scan would bury the notes that describe the compilation.
    let mut unverified: Option<String> = None;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return WatchedChild {
                    timed_out: false,
                    status: Some(status),
                    notes: unverified.into_iter().collect(),
                    overflow: None,
                }
            }
            Ok(None) => {}
            Err(error) => {
                let mut notes = vec![format!(
                    "the compiler's exit status stopped being observable while it was being \
                     watched: {}; it was terminated and reaped so nothing of ours is left \
                     running, and the absent status is reported as unobservable rather than as a \
                     timeout",
                    sanitize_text_for_report(&error.to_string())
                )];
                notes.extend(unverified);
                notes.extend(terminate_and_reap(child));
                return WatchedChild {
                    timed_out: false,
                    status: None,
                    notes,
                    overflow: None,
                };
            }
        }
        if Instant::now() >= deadline {
            let mut notes: Vec<String> = unverified.into_iter().collect();
            notes.extend(terminate_and_reap(child));
            return WatchedChild {
                timed_out: true,
                status: None,
                notes,
                overflow: None,
            };
        }
        if scanned_at.elapsed() >= WORKSPACE_SCAN_INTERVAL {
            scanned_at = Instant::now();
            match scan_workspace(workspace) {
                WorkspaceScan::Within => {}
                WorkspaceScan::Unverified(detail) => {
                    unverified.get_or_insert(detail);
                }
                WorkspaceScan::Exceeded(detail) => {
                    let mut notes: Vec<String> = unverified.into_iter().collect();
                    notes.extend(terminate_and_reap(child));
                    return WatchedChild {
                        timed_out: false,
                        status: None,
                        notes,
                        overflow: Some(detail),
                    };
                }
            }
        }
        thread::sleep(WAIT_POLL_INTERVAL);
    }
}

/// What one live measurement of the cell workspace established.
#[derive(Debug)]
enum WorkspaceScan {
    /// The workspace was measured whole and is inside both ceilings.
    Within,
    /// The workspace is inside both ceilings *as measured*, but the measurement was not complete, so
    /// "inside" is a lower bound rather than a fact. Carries a sentence for the outcome's notes.
    Unverified(String),
    /// A ceiling was positively exceeded. Carries a sentence naming which and by what.
    Exceeded(String),
}

/// Measure the cell workspace and say whether a live ceiling has been crossed.
///
/// # Positive evidence only, which is the opposite of the retention accounting's rule
///
/// A ceiling is enforced here only on what was actually measured. A walk that could not read a
/// directory, or that stopped at one of its own ceilings, does **not** stop the compilation — even
/// though the very same incompleteness makes `sandbox.rs`'s retention accounting refuse an entry
/// outright. The two rules are opposite because the two questions are:
///
/// - Retention asks *may I claim this fits inside the run's budget?* An entry nobody could measure
///   cannot be claimed, so it fails closed, or the ceiling stops meaning anything.
/// - This asks *is something running away with the disk right now?* A compiler deleting its own
///   temporary file between the listing and the measurement makes an entry vanish mid-walk, which is
///   an ordinary race and not a runaway. Failing closed on it would terminate honest compilations on
///   a busy machine and manufacture findings against a compiler that did nothing wrong.
///
/// A genuine runaway produces positive evidence by definition — bytes on the disk or entries in the
/// directory — so requiring it costs nothing. What incompleteness does earn is a **note**, once per
/// invocation, so that a reader is never left to assume the ceiling was checked against a whole
/// measurement when it was checked against a lower bound.
fn scan_workspace(workspace: &Path) -> WorkspaceScan {
    let measured = measure_tree(workspace, WORKSPACE_TREE_LIMITS);
    if measured.bytes > WORKSPACE_LIVE_BYTES_MAX {
        return WorkspaceScan::Exceeded(format!(
            "the cell workspace held at least {} byte(s) while the compilation was still running, \
             which is past the {WORKSPACE_LIVE_BYTES_MAX}-byte ceiling on a workspace in flight",
            measured.bytes
        ));
    }
    if measured.entries > WORKSPACE_LIVE_ENTRIES_MAX {
        return WorkspaceScan::Exceeded(format!(
            "the cell workspace held at least {} entr(y/ies) while the compilation was still \
             running, which is past the {WORKSPACE_LIVE_ENTRIES_MAX}-entry ceiling on a workspace in \
             flight",
            measured.entries
        ));
    }
    match measured.shortfall() {
        None => WorkspaceScan::Within,
        Some(shortfall) => WorkspaceScan::Unverified(sanitize_text_for_report(&format!(
            "the live measurement of the cell workspace was incomplete — {shortfall} — so the \
             ceilings on a workspace in flight were tested against a lower bound; the compilation \
             was allowed to continue, because stopping it on the strength of a measurement that \
             could not be completed would terminate an honest invocation on a busy machine"
        ))),
    }
}

/// Kill a child's whole process group, then the child itself, and reap it.
///
/// The order matters and is the point of this function. Signalling the group first reaches the
/// sub-processes a compiler driver started — the assembler, the linker, a wrapped `timeout`'s own
/// child — which a kill directed at the driver alone leaves running and holding the output pipe
/// open. Killing the direct child afterwards is not redundant: the group signal is delivered by
/// an external utility whose absence must not leave the child alive, so the built-in kill is the
/// guarantee that at least the process this module started is stopped.
///
/// A failure of the kill is *ordinarily* not a failure at all — a kill fails when the child has
/// already exited — so it is still absorbed, and the reap that follows is what proves the child is
/// gone. What is no longer absorbed is a reap that could not be completed inside its deadline, or a
/// group signal that could not be attempted: each means a process may still be running, which is a
/// fact about the machine a maintainer needs rather than one this function may quietly decide for
/// them.
///
/// The reap is bounded rather than a plain wait for the reason the whole cleanup path exists: it is
/// what makes the per-cell budget a real bound, so a wait here without a deadline of its own would
/// turn the mechanism that stops a hung compiler into a hang that nothing outside it would catch.
fn terminate_and_reap(child: &mut Child) -> Vec<String> {
    let group = child.id();
    let reaped = reap_bounded(child);
    // Swept after the reap, never before: a killed-but-unwaited child is a zombie, and a zombie
    // still answers an existence probe, so sweeping first would report a survivor that is nothing
    // of the kind.
    let mut notes = Vec::new();
    if let Some(note) = reaped.note() {
        record_infrastructure_breach(String::from(note));
        notes.push(sanitize_text_for_report(&redact_secrets(note)));
    }
    match terminate_process_group(group, kill_tool()) {
        GroupTermination::Cleared => {}
        GroupTermination::Survivors(detail) | GroupTermination::Unsupervised(detail) => {
            notes.push(sanitize_text_for_report(&redact_secrets(&detail)));
        }
    }
    notes
}

/// Withdraw an attribution that was read out of diagnostics this run did not capture whole.
///
/// The gate that makes [`FailureScope::Indeterminate`] mean something. Applied at exactly the
/// return sites of [`classify_failure`] whose scope was decided by matching — or by failing to
/// match — a signature against the compiler's own text, and applied at none of the sites whose
/// scope came from a directly observed fact.
///
/// The distinction is load-bearing in both directions:
///
/// - A **text-derived** scope is only as sound as the text. A truncated diagnostic stream may be
///   missing the line naming an absent C runtime, in which case the environment test above would
///   not have fired and the failure would fall through to the compiler-answerable default — a
///   manufactured finding against a compiler that was never given the inputs it needed. The
///   *absence* of a signature is the weakest evidence of all in a stream that was cut short, which
///   is why the final fallback is gated too.
/// - A **fact-derived** scope is untouched by how much text survived. A timeout, a bounding
///   utility's own status, an unobservable wait, a signal death and a missing artifact are each
///   observed directly, and withdrawing those attributions because a diagnostic listing overflowed
///   a quota would discard sound conclusions and bury real defects under an unattributable label.
///
/// Returns the judgement unchanged in the ordinary case, so a complete capture — which every
/// well-behaved invocation produces — pays nothing and reads exactly as it did before.
fn scoped_by_diagnostics(failure: BuildFailure, outcome: &CompileOutcome) -> BuildFailure {
    if outcome.diagnostics_complete() {
        return failure;
    }
    failure.into_indeterminate(&format!(
        "this attribution was read from the compiler's diagnostics and that stream was not \
         captured whole{}",
        outcome
            .stderr_integrity()
            .describe("the compiler's diagnostics")
    ))
}

/// The size of a regular file at `path`, or `None` when there is no regular file there.
///
/// Symbolic links are not followed, so a link is reported as no artifact rather than as the file
/// it points at. A compiler has no reason to place one here, and the strict reading is the safe
/// one: an artifact reached through a link could resolve outside the workspace the cell owns,
/// which would then be executed.
fn regular_file_size(path: &Path) -> Option<u64> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Some(metadata.len()),
        Ok(_) => None,
        Err(_) => None,
    }
}

/// Decide why one build did not produce a usable artifact, or that it did.
///
/// # Why this is not the verdict
///
/// Nothing here decides whether the suite passes. A rejected program may be an expected
/// divergence against a documented limitation, a finding, or — in the case of a target whose C
/// runtime was never installed — a fact about the machine that says nothing about any compiler.
/// This function establishes *what happened* and *who is answerable*, and the classifier that
/// consumes it decides what that is worth. Keeping the two apart is what allows a program the
/// compiler under test refuses to remain under test rather than being quietly dropped.
///
/// # The order of the tests, and why it is this order
///
/// Every test below can match evidence another would also match, so the sequence is the
/// judgement. Two principles set the order:
///
/// - **Facts before text.** A signal, an exit status and this process's own watchdog observation
///   are facts about a process. A diagnostic signature is a guess about a string the subject
///   chose to print. Every fact is therefore established before any text is interpreted.
/// - **Attributing a failure to the machine requires corroboration the subject cannot fabricate.**
///   An `Environment` attribution is not a neutral classification: it becomes `UNAVAILABLE`, and
///   `UNAVAILABLE` does not fail a run by default. A compiler able to reach that verdict by
///   printing a chosen line could excuse its own defects. So a diagnostic signature from the
///   **compiler under test** never attributes a failure to the machine on its own: each such
///   signature must be corroborated by an independent observation this process made itself — the C
///   runtime probe of test 5, performed against a driver of this module's own choosing before the
///   subject ran.
///
///   **There is exactly one exception, and it is a stated trust assumption rather than an
///   oversight: test 6.** A stage-execution diagnostic — the reference driver reporting that it
///   could not `exec` its own preprocessor, assembler or linker — reaches
///   [`FailureScope::Environment`] on the strength of the diagnostic alone, with no independent
///   corroboration, and it is available to the **reference compiler only**. Two things make that
///   sound. The reference compiler is this suite's oracle rather than its subject: it has no
///   verdict to escape, so it has no motive to fabricate, and if it were compromised the whole
///   comparison would already be worthless. And the diagnostic is not merely a claim but a claim
///   only a multi-process driver can even make; the compiler under test is a single self-contained
///   binary with an integrated assembler and linker, so the identical text from it is a string it
///   chose to print and is attributed to the **compiler**, which test 6 does explicitly. The
///   exception therefore cannot be reached by the side that would benefit from it.
///
/// The sequence:
///
/// 1. **Timeout first.** An invocation that outlived its budget was killed by this module's own
///    watchdog, so whatever it had printed by then is incidental. A hang is its own divergence
///    class, and the observation is this process's — not an exit status, and not the bounding
///    utility's.
/// 2. **A status that could not be observed next.** No verdict was seen, so none may be inferred;
///    this is a fault in this process's own bookkeeping and is answerable by *this process*, at
///    [`FailureScope::Harness`]. It is deliberately not attributed to the machine: an environment
///    gap is a reported absence the run does not fail for, and a launched build whose result was
///    lost must fail rather than read as an arm that could not be attempted.
/// 3. **Termination by a signal next**, ahead of every text test. A signal is an unambiguous fact
///    about the process, and a compiler that dies on a valid program is a defect that no
///    diagnostic it managed to print beforehand may excuse. Ordering it above the two
///    environment-signature tests is what settles the awkward case: a subject that crashes while
///    printing `cannot find crt1.o` is a crash, not a fact about the machine.
/// 4. **Success next**, which is delegated to [`classify_successful_build`]. Answering it here,
///    rather than as an afterthought at the end, is what makes every test below a *failure-only*
///    test: none of them can ever see a build that succeeded, so none has to guard against one.
/// 5. **A missing C runtime next**, ahead of every other link diagnostic — but attributed to the
///    machine **only when the target's probed C runtime is positively incomplete**. This is the
///    test that prevents the suite manufacturing findings: a static link that fails because a
///    target's start files were never installed has *exactly* the shape of one that fails because
///    of a defect in code generation. The diagnostic alone cannot separate them, because the
///    subject writes the diagnostic. The probe can, because this process performed it, against a
///    driver of its own choosing, before the subject ran. Where the probe says the runtime is
///    complete — or says nothing, because no driver could be asked — the same diagnostic is
///    attributed to the **compiler**, and the summary says so and quotes both.
/// 6. **A driver that cannot run its own stages next**, which is an installation fault — and
///    available only to the reference compiler, which is a driver that execs separate stages and
///    can therefore genuinely fail to find one. The compiler under test is a single self-contained
///    binary with an integrated assembler and linker: it has no stage to exec, so the same
///    diagnostic from it is not an installation fault but a string it chose to print, and it is
///    attributed to the compiler.
/// 7. **Other linker-stage diagnostics next**, which are genuine link failures — a distinct class
///    from a rejected translation, because the two implicate different parts of a compiler. Their
///    scope is decided by [`link_scope`], on the same corroborated basis as test 5.
/// 8. **A diagnostic at a source location next**, which is a rejected program: the documented
///    diagnostic shape is `file:line:col: error:`, and a diagnostic that names a place in the
///    source is a statement about the source.
/// 9. **Anything else that failed** is a compile failure with the compiler answerable, which is
///    the conservative default: it keeps the full diagnostics attached for a human to adjudicate
///    rather than guessing a narrower class from evidence that did not match anything.
///
/// Note what is **not** in the list. No exit status is read as belonging to the bounding utility.
/// The installed utility passes its child's status through verbatim, so 124, 125, 126 and 127
/// carry no information about which process produced them: a compiler exiting 125 is a compiler
/// exiting 125, and reading it as the utility's own failure would excuse it from failing the run.
/// Those statuses reach test 9 like any other, and the watchdog's own observation carries the only
/// timeout fact this module has.
///
/// A build that succeeded is checked once more, for an artifact that is absent or empty. That
/// case is reported here rather than left to surface later as an execution that could not start,
/// because "it exited successfully and produced nothing" is a compiler defect and looks nothing
/// like a program that failed to run.
///
/// Quoted diagnostics pass through [`truncate_for_summary`], so no captured byte can forge a
/// column, erase a line or repaint a verdict in a report. The bytes themselves remain intact in
/// [`CompileOutcome::stderr`], which is what a finding artifact records.
fn classify_failure(
    outcome: &CompileOutcome,
    request: &CompileRequest<'_>,
    caps: &Capabilities,
) -> Option<BuildFailure> {
    let diagnostics = outcome.stderr_text();
    if outcome.timed_out {
        let outer = if outcome.timeout_tool_used {
            ", with the system timeout utility standing behind it as an outer net at the budget \
             plus a further margin"
        } else {
            ""
        };
        return Some(BuildFailure::new(
            DivergenceClass::Timeout,
            FailureScope::Compiler,
            format!(
                "the {} did not terminate within its {}-second budget and was killed by this \
                 module's own watchdog{outer}; a compilation that completes promptly under one \
                 implementation and hangs under another is a defect worth surfacing rather than \
                 an infrastructure error{}",
                outcome.compiler().label(),
                outcome.budget().as_secs(),
                excerpt_clause(&diagnostics)
            ),
        ));
    }
    let Some(status) = outcome.status() else {
        return Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Harness,
            format!(
                "the status of the {} could not be observed, so no verdict about the program was \
                 seen and none may be inferred; this is a fault in this process's own bookkeeping \
                 rather than a judgement any compiler made, and is therefore reported as a failure \
                 of this suite rather than as a gap in this machine{}",
                outcome.compiler().label(),
                excerpt_clause(&diagnostics)
            ),
        ));
    };
    // Facts before text. A signal is an unambiguous statement about the process, so it is settled
    // before any diagnostic is read: a subject that crashed while printing a provisioning-shaped
    // line must not be able to have the crash attributed to the machine.
    if status.code().is_none() {
        return Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} was terminated by a signal ({status}) rather than exiting, so it crashed \
                 on this program instead of forming a judgement about it. Recorded as a compile \
                 failure because no artifact was produced, and reported as a crash in the summary \
                 because a compiler that dies on a valid program looks nothing like one that \
                 declines it. Established before any diagnostic signature is consulted, so a \
                 provisioning-shaped line printed on the way down cannot move the blame to this \
                 machine{}",
                outcome.compiler().label(),
                excerpt_clause(&diagnostics)
            ),
        ));
    }
    if status.code() == Some(0) {
        return classify_successful_build(outcome);
    }
    if let Some(line) = first_line_matching_signature(&diagnostics, ENVIRONMENT_LINK_SIGNATURES) {
        // The diagnostic says a C runtime input was missing. The subject wrote the diagnostic, so
        // it is a claim rather than evidence, and it is corroborated against the runtime probe
        // this process performed itself — before the subject ran, through a driver of its own
        // choosing. Only a probe that positively found the runtime incomplete may move a failure
        // to the machine.
        let (scope, corroboration) = link_scope(request, caps);
        // Expressed as a conditional rather than a match over the scope enumeration on purpose:
        // `link_scope` answers exactly one question — did a probe positively find the runtime
        // incomplete — so only two of the three scopes can arrive here, and anything that is not
        // the corroborated environment answer must take the conservative one. A match would need
        // an arm for a value this call cannot produce, and the honest reading of such an arm is
        // "attribute to the compiler", which is what the fallthrough already does.
        let summary = if scope == FailureScope::Environment {
            format!(
                "the static link for {} failed for want of a C runtime input rather than because \
                 of anything the {} produced, so this is a gap in this machine's provisioning and \
                 not a defect: {}. {corroboration}. Installing the target's development package \
                 restores the arm; until then it is reported as an environment gap, loudly, and \
                 never as a finding",
                request.target().triple(),
                outcome.compiler().label(),
                truncate_for_summary(&line)
            )
        } else {
            format!(
                "the {} reported a missing C runtime input for {} — {} — but that claim is not \
                 corroborated: {corroboration}. A diagnostic is text the compiler chose to \
                 print, and attributing a failure to this machine on that basis alone would let \
                 an implementation excuse its own defects by naming a file it decided not to \
                 find, so the failure is recorded against the compiler",
                outcome.compiler().label(),
                request.target().triple(),
                truncate_for_summary(&line)
            )
        };
        return Some(scoped_by_diagnostics(
            BuildFailure::new(DivergenceClass::LinkFailure, scope, summary),
            outcome,
        ));
    }
    if let Some(line) =
        first_line_matching_signature(&diagnostics, TOOLCHAIN_INSTALLATION_SIGNATURES)
    {
        // An installation fault means a driver could not exec a stage of its own. Only a
        // multi-process driver can suffer one, and only the reference compiler is one: it runs a
        // preprocessor, a compiler proper, an assembler and a linker as separate programs. The
        // compiler under test is a single self-contained binary carrying its own assembler and
        // integrated linker, so it has no stage to exec and cannot have failed to find one. The
        // same text from it is a string it chose to print.
        let summary = match request.compiler() {
            Compiler::Reference => format!(
                "the {} could not run one of its own stages, which is an installation fault on \
                 this machine rather than a judgement about the program: {}. The reference \
                 compiler is a driver that execs separate stages, so it is the one side of this \
                 comparison for which this diagnostic is a fact about the machine",
                outcome.compiler().label(),
                truncate_for_summary(&line)
            ),
            Compiler::Bcc => format!(
                "the {} printed a stage-execution diagnostic — {} — but it is a single \
                 self-contained binary with an integrated assembler and linker and has no \
                 separate stage to exec, so this cannot be an installation fault on this machine \
                 and is recorded against the compiler. Attributing it to the environment would \
                 let the subject of the comparison excuse a defect by choosing what to print",
                outcome.compiler().label(),
                truncate_for_summary(&line)
            ),
        };
        let scope = match request.compiler() {
            Compiler::Reference => FailureScope::Environment,
            Compiler::Bcc => FailureScope::Compiler,
        };
        return Some(scoped_by_diagnostics(
            BuildFailure::new(DivergenceClass::CompileFailure, scope, summary),
            outcome,
        ));
    }
    if let Some(line) = first_line_matching_signature(&diagnostics, LINKER_STAGE_SIGNATURES) {
        let (scope, corroboration) = link_scope(request, caps);
        return Some(scoped_by_diagnostics(
            BuildFailure::new(
                DivergenceClass::LinkFailure,
                scope,
                format!(
                    "translation completed but the link did not, for {} at {}: {}. {corroboration}",
                    request.target().triple(),
                    outcome.opt().flag(),
                    truncate_for_summary(&line)
                ),
            ),
            outcome,
        ));
    }
    if let Some(line) = first_source_location_error(&diagnostics) {
        return Some(scoped_by_diagnostics(
            BuildFailure::new(
                DivergenceClass::CompileFailure,
                FailureScope::Compiler,
                format!(
                    "the {} rejected the program at a source location: {}. A program one \
                     implementation refuses and another accepts is exactly the asymmetry this \
                     suite exists to surface, so it is reported rather than treated as an error",
                    outcome.compiler().label(),
                    truncate_for_summary(&line)
                ),
            ),
            outcome,
        ));
    }
    // The weakest evidence in the function, and therefore the site that most needs the gate: this
    // branch concludes "compiler answerable" from the *absence* of every recognised signature, and
    // a stream that was cut short is exactly where a signature goes missing without being absent.
    Some(scoped_by_diagnostics(
        BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} did not produce an artifact and its diagnostics match no recognised \
                 signature, so the conservative class is recorded and the whole captured text is \
                 kept for a human to adjudicate: {}{}",
                outcome.compiler().label(),
                describe_termination(outcome),
                excerpt_clause(&diagnostics)
            ),
        ),
        outcome,
    ))
}

/// Check a build that reported success for the one way it can still have failed.
///
/// A compiler that exits successfully and writes nothing, or writes an empty file, has failed —
/// and it has failed in a way that would otherwise surface much later and much less clearly, as
/// an execution that could not start. Named here, at the point the evidence exists.
fn classify_successful_build(outcome: &CompileOutcome) -> Option<BuildFailure> {
    match outcome.artifact_size() {
        None => Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} reported success but left no regular file at {}; an exit status claiming \
                 an artifact that is not there is a defect in its own right, and naming it here \
                 keeps it from surfacing later as an execution that could not start",
                outcome.compiler().label(),
                shown_path(outcome.artifact())
            ),
        )),
        Some(0) => Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} reported success but left an empty file at {}; nothing can be executed \
                 from it, so the success is not one",
                outcome.compiler().label(),
                shown_path(outcome.artifact())
            ),
        )),
        Some(_) => None,
    }
}

/// Decide whether a linker-stage failure is the compiler's or the machine's, and say why.
///
/// The default is the compiler, because that is the answer a link failure ordinarily deserves and
/// because the alternative — excusing a real defect as a provisioning gap — is the more damaging
/// mistake of the two. The environment answer is returned only on positive evidence: a runtime
/// that was actually probed, through a driver that was actually present, and found to be missing
/// a required input. A target for which no reference driver exists yields no evidence either way,
/// so it does not excuse anything, and the returned phrase says so rather than implying the
/// question was settled.
fn link_scope(request: &CompileRequest<'_>, caps: &Capabilities) -> (FailureScope, String) {
    let target = request.target();
    let Some(status) = caps.c_runtime_for(target) else {
        return (
            FailureScope::Compiler,
            format!(
                "no C runtime probe was recorded for {}, so nothing corroborates a provisioning \
                 explanation and the failure is attributed to the compiler",
                target.triple()
            ),
        );
    };
    if status.driver().is_none() {
        return (
            FailureScope::Compiler,
            format!(
                "the C runtime for {} could not be probed, because probing asks a reference \
                 driver and none is installed for that target; an unprobed runtime is not \
                 evidence of a missing one, so the failure is attributed to the compiler",
                target.triple()
            ),
        );
    }
    let missing = status.missing();
    if missing.is_empty() {
        return (
            FailureScope::Compiler,
            format!(
                "the C runtime for {} was probed and is complete ({}), so the machine supplied \
                 everything the link needed and the failure is the compiler's",
                target.triple(),
                truncate_for_summary(&status.describe())
            ),
        );
    }
    (
        FailureScope::Environment,
        format!(
            "the C runtime for {} was probed and is incomplete — {} — so the link was attempted \
             without inputs it requires. Reported as a gap in this machine's provisioning rather \
             than as a defect, because a compiler cannot be held to a link it was never given the \
             parts for",
            target.triple(),
            missing.join(", ")
        ),
    )
}

/// The first line of `text` containing any of `signatures`, matched case-insensitively.
///
/// Returned in its original spelling and trimmed, because it is quoted into a summary a human
/// reads. Matching is case-insensitive because the same condition is reported with different
/// capitalisation by different toolchains, and a signature test that a change of case defeats
/// would let a provisioning gap be reported as a compiler defect.
fn first_line_matching_signature(text: &str, signatures: &[&str]) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let folded = trimmed.to_lowercase();
        if signatures
            .iter()
            .any(|signature| folded.contains(*signature))
        {
            return Some(String::from(trimmed));
        }
    }
    None
}

/// The first line of `text` that carries an error diagnostic naming a place in the source.
///
/// The documented diagnostic shape is `file:line:col: error: description`, so a line qualifies
/// when the text before its error marker ends in at least two colon-separated numbers. That is
/// the test rather than the presence of the marker alone, because `ld: error: ...` also carries
/// the marker and is a link failure: it names a stage, not a place in the program.
///
/// Only the structure is relied upon, never the wording. A description is free text that differs
/// legitimately between implementations, and the suite never compares one against another.
fn first_source_location_error(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(marker) = trimmed.find(DIAGNOSTIC_ERROR_MARKER) else {
            continue;
        };
        if has_source_location(&trimmed[..marker]) {
            return Some(String::from(trimmed));
        }
    }
    None
}

/// True when `prefix` ends in the line-and-column pair a source diagnostic is introduced by.
///
/// Splitting on the colon and testing only the final two fields is what makes this correct for a
/// path that itself contains a colon — a drive letter, or a directory named with one — because
/// such a path contributes leading fields and never the trailing numeric pair.
fn has_source_location(prefix: &str) -> bool {
    let trimmed = prefix.trim_end();
    let body = match trimmed.strip_suffix(':') {
        Some(shortened) => shortened,
        None => trimmed,
    };
    let fields: Vec<&str> = body.split(':').collect();
    if fields.len() < 3 {
        return false;
    }
    fields
        .iter()
        .rev()
        .take(2)
        .all(|field| !field.is_empty() && field.chars().all(|digit| digit.is_ascii_digit()))
}

/// How an invocation ended, phrased for a summary.
///
/// A signal is reported as a signal rather than folded into a numeric code, because a compiler
/// that crashes on a program and a compiler that rejects one are different events, and conflating
/// them is how a crash comes to be reported as an ordinary refusal. The standard library's own
/// rendering of a status is used, so the text is generated here rather than quoted from a tool
/// and cannot carry hostile bytes into a report.
fn describe_termination(outcome: &CompileOutcome) -> String {
    match outcome.status() {
        Some(status) => match status.code() {
            Some(code) => format!("it exited with status {code}"),
            None => format!(
                "it was terminated by a signal ({status}), which is a crash in the compiler \
                 itself rather than a judgement about the program"
            ),
        },
        None => String::from(
            "its status could not be observed, which is a fault in this process's own bookkeeping \
             rather than a judgement about the program",
        ),
    }
}

/// A summary clause quoting the first line of diagnostics, or nothing when there are none.
///
/// Written as a clause so that a summary reads as one sentence whether or not the compiler
/// printed anything, which several of them legitimately do not.
fn excerpt_clause(diagnostics: &str) -> String {
    match first_non_empty_line(diagnostics) {
        Some(line) => format!("; it had printed: {}", truncate_for_summary(line)),
        None => String::new(),
    }
}

/// The first line of `text` that is not blank.
fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

/// Render captured text so it is safe to place in a report, and short enough to belong in one.
///
/// Three transformations, in this order and for three different reasons.
///
/// [`redact_secrets`] comes first, because it recognises a credential by the characters the
/// environment holds and escaping would have altered them: a value containing a tab or a byte
/// sanitization spells out is no longer the value being searched for once it has been escaped. This
/// is an ordinary diagnostic excerpt rather than a command line, and it needs the same treatment for
/// the same reason — a compiler that echoes an environment variable back in a warning puts it into
/// every artifact this string reaches.
///
/// Escaping comes second, because captured bytes are untrusted: a tab would forge a column in a
/// tab-separated report, a carriage return would erase the line it ends, and an escape introducer
/// would begin a terminal sequence that could repaint a verdict the run never reached.
///
/// Truncation comes last, because a summary is one line and a compiler can print a great deal — and
/// it is applied to the escaped text so the limit bounds what a reader actually sees.
///
/// The elision is marked, so a truncated line is visibly truncated rather than merely short. The
/// unabridged bytes remain in [`CompileOutcome::stderr`] and are what a finding artifact records,
/// so nothing is lost by shortening what a summary shows.
fn truncate_for_summary(text: &str) -> String {
    let safe = sanitize_text_for_report(&redact_secrets(text.trim()));
    let mut kept = String::with_capacity(safe.len().min(SUMMARY_EXCERPT_CHARS_MAX));
    for character in safe.chars().take(SUMMARY_EXCERPT_CHARS_MAX) {
        kept.push(character);
    }
    if kept.chars().count() < safe.chars().count() {
        kept.push_str(" [...]");
    }
    kept
}
