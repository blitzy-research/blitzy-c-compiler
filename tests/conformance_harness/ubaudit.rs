//! Undefined-behaviour freedom audit: the machine half of requirement 1.
//!
//! Every program in the corpus passes through two gates before any divergence it produces may be
//! read as evidence about a compiler. Both gates are driven by the **reference compiler alone**.
//!
//! # Why this module exists at all
//!
//! If a program contains undefined or unspecified behaviour, two compilers disagreeing about it
//! proves nothing about either of them, because both are permitted to do anything. Freedom from
//! undefined behaviour is therefore not a stylistic preference here — it is the precondition that
//! makes all three oracles sound, and it is **machine-enforced rather than asserted**. This module
//! is that enforcement; the `ub_notes` argument recorded in each program's own `.expected` file is
//! the human half, and it is required to be present for every program.
//!
//! # The two gates
//!
//! | Gate | Flags | Shape |
//! |---|---|---|
//! | Warning | [`UB_AUDIT_GATE_DEFAULT`] | compile only, no link |
//! | Sanitizer | [`SANITIZER_GATE_FLAGS`] | dynamically linked, native only, then **executed** |
//!
//! The warning gate is `-Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror`.
//! Because `-Werror` promotes every warning to an error, **any** diagnostic fails the gate. The
//! gate was measured to genuinely bite rather than decorate — it rejected a probe program on a
//! real diagnostic during design — so a failure here should be believed and treated as a defect in
//! the test program.
//!
//! The sanitizer gate is `-fsanitize=undefined,address -fno-sanitize-recover=all`. Compiling alone
//! would prove nothing about undefined behaviour, so the instrumented artifact is **run**, and its
//! termination and both of its streams are examined.
//!
//! # The reference compiler, and only the reference compiler
//!
//! Neither gate ever invokes the compiler under test, and that is a hard invariant rather than a
//! convention: every argument vector this module assembles is passed through
//! [`guard_invocation`], which refuses to spawn anything whose program is not the reference
//! compiler discovery vetted, and refuses any argument outside the small permitted set for that
//! gate.
//!
//! Two independent reasons make this non-negotiable:
//!
//! - **Shared-flag discipline (requirement 3).** A differential invocation may carry only flags
//!   both compilers honour with the same meaning. Every warning flag and every sanitizer flag is
//!   reference-only, so letting one reach the compiler under test would breach that discipline. The
//!   harness root records the same fact structurally: every member of the audit gate is also an
//!   entry of `FORBIDDEN_IN_DIFFERENTIAL`.
//! - **Sanitizers are documented as out of scope for the compiler under test.** The repository's
//!   technical specification lists "Sanitizers (ASan, TSan, UBSan) | Not supported" among its
//!   explicit exclusions, so `-fsanitize=` is not merely inadmissible in a shared invocation, it is
//!   unimplemented on that side.
//!
//! ## The sanitizer gate never renders a verdict about the compiler under test
//!
//! This is the single most important thing to understand about this module. A sanitizer diagnostic
//! means the **test program** is defective and must be rewritten by a human, in the corpus. It is
//! never a finding against the compiler under test, is never reported as one, and cannot be: the
//! compiler under test is not involved in either gate. The audit is a suite-authoring gate that
//! establishes the precondition under which a divergence is meaningful at all, which is why its
//! outcomes use the vocabulary in [`GateStatus`] rather than the oracle verdicts in
//! `super::Verdict`.
//!
//! # Two deliberate exceptions, which must not be "harmonized" away
//!
//! - **This is the only place in the suite that links dynamically.** Every differential artifact is
//!   statically linked, because that is the one linkage mode both compilers spell identically and
//!   the mode that needs no dynamic loader under emulation. The sanitizer runtimes need dynamic
//!   linking, so the instrumented build deliberately omits static linkage.
//! - **This is the only place in the suite where a sanitizer flag appears at all.**
//!
//! The sanitizer gate is also **native only**, because the sanitizer runtimes are not available for
//! the cross targets under emulation. That limitation is stated explicitly in the rendered report
//! rather than left implicit, and it costs nothing: undefined behaviour is a property of the
//! program, not of the target it is later built for.
//!
//! # Per-program deviations: narrow, sanctioned, and recorded
//!
//! A program that cannot pass the full warning gate records the gate it does want in its own
//! `ub_audit_flags`. A deviation is a **removal, never an addition**, so no option outside the gate
//! can be introduced through it, and exactly two categories are sanctioned:
//!
//! - The supported-extension area drops `-pedantic`, because an extension is non-standard by
//!   definition and that flag exists precisely to reject one.
//! - The deliberate narrowing-conversion program drops `-Wconversion` and `-Wsign-conversion`,
//!   because there a narrowing conversion is the behaviour under test rather than a mistake. The
//!   corpus contains exactly one such program, `01_integer_conversions/004_narrowing_conversions`.
//!
//! Both keep every other member, including `-Werror`. A deviation whose reason is not recorded in
//! `impl_defined_notes`, naming every flag it drops by that flag's exact spelling, is itself a defect
//! in the test and fails the gate; so, separately, is an empty `ub_notes`. Every deviation is
//! listed with its reason in the rendered report, so the complete set of relaxations is auditable
//! in one place. The gate is never weakened globally to make a stubborn program pass: that would
//! quietly re-admit the undefined behaviour this suite depends on excluding.
//!
//! # The authoring rulebook this gate enforces
//!
//! Every corpus program is written to obey all of the following, and the two gates are what make
//! the obedience checkable rather than claimed:
//!
//! - no signed overflow;
//! - shift counts strictly within range;
//! - no aliasing violations;
//! - no reads of uninitialized storage;
//! - one-past-end pointers may be formed but never dereferenced;
//! - no object modified twice between sequence points;
//! - at most one side-effecting argument per call;
//! - no dependence on padding bytes or on the relative addresses of unrelated objects.
//!
//! ## Determinism rules, which a violation would surface here first
//!
//! No addresses or pointer values are printed; no timestamps; no randomness; no locale-dependent
//! formatting; iteration order is fixed; floating-point values are printed at fixed precision with
//! margin; and an expected exit code lies within 0 to 125, because the operating system truncates
//! larger values — `return 300` was measured as status 44.
//!
//! ## Implementation-defined facts the corpus normalizes away
//!
//! A program that ignored one of these would produce a spurious cross-backend divergence rather
//! than a gate failure, so they are handled by construction in the corpus:
//!
//! - plain-`char` signedness is signed on x86-64 and i686 and **unsigned** on AArch64 and RISC-V
//!   64, hence explicit `signed char` and `unsigned char`;
//! - `sizeof(long)` and `sizeof(void *)` are 4 on i686 and 8 on the other three, hence width
//!   normalization;
//! - `sizeof(long double)` was measured at 16, 12, 16 and 16 bytes (x87 80-bit versus IEEE
//!   binary128), hence the per-oracle exclusion recorded by the long-double program itself.
//!
//! ## Headers, and why a program declares `printf` by hand
//!
//! Every program declares `int printf(const char *, ...);` by hand, the `_Noreturn` program
//! additionally declares `_Noreturn void exit(int);`, and no program includes a hosted header,
//! because the compiler under test bundles only freestanding headers and ships no `stdio.h`: an
//! `#include <stdio.h>` would fail on that side while succeeding on the reference side, which is a
//! spurious divergence caused by the test rather than by a compiler. The one sanctioned inclusion is
//! area 07's `<stdarg.h>` — freestanding, shipped by both compilers, and unavoidable, since a
//! variadic function cannot be written without it — and every program taking it records the exception
//! and its reason in its own `ub_notes`. A hand-declared prototype is also what published
//! output-comparison experience identifies as the fix for the most common portability problem in this
//! class of suite. Should a bare declaration ever provoke a diagnostic under the strict gate, the
//! correct resolution is a recorded per-program deviation, never a silent relaxation of the gate for
//! every program.
//!
//! # Coverage, and the absence of a silent skip
//!
//! Every program the corpus contains is audited under both gates; the corpus is enumerated by
//! [`manifest::discover_all`] rather than from any list written here, so adding a program needs no
//! change to this file. What it adds is two **gate applications** and three **process
//! invocations** — the counts differ because the sanitizer gate builds and then runs, and the run
//! is what makes it a gate about undefined behaviour rather than about compilability. Both are
//! reported, by [`AuditReport::gate_result_count`] and [`AuditReport::invocations_expected`]
//! respectively; conflating them understates what the audit does.
//!
//! A program that cannot be audited because the reference compiler is absent is reported
//! [`GateStatus::Unavailable`] — loudly, in the summary, and escalated to a failure under the
//! strict setting intended for continuous integration, where the toolchain is installed
//! deliberately. There is no path through this module by which a program is quietly not audited.
//!
//! # Workspace discipline, bounded execution, and parallel safety
//!
//! Each gate applied to each program gets its own workspace from
//! [`sandbox::audit_workspace`], beneath the audit root of the Cargo build directory. Paths are a
//! pure function of area, program and gate — no process identifier, no clock reading — so two
//! concurrently executing tests cannot collide and a reader of a report row can predict exactly
//! which directory to open. This module constructs no path outside that directory, opens no socket,
//! and treats the corpus as strictly read-only: a program is read and copied *into* a workspace,
//! never modified.
//!
//! The children it spawns are not confined by any of that. No namespace, `chroot`, syscall filter or
//! network restriction is applied, so the reference driver that hard-codes intermediates under the
//! system temporary directory keeps putting them there. Their **environment**, though, is not
//! inherited: the shared spawn path clears it, installs a vetted search path, points `TMPDIR`, `TMP`,
//! `TEMP` and `HOME` at the gate's own workspace, and — the part that matters most in this module —
//! forces `ASAN_OPTIONS` and `UBSAN_OPTIONS` to their strictest values, so an inherited setting
//! cannot weaken the diagnostic a gate exists to observe. The claim is therefore about the paths this
//! module builds, about what every child is told, and — for the program being audited — about the
//! corpus-authoring policy that gives every program its whole input as literals in its own source.
//!
//! Every invocation is bounded by the shared timed-wait facility in
//! [`run_command_captured_with`], using the discovered `timeout` utility when there is one
//! and a watchdog thread otherwise. The bound matters more here than anywhere else in the suite,
//! because this is the one gate that *runs* instrumented programs.
//!
//! # Bounded diagnostics retention, and why the bytes stay recoverable
//!
//! A gate's captured output is the point of the record — a status without the diagnostic that
//! justifies it is not something anyone can fix a program from — and it is also the one thing here
//! that would otherwise grow without a ceiling. Each stream is already capped by the execute layer,
//! but the audit holds **five** of them per program: two for the warning gate, and three for the
//! sanitizer gate, whose build and run are captured separately so neither overwrites the other. The
//! report-safe rendering can then expand a stream several-fold, because a byte that is not printable
//! becomes a visible escape. Across a hundred programs, and then again for the rendered report, an
//! audit of a toolchain that fails on every program would hold gigabytes of text in memory to say
//! what a few hundred lines already say.
//!
//! Three ceilings, therefore, and not one of them silent:
//!
//! - **Per gate result** — at most [`MAX_GATE_DIAGNOSTIC_LINES`] lines and
//!   [`MAX_GATE_DIAGNOSTIC_BYTES`] rendered bytes, taken from the **head** of the capture, because a
//!   compiler stops at its first error under `-Werror` and a sanitizer writes its `runtime error:`
//!   line before its stack trace: the head is the actionable part.
//! - **Per line** — [`MAX_GATE_DIAGNOSTIC_LINE_BYTES`], so that one pathological line contributes
//!   its head rather than being dropped whole and leaving a failing gate with no text at all.
//! - **Per run** — [`RUN_DIAGNOSTIC_BYTES_MAX`] across every gate result together, charged as each
//!   capture is taken rather than swept afterwards, so the bound holds throughout rather than
//!   eventually.
//! - **One buffer for the report** — [`AuditReport::render`] streams every section into a single
//!   string. The alternative it replaced concatenated five section strings, which held a second
//!   complete copy of the largest thing in the module at the moment of concatenation.
//!
//! A gate that **passed** retains nothing at all, because the failure section is the only place this
//! module renders captured output and a passing gate never appears in it. That is not merely a saving:
//! the allowance is charged in corpus order, so a passing program's ordinary standard output could
//! otherwise crowd out the diagnostic of a gate that failed later in the corpus — the one text an
//! author actually has to read.
//!
//! Every reduction is reported: [`Diagnostics::account`] states what was kept and what was left out,
//! in both lines and bytes, and the environment section states the run's total whether or not a
//! ceiling bit — because "no notice appeared" is not evidence that a report is complete.
//!
//! What keeps the loss recoverable is that the bytes were never only in memory. Each gate persists
//! every stream into its own workspace **before** the excerpt is taken, so a bounded result names
//! files that already exist; and for a gate that passed, whose workspace is discarded, the recorded
//! command lines reproduce the capture exactly. This is the same discipline
//! [`sandbox`](super::sandbox) applies to retained files, for the same reason: a record that quietly
//! said less than it appeared to would be worse than one that said nothing.
//!
//! # Report, do not patch
//!
//! A gate failure means a human rewrites the test program. No compiler source change is ever made
//! in response to anything this module reports, and this module never edits the corpus.
//!
//! Edition 2021, minimum supported Rust 1.70. Only the standard library is used, as the project
//! permits no third-party crate of any kind.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::env::{Capabilities, ToolRecord, VAR_ONLY, VAR_REF_CC, VAR_STRICT};
use super::execute::{
    budget_for, run_command_captured_with, CaptureNames, RunOutcome, Termination,
};
use super::manifest::{self, Manifest};
use super::sandbox::{self, Workspace, COMMANDS_NAME, PROGRAM_SOURCE_NAME};
use super::{
    comma_separated, ensure_within, is_bcc_target_selector, is_ub_audit_gate_member,
    is_ub_audit_gate_removable, posix_command_line, redact_secrets, require_regular_file,
    sanitize_text_for_report, shown_path, ub_audit_gate_required, HarnessError, HarnessResult,
    EXTENSION_AREA, PROGRAM_COUNT, UB_AUDIT_GATE_DEFAULT, UB_AUDIT_GATE_MANDATORY,
    UB_AUDIT_GATE_REMOVABLE,
};

/// The sanitizer gate, in the order the flags are passed.
///
/// Spelled once, here, and never combined with the warning gate: the two gates ask different
/// questions and a program that failed a merged invocation would not say which question it failed.
///
/// `-fno-sanitize-recover=all` is what turns a diagnostic into a termination rather than a note on
/// standard error that a passing exit code would then contradict. Without it a recoverable
/// diagnostic would be printed and the program would carry on to exit zero, and an audit that read
/// only the exit code would call that clean.
pub const SANITIZER_GATE_FLAGS: &[&str] =
    &["-fsanitize=undefined,address", "-fno-sanitize-recover=all"];

/// Text that identifies a sanitizer diagnostic in a captured stream.
///
/// The termination check is the primary signal and this is the corroborating one, present because
/// the two fail in different directions: a runtime that reported a diagnostic and still exited
/// cleanly would slip past a status test, while a program terminated for an unrelated reason would
/// slip past a text test.
///
/// Each entry is a phrase a sanitizer runtime writes and a corpus program has no reason to write:
/// the corpus prints `key=value` lines and nothing else. A program that did print one of these
/// would be flagged, which is the safe direction to be wrong in — a false alarm is investigated,
/// whereas a missed diagnostic silently invalidates every differential cell the program takes part
/// in.
pub const SANITIZER_DIAGNOSTIC_MARKERS: &[&str] = &[
    "runtime error:",
    "AddressSanitizer",
    "UndefinedBehaviorSanitizer",
    "LeakSanitizer",
    "MemorySanitizer",
    "ThreadSanitizer",
    "SUMMARY:",
];

