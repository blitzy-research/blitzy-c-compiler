//! Per-area reports and the run summary — the deliverable of the differential conformance
//! suite.
//!
//! Every other module in this harness answers a question about one cell. This one answers the
//! question the requirements close with: *what did the whole run find?* The summary it writes
//! is the artifact that reports the feature areas covered, the total tests and their outcomes,
//! every expected divergence with its documented basis, and every finding with its minimized
//! reproducer and reproduction commands. Those four elements are the module's specification,
//! and [`render_summary_markdown`] carries them as four numbered sections so the artifact can
//! be checked against the requirement literally rather than impressionistically.
//!
//! # Artifacts
//!
//! | Path | Content |
//! |---|---|
//! | `target/conformance-report/summary.md` | Human-readable deliverable summary |
//! | `target/conformance-report/summary.tsv` | The same data, machine-readable for aggregation |
//! | `target/conformance-report/areas/<area>.md` | Per-area human-readable report |
//! | `target/conformance-report/areas/<area>.tsv` | Per-area machine-readable report |
//!
//! Those four paths are a contract shared with the suite driver, with the build directory's
//! ignore rules and with the continuous-integration job that uploads them, so they are named by
//! the constants and helpers below rather than spelled at a call site. Everything is written
//! beneath [`report_root`] and nothing is written anywhere else — in particular nothing is ever
//! written under the corpus root, whose two registers
//! (`tests/conformance/EXPECTED_DIVERGENCES.md`, `tests/conformance/FINDINGS.md`) and curated
//! finding set are committed deliverables maintained by hand, not run output.
//!
//! # Why one file per area, and no lock anywhere
//!
//! The test harness runs the fourteen area tests concurrently by default. Each one calls
//! [`write_area`] for its own area and therefore writes only `areas/<its own area>.md` and
//! `areas/<its own area>.tsv`: two concurrent areas cannot contend, because they cannot name the
//! same file. Parallel safety is structural rather than enforced, so this module contains no
//! mutex, no lock file, no global mutable state and no shared append-only log. Every write goes
//! to a uniquely named temporary sibling and is then renamed into place, which is atomic on the
//! platforms this suite supports, so a concurrent reader observes either the previous file or the
//! complete new one and never a half-written one.
//!
//! # Once-only finalization, without ordering and without an extra test
//!
//! [`try_finalize`] is called by **every** area test at the end of its run. It is a
//! check-and-write: if all fourteen machine-readable area files exist it aggregates them and
//! writes the summary, and otherwise it does nothing and reports that it did nothing. Whichever
//! area finishes last therefore produces the summary, no ordering between tests is required, and
//! no fifteenth test has to exist to do it — which matters because the suite's test count is
//! itself a mechanical check that no existing test was skipped or removed. If two areas finish
//! at once and both see all fourteen files, both may write; the write is idempotent and atomic,
//! so the outcome is identical either way. Losing that race is never an error.
//!
//! # Determinism
//!
//! Identical inputs produce byte-identical reports. There is no wall-clock timestamp, no elapsed
//! duration, no process identifier and no iteration over an unordered collection anywhere in the
//! rendered text: rows are sorted by program, then target in [`Target::ALL`] order, then level in
//! [`OptLevel::ALL`] order, then oracle in [`Oracle::ALL`] order, and every map is ordered. A
//! report that reordered itself between runs would produce phantom differences and lose exactly
//! the regression value it exists to provide.
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
use std::fs;
use std::path::{Path, PathBuf};

use super::classify::{verdict_fails_run, EXPECTED_DIVERGENCE_REGISTER, FINDINGS_REGISTER};
use super::env::{
    Capabilities, VAR_ALLOW_MISSING_ORACLES, VAR_ALLOW_XPASS, VAR_KEEP_WORK, VAR_ONLY, VAR_QUICK,
    VAR_STRICT, VAR_TIMEOUT_SECS,
};
use super::findings::{FindingId, COMMANDS_NAME};
use super::manifest::{self, ExpectedDivergence};
use super::{
    posix_quote, report_root, sanitize_text_for_report, AreaSpec, DivergenceClass, HarnessError,
    HarnessResult, OptLevel, Oracle, Outcome, Target, Verdict, AREAS, AREA_COUNT, BCC_CELL_COUNT,
    MIN_PROGRAMS_PER_MANDATED_AREA, ORACLE_A_COMPARISON_COUNT, ORACLE_B_COMPARISON_COUNT,
    ORACLE_C_ASSERTION_COUNT, PROGRAM_COUNT, REFERENCE_CROSS_CELL_COUNT_MAX,
    REFERENCE_NATIVE_CELL_COUNT, TOTAL_ASSERTION_COUNT,
};

/// Directory beneath [`report_root`] that holds the per-area reports.
pub const AREAS_DIR_NAME: &str = "areas";

/// File stem of the two run-summary artifacts.
pub const SUMMARY_STEM: &str = "summary";

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

