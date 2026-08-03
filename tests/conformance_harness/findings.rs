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
//! | [`REPRODUCER_SOURCE_NAME`] | The program, copied **verbatim**. A run performs no reduction; the recorded minimization status says so and what to do next. See [`Minimization`]. |
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
//! - `.compile.stdout`, `.compile.stderr` and `.compile.exit` are always the **compiler's** own
//!   streams and status, present for every side that ran a build and absent for one that did not.
//!
//! Standard error is captured here even though **no oracle ever compares it**: diagnostic wording
//! legitimately differs between compilers, so comparing it would bury every real finding under a
//! flood of differences that say nothing about code correctness — yet it is often the fastest
//! route to a diagnosis, so it belongs in the artifact, read by a human rather than by an oracle.
//!
//! # The identifier, and why it is derived rather than counted
//!
//! ```text
//! F-<16 hex digits>-<cell slug>-<oracle letter>-<divergence class>
//! ```
//!
//! The **cell slug** is [`CellKey::slug`]: the area, program, target and optimization level with
//! every byte outside `[A-Za-z0-9_]` escaped as `%XX` and the four parts joined with `+`. It is
//! **injective** — two different cells cannot produce the same slug — and nothing in it is
//! abbreviated or truncated. The **oracle letter** and the kebab-cased **divergence class** complete
//! the identity, and the leading **digest** is [`super::stable_digest`] over exactly those same
//! components, giving a short fixed-width handle to quote without it being the thing that
//! distinguishes two findings.
//!
//! Every part is present in full, which is the property that matters: **two different divergences
//! can never name the same directory**, so one can never overwrite another's evidence. An earlier
//! design abbreviated the names to a fixed width and distinguished the remainder with four decimal
//! digits of a hash; chosen program names could collide under it, and a collision here silently
//! replaces one finding's reproducer and captures with another's.
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
//! **Two** places legitimately differ, and both are facts about the **run** rather than about the
//! writer: the `.exit` and `.compile.exit` records state how long the process they describe took,
//! and [`ENVIRONMENT_NAME`] states the machine's tool versions. Neither is noise — a timing is
//! part of a capture, and the fingerprint is what lets a later reader tell a toolchain change from a
//! compiler change — but neither carries information about whether the divergence itself changed.
//!
//! [`DIFF_NAME`] is byte-identical in full, **including** its trailing section, which reproduces the
//! comparator's own account verbatim. That holds because the account is built from
//! [`RunOutcome::describe`] and [`CompileOutcome::describe`], and neither carries a measured
//! duration: each states the *budget* it was bounded against, which is a pure function of the
//! configuration. The measured duration is reachable only through `describe_with_timing` on those
//! same two types, and that variant is called only from progress output printed to a terminal —
//! never from a file whose bytes are compared. Confining the wall clock to `duration_ms` is what
//! buys this file its stability, and widening either `describe` would silently take it away.
//!
//! # Minimization
//!
//! **A run reduces nothing.** The reproducer it writes is a verbatim copy of the corpus program,
//! and the manifest records that status explicitly — that no automated reduction was performed, why
//! not, and the exact reducer command a maintainer can run against the copy. Nothing here, and
//! nothing in the reports, describes the artifact as *minimized*, because that would name something
//! it does not contain; reduction is a supervised activity, and a reduced reproducer reaches the
//! curated set through a human.
//!
//! Minimization is therefore best-effort **by design**. No run invokes an external reducer at all;
//! where the environment has one, its exact command is *recorded* for the maintainer who curates the
//! finding, and where it has none, minimization is manual. Either way a reducer is never required,
//! and its absence never fails a run and never suppresses an artifact — see [`Minimization`] for the
//! full reasoning and for what is recorded instead.
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
//! - Every write is beneath [`findings_root`], and every directory from that root down to the leaf
//!   is verified to be a real directory rather than a symbolic link before a byte is published.
//!   Nothing is written outside the build directory and no socket is ever opened. That is
//!   discipline over the paths this module constructs rather than an operating-system sandbox:
//!   nothing here isolates a syscall or the network, and the captured bytes it copies were
//!   produced by tools running unconfined.
//! - Writing one finding touches only that finding's own directory, so two concurrent areas of the
//!   same run never contend. Two concurrent *runs* sharing one build directory would address the
//!   same directory, because its name is derived from the divergence rather than from the run, so
//!   each finding directory carries a run-ownership stamp and a live foreign owner is refused —
//!   the same mechanism the per-cell workspaces use, shared with them rather than reimplemented.
//!
//! Edition 2021, minimum supported Rust 1.70.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use super::compare::{locate_stdout_divergence, unified_diff, Comparison};
use super::compile::CompileOutcome;
use super::env::Capabilities;
use super::execute::{RunOutcome, Termination, TIMEOUT_UTILITY_OUTER_MARGIN};
use super::manifest::{self, Manifest};
use super::sandbox::{
    claim_ownership, live_foreign_owner_identity, retire_directory_contents, RUN_OWNER_ENTRY,
};
use super::{
    create_directory_chain_below, digest_hex_of_bytes, disclosure_defects, findings_root,
    is_forbidden_for_side, posix_quote, publish_bytes_no_follow, read_file_bounded, redact_secrets,
    remove_entry, require_contained_corpus_file, require_directory_chain_below,
    require_replaceable, run_generation, sanitize_text_for_report, shown_path, stable_digest,
    CellKey, CompilerSide, DivergenceClass, HarnessError, HarnessResult, OptLevel, Oracle, Outcome,
    Replaceable, Target, Verdict, MAX_INSPECTED_FILE_BYTES,
};

/// The reproducer: a **verbatim**, byte-for-byte copy of the corpus program.
///
/// A run performs no automated reduction, so this file is never a reduced program. The
/// accompanying [`Minimization`] record states that, states why, and — when the environment has
/// a reducer — carries the exact command to reduce this copy. Reduction is a supervised
/// activity performed on the copy before a finding is promoted to the curated set.
pub const REPRODUCER_SOURCE_NAME: &str = "reproducer.c";

/// The reproducer's expectation record, so the pair remains runnable by the harness and
/// reproducible by hand.
pub const REPRODUCER_RECORD_NAME: &str = "reproducer.expected";

/// The plain-text description of the finding, read in a terminal.
pub const MANIFEST_NAME: &str = "MANIFEST.txt";

/// The [`MANIFEST_NAME`] line that records a finding's identifier, without its value.
///
/// Shared by the renderer that writes the line and the identity check that reads it back, so that
/// changing the manifest's field alignment can never quietly stop the check from finding the line it
/// depends on. A check that silently stops checking is worse than no check, because it still reads
/// like one.
pub const MANIFEST_IDENTIFIER_PREFIX: &str = "finding_id       = ";

/// The [`MANIFEST_NAME`] line recording the digest of the reproducer the evidence was produced from.
///
/// Shared by the renderer that writes it and the curated-finding check that reads it back, for the
/// same reason as [`MANIFEST_IDENTIFIER_PREFIX`]: a check that silently stops finding its line is
/// worse than no check, because it still reads like one.
pub const MANIFEST_REPRODUCER_DIGEST_PREFIX: &str = "reproducer_digest = ";

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

// -------------------------------------------------------------------------------------------------
// The run's artifact budget
//
// A finding directory is written for every divergence a run observes, and the number of divergences
// a run can observe is bounded only by the size of the matrix: 1,296 cells, each judged by up to
// three oracles. A compiler under test that cannot build anything produces exactly that run. The
// evidence in one directory is text and is small — tens of kilobytes — but "small times unbounded"
// is not a bound, and the streams a capture holds are bounded per stream rather than in aggregate,
// so a pathological program that prints megabytes multiplies through the whole matrix.
//
// Three ceilings therefore apply, and the policy differs from the one the per-cell workspaces use in
// exactly one respect that matters. A retained workspace is *optional* evidence, so `sandbox` prunes
// it and reports the pruning. A finding's artifacts **are** the deliverable, so there is nothing here
// it would be honest to prune: a directory missing its captures is not a smaller finding, it is a
// finding that cannot be acted on. Exceeding a ceiling is therefore a **loud refusal** — the write
// fails, `record` turns that into a failing verdict naming the ceiling and the totals, and the run
// says so rather than filling the build directory and discovering the limit from the filesystem.
//
// The refusal happens **before** anything is written, because every artifact's bytes are known before
// the first one is published: the whole set is rendered first, and a capture's streams are already in
// memory. So a refused finding leaves no partial directory behind, exactly as a refused reproduction
// script does.
// -------------------------------------------------------------------------------------------------

/// Upper bound on the bytes one artifact inside a finding directory may hold.
///
/// Every text artifact this module renders is already bounded by its own renderer, so this ceiling is
/// reached only by a captured stream — which is bounded per stream by the execution layer and can
/// still be large. Eight mebibytes is far more than a corpus program's few hundred bytes of output
/// and far less than a file a maintainer cannot open.
pub const FINDING_ARTIFACT_BYTES_MAX: u64 = 8 * 1024 * 1024;

/// Upper bound on the bytes one finding directory may hold in total.
///
/// A directory holds the reproducer, its record, four rendered text artifacts and up to six entries
/// per capture involved. Sixteen mebibytes is roughly two hundred times what the largest finding this
/// suite has produced needs, and reaching it means a capture is pathological rather than informative.
pub const FINDING_DIRECTORY_BYTES_MAX: u64 = 16 * 1024 * 1024;

/// Upper bound on the bytes every finding directory of one run may hold together.
///
/// This is the ceiling the review's resource finding is really about. Without it, a run against a
/// compiler that diverges everywhere writes one directory per divergence with no aggregate limit at
/// all. A gibibyte is more evidence than any investigation reads and far less than a build directory
/// can be allowed to lose.
pub const FINDING_RUN_BYTES_MAX: u64 = 1024 * 1024 * 1024;

/// How many finding directories one run may publish.
///
/// The count matters independently of the bytes: the matrix has 1,296 cells, so a run in which every
/// cell diverges legitimately files 1,296 directories and must be allowed to. Past that the number
/// can only be a defect in the derivation or a corpus that has grown, and either is worth stopping
/// for — an investigation that begins with more candidate directories than the matrix has cells has
/// not been helped by the surplus.
pub const FINDING_RUN_COUNT_MAX: u64 = 1536;

/// Bytes this run has published into finding directories so far.
static PUBLISHED_BYTES: AtomicU64 = AtomicU64::new(0);

/// Finding directories this run has published so far.
static PUBLISHED_COUNT: AtomicU64 = AtomicU64::new(0);

/// Bytes and directories this run has published as finding artifacts.
///
/// Read once, when the run summary is assembled, so the deliverable states what the run cost rather
/// than leaving it to be measured from the filesystem afterwards.
pub fn artifact_totals() -> (u64, u64) {
    (
        PUBLISHED_BYTES.load(Ordering::Relaxed),
        PUBLISHED_COUNT.load(Ordering::Relaxed),
    )
}

/// Which oracles have filed into each finding directory during this process.
///
/// The counterpart of dropping the oracle from [`FindingId::derive`]: one root cause files once, and
/// this is where the set of windows that observed it accumulates so the manifest can name all of
/// them. Keyed by identifier text rather than by [`FindingId`] so the map has one obvious ordering
/// when it is read back.
///
/// It is in-process state, and that is sufficient rather than a compromise. Every oracle arm of a
/// given cell is judged by the one area test that owns that cell, so the contributions to a
/// directory are always made by a single thread in sequence; and a *second run* re-files every
/// contribution from scratch, because the generated-findings root is emptied once per process before
/// the first directory is created.
fn contributions() -> &'static Mutex<BTreeMap<String, BTreeSet<Oracle>>> {
    static CONTRIBUTIONS: OnceLock<Mutex<BTreeMap<String, BTreeSet<Oracle>>>> = OnceLock::new();
    CONTRIBUTIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Refuse a finding whose artifacts would take this run past a ceiling.
///
/// Called once, after everything is rendered and before anything is written, with the exact byte
/// count the directory will hold. `fresh` says whether this contribution is publishing a *new*
/// directory, which is what the count ceiling governs.
///
/// The three ceilings are checked from the most specific outward, so the message names the one that
/// actually bit. Each names the ceiling, what was measured against it and the run's totals, because a
/// refusal a reader cannot size up is a refusal they cannot act on.
fn require_within_artifact_budget(
    context: &str,
    directory: &Path,
    largest_artifact: u64,
    directory_bytes: u64,
    fresh: bool,
) -> HarnessResult<()> {
    let refuse = |cause: String| {
        Err(HarnessError::new(
            String::from(context),
            format!(
                "{cause}. The artifacts are the whole of a finding, so there is nothing here it \
                 would be honest to shorten: this is refused loudly rather than published \
                 incomplete or allowed to exhaust the build directory. Run totals so far: {} \
                 byte(s) of {FINDING_RUN_BYTES_MAX} across {} directory(ies) of \
                 {FINDING_RUN_COUNT_MAX}. The directory that would have been written is {}",
                PUBLISHED_BYTES.load(Ordering::Relaxed),
                PUBLISHED_COUNT.load(Ordering::Relaxed),
                shown_path(directory)
            ),
        ))
    };

    if largest_artifact > FINDING_ARTIFACT_BYTES_MAX {
        return refuse(format!(
            "one artifact of this finding would hold {largest_artifact} byte(s), past the \
             {FINDING_ARTIFACT_BYTES_MAX}-byte ceiling on a single artifact"
        ));
    }
    if directory_bytes > FINDING_DIRECTORY_BYTES_MAX {
        return refuse(format!(
            "this finding's artifacts would hold {directory_bytes} byte(s) together, past the \
             {FINDING_DIRECTORY_BYTES_MAX}-byte ceiling on one finding directory"
        ));
    }
    let projected = PUBLISHED_BYTES
        .load(Ordering::Relaxed)
        .saturating_add(directory_bytes);
    if projected > FINDING_RUN_BYTES_MAX {
        return refuse(format!(
            "publishing this finding's {directory_bytes} byte(s) would take the run to {projected} \
             byte(s), past the {FINDING_RUN_BYTES_MAX}-byte ceiling on everything one run files"
        ));
    }
    if fresh && PUBLISHED_COUNT.load(Ordering::Relaxed) >= FINDING_RUN_COUNT_MAX {
        return refuse(format!(
            "this would be finding directory number {}, past the {FINDING_RUN_COUNT_MAX}-directory \
             ceiling on one run — which is above the number of cells the matrix has, so reaching it \
             means the derivation is filing more directories than there are divergences to file",
            PUBLISHED_COUNT.load(Ordering::Relaxed).saturating_add(1)
        ));
    }
    Ok(())
}

