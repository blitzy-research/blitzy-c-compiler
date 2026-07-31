//! Builds one cell: a single compiler invocation, assembled under the shared-flag
//! discipline, executed hermetically, and reported as data rather than raised as an error.
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
//! # Hermeticity and parallel safety
//!
//! Each child runs with its current directory set to the cell's own workspace, so a compiler
//! that writes a temporary file beside its output writes it inside the one directory the cell
//! owns. The process-wide working directory is **never** changed: the feature-area tests run
//! concurrently in one process, so that would be a data race rather than a confinement. The
//! artifact is required to be a direct child of that workspace, and it is required to be the
//! canonical name for its compiler — which is what stops two compilers in one cell from
//! writing over each other's binary and leaving a comparison to be made against a single file
//! twice.
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
//! [`DivergenceClass`] says what shape the failure took, and its [`FailureScope`] says whether
//! the machine or the compiler is answerable for it: a target whose C runtime is not installed
//! produces a link failure at [`FailureScope::Environment`], which must be reported as a gap in
//! the environment rather than as a finding against a compiler that never had the inputs it
//! needed.
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
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use super::env::Capabilities;
use super::manifest::{CommandSubstitutions, Manifest};
use super::sandbox::{Workspace, BCC_ARTIFACT_NAME, REFERENCE_ARTIFACT_NAME};
use super::{
    bcc_requires_explicit_target, bcc_target_arguments, corpus_root, ensure_within,
    is_bcc_target_selector, is_forbidden_for_side, posix_command_line, require_regular_file,
    sanitize_text_for_report, CompilerSide, DivergenceClass, HarnessError, HarnessResult, OptLevel,
    Target, BCC_TARGET_FLAG, DIFFERENTIAL_FLAGS_MINIMAL,
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
/// Generous for a diagnostic and finite by design. The bound is what makes a compiler stuck in
/// a diagnostic loop harmless: the reader stops at the cap, drops the pipe, and the operating
/// system terminates the writer, so neither this process's memory nor the run's duration
/// depends on how much a misbehaving tool decides to print.
const CAPTURE_BYTES_MAX: u64 = 4 * 1024 * 1024;

/// Interval between checks on a child that has not yet finished.
///
/// Short enough that the measured duration of a compilation — roughly seventy milliseconds for
/// a whole compile-and-run pair — is not dominated by polling granularity, and long enough that
/// waiting costs no measurable processor time.
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Extra time the watchdog allows beyond the budget when the system timeout utility is also in
/// use.
///
/// The utility is given the first opportunity to act, because it signals the whole process
/// group and therefore also stops the sub-processes a compiler driver started, which this
/// module cannot see. The watchdog is the backstop for the case where the utility itself fails
/// to act, so it must fire later than the utility rather than at the same moment.
const WATCHDOG_GRACE: Duration = Duration::from_secs(5);

/// Time allowed for the capture threads to deliver after the child has been waited on.
///
/// Without it, a compilation that finished just inside its budget could have its diagnostics
/// truncated by the same deadline that governed the wait — losing exactly the text a
/// divergence has to be read from. With it, the whole invocation is still bounded, at the
/// budget plus this grace plus [`WATCHDOG_GRACE`].
const CAPTURE_GRACE: Duration = Duration::from_secs(2);

/// Exit status the system timeout utility reports when it terminated the command it was
/// wrapping.
///
/// The conventional value, verified against the utility installed in this environment. No
/// compiler in this suite exits with it of its own accord, and [`expired_under_timeout_tool`]
/// additionally requires the observed duration to have reached the budget, so a command that
/// somehow chose this status for itself is not mistaken for a timeout.
const TIMEOUT_TOOL_EXPIRED_STATUS: i32 = 124;

/// Exit statuses the system timeout utility reports for failures of its own: it could not run
/// at all, it found the command but could not invoke it, or it could not find the command.
///
/// These describe the machine rather than the compilation, so they are reported at
/// [`FailureScope::Environment`].
const TIMEOUT_TOOL_OWN_FAILURE_STATUSES: &[i32] = &[125, 126, 127];

/// Tolerance applied when deciding whether an invocation really reached its budget.
///
/// The elapsed time is measured around the spawn, so a genuine expiry always exceeds the
/// budget; this slack only absorbs clock granularity and cannot turn a prompt failure into a
/// reported timeout.
const TIMEOUT_ELAPSED_SLACK: Duration = Duration::from_millis(250);

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
}