/// Column order of a per-area machine-readable report, one row per recorded outcome.
///
/// Fixed and stable across runs so the file can be diffed between runs and aggregated by an
/// external tool, and re-read by [`try_finalize`] — which is why every enumeration in it is
/// written in the spelling its own `parse` function accepts. `detail` is last because it is the
/// only column of unbounded length.
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
/// | `outcome` | `area`, `program`, `target`, `opt`, `oracle`, `verdict`, `class`, `marker_id`, `label`, `reference`, `detail` |
/// | `expected_divergence` | `area`, `program`, `class`, `marker_id`, `count`, `label` (scope), `reference` (basis path), `detail` |
/// | `finding` | `area`, `program`, `target`, `opt`, `oracle`, `class`, `label` (identifier), `reference` (directory), `detail` |
/// | `unavailable` | `area`, `program`, `target`, `opt`, `oracle`, `detail` |
/// | `exclusion` | `area`, `program`, `oracle`, `label` (kind), `reference` (what was narrowed), `detail` |
/// | `diagnostic` | `detail` |
/// | `fingerprint` | `label`, `detail` |
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

/// Absolute path of the human-readable run summary — the suite's deliverable.
pub fn summary_markdown_path() -> PathBuf {
    report_root().join(format!("{SUMMARY_STEM}.{MARKDOWN_EXTENSION}"))
}

/// Absolute path of the machine-readable run summary.
pub fn summary_tsv_path() -> PathBuf {
    report_root().join(format!("{SUMMARY_STEM}.{TSV_EXTENSION}"))
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

/// Publish `contents` at `path`, atomically.
///
/// The bytes are written to a uniquely named sibling and then renamed over the destination.
/// Renaming within one directory replaces the destination atomically on the platforms this suite
/// supports, so the destination is only ever the previous complete file or the new complete file.
/// The temporary name carries the process and thread identity, which is what makes it unique
/// between concurrent writers without a lock; it never appears in an artifact, so it cannot
/// affect determinism.
///
/// # Errors
///
/// Returns an explanatory failure naming `context` when the parent directory cannot be created,
/// when the temporary file cannot be written, or when the rename fails. A failed write removes
/// its temporary file on a best-effort basis, so a failure does not accumulate debris.
fn write_atomic(context: &str, path: &Path, contents: &str) -> HarnessResult<()> {
    let parent = path.parent().ok_or_else(|| {
        HarnessError::new(
            String::from(context),
            format!(
                "{} has no parent directory, so it cannot name a report artifact; every artifact \
                 is published into a directory beneath the build directory",
                shown_path(path)
            ),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        HarnessError::new(
            String::from(context),
            format!(
                "the report directory {} could not be created: {error}",
                shown_path(parent)
            ),
        )
    })?;

    let temporary = parent.join(temporary_name(path));
    if let Err(error) = fs::write(&temporary, contents) {
        let _ = fs::remove_file(&temporary);
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the temporary report file {} could not be written: {error}",
                shown_path(&temporary)
            ),
        ));
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(HarnessError::new(
            String::from(context),
            format!(
                "the temporary report file {} could not be renamed onto {}: {error}; the previous \
                 report, if any, is left intact rather than replaced by a partial one",
                shown_path(&temporary),
                shown_path(path)
            ),
        ));
    }
    Ok(())
}

/// A temporary file name for `path` that no concurrent writer can also choose.
///
/// Built from the destination's own file name plus the writing process and thread, so two
/// concurrent writers of the same destination — which only the summary can have — cannot collide,
/// and a leftover file from an earlier crashed run is simply truncated and reused rather than
/// accumulating. The leading dot keeps it out of the way of anything listing the reports, and the
/// name is not one of the fourteen area file names, so a temporary file can never be mistaken for
/// a finished area report by the finalization gate.
fn temporary_name(path: &Path) -> String {
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("report");
    format!(
        ".{}.{}.{}.tmp",
        keep_alphanumeric(stem),
        std::process::id(),
        keep_alphanumeric(&format!("{:?}", std::thread::current().id()))
    )
}