/// Charge one published finding directory against the run's totals.
///
/// Called after a successful write, so the totals describe what is actually on disk. A directory a
/// later contribution adds to is charged for the bytes it added and is not counted a second time.
fn charge_artifact_budget(directory_bytes: u64, fresh: bool) {
    PUBLISHED_BYTES.fetch_add(directory_bytes, Ordering::Relaxed);
    if fresh {
        PUBLISHED_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

/// What `oracle` observing the finding `id` *would* mean, without committing to it.
///
/// Returns whether this would be the **first** contribution of the run to that directory — which
/// decides whether the captured outputs are replaced or added to — and the set of oracles that would
/// then have observed it, which the manifest names.
///
/// # Why this does not record anything
///
/// Recording here is what an earlier form of this did, and it made the directory-count ceiling
/// bypassable in a way that measured cleanly and behaved wrongly: a first contribution refused by the
/// budget still left its identifier in the map, so the *next* oracle for that cell was told the
/// directory already existed, skipped the count check on that basis and published it. The ceiling was
/// set to five and twelve directories appeared. Recording only [after a successful
/// write](commit_contribution) keeps the two facts in agreement — a directory is "already ours" only
/// when it is actually on disk — so a cell whose first contribution the budget refuses has every
/// contribution refused and leaves nothing behind.
fn peek_contribution(id: &FindingId, oracle: Oracle) -> (bool, Vec<Oracle>) {
    let held = match contributions().lock() {
        Ok(held) => held,
        // A poisoned lock means a thread panicked while recording. The records already made are still
        // true, and losing them would make a subsequent contribution purge captures its siblings had
        // written — so they are recovered rather than discarded.
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut observed = held.get(id.as_str()).cloned().unwrap_or_default();
    let first = observed.is_empty();
    observed.insert(oracle);
    (first, observed.into_iter().collect())
}

/// Record that `oracle`'s evidence for the finding `id` is on disk.
///
/// Called only after the write has completed, for the reason on [`peek_contribution`].
fn commit_contribution(id: &FindingId, oracle: Oracle) {
    let mut held = match contributions().lock() {
        Ok(held) => held,
        Err(poisoned) => poisoned.into_inner(),
    };
    held.entry(String::from(id.as_str()))
        .or_default()
        .insert(oracle);
}

// Why this module derives an identifier that is injective rather than merely unlikely to repeat.
//
// A finding identifier decides a directory name, and that directory holds the whole of the
// evidence for one divergence. Two identities that derived the same name would not produce a
// warning: the second would publish over the first, and the surviving directory would carry a
// manifest describing one divergence beside captures produced by another. Nobody reading it could
// tell, which makes a collision worse than a crash.
//
// An earlier form of this identifier abbreviated the area and program names to a fixed character
// budget, cut them at a hyphen, and distinguished what remained with four decimal digits of a
// 64-bit hash. Neither half of that is sound. Abbreviation is not injective — two programs whose
// kebab-cased names agree on their first characters render identically, and cutting at a hyphen
// collapses more pairs still — and four decimal digits leave ten thousand buckets, so the
// discriminator is a coincidence away from being no discriminator at all.
//
// The identifier below is injective by construction instead, which is what removes the need for any
// collision check: a pre-existing directory bearing this name can only ever be the same identity.
// Three facts carry the argument.
//
// - `CellKey::slug` is injective over the four cell components. It escapes every byte outside
//   `[A-Za-z0-9_]` into a `%`-introduced hexadecimal pair and separates components with `+`, which
//   escaping guarantees cannot appear inside one. So a slug recovers exactly the components that
//   produced it.
// - A slug therefore contains no hyphen, and neither does a hexadecimal digest nor a single oracle
//   letter. The identifier's hyphens are consequently unambiguous separators, and the string
//   decomposes back into digest, slug, oracle letter and class with no parsing rule beyond
//   splitting.
// - The oracle letters are pairwise distinct, and the six divergence-class labels remain pairwise
//   distinct after kebab-casing, because each is already lower-case ASCII with underscores.
//
// The digest is retained, in full width rather than reduced to four decimal digits, and it is now a
// label rather than a discriminator: it gives a stable short prefix a maintainer can grep for and
// keeps the documented `F-<digits>-<slug>` shape. It is deliberately produced by the harness's
// shared `stable_digest` rather than by a private hash, so that a digest written here means the
// same thing as a digest written by a report.

/// Render text as a kebab-case token: lower-case ASCII alphanumerics, single hyphens between
/// runs of anything else, and no leading or trailing hyphen.
///
/// Applied to the divergence class label, which is already lower-case ASCII words by construction,
/// so in practice this only exchanges spaces and underscores for hyphens. The cell identity is not
/// rendered through this function — it goes through [`CellKey::slug`], whose encoding is injective —
/// but this one is still written to cope with anything, because an identifier names a directory and
/// a name that reached the filesystem carrying a path separator or a control character would be a
/// defect rather than an untidiness.
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
        // `timed_out` above is this harness's own watchdog observation and nothing else. Whether a
        // bounding utility also stood behind it is recorded separately, as a fact about how the run
        // was supervised, because no status that utility might report is interpreted anywhere.
        report.push_str(&format!(
            "outer_timeout_utility_used = {}\n",
            compile.timeout_tool_used()
        ));
        report.push_str(&format!(
            "artifact_present = {}\n",
            compile.artifact_exists()
        ));
        report.push_str(&format!(
            "artifact = {}\n",
            match compile.artifact_size() {
                Some(bytes) => format!("{bytes} bytes at {}", shown_path(compile.artifact())),
                None => format!("absent at {}", shown_path(compile.artifact())),
            }
        ));
        // A compiler ordinarily prints nothing here, and the suite never compares it — the streams
        // that decide a verdict are the *program's*. The byte count is recorded because a compiler
        // that suddenly printed to standard output is worth a maintainer noticing, and the bytes
        // themselves are written beside this record.
        report.push_str(&format!("stdout_bytes = {}\n", compile.stdout().len()));
        report.push_str(&format!("stderr_bytes = {}\n", compile.stderr().len()));
        report.push_str(&format!(
            "duration_ms = {}\n",
            compile.duration().as_millis()
        ));
        for note in compile.notes() {
            report.push_str(&format!("supervision_note = {note}\n"));
        }
        if let Some(failure) = compile.failure() {
            // The three facts are written on separate rows rather than through the `Display`
            // rendering of the failure, which packs class, scope and summary into one line. A row
            // per fact is what lets a reader compare two findings field by field, and it keeps the
            // summary — the only part that quotes the compiler — on a line of its own.
            report.push_str(&format!("failure_class = {}\n", failure.class()));
            report.push_str(&format!("failure_scope = {}\n", failure.scope()));
            report.push_str(&format!("failure = {}\n", failure.summary()));
        }
        // The line that reproduces the artifact by hand, and then the line that actually ran. They
        // differ whenever a bounding utility wrapped the invocation, and a record that conflated
        // them would either document a command a maintainer cannot use or hide the supervision the
        // run really applied.
        report.push_str(&format!("argv = {}\n", compile.command_line()));
        // What reproduces the artifact and what this run actually spawned are two different lines
        // whenever the system timeout utility was available to wrap the compiler. Recording both,
        // and saying which mechanism bounded the build, is what lets a reader attribute a build
        // that was killed at its budget to the bound rather than to the compiler — while `argv`
        // above stays the line a maintainer pastes, free of scaffolding this run added.
        report.push_str(&format!("timeout_tool = {}\n", compile.timeout_tool_used()));
        if compile.timeout_tool_used() {
            report.push_str(&format!("spawned = {}\n", compile.spawned_command_line()));
        }
        Some(report)
    }

    /// How this side terminated, and what a reproduction should therefore observe.
    ///
    /// # Why an exit code alone is not the expectation
    ///
    /// The oracles compare stdout bytes **and** termination, and the three ways a program can stop are
    /// genuinely different outcomes: an ordinary exit carries a code the program chose, a signal death
    /// carries a number the kernel chose, and a timeout means it never chose anything. The suite keeps
    /// them apart by comparing raw wait status, which is why a crash can never be mistaken here for a
    /// program that exited with the same small number.
    ///
    /// A reproduction script sees none of that. A shell reports every one of the three as a single
    /// `$?`, folding a signal death into `128 + signal` and leaving a timeout to look like whichever
    /// mechanism enforced it. So the expectation is returned in two parts: a sentence naming the
    /// outcome as the suite observed it, and the **classification** a reproduction should record for
    /// it, in the vocabulary [`SH_BOUNDED`] writes — [`TERMINATION_EXITED`],
    /// [`TERMINATION_SIGNALLED`] or [`TERMINATION_TIMEOUT`].
    ///
    /// A timeout has a classification here where it could not have a number. That is the point of the
    /// vocabulary: [`TIMEOUT_UTILITY_STATUS`] and a signalled `137` are both plausible *numbers* for
    /// an expiry, so naming either as the expectation would be untrue half the time — whereas the
    /// script derives `timeout` from having performed the kill itself and can therefore be compared
    /// against directly. The one distinction the shell genuinely cannot make is between a signal
    /// death and a program that exited with `128 + n` of its own accord; the sentence beside the
    /// classification says which the harness observed, which is where that difference is kept.
    fn shell_expectation(&self) -> (String, Option<String>) {
        if let Some(run) = &self.run {
            return match run.termination() {
                Termination::Exited(code) => (
                    format!("exited with {code}"),
                    Some(format!("{TERMINATION_EXITED} {code}")),
                ),
                Termination::Signalled(signal) => (
                    format!(
                        "killed by signal {signal}, which a shell reports as {}",
                        SHELL_SIGNAL_STATUS_BASE.saturating_add(signal)
                    ),
                    Some(format!(
                        "{TERMINATION_SIGNALLED} {}",
                        SHELL_SIGNAL_STATUS_BASE.saturating_add(signal)
                    )),
                ),
                Termination::TimedOut => (
                    format!(
                        "exceeded its {} second budget and was terminated; a reproduction records \
                         that as `{TERMINATION_TIMEOUT}` on the strength of its own watchdog having \
                         killed the command, never from a status — {TIMEOUT_UTILITY_STATUS} and {} \
                         are both plausible numbers for an expiry and neither is evidence of one",
                        run.budget().as_secs(),
                        SHELL_SIGNAL_STATUS_BASE.saturating_add(9)
                    ),
                    Some(String::from(TERMINATION_TIMEOUT)),
                ),
            };
        }
        if let Some(recorded) = &self.recorded {
            return (
                format!(
                    "never executed: this authority is the exit status the program's own \
                     expectation record prescribes, {}",
                    recorded.expect_exit
                ),
                Some(format!("{TERMINATION_EXITED} {}", recorded.expect_exit)),
            );
        }
        (
            String::from("never executed: the build produced no artifact to run"),
            None,
        )
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
    /// True when a program actually executed, as opposed to a build that failed or a recorded
    /// contract that never ran.
    pub fn ran(&self) -> bool {
        self.run.is_some()
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
/// The fields are private because this value *decides a filesystem path*. A caller able to assign it
/// could name a directory that no divergence would ever derive, which would leave an artifact
/// nobody could find again from a report row, or — worse — one whose name claimed a different
/// identity from the one its contents describe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingId {
    text: String,
    digest: String,
}

impl FindingId {
    /// Derive the identifier of the finding for one cell and one divergence class.
    ///
    /// A pure function of those two: no counter, no process identifier, no clock, no environment.
    /// The same divergence therefore derives the same identifier in every run and on every machine,
    /// which is what lets a second run rewrite one directory instead of accumulating another and
    /// lets a register entry keep pointing at the same evidence.
    ///
    /// # Why the oracle is not part of the identity
    ///
    /// It used to be, and that is what made a single divergence file itself three times. One cell
    /// whose build was refused is refused for oracle (a), for oracle (b) and for oracle (c) alike —
    /// one root cause, observed through three windows — and an oracle-keyed identity gave each window
    /// its own directory holding its own copy of the same reproducer, the same record, the same
    /// commands, the same fingerprint and the same captured compiler diagnostics. Measured on one
    /// program of twelve cells against a compiler that refuses everything: **33 directories for 12
    /// divergences.**
    ///
    /// Keying on the **root cause** instead — the cell and the class of what went wrong — files it
    /// once, and the oracles that observed it are recorded *inside* the manifest, where a set belongs.
    /// Nothing is lost: the directory then holds the union of the authority captures rather than one
    /// oracle's, so it is strictly more evidence in strictly less space, and every report row for
    /// every affected oracle points at the same directory instead of at a near-identical sibling.
    ///
    /// The class stays in the identity because it is not one root cause seen twice: a cell that
    /// refuses to build and a cell that builds and then prints the wrong bytes are different findings
    /// with different evidence, and folding them together would overwrite one with the other.
    ///
    /// # Why this is injective, and why that is not a nicety
    ///
    /// The identifier *decides a directory*, and that directory holds the only copy of a finding's
    /// evidence. Two different findings that rendered the same identifier would file into the same
    /// directory, and the second would overwrite the first — losing a deliverable silently, which
    /// is the one outcome a findings register exists to prevent. Injectivity is therefore a
    /// correctness property of this function, not a tidiness one, and it is what removes the need
    /// for a collision check: a directory already bearing this name can only be this same identity.
    ///
    /// It is established structurally rather than hoped for:
    ///
    /// - [`CellKey::slug`] is injective over the four components of a cell identity. It renders
    ///   each component through an escaping encoder whose output alphabet is `[A-Za-z0-9_]` plus
    ///   `%`-introduced hexadecimal escapes, and joins them with `+`. Two different cell identities
    ///   cannot render the same slug.
    /// - That alphabet contains **no hyphen**, and neither does a hexadecimal digest, so each
    ///   occupies exactly one hyphen-delimited field. The identifier therefore parses uniquely from
    ///   the left: `F`, the digest, the slug, and then the divergence class.
    /// - The six divergence-class labels remain pairwise distinct after kebab-casing, so the trailing
    ///   field can neither absorb nor be confused with the slug before it.
    ///
    /// Nothing is abbreviated or truncated, deliberately — truncating the descriptive part is
    /// precisely how an earlier form of this function could map two distinct programs onto one
    /// directory, and reducing the digest to four decimal digits left ten thousand buckets where a
    /// collision was a coincidence away. The digest is carried at full width and produced by the
    /// harness's shared [`stable_digest`], so a digest written here means the same thing as a digest
    /// written by a report.
    pub fn derive(key: &CellKey, class: DivergenceClass) -> FindingId {
        // Hashed from the unabbreviated names, so the digest reflects the identity in full.
        let digest = stable_digest(&[
            key.area(),
            key.program(),
            key.target().short_name(),
            key.opt().short(),
            class.label(),
        ]);
        FindingId {
            text: format!("F-{digest}-{}-{}", key.slug(), kebab(class.label())),
            digest,
        }
    }

    /// The identifier as text, for example
    /// `F-9d3c1a5f7b204e68-04_bitfields+005_straddling_and_zero_width+aarch64+O2-stdout-mismatch`.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The digest of the identity this identifier was derived from, as fixed-width hexadecimal.
    ///
    /// The whole digest, never a remainder of it, and produced by the harness's shared
    /// [`stable_digest`] so that the same components digested anywhere else in the suite render the
    /// same text. Published so that a check can compare two identities without parsing a name apart,
    /// which is what the identity verification on the write path does. Two identifiers with equal
    /// digests and equal text describe the same finding; equal digests with differing text cannot
    /// occur, because the digest is rendered into the text.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// The directory this finding owns, always a direct child of [`findings_root`].
    ///
    /// The area and program names are ones [`CellKey`] has already refused to accept unless they are
    /// canonical stems, and every part of the identifier draws on an alphabet that excludes the path
    /// separator. The digest is hexadecimal; [`CellKey::slug`] emits only `[A-Za-z0-9_]`, `+` and
    /// `%`-introduced hexadecimal pairs; the oracle letter is one ASCII letter; and [`kebab`] emits
    /// only lower-case ASCII alphanumerics and hyphens. The identifier can therefore contain no path
    /// separator and can be neither `.` nor `..`, so joining it onto the findings root always
    /// yields a direct child of that root. [`guarded_path`] re-establishes that on every write
    /// regardless.
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
/// **This module never invokes a reducer.** Nothing on the runtime path reduces anything: the
/// reproducer a run writes is a verbatim copy of the corpus program. Three reasons, worth stating
/// because the decision looks like a shortcut and is not:
///
/// - **A reduction is not a bounded operation.** It re-compiles and re-runs a candidate program
///   thousands of times; minutes is a good outcome and hours is an ordinary one. Doing that inside
///   `cargo test` would turn a suite with a measured per-cell budget into one that cannot be run.
/// - **A reduction is not deterministic.** Its result depends on the reducer's version, its pass
///   schedule and how it interleaves. A finding directory is a deliverable that is compared between
///   runs to see whether a divergence changed, and a reproducer that changed on its own would make
///   every such comparison meaningless.
/// - **A verbatim copy is already a narrow reproducer**, though it is not a minimized one. Every
///   corpus program exercises one semantic concern and prints one line per property it claims, so a
///   single divergent line already points at a single construct — which is what makes the unreduced
///   copy usable immediately. It is not the same thing as minimization: the program still carries
///   the other properties of its concern, and a human curating the finding may well cut it further.
///
/// **This module never invokes a reducer.** Reduction is curation, a human step outside any run:
/// where the environment has one, this type records the exact command a maintainer can run against
/// the copy, and where it has none, minimization is manual and this type says so. Adding one as a
/// dependency is forbidden, and a finding must not depend on a system tool being installed.
///
/// So the reproducer is a verbatim copy, this type records that no automated reduction was
/// performed and why, and — when the environment has a reducer — it carries the exact command a
/// maintainer can run against the copy. A missing reducer changes one line of a manifest and
/// nothing else: it never fails a run and never suppresses an artifact.
/// What reducing a reproducer obliges, stated in every manifest whether or not a reducer exists.
///
/// # Why this sentence is in the deliverable rather than only in the register
///
/// Every other artifact in a finding directory was produced **from** the program the reproducer was
/// when the run observed the divergence: the captured streams, the diff computed from them, the
/// recorded terminations, the description of what differed. Reducing the reproducer and leaving those
/// in place produces the one kind of broken deliverable a reader cannot detect — evidence that looks
/// complete and describes a program that is no longer there. A reduced program with unreduced
/// captures is worse than an unreduced finding, because the unreduced one is at least true.
///
/// So the obligation travels with the artifact that provokes it. The reduction happens first, the
/// affected cells are re-run against the reduced program, and the evidence is regenerated from that
/// run — not copied from the one before it. It is also a *checked* obligation rather than an
/// instruction: the manifest records the digest of the program filed beside it, and
/// [`curated_finding_defects`] refuses a curated directory whose reproducer no longer matches, so a
/// reduction that skipped the refresh fails the suite instead of reaching a reader.
const REFRESH_AFTER_REDUCING: &str = "Reducing obliges a refresh: every other artifact here was \
     produced from the program as it was, so after reducing, re-run each affected cell against the \
     reduced program and regenerate the record, the captured outputs, the diff, the commands and \
     this manifest from that run rather than copying them forward — including the manifest's \
     reproducer digest, which the curated-finding audit compares against the program actually filed.";

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
        let reducer = caps.reducer().path().map(shown_path);
        let guidance = match &reducer {
            Some(path) => format!(
                "A reducer is available at {path}. To reduce this reproducer, copy the {id} \
                 directory elsewhere, write an interestingness test that rebuilds both sides with \
                 the lines in {COMMANDS_NAME} and exits zero only while the difference persists, \
                 then run the reducer over the copy of {REPRODUCER_SOURCE_NAME}. Reduce the copy, \
                 never the corpus program: the corpus is read-only to this suite, and the program \
                 it holds is exercised by the whole matrix rather than by this finding alone. \
                 {}",
                REFRESH_AFTER_REDUCING
            ),
            None => format!(
                "No reducer was found in this environment, which changes nothing about the \
                 completeness of {id}: the reproducer, the exact commands in {COMMANDS_NAME}, the \
                 captured outputs and the environment fingerprint do not depend on one. Reduce by \
                 hand if the reproducer is larger than the difference needs, always on a copy \
                 rather than on the corpus program. {}",
                REFRESH_AFTER_REDUCING
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
    /// The contract the program's own record declares, rendered once at construction.
    ///
    /// Copied out of the record here rather than read again at write time, and for the same reason
    /// every other field is: a finding's manifest must describe the record that governed the cell
    /// that diverged, and a record re-read later could have been edited in between.
    contract: RecordedContract,
    captures: Vec<Capture>,
}

/// What a program's expectation record declares, as a finding's manifest states it.
///
/// Four facts, and each answers a question a maintainer asks before anything else.
///
/// - The **shared flag set** is requirement 3 made auditable at the point of failure: a reader
///   judging a divergence must be able to see, without opening a second file, that both compilers
///   were given the same flags and which ones.
/// - The **raw run template** is requirement 4's "build commands recorded together". `commands.sh`
///   carries the *expanded* line, which is what reproduces the cell; the unexpanded template is
///   what shows the shape the record promised, and a difference between the two is exactly the
///   kind of drift the harness cross-checks on every invocation.
/// - The **golden stdout** is oracle (c)'s authority in text form. A finding whose divergence is a
///   stdout difference is unreadable without it, and sending the reader to the record for it
///   defeats the purpose of a self-contained artifact directory.
/// - **Whether the program carries a marker at all** is stated explicitly, in both directions.
///   When one exists but does not cover the cell, `marker_note` explains it; when none exists,
///   nothing else in the manifest says so, and "this program is documented nowhere" is precisely
///   what makes the divergence a finding rather than an expected divergence.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedContract {
    /// The flags the record declares are passed identically to both compilers.
    shared_flags: Vec<String>,
    /// The record's `run_command` template, unexpanded.
    run_template: String,
    /// The record's `expected_stdout`, verbatim.
    golden_stdout: String,
    /// Whether the record carries an expected-divergence marker of any scope.
    carries_marker: bool,
}

impl RecordedContract {
    /// Read the contract out of a validated record.
    fn of(manifest: &Manifest) -> RecordedContract {
        RecordedContract {
            shared_flags: manifest.shared_flags(),
            run_template: String::from(manifest.run_command()),
            golden_stdout: String::from(manifest.expected_stdout()),
            carries_marker: manifest.has_marker(),
        }
    }

    /// The manifest section stating the contract, ready to append.
    fn render(&self) -> String {
        let mut text = String::from("\nTHE RECORDED CONTRACT\n---------------------\n");
        text.push_str(
            "Read from the program's own expectation record, so this finding can be judged without \
             opening a second file. The record itself travels beside this manifest as \
             reproducer.expected.\n\n",
        );
        text.push_str(&format!(
            "  shared_flags        = {}\n",
            if self.shared_flags.is_empty() {
                String::from("(none declared)")
            } else {
                sanitize_text_for_report(&self.shared_flags.join(" "))
            }
        ));
        text.push_str(&format!(
            "  run_command         = {}\n",
            sanitize_text_for_report(&self.run_template)
        ));
        text.push_str(&format!(
            "  documented_by_marker = {}{}\n",
            self.carries_marker,
            if self.carries_marker {
                " (a marker is present; the note above states why it does not cover this cell)"
            } else {
                " (no expected-divergence marker of any scope, which is what makes this \
                 divergence undocumented and therefore a finding)"
            }
        ));
        text.push_str("\n  expected_stdout (oracle c's authority, verbatim):\n");
        if self.golden_stdout.is_empty() {
            text.push_str("    (the record declares empty stdout)\n");
        } else {
            for line in self.golden_stdout.lines() {
                text.push_str(&format!("    {}\n", sanitize_text_for_report(line)));
            }
        }
        text
    }
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
            contract: RecordedContract::of(manifest),
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
    ///
    /// The directory itself is not offered here: it is derived from this identifier by
    /// [`FindingId::directory`], and a write goes through [`prepare_directory`] rather than
    /// through any path a caller could have obtained in advance, so there is exactly one
    /// derivation and no second spelling that could drift from it.
    ///
    /// The oracle is deliberately not part of it — see [`FindingId::derive`]. Two findings of this
    /// cell and class observed through different oracles are one root cause, and they share the
    /// directory and are both named in its manifest.
    pub fn id(&self) -> FindingId {
        FindingId::derive(&self.key, self.class)
    }

    // There is deliberately no `directory` accessor here. It would return exactly
    // `self.id().directory()`, and a second spelling of one path is a second thing that can be
    // updated without the other: a caller reading a finding's directory from the finding and another
    // reading it from the identifier would look interchangeable right up until the derivation changed.
    // Before a write, `FindingId::directory` is the one answer; after one, `FindingArtifacts`
    // reports the directory that was actually written, which is the stronger fact and the one a report
    // row should cite.

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
/// Published so that a report can point a reader at the directory, and at the one command that
/// reproduces the divergence there, without re-deriving either from the identity. The entries are
/// recorded in the order they were written, which is also the order [`REQUIRED_ARTIFACTS`] lists,
/// and their count is what [`FindingArtifacts::describe`] reports.
///
/// Completeness is deliberately **not** asserted against this list. [`require_complete`] probes the
/// directory on disk instead, because the claim a finding makes is about the artifact a maintainer
/// will open rather than about the paths this writer believes it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingArtifacts {
    id: FindingId,
    directory: PathBuf,
    entries: Vec<PathBuf>,
    observers: Vec<Oracle>,
}

impl FindingArtifacts {
    /// The directory holding them, always a direct child of [`findings_root`].
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The command that reproduces this finding with no harness, no Cargo and no Rust toolchain.
    ///
    /// The path goes through [`posix_quote`] rather than [`shown_path`] because this is a **shell**
    /// sink, not a report sink: the returned text is meant to be pasted into a shell and must name
    /// the directory exactly, so a byte is quoted for the shell rather than replaced by a visible
    /// escape. Every consumer that renders it into a report — [`FindingArtifacts::describe`] and
    /// [`Outcome::new`] — sanitizes the line it appears in, so the two requirements do not conflict.
    pub fn reproduction_command(&self) -> String {
        format!(
            "sh {}",
            posix_quote(&self.directory.join(COMMANDS_NAME).display().to_string())
        )
    }

    /// One line naming the finding, its directory and how many files it holds.
    ///
    /// Reads through this type's own accessors rather than its fields. That is not ceremony: the
    /// accessors are the surface every caller outside this module has, and routing the renderer
    /// through them means the surface is exercised by the code that depends on it most, so an
    /// accessor that stopped agreeing with the field behind it would change this line rather than
    /// going unnoticed until a caller relied on it.
    pub fn describe(&self) -> String {
        sanitize_text_for_report(&format!(
            "{} in {} ({} files); reproduce with: {}",
            self.id,
            shown_path(&self.directory),
            self.entries.len(),
            self.reproduction_command()
        ))
    }
    /// The finding these artifacts belong to.
    pub fn id(&self) -> &FindingId {
        &self.id
    }

    /// Every file written, in write order.
    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
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
    if component == RUN_OWNER_ENTRY {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the artifact name {shown:?} is the finding directory's own run-ownership stamp; no \
                 artifact may be written to that name, because a directory able to rewrite its own \
                 claim could hand itself to a concurrent run halfway through publication"
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

/// Claim the generated-findings root for this run, retiring an earlier run's finding directories.
///
/// The driver calls this once at start-up, so a conflict with a concurrent run is reported before the
/// first cell is compiled rather than at the moment a divergence needs filing. [`prepare_directory`]
/// calls it again on the path that must not depend on the driver having asked: the work is remembered,
/// so the second call costs nothing and the guarantee is structural rather than a convention.
///
/// # Errors
///
/// Returns the same explanatory failure [`prepare_directory`] would have returned later — the root
/// could not be established or verified, another live run owns it, or an earlier run's directories
/// could not be retired.
pub fn prepare_namespace() -> HarnessResult<()> {
    ensure_findings_namespace("preparing the generated-findings directory")
}

/// Establish this run's ownership of the findings root, exactly once per process.
///
/// [`OnceLock::get_or_init`] blocks every other feature-area thread until the first one has finished,
/// which is the whole of the coordination needed, and the outcome is remembered so a failure is
/// reported identically to every caller rather than retried once per finding.
fn ensure_findings_namespace(context: &str) -> HarnessResult<()> {
    static PREPARED: OnceLock<Result<(), String>> = OnceLock::new();
    match PREPARED
        .get_or_init(|| prepare_findings_namespace().map_err(|error| String::from(error.cause())))
    {
        Ok(()) => Ok(()),
        Err(cause) => Err(HarnessError::new(
            String::from(context),
            format!("the generated-findings directory for this run could not be prepared: {cause}"),
        )),
    }
}

/// Retire the previous run's finding directories and claim the root for this one.
///
/// # Why the root is retired at all
///
/// A finding directory is named from the divergence itself, so it is rewritten in place when the same
/// divergence recurs — that is the documented idempotence [`prepare_directory`] relies on. What that
/// leaves behind is the opposite case: a directory for a divergence this run *does not* reproduce.
/// Nothing in this run's rows refers to it, so no report mentions it, and it sits in the generated set
/// looking exactly like a current deliverable to the next maintainer who opens that directory. Since
/// the whole value of a finding is that a reader can trust it describes the run that produced it, the
/// previous run's set is retired whole rather than left to be told apart by hand.
///
/// # Why the order is exactly this order
///
/// A **live foreign owner is refused before anything is removed**, so a run that finds another run
/// still filing findings here destroys nothing — the removal is the destructive step, and once it has
/// happened no later check can undo it. The retirement then happens, entry by entry and following no
/// symbolic link, so the directory itself stays valid for a concurrent reader's handle. Only then is
/// this run's stamp written, so a process that dies midway leaves a directory the next run will
/// retire again rather than one it believes is owned.
///
/// This is deliberately *not* done by the function that creates the three artifact roots. That
/// function runs before any ownership question has been asked — it is what creates the directory the
/// stamp would live in — so a removal there could not tell one run's findings from another's, and
/// would delete a live run's evidence. Only a *live* foreign stamp refuses: a stamp from a run that
/// has exited is stale and is replaced, which is what keeps a sequential re-run working.
fn prepare_findings_namespace() -> HarnessResult<()> {
    let context = "preparing the generated-findings directory for this run";
    let root = findings_root();
    create_directory_chain_below(context, &root, &root)?;

    if let Some((run, pid)) = live_foreign_owner_identity(&root) {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "{} is already owned by run {} (process {}), which is still running. A finding \
                 directory is named from the divergence itself so that a register row leads straight \
                 to it, which means two concurrent runs address the same directories; retiring them \
                 here would destroy the evidence that run is still writing, and leaving them would \
                 put its findings in this run's generated set. Let that run finish, or set \
                 CARGO_TARGET_DIR to a different build directory for this one",
                shown_path(&root),
                sanitize_text_for_report(&run),
                pid
            ),
        ));
    }

    retire_directory_contents(context, &root)?;
    claim_ownership(context, &root)
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
                    shown_path(candidate),
                    shown_path(&root)
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
                        shown_path(candidate),
                        shown_path(&root)
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
                shown_path(candidate),
                shown_path(&root)
            ),
        ));
    }
    Ok(())
}