impl FailureScope {
    /// The token used in reports and summaries.
    pub fn label(self) -> &'static str {
        match self {
            FailureScope::Compiler => "compiler scope",
            FailureScope::Environment => "environment scope",
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
/// [`DivergenceClass::Timeout`] — and the scope says whether the compiler or the machine is
/// answerable.
///
/// The fields are private because the three travel together as one judgement. A caller able to
/// rewrite the scope could turn an environment gap into a finding against a compiler, or hide a
/// real defect behind a claim about the machine, while every report still displayed the
/// original summary.
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

    /// Whether the compiler or the machine is answerable.
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

    /// Whether this failure is a property of the machine rather than of the compiler.
    ///
    /// The predicate a caller wants far more often than the scope value itself: it is the test
    /// that decides whether an outcome becomes an environment report or a candidate finding.
    pub fn is_environment(&self) -> bool {
        matches!(self.scope, FailureScope::Environment)
    }
}

impl std::fmt::Display for BuildFailure {
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
                    output.display(),
                    root.display(),
                    self.compiler.artifact_name()
                ),
            ));
        }
        let expected = self.compiler.artifact_name();
        let actual = output.file_name().and_then(|name| name.to_str());
        if actual != Some(expected) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the artifact {} is not named {expected:?}, which is the canonical name for \
                     the {}; both compilers build into one workspace per cell, so a shared or \
                     swapped artifact name would leave oracle (a) comparing one binary against \
                     itself and reporting agreement — the one failure that leaves no trace",
                    output.display(),
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
                source.display(),
                corpus_root().display(),
                workspace_root.display()
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
                    self.manifest.path().display(),
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
                    self.manifest.path().display(),
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
    artifact: PathBuf,
    artifact_size: Option<u64>,
    duration: Duration,
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
    /// in it — see [`CompileOutcome::spawned_argv`].
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

    /// The vector actually spawned, which is [`CompileOutcome::argv`] prefixed with the system
    /// timeout utility when the environment provided one.
    ///
    /// Reported separately, and never used as the reproduction line, because the wrapper is an
    /// implementation detail of how this run bounded the compilation rather than part of the
    /// command that reproduces the artifact.
    pub fn spawned_argv(&self) -> &[String] {
        &self.spawned_argv
    }

    /// The spawned vector as one shell line, for a report that needs to state exactly what ran.
    pub fn spawned_command_line(&self) -> String {
        posix_command_line(&self.spawned_argv)
    }

    /// Whether the system timeout utility bounded this invocation in addition to the harness's
    /// own watchdog.
    pub fn timeout_tool_used(&self) -> bool {
        self.timeout_tool_used
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
    /// Captured for completeness and for finding artifacts. A compiler ordinarily prints nothing
    /// here, and nothing in the suite compares it: the streams that decide a verdict are those
    /// of the *program*, not of the compiler that built it.
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

    /// Where the artifact was asked to be written.
    pub fn artifact(&self) -> &Path {
        &self.artifact
    }

    /// Whether a regular file appeared at the artifact path.
    ///
    /// Derived from the recorded size rather than probed again, so this answer and
    /// [`CompileOutcome::artifact_size`] can never disagree, and neither changes if something
    /// later removes the file.
    pub fn artifact_exists(&self) -> bool {
        self.artifact_size.is_some()
    }

    /// The artifact's size in bytes, or `None` when no regular file was there.
    ///
    /// Recorded so that "it exited successfully and produced nothing" is visible as the distinct
    /// failure it is, rather than surfacing later as an execution that could not start.
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

    /// Whether the failure, if any, is a property of the machine rather than of the compiler.
    ///
    /// False for a successful build. A caller deciding whether to raise a finding must consult
    /// this: a target whose C runtime is absent produces a link failure that says nothing about
    /// the compiler, and reporting it as a defect would manufacture findings on any modestly
    /// provisioned machine.
    pub fn is_environment_failure(&self) -> bool {
        self.failure
            .as_ref()
            .is_some_and(BuildFailure::is_environment)
    }

    /// One line describing this outcome for a report or a failure message.
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
        format!(
            "{} {} {}: {termination}; {size}; {} ms; {}",
            self.compiler.label(),
            self.target.triple(),
            self.opt.flag(),
            self.duration.as_millis(),
            self.command_line
        )
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
    let (spawned_argv, timeout_tool_used) = wrap_with_timeout_tool(&argv, caps, budget);
    // The utility, when present, is given the first opportunity to act, because it signals the
    // whole process group and therefore also reaches the sub-processes a compiler driver started
    // and this module cannot see. The watchdog then fires later, as a backstop for the case where
    // the utility does not act at all.
    let watchdog = if timeout_tool_used {
        budget + WATCHDOG_GRACE
    } else {
        budget
    };
    let capture = spawn_bounded(
        &spawned_argv,
        request.workspace().root(),
        watchdog,
        &context,
    )?;

    let artifact = request.output().to_path_buf();
    let artifact_size = regular_file_size(&artifact);
    let timed_out =
        capture.timed_out || expired_under_timeout_tool(&capture, timeout_tool_used, budget);
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
        artifact,
        artifact_size,
        duration: capture.duration,
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
            resolved.display(),
            declared.display()
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
                path.display()
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
                manifest.path().display(),
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
            manifest.path().display(),
            posix_command_line(argv),
            posix_command_line(&recorded)
        ),
    ))
}