/// The gate member whose removal is sanctioned in one feature area only.
///
/// Dropping it anywhere but [`EXTENSION_AREA`] discards a diagnostic that would be a genuine defect
/// in the test program, so the area is part of the check rather than a comment beside it. The
/// harness root is the authority on the flag tables; [`verify_gate_tables`] confirms this spelling
/// is one of its removable members, so a rename there cannot leave this check silently testing a
/// flag nobody passes.
const EXTENSION_ONLY_REMOVABLE: &str = "-pedantic";

/// Compile without linking, for the warning gate.
///
/// The gate asks about diagnostics, so linking would add a step that can fail for reasons the gate
/// is not asking about — and it keeps the question of linkage off this path entirely, leaving the
/// sanitizer gate as the only place in the suite where linkage is anything but static.
const COMPILE_ONLY_FLAG: &str = "-c";

/// Name the output explicitly rather than letting the driver derive it.
///
/// Both invocations name their output so that the recorded command line is complete and so that
/// nothing is written outside the gate's own workspace.
const OUTPUT_FLAG: &str = "-o";

/// Workspace entry the warning gate compiles into.
const WARNING_GATE_OBJECT_NAME: &str = "gate.o";

/// Workspace entry the sanitizer gate links into, and then executes.
const SANITIZER_ARTIFACT_NAME: &str = "sanitized.out";

/// Workspace entries the sanitizer *run* captures into.
///
/// Distinct from the build's entries on purpose: one workspace holds both invocations, and the
/// workspace's standard-error entry is shared, so reusing the build's names would let the run's
/// capture overwrite the build's diagnostics — exactly the text an author needs when a gate fails.
const SANITIZER_RUN_STDOUT_NAME: &str = "run.stdout";
const SANITIZER_RUN_STDERR_NAME: &str = "run.stderr";
const SANITIZER_RUN_EXIT_NAME: &str = "run.exit";

/// Most lines of captured output one gate result retains in memory.
///
/// Four hundred lines is far more than any gate failure needs to be actionable — a strict warning
/// gate stops the compiler at its first error and a sanitizer writes its diagnosis, its stack trace
/// and its summary in a few dozen lines — and it is small enough that a hundred programs cannot
/// accumulate a report nobody can hold. The excerpt is taken from the **head** of the capture for
/// exactly that reason: the first diagnostic is the one an author acts on, and the later ones are
/// usually cascades of it.
pub const MAX_GATE_DIAGNOSTIC_LINES: usize = 400;

/// Most bytes of rendered output one gate result retains in memory.
///
/// Bounds the line ceiling from the other side, because a single line can be pathologically long: a
/// diagnostic quoting a generated declaration, or a stream whose bytes are not text and therefore
/// expand fourfold when the report-safe rendering escapes each one. Either ceiling alone would leave
/// the other unguarded.
pub const MAX_GATE_DIAGNOSTIC_BYTES: usize = 64 * 1024;

/// Most bytes of one captured line the excerpt reproduces.
///
/// A diagnostic line a human reads is a few hundred bytes at most — a compiler names a file, a
/// position and a problem; a sanitizer names a frame — so two kilobytes is generous for the text that
/// carries the meaning. The cap exists so that a pathological line contributes its **head** rather
/// than being dropped whole: a single line longer than the per-result ceiling would otherwise leave a
/// failing gate with no text at all, and the head of `file.c:12:5: error: ...` is exactly the part an
/// author acts on. Elision is announced on the line itself, following the same convention the
/// comparator uses for an over-long rendered line.
pub const MAX_GATE_DIAGNOSTIC_LINE_BYTES: usize = 2 * 1024;

/// Most bytes of rendered output every gate result of one run retains together.
///
/// This is the ceiling the review's resource-management finding is really about. The audit holds
/// five sanitized streams per program — two for the warning gate, three for the sanitizer gate,
/// whose build and run are captured separately — and each is already capped by the execute layer at
/// a size the report-safe rendering can then expand several-fold. Across a corpus of a hundred
/// programs, an audit of a toolchain that fails on every one of them would otherwise hold gigabytes
/// of text to say what a few hundred lines already say. Four megabytes is more diagnostic text than
/// any single investigation reads, and it is charged continuously at the moment of retention rather
/// than swept afterwards, so the bound holds throughout the run rather than eventually.
pub const RUN_DIAGNOSTIC_BYTES_MAX: usize = 4 * 1024 * 1024;

/// Capacity the rendered report starts with, before the per-program and per-diagnostic allowance.
///
/// The fixed sections — the preamble, the environment, the gate table and the coverage figures —
/// are a little over eight kilobytes together. Sizing for them up front is what keeps
/// [`AuditReport::render`] to a single buffer that grows a bounded number of times instead of a
/// chain of concatenations that each held a complete copy of everything before it.
const REPORT_BASE_BYTES: usize = 16 * 1024;

/// Which ceiling stopped a gate result's diagnostics from being retained in full.
///
/// Named rather than reduced to a boolean because the three mean different things to a reader. Two
/// are properties of this one capture and say the stream was unusually large; the third says the
/// *run* had already retained its allowance, which is a statement about every program audited
/// before this one and is the case in which a reader should expect the same notice further down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticsBound {
    /// The per-result line ceiling, [`MAX_GATE_DIAGNOSTIC_LINES`].
    Lines,
    /// The per-result byte ceiling, [`MAX_GATE_DIAGNOSTIC_BYTES`].
    Bytes,
    /// The run-wide byte ceiling, [`RUN_DIAGNOSTIC_BYTES_MAX`], reached by an earlier result.
    Run,
}

impl DiagnosticsBound {
    /// The ceiling and its value, for the one-line account beside the excerpt.
    ///
    /// Reads the constants rather than restating their values, so a report can never advertise a
    /// bound other than the one that was applied.
    fn describe(self) -> String {
        match self {
            DiagnosticsBound::Lines => {
                format!("the per-result ceiling of {MAX_GATE_DIAGNOSTIC_LINES} line(s)")
            }
            DiagnosticsBound::Bytes => {
                format!("the per-result ceiling of {MAX_GATE_DIAGNOSTIC_BYTES} byte(s)")
            }
            DiagnosticsBound::Run => format!(
                "the run-wide ceiling of {RUN_DIAGNOSTIC_BYTES_MAX} byte(s), already spent by \
                 earlier gate results"
            ),
        }
    }
}

/// The run-wide ceiling on retained diagnostics, and what it has cost so far.
///
/// Threaded by `&mut` through [`run`] rather than held in a static, for two reasons. The audit is
/// performed once per run and executes sequentially, so a threaded budget is charged in corpus
/// order and the resulting report is a pure function of the corpus — the same property the
/// sequential audit exists to have. And a budget that is passed in can be reasoned about locally: no
/// caller has to know whether some other test already spent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsBudget {
    /// Rendered bytes retained by every gate result of this run so far.
    spent: usize,
    /// Gate results whose captured output was retained in full.
    complete: usize,
    /// Gate results reduced to an excerpt by one of the two per-result ceilings.
    bounded: usize,
    /// Gate results reduced to an excerpt because the run-wide ceiling was already spent.
    curtailed: usize,
}

impl DiagnosticsBudget {
    /// A fresh budget with nothing spent.
    fn new() -> DiagnosticsBudget {
        DiagnosticsBudget {
            spent: 0,
            complete: 0,
            bounded: 0,
            curtailed: 0,
        }
    }

    /// How many rendered bytes the next gate result may retain.
    ///
    /// The smaller of what the per-result ceiling allows and what the run has left, so neither
    /// ceiling can be exceeded by a result that satisfies the other.
    fn allowance(&self) -> usize {
        RUN_DIAGNOSTIC_BYTES_MAX
            .saturating_sub(self.spent)
            .min(MAX_GATE_DIAGNOSTIC_BYTES)
    }

    /// Charge one completed capture against the budget and record which bucket it fell into.
    ///
    /// A gate that was never invoked is not charged and is not counted: the buckets below add up to
    /// the number of captures actually taken, which is what makes the reported figure checkable
    /// against [`AuditReport::invocations_performed`].
    fn charge(&mut self, retained: usize, bound: Option<DiagnosticsBound>) {
        self.spent = self.spent.saturating_add(retained);
        match bound {
            None => self.complete += 1,
            Some(DiagnosticsBound::Run) => self.curtailed += 1,
            Some(DiagnosticsBound::Lines | DiagnosticsBound::Bytes) => self.bounded += 1,
        }
    }

    /// One line of accounting for the report's environment section.
    ///
    /// Stated whether or not a ceiling bit, because a reader has to be able to tell a report whose
    /// diagnostics are complete from one whose diagnostics are excerpts, and "no notice appeared"
    /// is not evidence of the former.
    fn describe(&self) -> String {
        format!(
            "{} of {RUN_DIAGNOSTIC_BYTES_MAX} byte(s) across {} capture(s) kept in full, {} \
             reduced by a per-result ceiling and {} by the run-wide ceiling; per result at most \
             {MAX_GATE_DIAGNOSTIC_LINES} line(s) and {MAX_GATE_DIAGNOSTIC_BYTES} byte(s), taken \
             from the head of the capture, and at most {MAX_GATE_DIAGNOSTIC_LINE_BYTES} byte(s) of \
             any one line. A gate that PASSED retains nothing, so the whole allowance stays \
             available to the gate failures whose diagnostics an author acts on; zero captures \
             means no gate failed rather than a bound that did not work",
            self.spent, self.complete, self.bounded, self.curtailed
        )
    }
}

/// One gate's captured output: a bounded excerpt, and the account of everything it was taken from.
///
/// The excerpt and the accounting are one value on purpose. A truncated diagnostic presented without
/// its counters is a fragment a reader would take for the whole thing, which is a worse record than
/// no record at all — the same reason the sandbox refuses to prune a retained file silently.
///
/// The counters describe the **capture**, in the decoded text the streams held, while the ceilings
/// apply to the **excerpt**, in the rendered bytes actually held in memory. The two are deliberately
/// different quantities: the rendered form is larger than the text it came from whenever a byte had
/// to be escaped, so bounding memory means bounding the rendered form, and telling a reader what was
/// left out means counting the captured form. Section headers and elision notices belong to the
/// excerpt's structure rather than to the capture, so they are charged against the ceilings and are
/// not counted as captured lines or bytes.
///
/// A line the per-line cap shortened counts as **retained**, because it was reached and is partly
/// shown; the bytes it did not reproduce appear in the omitted byte figure. That keeps the retained
/// and captured figures one quantity whose difference is exactly what a reader is not being shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    /// The retained text, in labelled sections, each line already made safe to render.
    excerpt: String,
    /// Lines the capture held in total, across every section.
    captured_lines: usize,
    /// Bytes of decoded text the capture held in total, across every section.
    captured_bytes: usize,
    /// Lines of captured text the excerpt reproduces.
    retained_lines: usize,
    /// Bytes of captured text the excerpt reproduces.
    retained_bytes: usize,
    /// Which ceiling stopped the retention, absent when the capture was retained in full.
    bound: Option<DiagnosticsBound>,
    /// Workspace entries holding the exact untruncated bytes, in section order.
    ///
    /// Every gate persists each stream into its own workspace *before* the excerpt is taken, so
    /// these name files that already exist rather than an intention to write them.
    sources: Vec<String>,
}

impl Diagnostics {
    /// The record for a gate whose output is not retained.
    ///
    /// Two cases, and both are deliberate. A gate that was **never invoked** — an unavailable driver,
    /// or a record so defective that the gate it asks for cannot be trusted — captured nothing to
    /// retain. And a gate that **passed** has nothing to explain: the failure section is the only
    /// place this module renders captured output, a passing gate never appears in it, and under
    /// `-Werror` a passing warning gate is provably silent anyway. Retaining a passing gate's stream
    /// would spend the run's allowance on text no reader can reach, and — because the allowance is
    /// charged in corpus order — could leave a later *failing* gate unable to show the diagnostic an
    /// author actually has to act on. The persisted streams are still on disk whenever the workspace
    /// was kept, so nothing becomes unrecoverable either way.
    fn none() -> Diagnostics {
        Diagnostics {
            excerpt: String::new(),
            captured_lines: 0,
            captured_bytes: 0,
            retained_lines: 0,
            retained_bytes: 0,
            bound: None,
            sources: Vec::new(),
        }
    }

    /// Retain what the budget allows of `sections`, and account for all of it.
    ///
    /// Each section is a label, the bytes it captured, and the workspace entry those exact bytes were
    /// persisted into. Empty sections are skipped entirely, so a gate that printed nothing renders as
    /// silence rather than as a run of empty headings.
    ///
    /// One pass, so a stream is decoded once: the counters are advanced for every line, and the
    /// excerpt is appended to only while no ceiling has bitten. Once one has, the remaining lines are
    /// still counted — which is what lets the account state how much was left out rather than merely
    /// that something was.
    ///
    /// A line longer than [`MAX_GATE_DIAGNOSTIC_LINE_BYTES`] contributes its head with the elision
    /// announced on the line itself, rather than being dropped: one such line would otherwise be able
    /// to exhaust a result's whole allowance and leave a failing gate showing nothing.
    fn capture(budget: &mut DiagnosticsBudget, sections: &[(&str, &[u8], &str)]) -> Diagnostics {
        let allowance = budget.allowance();
        // A shortfall against the per-result ceiling can only come from the run-wide one, so the
        // comparison below is what distinguishes an unusually large capture from a run that had
        // already spent its allowance on earlier programs.
        let byte_bound = if allowance < MAX_GATE_DIAGNOSTIC_BYTES {
            DiagnosticsBound::Run
        } else {
            DiagnosticsBound::Bytes
        };
        let mut record = Diagnostics::none();
        for (label, bytes, source) in sections {
            if bytes.is_empty() {
                continue;
            }
            record.sources.push(String::from(*source));
            let text = String::from_utf8_lossy(bytes);
            let mut header_pending = true;
            for line in text.trim_end_matches('\n').split('\n') {
                record.captured_lines += 1;
                // Counted in the decoded text as the report reproduces it — the content of the line
                // plus the one line feed that ends it — so the retained and captured figures are the
                // same quantity and their difference is exactly what a reader is not being shown.
                // It tracks the persisted stream's size on disk closely rather than exactly: bytes
                // that were not valid text were replaced when the stream was decoded, and trailing
                // blank lines are trimmed rather than reproduced.
                record.captured_bytes += line.len() + 1;
                if record.bound.is_some() {
                    continue;
                }
                if record.retained_lines >= MAX_GATE_DIAGNOSTIC_LINES {
                    record.bound = Some(DiagnosticsBound::Lines);
                    continue;
                }
                let (head, elided) = head_of_line(line);
                let mut addition = String::new();
                if header_pending {
                    addition.push_str(&format!("--- {label} ---\n"));
                }
                addition.push_str(&sanitize_line(head));
                if elided {
                    addition.push_str(&format!(
                        " [line truncated: the first {} of {} byte(s) are shown]",
                        head.len(),
                        line.len()
                    ));
                }
                addition.push('\n');
                if record.excerpt.len() + addition.len() > allowance {
                    record.bound = Some(byte_bound);
                    continue;
                }
                record.excerpt.push_str(&addition);
                header_pending = false;
                record.retained_lines += 1;
                // The head, not the whole line: a line the per-line cap elided is reached and
                // partly shown, so it counts as retained while the bytes it did not reproduce show
                // up in the omitted figure. That is what keeps the two counters one quantity.
                record.retained_bytes += head.len() + 1;
            }
        }
        budget.charge(record.excerpt.len(), record.bound);
        record
    }

    /// The retained text, in labelled sections, ready to be indented into a report.
    pub fn excerpt(&self) -> &str {
        &self.excerpt
    }