/// Resolve one entry of a finding directory from its components, guarding the result.
///
/// Every path this module writes passes through here, which is what makes the write-path claim
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

/// Create `path`, and every missing directory between [`findings_root`] and it, one level at a time.
///
/// Deliberately **not** `fs::create_dir_all`. That call follows a symbolic link at any level and
/// reports success, so one planted link anywhere on the way down would place a whole finding — the
/// reproducer, the captured evidence from every cell, the reproduction commands — wherever the link
/// pointed, while every report row still named the path that was asked for. Because a finding
/// directory's name is derived deterministically from the divergence, that name is predictable
/// before the run that will use it, which is exactly the precondition such a link needs.
///
/// The walk itself lives in [`create_directory_chain_below`], which creates each level below the
/// verified root and then requires it to be a real directory, refusing **at** the offending level
/// and creating nothing beyond it. It is shared rather than written again here for the same reason
/// as every other filesystem guard in this suite: `report.rs` publishes into deterministically
/// named directories beneath a root too, and a second copy of the walk is a second chance to get
/// one level wrong. What this function contributes is the part specific to a finding — that the
/// path lies beneath the generated-findings root and nowhere else.
///
/// An already-present level is not an error. Two areas can record findings concurrently, and the
/// idempotent re-run documented on [`prepare_directory`] rewrites a directory it created before.
fn create_directory(context: &str, path: &Path) -> HarnessResult<()> {
    require_beneath_findings_root(context, path)?;
    create_directory_chain_below(context, &findings_root(), path)
}

/// Write bytes to `path`, replacing whatever was there through a guarded removal and a fresh create.
///
/// Bytes rather than text for the captured streams, which are compared byte for byte and must be
/// stored exactly as produced: no line-ending normalization, and no substitution for a byte that is
/// not valid text.
///
/// Four guards run before a byte is written, and each closes a different way a predictable
/// artifact name could be turned into a write somewhere else:
///
/// - the path must lie lexically beneath [`findings_root`], which is what keeps publication out of
///   the corpus, out of the compiler's source and out of the committed finding set;
/// - every directory from that root down to the parent must be a real directory rather than a link,
///   established by [`require_directory_chain_below`];
/// - the leaf is refused outright by [`require_replaceable`] if anything other than a regular file
///   is already at it, because this module never puts anything else there and repairing it silently
///   would discard the only evidence that something else did;
/// - the bytes are then staged into a fresh temporary created exclusively — `O_CREAT | O_EXCL`
///   refuses a link, a device node, a FIFO and a hard link into another file, even if one appears
///   between the check and the write — and renamed over the destination.
///
/// A plain truncating write is the operation being replaced, and it is the one a planted link
/// exploits: it would have opened the link's target and written a finding's evidence through it.
/// Publishing by rename rather than by removing the destination first is what additionally keeps the
/// previous evidence in place when a write fails, and keeps a concurrently reading maintainer from
/// finding the file briefly absent.
fn write_bytes(context: &str, path: &Path, bytes: &[u8]) -> HarnessResult<()> {
    require_beneath_findings_root(context, path)?;
    let parent = path.parent().ok_or_else(|| {
        HarnessError::new(
            String::from(context),
            format!(
                "{} has no parent directory, so the chain of directories above it cannot be \
                 verified and nothing is written",
                shown_path(path)
            ),
        )
    })?;
    require_directory_chain_below(context, &findings_root(), parent)?;
    require_replaceable(context, path, Replaceable::RegularFile)?;
    publish_bytes_no_follow(context, path, bytes).map_err(|error| {
        HarnessError::new(
            String::from(error.context()),
            format!(
                "{}; a finding without its evidence is not a deliverable, so this is reported \
                 rather than skipped",
                error.cause()
            ),
        )
    })
}

/// Publish text at `path` as UTF-8, through **exactly** the guards [`write_bytes`] applies.
///
/// One funnel rather than two, and that is the whole point of this function existing at all. It used
/// to call the shared publisher directly, which meant three of the four guards above — the lexical
/// containment beneath [`findings_root`], the resolved directory chain down to the parent, and the
/// refusal of a leaf that is not already a regular file — applied to the captured streams and *not* to
/// the manifest, the reproduction commands, the environment fingerprint or the diff. Those are the
/// artifacts a maintainer actually reads and executes, so the weaker path was guarding the less
/// valuable half of the deliverable. A sibling writer that quietly skips its siblings' checks is also
/// exactly the shape of defect that survives maintenance, because both names read as equivalent at
/// every call site.
///
/// UTF-8 text is bytes here, so the conversion is the whole of the difference between the two.
fn write_text(context: &str, path: &Path, text: &str) -> HarnessResult<()> {
    write_bytes(context, path, text.as_bytes())
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
/// by a derived identifier. Nothing else is removed, and no parent is ever touched. It goes through
/// the shared [`remove_entry`], so an `outputs` that is a symbolic link is unlinked as a link rather
/// than recursed into — a recursive removal aimed at a link is how a suite deletes somebody else's
/// tree — and a stale plain file at that name is cleared instead of blocking the run.
///
/// # The two containment checks, and why one is not enough
///
/// [`require_beneath_findings_root`] decides the question **lexically**, before anything exists,
/// which is the only way to vet a path that is about to be created. [`create_directory`] then decides
/// it again **on the filesystem**, level by level as it creates, which is the only way to detect that
/// a name was a symbolic link out of the build tree all along. A lexical check alone accepts a
/// spelling whose resolution is elsewhere; a resolved check alone cannot run before the entry exists.
/// Both are applied, in that order.
///
/// # Why a foreign occupant is refused
///
/// A directory that already holds a *different* finding's manifest is refused rather than written
/// over. Under the identifier scheme above that cannot arise from two distinct divergences, so it
/// means either that the scheme has changed or that the directory was created outside the suite —
/// and in both cases overwriting would destroy another divergence's only copy of its evidence.
///
/// # Why ownership is stamped
///
/// Idempotence across *sequential* runs is desirable; the same behaviour between two *concurrent*
/// runs sharing one build directory is not. Both would derive this identical directory, and the
/// second would purge `outputs/` while the first was still writing captures into it, leaving a
/// finding whose evidence came from two different runs — the one failure mode a reader of a finding
/// cannot detect, because half-and-half evidence looks exactly like whole evidence.
///
/// So a live foreign owner is refused **before** anything is removed, and this run's identity is
/// stamped afterwards, both through the same mechanism the per-cell workspaces use. Only a *live*
/// foreign stamp refuses: a stamp from a run that has exited is stale and is replaced, which is what
/// keeps the sequential re-run working exactly as documented above.
///
/// The stamp is bookkeeping rather than evidence. [`REQUIRED_ARTIFACTS`] does not list it,
/// [`require_complete`] does not look for it, [`validate_component`] refuses it as an artifact name
/// so nothing else can write to it, and it is not among the entries a [`FindingArtifacts`] reports.
fn prepare_directory(id: &FindingId, fresh: bool) -> HarnessResult<(PathBuf, PathBuf)> {
    let context = format!("preparing the artifact directory for finding {id}");
    // The root is claimed and retired before the first finding directory is created inside it, so a
    // previous run's set cannot survive beside this one's. Remembered per process: this is a no-op
    // after the driver's own call, and correct even if the driver never made one.
    ensure_findings_namespace(&context)?;
    let directory = id.directory();
    require_beneath_findings_root(&context, &directory)?;
    require_no_foreign_occupant(&context, id, &directory)?;

    if let Some((run, pid)) = live_foreign_owner_identity(&directory) {
        return Err(HarnessError::new(
            context,
            format!(
                "{} is already owned by run {} (process {}), which is still running. A finding \
                 directory is named from the divergence itself so that a register row leads \
                 straight to it, which means two concurrent runs recording the same divergence \
                 address the same directory; publishing here would purge the captures that run is \
                 still writing and leave a finding whose evidence came from two runs at once. Let \
                 that run finish, or set CARGO_TARGET_DIR to a different build directory for this \
                 one",
                shown_path(&directory),
                sanitize_text_for_report(&run),
                pid
            ),
        ));
    }

    create_directory(&context, &directory)?;

    let outputs = guarded_path(&context, &directory, &[OUTPUTS_DIR_NAME])?;
    // Replaced for the run's **first** contribution to this directory, added to for every one after.
    //
    // Both halves of that are load-bearing. Replacing on the first contribution is what keeps a
    // previous run's captures from surviving beside this run's, and `outputs/` is a set whose
    // membership can shrink, so a stale entry would look like evidence of the current divergence.
    // *Not* replacing on a later contribution is what makes one directory per root cause work at all:
    // the oracles that observed this cell each bring their own authority capture, and purging here
    // would leave the directory holding only whichever oracle happened to file last.
    if fresh {
        require_replaceable(&context, &outputs, Replaceable::Directory)?;
        remove_entry(&context, &outputs).map_err(|error| {
            HarnessError::new(
                error.context().to_string(),
                format!(
                    "{}; a previous run's captures are replaced rather than merged so that a stale \
                     capture cannot be mistaken for evidence of the current divergence",
                    error.cause()
                ),
            )
        })?;
    }
    create_directory(&context, &outputs)?;

    claim_ownership(&context, &directory)?;
    Ok((directory, outputs))
}

/// Refuse to write into a directory a *different* finding already owns.
///
/// [`FindingId::derive`] is injective, so under normal operation the only finding that can name this
/// directory is the one being written, and re-running the suite legitimately rewrites its own
/// directory in place. This check is the belt for the case that injectivity argument does not
/// cover: an identifier scheme changed by maintenance, a directory created by hand, or a stale
/// directory left by an older build of the suite. In any of those, the occupant's evidence belongs
/// to a divergence that is not this one, and overwriting it would destroy a deliverable — the
/// single failure mode the findings register exists to prevent.
///
/// The occupant's identity is read from the first line of its own [`MANIFEST_NAME`] that declares
/// `finding_id`, which [`render_manifest`] writes as the first identity line of every manifest. A
/// directory with no manifest, or a manifest that declares nothing, is treated as this finding's own
/// partial output from an interrupted run and is rewritten: refusing there would leave a run
/// unable to make progress after a crash, and there is no other finding's evidence to protect.
///
/// # The inspection is itself guarded, because it happens first
///
/// This check runs before the directory has been created under verification, which makes *it* the
/// first thing to touch a predictable path — so it cannot borrow the later steps' guarantees. Three
/// things follow, in this order: the resolved chain from the findings root down is verified **before**
/// a byte is read, so a link planted at any level is refused rather than followed; the manifest path
/// is built through the shared containment helper rather than joined by hand; and the read is
/// **bounded** and refuses a link or a special file, so an entry at that name can neither decide how
/// much memory this process uses nor block the run without end.
///
/// # Errors
///
/// Returns an explanatory failure naming both identifiers when the occupant declares a different
/// one, and one naming the offending level or entry when the chain or the manifest cannot be trusted.
/// The caller turns either into a `FAIL` verdict rather than a `FINDING`, because a divergence whose
/// evidence could not be filed has not been delivered.
fn require_no_foreign_occupant(
    context: &str,
    id: &FindingId,
    directory: &Path,
) -> HarnessResult<()> {
    // A directory that is not there has no occupant, and this is the ordinary first-write case. It is
    // asked before the chain is verified because the chain verification requires every level to
    // already exist, which on a first write none of them do.
    if fs::symlink_metadata(directory).is_err() {
        return Ok(());
    }
    // Verified **before** a byte is read, not after. The lexical check the caller performed says the
    // spelling lies beneath the findings root; it says nothing about what the names resolve to, so a
    // link planted at any level would have the read below take its bytes from wherever that link
    // pointed. A finding directory's name is derived from the divergence, so it is predictable before
    // the run that will write it — which is exactly the precondition such a link needs.
    require_directory_chain_below(context, &findings_root(), directory)?;
    let manifest = guarded_path(context, directory, &[MANIFEST_NAME])?;
    if fs::symlink_metadata(&manifest).is_err() {
        return Ok(());
    }
    // Bounded, and refusing a link or a special file. The previous unbounded read let an entry at a
    // predictable name decide how much memory this process used, and let a link at that name be
    // followed to any file on the machine — a directory or a FIFO there would have blocked without end.
    let bytes = read_file_bounded(context, &manifest, MAX_INSPECTED_FILE_BYTES)?;
    let text = String::from_utf8_lossy(&bytes);
    let Some(occupant) = declared_finding_id(&text) else {
        return Ok(());
    };
    if occupant == id.as_str() {
        return Ok(());
    }
    Err(HarnessError::new(
        String::from(context),
        format!(
            "{} already holds the artifacts of finding {}, which is a different finding from {}. \
             Writing here would overwrite another divergence's only copy of its reproducer, its \
             captured outputs and its reproduction commands, so the write is refused and this cell \
             is reported as a failure rather than as a filed finding. Two findings can only name \
             one directory if the identifier scheme has changed or the directory was created \
             outside the suite: move or delete {} and re-run, and if the identifiers genuinely \
             collide, treat that as a defect in the derivation rather than as something to \
             overwrite",
            directory.display(),
            sanitize_text_for_report(&occupant),
            id,
            directory.display()
        ),
    ))
}

/// The identifier a manifest declares for itself, if it declares one.
///
/// Reads the first `finding_id = ...` line, which is the first identity line every manifest carries.
/// Kept deliberately tolerant — leading whitespace and any run of spaces around the separator are
/// accepted — because this parses an artifact written by a possibly older build of the suite, and
/// the only question being asked is whose evidence is in this directory.
fn declared_finding_id(manifest_text: &str) -> Option<String> {
    for line in manifest_text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "finding_id" {
            let declared = value.trim();
            if !declared.is_empty() {
                return Some(String::from(declared));
            }
        }
    }
    None
}