/// Prefix an invocation with the system timeout utility when the environment provides one.
///
/// Returns the vector to spawn and whether the utility is in it.
///
/// The utility is preferred over the harness's own watchdog for one reason that matters: it
/// signals the whole process group, and a compiler driver is ordinarily a process group. The
/// driver this module spawns starts sub-processes of its own — a preprocessor, a compiler proper,
/// an assembler, a linker — and it is one of *those* that hangs. Killing only the driver would
/// leave the sub-process running, holding the pipe this module is reading, so the harness would
/// go on waiting for a process it never started and cannot see.
///
/// The watchdog in [`spawn_bounded`] remains in force regardless, set to fire later, so the two
/// compose rather than compete: the utility acts first and reaches further, and the watchdog
/// covers the case where the utility does not act at all.
///
/// Where the utility is absent, or where its path cannot be rendered as text without loss, the
/// vector is returned untouched and the watchdog is the whole bound. That is a complete
/// substitute rather than a degradation of correctness — it bounds the invocation just as surely,
/// it simply cannot reach a grandchild — so it is taken silently rather than refused. Reporting
/// the untouched vector is also what keeps [`CompileOutcome::spawned_argv`] a faithful record of
/// what ran.
fn wrap_with_timeout_tool(
    argv: &[String],
    caps: &Capabilities,
    budget: Duration,
) -> (Vec<String>, bool) {
    let Some(tool) = caps.timeout_tool().path() else {
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
    /// Whether the harness's watchdog killed the child for outliving its bound.
    timed_out: bool,
    /// How the child terminated, or `None` when it was killed or became unobservable.
    status: Option<ExitStatus>,
    /// Wall-clock time from spawn to termination, excluding the time spent collecting output.
    duration: Duration,
}

/// Run one invocation inside a cell workspace, bounded, with its output captured.
///
/// Five properties hold together, and each closes a distinct way a compiler invocation can fail
/// to return or can reach outside the directory it was given:
///
/// - **The working directory is the cell's own workspace.** Set on the child rather than by
///   changing this process's directory, which would be a data race: the suite's area tests run
///   concurrently by default, and a process-wide change of directory made by one of them would
///   silently relocate every other test's relative paths. It is also what makes a compiler's
///   incidental output — a temporary file, a listing, an artifact written to a bare name — land
///   inside the one directory the cell owns.
/// - **Standard input is the null device**, so a driver that reads it sees end of file at once
///   rather than waiting for input that will never arrive.
/// - **Each output stream is drained by its own thread, bounded** at [`CAPTURE_BYTES_MAX`]. Two
///   hazards are closed by this: a driver that prints without end cannot exhaust memory, and the
///   classic deadlock in which a parent waits for a child that is itself blocked writing into a
///   pipe nobody is reading cannot occur.
/// - **The wait is bounded** by polling, because the standard library offers no timed wait on a
///   child, and the child is **always reaped**, so no zombie is left behind on any path.
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
    let deadline = started + bound;
    let stdout_reader = child.stdout.take().map(spawn_capped_reader);
    let stderr_reader = child.stderr.take().map(spawn_capped_reader);
    let (timed_out, status) = await_child_within_deadline(&mut child, deadline);
    // Measured before the harvest, so the reported duration is how long the compilation took and
    // not how long its diagnostics took to arrive.
    let duration = started.elapsed();
    let harvest_deadline = deadline.max(Instant::now() + CAPTURE_GRACE);
    let stdout = harvest_within_deadline(stdout_reader, harvest_deadline);
    let stderr = harvest_within_deadline(stderr_reader, harvest_deadline);
    Ok(Capture {
        stdout,
        stderr,
        timed_out,
        status,
        duration,
    })
}