    /// Whether the invocations produced any output at all.
    ///
    /// Distinct from an empty excerpt, and the distinction is the point: a gate that printed nothing
    /// and a gate whose output was withheld to stay inside the run's ceiling must not render the
    /// same way, because one says the compiler was silent and the other says the report is not
    /// showing what it said.
    pub fn captured(&self) -> bool {
        self.captured_lines > 0
    }

    /// The one-line account of what was left out and where the exact bytes are, when anything was.
    ///
    /// `None` for a capture retained in full, which is the ordinary case and reads as silence.
    ///
    /// `workspace` is the gate's retained directory, present exactly when the gate failed or the run
    /// was asked to keep every workspace. When it is present the omitted bytes are named file by
    /// file, so recovering them is a matter of opening a path this report states. When it is absent
    /// the gate passed and its workspace was discarded, so the honest instruction is the command
    /// lines listed immediately above, which reproduce the capture exactly.
    pub fn account(&self, workspace: Option<&Path>) -> Option<String> {
        let bound = self.bound?;
        let mut text = format!(
            "[bounded by {}: the {} line(s) / {} byte(s) shown above are the head of a capture of \
             {} line(s) / {} byte(s), so {} line(s) / {} byte(s) are omitted here]",
            bound.describe(),
            self.retained_lines,
            self.retained_bytes,
            self.captured_lines,
            self.captured_bytes,
            self.captured_lines.saturating_sub(self.retained_lines),
            self.captured_bytes.saturating_sub(self.retained_bytes),
        );
        match workspace {
            Some(root) if !self.sources.is_empty() => {
                text.push_str(
                    " The exact untruncated bytes were written before this excerpt was taken and \
                     are on disk at: ",
                );
                for (index, source) in self.sources.iter().enumerate() {
                    if index > 0 {
                        text.push_str(", ");
                    }
                    text.push_str(&shown_path(&root.join(source)));
                }
                text.push('.');
            }
            _ => text.push_str(
                " This gate's workspace was not retained, so re-run the command line(s) listed \
                 above to reproduce the capture in full; nothing about it depends on this run.",
            ),
        }
        Some(text)
    }
}

/// One of the two gates every program passes through.
///
/// Both are reference-compiler-only and neither says anything about the compiler under test. They
/// are kept distinct rather than merged because they ask different questions — "does this program
/// provoke a diagnostic?" and "does this program actually perform undefined behaviour when it
/// runs?" — and a merged invocation would leave a failure unable to say which question it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gate {
    /// The strict warning gate: compile only, every diagnostic promoted to an error.
    Warning,
    /// The sanitizer gate: instrumented, dynamically linked, native, and executed.
    Sanitizer,
}

impl Gate {
    /// Both gates, in the order the audit applies them.
    ///
    /// The warning gate runs first because a program that cannot be compiled cleanly is a defect an
    /// author fixes before anything about its runtime behaviour is worth reporting.
    pub const ALL: [Gate; 2] = [Gate::Warning, Gate::Sanitizer];

    /// The single lowercase token used in a report row and as the workspace path component.
    ///
    /// Also the value handed to [`sandbox::audit_workspace`], so a retained directory is named
    /// after the gate that retained it.
    pub fn label(self) -> &'static str {
        match self {
            Gate::Warning => "warning",
            Gate::Sanitizer => "sanitizer",
        }
    }

    /// A one-line description of what this gate does, for the report header.
    pub fn description(self) -> &'static str {
        match self {
            Gate::Warning => {
                "compile only, driven by the native reference compiler, every diagnostic promoted \
                 to an error"
            }
            Gate::Sanitizer => {
                "instrumented build, dynamically linked, native only, then executed and required \
                 to terminate cleanly"
            }
        }
    }

    /// How many process invocations this gate makes per program.
    ///
    /// The warning gate compiles. The sanitizer gate builds and then runs, and the run is what
    /// makes it a gate about undefined behaviour rather than about compilability, so it counts two.
    /// The report uses this to state the invocation count honestly rather than assuming one per
    /// gate.
    pub fn invocations_per_program(self) -> usize {
        match self {
            Gate::Warning => 1,
            Gate::Sanitizer => 2,
        }
    }
}

impl fmt::Display for Gate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// The outcome of one gate applied to one program.
///
/// Deliberately **not** `super::Verdict`. That vocabulary — pass, expected divergence, unexpected
/// success, finding, failure — describes a comparison between a compiler and an oracle, and three
/// of its six values are judgements about the compiler under test. A gate says something else
/// entirely: whether a *test program* is fit to serve as evidence. Reusing the oracle verdicts
/// would invite exactly the confusion this module exists to prevent, in which a defective test
/// program is reported as a finding against a compiler that was never invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GateStatus {
    /// The gate was applied and the program satisfied it.
    Passed,
    /// The gate was applied and the program did not satisfy it, or its expectation record is
    /// defective. Either way the corpus must be corrected by a human; nothing about the compiler
    /// under test follows.
    Failed,
    /// The gate could not be applied because the reference compiler that drives it is absent.
    ///
    /// Reported loudly and never as a pass. Escalated to a failure under the strict setting
    /// intended for continuous integration, where the toolchain is installed deliberately and an
    /// absent reference compiler means a broken workflow rather than a modest machine.
    Unavailable,
}

impl GateStatus {
    /// Every status, in reporting order, so a tally is complete and ordered including zero rows.
    pub const ALL: [GateStatus; 3] = [
        GateStatus::Passed,
        GateStatus::Failed,
        GateStatus::Unavailable,
    ];

    /// The uppercase report token.
    pub fn label(self) -> &'static str {
        match self {
            GateStatus::Passed => "PASS",
            GateStatus::Failed => "FAIL",
            GateStatus::Unavailable => "UNAVAILABLE",
        }
    }

    /// True only for a gate that was applied and satisfied.
    pub fn passed(self) -> bool {
        matches!(self, GateStatus::Passed)
    }

    /// True for a gate the program did not satisfy. Always fails the audit.
    pub fn failed(self) -> bool {
        matches!(self, GateStatus::Failed)
    }
    /// True for a gate that could not be applied at all.
    pub fn unavailable(self) -> bool {
        matches!(self, GateStatus::Unavailable)
    }
}

impl fmt::Display for GateStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Everything one gate did to one program.
///
/// The fields are private because a caller that could assign them could report a status the
/// invocations never produced — and this record is what an author reads to fix a defective program,
/// so a status without the diagnostics that justify it would be worse than no record at all. Every
/// failing result carries the exact command lines and the compiler's or program's verbatim output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    gate: Gate,
    status: GateStatus,
    /// The gate flags actually passed, in the order they were passed. Empty only when the gate was
    /// never invoked.
    flags: Vec<String>,
    /// Every command line this gate ran, in execution order, shell-quoted so that each can be
    /// pasted into a terminal and reproduce the invocation exactly.
    commands: Vec<String>,
    /// The output the invocations produced, in labelled sections, already made safe to render line
    /// by line — bounded, and carrying the account of everything it was taken from. Empty when there
    /// was nothing to report; see [`Diagnostics`] for why an empty excerpt and an empty capture are
    /// deliberately distinguishable.
    diagnostics: Diagnostics,
    /// One line saying what happened and, for a failure, what it means.
    detail: String,
    /// The retained workspace, present exactly when this gate failed and its evidence was kept.
    workspace: Option<PathBuf>,
}

impl GateResult {
    /// Which gate this result belongs to.
    pub fn gate(&self) -> Gate {
        self.gate
    }

    /// Whether the program satisfied the gate, did not, or could not be judged.
    pub fn status(&self) -> GateStatus {
        self.status
    }

    /// The gate flags actually passed to the reference compiler.
    pub fn flags(&self) -> &[String] {
        &self.flags
    }

    /// Every command line this gate ran, in execution order.
    pub fn commands(&self) -> &[String] {
        &self.commands
    }

    /// The output captured from those commands, in labelled sections, with its retention account.
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// One line saying what happened.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// The retained workspace, when this gate failed and its evidence was kept.
    pub fn workspace(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }

    /// A single report row: the status, the gate and the explanation.
    pub fn describe(&self) -> String {
        format!("{} {} gate: {}", self.status, self.gate, self.detail)
    }

    /// Append a cleanup note to this gate's explanation, leaving its status untouched.
    ///
    /// The same sentence shape and the same sanitization [`conclude`] uses when a gate's own
    /// workspace resists removal, so a reader meets one convention rather than two. Deliberately
    /// unable to change the status, under the invariant `Workspace::discard_advisory` states: a
    /// tidy-up that did not work is untidy, not a failed gate.
    fn note_cleanup(&mut self, note: &str) {
        self.detail.push_str(". Cleanup note: ");
        self.detail.push_str(&sanitize_line(note));
    }

    /// Build a result for a gate that was applied.
    fn new(
        gate: Gate,
        status: GateStatus,
        flags: Vec<String>,
        commands: Vec<String>,
        diagnostics: Diagnostics,
        detail: String,
        workspace: Option<PathBuf>,
    ) -> GateResult {
        GateResult {
            gate,
            status,
            flags,
            commands,
            diagnostics,
            detail,
            workspace,
        }
    }

    /// Build a result for a gate that could not be applied because its driver is absent.
    ///
    /// Carries the diagnosis rather than a bare token, so a modest environment explains itself
    /// without the reader opening this file.
    fn unavailable(gate: Gate, diagnosis: &str) -> GateResult {
        GateResult::new(
            gate,
            GateStatus::Unavailable,
            Vec::new(),
            Vec::new(),
            Diagnostics::none(),
            format!(
                "not applied: {}. Both gates are driven by the native reference compiler, so its \
                 absence leaves this program unaudited. This is reported rather than passed over, \
                 and it becomes a failure under {VAR_STRICT}",
                sanitize_line(diagnosis)
            ),
            None,
        )
    }

    /// Build a result for a gate refused before any process was spawned, because the program's own
    /// expectation record is defective.
    ///
    /// A record defect is a failure rather than an unavailability: nothing about the environment is
    /// missing, and the gate the record asks for cannot be trusted to be the gate it should have
    /// asked for.
    fn record_defect(gate: Gate, defects: &[String]) -> GateResult {
        let joined = defects
            .iter()
            .map(|defect| sanitize_line(defect))
            .collect::<Vec<_>>()
            .join("; ");
        GateResult::new(
            gate,
            GateStatus::Failed,
            Vec::new(),
            Vec::new(),
            Diagnostics::none(),
            format!(
                "not invoked: the program's expectation record is defective, so the gate it asks \
                 for cannot be trusted — {joined}. Correct the record; the compiler under test is \
                 not involved in this judgement"
            ),
            None,
        )
    }
}

impl fmt::Display for GateResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.describe())
    }
}

/// One program's complete audit: the gate it asked for, why it asked for it, and both outcomes.
///
/// Holds the deviation facts as well as the results, because a relaxation that is not reported is
/// indistinguishable from a gate that was never strict. The report lists every deviation with its
/// recorded reason for exactly that purpose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramAudit {
    area: String,
    program: String,
    /// The corpus program, as the path discovery resolved it. This is the file an author edits, so
    /// it is the path every diagnostic and every command line names.
    source: PathBuf,
    /// The program's expectation record, absent only when it could not be read.
    record: Option<PathBuf>,
    /// The warning gate this program asked for: its own `ub_audit_flags` when it states them, and
    /// the default gate otherwise.
    gate_flags: Vec<String>,
    /// Whether that gate actually differs from the default gate. An explicit restatement of the
    /// default narrows nothing and is not a deviation.
    deviated: bool,
    /// The default-gate members this program drops, in gate order.
    dropped: Vec<String>,
    /// The reason recorded in the program's own record for the deviation, when there is one.
    reason: Option<String>,
    /// Whether this program's record carries the written undefined-behaviour-freedom argument.
    ///
    /// Requirement 1 has a machine half and a human half. An absent argument already fails the
    /// warning gate, so this flag never decides a verdict — it is recorded so the report can state
    /// the human half's coverage as a count and name each program that owes one, which is a
    /// different question from whether any gate failed and is not answerable from the gate results.
    ub_notes_recorded: bool,
    /// The two gate results.
    warning: GateResult,
    sanitizer: GateResult,
}

impl ProgramAudit {
    /// The `<area>/<program>` identity used in every report row and every diagnostic.
    pub fn label(&self) -> String {
        format!("{}/{}", self.area, self.program)
    }

    /// The corpus program this audit judged.
    pub fn source(&self) -> &Path {
        self.source.as_path()
    }

    /// The program's expectation record, absent only when it could not be read.
    pub fn record(&self) -> Option<&Path> {
        self.record.as_deref()
    }

    /// The warning gate this program asked for.
    pub fn gate_flags(&self) -> &[String] {
        &self.gate_flags
    }

    /// Whether that gate deviates from the default gate.
    pub fn deviated(&self) -> bool {
        self.deviated
    }

    /// The default-gate members this program drops.
    pub fn dropped_flags(&self) -> &[String] {
        &self.dropped
    }

    /// The recorded reason for the deviation, when this program deviates.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// The warning-gate result.
    pub fn warning(&self) -> &GateResult {
        &self.warning
    }

    /// The sanitizer-gate result.
    pub fn sanitizer(&self) -> &GateResult {
        &self.sanitizer
    }

    /// Both results, in [`Gate::ALL`] order.
    pub fn results(&self) -> [&GateResult; 2] {
        [&self.warning, &self.sanitizer]
    }

    /// The result for one named gate.
    pub fn result(&self, gate: Gate) -> &GateResult {
        match gate {
            Gate::Warning => &self.warning,
            Gate::Sanitizer => &self.sanitizer,
        }
    }

    /// The gates this program did not satisfy, in gate order.
    pub fn failures(&self) -> Vec<&GateResult> {
        self.results()
            .into_iter()
            .filter(|result| result.status().failed())
            .collect()
    }

    /// The number of process invocations this program's audit actually performed.
    ///
    /// Counted from the command lines recorded rather than assumed from the gate count, so a gate
    /// that was refused before spawning anything contributes nothing and the reported total is what
    /// really happened.
    pub fn invocations_performed(&self) -> usize {
        self.results()
            .iter()
            .map(|result| result.commands().len())
            .sum()
    }
    /// The feature-area directory this program lives in.
    pub fn area(&self) -> &str {
        &self.area
    }

    /// The program stem, without extension.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// True when both gates were applied and both were satisfied.
    pub fn passed(&self) -> bool {
        self.results().iter().all(|result| result.status().passed())
    }

    /// The gates that could not be applied to this program, in gate order.
    pub fn unavailable(&self) -> Vec<&GateResult> {
        self.results()
            .into_iter()
            .filter(|result| result.status().unavailable())
            .collect()
    }

    /// Whether the written undefined-behaviour-freedom argument is recorded.
    pub fn ub_notes_recorded(&self) -> bool {
        self.ub_notes_recorded
    }
}

impl fmt::Display for ProgramAudit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} {}",
            self.warning.status(),
            self.sanitizer.status(),
            self.label()
        )
    }
}

/// The result of auditing the whole corpus.
///
/// Produced only by [`run`], and read by the driver's infrastructure test, which asserts on it and
/// prints [`AuditReport::render`]. The fields are private for the same reason every other record in
/// this harness keeps them private: an assignable report could state a coverage figure the
/// invocations never produced, and this report is the evidence for requirement 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    /// One entry per audited program, in corpus order: feature-area table order, then file name.
    programs: Vec<ProgramAudit>,
    /// The reference compiler's own summary line, or its diagnosis when it is absent.
    reference: String,
    /// The reference compiler that drove both gates, absent when there was none.
    reference_path: Option<PathBuf>,
    /// Host architecture, operating system and kernel identification, for the report header.
    host: String,
    /// The per-invocation budget every spawn was bounded by.
    budget: Duration,
    /// Whether an unavailable gate must be escalated to a failure.
    strict: bool,
    /// Why no gate could be applied at all, when that is the case.
    unavailable_reason: Option<String>,
    /// The program filter in force, when one was set, in the spelling the variable accepts.
    filter: Option<String>,
    /// How many discovered programs the filter excluded.
    excluded_by_filter: usize,
    /// How many programs the corpus contains, whether or not the filter selected them.
    discovered: usize,
    /// Whether the run's build matrix was reduced by quick mode.
    ///
    /// Recorded so the report can state that quick mode narrowed the *matrix* and not the audit.
    /// Both gates are properties of the program rather than of a target-and-level cell, so there is
    /// nothing in them for a matrix reduction to narrow — and saying so is what stops a reader from
    /// assuming the audit was reduced too.
    quick: bool,
    /// What the run's diagnostics retention cost, and how many captures a ceiling reduced.
    ///
    /// Carried on the report so the bound is *reported* rather than merely applied. A reader who
    /// finds an excerpt where a complete diagnostic was expected can settle in one line whether the
    /// run as a whole was reducing captures or whether this one capture was unusually large.
    diagnostics: DiagnosticsBudget,
}

