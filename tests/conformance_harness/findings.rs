//! Laying down the self-contained artifact directory that **is** a finding.
//!
//! A finding is an undocumented divergence: bcc's observable behaviour differed from an
//! independent oracle's, and no marker in the program's own expectation record explains the
//! difference by citing a limitation the repository already documents. This module turns that
//! observation into a deliverable.
//!
//! # Report, do not patch
//!
//! This module's output is the **entire** response to a divergence. **No compiler source change
//! is ever made in response to any finding** — the requirement that asks for findings says so in
//! the same sentence that asks for them, and the constraint that forbids modifying the compiler's
//! source says it independently. The two agree completely, so there is no tension to resolve:
//! write the evidence and stop. Nothing in this file reads, writes or even names a path under the
//! compiler's source tree.
//!
//! A finding therefore **does not fail the run**. It is reported loudly, counted in the summary
//! and accompanied by complete artifacts — see [`Verdict::Finding`], for which
//! [`Verdict::fails_run`] is false. The single exception is an artifact directory that could not
//! be written: a finding without its reproducer is not a deliverable, so [`record`] reports that
//! as [`Verdict::Fail`] with the cause named. It is never downgraded to a silent pass.
//!
//! # Two artifact sets, which must never be confused
//!
//! | Set | Location | Written by |
//! | --- | --- | --- |
//! | Transient, per run, auto-generated | [`findings_root`], i.e. `target/conformance-findings/` | **this module, and only here** |
//! | Committed, curated | `tests/conformance/findings/`, indexed by `tests/conformance/FINDINGS.md` | a human, after review |
//!
//! Transient artifacts are written beneath the build directory and are never committed, so an
//! in-progress run cannot pollute the curated set. **This module never writes into the corpus
//! directory** — not the curated findings directory, not the findings register, not the
//! expected-divergence register. It exposes no function that could name one: every path it writes
//! is derived from [`findings_root`] and re-checked against it by [`guarded_path`] immediately
//! before the write, and the corpus is opened for reading only, through
//! [`require_contained_corpus_file`]. Promotion from the transient set to the curated one is a
//! human act of review.
//!
//! There is a third location that is neither of these, and keeping it distinct matters: the
//! per-cell workspaces the sandbox module allocates under the work root. Those are scratch — a
//! workspace is removed when its cell succeeds — whereas a finding directory is the output that
//! survives the run. That is why this module manages its own paths with `std::fs` and its own
//! containment guard rather than borrowing a workspace handle: a workspace is confined to the work
//! root by construction, so it could not name a path under the findings root even if asked to, and
//! a finding must not vanish when the cell that produced it is cleaned up.
//!
//! # The artifact directory
//!
//! One directory per finding, named by its [`FindingId`], holding exactly this:
//!
//! | Artifact | Content |
//! | --- | --- |
//! | [`REPRODUCER_SOURCE_NAME`] | The program, minimized as far as practical. See "Minimization" below. |
//! | [`REPRODUCER_RECORD_NAME`] | Its expectation record, so the reproducer remains runnable by the harness. |
//! | [`MANIFEST_NAME`] | Identifier, area, program, divergence class, every diverging oracle and cell, the minimization status, and a one-paragraph description of the observed difference. |
//! | [`COMMANDS_NAME`] | Exact, copy-pasteable compile and run lines for every cell involved — the artifact that reproduces the finding with **no harness, no Cargo and no Rust toolchain**. |
//! | [`OUTPUTS_DIR_NAME`]`/…` | The captured output of every compiler and every backend involved. |
//! | [`ENVIRONMENT_NAME`] | Fingerprint: bcc, the reference compiler, each cross driver, each emulator, and the kernel. |
//! | [`DIFF_NAME`] | The computed difference, with the first divergent line and byte offset highlighted, bounded. |
//!
//! ## Naming inside `outputs/`
//!
//! Every entry is `<prefix>-<target>-<opt>` plus an extension, where the prefix says **what role
//! the capture played in the comparison**:
//!
//! - `bcc` — the *subject*: the compiler under test at the diverging cell. There is exactly one
//!   subject, because all three oracles judge the same observation.
//! - `a`, `b`, `c` — the *authority* each oracle judged it against, spelled by
//!   [`Oracle::letter`]: `a` the reference compiler at the same target and level, `b` the
//!   baseline backend at the same level, `c` the stdout recorded in the program's own record.
//!
//! The target in an authority's name is the authority's **own** target, so
//! `b-x86_64-O2.stdout` is unambiguously the baseline capture that judged an
//! `bcc-aarch64-O2` subject, and no two captures of one finding can collide.
//!
//! Three extensions, and one rule for each with no case analysis:
//!
//! - `.stdout` and `.stderr` are always the **program's** streams, byte for byte, empty when the
//!   program never ran.
//! - `.exit` is the termination: the raw wait status **and** its decoding, so `exited 0`,
//!   `signalled 11` and `timed out` can never be confused. When there was no run it says so.
//! - `.compile.stderr` and `.compile.exit` are always the **compiler's** diagnostics and status.
//!
//! Standard error is captured here even though **no oracle ever compares it**: diagnostic wording
//! legitimately differs between compilers, so comparing it would bury every real finding under a
//! flood of differences that say nothing about code correctness — yet it is often the fastest
//! route to a diagnosis, so it belongs in the artifact, read by a human rather than by an oracle.
//!
//! # The identifier, and why it is derived rather than counted
//!
//! ```text
//! F-<NNNN>-<head>-<oracle letter>-<target>-<opt>-<class>
//! ```
//!
//! `NNNN` is four digits derived from the finding's full identity — area, program, target,
//! optimization level, oracle and divergence class — by the FNV-1a hash in [`fnv1a64`], reduced
//! modulo [`ID_DIGIT_MODULUS`]. `head` is the area and program names in kebab case with their
//! numeric prefixes dropped, and the four short components after it are never abbreviated, so the
//! directory name states the whole identity in full.
//!
//! It is **derived, never counted**, and that is a correctness requirement rather than a
//! preference. The fourteen feature-area tests run concurrently in one process, so a shared
//! incrementing counter would be a data race *and* would make an identifier depend on thread
//! scheduling — two runs of the same suite would file the same divergence under different names,
//! and neither name would mean anything. Derivation gives the property that actually matters: the
//! **same divergence always gets the same identifier**, in every process, on every run, so
//! re-running overwrites one directory idempotently instead of accumulating duplicates, and two
//! artifact directories can be diffed against each other to see whether a divergence changed.
//!
//! Nothing in the derivation consults a counter, a process identifier, a clock or the environment;
//! it is a pure function of the identity. The standard library's default hasher is deliberately
//! **not** used: its output is documented as unspecified and free to change between releases, so
//! it would silently rename every finding on a toolchain upgrade.
//!
//! ## What "diffable between runs" guarantees, precisely
//!
//! For one unchanged divergence, two runs produce the same identifier, the same file set, and
//! byte-identical [`REPRODUCER_SOURCE_NAME`], [`REPRODUCER_RECORD_NAME`], [`MANIFEST_NAME`],
//! [`COMMANDS_NAME`] and `.stdout`/`.stderr` captures. That is verified rather than hoped for, and
//! it is why [`Capture::describe`] carries no wall-clock duration even though both underlying
//! observations can report one.
//!
//! Three places legitimately differ, and every one of them is a fact about the **run** rather than
//! about the writer: the `.exit` and `.compile.exit` records state how long the process they
//! describe took; [`ENVIRONMENT_NAME`] states the machine's tool versions; and the trailing section
//! of [`DIFF_NAME`], which reproduces the comparator's own account verbatim, inherits the timings
//! that account contains. The computed difference above it — the located first divergent byte and
//! the unified rendering — is byte-identical.
//!
//! None of the three is noise: a timing is part of a capture, the fingerprint is what lets a later
//! reader tell a toolchain change from a compiler change, and the comparator's verbatim account is
//! the richest statement of the difference there is. They are simply not the parts of a two-run
//! diff that carry information about whether the divergence changed.
//!
//! # Minimization
//!
//! Minimization is best-effort **by design**. The external reducer is used only if the
//! environment has one, is never required, and its absence never fails a run and never suppresses
//! an artifact — see [`Minimization`] for the full reasoning and for what is recorded instead.
//!
//! The corpus is read-only to the harness: a reproducer is a *copy* inside the finding directory,
//! and no corpus program is ever modified in place.
//!
//! # Invariants
//!
//! - Only `std` is used; the project permits no third-party crate, so `MANIFEST.txt` is plain text
//!   and `commands.sh` is a plain shell script rather than anything a serializer would produce.
//! - The `unsafe` keyword does not appear. In particular no platform call is made to set an
//!   executable bit on `commands.sh`: it is written as a plain file whose header documents
//!   `sh commands.sh`.
//! - No `#[test]` function and no `main.rs` live in this directory, so Cargo treats it as a plain
//!   module directory rather than a test target, and the suite's test count is unchanged by it.
//! - Every write is beneath [`findings_root`]. Nothing is written outside the build directory and
//!   no socket is ever opened.
//! - Writing one finding touches only that finding's own directory, so concurrent areas never
//!   contend and no lock is needed.
//!
//! Edition 2021, minimum supported Rust 1.70.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use super::compare::{locate_stdout_divergence, unified_diff, Comparison};
use super::compile::CompileOutcome;
use super::env::Capabilities;
use super::execute::RunOutcome;
use super::manifest::Manifest;
use super::{
    findings_root, is_forbidden_for_side, posix_quote, require_contained_corpus_file,
    sanitize_text_for_report, CellKey, CompilerSide, DivergenceClass, HarnessError, HarnessResult,
    OptLevel, Oracle, Outcome, Target, Verdict,
};

/// The minimized reproducer, a copy of the corpus program.
pub const REPRODUCER_SOURCE_NAME: &str = "reproducer.c";

/// The reproducer's expectation record, so the pair remains runnable by the harness and
/// reproducible by hand.
pub const REPRODUCER_RECORD_NAME: &str = "reproducer.expected";

/// The plain-text description of the finding, read in a terminal.
pub const MANIFEST_NAME: &str = "MANIFEST.txt";

/// The exact reproduction commands, runnable with `sh commands.sh`.
pub const COMMANDS_NAME: &str = "commands.sh";

/// Directory holding one capture per compiler and per backend involved.
pub const OUTPUTS_DIR_NAME: &str = "outputs";

/// The environment fingerprint, which is what lets a divergence be attributed to toolchain drift
/// rather than to the compiler.
pub const ENVIRONMENT_NAME: &str = "environment.txt";

/// The computed difference between the subject and the authority.
pub const DIFF_NAME: &str = "diff.txt";

/// Every entry a complete finding directory must contain, in the order this module writes them.
///
/// Published so that a driver, a report or a check can assert completeness without restating the
/// list — a finding missing one of these is not a deliverable, and the set must be assertable in
/// one place.
pub const REQUIRED_ARTIFACTS: &[&str] = &[
    REPRODUCER_SOURCE_NAME,
    REPRODUCER_RECORD_NAME,
    MANIFEST_NAME,
    COMMANDS_NAME,
    ENVIRONMENT_NAME,
    DIFF_NAME,
    OUTPUTS_DIR_NAME,
];

/// Prefix naming the subject of a comparison, as distinct from the [`Oracle::letter`] prefixes
/// that name the authorities.
///
/// Deliberately a word rather than a letter: the subject is the compiler under test, not an
/// oracle, and a reader scanning `outputs/` should be able to tell the two apart at a glance.
pub const SUBJECT_PREFIX: &str = "bcc";

/// Upper bound on the bytes [`DIFF_NAME`] may hold.
///
/// The line diff is already bounded by its own row limit, so this is the second belt: a
/// pathological single line cannot turn a deliverable into a file nobody can open. Truncation
/// always announces itself, so a bounded diff can never be mistaken for a complete one.
pub const MAX_DIFF_BYTES: usize = 256 * 1024;