/// Drain one output stream on its own thread, retaining at most [`CAPTURE_BYTES_MAX`] bytes.
///
/// Delivers through a channel rather than a join handle so the caller can give up on it. A join
/// handle offers no timed wait, so holding one would force the caller to block until the stream
/// reached end of file — which, as [`spawn_bounded`] explains, is an event a terminated compiler
/// driver does not guarantee.
///
/// A read error and a send failure are both discarded, and for the same reason: the bytes already
/// collected are worth exactly what they contain, a stream that failed mid-read is
/// indistinguishable from a driver that stopped printing, and a send that fails means the caller
/// has already reached its deadline and moved on. Neither can change a verdict, because a verdict
/// is decided by the *program's* output and never by the compiler's diagnostics.
fn spawn_capped_reader<R>(stream: R) -> Receiver<Vec<u8>>
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer: Vec<u8> = Vec::new();
        let mut bounded = stream.take(CAPTURE_BYTES_MAX);
        let _ = bounded.read_to_end(&mut buffer);
        let _ = sender.send(buffer);
    });
    receiver
}

/// Take a reader's bytes if they arrive by the deadline, and nothing if they do not.
///
/// A deadline already in the past yields a zero wait, which makes this a non-blocking poll —
/// exactly right after a timeout, where bytes already delivered are kept and nothing is waited
/// for.
fn harvest_within_deadline(reader: Option<Receiver<Vec<u8>>>, deadline: Instant) -> Vec<u8> {
    let Some(receiver) = reader else {
        return Vec::new();
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    receiver.recv_timeout(remaining).unwrap_or_default()
}

/// Wait for a child until the deadline, killing and reaping it if it outlives one.
///
/// Returns `(timed_out, status)`. A status is present on exactly one path — the child was
/// observed to finish on its own — and absent on the two paths where none exists to report: the
/// deadline was reached, or the status can no longer be observed at all.
///
/// Those two absences are deliberately not conflated. A child that outlived its budget is a
/// timeout, which is one of the suite's divergence classes and a real finding about a compiler. A
/// child whose status became unobservable is a condition of the machine, and reporting it as a
/// timeout would attribute a fault in this process's bookkeeping to the compiler it was watching.
fn await_child_within_deadline(child: &mut Child, deadline: Instant) -> (bool, Option<ExitStatus>) {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return (false, Some(status)),
            Ok(None) => {}
            Err(_) => {
                terminate_and_reap(child);
                return (false, None);
            }
        }
        if Instant::now() >= deadline {
            terminate_and_reap(child);
            return (true, None);
        }
        thread::sleep(WAIT_POLL_INTERVAL);
    }
}