impl AuditReport {
    /// Every audited program, in corpus order.
    ///
    /// Every method below reads the corpus through this accessor rather than through the field, so
    /// there is exactly one read path for the audited set. That is what keeps a tally, a section
    /// heading and the per-program table from ever disagreeing about which programs were audited.
    pub fn programs(&self) -> &[ProgramAudit] {
        &self.programs
    }

    /// How many programs were audited.
    ///
    /// Reported as the measured number rather than compared against an expected one: this module
    /// enumerates the corpus and states what it found, and the driver owns any assertion about how
    /// large the corpus ought to be.
    pub fn program_count(&self) -> usize {
        self.programs().len()
    }

    /// How many programs the corpus contains, including any the filter excluded.
    pub fn discovered_count(&self) -> usize {
        self.discovered
    }

    /// How many gate results this report holds: one per gate per audited program.
    pub fn gate_result_count(&self) -> usize {
        self.programs().len() * Gate::ALL.len()
    }

    /// How many process invocations were actually performed.
    pub fn invocations_performed(&self) -> usize {
        self.programs()
            .iter()
            .map(ProgramAudit::invocations_performed)
            .sum()
    }

    /// How many invocations a fully applied audit of this many programs would make.
    ///
    /// The two gates make three invocations per program, because the sanitizer gate builds and then
    /// runs. Stating both this and [`AuditReport::invocations_performed`] is what keeps a degraded
    /// run visible: the two are equal exactly when every gate was applied.
    pub fn invocations_expected(&self) -> usize {
        self.programs().len()
            * Gate::ALL
                .iter()
                .map(|gate| gate.invocations_per_program())
                .sum::<usize>()
    }

    /// How many gate results carry `status`, across both gates.
    pub fn tally(&self, status: GateStatus) -> usize {
        self.programs()
            .iter()
            .flat_map(|audit| audit.results())
            .filter(|result| result.status() == status)
            .count()
    }

    /// How many results of one gate carry `status`.
    pub fn gate_tally(&self, gate: Gate, status: GateStatus) -> usize {
        self.programs()
            .iter()
            .filter(|audit| audit.result(gate).status() == status)
            .count()
    }

    /// Every gate a program did not satisfy, paired with the program, in corpus order.
    ///
    /// This is the list an author works through: each entry carries the program path, the exact
    /// command lines and the verbatim output.
    pub fn failures(&self) -> Vec<(&ProgramAudit, &GateResult)> {
        self.programs()
            .iter()
            .flat_map(|audit| {
                audit
                    .failures()
                    .into_iter()
                    .map(move |result| (audit, result))
            })
            .collect()
    }

    /// Every gate that could not be applied, paired with the program, in corpus order.
    ///
    /// The counterpart of [`AuditReport::failures`], and it exists for the same reason: the suite's
    /// standing rule is that an absent oracle is reported loudly and never passed over in silence,
    /// and a tally alone does not say *which* programs went unaudited. Without this list a reader
    /// knows that some gate did not run but not which precondition is therefore unestablished — and
    /// the whole purpose of these gates is to establish a precondition, program by program.
    pub fn unapplied(&self) -> Vec<(&ProgramAudit, &GateResult)> {
        self.programs()
            .iter()
            .flat_map(|audit| {
                audit
                    .unavailable()
                    .into_iter()
                    .map(move |result| (audit, result))
            })
            .collect()
    }

    /// How many audited programs satisfied **both** gates.
    ///
    /// Reported alongside the gate-result tallies rather than in place of them, because the two
    /// answer different questions. A gate tally says how many of the 2N gate applications held; this
    /// says how many programs are fit to serve as differential evidence, which is the number that
    /// actually bears on requirement 1 — a program that passed one gate and not the other is not
    /// half-eligible.
    pub fn fully_audited_count(&self) -> usize {
        self.programs()
            .iter()
            .filter(|audit| audit.passed())
            .count()
    }

    /// How many audited programs record the written undefined-behaviour-freedom argument.
    ///
    /// Requirement 1 has a machine half and a human half. The gates are the machine half; the
    /// argument recorded in each program's expectation record is the human half, and it is the first
    /// thing a reader consults when a divergence appears. Counting it here means the report states
    /// the human half's coverage rather than leaving it to be assumed.
    pub fn ub_notes_recorded_count(&self) -> usize {
        self.programs()
            .iter()
            .filter(|audit| audit.ub_notes_recorded())
            .count()
    }

    /// The distinct feature areas audited, in corpus order.
    ///
    /// The final deliverable summary must report the feature areas covered, so the audit states the
    /// areas it actually reached rather than restating the corpus tables.
    pub fn areas_audited(&self) -> Vec<&str> {
        let mut areas: Vec<&str> = Vec::new();
        for audit in self.programs() {
            if !areas.contains(&audit.area()) {
                areas.push(audit.area());
            }
        }
        areas
    }

    /// Every program whose warning gate deviates from the default, in corpus order.
    pub fn deviations(&self) -> Vec<&ProgramAudit> {
        self.programs()
            .iter()
            .filter(|audit| audit.deviated())
            .collect()
    }