/// Shell variable holding the path to the source program, so every command line names one file.
const VAR_SOURCE: &str = "SRC";

/// Shell variable holding the scratch directory a reproduction writes into.
const VAR_WORK: &str = "WORK";

/// Shell variable a reader sets to keep the scratch directory the script created for itself.
///
/// The cleanup trap is unconditional otherwise, so this is the documented way to inspect the
/// captured streams after the script has finished rather than only while it is running.
const VAR_KEEP: &str = "REPRO_KEEP";

/// Shell variable the script sets for itself to record that it, and not its caller, owns the scratch
/// directory — and therefore that the cleanup trap may remove it.
///
/// Kept distinct from [`VAR_WORK`] being set, because the two answer different questions: `WORK`
/// says *where* the scratch is, this says *whose it is*. A directory the caller supplied is never
/// removed however the script exits.
const VAR_WORK_OWNED: &str = "WORK_OWNED";

/// The shell function the script defines to refuse a scratch path that is already taken.
///
/// Named rather than repeated inline because it guards every one of the four or five redirection
/// targets each capture block writes, and a guard that is written out five times per capture is a
/// guard that will eventually be written out four times.
const SH_REFUSE_EXISTING: &str = "refuse_existing";

/// The shell function the script defines to remove a scratch directory it created.
///
/// Named rather than repeated inline because four traps share it, and because separating the
/// *removal* from the *disposition of a signal* is what makes the signal handling correct. A single
/// `trap '<remove>' EXIT HUP INT TERM` looks tidier and is wrong: on a signal the handler runs,
/// returns, and the shell **resumes at the next command** with the scratch directory already gone,
/// so every later redirection writes into a path that no longer exists and the script carries on
/// after the reader asked it to stop. Removal therefore sits on `EXIT`, and each signal handler
/// removes, restores the signal's default disposition and re-raises it at this shell, so the script
/// dies from the signal and its caller sees the conventional `128 + signal` status.
const SH_CLEANUP: &str = "repro_cleanup";

/// Shell variable holding the directory the script itself lives in, so the reproducer beside it can
/// be found however the script was invoked.
const VAR_FINDING_DIR: &str = "FINDING_DIR";

/// Shell variable holding the compiler under test.
const VAR_BCC: &str = "BCC";

/// Shell variable holding the timeout utility every reproduced execution is bounded with.
///
/// # Why a reproduction must be bounded
///
/// One of the divergence classes this suite records **is** a timeout: a program that finishes
/// promptly under one compiler and never finishes under another. An unbounded reproduction of that
/// finding hangs the reader's shell, and the artifact whose purpose is to make the divergence easy to
/// see instead makes it the reader's problem to notice and interrupt. A run that is bounded reports
/// the timeout as a result.
///
/// Left empty when the environment had no such utility, in which case the script says so and runs
/// unbounded rather than silently declining to reproduce.
const VAR_TIMEOUT: &str = "TIMEOUT";

/// Shell variable holding the per-execution budget, in whole seconds, that the recorded run used.
///
/// This is the number [`SH_BOUNDED`] polls to, so it is the script's authoritative bound and is
/// declared for every finding — including one whose compiler refused the program and which therefore
/// has no execution at all.
const VAR_BUDGET: &str = "BUDGET_SECS";

/// Shell variable holding the seconds the outer net sits behind the watchdog, when there is one.
///
/// Mirrors the harness's own margin, so a reproduction that has a `timeout` utility places it exactly
/// where the run did: at the budget plus this, behind the watchdog that actually classifies. A reader
/// who wants the two levels closer together or further apart changes one number.
const VAR_OUTER_MARGIN: &str = "OUTER_MARGIN_SECS";

/// Shell variable holding the search path every reproduced invocation is given.
///
/// Declared separately from the other assignments in the isolation function so a reader on another
/// machine can change it in one place — it is the one value in that environment that is a property of
/// the machine the run happened on rather than of the suite.
const VAR_CHILD_PATH: &str = "CHILD_PATH";

/// Shell variable holding the search path the **script itself** runs with.
///
/// Kept distinct from [`VAR_CHILD_PATH`] because the two are adjusted for different reasons: this one
/// decides which `mktemp`, `rm`, `cmp` and `env` the script runs, and the other decides what a
/// compiler driver and an emulator can reach. A reader relocating their coreutils changes this one; a
/// reader relocating a toolchain changes the other.
const VAR_SCRIPT_PATH: &str = "SCRIPT_PATH";

/// Search path the script gives itself when the run recorded none.
///
/// Reached only when the run's own path could not be computed — an environment with no usable `PATH`
/// at all. Two absolute system directories rather than an empty value, because an empty search path
/// makes every helper unfindable and turns a safety measure into a script that cannot run; and only
/// those two, because a fallback should name the fewest places a POSIX utility is expected to be.
const FALLBACK_SCRIPT_PATH: &str = "/usr/bin:/bin";

/// Basename prefix of the private scratch directory the script creates inside a caller-supplied one.
///
/// A distinctive prefix rather than a bare template, so that a reader inspecting a retained directory
/// can see at a glance which entry the replay created.
const SCRATCH_TEMPLATE: &str = "bcc-repro.XXXXXX";

/// Shell variable holding the directory a caller supplied, once the private one is made inside it.
///
/// Retained so the commentary and the closing report can name both — the directory the reader chose,
/// which the script neither created nor removes, and the private one inside it, which it did both.
const VAR_WORK_PARENT: &str = "WORK_PARENT";

/// The phrase the scratch commentary uses for how a private directory is obtained.
///
/// A constant so the explanation in the comment and the diagnostic printed when the utility is absent
/// cannot describe two different mechanisms.
const SH_MKTEMP_MSG: &str = "created with `mktemp -d` under a 077 umask";

/// Every helper name the generated script may reach, in roughly the order it first uses them.
///
/// Emitted into a single `unset -f` so an inherited exported shell function cannot stand in for one of
/// them. `command` is included deliberately: unsetting a *function* by that name leaves the builtin
/// intact, and the builtin is what the rest of the script relies on to bypass function lookup.
const SH_HELPERS: &str = "command printf mktemp rm env cmp cat diff kill test";

/// The shell function through which every reproduced invocation is run.
///
/// # Why an invocation is not simply run
///
/// The harness does not spawn a compiler or a program with the environment it inherited: it clears the
/// environment and installs a small fixed set. Every one of those variables changes an observable
/// result. `LC_ALL=C` fixes number and message formatting, and a reader whose locale prints a decimal
/// comma would see a stdout difference that is theirs and not the compiler's. `ASAN_OPTIONS` and
/// `UBSAN_OPTIONS` are forced to their strictest values, and a reader with
/// `ASAN_OPTIONS=halt_on_error=0` exported would watch an instrumented artefact pass where the run
/// recorded a diagnostic. `TERM=dumb` stops a tool emitting colour escapes into a stream that is
/// compared byte for byte. `PATH` is the vetted list, so a compiler driver finds the same `cc1` and
/// `as` the run used. `HOME` and the three temporary-directory names point at the scratch directory,
/// so nothing is written into the reader's own home.
///
/// A script that reproduced the command line but not the environment would therefore be reproducing a
/// *different* invocation, and the two ways that can go wrong are both bad: it can fail to show the
/// divergence, which reads as a finding that was never real, or it can show a different one, which
/// sends a reader after a defect that is not there.
///
/// It is a function rather than a variable holding a command prefix because a prefix has to be left
/// unquoted to be split into words, and a search path containing a space in a directory name would
/// then be split into two arguments. `"$@"` passes the invocation through element for element.
const SH_ISOLATED: &str = "isolated";

/// Exit status a `timeout` utility reports when it terminates the command it was bounding.
///
/// **Never interpreted**, here or in the emitted script, and named only so that both can say why.
/// The corpus contract admits any status from 0 to [`MAX_CONTRACT_EXIT_CODE`], so this value lies
/// *inside* the range a program may legitimately return, and a script that read it as expiry would
/// report a program which deliberately returned 124 as a program that never finished. The emitted
/// script therefore classifies a termination by [whether the script itself did the
/// killing](SH_BOUNDED) and records that classification out of band, exactly as the harness does —
/// so a 124 from a command that returned promptly is reported as `exited 124`, which is what it is.
const TIMEOUT_UTILITY_STATUS: i32 = 124;

/// Offset a POSIX shell adds to a signal number when reporting a command killed by a signal.
const SHELL_SIGNAL_STATUS_BASE: i32 = 128;

/// The shell function through which every reproduced invocation is bounded, run and classified.
///
/// # Why a reproduction has its own watchdog rather than a `timeout` prefix
///
/// The harness does not decide a timeout by reading a status: it decides one when, and only when, it
/// killed the child itself, because expiry statuses are not distinguishable from statuses a program
/// may legitimately return. A reproduction script that wrapped each command in the utility and read
/// `$?` would throw that away and reintroduce the exact ambiguity — and where the reader had no
/// utility at all it would run unbounded, which turns the one divergence class that *is* a
/// non-terminating program into a script that hangs instead of reporting it.
///
/// So this function is the script's watchdog. It backgrounds the invocation, polls for the recorded
/// budget, kills what is still running, and then reports [`TERMINATION_TIMEOUT`] **because it did the
/// killing** rather than because of any number it read. The bound therefore exists on every machine,
/// with or without a utility, and the classification is out of band on all of them.
///
/// # How the outer net is preserved
///
/// Where the run's own launch was wrapped, the utility is still applied — at the budget plus
/// [`VAR_OUTER_MARGIN`], behind this watchdog, which is precisely where the harness puts it and
/// precisely why it never fires in a healthy reproduction. Its status is not read, so its presence
/// cannot change a classification; it is there for the case this shell's own polling is starved.
const SH_BOUNDED: &str = "bounded_run";

/// Classification written out of band for a command that chose its own exit status.
const TERMINATION_EXITED: &str = "exited";

/// Classification written out of band for a command a signal ended.
///
/// A shell folds a signal death into `128 + signal`, and a program that itself exited with such a
/// number is indistinguishable from one that was signalled — a property of `wait`, not of this
/// suite. The harness compares raw wait status and so keeps the two apart; the script says which it
/// saw in the terms a shell can actually observe, and the recorded expectation beside it names what
/// the harness observed.
const TERMINATION_SIGNALLED: &str = "signalled";