/// Modulus applied to the identity hash to produce the four-digit component of an identifier.
///
/// Four digits is what the artifact naming scheme calls for. The digits are a *rendering* of the
/// identity, not a count, and the readable components after them carry the identity in full, so a
/// coincidence in these four digits cannot make two findings share a directory.
pub const ID_DIGIT_MODULUS: u64 = 10_000;

/// Longest readable head an identifier may carry before it is abbreviated at a word boundary.
///
/// Only the descriptive head — the area and program names — is ever abbreviated. The oracle
/// letter, target, optimization level and divergence class always follow it in full, so
/// abbreviation can never make two identities that differ in one of those four render alike.
const MAX_ID_HEAD_CHARS: usize = 48;

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Byte separating the components of the identity hashed into an identifier.
///
/// A zero byte, which cannot occur in any component, so the concatenation is unambiguous and two
/// different component lists cannot hash the same input.
const IDENTITY_SEPARATOR: u8 = 0;

/// Hash a component list with FNV-1a, 64-bit.
///
/// Chosen because it is four lines of arithmetic with a fixed specification, so it produces the
/// same value in every process, on every machine and under every toolchain version. That is the
/// whole requirement here: the digits of an identifier must be stable forever, because they name a
/// directory that is compared between runs and cited in a register. The standard library's default
/// hasher is explicitly documented as unstable across releases and would rename every finding on a
/// toolchain upgrade.
///
/// Not a cryptographic hash, and it does not need to be: the identity it summarizes is written out
/// in full in the readable part of the identifier, so the digits are a label rather than a
/// guarantee.
fn fnv1a64(components: &[&str]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            hash ^= u64::from(IDENTITY_SEPARATOR);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for byte in component.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

/// Render text as a kebab-case token: lower-case ASCII alphanumerics, single hyphens between
/// runs of anything else, and no leading or trailing hyphen.
///
/// Applied to the area and program names, which are already lower-case ASCII with underscores by
/// corpus convention, so in practice this only exchanges underscores for hyphens. It is written to
/// cope with anything because an identifier names a directory, and a name that reached the
/// filesystem carrying a path separator or a control character would be a defect rather than an
/// untidiness.
fn kebab(raw: &str) -> String {
    let mut rendered = String::with_capacity(raw.len());
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() {
            rendered.push(character.to_ascii_lowercase());
        } else if !rendered.ends_with('-') {
            rendered.push('-');
        }
    }
    String::from(rendered.trim_matches('-'))
}

/// Drop the numeric ordering prefix a corpus name carries, leaving the descriptive part.
///
/// `04_bitfields` becomes `bitfields` and `005_straddling_and_zero_width` becomes
/// `straddling_and_zero_width`. The ordering digits are how the corpus sorts its directories; they
/// say nothing about the finding, and dropping them is what keeps an identifier readable within its
/// length budget.
///
/// Anything that is not `<digits>_<rest>` is returned unchanged, so a name outside the convention
/// is preserved rather than mangled.
fn strip_numeric_prefix(stem: &str) -> &str {
    match stem.split_once('_') {
        Some((prefix, rest))
            if !prefix.is_empty()
                && !rest.is_empty()
                && prefix.chars().all(|digit| digit.is_ascii_digit()) =>
        {
            rest
        }
        _ => stem,
    }
}

/// Abbreviate `head` to at most [`MAX_ID_HEAD_CHARS`] characters, cutting at a hyphen where one is
/// available so the result still reads as whole words.
///
/// Deterministic: the same input always abbreviates to the same output, which it must, because the
/// result is part of a directory name that is compared between runs.
///
/// Counted and cut by characters rather than by bytes, and never sliced at an arbitrary index, so
/// the function cannot panic on any input. That matters here beyond tidiness: an identifier is
/// built on the failure path of a cell, where a panic would replace a diagnosable divergence with a
/// harness crash.
fn abbreviate_head(head: &str) -> String {
    let mut budgeted: String = head.chars().take(MAX_ID_HEAD_CHARS).collect();
    if budgeted.len() == head.len() {
        return budgeted;
    }
    // `rfind` returns a character boundary by construction, so the truncation is always valid.
    if let Some(boundary) = budgeted.rfind('-') {
        if boundary > 0 {
            budgeted.truncate(boundary);
        }
    }
    String::from(budgeted.trim_matches('-'))
}

/// What part one capture played in the comparison that produced a finding.
///
/// A finding has exactly one **subject** — the compiler under test at the diverging cell — and one
/// **authority** per oracle that judged it. Modelling the role rather than the compiler is what
/// lets `outputs/` be named after the comparison a reader is trying to understand: the same
/// observation is the subject of all three oracles, so naming it after a compiler would leave a
/// reader guessing which of the three differences a file belonged to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CaptureRole {
    /// The compiler under test at the diverging cell: the subject of every comparison.
    UnderTest,
    /// Oracle (a)'s authority: the reference compiler at the same target and optimization level.
    ReferenceCompiler,
    /// Oracle (b)'s authority: the compiler under test on [`Target::BASELINE`] at the same
    /// optimization level.
    Baseline,
    /// Oracle (c)'s authority: the stdout recorded in the program's own expectation record. It
    /// never ran and has no compiler, which is why it is a role rather than a compiler.
    GoldenRecord,
}

impl CaptureRole {
    /// Every role, subject first, then the authorities in oracle order.
    pub const ALL: [CaptureRole; 4] = [
        CaptureRole::UnderTest,
        CaptureRole::ReferenceCompiler,
        CaptureRole::Baseline,
        CaptureRole::GoldenRecord,
    ];

    /// The oracle whose authority this role is, and `None` for the subject.
    pub fn oracle(self) -> Option<Oracle> {
        match self {
            CaptureRole::UnderTest => None,
            CaptureRole::ReferenceCompiler => Some(Oracle::ReferenceCompiler),
            CaptureRole::Baseline => Some(Oracle::CrossBackend),
            CaptureRole::GoldenRecord => Some(Oracle::GoldenRecord),
        }
    }

    /// The filename prefix for this role's captures.
    ///
    /// Taken from [`Oracle::letter`] for an authority rather than spelled again here, so the
    /// artifact names and the oracle vocabulary cannot drift apart, and [`SUBJECT_PREFIX`] for the
    /// subject.
    pub fn prefix(self) -> String {
        match self.oracle() {
            Some(oracle) => String::from(oracle.letter()),
            None => String::from(SUBJECT_PREFIX),
        }
    }

    /// The phrase used in a manifest, a comment and a diagnostic.
    pub fn label(self) -> &'static str {
        match self {
            CaptureRole::UnderTest => "compiler under test (subject)",
            CaptureRole::ReferenceCompiler => "reference compiler (oracle_a authority)",
            CaptureRole::Baseline => "baseline backend (oracle_b authority)",
            CaptureRole::GoldenRecord => "recorded expectation (oracle_c authority)",
        }
    }

    /// Which side of the shared-flag discipline this role's compiler invocation belongs to, and
    /// `None` for the recorded expectation, which has no invocation at all.
    ///
    /// This is what decides how strictly a reproduction command is checked: the target-selection
    /// flag is legitimate on the compiler under test and forbidden on the reference compiler, and
    /// [`is_forbidden_for_side`] is the single authority on that split.
    pub fn compiler_side(self) -> Option<CompilerSide> {
        match self {
            CaptureRole::UnderTest | CaptureRole::Baseline => Some(CompilerSide::UnderTest),
            CaptureRole::ReferenceCompiler => Some(CompilerSide::Reference),
            CaptureRole::GoldenRecord => None,
        }
    }
}

impl fmt::Display for CaptureRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// The stdout and exit status a program's own expectation record prescribes.
///
/// Held separately from an observation because it is a *contract*, not something that happened: it
/// was read from a committed file rather than produced by a process, so it has no diagnostics, no
/// duration and no wait status, and pretending otherwise in an artifact would be a small lie in a
/// document whose whole value is that it can be trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedExpectation {
    stdout: Vec<u8>,
    expect_exit: i32,
}

/// One compiler-and-backend observation a finding must preserve.
///
/// # Why the fields are private
///
/// This is evidence. Every accessor hands out a borrow of something that was **observed** — the
/// bytes a program wrote, the status the operating system reported, the diagnostics a compiler
/// printed — and the entire worth of a differential oracle is that nothing between the program and
/// the artifact may adjust what the program did. A writable field would let a caller edit the
/// evidence after the comparison had already drawn its conclusion from it, leaving a finding whose
/// captured outputs no longer support the difference it describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    role: CaptureRole,
    target: Target,
    opt: OptLevel,
    /// The build that produced the artifact, absent only for [`CaptureRole::GoldenRecord`].
    compile: Option<CompileOutcome>,
    /// The execution of that artifact, absent when the build failed and for
    /// [`CaptureRole::GoldenRecord`].
    run: Option<RunOutcome>,
    /// The recorded contract, present only for [`CaptureRole::GoldenRecord`].
    recorded: Option<RecordedExpectation>,
}

impl Capture {
    /// Record what was observed for one role at one target and optimization level.
    ///
    /// Both observations are optional because a finding is often about their absence: a program one
    /// compiler rejects has a build and no run, and that asymmetry *is* the divergence. Passing
    /// neither is a caller defect rather than a shape the world can produce, so it is refused.
    ///
    /// # Errors
    ///
    /// Refuses [`CaptureRole::GoldenRecord`], which has no compiler and no run and is built by
    /// [`Capture::golden`], and refuses a capture carrying no observation at all.
    pub fn observed(
        role: CaptureRole,
        target: Target,
        opt: OptLevel,
        compile: Option<CompileOutcome>,
        run: Option<RunOutcome>,
    ) -> HarnessResult<Capture> {
        let context = format!(
            "recording the {role} capture for the {} cell at {}",
            target.triple(),
            opt.flag()
        );
        if role == CaptureRole::GoldenRecord {
            return Err(HarnessError::new(
                context,
                String::from(
                    "the recorded expectation is not an observation: it has no compiler \
                     invocation, no execution and no diagnostics, because it was read from a \
                     committed record rather than produced by a process. Build it with \
                     Capture::golden, which stores the recorded stdout and expected exit status \
                     and nothing it did not have",
                ),
            ));
        }
        if compile.is_none() && run.is_none() {
            return Err(HarnessError::new(
                context,
                String::from(
                    "neither a build nor an execution was supplied, so this capture would carry no \
                     evidence at all; a finding's artifacts are the evidence, and one that records \
                     nothing cannot support the difference the finding describes",
                ),
            ));
        }
        Ok(Capture {
            role,
            target,
            opt,
            compile,
            run,
            recorded: None,
        })
    }

    /// Record the golden-record authority for one cell, taken from the program's own expectation
    /// record.
    ///
    /// Infallible: both values come from a record the parser has already validated, and there is
    /// nothing here that can be inconsistent.
    pub fn golden(target: Target, opt: OptLevel, manifest: &Manifest) -> Capture {
        Capture {
            role: CaptureRole::GoldenRecord,
            target,
            opt,
            compile: None,
            run: None,
            recorded: Some(RecordedExpectation {
                stdout: manifest.expected_stdout_bytes().to_vec(),
                expect_exit: manifest.expect_exit(),
            }),
        }
    }

    /// What part this capture played in the comparison.
    pub fn role(&self) -> CaptureRole {
        self.role
    }

    /// The target this capture is for. For an authority this is the authority's own target, which
    /// is what keeps `outputs/` names unambiguous.
    pub fn target(&self) -> Target {
        self.target
    }

    /// The optimization level this capture is for.
    pub fn opt(&self) -> OptLevel {
        self.opt
    }