/// `raw` with every character outside the ASCII alphanumerics and the underscore removed.
///
/// Used only to build a temporary file name from a debug rendering, where the requirement is a
/// short token that is safe as a single path component rather than a faithful reproduction.
fn keep_alphanumeric(raw: &str) -> String {
    raw.chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Text rendering
//
// Three kinds of text reach a report from outside this module: an outcome's detail, which the
// harness assembled from compiler diagnostics and program output; free text a maintainer wrote
// into an expectation record, which may legitimately span several lines; and a tool's own
// identification banner. None of them may be emitted verbatim into a line-oriented artifact, and
// the three functions below are the only places that decide how each is made safe.
// ---------------------------------------------------------------------------------------------

/// One field of a machine-readable report.
///
/// [`sanitize_text_for_report`] escapes every control character — including the tab and the line
/// feed — and every formatting character that could make a line render as something other than
/// what it says. A field therefore cannot forge a column, cannot split one record into two and
/// cannot repaint a verdict, which is what lets a row be counted as one outcome.
fn tsv_field(raw: &str) -> String {
    sanitize_text_for_report(raw)
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
/// Collapsed to a single line, escaped so it cannot act, and then bounded by
/// [`TABLE_CELL_MAX_CHARS`]. The vertical bar is escaped because it would otherwise end the cell
/// and shift every value after it into the wrong column — the table equivalent of forging a
/// tab-separated field. Truncation is marked with [`TRUNCATION_MARK`] and happens on a character
/// boundary, never inside a character.
fn table_cell(raw: &str) -> String {
    let collapsed = sanitize_text_for_report(&collapse_whitespace(raw)).replace('|', "\\|");
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
/// is escaped independently, and an empty input line becomes a bare quote marker so the paragraph
/// break is preserved.
fn quoted_block(raw: &str) -> Vec<String> {
    let trimmed = raw.trim_end();
    if trimmed.trim().is_empty() {
        return vec![String::from("> *(no text recorded)*")];
    }
    trimmed
        .lines()
        .map(|line| {
            let safe = sanitize_text_for_report(line.trim_end());
            if safe.is_empty() {
                String::from(">")
            } else {
                format!("> {safe}")
            }
        })
        .collect()
}

/// A filesystem path rendered safely for a report line.
fn shown_path(path: &Path) -> String {
    sanitize_text_for_report(&path.display().to_string())
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
    detail: String,
}

impl Row {
    /// Build a row from an accumulated outcome.
    ///
    /// A finding's artifact directory is *derived* rather than looked up: [`FindingId::derive`] is
    /// a pure function of the cell, the oracle and the divergence class, and the directory is that
    /// identifier beneath the findings root. The report can therefore name the artifacts of a
    /// finding without being handed anything by the module that wrote them, and the name it prints
    /// is the same one that module chose. A finding carrying no divergence class is the one case
    /// where no directory can be derived; it is left empty here and reported as a diagnostic by
    /// [`AreaReport::from_outcomes`], because a finding without a class is an internal
    /// inconsistency rather than a fact about the compiler.
    fn from_outcome(outcome: &Outcome) -> Row {
        let key = outcome.key();
        let identifier = match (outcome.verdict(), outcome.class()) {
            (Verdict::Finding, Some(class)) => {
                Some(FindingId::derive(key, outcome.oracle(), class))
            }
            _ => None,
        };
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
    fn to_tsv(&self) -> String {
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
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            self.detail.clone(),
        ])
    }

    /// Rebuild a row from one line of a per-area machine-readable report.
    ///
    /// # Errors
    ///
    /// Returns a sentence describing the defect — the wrong number of fields, or a value no
    /// canonical spelling matches — rather than a [`HarnessError`], because the caller turns it
    /// into a loud diagnostic in the summary instead of failing the run. An area file that cannot
    /// be read is a defect in the artifact, and a summary that says so is more useful than a run
    /// that aborts before writing one.
    fn parse(line: &str, number: usize) -> Result<Row, String> {
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
        Ok(Row {
            area: String::from(value(0)),
            program: String::from(value(1)),
            target,
            opt,
            oracle,
            verdict,
            class,
            marker_id: optional_field(fields[7]),
            finding_id: optional_field(fields[8]),
            finding_dir: optional_field(fields[9]).map(PathBuf::from),
            detail: String::from(fields[10]),
        })
    }
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
    fn from_outcomes(spec: &'static AreaSpec, outcomes: &[Outcome]) -> AreaReport {
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
            report.absorb(Row::from_outcome(outcome), &mut seen);
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
                Err(error) => facts.diagnostics.push(format!(
                    "⚠️ the expectation record beside {} could not be read: {error}",
                    shown_path(&source)
                )),
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
        }
        facts
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

/// The whole run, aggregated from the fourteen per-area machine-readable reports.
#[derive(Debug, Clone, Default)]
struct RunReport {
    areas: Vec<AreaReport>,
    /// Areas whose file existed but could not be used, so their outcomes are absent from the
    /// totals. Listed, never quietly skipped.
    unusable: Vec<&'static AreaSpec>,
    tally: Tally,
    facts: CorpusFacts,
    /// Test-name filters this process was started with, which is what tells the summary that some
    /// area files may predate this run.
    filters: Vec<String>,
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

    /// How many distinct programs produced at least one comparison.
    fn observed_programs(&self) -> usize {
        self.areas.iter().map(|area| area.programs.len()).sum()
    }

    /// How many distinct cells produced at least one comparison.
    fn observed_cells(&self) -> usize {
        self.areas.iter().map(|area| area.cells.len()).sum()
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
// does, plus a test-name filter, an area whose file could not be used, and a defect in the corpus
// the report had to read. Partial therefore always holds when reduced does.
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

/// Assess one area's coverage.
fn assess_area_coverage(caps: &Capabilities, report: &AreaReport) -> Coverage {
    let mut coverage = Coverage::default();
    note_configuration_reductions(caps, &mut coverage);
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
    if !report.diagnostics.is_empty() {
        coverage.note_partial(format!(
            "{} diagnostic(s) were raised while assembling this report; see the Diagnostics \
             section.",
            report.diagnostics.len()
        ));
    }
    coverage
}

/// Assess the whole run's coverage.
fn assess_run_coverage(caps: &Capabilities, run: &RunReport) -> Coverage {
    let mut coverage = Coverage::default();
    note_configuration_reductions(caps, &mut coverage);
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
            "{} of {AREA_COUNT} feature areas contributed outcomes to this summary.",
            run.areas.len()
        ));
    }
    for spec in &run.unusable {
        coverage.note_partial(format!(
            "The machine-readable report of feature area `{}` could not be used, so its outcomes \
             are missing from every total below.",
            spec.directory()
        ));
    }
    if !run.facts.diagnostics.is_empty() {
        coverage.note_partial(format!(
            "{} corpus defect(s) prevented part of the expectation-record material from being \
             read; see the Diagnostics section.",
            run.facts.diagnostics.len()
        ));
    }
    coverage
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

/// The test-name filters this process was started with.
///
/// A filter means the process ran a subset of the suite, which matters because the summary is
/// assembled from the per-area files on disk: with a filter active, some of those files may have
/// been written by an earlier run, and a summary that combined them without saying so would present
/// stale results as current.
///
/// The reading is deliberately conservative. Anything that is not an option and is not the value of
/// a value-taking option counts as a filter, and a `--skip` value counts too. Reading one argument
/// too many can only add a partial stamp that was not strictly required, which is the harmless
/// direction; failing to notice a filter would be the harmful one.
///
/// [`std::env::args_os`] is used rather than [`std::env::args`] because the latter panics on an
/// argument that is not valid Unicode, and this module contains no panicking path.
fn libtest_name_filters() -> Vec<String> {
    let mut filters = Vec::new();
    let mut expect_value_of: Option<String> = None;
    for argument in std::env::args_os().skip(1) {
        let text = argument.to_string_lossy().to_string();
        if let Some(option) = expect_value_of.take() {
            if option == "--skip" {
                filters.push(text);
            }
            continue;
        }
        if text.starts_with('-') {
            let name = text.split('=').next().unwrap_or(text.as_str());
            if !text.contains('=') && LIBTEST_VALUE_OPTIONS.contains(&name) {
                expect_value_of = Some(String::from(name));
            }
            continue;
        }
        filters.push(text);
    }
    filters
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

/// The state of a finding's artifact directory, checked rather than assumed.
///
/// A report that named a directory nobody could open would be worse than one that admitted the
/// artifacts are missing, because the reproduction commands are the whole point of a finding.
fn finding_artifact_state(directory: &Path) -> &'static str {
    if !directory.is_dir() {
        "⚠️ directory absent"
    } else if !directory.join(COMMANDS_NAME).is_file() {
        "⚠️ commands.sh absent"
    } else {
        "✅ present"
    }
}

/// The command that reproduces a finding with no harness, no Cargo and no Rust toolchain.
fn reproduce_command(directory: &Path) -> String {
    format!(
        "sh {}",
        posix_quote(&directory.join(COMMANDS_NAME).display().to_string())
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
    lines.push(format!("### `{}`", sanitize_text_for_report(marker.id())));
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
                Some(facts) => format!("`{}`", facts.label()),
                None => format!("`{}`", marker.program_label()),
            },
        ),
        (String::from("Documented basis"), table_cell(marker.basis())),
        (
            String::from("Cited document"),
            format!(
                "`{}` — {}",
                shown_path(marker.basis_path()),
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
                "- `{}` under `oracle_{}`",
                cell.cell_label(),
                cell.oracle.letter()
            ));
        }
    }
    lines
}