/// Classification written out of band for a command the script's own watchdog terminated.
///
/// This is the sentinel the ambiguity argument on [`TIMEOUT_UTILITY_STATUS`] turns on: it is written
/// on the strength of the script having performed the kill, never on the strength of a status, so no
/// exit code a program can return will produce it and no expiry can fail to.
const TERMINATION_TIMEOUT: &str = "timeout";

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
/// referenced line reconstructs the recorded argument vector element for element — the executed line
/// is exact. The same line is also rendered as a comment above each command so the indirection hides
/// nothing, and that rendering is a *safe* one rather than a literal echo: [`comment`] passes it
/// through [`sanitize_text_for_report`], which escapes anything that could break out of a comment or
/// repaint a terminal. On every ordinary path the two read identically; where a path or a diagnostic
/// contains such a byte, the comment shows it encoded while the executed argument still carries the
/// true value.
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
        // The timeout utility and the budget, taken from the vector the harness actually spawned
        // rather than from the capability record, for the same reason every other tool path here is:
        // the script must state what ran, not what was discovered.
        //
        // A wrapped launch is `<timeout> <secs> <program> ...`, so the utility is its first element.
        // When the launch was not wrapped the harness enforced the budget with its own watchdog and no
        // utility appears in the vector; the budget is still declared, so a reader whose machine does
        // have the utility can bound the reproduction with the same number.
        //
        // Declared from the **build** as well as the execution, and that is the load-bearing half. A
        // finding whose compiler refused the program has no execution at all, so a collection that
        // looked only at executions would declare no budget for it, leaving the one class of finding
        // most dangerous to reproduce — a compiler that never returns — bounded by the script's
        // default rather than by the number the run actually used.
        //
        // The budget is unconditional because `bounded_run` polls it. The utility is not: it is the
        // outer net, it is declared only when the run itself used one, and its absence changes where
        // the net sits rather than whether there is a bound at all.
        if let Some(compile) = capture.compile() {
            if let Some(tool) = compile.timeout_tool() {
                variables.declare(VAR_TIMEOUT, tool);
            }
            variables.declare(VAR_BUDGET, &compile.budget().as_secs().to_string());
        }
        if let Some(run) = capture.run() {
            if run.launch_was_wrapped() {
                if let Some(tool) = run.launch_argv().first() {
                    variables.declare(VAR_TIMEOUT, tool);
                }
            }
            variables.declare(VAR_BUDGET, &run.budget().as_secs().to_string());
        }
    }
    // The margin the harness places between its own watchdog and the outer net, so a reproduction
    // that has a utility puts it exactly where the run did. Declared for every finding rather than
    // only for a wrapped one: it costs one line, and a reader who supplies their own `TIMEOUT` on a
    // finding recorded without one then gets the same two-level arrangement instead of the default.
    variables.declare(
        VAR_OUTER_MARGIN,
        &TIMEOUT_UTILITY_OUTER_MARGIN.as_secs().to_string(),
    );
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
/// reconstructs that vector element for element. Above each one, the same line is repeated as a
/// comment so the variable references cannot hide what ran — as a sanitized rendering, since a
/// comment is text a reader's terminal will display: see [`comment`] for what that escapes and why
/// the executed line beside it is the unescaped authority.
///
/// That vector is the program's own expectation record already rendered: the command templates in
/// the record are turned into arguments by `manifest`'s renderers, which `compile` and `execute`
/// then spawn and retain. Taking the retained vector rather than rendering the template a second
/// time is what makes "exact" literally true — a second rendering could only ever be *equal* to
/// what ran, whereas this *is* what ran, and a substitution that differed between the two would be
/// invisible in a re-render and obvious here.
///
/// # What makes it reproduce the oracle, and not half of it
///
/// Reconstructing the command lines is necessary and not sufficient. The oracles judge a cell on
/// **stdout bytes and termination together**, so a script that reproduced the commands and then
/// compared only stdout would report "did not reproduce" for every exit-code divergence — the one
/// class where both sides' stdout is identical by definition — and would tell a maintainer their
/// finding had gone away when nothing had changed at all. Three things follow, and the script does all
/// three:
///
/// - **Each side's termination is recorded and compared.** Every executed side writes its status to a
///   file, the recorded authority's prescribed status is materialized the same way, and the closing
///   block compares the two. Both halves of the comparison are reported separately, because which one
///   differs is itself the diagnosis.
/// - **The recorded termination travels with each line.** A shell reports an ordinary exit, a signal
///   death and a timeout as one number, folding the second into `128 + signal`; the suite keeps them
///   apart by raw wait status and cannot hand that distinction to `sh`. So each side states how it
///   terminated in the suite's own terms beside the number a reproduction should see — see
///   [`Capture::shell_expectation`].
/// - **Every build and every execution is bounded.** A timeout *is* one of the divergence classes, so
///   an unbounded script reproduces that class by hanging, which converts the deliverable into a trap.
///   The bound is applied through a runtime test on [`VAR_TIMEOUT`], so one script works with or
///   without the utility and says which it is doing.
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
        "Each command below is the line this run performed. The comment above it shows the same \
         line in a form that is safe to display, so a byte that could not be shown literally appears \
         encoded there while the command itself carries the true value. Only the tool paths are \
         lifted into the variables in the preamble, so the script can be \
         adjusted on another machine without editing every line. Every build carries -static and one \
         optimization level, and nothing else beyond the output path: that is the whole set of flags \
         the suite verified both compilers honour with the same meaning. The compiler under test \
         selects its backend with a target argument, which it alone accepts; the reference compiler \
         selects a target by being a different driver binary.",
    ));
    script.push_str("#\n");
    script.push_str(&comment(&format!(
        "Every command runs through the {SH_ISOLATED} function below, which reproduces the \
         environment and the working directory the run used — a cleared environment with a fixed C \
         locale, fixed sanitizer options, the vetted search path, and the scratch directory as HOME \
         and TMPDIR. A command line alone is not the invocation: an exported LD_PRELOAD, a locale \
         that prints a decimal comma, or a relaxed ASAN_OPTIONS each change the result, so the \
         environment is reproduced rather than assumed."
    )));
    script.push_str("#\n");
    script.push_str(&comment(&format!(
        "Scratch output goes to ${VAR_WORK}. By default the script creates that directory for itself \
         with `mktemp -d` under a 077 umask — an unpredictable name, created exclusively, readable \
         only by you — and removes it again however the script exits. Set {VAR_KEEP}=1 to keep it, \
         or set {VAR_WORK} to an existing directory of your own, which the script then writes into \
         but never creates and never removes. Nothing this script names is written outside it; the tools \
         it runs keep their own temporaries wherever they normally do."
    )));
    script.push_str(&comment(&format!(
        "A fixed scratch name such as ./repro-work is deliberately not used. Its path would be \
         predictable before the script ran, so anything able to create that one name — or a single \
         entry inside it — could redirect a build output or a captured stream through a symbolic \
         link and have this script overwrite a file elsewhere on your machine while still reporting \
         success. Every scratch path is therefore refused rather than reused if something is already \
         at it, by {SH_REFUSE_EXISTING} below."
    )));
    script.push_str("\nset -eu\n");
    script.push_str(&render_script_own_environment());

    script.push_str(&comment(
        "--- tools this run used (adjust the paths if yours differ) ------------------",
    ));
    script.push_str(&comment(
        "These correspond to the suite's own override variables, each with a BCC_ prefix: BCC_BIN, \
         BCC_REF_CC, BCC_REF_CC_<ARCH> and BCC_QEMU_<ARCH>.",
    ));
    script.push_str(&comment(&format!(
        "{VAR_BUDGET} bounds every build and every execution below, because one of the divergence \
         classes this suite records is a program that never finishes. The bound is enforced by \
         {SH_BOUNDED}, this script's own watchdog, so it applies whether or not your machine has a \
         timeout utility: no invocation here runs unbounded. {VAR_TIMEOUT}, when the preamble \
         declares one, is the outer net the run itself used and sits at \
         {VAR_BUDGET} + {VAR_OUTER_MARGIN} behind that watchdog; its exit status is never read."
    )));
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
    // The directory is computed with shell parameter expansion rather than by running `dirname`,
    // because this is the first thing the script does with a path and it must not depend on an
    // external program at all: a planted `dirname` earlier on the reader's search path would
    // otherwise choose where every input below is read from. `${0%/*}` strips the last component,
    // and the `case` covers the one shape it cannot handle — a bare name with no slash, which is
    // the current directory.
    script.push_str(&format!(
        "case \"$0\" in\n    */*) {VAR_FINDING_DIR}=${{0%/*}} ;;\n    *) {VAR_FINDING_DIR}=. ;;\nesac\n"
    ));
    script.push_str(&format!(
        "{VAR_FINDING_DIR}=$(CDPATH= cd -- \"${VAR_FINDING_DIR}\" && pwd) || exit 1\n"
    ));
    script.push_str(&format!(
        "{VAR_SOURCE}=\"${VAR_FINDING_DIR}/{REPRODUCER_SOURCE_NAME}\"\n"
    ));
    script.push_str(&render_scratch_setup());

    // After the scratch setup, because the isolation points the private-directory variables at
    // `$WORK` and therefore needs it to exist and to be known; and before the first capture block,
    // because every one of them runs through it.
    script.push('\n');
    script.push_str(&render_isolated_environment());
    script.push_str(&render_working_directory());
    // After the isolation, which it calls, and before the first capture block, every invocation of
    // which goes through it.
    script.push('\n');
    script.push_str(&render_bounded_run_function());

    for capture in &captures {
        script.push_str(&render_capture_block(capture, &variables));
    }
    script.push_str(&render_comparison_block(finding, &captures));
    Ok(script)
}

/// Render the scratch-directory setup a reproduction script performs before it builds anything.
///
/// # Why this is not `mkdir -p ./repro-work`
///
/// A reproduction script runs on a maintainer's own machine, from a directory this suite knows
/// nothing about, and it is the one artifact of a finding that a reader is explicitly invited to
/// execute. A fixed relative scratch name is predictable **before** the script runs, and every path
/// the script writes is then predictable too: the build outputs, the captured stdout of each side,
/// the captured stderr. Anything able to create one of those names first — a symbolic link pointing
/// at a file elsewhere — would have the script's own redirections truncate that file, because a
/// shell `>` follows a link and creates through it. The script would report success throughout, and
/// the damage would be attributed to whoever ran the reproduction rather than to the name that was
/// planted.
///
/// So three properties are established, in this order:
///
/// 1. **An unpredictable, private, exclusively created root.** `mktemp -d` creates a directory whose
///    name is not known in advance and, run under `umask 077`, one that only its owner can enter.
///    The umask is set inside the command substitution so it applies to the creation and is gone
///    again immediately, rather than silently changing the modes of everything the compilers write
///    afterwards.
/// 2. **A caller-supplied root is used but never created.** If `WORK` is already set, the script
///    requires it to be an existing real directory and refuses a symbolic link — the one case where
///    `mkdir -p` would have quietly accepted a link and written through it — and it is never removed,
///    because the script did not create it.
/// 3. **Every leaf is refused rather than reused.** `refuse_existing` is called on each redirection
///    target before it is written, testing `-e` *and* `-L` so that a dangling link, which `-e`
///    reports as absent, is caught as well.
///
/// # Cleanup
///
/// A directory the script created is removed on `EXIT`, and on `HUP`, `INT` and `TERM` so an
/// interrupted reproduction does not leave one behind either — but the two cases are handled
/// differently on purpose, for the reason given on [`SH_CLEANUP`]: `EXIT` carries the removal, while
/// each signal handler removes, restores that signal's default disposition and re-raises it, so an
/// interrupted script **dies from the signal** instead of resuming with its scratch directory
/// already deleted. The guard is the ownership flag rather than the mere presence of `WORK`, so a
/// caller's directory is never removed however the script ends, and the removal additionally
/// re-tests that the path is non-empty before running `rm -rf`.
/// Setting `REPRO_KEEP` keeps it, which is how a reader inspects the captured streams after the
/// script has finished; the path is printed either way, so it can be found without reading the source
/// of the script.
/// Render the environment the script gives **itself**, before it runs its first external helper.
///
/// # The gap this closes
///
/// [`render_isolated_environment`] gives every *child* the run's own vetted search path, and that is
/// necessary but not sufficient: the script is itself a program made of external helpers. It creates a
/// scratch directory, removes it, compares two files, prints, and clears the environment — and every
/// one of those was resolved through whatever `PATH` the reader happened to have exported. A reader
/// who runs `PATH=/somewhere/else:$PATH sh commands.sh`, or whose shell profile prepends a directory,
/// therefore hands the choice of `mktemp`, `rm`, `cmp`, `cat` and `env` to that directory — before the
/// vetted path is applied to anything. The consequence is not a wrong comparison but arbitrary code
/// execution at the reader's privilege, in a script the report tells them to run.
///
/// Three measures close it, and they are complementary rather than alternatives:
///
/// * **The vetted search path is installed for the script itself**, as the first executable statement
///   after `set -eu` and before any external program is named. It is the same value the run gave its
///   children: the entries of `PATH` that were neither relative nor writable by an account the suite
///   does not trust.
/// * **Every external helper is invoked through `command`**, which POSIX defines as suppressing shell
///   function lookup. A `PATH` that is trusted does not help against an *exported shell function*
///   named `printf` or `rm`, which a function-exporting shell will happily inherit and prefer over
///   both the builtin and the file; `command` is what makes that inheritance irrelevant.
/// * **`IFS` and `CDPATH` are reset**, so word splitting and `cd` resolution are the standard ones
///   rather than whatever was exported, and `set -C` makes every redirection in the script refuse to
///   write through anything that already exists.
///
/// # What it cannot close, stated rather than implied
///
/// A shell that sources a startup file named by an environment variable does so *before* the first
/// line of this script runs, so no statement here can prevent it. That is a property of the reader's
/// shell rather than of this script, and the comment the script carries says so, because a reader who
/// believes they are protected against something they are not is worse off than one who knows.
fn render_script_own_environment() -> String {
    let mut text = String::new();
    text.push_str(&comment(
        "--- the environment this SCRIPT runs in ------------------------------------",
    ));
    text.push_str(&comment(
        "This script is itself made of external helpers — it creates a scratch directory, removes \
         it, compares files and prints. Those were resolved through whatever PATH you had exported, \
         so a directory earlier on it could substitute any of them. The search path is therefore \
         installed HERE, before the first external program is named, and it is the same vetted value \
         the run gave every compiler and emulator it spawned.",
    ));
    text.push_str(&comment(&format!(
        "Every helper is additionally invoked through `command`, which POSIX defines as suppressing \
         shell FUNCTION lookup: a trusted PATH is no defence against an exported shell function \
         named `rm` or `printf`, and this is. Adjust {VAR_SCRIPT_PATH} below if a tool this script \
         needs is somewhere else on your machine."
    )));
    text.push_str(&comment(
        "Not closed by anything here, and stated rather than implied: a shell that sources a startup \
         file named by an environment variable does so before line one of this script, so run it \
         from a shell you trust.",
    ));
    text.push_str(&format!(
        "{VAR_SCRIPT_PATH}={}\n",
        posix_quote(&match super::env::sanitized_search_path() {
            Some(path) => path.to_string_lossy().into_owned(),
            None => String::from(FALLBACK_SCRIPT_PATH),
        })
    ));
    text.push_str(&format!("PATH=\"${VAR_SCRIPT_PATH}\"\n"));
    text.push_str("export PATH\n");
    text.push_str("unset IFS\n");
    text.push_str("CDPATH=\n");
    text.push_str("export CDPATH\n");
    text.push_str(&comment(
        "A shell that exports FUNCTIONS through the environment would let an inherited function named \
         after a helper win over both the builtin and the file on the vetted path above. Removing any \
         such definition is one line and covers every helper uniformly; the ones whose misuse would be \
         most damaging are additionally invoked through `command`, which suppresses function lookup on \
         its own.",
    ));
    text.push_str(&format!("unset -f {SH_HELPERS} 2>/dev/null || :\n"));
    text.push_str(&comment(
        "noclobber: every redirection below creates its target exclusively, so none can write \
         through a file — or a symbolic link, including a dangling one — that is already at the name.",
    ));
    text.push_str("set -C\n");
    text.push_str("umask 077\n\n");
    text
}

fn render_scratch_setup() -> String {
    let mut text = String::new();
    text.push_str(&comment(&format!(
        "{SH_REFUSE_EXISTING} guards every scratch path this script writes. A shell redirection \
         follows a symbolic link and truncates its target, so a scratch name that already exists is \
         refused rather than reused. -L is tested as well as -e, because a link pointing at nothing \
         is reported absent by -e alone."
    )));
    text.push_str(&format!("{SH_REFUSE_EXISTING}() {{\n"));
    text.push_str("    for candidate in \"$@\"; do\n");
    text.push_str("        if [ -e \"$candidate\" ] || [ -L \"$candidate\" ]; then\n");
    text.push_str(
        "            printf 'refusing to write %s: something is already at that name, and a \
         redirection would write through it\\n' \"$candidate\" >&2\n",
    );
    text.push_str("            exit 1\n");
    text.push_str("        fi\n");
    text.push_str("    done\n");
    text.push_str("}\n\n");

    text.push_str(&comment(&format!(
        "{SH_CLEANUP} removes a scratch directory this script created, and only such a directory: \
         the guard is the ownership flag rather than the mere presence of {VAR_WORK}, so a directory \
         you supplied is never removed, and {VAR_KEEP} suppresses the removal entirely. It is a \
         function because the traps below share it: removal is installed on EXIT, while each signal \
         handler removes, restores that signal's default disposition and re-raises it, so an \
         interrupted script dies from the signal instead of resuming with its scratch directory \
         already deleted."
    )));
    text.push_str(&format!("{SH_CLEANUP}() {{\n"));
    text.push_str(&format!(
        "    if [ \"${{{VAR_WORK_OWNED}:-0}}\" = 1 ] && [ -z \"${{{VAR_KEEP}:-}}\" ] && \
         [ -n \"${{{VAR_WORK}:-}}\" ]; then\n"
    ));
    text.push_str(&format!("        command rm -rf -- \"${VAR_WORK}\"\n"));
    text.push_str("    fi\n");
    text.push_str("}\n\n");

    text.push_str(&comment(&format!(
        "Scratch root: ALWAYS a private directory this script creates itself. When {VAR_WORK} names a \
         directory, the private one is created INSIDE it rather than used in place of it."
    )));
    text.push_str(&comment(&format!(
        "Creating one unconditionally is what makes the guards below sufficient rather than merely \
         helpful. Every scratch name is derived from a stem this script chooses, so if the script \
         wrote directly into a directory you supplied, all of those names would be predictable to \
         anyone who knew which finding was being replayed — and a symbolic link planted at one of \
         them before the script reached it would receive the write. Checking each name first narrows \
         that window but cannot close it, because the check and the write are two operations. A \
         directory {SH_MKTEMP_MSG} closes it instead: the name is unpredictable, the creation is \
         exclusive, and nothing existed inside it to plant."
    )));
    text.push_str("if ! command -v mktemp > /dev/null 2>&1; then\n");
    text.push_str(
        "    printf 'mktemp is required: this script always writes into a private directory it \
         creates itself, and mktemp is what creates one exclusively under an unpredictable name\\n' \
         >&2\n",
    );
    text.push_str("    exit 1\n");
    text.push_str("fi\n");
    text.push_str(&format!("if [ -n \"${{{VAR_WORK}:-}}\" ]; then\n"));
    text.push_str(&format!("    if [ -L \"${VAR_WORK}\" ]; then\n"));
    text.push_str(&format!(
        "        printf '{VAR_WORK} is a symbolic link (%s); it is refused rather than followed, \
         because every scratch write would go through it\\n' \"${VAR_WORK}\" >&2\n"
    ));
    text.push_str("        exit 1\n");
    text.push_str("    fi\n");
    text.push_str(&format!("    if [ ! -d \"${VAR_WORK}\" ]; then\n"));
    text.push_str(&format!(
        "        printf '{VAR_WORK} is not an existing directory (%s); create it yourself, or unset \
         {VAR_WORK} to let this script make a private one\\n' \"${VAR_WORK}\" >&2\n"
    ));
    text.push_str("        exit 1\n");
    text.push_str("    fi\n");
    text.push_str(&format!(
        "    {VAR_WORK_PARENT}=\"${VAR_WORK}\"\n    {VAR_WORK}=$(umask 077; command mktemp -d \
         \"${VAR_WORK_PARENT}/{SCRATCH_TEMPLATE}\") || exit 1\n"
    ));
    text.push_str("else\n");
    text.push_str(&format!(
        "    {VAR_WORK}=$(umask 077; command mktemp -d) || exit 1\n"
    ));
    text.push_str("fi\n");
    text.push_str(&format!("if [ -z \"${VAR_WORK}\" ]; then\n"));
    text.push_str(
        "    printf 'mktemp -d produced no directory, so there is nowhere safe to write\\n' >&2\n",
    );
    text.push_str("    exit 1\n");
    text.push_str("fi\n");
    text.push_str(&format!("{VAR_WORK_OWNED}=1\n"));
    text.push_str(&format!("trap {SH_CLEANUP} EXIT\n"));
    for signal in ["HUP", "INT", "TERM"] {
        text.push_str(&format!(
            "trap '{SH_CLEANUP}; trap - {signal}; kill -{signal} $$' {signal}\n"
        ));
    }
    text.push_str(&format!(
        "command printf 'scratch directory: %s\\n' \"${VAR_WORK}\"\n"
    ));
    text.push_str(&format!("if [ -n \"${{{VAR_WORK_PARENT}:-}}\" ]; then\n"));
    text.push_str(&format!(
        "    command printf 'it was created inside the %s you supplied, which is not itself \
         removed\\n' {VAR_WORK}\n"
    ));
    text.push_str("fi\n");
    text.push_str(&format!(
        "if [ -n \"${{{VAR_KEEP}:-}}\" ]; then\n    command printf 'it is kept on exit because \
         {VAR_KEEP} is set\\n'\nelse\n    command printf 'it is removed on exit; set {VAR_KEEP}=1 \
         to keep it\\n'\nfi\n"
    ));
    text
}