    /// The build that produced the artifact, when there was one.
    pub fn compile(&self) -> Option<&CompileOutcome> {
        self.compile.as_ref()
    }

    /// The execution of that artifact, when it ran.
    pub fn run(&self) -> Option<&RunOutcome> {
        self.run.as_ref()
    }

    /// True when a program actually executed, as opposed to a build that failed or a recorded
    /// contract that never ran.
    pub fn ran(&self) -> bool {
        self.run.is_some()
    }

    /// The program's standard output: what the program printed, what it was recorded as printing,
    /// or nothing at all when it never ran.
    ///
    /// Bytes rather than text throughout, because this is the stream the oracles compare byte for
    /// byte and it must be preserved exactly as produced, with no line-ending normalization and no
    /// substitution for a byte that is not valid text.
    pub fn stdout(&self) -> &[u8] {
        if let Some(run) = &self.run {
            return run.stdout();
        }
        match &self.recorded {
            Some(recorded) => &recorded.stdout,
            None => &[],
        }
    }

    /// The program's diagnostics, and an empty slice when it never ran.
    ///
    /// Captured for the artifact and **never compared** by any oracle. The compiler's diagnostics
    /// are a different stream and live in [`Capture::compile`].
    pub fn stderr(&self) -> &[u8] {
        match &self.run {
            Some(run) => run.stderr(),
            None => &[],
        }
    }

    /// The filename stem this capture's entries take inside `outputs/`, for example
    /// `a-aarch64-O2`.
    pub fn file_stem(&self) -> String {
        format!(
            "{}-{}-{}",
            self.role.prefix(),
            self.target.short_name(),
            self.opt.short()
        )
    }

    /// The full text of this capture's `.exit` entry.
    ///
    /// Always states the termination in both forms: the decoded interpretation, so a reader can see
    /// at a glance whether the program exited, died on a signal or outlived its budget, and the raw
    /// wait status, so a signal death can never be confused with a numerically equal ordinary exit.
    /// A capture that never ran says so, and names why, rather than presenting a status it does not
    /// have.
    pub fn exit_report(&self) -> String {
        let mut report = format!("role = {}\n", self.role.label());
        report.push_str(&format!("target = {}\n", self.target.triple()));
        report.push_str(&format!("opt_level = {}\n", self.opt.flag()));

        if let Some(run) = &self.run {
            report.push_str(&format!("termination_summary = {}\n", run.termination()));
            report.push_str(&run.status_record());
            return report;
        }

        if let Some(recorded) = &self.recorded {
            report.push_str(
                "observed = no; this is the expectation recorded in the program's own \
                             .expected file, not an execution\n",
            );
            report.push_str(&format!("expect_exit = {}\n", recorded.expect_exit));
            report.push_str(&format!(
                "recorded_stdout_bytes = {}\n",
                recorded.stdout.len()
            ));
            return report;
        }

        report.push_str("did_not_run = yes\n");
        match &self.compile {
            Some(compile) => {
                report.push_str(&format!(
                    "reason = the build did not produce a usable artifact, so there was nothing to \
                     execute: {}\n",
                    match compile.failure() {
                        Some(failure) => failure.to_string(),
                        None => String::from(
                            "the build reported success but no execution was recorded for it",
                        ),
                    }
                ));
                report.push_str(&format!(
                    "compiler_exit_code = {}\n",
                    match compile.exit_code() {
                        Some(code) => code.to_string(),
                        None =>
                            String::from("(none: terminated by a signal or killed at its budget)"),
                    }
                ));
                report.push_str(&format!("compiler_timed_out = {}\n", compile.timed_out()));
            }
            None => report
                .push_str("reason = no build and no execution were recorded for this capture\n"),
        }
        report
    }

    /// The full text of this capture's `.compile.exit` entry, and `None` when there was no build.
    pub fn compile_report(&self) -> Option<String> {
        let compile = self.compile.as_ref()?;
        let mut report = format!("role = {}\n", self.role.label());
        report.push_str(&format!("compiler = {}\n", compile.compiler().label()));
        report.push_str(&format!("target = {}\n", compile.target().triple()));
        report.push_str(&format!("opt_level = {}\n", compile.opt().flag()));
        report.push_str(&format!("succeeded = {}\n", compile.succeeded()));
        report.push_str(&format!(
            "exit_code = {}\n",
            match compile.exit_code() {
                Some(code) => code.to_string(),
                None => String::from("(none)"),
            }
        ));
        report.push_str(&format!(
            "terminated_by_signal = {}\n",
            compile.terminated_by_signal()
        ));
        report.push_str(&format!("timed_out = {}\n", compile.timed_out()));
        report.push_str(&format!(
            "artifact = {}\n",
            match compile.artifact_size() {
                Some(bytes) => format!("{bytes} bytes at {}", compile.artifact().display()),
                None => format!("absent at {}", compile.artifact().display()),
            }
        ));
        report.push_str(&format!(
            "duration_ms = {}\n",
            compile.duration().as_millis()
        ));
        if let Some(failure) = compile.failure() {
            report.push_str(&format!("failure = {failure}\n"));
            report.push_str(&format!("failure_scope = {}\n", failure.scope()));
        }
        report.push_str(&format!("argv = {}\n", compile.command_line()));
        Some(report)
    }

    /// One line describing this capture for a manifest row.
    ///
    /// Deliberately carries no wall-clock duration, even though both underlying observations can
    /// report one. `MANIFEST.txt` is compared between runs to see whether a divergence changed, and
    /// a millisecond count would make every such comparison show a difference that says nothing
    /// about the divergence. The timings are not lost: each is recorded in the capture's own `.exit`
    /// entry, where a fact about how long a run took belongs.
    pub fn describe(&self) -> String {
        let observation = match (&self.run, &self.recorded, &self.compile) {
            (Some(run), _, _) => format!(
                "{} (raw wait status {}), {} stdout bytes, {} stderr bytes, command: {}",
                run.termination(),
                run.raw_wait_status(),
                run.stdout().len(),
                run.stderr().len(),
                run.command_line()
            ),
            (None, Some(recorded), _) => format!(
                "recorded expectation: {} stdout bytes, expected exit status {}",
                recorded.stdout.len(),
                recorded.expect_exit
            ),
            (None, None, Some(compile)) => format!(
                "build {}, {} stderr bytes, command: {}",
                if compile.succeeded() {
                    String::from("succeeded but no execution was recorded")
                } else {
                    match compile.failure() {
                        Some(failure) => failure.to_string(),
                        None => String::from("did not succeed"),
                    }
                },
                compile.stderr().len(),
                compile.command_line()
            ),
            (None, None, None) => String::from("no observation recorded"),
        };
        sanitize_text_for_report(&format!(
            "{} [{}] {} {}: {observation}",
            self.file_stem(),
            self.role.label(),
            self.target.triple(),
            self.opt.flag()
        ))
    }
}

impl fmt::Display for Capture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

/// The deterministic identity of one finding, and therefore the name of its directory.
///
/// Derived from the divergence itself by [`FindingId::derive`], never counted and never generated,
/// so the same divergence is filed under the same name in every process and on every run. The
/// module documentation explains why that is a correctness requirement rather than a convenience.
///
/// The field is private because this value *decides a filesystem path*. A caller able to assign it
/// could name a directory that no divergence would ever derive, which would leave an artifact
/// nobody could find again from a report row, or — worse — one whose name claimed a different
/// identity from the one its contents describe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingId {
    text: String,
}

impl FindingId {
    /// Derive the identifier of the finding for one cell, one oracle and one divergence class.
    ///
    /// A pure function of those three: no counter, no process identifier, no clock, no environment.
    /// The four digits summarize the whole identity through [`fnv1a64`], and the readable components
    /// after them state it in full, with only the descriptive head ever abbreviated.
    pub fn derive(key: &CellKey, oracle: Oracle, class: DivergenceClass) -> FindingId {
        let oracle_letter = String::from(oracle.letter());
        let digits = fnv1a64(&[
            key.area(),
            key.program(),
            key.target().short_name(),
            key.opt().short(),
            &oracle_letter,
            class.label(),
        ]) % ID_DIGIT_MODULUS;
        let head = abbreviate_head(&format!(
            "{}-{}",
            kebab(strip_numeric_prefix(key.area())),
            kebab(strip_numeric_prefix(key.program()))
        ));
        FindingId {
            text: format!(
                "F-{digits:04}-{head}-{oracle_letter}-{}-{}-{}",
                key.target().short_name(),
                key.opt().short(),
                kebab(class.label())
            ),
        }
    }

    /// The identifier as text, for example
    /// `F-4821-bitfields-straddling-and-zero-width-b-aarch64-O2-stdout-mismatch`.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The directory this finding owns, always a direct child of [`findings_root`].
    ///
    /// The identifier is built from an area and program name that [`CellKey`] has already refused
    /// to accept unless it is a canonical stem, and is then rendered through [`kebab`], which emits
    /// only lower-case ASCII alphanumerics and hyphens. It can therefore contain no path separator
    /// and can be neither `.` nor `..`, so joining it onto the findings root always yields a direct
    /// child of that root. [`guarded_path`] re-establishes that on every write regardless.
    pub fn directory(&self) -> PathBuf {
        findings_root().join(&self.text)
    }
}

impl fmt::Display for FindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// What was done to reduce the reproducer, and what a maintainer can do next.
///
/// # Why reduction is not performed during a run
///
/// The external reducer is used **if the environment has one** and is **never required**. This
/// module goes one step further and does not invoke it during a run at all, for three reasons that
/// are worth stating because the decision looks like a shortcut and is not:
///
/// - **A reduction is not a bounded operation.** It re-compiles and re-runs a candidate program
///   thousands of times; minutes is a good outcome and hours is an ordinary one. Doing that inside
///   `cargo test` would turn a suite with a measured per-cell budget into one that cannot be run.
/// - **A reduction is not deterministic.** Its result depends on the reducer's version, its pass
///   schedule and how it interleaves. A finding directory is a deliverable that is compared between
///   runs to see whether a divergence changed, and a reproducer that changed on its own would make
///   every such comparison meaningless.
/// - **The corpus is already minimal by construction.** Every program in it exercises exactly one
///   semantic concern and prints one line per property it claims, which is what makes a single
///   divergent line point at a single construct. There is usually very little left to remove.
///
/// So the reproducer is a verbatim copy, this type records that no automated reduction was
/// performed and why, and — when the environment has a reducer — it carries the exact command a
/// maintainer can run against the copy. A missing reducer changes one line of a manifest and
/// nothing else: it never fails a run and never suppresses an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Minimization {
    performed: bool,
    method: String,
    reducer: Option<String>,
    guidance: String,
}

impl Minimization {
    /// Describe the minimization of one finding's reproducer in this environment.
    pub fn describe_for(id: &FindingId, caps: &Capabilities) -> Minimization {
        let reducer = caps
            .reducer()
            .path()
            .map(|path| sanitize_text_for_report(&path.display().to_string()));
        let guidance = match &reducer {
            Some(path) => format!(
                "A reducer is available at {path}. To reduce this reproducer, copy the {id} \
                 directory elsewhere, write an interestingness test that rebuilds both sides with \
                 the lines in {COMMANDS_NAME} and exits zero only while the difference persists, \
                 then run the reducer over the copy of {REPRODUCER_SOURCE_NAME}. Reduce the copy, \
                 never the corpus program: the corpus is read-only to this suite, and the program \
                 it holds is exercised by the whole matrix rather than by this finding alone."
            ),
            None => format!(
                "No reducer was found in this environment, which changes nothing about the \
                 completeness of {id}: the reproducer, the exact commands in {COMMANDS_NAME}, the \
                 captured outputs and the environment fingerprint do not depend on one. Reduce by \
                 hand if the reproducer is larger than the difference needs, always on a copy \
                 rather than on the corpus program."
            ),
        };
        Minimization {
            performed: false,
            method: format!(
                "Not performed automatically. {REPRODUCER_SOURCE_NAME} is a verbatim copy of the \
                 corpus program, which already exercises one semantic concern and prints one line \
                 per property it claims. A run does not reduce, because a reduction is unbounded in \
                 time and not reproducible byte for byte, and this directory is a deliverable that \
                 is compared between runs. Reduction is a supervised activity performed on the copy."
            ),
            reducer,
            guidance,
        }
    }

