//! Per-area reports and the run summary — the deliverable of the differential conformance
//! suite.
//!
//! Every other module in this harness answers a question about one cell. This one answers the
//! question the requirements close with: *what did the whole run find?* The summary it writes
//! is the artifact that reports the feature areas covered, the total tests and their outcomes,
//! every expected divergence with its documented basis, and every finding with its reproducer
//! and reproduction commands. Those four elements are the module's specification,
//! and [`render_summary_markdown`] carries them as four numbered sections so the artifact can
//! be checked against the requirement literally rather than impressionistically.
//!
//! The requirement asks for a *minimized* reproducer, and the wording here is deliberately more
//! exact than that rather than less: a run never reduces, because a reduction is unbounded in time
//! and not byte-reproducible, so what it delivers is the reproducer plus a manifest entry stating
//! that no automated reduction was performed, why, and the exact reducer command a maintainer can
//! run. Calling that "minimized" in a summary would describe something the artifact does not
//! contain. See `findings::Minimization` for the full reasoning.
//!
//! # Artifacts
//!
//! | Path | Content | Written by |
//! |---|---|---|
//! | `target/conformance-report/summary.md` | Human-readable deliverable summary | this module |
//! | `target/conformance-report/summary.tsv` | The same data, machine-readable for aggregation | this module |
//! | `target/conformance-report/areas/<area>.md` | Per-area human-readable report | this module |
//! | `target/conformance-report/areas/<area>.tsv` | Per-area machine-readable report | this module |
//! | `target/conformance-report/evidence/<cell>+oracle_<x>.txt` | Durable, sanitized evidence for an outcome that was reported without failing, whose cell workspace was therefore discarded | this module |
//! | `target/conformance-report/findings/<finding-id>/…` | The **review copy** of every finding this run recorded: all seven artifact classes, rendered to report grade, plus a `BUNDLE.txt` index. See [`publish_finding_bundle`] | this module |
//! | `target/conformance-report/.run-owner` | Which run owns this directory, so a second one is refused | this module |
//! | `target/conformance-report/run.txt` | The run manifest: which run produced the reports beside it, under what configuration, and the retention ceilings | `sandbox.rs`, named by [`super::sandbox::RUN_MANIFEST_NAME`] |
//!
//! Those eight paths are a contract shared with the suite driver, with the build directory's
//! ignore rules and with the continuous-integration job that uploads them, so they are named by
//! the constants and helpers below rather than spelled at a call site. The run manifest is listed
//! because it lands in the same directory and is uploaded with the rest, even though the module
//! that establishes the run's identity is the one that writes it. Everything is written
//! beneath [`report_root`] and nothing is written anywhere else — in particular nothing is ever
//! written under the corpus root, whose two registers
//! (`tests/conformance/EXPECTED_DIVERGENCES.md`, `tests/conformance/FINDINGS.md`) and curated
//! finding set are committed deliverables maintained by hand, not run output.
//!
//! # Concurrency: structural between threads, owned between processes
//!
//! The test harness runs the fourteen area tests concurrently **in one process** by default. Each
//! one calls [`write_area`] for its own area and therefore writes only `areas/<its own area>.md` and
//! `areas/<its own area>.tsv`: two concurrent areas cannot contend, because they cannot name the
//! same file. Safety of the artifacts themselves is therefore structural rather than enforced, and
//! there is no shared append-only log anywhere: every write goes to a uniquely named temporary
//! sibling and is then renamed into place, which is atomic on the platforms this suite supports, so
//! a concurrent reader observes either the previous file or the complete new one and never a
//! half-written one.
//!
//! Two facts are about the **run** rather than about any one area, and they are the only things
//! this module coordinates between threads: which areas have published their report pair, and
//! whether the summary has already been finalized. They live behind one small mutex, `run_registry`,
//! because they are genuinely shared — the summary must be written exactly once, by whichever
//! selected area happens to finish last, and "exactly once" is not a property any single area can
//! establish on its own. The guard is never held across a file write, and the command line is read
//! before it is taken, so the lock orders a decision rather than serializing the reporting.
//!
//! Between **processes**, structure is not enough, because the four artifact paths are derived from
//! the corpus rather than from the run: a second `cargo test` sharing one build directory addresses
//! the same files. Two things follow. The report directory carries a run-ownership stamp, so a run
//! that finds another run still working here refuses to start rather than clearing artifacts that
//! run is still producing. And the artifacts of a *finished* earlier run are cleared once, before
//! this run writes its first file, because [`try_finalize`] aggregates whatever it finds and a
//! stale set of thirteen files plus one fresh one is otherwise a complete-looking run.
//!
//! Every persisted row additionally carries the identity of the run that wrote it and a digest of
//! the configuration it was written under, and [`try_finalize`] refuses a row that carries anything
//! else. That is the measure that does not depend on the clearing having happened: a stale or
//! hand-written PASS row cannot be counted into a total without being named in the Diagnostics
//! section.
//!
//! # Once-only finalization, without ordering and without an extra test
//!
//! [`try_finalize`] is called by **every** area test at the end of its run. It is a
//! check-and-write: if the machine-readable area file of every area this invocation can publish
//! exists **and every one of them belongs to this run** it aggregates them and writes the summary,
//! and otherwise it does nothing and reports that it did nothing — [`finalization_pending`] then
//! states which of the two it was, so a summary that did not appear is never left unexplained.
//! Whichever area finishes last therefore produces the summary, no ordering between tests is
//! required, and no fifteenth test has to exist to do it — which matters because the suite's test
//! count is itself a mechanical check that no existing test was skipped or removed.
//!
//! **Reading the area files is the finalizer's privilege alone.** Every other caller is answered from
//! the run registry: the completeness question is decided inside one critical section, so a caller
//! that is merely explaining the absence of a summary already has its answer in memory, and going
//! back to the filesystem for it would re-parse the whole set once per non-final area — thirteen times
//! over in a fourteen-area run, on files that grow with the matrix. The one thing memory cannot know
//! is what the files say, so when the finalizer declines on the strength of what it read it records
//! that reason for the others to quote.
//!
//! Only **one** caller ever writes it, and that is enforced rather than hoped for. The check and the
//! claim happen together inside one critical section: [`claim_finalization`] takes the run registry's
//! `Mutex`, tests completeness against the areas this invocation selected, and sets the registry's
//! `finalized` flag before releasing the guard, so two areas finishing at the same instant cannot
//! both see an unclaimed complete set. The loser is told nothing was written and treats that as the
//! ordinary answer it is. A claim taken by a caller that then fails to publish is released again, so
//! the next caller can try rather than leaving the run with no summary and no way to produce one.
//!
//! # The report session — why a summary can never blend two runs
//!
//! Assembling the summary from the per-area files on disk buys once-only finalization without a
//! lock, but on its own it would buy something else too: a file left behind by an earlier run is
//! indistinguishable from one this run wrote, so a single fresh area could be aggregated with
//! thirteen stale ones and the result would be presented as current. An infrastructure-only or
//! name-filtered run would be worse still — it writes no area file at all, so a previous full
//! summary would simply survive and go on looking like this run's verdict.
//!
//! [`prepare_namespace`] closes both holes, and it is the **only** path that clears anything here.
//! Exactly once per process, guarded by a [`OnceLock`] so that concurrent callers block until it has
//! finished rather than racing it, it refuses a live foreign owner, removes the two summary artifacts
//! and the whole per-area directory, purges any temporary a crashed run left behind, recreates the
//! per-area directory, and stamps the root with this run's ownership. Every level on the way to each
//! of those removals is verified to be a real directory rather than a link first.
//!
//! There is deliberately no second, lighter entry point that merely scans the per-area directory and
//! deletes the report-shaped files it finds. Clearing this directory is destructive, and a route that
//! removed without first refusing a live foreign owner would delete a concurrent run's artifacts,
//! while one that removed without verifying each level would follow a planted link and delete
//! somewhere else entirely. Both hazards are real precisely because the four artifact paths are
//! deterministic, so their names are predictable before the run that will write them.
//!
//! Every artifact this run then writes is stamped with a **session signature**: a rendering of the
//! effective matrix, the program filter, the verdict policies, the per-cell budget and the test-name
//! filters this process was started with — the configuration that decides what a row means — plus a
//! token unique to this process. [`try_finalize`] reads that stamp back and refuses to aggregate an
//! artifact carrying any other signature, so neither two configurations nor two runs of one
//! configuration can be merged into one summary even if the clearing was somehow defeated.
//!
//! Both [`write_area`] and [`try_finalize`] prepare the namespace themselves before touching the
//! report root, so the guarantee is structural rather than a convention the callers must remember:
//! no artifact can be written and none can be read before the clearing has completed.
//!
//! # Generation identity — why a summary never aggregates another run's file
//!
//! The check-and-write above reads files off disk, and a file on disk outlives the run that wrote
//! it. Without an identity in the file, the summary could not tell this run's area report from one
//! a previous run left behind: a filtered run of a single area would find the other thirteen files
//! still sitting there, aggregate them, and publish a summary that looked like a complete sweep
//! while twelve fourteenths of it described a different compiler, a different corpus or a different
//! configuration. That is the one failure mode a report must never have, because its whole value is
//! that a reader can trust what it says was actually run.
//!
//! Every machine-readable area report therefore opens with a generation preamble naming the run that
//! wrote it, the configuration it ran under and the process that wrote it — see [`Generation`] — and
//! [`try_finalize`] aggregates **only** files stamped with the generation of the process reading them.
//! A file from another run is neither used nor deleted: it is listed as stale, by name and by the
//! generation it carries, and it holds the summary back until this run replaces it. Clearing the
//! report directory instead would be the wrong instrument on its own, because the fourteen area tests
//! run concurrently and a directory-wide delete would race with a sibling's write; an identity in the
//! file achieves the same guarantee with no destructive step and no lock.
//!
//! The identity is built from the run's actual inputs rather than a description of them. The sweep
//! digest covers the effective matrix, every verdict policy and the test-name filters; the
//! configuration digest covers those plus the discovered tool set and **the bytes of every program and
//! every record in the corpus**, so a report written before a program was edited is recognised as
//! describing a different corpus even though nothing about the configuration moved. And because two
//! runs of one configuration over one corpus are by construction indistinguishable by any
//! deterministic value, the preamble carries a per-process token as well — the single, narrowly drawn
//! exception to the determinism rule below, argued in full where it is written.
//!
//! # Completeness is one predicate, used everywhere
//!
//! A report is stamped `FULL` only when every dimension it publishes met its plan and nothing went
//! wrong while assembling it. The planned-against-recorded matrix, the coverage stamp in the first
//! heading and the `partial` field of the machine-readable summary are all derived from the same
//! [`MatrixDimension`] list and the same diagnostic sources, so the table and the stamp cannot
//! disagree — a table showing a shortfall beside a heading claiming full coverage would be worse
//! than either alone, because a reader who trusts the heading would never look at the table.
//!
//! # Determinism
//!
//! **Identical inputs produce byte-identical area-report Markdown.** That is the promise, and it is
//! stated at exactly that scope because two fields elsewhere are deliberately run-specific and a
//! broader claim would be wrong. There is no wall-clock timestamp, no elapsed duration, no process
//! identifier and no iteration over an unordered collection anywhere in the rendered text, and rows
//! are sorted by program, then target in [`Target::ALL`] order, then level in [`OptLevel::ALL`]
//! order, then oracle in [`Oracle::ALL`] order, with every map ordered. A report that reordered
//! itself between runs would produce phantom differences and lose exactly the regression value it
//! exists to provide.
//!
//! Two values are run-specific by design, and both are outside the area Markdown:
//!
//! * the `token=` field of an area report's generation preamble — a comment line, never rendered
//!   into a table, present because without it a report an earlier identically configured run left
//!   behind cannot be refused. The reasoning and the exact bound on where the token may appear are
//!   recorded under "Generation identity";
//! * the session signature, and with it the `identity` column of every data row and the
//!   configuration digest and fingerprint the summary reports. The signature is derived from
//!   configuration alone — never from a clock, a process identifier or a counter — but
//!   "configuration" includes the environment fingerprint, and one part of that fingerprint is
//!   itself per-run: each emulator is attested by being required to print a token derived from this
//!   run and exit with a status derived from it, and the status it had to produce is recorded in its
//!   fingerprint line. Two identically configured runs on the same machine therefore agree on every
//!   rendered verdict and differ in that digest.
//!
//! The asymmetry is the point rather than a defect to file. The area Markdown is what a maintainer
//! diffs to ask whether a change altered a result, so it is kept free of anything run-specific; the
//! digest is what answers "which run, in which environment, proved to have really executed", and an
//! attestation that did not vary could be satisfied by a stand-in that ignored its arguments, which
//! would make a silently absent emulator indistinguishable from a working one.
//!
//! # Coverage is a matrix, never a percentage
//!
//! No percentage is emitted here, and that is a matter of honesty rather than modesty. Coverage
//! instrumentation would require a development dependency, which the project's dependency rule
//! forbids absolutely, so no percentage in this repository is measurable and publishing one would
//! be fabrication. The evidence this artifact publishes instead is the enumerable matrix —
//! fourteen areas, one hundred and eight programs, three optimization levels, four targets — with
//! the planned count and the count actually run side by side, so a shortfall is visible rather
//! than averaged away.
//!
//! # What the report refuses to hide
//!
//! - **Unexpected success** is listed separately and prominently, whether or not the environment
//!   is downgrading it to a warning during a marker-retirement window.
//! - **An unavailable oracle** is listed with the diagnosis that names the missing tool and the
//!   environment variable that would supply it. A missing oracle is never a silent pass.
//! - **A reduced or partial run** is stamped in the first heading and the first line, never in a
//!   footnote, and the machine-readable summary carries `reduced=true` and `partial=true` fields.
//! - **A deliberate exclusion** — a program that restricts its targets, its optimization levels,
//!   its oracles or its warning gate — is listed with the reason its own expectation record
//!   records, so that the set of things not compared is as visible as the set that is.
//! - **A defect in the suite's own inputs** — an unreadable corpus, an unusable area file, a
//!   discovered program count that disagrees with the corpus table — becomes a loud diagnostic
//!   section and a partial stamp. It never becomes a missing row.
//!
//! Only the standard library is used: no third-party crate, no templating engine, no serialization
//! library and no table formatter — the Markdown and the tab-separated values are both written with
//! `std::fmt`. This module declares no test function of its own, so it moves neither the suite's
//! test count nor its ignored count, both of which are relied on elsewhere as mechanical evidence
//! that no pre-existing test was skipped or weakened. It performs no unchecked operation and
//! suppresses no lint. Edition 2021, minimum supported Rust 1.70.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use super::classify::{verdict_fails_run, EXPECTED_DIVERGENCE_REGISTER, FINDINGS_REGISTER};
use super::env::{
    Capabilities, RunConfig, VAR_ALLOW_MISSING_ORACLES, VAR_ALLOW_XPASS, VAR_KEEP_WORK, VAR_ONLY,
    VAR_QUICK, VAR_STRICT, VAR_TIMEOUT_SECS,
};
use super::findings::{self, FindingId, COMMANDS_NAME};
use super::manifest::{self, ExpectedDivergence};
use super::sandbox::{
    claim_ownership, live_foreign_owner_identity, workspace_path, RUN_OWNER_ENTRY,
};
use super::{
    claim_run_namespace, create_directory_chain_below, escape_markdown_inline, posix_quote,
    read_file_bounded, redact_secrets, report_root, require_directory_chain_below,
    require_replaceable, resolve_shown_path, run_generation, sanitize_text_for_report, shown_path,
    stable_digest, AreaSpec, CellKey, DivergenceClass, HarnessError, HarnessResult, OptLevel,
    Oracle, Outcome, PinnedDirectory, Replaceable, RunClaim, Target, Verdict, AREAS, AREA_COUNT,
    BCC_CELL_COUNT, MAX_INSPECTED_FILE_BYTES, MIN_PROGRAMS_PER_MANDATED_AREA,
    ORACLE_A_COMPARISON_COUNT, ORACLE_B_COMPARISON_COUNT, ORACLE_C_ASSERTION_COUNT, PROGRAM_COUNT,
    REFERENCE_CROSS_CELL_COUNT_MAX, REFERENCE_NATIVE_CELL_COUNT, TOTAL_ASSERTION_COUNT,
};

/// Directory beneath [`report_root`] that holds the per-area reports.
pub const AREAS_DIR_NAME: &str = "areas";

/// Directory beneath [`report_root`] that holds durable evidence for discarded workspaces.
///
/// Inside the report root rather than beside it, for one reason: the report root is what a maintainer
/// attaches to an issue and what continuous integration uploads. Evidence published anywhere else
/// would be evidence that, once again, reached nobody — which is the whole defect being closed.
pub const EVIDENCE_DIR_NAME: &str = "evidence";

/// Extension of an evidence document.
pub const EVIDENCE_EXTENSION: &str = "txt";

/// Directory beneath [`report_root`] that carries the review copy of every finding this run recorded.
///
/// Deliberately the same name the generated-finding root uses beneath the build directory, because it
/// holds the same seven artifact classes for the same findings — what differs is the grade of the
/// bytes, which [`BUNDLE_INDEX_NAME`] states inside every copy. Inside the report root for exactly the
/// reason [`EVIDENCE_DIR_NAME`] is: this is the directory continuous integration uploads and a
/// maintainer attaches to an issue, and a finding published anywhere else is a deliverable that
/// reaches nobody.
pub const FINDING_BUNDLE_DIR_NAME: &str = "findings";

/// Index written into every review copy, naming what the copy is and what it is not.
pub const BUNDLE_INDEX_NAME: &str = "BUNDLE.txt";

/// File stem of the two run-summary artifacts.
pub const SUMMARY_STEM: &str = "summary";

/// Scope name under which the run summary's own revalidation is recorded.
///
/// Not a feature area's directory name, and deliberately unable to collide with one: every area's
/// name is a bare directory component, so the space this contains is the one thing that keeps the two
/// namespaces apart in [`artifact_shortfalls`].
pub const SUMMARY_SCOPE: &str = "run summary";

/// Extension of the human-readable half of every report.
pub const MARKDOWN_EXTENSION: &str = "md";

/// Extension of the machine-readable half of every report.
pub const TSV_EXTENSION: &str = "tsv";

// Column names. Named once so that a writer, the parser and every call site that fills a column
// refer to the same string: a mistyped column name in a row builder would silently drop a value
// into a column nobody reads, and a constant makes that a compilation error instead.
const COL_RECORD: &str = "record";
const COL_AREA: &str = "area";
const COL_PROGRAM: &str = "program";
const COL_TARGET: &str = "target";
const COL_OPT: &str = "opt";
const COL_ORACLE: &str = "oracle";
const COL_VERDICT: &str = "verdict";
const COL_CLASS: &str = "class";
const COL_MARKER_ID: &str = "marker_id";
const COL_FINDING_ID: &str = "finding_id";
const COL_FINDING_DIR: &str = "finding_dir";
const COL_COUNT: &str = "count";
const COL_LABEL: &str = "label";
const COL_REFERENCE: &str = "reference";
const COL_DETAIL: &str = "detail";
const COL_RUN: &str = "run";
const COL_IDENTITY: &str = "identity";
// Provenance columns. Every one of them is populated on EVERY verdict that had an execution behind
// it, agreement included, which is the property the free-text `detail` column could never offer: its
// content varies by verdict, so a consumer selecting on a command line had nothing to select on for
// most of a run.
const COL_SOURCE: &str = "source";
const COL_RECORD_PATH: &str = "record_path";
const COL_SUBJECT: &str = "subject";
const COL_SUBJECT_COMMAND: &str = "subject_command";
const COL_SUBJECT_TERMINATION: &str = "subject_termination";
const COL_SUBJECT_CAPTURES: &str = "subject_captures";
const COL_AUTHORITY: &str = "authority";
const COL_AUTHORITY_COMMAND: &str = "authority_command";
const COL_AUTHORITY_TERMINATION: &str = "authority_termination";
const COL_AUTHORITY_CAPTURES: &str = "authority_captures";
const COL_FIRST_DIFFERENCE: &str = "first_difference";
const COL_SEVERITY: &str = "severity";
const COL_CELL_FINGERPRINT: &str = "cell_fingerprint";

/// The repository-relative root of the corpus, for the `source` and `record_path` columns.
///
/// Spelled relative rather than absolute for the reason every published path is: a report travels off
/// the machine that produced it, and an absolute path names a continuous-integration workspace or an
/// agent clone.
const CORPUS_RELATIVE_ROOT: &str = "tests/conformance";

/// Extension of a corpus program, matching `manifest.rs`'s own spelling.
const SOURCE_EXTENSION: &str = "c";

/// Extension of an expectation record, matching `manifest.rs`'s own spelling.
const RECORD_EXTENSION: &str = "expected";

/// `severity` value for a row that fails the run under this run's configured policy.
const SEVERITY_FAILS_RUN: &str = "fails-run";

/// `severity` value for a row that is reported and counted but does not fail the run.
const SEVERITY_REPORTED: &str = "reported";

/// Column order of a per-area machine-readable report, one row per recorded outcome.
///
/// Fixed and stable across runs so the file can be diffed between runs and aggregated by an
/// external tool, and re-read by [`try_finalize`] — which is why every enumeration in it is
/// written in the spelling its own `parse` function accepts. `detail` is last because it is the
/// only column of unbounded length.
///
/// `run` and `identity` carry the provenance of the RUN: which run wrote the row, and the digest of
/// the matrix, tool set and corpus it was written under. [`try_finalize`] refuses a row whose
/// provenance is not this run's, which is what stops an earlier run's outcomes from being aggregated
/// into this run's totals.
///
/// The thirteen columns after them carry the provenance of the OUTCOME, and they exist because the row
/// schema previously carried none. A row named the cell, the verdict and the class, and then handed
/// everything else to `detail` — one unbounded free-text field whose content varies by verdict. An
/// aggregator could therefore not answer "what command produced this", "how did the process end",
/// "where are the captured streams", "which source and which record is this about", "does this row
/// fail the run" or "is this the same cell configuration as that one" for any row at all, and for a
/// PASS row the facts were not merely unstructured but absent. The columns are:
///
/// | Column | What it carries |
/// |---|---|
/// | `source`, `record_path` | The program and its expectation record, repository-relative, so a row identifies its own inputs |
/// | `subject`, `subject_command`, `subject_termination`, `subject_captures` | The side under judgement: who produced it, the exact command, the structured ending with its raw wait status, and where the raw streams were persisted |
/// | `authority`, `authority_command`, `authority_termination`, `authority_captures` | The same four for what it was judged against. `authority_command` is empty for the golden oracle, whose authority is a committed record rather than a process |
/// | `first_difference` | Where the two sides first differ; empty for an agreement, which is a value rather than a silence |
/// | `severity` | Whether this row fails the run **under the policy this run was configured with**, so an aggregator does not have to re-implement the strict and allow-xpass rules to know what a verdict meant here |
/// | `cell_fingerprint` | A digest of the cell's identity and both commands, so the same configuration is recognisable across runs and two rows that differ only in output are distinguishable from two rows that were not the same experiment |
///
/// They all sit before `detail` so that the unbounded column stays last.
pub const AREA_TSV_COLUMNS: &[&str] = &[
    COL_AREA,
    COL_PROGRAM,
    COL_TARGET,
    COL_OPT,
    COL_ORACLE,
    COL_VERDICT,
    COL_CLASS,
    COL_MARKER_ID,
    COL_FINDING_ID,
    COL_FINDING_DIR,
    COL_RUN,
    COL_IDENTITY,
    COL_SOURCE,
    COL_RECORD_PATH,
    COL_SUBJECT,
    COL_SUBJECT_COMMAND,
    COL_SUBJECT_TERMINATION,
    COL_SUBJECT_CAPTURES,
    COL_AUTHORITY,
    COL_AUTHORITY_COMMAND,
    COL_AUTHORITY_TERMINATION,
    COL_AUTHORITY_CAPTURES,
    COL_FIRST_DIFFERENCE,
    COL_SEVERITY,
    COL_CELL_FINGERPRINT,
    COL_DETAIL,
];

/// Column order of the machine-readable run summary.
///
/// The summary carries several kinds of information, so its rows are tagged: the first column
/// names the record kind and every row has exactly [`SUMMARY_TSV_COLUMNS`] fields, with the
/// columns a kind does not use left empty. One arity and one header keep the file parseable by a
/// plain split on the tab character, while the tag keeps the kinds distinguishable.
///
/// | Record | Columns it fills |
/// |---|---|
/// | `meta` | `label`, `count` when the value is a number, `detail` |
/// | `coverage_reason` | `detail` |
/// | `matrix` | `label`, `count` (actual), `reference` (planned), `detail` |
/// | `area` | `area`, `label` (classification), `count` (outcomes), `reference` (programs), `detail` |
/// | `tally` | `area` (empty for the run total), `verdict`, `count` |
/// | `outcome` | `area`, `program`, `target`, `opt`, `oracle`, `verdict`, `class`, `marker_id`, `label`, `reference`, every provenance column, `detail` |
/// | `expected_divergence` | `area`, `program`, `class`, `marker_id`, `count`, `label` (scope), `reference` (basis path), `detail` |
/// | `finding` | `area`, `program`, `target`, `opt`, `oracle`, `class`, `label` (identifier), `reference` (directory), `detail` |
/// | `unavailable` | `area`, `program`, `target`, `opt`, `oracle`, `detail` |
/// | `exclusion` | `area`, `program`, `oracle`, `label` (kind), `reference` (what was narrowed), `detail` |
/// | `diagnostic` | `detail` |
/// | `fingerprint` | `label`, `detail` |
///
/// The provenance block an `outcome` record fills is the same block, in the same order, that an area
/// report's [`AREA_TSV_COLUMNS`] carries. Deliberately the same: the summary's outcome rows are the
/// area rows re-read, so a consumer that learned to read the evidence of a cell from one file reads it
/// unchanged from the other, and a value cannot be transcribed into a differently named column on the
/// way through. No other record kind fills them — a tally, a marker or a fingerprint is not an
/// observation of a cell and has no command, termination or capture to name.
pub const SUMMARY_TSV_COLUMNS: &[&str] = &[
    COL_RECORD,
    COL_AREA,
    COL_PROGRAM,
    COL_TARGET,
    COL_OPT,
    COL_ORACLE,
    COL_VERDICT,
    COL_CLASS,
    COL_MARKER_ID,
    COL_COUNT,
    COL_LABEL,
    COL_REFERENCE,
    COL_SOURCE,
    COL_RECORD_PATH,
    COL_SUBJECT,
    COL_SUBJECT_COMMAND,
    COL_SUBJECT_TERMINATION,
    COL_SUBJECT_CAPTURES,
    COL_AUTHORITY,
    COL_AUTHORITY_COMMAND,
    COL_AUTHORITY_TERMINATION,
    COL_AUTHORITY_CAPTURES,
    COL_FIRST_DIFFERENCE,
    COL_SEVERITY,
    COL_CELL_FINGERPRINT,
    COL_DETAIL,
];

// Record tags of the machine-readable summary. Defined once so the writer and any reader agree.
const RECORD_META: &str = "meta";
const RECORD_COVERAGE_REASON: &str = "coverage_reason";
const RECORD_MATRIX: &str = "matrix";
const RECORD_AREA: &str = "area";
const RECORD_TALLY: &str = "tally";
const RECORD_OUTCOME: &str = "outcome";
const RECORD_EXPECTED_DIVERGENCE: &str = "expected_divergence";
const RECORD_FINDING: &str = "finding";
const RECORD_UNAVAILABLE: &str = "unavailable";
const RECORD_EXCLUSION: &str = "exclusion";
const RECORD_DIAGNOSTIC: &str = "diagnostic";
const RECORD_FINGERPRINT: &str = "fingerprint";
const RECORD_PREFLIGHT: &str = "preflight";

/// Longest free-text cell rendered into a Markdown table before it is truncated.
///
/// Truncation applies to the human-readable table only, and the tables that use it say so: the
/// machine-readable sibling always carries the text in full, so nothing is lost from the
/// artifact. Without a bound a single compiler diagnostic could make one table column wider than
/// every other row's combined, which is a table nobody reads.
const TABLE_CELL_MAX_CHARS: usize = 180;

/// Appended to a Markdown table cell that was truncated, and explained beneath the table.
const TRUNCATION_MARK: &str = "…";

/// Rendered in a Markdown table cell that has no value, so an empty cell is visibly empty
/// rather than ambiguously blank.
const ABSENT_CELL: &str = "—";

/// Separator of the machine-readable reports. A field may never contain one, which is what
/// [`tsv_field`] guarantees.
const TSV_SEPARATOR: char = '\t';

// ---------------------------------------------------------------------------------------------
// Generation identity
//
// One line, at the top of every machine-readable area report, saying which run wrote it, under what
// configuration, and — uniquely — which *process*. It exists so that aggregation can be restricted to
// one run: a report file outlives the process that wrote it, and a summary assembled from whatever
// happens to be on disk would present another run's results as this one's. The line begins with `#`,
// so it is visibly not a data row, and it is the first line of the file, so a file that lacks it is
// recognised as predating the stamp on the very first read rather than after fourteen rows have been
// counted.
//
// # The one report artifact a process token reaches, and why it reaches this one
//
// This module's determinism rule is that identical inputs produce byte-identical area-report
// Markdown, and a per-process token breaks that for whatever carries it. It is carried here anyway,
// in exactly one field of exactly one line, because the alternative is worse: without it, a report
// left behind by an earlier run of the *same configuration over the same corpus* is byte-for-byte a
// report this run could have written, and the identity check that exists to refuse it cannot.
// Clearing the report directory at the start of a run is the primary defence and it is not
// sufficient on its own — a purge that cannot remove an entry reports the failure and the file
// survives it, which is exactly the case this check is the last line against.
//
// The exception is kept as narrow as it can be:
//
//   * of the artifacts this module renders, the token reaches **only** the `token=` field of an area
//     report's generation preamble, which is a comment line whose sole consumer is the aggregation
//     check it serves;
//   * it appears in **no** rendered Markdown, in **no** summary field of either half, and in **no**
//     diagnostic — a token-only mismatch is described in words rather than by quoting either token —
//     so the area Markdown a maintainer diffs between runs stays byte-identical for identical inputs;
//   * `run` and `config` are unchanged and still derived from configuration alone, so a *differently*
//     configured file is still recognised by a deterministic value and can still be explained.
//
// Two writes outside this module complete the contract, and they are not rendered reports: the
// `run_token` line of the `run.txt` run manifest `sandbox.rs` places beside these reports, and the
// `run_token` line of a finding's `environment.txt`. Those three physical writes are the whole set;
// [`super::RunGeneration::token`] enumerates them in one place.
//
// The token is not the only value that differs between two identically configured runs — the session
// signature does too, because the environment fingerprint it is derived from records the per-run
// status each emulator had to exit with to be attested. That is a separate, deliberate exception,
// documented under "Determinism" above and at `RunIdentity::compute`. The bound stated here is about
// the token alone.
// ---------------------------------------------------------------------------------------------

/// First token of the generation preamble, which is also how a preamble is recognised.
const GENERATION_PREAMBLE_PREFIX: &str = "#generation";

/// Label under which the sweep identity appears as a `meta` row of the machine-readable summary.
///
/// The same words the human-readable reports use for it, so the two artifacts of one run cannot
/// name one fact two ways.
const SESSION_LABEL: &str = "run_identity";

/// Preamble key naming the run that wrote the file.
const GENERATION_KEY_RUN: &str = "run";

/// Preamble key naming the configuration the run was executed under.
const GENERATION_KEY_CONFIG: &str = "config";

/// Preamble key naming the process that wrote the file.
const GENERATION_KEY_TOKEN: &str = "token";

/// Preamble field naming whether the matrix that ran was smaller than the full one.
///
/// Published as a field of the FIRST line, rather than left to be inferred from the `config` field or
/// read out of the Markdown sibling, because that is where a machine consumer looks. A reduced report
/// that a machine reads as a full one is the one misreading these artifacts must not permit: it turns
/// "these 96 cells agreed" into "this area agreed", and the difference is the whole claim.
const GENERATION_KEY_REDUCED: &str = "reduced";

/// Preamble field naming whether this report may not describe one complete, coherent run.
///
/// Distinct from `reduced`: a reduced matrix is always partial, but a full matrix can also be partial
/// — an unmet precondition, an unreadable record, a diagnostic raised while assembling. Both are
/// published so neither has to be inferred from the other.
const GENERATION_KEY_PARTIAL: &str = "partial";

/// Which run wrote an artifact, under what configuration it ran, and which process it was.
///
/// The three fields answer three different questions and all three are needed.
///
/// - `run` identifies the **sweep** as a digest, so a file written under settings this run did not
///   use is recognised as foreign in one comparison.
/// - `config` spells those settings out, so a foreign file's mismatch can be explained to a reader —
///   "that report swept one target at two levels, this run sweeps four at three" is actionable, where
///   an opaque identifier alone is not.
/// - `token` identifies the **process**, so a file left behind by an earlier run of an *identical*
///   configuration over an *identical* corpus is recognised too. The first two fields are pure
///   functions of the run's inputs and therefore cannot tell a repeat run from the run it repeats;
///   this one can, and it is the only reason it exists. It is written into no other artifact and into
///   no diagnostic — see the section comment above for why the determinism exception is drawn exactly
///   here and nowhere wider.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Generation {
    run: String,
    config: String,
    token: String,
}

impl Generation {
    /// The generation of the process reading this.
    ///
    /// Stable for the lifetime of the process — the run identifier is memoized and the
    /// configuration is itself a process singleton — so every area report this process writes
    /// carries the same stamp, which is exactly what lets [`try_finalize`] recognise its own.
    fn current(caps: &Capabilities) -> Generation {
        Generation {
            run: String::from(run_identifier(caps)),
            config: configuration_fingerprint(caps),
            token: String::from(run_generation().token()),
        }
    }

    /// The line that opens a machine-readable report — an area's and the summary's alike.
    ///
    /// Assembled here rather than through [`tsv_field`] because it is a comment line rather than a
    /// row; every identity value is built from a restricted character set by [`safe_token`] and the
    /// two coverage fields render as one of two fixed words, so none of the five can contain the
    /// separator, a line break or anything else that would let the preamble be read as two lines or
    /// as a data row.
    ///
    /// Carries the coverage of the run as well as its identity. The three identity fields answer
    /// "which run wrote this"; `reduced` and `partial` answer "what does it describe", which is the
    /// question a machine consumer has to answer before it may aggregate a single row. They are
    /// deliberately **not** read back by [`Generation::parse`] and take no part in staleness: a
    /// report is foreign because of *whose* it is, never because of how much it covered, and folding
    /// coverage into identity would make this run reject its own reports the moment a filter changed
    /// what they described.
    fn preamble(&self, coverage: &Coverage) -> String {
        format!(
            "{GENERATION_PREAMBLE_PREFIX}{TSV_SEPARATOR}{GENERATION_KEY_RUN}={}\
             {TSV_SEPARATOR}{GENERATION_KEY_CONFIG}={}\
             {TSV_SEPARATOR}{GENERATION_KEY_TOKEN}={}\
             {TSV_SEPARATOR}{GENERATION_KEY_REDUCED}={}\
             {TSV_SEPARATOR}{GENERATION_KEY_PARTIAL}={}",
            self.run,
            self.config,
            self.token,
            true_false(coverage.reduced),
            true_false(coverage.is_partial())
        )
    }

    /// Read a generation back from the first line of a machine-readable area report.
    ///
    /// Returns `None` for any line that is not a well-formed preamble, including the bare column
    /// header a report written before the stamp existed begins with. The caller treats every such
    /// file as foreign, which is the safe direction: an unstamped file cannot be shown to belong to
    /// this run, and counting it would be the failure the stamp exists to prevent.
    ///
    /// A value may itself contain `=`, because the split takes the first one only, and the keys may
    /// appear in either order — the format is read as a set of fields rather than as a fixed shape,
    /// so a later key can be added without invalidating files that lack it.
    fn parse(line: &str) -> Option<Generation> {
        let mut fields = line.split(TSV_SEPARATOR);
        if fields.next()? != GENERATION_PREAMBLE_PREFIX {
            return None;
        }
        let mut run = None;
        let mut config = None;
        let mut token = None;
        for field in fields {
            let (key, value) = field.split_once('=')?;
            match key {
                GENERATION_KEY_RUN => run = Some(String::from(value)),
                GENERATION_KEY_CONFIG => config = Some(String::from(value)),
                GENERATION_KEY_TOKEN => token = Some(String::from(value)),
                _ => {}
            }
        }
        // Every key is required, the process token included. A file that carries no token cannot be
        // shown to belong to this process, so treating its absence as "belongs to whoever is reading"
        // would reopen the hole the token closes. A file written by a harness that predates the token
        // is therefore stale, which is the same treatment a file predating the whole preamble already
        // receives: re-run the area to replace it.
        Some(Generation {
            run: run?,
            config: config?,
            token: token?,
        })
    }

    /// How a generation is named in a diagnostic or a report table.
    ///
    /// The process token is deliberately omitted. This string is rendered into the area report's
    /// Markdown, which is documented as byte-identical for identical inputs, and a token would change
    /// it on every run for no reader's benefit — the token exists to be *compared*, not read. The
    /// token-only mismatch is described in words where it arises, in [`read_area`].
    fn describe(&self) -> String {
        format!("run `{}`, configuration `{}`", self.run, self.config)
    }
}

/// Identifier of the sweep this run is performing.
///
/// Deliberately the *same* token [`RunIdentity`] stamps into the provenance columns, rather than a
/// second one computed here. Two spellings of one run's identity would put one of them in the
/// generation preamble and the other in the provenance columns of the very same file, and a reader
/// comparing the two would have no way to tell whether they described one run or two.
///
/// Derived from configuration alone and memoized for the life of the process, so two identically
/// configured runs stamp identical bytes — the determinism rule this module opens with. A run that
/// merely *repeats* an earlier one is therefore indistinguishable **by this value**, which is why it
/// is not the only thing stamped: [`prepare_namespace`] removes an earlier run's artifacts before this
/// run writes anything, [`Generation`] additionally carries this process's own
/// [`super::RunGeneration::token`], and this value is what makes a *differently configured* file
/// recognisable as foreign and explainable to a reader.
fn run_identifier(caps: &Capabilities) -> &'static str {
    &RunIdentity::of(caps).run
}

/// A one-line fingerprint of everything about the run configuration that changes what a report
/// means.
///
/// Every field that narrows the matrix, changes a verdict policy or changes which oracle arms can
/// be attempted is included, so two artifacts carrying the same fingerprint describe comparable
/// sweeps and two carrying different ones do not. The oracle field is a bit per oracle and target
/// in [`Oracle::ALL`] and [`Target::ALL`] order, which is what makes a machine with one missing
/// cross driver distinguishable from a fully equipped one.
///
/// Deliberately excluded: the retained-workspace setting, which changes what is left on disk after
/// a cell but not what the cell decided.
fn configuration_fingerprint(caps: &Capabilities) -> String {
    let config = caps.config();
    let (targets, levels) = config.effective_matrix();
    let oracles: String = Oracle::ALL
        .iter()
        .map(|oracle| {
            let bits: String = Target::ALL
                .iter()
                .map(|target| {
                    if caps.oracle_available(*oracle, *target) {
                        '1'
                    } else {
                        '0'
                    }
                })
                .collect();
            format!("{}{bits}", oracle.letter())
        })
        .collect();
    let fields = [
        format!("quick:{}", digit(config.quick_mode())),
        format!(
            "only:{}",
            match config.only() {
                Some(filter) => safe_token(&format!("{}/{}", filter.area(), filter.program())),
                None => String::from("-"),
            }
        ),
        format!("strict:{}", digit(config.strict())),
        format!("allow_xpass:{}", digit(config.allow_xpass())),
        format!(
            "ack_missing:{}",
            digit(config.missing_oracles_acknowledged())
        ),
        format!("timeout:{}", config.timeout_secs()),
        format!(
            "targets:{}",
            targets
                .iter()
                .map(|target| target.short_name())
                .collect::<Vec<_>>()
                .join("+")
        ),
        format!(
            "levels:{}",
            levels
                .iter()
                .map(|level| level.short())
                .collect::<Vec<_>>()
                .join("+")
        ),
        format!("oracles:{oracles}"),
    ];
    fields.join(";")
}

/// `1` for true and `0` for false — the fingerprint's spelling of a flag.
fn digit(value: bool) -> char {
    if value {
        '1'
    } else {
        '0'
    }
}

/// `raw` reduced to a token that is safe inside a preamble field.
///
/// Keeps the characters a program filter legitimately contains and replaces every other one,
/// including any whitespace, with an underscore. An empty result becomes `-`, so a field is never
/// silently absent. The result is a fingerprint component rather than a faithful reproduction: the
/// filter is also reported verbatim in the run-configuration table, where it is not constrained.
fn safe_token(raw: &str) -> String {
    let token: String = raw
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character == '_'
                || character == '-'
                || character == '.'
                || character == '/'
            {
                character
            } else {
                '_'
            }
        })
        .collect();
    if token.is_empty() {
        String::from("-")
    } else {
        token
    }
}

/// Absolute path of the directory holding the per-area reports.
pub fn areas_dir() -> PathBuf {
    report_root().join(AREAS_DIR_NAME)
}

/// Absolute path of one area's human-readable report.
///
/// Takes the area specification rather than a name so that the file name is a
/// `&'static str` from the corpus table. Path safety is then a property of the type: no caller
/// can name a file outside [`areas_dir`], because no value of [`AreaSpec`] carries anything but
/// one of the fourteen canonical directory names.
pub fn area_markdown_path(area: &AreaSpec) -> PathBuf {
    areas_dir().join(format!("{}.{MARKDOWN_EXTENSION}", area.directory()))
}

/// Absolute path of one area's machine-readable report. The file whose existence
/// [`try_finalize`] checks.
pub fn area_tsv_path(area: &AreaSpec) -> PathBuf {
    areas_dir().join(format!("{}.{TSV_EXTENSION}", area.directory()))
}

/// File name of the human-readable run summary, as an entry of the report root.
///
/// Named separately from [`summary_markdown_path`] because the namespace preparation addresses it as
/// an **entry of a pinned directory** rather than as a path — a removal aimed at a name inside a
/// handle this run holds cannot be redirected by replacing a level of the root's name.
fn summary_markdown_name() -> String {
    format!("{SUMMARY_STEM}.{MARKDOWN_EXTENSION}")
}

/// File name of the machine-readable run summary, as an entry of the report root.
fn summary_tsv_name() -> String {
    format!("{SUMMARY_STEM}.{TSV_EXTENSION}")
}

/// Absolute path of the human-readable run summary — the suite's deliverable.
pub fn summary_markdown_path() -> PathBuf {
    report_root().join(summary_markdown_name())
}

/// Absolute path of the machine-readable run summary.
pub fn summary_tsv_path() -> PathBuf {
    report_root().join(summary_tsv_name())
}

/// Absolute path of the directory holding durable evidence for outcomes whose workspace was discarded.
pub fn evidence_dir() -> PathBuf {
    report_root().join(EVIDENCE_DIR_NAME)
}

/// Absolute path of the evidence document for one cell and one oracle.
///
/// Deterministic, and built from the **same** slug the cell's workspace is named with, so the two are
/// recognisably one cell's artifacts and a reader holding a report row can compute either without
/// being handed a path. The oracle is part of the name because the verdict is per oracle: one cell can
/// have an expected divergence on one arm and a plain pass on the others, and only the first needs a
/// document.
pub fn evidence_document_path(key: &CellKey, oracle: Oracle) -> PathBuf {
    evidence_dir().join(format!(
        "{}+oracle_{}.{EVIDENCE_EXTENSION}",
        key.slug(),
        oracle.letter()
    ))
}

/// Absolute path of the directory holding the review copy of every finding this run recorded.
pub fn finding_bundle_root() -> PathBuf {
    report_root().join(FINDING_BUNDLE_DIR_NAME)
}

/// Absolute path of the review copy of one finding.
///
/// Named from the finding's own identifier, exactly as the generated directory is, so the two copies
/// of one finding are recognisably one finding and a reader holding a report row can compute either.
/// The identifier draws on an alphabet that excludes the path separator and can be neither `.` nor
/// `..` — [`FindingId::directory`] carries that argument in full — so joining it here always yields a
/// direct child of this root, which every write below re-establishes regardless.
pub fn finding_bundle_dir(id: &FindingId) -> PathBuf {
    finding_bundle_root().join(id.as_str())
}

/// The review copy's location as a report reader sees it: relative to the report root.
///
/// Takes the identifier as text rather than as a [`FindingId`], because the two callers hold it in
/// different forms — the publisher has the derived value, a report row has the rendered string it
/// carries in its own column — and one definition of the address is what keeps a row's pointer and the
/// directory the publisher creates from drifting apart.
///
/// Deliberately **relative**. Every area report is required to be byte-identical between two runs over
/// identical inputs, so a column carrying an absolute build path would differ between two machines that
/// agree perfectly; and the reader of that column already holds the report root, because they are
/// reading a file inside it.
pub fn finding_bundle_reference(identifier: &str) -> String {
    format!("{FINDING_BUNDLE_DIR_NAME}/{identifier}/")
}

// ---------------------------------------------------------------------------------------------
// Run identity
//
// The four artifact paths above are deterministic, and that is deliberate: a report row has to lead
// a maintainer straight to a file. Determinism across *processes*, though, means two runs address
// the same files, and it means a file an earlier run left behind parses perfectly. Both are real
// hazards for a report specifically, because a report is aggregated: `try_finalize` reads the
// fourteen area files back and adds their rows up, so a stale file's PASS rows would be counted into
// a summary describing a matrix this run never executed, and nothing in the artifact would say so.
//
// Two independent measures answer it, because they fail differently. The report directory carries a
// run-ownership stamp, so a second concurrent run is refused rather than allowed to interleave, and
// the artifacts of a *finished* earlier run are cleared before this run writes its first file. And
// every persisted row carries the identity of the run that wrote it, so a row that survives all of
// that anyway is refused at aggregation and reported as a diagnostic instead of counted.
// ---------------------------------------------------------------------------------------------

/// The provenance stamped into every persisted row and checked when one is read back.
///
/// Both halves are digests of *what a report means* rather than of when it was produced. `run`
/// identifies the run in the sense a reader cares about — the matrix, the policy and the corpus it
/// was taken over — and `identity` identifies the *configuration* underneath it: the tools that were
/// discovered and the corpus that was read. A row is aggregated only when both match, so neither a
/// file left behind by a differently configured run nor one written against a different tool set or
/// corpus can contribute to a total without being named.
///
/// Neither half distinguishes this **process** from another that is configured identically, and
/// deliberately so: both are derived from configuration and corpus content alone, never from a clock,
/// a process identifier or a counter, which is what makes an artifact's provenance reproducible. The
/// per-process discriminator is [`Generation`]'s own token, and that is the value which catches a row
/// left behind by a previous run of the very same configuration — see [`RunIdentity::compute`] for
/// why the division is drawn here rather than by making these values unique per process.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunIdentity {
    run: String,
    identity: String,
}

impl RunIdentity {
    /// This run's provenance, computed once for the whole process.
    ///
    /// Memoized because it is stamped into every row and checked for every area, and because it must
    /// not change between the area that wrote a row and the area that reads it back — the two are
    /// different threads of one process. The capability record is itself resolved once per process,
    /// so the first caller's `caps` is every caller's `caps`.
    fn of(caps: &Capabilities) -> &'static RunIdentity {
        static IDENTITY: OnceLock<RunIdentity> = OnceLock::new();
        IDENTITY.get_or_init(|| RunIdentity::compute(caps))
    }

    /// Derive the provenance from the things a report's meaning depends on.
    ///
    /// Both halves are derived from configuration and corpus content alone — never from a clock, a
    /// process identifier or a counter. Recognising a file written by *another* run of the same
    /// configuration over the same corpus is deliberately not this value's job:
    /// [`prepare_namespace`] removes the previous run's artifacts before this run writes any, and
    /// [`Generation`]'s per-process token catches whatever that purge could not reach.
    ///
    /// # Why `identity` still differs between two identically configured runs
    ///
    /// "Configuration alone" is not the same as "stable across runs", and the difference is worth
    /// stating because `identity` is rendered — as the `identity` column of every data row and as
    /// the configuration digest and fingerprint of the summary. One of this value's inputs is the
    /// discovered tool set, and a tool record carries how that tool was established. Under strict
    /// mode an emulator is established by **attestation**: it is required to print a token derived
    /// from this run and to exit with a status derived from it, so a stand-in that ignored its
    /// arguments cannot satisfy the check by luck. The status it had to produce is part of the
    /// record, so the fingerprint — and therefore this digest — legitimately differs from run to
    /// run on one unchanged machine.
    ///
    /// That is why the module's determinism promise is scoped to the **area Markdown**, which
    /// renders no part of this value, rather than to every artifact. The two serve different
    /// questions: the Markdown answers "did a result change", so it must not move; this digest
    /// answers "which sweep, against which corpus bytes, in an environment each tool proved itself
    /// in", and an attestation that never varied would answer the last part falsely. A run
    /// comparing two reports' `identity` values is asking whether they describe the same sweep of
    /// the same corpus in the same process — which is exactly the check
    /// [`prepare_namespace`] and the aggregation step need, and which the memoization above keeps
    /// consistent for every thread of one process.
    ///
    /// `run` digests the sweep that was configured: the effective matrix and policy above, plus
    /// the test-name filters this process was started with, which decide which areas could run at
    /// all. `identity` digests what the sweep ran against: the row schema, the effective matrix and
    /// policy, the discovered tool set, and the corpus this run read — the latter as
    /// [`manifest::corpus_content_digest`], which is the **bytes** of every program and every record
    /// rather than their paths or their declared counts. That distinction is the whole point of the
    /// field: an edit to a program changes nothing else in this digest, so a path-and-count derivation
    /// would accept a report written before the edit and add its rows to a summary describing the
    /// corpus after it. Each input is recorded in the summary beside the digest, so a mismatch can be
    /// diagnosed rather than merely detected. [`stable_digest`] is a fixed specification rather than
    /// the standard library's hasher, whose output is documented as unstable between releases and
    /// would make one run's rows unrecognisable to the next.
    fn compute(caps: &Capabilities) -> RunIdentity {
        let config = caps.config();
        let (targets, levels) = config.effective_matrix();
        let schema = tsv_header(AREA_TSV_COLUMNS);
        let matrix = format!(
            "targets={};levels={};quick={};only={};strict={};allow_xpass={};allow_missing={};\
             timeout={};keep_work={}",
            join_targets(&targets),
            join_opt_levels(&levels),
            config.quick_mode(),
            config
                .only()
                .map(|filter| format!("{}/{}", filter.area(), filter.program()))
                .unwrap_or_default(),
            config.strict(),
            config.allow_xpass(),
            config.missing_oracles_acknowledged(),
            config.timeout_secs(),
            config.keep_work(),
        );
        let tools = caps.render_fingerprint();
        // The corpus's own bytes, not its path and not the declared program counts. Everything else
        // in this digest is unchanged by an edit to a program or a record — same configuration, same
        // tools, same declared counts — so a digest derived from anything but the content would
        // accept an area report written before the edit and add its rows to a summary describing the
        // corpus after it. Content is the only input that detects that, and it costs no determinism:
        // two runs over an unchanged corpus still digest identically.
        let corpus = format!("content={}", manifest::corpus_content_digest());
        let names = libtest_filter_selection().all();
        let sweep = format!(
            "{matrix};filters={}",
            if names.is_empty() {
                String::from("none")
            } else {
                names.join("+")
            }
        );
        RunIdentity {
            run: stable_digest(&[sweep.as_str()]),
            identity: stable_digest(&[
                schema.as_str(),
                matrix.as_str(),
                tools.as_str(),
                corpus.as_str(),
            ]),
        }
    }

    /// Whether a row read back was written by this run under this configuration.
    fn accepts(&self, run: &str, identity: &str) -> bool {
        self.run == run && self.identity == identity
    }

    /// A sentence naming what a refused row claimed, and which half of the claim was wrong.
    ///
    /// Phrased as plain sanitized text, like every other diagnostic this module produces: the
    /// Markdown half escapes a whole diagnostic at its sink, so a fragment escaped here as well
    /// would be escaped twice.
    fn describe_mismatch(&self, run: &str, identity: &str) -> String {
        let which = match (self.run == run, self.identity == identity) {
            (false, false) => String::from(
                "it names a different run and a different configuration, so it is an artifact of an \
                 earlier or concurrent run",
            ),
            (false, true) => format!(
                "it names sweep `{}` rather than this run's `{}`, so it was written under a \
                 different matrix, a different policy or a different test-name filter — an \
                 earlier run whose file survived, or a concurrent one",
                sanitize_text_for_report(run),
                sanitize_text_for_report(&self.run)
            ),
            (true, false) => format!(
                "it names configuration digest `{}` rather than this run's `{}`, so it was written \
                 under a different matrix, a different tool set or a different corpus",
                sanitize_text_for_report(identity),
                sanitize_text_for_report(&self.identity)
            ),
            (true, true) => String::from("it matches this run, so it was not refused"),
        };
        format!(
            "{which}. It is reported here rather than counted, because a total that silently \
             included it would describe a matrix this run did not execute."
        )
    }
}

// ---------------------------------------------------------------------------------------------
// The report namespace
//
// Cleared once per run, owned for the duration of the run, and verified level by level on the way
// to every file. The clearing is what stops a stale area file from being aggregated; the ownership
// stamp is what stops two concurrent runs from clearing each other; and the verification is what
// stops a planted symbolic link from deciding where a report is written.
// ---------------------------------------------------------------------------------------------

/// Refuse a report path that does not lie strictly beneath [`report_root`].
///
/// Purely lexical and deliberately so: it runs *before* anything is created, so there is nothing to
/// resolve yet. Every component below the root must be a plain name, which is what makes the
/// subsequent walk unable to climb, and the root itself is refused because a report is a file or a
/// directory beneath the root and never the root.
fn require_beneath_report_root(context: &str, candidate: &Path) -> HarnessResult<()> {
    let root = report_root();
    let mut remaining = candidate.components();
    for expected in root.components() {
        if remaining.next() != Some(expected) {
            return Err(HarnessError::new(
                String::from(context),
                format!(
                    "{} does not lie beneath the run-report root {}; this module writes the two \
                     summary files and the per-area reports and nothing anywhere else",
                    shown_path(candidate),
                    shown_path(&root)
                ),
            ));
        }
    }
    let mut depth = 0usize;
    for component in remaining {
        match component {
            Component::Normal(_) => depth += 1,
            _ => {
                return Err(HarnessError::new(
                    String::from(context),
                    format!(
                        "{} contains a component that is not a plain name below {}; a \
                         parent-directory component would let the path climb out of the build \
                         directory, so it is refused instead of resolved",
                        shown_path(candidate),
                        shown_path(&root)
                    ),
                ));
            }
        }
    }
    if depth == 0 {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "{} is the run-report root itself rather than a path beneath it",
                shown_path(&root)
            ),
        ));
    }
    Ok(())
}

/// Establish this run's ownership of the report directory before any area begins.
///
/// The driver calls this once at start-up so the clearing, the ownership claim and every refusal
/// they can produce happen *before* the first cell is compiled rather than after a whole area's
/// matrix has been executed. Two things follow, and both matter: a conflict with a concurrent run is
/// reported in seconds instead of minutes, and the report directory holds this run's artifacts for
/// the whole of this run instead of an earlier run's until the first area finishes.
///
/// Idempotent and shared with [`write_area`], which asks the same question again on the path that
/// must not depend on the driver having asked it. Calling it twice costs nothing: the work is
/// remembered.
///
/// # Errors
///
/// Returns the same explanatory failure [`write_area`] would have returned later.
pub fn prepare_namespace() -> HarnessResult<()> {
    ensure_report_namespace("preparing the run report directory")
}

/// Establish this run's ownership of the report directory, clearing an earlier run's artifacts.
///
/// Runs exactly once per process — [`OnceLock::get_or_init`] blocks every other feature-area thread
/// until the first one has finished, which is the whole of the coordination needed — and its outcome
/// is remembered so a failure is reported identically to every caller rather than retried
/// fourteen times.
///
/// # Errors
///
/// Returns an explanatory failure when the report root cannot be established or verified, when
/// another run still owns it, or when an earlier run's artifacts cannot be cleared.
fn ensure_report_namespace(context: &str) -> HarnessResult<()> {
    static PREPARED: OnceLock<Result<(), String>> = OnceLock::new();
    match PREPARED
        .get_or_init(|| prepare_report_namespace().map_err(|error| String::from(error.cause())))
    {
        Ok(()) => Ok(()),
        Err(cause) => Err(HarnessError::new(
            String::from(context),
            format!("the report directory for this run could not be prepared: {cause}"),
        )),
    }
}

/// Clear the previous run's report artifacts and claim the directory for this one.
///
/// The order is load-bearing, and the **claim comes first**.
///
/// The report paths are deterministic so that a row leads straight to a file, which means two
/// concurrent runs against one build directory address the same ones — and everything below this
/// point is destructive. Detecting a live foreign owner and *then* clearing cannot establish
/// exclusivity: the detection and the clearing are two steps, so two runs starting together both find
/// no owner, both clear, and each destroys the artifacts the other is writing. `claim_run_namespace`
/// is a single operation only one of two contenders can win, so it decides that before the first
/// removal, and the claim is held for the life of the process by [`report_namespace_claim`].
///
/// The ownership stamp written at the end is not redundant with the claim: the claim answers "may I
/// clear this now" and exists only while this run does, while the stamp answers "whose reports are
/// these" and is what a reader of a directory left behind by a finished run has. The live-owner check
/// is kept for the same reason, and it runs **after** the claim so that its answer cannot change
/// underneath this run. A crash midway still leaves a directory the next run will clear again rather
/// than one another run believes is owned.
///
/// Clearing is what makes aggregation honest. `try_finalize` writes the summary once the area file of
/// every area this invocation selected exists, so without clearing, files from an earlier run
/// standing beside one from this run would be a complete-looking set — thirteen stale plus one fresh
/// in a full run, and fewer than that in a filtered one, where the selected set is smaller and
/// therefore easier to complete by accident. A filtered run publishes a summary over the areas it
/// selected and stamps it **partial**, listing every area that did not contribute; what clearing
/// guarantees is that those contributions are all this run's own.
///
/// Every removal is addressed through **one pinned handle** on the report root rather than by name.
/// The root's name is predictable and this function removes two summaries, a whole directory tree and
/// any temporary debris through it, so a name-based sequence would re-resolve the root four times and
/// leave three windows in which an intermediate level could be replaced — aiming a recursive removal
/// outside the build tree while every diagnostic still named the report directory.
fn prepare_report_namespace() -> HarnessResult<()> {
    let context = "preparing the report directory for this run";
    let root = report_root();
    create_directory_chain_below(context, &root, &root)?;

    // Held for the life of the process: the claim is what keeps a second run out of these paths for
    // as long as this one is writing them, so releasing it at the end of this function would protect
    // only the clearing and not the fourteen area reports that follow.
    let claim = claim_run_namespace(context, &root)?;

    if let Some((run, pid)) = live_foreign_owner_identity(&root) {
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "{} is already owned by run {} (process {}), which is still running. The report \
                 paths are deterministic so that a row leads straight to a file, which means two \
                 concurrent runs address the same ones; clearing them here would destroy the \
                 artifacts that run is still writing, and leaving them would let its rows be \
                 aggregated into this run's summary. Let that run finish, or set CARGO_TARGET_DIR \
                 to a different build directory for this one",
                shown_path(&root),
                sanitize_text_for_report(&run),
                pid
            ),
        ));
    }

    let pinned = PinnedDirectory::pin(context, &root)?;
    for stale in [summary_markdown_name(), summary_tsv_name()] {
        require_replaceable(
            context,
            &pinned.shown_entry(&stale),
            Replaceable::RegularFile,
        )?;
        pinned.remove_within(context, &stale)?;
    }
    // Three directories rather than one, and the two beyond `areas/` are not an afterthought.
    // Everything beneath this root is named deterministically, so an earlier run's document does not
    // collide with this run's — it *survives beside* it, and a reader of the uploaded artifact cannot
    // tell which run produced which file. For an area report that would corrupt an aggregate, which is
    // why it was always cleared; for an evidence document and a finding's review copy it is worse in a
    // different way, because both are evidence: a marker retired since the last run, or a finding that
    // has stopped diverging, would keep publishing its old evidence into every later report as though
    // this run had observed it. Clearing all three is also what makes this suite's own statement true —
    // `tests/conformance/FINDINGS.md` §4 says the report root is emptied whole at the start of every
    // run — and each removal keeps the same guard as the first: the entry must be a real directory this
    // module put there, addressed through the one pinned handle.
    for directory in [AREAS_DIR_NAME, EVIDENCE_DIR_NAME, FINDING_BUNDLE_DIR_NAME] {
        require_replaceable(
            context,
            &pinned.shown_entry(directory),
            Replaceable::Directory,
        )?;
        pinned.remove_within(context, directory)?;
    }
    purge_stale_temporaries(context, &pinned)?;
    create_directory_chain_below(context, &root, &areas_dir())?;
    claim_ownership(context, &root)?;
    // Stored only now, after everything that could fail has succeeded. A claim recorded before a
    // failed clearing would be released by the process exit rather than by this run's own accounting,
    // and a reader of the failure would have no way to tell whether the directory had been claimed.
    remember_report_claim(claim);
    Ok(())
}

/// Keep this run's claim on the report root alive for the life of the process.
///
/// A [`RunClaim`] releases itself when it is dropped, which is exactly right for a workspace whose
/// lifetime is one cell — and exactly wrong for the report root, whose lifetime is the whole run. So
/// the value is moved somewhere that outlives every area thread. It is never read: holding it *is* the
/// guarantee.
///
/// Rust runs no destructor for such a value at process exit, so the claim entry is still present when
/// the run ends. That is a designed outcome rather than a leak — no area is "last" in a filtered or a
/// failing run, so there is nowhere honest to release it from — and [`RunClaim`]'s own documentation
/// carries the argument: the next run reclaims a claim whose process is provably gone, and still
/// refuses one whose process is alive.
fn remember_report_claim(claim: RunClaim) {
    static HELD: OnceLock<Mutex<Vec<RunClaim>>> = OnceLock::new();
    let held = HELD.get_or_init(|| Mutex::new(Vec::new()));
    match held.lock() {
        Ok(mut held) => held.push(claim),
        // A poisoned mutex means an area thread panicked while holding it, which cannot happen here —
        // nothing between the lock and the push can panic — but the claim must not be dropped on the
        // strength of that reasoning, because dropping it would release the namespace while the run is
        // still writing into it. Leaking it is the safe direction: the entry is reclaimed as stale by
        // the next run, whose protocol for exactly that is already exercised.
        Err(poisoned) => poisoned.into_inner().push(claim),
    }
}

/// Remove any temporary file a crashed earlier run left at the report root.
///
/// A temporary name carries the writing run's generation token and a counter unique within that
/// run, so a leftover can never be reused or collided with; it would simply accumulate. Removing them here keeps the claim that a failed write
/// leaves no debris true across runs as well as within one. An entry that is not one of ours by name
/// is left alone, and the ownership stamp is not one of ours by name.
///
/// Takes the report root as a **pinned directory** rather than as a path, so the listing and every
/// removal that follows from it address the same held object. Listing by name and then removing by
/// name are two resolutions of one predictable path: between them the root can be replaced, and the
/// removals — aimed at names this function found in the *previous* directory — would then execute in
/// the substitute. Since the names it acts on come from the listing, the listing and the removals have
/// to describe one directory or neither answer means anything.
fn purge_stale_temporaries(context: &str, root: &PinnedDirectory) -> HarnessResult<()> {
    for name in root.entry_names(context)? {
        if name == RUN_OWNER_ENTRY || !name.starts_with('.') || !name.ends_with(TEMPORARY_SUFFIX) {
            continue;
        }
        root.remove_within(context, &name)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Writing
//
// Every artifact is produced whole in memory and then published by a single rename. Two
// properties follow, and both are needed rather than merely tidy: a reader never observes a
// partially written report, and a writer that fails leaves the previous report intact instead of
// replacing it with a truncated one. A report is the only account of a run that outlives the
// process, so a torn report is worse than no new report at all.
// ---------------------------------------------------------------------------------------------

/// Suffix of every temporary file published beneath the report root, and the only name this module
/// will clean up.
///
/// The shared publisher builds its temporary names from the destination's own name, this run's
/// generation token and a counter unique within the run, and ends every one of them with this
/// suffix. Nothing in this module creates a file it does not then rename, so a name ending this way
/// beneath the report root is always debris from a run that died mid-write.
const TEMPORARY_SUFFIX: &str = ".tmp";

/// Publish a Markdown report and its machine-readable sibling as one operation.
///
/// # Why the pair is published together and not one file at a time
///
/// Both documents are rendered from a single snapshot, so their contents always agree at the moment
/// they are produced. Publishing them as two independent operations throws that away: a reader who
/// listed the directory between the two renames would find a new Markdown report beside the previous
/// run's tab-separated one — two complete files that disagree, with nothing in either to say they do
/// not belong together. The machine-readable sibling exists precisely so that totals can be
/// aggregated without re-reading prose, which makes a silent disagreement between them the one
/// inconsistency most likely to be believed.
///
/// So both are written and flushed **first**, through the harness's no-follow publisher, and only then
/// are the two renames performed one after the other with no work between them. Every way a write can
/// fail has already happened by that point, so the failure modes that could leave the pair
/// *permanently* mismatched are gone, and the remaining exposure is the gap between two adjacent
/// syscalls. Two renames are not one atomic operation and nothing here pretends they are; a
/// directory-level swap would be required for that and `std` does not offer one portably.
///
/// A staged publication that is never committed removes its own temporary file, so a failure to stage
/// the second document leaves the first destination untouched rather than half-updated.
///
/// # Why publication refuses to follow a link
///
/// A report path is fixed and public — `areas/<area>.md` and `summary.tsv` beneath the report root —
/// and so is the temporary name beside it. Anything able to write in the build directory can therefore
/// predict where a report is about to be published and plant a symbolic link there first. A plain
/// write follows such a link, so the run's only durable account of itself would be delivered to
/// wherever the link pointed, while the harness reported success because the write did succeed.
///
/// Four guards stand between a caller and the filesystem, and each closes a distinct way the write
/// could be *redirected* rather than merely fail:
///
/// - the destination is required to lie strictly beneath [`report_root`], with every component below
///   it a plain name, so nothing can be published outside the build directory and no
///   parent-directory component can climb out of it;
/// - every directory from that root down to the parent is created and then required to be a **real
///   directory** rather than a symbolic link, so a link planted at any level is refused *at* that
///   level and nothing is written beyond it;
/// - the destination is required to be absent or a regular file, because this module never puts
///   anything else there and repairing it silently would discard the only evidence that something
///   else did;
/// - the temporary is created with `O_CREAT | O_EXCL`, which fails rather than follows, so a
///   pre-planted temporary sibling cannot be truncated or written through.
///
/// # Errors
///
/// Returns an explanatory failure naming `context` when the report namespace cannot be established,
/// when a path is not one this module may write, when the parent chain cannot be created or
/// verified, when something unexpected occupies a destination, when either document cannot be
/// staged, or when either rename fails. A failure of the *second* rename is reported in terms of the
/// inconsistency it leaves, because that is the one case a reader has to know about.
fn write_report_pair(
    context: &str,
    markdown_path: &Path,
    markdown: &str,
    tsv_path: &Path,
    tsv: &str,
) -> HarnessResult<()> {
    ensure_report_namespace(context)?;
    for path in [markdown_path, tsv_path] {
        require_beneath_report_root(context, path)?;
        let parent = path.parent().ok_or_else(|| {
            HarnessError::new(
                String::from(context),
                format!(
                    "{} has no parent directory, so it cannot name a report artifact; every \
                     artifact is published into a directory beneath the build directory",
                    shown_path(path)
                ),
            )
        })?;
        create_directory_chain_below(context, &report_root(), parent)?;
        require_replaceable(context, path, Replaceable::RegularFile)?;
    }

    // One call, because the guarantee is about the pair rather than about either half. It stages both
    // documents before claiming either, commits them back to back, and — the part two adjacent renames
    // cannot give on their own — puts the Markdown half back if the machine-readable half cannot be
    // published, so a half-failed publication never leaves a new document standing beside a stale
    // sibling for a later reader or an unconditional artifact upload to collect.
    super::publish_pair_no_follow(
        context,
        markdown_path,
        markdown.as_bytes(),
        tsv_path,
        tsv.as_bytes(),
    )
}

// ---------------------------------------------------------------------------------------------
// Durable evidence for outcomes whose workspace is not retained
//
// A cell workspace is retained when the cell has something to investigate and discarded otherwise, and
// "something to investigate" is decided by the run's policy. That leaves a class of outcomes that are
// *reported* but do not fail: an expected divergence, and a permissive run's absent oracle. Their raw
// compiler diagnostics and termination records lived only in the workspace, so they were deleted the
// moment the cell concluded — and the report row that named the workspace as their location was
// published pointing at a directory that no longer existed. Continuous integration made it starker:
// the report root is uploaded, the workspace root is not.
//
// So each such outcome now gets a small document inside the report root, holding what the row holds
// plus the archived content of the captures the cell persisted. The document is sanitized, bounded,
// deterministically named from the same slug the workspace uses, and counted so the summary can state
// how much evidence a run published and whether any of it was refused.

/// Longest evidence document one cell may publish, in bytes.
///
/// The archive it is rendered from is already bounded per entry and in total by `execute.rs`; this is
/// the bound on the rendered document, which adds a header and one section per entry. A document that
/// would exceed it is truncated with a final line saying so, so the file is never silently short.
const EVIDENCE_DOCUMENT_BYTES_MAX: usize = 384 * 1024;

/// Longest total evidence one run may publish, in bytes, across every document.
///
/// The matrix is 1,296 cells and three oracles, so a run in which a marker fires on every arm would
/// otherwise publish 3,888 documents with no aggregate ceiling. 64 MiB is far more than any real run
/// needs and still bounds the pathological one; documents beyond it are refused, and the refusal is
/// reported in the summary rather than passed over.
const EVIDENCE_RUN_BYTES_MAX: u64 = 64 * 1024 * 1024;

/// Bytes of evidence this run has published so far.
static EVIDENCE_BYTES: AtomicU64 = AtomicU64::new(0);

/// Documents of evidence this run has published so far.
static EVIDENCE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Everything this run could not publish as evidence, and why.
fn evidence_refusals() -> &'static Mutex<Vec<String>> {
    static REFUSALS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    REFUSALS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Documents and bytes of evidence this run has published.
pub fn evidence_totals() -> (u64, u64) {
    (
        EVIDENCE_COUNT.load(Ordering::Relaxed),
        EVIDENCE_BYTES.load(Ordering::Relaxed),
    )
}

/// Everything this run declined to publish as evidence.
///
/// Consulted once, when the summary is assembled, for the same reason the retention prunings are: a
/// piece of evidence that was refused must be stated in the deliverable rather than discovered by a
/// maintainer who went looking for a document that is not there.
pub fn evidence_refusal_notes() -> Vec<String> {
    match evidence_refusals().lock() {
        Ok(held) => held.clone(),
        // A poisoned lock means a thread panicked while recording a refusal. The refusals already
        // recorded are still true, so they are recovered rather than discarded.
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// Record one refusal so that it reaches the run summary.
fn note_evidence_refusal(note: String) -> String {
    match evidence_refusals().lock() {
        Ok(mut held) => held.push(note.clone()),
        Err(poisoned) => poisoned.into_inner().push(note.clone()),
    }
    note
}

/// Publish one outcome's evidence to a durable document beneath the report root.
///
/// Returns the sentence to print beside the cell: the path on success, the reason on refusal. Nothing
/// is raised, and that is deliberate rather than lax — this runs while a cell is being retired, after
/// its verdicts are decided, and a decided cell must not be re-decided by a problem with its own
/// archiving. The refusal is instead recorded for the summary, so it is reported at run scope where it
/// is a fact about the run rather than about the cell.
///
/// `reason` states why the workspace this evidence came from is not being kept, so the document
/// explains its own existence to whoever opens it.
pub fn publish_cell_evidence(
    outcome: &Outcome,
    archived: &[super::execute::ArchivedCapture],
    reason: &str,
) -> String {
    let key = outcome.key();
    let path = evidence_document_path(key, outcome.oracle());
    let context = format!(
        "publishing durable evidence for {}/{} @ {} {} oracle_{}",
        key.area(),
        key.program(),
        key.target().triple(),
        key.opt().flag(),
        outcome.oracle().letter()
    );

    let document = render_evidence_document(outcome, archived, reason);
    let bytes = document.len() as u64;
    let charged = EVIDENCE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    if charged > EVIDENCE_RUN_BYTES_MAX {
        // Give the charge back, so one refused document does not close the sink for every later one.
        EVIDENCE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        return note_evidence_refusal(format!(
            "evidence for {} was not published: it would have taken this run past the \
             {EVIDENCE_RUN_BYTES_MAX}-byte evidence ceiling ({charged} bytes charged). Re-run this \
             cell on its own, or set {VAR_KEEP_WORK}, to obtain it",
            sanitize_text_for_report(&context)
        ));
    }

    if let Err(error) = ensure_report_namespace(&context) {
        EVIDENCE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        return note_evidence_refusal(format!(
            "evidence for {} was not published: {}",
            sanitize_text_for_report(&context),
            error.cause()
        ));
    }
    if let Err(error) = create_directory_chain_below(&context, &report_root(), &evidence_dir())
        .and_then(|()| require_replaceable(&context, &path, Replaceable::RegularFile))
        .and_then(|()| super::publish_bytes_no_follow(&context, &path, document.as_bytes()))
    {
        EVIDENCE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        return note_evidence_refusal(format!(
            "evidence for {} was not published: {}",
            sanitize_text_for_report(&context),
            error.cause()
        ));
    }
    EVIDENCE_COUNT.fetch_add(1, Ordering::Relaxed);
    format!("evidence retained: {} ({bytes} byte(s))", shown_path(&path))
}

/// Render one outcome's evidence document.
///
/// Every field is already sanitized where it came from — the provenance by [`super::SideRecord`], the
/// archive by `execute.rs` — and the assembled document is passed through sanitization once more, for
/// the reason the whole module applies it at sinks rather than at sources: a rule applied at some sinks
/// is a rule the next sink will be written without.
fn render_evidence_document(
    outcome: &Outcome,
    archived: &[super::execute::ArchivedCapture],
    reason: &str,
) -> String {
    let key = outcome.key();
    let mut text = String::new();
    text.push_str("# Durable evidence for one comparison\n\n");
    text.push_str(&format!("area = {}\n", key.area()));
    text.push_str(&format!("program = {}\n", key.program()));
    text.push_str(&format!(
        "source = {CORPUS_RELATIVE_ROOT}/{}/{}.{SOURCE_EXTENSION}\n",
        key.area(),
        key.program()
    ));
    text.push_str(&format!(
        "record = {CORPUS_RELATIVE_ROOT}/{}/{}.{RECORD_EXTENSION}\n",
        key.area(),
        key.program()
    ));
    text.push_str(&format!("target = {}\n", key.target().triple()));
    text.push_str(&format!("opt_level = {}\n", key.opt().flag()));
    text.push_str(&format!("oracle = oracle_{}\n", outcome.oracle().letter()));
    text.push_str(&format!("verdict = {}\n", outcome.verdict().label()));
    if let Some(class) = outcome.class() {
        text.push_str(&format!("divergence_class = {}\n", class.label()));
    }
    if let Some(marker) = outcome.marker_id() {
        text.push_str(&format!("marker = {marker}\n"));
    }
    text.push_str(&format!(
        "workspace = {}\n",
        shown_path(&workspace_path(key))
    ));
    text.push_str(&format!("why_this_document_exists = {reason}\n"));

    if let Some(provenance) = outcome.provenance() {
        for (role, side) in [
            ("subject", provenance.subject()),
            ("authority", provenance.authority()),
        ] {
            if let Some(side) = side {
                text.push_str(&format!("\n[{role}]\n"));
                text.push_str(&format!("who = {}\n", side.role()));
                text.push_str(&format!("command = {}\n", side.command()));
                text.push_str(&format!("termination = {}\n", side.termination()));
                text.push_str(&format!("captures = {}\n", side.captures()));
            }
        }
        if !provenance.first_difference().is_empty() {
            text.push_str(&format!(
                "\nfirst_difference = {}\n",
                provenance.first_difference()
            ));
        }
    }

    text.push_str("\n[detail]\n");
    text.push_str(outcome.detail());
    text.push('\n');

    for capture in archived {
        text.push_str(&format!(
            "\n[capture {}] {} byte(s) on disk{}\n",
            capture.name(),
            capture.bytes(),
            // Three distinct states, said in three distinct ways, because a reader acts differently
            // on each: this is the whole entry, this is its beginning, or this is a description of an
            // entry no text document could carry faithfully.
            match (capture.binary(), capture.truncated()) {
                (true, _) => ", described rather than transcribed",
                (false, true) => ", archived prefix only",
                (false, false) => "",
            }
        ));
        text.push_str(capture.text());
        if !capture.text().ends_with('\n') {
            text.push('\n');
        }
    }

    let mut document = sanitize_document_for_evidence(&text);
    if document.len() > EVIDENCE_DOCUMENT_BYTES_MAX {
        // Truncate on a character boundary, then say so. A silently shortened document is evidence a
        // reader can draw a wrong conclusion from; a document that states its own truncation is not.
        let mut cut = EVIDENCE_DOCUMENT_BYTES_MAX;
        while cut > 0 && !document.is_char_boundary(cut) {
            cut -= 1;
        }
        document.truncate(cut);
        document.push_str(&format!(
            "\n[truncated] this document reached the {EVIDENCE_DOCUMENT_BYTES_MAX}-byte limit for \
             one cell's evidence; re-run this cell with {VAR_KEEP_WORK} set to obtain the whole of \
             it\n"
        ));
    }
    document
}

/// Make an assembled evidence document safe to publish, line by line.
///
/// Applied to the whole document rather than to each field, and it keeps line feeds while escaping
/// everything else sanitization escapes — because unlike a report row, this artifact *is*
/// multi-line: a compiler's diagnostic output is only readable with its line structure intact, and
/// collapsing it would defeat the purpose of archiving it. Every other forgeable character, and every
/// credential-bearing value, is still removed, by the same two functions the report rows use.
fn sanitize_document_for_evidence(text: &str) -> String {
    let redacted = redact_secrets(text);
    redacted
        .lines()
        .map(sanitize_text_for_report)
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------------
// The review copy of a finding
//
// A finding is the one verdict the requirements call a *deliverable*: keep the program, keep both
// sides' output, keep the exact reproduction commands. `findings.rs` does all of that, into
// `<build>/conformance-findings/<id>/`, and that directory is complete — seven artifact classes,
// checked against its own manifest at the moment it is written and again immediately before any
// report advertises it.
//
// It was, however, the one deliverable a run could publish and then lose. A FINDING does not fail the
// run, so a run carrying findings *passes*; the cell's workspace is retained, which means the cell
// never reaches the evidence path above; and the generated finding root is not the directory
// continuous integration uploads. The report named the directory in its findings table and the
// directory went with the runner. Nothing was wrong with the finding — it simply reached nobody, which
// is the same defect the evidence documents above exist to close, arriving through the one door they
// did not cover.
//
// So every finding now also gets a **review copy** beneath the report root, and the two copies are
// deliberately different things rather than duplicates:
//
// - The **generated** directory holds the exact bytes and the exact commands. It is what a
//   reproduction runs, and `report.rs` says elsewhere why nothing in it is redacted: a redacted
//   command is not a runnable command, and captured bytes compared against redacted ones report a
//   divergence in the compiler. It stays git-ignored, stays behind the workflow's explicit opt-in, and
//   keeps its manifest's `disclosure_review = not-performed`.
// - The **review copy** holds the same seven classes rendered to *report grade*: redacted, sanitized,
//   bounded, and — where a capture is not text at all — described rather than transcribed. It is what
//   a reader of the uploaded report gets without asking, and it is enough to read a finding, judge it
//   and decide whether to ask for the exact bytes. [`BUNDLE_INDEX_NAME`] states that distinction
//   inside every copy, so the copy cannot be mistaken for the evidence it points at.
//
// That preserves the disclosure gate rather than routing around it. The gate exists because a curated
// finding is committed and permanent, and `FINDINGS.md` §5.3 requires a person to read every byte
// before it lands; publishing the *unredacted* directory by default would hand that judgement to a
// workflow. A report-grade copy makes no claim the report does not already make about itself, and the
// copy's own index repeats the report's bound: sanitized against the named forms, with the narrow
// residual class `mod.rs` enumerates, so inspect it before republishing it outside the repository.

/// Longest one artifact of a review copy may be, in bytes, after rendering.
///
/// The same ceiling an evidence document gets, and for the same reason: these are the two things this
/// module publishes from bytes it did not itself compose. A capture is already bounded where it was
/// produced, so this bites only on the pathological one, and an artifact that reaches it is truncated
/// with a final line saying so — never silently shortened.
const BUNDLE_ARTIFACT_BYTES_MAX: usize = 384 * 1024;

/// Longest total the review copies of one run may take, in bytes.
///
/// A run against a compiler that diverges everywhere files a finding per cell and class, and
/// `findings.rs` bounds that at 1,536 directories and a gibibyte. This is the report root's own,
/// smaller ceiling on the *copies*: 64 MiB, chosen to match the evidence ceiling beside it, which is
/// far more than any real run needs and still bounds the pathological one. What does not fit is
/// refused and the refusal is reported in the summary, because the generated directories still hold
/// everything and the run must say which copies it did not publish.
const BUNDLE_RUN_BYTES_MAX: u64 = 64 * 1024 * 1024;

/// Most entries of one finding's `outputs/` directory a review copy carries.
///
/// One capture per side per stream per cell, and a finding is one cell, so a real directory holds a
/// handful. The ceiling exists so that a directory that somehow holds thousands cannot turn one
/// finding's copy into the whole run's budget before the byte ceiling notices.
const BUNDLE_CAPTURE_COUNT_MAX: usize = 64;

/// Bytes of review copy this run has published.
static BUNDLE_BYTES: AtomicU64 = AtomicU64::new(0);

/// Artifact files of review copies this run has published.
static BUNDLE_FILES: AtomicU64 = AtomicU64::new(0);

/// Findings whose review copy this run has published.
static BUNDLE_FINDINGS: AtomicU64 = AtomicU64::new(0);

/// Findings whose review copy has already been published, so a second oracle does not publish it again.
///
/// A finding identifier is derived from the cell and the divergence class, never from the oracle, so
/// one directory answers for every arm that observed the divergence — up to three per cell. Publishing
/// it once per arm would charge the run's budget three times for identical bytes.
fn published_bundles() -> &'static Mutex<BTreeSet<String>> {
    static PUBLISHED: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
    PUBLISHED.get_or_init(|| Mutex::new(BTreeSet::new()))
}

/// Claim the right to publish this finding's review copy, or report that it is already published.
fn claim_bundle(id: &FindingId) -> bool {
    match published_bundles().lock() {
        Ok(mut held) => held.insert(String::from(id.as_str())),
        // A poisoned lock means a thread panicked while claiming. Publishing again writes identical
        // bytes to the same paths, so the safe direction is to allow it: a duplicated charge against a
        // 64 MiB ceiling is a smaller problem than a finding whose copy was never published because a
        // lock was poisoned somewhere else.
        Err(poisoned) => poisoned.into_inner().insert(String::from(id.as_str())),
    }
}

/// Findings, files and bytes of review copy this run has published.
pub fn finding_bundle_totals() -> (u64, u64, u64) {
    (
        BUNDLE_FINDINGS.load(Ordering::Relaxed),
        BUNDLE_FILES.load(Ordering::Relaxed),
        BUNDLE_BYTES.load(Ordering::Relaxed),
    )
}

/// What became of one artifact on its way into a review copy.
enum Carriage {
    /// Carried whole, and rendering changed nothing.
    Whole(u64),
    /// Carried whole, but rendering changed it — a redaction, or a byte written as a visible escape.
    Rendered(u64),
    /// Carried as a prefix, because the whole of it exceeded the per-artifact ceiling.
    Truncated { original: u64, carried: usize },
    /// Described rather than transcribed, because the bytes are not text.
    Described(u64),
    /// Not carried, with the reason.
    Refused(String),
}

impl Carriage {
    /// The sentence [`BUNDLE_INDEX_NAME`] records for this artifact.
    fn describe(&self) -> String {
        match self {
            Carriage::Whole(bytes) => {
                format!("carried whole, {bytes} byte(s), rendering changed nothing")
            }
            Carriage::Rendered(bytes) => format!(
                "carried whole, {bytes} byte(s), rendered for a report — a redaction or a visible \
                 escape changed it, so read it rather than run it"
            ),
            Carriage::Truncated { original, carried } => format!(
                "carried as a prefix: {carried} of {original} byte(s), past the \
                 {BUNDLE_ARTIFACT_BYTES_MAX}-byte per-artifact ceiling"
            ),
            Carriage::Described(bytes) => format!(
                "described rather than transcribed: {bytes} byte(s) that are not text, so no report \
                 could carry them faithfully"
            ),
            Carriage::Refused(reason) => format!("NOT carried: {reason}"),
        }
    }

    /// Whether anything reached the review copy for this artifact.
    fn carried(&self) -> bool {
        !matches!(self, Carriage::Refused(_))
    }
}

/// Publish the review copy of one finding beneath the report root.
///
/// Returns the sentence to print beside the cell, or `None` when this finding's copy has already been
/// published by an earlier arm of the same divergence.
///
/// Nothing is raised, deliberately and for the same reason [`publish_cell_evidence`] raises nothing:
/// this runs while a cell is being retired, after its verdicts are decided, and a decided cell must not
/// be re-decided by a problem with its own archiving. A refusal is recorded for the summary instead, at
/// run scope, where a copy that was not published is a fact about the run.
///
/// The generated directory is **read, never moved**: the finding itself stays exactly where
/// `findings.rs` published it, complete and unredacted, so the checks that validate it — at write time
/// and again before any report advertises it — go on seeing the artifact they were written for.
pub fn publish_finding_bundle(id: &FindingId) -> Option<String> {
    if !claim_bundle(id) {
        return None;
    }
    let source = id.directory();
    let destination = finding_bundle_dir(id);
    let context = format!("publishing the review copy of finding {id}");

    if let Err(error) = ensure_report_namespace(&context) {
        return Some(note_evidence_refusal(format!(
            "the review copy of finding {} was not published: {}",
            sanitize_text_for_report(id.as_str()),
            error.cause()
        )));
    }

    let mut carried: Vec<(String, Carriage)> = Vec::new();
    for name in findings::REQUIRED_ARTIFACTS
        .iter()
        .filter(|name| **name != findings::OUTPUTS_DIR_NAME)
    {
        let state = copy_bundle_artifact(&context, &source, &destination, name, name);
        carried.push((String::from(*name), state));
    }
    for name in bundle_capture_names(&context, &source, &mut carried) {
        let shown = format!("{}/{name}", findings::OUTPUTS_DIR_NAME);
        let state = copy_bundle_artifact(
            &context,
            &source.join(findings::OUTPUTS_DIR_NAME),
            &destination.join(findings::OUTPUTS_DIR_NAME),
            &name,
            &shown,
        );
        carried.push((shown, state));
    }

    // Written last, because it reports what happened to everything above it. Its own bytes are charged
    // like any other artifact's, and a run that cannot write it says so in the returned sentence: a
    // copy whose index is missing is still readable, but nothing in it would state its grade.
    let index = render_bundle_index(id, &source, &carried);
    let index_state = write_bundle_bytes(&context, &destination, BUNDLE_INDEX_NAME, &index);

    let published = carried.iter().filter(|(_, state)| state.carried()).count();
    let refused: Vec<&str> = carried
        .iter()
        .filter(|(_, state)| !state.carried())
        .map(|(name, _)| name.as_str())
        .collect();
    BUNDLE_FINDINGS.fetch_add(1, Ordering::Relaxed);

    let mut sentence = format!(
        "finding review copy published: {} ({published} artifact file(s))",
        shown_path(&destination)
    );
    if let Err(reason) = &index_state {
        sentence.push_str(&format!(
            "; its {BUNDLE_INDEX_NAME} could not be written, so the copy does not state its own \
             grade: {reason}"
        ));
    }
    if !refused.is_empty() {
        // Named in the sentence *and* recorded at run scope: the first tells whoever is watching the
        // run, the second reaches the summary, which is what a reader of the artifact has.
        let note = note_evidence_refusal(format!(
            "the review copy of finding {} is incomplete: {} was not carried. The generated \
             directory still holds all of it — publish it with the workflow's raw-findings opt-in, or \
             read it beside the run",
            sanitize_text_for_report(id.as_str()),
            refused.join(", ")
        ));
        sentence.push_str(&format!("; {note}"));
    }
    Some(sanitize_text_for_report(&sentence))
}

/// Every capture name in one finding's `outputs/` directory, in a fixed order.
///
/// Listed through a pinned handle rather than by name, so a directory substituted between the listing
/// and the reads is refused at the listing rather than read through. Sorted, because the copy is
/// compared between runs and a filesystem's own enumeration order is not a property of the finding. A
/// directory that cannot be listed, and every entry past the count ceiling, is recorded as a refusal
/// against the `outputs/` class itself rather than silently omitted.
fn bundle_capture_names(
    context: &str,
    source: &Path,
    carried: &mut Vec<(String, Carriage)>,
) -> Vec<String> {
    let captures = source.join(findings::OUTPUTS_DIR_NAME);
    let pinned = match PinnedDirectory::pin(context, &captures) {
        Ok(pinned) => pinned,
        Err(error) => {
            carried.push((
                format!("{}/", findings::OUTPUTS_DIR_NAME),
                Carriage::Refused(format!(
                    "{} could not be held open, so no capture could be copied from it: {}",
                    shown_path(&captures),
                    error.cause()
                )),
            ));
            return Vec::new();
        }
    };
    let mut names = match pinned.entry_names(context) {
        Ok(names) => names,
        Err(error) => {
            carried.push((
                format!("{}/", findings::OUTPUTS_DIR_NAME),
                Carriage::Refused(format!(
                    "{} could not be listed, so no capture could be copied from it: {}",
                    shown_path(&captures),
                    error.cause()
                )),
            ));
            return Vec::new();
        }
    };
    names.sort();
    if names.len() > BUNDLE_CAPTURE_COUNT_MAX {
        let dropped = names.split_off(BUNDLE_CAPTURE_COUNT_MAX);
        carried.push((
            format!("{}/", findings::OUTPUTS_DIR_NAME),
            Carriage::Refused(format!(
                "{} holds {} entries, past the {BUNDLE_CAPTURE_COUNT_MAX} this copy carries; {} \
                 further entry(ies) were left in the generated directory",
                shown_path(&captures),
                BUNDLE_CAPTURE_COUNT_MAX + dropped.len(),
                dropped.len()
            )),
        ));
    }
    names
}

/// Copy one artifact of a finding into its review copy, rendering it for a report on the way.
///
/// `source` and `destination` are the artifact's directory in each copy — the same relative position in
/// the copy as in the original — `name` is its entry name in both, and `shown` is how the index names
/// it, which for a capture carries the `outputs/` prefix the reader expects.
fn copy_bundle_artifact(
    context: &str,
    source: &Path,
    destination: &Path,
    name: &str,
    shown: &str,
) -> Carriage {
    let entry = source.join(name);
    let observed = match fs::symlink_metadata(&entry) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Carriage::Refused(format!(
                "{} could not be inspected: {error}",
                shown_path(&entry)
            ))
        }
    };
    if !observed.is_file() {
        return Carriage::Refused(format!(
            "{} is not a regular file this run wrote, so it is refused rather than followed",
            shown_path(&entry)
        ));
    }
    let original = observed.len();
    // The ceiling one artifact of a *generated* finding may hold, so an artifact this run published
    // always fits and anything larger is refused rather than held in memory.
    let bytes = match read_file_bounded(context, &entry, findings::FINDING_ARTIFACT_BYTES_MAX) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Carriage::Refused(format!(
                "{} could not be read: {}",
                shown_path(&entry),
                error.cause()
            ))
        }
    };

    let (document, carriage) = match String::from_utf8(bytes) {
        Ok(text) => {
            let rendered = render_artifact_for_review(&text);
            let changed = rendered != text;
            if rendered.len() > BUNDLE_ARTIFACT_BYTES_MAX {
                let mut cut = BUNDLE_ARTIFACT_BYTES_MAX;
                while cut > 0 && !rendered.is_char_boundary(cut) {
                    cut -= 1;
                }
                let mut prefix = String::from(&rendered[..cut]);
                prefix.push_str(&format!(
                    "\n[truncated] this is the first {cut} byte(s) of {shown}, which reached the \
                     {BUNDLE_ARTIFACT_BYTES_MAX}-byte ceiling one artifact of a review copy may take. \
                     The whole of it is in the generated finding directory named in \
                     {BUNDLE_INDEX_NAME}\n"
                ));
                (
                    prefix,
                    Carriage::Truncated {
                        original,
                        carried: cut,
                    },
                )
            } else if changed {
                (rendered, Carriage::Rendered(original))
            } else {
                (rendered, Carriage::Whole(original))
            }
        }
        Err(_) => (
            format!(
                "[described] {shown} holds {original} byte(s) that are not valid UTF-8, so no report \
                 could carry them faithfully and this copy does not try. The exact bytes are in the \
                 generated finding directory named in {BUNDLE_INDEX_NAME}, and the finding's own \
                 {} records this entry's size and digest, which is what a comparison should be made \
                 against.\n",
                findings::MANIFEST_NAME
            ),
            Carriage::Described(original),
        ),
    };

    match write_bundle_bytes(context, destination, name, &document) {
        Ok(()) => carriage,
        Err(reason) => Carriage::Refused(reason),
    }
}

/// Render one artifact of a finding for a report, keeping its final newline.
///
/// [`sanitize_document_for_evidence`] rebuilds a document from its lines, which drops a trailing
/// newline — harmless where that function assembles a document of its own, and not harmless here. An
/// artifact of a review copy stands in for a file a maintainer will read beside the original, and a
/// copy one byte shorter than the file it copies is a difference a reader has to explain: a captured
/// stdout whose last line has quietly lost its terminator looks like exactly the kind of divergence
/// this suite exists to report. Restoring it also makes the copy byte-identical to its source whenever
/// nothing was actually redacted or escaped, which is what lets [`Carriage`] say so honestly instead of
/// claiming a change that never happened.
fn render_artifact_for_review(text: &str) -> String {
    let mut rendered = sanitize_document_for_evidence(text);
    if text.ends_with('\n') && !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

/// Publish one rendered document into a review copy, charging the run's budget for it.
///
/// Returns the reason on any failure. Publication takes the same four guards every other write beneath
/// the report root takes — strictly beneath the root, every directory on the way proved to be a real
/// directory rather than a link, the destination absent or a plain file, and the temporary created
/// exclusively — so a copy cannot be redirected out of the build directory by anything that can predict
/// where it is about to be written.
fn write_bundle_bytes(
    context: &str,
    destination: &Path,
    name: &str,
    document: &str,
) -> Result<(), String> {
    let path = destination.join(name);
    let bytes = document.len() as u64;
    let charged = BUNDLE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    if charged > BUNDLE_RUN_BYTES_MAX {
        // Give the charge back, so one refused artifact does not close the sink for every later one.
        BUNDLE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        return Err(format!(
            "carrying it would have taken this run past the {BUNDLE_RUN_BYTES_MAX}-byte ceiling on \
             finding review copies ({charged} bytes charged)"
        ));
    }
    if let Err(error) = create_directory_chain_below(context, &report_root(), destination)
        .and_then(|()| require_replaceable(context, &path, Replaceable::RegularFile))
        .and_then(|()| super::publish_bytes_no_follow(context, &path, document.as_bytes()))
    {
        BUNDLE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
        return Err(String::from(error.cause()));
    }
    BUNDLE_FILES.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

/// Render the index that states what a review copy is, what it is not, and what reached it.
fn render_bundle_index(id: &FindingId, source: &Path, carried: &[(String, Carriage)]) -> String {
    let mut text = String::new();
    text.push_str("# Review copy of one finding — what this is, and what it is not\n\n");
    text.push_str(&format!("finding_id = {id}\n"));
    text.push_str(&format!("generated_from = {}\n", shown_path(source)));
    // Spelled against the elided root above rather than as an absolute path, because this document is a
    // report artifact and every path a report renders is elided. A reader who has the generated
    // directory has its absolute location already; one who does not would gain nothing from the build
    // path of a machine they are not on, and the report would have published it for no reader at all.
    text.push_str(&format!(
        "reproduce_with = sh <generated_from>/{COMMANDS_NAME}\n"
    ));
    text.push('\n');
    text.push_str(
        "This directory is a REPORT-GRADE copy of the finding named above, published inside the run\n\
         report so that a reader of the uploaded report has the finding itself and not only a row\n\
         naming a directory that went with the runner. Every artifact beside this file has been\n\
         redacted for credentials, sanitized so no byte in it can forge a line of a report, and\n\
         bounded; a capture that is not text is described rather than transcribed. So this copy is for\n\
         READING a finding — deciding whether it is real, what it is about, and whether to ask for the\n\
         rest. It is not the evidence itself.\n\n\
         The EXACT bytes and the runnable commands are in the generated directory named above, which\n\
         is git-ignored, is not redacted, and is uploaded only behind the workflow's explicit\n\
         raw-findings opt-in — because that directory names absolute tool paths and carries captured\n\
         diagnostics verbatim, and `tests/conformance/FINDINGS.md` §5.3 requires a person to read all of\n\
         it before any of it is published or committed. Nothing here performs that review or stands in\n\
         for it.\n\n\
         This copy inherits the report's own disclosure bound rather than improving on it:\n\
         `conformance_harness/mod.rs` documents, beside BARE_REDACTION_MIN_CHARS and is_path_value,\n\
         exactly what the redactor leaves in place — a credential-bearing value shorter than the floor\n\
         and appearing bare, and a value recognised as an absolute filesystem path. Treat this copy as\n\
         sanitized against the named forms with a narrow residual class, and read it before\n\
         republishing it outside this repository.\n",
    );
    text.push('\n');
    text.push_str("## The two curation judgements, as the finding itself records them\n\n");
    text.push_str(
        "Both are copied from the finding's own manifest, unchanged. A run writes `not-performed` for\n\
         each — truthfully, because nothing under the build directory is committed — and the curated\n\
         audit refuses that value, so a promotion into `tests/conformance/findings/` cannot happen\n\
         without a person replacing it with what they found. A review copy changes neither.\n\n",
    );
    let manifest_text = read_file_bounded(
        "reading the manifest of the finding this review copy was made from",
        &source.join(findings::MANIFEST_NAME),
        findings::FINDING_ARTIFACT_BYTES_MAX,
    )
    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    .unwrap_or_default();
    for prefix in [
        findings::MANIFEST_DISCLOSURE_PREFIX,
        findings::MANIFEST_MINIMIZATION_PREFIX,
    ] {
        let line = manifest_text
            .lines()
            .find(|line| line.starts_with(prefix))
            .map(|line| sanitize_text_for_report(line.trim()))
            .unwrap_or_else(|| {
                format!(
                    "{}<not stated in the manifest this copy was made from>",
                    prefix.trim_end()
                )
            });
        text.push_str(&format!("{line}\n"));
    }
    text.push('\n');
    text.push_str("## What reached this copy\n\n");
    let published = carried.iter().filter(|(_, state)| state.carried()).count();
    text.push_str(&format!(
        "{published} of {} entry(ies) were carried, and every one is listed below whether or not it\n\
         was — an absence is stated here rather than inferred from a missing file. The six single-file\n\
         artifact classes appear by name; the seventh, `{}/`, appears as its individual captures, or,\n\
         if the directory itself could not be read, as one row naming the class.\n\n",
        carried.len(),
        findings::OUTPUTS_DIR_NAME
    ));
    for (name, state) in carried {
        text.push_str(&format!(
            "[artifact] {} — {}\n",
            sanitize_text_for_report(name),
            state.describe()
        ));
    }
    sanitize_document_for_evidence(&text)
}

// ---------------------------------------------------------------------------------------------
// Text rendering
//
// Four kinds of text reach a report from outside this module: an outcome's detail, which the
// harness assembled from compiler diagnostics and program output; free text a maintainer wrote
// into an expectation record, which may legitimately span several lines; a tool's own
// identification banner; and a discovered filesystem path. None of them may be emitted verbatim
// into a line-oriented artifact, and the functions below are the only places that decide how each
// is made safe.
//
// Three distinct hazards are answered, and they need different treatments:
//
// - **A line-oriented artifact can be forged.** A tab forges a column, a line feed splits one
//   record into two, an escape introducer repaints a terminal, a directional override makes a line
//   render as its opposite. [`sanitize_text_for_report`] escapes exactly those characters and is
//   applied to *everything*, in both artifacts.
// - **A Markdown document can be restructured.** Sanitization deliberately leaves printable
//   characters alone, and in Markdown several printable characters are syntax: a permissive
//   renderer honours raw HTML, and a link, an emphasis run or a code-span introducer can
//   restructure the document around it. [`escape_markdown_inline`] neutralizes those, and is
//   applied **only** on the way into the Markdown half.
// - **A credential can be committed.** A report is an artifact a maintainer publishes, attaches to
//   an issue or uploads from continuous integration, and captured text is the one thing in it the
//   suite did not write: a compiler that echoes an environment variable back in a diagnostic, a tool
//   banner that prints a licence key, a path assembled from a variable that holds one.
//   [`redact_secrets`] is applied to *everything*, in both artifacts, for the same reason
//   sanitization is — a rule applied at some sinks is a rule a new sink will be written without.
//   What it recognises is bounded, and worth stating exactly: values this process can read in its own
//   environment under a credential-bearing name, in the `NAME=value` form at any length and bare only
//   from `BARE_REDACTION_MIN_CHARS` characters up. A secret the suite never saw, one embedded in a
//   tool's own configuration file, and a short bare value are each outside that. So this bounds an
//   ordinary hazard rather than proving a report carries no credential, and a maintainer publishing
//   one still reads it.
//
// All three are applied at the same three funnels, and the order between them is fixed and
// load-bearing: **redact, then sanitize, then escape for Markdown**. Redaction must come first
// because it recognises a credential by the characters the environment holds, and a value containing
// a tab, a newline or a byte sanitization spells out is no longer that value once it has been
// escaped — redacting afterwards would search for text that no longer exists. This is the same order
// `ubaudit.rs` and `flagprobe.rs` already apply to a command line, stated here once for every sink.
//
// Two things are deliberately **not** redacted, and both are outside this module. A finding's
// captured streams and the diff computed from them hold the exact bytes a program produced, because
// they are evidence rather than prose and a reproduction has to be able to compare them byte for
// byte. And a finding's `commands.sh` holds exact paths, because a redacted command is not a runnable
// command and the script's whole purpose is to be run. The trade is therefore bounded rather than
// absent: exact bytes and exact commands only inside a finding directory beneath the build directory,
// redacted text in everything the suite renders as a report. A *corpus program* is not a plausible
// source of a credential either — the authoring rules forbid it from reading the environment at all,
// and the child environment those programs receive is cleared before they run — so what remains
// exposed is what a *tool* chose to print, which is exactly what the bounded rule above covers, and
// where that rule stops is where a maintainer's own reading of the artifact takes over.
//
// The rule the whole module follows, stated once so it can be checked: *untrusted text is escaped
// for Markdown exactly once, at the moment it is placed into a Markdown-bearing string, and never
// again.* In practice that means every interpolation of untrusted text into a rendered line goes
// through [`md`], [`md_code`], [`md_path`], [`table_cell`], [`optional_cell`] or [`quoted_block`],
// and the functions that assemble whole lines out of already-escaped fragments do not escape a
// second time. Escaping twice would render `&amp;lt;` where a reader expects `<`, which is a
// different kind of dishonest report.
//
// The machine-readable sibling is not escaped for Markdown: it is data for an aggregator rather
// than a document, and escaping would corrupt the values an external tool reads. A diagnostic
// sentence is the one text that appears in both halves, and it follows the same rule: it is
// assembled unescaped — sanitized only, so it can never forge a column or a line — and escaped once
// by [`md`] at the sink that renders it into Markdown. Escaping the sentence whole rather than each
// fragment of it is what makes the rule checkable, and it has one visible consequence worth naming
// so it is not later mistaken for a defect: a backtick the harness itself wrote into the sentence is
// escaped along with everything else, so the raw Markdown reads `\`` where the rendered document
// reads a plain backtick. That is the correct trade — a report is a document to be rendered, and
// escaping only the fragments a reader guessed were untrusted is exactly the design that lets one
// through.
// ---------------------------------------------------------------------------------------------

/// One field of a machine-readable report.
///
/// [`redact_secrets`] first, for the reason the section comment above gives: this is one of the three
/// funnels, and redacting here is what makes the machine-readable half safe as a whole rather than
/// one sink at a time.
///
/// Then [`sanitize_text_for_report`], which escapes every control character — including the tab and
/// the line feed — and every formatting character that could make a line render as something other
/// than what it says. A field therefore cannot forge a column, cannot split one record into two and
/// cannot repaint a verdict, which is what lets a row be counted as one outcome.
fn tsv_field(raw: &str) -> String {
    sanitize_text_for_report(&redact_secrets(raw))
}

/// Assemble one machine-readable row from its fields, tab-separated.
fn tsv_row(fields: &[String]) -> String {
    fields
        .iter()
        .map(|field| tsv_field(field))
        .collect::<Vec<_>>()
        .join(&TSV_SEPARATOR.to_string())
}

/// The header row of a machine-readable report.
fn tsv_header(columns: &[&str]) -> String {
    columns.join(&TSV_SEPARATOR.to_string())
}

/// `raw` with every run of whitespace, including line breaks, replaced by a single space.
///
/// Applied before a multi-line value is placed in a table cell. Collapsing is preferred to
/// escaping there because a recorded reason is prose a human reads: `\x0a` between every sentence
/// would be faithful and unreadable, whereas a single space is readable and loses nothing a
/// reader of a table needs. The unabridged text is always available in the sections that quote it
/// line by line and in the machine-readable sibling.
fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One cell of a Markdown table, from free text of any length.
///
/// Collapsed to a single line, escaped by [`md`] so it can neither repaint a terminal nor act as
/// Markdown, and then bounded by [`TABLE_CELL_MAX_CHARS`]. The vertical bar is among the characters
/// [`md`] escapes, which matters here specifically: an unescaped one would end the cell and shift
/// every value after it into the wrong column — the table equivalent of forging a tab-separated
/// field. Truncation is marked with [`TRUNCATION_MARK`] and happens on a character boundary, never
/// inside a character; because escaping happens first, truncation can shorten an escape sequence but
/// can never leave a half-written character.
fn table_cell(raw: &str) -> String {
    let collapsed = md(&collapse_whitespace(raw));
    if collapsed.is_empty() {
        return String::from(ABSENT_CELL);
    }
    if collapsed.chars().count() <= TABLE_CELL_MAX_CHARS {
        return collapsed;
    }
    let kept: String = collapsed.chars().take(TABLE_CELL_MAX_CHARS).collect();
    format!("{kept}{TRUNCATION_MARK}")
}

/// One cell of a Markdown table from an optional value, rendering absence visibly.
fn optional_cell(raw: Option<&str>) -> String {
    match raw {
        Some(value) => table_cell(value),
        None => String::from(ABSENT_CELL),
    }
}

/// Multi-line free text as a Markdown block quote, one output line per input line.
///
/// Used where the whole of a recorded reason matters — the written basis of an expected
/// divergence, the reason a comparison was narrowed — so that its structure survives. Each line
/// is escaped independently by [`md`], and an empty input line becomes a bare quote marker so the
/// paragraph break is preserved. A quoted block is prose a maintainer wrote, so it is the sink most
/// likely to carry a stray angle bracket or underscore; escaping every line means the block renders
/// as the text that was recorded rather than as whatever that text happens to spell in Markdown.
fn quoted_block(raw: &str) -> Vec<String> {
    let trimmed = raw.trim_end();
    if trimmed.trim().is_empty() {
        return vec![String::from("> *(no text recorded)*")];
    }
    trimmed
        .lines()
        .map(|line| {
            let safe = md(line.trim_end());
            if safe.is_empty() {
                String::from(">")
            } else {
                format!("> {safe}")
            }
        })
        .collect()
}

/// One fragment of untrusted text, safe in Markdown inline position.
///
/// Three transformations in a fixed order, and every one of them depends on being where it is.
/// [`redact_secrets`] first, because it matches the characters the environment holds and neither of
/// the two escapings preserves them. Sanitization second, so no control character or directional
/// override survives. Markdown escaping last, so no character that is syntax there can act —
/// sanitization inserts backslash escapes of its own, and escaping for Markdown afterwards escapes
/// those backslashes too, which is why `\x1b` renders as the four characters a reader can search for
/// rather than as an escape Markdown then swallows.
///
/// This is the funnel the Markdown half rests on: [`table_cell`], [`optional_cell`], [`quoted_block`]
/// and every direct interpolation of untrusted text reach a document through here.
fn md(raw: &str) -> String {
    escape_markdown_inline(&sanitize_text_for_report(&redact_secrets(raw)))
}

/// One fragment of untrusted text inside a Markdown code span, with the delimiters included.
///
/// A code span is literal, so nothing inside it needs escaping for Markdown — with exactly one
/// exception, the backtick that would end the span early and hand the rest of the fragment to the
/// renderer as document syntax. There is no way to escape a backtick *inside* a span, so it is
/// replaced by the same visible escape spelling [`sanitize_text_for_report`] uses for a character it
/// refuses to emit. Everything else passes through as the text it is, which is the point: an
/// identifier, a path or a command reads correctly only if it is not littered with backslashes.
///
/// [`redact_secrets`] is applied first, as it is in the other two funnels. A code span is the sink
/// that renders a path and a command line, so it is the likeliest of the three to carry a value an
/// environment variable supplied.
fn md_code(raw: &str) -> String {
    format!(
        "`{}`",
        sanitize_text_for_report(&redact_secrets(raw)).replace('`', "\\x60")
    )
}

/// A filesystem path inside a Markdown code span.
///
/// Paths are the sink this module renders most often and trusts least: a path can carry a byte from
/// an environment variable, from a discovered tool location or from a corpus entry.
///
/// Rendered through [`shown_path`], so the package and build roots appear as their symbolic tokens
/// rather than as this machine's absolute locations. A report is read by whoever receives it, which is
/// routinely not whoever produced it, and a table of finding directories that spelled out a
/// continuous-integration workspace or an agent clone in every row disclosed the run's own location
/// without telling a reader anything they could act on.
///
/// There is no exception left. [`reproduce_command`] was one, on the reasoning that a line a reader is
/// told to paste has to name the directory exactly; it renders through this same elision now, because
/// a report is uploaded and one substitution by its reader is a smaller cost than disclosing where the
/// run happened. Its own documentation carries the argument.
fn md_path(path: &Path) -> String {
    md_code(&shown_path(path))
}

// ---------------------------------------------------------------------------------------------
// Ordering
//
// Reports are diffed between runs, so their order has to be a property of the data rather than of
// how it happened to be collected. Position in the canonical table is used rather than the derived
// ordering of the enumerations, so that reordering a variant one day could not silently reorder
// every report ever written against the tables the rest of the suite documents.
// ---------------------------------------------------------------------------------------------

/// Position of `value` in its canonical table, or one past the end for a value not in it.
///
/// A value outside the table cannot arise from these closed enumerations; ranking it last rather
/// than panicking keeps this module free of any path that could replace a diagnosable divergence
/// with a harness crash.
fn rank<T: Copy + PartialEq>(table: &[T], value: T) -> usize {
    table
        .iter()
        .position(|candidate| *candidate == value)
        .unwrap_or(table.len())
}

fn target_rank(target: Target) -> usize {
    rank(&Target::ALL, target)
}

fn opt_rank(opt: OptLevel) -> usize {
    rank(&OptLevel::ALL, opt)
}

fn oracle_rank(oracle: Oracle) -> usize {
    rank(&Oracle::ALL, oracle)
}

fn verdict_rank(verdict: Verdict) -> usize {
    rank(&Verdict::ALL, verdict)
}

// ---------------------------------------------------------------------------------------------
// Aggregation
//
// Everything from here to the rendering section turns outcomes into counted, ordered facts. The
// two halves are kept apart deliberately: aggregation decides what is true, rendering decides how
// it reads, and a change to one should not require reasoning about the other.
// ---------------------------------------------------------------------------------------------

/// One recorded comparison, in the form both report halves and the summary aggregate share.
///
/// This is the row of a per-area machine-readable report and the unit [`try_finalize`] reads back.
/// It is built from an [`Outcome`] by [`Row::from_outcome`] and can be reconstructed from its own
/// rendered form by [`Row::parse`], which is what lets the summary be an aggregate of the files the
/// area tests wrote rather than a second traversal of the matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    area: String,
    program: String,
    target: Target,
    opt: OptLevel,
    oracle: Oracle,
    verdict: Verdict,
    class: Option<DivergenceClass>,
    marker_id: Option<String>,
    finding_id: Option<String>,
    finding_dir: Option<PathBuf>,
    /// The outcome's structured provenance, rendered into the thirteen columns documented on
    /// [`AREA_TSV_COLUMNS`]. Every field is already report-safe.
    provenance: RowProvenance,
    detail: String,
}

/// The provenance half of a row, in the fields the machine-readable report publishes.
///
/// A flat record of already-sanitized strings rather than a reference to the [`Provenance`] it was
/// built from, for two reasons that both matter here. A row must survive being written to a file and
/// read back — [`try_finalize`] aggregates the summary by parsing the area files rather than by
/// walking the matrix a second time — so every field has to have exactly one textual form, and that
/// form has to be the one the parser reconstructs. And the two derived fields, `severity` and
/// `cell_fingerprint`, are not properties of the observation at all: severity depends on the run's
/// configured policy and the fingerprint on the cell's identity, so they belong to the row rather
/// than to the provenance.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RowProvenance {
    source: String,
    record_path: String,
    subject: String,
    subject_command: String,
    subject_termination: String,
    subject_captures: String,
    authority: String,
    authority_command: String,
    authority_termination: String,
    authority_captures: String,
    first_difference: String,
    severity: String,
    cell_fingerprint: String,
}

impl RowProvenance {
    /// Render one outcome's provenance into the row's fields.
    ///
    /// The two path fields are DERIVED from the cell identity rather than plumbed through from the
    /// loader, and that is sound rather than convenient: `Cell::require_identity` refuses to build a
    /// cell whose source file does not live in the area directory the key names and does not carry the
    /// program's own stem, so the identity and the paths cannot disagree by the time an outcome exists.
    /// Deriving them here means a row identifies its own inputs even for an outcome reached without a
    /// record — an absent tool, an unreadable corpus entry — where nothing was loaded to plumb.
    ///
    /// `severity` is answered with this run's own policy, through the same
    /// [`verdict_fails_run`] the driver asserts on, so a published row cannot disagree with the run
    /// that produced it about whether it was a failure.
    fn of(outcome: &Outcome, config: &RunConfig) -> RowProvenance {
        let key = outcome.key();
        let stem = format!("{}/{}", key.area(), key.program());
        let source = format!("{CORPUS_RELATIVE_ROOT}/{stem}.{SOURCE_EXTENSION}");
        let record_path = format!("{CORPUS_RELATIVE_ROOT}/{stem}.{RECORD_EXTENSION}");
        let provenance = outcome.provenance();
        let subject = provenance.and_then(super::Provenance::subject);
        let authority = provenance.and_then(super::Provenance::authority);
        let field = |side: Option<&super::SideRecord>, read: fn(&super::SideRecord) -> &str| {
            side.map(read).unwrap_or_default().to_string()
        };
        let subject_command = field(subject, super::SideRecord::command);
        let authority_command = field(authority, super::SideRecord::command);
        let cell_fingerprint = stable_digest(&[
            key.area(),
            key.program(),
            key.target().triple(),
            key.opt().flag(),
            &format!("oracle_{}", outcome.oracle().letter()),
            &subject_command,
            &authority_command,
        ]);
        RowProvenance {
            source,
            record_path,
            subject: field(subject, super::SideRecord::role),
            subject_command,
            subject_termination: field(subject, super::SideRecord::termination),
            subject_captures: field(subject, super::SideRecord::captures),
            authority: field(authority, super::SideRecord::role),
            authority_command,
            authority_termination: field(authority, super::SideRecord::termination),
            authority_captures: field(authority, super::SideRecord::captures),
            first_difference: provenance
                .map(super::Provenance::first_difference)
                .unwrap_or_default()
                .to_string(),
            severity: match verdict_fails_run(outcome.verdict(), config) {
                true => String::from(SEVERITY_FAILS_RUN),
                false => String::from(SEVERITY_REPORTED),
            },
            cell_fingerprint,
        }
    }

    /// Copy this block onto a row of the machine-readable summary.
    ///
    /// A copy rather than a second derivation. The summary's outcome rows *are* the area rows, read
    /// back from the files the areas published, so recomputing the evidence here would introduce a
    /// second answer to a question that already has one — and the disagreement would appear between
    /// two artifacts of a single run, which is the one place a reader has no way to adjudicate.
    ///
    /// Empty fields are dropped by [`SummaryRow::set`], so an outcome that carries no provenance —
    /// one reached from no execution, such as the register audit's synthetic divergence — renders
    /// the same empty columns it does in its area report rather than an invented value.
    fn apply_to(&self, row: SummaryRow) -> SummaryRow {
        row.set(COL_SOURCE, self.source.clone())
            .set(COL_RECORD_PATH, self.record_path.clone())
            .set(COL_SUBJECT, self.subject.clone())
            .set(COL_SUBJECT_COMMAND, self.subject_command.clone())
            .set(COL_SUBJECT_TERMINATION, self.subject_termination.clone())
            .set(COL_SUBJECT_CAPTURES, self.subject_captures.clone())
            .set(COL_AUTHORITY, self.authority.clone())
            .set(COL_AUTHORITY_COMMAND, self.authority_command.clone())
            .set(
                COL_AUTHORITY_TERMINATION,
                self.authority_termination.clone(),
            )
            .set(COL_AUTHORITY_CAPTURES, self.authority_captures.clone())
            .set(COL_FIRST_DIFFERENCE, self.first_difference.clone())
            .set(COL_SEVERITY, self.severity.clone())
            .set(COL_CELL_FINGERPRINT, self.cell_fingerprint.clone())
    }
}

impl Row {
    /// Build a row from an accumulated outcome.
    ///
    /// A finding's artifact directory is *derived* rather than looked up: [`FindingId::derive`] is
    /// a pure function of the cell and the divergence class, and the directory is that identifier
    /// beneath the findings root. The report can therefore name the artifacts of a finding without
    /// being handed anything by the module that wrote them, and the name it prints is the same one
    /// that module chose. A finding carrying no divergence class is the one case where no directory
    /// can be derived; it is left empty here and reported as a diagnostic by
    /// [`AreaReport::from_outcomes`], because a finding without a class is an internal
    /// inconsistency rather than a fact about the compiler.
    ///
    /// The oracle is deliberately not part of that derivation, so two rows for the same cell and
    /// class observed through different oracles name the **same** directory. That is not a
    /// duplicate: one root cause is filed once, its manifest lists every oracle that observed it,
    /// and each row points at the whole of the evidence rather than at one oracle's slice of it.
    fn from_outcome(outcome: &Outcome, config: &RunConfig) -> Row {
        let key = outcome.key();
        let identifier = match (outcome.verdict(), outcome.class()) {
            (Verdict::Finding, Some(class)) => Some(FindingId::derive(key, class)),
            _ => None,
        };
        let provenance = RowProvenance::of(outcome, config);
        Row {
            area: String::from(key.area()),
            program: String::from(key.program()),
            target: key.target(),
            opt: key.opt(),
            oracle: outcome.oracle(),
            verdict: outcome.verdict(),
            class: outcome.class(),
            marker_id: outcome.marker_id().map(String::from),
            finding_id: identifier
                .as_ref()
                .map(|value| String::from(value.as_str())),
            finding_dir: identifier.as_ref().map(FindingId::directory),
            provenance,
            detail: String::from(outcome.detail()),
        }
    }

    /// The total order every report imposes: program, then target, then optimization level, then
    /// oracle, all in canonical table order. The area leads it so that a row that arrived under
    /// the wrong area still sorts predictably instead of interleaving.
    fn order_key(&self) -> (&str, &str, usize, usize, usize) {
        (
            self.area.as_str(),
            self.program.as_str(),
            target_rank(self.target),
            opt_rank(self.opt),
            oracle_rank(self.oracle),
        )
    }

    /// The cell this row belongs to, for counting distinct cells.
    fn cell(&self) -> (String, usize, usize) {
        (
            self.program.clone(),
            target_rank(self.target),
            opt_rank(self.opt),
        )
    }

    /// The comparison this row is, for detecting a comparison recorded twice.
    fn comparison(&self) -> (String, usize, usize, usize) {
        (
            self.program.clone(),
            target_rank(self.target),
            opt_rank(self.opt),
            oracle_rank(self.oracle),
        )
    }

    /// `area/program @ triple -Olevel` — the same identity spelling the rest of the harness uses.
    fn cell_label(&self) -> String {
        format!(
            "{}/{} @ {} {}",
            sanitize_text_for_report(&self.area),
            sanitize_text_for_report(&self.program),
            self.target.triple(),
            self.opt.flag()
        )
    }

    /// This row as one line of a per-area machine-readable report.
    ///
    /// Every enumeration is written in a spelling its own `parse` accepts — the target triple, the
    /// optimization flag, the oracle's record key, the verdict token and the class token — so the
    /// file the summary reads back is the file the area wrote, with no separate encoding to keep
    /// in step.
    ///
    /// `identity` is stamped in rather than derived here, so that every row of one file carries the
    /// same provenance by construction and the aggregating half has exactly one thing to compare
    /// against.
    fn to_tsv(&self, identity: &RunIdentity) -> String {
        tsv_row(&[
            self.area.clone(),
            self.program.clone(),
            String::from(self.target.triple()),
            String::from(self.opt.flag()),
            format!("oracle_{}", self.oracle.letter()),
            String::from(self.verdict.label()),
            self.class
                .map(|class| String::from(class.label()))
                .unwrap_or_default(),
            self.marker_id.clone().unwrap_or_default(),
            self.finding_id.clone().unwrap_or_default(),
            self.finding_dir
                .as_ref()
                .map(|path| shown_path(path))
                .unwrap_or_default(),
            identity.run.clone(),
            identity.identity.clone(),
            self.provenance.source.clone(),
            self.provenance.record_path.clone(),
            self.provenance.subject.clone(),
            self.provenance.subject_command.clone(),
            self.provenance.subject_termination.clone(),
            self.provenance.subject_captures.clone(),
            self.provenance.authority.clone(),
            self.provenance.authority_command.clone(),
            self.provenance.authority_termination.clone(),
            self.provenance.authority_captures.clone(),
            self.provenance.first_difference.clone(),
            self.provenance.severity.clone(),
            self.provenance.cell_fingerprint.clone(),
            self.detail.clone(),
        ])
    }

    /// Rebuild a row from one line of a per-area machine-readable report, with its provenance.
    ///
    /// The provenance is returned rather than checked here: this function's job is to say what the
    /// line *claims*, and the caller decides whether a row claiming a different run may contribute
    /// to a total. Separating the two keeps a refused row diagnosable — the caller can report what
    /// the row claimed and what this run expected.
    ///
    /// # Errors
    ///
    /// Returns a sentence describing the defect — the wrong number of fields, or a value no
    /// canonical spelling matches — rather than a [`HarnessError`], because the caller turns it
    /// into a loud diagnostic in the summary instead of failing the run. An area file that cannot
    /// be read is a defect in the artifact, and a summary that says so is more useful than a run
    /// that aborts before writing one.
    fn parse(line: &str, number: usize) -> Result<(Row, RunIdentity, Option<String>), String> {
        let fields: Vec<&str> = line.split(TSV_SEPARATOR).collect();
        if fields.len() != AREA_TSV_COLUMNS.len() {
            return Err(format!(
                "line {number} has {} tab-separated fields, expected {}: {}",
                fields.len(),
                AREA_TSV_COLUMNS.len(),
                tsv_header(AREA_TSV_COLUMNS)
            ));
        }
        let value = |index: usize| fields[index].trim();

        let target = required_value(number, "target", value(2), Target::parse(value(2)))?;
        let opt = required_value(
            number,
            "optimization level",
            value(3),
            OptLevel::parse(value(3)),
        )?;
        let oracle = required_value(number, "oracle", value(4), Oracle::parse(value(4)))?;
        let verdict = required_value(number, "verdict", value(5), Verdict::parse(value(5)))?;
        let class = if value(6).is_empty() {
            None
        } else {
            Some(required_value(
                number,
                "divergence class",
                value(6),
                DivergenceClass::parse(value(6)),
            )?)
        };
        let area = String::from(value(0));
        let program = String::from(value(1));
        let (finding_dir, defect) = parse_finding_dir(
            number,
            optional_field(fields[9]).as_deref(),
            &area,
            &program,
            target,
            opt,
            class,
        );
        Ok((
            Row {
                area,
                program,
                target,
                opt,
                oracle,
                verdict,
                class,
                marker_id: optional_field(fields[7]),
                finding_id: optional_field(fields[8]),
                finding_dir,
                provenance: RowProvenance {
                    source: String::from(fields[12]),
                    record_path: String::from(fields[13]),
                    subject: String::from(fields[14]),
                    subject_command: String::from(fields[15]),
                    subject_termination: String::from(fields[16]),
                    subject_captures: String::from(fields[17]),
                    authority: String::from(fields[18]),
                    authority_command: String::from(fields[19]),
                    authority_termination: String::from(fields[20]),
                    authority_captures: String::from(fields[21]),
                    first_difference: String::from(fields[22]),
                    severity: String::from(fields[23]),
                    cell_fingerprint: String::from(fields[24]),
                },
                detail: String::from(fields[25]),
            },
            RunIdentity {
                run: String::from(value(10)),
                identity: String::from(value(11)),
            },
            defect,
        ))
    }
}

/// The artifact directory a parsed row may be asked about, **re-derived** rather than trusted.
///
/// # Why the recorded text cannot simply be used
///
/// The field was written through [`shown_path`], which elides the build root so a published report
/// discloses nothing about the machine that produced it — the recorded text is
/// `<build>/conformance-findings/…` rather than a path. The summary is aggregated by parsing these
/// files back and then asking the file system whether each finding's artifacts are really there, so
/// something has to turn that text into a path again, and [`resolve_shown_path`] is that something.
///
/// What it cannot do is *validate*. It expands a leading token and returns whatever follows, so a
/// report file whose tenth field had been altered — by a corrupted write, a truncated concurrent
/// publish, or a hand edit — would send the summary's completeness check reading directories chosen by
/// that field: `<build>/../../etc`, or a bare relative path resolved against the process's working
/// directory. Nothing was written through it, so this is a read rather than a write, and the cost is
/// still real: the summary would report the artifacts of whatever it was pointed at as the artifacts
/// of this finding, and a directory that happened to hold the right file names would be counted as a
/// complete deliverable.
///
/// # What is done instead
///
/// The directory is a **pure function of the row's own identity**: [`FindingId::derive`] over the cell
/// and the divergence class, beneath [`findings_root`]. Both are already parsed and validated — the
/// target, the level and the class each had to match a canonical spelling for this row to exist at
/// all, and [`CellKey::new`] re-validates the two names as corpus stems. So the expected directory is
/// recomputed here from those values, and the recorded field is required to agree with it.
///
/// Three outcomes, and only the first yields a path:
///
/// - **Agreement** — the recomputed directory is returned. It is derived rather than parsed, so it is
///   inside the findings root by construction and no separate containment check is needed.
/// - **Disagreement** — the path is dropped and a diagnostic is returned. The row survives with its
///   verdict intact, because the comparison it records is still a fact; what is refused is reading a
///   directory this row's identity does not name.
/// - **A directory named with no divergence class** — also dropped and diagnosed, because no
///   identifier can be derived without one, so nothing could be compared against it.
///
/// A row with an empty field is the ordinary case for every verdict but `FINDING`, and yields no path
/// and no diagnostic.
fn parse_finding_dir(
    number: usize,
    recorded: Option<&str>,
    area: &str,
    program: &str,
    target: Target,
    opt: OptLevel,
    class: Option<DivergenceClass>,
) -> (Option<PathBuf>, Option<String>) {
    let Some(recorded) = recorded else {
        return (None, None);
    };
    let Some(class) = class else {
        return (
            None,
            Some(format!(
                "line {number} names a finding artifact directory but records no divergence class, \
                 so the identifier that directory is named from cannot be derived and the recorded \
                 path cannot be confirmed; the row is kept and the path is dropped"
            )),
        );
    };
    let key = match CellKey::new(area, program, target, opt) {
        Ok(key) => key,
        Err(error) => {
            return (
                None,
                Some(format!(
                    "line {number} names a finding artifact directory, but its own area and program \
                     are not a valid cell identity, so the directory that identity would name cannot \
                     be derived: {error}"
                )),
            )
        }
    };
    let expected = FindingId::derive(&key, class).directory();
    let resolved = resolve_shown_path(recorded);
    if resolved == expected {
        return (Some(expected), None);
    }
    (
        None,
        Some(format!(
            "line {number} names the finding artifact directory {}, but the identity that row \
             records — {}/{} at {} {} with class {} — names {}. A directory is derived from the \
             identity and nothing else, so the two can only differ if the field was altered after it \
             was written; the row is kept and the path is dropped rather than read, because reading \
             it would report whatever that directory holds as this finding's evidence",
            shown_path(&resolved),
            sanitize_text_for_report(area),
            sanitize_text_for_report(program),
            target.triple(),
            opt.flag(),
            class.label(),
            shown_path(&expected)
        )),
    )
}

/// A parsed field, or a sentence naming the line, the role and the value that matched nothing.
///
/// A free function rather than a closure inside [`Row::parse`], because each call parses a
/// different type and a closure cannot be generic.
fn required_value<T>(number: usize, role: &str, raw: &str, parsed: Option<T>) -> Result<T, String> {
    parsed.ok_or_else(|| {
        format!(
            "line {number} carries the {role} {:?}, which matches no canonical spelling",
            sanitize_text_for_report(raw)
        )
    })
}

/// `None` for an empty field, `Some` for anything else, with surrounding whitespace trimmed.
fn optional_field(field: &str) -> Option<String> {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(String::from(trimmed))
    }
}

/// The count of each verdict, in [`Verdict::ALL`] order.
///
/// Indexed by canonical position rather than held in a map, so the tally is complete by
/// construction: every verdict has a row in every report, **including the ones that scored zero**.
/// A tally that omitted its zeros would let a reader mistake an absent verdict for an unasked
/// question.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tally {
    counts: [usize; Verdict::ALL.len()],
}

impl Tally {
    /// Count one outcome.
    fn record(&mut self, verdict: Verdict) {
        let index = verdict_rank(verdict);
        if let Some(slot) = self.counts.get_mut(index) {
            *slot += 1;
        }
    }

    /// How many outcomes carried `verdict`.
    fn get(&self, verdict: Verdict) -> usize {
        self.counts.get(verdict_rank(verdict)).copied().unwrap_or(0)
    }

    /// How many outcomes were counted in total.
    fn total(&self) -> usize {
        self.counts.iter().sum()
    }

    /// Add another tally into this one.
    fn merge(&mut self, other: &Tally) {
        for (slot, addend) in self.counts.iter_mut().zip(other.counts.iter()) {
            *slot += *addend;
        }
    }

    /// The non-zero verdicts as one compact line, for a table column that has room for one.
    ///
    /// Zeros are omitted **here only**, because this rendering exists to fit a summary row; the
    /// full tally with its zero rows is always present as its own table.
    fn compact(&self) -> String {
        let parts: Vec<String> = Verdict::ALL
            .iter()
            .copied()
            .filter(|verdict| self.get(*verdict) > 0)
            .map(|verdict| format!("{} {}", verdict.label(), self.get(verdict)))
            .collect();
        if parts.is_empty() {
            String::from("no outcomes")
        } else {
            parts.join(", ")
        }
    }
}

/// One feature area's accumulated result: its rows, its counts and anything wrong with it.
#[derive(Debug, Clone)]
struct AreaReport {
    spec: &'static AreaSpec,
    rows: Vec<Row>,
    tally: Tally,
    programs: BTreeSet<String>,
    cells: BTreeSet<(String, usize, usize)>,
    targets: BTreeSet<Target>,
    opt_levels: BTreeSet<OptLevel>,
    /// Defects in the material this report was built from, rendered loudly rather than dropped.
    diagnostics: Vec<String>,
}

impl AreaReport {
    /// Accumulate one area from the outcomes its test recorded.
    ///
    /// Three inconsistencies are detected here and reported rather than corrected, because each
    /// one means a verdict cannot be trusted at face value and silently repairing it would hide
    /// that: an outcome filed under a different area than the one being written, the same cell and
    /// oracle recorded twice — which would double-count the tally — and a finding carrying no
    /// divergence class, which leaves its artifacts unnameable.
    fn from_outcomes(
        spec: &'static AreaSpec,
        outcomes: &[Outcome],
        config: &RunConfig,
    ) -> AreaReport {
        let mut report = AreaReport {
            spec,
            rows: Vec::with_capacity(outcomes.len()),
            tally: Tally::default(),
            programs: BTreeSet::new(),
            cells: BTreeSet::new(),
            targets: BTreeSet::new(),
            opt_levels: BTreeSet::new(),
            diagnostics: Vec::new(),
        };
        let mut seen: BTreeSet<(String, usize, usize, usize)> = BTreeSet::new();
        for outcome in outcomes {
            if outcome.key().area() != spec.directory() {
                report.diagnostics.push(format!(
                    "⚠️ an outcome for {} was recorded into the report of feature area `{}`; the \
                     row is kept so the verdict is not lost, but the caller passed an outcome that \
                     belongs to another area",
                    outcome.key(),
                    spec.directory()
                ));
            }
            if outcome.verdict() == Verdict::Finding && outcome.class().is_none() {
                report.diagnostics.push(format!(
                    "⚠️ the finding for {} under {} carries no divergence class, so its artifact \
                     directory cannot be derived and this report cannot name it; a finding without \
                     a class is an internal inconsistency in the suite rather than a fact about \
                     the compiler",
                    outcome.key(),
                    outcome.oracle()
                ));
            }
            let row = Row::from_outcome(outcome, config);
            // A finding is a deliverable, and the deliverable is the artifact directory. A row that
            // says `FINDING` while that directory holds nothing is the one shape of report that is
            // actively misleading: it reads as a recorded observation and is an empty promise. So the
            // directory is checked here, against disk, while the run that produced the row is still
            // able to say so — rather than left for whoever later opens the register and finds
            // nothing there.
            if let Some(shortfall) = finding_shortfall(&row) {
                report.diagnostics.push(format!("⚠️ {shortfall}"));
            }
            report.absorb(row, &mut seen);
        }
        report.rows.sort_by(|left, right| {
            let (left_key, right_key) = (left.order_key(), right.order_key());
            left_key.cmp(&right_key)
        });
        report
    }

    /// Rebuild an area from the machine-readable file its test wrote.
    ///
    /// The rows are re-sorted rather than trusted to be in order, so that a file edited by hand or
    /// written by an older revision still aggregates into a deterministic summary.
    fn from_rows(spec: &'static AreaSpec, rows: Vec<Row>, diagnostics: Vec<String>) -> AreaReport {
        let mut report = AreaReport {
            spec,
            rows: Vec::with_capacity(rows.len()),
            tally: Tally::default(),
            programs: BTreeSet::new(),
            cells: BTreeSet::new(),
            targets: BTreeSet::new(),
            opt_levels: BTreeSet::new(),
            diagnostics,
        };
        let mut seen: BTreeSet<(String, usize, usize, usize)> = BTreeSet::new();
        for row in rows {
            report.absorb(row, &mut seen);
        }
        report.rows.sort_by(|left, right| {
            let (left_key, right_key) = (left.order_key(), right.order_key());
            left_key.cmp(&right_key)
        });
        report
    }

    /// Count one row into every index this report keeps, recording a duplicate comparison.
    fn absorb(&mut self, row: Row, seen: &mut BTreeSet<(String, usize, usize, usize)>) {
        let comparison = row.comparison();
        if !seen.insert(comparison) {
            self.diagnostics.push(format!(
                "⚠️ {} under {} was recorded more than once; every duplicate is counted in the \
                 tally, so the totals of this area are inflated until the duplicate is removed",
                row.cell_label(),
                row.oracle
            ));
        }
        self.tally.record(row.verdict);
        self.programs.insert(row.program.clone());
        self.cells.insert(row.cell());
        self.targets.insert(row.target);
        self.opt_levels.insert(row.opt);
        self.rows.push(row);
    }

    /// Every row carrying one verdict, in report order.
    fn rows_with(&self, verdict: Verdict) -> Vec<&Row> {
        self.rows
            .iter()
            .filter(|row| row.verdict == verdict)
            .collect()
    }

    /// How many comparisons one oracle contributed.
    fn oracle_count(&self, oracle: Oracle) -> usize {
        self.rows.iter().filter(|row| row.oracle == oracle).count()
    }

    /// The observed targets, in canonical table order rather than set order.
    fn observed_targets(&self) -> Vec<Target> {
        Target::ALL
            .iter()
            .copied()
            .filter(|target| self.targets.contains(target))
            .collect()
    }

    /// The observed optimization levels, in canonical table order.
    fn observed_opt_levels(&self) -> Vec<OptLevel> {
        OptLevel::ALL
            .iter()
            .copied()
            .filter(|level| self.opt_levels.contains(level))
            .collect()
    }

    /// Programs that produced a comparison and were **runnable**, so the count describes the sweep.
    ///
    /// A program whose expectation record could not be read still produces one row — a corpus defect
    /// is reported, never skipped — and that row lands in `programs` and in `cells` like any other.
    /// Counting it as a swept program is what let a run report the full corpus while part of it had no
    /// record at all. The row is still in the table, still fails the run and is still tallied; it is
    /// only excluded from the counts that claim something was *swept*.
    fn swept_programs(&self, facts: &CorpusFacts) -> usize {
        self.programs
            .iter()
            .filter(|program| facts.is_runnable(self.spec.directory(), program))
            .count()
    }

    /// Cells that were actually executed, excluding the synthetic cell a corpus defect is filed under.
    fn executed_cells(&self, facts: &CorpusFacts) -> usize {
        self.cells
            .iter()
            .filter(|(program, _, _)| facts.is_runnable(self.spec.directory(), program))
            .count()
    }

    /// Rows that exist to report a corpus defect rather than to record a comparison.
    fn corpus_defect_rows(&self, facts: &CorpusFacts) -> usize {
        self.rows
            .iter()
            .filter(|row| !facts.is_runnable(self.spec.directory(), &row.program))
            .count()
    }
}

// ---------------------------------------------------------------------------------------------
// Corpus facts
//
// Two of the report's obligations cannot be met from outcomes alone. An expected divergence has to
// be listed with its documented basis, and the basis lives in the program's own expectation
// record; and a comparison a program deliberately excluded produced no outcome at all, so the only
// evidence it was excluded — and the only statement of why — is again in that record. This section
// reads those records.
//
// Every failure here is collected as a diagnostic instead of aborting. The report has to be written
// even when the run is about to fail, and a corpus defect is exactly the kind of thing a reader
// needs the report to tell them about.
// ---------------------------------------------------------------------------------------------

/// What one expectation record contributes to a report.
#[derive(Debug, Clone)]
struct ProgramFacts {
    area: &'static str,
    program: String,
    description: String,
    targets: Vec<Target>,
    opt_levels: Vec<OptLevel>,
    disabled_oracles: Vec<Oracle>,
    /// The record's own statement of why it narrowed something, when it narrowed anything.
    reason: Option<String>,
    /// The per-program warning gate, present only when it actually differs from the default.
    ub_gate: Option<Vec<String>>,
    marker: Option<ExpectedDivergence>,
}

impl ProgramFacts {
    /// `area/program`, the spelling every register and report uses for a program.
    fn label(&self) -> String {
        format!("{}/{}", self.area, sanitize_text_for_report(&self.program))
    }

    /// Every narrowing this program applies, each with the reason its record records.
    fn narrowings(&self) -> Vec<Narrowing> {
        let mut narrowed = Vec::new();
        if self.targets.len() < Target::ALL.len() {
            narrowed.push(Narrowing {
                kind: "target restriction",
                scope: join_targets(&self.targets),
                reason: self.reason.clone(),
            });
        }
        if self.opt_levels.len() < OptLevel::ALL.len() {
            narrowed.push(Narrowing {
                kind: "optimization-level restriction",
                scope: join_opt_levels(&self.opt_levels),
                reason: self.reason.clone(),
            });
        }
        for oracle in &self.disabled_oracles {
            narrowed.push(Narrowing {
                kind: "oracle exclusion",
                scope: String::from(oracle.label()),
                reason: self.reason.clone(),
            });
        }
        if let Some(gate) = &self.ub_gate {
            narrowed.push(Narrowing {
                kind: "warning-gate deviation",
                scope: gate.join(" "),
                reason: self.reason.clone(),
            });
        }
        narrowed
    }

    /// How many cells this program contributes to a run sweeping `targets` at `levels`.
    ///
    /// The intersection, not the product: a program that restricts itself to two targets
    /// contributes two, and a run that reduced its matrix to one target contributes at most one.
    fn planned_cells(&self, targets: &[Target], levels: &[OptLevel]) -> usize {
        let selected_targets = self
            .targets
            .iter()
            .filter(|target| targets.contains(target))
            .count();
        let selected_levels = self
            .opt_levels
            .iter()
            .filter(|level| levels.contains(level))
            .count();
        selected_targets * selected_levels
    }
}

/// One deliberate reduction in what a program is compared on, and the reason recorded for it.
#[derive(Debug, Clone)]
struct Narrowing {
    kind: &'static str,
    scope: String,
    reason: Option<String>,
}

/// Everything the corpus contributes to a report, plus anything wrong with the corpus.
#[derive(Debug, Clone, Default)]
struct CorpusFacts {
    programs: Vec<ProgramFacts>,
    diagnostics: Vec<String>,
    /// How many `.c` sources were **discovered**, whether or not their records could be read.
    ///
    /// Held separately from `programs` because the two answer different questions and conflating them
    /// is what let the summary claim a full corpus while a sixth of it had no readable record. A
    /// source is discovered by existing; a program becomes *runnable* only when its own expectation
    /// record parses, because the record is what supplies the matrix, the golden stdout and the
    /// reproduction commands.
    discovered: usize,
    /// `area/program` for every discovered source whose record could not be read.
    ///
    /// The complement of `programs` within `discovered`, kept as labels so the summary can name them
    /// rather than only count them.
    unreadable: Vec<String>,
}

impl CorpusFacts {
    /// Read every expectation record of one feature area.
    fn for_area(spec: &'static AreaSpec) -> CorpusFacts {
        let mut facts = CorpusFacts::default();
        let sources = match manifest::discover_area(spec.directory()) {
            Ok(sources) => sources,
            Err(error) => {
                facts.diagnostics.push(format!(
                    "⚠️ the expectation records of feature area `{}` could not be read, so this \
                     report cannot state its documented bases or its recorded exclusions: {error}",
                    spec.directory()
                ));
                return facts;
            }
        };
        facts.discovered = sources.len();
        for source in sources {
            match manifest::load_for_source(&source) {
                Ok(record) => facts.programs.push(ProgramFacts {
                    area: spec.directory(),
                    program: String::from(record.program()),
                    description: String::from(record.description()),
                    targets: record.targets().to_vec(),
                    opt_levels: record.opt_levels().to_vec(),
                    disabled_oracles: record.disabled_oracles(),
                    reason: record.impl_defined_notes().map(String::from),
                    ub_gate: if record.has_ub_audit_deviation() {
                        record.ub_audit_flags().map(<[String]>::to_vec)
                    } else {
                        None
                    },
                    marker: record.marker().cloned(),
                }),
                Err(error) => {
                    facts.unreadable.push(format!(
                        "{}/{}",
                        spec.directory(),
                        sanitize_text_for_report(
                            source
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .unwrap_or_default()
                        )
                    ));
                    facts.diagnostics.push(format!(
                        "⚠️ the expectation record beside {} could not be read: {error}",
                        shown_path(&source)
                    ));
                }
            }
        }
        facts
    }

    /// Read the expectation records of several feature areas, in the order given.
    fn for_areas(specs: &[&'static AreaSpec]) -> CorpusFacts {
        let mut facts = CorpusFacts::default();
        for spec in specs {
            let mut area = CorpusFacts::for_area(spec);
            facts.programs.append(&mut area.programs);
            facts.diagnostics.append(&mut area.diagnostics);
            facts.discovered += area.discovered;
            facts.unreadable.append(&mut area.unreadable);
        }
        facts
    }

    /// Whether this program could take part in the sweep at all.
    ///
    /// True exactly when its expectation record parsed. A program without one produces no cell: there
    /// is no declared matrix to sweep, no golden stdout to compare against and no command template to
    /// reproduce, so the single row it does produce is a *statement about the corpus* rather than an
    /// observation of the compiler — and counting that row as a swept program or an executed cell is
    /// what made the summary overstate a run.
    fn is_runnable(&self, area: &str, program: &str) -> bool {
        self.programs
            .iter()
            .any(|facts| facts.area == area && facts.program == program)
    }

    /// The record of one program, when its area's records could be read.
    fn program(&self, area: &str, program: &str) -> Option<&ProgramFacts> {
        self.programs
            .iter()
            .find(|facts| facts.area == area && facts.program == program)
    }

    /// Every expected-divergence marker the corpus declares, with the program that owns it.
    fn markers(&self) -> Vec<(&ProgramFacts, &ExpectedDivergence)> {
        self.programs
            .iter()
            .filter_map(|facts| facts.marker.as_ref().map(|marker| (facts, marker)))
            .collect()
    }

    /// Every narrowing every program declares, with the program that declares it.
    fn narrowings(&self) -> Vec<(&ProgramFacts, Narrowing)> {
        let mut narrowed = Vec::new();
        for facts in &self.programs {
            for narrowing in facts.narrowings() {
                narrowed.push((facts, narrowing));
            }
        }
        narrowed
    }

    /// How many cells the corpus plans for a run sweeping `targets` at `levels`.
    fn planned_cells(&self, targets: &[Target], levels: &[OptLevel]) -> usize {
        self.programs
            .iter()
            .map(|facts| facts.planned_cells(targets, levels))
            .sum()
    }
}

/// Target triples in canonical table order, comma-separated.
fn join_targets(targets: &[Target]) -> String {
    let ordered: Vec<&str> = Target::ALL
        .iter()
        .filter(|target| targets.contains(target))
        .map(|target| target.triple())
        .collect();
    if ordered.is_empty() {
        String::from(ABSENT_CELL)
    } else {
        ordered.join(", ")
    }
}

/// Optimization-level flags in ascending order, comma-separated.
fn join_opt_levels(levels: &[OptLevel]) -> String {
    let ordered: Vec<&str> = OptLevel::ALL
        .iter()
        .filter(|level| levels.contains(level))
        .map(|level| level.flag())
        .collect();
    if ordered.is_empty() {
        String::from(ABSENT_CELL)
    } else {
        ordered.join(", ")
    }
}

/// An area report on disk that this run did not write.
///
/// Kept as a first-class list rather than folded into a diagnostic string, because the summary has
/// to state three separate things about it: which area it claims to be, which run wrote it, and that
/// its rows were excluded from every total. A stale file is never deleted — this module does not
/// remove another run's artifacts — so replacing it is the run's own next write.
#[derive(Debug, Clone)]
struct StaleArea {
    spec: &'static AreaSpec,
    /// The generation the file carries, when it carries a readable one at all.
    generation: Option<Generation>,
    /// Why the file is not this run's, in the words the report prints.
    note: String,
}

/// The whole run, aggregated from the fourteen per-area machine-readable reports.
#[derive(Debug, Clone, Default)]
struct RunReport {
    areas: Vec<AreaReport>,
    /// Areas whose file existed but could not be used, so their outcomes are absent from the
    /// totals. Listed, never quietly skipped.
    unusable: Vec<&'static AreaSpec>,
    /// Areas whose file belongs to a different run. Excluded from every total and listed by name,
    /// which is what stops one run's summary from reporting another run's results.
    stale: Vec<StaleArea>,
    tally: Tally,
    facts: CorpusFacts,
    /// Test-name filters this process was started with, which is what tells the summary that some
    /// areas were never going to run in this process at all.
    filters: Vec<String>,
    /// Signature of the run every aggregated area file was verified to carry. Recorded in the
    /// summary so that the identity the areas were checked against is itself auditable, rather than
    /// being a check whose subject is invisible in its own output.
    session: String,
    /// Areas this invocation expected to contribute, whether or not they did. Narrower than the
    /// full table exactly when a filter is active, and it is the denominator the coverage
    /// classification compares against.
    expected: usize,
    diagnostics: Vec<String>,
}

impl RunReport {
    /// Every row of every area carrying one verdict, in area order and then report order.
    fn rows_with(&self, verdict: Verdict) -> Vec<&Row> {
        self.areas
            .iter()
            .flat_map(|area| area.rows_with(verdict))
            .collect()
    }

    /// How many comparisons one oracle contributed across the run.
    fn oracle_count(&self, oracle: Oracle) -> usize {
        self.areas
            .iter()
            .map(|area| area.oracle_count(oracle))
            .sum()
    }

    /// How many distinct **runnable** programs produced at least one comparison.
    ///
    /// Reads the run's own corpus facts, so a program whose record could not be read is not counted as
    /// swept — see [`AreaReport::swept_programs`] for why that distinction is the whole of this
    /// finding. The facts are populated before any matrix or table is assembled.
    fn observed_programs(&self) -> usize {
        self.areas
            .iter()
            .map(|area| area.swept_programs(&self.facts))
            .sum()
    }

    /// How many distinct cells were actually executed.
    fn observed_cells(&self) -> usize {
        self.areas
            .iter()
            .map(|area| area.executed_cells(&self.facts))
            .sum()
    }

    /// How many rows exist to report a corpus defect rather than to record a comparison.
    fn corpus_defect_rows(&self) -> usize {
        self.areas
            .iter()
            .map(|area| area.corpus_defect_rows(&self.facts))
            .sum()
    }

    /// Every target that at least one recorded row was observed at, in canonical order.
    ///
    /// The *observed* set, never the configured one. A run can be configured for four targets and
    /// record rows at one — an emulator absent from the environment does exactly that — and reporting
    /// the configuration in the recorded column would answer "what did this run sweep?" with what it
    /// intended to sweep.
    fn observed_targets(&self) -> Vec<Target> {
        Target::ALL
            .iter()
            .copied()
            .filter(|target| self.areas.iter().any(|area| area.targets.contains(target)))
            .collect()
    }

    /// Every optimization level at least one recorded row was observed at, in canonical order.
    fn observed_opt_levels(&self) -> Vec<OptLevel> {
        OptLevel::ALL
            .iter()
            .copied()
            .filter(|level| {
                self.areas
                    .iter()
                    .any(|area| area.opt_levels.contains(level))
            })
            .collect()
    }

    /// How many recorded outcomes fail the run under the policy in force.
    fn failing(&self, caps: &Capabilities) -> usize {
        self.areas
            .iter()
            .flat_map(|area| area.rows.iter())
            .filter(|row| verdict_fails_run(row.verdict, caps.config()))
            .count()
    }
}

// ---------------------------------------------------------------------------------------------
// The enumerable matrix
//
// The suite's coverage evidence is a count of what was planned beside a count of what was actually
// recorded, in every dimension the requirements name. It is assembled once, here, and read three
// times: by the Markdown table, by the machine-readable summary, and by the coverage assessment
// that decides whether the report may call itself full. Assembling it once is the point — a table
// that showed a shortfall next to a heading claiming full coverage would mislead every reader who
// trusted the heading, and deriving both from the same list makes that disagreement impossible.
// ---------------------------------------------------------------------------------------------

/// One dimension of the matrix: what was planned, what was recorded, and what it is called.
#[derive(Debug, Clone)]
struct MatrixDimension {
    /// Machine-readable name, used as the `label` column of a `matrix` row. Stable across runs so
    /// an external aggregator can address one dimension by name.
    label: &'static str,
    /// Human-readable name, used as the first cell of the Markdown table's row.
    heading: &'static str,
    planned: usize,
    actual: usize,
    note: &'static str,
    plan: PlanKind,
}

/// How a dimension's planned number is to be read against its recorded one.
///
/// Most of the matrix is an **exact** target: a cell count, a comparison count, a target count. For
/// those, recording more than was planned is a defect — a duplicate, a synthetic row counted as a
/// cell, or a corpus that outgrew its declared count — and the excess is reported as one.
///
/// One dimension is a **floor**, and it has to be read the other way round. The coverage requirement
/// states a *minimum* number of programs for each mandated area, and this corpus deliberately exceeds
/// it in nine of the nine: the plan's own words are "no fewer than six programs each", and the areas
/// hold seven, eight, ten and eleven. Comparing a floor for equality turned that surplus into a
/// warning and stamped eight of the fourteen area reports `partial` on a completely clean run, which
/// is the exact misreading the machine-readable coverage field exists to prevent — inverted, and
/// therefore worse, because a consumer now acts on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanKind {
    /// The recorded number is expected to equal the planned one; either direction is a defect.
    Exact,
    /// The recorded number is expected to be **at least** the planned one; a surplus is coverage above
    /// plan and is reported as met, with the margin named rather than hidden behind a bare tick.
    Floor,
}

impl MatrixDimension {
    /// Assemble one dimension whose planned number is an exact target.
    fn new(
        label: &'static str,
        heading: &'static str,
        planned: usize,
        actual: usize,
        note: &'static str,
    ) -> MatrixDimension {
        MatrixDimension {
            label,
            heading,
            planned,
            actual,
            note,
            plan: PlanKind::Exact,
        }
    }

    /// Assemble one dimension whose planned number is a floor rather than a target.
    fn floor(
        label: &'static str,
        heading: &'static str,
        planned: usize,
        actual: usize,
        note: &'static str,
    ) -> MatrixDimension {
        MatrixDimension {
            plan: PlanKind::Floor,
            ..MatrixDimension::new(label, heading, planned, actual, note)
        }
    }

    /// How far short of its plan this dimension fell, or `None` when it did not fall short.
    fn shortfall(&self) -> Option<usize> {
        self.planned.checked_sub(self.actual).filter(|gap| *gap > 0)
    }

    /// How far *past* its plan this dimension went, or `None` when it did not exceed it.
    ///
    /// # Why an excess is a warning rather than a success
    ///
    /// It used to render as `✅ complete`, on the reading that recording more than was planned cannot
    /// be a coverage gap. That reading is right about coverage and wrong about the report: nothing in
    /// this suite plans to record more comparisons than the matrix contains, so an excess is always
    /// one of a small number of defects — a duplicated comparison, which inflates every tally it
    /// touches; a synthetic row counted as though a cell had run; or a corpus that has grown past the
    /// count the tables still declare. Every one of those makes the recorded number untrustworthy,
    /// and a tick beside it told a reader the opposite. The count states are therefore three rather
    /// than two, and the excess is named with the same prominence as a shortfall.
    ///
    /// A [`PlanKind::Floor`] dimension has no excess at all, by definition: its planned number is a
    /// minimum, so exceeding it is the plan being met with room to spare rather than a count that
    /// cannot be trusted. The margin is still reported — see [`MatrixDimension::surplus`] — because
    /// "ten programs where six were required" is coverage information, but it is never a warning and
    /// never makes a report partial.
    fn excess(&self) -> Option<usize> {
        match self.plan {
            PlanKind::Exact => self.actual.checked_sub(self.planned).filter(|gap| *gap > 0),
            PlanKind::Floor => None,
        }
    }

    /// By how much a floor dimension exceeds its minimum, or `None` when it is exactly at it or is not
    /// a floor at all.
    fn surplus(&self) -> Option<usize> {
        match self.plan {
            PlanKind::Floor => self.actual.checked_sub(self.planned).filter(|gap| *gap > 0),
            PlanKind::Exact => None,
        }
    }

    /// The status cell: met, met with a margin, short by how much, or over by how much.
    fn status(&self) -> String {
        match (self.shortfall(), self.excess(), self.surplus()) {
            (Some(gap), _, _) => format!("⚠️ short by {gap}"),
            (_, Some(gap), _) => format!("⚠️ over by {gap}"),
            (_, _, Some(margin)) => format!("✅ met, {margin} above the floor"),
            (None, None, None) => String::from("✅ complete"),
        }
    }

    /// The status as it appears in the machine-readable summary, where no glyph is used.
    fn machine_status(&self) -> String {
        match (self.shortfall(), self.excess(), self.surplus()) {
            (Some(gap), _, _) => format!("short by {gap}; {}", self.note),
            (_, Some(gap), _) => format!("over by {gap}; {}", self.note),
            (_, _, Some(margin)) => format!("met, {margin} above the floor; {}", self.note),
            (None, None, None) => format!("complete; {}", self.note),
        }
    }

    /// This dimension as a row of the Markdown matrix table.
    fn markdown_row(&self) -> Vec<String> {
        vec![
            String::from(self.heading),
            self.planned.to_string(),
            self.actual.to_string(),
            self.status(),
            String::from(self.note),
        ]
    }

    /// This dimension's shortfall stated as a coverage reason, or `None` when it met its plan.
    fn shortfall_reason(&self) -> Option<String> {
        self.shortfall().map(|gap| {
            format!(
                "{}: {} planned, {} recorded — short by {gap} ({}).",
                self.heading, self.planned, self.actual, self.note
            )
        })
    }

    /// This dimension's excess stated as a coverage reason, or `None` when it did not exceed its plan.
    ///
    /// An excess keeps a report from calling itself full for the same reason a shortfall does: the
    /// recorded count and the planned count disagree, and until the cause is known the table cannot be
    /// read as a description of one clean sweep. The wording says "recorded more than planned" rather
    /// than naming a cause, because the causes differ — a duplicate, a synthetic row, or a corpus that
    /// outgrew its declared count — and the diagnostics list is where each is named.
    fn excess_reason(&self) -> Option<String> {
        self.excess().map(|gap| {
            format!(
                "{}: {} planned, {} recorded — {gap} more than planned ({}).",
                self.heading, self.planned, self.actual, self.note
            )
        })
    }
}

/// The run's matrix: the nine dimensions the coverage requirement enumerates.
///
/// `planned` is the full declared matrix in every dimension, never the reduced one, because the
/// question this table answers is what the suite claims to cover — a run that narrowed itself
/// should show the narrowing here rather than redefine the target it is measured against.
fn run_matrix(run: &RunReport, caps: &Capabilities) -> Vec<MatrixDimension> {
    let (targets, levels) = caps.config().effective_matrix();
    let runnable = run.facts.programs.len();
    vec![
        MatrixDimension::new(
            "feature_areas",
            "Feature areas",
            AREA_COUNT,
            run.areas.len(),
            "nine mandated by the coverage requirement, five supplementary",
        ),
        // Five program-and-cell dimensions where there used to be two, because "the corpus holds
        // 108 programs", "108 of them have a readable record" and "108 of them were swept" are three
        // different claims and only the last is evidence about the compiler. Reporting the last as
        // though it followed from the first is what let a summary claim the full corpus while a sixth
        // of it had no expectation record at all.
        MatrixDimension::new(
            "programs_discovered",
            "Programs (sources discovered)",
            PROGRAM_COUNT,
            run.facts.discovered,
            "`.c` files found in the areas this run aggregated",
        ),
        MatrixDimension::new(
            "programs_runnable",
            "Programs (runnable pairs)",
            run.facts.discovered,
            runnable,
            "a source is runnable only once its own `.expected` record parses",
        ),
        MatrixDimension::new(
            "programs_swept",
            "Programs (produced a comparison)",
            runnable,
            run.observed_programs(),
            "every runnable pair is expected to be judged",
        ),
        MatrixDimension::new(
            "targets",
            "Targets swept",
            Target::ALL.len(),
            run.observed_targets().len(),
            "recorded rows, not the configuration — x86-64 is the baseline and runs natively",
        ),
        MatrixDimension::new(
            "opt_levels",
            "Optimization levels swept",
            OptLevel::ALL.len(),
            run.observed_opt_levels().len(),
            "recorded rows, not the configuration — -O0, -O1, -O2 mean the same to both compilers",
        ),
        // The configuration stated as its own dimension rather than substituted for the sweep. Both
        // facts matter and they answer different questions: this one is what the run set out to do,
        // the two above are what it did. A quick run shows the narrowing here, and an environment
        // missing an emulator shows it above.
        MatrixDimension::new(
            "targets_configured",
            "Targets configured for this run",
            Target::ALL.len(),
            targets.len(),
            "the effective matrix this invocation was configured to sweep",
        ),
        MatrixDimension::new(
            "opt_levels_configured",
            "Optimization levels configured for this run",
            OptLevel::ALL.len(),
            levels.len(),
            "the effective matrix this invocation was configured to sweep",
        ),
        MatrixDimension::new(
            "bcc_cells",
            "Compile-and-run cells executed",
            BCC_CELL_COUNT,
            run.observed_cells(),
            "one program, one target, one optimization level — synthetic rows excluded",
        ),
        MatrixDimension::new(
            "corpus_defect_rows",
            "Corpus-defect rows",
            0,
            run.corpus_defect_rows(),
            "rows filed for a source with no readable record; each is a failure, never a cell",
        ),
        MatrixDimension::new(
            "oracle_a_comparisons",
            "Oracle (a) comparisons",
            ORACLE_A_COMPARISON_COUNT,
            run.oracle_count(Oracle::ReferenceCompiler),
            "reference compiler, same target and same optimization level",
        ),
        MatrixDimension::new(
            "oracle_b_comparisons",
            "Oracle (b) comparisons",
            ORACLE_B_COMPARISON_COUNT,
            run.oracle_count(Oracle::CrossBackend),
            "every non-baseline target against the baseline at the same level",
        ),
        MatrixDimension::new(
            "oracle_c_assertions",
            "Oracle (c) assertions",
            ORACLE_C_ASSERTION_COUNT,
            run.oracle_count(Oracle::GoldenRecord),
            "every cell against the stdout its own record declares",
        ),
        MatrixDimension::new(
            "total_assertions",
            "**Differential and golden assertions**",
            TOTAL_ASSERTION_COUNT,
            run.tally.total(),
            "the sum of the three oracles",
        ),
    ]
}

/// One area's matrix, in the same shape as the run's so the same predicate judges both.
///
/// Two of the dimensions take their plan from the corpus rather than from the declared totals,
/// because an area's plan is what its own records declare: a program that restricts its targets for
/// a recorded reason has not left a gap, and counting one would turn every reasoned exclusion into
/// a permanent coverage complaint. The union of the programs' own declared targets, intersected with
/// the matrix this run sweeps, is therefore the honest plan.
fn area_matrix(
    report: &AreaReport,
    facts: &CorpusFacts,
    caps: &Capabilities,
) -> Vec<MatrixDimension> {
    let spec = report.spec;
    let (targets, levels) = caps.config().effective_matrix();
    let discovered = facts.programs.len();
    let planned_targets = Target::ALL
        .iter()
        .filter(|target| targets.contains(target))
        .filter(|target| {
            facts
                .programs
                .iter()
                .any(|program| program.targets.contains(target))
        })
        .count();
    let planned_levels = OptLevel::ALL
        .iter()
        .filter(|level| levels.contains(level))
        .filter(|level| {
            facts
                .programs
                .iter()
                .any(|program| program.opt_levels.contains(level))
        })
        .count();

    let mut dimensions = vec![
        MatrixDimension::new(
            "programs_discovered",
            "Programs (sources discovered)",
            spec.program_count(),
            facts.discovered,
            "`.c` files found in this area's directory",
        ),
        MatrixDimension::new(
            "programs_runnable",
            "Programs (runnable pairs)",
            facts.discovered,
            discovered,
            "a source is runnable only once its own `.expected` record parses",
        ),
    ];
    if spec.mandated() {
        // A floor, not a target: the coverage requirement states a minimum per mandated area and this
        // corpus deliberately exceeds it in every one of the nine. Read as an equality it reported a
        // surplus as a defect and made a clean area report call itself partial.
        dimensions.push(MatrixDimension::floor(
            "mandated_floor",
            "Mandated-area program floor",
            MIN_PROGRAMS_PER_MANDATED_AREA,
            discovered,
            "an area named by the coverage requirement holds at least this many programs",
        ));
    }
    dimensions.extend([
        MatrixDimension::new(
            "programs_compared",
            "Programs that produced a comparison",
            discovered,
            report.swept_programs(facts),
            "every runnable pair is expected to be judged",
        ),
        MatrixDimension::new(
            "targets_swept",
            "Targets swept",
            planned_targets,
            report.observed_targets().len(),
            "the targets this area's own records declare, within the matrix this run sweeps",
        ),
        MatrixDimension::new(
            "opt_levels_swept",
            "Optimization levels swept",
            planned_levels,
            report.observed_opt_levels().len(),
            "the levels this area's own records declare, within the matrix this run sweeps",
        ),
        MatrixDimension::new(
            "cells_compared",
            "Compile-and-run cells executed",
            facts.planned_cells(&targets, &levels),
            report.executed_cells(facts),
            "each record's own target and level lists, intersected with this run's matrix",
        ),
        MatrixDimension::new(
            "corpus_defect_rows",
            "Corpus-defect rows",
            0,
            report.corpus_defect_rows(facts),
            "rows filed for a source with no readable record; each is a failure, never a cell",
        ),
    ]);
    dimensions
}

/// The matrix as a Markdown table.
fn render_matrix_table(dimensions: &[MatrixDimension]) -> Vec<String> {
    let rows: Vec<Vec<String>> = dimensions
        .iter()
        .map(MatrixDimension::markdown_row)
        .collect();
    markdown_table(
        &["Dimension", "Planned", "Recorded", "Status", "Note"],
        &rows,
    )
}

// ---------------------------------------------------------------------------------------------
// The preflight gates
//
// Two gates establish the preconditions the two differential oracles rest on, and neither is a
// comparison: the flag-capability probe establishes that every flag a differential invocation passes
// means the same thing to both compilers (requirement 3), and the undefined-behaviour audit
// establishes that the corpus is free of undefined behaviour (requirement 1). While either is
// unsatisfied, a PASS is not evidence of agreement and a divergence is not evidence of a defect —
// requirement 1 says so in those terms, because a program containing undefined behaviour permits
// both compilers to do anything.
//
// They therefore cannot be independent tests whose failure leaves every area's verdict standing.
// Under the built-in harness the eighteen tests run concurrently in one process with no ordering
// between them, so "run the gate test first" is not something a caller can arrange and not something
// a test can assert. What is achievable, and what this section implements, is:
//
//   * the driver performs each gate **once per process**, memoized, so every test — area or
//     infrastructure — observes the same result at the cost of one execution;
//   * that result is **recorded here**, once, so that every artifact this module renders carries it
//     without the gate having to be threaded through a dozen signatures or re-run per area;
//   * a gate that did not hold is **named in the area report and in the run summary**, counted in
//     the run's verdict, and asserted on by every area it governs.
//
// A gate that could not be performed at all is deliberately not the same as one that failed, and
// neither is silently a pass. A gate that was performed and did not hold is a defect in the test
// material: it blocks, always, and no setting excuses it. A gate that could not be performed because
// the tool it needs is absent is the environment's incompleteness rather than the corpus's, so it is
// reported as reduced coverage and escalated to a failure only under the strict setting — the same
// rule the suite already applies to an oracle whose tooling is missing, and the same rule
// `AuditReport::fails_run` and `FlagProbeReport::fails_run` already implement. Either way the gate is
// named in every artifact, so neither shape can be mistaken for a pass, and the wording tells a
// reader whether to fix a program or fix the machine.
// ---------------------------------------------------------------------------------------------

/// What one preflight gate established, or did not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateVerdict {
    /// The gate was performed and its precondition holds.
    Held,
    /// The gate was performed and its precondition does not hold.
    Failed,
    /// The gate could not be performed, so nothing has been established either way.
    Unperformed,
}

impl GateVerdict {
    /// The word the reports use.
    pub fn label(self) -> &'static str {
        match self {
            GateVerdict::Held => "HELD",
            GateVerdict::Failed => "FAILED",
            GateVerdict::Unperformed => "UNPERFORMED",
        }
    }
}

impl fmt::Display for GateVerdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// One preflight gate as the reports record it.
#[derive(Debug, Clone)]
pub struct PreflightGate {
    name: String,
    requirement: String,
    verdict: GateVerdict,
    detail: String,
    /// Areas whose comparisons this gate's outcome bears on. Empty means every area, which is the
    /// ordinary case: a corpus-wide or configuration-wide precondition governs the whole run.
    areas: Vec<String>,
    /// Whether this gate's outcome forbids the comparisons it governs from being trusted.
    blocks: bool,
}

impl PreflightGate {
    /// Record one gate.
    ///
    /// `requirement` names the numbered requirement the gate establishes, and `detail` is the
    /// sentence a report prints beside the verdict — for a gate that did not hold it must say what
    /// was observed, because a bare `FAILED` sends a reader to the harness rather than to the
    /// program or the machine that caused it.
    ///
    /// `areas` narrows the gate to the areas it actually bears on. The undefined-behaviour audit
    /// uses that: a program in one area whose gate failed says nothing about another area's
    /// programs, and failing every area for it would report thirteen defects where there is one.
    ///
    /// # Why `blocks` is given rather than derived from the verdict
    ///
    /// The verdict is an **observation** and `blocks` is a **policy decision**, and the two come
    /// apart in exactly one case that the suite's own rules settle: a gate that could not be applied
    /// because the tool it needs is absent. That is `Unperformed`, and it fails the run only under
    /// the strict setting intended for continuous integration — outside it the gap is reported and
    /// does not fail, because there the environment rather than the corpus is what is incomplete.
    /// Deriving `blocks` from the verdict here would either contradict that rule or force the verdict
    /// word to lie about what was observed.
    ///
    /// A `Failed` gate always blocks, whatever the caller passes, because a precondition that was
    /// tested and does not hold is a defect in the test material and no setting excuses it.
    pub fn new(
        name: impl Into<String>,
        requirement: impl Into<String>,
        verdict: GateVerdict,
        detail: impl Into<String>,
        areas: Vec<String>,
        blocks: bool,
    ) -> PreflightGate {
        PreflightGate {
            name: name.into(),
            requirement: requirement.into(),
            verdict,
            detail: detail.into(),
            areas,
            blocks: blocks || verdict == GateVerdict::Failed,
        }
    }

    /// The gate's name, as the reports print it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The numbered requirement this gate establishes.
    pub fn requirement(&self) -> &str {
        &self.requirement
    }

    /// What the gate established, or did not.
    pub fn verdict(&self) -> GateVerdict {
        self.verdict
    }

    /// What was observed, in the sentence a report prints beside the verdict.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Whether this gate's outcome forbids the comparisons it governs from being read as evidence.
    pub fn blocks(&self) -> bool {
        self.blocks
    }

    /// Whether this gate's outcome bears on the given area.
    ///
    /// A gate with no recorded areas governs every one of them, which is what a configuration-wide
    /// precondition such as flag parity actually does.
    pub fn governs(&self, area: &str) -> bool {
        self.areas.is_empty() || self.areas.iter().any(|recorded| recorded == area)
    }

    /// The one-line account a report prints: verdict, requirement and detail.
    pub fn describe(&self) -> String {
        // Read through this type's own accessors rather than its fields, here and at every other
        // read in this module, so a gate has exactly one read path and a future accessor cannot
        // drift away from what is rendered. The same discipline the flag probe's row type follows.
        format!(
            "{} — {} ({}): {}",
            self.verdict().label(),
            self.name(),
            self.requirement(),
            self.detail()
        )
    }
}

/// Every preflight gate this run performed.
#[derive(Debug, Clone, Default)]
pub struct Preflight {
    gates: Vec<PreflightGate>,
}

impl Preflight {
    /// Assemble a preflight from its gates.
    pub fn new(gates: Vec<PreflightGate>) -> Preflight {
        Preflight { gates }
    }

    /// The gates, in the order they were recorded.
    pub fn gates(&self) -> &[PreflightGate] {
        &self.gates
    }

    /// Every gate that forbids the comparisons it governs from being read as evidence.
    pub fn blocking(&self) -> Vec<&PreflightGate> {
        self.gates.iter().filter(|gate| gate.blocks()).collect()
    }

    /// Every gate that blocks the given area, which is what an area asserts on.
    pub fn blocking_area(&self, area: &str) -> Vec<&PreflightGate> {
        self.blocking()
            .into_iter()
            .filter(|gate| gate.governs(area))
            .collect()
    }

    /// Every gate that was not performed and, by the policy in force, does not block.
    ///
    /// Reported as reduced coverage rather than as a failure: nothing was established, so the report
    /// may not call itself full, but the gap belongs to the machine rather than to the corpus and the
    /// suite's own rule is that only the strict setting escalates it.
    pub fn unperformed_permitted(&self) -> Vec<&PreflightGate> {
        self.gates
            .iter()
            .filter(|gate| !gate.blocks() && gate.verdict() == GateVerdict::Unperformed)
            .collect()
    }
}

/// Record this run's preflight once, so that every artifact this module renders carries it.
///
/// The driver performs the gates and calls this exactly once per process. Recording rather than
/// threading is deliberate: the summary is finalized by whichever area finishes last, on whichever
/// thread that is, and the area reports are written by fourteen different threads. Passing the
/// preflight down every one of those paths would mean fourteen chances to pass it and one to forget,
/// and a renderer that had not been given it would silently omit the precondition instead of naming
/// it.
///
/// Returns whether this call was the one that recorded it. A second call is a no-op and returns
/// `false` rather than replacing the record: a preflight that could be overwritten mid-run would let
/// a later, narrower gate result stand in for the one the areas were actually judged against.
pub fn record_preflight(preflight: Preflight) -> bool {
    RECORDED_PREFLIGHT.set(preflight).is_ok()
}

/// This run's recorded preflight, or `None` when the driver has not recorded one.
///
/// `None` is treated as fail-closed everywhere it is read: the reports say the gates were not
/// recorded and stamp themselves partial, rather than rendering a section that looks like a clean
/// preflight. A report that omitted the preconditions silently would invite exactly the reading the
/// gates exist to prevent.
fn recorded_preflight() -> Option<&'static Preflight> {
    RECORDED_PREFLIGHT.get()
}

/// This run's preflight, recorded once by the driver. See [`record_preflight`].
static RECORDED_PREFLIGHT: OnceLock<Preflight> = OnceLock::new();

/// Record what the preflight means for a report's coverage claim.
///
/// Shared by the area assessment and the run assessment so the two cannot describe the same preflight
/// differently. `area` narrows the blocking set to the gates that bear on one area; `None` reports the
/// whole run's.
///
/// Two distinct facts are recorded under the two words this module already uses. A **blocking** gate
/// makes the report *partial*: its comparisons are not evidence, so the report may not claim to
/// describe one coherent result. A gate that merely could not be applied, and which the policy in
/// force does not escalate, makes the report *reduced*: something the full sweep would have
/// established was not established, which is the same shape as an oracle whose tooling is absent.
fn note_preflight(coverage: &mut Coverage, area: Option<&str>) {
    let Some(preflight) = recorded_preflight() else {
        coverage.note_partial(String::from(
            "This run's preflight gates were not recorded, so neither flag parity (requirement 3) \
             nor undefined-behaviour freedom (requirement 1) has been established for the \
             comparisons below. Nothing here may be read as evidence about a compiler.",
        ));
        return;
    };
    let blocking = match area {
        Some(area) => preflight.blocking_area(area),
        None => preflight.blocking(),
    };
    if !blocking.is_empty() {
        coverage.note_partial(format!(
            "{} preflight gate(s) did not hold, so the preconditions the differential oracles rest \
             on are unmet and no comparison below is evidence about a compiler: {}",
            blocking.len(),
            blocking
                .iter()
                .map(|gate| gate.describe())
                .collect::<Vec<String>>()
                .join(" | ")
        ));
    }
    let permitted: Vec<&PreflightGate> = preflight
        .unperformed_permitted()
        .into_iter()
        .filter(|gate| area.map(|area| gate.governs(area)).unwrap_or(true))
        .collect();
    if !permitted.is_empty() {
        coverage.note_reduced(format!(
            "{} preflight gate(s) could not be applied on this machine, so the precondition each \
             one establishes is unproven rather than proven or disproven. Under the policy in force \
             that is reported and does not fail the run; {VAR_STRICT} escalates it: {}",
            permitted.len(),
            permitted
                .iter()
                .map(|gate| gate.describe())
                .collect::<Vec<String>>()
                .join(" | ")
        ));
    }
}

/// The preflight section every report carries, as Markdown lines.
///
/// Rendered whether or not the gates held, and rendered when they were not recorded at all, because
/// the section's presence is what tells a reader the preconditions were considered. `area` narrows
/// the blocking set to the gates that bear on one area; `None` reports the whole run's.
fn render_preflight_section(area: Option<&str>) -> Vec<String> {
    let mut lines = vec![
        String::from("## Preflight gates — the preconditions the oracles rest on"),
        String::new(),
    ];
    let Some(preflight) = recorded_preflight() else {
        lines.push(String::from(
            "**⚠️ NOT RECORDED.** This run did not record its preflight gates, so neither flag \
             parity (requirement 3) nor undefined-behaviour freedom of the corpus (requirement 1) \
             has been established. Every comparison below was still performed and is still \
             reported, but none of it is evidence about a compiler: a program containing undefined \
             behaviour permits both compilers to do anything, and a flag that means two things \
             makes two invocations incomparable.",
        ));
        lines.push(String::new());
        return lines;
    };
    let mut rows: Vec<(String, String)> = Vec::new();
    for gate in preflight.gates() {
        rows.push((
            format!("{} — {}", gate.name(), gate.requirement()),
            format!("{} — {}", gate.verdict().label(), table_cell(gate.detail())),
        ));
    }
    let blocking = match area {
        Some(area) => preflight.blocking_area(area),
        None => preflight.blocking(),
    };
    rows.push((
        String::from("Verdict"),
        match blocking.is_empty() {
            true => String::from("✅ every gate that governs this report held"),
            false => format!(
                "⚠️ {} gate(s) did not hold, so no comparison here is evidence about a compiler",
                blocking.len()
            ),
        },
    ));
    lines.extend(property_table(&rows));
    lines.push(String::new());
    if !blocking.is_empty() {
        lines.push(String::from(
            "A gate that did not hold is a defect in the **test material or the machine**, never a \
             verdict about the compiler under test. Correct the program the audit names, or install \
             the tool the probe names, and run again; the comparisons below are retained so the \
             correction can be checked against them, not so they can be relied on.",
        ));
        lines.push(String::new());
    }
    lines
}

/// How many recorded gates block, for a caller that needs the count alone.
///
/// `area` narrows the count to the gates that bear on one area; `None` counts the whole run's.
///
/// A run that recorded no preflight counts as **one** blocking gate rather than none. That is the
/// fail-closed reading and it is deliberate: `0` would be indistinguishable, in every table and every
/// machine-readable field, from a preflight that ran and held.
fn blocking_gate_count(area: Option<&str>) -> usize {
    match (recorded_preflight(), area) {
        (Some(preflight), Some(area)) => preflight.blocking_area(area).len(),
        (Some(preflight), None) => preflight.blocking().len(),
        (None, _) => 1,
    }
}

// ---------------------------------------------------------------------------------------------
// Coverage stamping
//
// A report that could be mistaken for a complete one is worse than no report, because it invites a
// reader to conclude that a matrix nobody swept came out clean. The stamp is therefore in the first
// heading and the first line, both halves of the summary carry it, and the reasons are enumerated
// rather than summarised.
//
// Two words are used, and they mean different things. **Reduced** says the matrix that ran was
// smaller than the full one — quick mode, a program filter, or an oracle whose tooling is absent.
// **Partial** says this report may not describe one complete, coherent run — everything reduced
// does, plus a test-name filter, an area whose file could not be used or belongs to another run, a
// defect in the corpus the report had to read, and any diagnostic raised while assembling it.
// Partial therefore always holds when reduced does.
//
// A dimension of the matrix that fell short of its plan settles it either way, and never leaves the
// report full: it is recorded as reduced when the run configuration itself asked for a smaller
// matrix, and as partial when it did not — because a shortfall nothing in the configuration explains
// is precisely the case where the report cannot claim to describe one complete run. Which of the two
// words it lands under is presentational; that it lands under one of them is what makes the FULL
// stamp mean something, and the reason text carries the planned and recorded counts either way.
// ---------------------------------------------------------------------------------------------

/// Whether a report describes the full matrix, and why not when it does not.
#[derive(Debug, Clone, Default)]
struct Coverage {
    reduced: bool,
    partial: bool,
    reasons: Vec<String>,
}

impl Coverage {
    /// Record a reason the matrix that ran was smaller than the full one.
    fn note_reduced(&mut self, reason: String) {
        self.reduced = true;
        self.reasons.push(reason);
    }

    /// Record a reason this report may not describe one complete, coherent run.
    fn note_partial(&mut self, reason: String) {
        self.partial = true;
        self.reasons.push(reason);
    }

    /// Whether the report is partial, which a reduced matrix always makes it.
    ///
    /// The two flags are recorded separately so the stamp can say which of the two facts holds, but
    /// a reduced matrix is by definition not one complete run, so this is what the published
    /// `partial` field reports.
    fn is_partial(&self) -> bool {
        self.partial || self.reduced
    }

    /// The words appended to the first heading, so the stamp cannot be missed or footnoted.
    ///
    /// When both facts hold, both are named. Announcing only the reduced matrix would let a reader
    /// conclude that the smaller matrix at least ran coherently, which is a different and unearned
    /// claim.
    fn heading_suffix(&self) -> &'static str {
        match (self.reduced, self.partial) {
            (true, true) => " — ⚠️ REDUCED COVERAGE AND PARTIAL REPORT",
            (true, false) => " — ⚠️ REDUCED COVERAGE",
            (false, true) => " — ⚠️ PARTIAL REPORT",
            (false, false) => "",
        }
    }

    /// The first line of the report, stating the coverage claim in full.
    fn statement(&self) -> String {
        match (self.reduced, self.partial) {
            (true, true) => String::from(
                "**⚠️ REDUCED COVERAGE AND PARTIAL REPORT — this run swept less than the full \
                 matrix, and this artifact may not describe one complete, coherent run. It must not \
                 be read as a complete result.**",
            ),
            (true, false) => String::from(
                "**⚠️ REDUCED COVERAGE — this run swept less than the full matrix, so it must not \
                 be read as a complete result.**",
            ),
            (false, true) => String::from(
                "**⚠️ PARTIAL REPORT — this artifact may not describe one complete, coherent run.**",
            ),
            (false, false) => String::from(
                "**Coverage: FULL — every feature area, every target and every optimization level \
                 in scope was swept, and every oracle was available.**",
            ),
        }
    }
}

/// Assess one area's coverage, from the same dimensions its matrix table publishes.
fn assess_area_coverage(
    caps: &Capabilities,
    report: &AreaReport,
    facts: &CorpusFacts,
    dimensions: &[MatrixDimension],
) -> Coverage {
    let mut coverage = Coverage::default();
    note_configuration_reductions(caps, &mut coverage);
    // The preconditions come first, because they decide what the rest of the assessment is worth. An
    // area whose gate did not hold is not a smaller run, it is a run whose comparisons cannot be read
    // as evidence, so the report may never call itself full.
    note_preflight(&mut coverage, Some(report.spec.directory()));
    let unavailable = report.tally.get(Verdict::Unavailable);
    if unavailable > 0 {
        coverage.note_reduced(format!(
            "{unavailable} comparison(s) in this area reported UNAVAILABLE: an oracle's tooling is \
             absent from this environment, so those comparisons were not attempted."
        ));
    }
    if report.rows.is_empty() {
        coverage.note_partial(String::from(
            "No comparison was recorded for this area at all, so nothing in it has been judged.",
        ));
    }
    note_matrix_shortfalls(caps, dimensions, &mut coverage);
    if !report.diagnostics.is_empty() {
        coverage.note_partial(format!(
            "{} diagnostic(s) were raised while assembling this report; see the Diagnostics \
             section.",
            report.diagnostics.len()
        ));
    }
    if !facts.diagnostics.is_empty() {
        coverage.note_partial(format!(
            "{} defect(s) in this area's expectation records prevented part of the material this \
             report rests on from being read; see the Diagnostics section.",
            facts.diagnostics.len()
        ));
    }
    coverage
}

/// Assess the whole run's coverage, from the same dimensions the matrix table publishes.
fn assess_run_coverage(
    caps: &Capabilities,
    run: &RunReport,
    dimensions: &[MatrixDimension],
) -> Coverage {
    let mut coverage = Coverage::default();
    note_configuration_reductions(caps, &mut coverage);
    // As in the per-area assessment, and for the same reason: an unmet precondition is not a narrower
    // run but a run whose comparisons are not evidence, so it settles the coverage claim first.
    note_preflight(&mut coverage, None);
    let unavailable = run.tally.get(Verdict::Unavailable);
    if unavailable > 0 {
        coverage.note_reduced(format!(
            "{unavailable} comparison(s) reported UNAVAILABLE: an oracle's tooling is absent from \
             this environment, so those comparisons were not attempted."
        ));
    }
    let arms = caps.unavailable_oracle_arms();
    if !arms.is_empty() {
        coverage.note_reduced(format!(
            "{} oracle arm(s) cannot be attempted on this machine; each one is listed with the \
             tool it needs and the environment variable that would supply it.",
            arms.len()
        ));
    }
    if !run.filters.is_empty() {
        coverage.note_partial(format!(
            "A test-name filter is active ({}), so this process ran a subset of the suite and some \
             per-area reports may predate this run.",
            run.filters
                .iter()
                .map(|filter| format!("`{}`", sanitize_text_for_report(filter)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if run.areas.len() < AREA_COUNT {
        coverage.note_partial(format!(
            "{} of {AREA_COUNT} feature areas contributed outcomes to this summary; this \
             invocation could produce at most {}.",
            run.areas.len(),
            run.expected
        ));
    }
    for spec in &run.unusable {
        coverage.note_partial(format!(
            "The machine-readable report of feature area `{}` could not be used, so its outcomes \
             are missing from every total below.",
            spec.directory()
        ));
    }
    for stale in &run.stale {
        coverage.note_partial(format!(
            "The machine-readable report of feature area `{}` was not written by this run, so it \
             was excluded from every total below: {}",
            stale.spec.directory(),
            stale.note
        ));
    }
    note_matrix_shortfalls(caps, dimensions, &mut coverage);
    if !run.facts.diagnostics.is_empty() {
        coverage.note_partial(format!(
            "{} corpus defect(s) prevented part of the expectation-record material from being \
             read; see the Diagnostics section.",
            run.facts.diagnostics.len()
        ));
    }
    if !run.diagnostics.is_empty() {
        coverage.note_partial(format!(
            "{} diagnostic(s) were raised while assembling this summary; see the Diagnostics \
             section.",
            run.diagnostics.len()
        ));
    }
    let area_diagnostics: usize = run.areas.iter().map(|area| area.diagnostics.len()).sum();
    if area_diagnostics > 0 {
        coverage.note_partial(format!(
            "{area_diagnostics} diagnostic(s) were raised in the per-area reports this summary \
             aggregates; see the Diagnostics section."
        ));
    }
    coverage
}

/// Record every dimension that fell short of its plan, so no shortfall can coexist with a full
/// stamp.
///
/// The bucket is chosen by whether the run configuration itself asked for a smaller matrix: quick
/// mode and a program filter narrow it deliberately and are therefore *reduced*, while a shortfall
/// on an unnarrowed run means this report does not describe one complete sweep and is therefore
/// *partial*. Either way the report is not full and the planned and recorded counts are stated.
fn note_matrix_shortfalls(
    caps: &Capabilities,
    dimensions: &[MatrixDimension],
    coverage: &mut Coverage,
) {
    let narrowed_by_configuration = caps.config().is_reduced_run();
    for reason in dimensions
        .iter()
        .filter_map(MatrixDimension::shortfall_reason)
    {
        if narrowed_by_configuration {
            coverage.note_reduced(reason);
        } else {
            coverage.note_partial(reason);
        }
    }
    // An excess is always *partial*, never *reduced*, and the asymmetry is deliberate. A reduced run
    // explains a smaller number — quick mode and a program filter both ask for one — but nothing in
    // any configuration asks for a number **larger** than the matrix contains, so a configuration
    // cannot excuse one. It means a duplicate, a synthetic row counted as a cell, or a corpus that has
    // outgrown its declared count, and none of the three is something a filter did.
    for reason in dimensions.iter().filter_map(MatrixDimension::excess_reason) {
        coverage.note_partial(reason);
    }
}

/// Record the reductions the run configuration itself imposes.
fn note_configuration_reductions(caps: &Capabilities, coverage: &mut Coverage) {
    let config = caps.config();
    if config.quick_mode() {
        coverage.note_reduced(format!(
            "{VAR_QUICK} is set: the matrix is reduced to the natively executing target at -O0 and \
             -O2, dropping the middle optimization level and every emulated target."
        ));
    }
    if let Some(filter) = config.only() {
        coverage.note_reduced(format!(
            "{VAR_ONLY} restricts the run to the single program `{}/{}`.",
            filter.area(),
            sanitize_text_for_report(filter.program())
        ));
    }
}

/// Long options of the built-in test harness that consume the argument after them.
///
/// Needed so that the value of an option is not mistaken for a test-name filter: `--test-threads 1`
/// must not be read as a filter named `1`. An option spelled `--name=value` needs no entry, because
/// its value is part of the same argument.
const LIBTEST_VALUE_OPTIONS: &[&str] = &[
    "--test-threads",
    "--logfile",
    "--skip",
    "--color",
    "--format",
    "--shuffle-seed",
    "--options",
    "-Z",
];

/// Which test-name filters this process was started with, split by what they do.
///
/// Kept apart because they answer different questions. A positive filter narrows the set of areas
/// that can possibly run, so it determines when the summary may be finalized. A `--skip` pattern
/// only removes tests, so it can leave an area's report permanently absent and must never be read
/// as evidence that the area was expected.
struct FilterSelection {
    /// Positional filters: a test runs only if its name contains one of these.
    positive: Vec<String>,
    /// Values of `--skip`: a test is excluded when its name contains one of these.
    skipped: Vec<String>,
}

impl FilterSelection {
    /// Whether the area test for `spec` could run in this process.
    ///
    /// Mirrors the built-in harness's own rule: a substring match against the test's name, which
    /// is `area_` followed by the area's directory. `--exact` is not honoured, and not honouring it
    /// is the safe direction — treating an area as expected when it was not merely leaves the
    /// summary pending, whereas treating one as unexpected when it did run would drop its outcomes
    /// from the totals.
    fn selects(&self, spec: &'static AreaSpec) -> bool {
        let name = format!("area_{}", spec.directory());
        if self.skipped.iter().any(|pattern| name.contains(pattern)) {
            return false;
        }
        self.positive.is_empty() || self.positive.iter().any(|filter| name.contains(filter))
    }

    /// The areas whose reports this process can ever produce.
    fn expected_areas(&self) -> Vec<&'static AreaSpec> {
        AREAS.iter().filter(|spec| self.selects(spec)).collect()
    }

    /// Every filter, in the order the command line gave them, for display.
    fn all(&self) -> Vec<String> {
        let mut every = self.positive.clone();
        every.extend(self.skipped.iter().cloned());
        every
    }
}

/// Read the filter selection from this process's own command line.
///
/// A filter means the process ran a subset of the suite, which matters twice over: it narrows the
/// set of area reports the summary may wait for, and it means some files in the report root may
/// have been written by an earlier run, which a summary that combined them without saying so would
/// present as current.
///
/// The reading is deliberately conservative in the direction that cannot lose outcomes. Anything
/// that is not an option and is not the value of a value-taking option counts as a positive filter.
/// Reading one argument too many narrows the expected set and can only leave the summary pending,
/// which is recoverable; failing to notice a filter would let the run claim full coverage it did not
/// have. `--skip` is read in both its spellings, `--skip pattern` and `--skip=pattern`.
///
/// [`std::env::args_os`] is used rather than [`std::env::args`] because the latter panics on an
/// argument that is not valid Unicode, and this module contains no panicking path.
fn libtest_filter_selection() -> FilterSelection {
    let mut positive = Vec::new();
    let mut skipped = Vec::new();
    let mut expect_value_of: Option<String> = None;
    for argument in std::env::args_os().skip(1) {
        let text = argument.to_string_lossy().to_string();
        if let Some(option) = expect_value_of.take() {
            if option == "--skip" {
                skipped.push(text);
            }
            continue;
        }
        if text.starts_with('-') {
            let name = text.split('=').next().unwrap_or(text.as_str());
            if let Some(value) = text.strip_prefix("--skip=") {
                skipped.push(String::from(value));
                continue;
            }
            if !text.contains('=') && LIBTEST_VALUE_OPTIONS.contains(&name) {
                expect_value_of = Some(String::from(name));
            }
            continue;
        }
        positive.push(text);
    }
    FilterSelection { positive, skipped }
}

// ---------------------------------------------------------------------------------------------
// Rendering
//
// From here down, nothing decides anything: every function turns already-aggregated facts into
// text. Reports are read by the same audience as the project guide, so they use the same register —
// pipe-delimited tables, em dashes, and the ✅ and ⚠️ glyphs for a state that is fine and a state
// that is not.
// ---------------------------------------------------------------------------------------------

/// A Markdown table: the header, the alignment rule, then one line per row.
fn markdown_table(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(format!("| {} |", headers.join(" | ")));
    lines.push(format!(
        "| {} |",
        headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for row in rows {
        lines.push(format!("| {} |", row.join(" | ")));
    }
    lines
}

/// A two-column `Property | Value` table.
fn property_table(pairs: &[(String, String)]) -> Vec<String> {
    let rows: Vec<Vec<String>> = pairs
        .iter()
        .map(|(property, value)| vec![property.clone(), value.clone()])
        .collect();
    markdown_table(&["Property", "Value"], &rows)
}

/// `✅ met` or `⚠️ NOT met`, for a claim the report checks rather than asserts.
fn met(ok: bool) -> &'static str {
    if ok {
        "✅ met"
    } else {
        "⚠️ NOT met"
    }
}

/// `yes` or `no`, for a policy question.
fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

/// A boolean as it is written into the machine-readable artifact.
///
/// Deliberately not [`yes_no`], which reads better in a prose table but is not what a consumer of
/// the tab-separated file expects. `true` and `false` are the conventional spellings there, and the
/// two the reporting discipline names for the reduced and partial fields specifically.
fn true_false(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// Why a finding's artifact directory falls short of being a deliverable, or `None` when it does not.
///
/// Delegates to [`findings::artifact_defect`], which is the **writer's own** completeness check: the
/// same list of required artifacts, the same refusal to follow a final symbolic link, and the same
/// requirement that the reproducer pair genuinely load. Sharing it is the point. This function
/// previously asked a narrower question — is the directory there, and is `commands.sh` in it — so a
/// finding that had lost its reproducer, its record, its manifest, its environment fingerprint, its
/// computed difference or its whole `outputs/` directory was still rendered as `present`. A row that
/// reads as a recorded observation with the observation missing is the one shape of report that is
/// actively misleading, and two implementations of "complete" is how that shape comes back.
fn finding_artifact_defect(directory: &Path) -> Option<String> {
    findings::artifact_defect(directory)
}

/// The state of a finding's artifact directory, checked rather than assumed, ready for a table cell.
///
/// A report that named a directory nobody could open would be worse than one that admitted the
/// artifacts are missing, because the reproduction commands are the whole point of a finding.
///
/// The question itself is answered in exactly one place, [`findings::artifact_defect`], so the cell a
/// reader sees, the diagnostic that makes the run notice, and the writer's check at the moment of
/// publication can never describe different states. The defect's own words are carried into the cell
/// rather than collapsed to a fixed phrase, because "missing `outputs`" and "missing
/// `reproducer.expected`" send a reader to different places.
///
/// # Why the whole rendered state goes through [`table_cell`]
///
/// Those words are not this module's. A defect sentence quotes what was found on disk — an entry name,
/// a manifest line, an operating-system error — and formatting it straight into a Markdown table cell
/// would let any vertical bar in it end the cell early and shift every value after it into the wrong
/// column, which is the table equivalent of forging a field; a bracket or an angle bracket could
/// restructure the document around it. The bound
/// [`table_cell`] applies matters as much: a defect that enumerated a large directory would otherwise
/// widen one row past anything a reader can scan.
///
/// The marker characters are prepended **after** escaping rather than included in it, so the cell keeps
/// the ✅/⚠️ a reader scans for while everything that came from outside this module is escaped.
fn finding_artifact_state(directory: &Path) -> String {
    match finding_artifact_defect(directory) {
        None => String::from("✅ present"),
        Some(defect) => format!("⚠️ {}", table_cell(&defect)),
    }
}

/// What is wrong with one row's finding artifacts, as a full sentence, or `None` when nothing is.
///
/// Answers for rows of every verdict, and answers `None` for all but [`Verdict::Finding`], so a
/// caller may hand it any row without first deciding which ones are eligible. A finding whose
/// divergence class is absent has no derivable directory and is reported by its own diagnostic in
/// [`AreaReport::from_outcomes`]; there is nothing on disk this check could ask about, so it is not
/// restated here.
fn finding_shortfall(row: &Row) -> Option<String> {
    if row.verdict != Verdict::Finding {
        return None;
    }
    let defect = row
        .finding_dir
        .as_deref()
        .and_then(finding_artifact_defect)?;
    Some(format!(
        "the finding for {} under {} is reported but {}; a finding is a deliverable — the \
         reproducer, the exact reproduction commands, the captured outputs, the environment \
         fingerprint and the computed difference are the whole point of recording one — so a row \
         naming artifacts that are not there is a defect in the suite rather than an observation \
         about the compiler",
        row.cell_label(),
        row.oracle,
        defect
    ))
}

/// Revalidate the artifacts of every finding these rows report, immediately before a publication.
///
/// # Why the same question is asked twice
///
/// [`AreaReport::from_outcomes`] already asks it while the rows are being assembled, and records the
/// answer as a diagnostic. That is not the same claim as this one. A report is an artifact a reader
/// opens later, and what it asserts is that the directories it names hold deliverables *as
/// published* — so the instant that matters is the one just before the bytes making that assertion
/// are written, not an earlier instant during assembly. Between the two, a concurrent run's cleanup,
/// a stray `rm`, or an interrupted write can empty a directory the report is about to advertise. The
/// window cannot be closed — the artifacts are separate files and the platform offers no way to
/// publish a report atomically with them — so it is *narrowed* to the smallest one available and the
/// result is turned into a failure rather than a note.
///
/// Returned as sentences rather than raised as an error, because the caller writes the report first
/// and fails afterwards. A report withheld on account of an incomplete finding would destroy the
/// evidence for the very failure being reported, which is the opposite of what a deliverable is for.
fn finding_artifact_shortfalls(rows: &[Row]) -> Vec<String> {
    rows.iter().filter_map(finding_shortfall).collect()
}

/// The command that reproduces a finding with no harness, no Cargo and no Rust toolchain.
///
/// Rendered through [`shown_path`] like every other path in this module, so the build root appears as
/// its token rather than as an absolute location. A reader substitutes their own build directory for
/// the token — which they know, because it is theirs — and the line then runs verbatim.
///
/// # Why this is elided rather than exact, having once been the other way round
///
/// An earlier form of this function was the module's one deliberate exception: it emitted the
/// absolute path on the reasoning that a command a reader pastes has to name the directory exactly,
/// and that a per-run report under the build directory is never committed. The second half of that is
/// true and the first half does not follow from it. A per-run report is **uploaded** — the continuous
/// integration job publishes the whole report tree as a build artifact, which is the point of writing
/// it — so this line travelled to wherever those artifacts are read, carrying the absolute location of
/// a package root that names the machine and the account that built it: a continuous-integration
/// workspace identifier, an agent clone directory, a maintainer's home. That is the disclosure
/// [`shown_path`] exists to prevent, and the sibling column of the very same table was already
/// eliding it, so the row disclosed through one cell what it withheld in another.
///
/// What is given up is one substitution by the reader, and nothing else: the directory beneath the
/// token, the script's name and every argument are unchanged. Exactness where it cannot be
/// substituted for is preserved elsewhere and deliberately — `commands.sh` itself carries the true,
/// unelided values for everything it executes, and `findings::curated_finding_defects` refuses to let
/// a **committed** artifact carry an absolute checkout path at all.
///
/// [`posix_quote`] still wraps the result, so the whole path is one shell word and no character in it
/// is special, and it is only ever rendered through [`md_code`], which is what makes it inert in the
/// document. Neither of those depends on which spelling of the path is used.
fn reproduce_command(directory: &Path) -> String {
    format!(
        "sh {}",
        posix_quote(&shown_path(&directory.join(COMMANDS_NAME)))
    )
}

/// The verdict tally as a table, one row per verdict **including the ones that scored zero**, with
/// the run-failing policy each verdict is under.
fn render_tally_table(tally: &Tally, caps: &Capabilities) -> Vec<String> {
    let mut rows: Vec<Vec<String>> = Verdict::ALL
        .iter()
        .copied()
        .map(|verdict| {
            vec![
                String::from(verdict.label()),
                tally.get(verdict).to_string(),
                String::from(yes_no(verdict_fails_run(verdict, caps.config()))),
            ]
        })
        .collect();
    rows.push(vec![
        String::from("**Total**"),
        format!("**{}**", tally.total()),
        String::from(ABSENT_CELL),
    ]);
    markdown_table(&["Verdict", "Count", "Fails this run"], &rows)
}

/// One outcome as a row of the per-cell verdict table.
fn outcome_table_row(row: &Row) -> Vec<String> {
    vec![
        table_cell(&row.program),
        String::from(row.target.short_name()),
        String::from(row.opt.flag()),
        format!("oracle_{}", row.oracle.letter()),
        String::from(row.verdict.label()),
        row.class
            .map(|class| String::from(class.label()))
            .unwrap_or_else(|| String::from(ABSENT_CELL)),
        optional_cell(row.marker_id.as_deref()),
        table_cell(&row.detail),
    ]
}

/// Column headers of the per-cell verdict table.
const OUTCOME_TABLE_HEADERS: &[&str] = &[
    "Program", "Target", "Level", "Oracle", "Verdict", "Class", "Marker", "Detail",
];

/// The note that keeps the truncation in the tables above honest.
fn truncation_note(machine_readable: &str) -> String {
    format!(
        "One row per cell per oracle. A trailing `{TRUNCATION_MARK}` in **Detail** means the text \
         was shortened for the table; `{machine_readable}` carries it in full."
    )
}

/// An expected divergence, rendered with everything the requirement asks a summary to carry: the
/// marker identifier, the class, the scope, the documented basis, the divergence as observed, and
/// the cells that were reclassified because of it.
fn render_marker_block(
    marker: &ExpectedDivergence,
    owner: Option<&ProgramFacts>,
    cells: &[&Row],
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("### {}", md_code(marker.id())));
    lines.push(String::new());
    let basis_path = marker.basis_absolute_path();
    let mut pairs = vec![
        (
            String::from("Divergence class"),
            String::from(marker.class().label()),
        ),
        (String::from("Scope"), table_cell(marker.scope().raw())),
        (
            String::from("Owning program"),
            match owner {
                Some(facts) => md_code(&facts.label()),
                None => md_code(&marker.program_label()),
            },
        ),
        (String::from("Documented basis"), table_cell(marker.basis())),
        (
            String::from("Cited document"),
            format!(
                "{} — {}",
                md_path(marker.basis_path()),
                if basis_path.is_file() {
                    "✅ present in this checkout"
                } else {
                    "⚠️ NOT found in this checkout"
                }
            ),
        ),
        (
            String::from("Cells reclassified as XFAIL"),
            cells.len().to_string(),
        ),
    ];
    if let Some(facts) = owner {
        pairs.push((
            String::from("Program under test"),
            table_cell(&facts.description),
        ));
    }
    lines.extend(property_table(&pairs));
    lines.push(String::new());
    lines.push(String::from("The divergence as observed:"));
    lines.push(String::new());
    lines.extend(quoted_block(marker.observed()));
    if !cells.is_empty() {
        lines.push(String::new());
        lines.push(String::from("Cells this marker reclassified:"));
        lines.push(String::new());
        for cell in cells {
            lines.push(format!(
                "- {} under `oracle_{}`",
                md_code(&cell.cell_label()),
                cell.oracle.letter()
            ));
        }
    }
    lines
}

/// Every finding as a table, each row naming its artifact directory, the review copy inside this
/// report, and its reproduction command.
///
/// The `Review copy` column is what makes the `Artifact directory` column usable to a reader who was
/// not present at the run. The directory holds the exact bytes and the runnable commands and is beneath
/// the build directory, so it is there for whoever is standing in front of the machine and gone for
/// everyone else; the review copy is the same seven artifact classes rendered to report grade, inside
/// the directory this report is published from and therefore inside whatever archive carried it here.
///
/// It is rendered from the identifier rather than looked up on disk, deliberately. Every area report is
/// required to be byte-identical between two runs over identical inputs, and a column reporting what a
/// run's budget happened to allow would break that for a reason that has nothing to do with the
/// comparison the row reports. Whether a copy was published, and what was refused, is therefore a
/// run-scope statement in the summary — where a reduced set of published evidence is a fact about the
/// run — and this column is the address the copy occupies when it exists.
fn render_finding_table(rows: &[&Row]) -> Vec<String> {
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let directory = row.finding_dir.clone();
            vec![
                match &row.finding_id {
                    Some(identifier) => md_code(identifier),
                    None => String::from("⚠️ not derivable (no divergence class)"),
                },
                md_code(&row.cell_label()),
                format!("oracle_{}", row.oracle.letter()),
                row.class
                    .map(|class| String::from(class.label()))
                    .unwrap_or_else(|| String::from(ABSENT_CELL)),
                match &directory {
                    Some(path) => md_path(path),
                    None => String::from(ABSENT_CELL),
                },
                match &directory {
                    Some(path) => finding_artifact_state(path),
                    None => String::from(ABSENT_CELL),
                },
                match &row.finding_id {
                    Some(identifier) => md_code(&finding_bundle_reference(identifier)),
                    None => String::from(ABSENT_CELL),
                },
                match &directory {
                    Some(path) => md_code(&reproduce_command(path)),
                    None => String::from(ABSENT_CELL),
                },
            ]
        })
        .collect();
    markdown_table(
        &[
            "Finding",
            "Cell",
            "Oracle",
            "Class",
            "Artifact directory",
            "Artifacts",
            "Review copy",
            "Reproduce",
        ],
        &table_rows,
    )
}

/// Every unavailable comparison as a table. The detail is the diagnosis the capability record
/// produced, which always names the tool and the environment variable that would supply it.
fn render_unavailable_table(rows: &[&Row]) -> Vec<String> {
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            vec![
                md_code(&row.cell_label()),
                format!("oracle_{}", row.oracle.letter()),
                table_cell(&row.detail),
            ]
        })
        .collect();
    markdown_table(
        &["Cell", "Oracle", "Missing tool and how to supply it"],
        &table_rows,
    )
}

/// Every recorded, reasoned narrowing as a table, so the set of comparisons deliberately not made
/// is as visible as the set that was.
fn render_exclusion_table(narrowings: &[(&ProgramFacts, Narrowing)]) -> Vec<String> {
    let rows: Vec<Vec<String>> = narrowings
        .iter()
        .map(|(facts, narrowing)| {
            vec![
                md_code(&facts.label()),
                String::from(narrowing.kind),
                table_cell(&narrowing.scope),
                match &narrowing.reason {
                    Some(reason) => table_cell(reason),
                    None => String::from(
                        "⚠️ no reason recorded — a narrowing without a recorded reason is a defect \
                         in the test",
                    ),
                },
            ]
        })
        .collect();
    markdown_table(
        &["Program", "Narrowing", "Applies to", "Recorded reason"],
        &rows,
    )
}

/// A bulleted list of diagnostics, or a line saying there were none.
fn render_diagnostics(diagnostics: &[String]) -> Vec<String> {
    if diagnostics.is_empty() {
        return vec![String::from("None.")];
    }
    diagnostics
        .iter()
        .map(|diagnostic| format!("- {}", md(diagnostic)))
        .collect()
}

/// A bulleted list of the reasons coverage is reduced or partial, or a line saying it is neither.
fn render_coverage_reasons(coverage: &Coverage) -> Vec<String> {
    if coverage.reasons.is_empty() {
        return vec![String::from(
            "None — the full matrix was swept and every oracle was available.",
        )];
    }
    coverage
        .reasons
        .iter()
        .map(|reason| format!("- {}", md(reason)))
        .collect()
}

/// The expected-divergence section, complete in both directions.
///
/// Every marker the corpus declares is listed, **including one that produced no expected
/// divergence at all**, because a marker that never fired is either a divergence that has gone —
/// which the unexpected-success section will have caught — or a marker whose scope no longer
/// reaches any cell, and both are facts a reader needs. Any marker identifier that appears on an
/// outcome but is declared nowhere in the records this report could read is listed too, with a
/// warning: an expected divergence with no traceable basis is precisely what the marker mechanism
/// exists to prevent.
///
/// # Every `XFail` cites a marker, so an `XFail` without one is reported as a defect
///
/// `XFail` is reached three ways and **all three carry a marker identifier**: a divergence a
/// marker covers; a comparison the program's own record narrows away, where the frozen contract
/// requires a marker naming the narrowed oracle beside the recorded reason; and an arm blocked by a
/// marked root refusal on another arm of the same cell, which cites the root marker. `classify.rs`
/// has no builder that produces an `XFail` with no identifier — a narrowing no marker names is
/// [`Verdict::Fail`] there, and the record parser refuses to load such a record in the first place.
///
/// Separating marker-less `XFail` rows into a "documented by the record's own recorded reason" group
/// and presenting them as legitimate expected divergences would be the reporting half of the same
/// defect: a reasoned exclusion is worth reporting, but it would then be *counted as an expected
/// divergence* while carrying no identifier a row could cite, no entry the bidirectional register
/// audit could find, and no basis resolved against any document.
///
/// So a marker-less `XFail` is reported for what it is: an outcome claiming the authority of a
/// documented limitation while naming none. It is listed with a warning naming the remedy, and it
/// is never presented as an expected divergence. Nothing about the *reporting* of reasoned
/// narrowings is lost — the recorded-exclusions table later in the summary lists every one of them
/// program by program, with the reason its record states — and now each also appears under the
/// marker that documents it.
fn render_expected_divergence_section(facts: &CorpusFacts, xfail: &[&Row]) -> Vec<String> {
    let mut grouped: BTreeMap<String, Vec<&Row>> = BTreeMap::new();
    let mut unattributed: Vec<&Row> = Vec::new();
    for row in xfail {
        match row.marker_id.clone() {
            Some(identifier) => grouped.entry(identifier).or_default().push(row),
            None => unattributed.push(row),
        }
    }

    let declared = facts.markers();
    let mut lines = Vec::new();
    if declared.is_empty() && grouped.is_empty() && unattributed.is_empty() {
        lines.push(String::from(
            "None — no expected-divergence marker applies here, so every divergence would be a \
             finding or a failure.",
        ));
        return lines;
    }
    if declared.is_empty() && grouped.is_empty() {
        lines.push(String::from(
            "**No expected-divergence marker applies here**, so a divergence of any class would be \
             a finding or a failure. What follows is not an expected divergence: it is every \
             outcome that claimed the authority of one while naming no marker.",
        ));
        lines.push(String::new());
    }

    for (owner, marker) in &declared {
        let cells = grouped.remove(marker.id()).unwrap_or_default();
        lines.extend(render_marker_block(marker, Some(owner), &cells));
        lines.push(String::new());
    }
    for (identifier, cells) in grouped {
        lines.push(format!("### {}", md_code(&identifier)));
        lines.push(String::new());
        lines.push(format!(
            "⚠️ {} comparison(s) were reclassified as expected divergences under this identifier, \
             but no expectation record read for this report declares it, so its documented basis \
             cannot be shown. An expected divergence whose basis cannot be traced is not an \
             expected divergence: check the marker against `{EXPECTED_DIVERGENCE_REGISTER}`.",
            cells.len()
        ));
        lines.push(String::new());
        for cell in cells {
            lines.push(format!(
                "- {} under `oracle_{}`: {}",
                md_code(&cell.cell_label()),
                cell.oracle.letter(),
                table_cell(&cell.detail)
            ));
        }
        lines.push(String::new());
    }
    if !unattributed.is_empty() {
        lines.push(String::from(
            "### ⚠️ Expected divergences claimed with no marker identifier — a defect, not a basis",
        ));
        lines.push(String::new());
        lines.push(format!(
            "{count} outcome(s) were recorded as `XFAIL` while naming no marker. **This is not a \
             reportable expected divergence and it is not presented as one.** All three `XFAIL` \
             forms cite a marker: a divergence a marker covers; a comparison a program's own record \
             narrows away, where the frozen contract requires a marker naming the narrowed oracle \
             beside the recorded reason; and an arm blocked by a marked root refusal on another arm \
             of the same cell, which cites the root marker. An outcome with no identifier therefore \
             claims the authority of a documented limitation while naming none — there is no row a \
             reader can follow, no entry for `{EXPECTED_DIVERGENCE_REGISTER}`'s bidirectional audit \
             to find, and no basis resolved against any document, which is exactly the silent \
             exclusion the marker mechanism exists to prevent. Neither the classifier nor the record \
             parser can produce this state, so reaching it means this report was written by an older \
             revision of the suite, or that a record reached classification without going through \
             the parser. Remedy: give each narrowing below a marker in the program's own record, \
             scoped to name the oracle it narrows, and mirror it in `{EXPECTED_DIVERGENCE_REGISTER}` \
             — the narrowing and its recorded reason stay exactly as they are. Every reasoned \
             narrowing is listed program by program in the recorded-exclusions table further down \
             whether or not it is marked, so nothing is hidden by refusing to count it here.",
            count = unattributed.len()
        ));
        lines.push(String::new());
        for cell in &unattributed {
            lines.push(format!(
                "- {} under `oracle_{}`: {}",
                md_code(&cell.cell_label()),
                cell.oracle.letter(),
                table_cell(&cell.detail)
            ));
        }
        lines.push(String::new());
    }
    while lines.last().map(String::is_empty).unwrap_or(false) {
        lines.pop();
    }
    lines
}

/// One feature area's human-readable report.
///
/// `session` is the signature of the run that produced it, recorded in the effective-matrix table
/// so that a reader who finds two area reports side by side can see at a glance whether they
/// describe the same run — the same question [`try_finalize`] answers mechanically from the
/// machine-readable sibling.
fn render_area_markdown(
    report: &AreaReport,
    facts: &CorpusFacts,
    caps: &Capabilities,
    coverage: &Coverage,
    dimensions: &[MatrixDimension],
    generation: &Generation,
) -> String {
    let spec = report.spec;
    let config = caps.config();
    let (targets, levels) = config.effective_matrix();

    let mut lines = Vec::new();
    lines.push(format!(
        "# Feature area — `{}`{}",
        spec.directory(),
        coverage.heading_suffix()
    ));
    lines.push(String::new());
    lines.push(coverage.statement());
    lines.push(String::new());
    lines.push(format!(
        "Written by the differential conformance harness. Machine-readable sibling: \
         `{AREAS_DIR_NAME}/{}.{TSV_EXTENSION}`. Run summary: `{SUMMARY_STEM}.{MARKDOWN_EXTENSION}`.",
        spec.directory()
    ));
    lines.push(String::new());
    lines.push(format!(
        "Generation: {}. The run summary aggregates only the area reports carrying its own \
         generation, so this artifact is never mistaken for one another run wrote.",
        generation.describe()
    ));
    lines.push(String::new());

    // Ahead of the verdicts, deliberately. Requirement 1 makes undefined-behaviour freedom the
    // precondition that makes both oracles sound, and requirement 3 does the same for flag parity, so
    // a reader has to learn whether those held before reading a single comparison as evidence.
    lines.extend(render_preflight_section(Some(spec.directory())));

    lines.push(String::from("## Area at a glance"));
    lines.push(String::new());
    lines.extend(property_table(&[
        (
            String::from("Feature-area directory"),
            format!("`{}`", spec.directory()),
        ),
        (
            String::from("Classification"),
            String::from(if spec.mandated() {
                "mandated — named explicitly by the coverage requirement"
            } else {
                "supplementary — added for its cross-backend divergence surface"
            }),
        ),
        (
            String::from("Comparisons recorded"),
            report.tally.total().to_string(),
        ),
        (
            String::from("Preflight gates governing this area that did not hold"),
            match blocking_gate_count(Some(spec.directory())) {
                0 => String::from("0 — every precondition this area rests on held"),
                blocking => format!(
                    "⚠️ {blocking} — no comparison in this area is evidence about a compiler; see \
                     the Preflight gates section above"
                ),
            },
        ),
        (
            String::from("Per-cell execution budget"),
            format!("{} s", config.timeout_secs()),
        ),
        (
            String::from("Unexpected success (XPASS)"),
            String::from(if config.xpass_fails_run() {
                "fails the run"
            } else {
                "⚠️ downgraded to a warning by the marker-retirement escape hatch"
            }),
        ),
        (
            String::from("Unavailable oracle"),
            String::from(if config.unavailable_fails_run() {
                "fails the run — the strict setting is in force"
            } else {
                "reported, and does not fail the run"
            }),
        ),
    ]));
    lines.push(String::new());

    lines.push(String::from(
        "## This area's matrix, planned against recorded",
    ));
    lines.push(String::new());
    lines.push(String::from(
        "The plan is what this area's own expectation records declare, intersected with the matrix \
         this run sweeps — so a program that narrows its targets for a recorded reason leaves no gap \
         here, while a program that should have been judged and was not leaves a visible one. Every \
         shortfall in this table is also a reason in the coverage section below: the stamp in the \
         first heading is derived from this table rather than assessed separately, so the two cannot \
         disagree.",
    ));
    lines.push(String::new());
    lines.extend(render_matrix_table(dimensions));
    lines.push(String::new());
    if facts.programs.is_empty() {
        lines.push(String::from(
            "⚠️ No expectation record of this area could be read, so the planned counts above are \
             not the corpus's own — see the Diagnostics section.",
        ));
        lines.push(String::new());
    }

    lines.push(String::from("## Effective matrix"));
    lines.push(String::new());
    lines.extend(property_table(&[
        (
            String::from("Targets this run sweeps"),
            join_targets(&targets),
        ),
        (
            String::from("Targets observed in this area"),
            join_targets(&report.observed_targets()),
        ),
        (
            String::from("Optimization levels this run sweeps"),
            join_opt_levels(&levels),
        ),
        (
            String::from("Optimization levels observed in this area"),
            join_opt_levels(&report.observed_opt_levels()),
        ),
        (
            String::from("Coverage"),
            String::from(if coverage.reduced {
                "⚠️ reduced"
            } else if coverage.partial {
                "⚠️ partial"
            } else {
                "full"
            }),
        ),
        (
            String::from("Run identity (session)"),
            format!("`{}`", generation.run),
        ),
    ]));
    lines.push(String::new());

    lines.push(String::from("## Verdict tally"));
    lines.push(String::new());
    lines.extend(render_tally_table(&report.tally, caps));
    lines.push(String::new());

    lines.push(String::from("## Outcomes"));
    lines.push(String::new());
    if report.rows.is_empty() {
        lines.push(String::from(
            "⚠️ No comparison was recorded for this area, so nothing in it has been judged.",
        ));
    } else {
        lines.push(truncation_note(&format!(
            "{}.{TSV_EXTENSION}",
            spec.directory()
        )));
        lines.push(String::new());
        let rows: Vec<Vec<String>> = report.rows.iter().map(outcome_table_row).collect();
        lines.extend(markdown_table(OUTCOME_TABLE_HEADERS, &rows));
    }
    lines.push(String::new());

    lines.push(String::from(
        "## Expected divergences (XFAIL) and their documented basis",
    ));
    lines.push(String::new());
    lines.extend(render_expected_divergence_section(
        facts,
        &report.rows_with(Verdict::XFail),
    ));
    lines.push(String::new());

    let xpass = report.rows_with(Verdict::XPass);
    lines.push(String::from("## ⚠️ Unexpected successes (XPASS)"));
    lines.push(String::new());
    if xpass.is_empty() {
        lines.push(String::from(
            "None — every marker in this area still describes a divergence that is really there.",
        ));
    } else {
        lines.push(String::from(
            "A marker claims a divergence that is no longer there. The marker is stale documented \
             knowledge and will mislead the next reader until it is retired, which is a test-only \
             edit.",
        ));
        lines.push(String::new());
        let rows: Vec<Vec<String>> = xpass.iter().map(|row| outcome_table_row(row)).collect();
        lines.extend(markdown_table(OUTCOME_TABLE_HEADERS, &rows));
    }
    lines.push(String::new());

    let failures = report.rows_with(Verdict::Fail);
    lines.push(String::from("## ⚠️ Failures (FAIL)"));
    lines.push(String::new());
    if failures.is_empty() {
        lines.push(String::from("None."));
    } else {
        let rows: Vec<Vec<String>> = failures.iter().map(|row| outcome_table_row(row)).collect();
        lines.extend(markdown_table(OUTCOME_TABLE_HEADERS, &rows));
    }
    lines.push(String::new());

    let findings = report.rows_with(Verdict::Finding);
    lines.push(String::from(
        "## Findings — verbatim reproducer, minimization status and reproduction commands",
    ));
    lines.push(String::new());
    if findings.is_empty() {
        lines.push(String::from(
            "None — no undocumented divergence was observed in this area.",
        ));
    } else {
        lines.push(format!(
            "A finding is a deliverable, not a defect to patch: no compiler source change is made \
             in response to one. Each directory holds a verbatim reproducer with its recorded \
             minimization status — a run performs no automated reduction — its expectation record, \
             the captured outputs, the environment fingerprint, the computed difference and \
             `{COMMANDS_NAME}` — the exact compile and run lines, which reproduce the divergence \
             with no harness at all. The curated set is indexed by `{FINDINGS_REGISTER}`.",
        ));
        lines.push(String::new());
        lines.extend(render_finding_table(&findings));
    }
    lines.push(String::new());

    let unavailable = report.rows_with(Verdict::Unavailable);
    lines.push(String::from("## ⚠️ Unavailable oracles"));
    lines.push(String::new());
    if unavailable.is_empty() {
        lines.push(String::from(
            "None — every oracle this area asked for was available.",
        ));
    } else {
        lines.push(String::from(
            "An oracle whose tooling is absent is reported, never silently passed. Under the strict \
             setting intended for continuous integration each of these fails the run, because there \
             the toolchain is installed deliberately.",
        ));
        lines.push(String::new());
        lines.extend(render_unavailable_table(&unavailable));
    }
    lines.push(String::new());

    let narrowings = facts.narrowings();
    lines.push(String::from(
        "## Recorded, reasoned exclusions — what was deliberately not compared",
    ));
    lines.push(String::new());
    if narrowings.is_empty() {
        lines.push(String::from(
            "None — every program in this area is compared on every target, at every optimization \
             level, under all three oracles, behind the default warning gate.",
        ));
    } else {
        lines.push(String::from(
            "A narrowing is permitted only with a recorded reason, and never by dropping a feature: \
             a program excluded from one oracle stays fully compared under the others.",
        ));
        lines.push(String::new());
        lines.extend(render_exclusion_table(&narrowings));
    }
    lines.push(String::new());

    lines.push(String::from("## Why this report is reduced or partial"));
    lines.push(String::new());
    lines.extend(render_coverage_reasons(coverage));
    lines.push(String::new());

    let mut diagnostics = report.diagnostics.clone();
    diagnostics.extend(facts.diagnostics.iter().cloned());
    lines.push(String::from("## Diagnostics"));
    lines.push(String::new());
    lines.extend(render_diagnostics(&diagnostics));
    lines.push(String::new());

    join_document(&lines)
}

/// One feature area's machine-readable report: the generation preamble, the fixed header, then one
/// row per outcome, each row stamped with this run's provenance.
///
/// The preamble comes first so that a reader — and [`read_area`] — can decide whether the file is
/// theirs before parsing a single row, and so that a file written before the stamp existed is
/// recognised on its first line. The per-row provenance answers the same question again at row
/// granularity, which is what lets a row that outlived the clearing contribute nothing rather than
/// contribute silently.
fn render_area_tsv(
    report: &AreaReport,
    generation: &Generation,
    identity: &RunIdentity,
    coverage: &Coverage,
) -> String {
    let mut lines = Vec::with_capacity(report.rows.len() + 2);
    lines.push(generation.preamble(coverage));
    lines.push(tsv_header(AREA_TSV_COLUMNS));
    for row in &report.rows {
        lines.push(row.to_tsv(identity));
    }
    join_document(&lines)
}

/// Join rendered lines into a file, collapsing a trailing run of blank lines and ending with
/// exactly one newline. Every artifact this module writes goes through it, so two artifacts never
/// differ merely in how many blank lines they happen to end with.
fn join_document(lines: &[String]) -> String {
    let mut end = lines.len();
    while end > 0 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    let mut document = lines[..end].join("\n");
    document.push('\n');
    document
}

// ---------------------------------------------------------------------------------------------
// The run summary
//
// This is the artifact the requirements ask for by name, so its four numbered sections are exactly
// the four things they ask it to report: the feature areas covered; the total tests and their
// outcomes; every expected divergence with its documented basis; and every finding with its
// reproducer, that reproducer's recorded minimization status, and its reproduction commands. The
// numbering is not decoration — it is what lets the artifact be checked against the requirement
// literally. The fourth section names the minimization status rather than claiming a minimized
// reproducer, because a run performs no automated reduction and the manifest says so.
// ---------------------------------------------------------------------------------------------

/// Which run this summary describes, and what it deliberately left out.
///
/// Placed before every count in the document, because a reader has to know what the numbers are
/// counting before the numbers mean anything. An area whose file belongs to another run is named
/// here with the generation it carries: excluded, not deleted, and not quietly absorbed.
fn render_provenance_section(run: &RunReport, generation: &Generation) -> Vec<String> {
    let absent: Vec<&'static AreaSpec> = AREAS
        .iter()
        .filter(|spec| {
            !run.areas
                .iter()
                .any(|area| area.spec.directory() == spec.directory())
                && !run.unusable.contains(spec)
                && !run
                    .stale
                    .iter()
                    .any(|stale| stale.spec.directory() == spec.directory())
        })
        .collect();

    let mut lines = vec![
        String::from("## Provenance — which run this summary describes"),
        String::new(),
        String::from(
            "This summary aggregates only the per-area reports carrying the generation below. A \
             report left behind by an earlier run is listed as stale and excluded from every total, \
             so a filtered or repeated run can never publish another run's results as its own.",
        ),
        String::new(),
    ];
    lines.extend(property_table(&[
        (
            String::from("Run identifier"),
            format!("`{}`", generation.run),
        ),
        (
            String::from("Configuration fingerprint"),
            format!("`{}`", generation.config),
        ),
        (
            String::from("Feature areas aggregated"),
            format!(
                "{} of {AREA_COUNT} — {}",
                run.areas.len(),
                met(run.areas.len() == AREA_COUNT)
            ),
        ),
        (
            String::from("Feature areas excluded as stale"),
            run.stale.len().to_string(),
        ),
        (
            String::from("Feature areas unusable"),
            run.unusable.len().to_string(),
        ),
        (
            String::from("Feature areas that filed no report"),
            absent.len().to_string(),
        ),
    ]));
    lines.push(String::new());

    if !run.stale.is_empty() {
        lines.push(String::from(
            "⚠️ Reports on disk that this run did not write, and therefore did not count:",
        ));
        lines.push(String::new());
        let rows: Vec<Vec<String>> = run
            .stale
            .iter()
            .map(|stale| {
                vec![
                    format!("`{}`", stale.spec.directory()),
                    match &stale.generation {
                        Some(generation) => format!("`{}`", generation.run),
                        None => String::from("⚠️ unstamped"),
                    },
                    table_cell(&stale.note),
                ]
            })
            .collect();
        lines.extend(markdown_table(
            &["Feature area", "Written by run", "Why it was excluded"],
            &rows,
        ));
        lines.push(String::new());
    }
    if !absent.is_empty() {
        lines.push(format!(
            "⚠️ Feature areas that filed no report in this run at all, so nothing in them has been \
             judged here: {}.",
            absent
                .iter()
                .map(|spec| format!("`{}`", spec.directory()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.push(String::new());
    }
    lines
}

/// The per-area table of the first deliverable section: all fourteen areas, named, counted and
/// classified, whether or not they contributed anything.
fn render_area_overview_table(run: &RunReport) -> Vec<String> {
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(AREA_COUNT);
    for (index, spec) in AREAS.iter().enumerate() {
        let report = run
            .areas
            .iter()
            .find(|candidate| candidate.spec.directory() == spec.directory());
        // The swept count rather than the row count, so this column and the run total below it mean the
        // same thing: a program whose record could not be read produced a row and swept nothing.
        let observed = report.map(|area| area.swept_programs(&run.facts));
        let comparisons = report.map(|area| area.tally.total());
        let verdicts = match report {
            Some(area) => area.tally.compact(),
            None if run.unusable.contains(&spec) => String::from("⚠️ its report could not be used"),
            None if run
                .stale
                .iter()
                .any(|stale| stale.spec.directory() == spec.directory()) =>
            {
                String::from("⚠️ its report belongs to another run and was excluded")
            }
            None => String::from("⚠️ no report contributed"),
        };
        rows.push(vec![
            (index + 1).to_string(),
            format!("`{}`", spec.directory()),
            String::from(if spec.mandated() {
                "mandated"
            } else {
                "supplementary"
            }),
            spec.program_count().to_string(),
            observed
                .map(|count| count.to_string())
                .unwrap_or_else(|| String::from(ABSENT_CELL)),
            comparisons
                .map(|count| count.to_string())
                .unwrap_or_else(|| String::from(ABSENT_CELL)),
            verdicts,
        ]);
    }
    rows.push(vec![
        String::from("**Total**"),
        format!("**{AREA_COUNT} areas**"),
        String::from(ABSENT_CELL),
        format!("**{PROGRAM_COUNT}**"),
        format!("**{}**", run.observed_programs()),
        format!("**{}**", run.tally.total()),
        run.tally.compact(),
    ]);
    markdown_table(
        &[
            "#",
            "Feature area",
            "Classification",
            "Programs planned",
            "Programs swept",
            "Comparisons",
            "Verdicts",
        ],
        &rows,
    )
}

/// The human-readable run summary — the suite's deliverable.
fn render_summary_markdown(
    run: &RunReport,
    caps: &Capabilities,
    coverage: &Coverage,
    dimensions: &[MatrixDimension],
    generation: &Generation,
) -> String {
    let config = caps.config();
    let failing = run.failing(caps);
    let mut lines = Vec::new();

    lines.push(format!(
        "# Differential conformance suite — run summary{}",
        coverage.heading_suffix()
    ));
    lines.push(String::new());
    lines.push(coverage.statement());
    lines.push(String::new());
    lines.push(String::from(
        "This artifact reports the feature areas covered, the total tests and their outcomes, every \
         expected divergence with its documented basis, and every finding with its reproducer, that \
         reproducer's recorded minimization status, and its reproduction commands. Each of those \
         four is a numbered section below. A finding's reproducer is a verbatim copy of the corpus \
         program. No automated reduction is performed during a run, so a \
         finding's manifest states its minimization status rather than the summary claiming a \
         reduced program.",
    ));
    lines.push(String::new());
    lines.push(String::from(
        "Every judgement here rests on stdout bytes and exit status alone. Standard error is \
         captured into finding artifacts and never compared, because diagnostic wording legitimately \
         differs between compilers and comparing it would report differences that say nothing about \
         code correctness.",
    ));
    lines.push(String::new());

    lines.extend(render_provenance_section(run, generation));

    // Before the verdict, because the verdict below depends on it: a run whose preconditions did not
    // hold cannot report "no outcome fails this run" as though the comparisons meant something.
    lines.extend(render_preflight_section(None));

    let blocking_gates = blocking_gate_count(None);

    lines.push(String::from("## Run verdict"));
    lines.push(String::new());
    lines.extend(property_table(&[
        (
            String::from("Comparisons recorded"),
            run.tally.total().to_string(),
        ),
        (
            String::from("PASS"),
            run.tally.get(Verdict::Pass).to_string(),
        ),
        (
            String::from("XFAIL — documented expected divergence"),
            run.tally.get(Verdict::XFail).to_string(),
        ),
        (
            String::from("FINDING — undocumented divergence, delivered as an artifact"),
            run.tally.get(Verdict::Finding).to_string(),
        ),
        (
            String::from("XPASS — a marker whose divergence has gone"),
            run.tally.get(Verdict::XPass).to_string(),
        ),
        (
            String::from("FAIL — unexplained"),
            run.tally.get(Verdict::Fail).to_string(),
        ),
        (
            String::from("UNAVAILABLE — an oracle's tooling is absent"),
            run.tally.get(Verdict::Unavailable).to_string(),
        ),
        (
            String::from("Outcomes that fail the run under the policy in force"),
            failing.to_string(),
        ),
        (
            String::from("Preflight gates that did not hold"),
            blocking_gates.to_string(),
        ),
        (
            String::from("Verdict"),
            match (failing, blocking_gates) {
                (0, 0) => String::from("✅ no outcome fails this run and every preflight gate held"),
                (0, gates) => format!(
                    "⚠️ no outcome fails this run, but {gates} preflight gate(s) did not hold, so \
                     no outcome above is evidence about a compiler"
                ),
                (failing, 0) => format!("⚠️ {failing} outcome(s) fail this run"),
                (failing, gates) => format!(
                    "⚠️ {failing} outcome(s) fail this run and {gates} preflight gate(s) did not hold"
                ),
            },
        ),
    ]));
    lines.push(String::new());

    let xpass = run.rows_with(Verdict::XPass);
    lines.push(String::from(
        "## ⚠️ Unexpected successes (XPASS) — stale expected-divergence markers",
    ));
    lines.push(String::new());
    if xpass.is_empty() {
        lines.push(String::from(
            "None — every marker in the corpus still describes a divergence that is really there.",
        ));
    } else {
        lines.push(format!(
            "{} comparison(s) agreed while a marker claimed they would diverge. Unexpected success \
             is listed here separately and prominently whether or not it is failing the run, \
             because a stale marker is stale documented knowledge that will mislead the next reader \
             until somebody retires it — and retiring one is a test-only edit. Under the policy in \
             force an unexpected success {} the run.",
            xpass.len(),
            if config.xpass_fails_run() {
                "**fails**"
            } else {
                "is **downgraded to a warning** by the marker-retirement escape hatch and does not \
                 fail"
            }
        ));
        lines.push(String::new());
        let rows: Vec<Vec<String>> = xpass.iter().map(|row| outcome_table_row(row)).collect();
        lines.extend(markdown_table(OUTCOME_TABLE_HEADERS, &rows));
    }
    lines.push(String::new());

    let unavailable = run.rows_with(Verdict::Unavailable);
    let arms = caps.unavailable_oracle_arms();
    lines.push(String::from("## ⚠️ Unavailable oracles"));
    lines.push(String::new());
    if unavailable.is_empty() && arms.is_empty() {
        lines.push(String::from(
            "None — every oracle this run asked for was available on this machine.",
        ));
    } else {
        lines.push(format!(
            "A missing oracle is never a silent pass. Each entry names the tool and the environment \
             variable that would supply it. Under the strict setting an unavailable oracle {} the \
             run, and strict is the intended continuous-integration setting because there the \
             toolchain is installed deliberately, so a missing oracle means a broken workflow rather \
             than a modest machine.",
            if config.unavailable_fails_run() {
                "**fails**"
            } else {
                "would fail"
            }
        ));
        if config.missing_oracles_acknowledged() {
            lines.push(String::new());
            lines.push(format!(
                "`{VAR_ALLOW_MISSING_ORACLES}` is set: the operator has acknowledged that this \
                 machine cannot attempt every arm. The acknowledgement annotates this report and \
                 changes no verdict."
            ));
        }
        if !arms.is_empty() {
            lines.push(String::new());
            lines.push(String::from(
                "Arms the capability record cannot attempt at all:",
            ));
            lines.push(String::new());
            for arm in &arms {
                lines.push(format!("- {}", md(arm)));
            }
        }
        if !unavailable.is_empty() {
            lines.push(String::new());
            lines.push(String::from("Comparisons reported unavailable:"));
            lines.push(String::new());
            lines.extend(render_unavailable_table(&unavailable));
        }
    }
    lines.push(String::new());

    lines.push(String::from("## 1 — Feature areas covered"));
    lines.push(String::new());
    lines.push(format!(
        "The corpus is organised by language feature area rather than by where defects are \
         suspected. Nine of the {AREA_COUNT} areas are named explicitly by the coverage \
         requirement; the remaining five are supplementary, chosen for the cross-backend \
         divergence surface they carry. A mandated area holds at least \
         {MIN_PROGRAMS_PER_MANDATED_AREA} programs."
    ));
    lines.push(String::new());
    lines.extend(render_area_overview_table(run));
    lines.push(String::new());

    lines.push(String::from("## 2 — Total tests and their outcomes"));
    lines.push(String::new());
    lines.push(String::from(
        "### The enumerable matrix, planned against recorded",
    ));
    lines.push(String::new());
    lines.push(String::from(
        "The planned column is the full declared matrix, never the reduced one: a run that narrowed \
         itself shows the narrowing here rather than moving the target it is measured against. Every \
         shortfall in this table is also a reason in the coverage section below, and the stamp in the \
         first heading is derived from this table — so a shortfall can never sit beside a claim of \
         full coverage.",
    ));
    lines.push(String::new());
    lines.extend(render_matrix_table(dimensions));
    lines.push(String::new());
    lines.push(String::from("### Verdict tally"));
    lines.push(String::new());
    lines.extend(render_tally_table(&run.tally, caps));
    lines.push(String::new());
    lines.push(String::from("### How coverage is reported here"));
    lines.push(String::new());
    lines.push(format!(
        "Coverage is the matrix above, never a percentage. Measuring a percentage would need \
         coverage instrumentation, which would need a development dependency, which this project \
         forbids absolutely — so no percentage here would be measurable, and publishing one would \
         be fabrication rather than evidence. The matrix is countable from the committed file set \
         and is re-reported on every run. The suite itself is built on the language's own test \
         harness and the standard library alone: no third-party crate, no coverage tool, no mocking \
         library, no fuzzer and no program generator is involved. Machine-readable form of \
         everything in this document: `{SUMMARY_STEM}.{TSV_EXTENSION}`."
    ));
    lines.push(String::new());

    lines.push(String::from(
        "## 3 — Expected divergences and their documented basis",
    ));
    lines.push(String::new());
    lines.push(format!(
        "A marker changes how a divergence is classified, never whether the feature is \
         exercised: the applicable phases are attempted in order — compile, link, run, compare \
         — and classification happens at the first terminal outcome or the completed \
         comparison, so nothing short-circuits a phase because a marker exists. Every marker \
         cites a limitation this repository already documents, and the set is cross-referenced \
         by `{EXPECTED_DIVERGENCE_REGISTER}`. `XFAIL` has three forms and every one of them cites \
         a marker: a divergence a marker covers; a comparison a program's own record declines to \
         make, which needs a marker naming the narrowed oracle beside the record's own recorded \
         reason; and an arm blocked by a marked root refusal on another arm of the same cell, \
         which cites the root marker and states that no comparison was attempted. Each cell's \
         detail says which form it is, because a marker asserting a divergence was OBSERVED, a \
         narrowing stating a comparison was deliberately NOT MADE, and an arm that lost its \
         subject to a refusal are three different statements, and reading any of them as another \
         would misstate what the run established."
    ));
    lines.push(String::new());
    lines.extend(render_expected_divergence_section(
        &run.facts,
        &run.rows_with(Verdict::XFail),
    ));
    lines.push(String::new());

    let findings = run.rows_with(Verdict::Finding);
    lines.push(String::from(
        "## 4 — Findings, with verbatim reproducer, minimization status and reproduction commands",
    ));
    lines.push(String::new());
    if findings.is_empty() {
        lines.push(String::from(
            "None — no undocumented divergence was observed in this run.",
        ));
    } else {
        lines.push(format!(
            "{} undocumented divergence(s) were observed. A finding is a deliverable, not a defect \
             to patch: no compiler source change is made in response to one. Each directory holds \
             a verbatim reproducer — a byte-for-byte copy of the corpus program, whose recorded \
             minimization status states that a run performs no automated reduction and what to do \
             next — its expectation record, a manifest, the captured stdout, exit status and \
             stderr per compiler and per backend, the environment fingerprint, the computed \
             difference, and `{COMMANDS_NAME}` — the exact compile and run lines, which reproduce \
             the divergence with no harness, no Cargo and no Rust toolchain. The curated set is \
             indexed by `{FINDINGS_REGISTER}`.",
            findings.len()
        ));
        lines.push(String::new());
        lines.extend(render_finding_table(&findings));
    }
    lines.push(String::new());

    let narrowings = run.facts.narrowings();
    lines.push(String::from(
        "## Recorded, reasoned exclusions — what was deliberately not compared",
    ));
    lines.push(String::new());
    if narrowings.is_empty() {
        lines.push(String::from(
            "None — every program is compared on every target, at every optimization level, under \
             all three oracles, behind the default warning gate.",
        ));
    } else {
        lines.push(String::from(
            "No feature is dropped because it is difficult. Where a comparison genuinely cannot be \
             made, the exclusion is narrow, scoped, and recorded with its reason in the program's \
             own expectation record — so the set of things not compared is as visible as the set \
             that is.",
        ));
        lines.push(String::new());
        lines.extend(render_exclusion_table(&narrowings));
    }
    lines.push(String::new());

    lines.push(String::from("## Environment fingerprint"));
    lines.push(String::new());
    lines.push(String::from(
        "Recorded so that a divergence can be attributed to toolchain drift rather than to the \
         compiler — the hazard the repository's own risk register raises about emulator version \
         skew. It carries no timestamp, so two fingerprints from one machine compare equal and any \
         difference between two of them is a real difference.",
    ));
    lines.push(String::new());
    // Rendered as an indented block, which Markdown treats as literal code: nothing inside one is
    // document syntax, so escaping for Markdown is the one transformation this sink does not need and
    // the banners read exactly as their tools printed them.
    //
    // The other two are still applied, and redaction is applied *here* rather than relied upon
    // upstream. A tool banner is redacted by the capability record as it is captured, but a
    // fingerprint line also carries a discovered tool path and a refusal reason, and neither of those
    // passes through that redaction — so a directory name taken from a credential-bearing variable
    // would otherwise reach the published summary. This is the same rule the three funnels apply; the
    // only difference is which of the three transformations a code block needs.
    for line in caps.render_fingerprint().lines() {
        lines.push(format!(
            "    {}",
            sanitize_text_for_report(&redact_secrets(line))
        ));
    }
    lines.push(String::new());

    lines.push(String::from("## Provenance"));
    lines.push(String::new());
    lines.push(String::from(
        "Every row aggregated into this summary was written by this run under this configuration,          and a row claiming anything else is refused and reported in the Diagnostics section rather          than counted. The run identifier distinguishes this process from any other; the          configuration digest covers the row schema, the effective matrix and policy, the discovered          tool set and the corpus that was read — all four of which are stated in full elsewhere in          this document, so a digest that differs can be diagnosed rather than merely noticed.",
    ));
    lines.push(String::new());
    let identity = RunIdentity::of(caps);
    lines.extend(property_table(&[
        (String::from("Run identifier"), md_code(&identity.run)),
        (
            String::from("Configuration digest"),
            md_code(&identity.identity),
        ),
        (
            String::from("Report directory ownership"),
            format!("claimed by this run — {}", md_code(RUN_OWNER_ENTRY)),
        ),
    ]));
    lines.push(String::new());

    lines.push(String::from("## Run configuration"));
    lines.push(String::new());
    lines.push(format!(
        "The fingerprint below is a digest of everything that could make two runs' numbers \
         incomparable — the effective matrix, the resolved identity of every tool, the per-cell \
         budget and the settings that decide which verdicts fail a run. Differ in any of those and \
         the digest differs, so a reduced run's totals can never be mistaken for a full run's. \
         Expect it to differ between two runs configured alike as well, and do not read that as a \
         change in configuration: under strict mode each emulator is attested by being made to \
         print a token derived from the run and exit with a status derived from it, so that a \
         stand-in ignoring its arguments cannot pass by luck, and the status it had to produce is \
         part of the tool identity this digest covers. The artifact to diff when asking whether a \
         result changed is an area report's Markdown, which renders none of this and is \
         byte-identical for identical inputs. The token identifying *this process's* run is not in \
         this file at all; it is recorded once, in `{}` beside this summary.",
        super::sandbox::RUN_MANIFEST_NAME
    ));
    lines.push(String::new());
    lines.extend(property_table(&[
        (
            String::from("Configuration fingerprint"),
            format!("`{}`", run_generation().configuration()),
        ),
        (
            format!("`{VAR_QUICK}` (reduced matrix)"),
            String::from(yes_no(config.quick_mode())),
        ),
        (
            format!("`{VAR_ONLY}` (single program)"),
            match config.only() {
                Some(filter) => md_code(&format!("{}/{}", filter.area(), filter.program())),
                None => String::from("unset"),
            },
        ),
        (
            format!("`{VAR_STRICT}` (unavailable oracle fails)"),
            String::from(yes_no(config.strict())),
        ),
        (
            format!("`{VAR_ALLOW_XPASS}` (unexpected success downgraded)"),
            String::from(yes_no(config.allow_xpass())),
        ),
        (
            format!("`{VAR_ALLOW_MISSING_ORACLES}` (reduced oracles acknowledged)"),
            String::from(yes_no(config.missing_oracles_acknowledged())),
        ),
        (
            format!("`{VAR_TIMEOUT_SECS}` (per-cell budget)"),
            format!("{} s", config.timeout_secs()),
        ),
        (
            format!("`{VAR_KEEP_WORK}` (cell workspaces retained)"),
            String::from(yes_no(config.keep_work())),
        ),
        (
            String::from("Test-name filters this process was started with"),
            if run.filters.is_empty() {
                String::from("none")
            } else {
                run.filters
                    .iter()
                    .map(|filter| md_code(filter))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ),
        (
            String::from("Feature areas this invocation could produce"),
            format!("{} of {AREA_COUNT}", run.expected),
        ),
        (
            String::from("Feature areas that contributed outcomes"),
            format!("{} of {AREA_COUNT}", run.areas.len()),
        ),
        // Derived from this run's configuration rather than from its process, so two runs of the
        // same sweep produce the same value and the artifact stays diffable. It is here because the
        // per-area files outlive the process that wrote them, and this is the value that says which
        // run they belong to.
        (
            String::from("Run identity (session)"),
            format!("`{}`", run.session),
        ),
    ]));
    lines.push(String::new());
    lines.push(String::from(
        "Every one of the area reports aggregated above was verified to carry that run identity. \
         The previous run's artifacts are removed once, when this process starts, and an area \
         report stamped with any other identity is refused rather than added in — so this summary \
         describes one configuration's run and can never be a blend of two.",
    ));
    lines.push(String::new());

    lines.push(String::from("## Retained evidence"));
    lines.push(String::new());
    lines.push(String::from(
        "A cell that did not pass keeps its workspace so the divergence can be investigated, and \
         that retention is bounded: a compiler under test that cannot build anything would otherwise \
         retain the whole matrix. The accounting is stated here rather than left to be discovered, \
         because a workspace that was pruned and a workspace that was never created look identical \
         on disk and mean opposite things.",
    ));
    lines.push(String::new());
    let (retained_bytes, retained_workspaces) = super::sandbox::retention_totals();
    lines.extend(property_table(&[
        (
            String::from("Workspaces retained"),
            retained_workspaces.to_string(),
        ),
        (
            String::from("Bytes retained"),
            format!(
                "{retained_bytes} of {} permitted for the run",
                super::sandbox::RETAINED_RUN_BYTES_MAX
            ),
        ),
        (
            String::from("Per-workspace ceiling"),
            super::sandbox::RETAINED_WORKSPACE_BYTES_MAX.to_string(),
        ),
        (
            String::from("Per-file ceiling"),
            super::sandbox::RETAINED_ENTRY_BYTES_MAX.to_string(),
        ),
        (
            String::from("Workspace count ceiling"),
            super::sandbox::RETAINED_WORKSPACE_COUNT_MAX.to_string(),
        ),
    ]));
    lines.push(String::new());
    // The findings budget is stated beside the retention budget because they bound the same disk from
    // two directions — and it is deliberately a *different* policy, which is worth a reader knowing
    // before they go looking for a directory that is not there. A retained workspace is optional
    // evidence, so exceeding its ceiling prunes and reports the pruning. A finding's artifacts are the
    // deliverable, so exceeding one of these ceilings refuses the write and fails the cell instead:
    // nothing here is ever silently shortened, and there is no pruning note to look for.
    let (finding_bytes, finding_directories) = super::findings::artifact_totals();
    lines.extend(property_table(&[
        (
            String::from("Finding directories published"),
            format!(
                "{finding_directories} of {} permitted for the run",
                super::findings::FINDING_RUN_COUNT_MAX
            ),
        ),
        (
            String::from("Finding artifact bytes"),
            format!(
                "{finding_bytes} of {} permitted for the run",
                super::findings::FINDING_RUN_BYTES_MAX
            ),
        ),
        (
            String::from("Per-finding ceiling"),
            super::findings::FINDING_DIRECTORY_BYTES_MAX.to_string(),
        ),
        (
            String::from("Per-artifact ceiling"),
            super::findings::FINDING_ARTIFACT_BYTES_MAX.to_string(),
        ),
    ]));
    lines.push(String::new());
    lines.push(String::from(
        "One divergence is filed once. A cell whose build was refused is refused for every oracle \
         watching it, so the directory is named from the cell and the divergence class rather than \
         from the oracle, its manifest names every oracle that observed it, and every affected row \
         above points at that one directory instead of at a near-identical sibling per oracle.",
    ));
    lines.push(String::new());
    let pruning = super::sandbox::retention_pruning_notes();
    if pruning.is_empty() {
        lines.push(String::from(
            "Nothing was pruned: every workspace this run retained fitted inside the budget, so \
             every retained directory holds the whole of what its cell produced.",
        ));
    } else {
        // The two ceilings prune for different reasons and leave different things behind, so this
        // paragraph must not describe only the byte case. Saying that what was dropped can be
        // rebuilt from the command lines retained beside it holds when a byte ceiling bites, but
        // not when the workspace count ceiling does: that one prunes every entry, the command
        // lines included. The recovery instruction true in both cases is the workspace path, which
        // names the cell precisely enough to re-run exactly it.
        lines.push(format!(
            "{} pruning(s) were performed to stay inside the budget. Each names what was removed \
             and why. When a byte ceiling is what bites, the largest entries go first, which keeps \
             the small text captures — the statuses, the command lines and the recorded streams an \
             investigation actually reads — and drops the linked executables ahead of them. When \
             the workspace count ceiling is what bites, every entry of the workspace is pruned and \
             the notes below are the only surviving record that the cell retained anything. In \
             either case the directory named in the note identifies the cell, so re-running that \
             one cell reproduces what was dropped.",
            pruning.len()
        ));
        lines.push(String::new());
        for note in &pruning {
            // Through `md` rather than sanitization alone: a note names a pruned path, so it is
            // untrusted text in Markdown inline position exactly like every other, and this sink is
            // no more entitled to skip the funnel than any other is.
            lines.push(format!("- {}", md(note)));
        }
    }
    lines.push(String::new());
    // Evidence published for the cells whose workspaces were NOT kept. Stated in the same section as
    // the retention accounting because the two together are the whole answer to "what of this run
    // survives it": a retained workspace holds a failing cell's evidence, and a document beneath
    // `evidence/` holds the evidence of a cell that was reported without failing — an expected
    // divergence, or a permissive run's absent oracle — whose workspace was removed. Without the
    // second, the report row for such an outcome named a directory that no longer existed, and in
    // continuous integration, where the report root is uploaded and the workspace root is not, its
    // evidence reached nobody at all.
    let (evidence_documents, evidence_bytes) = evidence_totals();
    lines.extend(property_table(&[
        (
            String::from("Evidence documents published"),
            evidence_documents.to_string(),
        ),
        (
            String::from("Evidence bytes"),
            format!("{evidence_bytes} of {EVIDENCE_RUN_BYTES_MAX} permitted for the run"),
        ),
        (
            String::from("Per-document ceiling"),
            EVIDENCE_DOCUMENT_BYTES_MAX.to_string(),
        ),
        (
            String::from("Where they are"),
            format!("`{EVIDENCE_DIR_NAME}/` beneath this report"),
        ),
    ]));
    lines.push(String::new());
    // The third and last part of the same answer, and the one a passing run needs most. A finding does
    // not fail the run, so its cell workspace is kept and it never publishes an evidence document — and
    // the directory holding its seven artifact classes is beneath the build directory, which is not what
    // an archive of this report carries. Each finding therefore also has a report-grade copy inside this
    // report, and the accounting below is what says so without a reader going to look.
    let (bundle_findings, bundle_files, bundle_bytes) = finding_bundle_totals();
    lines.extend(property_table(&[
        (
            String::from("Findings with a review copy in this report"),
            bundle_findings.to_string(),
        ),
        (
            String::from("Review-copy artifact files published"),
            bundle_files.to_string(),
        ),
        (
            String::from("Review-copy bytes"),
            format!("{bundle_bytes} of {BUNDLE_RUN_BYTES_MAX} permitted for the run"),
        ),
        (
            String::from("Per-artifact ceiling"),
            BUNDLE_ARTIFACT_BYTES_MAX.to_string(),
        ),
        (
            String::from("Where they are"),
            format!("`{FINDING_BUNDLE_DIR_NAME}/<finding-id>/` beneath this report"),
        ),
    ]));
    lines.push(String::new());
    lines.push(format!(
        "A review copy is redacted, sanitized and bounded, and a capture that is not text is described \
         rather than transcribed, so it is what a reader judges a finding *from* and not the evidence \
         itself. The exact bytes and the runnable commands stay in the generated directory each row of \
         section 4 names, which is git-ignored and — in continuous integration — released only behind \
         the explicit raw-findings opt-in, because `{FINDINGS_REGISTER}` §5.3 requires a person to read \
         all of it before any of it is published. Every copy states that distinction in its own \
         `{BUNDLE_INDEX_NAME}`, together with what reached it and what did not."
    ));
    lines.push(String::new());
    let refusals = evidence_refusal_notes();
    if refusals.is_empty() {
        lines.push(String::from(
            "Every outcome that needed a durable evidence document received one: no archive was \
             refused, so nothing this run reported is missing the record of how it was reached.",
        ));
    } else {
        lines.push(format!(
            "{} evidence document(s) could NOT be published, so the outcomes they belong to are \
             reported without the captures behind them. Each refusal names the cell, which is \
             enough to re-run exactly it.",
            refusals.len()
        ));
        lines.push(String::new());
        for note in &refusals {
            lines.push(format!("- {}", md(note)));
        }
    }
    lines.push(String::new());

    lines.push(String::from("## Why this report is reduced or partial"));
    lines.push(String::new());
    lines.extend(render_coverage_reasons(coverage));
    lines.push(String::new());

    let mut diagnostics = run.diagnostics.clone();
    diagnostics.extend(run.facts.diagnostics.iter().cloned());
    for area in &run.areas {
        diagnostics.extend(area.diagnostics.iter().cloned());
    }
    lines.push(String::from("## Diagnostics"));
    lines.push(String::new());
    lines.extend(render_diagnostics(&diagnostics));
    lines.push(String::new());

    lines.push(String::from("## Per-area reports"));
    lines.push(String::new());
    let area_rows: Vec<Vec<String>> = AREAS
        .iter()
        .map(|spec| {
            vec![
                format!("`{}`", spec.directory()),
                format!(
                    "`{AREAS_DIR_NAME}/{}.{MARKDOWN_EXTENSION}`",
                    spec.directory()
                ),
                format!("`{AREAS_DIR_NAME}/{}.{TSV_EXTENSION}`", spec.directory()),
            ]
        })
        .collect();
    lines.extend(markdown_table(
        &["Feature area", "Human-readable", "Machine-readable"],
        &area_rows,
    ));
    lines.push(String::new());

    lines.push(String::from("## Reproducing one cell by hand"));
    lines.push(String::new());
    lines.push(String::from(
        "Every cell is reproducible from two files and nothing else: the program and the sibling \
         expectation record beside it. The record states the target list, the optimization levels, \
         the shared flags, the command templates for the compiler under test, for the reference \
         compiler and for execution, the expected exit status and the expected stdout verbatim. \
         Render the templates and run them; no harness, no Cargo and no Rust toolchain is involved. \
         For a finding, its directory already contains those lines, rendered, in its commands \
         script.",
    ));
    lines.push(String::new());

    join_document(&lines)
}

/// One row of the machine-readable summary, addressed by column name.
///
/// Every row of that file has the same arity, so most rows leave most columns empty. Filling them
/// by name rather than by position means a column the schema later gains cannot silently shift a
/// value into the wrong place, and a mistyped column name is a panic in `set` rather than a value
/// that quietly vanishes — but `set` is only ever called with one of the `COL_*` constants, so the
/// panic is unreachable in practice and no caller has to handle it.
struct SummaryRow {
    kind: &'static str,
    fields: Vec<(&'static str, String)>,
}

impl SummaryRow {
    /// Begin a row of the given record kind.
    fn new(kind: &'static str) -> Self {
        Self {
            kind,
            fields: Vec::new(),
        }
    }

    /// Set one column. The column must be one of [`SUMMARY_TSV_COLUMNS`]; a value that is empty
    /// after sanitization is dropped, so an absent value and an empty one render alike.
    fn set(mut self, column: &'static str, value: impl Into<String>) -> Self {
        debug_assert!(
            SUMMARY_TSV_COLUMNS.contains(&column),
            "summary column {column} is not part of the schema"
        );
        let value = value.into();
        if !value.is_empty() {
            self.fields.push((column, value));
        }
        self
    }

    /// Set one column from an optional value, dropping the column when there is nothing to say.
    fn set_opt(self, column: &'static str, value: Option<impl Into<String>>) -> Self {
        match value {
            Some(value) => self.set(column, value),
            None => self,
        }
    }

    /// Render the row: exactly one field per schema column, in schema order, tab-separated.
    fn render(&self) -> String {
        let cells: Vec<String> = SUMMARY_TSV_COLUMNS
            .iter()
            .map(|column| {
                if *column == COL_RECORD {
                    return String::from(self.kind);
                }
                self.fields
                    .iter()
                    .find(|(name, _)| name == column)
                    .map(|(_, value)| value.clone())
                    .unwrap_or_default()
            })
            .collect();
        tsv_row(&cells)
    }
}

/// The machine-readable run summary — the same content as the Markdown, shaped for aggregation.
///
/// Free text is carried in full here: the Markdown truncates long cells so its tables stay
/// readable, this file never does, so nothing observed is lost from the artifact pair.
fn render_summary_tsv(
    run: &RunReport,
    caps: &Capabilities,
    coverage: &Coverage,
    dimensions: &[MatrixDimension],
    generation: &Generation,
) -> String {
    let config = caps.config();
    let failing = run.failing(caps);
    let mut rows: Vec<SummaryRow> = Vec::new();

    // Meta — the fields an aggregator reads first, including the two the reporting discipline
    // requires to be explicit rather than inferred from the prose, and the provenance that says
    // which run and which configuration every aggregated row belongs to.
    let identity = RunIdentity::of(caps);
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "run")
            .set(COL_DETAIL, identity.run.clone()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "identity")
            .set(COL_DETAIL, identity.identity.clone()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "run_id")
            .set(COL_DETAIL, generation.run.clone()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "config_fingerprint")
            .set(COL_DETAIL, generation.config.clone()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "reduced")
            .set(COL_DETAIL, true_false(coverage.reduced)),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "partial")
            .set(COL_DETAIL, true_false(coverage.is_partial())),
    );
    // The suite defines fourteen areas; how many of them *this* invocation could produce is a
    // different number whenever a filter is active, and an aggregator that compared
    // `areas_contributed` against the constant would read every filtered run as missing data rather
    // than as deliberately scoped. Both numbers are published so neither has to be inferred.
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_defined")
            .set(COL_COUNT, AREA_COUNT.to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_expected")
            .set(COL_COUNT, run.expected.to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_contributed")
            .set(COL_COUNT, run.areas.len().to_string()),
    );
    // Published so an aggregator can tell two runs apart and so a per-area file can be matched to
    // the summary that consumed it. Derived from the configuration this run swept rather than from
    // the process that swept it, so the field is stable across two runs of the same sweep.
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "run_identity")
            .set(COL_DETAIL, run.session.clone()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_unusable")
            .set(COL_COUNT, run.unusable.len().to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_stale")
            .set(COL_COUNT, run.stale.len().to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "comparisons_recorded")
            .set(COL_COUNT, run.tally.total().to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "outcomes_failing_run")
            .set(COL_COUNT, failing.to_string()),
    );
    // The run's own verdict field accounts for the preconditions as well as the outcomes, because an
    // aggregator that read `run_fails=false` while a gate was unmet would publish a green result for a
    // matrix whose comparisons are not evidence. The two contributions are also published separately —
    // `outcomes_failing_run` and `preflight_gates_blocking` — so a reader can tell which one it was.
    let blocking_gates = blocking_gate_count(None);
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "preflight_gates_blocking")
            .set(COL_COUNT, blocking_gates.to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "preflight_held")
            .set(COL_DETAIL, true_false(blocking_gates == 0)),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "run_fails")
            .set(COL_DETAIL, true_false(failing > 0 || blocking_gates > 0)),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "xpass_fails_run")
            .set(COL_DETAIL, true_false(config.xpass_fails_run())),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "unavailable_fails_run")
            .set(COL_DETAIL, true_false(config.unavailable_fails_run())),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "quick_mode")
            .set(COL_DETAIL, true_false(config.quick_mode())),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "program_filter")
            .set_opt(
                COL_DETAIL,
                config
                    .only()
                    .map(|filter| format!("{}/{}", filter.area(), filter.program())),
            ),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "per_cell_timeout_secs")
            .set(COL_COUNT, config.timeout_secs().to_string()),
    );
    for filter in &run.filters {
        rows.push(
            SummaryRow::new(RECORD_META)
                .set(COL_LABEL, "test_name_filter")
                .set(COL_DETAIL, filter.clone()),
        );
    }
    // The configuration identity every aggregated area file was verified to carry. An aggregator
    // comparing two summaries reads this first: two rows of the same shape mean different things
    // under different matrices, filters or verdict policies, and this is what says which applied.
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, SESSION_LABEL)
            .set(COL_DETAIL, run.session.clone()),
    );
    // Stated rather than computed, so an aggregator never has to guess why no percentage is here.
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "coverage_metric")
            .set(COL_DETAIL, "enumerable matrix; no percentage is measurable"),
    );
    // Derived from configuration and the resolved tool identities, so it states what this run was
    // comparable with. It is not constant across runs — an attested emulator's required exit status
    // is part of a tool identity — which is why the stability promise belongs to the area Markdown
    // rather than to this summary. The per-process run token is a separate matter and is not here at
    // all: it lives in the run manifest alone.
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "configuration_fingerprint")
            .set(COL_DETAIL, run_generation().configuration()),
    );

    // The retention accounting, in the same numbers the Markdown sibling reports.
    let (retained_bytes, retained_workspaces) = super::sandbox::retention_totals();
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "retained_workspaces")
            .set(COL_COUNT, retained_workspaces.to_string())
            .set(
                COL_REFERENCE,
                super::sandbox::RETAINED_WORKSPACE_COUNT_MAX.to_string(),
            ),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "retained_bytes")
            .set(COL_COUNT, retained_bytes.to_string())
            .set(
                COL_REFERENCE,
                super::sandbox::RETAINED_RUN_BYTES_MAX.to_string(),
            ),
    );
    for note in super::sandbox::retention_pruning_notes() {
        rows.push(
            SummaryRow::new(RECORD_META)
                .set(COL_LABEL, "retention_pruning")
                .set(COL_DETAIL, note),
        );
    }

    // The other half of "what survives this run": documents published for the cells whose workspaces
    // were not kept. Emitted beside the retention totals so an aggregator reading only this file can
    // account for every reported outcome's evidence, not only the failing ones'.
    let (evidence_documents, evidence_bytes) = evidence_totals();
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "evidence_documents")
            .set(COL_COUNT, evidence_documents.to_string())
            .set(COL_REFERENCE, EVIDENCE_DIR_NAME),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "evidence_bytes")
            .set(COL_COUNT, evidence_bytes.to_string())
            .set(COL_REFERENCE, EVIDENCE_RUN_BYTES_MAX.to_string()),
    );

    // The findings' own half of the same account. Emitted as `meta` rows beside the evidence rows rather
    // than as new columns, deliberately: the column header of an area report is the contract the summary
    // aggregates through, so a run-scope total belongs in a row an aggregator can read or ignore, never
    // in a column every per-outcome row would then have to carry and leave empty.
    let (bundle_findings, bundle_files, bundle_bytes) = finding_bundle_totals();
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "finding_review_copies")
            .set(COL_COUNT, bundle_findings.to_string())
            .set(COL_REFERENCE, FINDING_BUNDLE_DIR_NAME),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "finding_review_copy_files")
            .set(COL_COUNT, bundle_files.to_string())
            .set(COL_REFERENCE, FINDING_BUNDLE_DIR_NAME),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "finding_review_copy_bytes")
            .set(COL_COUNT, bundle_bytes.to_string())
            .set(COL_REFERENCE, BUNDLE_RUN_BYTES_MAX.to_string()),
    );
    for note in evidence_refusal_notes() {
        rows.push(
            SummaryRow::new(RECORD_META)
                .set(COL_LABEL, "evidence_refused")
                .set(COL_DETAIL, note),
        );
    }

    for reason in &coverage.reasons {
        rows.push(SummaryRow::new(RECORD_COVERAGE_REASON).set(COL_DETAIL, reason.clone()));
    }

    // One row per gate, so an aggregator reading only this half learns which precondition failed and
    // what was observed, rather than only that the count above was not zero. A run that recorded no
    // preflight emits one row saying exactly that: silence here would be indistinguishable from an
    // aggregator that did not look.
    match recorded_preflight() {
        Some(preflight) => {
            for gate in preflight.gates() {
                rows.push(
                    SummaryRow::new(RECORD_PREFLIGHT)
                        .set(COL_LABEL, gate.name())
                        .set(COL_REFERENCE, gate.requirement())
                        .set(COL_VERDICT, gate.verdict().label())
                        .set(COL_DETAIL, gate.detail()),
                );
            }
        }
        None => rows.push(
            SummaryRow::new(RECORD_PREFLIGHT)
                .set(COL_LABEL, "not_recorded")
                .set(COL_VERDICT, GateVerdict::Unperformed.label())
                .set(
                    COL_DETAIL,
                    "the driver recorded no preflight, so neither flag parity nor \
                     undefined-behaviour freedom has been established for this run",
                ),
        ),
    }

    // The same dimension list the Markdown table and the coverage stamp were derived from, so the
    // three cannot disagree about what fell short.
    for dimension in dimensions {
        rows.push(
            SummaryRow::new(RECORD_MATRIX)
                .set(COL_LABEL, dimension.label)
                .set(COL_COUNT, dimension.actual.to_string())
                .set(COL_REFERENCE, dimension.planned.to_string())
                .set(COL_DETAIL, dimension.machine_status()),
        );
    }
    rows.push(
        SummaryRow::new(RECORD_MATRIX)
            .set(COL_LABEL, "reference_native_cells_planned")
            .set(COL_REFERENCE, REFERENCE_NATIVE_CELL_COUNT.to_string())
            .set(COL_DETAIL, "oracle (a) native arm"),
    );
    rows.push(
        SummaryRow::new(RECORD_MATRIX)
            .set(COL_LABEL, "reference_cross_cells_planned_max")
            .set(COL_REFERENCE, REFERENCE_CROSS_CELL_COUNT_MAX.to_string())
            .set(
                COL_DETAIL,
                "oracle (a) cross arms, when the drivers are present",
            ),
    );

    // Every area, contributing or not — a missing area must be visible as a row, not as a gap.
    for spec in AREAS.iter() {
        let report = run
            .areas
            .iter()
            .find(|candidate| candidate.spec.directory() == spec.directory());
        let stale = run
            .stale
            .iter()
            .find(|candidate| candidate.spec.directory() == spec.directory());
        let state = match (report, stale) {
            (Some(_), _) => "contributed",
            (None, Some(_)) => "stale",
            (None, None) if run.unusable.contains(&spec) => "unusable",
            (None, None) => "absent",
        };
        rows.push(
            SummaryRow::new(RECORD_AREA)
                .set(COL_AREA, spec.directory())
                .set(
                    COL_LABEL,
                    if spec.mandated() {
                        "mandated"
                    } else {
                        "supplementary"
                    },
                )
                .set(
                    COL_COUNT,
                    report
                        .map(|area| area.tally.total())
                        .unwrap_or(0)
                        .to_string(),
                )
                .set(COL_REFERENCE, spec.program_count().to_string())
                .set(
                    COL_DETAIL,
                    format!(
                        "{state}; programs observed {}{}",
                        report.map(|area| area.programs.len()).unwrap_or(0),
                        match stale {
                            Some(stale) => format!(
                                "; written by run {}; excluded because {}",
                                match &stale.generation {
                                    Some(generation) => generation.run.clone(),
                                    None => String::from("(unstamped)"),
                                },
                                stale.note
                            ),
                            None => String::new(),
                        }
                    ),
                ),
        );
    }

    // Tallies: one per area, then the run total with the area column empty. Every verdict is
    // emitted for every scope, including the zero rows, so a consumer never has to know which
    // verdicts happened to occur in order to read the file.
    for area in &run.areas {
        for verdict in Verdict::ALL {
            rows.push(
                SummaryRow::new(RECORD_TALLY)
                    .set(COL_AREA, area.spec.directory())
                    .set(COL_VERDICT, verdict.label())
                    .set(COL_COUNT, area.tally.get(verdict).to_string()),
            );
        }
    }
    for verdict in Verdict::ALL {
        rows.push(
            SummaryRow::new(RECORD_TALLY)
                .set(COL_VERDICT, verdict.label())
                .set(COL_COUNT, run.tally.get(verdict).to_string()),
        );
    }

    // Outcomes, carrying the provenance block through unchanged from the area row it was read from.
    // Copied rather than recomputed on purpose: the summary is an aggregation, and a second
    // derivation here could disagree with the area report it claims to summarize.
    for area in &run.areas {
        for row in &area.rows {
            rows.push(
                row.provenance
                    .apply_to(
                        SummaryRow::new(RECORD_OUTCOME)
                            .set(COL_AREA, row.area.clone())
                            .set(COL_PROGRAM, row.program.clone())
                            .set(COL_TARGET, row.target.triple())
                            .set(COL_OPT, row.opt.flag())
                            .set(COL_ORACLE, format!("oracle_{}", row.oracle.letter()))
                            .set(COL_VERDICT, row.verdict.label())
                            .set_opt(COL_CLASS, row.class.map(|class| class.label()))
                            .set_opt(COL_MARKER_ID, row.marker_id.clone())
                            .set_opt(COL_LABEL, row.finding_id.clone())
                            .set_opt(COL_REFERENCE, row.finding_dir.as_deref().map(shown_path)),
                    )
                    .set(COL_DETAIL, row.detail.clone()),
            );
        }
    }

    // Every declared marker, with the count of comparisons it explained in this run — including
    // the markers that explained nothing, since a marker that never fires is the stale-marker
    // hazard and must be visible here too.
    let xfail = run.rows_with(Verdict::XFail);
    let declared = run.facts.markers();
    for (owner, marker) in &declared {
        let observed = xfail
            .iter()
            .filter(|row| row.marker_id.as_deref() == Some(marker.id()))
            .count();
        rows.push(
            SummaryRow::new(RECORD_EXPECTED_DIVERGENCE)
                .set(COL_AREA, owner.area)
                .set(COL_PROGRAM, owner.program.clone())
                .set(COL_CLASS, marker.class().label())
                .set(COL_MARKER_ID, marker.id())
                .set(COL_COUNT, observed.to_string())
                .set(COL_LABEL, marker.scope().raw())
                .set(COL_REFERENCE, shown_path(marker.basis_path()))
                .set(COL_DETAIL, marker.basis()),
        );
    }
    // Any identifier that explained an outcome but is declared in no record this report could
    // read. An expected divergence whose basis cannot be traced is not an expected divergence, so
    // it is emitted as its own row rather than folded into the ones above.
    let mut undeclared: BTreeSet<String> = BTreeSet::new();
    for row in &xfail {
        let Some(identifier) = row.marker_id.as_deref() else {
            continue;
        };
        if !declared.iter().any(|(_, marker)| marker.id() == identifier) {
            undeclared.insert(String::from(identifier));
        }
    }
    for identifier in undeclared {
        rows.push(
            SummaryRow::new(RECORD_EXPECTED_DIVERGENCE)
                .set(COL_MARKER_ID, identifier)
                .set(
                    COL_DETAIL,
                    format!(
                        "no expectation record read for this report declares this identifier, so \
                         its documented basis cannot be shown; check it against \
                         {EXPECTED_DIVERGENCE_REGISTER}"
                    ),
                ),
        );
    }

    for row in run.rows_with(Verdict::Finding) {
        rows.push(
            SummaryRow::new(RECORD_FINDING)
                .set(COL_AREA, row.area.clone())
                .set(COL_PROGRAM, row.program.clone())
                .set(COL_TARGET, row.target.triple())
                .set(COL_OPT, row.opt.flag())
                .set(COL_ORACLE, format!("oracle_{}", row.oracle.letter()))
                .set_opt(COL_CLASS, row.class.map(|class| class.label()))
                .set_opt(COL_LABEL, row.finding_id.clone())
                .set_opt(COL_REFERENCE, row.finding_dir.as_deref().map(shown_path))
                .set(
                    COL_DETAIL,
                    match &row.finding_dir {
                        Some(directory) => {
                            format!("{}/{COMMANDS_NAME}: {}", shown_path(directory), row.detail)
                        }
                        None => row.detail.clone(),
                    },
                ),
        );
    }

    for row in run.rows_with(Verdict::Unavailable) {
        rows.push(
            SummaryRow::new(RECORD_UNAVAILABLE)
                .set(COL_AREA, row.area.clone())
                .set(COL_PROGRAM, row.program.clone())
                .set(COL_TARGET, row.target.triple())
                .set(COL_OPT, row.opt.flag())
                .set(COL_ORACLE, format!("oracle_{}", row.oracle.letter()))
                .set(COL_DETAIL, row.detail.clone()),
        );
    }
    for arm in caps.unavailable_oracle_arms() {
        rows.push(
            SummaryRow::new(RECORD_UNAVAILABLE)
                .set(COL_LABEL, "capability")
                .set(COL_DETAIL, arm),
        );
    }

    for (owner, narrowing) in run.facts.narrowings() {
        rows.push(
            SummaryRow::new(RECORD_EXCLUSION)
                .set(COL_AREA, owner.area)
                .set(COL_PROGRAM, owner.program.clone())
                .set(COL_LABEL, narrowing.kind)
                .set(COL_REFERENCE, narrowing.scope.clone())
                .set(
                    COL_DETAIL,
                    narrowing.reason.clone().unwrap_or_else(|| {
                        String::from(
                            "no reason recorded — a narrowing without a recorded reason is a defect \
                             in the test",
                        )
                    }),
                ),
        );
    }

    let mut diagnostics = run.diagnostics.clone();
    diagnostics.extend(run.facts.diagnostics.iter().cloned());
    for area in &run.areas {
        diagnostics.extend(area.diagnostics.iter().cloned());
    }
    for diagnostic in diagnostics {
        rows.push(SummaryRow::new(RECORD_DIAGNOSTIC).set(COL_DETAIL, diagnostic));
    }

    for line in caps.render_fingerprint().lines() {
        let (label, detail) = match line.split_once(':') {
            Some((label, detail)) => (label.trim(), detail.trim()),
            None => (line.trim(), ""),
        };
        rows.push(
            SummaryRow::new(RECORD_FINGERPRINT)
                .set(COL_LABEL, label)
                .set(COL_DETAIL, detail),
        );
    }

    let mut lines = Vec::with_capacity(rows.len() + 2);
    // The same generation preamble the per-area files carry, and for the same reason it is FIRST
    // there. The `reduced` and `partial` facts are also published below as `meta` records, and that
    // was previously the only place they appeared — which meant a machine consumer had to parse and
    // scan an unbounded number of records before it could learn whether the totals it was about to
    // aggregate described a full run. A reader that stops after one line now knows.
    lines.push(generation.preamble(coverage));
    lines.push(tsv_header(SUMMARY_TSV_COLUMNS));
    lines.extend(rows.iter().map(SummaryRow::render));
    join_document(&lines)
}

// ---------------------------------------------------------------------------------------------
// The run's area registry
//
// A report directory is a fixed set of names — `areas/<area>.tsv`, one per feature area — and those
// names carry no run identity of their own. Existence on disk therefore cannot answer the question
// the summary actually depends on, which is not "does a report for this area exist?" but "did *this*
// run produce one?". Two different files answer those two questions, and treating the first as the
// second is how a summary comes to describe a blend of two runs: a previous run's reduced,
// filtered or differently configured area file is byte-for-byte a perfectly valid report, and
// aggregating it produces totals that look complete and belong to nothing.
//
// Two mechanisms close that gap, and both are needed:
//
// - This module's own namespace preparation retires the previous run's `areas/` directory once per
//   process, before this run publishes anything: [`prepare_report_namespace`], reached through
//   [`ensure_report_namespace`] from [`write_area`] and [`try_finalize`]. It is a precondition of
//   both rather than an assumption about call order, so an area report can never be published beside
//   a stale neighbour. [`super::sandbox::ensure_roots`] only *creates* the three artifact roots and
//   publishes the run manifest; it clears nothing, because it runs before any ownership question has
//   been asked.
// - The registry below records which areas *this process* published. It is the set the summary
//   aggregates from, so a file this run did not write cannot enter a total even if something else
//   put one there — the completeness check consults memory, not the directory listing.
//
// The registry also carries the finalization claim, which fixes the second half of the same problem.
// Every one of the fourteen area tests calls [`try_finalize`]; without a claim, each of them rescans
// and re-renders, and two finishing together both write the summary. One critical section performs
// the completeness check and the claim together, so exactly one caller proceeds and the other
// thirteen return having touched no file at all. The claim is released again if finalization fails,
// because a claim that outlives a failed attempt would leave the run with no summary and no way to
// produce one.
//
// The lock guards a set of static names and a boolean, neither of which has an invariant a panic
// could break, so a poisoned lock is recovered rather than reported: a failing area test panics by
// design in this suite, and losing the summary because a *different* area failed would suppress
// exactly the report that explains the failure.
// ---------------------------------------------------------------------------------------------

/// What this process published, and whether it has already written the summary.
#[derive(Debug, Default)]
struct RunRegistry {
    /// Directory names of the areas whose report pair this process published, canonical spellings.
    published: BTreeSet<&'static str>,
    /// Whether the one finalization of this run has been claimed.
    finalized: bool,
    /// Finding-artifact shortfalls observed immediately before a publication, keyed by the scope
    /// that observed them: a feature area's directory name, or [`SUMMARY_SCOPE`] for the run
    /// summary.
    ///
    /// Kept here rather than returned from [`write_area`] because the caller that must fail is the
    /// area test, and the caller that must fail on the summary's account is whichever area happened
    /// to finalize — two different callers reached through the one function each of them already
    /// calls. Recording the answer where both can read it is what lets the report be published
    /// first and the run be failed afterwards, which is this module's standing discipline.
    shortfalls: BTreeMap<String, Vec<String>>,
    /// Why the last finalization attempt that reached the artifacts declined to write the summary.
    ///
    /// Only [`finalize`] can fill this in, and only from what it actually read, because it is the one
    /// caller that reads the area files at all. It exists so that [`finalization_pending`] can explain
    /// a disk-derived refusal — a published area's file that has since been removed, or one that
    /// belongs to another run — **without** re-reading fourteen files to rediscover a conclusion that
    /// has already been reached. Empty in the ordinary case, where the reason is simply that an area
    /// this invocation selected has not published yet, which the registry already knows.
    deferral: Option<String>,
}

/// This process's registry, created on first use.
fn run_registry() -> &'static Mutex<RunRegistry> {
    static REGISTRY: OnceLock<Mutex<RunRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(RunRegistry::default()))
}

/// Borrow the registry, recovering rather than propagating a lock poisoned by a panicking test.
fn registry() -> std::sync::MutexGuard<'static, RunRegistry> {
    run_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Record that this run published `spec`'s report pair.
///
/// Called only after a successful publication, so membership implies both files are on disk.
fn register_published_area(spec: &'static AreaSpec) {
    registry().published.insert(spec.directory());
}

/// Record the finding-artifact shortfalls one scope observed, replacing any earlier answer for it.
///
/// Replacement rather than accumulation, so that a scope re-entered — a summary re-finalized after a
/// released claim — reports what its own last revalidation found rather than the union of every
/// attempt. An empty list is recorded as an empty list, which is what makes "this scope was checked
/// and was sound" distinguishable from "this scope was never checked".
fn record_artifact_shortfalls(scope: &str, shortfalls: Vec<String>) {
    registry()
        .shortfalls
        .insert(String::from(scope), shortfalls);
}

/// The finding-artifact shortfalls recorded for one scope by the publication that checked it.
///
/// `scope` is a feature area's directory name, or [`SUMMARY_SCOPE`] for the run summary. An empty
/// result means either that the scope was checked and every finding it reports is complete, or that
/// the scope has not published yet — the caller asks after the publication it performed, so the two
/// are never confused at a call site.
///
/// The driver asserts this is empty, which is what turns an incomplete deliverable into a failed run
/// rather than a warning inside an artifact nobody is obliged to read.
pub fn artifact_shortfalls(scope: &str) -> Vec<String> {
    registry()
        .shortfalls
        .get(scope)
        .cloned()
        .unwrap_or_default()
}

/// Claim the single finalization of this run, if every feature area has filed its report.
///
/// Returns the areas to aggregate, in canonical [`AREAS`] order rather than the order they
/// finished, so the summary's bytes do not depend on how the harness scheduled its threads.
/// Returns `None` when an area that is going to run is still outstanding, when nothing has been
/// published at all, or when finalization is already claimed; all three are ordinary answers, and
/// none writes anything.
///
/// Completeness is measured against the areas this invocation *can* publish rather than against
/// all [`AREA_COUNT`](super::AREA_COUNT) of them. Waiting for the whole table would withhold the
/// deliverable indefinitely from a filtered run, because the areas the filter excluded are never
/// going to publish; waiting for the selected ones is what stops the summary being written after
/// the first of several selected areas finishes, which would publish a subset of a subset. The
/// summary such a run writes is stamped partial and lists every area that did not contribute —
/// which is the promise [`try_finalize`] documents. What it never does is fill those gaps from
/// another run's files.
fn claim_finalization() -> Option<Vec<&'static AreaSpec>> {
    // Read before the lock is taken: scanning the command line has nothing to do with the registry,
    // and holding the guard across it would widen the critical section for no reason.
    let expected = libtest_filter_selection().expected_areas();
    let mut held = registry();
    let complete = expected
        .iter()
        .all(|spec| held.published.contains(&spec.directory()));
    if held.finalized || held.published.is_empty() || !complete {
        return None;
    }
    held.finalized = true;
    Some(
        AREAS
            .iter()
            .filter(|spec| held.published.contains(&spec.directory()))
            .collect(),
    )
}

/// Give the finalization claim back after an attempt that failed to publish a summary.
fn release_finalization_claim() {
    registry().finalized = false;
}

/// Record why the finalizer declined, from what it read, or clear a reason that no longer applies.
///
/// Called only by [`finalize`], which is the only function in this module that reads the area files.
/// Overwrites rather than accumulates: the current state of the report directory is what a caller
/// asking "why is there no summary?" needs, not the history of every attempt.
fn record_finalization_deferral(reason: Option<String>) {
    registry().deferral = reason;
}

/// The areas this invocation selected that have not yet published, and any recorded refusal.
///
/// Answered entirely from memory. This is what makes the pending explanation free: the completeness
/// question was already settled inside [`claim_finalization`]'s critical section, so re-deriving it
/// from the filesystem would re-read every area file to learn something the registry already knows —
/// thirteen times in a fourteen-area run, once for each caller that is not the finalizer.
fn finalization_state() -> (Vec<&'static str>, Option<String>) {
    // Read before the lock is taken, exactly as [`claim_finalization`] does and for the same reason:
    // scanning the command line has nothing to do with the registry, and holding the guard across it
    // would widen the critical section for no reason.
    let expected = libtest_filter_selection().expected_areas();
    let held = registry();
    let awaited = expected
        .iter()
        .map(|spec| spec.directory())
        .filter(|directory| !held.published.contains(directory))
        .collect();
    (awaited, held.deferral.clone())
}

// ---------------------------------------------------------------------------------------------
// The public surface
//
// Two functions. One area test calls both, in this order, at the end of its run — including when it
// is about to fail, because a report written only on success would omit exactly the divergences the
// deliverable exists to describe.
// ---------------------------------------------------------------------------------------------

/// Write one feature area's pair of reports.
///
/// `area` is the area's directory name, as it appears in the corpus and in [`AREAS`]; an unknown
/// name is an error rather than a new directory, because a report filed under a name the corpus
/// does not have would never be aggregated and its outcomes would vanish silently.
///
/// This writes `areas/<area>.md` and `areas/<area>.tsv` beneath the report root, and nothing else
/// anywhere. Both files are rendered from one snapshot and published together by rename over
/// temporary siblings created exclusively, so a concurrently finalizing summary never reads a
/// half-written file, a pre-planted temporary cannot be written through, and a reader never finds a
/// Markdown report beside a tab-separated sibling from a different run. Only this area's own two
/// files are touched, which is why no lock between threads is needed; ownership *between runs* is
/// established once by the first call, which clears an earlier run's artifacts and refuses to
/// proceed while another run still owns the directory.
///
/// The run's roots are initialized first. That is deliberately a precondition of the write rather
/// than an assumption about who ran earlier: initialization is what retires the previous run's area
/// reports, and an area report published before it happened would be filed beside stale neighbours
/// and then retired along with them, so this run's own account of itself would disappear.
///
/// Every row written carries this run's provenance, and the file opens with this run's generation,
/// which is what lets [`try_finalize`] tell a report this run produced from one that was already
/// lying there.
///
/// Call it once per area test, with every outcome that area accumulated — passes included. The
/// passes are what make the tally, the matrix and the planned-against-recorded comparison mean
/// anything; a report of the failures alone could not show that the rest of the matrix ran.
///
/// `notes` carries statements the *caller* knows and this module cannot derive from the outcomes it
/// is handed. There is exactly one such statement today and it is the reason the parameter exists: an
/// area whose preflight gate did not hold never runs a cell, so it arrives here with no outcome at
/// all, and "no outcome" alone is indistinguishable from a matrix that produced nothing for some
/// other reason. The caller states which gate withheld the area and how many programs it withheld,
/// and it lands in the report's Diagnostics section beside everything else a reader has to know
/// before reading the tally.
///
/// # Errors
///
/// Fails when `area` is not a known feature area, when the run's roots cannot be prepared, or when
/// the report pair cannot be published. A failure here is reported rather than swallowed: a verdict
/// that was computed and then silently not recorded is worse than a loud I/O error, because the run
/// would look clean.
pub fn write_area(
    area: &str,
    outcomes: &[Outcome],
    caps: &Capabilities,
    notes: &[String],
) -> HarnessResult<()> {
    super::sandbox::ensure_roots()?;
    let spec = AreaSpec::lookup(area).ok_or_else(|| {
        HarnessError::new(
            format!("writing the report of feature area `{area}`"),
            format!(
                "`{area}` is not one of the {AREA_COUNT} feature areas this suite defines: {}",
                AREAS
                    .iter()
                    .map(AreaSpec::directory)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })?;

    // Before anything is written: establish this run's ownership of the report directory and clear
    // the previous run's artifacts, exactly once per process. Doing it here rather than trusting the
    // caller is what makes the guarantee structural — no area file can reach the report root ahead of
    // the clearing that would have removed a stale one, on any thread, however the tests are filtered
    // or ordered. It is the *ownership-aware* preparation and the only clearing path there is: a
    // second route that scanned and removed without first refusing a live foreign owner and without
    // verifying every level on the way down would be able to delete a concurrent run's artifacts, and
    // to do it through a redirected directory.
    ensure_report_namespace(&format!("writing the report of feature area `{area}`"))?;

    let mut report = AreaReport::from_outcomes(spec, outcomes, caps.config());
    let facts = CorpusFacts::for_area(spec);
    // The caller's statements first, because they explain the shape of everything below them: a
    // withheld area's empty tally reads as an absence until the note says what withheld it.
    report
        .diagnostics
        .extend(notes.iter().map(|note| sanitize_text_for_report(note)));
    report
        .diagnostics
        .extend(diagnose_unrecorded_programs(&report.rows, &facts));
    let dimensions = area_matrix(&report, &facts, caps);
    let coverage = assess_area_coverage(caps, &report, &facts, &dimensions);
    let generation = Generation::current(caps);

    // The last thing done before the bytes that advertise these findings are written, and recorded
    // where the area test can read it. Every finding this report names is revalidated against disk —
    // the whole artifact set, and the captures inside it, without following a link — so that the
    // interval between "checked" and "published" is as short as this platform allows. The answer is
    // recorded rather than raised: the report is published either way, and the area test then fails
    // on it. Withholding the report would delete the evidence for the failure.
    let shortfalls = finding_artifact_shortfalls(&report.rows);
    record_artifact_shortfalls(spec.directory(), shortfalls.clone());
    for shortfall in &shortfalls {
        // Also stated inside the artifact, in the same words, so a reader who opens the report sees
        // exactly what made the run fail rather than having to correlate it with a panic message.
        // `from_outcomes` already recorded the state it observed during assembly; this line records
        // the state at publication, which is the one the report asserts.
        let restated = format!("⚠️ at publication: {shortfall}");
        if !report.diagnostics.contains(&restated) {
            report.diagnostics.push(restated);
        }
    }

    write_report_pair(
        &format!("writing the report of feature area `{area}`"),
        &area_markdown_path(spec),
        &render_area_markdown(&report, &facts, caps, &coverage, &dimensions, &generation),
        &area_tsv_path(spec),
        &render_area_tsv(&report, &generation, RunIdentity::of(caps), &coverage),
    )?;

    // Registration follows publication, never precedes it, so a registered area is always an area
    // whose two files are on disk. That ordering is what lets the summary treat a complete registry
    // as a complete set of readable artifacts.
    register_published_area(spec);
    Ok(())
}

/// Diagnose comparisons whose program has no readable expectation record.
///
/// A program can only be reported honestly if its own record could be read: the record is where the
/// documented basis of an expected divergence lives, where a narrowed comparison states its reason,
/// and where the golden stdout comes from. A comparison whose program is absent from the records is
/// therefore called out rather than quietly folded into a total, because everything this report
/// would otherwise say about that program silently would be nothing at all.
///
/// An area whose records could not be read at all already carries its own diagnostic, and every one
/// of its programs would trip this check, so such an area is skipped here — one accurate diagnostic
/// beats ten derived from it.
fn diagnose_unrecorded_programs(rows: &[Row], facts: &CorpusFacts) -> Vec<String> {
    let readable: BTreeSet<&str> = facts.programs.iter().map(|program| program.area).collect();
    let mut missing: BTreeSet<(&str, &str)> = BTreeSet::new();
    for row in rows {
        if !readable.contains(row.area.as_str()) {
            continue;
        }
        if facts.program(&row.area, &row.program).is_none() {
            missing.insert((row.area.as_str(), row.program.as_str()));
        }
    }
    missing
        .into_iter()
        .map(|(area, program)| {
            format!(
                "⚠️ comparisons were recorded for `{area}/{program}`, but no expectation record of \
                 that name was found in the corpus, so this report cannot state that program's \
                 documented basis, its recorded exclusions or its golden stdout."
            )
        })
        .collect()
}

/// What one area's machine-readable report turned out to be when this run read it.
///
/// Absence is the ordinary answer for an area that has not filed a report in this process — a
/// filtered run leaves most of the fourteen absent — and is deliberately not a diagnostic. A
/// readable report that belongs to another run, or one written before generations were stamped, is
/// [`Stale`] and contributes nothing to any total while still being listed by name. A file that
/// exists but cannot be used at all is [`Unusable`], which is counted and reported rather than
/// quietly skipped, because an area silently missing from a summary is indistinguishable from an
/// area that passed.
///
/// [`Stale`]: AreaState::Stale
/// [`Unusable`]: AreaState::Unusable
enum AreaState {
    /// No file: the area has not filed a report in this process yet.
    Absent,
    /// Written by this run, under this configuration.
    Current(AreaReport),
    /// A readable report from a different run, or one written before generations were stamped.
    Stale(StaleArea),
    /// The file exists but nothing could be made of it.
    Unusable(AreaReport),
}

/// Upper bound on the number of lines a per-area machine-readable report may carry.
///
/// Derived rather than chosen: the largest legitimate area file holds one row per program, per
/// target, per optimization level and per oracle, plus its generation preamble and its header. Using
/// the whole suite's program count makes the bound generous by a factor of about ten for any single
/// area while still being a bound, so a file that has been replaced by something enormous is refused
/// instead of parsed line by line into memory.
const MAX_AREA_TSV_LINES: usize =
    PROGRAM_COUNT * Target::ALL.len() * OptLevel::ALL.len() * Oracle::ALL.len() + 2;

/// Read one area's machine-readable report back, refusing anything it should not be.
///
/// Reading the published artifact back, rather than aggregating the in-memory report a moment
/// earlier, is what makes the summary provably an aggregate of the files a reader can open. The
/// machine-readable sibling exists precisely so totals can be derived without re-reading prose, and
/// deriving them from anything else would let the summary and the per-area files disagree.
///
/// Absence returns [`AreaState::Absent`] and is never a diagnostic: the area has not run in this
/// process, or the file vanished between the existence check and the read, which is a benign race
/// and never a reason to fail a run. Everything else the file can do to us comes back either as a
/// stale-file record or as a diagnostic on an otherwise usable report, so a damaged, redirected,
/// oversized or foreign file degrades the summary **loudly** instead of aborting it or contributing
/// silently:
///
/// - a **symbolic link** at the file, or anywhere in the directory chain above it, is refused rather
///   than followed, because following one would read rows from anywhere on the machine while the
///   report still named this path;
/// - a file **larger than [`MAX_INSPECTED_FILE_BYTES`]**, or with more than [`MAX_AREA_TSV_LINES`]
///   lines, is refused rather than held in memory;
/// - bytes that are **not text** are refused rather than lossily reinterpreted;
/// - a first line that is not this run's generation preamble makes the file stale, which is the one
///   case that is neither counted nor reported as damage;
/// - a header from a different schema, or a row that will not parse, is reported;
/// - a row whose **provenance is not this run's** is refused and counted in a single diagnostic per
///   distinct claim, so a whole foreign file produces one readable sentence rather than one per row.
fn read_area(spec: &'static AreaSpec, current: &Generation, identity: &RunIdentity) -> AreaState {
    let path = area_tsv_path(spec);
    let context = format!(
        "reading the machine-readable report of feature area `{}`",
        spec.directory()
    );
    // For the cases whose message already reads as a whole sentence.
    let unusable = |problem: String| {
        AreaState::Unusable(AreaReport::from_rows(spec, Vec::new(), vec![problem]))
    };
    // For the guarded reads, which report a cause in terms of the path they refused.
    let damaged = |problem: String| {
        AreaState::Unusable(AreaReport::from_rows(
            spec,
            Vec::new(),
            vec![format!(
                "⚠️ the machine-readable report of feature area `{}` could not be used, so its \
                 outcomes are missing from every total: {problem}",
                spec.directory()
            )],
        ))
    };

    // Absence is the ordinary answer for an area that has not run, and is the one case that is not a
    // diagnostic. It is asked without following a final link, so a link is reported as present here
    // and refused by the guarded read below rather than silently resolved.
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return AreaState::Absent,
        Err(error) => {
            return damaged(format!(
                "{} could not be inspected: {error}",
                shown_path(&path)
            ));
        }
    }
    if let Some(parent) = path.parent() {
        if let Err(error) = require_directory_chain_below(&context, &report_root(), parent) {
            return damaged(String::from(error.cause()));
        }
    }
    let bytes = match read_file_bounded(&context, &path, MAX_INSPECTED_FILE_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => return damaged(String::from(error.cause())),
    };
    let contents = match String::from_utf8(bytes) {
        Ok(contents) => contents,
        Err(error) => {
            return damaged(format!(
                "{} is not valid text ({error}); a report this harness wrote is always text, so \
                 these bytes were written by something else",
                shown_path(&path)
            ));
        }
    };
    if contents.lines().count() > MAX_AREA_TSV_LINES {
        return damaged(format!(
            "{} carries more than the {MAX_AREA_TSV_LINES} lines the largest legitimate area report \
             can have, so it is refused rather than parsed",
            shown_path(&path)
        ));
    }

    let mut lines = contents.lines();
    let Some(preamble) = lines.next() else {
        return unusable(format!(
            "⚠️ the machine-readable report of feature area `{}` is empty, so its outcomes are \
             missing from every total.",
            spec.directory()
        ));
    };
    let Some(generation) = Generation::parse(preamble) else {
        return AreaState::Stale(StaleArea {
            spec,
            generation: None,
            note: format!(
                "its first line is not a generation preamble, so it cannot be shown to belong to \
                 this run. A report written before the harness stamped its reports looks exactly \
                 like this; re-run the area to replace it. Found `{}`.",
                sanitize_text_for_report(preamble)
            ),
        });
    };
    if &generation != current {
        // The configuration is named only when it differs, because the sweep digest alone already
        // settles that the file is foreign and repeating an identical fingerprint would crowd out the
        // one fact a reader needs.
        //
        // Neither token is quoted, in any branch. A token is unique to a process, so printing one
        // here would put a value that changes on every run into the summary's stale-area list and
        // cost this module its determinism rule for a string no reader can act on. What a reader
        // needs is which of the three fields disagreed, and that is said in words.
        let note = if generation.run != current.run && generation.config == current.config {
            format!(
                "it names sweep `{}` though its configuration matches this run's, so it was \
                 written by a concurrent run; this run is `{}`.",
                generation.run, current.run
            )
        } else if generation.run != current.run || generation.config != current.config {
            format!(
                "it was written by run `{}` under a different configuration, `{}`.",
                generation.run, generation.config
            )
        } else {
            String::from(
                "it names this run's sweep and this run's configuration but a different process, so \
                 it is an artifact of an earlier run over the same corpus under the same settings — \
                 one that survived the clearing this run performs before it writes anything, or a \
                 concurrent run of an identical configuration. Its rows are excluded rather than \
                 counted: a total that included them would describe a matrix executed by a process \
                 whose result nothing here can vouch for.",
            )
        };
        return AreaState::Stale(StaleArea {
            spec,
            note,
            generation: Some(generation),
        });
    }

    let expected = tsv_header(AREA_TSV_COLUMNS);
    match lines.next() {
        Some(header) if header == expected => {}
        Some(header) => {
            return unusable(format!(
                "⚠️ the machine-readable report of feature area `{}` has a header this harness \
                     does not recognise, so its outcomes are missing from every total. Expected \
                     `{expected}`, found `{}`. A report left behind by an older revision of the \
                     harness will do this; re-run the area to replace it.",
                spec.directory(),
                sanitize_text_for_report(header)
            ));
        }
        None => {
            return unusable(format!(
                "⚠️ the machine-readable report of feature area `{}` is empty, so its outcomes \
                     are missing from every total.",
                spec.directory()
            ));
        }
    }

    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    let mut foreign: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        // The preamble and the header took the first two lines, so a body line's number in the file
        // is its offset plus three.
        match Row::parse(line, offset + 3) {
            Ok((row, provenance, defect)) => {
                if identity.accepts(&provenance.run, &provenance.identity) {
                    // Reported only for a row this run owns. A refused foreign row is already
                    // accounted for by name below, and adding a second diagnostic about a field of a
                    // row that contributes to nothing would describe another run's artifact as
                    // though it were a defect in this one.
                    if let Some(defect) = defect {
                        diagnostics.push(format!(
                            "⚠️ a row of the machine-readable report of feature area `{}` names an \
                             artifact directory that its own identity does not: {defect}",
                            spec.directory()
                        ));
                    }
                    rows.push(row);
                } else {
                    *foreign
                        .entry((provenance.run, provenance.identity))
                        .or_default() += 1;
                }
            }
            Err(problem) => diagnostics.push(format!(
                "⚠️ a row of the machine-readable report of feature area `{}` could not be read, so \
                 that comparison is missing from every total: {problem}",
                spec.directory()
            )),
        }
    }
    for ((run, claimed), count) in foreign {
        diagnostics.push(format!(
            "⚠️ {count} row(s) of the machine-readable report of feature area `{}` were refused \
             because {}",
            spec.directory(),
            identity.describe_mismatch(&run, &claimed)
        ));
    }
    AreaState::Current(AreaReport::from_rows(spec, rows, diagnostics))
}

/// Write the run summary, if and only if every feature area of *this run* has filed its report.
///
/// Every area test calls this at the end of its run. All but one find at least one area still
/// outstanding, or find that another thread has already claimed the work, and return `Ok(false)`
/// having read and written nothing at all; exactly one finds all
/// [`AREA_COUNT`](super::AREA_COUNT) of them registered, claims finalization, aggregates the lot and
/// writes the summary. That is the whole of the coordination — a check and a claim in one critical
/// section — and it requires no ordering between the area tests and no extra test of its own, which
/// matters because the suite's test count is itself one of the mechanical checks that no
/// pre-existing test was skipped or weakened.
///
/// Completeness is decided from the registry of areas this process published, never from a directory
/// listing. The two are not the same question: a report file left behind by an earlier — possibly
/// reduced, possibly differently configured — run is a perfectly valid artifact, and counting it
/// would produce a summary that looks complete and describes no single run. Registration follows
/// publication, so a complete registry also means a complete set of files on disk to read back.
///
/// The files are then read as *this run's* files, and the two independent stamps are what makes that
/// true rather than assumed: an area file whose generation names another run is excluded from every
/// total and listed as stale, and any individual row that survived the clearing anyway is refused
/// because its provenance names a different run. A complete-looking set of stale files therefore
/// produces a summary full of loud diagnostics and empty totals rather than a quiet pass over a
/// matrix nobody ran.
///
/// Two areas finishing at the same instant cannot both write, because the claim below is taken in
/// one critical section; if the claim is released after a failed attempt, a later area retries it and
/// each writes by rename over its own temporary sibling, so a reader sees one complete summary or
/// the other and never a blend of the two.
///
/// Returns whether the summary was written. A `false` is not a failure — it is the ordinary answer
/// for every call but the last one of the run, and also the answer while a selected area's report is
/// still missing or still belongs to an earlier run.
///
/// # Completeness is measured against the areas this invocation selected
///
/// Not against all fourteen. [`libtest_filter_selection`]'s `expected_areas` is the set this process
/// can ever publish, and it is what [`claim_finalization`] and [`finalize`] both test against:
/// waiting for an area a test-name filter excluded would withhold the deliverable indefinitely, and
/// writing before the selected areas have all reported would publish a subset of a subset. A run
/// whose selection is narrower than the whole table publishes a summary over what it selected and
/// stamps it **partial**, listing every area that did not contribute as absent or stale. What it
/// never does is fill those gaps from another run's files.
///
/// # Errors
///
/// Fails only when the summary itself cannot be written, and releases its claim first so that a
/// later caller can try again rather than leaving the run with no summary and no way to produce one.
/// A damaged or unreadable area file does not fail the run: it becomes a diagnostic, the summary is
/// stamped partial, and the outcomes it would have contributed are reported as missing. Losing the
/// whole summary because one area file was unreadable would hide every other area's results to
/// punish one.
pub fn try_finalize(caps: &Capabilities) -> HarnessResult<bool> {
    // The same ownership-aware preparation `write_area` performs, and for the same reason: this
    // function *reads* the report root, so it must not read it before an earlier run's artifacts
    // have been cleared out of it. Remembered per process, so this is a no-op after the first area.
    ensure_report_namespace("finalizing the differential conformance run summary")?;
    let Some(specs) = claim_finalization() else {
        return Ok(false);
    };
    match finalize(caps, &specs) {
        Ok(true) => Ok(true),
        // Nothing was written, so the claim belongs to whoever tries next rather than to this
        // caller: an area file that is still outstanding must not cost the run its summary.
        Ok(false) => {
            release_finalization_claim();
            Ok(false)
        }
        Err(error) => {
            release_finalization_claim();
            Err(error)
        }
    }
}

/// Aggregate the areas this run published and publish the summary pair.
///
/// Split from [`try_finalize`] so that the claim is taken, and released on failure, in exactly one
/// place; every path out of this function has already passed the completeness check.
fn finalize(caps: &Capabilities, specs: &[&'static AreaSpec]) -> HarnessResult<bool> {
    let current = Generation::current(caps);
    let identity = RunIdentity::of(caps);
    let selection = libtest_filter_selection();
    // The set to wait for is the set this process can ever produce, not all fourteen areas. A
    // filtered invocation runs a subset by design, so requiring the whole table would withhold the
    // summary from exactly the runs whose scope most needs recording — and the summary is a named
    // deliverable, so withholding it is a loss rather than a neutral omission. The reduction is
    // never hidden: a narrower set is reported as partial below.
    let expected = selection.expected_areas();
    let mut run = RunReport {
        filters: selection.all(),
        session: current.run.clone(),
        ..RunReport::default()
    };
    let mut accounted: BTreeSet<&'static str> = BTreeSet::new();
    for spec in AREAS.iter() {
        // Existence is decided by the read itself, so there is no window between a check and a use.
        match read_area(spec, &current, identity) {
            AreaState::Absent => {
                // Absence is ordinary for an area that did not run in this process, and a defect for
                // one this run published: the file existed when it was registered, so something
                // removed or replaced it since, and saying nothing would report the area as
                // "not run" when it did run.
                if specs
                    .iter()
                    .any(|registered| registered.directory() == spec.directory())
                {
                    accounted.insert(spec.directory());
                    run.unusable.push(spec);
                    run.diagnostics.push(format!(
                        "⚠️ this run published the machine-readable report of feature area `{}` and \
                         it is no longer there, so its outcomes are missing from every total; \
                         something removed or replaced the file after it was written.",
                        spec.directory()
                    ));
                }
            }
            AreaState::Current(report) => {
                accounted.insert(spec.directory());
                run.tally.merge(&report.tally);
                run.areas.push(report);
            }
            AreaState::Unusable(report) => {
                accounted.insert(spec.directory());
                run.unusable.push(spec);
                run.diagnostics.extend(report.diagnostics);
            }
            AreaState::Stale(stale) => run.stale.push(stale),
        }
    }

    // Completeness is measured against the areas this invocation selected, not against the whole
    // table. An area the filter excluded is never going to file a report, so waiting for it would
    // withhold the summary for ever; an area that was selected and has not filed yet is exactly
    // what waiting is for, because the caller that finishes it will find the set complete and
    // write the summary then. Waiting is also what keeps an earlier run's rows out of this run's
    // totals: a stale file counts as outstanding, not as a contribution.
    if expected
        .iter()
        .any(|spec| !accounted.contains(&spec.directory()))
    {
        // The registry said the set was complete or this function would not have been reached, so an
        // area unaccounted for here is one whose file is no longer usable: removed since it was
        // published, or belonging to a different run. That conclusion was reached by reading, and
        // reading is this function's privilege alone, so it is recorded for the callers that only
        // want to explain the absence of a summary.
        record_finalization_deferral(Some(deferral_reason(&run, &current, &expected, &accounted)));
        return Ok(false);
    }
    if run.areas.is_empty() && !expected.is_empty() {
        // Nothing of this run's own is on disk, so there is nothing to summarise. Writing a summary
        // of zero areas would replace a previous run's summary with an empty one, which is worse
        // than leaving the previous one in place and saying nothing. A filter that selects no area
        // at all is the one exception, and it is handled below: there the emptiness is the fact
        // being recorded.
        record_finalization_deferral(Some(deferral_reason(&run, &current, &expected, &accounted)));
        return Ok(false);
    }
    // Everything this invocation selected is present and belongs to this run, so any reason recorded
    // by an earlier attempt describes a state that no longer holds and must not outlive it.
    record_finalization_deferral(None);
    if expected.is_empty() {
        run.diagnostics.push(String::from(
            "⚠️ no feature area could be selected by this process's test-name filters, so this \
             summary describes no outcomes at all. It is written rather than withheld so that the \
             filter is on record.",
        ));
    }

    run.expected = expected.len();

    let specs: Vec<&'static AreaSpec> = run.areas.iter().map(|area| area.spec).collect();
    run.facts = CorpusFacts::for_areas(&specs);

    let rows: Vec<Row> = run
        .areas
        .iter()
        .flat_map(|area| area.rows.iter().cloned())
        .collect();
    run.diagnostics
        .extend(diagnose_unrecorded_programs(&rows, &run.facts));

    // One line, not one per pruning: the detail belongs in the retained-evidence section, and a
    // hundred pruning notes in the diagnostics list would bury the diagnostics that describe
    // verdicts. What must not be missing from here is the *fact* that some retained workspace holds
    // less than its cell produced, because a reader who never reaches that section would otherwise
    // go looking for a file that was accounted for and removed.
    let prunings = super::sandbox::retention_pruning_notes().len();
    if prunings > 0 {
        run.diagnostics.push(format!(
            "⚠️ {prunings} pruning(s) were performed to keep the run inside its retention budget, so \
             at least one retained workspace holds less than its cell produced. Each is named in the \
             retained-evidence section below."
        ));
    }

    // The corpus states how many programs each area holds; a discovered count that disagrees is a
    // corpus defect, and the summary says so rather than quietly reporting the number it found.
    //
    // Only the areas this run aggregated are checked. An area that filed no report in this process
    // had its records left unread, so its discovered count would be zero for a reason that has
    // nothing to do with the corpus; reporting that as a corpus defect would be a fabricated
    // diagnostic. What was left out is stated by the provenance section instead, in the terms that
    // are actually true of it — absent, stale or unusable.
    for spec in specs.iter() {
        let discovered = run
            .facts
            .programs
            .iter()
            .filter(|facts| facts.area == spec.directory())
            .count();
        if discovered != spec.program_count() {
            run.diagnostics.push(format!(
                "⚠️ feature area `{}` is defined as holding {} program(s) but {discovered} \
                 expectation record(s) were discovered in it. Either the corpus or the area table \
                 is wrong; the counts reported below are the discovered ones.",
                spec.directory(),
                spec.program_count()
            ));
        }
        if spec.mandated() && discovered < MIN_PROGRAMS_PER_MANDATED_AREA {
            run.diagnostics.push(format!(
                "⚠️ feature area `{}` is mandated by the coverage requirement, which sets a floor \
                 of {MIN_PROGRAMS_PER_MANDATED_AREA} programs, but {discovered} were discovered.",
                spec.directory()
            ));
        }
    }

    // The summary is the deliverable that promises every finding with its reproducer and its
    // reproduction commands, so the promise is checked against disk immediately before it is made —
    // not inherited from the area reports this summary aggregated. Those checks ran when each area
    // published, which for the first of fourteen areas is the whole length of the run ago, and the
    // artifacts are separate files that nothing has held since. Re-asking here is what stops the
    // run's final artifact from advertising a directory that was emptied while the rest of the
    // matrix executed. The recorded answer is what the finalizing area test fails on, after the
    // summary has been written.
    let shortfalls = finding_artifact_shortfalls(&rows);
    record_artifact_shortfalls(SUMMARY_SCOPE, shortfalls.clone());
    for shortfall in shortfalls {
        run.diagnostics
            .push(format!("⚠️ at summary publication: {shortfall}"));
    }

    let dimensions = run_matrix(&run, caps);
    let coverage = assess_run_coverage(caps, &run, &dimensions);
    write_report_pair(
        "writing the differential conformance run summary",
        &summary_markdown_path(),
        &render_summary_markdown(&run, caps, &coverage, &dimensions, &current),
        &summary_tsv_path(),
        &render_summary_tsv(&run, caps, &coverage, &dimensions, &current),
    )?;
    Ok(true)
}

/// The finalizer's own account of why it declined, built from the files it read.
///
/// Composed here rather than at the two `return Ok(false)` sites so that both record the same shape of
/// answer, and so that the sentence names every selected area it could not account for — an area whose
/// file was removed after this run published it, and one whose file belongs to another run, fail the
/// same way round but for opposite reasons, and a reader needs to be told which.
///
/// The result is stored in the registry and printed verbatim by [`finalization_pending`], which is what
/// lets that function explain a disk-derived refusal without reading anything itself.
fn deferral_reason(
    run: &RunReport,
    current: &Generation,
    expected: &[&'static AreaSpec],
    accounted: &BTreeSet<&'static str>,
) -> String {
    let missing: Vec<&'static str> = expected
        .iter()
        .map(|spec| spec.directory())
        .filter(|directory| !accounted.contains(directory))
        .collect();
    let mut clauses: Vec<String> = Vec::new();
    if !missing.is_empty() {
        clauses.push(format!(
            "{} selected area report(s) could not be accounted for when the set was aggregated \
             ({})",
            missing.len(),
            missing.join(", ")
        ));
    }
    if !run.stale.is_empty() {
        clauses.push(format!(
            "{} report(s) were REFUSED as belonging to a different run — {}",
            run.stale.len(),
            run.stale
                .iter()
                .map(|stale| format!("`{}` — {}", stale.spec.directory(), stale.note))
                .collect::<Vec<String>>()
                .join("; ")
        ));
    }
    if !run.unusable.is_empty() {
        clauses.push(format!(
            "{} report(s) were present but unusable — {}",
            run.unusable.len(),
            run.unusable
                .iter()
                .map(|spec| format!("`{}`", spec.directory()))
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }
    if clauses.is_empty() {
        // Reached when every selected area was accounted for and yet nothing of this run's own could
        // be aggregated, which is the empty-selection case the caller handles separately. Named rather
        // than left blank, because an unexplained refusal is the one answer this module never gives.
        clauses.push(String::from(
            "no area report belonging to this run could be aggregated at all",
        ));
    }
    format!(
        "{}. This run is {}.",
        clauses.join("; "),
        current.describe()
    )
}

/// Why the run summary has not been written, in a sentence a caller can print.
///
/// [`try_finalize`] answers whether the summary was written; this answers why not, which is the
/// question the answer `false` actually raises. It distinguishes the two reasons, because they call
/// for opposite responses from a reader: areas still awaited are the ordinary state of every call but
/// the last one of the run and need no action at all, whereas a refused artifact means a file this
/// run published is no longer there, or belongs to another run's configuration, and the summary will
/// not appear until that is put right.
///
/// # Answered from the registry, not from the report directory
///
/// This used to re-derive its answer by reading every area report on disk, which was work performed
/// to rediscover a conclusion that had already been reached moments earlier: the completeness test
/// lives inside [`claim_finalization`]'s critical section, so by the time a caller is asking *why*
/// nothing was written, the registry already knows which selected areas have published. In a
/// fourteen-area run the thirteen non-final callers each re-parsed the whole set — quadratic in the
/// number of areas, against files that grow with the matrix — to learn something memory could answer
/// for nothing.
///
/// So the answer now comes from memory, and **artifact reads belong to the single finalizer**. The one
/// thing memory cannot know is a disk-derived refusal, because only the finalizer looks at the files;
/// that is why [`finalize`] records its reason through [`record_finalization_deferral`] when it
/// declines, and why this reports that recorded reason verbatim. Nothing is lost and nothing is
/// guessed: every sentence below is either a fact the registry holds or a fact the finalizer
/// established by reading.
///
/// Never fails, and never reads a file: this is the explanation of a non-failure.
pub fn finalization_pending() -> String {
    let (awaited, deferral) = finalization_state();

    if awaited.is_empty() && deferral.is_none() {
        return format!(
            "every area this invocation selected has published a report belonging to this run, so \
             the summary was written by whichever area completed the set. Nothing is outstanding. \
             (The table holds {AREA_COUNT} areas in all; a filtered invocation selects fewer, and \
             its summary is stamped partial.)"
        );
    }

    let mut sentences: Vec<String> = Vec::new();
    if !awaited.is_empty() {
        sentences.push(format!(
            "it is written once every area this invocation selected has filed a report belonging \
             to this run; {} of the selected areas have not published yet ({}), out of the \
             {AREA_COUNT} in the table. Under a name filter or a single-area invocation only the \
             selected areas are waited for, so such a run does not sit pending for ever: its \
             summary is published over the areas it did select and stamped partial, and each area's \
             own report was written regardless.",
            awaited.len(),
            awaited.join(", ")
        ));
    }
    if let Some(reason) = deferral {
        sentences.push(format!(
            "⚠️ the finalizer read the report directory and declined: {reason} A summary that mixed \
             two configurations would present another run's verdicts as this one's — a row means \
             something different under a different matrix, filter, verdict policy or per-cell budget, \
             so the two cannot be added together — and one assembled over a file that has since \
             vanished would report an area as not run when it did run."
        ));
    }
    sentences.join(" ")
}