    /// Why no gate could be applied, when the reference compiler is absent.
    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }

    /// True when at least one gate was not satisfied.
    pub fn has_failures(&self) -> bool {
        self.tally(GateStatus::Failed) > 0
    }

    /// True when at least one gate could not be applied.
    pub fn has_unavailable(&self) -> bool {
        self.tally(GateStatus::Unavailable) > 0
    }

    /// Whether this audit must fail the run.
    ///
    /// A gate that was not satisfied always fails, because the program taking part in the
    /// differential matrix has not been shown to be free of undefined behaviour and every
    /// divergence it produces would be unreadable. An unavailable gate fails only under the strict
    /// setting intended for continuous integration; outside it the gap is reported and does not
    /// fail, because there the environment rather than the corpus is what is incomplete.
    pub fn fails_run(&self) -> bool {
        self.has_failures() || (self.strict && self.has_unavailable())
    }

    /// The negation of [`AuditReport::fails_run`], for a caller that reads better in the positive.
    pub fn is_clean(&self) -> bool {
        !self.fails_run()
    }

    /// True when a filter meant this audit covered less than the whole corpus.
    ///
    /// The rendered report is stamped in that case, so a restricted run can never be mistaken for a
    /// complete one.
    pub fn is_partial(&self) -> bool {
        self.filter.is_some() || self.excluded_by_filter > 0
    }

    /// The whole audit as readable text: what was applied, what deviated and why, and every failure
    /// with its exact commands and verbatim output.
    ///
    /// Written to be the artifact an author acts on, so nothing is summarized away: a failing gate
    /// contributes its command lines and the compiler's own words, because a summary of a
    /// diagnostic is not something anyone can fix a program from.
    ///
    /// Deliberately free of timings. The same corpus on the same machine renders byte-identically,
    /// which is what lets two runs be compared directly; a duration would make every run differ and
    /// destroy that property for no diagnostic gain. Captured diagnostics are reproduced as they
    /// were captured, so a sanitizer report retains the machine-specific detail a reader needs.
    ///
    /// # One buffer, and why that matters here
    ///
    /// Every section streams into a single string. The obvious alternative — a section helper per
    /// part, each returning its own `String`, concatenated at the end — held a second complete copy
    /// of the report at the moment of concatenation, and the failure section of this particular
    /// report is made of captured compiler output, so that copy is the largest thing in the module.
    /// Bounding what is retained per gate result and per run is one half of keeping this report
    /// affordable; not duplicating it while rendering is the other.
    pub fn render(&self) -> String {
        // Sized for the fixed sections, the per-program table row and the diagnostics the run
        // actually retained, so growth is a bounded number of reallocations rather than one per
        // section. Every term is a figure this report already knows, so the hint costs no work.
        let mut out = String::with_capacity(
            REPORT_BASE_BYTES
                + self.diagnostics.spent.saturating_mul(2)
                + self.programs.len().saturating_mul(512),
        );
        self.render_into(&mut out);
        out
    }

    /// Stream the whole report into `out`, appending to whatever it already holds.
    ///
    /// The single writer every section shares. Kept separate from [`AuditReport::render`] so the
    /// buffer is allocated once, in one place, with one capacity hint.
    fn render_into(&self, out: &mut String) {
        out.push_str("undefined-behaviour freedom audit — requirement 1, machine half\n");
        out.push_str("===============================================================\n\n");
        out.push_str(
            "Both gates are driven by the reference compiler alone, and neither renders a verdict\n\
             about the compiler under test. A gate failure is a defect in the TEST PROGRAM, to be\n\
             corrected in the corpus by a human; it is never a finding against a compiler, because\n\
             no compiler under test is invoked here at all. The gates establish the precondition\n\
             that makes a divergence readable as evidence: a program containing undefined or\n\
             unspecified behaviour proves nothing when two compilers disagree about it.\n\n",
        );

        if let Some(reason) = &self.unavailable_reason {
            out.push_str("UNAVAILABLE\n");
            out.push_str("-----------\n");
            indent_block_into(out, reason, "  ");
            out.push('\n');
        }
        if let Some(filter) = &self.filter {
            out.push_str("PARTIAL RUN\n");
            out.push_str("-----------\n");
            indent_block_into(
                out,
                &format!(
                    "the audit was restricted to {filter} by {VAR_ONLY}, so {} of the {} programs \
                     the corpus contains were not audited. This report is PARTIAL and must not be \
                     read as a complete audit of the corpus.",
                    self.excluded_by_filter, self.discovered
                ),
                "  ",
            );
            out.push('\n');
        }

        out.push_str("environment\n");
        out.push_str("-----------\n");
        out.push_str(&format!(
            "  reference compiler    : {}\n",
            sanitize_line(&self.reference)
        ));
        out.push_str(&format!(
            "  audit driver          : {}\n",
            match &self.reference_path {
                Some(path) => shown_path(path),
                None => String::from("(none: both gates are unavailable)"),
            }
        ));
        out.push_str(&format!("  host                  : {}\n", self.host));
        out.push_str(&format!(
            "  per-invocation budget : {} s\n",
            self.budget.as_secs()
        ));
        // Stated unconditionally, beside the other bound this audit applies. A retention ceiling
        // that only announced itself when it bit would leave a reader unable to tell a report whose
        // diagnostics are complete from one whose diagnostics are excerpts, and the difference is
        // exactly what decides whether the text below can be acted on directly.
        indent_block_into(
            out,
            &format!("diagnostics retention : {}", self.diagnostics.describe()),
            "  ",
        );
        out.push_str(&format!(
            "  unavailable policy    : {}\n",
            if self.strict {
                format!("an unavailable gate FAILS the run ({VAR_STRICT} is set)")
            } else {
                format!(
                    "an unavailable gate is reported and does not fail the run ({VAR_STRICT} is \
                     unset)"
                )
            }
        ));
        if self.quick {
            out.push_str(
                "  quick mode            : set, and it narrows the build matrix rather than this\n\
                 \x20                         audit: both gates are properties of the program, not\n\
                 \x20                         of a target-and-level cell, so every program is still\n\
                 \x20                         audited under both\n",
            );
        }
        out.push('\n');

        out.push_str("gates\n");
        out.push_str("-----\n");
        for gate in Gate::ALL {
            out.push_str(&format!(
                "  {:<9} : {}\n",
                gate.label(),
                gate_flag_line(gate)
            ));
            out.push_str(&format!("  {:<9}   {}\n", "", gate.description()));
        }
        out.push_str(
            "  sanitizer   the instrumented build is dynamically linked and executed on the host\n\
             \x20             only. The sanitizer runtimes are not available for the cross targets\n\
             \x20             under emulation, and this is the only place in the suite that links\n\
             \x20             dynamically or names a sanitizer flag at all — both deliberately.\n",
        );
        out.push('\n');

        out.push_str("coverage\n");
        out.push_str("--------\n");
        out.push_str(&format!("  programs in corpus    : {}\n", self.discovered));
        out.push_str(&format!(
            "  programs audited      : {}\n",
            self.program_count()
        ));
        if self.discovered != PROGRAM_COUNT {
            out.push_str(&format!(
                "  note                  : the corpus tables declare {PROGRAM_COUNT} programs and \
                 {} were discovered; this module reports what it found and asserts nothing about \
                 the expected size\n",
                self.discovered
            ));
        }
        out.push_str(&format!(
            "  feature areas audited : {} ({})\n",
            self.areas_audited().len(),
            if self.areas_audited().is_empty() {
                String::from("none")
            } else {
                sanitize_line(&self.areas_audited().join(", "))
            }
        ));
        out.push_str(&format!(
            "  gate results          : {} ({} gates per program)\n",
            self.gate_result_count(),
            Gate::ALL.len()
        ));
        // Requirement 1 has two halves and this report is named for one of them, so the other has
        // to be visible here too. The gates below already fail a program that records no written
        // argument, which means a clean audit implies a complete set — but "implies" is exactly
        // what evidence should not require a reader to work out. Stating the count says the human
        // half was checked, rather than leaving it to be inferred from the absence of a failure.
        out.push_str(&format!(
            "  ub-freedom arguments  : {} of {} audited program(s) record a written argument \
             (requirement 1's human half; a program without one fails the warning gate){}\n",
            self.ub_notes_recorded_count(),
            self.program_count(),
            if self.ub_notes_recorded_count() == self.program_count() {
                ""
            } else {
                " — INCOMPLETE"
            }
        ));
        out.push_str(&format!(
            "  satisfied both gates  : {} of {} program(s)\n",
            self.fully_audited_count(),
            self.program_count()
        ));
        out.push_str(&format!(
            "  invocations performed : {} of {} expected (per program: one warning-gate compile, \
             one sanitizer build, one sanitizer run)\n",
            self.invocations_performed(),
            self.invocations_expected()
        ));
        for gate in Gate::ALL {
            out.push_str(&format!(
                "  {:<9} gate outcome: {}\n",
                gate.label(),
                GateStatus::ALL
                    .iter()
                    .map(|status| format!("{} {}", status, self.gate_tally(gate, *status)))
                    .collect::<Vec<_>>()
                    .join("  ")
            ));
        }
        out.push_str(&format!(
            "  audit verdict         : {}\n\n",
            self.verdict_line()
        ));

        self.render_area_coverage_into(out);
        self.render_deviations_into(out);
        self.render_failures_into(out);
        self.render_unapplied_into(out);
        self.render_program_table_into(out);
    }

    /// One row per feature area: the programs it contributes, how many satisfied both gates, how
    /// many gate results could not be applied at all, how many deviate, and how many carry the
    /// written undefined-behaviour-freedom argument.
    ///
    /// # Why the written argument is counted, and counted here
    ///
    /// Requirement 1 has two halves. The gates above are the machine half. The written argument in
    /// each program's own expectation record is the **human** half: it is what a reviewer reads
    /// first when a divergence appears, and it is the one part of requirement 1 that no gate can
    /// establish, because a compiler cannot be asked whether a program's author understood why the
    /// program is free of undefined behaviour. An audit that measured the machine half and stayed
    /// silent about the human one would present requirement 1 as satisfied on half its evidence.
    ///
    /// The *presence* of the argument is already enforced elsewhere and is not re-decided here: a
    /// record with no `ub_notes` key at all fails to parse, and one whose value is empty is
    /// recorded as a defect by [`decide_warning_gate`], which fails the gate. So a row here whose
    /// written-argument count falls short of its program count always belongs to a run that is
    /// already failing. What this table adds is *where*: the failure listing states the defect once
    /// per program, and a systematic omission across one feature area is a pattern only a per-area
    /// count makes visible.
    ///
    /// What no gate can decide is whether a recorded argument is *convincing*. That is a review
    /// matter, and it is why the argument is named beside the program rather than reduced to a
    /// corpus-wide total a reader could not act on.
    ///
    /// Grouped in first-appearance order, which is corpus order, so the table renders identically
    /// on every run of the same corpus.
    fn render_area_coverage_into(&self, out: &mut String) {
        out.push_str("per-feature-area coverage\n");
        out.push_str("-------------------------\n");
        let programs = self.programs();
        if programs.is_empty() {
            out.push_str("  (no program was audited)\n\n");
            return;
        }
        let mut areas: Vec<&str> = Vec::new();
        for audit in programs {
            if !areas.contains(&audit.area()) {
                areas.push(audit.area());
            }
        }
        for area in areas {
            let members: Vec<&ProgramAudit> = programs
                .iter()
                .filter(|audit| audit.area() == area)
                .collect();
            let satisfied = members.iter().filter(|audit| audit.passed()).count();
            let not_applied: usize = members.iter().map(|audit| audit.unavailable().len()).sum();
            let deviating = members.iter().filter(|audit| audit.deviated()).count();
            let argued = members
                .iter()
                .filter(|audit| audit.ub_notes_recorded())
                .count();
            out.push_str(&format!(
                "  {:<28} {:>3} program(s), {:>3} satisfied both gates, {:>3} gate result(s) not \
                 applied, {:>3} deviating, {:>3} with a written argument\n",
                sanitize_line(area),
                members.len(),
                satisfied,
                not_applied,
                deviating,
                argued
            ));
            for audit in members.iter().filter(|audit| !audit.ub_notes_recorded()) {
                out.push_str(&format!(
                    "      NO WRITTEN ARGUMENT: {} — requirement 1's human half is absent from \
                     this program's expectation record; the gates judged the program, but no \
                     recorded reasoning explains why it is free of undefined behaviour\n",
                    sanitize_line(audit.program())
                ));
            }
        }
        out.push('\n');
    }

    /// One line stating whether the audit passes, and why not when it does not.
    fn verdict_line(&self) -> String {
        let failed = self.tally(GateStatus::Failed);
        let unavailable = self.tally(GateStatus::Unavailable);
        if failed > 0 {
            return format!(
                "FAILING — {failed} gate result(s) were not satisfied, so the programs concerned \
                 have not been shown free of undefined behaviour and every divergence they produce \
                 would be unreadable. Correct the programs listed below"
            );
        }
        if unavailable > 0 && self.strict {
            return format!(
                "FAILING — every applied gate was satisfied, but {unavailable} gate result(s) could \
                 not be applied at all and {VAR_STRICT} is set, where a missing oracle means a \
                 broken workflow rather than a modest machine"
            );
        }
        if unavailable > 0 {
            return format!(
                "REPORTED GAP — every applied gate was satisfied, but {unavailable} gate result(s) \
                 could not be applied. This does not fail the run outside {VAR_STRICT}, and it is \
                 not a pass: the programs concerned were not audited"
            );
        }
        String::from("CLEAN — every audited program satisfied both gates")
    }

    /// The recorded deviations, each with the gate it uses, the members it drops and its reason.
    fn render_deviations_into(&self, out: &mut String) {
        let deviations = self.deviations();
        out.push_str(&format!(
            "recorded warning-gate deviations ({})\n",
            deviations.len()
        ));
        out.push_str("-------------------------------------\n");
        if deviations.is_empty() {
            out.push_str("  (none: every audited program passed the full default gate)\n\n");
            return;
        }
        // The two lists are read from the gate tables rather than restated, for the same reason
        // `gate_flag_line` reads them: a report that names its own flags can advertise a guarantee
        // other than the one the audit enforces. Restating one flag also understated the guarantee
        // — four members are non-negotiable, not just the one that turns diagnostics into errors.
        out.push_str(&format!(
            "  A deviation is a removal and never an addition, always retains {}, and is \
             recorded\n  with its reason in the program's own expectation record. Only {} may be \
             dropped at all, and\n  exactly two categories are sanctioned: the supported-extension \
             area drops {EXTENSION_ONLY_REMOVABLE},\n  and the one deliberate \
             narrowing-conversion program drops the two conversion diagnostics.\n\n",
            comma_separated(&ub_audit_gate_required()),
            comma_separated(UB_AUDIT_GATE_REMOVABLE),
        ));
        for audit in deviations {
            out.push_str(&format!("  {}\n", sanitize_line(&audit.label())));
            out.push_str(&format!(
                "    gate    : {}\n",
                sanitize_line(&audit.gate_flags().join(" "))
            ));
            out.push_str(&format!(
                "    dropped : {}\n",
                sanitize_line(&audit.dropped_flags().join(" "))
            ));
            match audit.reason() {
                Some(reason) => {
                    out.push_str("    reason  :\n");
                    indent_block_into(out, reason, "      ");
                }
                None => out.push_str(
                    "    reason  : NONE RECORDED — this is itself a defect in the test, and the \
                     gate failed for it\n",
                ),
            }
            out.push('\n');
        }
    }

    /// Every failure in full: the program, the gate, the exact commands and the verbatim output.
    fn render_failures_into(&self, out: &mut String) {
        let failures = self.failures();
        out.push_str(&format!("gate failures ({})\n", failures.len()));
        out.push_str("------------------\n");
        if failures.is_empty() {
            out.push_str("  (none)\n\n");
            return;
        }
        out.push_str(
            "  Each entry is a defect in the TEST PROGRAM or in its expectation record. Fix the \
             corpus;\n  never weaken the gate globally, and never change a compiler in response to \
             one of these.\n\n",
        );
        for (audit, result) in failures {
            out.push_str(&format!(
                "  {} — {} gate\n",
                sanitize_line(&audit.label()),
                result.gate()
            ));
            out.push_str(&format!("    program : {}\n", shown_path(audit.source())));
            if let Some(record) = audit.record() {
                out.push_str(&format!("    record  : {}\n", shown_path(record)));
            }
            out.push_str("    detail  :\n");
            indent_block_into(out, result.detail(), "      ");
            if !result.flags().is_empty() {
                out.push_str(&format!(
                    "    flags   : {}\n",
                    sanitize_line(&result.flags().join(" "))
                ));
            }
            if result.commands().is_empty() {
                out.push_str(
                    "    commands: (none: the gate was refused before any process was spawned)\n",
                );
            } else {
                out.push_str("    commands:\n");
                for command in result.commands() {
                    out.push_str(&format!("      $ {}\n", sanitize_line(command)));
                }
            }
            if let Some(workspace) = result.workspace() {
                out.push_str(&format!("    retained: {}\n", shown_path(workspace)));
            }
            // A gate that printed nothing and a gate whose output a ceiling reduced are rendered
            // differently on purpose: the first is a compiler that was silent, the second is a
            // report that is not showing everything the compiler said. Conflating them would put a
            // reader in the one position this section exists to prevent — acting on a fragment in
            // the belief that it is the whole diagnostic.
            let diagnostics = result.diagnostics();
            if diagnostics.captured() {
                out.push_str("    output  :\n");
                if !diagnostics.excerpt().is_empty() {
                    indent_block_into(out, diagnostics.excerpt(), "      ");
                }
                if let Some(account) = diagnostics.account(result.workspace()) {
                    indent_block_into(out, &account, "      ");
                }
            } else {
                out.push_str("    output  : (empty)\n");
            }
            out.push('\n');
        }
    }

    /// Every gate that could not be applied, named program by program.
    ///
    /// Separate from the failure section on purpose, because the two call for different actions. A
    /// failure is a defect in a test program and is fixed by editing the corpus. An unapplied gate
    /// is a gap in this machine, fixed by installing a tool — and until it is, the programs listed
    /// here take part in the differential matrix without their precondition having been
    /// established. Saying which ones, rather than only how many, is what makes that gap actionable
    /// instead of merely acknowledged.
    fn render_unapplied_into(&self, out: &mut String) {
        let unapplied = self.unapplied();
        out.push_str(&format!(
            "gates that could not be applied ({})\n",
            unapplied.len()
        ));
        out.push_str("------------------------------------\n");
        if unapplied.is_empty() {
            out.push_str("  (none: every gate was applied to every audited program)\n\n");
            return;
        }
        // Indented by `indented_block` rather than by hand, so every line of the paragraph carries
        // the section's indent and is sanitized on the same terms as the entries below it. The line
        // breaks are explicit because this is a plain-text report read in a terminal, and the helper
        // indents rather than reflows.
        indent_block_into(
            out,
            &format!(
                "Each entry names a program that was NOT audited under the gate given. This is a \
                 gap in\nTHIS MACHINE rather than a defect in the program: the precondition \
                 requirement 1 asks\nfor is unestablished for that program, and it takes part in \
                 the differential matrix\nregardless. This is never a pass. Under {VAR_STRICT} each \
                 entry fails the run, because\nthere the toolchain is installed deliberately and an \
                 absent tool means a broken\nworkflow rather than a modest machine.",
            ),
            "  ",
        );
        out.push('\n');
        for (audit, result) in unapplied {
            out.push_str(&format!(
                "  {} / {} — {} gate\n",
                sanitize_line(audit.area()),
                sanitize_line(audit.program()),
                result.gate()
            ));
            out.push_str(&format!("    program : {}\n", shown_path(audit.source())));
            out.push_str("    detail  :\n");
            indent_block_into(out, result.detail(), "      ");
            out.push('\n');
        }
    }

    /// One row per program, so the complete outcome is in the report even when nothing failed.
    fn render_program_table_into(&self, out: &mut String) {
        out.push_str("per-program results (warning gate, sanitizer gate)\n");
        out.push_str("--------------------------------------------------\n");
        if self.programs().is_empty() {
            out.push_str("  (no program was audited)\n");
            return;
        }
        for audit in self.programs() {
            out.push_str(&format!(
                "  {:<11} {:<11} {}{}\n",
                audit.warning().status(),
                audit.sanitizer().status(),
                sanitize_line(&audit.label()),
                if audit.deviated() { " [deviates]" } else { "" }
            ));
        }
    }
}

/// The gate flags of `gate` as one space-separated line, for the report header.
///
/// Reads the tables rather than restating them, so the report can never advertise a gate other than
/// the one that runs.
fn gate_flag_line(gate: Gate) -> String {
    match gate {
        Gate::Warning => UB_AUDIT_GATE_DEFAULT.join(" "),
        Gate::Sanitizer => SANITIZER_GATE_FLAGS.join(" "),
    }
}

/// One line of text, made safe to render.
///
/// Two transformations, in this order, and the order matters: redaction reads names and values, so
/// it must see the text before escaping rewrites it.
///
/// [`redact_secrets`] first. A gate command line carries every flag the audit passes, an audit
/// invocation records the sanitizer options it forced, and a diagnostic excerpt can quote whatever
/// the compiler was told — so a value that looks like a credential can reach this report from a
/// definition on a command line. Redacting it here keeps it out of the rendered audit and out of
/// any log that captures one.
///
/// Then [`sanitize_text_for_report`]. Every control character and every formatting character that
/// could forge a column, erase the line it ends or reverse how the line reads is replaced by a
/// visible escape, so nothing a compiler or a corpus path puts into this report can act rather than
/// merely say.
fn sanitize_line(raw: &str) -> String {
    sanitize_text_for_report(&redact_secrets(raw))
}

/// The leading [`MAX_GATE_DIAGNOSTIC_LINE_BYTES`] of `line`, and whether anything was left off.
///
/// The cut is moved back to a character boundary, so a multi-byte sequence is never split: the
/// remainder would render as replacement characters and read as corruption rather than as elision.
/// Pure, so the same line always yields the same head and the report stays byte-deterministic.
fn head_of_line(line: &str) -> (&str, bool) {
    if line.len() <= MAX_GATE_DIAGNOSTIC_LINE_BYTES {
        return (line, false);
    }
    let mut cut = MAX_GATE_DIAGNOSTIC_LINE_BYTES;
    while cut > 0 && !line.is_char_boundary(cut) {
        cut -= 1;
    }
    (&line[..cut], true)
}

/// Append multi-line text to `out`, each line sanitized and indented, with a trailing newline.
///
/// Sanitization is per line rather than over the whole block because the report-safe rendering
/// escapes the line feed too — it exists for single-line values — so a block passed through it
/// whole would collapse into one very long line carrying literal `\x0a` sequences.
///
/// Appends into the caller's buffer rather than returning a block of its own. The report's failure
/// section is made of captured compiler output, and a helper that returned a `String` would allocate
/// a second copy of every one of those blocks on the way into the report — the same duplication
/// [`AuditReport::render`] avoids at the section level, avoided here at the block level.
fn indent_block_into(out: &mut String, raw: &str, indent: &str) {
    let trimmed = raw.trim_end_matches('\n');
    if trimmed.is_empty() {
        out.push_str(indent);
        out.push_str("(empty)\n");
        return;
    }
    for line in trimmed.split('\n') {
        out.push_str(indent);
        out.push_str(&sanitize_line(line));
        out.push('\n');
    }
}

/// Audit every program in the corpus under both gates.
///
/// The corpus is enumerated by [`manifest::discover_all`], so nothing here is hard-coded and no
/// program can be omitted by an oversight in this file: a feature area that is absent or empty
/// fails discovery outright rather than shrinking the audit quietly.
///
/// Execution is sequential, and deliberately so. The measured cost is around thirty milliseconds
/// per invocation, which is a small fraction of the differential matrix, and a single-threaded pass
/// makes the rendered report a pure function of the corpus — two runs on one machine render
/// identically, which is what lets them be compared directly.
///
/// # Errors
///
/// A hard failure is reserved for something that makes the audit itself meaningless rather than for
/// anything a program can do wrong: the gate tables disagreeing with the harness root, a corpus
/// that cannot be enumerated, a workspace that cannot be allocated or written, a process that
/// cannot be spawned or reaped, or a program filter that selected nothing. A program that fails a
/// gate — or one whose expectation record cannot be read — is a **result** in the returned report,
/// never an error, because the report must list every outcome rather than stop at the first.
pub fn run(caps: &Capabilities) -> HarnessResult<AuditReport> {
    verify_gate_tables()?;

    let config = caps.config();
    let reference = caps.ref_cc_native();
    let driver = reference.path();
    let unavailable_reason = unavailability_of(reference);

    let discovered = manifest::discover_all()?;
    let filter = config.only();

    // The run's diagnostics ceiling, charged as each capture is taken rather than swept afterwards,
    // so the bound holds continuously. Threaded through the loop instead of held in a static because
    // the audit is sequential: charging in corpus order keeps the rendered report a pure function of
    // the corpus, which is the property the sequential pass exists to give it.
    let mut diagnostics = DiagnosticsBudget::new();

    let mut programs: Vec<ProgramAudit> = Vec::with_capacity(discovered.len());
    let mut matched = 0usize;
    for source in &discovered {
        let (area, program) = identity_of(source)?;
        if let Some(active) = filter {
            if !active.matches(&area, &program) {
                continue;
            }
            matched += 1;
        }
        programs.push(audit_program(
            caps,
            source,
            &area,
            &program,
            driver,
            &mut diagnostics,
        )?);
    }
    // A filter that selected nothing fails the run rather than shrinking it: an audit of zero
    // programs reports success for want of anything to judge, which in continuous integration is
    // indistinguishable from an audit that works. The policy lives in `env.rs`; the count is ours.
    if let Some(active) = filter {
        active.require_match(matched)?;
    }

    // Nothing is excluded when no filter is in force: every discovered program was audited.
    let excluded_by_filter = if filter.is_some() {
        discovered.len().saturating_sub(programs.len())
    } else {
        0
    };

    Ok(AuditReport {
        programs,
        reference: reference.summary(),
        reference_path: driver.map(Path::to_path_buf),
        host: host_line(caps),
        budget: budget_for(caps),
        strict: config.unavailable_fails_run(),
        unavailable_reason,
        filter: filter.map(|active| active.to_string()),
        excluded_by_filter,
        discovered: discovered.len(),
        quick: config.quick_mode(),
        diagnostics,
    })
}