    /// Whether an automated reduction was performed. Always false during a run — see the type
    /// documentation for why, and note that a manually reduced reproducer is committed to the
    /// curated set by a human rather than produced here.
    pub fn performed(&self) -> bool {
        self.performed
    }

    /// What was done, and why.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// The reducer available in this environment, when there is one.
    pub fn reducer(&self) -> Option<&str> {
        self.reducer.as_deref()
    }

    /// What a maintainer can do next, phrased for whichever of the two environments this is.
    pub fn guidance(&self) -> &str {
        &self.guidance
    }
}

/// An undocumented divergence, together with the evidence that establishes it.
///
/// Assembled by the caller that observed the divergence and handed to [`write`] or [`record`],
/// which lay the evidence down and form no opinion about it. Keeping the writer independent of
/// classification policy is deliberate: whether a divergence is a finding, an expected divergence or
/// an unexplained failure is decided by the classifier, and a writer that could second-guess that
/// decision would be a second, undocumented classifier.
///
/// # Why the fields are private
///
/// Two of them decide a path — the corpus source and record are copied out, and the cell identity
/// names the directory — and the rest are the observations the difference was drawn from. A
/// writable field would let a finding claim one identity while its contents described another, or
/// let the evidence be edited after the comparison had already concluded from it. The builder
/// methods are the only way in, which makes "the artifacts support the description" a property of
/// the type rather than a convention every caller must remember.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    key: CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    summary: String,
    detail: String,
    description: String,
    source: PathBuf,
    record: PathBuf,
    marker_note: Option<String>,
    captures: Vec<Capture>,
}

impl Finding {
    /// Build a finding from the comparison that observed it and the program's own expectation
    /// record.
    ///
    /// The oracle and the divergence class are read from the comparison rather than passed
    /// separately, so a finding cannot be filed under one oracle while carrying another's evidence,
    /// and its class is always the one the comparator actually determined.
    ///
    /// Both corpus paths are validated on the way in through [`require_contained_corpus_file`],
    /// which requires an existing regular file — not a symbolic link — resolving strictly beneath
    /// the corpus root with the extension its role demands. A finding copies those two files into
    /// its directory, so this is where the suite proves it is copying a committed corpus program and
    /// not something a link pointed at.
    ///
    /// # Errors
    ///
    /// Refuses a comparison that observed no divergence, a comparison the record excluded from its
    /// oracle, a divergence an expected-divergence marker already covers, a record that describes a
    /// different program from the cell, and a corpus path that does not satisfy the containment
    /// rules above. Every one of those is a defect in the caller or in the corpus rather than
    /// something an environment can produce, so each is an explanatory hard failure rather than a
    /// finding written anyway.
    pub fn new(
        key: CellKey,
        comparison: &Comparison,
        manifest: &Manifest,
    ) -> HarnessResult<Finding> {
        let context = format!("assembling the finding for {key}");
        let Some(class) = comparison.class else {
            return Err(HarnessError::new(
                context,
                String::from(
                    "the comparison recorded no divergence class, so there is no divergence to \
                     report; a finding is an undocumented DIFFERENCE, and writing one for an \
                     agreement would put a defect in the register that no evidence supports",
                ),
            ));
        };
        if let Some(reason) = &comparison.excluded {
            return Err(HarnessError::new(
                context,
                format!(
                    "this comparison was not attempted — the program's own record excludes it, \
                     because: {reason}. An unattempted comparison establishes nothing, so it can be \
                     neither a pass nor a finding",
                ),
            ));
        }
        if manifest.area() != key.area() || manifest.program() != key.program() {
            return Err(HarnessError::new(
                context,
                format!(
                    "the expectation record describes {}/{} but the cell is {}/{}; a finding copies \
                     the record beside the reproducer, so a mismatch would ship a record that does \
                     not govern the program next to it",
                    sanitize_text_for_report(manifest.area()),
                    sanitize_text_for_report(manifest.program()),
                    sanitize_text_for_report(key.area()),
                    sanitize_text_for_report(key.program())
                ),
            ));
        }

        let record = require_contained_corpus_file(
            &context,
            "expectation record of the program under test",
            manifest.path(),
            "expected",
        )?;
        let source = require_contained_corpus_file(
            &context,
            "source of the program under test",
            &manifest.source_path(),
            "c",
        )?;

        // A marker whose scope DOES cover this cell means the divergence is documented, which makes
        // it an expected divergence rather than a finding. Refused rather than written, for the same
        // reason an agreement is: a finding names an *undocumented* difference, so filing a
        // documented one would add an entry to the register that the repository has already
        // explained. This enforces the invariant the rest of the module states — a marker
        // identifier never appears on a finding's outcome — instead of merely relying on it.
        //
        // This is not the writer taking over classification. It renders no verdict and chooses no
        // alternative; it declines a precondition violation and names the classification the caller
        // should have reached, exactly as the three refusals above do.
        if let Some(marker) = manifest.marker() {
            if marker.covers(&key, comparison.oracle) {
                return Err(HarnessError::new(
                    context,
                    format!(
                        "expected-divergence marker {} covers this cell (class {}, scope {}), so \
                         this divergence is DOCUMENTED and belongs in the expected-divergence \
                         register, not the finding register; classify it as an expected divergence \
                         instead of writing a finding for it",
                        sanitize_text_for_report(marker.id()),
                        sanitize_text_for_report(&marker.class().to_string()),
                        sanitize_text_for_report(marker.scope().raw())
                    ),
                ));
            }
        }

        // A marker that exists but does not cover this cell is evidence worth carrying: it tells a
        // reader that the divergence was considered and scoped elsewhere, which is a different
        // situation from a program nobody has documented at all.
        let marker_note = manifest.marker().map(|marker| {
            sanitize_text_for_report(&format!(
                "the program carries expected-divergence marker {} (class {}, scope {}), whose \
                 scope does not cover this cell, so this divergence is undocumented",
                marker.id(),
                marker.class(),
                marker.scope().raw()
            ))
        });

        Ok(Finding {
            key,
            oracle: comparison.oracle,
            class,
            summary: comparison.summary.clone(),
            detail: comparison.detail.clone(),
            description: String::from(manifest.description()),
            source,
            record,
            marker_note,
            captures: Vec::new(),
        })
    }

    /// Add one capture to the evidence.
    ///
    /// Consuming and returning `self` so a finding reads as one expression at the call site. A
    /// capture whose stem duplicates one already held replaces it, which keeps the evidence a map
    /// from role and cell to observation rather than a list that could hold two answers for one
    /// question.
    pub fn with_capture(mut self, capture: Capture) -> Finding {
        let stem = capture.file_stem();
        match self
            .captures
            .iter()
            .position(|existing| existing.file_stem() == stem)
        {
            Some(index) => self.captures[index] = capture,
            None => self.captures.push(capture),
        }
        self
    }

    /// The cell the divergence was observed at.
    pub fn key(&self) -> &CellKey {
        &self.key
    }

    /// Which oracle observed it.
    pub fn oracle(&self) -> Oracle {
        self.oracle
    }

    /// The shape the divergence took.
    pub fn class(&self) -> DivergenceClass {
        self.class
    }

    /// The comparator's one-line statement of the difference.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The comparator's multi-line expansion of it.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Every capture held as evidence, in the order it was added.
    pub fn captures(&self) -> &[Capture] {
        &self.captures
    }

    /// The identifier, and therefore the directory name.
    pub fn id(&self) -> FindingId {
        FindingId::derive(&self.key, self.oracle, self.class)
    }

    /// The directory this finding will occupy beneath [`findings_root`].
    pub fn directory(&self) -> PathBuf {
        self.id().directory()
    }

    /// The subject capture: the compiler under test at the diverging cell.
    pub fn subject(&self) -> Option<&Capture> {
        self.captures
            .iter()
            .find(|capture| capture.role() == CaptureRole::UnderTest)
    }

    /// The authority capture belonging to the oracle that observed the divergence.
    pub fn authority(&self) -> Option<&Capture> {
        self.captures
            .iter()
            .find(|capture| capture.role().oracle() == Some(self.oracle))
    }

    /// How this finding's reproducer was minimized, and what a maintainer can do next.
    pub fn minimization(&self, caps: &Capabilities) -> Minimization {
        Minimization::describe_for(&self.id(), caps)
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{} {} class {}] {} captures: {}",
            self.id(),
            self.oracle,
            self.key,
            self.class,
            self.captures.len(),
            self.summary
        )
    }
}

/// Every path one finding's artifacts occupy, returned by [`write`].
///
/// Published so that a report can point a reader at the directory and a check can assert the file
/// set without re-deriving either. The entries are in the order they were written, which is also the
/// order [`REQUIRED_ARTIFACTS`] lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingArtifacts {
    id: FindingId,
    directory: PathBuf,
    entries: Vec<PathBuf>,
}

impl FindingArtifacts {
    /// The finding these artifacts belong to.
    pub fn id(&self) -> &FindingId {
        &self.id
    }

    /// The directory holding them, always a direct child of [`findings_root`].
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Every file written, in write order.
    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    /// The command that reproduces this finding with no harness, no Cargo and no Rust toolchain.
    pub fn reproduction_command(&self) -> String {
        format!(
            "sh {}",
            posix_quote(&self.directory.join(COMMANDS_NAME).display().to_string())
        )
    }

    /// One line naming the finding, its directory and how many files it holds.
    pub fn describe(&self) -> String {
        sanitize_text_for_report(&format!(
            "{} in {} ({} files); reproduce with: {}",
            self.id,
            self.directory.display(),
            self.entries.len(),
            self.reproduction_command()
        ))
    }
}

impl fmt::Display for FindingArtifacts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

/// Reject a path component that is not a plain entry name.
///
/// A component must be non-empty, must be neither `.` nor `..`, must carry no path separator of
/// either spelling, and must contain no character that cannot appear literally in a report. The
/// first three keep a write inside the directory it names; the fourth keeps a filename out of a
/// report row it could corrupt, since every artifact name is also printed.
fn validate_component(context: &str, component: &str) -> HarnessResult<()> {
    let shown = sanitize_text_for_report(component);
    if component.is_empty() {
        return Err(HarnessError::new(
            String::from(context),
            String::from("an artifact name is empty, so it names no file"),
        ));
    }
    if component == "." || component == ".." {
        return Err(HarnessError::new(
            String::from(context),
            format!("the artifact name {shown:?} names a directory relative to another rather than an entry"),
        ));
    }
    if component.contains('/') || component.contains(std::path::MAIN_SEPARATOR) {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the artifact name {shown:?} contains a path separator; each component is named \
                 separately so that a write cannot reach outside the finding directory"
            ),
        ));
    }
    if component.chars().any(super::must_escape_for_report) {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the artifact name {shown:?} contains a character that cannot appear literally in a \
                 report, and every artifact name is printed in one"
            ),
        ));
    }
    Ok(())
}