/// Kill a child and wait for it, ignoring both results.
///
/// Both results are ignored on purpose. A kill fails when the child has already exited, and a
/// wait fails when it has already been reaped; either way the postcondition this function exists
/// to establish — that no process of ours is still running and none is left unreaped — already
/// holds.
fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// True when the system timeout utility, and not the compiler, produced this status.
///
/// Two conditions are required together, because the utility reports expiry by exiting with a
/// status a compiler is equally entitled to exit with. The status alone would misread a compiler
/// that legitimately exited 124 as a hang, so the elapsed time must corroborate it: a genuine
/// expiry cannot have taken materially less than the budget. [`TIMEOUT_ELAPSED_SLACK`] absorbs
/// the difference between the utility's own clock and this module's.
fn expired_under_timeout_tool(capture: &Capture, tool_used: bool, budget: Duration) -> bool {
    if !tool_used {
        return false;
    }
    let expired_status = capture
        .status
        .and_then(|status| status.code())
        .is_some_and(|code| code == TIMEOUT_TOOL_EXPIRED_STATUS);
    expired_status && capture.duration + TIMEOUT_ELAPSED_SLACK >= budget
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
/// judgement:
///
/// 1. **Timeout first.** An invocation that outlived its budget was killed, so whatever it had
///    printed by then is incidental. A hang is its own divergence class.
/// 2. **The bounding utility's own failure next**, because it exits with statuses that mean "I
///    could not run the command" rather than "the command failed". Reading one of those as a
///    compiler's verdict would attribute the machine's problem to the compiler.
/// 3. **A status that could not be observed next.** No verdict was seen, so none may be
///    inferred; this is a fault in this process's own bookkeeping and is answerable by the
///    machine.
/// 4. **A missing C runtime next**, ahead of every other link diagnostic. This is the test that
///    prevents the suite manufacturing findings: a static link that fails because a target's
///    start files were never installed has *exactly* the shape of one that fails because of a
///    defect in code generation, and only the probed runtime and the diagnostic together
///    distinguish them. Getting this wrong would turn a modestly provisioned machine into a
///    stream of false findings against a compiler that did nothing wrong.
/// 5. **A driver that cannot run its own stages next**, which is an installation fault reported
///    in the driver's own words.
/// 6. **Termination by a signal next.** The two tests above come first because attributing a
///    gap in the machine to a compiler is the more damaging mistake of the two; everything after
///    this point is a heuristic over diagnostic text, whereas a signal is an unambiguous fact
///    about the process, so it is established before any text is interpreted.
/// 7. **Other linker-stage diagnostics next**, which are genuine link failures — a distinct
///    class from a rejected translation, because the two implicate different parts of a compiler.
/// 8. **A diagnostic at a source location next**, which is a rejected program: the documented
///    diagnostic shape is `file:line:col: error:`, and a diagnostic that names a place in the
///    source is a statement about the source.
/// 9. **Anything else that failed** is a compile failure with the compiler answerable, which is
///    the conservative default: it keeps the full diagnostics attached for a human to adjudicate
///    rather than guessing a narrower class from evidence that did not match anything.
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
        let bound = if outcome.timeout_tool_used {
            "the system timeout utility"
        } else {
            "the harness watchdog"
        };
        return Some(BuildFailure::new(
            DivergenceClass::Timeout,
            FailureScope::Compiler,
            format!(
                "the {} did not terminate within its {}-second budget and was killed by {bound}; \
                 a compilation that completes promptly under one implementation and hangs under \
                 another is a defect worth surfacing rather than an infrastructure error{}",
                outcome.compiler().label(),
                outcome.budget().as_secs(),
                excerpt_clause(&diagnostics)
            ),
        ));
    }
    if outcome.timeout_tool_used {
        if let Some(code) = outcome.exit_code() {
            if TIMEOUT_TOOL_OWN_FAILURE_STATUSES.contains(&code) {
                return Some(BuildFailure::new(
                    DivergenceClass::CompileFailure,
                    FailureScope::Environment,
                    format!(
                        "the system timeout utility bounding this invocation reported {} rather \
                         than a status from the {}, so no compiler verdict was observed: {}{}",
                        code,
                        outcome.compiler().label(),
                        describe_timeout_tool_status(code),
                        excerpt_clause(&diagnostics)
                    ),
                ));
            }
        }
    }
    let Some(status) = outcome.status() else {
        return Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Environment,
            format!(
                "the status of the {} could not be observed, so no verdict about the program was \
                 seen and none may be inferred; this is a fault in this process's own bookkeeping \
                 rather than a judgement any compiler made{}",
                outcome.compiler().label(),
                excerpt_clause(&diagnostics)
            ),
        ));
    };
    if status.code() == Some(0) {
        return classify_successful_build(outcome);
    }
    if let Some(line) = first_line_matching_signature(&diagnostics, ENVIRONMENT_LINK_SIGNATURES) {
        let runtime = match caps.c_runtime_for(request.target()) {
            Some(status) => status.describe(),
            None => String::from("not probed"),
        };
        return Some(BuildFailure::new(
            DivergenceClass::LinkFailure,
            FailureScope::Environment,
            format!(
                "the static link for {} failed for want of a C runtime input rather than because \
                 of anything the {} produced, so this is a gap in this machine's provisioning and \
                 not a defect: {}. The probed runtime for that target is {}. Installing the \
                 target's development package restores the arm; until then it is reported as an \
                 environment gap, loudly, and never as a finding",
                request.target().triple(),
                outcome.compiler().label(),
                truncate_for_summary(&line),
                truncate_for_summary(&runtime)
            ),
        ));
    }
    if let Some(line) =
        first_line_matching_signature(&diagnostics, TOOLCHAIN_INSTALLATION_SIGNATURES)
    {
        return Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Environment,
            format!(
                "the {} could not run one of its own stages, which is an installation fault on \
                 this machine rather than a judgement about the program: {}",
                outcome.compiler().label(),
                truncate_for_summary(&line)
            ),
        ));
    }
    if status.code().is_none() {
        return Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} was terminated by a signal ({status}) rather than exiting, so it crashed \
                 on this program instead of forming a judgement about it. Recorded as a compile \
                 failure because no artifact was produced, and reported as a crash in the summary \
                 because a compiler that dies on a valid program looks nothing like one that \
                 declines it{}",
                outcome.compiler().label(),
                excerpt_clause(&diagnostics)
            ),
        ));
    }
    if let Some(line) = first_line_matching_signature(&diagnostics, LINKER_STAGE_SIGNATURES) {
        let (scope, corroboration) = link_scope(request, caps);
        return Some(BuildFailure::new(
            DivergenceClass::LinkFailure,
            scope,
            format!(
                "translation completed but the link did not, for {} at {}: {}. {corroboration}",
                request.target().triple(),
                outcome.opt().flag(),
                truncate_for_summary(&line)
            ),
        ));
    }
    if let Some(line) = first_source_location_error(&diagnostics) {
        return Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} rejected the program at a source location: {}. A program one \
                 implementation refuses and another accepts is exactly the asymmetry this suite \
                 exists to surface, so it is reported rather than treated as an error",
                outcome.compiler().label(),
                truncate_for_summary(&line)
            ),
        ));
    }
    Some(BuildFailure::new(
        DivergenceClass::CompileFailure,
        FailureScope::Compiler,
        format!(
            "the {} did not produce an artifact and its diagnostics match no recognised \
             signature, so the conservative class is recorded and the whole captured text is kept \
             for a human to adjudicate: {}{}",
            outcome.compiler().label(),
            describe_termination(outcome),
            excerpt_clause(&diagnostics)
        ),
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
                outcome.artifact().display()
            ),
        )),
        Some(0) => Some(BuildFailure::new(
            DivergenceClass::CompileFailure,
            FailureScope::Compiler,
            format!(
                "the {} reported success but left an empty file at {}; nothing can be executed \
                 from it, so the success is not one",
                outcome.compiler().label(),
                outcome.artifact().display()
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
            "its status could not be observed, which is a condition of this machine rather than a \
             judgement about the program",
        ),
    }
}