/// Every finding as a table, each row naming its artifact directory and its reproduction command.
fn render_finding_table(rows: &[&Row]) -> Vec<String> {
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let directory = row.finding_dir.clone();
            vec![
                match &row.finding_id {
                    Some(identifier) => format!("`{}`", sanitize_text_for_report(identifier)),
                    None => String::from("⚠️ not derivable (no divergence class)"),
                },
                format!("`{}`", row.cell_label()),
                format!("oracle_{}", row.oracle.letter()),
                row.class
                    .map(|class| String::from(class.label()))
                    .unwrap_or_else(|| String::from(ABSENT_CELL)),
                match &directory {
                    Some(path) => format!("`{}`", shown_path(path)),
                    None => String::from(ABSENT_CELL),
                },
                match &directory {
                    Some(path) => String::from(finding_artifact_state(path)),
                    None => String::from(ABSENT_CELL),
                },
                match &directory {
                    Some(path) => {
                        format!("`{}`", sanitize_text_for_report(&reproduce_command(path)))
                    }
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
                format!("`{}`", row.cell_label()),
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
                format!("`{}`", facts.label()),
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
        .map(|diagnostic| format!("- {}", sanitize_text_for_report(diagnostic)))
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
        .map(|reason| format!("- {}", sanitize_text_for_report(reason)))
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
fn render_expected_divergence_section(facts: &CorpusFacts, xfail: &[&Row]) -> Vec<String> {
    let mut grouped: BTreeMap<String, Vec<&Row>> = BTreeMap::new();
    for row in xfail {
        let identifier = row
            .marker_id
            .clone()
            .unwrap_or_else(|| String::from("(no marker identifier recorded)"));
        grouped.entry(identifier).or_default().push(row);
    }

    let declared = facts.markers();
    let mut lines = Vec::new();
    if declared.is_empty() && grouped.is_empty() {
        lines.push(String::from(
            "None — no expected-divergence marker applies here, so every divergence would be a \
             finding or a failure.",
        ));
        return lines;
    }

    for (owner, marker) in &declared {
        let cells = grouped.remove(marker.id()).unwrap_or_default();
        lines.extend(render_marker_block(marker, Some(owner), &cells));
        lines.push(String::new());
    }
    for (identifier, cells) in grouped {
        lines.push(format!("### `{}`", sanitize_text_for_report(&identifier)));
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
                "- `{}` under `oracle_{}`: {}",
                cell.cell_label(),
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
fn render_area_markdown(
    report: &AreaReport,
    facts: &CorpusFacts,
    caps: &Capabilities,
    coverage: &Coverage,
) -> String {
    let spec = report.spec;
    let config = caps.config();
    let (targets, levels) = config.effective_matrix();
    let discovered = facts.programs.len();
    let planned_cells = facts.planned_cells(&targets, &levels);

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
            String::from("Programs planned (corpus table)"),
            spec.program_count().to_string(),
        ),
        (
            String::from("Programs discovered (expectation records read)"),
            format!("{discovered} — {}", met(discovered == spec.program_count())),
        ),
        (
            String::from("Programs that produced a comparison"),
            report.programs.len().to_string(),
        ),
        (
            format!("Mandated floor (at least {MIN_PROGRAMS_PER_MANDATED_AREA} programs)"),
            if spec.mandated() {
                String::from(met(discovered >= MIN_PROGRAMS_PER_MANDATED_AREA))
            } else {
                String::from("not applicable — this area is supplementary")
            },
        ),
        (
            String::from("Cells planned for this run"),
            if facts.programs.is_empty() {
                String::from("⚠️ not determined — no expectation record could be read")
            } else {
                planned_cells.to_string()
            },
        ),
        (
            String::from("Cells that produced a comparison"),
            report.cells.len().to_string(),
        ),
        (
            String::from("Comparisons recorded"),
            report.tally.total().to_string(),
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
        "## Findings — minimized reproducer and reproduction commands",
    ));
    lines.push(String::new());
    if findings.is_empty() {
        lines.push(String::from(
            "None — no undocumented divergence was observed in this area.",
        ));
    } else {
        lines.push(format!(
            "A finding is a deliverable, not a defect to patch: no compiler source change is made \
             in response to one. Each directory holds the reproducer, its expectation record, the \
             captured outputs, the environment fingerprint, the computed difference and \
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

/// One feature area's machine-readable report: the fixed header, then one row per outcome.
fn render_area_tsv(report: &AreaReport) -> String {
    let mut lines = Vec::with_capacity(report.rows.len() + 1);
    lines.push(tsv_header(AREA_TSV_COLUMNS));
    for row in &report.rows {
        lines.push(row.to_tsv());
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
// minimized reproducer and reproduction commands. The numbering is not decoration — it is what lets
// the artifact be checked against the requirement literally.
// ---------------------------------------------------------------------------------------------

/// One row of the matrix table: what was planned, what was recorded, and whether it fell short.
fn matrix_row(dimension: &str, planned: usize, actual: usize, note: &str) -> Vec<String> {
    let status = if actual >= planned {
        String::from("✅ complete")
    } else {
        format!("⚠️ short by {}", planned - actual)
    };
    vec![
        String::from(dimension),
        planned.to_string(),
        actual.to_string(),
        status,
        String::from(note),
    ]
}

/// The enumerable matrix, planned against recorded. This table is the suite's coverage evidence.
fn render_matrix_table(run: &RunReport, caps: &Capabilities) -> Vec<String> {
    let (targets, levels) = caps.config().effective_matrix();
    let rows = vec![
        matrix_row(
            "Feature areas",
            AREA_COUNT,
            run.areas.len(),
            "nine mandated by the coverage requirement, five supplementary",
        ),
        matrix_row(
            "Programs",
            PROGRAM_COUNT,
            run.observed_programs(),
            "one semantic concern per program",
        ),
        matrix_row(
            "Targets swept",
            Target::ALL.len(),
            targets.len(),
            "x86-64 is the cross-backend baseline and executes natively",
        ),
        matrix_row(
            "Optimization levels swept",
            OptLevel::ALL.len(),
            levels.len(),
            "-O0, -O1, -O2 — the levels both compilers honour identically",
        ),
        matrix_row(
            "Compile-and-run cells",
            BCC_CELL_COUNT,
            run.observed_cells(),
            "one program, one target, one optimization level",
        ),
        matrix_row(
            "Oracle (a) comparisons",
            ORACLE_A_COMPARISON_COUNT,
            run.oracle_count(Oracle::ReferenceCompiler),
            &format!(
                "{REFERENCE_NATIVE_CELL_COUNT} native and up to {REFERENCE_CROSS_CELL_COUNT_MAX} \
                 cross reference cells"
            ),
        ),
        matrix_row(
            "Oracle (b) comparisons",
            ORACLE_B_COMPARISON_COUNT,
            run.oracle_count(Oracle::CrossBackend),
            "every non-baseline target against the baseline at the same level",
        ),
        matrix_row(
            "Oracle (c) assertions",
            ORACLE_C_ASSERTION_COUNT,
            run.oracle_count(Oracle::GoldenRecord),
            "every cell against the stdout its own record declares",
        ),
        matrix_row(
            "**Differential and golden assertions**",
            TOTAL_ASSERTION_COUNT,
            run.tally.total(),
            "the sum of the three oracles",
        ),
    ];
    markdown_table(
        &["Dimension", "Planned", "Recorded", "Status", "Note"],
        &rows,
    )
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
        let observed = report.map(|area| area.programs.len());
        let comparisons = report.map(|area| area.tally.total());
        let verdicts = match report {
            Some(area) => area.tally.compact(),
            None if run.unusable.contains(&spec) => String::from("⚠️ its report could not be used"),
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
            "Programs observed",
            "Comparisons",
            "Verdicts",
        ],
        &rows,
    )
}

/// The human-readable run summary — the suite's deliverable.
fn render_summary_markdown(run: &RunReport, caps: &Capabilities, coverage: &Coverage) -> String {
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
         expected divergence with its documented basis, and every finding with its minimized \
         reproducer and reproduction commands. Each of those four is a numbered section below.",
    ));
    lines.push(String::new());
    lines.push(String::from(
        "Every judgement here rests on stdout bytes and exit status alone. Standard error is \
         captured into finding artifacts and never compared, because diagnostic wording legitimately \
         differs between compilers and comparing it would report differences that say nothing about \
         code correctness.",
    ));
    lines.push(String::new());

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
            String::from("Verdict"),
            if failing == 0 {
                String::from("✅ no outcome fails this run")
            } else {
                format!("⚠️ {failing} outcome(s) fail this run")
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
                lines.push(format!("- {}", sanitize_text_for_report(arm)));
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
    lines.extend(render_matrix_table(run, caps));
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
        "A marker changes how a divergence is classified, never whether the feature is exercised: a \
         marked program still compiles and still runs. Every marker cites a limitation this \
         repository already documents, and the set is cross-referenced by \
         `{EXPECTED_DIVERGENCE_REGISTER}`."
    ));
    lines.push(String::new());
    lines.extend(render_expected_divergence_section(
        &run.facts,
        &run.rows_with(Verdict::XFail),
    ));
    lines.push(String::new());

    let findings = run.rows_with(Verdict::Finding);
    lines.push(String::from(
        "## 4 — Findings, with minimized reproducer and reproduction commands",
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
             the reproducer, its expectation record, a manifest, the captured stdout, exit status \
             and stderr per compiler and per backend, the environment fingerprint, the computed \
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
    for line in caps.render_fingerprint().lines() {
        lines.push(format!("    {}", sanitize_text_for_report(line)));
    }
    lines.push(String::new());

    lines.push(String::from("## Run configuration"));
    lines.push(String::new());
    lines.extend(property_table(&[
        (
            format!("`{VAR_QUICK}` (reduced matrix)"),
            String::from(yes_no(config.quick_mode())),
        ),
        (
            format!("`{VAR_ONLY}` (single program)"),
            match config.only() {
                Some(filter) => format!(
                    "`{}/{}`",
                    filter.area(),
                    sanitize_text_for_report(filter.program())
                ),
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
                    .map(|filter| format!("`{}`", sanitize_text_for_report(filter)))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ),
    ]));
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
fn render_summary_tsv(run: &RunReport, caps: &Capabilities, coverage: &Coverage) -> String {
    let config = caps.config();
    let failing = run.failing(caps);
    let mut rows: Vec<SummaryRow> = Vec::new();

    // Meta — the fields an aggregator reads first, including the two the reporting discipline
    // requires to be explicit rather than inferred from the prose.
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
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_expected")
            .set(COL_COUNT, AREA_COUNT.to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_contributed")
            .set(COL_COUNT, run.areas.len().to_string()),
    );
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "areas_unusable")
            .set(COL_COUNT, run.unusable.len().to_string()),
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
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "run_fails")
            .set(COL_DETAIL, true_false(failing > 0)),
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
    // Stated rather than computed, so an aggregator never has to guess why no percentage is here.
    rows.push(
        SummaryRow::new(RECORD_META)
            .set(COL_LABEL, "coverage_metric")
            .set(COL_DETAIL, "enumerable matrix; no percentage is measurable"),
    );

    for reason in &coverage.reasons {
        rows.push(SummaryRow::new(RECORD_COVERAGE_REASON).set(COL_DETAIL, reason.clone()));
    }

    let (targets, levels) = config.effective_matrix();
    let matrix: [(&str, usize, usize, &str); 9] = [
        (
            "feature_areas",
            AREA_COUNT,
            run.areas.len(),
            "nine mandated, five supplementary",
        ),
        (
            "programs",
            PROGRAM_COUNT,
            run.observed_programs(),
            "one semantic concern per program",
        ),
        (
            "targets",
            Target::ALL.len(),
            targets.len(),
            "x86-64 is the cross-backend baseline",
        ),
        (
            "opt_levels",
            OptLevel::ALL.len(),
            levels.len(),
            "-O0, -O1, -O2",
        ),
        (
            "bcc_cells",
            BCC_CELL_COUNT,
            run.observed_cells(),
            "program x target x optimization level",
        ),
        (
            "oracle_a_comparisons",
            ORACLE_A_COMPARISON_COUNT,
            run.oracle_count(Oracle::ReferenceCompiler),
            "reference compiler, same target and level",
        ),
        (
            "oracle_b_comparisons",
            ORACLE_B_COMPARISON_COUNT,
            run.oracle_count(Oracle::CrossBackend),
            "non-baseline target against the baseline",
        ),
        (
            "oracle_c_assertions",
            ORACLE_C_ASSERTION_COUNT,
            run.oracle_count(Oracle::GoldenRecord),
            "cell against its own recorded stdout",
        ),
        (
            "total_assertions",
            TOTAL_ASSERTION_COUNT,
            run.tally.total(),
            "sum of the three oracles",
        ),
    ];
    for (label, planned, actual, note) in matrix {
        rows.push(
            SummaryRow::new(RECORD_MATRIX)
                .set(COL_LABEL, label)
                .set(COL_COUNT, actual.to_string())
                .set(COL_REFERENCE, planned.to_string())
                .set(COL_DETAIL, note),
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
        let state = match report {
            Some(_) => "contributed",
            None if run.unusable.contains(&spec) => "unusable",
            None => "absent",
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
                        "{state}; programs observed {}",
                        report.map(|area| area.programs.len()).unwrap_or(0)
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

    for area in &run.areas {
        for row in &area.rows {
            rows.push(
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
                    .set_opt(COL_REFERENCE, row.finding_dir.as_deref().map(shown_path))
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

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(tsv_header(SUMMARY_TSV_COLUMNS));
    lines.extend(rows.iter().map(SummaryRow::render));
    join_document(&lines)
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
/// anywhere. Both files are written by rename over a temporary sibling, so a concurrently
/// finalizing summary never reads a half-written file. Only this area's own two files are touched,
/// which is why no lock is needed here or anywhere else in this module.
///
/// Call it once per area test, with every outcome that area accumulated — passes included. The
/// passes are what make the tally, the matrix and the planned-against-recorded comparison mean
/// anything; a report of the failures alone could not show that the rest of the matrix ran.
///
/// # Errors
///
/// Fails when `area` is not a known feature area, or when the report root cannot be created or
/// written. A failure here is reported rather than swallowed: a verdict that was computed and then
/// silently not recorded is worse than a loud I/O error, because the run would look clean.
pub fn write_area(area: &str, outcomes: &[Outcome], caps: &Capabilities) -> HarnessResult<()> {
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

    let mut report = AreaReport::from_outcomes(spec, outcomes);
    let facts = CorpusFacts::for_area(spec);
    report
        .diagnostics
        .extend(diagnose_unrecorded_programs(&report.rows, &facts));
    let coverage = assess_area_coverage(caps, &report);

    write_atomic(
        &format!("writing the report of feature area `{area}`"),
        &area_markdown_path(spec),
        &render_area_markdown(&report, &facts, caps, &coverage),
    )?;
    write_atomic(
        &format!("writing the machine-readable report of feature area `{area}`"),
        &area_tsv_path(spec),
        &render_area_tsv(&report),
    )?;
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

/// Read one area's machine-readable report back.
///
/// Returns `Ok(None)` when the file is not there — either the area has not run yet, or it vanished
/// between the existence check and the read, which is a benign race and never a reason to fail a
/// run. Anything else the file can do to us — unreadable bytes, a header from a different schema, a
/// row that will not parse — comes back as a diagnostic on an otherwise usable report, so a damaged
/// file degrades the summary loudly instead of aborting it.
fn read_area(spec: &'static AreaSpec) -> Option<AreaReport> {
    let path = area_tsv_path(spec);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            return Some(AreaReport::from_rows(
                spec,
                Vec::new(),
                vec![format!(
                    "⚠️ the machine-readable report of feature area `{}` could not be read, so its \
                     outcomes are missing from every total: {} ({error})",
                    spec.directory(),
                    shown_path(&path)
                )],
            ));
        }
    };

    let mut lines = contents.lines();
    let expected = tsv_header(AREA_TSV_COLUMNS);
    match lines.next() {
        Some(header) if header == expected => {}
        Some(header) => {
            return Some(AreaReport::from_rows(
                spec,
                Vec::new(),
                vec![format!(
                    "⚠️ the machine-readable report of feature area `{}` has a header this harness \
                     does not recognise, so its outcomes are missing from every total. Expected \
                     `{expected}`, found `{}`. A report left behind by an older revision of the \
                     harness will do this; re-run the area to replace it.",
                    spec.directory(),
                    sanitize_text_for_report(header)
                )],
            ));
        }
        None => {
            return Some(AreaReport::from_rows(
                spec,
                Vec::new(),
                vec![format!(
                    "⚠️ the machine-readable report of feature area `{}` is empty, so its outcomes \
                     are missing from every total.",
                    spec.directory()
                )],
            ));
        }
    }

    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        // Line one was the header, so a body line's number in the file is its offset plus two.
        match Row::parse(line, offset + 2) {
            Ok(row) => rows.push(row),
            Err(problem) => diagnostics.push(format!(
                "⚠️ a row of the machine-readable report of feature area `{}` could not be read, so \
                 that comparison is missing from every total: {problem}",
                spec.directory()
            )),
        }
    }
    Some(AreaReport::from_rows(spec, rows, diagnostics))
}

/// Write the run summary, if and only if every feature area has filed its report.
///
/// Every area test calls this at the end of its run. All but one call find at least one area still
/// missing and return `Ok(false)` having written nothing; the last one finds all
/// [`AREA_COUNT`](super::AREA_COUNT) of them, aggregates the lot and writes the summary. That is
/// the whole of the coordination: a check followed by a write, requiring no ordering between the
/// area tests, no lock, and no extra test of its own — which matters, because the suite's test
/// count is itself one of the mechanical checks that no existing test was skipped or weakened.
///
/// Two areas finishing at the same instant may both see a complete set and both write. That is
/// harmless and deliberate: they aggregate the same fourteen files, so they render identical bytes,
/// and each writes by rename over its own temporary sibling, so a reader sees one complete summary
/// or the other and never a blend of the two.
///
/// Returns whether the summary was written. A `false` is not a failure — it is the ordinary answer
/// for thirteen of the fourteen calls, and also the answer when a name filter meant some areas were
/// never going to run.
///
/// # Errors
///
/// Fails only when the summary itself cannot be written. A damaged or unreadable area file does not
/// fail the run: it becomes a diagnostic, the summary is stamped partial, and the outcomes it would
/// have contributed are reported as missing. Losing the whole summary because one file of fourteen
/// was unreadable would hide thirteen areas' worth of results to punish one.
pub fn try_finalize(caps: &Capabilities) -> HarnessResult<bool> {
    let mut run = RunReport {
        filters: libtest_name_filters(),
        ..RunReport::default()
    };

    for spec in AREAS.iter() {
        // Existence is checked by the read itself, so there is no window between a check and a use.
        let Some(report) = read_area(spec) else {
            return Ok(false);
        };
        if report.rows.is_empty() && !report.diagnostics.is_empty() {
            run.unusable.push(spec);
        }
        run.tally.merge(&report.tally);
        run.areas.push(report);
    }

    let specs: Vec<&'static AreaSpec> = run.areas.iter().map(|area| area.spec).collect();
    run.facts = CorpusFacts::for_areas(&specs);

    let rows: Vec<Row> = run
        .areas
        .iter()
        .flat_map(|area| area.rows.iter().cloned())
        .collect();
    run.diagnostics
        .extend(diagnose_unrecorded_programs(&rows, &run.facts));

    // The corpus states how many programs each area holds; a discovered count that disagrees is a
    // corpus defect, and the summary says so rather than quietly reporting the number it found.
    for spec in AREAS.iter() {
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

    let coverage = assess_run_coverage(caps, &run);
    let context = "writing the differential conformance run summary";
    write_atomic(
        context,
        &summary_markdown_path(),
        &render_summary_markdown(&run, caps, &coverage),
    )?;
    write_atomic(
        context,
        &summary_tsv_path(),
        &render_summary_tsv(&run, caps, &coverage),
    )?;
    Ok(true)
}