/// Require that `candidate` lies strictly beneath [`findings_root`], deciding it lexically.
///
/// Deliberately **lexical** rather than resolved: [`super::ensure_within`] answers the same question
/// on disk with symbolic links reduced, but it can only answer it for a path that already exists,
/// which is never true of a file about to be written. This runs before the write; the containment
/// argument needs both, and the resolved form is available to a caller that wants to re-check
/// afterwards.
///
/// Three conditions, each closing a distinct way a path could reach somewhere it must not:
///
/// - the findings root's component sequence is a prefix of the candidate's, compared component by
///   component rather than as text, because a text prefix test would answer "does
///   `…/conformance-findings-old/x` start with `…/conformance-findings`" with yes, which is wrong;
/// - every remaining component is a normal one, so a candidate carrying a parent-directory
///   component in its tail is refused rather than followed upward — this is the condition that makes
///   it impossible for anything here to reach the corpus, the compiler's source or the committed
///   finding set;
/// - at least one component remains, so the findings root itself is never a candidate, which keeps
///   the targeted removal in [`prepare_directory`] unable to reach beyond one finding.
fn require_beneath_findings_root(context: &str, candidate: &Path) -> HarnessResult<()> {
    let root = findings_root();
    let mut remaining = candidate.components();
    for expected in root.components() {
        if remaining.next() != Some(expected) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "{} does not lie beneath the generated-findings root {}; this module writes \
                     nothing outside that directory, and in particular never into the committed \
                     finding set, which a human promotes after review",
                    candidate.display(),
                    root.display()
                ),
            ));
        }
    }
    let mut depth = 0usize;
    for component in remaining {
        match component {
            std::path::Component::Normal(_) => depth += 1,
            _ => {
                return Err(HarnessError::new(
                    String::from(context),
                    format!(
                        "{} contains a path component that is not a plain name, so it could lead \
                         out of the generated-findings root {}",
                        candidate.display(),
                        root.display()
                    ),
                ))
            }
        }
    }
    if depth == 0 {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "{} is the generated-findings root {} itself rather than a path beneath it; a \
                 finding owns one directory under that root and touches nothing else",
                candidate.display(),
                root.display()
            ),
        ));
    }
    Ok(())
}

/// Resolve one entry of a finding directory from its components, guarding the result.
///
/// Every path this module writes passes through here, which is what makes the hermeticity claim
/// checkable in one place instead of at each write site.
fn guarded_path(context: &str, directory: &Path, components: &[&str]) -> HarnessResult<PathBuf> {
    let mut resolved = PathBuf::from(directory);
    for component in components {
        validate_component(context, component)?;
        resolved.push(component);
    }
    require_beneath_findings_root(context, &resolved)?;
    Ok(resolved)
}

/// Create a directory and every missing parent, attributing a failure to `context`.
fn create_directory(context: &str, path: &Path) -> HarnessResult<()> {
    fs::create_dir_all(path).map_err(|error| {
        HarnessError::new(
            String::from(context),
            format!(
                "{} could not be created: {error}; every artifact this module produces is written \
                 beneath the Cargo build directory, so a directory that cannot be created stops the \
                 finding from being recorded rather than diverting it elsewhere",
                path.display()
            ),
        )
    })
}

/// Write bytes to `path`, replacing whatever was there.
///
/// Bytes rather than text for the captured streams, which are compared byte for byte and must be
/// stored exactly as produced: no line-ending normalization, and no substitution for a byte that is
/// not valid text.
fn write_bytes(context: &str, path: &Path, bytes: &[u8]) -> HarnessResult<()> {
    fs::write(path, bytes).map_err(|error| {
        HarnessError::new(
            String::from(context),
            format!(
                "{} could not be written: {error}; a finding without its evidence is not a \
                 deliverable, so this is reported rather than skipped",
                path.display()
            ),
        )
    })
}

/// Write text to `path` as UTF-8, replacing whatever was there.
fn write_text(context: &str, path: &Path, text: &str) -> HarnessResult<()> {
    write_bytes(context, path, text.as_bytes())
}

/// Copy a corpus file into the finding directory.
///
/// One-directional by construction: this is how a program and its record enter a finding, and there
/// is no counterpart that writes back out, so the corpus stays read-only to the suite. Both paths
/// have already been proved to be regular files inside the corpus by [`Finding::new`].
fn copy_corpus_file(context: &str, source: &Path, destination: &Path) -> HarnessResult<()> {
    fs::copy(source, destination).map(|_| ()).map_err(|error| {
        HarnessError::new(
            String::from(context),
            format!(
                "{} could not be copied to {}: {error}",
                source.display(),
                destination.display()
            ),
        )
    })
}

/// Create this finding's directory, replacing the captured outputs of any previous run.
///
/// Re-running must be idempotent: the same divergence derives the same identifier, so a second run
/// rewrites one directory rather than accumulating a second one. Every top-level artifact is
/// rewritten unconditionally, but `outputs/` is a *set* whose membership can shrink — a divergence
/// that now involves fewer cells would otherwise leave a stale capture behind, and a stale capture
/// in a finding is worse than a missing one because it looks like evidence.
///
/// The removal is therefore targeted at exactly one directory, whose path has been proved to lie
/// strictly beneath [`findings_root`] and to be named [`OUTPUTS_DIR_NAME`] inside a directory named
/// by a derived identifier. Nothing else is removed, and no parent is ever touched.
fn prepare_directory(id: &FindingId) -> HarnessResult<(PathBuf, PathBuf)> {
    let context = format!("preparing the artifact directory for finding {id}");
    let directory = id.directory();
    require_beneath_findings_root(&context, &directory)?;
    create_directory(&context, &directory)?;

    let outputs = guarded_path(&context, &directory, &[OUTPUTS_DIR_NAME])?;
    if outputs.is_dir() {
        fs::remove_dir_all(&outputs).map_err(|error| {
            HarnessError::new(
                context.clone(),
                format!(
                    "the previous run's captures in {} could not be removed: {error}; they are \
                     replaced rather than merged so that a stale capture cannot be mistaken for \
                     evidence of the current divergence",
                    outputs.display()
                ),
            )
        })?;
    }
    create_directory(&context, &outputs)?;
    Ok((directory, outputs))
}

/// Shell variable holding the path to the source program, so every command line names one file.
const VAR_SOURCE: &str = "SRC";

/// Shell variable holding the scratch directory a reproduction writes into.
const VAR_WORK: &str = "WORK";

/// Shell variable holding the directory the script itself lives in, so the reproducer beside it can
/// be found however the script was invoked.
const VAR_FINDING_DIR: &str = "FINDING_DIR";

/// Shell variable holding the compiler under test.
const VAR_BCC: &str = "BCC";

/// The tool paths a reproduction script lifts into its preamble.
///
/// Two things have to be true at once, and the design of this type is what makes both true rather
/// than one at the expense of the other. The commands must be **exactly** what the run performed —
/// otherwise a reproduction proves something about a different invocation — and they must not bake
/// in absolute paths that belong to the machine the run happened on, or a reader on another machine
/// has to edit every line before the script will run at all.
///
/// So each tool path is declared once, and each command line references the declaration. The
/// substitution is by **exact string equality** on a whole argument, never a textual splice, so the
/// referenced line reconstructs the recorded argument vector element for element. The exact line is
/// also emitted verbatim as a comment above each command, which means nothing is hidden by the
/// indirection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ShellVariables {
    entries: Vec<(String, String)>,
}

impl ShellVariables {
    /// Declare `name` for `value`, if that is possible without contradicting an earlier
    /// declaration.
    ///
    /// First declaration wins in both directions. A value already declared keeps its original name,
    /// so two roles that happen to use the same binary share one variable; and a name already bound
    /// to a different value is left alone, in which case [`ShellVariables::reference`] returns
    /// nothing for the second value and it is written out literally. Never silently rebinding is
    /// what keeps the preamble a faithful statement of what ran.
    fn declare(&mut self, name: &str, value: &str) {
        let name_taken = self.entries.iter().any(|(existing, _)| existing == name);
        let value_declared = self.entries.iter().any(|(_, existing)| existing == value);
        if name_taken || value_declared {
            return;
        }
        self.entries.push((String::from(name), String::from(value)));
    }

    /// The shell reference for `value`, when it has a declaration.
    fn reference(&self, value: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|(_, declared)| declared == value)
            .map(|(name, _)| format!("\"${name}\""))
    }

    /// The preamble, one assignment per line, each value quoted so a path containing a space, a
    /// semicolon or a command substitution reproduces as data rather than as grammar.
    fn render(&self) -> String {
        self.entries
            .iter()
            .map(|(name, value)| format!("{name}={}\n", posix_quote(value)))
            .collect()
    }
}

/// The reference-compiler variable name for one target.
///
/// Spelled to match the harness's own documented override variables, minus their common prefix:
/// `BCC_REF_CC` for the native driver and `BCC_REF_CC_<ARCH>` for a cross driver. A reader who
/// wants to point the suite at the same compiler this script uses can therefore transfer the value
/// straight across.
fn reference_variable_name(target: Target) -> String {
    if target.is_native() {
        return String::from("REF_CC");
    }
    format!("REF_CC_{}", target.short_name().to_ascii_uppercase())
}

/// The runner variable name for one target.
///
/// The i686 suffix is `I386`, deliberately not the target slug: the emulator for that target is
/// named after `i386`, and the harness's override variable is spelled the same way. Using the
/// target slug here would produce a variable name that looks like the documented one and is not.
fn runner_variable_name(target: Target) -> String {
    let suffix = match target {
        Target::X86_64 => "X86_64",
        Target::I686 => "I386",
        Target::Aarch64 => "AARCH64",
        Target::Riscv64 => "RISCV64",
    };
    format!("QEMU_{suffix}")
}

/// Collect the tool declarations a finding's captures imply.
///
/// Derived from the captures themselves — the first element of each recorded argument vector, and
/// each recorded runner — rather than from the capability record, so the preamble states the tools
/// that were **used** rather than the tools that happened to be discovered.
fn collect_shell_variables(finding: &Finding) -> ShellVariables {
    let mut variables = ShellVariables::default();
    for capture in finding.captures() {
        if let Some(compile) = capture.compile() {
            if let Some(program) = compile.argv().first() {
                let name = match capture.role() {
                    CaptureRole::UnderTest | CaptureRole::Baseline => String::from(VAR_BCC),
                    CaptureRole::ReferenceCompiler => reference_variable_name(capture.target()),
                    // Unreachable: a recorded expectation has no build. Handled rather than
                    // asserted, because a panic here would replace a diagnosable divergence with a
                    // harness crash.
                    CaptureRole::GoldenRecord => continue,
                };
                variables.declare(&name, program);
            }
        }
        if let Some(runner) = capture.run().and_then(RunOutcome::runner) {
            if let Some(text) = runner.to_str() {
                variables.declare(&runner_variable_name(capture.target()), text);
            }
        }
    }
    variables
}

/// Render one recorded argument vector as a shell line, substituting the declared variables and the
/// per-cell paths.
///
/// `substitutions` maps a whole argument to its replacement text, already shell-safe. Matching is by
/// exact equality on the complete argument, so no partial match can rewrite a flag that merely
/// contains a path, and every argument the tables do not mention is quoted verbatim by
/// [`posix_quote`].
fn render_argv(
    argv: &[String],
    variables: &ShellVariables,
    substitutions: &[(String, String)],
) -> String {
    argv.iter()
        .map(|argument| {
            if let Some((_, replacement)) = substitutions
                .iter()
                .find(|(original, _)| original == argument)
            {
                return replacement.clone();
            }
            match variables.reference(argument) {
                Some(reference) => reference,
                None => posix_quote(argument),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Refuse an argument vector that carries a flag the shared-flag discipline forbids on its side.
///
/// Defence in depth rather than a check that is expected to fire: the record parser already refuses
/// a program whose shared-flag list names a flag only one compiler honours, so a forbidden flag
/// should be unreachable here. It is checked anyway because this script is the artifact a maintainer
/// copies from, and a reproduction script that suggested a flag the two compilers do not honour
/// identically would teach the wrong lesson to every future reader of the register.
///
/// The compiler-under-test side is judged by the same table minus the target selectors, since
/// selecting a target by flag is precisely how that side reaches a non-native backend —
/// [`is_forbidden_for_side`] is the single authority on that split.
fn require_permitted_flags(context: &str, role: CaptureRole, argv: &[String]) -> HarnessResult<()> {
    let Some(side) = role.compiler_side() else {
        return Ok(());
    };
    for argument in argv {
        if is_forbidden_for_side(argument, side) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the recorded invocation for the {role} capture passes {} to the {side}, which \
                     the shared-flag discipline forbids there; the reproduction script is the \
                     artifact a maintainer copies from, so it is not written at all rather than \
                     written with a flag the two compilers do not honour identically",
                    sanitize_text_for_report(argument)
                ),
            ));
        }
    }
    Ok(())
}