/// Confirm this module's own constants still agree with the harness root's gate tables.
///
/// The tables are defined once, at the harness root, and have two consumers: `manifest.rs`
/// validates a record's declared gate against them and this module runs whichever gate applies.
/// This function is what keeps a third value — the one flag spelling this module names for itself —
/// from drifting away from them during maintenance. A rename at the root would otherwise leave the
/// extension-area check silently testing a flag nobody passes, which is the quietest possible way
/// to lose a gate.
///
/// # Errors
///
/// Returns an explanatory failure when the tables and this module disagree. That is a defect in the
/// harness rather than in any program, so it fails the audit outright instead of being reported as
/// a program's problem.
fn verify_gate_tables() -> HarnessResult<()> {
    let context = "verifying the undefined-behaviour audit gate tables";
    if UB_AUDIT_GATE_DEFAULT.is_empty() || SANITIZER_GATE_FLAGS.is_empty() {
        return Err(HarnessError::new(
            context,
            "a gate table is empty, so the gate it describes would examine nothing while still \
             reporting a pass",
        ));
    }
    if !UB_AUDIT_GATE_DEFAULT.contains(&UB_AUDIT_GATE_MANDATORY) {
        return Err(HarnessError::new(
            context,
            format!(
                "{UB_AUDIT_GATE_MANDATORY} is named as the member no deviation may drop, but it is \
                 not a member of the default gate at all; a gate that merely warned would be a gate \
                 in name only"
            ),
        ));
    }
    if is_ub_audit_gate_removable(UB_AUDIT_GATE_MANDATORY) {
        return Err(HarnessError::new(
            context,
            format!(
                "{UB_AUDIT_GATE_MANDATORY} is listed as removable, which would let a program keep \
                 every diagnostic while downgrading all of them to advice"
            ),
        ));
    }
    for removable in UB_AUDIT_GATE_REMOVABLE {
        if !is_ub_audit_gate_member(removable) {
            return Err(HarnessError::new(
                context,
                format!(
                    "{removable:?} is listed as a removable gate member but is not a member of the \
                     default gate, so a record could name it and drop a flag the gate never passed"
                ),
            ));
        }
    }
    if !is_ub_audit_gate_member(EXTENSION_ONLY_REMOVABLE)
        || !is_ub_audit_gate_removable(EXTENSION_ONLY_REMOVABLE)
    {
        return Err(HarnessError::new(
            context,
            format!(
                "this module treats {EXTENSION_ONLY_REMOVABLE:?} as the gate member droppable only \
                 in the {EXTENSION_AREA} area, but the harness root no longer lists it as a \
                 removable member of the default gate ({}); the two must agree, because the area \
                 restriction is the only thing that keeps that removal from spreading",
                comma_separated(UB_AUDIT_GATE_DEFAULT)
            ),
        ));
    }
    Ok(())
}

/// Why neither gate can be applied, when the reference compiler that drives both is absent.
///
/// The tool's own diagnosis is carried through rather than replaced, because it names the override
/// variable and states exactly what was tried — the difference between a message a maintainer can
/// act on and one that sends them to install a package that is already installed.
fn unavailability_of(reference: &ToolRecord) -> Option<String> {
    if reference.is_available() {
        return None;
    }
    Some(format!(
        "no native reference compiler was found, so neither gate could be applied to any program: \
         {}. Both gates are reference-compiler-only by design, so this leaves the corpus unaudited \
         rather than partially audited. Set {VAR_REF_CC} to a working C compiler. Every affected \
         gate is reported UNAVAILABLE below — never as a pass — and under {VAR_STRICT}, the setting \
         intended for continuous integration, that becomes a failure of the run.",
        reference.diagnosis()
    ))
}

/// Host architecture, operating system and kernel, as one line for the report header.
fn host_line(caps: &Capabilities) -> String {
    sanitize_line(&format!(
        "{} {}, {}",
        caps.host_arch(),
        caps.host_os(),
        caps.kernel()
    ))
}

/// The `<area>`, `<program>` identity of a discovered corpus program.
///
/// Derived from the path rather than from the program's record, because a record that cannot be
/// read must still be reported under the identity of the program it governs — and because the
/// workspace for a gate has to be named before anything about the record is known.
///
/// # Errors
///
/// Fails when the path has no parent directory name or no file stem, or when either is not text. A
/// discovered corpus path always has both; a path that does not is a defect worth reporting rather
/// than a program to audit under an invented name.
fn identity_of(source: &Path) -> HarnessResult<(String, String)> {
    let context = format!("identifying the corpus program {}", shown_path(source));
    let area = source
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            HarnessError::new(
                context.clone(),
                "the program has no readable parent directory name, and a feature area is the \
                 directory a program lives in",
            )
        })?;
    let program = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            HarnessError::new(
                context,
                "the program has no readable file stem, and a program is identified by its stem",
            )
        })?;
    Ok((String::from(area), String::from(program)))
}

/// The warning gate one program asks for, together with everything wrong with the asking.
///
/// Separating the decision from the invocation is what lets a defective record fail the gate
/// *before* any process is spawned: a gate that cannot be trusted to be the gate it should have
/// been is not worth running, and running it anyway would report a pass earned under unknown flags.
struct GateDecision {
    /// The flags to pass, in gate order. Empty when the record could not be read.
    flags: Vec<String>,
    /// The default-gate members this program drops, in gate order.
    dropped: Vec<String>,
    /// Whether the gate actually differs from the default gate.
    deviated: bool,
    /// The reason recorded in the program's own record, when it records one.
    reason: Option<String>,
    /// Whether the record carries a non-empty `ub_notes`, the written
    /// undefined-behaviour-freedom argument requirement 1 obliges every program to record.
    ///
    /// Decided here because this is where the record is read, and carried on to the audit rather
    /// than only turned into a defect: an absent argument fails the warning gate *and* is counted
    /// in the report, so a reader is told which programs lack one instead of inferring it from the
    /// absence of a failure.
    ub_notes_recorded: bool,
    /// Everything wrong with the record, each phrased as a sentence an author can act on. A
    /// non-empty list fails the warning gate.
    defects: Vec<String>,
}

/// Decide the warning gate for one program, and validate the record's claim to deviate.
///
/// The record parser already refuses a gate that is not a sanctioned reduction, so in a healthy
/// checkout every check below passes. They are performed again here, independently, for two
/// reasons: this module is the thing that actually *runs* the gate, so it must not depend on
/// another module's diligence for the guarantee it publishes; and the checks are what make the
/// requirement enforced in code rather than by convention.
///
/// Five conditions are required, and each closes a distinct way a relaxation could spread:
///
/// - every named flag is a member of the default gate, so a deviation cannot introduce an option
///   the gate never contained — no suppression, no search-path change, no output redirection;
/// - the gate retains the mandatory member, because a gate that warned without failing is a gate in
///   name only;
/// - every dropped member is one the tables sanction dropping;
/// - the extension-area-only member is dropped only inside that area, since dropping it elsewhere
///   discards a diagnostic that would be a genuine defect;
/// - a gate that deviates carries a recorded reason that names every flag it drops, and every
///   program carries a non-empty written undefined-behaviour-freedom argument.
///
/// The reason is required to *name the dropped flag*, rather than merely to exist. A record that
/// carries prose in the right field satisfies a presence check while explaining nothing about the
/// relaxation, and the requirement that a deviation carry a reason is worth only as much as the
/// link between the reason and the removal it is supposed to justify. Both notes fields are
/// searched, because a program may reasonably record the deviation in either — the field whose
/// purpose is narrowing, or the argument a reviewer reads first — and only the paragraphs that name
/// a dropped flag are reported, so the report shows the explanation rather than the whole record.
fn decide_warning_gate(manifest: &Manifest) -> GateDecision {
    let mut defects: Vec<String> = Vec::new();

    let flags: Vec<String> = match manifest.ub_audit_flags() {
        Some(declared) => declared.to_vec(),
        None => UB_AUDIT_GATE_DEFAULT
            .iter()
            .map(|flag| String::from(*flag))
            .collect(),
    };

    for flag in &flags {
        if !is_ub_audit_gate_member(flag) {
            defects.push(format!(
                "the declared gate names {flag:?}, which is not a member of the default gate ({}); \
                 a deviation is a removal and never an addition, so no option outside the gate may \
                 be introduced through it",
                comma_separated(UB_AUDIT_GATE_DEFAULT)
            ));
        }
    }
    if !flags.iter().any(|flag| flag == UB_AUDIT_GATE_MANDATORY) {
        defects.push(format!(
            "the declared gate drops {UB_AUDIT_GATE_MANDATORY}, which it may never drop: without it \
             every remaining diagnostic becomes advice and the gate stops being a gate"
        ));
    }

    let dropped: Vec<String> = UB_AUDIT_GATE_DEFAULT
        .iter()
        .filter(|member| !flags.iter().any(|flag| flag == *member))
        .map(|member| String::from(*member))
        .collect();
    for member in &dropped {
        if !is_ub_audit_gate_removable(member) {
            defects.push(format!(
                "the declared gate drops {member:?}, which no program may drop; only {} may be \
                 dropped, and only for the reason each exists to allow",
                comma_separated(UB_AUDIT_GATE_REMOVABLE)
            ));
        }
        if member == EXTENSION_ONLY_REMOVABLE && manifest.area() != EXTENSION_AREA {
            defects.push(format!(
                "the declared gate drops {EXTENSION_ONLY_REMOVABLE:?} outside the {EXTENSION_AREA} \
                 area. That removal is sanctioned only where the subject under test is by definition \
                 non-standard and this flag exists precisely to reject it; anywhere else it discards \
                 a diagnostic that would be a genuine defect in the test program"
            ));
        }
    }

    // The record is the authority on whether it deviates, and the drop list is computed here. A
    // disagreement between them would mean one of the two is reading a different gate, so it is
    // reported rather than resolved by preferring either.
    let deviated = manifest.has_ub_audit_deviation();
    let drops_a_member = !dropped.is_empty();
    if deviated != drops_a_member {
        defects.push(format!(
            "the record reports deviation as {deviated} while the gate it declares drops {} \
             member(s) of the default gate; the two derive from one table and cannot legitimately \
             disagree",
            dropped.len()
        ));
    }

    let narrowing_notes_present = manifest
        .impl_defined_notes()
        .is_some_and(|notes| !notes.trim().is_empty());
    if !dropped.is_empty() && !narrowing_notes_present {
        defects.push(String::from(
            "the warning gate deviates from the default gate but the record carries no non-empty \
             `impl_defined_notes`. A record that narrows anything — a target list, an oracle or, as \
             here, the gate itself — states what it narrowed and why in that field, so a gate \
             declared without it has narrowed the audit while recording nothing about the narrowing",
        ));
    }

    let (reason, unexplained) = explain_deviation(manifest, &dropped);
    if !unexplained.is_empty() {
        defects.push(format!(
            "the warning gate drops {} but the record's `impl_defined_notes` does not name {} \
             anywhere, so the removal has no recorded reason. A DEVIATION WITHOUT A RECORDED REASON \
             IS ITSELF A DEFECT IN THE TEST: the gate's whole value is its strictness, and an \
             unexplained relaxation quietly re-admits the undefined behaviour this suite depends on \
             excluding. `impl_defined_notes` is the one field this reason is read from — `ub_notes` \
             carries the undefined-behaviour-freedom argument and is not searched for it — so name \
             each dropped flag there by its exact spelling and state why the program cannot be \
             compiled with it",
            comma_separated(&dropped.iter().map(String::as_str).collect::<Vec<&str>>()),
            comma_separated(&unexplained.iter().map(String::as_str).collect::<Vec<&str>>())
        ));
    }

    let ub_notes_recorded = !manifest.ub_notes().trim().is_empty();
    if !ub_notes_recorded {
        defects.push(String::from(
            "the record carries no non-empty `ub_notes`, so this program has no written \
             undefined-behaviour-freedom argument. That argument is the human half of the \
             requirement whose machine half is this gate, and it is what a reviewer reads first when \
             a divergence appears",
        ));
    }

    GateDecision {
        flags,
        dropped,
        deviated,
        reason,
        ub_notes_recorded,
        defects,
    }
}

/// Find the recorded reason for a gate deviation, and the dropped flags nothing accounts for.
///
/// Returns the paragraphs that justify the removal, in a deterministic order, together with the
/// list of dropped flags no paragraph names. A non-empty second element is what fails the gate: a
/// record that narrows the audit without saying which flag it removed, or why, has recorded nothing
/// that could be reviewed.
///
/// Exactly one field is searched: `impl_defined_notes`, the **single canonical field** for the
/// reason behind a gate deviation. `ub_notes` is deliberately **not** consulted, and the difference
/// matters in both directions. `impl_defined_notes` is the field a record must already carry
/// whenever it narrows anything, and a gate deviation is a narrowing, so a reason recorded there is
/// recorded where the parser has already insisted something be written. `ub_notes` answers a
/// different question — the written undefined-behaviour-freedom argument, which every program owes
/// whether or not its gate deviates — and a field that answers two questions answers neither
/// reliably. Searching both would also leave a reviewer with two places to look and no guarantee
/// about which holds the reason. The parser enforces the same contract at load time, so by the time
/// a record reaches this function the explanation is already required to be here; the check below
/// remains because a gate this suite runs must never depend on a validation performed elsewhere.
///
/// Only paragraphs that name a dropped flag are kept, which is what makes the returned text an
/// explanation of the removal rather than a reprint of the record: this field legitimately discusses
/// target widths, char signedness and oracle scope as well, and none of that justifies relaxing a
/// diagnostic.
///
/// Matching is on the flag's exact spelling including its leading hyphen, so prose that happens to
/// use the word in another sense does not count as an explanation. The spellings cannot shadow one
/// another either: no member of the gate is a substring of another member.
///
/// Returns `(None, empty)` when nothing was dropped, since a gate that deviates in no way needs no
/// reason at all.
fn explain_deviation(manifest: &Manifest, dropped: &[String]) -> (Option<String>, Vec<String>) {
    if dropped.is_empty() {
        return (None, Vec::new());
    }
    let source = manifest.impl_defined_notes().unwrap_or_default();

    let mut paragraphs: Vec<String> = Vec::new();
    for paragraph in source.split("\n\n") {
        let text = paragraph.trim();
        if text.is_empty() {
            continue;
        }
        let names_a_dropped_flag = dropped.iter().any(|flag| text.contains(flag.as_str()));
        if names_a_dropped_flag && !paragraphs.iter().any(|kept| kept == text) {
            paragraphs.push(String::from(text));
        }
    }

    let unexplained: Vec<String> = dropped
        .iter()
        .filter(|flag| !source.contains(flag.as_str()))
        .cloned()
        .collect();

    let reason = if paragraphs.is_empty() {
        None
    } else {
        Some(paragraphs.join("\n\n"))
    };
    (reason, unexplained)
}