/// Explain what a status from the bounding utility means.
///
/// The three are conventional and are all statements about the utility rather than about the
/// command it was asked to run, which is precisely why they must not be read as a compiler's
/// verdict.
fn describe_timeout_tool_status(code: i32) -> &'static str {
    match code {
        125 => "the utility itself failed",
        126 => "the compiler was found but could not be executed",
        127 => "the compiler could not be found",
        _ => "the utility reported a failure of its own",
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
/// Two transformations, in this order and for two different reasons. Escaping comes first,
/// because captured bytes are untrusted: a tab would forge a column in a tab-separated report, a
/// carriage return would erase the line it ends, and an escape introducer would begin a terminal
/// sequence that could repaint a verdict the run never reached. Truncation comes second, because a
/// summary is one line and a compiler can print a great deal — and it is applied to the escaped
/// text so the limit bounds what a reader actually sees.
///
/// The elision is marked, so a truncated line is visibly truncated rather than merely short. The
/// unabridged bytes remain in [`CompileOutcome::stderr`] and are what a finding artifact records,
/// so nothing is lost by shortening what a summary shows.
fn truncate_for_summary(text: &str) -> String {
    let safe = sanitize_text_for_report(text.trim());
    let mut kept = String::with_capacity(safe.len().min(SUMMARY_EXCERPT_CHARS_MAX));
    for character in safe.chars().take(SUMMARY_EXCERPT_CHARS_MAX) {
        kept.push(character);
    }
    if kept.chars().count() < safe.chars().count() {
        kept.push_str(" [...]");
    }
    kept
}