/// Every capture in a fixed order — subject first, then the authorities in oracle order, each
/// grouped by target and optimization level.
///
/// Ordered here rather than relying on the order the driver happened to add them, so that the same
/// finding produces byte-identical artifacts no matter how the caller assembled its evidence. An
/// artifact directory is compared between runs, so a difference in it must mean a difference in the
/// divergence.
fn ordered_captures(finding: &Finding) -> Vec<&Capture> {
    let mut ordered: Vec<&Capture> = finding.captures().iter().collect();
    ordered.sort_by_key(|capture| (capture.role(), capture.target(), capture.opt()));
    ordered
}

/// A comment line, with the text made safe so nothing can break out of the comment.
///
/// A single line feed in a captured path or a compiler message would end the comment and leave the
/// remainder of the text as shell code. Sanitizing is what makes that impossible; the executed lines
/// beside these comments carry the true, unescaped values, quoted by [`posix_quote`].
fn comment(text: &str) -> String {
    format!("# {}\n", sanitize_text_for_report(text))
}

/// Render the exact reproduction script for one finding.
///
/// # What makes it exact
///
/// Every command line is rendered from the argument vector the run actually spawned, so pasting it
/// reconstructs that vector element for element. Above each one, the recorded line is repeated
/// verbatim as a comment, so the variable references cannot hide what ran.
///
/// That vector is the program's own expectation record already rendered: the command templates in
/// the record are turned into arguments by `manifest`'s renderers, which `compile` and `execute`
/// then spawn and retain. Taking the retained vector rather than rendering the template a second
/// time is what makes "exact" literally true — a second rendering could only ever be *equal* to
/// what ran, whereas this *is* what ran, and a substitution that differed between the two would be
/// invisible in a re-render and obvious here.
///
/// # Why it is not marked executable
///
/// Setting a mode bit needs a platform-specific interface, and the `unsafe` keyword and
/// platform-conditional code are both things this suite does without. A plain file with a shebang
/// and a documented `sh commands.sh` reproduces just as exactly, on every machine, with no
/// conditional compilation at all.
///
/// # Errors
///
/// Refuses to render a script whose recorded invocations carry a flag the shared-flag discipline
/// forbids on that side — see [`require_permitted_flags`].
fn render_commands(finding: &Finding, id: &FindingId) -> HarnessResult<String> {
    let context = format!("rendering the reproduction commands for finding {id}");
    let captures = ordered_captures(finding);
    for capture in &captures {
        if let Some(compile) = capture.compile() {
            require_permitted_flags(&context, capture.role(), compile.argv())?;
        }
        if let Some(run) = capture.run() {
            require_permitted_flags(&context, capture.role(), run.argv())?;
        }
    }

    let variables = collect_shell_variables(finding);
    let mut script = String::from("#!/bin/sh\n");
    script.push_str(&comment(&format!(
        "Exact reproduction commands for finding {id}."
    )));
    script.push_str("#\n");
    script.push_str(&comment(&format!(
        "Cell    : {} under {}",
        finding.key(),
        finding.oracle()
    )));
    script.push_str(&comment(&format!("Class   : {}", finding.class())));
    script.push_str("#\n");
    script.push_str(&comment(
        "WARNING: this script reproduces a DIVERGENCE. It is expected to produce two different \
         results for the same program, and that difference is the finding.",
    ));
    script.push_str("#\n");
    script.push_str(&comment(
        "A finding is a deliverable, not a defect to patch. Nothing here changes the compiler, and \
         no compiler source file was modified in response to it.",
    ));
    script.push_str("#\n");
    script.push_str(&comment(&format!("Run it with:  sh {COMMANDS_NAME}")));
    script.push_str(&comment(
        "No executable bit is set, so no platform-specific call was needed to write this file.",
    ));
    script.push_str("#\n");
    script.push_str(&comment(
        "Each command below is the line this run performed, repeated verbatim in the comment above \
         it. Only the tool paths are lifted into the variables in the preamble, so the script can be \
         adjusted on another machine without editing every line. Every build carries -static and one \
         optimization level, and nothing else beyond the output path: that is the whole set of flags \
         the suite verified both compilers honour with the same meaning. The compiler under test \
         selects its backend with a target argument, which it alone accepts; the reference compiler \
         selects a target by being a different driver binary.",
    ));
    script.push_str("#\n");
    script.push_str(&comment(&format!(
        "Scratch output goes to ${VAR_WORK}, which defaults to ./repro-work in the current \
         directory. Set {VAR_WORK} to put it elsewhere. Nothing outside it is written."
    )));
    script.push_str("\nset -eu\n\n");

    script.push_str(&comment(
        "--- tools this run used (adjust the paths if yours differ) ------------------",
    ));
    script.push_str(&comment(
        "These correspond to the suite's own override variables, each with a BCC_ prefix: BCC_BIN, \
         BCC_REF_CC, BCC_REF_CC_<ARCH> and BCC_QEMU_<ARCH>.",
    ));
    let preamble = variables.render();
    if preamble.is_empty() {
        script.push_str(&comment(
            "No tool path was recorded for this finding, which happens when no build and no \
             execution reached the point of being spawned.",
        ));
    } else {
        script.push_str(&preamble);
    }

    script.push('\n');
    script.push_str(&comment(
        "--- inputs and scratch -----------------------------------------------------",
    ));
    script.push_str(&comment(&format!(
        "The reproducer travels with this script, so {VAR_SOURCE} points beside it rather than into \
         a checkout."
    )));
    script.push_str(&format!(
        "{VAR_FINDING_DIR}=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\n"
    ));
    script.push_str(&format!(
        "{VAR_SOURCE}=\"${VAR_FINDING_DIR}/{REPRODUCER_SOURCE_NAME}\"\n"
    ));
    script.push_str(&format!("{VAR_WORK}=\"${{{VAR_WORK}:-./repro-work}}\"\n"));
    script.push_str(&format!("mkdir -p \"${VAR_WORK}\"\n"));

    for capture in &captures {
        script.push_str(&render_capture_block(capture, &variables));
    }
    script.push_str(&render_comparison_block(finding, &captures));
    Ok(script)
}

/// Render the build-and-run block for one capture.
fn render_capture_block(capture: &Capture, variables: &ShellVariables) -> String {
    let stem = capture.file_stem();
    let mut block = String::from("\n");
    block.push_str(&comment(
        "---------------------------------------------------------------------------",
    ));
    block.push_str(&comment(&format!(
        "{stem} — {} — {} at {}",
        capture.role().label(),
        capture.target().triple(),
        capture.opt().flag()
    )));
    block.push_str(&comment(
        "---------------------------------------------------------------------------",
    ));

    if capture.role() == CaptureRole::GoldenRecord {
        block.push_str(&comment(&format!(
            "Nothing to build or run: this authority is the stdout recorded in \
             {REPRODUCER_RECORD_NAME}, which travels with this script. The bytes it prescribes are \
             in {OUTPUTS_DIR_NAME}/{stem}.stdout beside it."
        )));
        return block;
    }

    // The artifact the run produced is machine-specific and lives in a workspace under the build
    // directory, so it is replaced by a path inside the reader's own scratch directory. One entry
    // covers both the build's output argument and the run's operand, because they are the same
    // recorded text.
    let mut substitutions: Vec<(String, String)> = Vec::new();
    if let Some(compile) = capture.compile() {
        if let Some(artifact) = compile.artifact().to_str() {
            substitutions.push((
                String::from(artifact),
                format!("\"${VAR_WORK}/{stem}.out\""),
            ));
        }
    }
    if let Some(source) = capture.compile().and_then(|compile| {
        compile
            .argv()
            .iter()
            .find(|argument| argument.ends_with(".c"))
    }) {
        substitutions.push((source.clone(), format!("\"${VAR_SOURCE}\"")));
    }

    if let Some(compile) = capture.compile() {
        block.push_str(&comment("build, exactly as this run performed it:"));
        block.push_str(&comment(&format!("  {}", compile.command_line())));
        block.push_str("status=0\n");
        block.push_str(&format!(
            "{} > \"${VAR_WORK}/{stem}.compile.stdout\" 2> \"${VAR_WORK}/{stem}.compile.stderr\" || status=$?\n",
            render_argv(compile.argv(), variables, &substitutions)
        ));
        block.push_str(&format!(
            "printf 'build %s : exit %s\\n' {} \"$status\"\n",
            posix_quote(&stem)
        ));
    }

    match capture.run() {
        Some(run) => {
            block.push_str(&comment("run, exactly as this run performed it:"));
            block.push_str(&comment(&format!("  {}", run.command_line())));
            block.push_str("status=0\n");
            block.push_str(&format!(
                "{} > \"${VAR_WORK}/{stem}.stdout\" 2> \"${VAR_WORK}/{stem}.stderr\" || status=$?\n",
                render_argv(run.argv(), variables, &substitutions)
            ));
            block.push_str(&format!(
                "printf 'run   %s : exit %s\\n' {} \"$status\"\n",
                posix_quote(&stem)
            ));
        }
        None => {
            block.push_str(&comment(
                "This side produced no runnable artifact in the recorded run, so there is no run \
                 line: the build above is the whole reproduction for it, and its diagnostics are in \
                 the outputs directory beside this script.",
            ));
            if let Some(failure) = capture.compile().and_then(CompileOutcome::failure) {
                block.push_str(&comment(&format!("  recorded outcome: {failure}")));
            }
        }
    }
    block
}

/// Render the closing block that states the difference and compares the two streams.
fn render_comparison_block(finding: &Finding, captures: &[&Capture]) -> String {
    let mut block = String::from("\n");
    block.push_str(&comment(
        "---------------------------------------------------------------------------",
    ));
    block.push_str(&comment("the difference this finding records"));
    block.push_str(&comment(
        "---------------------------------------------------------------------------",
    ));
    block.push_str(&comment(finding.summary()));
    block.push_str(&comment(
        "Only stdout bytes and exit status are compared. Standard error is captured beside this \
         script and never compared, because diagnostic wording legitimately differs between \
         compilers.",
    ));

    let subject = captures
        .iter()
        .find(|capture| capture.role() == CaptureRole::UnderTest);
    let authority = captures
        .iter()
        .find(|capture| capture.role().oracle() == Some(finding.oracle()));
    let (Some(subject), Some(authority)) = (subject, authority) else {
        block.push_str(&comment(
            "Both sides of the comparison were not captured, so no automatic comparison is offered \
             here; the captured outputs beside this script are the evidence.",
        ));
        return block;
    };

    // A side that ran is re-produced by this script in the scratch directory; the recorded
    // expectation was never produced by a process, so it is read from the copy that travels with the
    // script.
    let subject_file = format!("\"${VAR_WORK}/{}.stdout\"", subject.file_stem());
    let authority_file = if authority.role() == CaptureRole::GoldenRecord {
        format!(
            "\"${VAR_FINDING_DIR}/{OUTPUTS_DIR_NAME}/{}.stdout\"",
            authority.file_stem()
        )
    } else {
        format!("\"${VAR_WORK}/{}.stdout\"", authority.file_stem())
    };
    block.push_str(&comment(&format!(
        "subject   : {} ({})",
        subject.file_stem(),
        subject.role().label()
    )));
    block.push_str(&comment(&format!(
        "authority : {} ({})",
        authority.file_stem(),
        authority.role().label()
    )));
    block.push('\n');
    block.push_str("if command -v cmp > /dev/null 2>&1; then\n");
    block.push_str(&format!(
        "    if cmp {authority_file} {subject_file}; then\n"
    ));
    block.push_str(
        "        printf 'stdout: identical here, so the recorded difference did not reproduce\\n'\n",
    );
    block.push_str("    else\n");
    block.push_str("        printf 'stdout: DIFFERS, which is the finding\\n'\n");
    block.push_str("    fi\n");
    block.push_str("else\n");
    block.push_str(&format!(
        "    printf 'stdout: cmp is unavailable; compare %s and %s by hand\\n' {authority_file} {subject_file}\n"
    ));
    block.push_str("fi\n");
    block
}