/// Render the working-directory change every reproduced invocation inherits.
///
/// The harness gives each spawned command the cell's own workspace as its working directory, and that
/// is not cosmetic: a compiler driver writes its intermediate files relative to where it was started,
/// an executed program that opened a relative path would resolve it there, and a diagnostic that
/// mentions a relative path means something different from a different directory. A reproduction run
/// from wherever the reader's shell happened to be is a different invocation in exactly the way that
/// matters — its scratch files land outside the scratch directory, which contradicts the hermeticity
/// this script otherwise establishes.
///
/// `cd` is used once for the whole script rather than per invocation, because the script writes nothing
/// by relative path: every scratch target it names is `"$WORK"/…` and every input is `"$SRC"`, both
/// absolute. So the change of directory affects the *children* — which is where it is needed — and
/// leaves every path the script itself spells unaffected.
fn render_working_directory() -> String {
    let mut text = String::new();
    text.push_str(&comment(
        "--- working directory -------------------------------------------------------",
    ));
    text.push_str(&comment(&format!(
        "The run spawned every command with the cell's own workspace as its working directory, so a \
         tool that writes an intermediate file relative to where it started wrote it inside the \
         workspace. {VAR_WORK} is this script's equivalent. Every path the script itself names is \
         absolute — ${VAR_WORK}/… or ${VAR_SOURCE} — so this changes where the tools write and \
         nothing else."
    )));
    text.push_str(&format!("CDPATH= cd -- \"${VAR_WORK}\" || exit 1\n\n"));
    text
}