/// The decision for a program whose expectation record could not be read at all.
///
/// The parse failure is carried through verbatim, because the parser's diagnostics name the key and
/// say what the record should have said. Nothing is assumed about the gate the program wanted: a
/// record that cannot be read cannot sanction a deviation, and applying the default gate anyway
/// could produce a diagnostic the program had a recorded right to avoid.
fn undecidable_gate(error: &HarnessError) -> GateDecision {
    GateDecision {
        flags: Vec::new(),
        dropped: Vec::new(),
        deviated: false,
        reason: None,
        // A record that could not be read states no argument either, and reporting the argument as
        // present because the field could not be inspected is the one answer no reader could act on.
        ub_notes_recorded: false,
        defects: vec![format!(
            "the expectation record could not be read, so the gate this program asks for is unknown: \
             {error}"
        )],
    }
}

/// Audit one program under both gates.
///
/// The record is loaded first, and a record that cannot be read is a *result* rather than an error:
/// the audit must report every defective record in one run rather than stop at the first, because
/// an author fixing the corpus needs the whole list.
///
/// The sanitizer gate runs even when the record is defective. Its flags are fixed and do not come
/// from the record, so it can still answer its own question — does this program perform undefined
/// behaviour when it runs — and answering it is more useful than withholding it.
///
/// # Errors
///
/// Only for a workspace that cannot be allocated or written, a path that cannot be represented as
/// text, or a process that cannot be spawned or reaped. Everything a program or its record can do
/// wrong is a status in the returned value.
fn audit_program(
    caps: &Capabilities,
    source: &Path,
    area: &str,
    program: &str,
    driver: Option<&Path>,
    diagnostics: &mut DiagnosticsBudget,
) -> HarnessResult<ProgramAudit> {
    let loaded = manifest::load_for_source(source);
    let record = loaded
        .as_ref()
        .ok()
        .map(|manifest| manifest.path().to_path_buf());
    let decision = match &loaded {
        Ok(manifest) => decide_warning_gate(manifest),
        Err(error) => undecidable_gate(error),
    };
    // A record that could not be read states no expectation, so the sanitizer run is judged against
    // the ordinary successful exit. Every record in the corpus declares this value explicitly.
    let expect_exit = loaded
        .as_ref()
        .map(Manifest::expect_exit)
        .unwrap_or_default();

    let (warning, mut sanitizer) = match driver {
        None => {
            let diagnosis = caps.ref_cc_native().diagnosis();
            (
                GateResult::unavailable(Gate::Warning, &diagnosis),
                GateResult::unavailable(Gate::Sanitizer, &diagnosis),
            )
        }
        Some(reference) => {
            let warning = if decision.defects.is_empty() {
                run_warning_gate(
                    caps,
                    reference,
                    area,
                    program,
                    source,
                    &decision.flags,
                    diagnostics,
                )?
            } else {
                GateResult::record_defect(Gate::Warning, &decision.defects)
            };
            let sanitizer = run_sanitizer_gate(
                caps,
                reference,
                area,
                program,
                source,
                expect_exit,
                diagnostics,
            )?;
            (warning, sanitizer)
        }
    };

    // Both gates have concluded, so the `<area>/<program>` scaffolding their leaf workspaces sat
    // inside belongs to the program rather than to either gate and is tidied here. Attempted only
    // when neither gate is holding a directory: `GateResult::workspace` is `Some` exactly when
    // something was kept — a failed gate's evidence, a run asked to retain every workspace, or a
    // leaf whose own removal did not work — and in every one of those cases the scaffolding must
    // stay so that what was kept remains reachable. Any note is folded into this program's audit
    // detail rather than raised, under the cleanup invariant `sandbox::prune_empty_audit_grouping`
    // documents.
    if warning.workspace().is_none() && sanitizer.workspace().is_none() {
        if let Some(note) = sandbox::prune_empty_audit_grouping(area, program) {
            sanitizer.note_cleanup(&note);
        }
    }

    Ok(ProgramAudit {
        area: String::from(area),
        program: String::from(program),
        source: source.to_path_buf(),
        record,
        gate_flags: decision.flags,
        deviated: decision.deviated,
        dropped: decision.dropped,
        reason: decision.reason,
        ub_notes_recorded: decision.ub_notes_recorded,
        warning,
        sanitizer,
    })
}

/// Gate one: compile the program with the reference compiler under its warning gate.
///
/// Compiling without linking is deliberate. The gate asks whether the program provokes a
/// diagnostic, and linking would add a step that can fail for reasons the gate is not asking about
/// — it also keeps the question of linkage off this path entirely, leaving the sanitizer gate as
/// the only place in the suite that links anything other than statically.
///
/// The **corpus** path is compiled rather than the workspace copy, so every diagnostic names the
/// file an author must edit. The copy is made anyway, so that a retained workspace stands on its
/// own.
///
/// # Errors
///
/// As [`audit_program`]: workspace, path-representation and process failures only. A program that
/// provokes a diagnostic is a failed gate, not an error.
fn run_warning_gate(
    caps: &Capabilities,
    reference: &Path,
    area: &str,
    program: &str,
    source: &Path,
    flags: &[String],
    budget: &mut DiagnosticsBudget,
) -> HarnessResult<GateResult> {
    let gate = Gate::Warning;
    let label = format!("{area}/{program}");
    let context = format!("applying the {gate} gate to {label}");
    let workspace = sandbox::audit_workspace(area, program, gate.label(), caps.config())?;
    workspace.copy_in(source, PROGRAM_SOURCE_NAME)?;

    let object = workspace.path(WARNING_GATE_OBJECT_NAME)?;
    let source_text = path_text(&context, "the program source", source)?;
    let object_text = path_text(&context, "the gate output", &object)?;

    let mut argv = vec![path_text(&context, "the reference compiler", reference)?];
    argv.extend(flags.iter().cloned());
    argv.push(String::from(COMPILE_ONLY_FLAG));
    argv.push(source_text.clone());
    argv.push(String::from(OUTPUT_FLAG));
    argv.push(object_text.clone());

    let mut permitted: Vec<&str> = flags.iter().map(String::as_str).collect();
    permitted.extend([
        COMPILE_ONLY_FLAG,
        OUTPUT_FLAG,
        source_text.as_str(),
        object_text.as_str(),
    ]);
    guard_invocation(&context, &argv, reference, &permitted)?;

    let outcome = spawn_guarded(caps, reference, &argv, &workspace)?;
    let commands = vec![outcome.command_line()];
    record_commands(&workspace, gate, &label, &commands)?;
    // Persisted **before** the excerpt is taken, so the recoverability the bounded record promises is
    // already true when it is promised: the entries named below hold the untruncated bytes whatever
    // the ceilings then allow into memory.
    let captures = CaptureNames::reference();
    outcome.persist(&workspace, &captures)?;

    // Judged first, so the capture below is taken only when the report will actually render it. A
    // passing gate's output is never shown, and spending the run's allowance on it could leave a
    // later failing gate without the diagnostic that explains it — see [`Diagnostics::none`].
    let (status, mut detail) = judge_warning_gate(&outcome, &object);
    let diagnostics = if status.passed() {
        Diagnostics::none()
    } else {
        Diagnostics::capture(
            budget,
            &[
                (
                    "reference compiler standard error",
                    outcome.stderr(),
                    captures.stderr(),
                ),
                (
                    "reference compiler standard output",
                    outcome.stdout(),
                    captures.stdout(),
                ),
            ],
        )
    };
    let flags_used = flags.to_vec();
    let workspace_kept = conclude(workspace, status, &mut detail);
    Ok(GateResult::new(
        gate,
        status,
        flags_used,
        commands,
        diagnostics,
        detail,
        workspace_kept,
    ))
}

/// Decide the warning gate's outcome from what the compiler did.
///
/// With every warning promoted to an error, a clean exit is the gate's own statement that it found
/// nothing to report, so the exit status is the primary signal. Output written alongside a clean
/// exit is still recorded and mentioned, because text a compiler chose to write is worth reading
/// even when it did not stop the compilation.
///
/// A crash or a timeout is reported as neither the program's fault nor the compiler-under-test's:
/// the audit says what happened and leaves the attribution to a human, because a driver that dies
/// on a well-formed program is an environment problem as often as a program one.
fn judge_warning_gate(outcome: &RunOutcome, object: &Path) -> (GateStatus, String) {
    match outcome.termination() {
        Termination::Exited(0) => {
            if !object.is_file() {
                return (
                    GateStatus::Failed,
                    format!(
                        "the reference compiler exited cleanly but produced no object at {}; the \
                         gate cannot be said to have examined the program",
                        shown_path(object)
                    ),
                );
            }
            let mut detail = String::from(
                "the program compiled with no diagnostic under the full gate it declares",
            );
            if !outcome.stderr().is_empty() || !outcome.stdout().is_empty() {
                detail.push_str(
                    ". The compiler exited cleanly yet wrote output, which is recorded below: with \
                     every warning promoted to an error a clean exit means it found nothing to \
                     reject, so this is reported rather than treated as a failure",
                );
            }
            (GateStatus::Passed, detail)
        }
        Termination::Exited(code) => (
            GateStatus::Failed,
            format!(
                "the reference compiler rejected the program (exit code {code}). Every warning is \
                 promoted to an error by this gate, so a diagnostic here is a real defect in the \
                 test program: the program must be rewritten, or — if the diagnostic is the very \
                 behaviour under test — a narrow per-program deviation must be recorded with its \
                 reason. The compiler's own words are reproduced below"
            ),
        ),
        Termination::Signalled(signal) => (
            GateStatus::Failed,
            format!(
                "the reference compiler was terminated by signal {signal} rather than exiting, so \
                 the gate reached no conclusion about the program. This is a defect in the test \
                 program or in the environment, and never a verdict about the compiler under test, \
                 which is not invoked by this gate"
            ),
        ),
        Termination::TimedOut => (
            GateStatus::Failed,
            format!(
                "the reference compiler exceeded the per-invocation budget of {} ms and was \
                 terminated, so the gate reached no conclusion about the program",
                outcome.budget().as_millis()
            ),
        ),
    }
}

/// Gate two: build the program with the sanitizers and **run** it.
///
/// Three properties of this invocation are deliberate and must survive maintenance:
///
/// - **Dynamically linked.** The sanitizer runtimes need it, and this is the only place in the
///   suite where an artifact is not statically linked. Nothing on this path names a linkage flag at
///   all.
/// - **Native only.** The runtimes are not available for the cross targets under emulation, and
///   they do not need to be: undefined behaviour is a property of the program rather than of the
///   target it is later built for. The artifact is therefore launched directly on the host, with no
///   emulator and no target dispatch anywhere in this function.
/// - **Actually executed.** A build alone proves nothing: the undefined-behaviour sanitizer reports
///   at run time, so a gate that only compiled would pass every program in the corpus regardless of
///   what it does.
///
/// Nothing here renders a verdict about the compiler under test, which is not invoked. A diagnostic
/// means the test program is defective and must be rewritten in the corpus by a human.
///
/// # Errors
///
/// As [`audit_program`]: workspace, path-representation and process failures only.
fn run_sanitizer_gate(
    caps: &Capabilities,
    reference: &Path,
    area: &str,
    program: &str,
    source: &Path,
    expect_exit: i32,
    budget: &mut DiagnosticsBudget,
) -> HarnessResult<GateResult> {
    let gate = Gate::Sanitizer;
    let label = format!("{area}/{program}");
    let context = format!("applying the {gate} gate to {label}");
    let workspace = sandbox::audit_workspace(area, program, gate.label(), caps.config())?;
    workspace.copy_in(source, PROGRAM_SOURCE_NAME)?;

    let artifact = workspace.path(SANITIZER_ARTIFACT_NAME)?;
    let source_text = path_text(&context, "the program source", source)?;
    let artifact_text = path_text(&context, "the instrumented artifact", &artifact)?;

    let mut argv = vec![path_text(&context, "the reference compiler", reference)?];
    argv.extend(SANITIZER_GATE_FLAGS.iter().map(|flag| String::from(*flag)));
    argv.push(source_text.clone());
    argv.push(String::from(OUTPUT_FLAG));
    argv.push(artifact_text.clone());

    // The permitted set is exactly the sanitizer flags, the output flag and the two paths. Anything
    // else in the vector is refused, which is how the linkage and target-selection invariants of
    // this path are enforced without naming the flags they exclude: an argument that is not one of
    // these cannot be passed at all.
    let mut permitted: Vec<&str> = SANITIZER_GATE_FLAGS.to_vec();
    permitted.extend([OUTPUT_FLAG, source_text.as_str(), artifact_text.as_str()]);
    guard_invocation(&context, &argv, reference, &permitted)?;

    let build = spawn_guarded(caps, reference, &argv, &workspace)?;
    let mut commands = vec![build.command_line()];
    // As on the warning-gate path: the streams reach disk before any ceiling is consulted, so a
    // bounded excerpt can name the file holding the bytes it left out.
    let build_captures = CaptureNames::reference();
    build.persist(&workspace, &build_captures)?;

    if !build.termination().succeeded() || !artifact.is_file() {
        record_commands(&workspace, gate, &label, &commands)?;
        let diagnostics = Diagnostics::capture(
            budget,
            &[
                (
                    "instrumented build standard error",
                    build.stderr(),
                    build_captures.stderr(),
                ),
                (
                    "instrumented build standard output",
                    build.stdout(),
                    build_captures.stdout(),
                ),
            ],
        );
        let mut detail = describe_failed_build(&build, &artifact);
        let status = GateStatus::Failed;
        let kept = conclude(workspace, status, &mut detail);
        return Ok(GateResult::new(
            gate,
            status,
            sanitizer_flags(),
            commands,
            diagnostics,
            detail,
            kept,
        ));
    }

    // The artifact is about to be executed, so the two properties that make that defensible are
    // established rather than assumed: it is a regular file and not a symbolic link, and it
    // resolves to a path strictly inside this gate's own workspace. Both hold by construction — the
    // path came from the workspace and the build has just written it — and both are checked anyway,
    // because this is the line the execute-path discipline rests on and a check that is
    // unconditional cannot be forgotten.
    require_regular_file(&context, &artifact)?;
    ensure_within(&context, workspace.root(), &artifact)?;

    let run = spawn_guarded_artifact(caps, &artifact, &workspace)?;
    commands.push(run.command_line());
    record_commands(&workspace, gate, &label, &commands)?;
    let run_captures = CaptureNames::new(
        SANITIZER_RUN_STDOUT_NAME,
        SANITIZER_RUN_STDERR_NAME,
        SANITIZER_RUN_EXIT_NAME,
    );
    run.persist(&workspace, &run_captures)?;

    // As on the warning-gate path: judged first, and retained only when a reader will see it. This
    // is where the saving is real — a passing sanitizer gate captures the program's ordinary standard
    // output, which every corpus program produces and no section of this report renders.
    let (status, mut detail) = judge_sanitizer_run(&run, expect_exit);
    let diagnostics = if status.passed() {
        Diagnostics::none()
    } else {
        Diagnostics::capture(
            budget,
            &[
                (
                    "instrumented build standard error",
                    build.stderr(),
                    build_captures.stderr(),
                ),
                (
                    "instrumented run standard error",
                    run.stderr(),
                    run_captures.stderr(),
                ),
                (
                    "instrumented run standard output",
                    run.stdout(),
                    run_captures.stdout(),
                ),
            ],
        )
    };
    let kept = conclude(workspace, status, &mut detail);
    Ok(GateResult::new(
        gate,
        status,
        sanitizer_flags(),
        commands,
        diagnostics,
        detail,
        kept,
    ))
}

/// The sanitizer gate flags as owned strings, for a result record.
fn sanitizer_flags() -> Vec<String> {
    SANITIZER_GATE_FLAGS
        .iter()
        .map(|flag| String::from(*flag))
        .collect()
}