/// Render the environment fingerprint.
///
/// # The question this artifact exists to answer
///
/// When a finding stops reproducing, or starts, there are two possible explanations and only one of
/// them is about the compiler: either the compiler changed, or the toolchain around it did. The
/// project's own risk register carries emulator version incompatibility as a known integration risk,
/// and without a fingerprint a divergence recorded on one toolchain and re-checked on another cannot
/// be attributed to either side. So every version the run depended on is recorded here — the
/// compiler under test, the native reference driver, each cross driver, each emulator, the C runtime
/// for each target, and the kernel — beside the values the reproduction script was written against.
///
/// The fingerprint proper comes from the capability record, which is the single place that knows what
/// was discovered and how. Appended to it are the shell variable values the script's preamble
/// declares, which is what lets a reader confirm that the script they are about to run points at the
/// same tools this run used before concluding anything from a difference in its output.
fn render_environment(
    finding: &Finding,
    caps: &Capabilities,
    variables: &ShellVariables,
) -> String {
    let id = finding.id();
    let mut text = format!("# environment for finding {id}\n");
    text.push_str(&format!("# cell   : {}\n", finding.key()));
    text.push_str(&format!("# oracle : {}\n", finding.oracle()));
    text.push_str(
        "#\n# Recorded so that a divergence which appears or disappears later can be attributed to \
         a\n# toolchain change rather than to the compiler. Compare this file before concluding \
         anything\n# from a difference between two runs.\n#\n",
    );
    text.push_str(&caps.render_fingerprint());

    text.push_str(&format!(
        "\n# tool paths the {COMMANDS_NAME} preamble was written against\n"
    ));
    let preamble = variables.render();
    if preamble.is_empty() {
        text.push_str(
            "# (none: no build and no execution reached the point of being spawned for this \
             finding)\n",
        );
    } else {
        text.push_str(&preamble);
    }
    text
}

/// Render the computed difference.
///
/// # What is highlighted, and by whom
///
/// The unified rendering is produced by the comparator, which is also what the oracles compare with,
/// so this file shows the difference exactly as it was judged rather than a second opinion about it.
/// That rendering already marks the first divergent line in its left margin and states the byte
/// offset, line and column of the first differing byte; the header here restates the same location
/// in one place at the top, so a reader does not have to scan for the marker to learn where the two
/// streams parted.
///
/// # Why it is bounded twice
///
/// The comparator bounds its own output by rows and says so when it truncates. A row bound is not a
/// byte bound: a program that prints one enormous line diverges in a single row that could be
/// megabytes wide. So the result is bounded again here by bytes, at a character boundary, with its
/// own explicit notice. An unbounded artifact is not more evidence than a bounded one — it is a file
/// a maintainer cannot open.
fn render_diff(finding: &Finding) -> String {
    let id = finding.id();
    let mut text = format!("# computed difference for finding {id}\n");
    text.push_str(&format!("# cell    : {}\n", finding.key()));
    text.push_str(&format!("# oracle  : {}\n", finding.oracle()));
    text.push_str(&format!("# class   : {}\n", finding.class()));
    text.push_str(&format!(
        "# summary : {}\n",
        sanitize_text_for_report(finding.summary())
    ));

    let (Some(subject), Some(authority)) = (finding.subject(), finding.authority()) else {
        text.push_str(
            "#\n# Both sides of the comparison were not captured, so no difference can be computed \
             here.\n# The captured outputs beside this file are the evidence, and the detail below \
             is the\n# comparator's own account of what it observed.\n#\n",
        );
        text.push_str(&sanitize_text_for_report(finding.detail()));
        text.push('\n');
        return text;
    };

    text.push_str(&format!(
        "# expected: {} — {}\n",
        authority.file_stem(),
        authority.role().label()
    ));
    text.push_str(&format!(
        "# actual  : {} — {}\n",
        subject.file_stem(),
        subject.role().label()
    ));
    match locate_stdout_divergence(authority.stdout(), subject.stdout()) {
        Some(divergence) => text.push_str(&format!(
            "# first divergent byte: offset {}, line {}, column {}\n",
            divergence.offset, divergence.line, divergence.column
        )),
        None => text.push_str(
            "# stdout is byte-identical on both sides: this divergence is in the exit status or in \
             whether a side ran at all, not in the bytes.\n",
        ),
    }
    text.push_str("#\n");
    text.push_str(&bound_text(&unified_diff(
        authority.stdout(),
        subject.stdout(),
    )));

    text.push_str("\n# comparator detail\n");
    text.push_str(&sanitize_text_for_report(finding.detail()));
    text.push('\n');
    text
}

/// Truncate `text` to [`MAX_DIFF_BYTES`] at a character boundary, appending an explicit notice.
///
/// The boundary walk is what keeps the result valid text: cutting a multi-byte character in half
/// would leave a file that cannot be read as text at all, which is a strictly worse artifact than a
/// shorter one.
fn bound_text(text: &str) -> String {
    if text.len() <= MAX_DIFF_BYTES {
        return String::from(text);
    }
    let mut boundary = MAX_DIFF_BYTES;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    let kept = &text[..boundary];
    format!(
        "{kept}\n[truncated: {} of {} bytes shown, because a difference a maintainer cannot open is \
         not evidence. The captured streams beside this file are complete and unmodified — compare \
         them directly for the remainder.]\n",
        kept.len(),
        text.len()
    )
}

/// Render the plain-text manifest.
///
/// # Describe, never diagnose
///
/// This is the paragraph a maintainer reads first, and everything in it is an observation: which
/// cell, which oracle, which shape the difference took, what each side produced, and where they
/// parted. It offers no theory about which compiler is wrong or which pass caused it. That
/// restraint is the point — the suite's authority comes from having compared two independent
/// implementations and reported what it saw, and a guess recorded beside the evidence in the same
/// voice would be indistinguishable from a measurement to every later reader.
fn render_manifest(
    finding: &Finding,
    id: &FindingId,
    caps: &Capabilities,
    minimization: &Minimization,
) -> String {
    let mut text = String::from("BLITZY C COMPILER — DIFFERENTIAL CONFORMANCE FINDING\n");
    text.push_str("====================================================\n\n");
    text.push_str(&format!("finding_id       = {id}\n"));
    text.push_str(&format!("area             = {}\n", finding.key().area()));
    text.push_str(&format!("program          = {}\n", finding.key().program()));
    text.push_str(&format!(
        "cell             = {} at {}\n",
        finding.key().target().triple(),
        finding.key().opt().flag()
    ));
    text.push_str(&format!(
        "oracle           = {}, letter {}\n",
        finding.oracle(),
        finding.oracle().letter()
    ));
    text.push_str(&format!("divergence_class = {}\n", finding.class()));
    text.push_str(&format!("verdict          = {}\n", Verdict::Finding));
    text.push_str(&format!(
        "description      = {}\n",
        sanitize_text_for_report(&finding.description)
    ));

    text.push_str("\nWHAT WAS OBSERVED\n-----------------\n");
    text.push_str(&sanitize_text_for_report(finding.summary()));
    text.push_str("\n\n");
    text.push_str(&render_observation_paragraph(finding));

    text.push_str("\nDIVERGING ORACLES AND CELLS\n---------------------------\n");
    text.push_str(&format!(
        "The divergence was observed by {} at the cell above. Every capture below is evidence for \
         it; a capture is listed whether or not it is one of the two sides that differed, because a \
         side that agreed is part of what makes the difference meaningful.\n\n",
        finding.oracle()
    ));
    for capture in ordered_captures(finding) {
        text.push_str(&format!("  {}\n", capture.describe()));
    }
    if let Some(note) = &finding.marker_note {
        text.push_str(&format!("\n  note: {note}\n"));
    }

    text.push_str("\nARTIFACTS IN THIS DIRECTORY\n---------------------------\n");
    text.push_str(&format!(
        "  {REPRODUCER_SOURCE_NAME:<22} the program, copied from the corpus\n"
    ));
    text.push_str(&format!(
        "  {REPRODUCER_RECORD_NAME:<22} its expectation record, so the pair remains runnable by the \
         harness\n"
    ));
    text.push_str(&format!("  {MANIFEST_NAME:<22} this file\n"));
    text.push_str(&format!(
        "  {COMMANDS_NAME:<22} the exact compile and run lines, reproducible with no harness \
         (sh {COMMANDS_NAME})\n"
    ));
    text.push_str(&format!(
        "  {:<22} the captured streams, one set per capture\n",
        format!("{OUTPUTS_DIR_NAME}/")
    ));
    text.push_str(&format!(
        "  {ENVIRONMENT_NAME:<22} every tool version this run depended on, plus the kernel\n"
    ));
    text.push_str(&format!(
        "  {DIFF_NAME:<22} the computed difference, with the first divergent byte located\n"
    ));

    text.push_str("\nREADING THE OUTPUTS DIRECTORY\n-----------------------------\n");
    text.push_str(&format!(
        "Each entry is named <role>-<target>-<opt>, where the leading token says whose result the \
         file holds:\n\n  {SUBJECT_PREFIX:<4} the compiler under test — the subject of every \
         comparison\n"
    ));
    for role in CaptureRole::ALL {
        if let Some(oracle) = role.oracle() {
            text.push_str(&format!(
                "  {:<4} {}\n",
                role.prefix(),
                sanitize_text_for_report(&format!("{oracle} — {}", role.label()))
            ));
        }
    }
    text.push_str(
        "\nFor each entry: .stdout and .stderr are the PROGRAM's streams, byte for byte as \
         produced;\n.exit states the termination in decoded form and as the raw wait status, so a \
         signal\ndeath can never be read as an ordinary exit with the same number. The COMPILER's \
         own\ndiagnostics are a different stream and are always in .compile.stderr, with \
         .compile.exit\nbeside it.\n\nStandard error is captured and NEVER compared. Diagnostic \
         wording legitimately differs\nbetween compilers, so comparing it would report differences \
         that say nothing about code\ncorrectness — but it is often the fastest route to a \
         diagnosis, so it belongs here.\n",
    );

    text.push_str("\nMINIMIZATION\n------------\n");
    text.push_str(&format!(
        "performed = {}\n",
        if minimization.performed() {
            "yes"
        } else {
            "no"
        }
    ));
    text.push_str(&format!("method    = {}\n", minimization.method()));
    text.push_str(&format!(
        "reducer   = {}\n",
        minimization
            .reducer()
            .unwrap_or("not present in this environment")
    ));
    text.push_str(&format!("next      = {}\n", minimization.guidance()));

    text.push_str("\nSTATUS OF THIS FINDING\n----------------------\n");
    text.push_str(
        "A finding is a DELIVERABLE, not a defect to patch. No compiler source file was modified \
         in\nresponse to it, and none will be by this suite: the requirement that produced this \
         file\nsays to keep the program, the outputs from each compiler and backend, and the exact\n\
         reproduction commands, and explicitly not to attempt a fix. Acting on it is a maintainer's\n\
         decision, informed by this evidence.\n\nA finding does not fail the run. It is reported \
         loudly, counted in the run summary, and\nindexed from the area report. It is not a pass, \
         and it is never silent.\n",
    );

    text.push_str("\nPROVENANCE\n----------\n");
    text.push_str(&format!(
        "corpus_source = {}\n",
        sanitize_text_for_report(&finding.source.display().to_string())
    ));
    text.push_str(&format!(
        "corpus_record = {}\n",
        sanitize_text_for_report(&finding.record.display().to_string())
    ));
    text.push_str(&format!(
        "reference_cc  = {}\n",
        match caps.ref_cc_for(finding.key().target()) {
            Some(path) => sanitize_text_for_report(&path.display().to_string()),
            None => String::from("(none discovered for this target)"),
        }
    ));
    text.push_str(&format!(
        "runner        = {}\n",
        match caps.runner_for(finding.key().target()) {
            Some(path) => sanitize_text_for_report(&path.display().to_string()),
            None => String::from("(native execution: no runner)"),
        }
    ));
    text.push_str(
        "This directory is generated output beneath the Cargo build directory and is not committed. \
         A\nmaintainer promotes a reviewed finding into the curated set by hand; nothing here writes \
         to\nit, so an in-progress run can never disturb the committed register.\n",
    );
    text
}