/// Render the shell function that gives every reproduced invocation the run's own environment.
///
/// The set installed is read from [`super::child_fixed_environment`],
/// [`super::CHILD_PRIVATE_DIRECTORY_VARIABLES`] and [`super::env::sanitized_search_path`] — the
/// same three sources [`super::isolate_child_environment`] installs on a spawned command — so the
/// script and the harness cannot describe different environments. A variable added to the
/// harness's set appears here on the next run without this function being edited.
///
/// # Why `env -i` and not a series of exports
///
/// Exporting the values would add them to whatever the reader already has exported, and the harness
/// *cleared* the environment first. The variables that matter most are the ones the suite does not
/// name: an inherited `LD_PRELOAD` loads a library into every program the script runs, an inherited
/// `GCC_EXEC_PREFIX` or `C_INCLUDE_PATH` changes what the reference compiler accepts, and neither
/// would be visible in a report. `env -i` starts from nothing, which is the state the run was in.
///
/// `env` is required rather than probed for the same reason `mktemp` is: it is specified by POSIX and
/// present wherever a reproduction is plausible, and a missing one is reported as the environment
/// problem it is instead of being silently worked around by running unisolated — which would produce a
/// result the reader could not trust and would have no way to know not to trust.
///
/// The private-directory variables point at the reader's own scratch directory rather than at the
/// workspace the run used, which no longer exists on any machine and never existed on theirs. That is
/// the one deliberate difference from the recorded environment, and it is the one that makes the
/// script hermetic on the reader's machine rather than on the machine that produced the finding.
///
/// # Why the last command is `exec`, and the invariant that follows
///
/// Without it this function leaves a shell between the watchdog and the command: the background job
/// is a subshell, the subshell waits on `env`, and the process [`SH_BOUNDED`] can see is the
/// subshell rather than the thing that has to be terminated. Killing it then leaves the actual
/// program running — measured, on a probe of exactly this shape, as a surviving process per bounded
/// invocation. `exec` replaces the subshell, so the direct child is the command itself (or the outer
/// net supervising it), which is both what the harness spawns and what can be killed.
///
/// It is therefore an invariant that **this function is only ever invoked as a background job**, as
/// [`SH_BOUNDED`] does. Called in the foreground it would replace the script.
fn render_isolated_environment() -> String {
    let mut text = String::new();
    text.push_str(&comment(
        "--- the environment every invocation below is given -------------------------",
    ));
    text.push_str(&comment(
        "The suite does not spawn a compiler with the environment it inherited: it clears the \
         environment and installs exactly the set below. Reproducing the command line without the \
         environment reproduces a different invocation — a locale that prints a decimal comma, an \
         exported ASAN_OPTIONS that suppresses a diagnostic, or a PATH entry holding a substituted \
         `cc1` are each enough to change the result and none of them would be visible in the output.",
    ));
    text.push_str(&comment(&format!(
        "{VAR_CHILD_PATH} is the search path the run gave every child: the entries of PATH that were \
         neither relative nor writable by an account the suite does not trust. It is the one value \
         here that belongs to the machine the run happened on, so it is the one to adjust if a tool \
         below cannot be found."
    )));
    text.push_str(&format!(
        "{VAR_CHILD_PATH}={}\n",
        posix_quote(&match super::env::sanitized_search_path() {
            Some(path) => path.to_string_lossy().into_owned(),
            None => String::new(),
        })
    ));
    text.push_str(&comment(&format!(
        "{}, {} and the three temporary-directory names point at this script's own scratch \
         directory, so nothing a tool writes for itself lands in your home directory.",
        super::CHILD_PRIVATE_DIRECTORY_VARIABLES
            .first()
            .copied()
            .unwrap_or("HOME"),
        super::CHILD_PRIVATE_DIRECTORY_VARIABLES
            .get(1)
            .copied()
            .unwrap_or("TMPDIR"),
    )));
    text.push_str(&format!("{SH_ISOLATED}() {{\n"));
    text.push_str("    if ! command -v env > /dev/null 2>&1; then\n");
    text.push_str(
        "        printf 'env is required: the recorded run cleared the environment before spawning, \
         and reproducing a build under an inherited environment would reproduce a different \
         invocation\\n' >&2\n",
    );
    text.push_str("        exit 1\n");
    text.push_str("    fi\n");
    text.push_str("    exec env -i \\\n");
    text.push_str(&format!("        PATH=\"${VAR_CHILD_PATH}\" \\\n"));
    for (name, value) in super::child_fixed_environment() {
        text.push_str(&format!("        {name}={} \\\n", posix_quote(value)));
    }
    for name in super::CHILD_PRIVATE_DIRECTORY_VARIABLES {
        text.push_str(&format!("        {name}=\"${VAR_WORK}\" \\\n"));
    }
    text.push_str("        \"$@\"\n");
    text.push_str("}\n\n");
    text
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

    let (expectation, expected_status) = capture.shell_expectation();

    if capture.role() == CaptureRole::GoldenRecord {
        block.push_str(&comment(&format!(
            "Nothing to build or run: this authority is the stdout recorded in \
             {REPRODUCER_RECORD_NAME}, which travels with this script. The bytes it prescribes are \
             in {OUTPUTS_DIR_NAME}/{stem}.stdout beside it."
        )));
        block.push_str(&comment(&format!("Termination: {expectation}")));
        // Materialized as a status file like every other side, so the comparison below can compare
        // termination uniformly instead of special-casing the one authority that never ran. The text
        // written is the same classification vocabulary `bounded_run` records for a side that did run,
        // which is what lets the comparison be a string equality rather than a case analysis.
        if let Some(classification) = expected_status {
            block.push_str(&format!(
                "printf '%s\\n' {} > \"${VAR_WORK}/{stem}.status\"\n",
                posix_quote(&classification)
            ));
        }
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
    // The program is taken from the build's own typed record of what it compiled, never inferred from
    // the argument text. A search for the first argument ending `.c` would match a compiler installed
    // at a path that happens to end that way — `…/cc-13.c`, or any driver under a directory so named —
    // and would then substitute the *compiler* with the reproducer beside the script. The resulting
    // line reads exactly like the invocation the run performed and reproduces nothing.
    if let Some(source) = capture
        .compile()
        .and_then(|compile| compile.source().to_str())
    {
        substitutions.push((String::from(source), format!("\"${VAR_SOURCE}\"")));
    }

    if let Some(compile) = capture.compile() {
        block.push_str(&comment("build, as this run performed it (shown safely):"));
        block.push_str(&comment(&format!("  {}", compile.command_line())));
        block.push_str(&comment(&format!(
            "  bounded by the recorded {} second budget; the build reported {}",
            compile.budget().as_secs(),
            describe_compile_termination(compile)
        )));
        // Every path this build writes is refused if something is already at it. The compiled
        // artifact is included: a link planted at that name would be written through by the
        // compiler's own -o, not by a redirection, and the guard covers both.
        block.push_str(&format!(
            "{SH_REFUSE_EXISTING} \"${VAR_WORK}/{stem}.out\" \"${VAR_WORK}/{stem}.compile.stdout\" \"${VAR_WORK}/{stem}.compile.stderr\"\n"
        ));
        block.push_str(&render_bounded_invocation(
            &render_argv(compile.argv(), variables, &substitutions),
            &format!("\"${VAR_WORK}/{stem}.compile.status\""),
            &format!("\"${VAR_WORK}/{stem}.compile.stdout\""),
            &format!("\"${VAR_WORK}/{stem}.compile.stderr\""),
        ));
        block.push_str(&format!(
            "printf 'build %s : %s (recorded: %s)\\n' {} \"$termination\" {}\n",
            posix_quote(&stem),
            posix_quote(&describe_compile_termination(compile))
        ));
    }

    match capture.run() {
        Some(run) => {
            block.push_str(&comment("run, as this run performed it (shown safely):"));
            block.push_str(&comment(&format!("  {}", run.command_line())));
            block.push_str(&comment(&format!("  termination recorded: {expectation}")));
            block.push_str(&format!(
                "{SH_REFUSE_EXISTING} \"${VAR_WORK}/{stem}.stdout\" \"${VAR_WORK}/{stem}.stderr\"\n"
            ));
            // The classification is written to a file as well as printed, because the comparison at
            // the foot of the script compares termination between the two sides and cannot read text
            // that only ever went to the terminal. The oracles compare stdout *and* termination, so a
            // script that reproduced only the stdout half would report "did not reproduce" for every
            // exit-code divergence — the one class where the stdout of both sides is identical by
            // definition. `bounded_run` performs that write, so the recorded classification and the
            // recorded number cannot disagree with each other.
            block.push_str(&render_bounded_invocation(
                &render_argv(run.argv(), variables, &substitutions),
                &format!("\"${VAR_WORK}/{stem}.status\""),
                &format!("\"${VAR_WORK}/{stem}.stdout\""),
                &format!("\"${VAR_WORK}/{stem}.stderr\""),
            ));
            block.push_str(&format!(
                "printf 'run   %s : %s (recorded: %s)\\n' {} \"$termination\" {}\n",
                posix_quote(&stem),
                posix_quote(&expectation)
            ));
        }
        None => {
            block.push_str(&comment(
                "This side produced no runnable artifact in the recorded run, so there is no run \
                 line: the build above is the whole reproduction for it, and its diagnostics are in \
                 the outputs directory beside this script.",
            ));
            block.push_str(&comment(&format!("  termination: {expectation}")));
            if let Some(failure) = capture.compile().and_then(CompileOutcome::failure) {
                block.push_str(&comment(&format!("  recorded outcome: {failure}")));
            }
        }
    }
    block
}

/// Whether the script will have materialized this capture's stdout and status files.
///
/// One predicate for both streams because one condition governs both: a side that executed has its
/// stdout redirected and its status recorded by the run line, and the recorded authority has both
/// because its prescribed bytes are copied into `outputs/` and its prescribed status is written out
/// literally, so termination can be compared uniformly rather than special-cased for the one
/// authority that never ran.
///
/// False for a side whose build produced no artifact — the shape a compile refusal takes, where the
/// capture holds diagnostics and no execution. That is not a gap in the script: it is the divergence,
/// and the comparison block says so in those terms instead of comparing against files that were never
/// written.
fn has_comparable_output(capture: &Capture) -> bool {
    capture.ran() || capture.recorded.is_some()
}

/// One sentence naming how a recorded build terminated.
///
/// The same three-way distinction [`Capture::shell_expectation`] draws for an execution, applied to a
/// compiler invocation: a build that timed out and a build that was rejected both leave no artifact,
/// and a reader told only "exit 1" cannot tell which happened.
fn describe_compile_termination(compile: &CompileOutcome) -> String {
    if compile.timed_out() {
        return format!(
            "exceeded its {} second budget and was terminated",
            compile.budget().as_secs()
        );
    }
    if compile.terminated_by_signal() {
        return String::from("was killed by a signal rather than exiting");
    }
    match compile.exit_code() {
        Some(code) => format!("exited with {code}"),
        None => String::from("stopped with a status that could not be observed"),
    }
}

/// Render the call that runs one invocation through [`SH_BOUNDED`].
///
/// One line, and deliberately no branch: the bound and the classification live in the function, so
/// every invocation in the script is bounded the same way and no call site can be the one that
/// forgot. The status file is written by the function rather than by the caller, which is what makes
/// the recorded classification and the recorded number arrive together and agree.
///
/// `status` and `termination` are left set for the caller's own reporting line, and `set -e` cannot
/// abort on a failing command — a finding's whole point is that at least one side does *not* succeed,
/// so a script that stopped at the first non-zero status would never reach the comparison it exists
/// to perform.
fn render_bounded_invocation(
    invocation: &str,
    status_file: &str,
    stdout_file: &str,
    stderr_file: &str,
) -> String {
    format!("{SH_BOUNDED} {status_file} {stdout_file} {stderr_file} {invocation}\n")
}

/// Render the watchdog function every reproduced invocation runs through.
///
/// The reasoning is on [`SH_BOUNDED`]; what follows is how the four properties it claims are actually
/// established in POSIX shell, since each one is a place a plausible-looking script would be wrong.
///
/// - **The bound exists unconditionally.** The invocation is backgrounded and polled, so the timing
///   is the script's own. A reader with no `timeout` utility gets the same bound as one who has it.
/// - **The classification is out of band.** `termination` is set to [`TERMINATION_TIMEOUT`] on the
///   strength of `kill` having been issued by this function, never from `$?`. A command that returns
///   [`TIMEOUT_UTILITY_STATUS`] promptly is reported as having exited with it.
/// - **Nothing is orphaned.** When the utility is in front, *it* is the direct child and the program
///   is its descendant, so killing the child alone would leave the program running. The process group
///   is therefore killed instead — but only after confirming the child leads a group of its own, which
///   a wrapped launch does and a bare one does not. Killing a group the script itself belongs to would
///   kill the script, so that case takes the direct kill.
/// - **The outer net keeps its margin.** The utility, when the run used one, is applied at
///   `BUDGET_SECS + OUTER_MARGIN_SECS`: behind the watchdog above, never in front of it.
///
/// The polling interval is one second, which is coarse for a machine and imperceptible to a reader
/// waiting on a reproduction. A finer interval would spawn thirty times as many `sleep` processes to
/// discover the same thing.
fn render_bounded_run_function() -> String {
    let mut text = String::new();
    text.push_str(&comment(&format!(
        "{SH_BOUNDED} <status-file> <stdout-file> <stderr-file> <command...> runs one invocation \
         under this script's own watchdog and records how it ended."
    )));
    text.push_str(&comment(&format!(
        "The classification is written to the status file as one of `{TERMINATION_EXITED} <n>`, \
         `{TERMINATION_SIGNALLED} <n>` or `{TERMINATION_TIMEOUT}`, and `{TERMINATION_TIMEOUT}` is \
         written only when this function performed the kill. That is the whole reason the watchdog is \
         here rather than a bare `{TERMINATION_TIMEOUT}` prefix: a utility reports expiry as \
         {TIMEOUT_UTILITY_STATUS}, a program is entitled to return {TIMEOUT_UTILITY_STATUS} of its \
         own accord, and no reading of the number can tell those apart."
    )));
    text.push_str(&comment(&format!(
        "${VAR_TIMEOUT}, when the preamble declares one, is applied at \
         ${VAR_BUDGET} + ${VAR_OUTER_MARGIN} — behind this watchdog, exactly where the run itself \
         put it. Its status is never read. Where the preamble declares none, the reproduction is \
         still bounded, by the loop below."
    )));
    text.push_str(&format!("{SH_BOUNDED}() {{\n"));
    text.push_str("    _b_status=$1\n");
    text.push_str("    _b_stdout=$2\n");
    text.push_str("    _b_stderr=$3\n");
    text.push_str("    shift 3\n");
    text.push_str(&format!("    _b_budget=${{{VAR_BUDGET}:-30}}\n"));
    text.push_str(&format!(
        "    _b_outer=$(( _b_budget + ${{{VAR_OUTER_MARGIN}:-5}} ))\n"
    ));
    text.push_str(&format!("    if [ -n \"${{{VAR_TIMEOUT}:-}}\" ]; then\n"));
    // The isolation wraps the utility as well as the command, which is what the harness does: it
    // installs the environment on the one command it spawns, and that command is already the wrapped
    // vector. Isolating only the inner command would leave the utility running under the reader's
    // environment and pass that environment on to the program it bounds.
    text.push_str(&format!(
        "        {SH_ISOLATED} \"${VAR_TIMEOUT}\" \"$_b_outer\" \"$@\" \
         > \"$_b_stdout\" 2> \"$_b_stderr\" &\n"
    ));
    text.push_str("    else\n");
    text.push_str(&format!(
        "        {SH_ISOLATED} \"$@\" > \"$_b_stdout\" 2> \"$_b_stderr\" &\n"
    ));
    text.push_str("    fi\n");
    text.push_str("    _b_child=$!\n");
    text.push_str("    _b_waited=0\n");
    text.push_str("    _b_killed=0\n");
    text.push_str("    while [ \"$_b_waited\" -lt \"$_b_budget\" ]; do\n");
    text.push_str("        kill -0 \"$_b_child\" 2> /dev/null || break\n");
    text.push_str("        sleep 1\n");
    text.push_str("        _b_waited=$(( _b_waited + 1 ))\n");
    text.push_str("    done\n");
    text.push_str("    if kill -0 \"$_b_child\" 2> /dev/null; then\n");
    text.push_str("        _b_killed=1\n");
    // Established at runtime rather than assumed. A wrapped launch puts the utility in a process
    // group of its own and the program inside it, so the group is what has to go; a bare launch stays
    // in this script's group, where a group kill would take the script with it.
    text.push_str("        _b_group=$(ps -o pgid= -p \"$_b_child\" 2> /dev/null | tr -d ' ')\n");
    text.push_str("        _b_self=$(ps -o pgid= -p $$ 2> /dev/null | tr -d ' ')\n");
    text.push_str("        if [ -n \"$_b_group\" ] && [ \"$_b_group\" != \"$_b_self\" ]; then\n");
    text.push_str("            kill -9 \"-$_b_group\" 2> /dev/null || :\n");
    text.push_str("        else\n");
    text.push_str("            kill -9 \"$_b_child\" 2> /dev/null || :\n");
    text.push_str("        fi\n");
    text.push_str("    fi\n");
    text.push_str("    status=0\n");
    text.push_str("    wait \"$_b_child\" || status=$?\n");
    text.push_str("    if [ \"$_b_killed\" -eq 1 ]; then\n");
    text.push_str(&format!("        termination={TERMINATION_TIMEOUT}\n"));
    text.push_str(&format!(
        "    elif [ \"$status\" -ge {SHELL_SIGNAL_STATUS_BASE} ]; then\n"
    ));
    text.push_str(&format!(
        "        termination=\"{TERMINATION_SIGNALLED} $status\"\n"
    ));
    text.push_str("    else\n");
    text.push_str(&format!(
        "        termination=\"{TERMINATION_EXITED} $status\"\n"
    ));
    text.push_str("    fi\n");
    text.push_str("    printf '%s\\n' \"$termination\" > \"$_b_status\"\n");
    text.push_str("}\n");
    text
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
        // One side was never observed, which is the shape a compile refusal takes: the compiler
        // declined the program, so the arm that would have been compared against it was never
        // attempted and there is no second stream to put beside the first.
        //
        // The conclusion is *printed* and not merely commented. A reader who runs this script has to
        // be told what it concluded, and a deliverable that reproduces one build and then prints
        // nothing reads like a script that stopped halfway rather than one whose finding is an
        // absence.
        let missing = if subject.is_none() {
            String::from("the compiler under test")
        } else {
            format!("the {} authority", finding.oracle())
        };
        block.push_str(&comment(
            "Only one side of this comparison was captured, so no automatic stdout or status \
             comparison is offered: the captured outputs beside this script are the evidence, and \
             the absence itself is the divergence.",
        ));
        block.push_str(&format!(
            "printf '\\nRESULT: %s was never observed, so there is nothing to compare against — \
             that absence is the finding. The side that was observed is reproduced above, and its \
             captured streams are in {OUTPUTS_DIR_NAME}/ beside this script.\\n' {}\n",
            posix_quote(&missing)
        ));
        return block;
    };

    // A side that produced no artifact in the recorded run produces none here either, so there is no
    // stream for the script to compare: offering a comparison against a file this script will never
    // create would print a difference whose cause is the missing file rather than the finding. The
    // absence is stated instead, with the build line above as its whole reproduction.
    for side in [subject, authority] {
        if side.role() != CaptureRole::GoldenRecord && !side.ran() {
            block.push_str(&comment(&format!(
                "The {} produced no runnable artifact in the recorded run, so there is no stdout \
                 stream to compare here: the difference this finding records IS that absence. Its \
                 build line above reproduces the refusal, and the compiler's own diagnostics for it \
                 are in {OUTPUTS_DIR_NAME}/{}.compile.stderr beside this script.",
                side.role().label(),
                side.file_stem()
            )));
            return block;
        }
    }

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
    let subject_status = format!("\"${VAR_WORK}/{}.status\"", subject.file_stem());
    let authority_status = format!("\"${VAR_WORK}/{}.status\"", authority.file_stem());
    let (subject_expectation, _) = subject.shell_expectation();
    let (authority_expectation, _) = authority.shell_expectation();

    block.push_str(&comment(&format!(
        "subject   : {} ({}), which {}",
        subject.file_stem(),
        subject.role().label(),
        subject_expectation
    )));
    block.push_str(&comment(&format!(
        "authority : {} ({}), which {}",
        authority.file_stem(),
        authority.role().label(),
        authority_expectation
    )));
    block.push('\n');
    block.push_str(&comment(
        "Both halves of the oracle are checked. Either one differing is the finding, and they are \
         reported separately because a difference in only one of them says something specific: an \
         identical stdout with a differing status is an exit-code divergence, and a differing stdout \
         with an identical status is an output divergence.",
    ));
    block.push_str("differed=0\n\n");

    // Whether each side's two files will exist is known here, at render time, so the script states
    // the answer rather than testing for it. A side that executed has both; the recorded authority
    // has both, because its prescribed bytes travel with this script and its prescribed status is
    // materialized like any other; a side whose build produced no artifact has neither. Emitting a
    // comparison against a file the renderer already knows cannot exist would put `cmp: No such file`
    // into the output of a deliverable and then report the result as though a comparison had been
    // made — and emitting a runtime `[ -f ]` guard instead would put a branch in the artifact that is
    // dead the moment it is written. That absence is not a gap in the script: it is the divergence,
    // and the block says so in those terms.
    if has_comparable_output(subject) && has_comparable_output(authority) {
        // --- stdout --------------------------------------------------------------------------
        block.push_str("if command -v cmp > /dev/null 2>&1; then\n");
        block.push_str(&format!(
            "    if cmp {authority_file} {subject_file}; then\n"
        ));
        block.push_str("        printf 'stdout: identical here\\n'\n");
        block.push_str("    else\n");
        block.push_str("        printf 'stdout: DIFFERS\\n'\n");
        block.push_str("        differed=1\n");
        block.push_str("    fi\n");
        block.push_str("else\n");
        block.push_str(&format!(
            "    printf 'stdout: cmp is unavailable; compare %s and %s by hand\\n' \
             {authority_file} {subject_file}\n"
        ));
        block.push_str("    differed=1\n");
        block.push_str("fi\n\n");

        // --- termination ---------------------------------------------------------------------
        block.push_str(&format!("authority_status=$(cat {authority_status})\n"));
        block.push_str(&format!("subject_status=$(cat {subject_status})\n"));
        block.push_str("if [ \"$authority_status\" = \"$subject_status\" ]; then\n");
        block.push_str("    printf 'status: identical here (both %s)\\n' \"$authority_status\"\n");
        block.push_str("else\n");
        block.push_str(
            "    printf 'status: DIFFERS (authority %s, subject %s)\\n' \"$authority_status\" \
             \"$subject_status\"\n",
        );
        block.push_str("    differed=1\n");
        block.push_str("fi\n\n");
    } else {
        let stalled = if has_comparable_output(subject) {
            authority
        } else {
            subject
        };
        block.push_str(&comment(&format!(
            "stdout and status: not compared, because the {} side never reached execution — it {}. \
             That asymmetry is itself the divergence; the terminations recorded above and the \
             compiler diagnostics in {OUTPUTS_DIR_NAME}/ beside this script are its evidence.",
            stalled.role().label(),
            stalled.shell_expectation().0
        )));
        block.push_str("differed=1\n\n");
    }

    block.push_str("if [ \"$differed\" -eq 0 ]; then\n");
    block.push_str(
        "    printf '\\nRESULT: neither stdout nor status differed here, so the recorded \
         divergence did NOT reproduce.\\n'\n",
    );
    block.push_str(&format!(
        "    printf 'Compare {ENVIRONMENT_NAME} against your toolchain before concluding the \
         compiler changed.\\n'\n"
    ));
    block.push_str("else\n");
    block.push_str(
        "    printf '\\nRESULT: the recorded divergence reproduced, which is the finding.\\n'\n",
    );
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
///
/// # Why the whole file is redacted, and `commands.sh` is not
///
/// This is the artifact of a finding most likely to be read by somebody other than the person who
/// produced it, and the one a maintainer promotes into the committed finding set. Its content is not
/// all authored here: a tool's version banner is whatever that tool chose to print, and a tool path
/// is whatever an override variable named. `env.rs` already redacts and sanitizes a banner at the
/// moment it captures it, so the fingerprint arrives clean; [`redact_secrets`] is applied to the
/// assembled file as well, as the last thing before it is returned, because the tool **paths** come
/// from a different source than the banners and a credential that appears in one of them would
/// otherwise be committed.
///
/// The reproduction script is deliberately **not** redacted, and the asymmetry is the point: that
/// file has to *run*, and a path with `[redacted]` spliced into it names nothing. The two artifacts
/// have different jobs — one is read, one is executed — so the one that is read is the one that is
/// rewritten to be safe to read.
fn render_environment(
    finding: &Finding,
    caps: &Capabilities,
    variables: &ShellVariables,
) -> String {
    let id = finding.id();
    let generation = run_generation();
    let mut text = format!("# environment for finding {id}\n");
    text.push_str(&format!("# cell   : {}\n", finding.key()));
    text.push_str(&format!("# oracle : {}\n", finding.oracle()));
    text.push_str(
        "#\n# Recorded so that a divergence which appears or disappears later can be attributed to \
         a\n# toolchain change rather than to the compiler. Compare this file before concluding \
         anything\n# from a difference between two runs.\n#\n",
    );
    // Provenance, recorded here rather than in the manifest. The generated findings root is emptied
    // at the start of every run, so a directory beneath it always belongs to the run in progress —
    // but a maintainer holding a copied or archived directory has no way to tell which run that was,
    // and a stale artifact mistaken for a fresh one is a wrong answer that reads like a right one.
    // It is deliberately absent from `MANIFEST.txt`, which is compared between runs to see whether a
    // divergence changed and would show a difference on every comparison if it carried a token.
    text.push_str(&format!("run_token     = {}\n", generation.token()));
    text.push_str(&format!("configuration = {}\n", generation.configuration()));
    text.push_str(
        "# The token names the process that produced this directory; the configuration fingerprint \
         is\n# deterministic, so a reduced run's evidence can never be mistaken for a full run's.\n",
    );
    text.push_str(&caps.render_fingerprint());

    text.push_str(&format!(
        "\n# tool paths and execution bounds the {COMMANDS_NAME} preamble was written against\n"
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
    redact_secrets(&text)
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
    reproducer_digest: &str,
    observers: &[Oracle],
) -> String {
    let mut text = String::from("BLITZY C COMPILER — DIFFERENTIAL CONFORMANCE FINDING\n");
    text.push_str("====================================================\n\n");
    text.push_str(&format!("{MANIFEST_IDENTIFIER_PREFIX}{id}\n"));
    // The identity digest, stated on its own line as well as inside the identifier. Two artifact
    // directories describe the same finding exactly when these agree, so a register cross-check or a
    // maintainer comparing two archived directories can settle that from one field instead of parsing
    // a name apart. Deterministic, so `MANIFEST.txt` stays comparable between runs.
    text.push_str(&format!("identity_digest  = {}\n", id.digest()));
    // The digest of the program this evidence was produced from, which is what makes a reduction
    // auditable. A run copies the corpus program verbatim, so at publication this always matches the
    // reproducer beside it; a maintainer who reduces that copy and refreshes the directory writes a
    // new digest here, and one who reduces it and forgets leaves a manifest that provably describes a
    // different program from the one it ships. `curated_finding_defects` compares the two, so
    // "refresh the evidence after reducing" is a checked obligation rather than an instruction.
    text.push_str(&format!(
        "{MANIFEST_REPRODUCER_DIGEST_PREFIX}{reproducer_digest}\n"
    ));
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
    // Every oracle that observed this root cause, not just the one whose write produced this text.
    // The identity is the cell and the class, so one refused build is one finding however many oracles
    // were watching, and this is where that set is recorded rather than in three near-identical
    // directories. The line is present even when a single oracle observed it, so a reader never has to
    // wonder whether an absent field means one observer or an older manifest.
    text.push_str(&format!(
        "observed_by      = {}\n",
        observers
            .iter()
            .map(|oracle| format!("{} ({})", oracle, oracle.letter()))
            .collect::<Vec<String>>()
            .join(", ")
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

    text.push_str(&finding.contract.render());

    text.push_str("\nARTIFACTS IN THIS DIRECTORY\n---------------------------\n");
    text.push_str(&format!(
        "  {REPRODUCER_SOURCE_NAME:<22} the program, copied from the corpus\n"
    ));
    text.push_str(&format!(
        "  {REPRODUCER_RECORD_NAME:<22} its expectation record; the pair was loaded back through \
         the harness's\n  {:<22} reproducer loader before this finding was declared complete, so it \
         is runnable\n  {:<22} as it stands — no editing of the stem, the area or the location is \
         required\n",
        "", ""
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
        shown_path(&finding.source)
    ));
    text.push_str(&format!(
        "corpus_record = {}\n",
        shown_path(&finding.record)
    ));
    text.push_str(&format!(
        "reference_cc  = {}\n",
        match caps.ref_cc_for(finding.key().target()) {
            Some(path) => shown_path(path),
            None => String::from("(none discovered for this target)"),
        }
    ));
    text.push_str(&format!(
        "runner        = {}\n",
        match caps.runner_for(finding.key().target()) {
            Some(path) => shown_path(path),
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

    // Stated from what the capture actually holds rather than from the common case: a finding about
    // a program the compiler refused has a build and no run, and an opening sentence claiming it
    // "built and ran" would be the one false statement in an artifact whose whole value is that
    // every line of it is an observation.
    let mut text = match subject.ran() {
        true => format!(
            "The compiler under test built and ran this program for {} at {}. Judged against {}, \
             the two results differ.\n\n",
            finding.key().target().triple(),
            finding.key().opt().flag(),
            authority.role().label()
        ),
        false => format!(
            "The compiler under test produced no usable artifact for this program at {} for {}, so \
             it never ran. Judged against {}, that absence is itself the difference.\n\n",
            finding.key().opt().flag(),
            finding.key().target().triple(),
            authority.role().label()
        ),
    };
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
        // Identical stdout when one side never executed is a different story from identical stdout
        // when both did, and the two must not read alike. A reader told only "the streams match"
        // would go looking for the difference in the exit status of a process that does not exist,
        // whereas the real difference is that one side produced no run at all. Which side is named
        // explicitly, because that is the first thing to look at next.
        None if !subject.ran() || !authority.ran() => {
            let absence = if !subject.ran() && !authority.ran() {
                String::from("neither side executed")
            } else if !subject.ran() {
                format!("the subject {} did not execute", subject.file_stem())
            } else {
                format!("the authority {} did not execute", authority.file_stem())
            };
            text.push_str(&format!(
                "\nThe two streams are byte-identical, but {absence}: the difference is the \
                 absence of a run rather than a difference in what was printed. The build \
                 diagnostics in {OUTPUTS_DIR_NAME} are the evidence for it.\n"
            ));
        }
        None => text.push_str(
            "\nBoth sides ran and their streams are byte-identical, so the difference is in the \
             exit status rather than in what was printed.\n",
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
/// Six entries at most, and the rule for which stream goes where has no exceptions:
///
/// - `.stdout` and `.stderr` always hold the **program's** streams, byte for byte, and are empty
///   when the program never ran;
/// - `.exit` always states how the program ended, or that it did not, and never presents a status it
///   does not have;
/// - `.compile.stdout`, `.compile.stderr` and `.compile.exit` always hold the **compiler's** own
///   streams and outcome, and are absent only when there was no build at all.
///
/// The compiler's standard output is written even though a compiler ordinarily leaves it empty,
/// because the reproduction script this module emits redirects that stream to a file of the same
/// name: a maintainer who runs the script and compares its output against the recorded evidence
/// needs both sides of every stream to exist, and an entry that is absent for one build and present
/// for another would make the comparison a case analysis rather than a diff.
///
/// One rule rather than a case analysis is deliberate. A reader opening `a-aarch64-O2.stderr` must
/// be able to know what stream it holds without first working out whether that side's build
/// succeeded, and a scheme that put compiler diagnostics in `.stderr` whenever a program had not run
/// would make exactly that question unavoidable.
/// The byte count of every entry [`write_capture`] will publish for one capture.
///
/// Kept beside that function and derived from the same conditions, because the budget is decided
/// before anything is written and a projection that disagreed with the write would either refuse a
/// finding that fitted or accept one that did not.
fn capture_artifact_sizes(capture: &Capture) -> Vec<u64> {
    let mut sizes = vec![
        capture.stdout().len() as u64,
        capture.stderr().len() as u64,
        capture.exit_report().len() as u64,
    ];
    if let Some(compile) = capture.compile() {
        sizes.push(compile.stdout().len() as u64);
        sizes.push(compile.stderr().len() as u64);
        if let Some(report) = capture.compile_report() {
            sizes.push(report.len() as u64);
        }
    }
    sizes
}

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
        // Written once, and written even though a compiler ordinarily prints nothing here, for two
        // reasons that both hold. `commands.sh` directs a maintainer's re-run into
        // `<stem>.compile.stdout`, so without this entry the reproduction would produce a file with
        // nothing from this run to compare it against. And an empty entry states that the stream was
        // captured and was empty, which is a different fact from an entry that was never written at
        // all — a compiler that does print here on the program that provoked the finding would
        // otherwise leave its only clue unrecorded.
        let compile_stdout = guarded_path(context, outputs, &[&format!("{stem}.compile.stdout")])?;
        write_bytes(context, &compile_stdout, compile.stdout())?;
        written.push(compile_stdout);

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
///
/// # Why the check does not follow a link
///
/// `is_file` and `is_dir` answer about a link's *target*, so a link planted at one of these
/// predictable names would report an artifact as present while the finding directory holds nothing but
/// a pointer elsewhere — the completeness check would then certify exactly the state it exists to
/// rule out. Metadata is read without following, so only an entry this run actually published counts.
fn require_complete(context: &str, directory: &Path, key: &CellKey) -> HarnessResult<()> {
    for name in REQUIRED_ARTIFACTS {
        let entry = guarded_path(context, directory, &[name])?;
        let observed = fs::symlink_metadata(&entry).ok();
        let present = match &observed {
            Some(metadata) if *name == OUTPUTS_DIR_NAME => metadata.is_dir(),
            Some(metadata) => metadata.is_file(),
            None => false,
        };
        if !present {
            let found = match &observed {
                None => String::from("nothing is there"),
                Some(metadata) if metadata.file_type().is_symlink() => String::from(
                    "a symbolic link is there, which is not an artifact this run published and is \
                     not followed",
                ),
                Some(metadata) if metadata.is_dir() => String::from("a directory is there"),
                Some(_) => String::from("a non-regular file is there"),
            };
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "the finding directory is missing {} ({found}); a finding without its complete \
                     evidence is not a deliverable, so this is reported rather than left to be \
                     discovered by whoever reads the register",
                    shown_path(&entry)
                ),
            ));
        }
    }
    require_reproducer_pair_usable(context, directory, key)
}

/// Why a finding's artifact directory falls short of being a deliverable, or `None`.
///
/// The published counterpart of [`require_complete`], and it exists so that nobody has to restate the
/// list of what a complete finding holds. A reporter checking a row it is about to call "present", a
/// driver deciding whether the finding it just announced is really there, and this module's own
/// completeness check at the moment of publication all answer the question the same way — because
/// they all ask this.
///
/// Every entry [`REQUIRED_ARTIFACTS`] names is required, not merely the directory and the script:
/// losing `reproducer.c`, `reproducer.expected`, `MANIFEST.txt`, `environment.txt`, `diff.txt` or
/// `outputs/` leaves a row that reads as a recorded observation with the observation missing. The
/// reproducer pair is additionally required to **load**, because "reproducible" is a claim about the
/// record parsing rather than about the file existing.
///
/// # Why nothing here follows a final symbolic link
///
/// A finding directory's name is derived deterministically from the divergence, so every name checked
/// here is predictable before the run that publishes it — which is exactly the precondition a planted
/// link needs. [`Path::is_dir`] and [`Path::is_file`] answer about a link's *target*, so a link at one
/// of these names would certify a present artifact while the directory holds nothing but a pointer
/// elsewhere. Metadata is read without following, so only an entry a run actually published counts.
pub fn artifact_defect(directory: &Path) -> Option<String> {
    let published_directory = fs::symlink_metadata(directory)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);
    if !published_directory {
        return Some(String::from(
            "its artifact directory is not there, so nothing it names can be opened",
        ));
    }
    for name in REQUIRED_ARTIFACTS {
        let entry = directory.join(name);
        let observed = fs::symlink_metadata(&entry).ok();
        let present = match &observed {
            Some(metadata) if *name == OUTPUTS_DIR_NAME => metadata.is_dir(),
            Some(metadata) => metadata.is_file(),
            None => false,
        };
        if !present {
            let found = match &observed {
                None => "nothing is there",
                Some(metadata) if metadata.file_type().is_symlink() => {
                    "a symbolic link is there, which is not an artifact a run published and is not \
                     followed"
                }
                Some(metadata) if metadata.is_dir() => "a directory is there",
                Some(_) => "a non-regular file is there",
            };
            return Some(format!("it is missing {name} ({found})"));
        }
    }
    // An empty captures directory satisfies every existence test above and delivers nothing, and it
    // is the one shape of incompleteness a run can produce by itself: an interrupted write leaves
    // the directory created and unfilled.  It is reported separately from the directory being
    // absent, because a finding that names per-compiler and per-backend evidence it does not carry
    // is the one shape of report that actively misleads.
    let captures = directory.join(OUTPUTS_DIR_NAME);
    let empty_captures = match fs::read_dir(&captures) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => true,
    };
    if empty_captures {
        return Some(format!(
            "its {OUTPUTS_DIR_NAME} directory holds nothing, so the per-compiler and per-backend \
             evidence it names was never published"
        ));
    }
    if let Err(error) = load_curated_pair(directory) {
        return Some(format!(
            "its reproducer pair cannot be loaded, so the reproduction it promises cannot be \
             performed: {error}"
        ));
    }
    None
}

/// Load the reproducer record of a published finding directory, whatever cell it came from.
///
/// The identity is taken from the record itself rather than supplied, because a caller inspecting a
/// directory on disk — a reporter, or a maintainer validating a curated finding — has no `CellKey` to
/// supply and deriving one from the directory name would test the name rather than the record. The
/// record's own `area` and `program` fields are therefore read first and then used as the expected
/// identity, which still proves the record is internally consistent and that its sibling source
/// resolves to the reproducer beside it.
fn load_curated_pair(directory: &Path) -> HarnessResult<Manifest> {
    let context = format!("loading the reproducer pair in {}", shown_path(directory));
    let record_path = directory.join(REPRODUCER_RECORD_NAME);
    let text = String::from_utf8(read_file_bounded(
        &context,
        &record_path,
        MAX_INSPECTED_FILE_BYTES,
    )?)
    .map_err(|error| {
        HarnessError::new(
            context.clone(),
            format!("{} is not valid UTF-8: {error}", shown_path(&record_path)),
        )
    })?;
    let (area, program) = manifest::declared_identity(&text, &record_path)?;
    let record = manifest::load_replay(&record_path, &area, &program)?;
    let sibling = record.source_path();
    let expected_source = directory.join(REPRODUCER_SOURCE_NAME);
    if sibling != expected_source {
        return Err(HarnessError::new(
            context,
            format!(
                "the record resolves its program to {}, but the reproducer beside it is {}; a record \
                 that names a different program than the one filed with it would reproduce something \
                 other than this finding",
                shown_path(&sibling),
                shown_path(&expected_source)
            ),
        ));
    }
    Ok(record)
}

/// Every way a **curated** finding directory falls short of being committable, or an empty list.
///
/// The mandatory check behind the curation procedure in `tests/conformance/FINDINGS.md`: a generated
/// finding is promoted into `tests/conformance/findings/` by a human, and this is what makes that
/// promotion an audited step rather than a copy. It is run by the register audit over every curated
/// directory on every run, so a curated finding that decays — because its reproducer was reduced and
/// its evidence was not refreshed, or because an artifact was dropped in a rebase — is reported
/// rather than discovered by the next reader.
///
/// Four classes of defect, each closing a way a curated finding can lie:
///
/// 1. **Incompleteness.** Exactly the check [`artifact_defect`] makes, so a curated finding is held
///    to the same standard as a generated one.
/// 2. **Identity.** The directory's name and the identifier inside its own `MANIFEST.txt` must agree.
///    A promoted directory that was renamed, or whose manifest came from a different finding, would
///    otherwise be indexed in the register under a name nothing inside it claims.
/// 3. **Post-reduction consistency.** The manifest records a digest of the reproducer it was written
///    against. Minimization edits `reproducer.c`, and the whole point of the curation flow is that
///    the record, the captures, the diff and the commands are refreshed to match the reduced program;
///    a digest that no longer matches is the mechanical signature of evidence that was not refreshed,
///    which is precisely the stale-artifact defect this check exists to catch.
/// 4. **Disclosure.** Every text artifact is scanned with [`disclosure_defects`]. A generated
///    artifact may name absolute paths — it is re-run on the machine that produced it — but a
///    committed one is read by people who never saw that machine, so the checkout's location and any
///    credential-bearing value must be elided before promotion, never after.
///
/// Every returned line is already safe to render.
pub fn curated_finding_defects(directory: &Path) -> Vec<String> {
    let mut defects: Vec<String> = Vec::new();
    if let Some(defect) = artifact_defect(directory) {
        defects.push(defect);
        // Everything below reads those artifacts, so there is nothing further to say until they are
        // there. Returning early keeps one missing file from producing a page of consequences.
        return defects;
    }

    let name = directory
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let context = format!("validating the curated finding {}", shown_path(directory));
    let manifest_path = directory.join(MANIFEST_NAME);
    let manifest_text = match read_file_bounded(&context, &manifest_path, MAX_INSPECTED_FILE_BYTES)
    {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            defects.push(format!("its {MANIFEST_NAME} could not be read: {error}"));
            String::new()
        }
    };

    match manifest_line_value(&manifest_text, MANIFEST_IDENTIFIER_PREFIX) {
        Some(identifier) if identifier == name => {}
        Some(identifier) => defects.push(format!(
            "its directory is named {} while its own {MANIFEST_NAME} declares the identifier {}; the \
             register indexes a finding by its directory name, so the two must agree or a reader \
             following an entry arrives at evidence for a different divergence",
            sanitize_text_for_report(&name),
            sanitize_text_for_report(&identifier)
        )),
        None => defects.push(format!(
            "its {MANIFEST_NAME} states no `{}` line, so nothing inside the directory claims the \
             identifier the register indexes it under",
            MANIFEST_IDENTIFIER_PREFIX.trim_end()
        )),
    }

    match (
        manifest_line_value(&manifest_text, MANIFEST_REPRODUCER_DIGEST_PREFIX),
        reproducer_digest(directory),
    ) {
        (Some(recorded), Some(observed)) if recorded == observed => {}
        (Some(recorded), Some(observed)) => defects.push(format!(
            "its {MANIFEST_NAME} was written against a reproducer whose digest is {recorded}, but \
             the {REPRODUCER_SOURCE_NAME} beside it digests to {observed}. Reducing a reproducer is \
             expected; leaving the record, the captures, the diff, the commands and this manifest \
             describing the program before the reduction is not — rerun every affected cell and \
             refresh the whole directory, so the evidence and the program agree"
        )),
        (Some(_), None) => defects.push(format!(
            "its {REPRODUCER_SOURCE_NAME} could not be digested, so the manifest's record of which \
             program the evidence was produced from cannot be confirmed"
        )),
        (None, _) => defects.push(format!(
            "its {MANIFEST_NAME} states no `{}` line, so there is no way to tell whether the \
             evidence beside it was produced from the reproducer it now holds — which is exactly \
             what a reduction changes",
            MANIFEST_REPRODUCER_DIGEST_PREFIX.trim_end()
        )),
    }

    for artifact in CURATED_TEXT_ARTIFACTS {
        let path = directory.join(artifact);
        let text = match read_file_bounded(&context, &path, MAX_INSPECTED_FILE_BYTES) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                defects.push(format!("its {artifact} could not be read: {error}"));
                continue;
            }
        };
        for defect in disclosure_defects(&text) {
            defects.push(format!(
                "its {artifact} must not be committed as it is: {defect}"
            ));
        }
    }

    defects
}

/// The text artifacts a curation step must scan before committing a finding.
///
/// The captured streams under `outputs/` are deliberately excluded: they are a program's own stdout
/// and a compiler's own diagnostics, byte for byte, and rewriting them would destroy the evidence.
/// A compiler diagnostic naming the workspace it was given is a fact about the run, and the
/// procedure in `FINDINGS.md` is what tells a curator to review those by eye.
const CURATED_TEXT_ARTIFACTS: &[&str] = &[
    MANIFEST_NAME,
    COMMANDS_NAME,
    DIFF_NAME,
    ENVIRONMENT_NAME,
    REPRODUCER_RECORD_NAME,
    REPRODUCER_SOURCE_NAME,
];

/// The value of the first line of `text` beginning with `prefix`, trimmed.
fn manifest_line_value(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(|value| String::from(value.trim()))
}

/// Digest of the reproducer program in a finding directory, or `None` when it cannot be read.
fn reproducer_digest(directory: &Path) -> Option<String> {
    let path = directory.join(REPRODUCER_SOURCE_NAME);
    let context = format!("digesting {}", shown_path(&path));
    let bytes = read_file_bounded(&context, &path, MAX_INSPECTED_FILE_BYTES).ok()?;
    Some(digest_hex_of_bytes(&bytes))
}

/// Confirm the emitted reproducer pair is one the harness can actually load.
///
/// # Why existence is not enough
///
/// The two files that make a finding reproducible are a program and its expectation record. The
/// record is what carries the build commands, the target and optimization matrix, and the golden
/// output, so "the pair remains runnable by the harness" is a claim about the record **parsing**,
/// not about the file existing. A directory holding an unparseable record satisfies every existence
/// check and still delivers nothing: the reproduction it promises cannot be performed, and the
/// verdict that announced it would be a `FINDING` with no usable evidence behind it.
///
/// So the record is read back from disk after it is written and put through the real parser, with the
/// finding's own area and program as the expected identity — see [`manifest::parse_standalone_str`]
/// for why the identity must be supplied rather than derived from the path here. Reading back rather
/// than trusting the copy is what makes this a check on the artifact a maintainer will open, not on
/// the value the writer intended to produce.
///
/// Three properties are established, each the failure of one real defect:
///
/// 1. The record parses under every rule of the format, so its templates and its golden output are
///    the ones the harness would honour.
/// 2. It declares the identity of the cell this finding is about, so the pair cannot be a copy of a
///    different program left behind by an earlier run or an interrupted write.
/// 3. Its sibling source resolves to the reproducer beside it, so the record governs the program in
///    this directory rather than one somewhere else.
///
/// # Errors
///
/// Returns an explanatory failure carrying the parser's own diagnostic. The caller turns that into a
/// [`Verdict::Fail`], which is the point: a finding that cannot be reproduced must not be reported
/// as a finding that can.
fn require_reproducer_pair_usable(
    context: &str,
    directory: &Path,
    key: &CellKey,
) -> HarnessResult<()> {
    let record_path = guarded_path(context, directory, &[REPRODUCER_RECORD_NAME])?;
    let record =
        manifest::load_replay(&record_path, key.area(), key.program()).map_err(|error| {
            HarnessError::new(
                String::from(context),
                format!(
                    "{} was written but the harness cannot load it: {error}. A finding is only a \
                     deliverable if its reproducer and its record can be run again, so this cell \
                     is reported as a failure rather than as a filed finding",
                    record_path.display()
                ),
            )
        })?;
    let sibling = record.source_path();
    let expected_source = guarded_path(context, directory, &[REPRODUCER_SOURCE_NAME])?;
    if sibling != expected_source {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the emitted record at {} resolves its program to {}, but the reproducer beside it \
                 is {}; a record that names a different program than the one filed with it would \
                 reproduce something other than this finding",
                record_path.display(),
                sibling.display(),
                expected_source.display()
            ),
        ));
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
/// # Write-path discipline
///
/// Every path written passes through [`guarded_path`], which proves it lies strictly beneath the
/// generated-findings root. Nothing here writes into the corpus, into the committed finding set, or
/// anywhere else in the repository; the curated set is promoted by a human after review. That is a
/// property of the paths this function builds, not isolation of anything: the outputs it records
/// were produced by unconfined tools, and the artifacts deliberately retain their stderr, their
/// absolute tool paths and their exact commands, because those are what make a finding
/// reproducible.
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

    // Establish the run's identity before anything is written beneath the findings root.
    //
    // `sandbox::ensure_roots` creates the three artifact roots and publishes this run's manifest; it
    // clears nothing, by design. Retiring the previous run's generated findings belongs to this
    // module's own namespace preparation — `prepare_findings_namespace`, reached through
    // `ensure_findings_namespace` from `prepare_directory` below, and performed once per process: it
    // refuses a live foreign owner first, then removes the stale directories and stamps this run's
    // ownership. Both are preconditions of a write rather than obligations on a caller, which is what
    // closes the hazard for good: until the findings root has been prepared it still holds the
    // *previous* run's directories, and a finding written beside them would be retired along with
    // them by whichever caller prepared the namespace second — so this run's own deliverable would
    // vanish and the report row pointing at it would name a directory that no longer existed. The
    // driver, the flag probe, the audit gate and any later caller all get the guarantee without
    // having to know they need it, and each initializer runs once per process under a `OnceLock`, so
    // every call after the first is a load and a comparison.
    super::sandbox::ensure_roots()?;

    // Consulted before anything is rendered, because the manifest names every oracle that has
    // observed this root cause and the answer depends on who has already filed here. Consulted rather
    // than recorded: the record is made once the bytes are on disk, so a refusal below cannot leave
    // the next oracle believing this directory already exists.
    let (fresh, observers) = peek_contribution(&id, finding.oracle());

    // Rendered before the directory exists, so a refusal leaves nothing behind.
    let commands = render_commands(finding, &id)?;
    let variables = collect_shell_variables(finding);
    let environment = render_environment(finding, caps, &variables);
    let diff = render_diff(finding);
    let minimization = finding.minimization(caps);
    // Read once, digested and published from the same bytes. Reading the program a second time to
    // digest it would leave the manifest describing one revision while the reproducer beside it held
    // another whenever the two reads straddled an edit — and the whole value of the digest is that it
    // is a statement about the bytes actually filed.
    let reproducer_bytes = read_file_bounded(&context, &finding.source, MAX_INSPECTED_FILE_BYTES)?;
    // Read up front for the same reason the program is: the budget below is decided before the first
    // byte is published, and it can only be decided from sizes that are already known.
    //
    // Both corpus files enter the finding by being read and then written, never by `fs::copy`, which
    // follows a symbolic link at *both* ends — at the source it would read through a link out of the
    // corpus, and at the destination it would write a corpus program through a link and out of the
    // build directory. `read_file_bounded` refuses a link, refuses anything that is not a regular
    // file, and refuses a file past the ceiling this suite reads; the write then takes the guarded
    // path every other artifact takes. The traffic is one-directional by construction: nothing here
    // writes back out, so the corpus stays read-only to the suite.
    let record_bytes = read_file_bounded(&context, &finding.record, MAX_INSPECTED_FILE_BYTES)?;
    let manifest = render_manifest(
        finding,
        &id,
        caps,
        &minimization,
        &digest_hex_of_bytes(&reproducer_bytes),
        &observers,
    );

    // Every artifact's bytes are known at this point, so the budget is settled before the directory
    // exists rather than discovered part way through writing it.
    let mut sizes: Vec<u64> = vec![
        reproducer_bytes.len() as u64,
        record_bytes.len() as u64,
        manifest.len() as u64,
        commands.len() as u64,
        environment.len() as u64,
        diff.len() as u64,
    ];
    for capture in ordered_captures(finding) {
        sizes.extend(capture_artifact_sizes(capture));
    }
    let directory_bytes: u64 = sizes.iter().copied().sum();
    let largest_artifact = sizes.iter().copied().max().unwrap_or(0);
    require_within_artifact_budget(
        &context,
        &id.directory(),
        largest_artifact,
        directory_bytes,
        fresh,
    )?;

    let (directory, outputs) = prepare_directory(&id, fresh)?;
    let mut entries = Vec::new();

    // Written in the order REQUIRED_ARTIFACTS lists, so the returned paths and the completeness
    // check read in the same sequence as the documented artifact table.
    let source = guarded_path(&context, &directory, &[REPRODUCER_SOURCE_NAME])?;
    write_bytes(&context, &source, &reproducer_bytes)?;
    entries.push(source.clone());

    let record = guarded_path(&context, &directory, &[REPRODUCER_RECORD_NAME])?;
    write_bytes(&context, &record, &record_bytes)?;
    entries.push(record.clone());

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

    require_complete(&context, &directory, finding.key())?;
    charge_artifact_budget(directory_bytes, fresh);
    commit_contribution(&id, finding.oracle());
    Ok(FindingArtifacts {
        id,
        directory,
        entries,
        observers,
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
                "{} — undocumented divergence recorded as a deliverable, not patched. Finding {} \
                 holds {} artifact files in {}; reproduce with no harness, no Cargo and no Rust \
                 toolchain using: {}",
                finding.summary(),
                artifacts.id(),
                artifacts.entries().len(),
                shown_path(artifacts.directory()),
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
                 there is no reproducer to act on: {error}. The directory they were being written \
                 into is {}, which may hold a partial set worth inspecting. The divergence itself \
                 was: {}",
                shown_path(&finding.id().directory()),
                finding.summary()
            ),
        ),
    }
}