/// Explain an instrumented build that did not produce a runnable artifact.
///
/// Kept separate from the run's judgement because the two failures mean different things: a build
/// that fails under instrumentation while the warning gate passed usually means the program uses
/// something the sanitizer runtimes cannot instrument, whereas a failing run means the program
/// actually performs undefined behaviour.
fn describe_failed_build(build: &RunOutcome, artifact: &Path) -> String {
    match build.termination() {
        Termination::Exited(0) => format!(
            "the instrumented build exited cleanly but produced no artifact at {}, so nothing could \
             be executed and the gate reached no conclusion",
            shown_path(artifact)
        ),
        Termination::Exited(code) => format!(
            "the instrumented build failed (exit code {code}). The program must be buildable under \
             the sanitizers for its freedom from undefined behaviour to be demonstrable at all; this \
             is a defect in the test program or in the sanitizer runtimes of this environment, and \
             never a verdict about the compiler under test, which this gate does not invoke"
        ),
        Termination::Signalled(signal) => format!(
            "the instrumented build was terminated by signal {signal} rather than exiting, so the \
             gate reached no conclusion about the program"
        ),
        Termination::TimedOut => format!(
            "the instrumented build exceeded the per-invocation budget of {} ms and was terminated, \
             so the gate reached no conclusion about the program",
            build.budget().as_millis()
        ),
    }
}

/// Decide the sanitizer gate's outcome from what the instrumented program did when it ran.
///
/// "Terminates cleanly" is checked in three independent ways, because each catches something the
/// others cannot:
///
/// - the run neither timed out nor died by a signal, which is the shape of an abort the sanitizers
///   raise when recovery is disabled;
/// - the exit status is the one the program's own record declares. The corpus declares zero
///   throughout, so this is the ordinary successful exit; reading it from the record rather than
///   assuming zero means a program that legitimately declared another status is judged against what
///   it promised instead of failing for keeping its promise;
/// - neither captured stream carries a sanitizer diagnostic. This is the check that survives a
///   runtime which reported an error and still exited cleanly.
///
/// Output on standard error without any diagnostic marker is reported rather than failed. A corpus
/// program writes only to standard output, so such text is worth a reader's attention, but failing
/// on it would turn an unrelated runtime message from a particular machine into a defect in the
/// corpus.
fn judge_sanitizer_run(run: &RunOutcome, expect_exit: i32) -> (GateStatus, String) {
    let markers = sanitizer_markers(run);
    if !markers.is_empty() {
        return (
            GateStatus::Failed,
            format!(
                "the instrumented run reported a sanitizer diagnostic ({}), so this program performs \
                 undefined behaviour. THE TEST PROGRAM IS DEFECTIVE and must be rewritten in the \
                 corpus: a program containing undefined behaviour makes every divergence it produces \
                 unreadable, because two compilers disagreeing about undefined behaviour proves \
                 nothing about either. This is never a finding against the compiler under test, which \
                 this gate does not invoke. The runtime's own words are reproduced below",
                comma_separated(&markers)
            ),
        );
    }
    match run.termination() {
        Termination::Exited(code) if code == expect_exit => {
            let mut detail = format!(
                "the instrumented program ran to completion with the declared exit status {code} and \
                 reported no sanitizer diagnostic"
            );
            if !run.stderr().is_empty() {
                detail.push_str(
                    ". It wrote to standard error without any sanitizer diagnostic in the text, which \
                     is recorded below: a corpus program writes only to standard output, so the text \
                     is worth reading, but on its own it is not a defect",
                );
            }
            (GateStatus::Passed, detail)
        }
        Termination::Exited(code) => (
            GateStatus::Failed,
            format!(
                "the instrumented program exited with status {code} while its record declares \
                 {expect_exit}. With recovery disabled the sanitizers terminate the program when they \
                 find undefined behaviour, so an unexpected status here is a defect in the test \
                 program even when the runtime printed nothing recognisable; it is never a verdict \
                 about the compiler under test"
            ),
        ),
        Termination::Signalled(signal) => (
            GateStatus::Failed,
            format!(
                "the instrumented program was terminated by signal {signal}. That is the shape of the \
                 abort the sanitizers raise when recovery is disabled, so the program is defective \
                 and must be rewritten in the corpus"
            ),
        ),
        Termination::TimedOut => (
            GateStatus::Failed,
            format!(
                "the instrumented program exceeded the per-invocation budget of {} ms and was \
                 terminated. A corpus program is bounded work with no input, so a program that does \
                 not finish is defective; the bound is what stops one from stalling the suite",
                run.budget().as_millis()
            ),
        ),
    }
}

/// The sanitizer diagnostic markers present in a run's captured streams, in table order.
///
/// Both streams are examined. A runtime normally writes to standard error, and standard output is
/// examined too because a configured log destination can redirect it there — and because the cost
/// of looking is one substring search over text the audit has already captured.
fn sanitizer_markers(run: &RunOutcome) -> Vec<&'static str> {
    let streams = [
        String::from_utf8_lossy(run.stderr()).into_owned(),
        String::from_utf8_lossy(run.stdout()).into_owned(),
    ];
    SANITIZER_DIAGNOSTIC_MARKERS
        .iter()
        .copied()
        .filter(|marker| streams.iter().any(|stream| stream.contains(marker)))
        .collect()
}

/// Refuse any argument vector this module must not spawn.
///
/// This is the mechanical form of the rule that both gates are reference-compiler-only. Two
/// conditions are required, and each closes a different way a gate flag could reach the wrong
/// program:
///
/// - **The invocation target is the reference compiler discovery vetted.** The check is positive —
///   the first element must equal that path exactly — rather than a list of names to refuse. A
///   positive check rejects *every* other program on the machine, including the compiler under
///   test, and needs no list to be kept up to date. Oracle independence itself is established
///   during discovery, which refuses to proceed when the reference compiler and the compiler under
///   test are the same implementation, so passing this check means the flags are reaching a
///   genuinely different compiler.
/// - **Every argument is one this gate declared.** The permitted set is the gate's flags plus the
///   handful of mechanical arguments that gate needs, so anything else — a linkage flag, an
///   optimization level, a target selection, a search path — cannot be passed at all. Expressing
///   the invariant as an allowlist rather than a denylist is what makes it complete: a flag nobody
///   thought to forbid is still refused.
///
/// The bcc-only target selectors are also refused by name. That is redundant while the allowlist
/// holds, and it is kept because it states the intent for a reader and would survive a maintenance
/// mistake in the allowlist: a target selection is the one flag whose appearance here would mean
/// this module had started trying to drive the compiler under test.
///
/// # Errors
///
/// Any violation is a hard failure rather than a gate result. A vector that should never have been
/// assembled is a defect in this module, and continuing would either spawn the wrong program or
/// certify a gate that was not the gate it claimed to be.
fn guard_invocation(
    context: &str,
    argv: &[String],
    reference: &Path,
    permitted: &[&str],
) -> HarnessResult<()> {
    // Rendered once, and quoted the way a shell would read it, so that every refusal below can show
    // the whole vector rather than only the argument that tripped the check. A guard that says
    // which flag was wrong without showing what was being assembled sends a maintainer looking for
    // it.
    let rendered = sanitize_line(&posix_command_line(argv));

    let Some((invoked, arguments)) = argv.split_first() else {
        return Err(HarnessError::new(
            String::from(context),
            "the assembled argument vector is empty, so there is no program to invoke",
        ));
    };
    let expected = path_text(context, "the reference compiler", reference)?;
    if invoked != &expected {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the invocation names {} rather than the vetted native reference compiler {}. Both \
                 undefined-behaviour gates are reference-compiler-only: every warning flag and every \
                 sanitizer flag is admissible on that side alone, and the compiler under test does \
                 not implement the sanitizers at all. Nothing in this module may spawn any other \
                 program. The refused invocation was: {rendered}",
                sanitize_line(invoked),
                sanitize_line(&expected)
            ),
        ));
    }
    for argument in arguments {
        if is_bcc_target_selector(argument) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the invocation carries the target selector {}, which belongs to the compiler \
                     under test. A target selection appearing in an audit invocation would mean this \
                     module had started trying to drive that compiler, which neither gate may ever \
                     do. The refused invocation was: {rendered}",
                    sanitize_line(argument)
                ),
            ));
        }
        if !permitted.iter().any(|allowed| allowed == argument) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the invocation carries {}, which is not one of the arguments this gate declares \
                     ({}). The gate passes its own flags and nothing else, so that no optimization \
                     level, linkage mode, search path or target selection can enter an audit \
                     invocation by accident. The refused invocation was: {rendered}",
                    sanitize_line(argument),
                    comma_separated(permitted)
                ),
            ));
        }
    }
    Ok(())
}

/// Spawn a guarded reference-compiler invocation and capture everything it did.
///
/// The working directory is the gate's own workspace, per child rather than process-wide: the
/// feature area tests run concurrently in one process, so changing the shared working directory
/// would be a data race rather than a confinement. An incidental file a driver writes *beside its
/// output* therefore lands inside the workspace.
///
/// The environment is **replaced, not inherited**, with the gate's workspace as the child's private
/// `HOME`, `TMPDIR`, `TMP` and `TEMP`. A reference driver reads a great many variables that decide
/// where it looks for headers and libraries and how it behaves, and this gate exists to establish a
/// property of a *program*. A result that depended on the invoking environment would not be that
/// property. A driver that ignores those variables and hard-codes a path under the system temporary
/// directory still writes there — a variable cannot bind a program that never reads it — so the
/// workspace bounds what this module names and what the child is told, not every byte the child
/// writes.
///
/// The bound comes from the shared timed-wait facility, using the discovered `timeout` utility when
/// there is one and a watchdog thread otherwise. Standard input is the null device and both streams
/// are captured, so nothing can block on input and nothing pollutes the runner's own output.
///
/// # Errors
///
/// The child could not be spawned or reaped, or the vector was empty. Both are defects rather than
/// properties of a program.
fn spawn_guarded(
    caps: &Capabilities,
    reference: &Path,
    argv: &[String],
    workspace: &Workspace,
) -> HarnessResult<RunOutcome> {
    let Some((_, arguments)) = argv.split_first() else {
        return Err(HarnessError::new(
            "spawning an audit gate invocation",
            "the assembled argument vector is empty, so there is no program to invoke",
        ));
    };
    let mut command = Command::new(reference);
    command.args(arguments);
    command.current_dir(workspace.root());
    run_command_captured_with(
        command,
        budget_for(caps),
        caps.timeout_tool().path(),
        Some(workspace.root()),
    )
}

/// Run the instrumented artifact directly on the host, and capture everything it did.
///
/// Launched with no arguments and the workspace as both its working directory and its private
/// `HOME` and `TMPDIR`. No emulator and no target dispatch appear here, because this gate is native
/// by design: the sanitizer runtimes are not available for the cross targets under emulation, and
/// undefined behaviour is a property of the program rather than of the target it is later built for.
///
/// # The sanitizer runtimes are configured, and configuring them is what makes the gate sound
///
/// This is the one child in the suite whose behaviour is *decided* by environment variables. Each
/// sanitizer runtime reads its own options variable — `ASAN_OPTIONS`, `UBSAN_OPTIONS`,
/// `LSAN_OPTIONS` — and those options can switch a diagnostic off, let one be reported without
/// terminating, or send the report somewhere nobody reads it. Inheriting them was therefore not a
/// neutral omission: an environment carrying `ASAN_OPTIONS=detect_leaks=0` or
/// `UBSAN_OPTIONS=halt_on_error=0` would have turned real undefined behaviour into a clean run,
/// and a clean run here is what licenses the whole suite to treat a divergence as a compiler
/// defect. The gate would have gone on reporting that it had proved something it had not.
///
/// So the environment is replaced and the options are **forced** to the strictest setting each
/// runtime offers: a diagnostic terminates the process rather than being recovered from, and it is
/// printed rather than suppressed. The forced set lives in one catalogue beside the rest of the
/// child environment, applied to every child in the suite rather than only to this one — a variable
/// that cannot reach *any* child cannot reach the one that matters, and a rule with no exceptions
/// is easier to verify than a rule with one.
///
/// Forcing is not a weakening of the earlier "run the runtimes as they come" posture; it is that
/// posture made true. As they come is precisely what an inherited variable prevented.
///
/// # Errors
///
/// The child could not be spawned or reaped.
fn spawn_guarded_artifact(
    caps: &Capabilities,
    artifact: &Path,
    workspace: &Workspace,
) -> HarnessResult<RunOutcome> {
    let mut command = Command::new(artifact);
    command.current_dir(workspace.root());
    run_command_captured_with(
        command,
        budget_for(caps),
        caps.timeout_tool().path(),
        Some(workspace.root()),
    )
}

/// One path as text, for an argument vector and a report line.
///
/// # Errors
///
/// Fails when the path is not valid text. Every audit path is either a corpus path or one derived
/// from the build directory, and both must be expressible in a command line an author can re-run,
/// so a path that cannot be written down is reported rather than approximated.
fn path_text(context: &str, role: &str, path: &Path) -> HarnessResult<String> {
    path.to_str().map(String::from).ok_or_else(|| {
        HarnessError::new(
            String::from(context),
            format!(
                "{role} at {} is not valid text, so it could not be written into an argument vector \
                 or into a command line a maintainer can re-run",
                shown_path(path)
            ),
        )
    })
}

/// Write the gate's exact command lines into its workspace.
///
/// This is what makes a retained directory reproducible without the harness: the program is beside
/// it, and these are the lines that produced everything else in it. The header states which gate
/// ran and that the compiler under test was not involved, so a reader who opens only this file
/// cannot mistake an audit artifact for a differential one.
///
/// # Errors
///
/// A workspace write that could not be performed.
fn record_commands(
    workspace: &Workspace,
    gate: Gate,
    label: &str,
    commands: &[String],
) -> HarnessResult<()> {
    let mut text = format!(
        "# Exact commands of the {gate} undefined-behaviour audit gate for {}.\n",
        sanitize_line(label)
    );
    text.push_str(
        "# Driven by the reference compiler alone: the compiler under test is not invoked by either\n\
         # gate, and no result here is a verdict about it. A failure means the test program is\n\
         # defective and must be rewritten in the corpus.\n",
    );
    for command in commands {
        text.push_str(&sanitize_line(command));
        text.push('\n');
    }
    workspace.write_text(COMMANDS_NAME, &text)?;
    Ok(())
}

/// Conclude a gate's workspace, and report the directory when it is kept.
///
/// A failed gate retains everything: the program, the artifacts, both captured streams, the
/// recorded termination and the exact commands. That is the evidence an author needs, and it is the
/// reason this module has no destructor that deletes — one would erase the evidence for precisely
/// the gate whose evidence matters most.
///
/// A passed gate removes its directory, unless the run asked for every workspace to be kept, in
/// which case the path is still reported so the report can name it. A removal that did not work
/// becomes a sentence in the detail rather than a failure: cleanup must never turn a satisfied gate
/// into a reported defect.
fn conclude(workspace: Workspace, status: GateStatus, detail: &mut String) -> Option<PathBuf> {
    let root = workspace.root().to_path_buf();
    let keeps_on_success = workspace.keeps_on_success();
    if !status.passed() {
        // The retention is bounded and its accounting is reported: a gate whose evidence had to be
        // pruned to stay inside the run's budget says so in its own detail, because an author sent
        // to a directory that no longer holds what the detail promised has been misled rather than
        // helped.
        let retention = workspace.retain();
        for note in retention.notes() {
            detail.push_str(". Retention note: ");
            detail.push_str(&sanitize_line(note));
        }
        return Some(retention.root().to_path_buf());
    }
    match workspace.discard_advisory() {
        Some(note) => {
            detail.push_str(". Cleanup note: ");
            detail.push_str(&sanitize_line(&note));
            Some(root)
        }
        None => {
            if keeps_on_success {
                Some(root)
            } else {
                None
            }
        }
    }
}