/// The plain-language paragraph describing what each side produced and where they parted.
fn render_observation_paragraph(finding: &Finding) -> String {
    let (Some(subject), Some(authority)) = (finding.subject(), finding.authority()) else {
        return String::from(
            "Only one side of the comparison was captured, so the difference is stated by the \
             comparator's summary above and by the captured outputs in this directory rather than \
             side by side here.\n",
        );
    };

    let mut text = format!(
        "The compiler under test built and ran this program for {} at {}. Judged against {}, the \
         two results differ.\n\n",
        finding.key().target().triple(),
        finding.key().opt().flag(),
        authority.role().label()
    );
    text.push_str(&format!(
        "  authority {:<14} {} stdout bytes, {}\n",
        authority.file_stem(),
        authority.stdout().len(),
        describe_termination(authority)
    ));
    text.push_str(&format!(
        "  subject   {:<14} {} stdout bytes, {}\n",
        subject.file_stem(),
        subject.stdout().len(),
        describe_termination(subject)
    ));

    match locate_stdout_divergence(authority.stdout(), subject.stdout()) {
        Some(divergence) => {
            text.push_str(&format!("\n{}\n", divergence.summary()));
        }
        None => text.push_str(
            "\nThe two streams are byte-identical, so the difference is in the exit status or in \
             whether a side ran at all rather than in what was printed.\n",
        ),
    }
    text
}

/// One phrase stating how a capture ended, or that it never ran.
fn describe_termination(capture: &Capture) -> String {
    match capture.run() {
        Some(run) => run.termination().to_string(),
        None if capture.role() == CaptureRole::GoldenRecord => {
            String::from("recorded expectation, never executed")
        }
        None => String::from("did not run"),
    }
}

/// Write one capture's entries into `outputs/`, returning the paths written.
///
/// Four entries at most, and the rule for which stream goes where has no exceptions:
///
/// - `.stdout` and `.stderr` always hold the **program's** streams, byte for byte, and are empty
///   when the program never ran;
/// - `.exit` always states how the program ended, or that it did not, and never presents a status it
///   does not have;
/// - `.compile.stderr` and `.compile.exit` always hold the **compiler's** diagnostics and outcome,
///   and are absent only when there was no build at all.
///
/// One rule rather than a case analysis is deliberate. A reader opening `a-aarch64-O2.stderr` must
/// be able to know what stream it holds without first working out whether that side's build
/// succeeded, and a scheme that put compiler diagnostics in `.stderr` whenever a program had not run
/// would make exactly that question unavoidable.
fn write_capture(context: &str, outputs: &Path, capture: &Capture) -> HarnessResult<Vec<PathBuf>> {
    let stem = capture.file_stem();
    let mut written = Vec::new();

    let stdout = guarded_path(context, outputs, &[&format!("{stem}.stdout")])?;
    write_bytes(context, &stdout, capture.stdout())?;
    written.push(stdout);

    let stderr = guarded_path(context, outputs, &[&format!("{stem}.stderr")])?;
    write_bytes(context, &stderr, capture.stderr())?;
    written.push(stderr);

    let exit = guarded_path(context, outputs, &[&format!("{stem}.exit")])?;
    write_text(context, &exit, &capture.exit_report())?;
    written.push(exit);

    if let Some(compile) = capture.compile() {
        let compile_stderr = guarded_path(context, outputs, &[&format!("{stem}.compile.stderr")])?;
        write_bytes(context, &compile_stderr, compile.stderr())?;
        written.push(compile_stderr);

        if let Some(report) = capture.compile_report() {
            let compile_exit = guarded_path(context, outputs, &[&format!("{stem}.compile.exit")])?;
            write_text(context, &compile_exit, &report)?;
            written.push(compile_exit);
        }
    }
    Ok(written)
}

/// Confirm that every entry a complete finding directory must hold is present on disk.
///
/// The last step of a write rather than a separate check a caller might not perform. A finding whose
/// directory is missing one artifact is not a deliverable, and the moment to discover that is while
/// the run that produced it is still able to say so.
fn require_complete(context: &str, directory: &Path) -> HarnessResult<()> {
    for name in REQUIRED_ARTIFACTS {
        let entry = guarded_path(context, directory, &[name])?;
        let present = if *name == OUTPUTS_DIR_NAME {
            entry.is_dir()
        } else {
            entry.is_file()
        };
        if !present {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the finding directory is missing {}; a finding without its complete evidence \
                     is not a deliverable, so this is reported rather than left to be discovered by \
                     whoever reads the register",
                    entry.display()
                ),
            ));
        }
    }
    Ok(())
}

/// Write one finding's complete artifact directory beneath [`findings_root`].
///
/// # What is produced
///
/// Exactly the entries [`REQUIRED_ARTIFACTS`] names, and nothing else: the reproducer and its
/// expectation record, the plain-text manifest, the exact reproduction script, one capture set per
/// compiler and per backend involved, the environment fingerprint, and the computed difference.
/// Completeness is confirmed on disk before the call returns.
///
/// # Render first, write second
///
/// Every artifact's content is produced before anything is written. The reproduction script is the
/// one artifact that can legitimately refuse to be produced — it will not carry a flag the shared
/// flag discipline forbids on the side that would receive it — and rendering it first means that
/// refusal leaves no directory behind at all, rather than a partial one a later reader might mistake
/// for a complete finding.
///
/// # Idempotence
///
/// The identifier is derived from the divergence, so re-running the same failing scenario rewrites
/// this same directory rather than adding a second one. `outputs/` is replaced rather than merged,
/// because its membership can shrink and a stale capture is worse than a missing one: it looks like
/// evidence.
///
/// # Hermeticity
///
/// Every path written passes through [`guarded_path`], which proves it lies strictly beneath the
/// generated-findings root. Nothing here writes into the corpus, into the committed finding set, or
/// anywhere else in the repository; the curated set is promoted by a human after review.
///
/// # Errors
///
/// Reports a refused reproduction script, a corpus file that cannot be copied, any write that fails,
/// and a directory that is incomplete once written. Each names the path and the cause. A finding
/// whose artifacts could not be written is surfaced as a failure by [`record`] rather than being
/// quietly downgraded, because the artifacts *are* the deliverable.
pub fn write(finding: &Finding, caps: &Capabilities) -> HarnessResult<FindingArtifacts> {
    let id = finding.id();
    let context = format!("writing the artifacts for finding {id}");

    // Rendered before the directory exists, so a refusal leaves nothing behind.
    let commands = render_commands(finding, &id)?;
    let variables = collect_shell_variables(finding);
    let environment = render_environment(finding, caps, &variables);
    let diff = render_diff(finding);
    let minimization = finding.minimization(caps);
    let manifest = render_manifest(finding, &id, caps, &minimization);

    let (directory, outputs) = prepare_directory(&id)?;
    let mut entries = Vec::new();

    // Written in the order REQUIRED_ARTIFACTS lists, so the returned paths and the completeness
    // check read in the same sequence as the documented artifact table.
    let source = guarded_path(&context, &directory, &[REPRODUCER_SOURCE_NAME])?;
    copy_corpus_file(&context, &finding.source, &source)?;
    entries.push(source);

    let record = guarded_path(&context, &directory, &[REPRODUCER_RECORD_NAME])?;
    copy_corpus_file(&context, &finding.record, &record)?;
    entries.push(record);

    let manifest_path = guarded_path(&context, &directory, &[MANIFEST_NAME])?;
    write_text(&context, &manifest_path, &manifest)?;
    entries.push(manifest_path);

    let commands_path = guarded_path(&context, &directory, &[COMMANDS_NAME])?;
    write_text(&context, &commands_path, &commands)?;
    entries.push(commands_path);

    let environment_path = guarded_path(&context, &directory, &[ENVIRONMENT_NAME])?;
    write_text(&context, &environment_path, &environment)?;
    entries.push(environment_path);

    let diff_path = guarded_path(&context, &directory, &[DIFF_NAME])?;
    write_text(&context, &diff_path, &diff)?;
    entries.push(diff_path);

    for capture in ordered_captures(finding) {
        entries.extend(write_capture(&context, &outputs, capture)?);
    }

    require_complete(&context, &directory)?;
    Ok(FindingArtifacts {
        id,
        directory,
        entries,
    })
}

/// Write one finding's artifacts and return the outcome a report should carry for it.
///
/// # Why a finding does not fail the run
///
/// A finding is a **deliverable**. The requirement that produced this module says to keep the
/// program, the outputs from each compiler and backend, and the exact reproduction commands, and
/// says explicitly not to attempt a fix — so a run that discovers one has done its job, not failed
/// at it. It is reported loudly, counted in the summary and pointed at from the area report; it is
/// never a silent pass, and the verdict is never softened into one.
///
/// # Why a failed write *does* fail the run
///
/// The artifacts are the whole deliverable. A finding announced with nothing behind it tells a
/// maintainer that something diverged and leaves them no way to see it, which is worse than a plain
/// failure because it looks like a result. So when the artifacts cannot be written, the verdict is
/// [`Verdict::Fail`] with the path and the cause in its detail — loud, actionable, and impossible to
/// mistake for a finding that was recorded properly.
///
/// The marker identifier is always absent: a finding is by definition an **undocumented**
/// divergence. One that a marker covered would have been classified as an expected divergence, and
/// cannot reach this point at all — [`Finding::new`] refuses to assemble it.
pub fn record(finding: &Finding, caps: &Capabilities) -> Outcome {
    match write(finding, caps) {
        Ok(artifacts) => Outcome::new(
            finding.key().clone(),
            finding.oracle(),
            Verdict::Finding,
            Some(finding.class()),
            None,
            format!(
                "{} — undocumented divergence recorded as a deliverable, not patched. Evidence in \
                 {}; reproduce with no harness, no Cargo and no Rust toolchain using: {}",
                finding.summary(),
                artifacts.directory().display(),
                artifacts.reproduction_command()
            ),
        ),
        Err(error) => Outcome::new(
            finding.key().clone(),
            finding.oracle(),
            Verdict::Fail,
            Some(finding.class()),
            None,
            format!(
                "an undocumented divergence was observed and its artifacts could NOT be written, so \
                 there is no reproducer to act on: {error}. The divergence itself was: {}",
                finding.summary()
            ),
        ),
    }
}
