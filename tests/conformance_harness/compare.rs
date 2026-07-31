//! Byte-exact comparison of two program observations, and the three oracles built on it.
//!
//! This module answers exactly one question, and deliberately nothing else: **are these two
//! observations equal, and if not, precisely where do they first differ?** It performs no
//! process execution and no filesystem access, decides no verdict, and formats no report.
//! Mapping a divergence class onto a verdict belongs to `classify.rs`; laying artifacts down
//! belongs to `findings.rs` and `report.rs`. Keeping the comparator a pure function of byte
//! slices and termination values is what makes it trivially testable, trivially parallel-safe,
//! and trivially hermetic.
//!
//! # What is compared
//!
//! **Stdout bytes and exit status. Nothing else.**
//!
//! - [`Oracle::ReferenceCompiler`] — bcc against an external reference C compiler at the same
//!   target and optimization level. Detects a wrong answer bcc produces consistently across
//!   all four of its backends, which no amount of cross-backend comparison could see.
//! - [`Oracle::CrossBackend`] — every non-baseline target against [`Target::BASELINE`] at the
//!   same optimization level. Detects a wrong answer confined to one backend: an ABI,
//!   register-allocation or instruction-selection defect.
//! - [`Oracle::GoldenRecord`] — every cell against the `expected_stdout` recorded in the
//!   program's own `.expected` file, plus its `expect_exit`. This is the only oracle that can
//!   detect **both** compilers changing behaviour in the same direction at the same time, which
//!   the reference-compiler oracle structurally reports as agreement. It also catches toolchain
//!   drift and plain regression over time, and it costs nothing, because the record has to
//!   exist anyway so that a cell is reproducible from its source file and its commands alone.
//!
//! # What is never compared, and why
//!
//! **Standard error is never compared.** Diagnostic wording legitimately differs between
//! compilers — two correct compilers can phrase the same warning in two ways, or emit a note
//! one of them does not — so comparing it would bury every real finding under a flood of
//! divergences that say nothing about code correctness. Standard error is captured into
//! finding artifacts, where it is often the fastest route to a diagnosis, and it is read there
//! by a human rather than by an oracle. [`RunOutcome::stderr`] is therefore never called from
//! this file, and no function here accepts or examines such a buffer, so the mistake cannot be
//! made by accident and its absence is auditable by search.
//!
//! # Byte-exact means byte-exact
//!
//! Compared data is `&[u8]` against `&[u8]`, and nothing is done to it: no trimming, no
//! line-ending normalization, no whitespace collapsing, no UTF-8 validation, no lossy
//! conversion, no case folding, no numeric tolerance, no "close enough" mode. Every one of
//! those would silently mask a real divergence, and this suite exists precisely to detect
//! divergences that a hand-written expectation could not have anticipated. Adding a tolerance
//! to make a comparison pass would be weakening an assertion, which the constraint set forbids
//! outright.
//!
//! Bytes are escaped for **display** by [`escape_bytes`], which is a separate step applied only
//! to rendered messages. The compared slices are never routed through it.
//!
//! # Exit status is compared as a raw wait status
//!
//! [`Termination`] keeps an ordinary exit, a signal death and a budget expiry structurally
//! distinct, and this module compares those structures rather than a single number. A crash is
//! therefore never conflated with a numerically equal ordinary exit, and a hang is never
//! reported as a fault. [`RunOutcome::raw_wait_status`] is compared as well, so two
//! terminations that look structurally alike but were reported differently by the operating
//! system are still surfaced rather than dropped.
//!
//! Expected exit codes are constrained to the range 0 to [`MAX_CONTRACT_EXIT_CODE`], because the
//! operating system truncates a larger returned value into a byte: `return 300` was measured to
//! produce wait status **44**. This module compares faithfully and **never clamps, masks or
//! reinterprets** a status — an out-of-contract value is reported as observed, because masking it
//! would destroy the evidence that something produced it.
//!
//! # When equality is claimed
//!
//! Equality is claimed only when **both** sides ran to an ordinary exit **and** their exit
//! codes, raw wait statuses and stdout bytes all agree.
//!
//! A signal death or a budget expiry on either side is reported as a divergence **even when
//! both sides agree on it**. [`Termination::divergence_class`] records the reason: a crash and
//! a timeout are divergent however they are compared. Two identical crashes are two crashes,
//! and reporting them as an agreement would let a program that never produced its output pass.
//!
//! # Class precedence
//!
//! Several facts can differ at once — a program can crash *and* have printed different bytes
//! first. Every fact that differs is named in [`Comparison::detail`], and the single value in
//! [`Comparison::class`] is the most diagnostic of them, chosen by this fixed precedence:
//!
//! 1. [`DivergenceClass::RunCrash`] — highest: a signal death explains everything downstream.
//! 2. [`DivergenceClass::Timeout`] — a program that never finished.
//! 3. [`DivergenceClass::ExitCodeMismatch`] — two completed runs that disagreed on status.
//! 4. [`DivergenceClass::StdoutMismatch`] — lowest: two completed runs that disagreed on bytes.
//!
//! Nothing is lost by the choice, because the detail carries all of it.
//!
//! # Exclusions are recorded, never silent
//!
//! A program may narrow a comparison to fewer than three oracles — a construct whose value
//! legitimately differs between architectures stays fully compared against its same-target
//! reference and its golden record while being excluded from cross-backend value equality
//! alone. That exclusion is never a silent pass: [`excluded_by_manifest`] produces a
//! [`Comparison`] that says which oracle was not attempted and carries the reason the
//! program's own record gives for it, so the set of comparisons deliberately not made is as
//! visible in the summary as the set that was. Narrowing to one oracle is permitted; dropping
//! a feature from testing is not.
//!
//! # Invariants callers may rely on
//!
//! - Only `std` is used. The project permits no third-party crate, so a snapshot-assertion or
//!   diff crate is not an option and none is needed: the golden records plus this hand-written
//!   comparator are the substitute.
//! - No compiler-unchecked code appears: the keyword that would permit reinterpreting bytes as
//!   text without validating them is absent from this file. Bytes that are not valid text are
//!   escaped by a safe routine instead, so a multi-byte sequence split across a divergence
//!   renders readably and cannot panic.
//! - Every function is pure: no clock, no environment, no randomness, no counter, no global
//!   mutable state. Identical inputs always produce byte-identical messages, including every
//!   truncation notice, so a report diffed between two runs shows only real change — and the
//!   fourteen feature-area tests may call this concurrently without a lock.
//! - Every rendered message names the cell and the oracle, so a row of the verdict table is
//!   self-sufficient.
//! - This module declares no test function of its own and no `main.rs` sits beside it, so Cargo
//!   treats the directory as a plain module directory rather than a test target and the package
//!   manifest needs no change. The suite's own test count therefore does not move, which is one
//!   of the mechanical checks that no existing test was skipped or weakened.
//!
//! Edition 2021, minimum supported Rust 1.70.

use std::fmt;

use super::execute::{RunOutcome, Termination, MAX_CONTRACT_EXIT_CODE};
use super::manifest::Manifest;
use super::{sanitize_text_for_report, CellKey, DivergenceClass, Oracle, Target};

/// Lines of context shown on each side of the divergent line.
///
/// Two is enough to orient a reader in output whose every line asserts one semantic property,
/// and small enough that a detail block stays readable in a failure message.
const CONTEXT_LINES: usize = 2;

/// Longest run of source bytes rendered from a single line before the rendering is truncated.
///
/// A corpus program prints short lines, but a defect can produce a very long one, and the
/// message must stay readable and bounded either way. Truncation is always announced, never
/// silent — see [`render_line`].
const MAX_RENDERED_LINE_BYTES: usize = 512;

/// Longest run of characters of a recorded free-text reason placed on a single-line summary.
///
/// The full text always reaches [`Comparison::detail`]; this bound applies only to the
/// one-line form, which has to fit in a report row beside the rest of the verdict.
const MAX_SUMMARY_REASON_CHARS: usize = 240;

/// Most body lines [`unified_diff`] emits before it stops and says so.
///
/// A pathological output must not be able to turn a finding artifact into a gigabyte of report
/// text. The bound is a constant rather than a function of the input so that the same inputs
/// always produce the same notice.
const MAX_DIFF_LINES: usize = 200;

/// Rendering of a byte position that lies past the end of one side's output.
const PAST_END_OF_OUTPUT: &str = "<end of output>";

/// Width of the label column shared by every row of every detail block.
///
/// Defined once and consumed only through [`row`]. A single detail block is assembled by several
/// independent functions — the block header, the divergence-class row, the two command rows, a
/// status divergence's own rows, a stdout divergence's own rows and any appended advisory — and a
/// maintainer reads the result as one table. Writing the width once is what stops those
/// contributions from drifting out of alignment with one another as any of them changes.
const LABEL_WIDTH: usize = 28;

/// Lower-case hexadecimal digits, indexed by nibble value.
///
/// Both index expressions in [`escape_bytes`] are four bits wide, so the lookup cannot be out of
/// range: the escaper needs no fallback arm and has no path on which it could panic. That matters
/// because escaping runs on the failure path of every diverging cell, where a panic would replace
/// a diagnosable divergence with a harness crash. Lower case matches the spelling the harness
/// root's report-safe rendering already uses, so one escape sequence is never written two ways.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Escape a byte string into printable ASCII that is safe to place in a report.
///
/// The rendering is unambiguous and reversible by eye:
///
/// - `\\` for a backslash, so an escape can never be confused with literal input;
/// - `\n`, `\r` and `\t` for the three whitespace controls a reader most needs to see, because
///   a difference that consists only of a trailing newline is otherwise invisible;
/// - every other byte in the printable ASCII range `0x20..=0x7e` verbatim;
/// - `\xNN` in lower-case hexadecimal for everything else — the remaining control bytes, the
///   delete byte, and every byte at or above `0x80`.
///
/// Because the output contains nothing but printable ASCII, it is **always exactly one line**
/// and is strictly safer than the character-level report rule this harness applies elsewhere:
/// no control byte, no terminal escape introducer, no carriage return and no directional
/// override can survive it, so a rendered byte can neither forge a tab-separated column nor
/// repaint a verdict in a maintainer's scrollback.
///
/// Escaping operates on bytes, so a byte sequence that is not valid text needs no validation
/// and no lossy substitution: it renders as its bytes. That is why a multi-byte character
/// split across a divergence is readable rather than a panic, and why no unchecked
/// reinterpretation of bytes as text is needed anywhere in this file.
///
/// This is a **display** routine. It is applied to messages only, never to the data a
/// comparison is performed on.
pub fn escape_bytes(bytes: &[u8]) -> String {
    // One character per byte is the exact size for output that is entirely printable, which is
    // the ordinary case for this corpus, and a lower bound otherwise. Reserving it costs one
    // allocation and removes the early regrowths that escaping a long line would otherwise force.
    let mut rendered = String::with_capacity(bytes.len());
    for byte in bytes.iter().copied() {
        match byte {
            b'\\' => rendered.push_str("\\\\"),
            b'\n' => rendered.push_str("\\n"),
            b'\r' => rendered.push_str("\\r"),
            b'\t' => rendered.push_str("\\t"),
            0x20..=0x7e => rendered.push(char::from(byte)),
            // Written a digit at a time from the table above rather than through a formatter, so
            // that escaping a long line does not allocate once per escaped byte.
            _ => {
                rendered.push('\\');
                rendered.push('x');
                rendered.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
                rendered.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
            }
        }
    }
    rendered
}

/// Escape a byte string for display, bounded to [`MAX_RENDERED_LINE_BYTES`] source bytes.
///
/// A truncated rendering always says so and always names both figures, so a reader can tell a
/// short line from a long one that was cut. The notice is a pure function of the input length,
/// so it is identical on every run.
fn render_line(bytes: &[u8]) -> String {
    if bytes.len() <= MAX_RENDERED_LINE_BYTES {
        return escape_bytes(bytes);
    }
    let mut rendered = escape_bytes(&bytes[..MAX_RENDERED_LINE_BYTES]);
    rendered.push_str(&format!(
        " [rendering truncated: showing the first {MAX_RENDERED_LINE_BYTES} of {} bytes]",
        bytes.len()
    ));
    rendered
}

/// Render one free-text reason on a single line, bounded and escaped.
///
/// The text comes from a maintainer's own expectation record, where a heredoc value carries
/// real line feeds, so it is passed through the harness's report-safe rendering before it can
/// reach a report row. Truncation is announced with both figures.
fn render_reason_inline(reason: &str) -> String {
    let safe = sanitize_text_for_report(reason);
    let characters = safe.chars().count();
    if characters <= MAX_SUMMARY_REASON_CHARS {
        return safe;
    }
    let mut shortened: String = safe.chars().take(MAX_SUMMARY_REASON_CHARS).collect();
    shortened.push_str(&format!(
        " [reason truncated: showing the first {MAX_SUMMARY_REASON_CHARS} of {characters} \
         characters; the full text is in the detail]"
    ));
    shortened
}

/// Render one free-text reason as an indented multi-line block for a detail.
///
/// Unbounded, because a detail is written into a failure message and a finding artifact rather
/// than into a report row, and the whole recorded reason is exactly what a reader of those
/// needs. Each line is escaped individually so the block cannot act, only be read.
fn render_reason_block(reason: &str) -> String {
    reason
        .split('\n')
        .map(|line| format!("      {}", sanitize_text_for_report(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One detail row: an indented, fixed-width label column and a value.
///
/// The single place a detail row is formatted, so every block lines up whichever function built
/// it. Values reaching this function are already escaped or already report-safe; the function
/// itself deliberately does no escaping, because it is also used with pre-rendered values that
/// escaping a second time would double.
///
/// An empty value yields the label and its colon alone, with no trailing space. That is the form
/// a row introducing an indented list needs, and producing it here rather than by trimming
/// afterwards keeps the rule in one place and the output byte-identical on every run.
fn row(label: &str, value: &str) -> String {
    if value.is_empty() {
        return format!("    {label:<LABEL_WIDTH$}:");
    }
    format!("    {label:<LABEL_WIDTH$}: {value}")
}

/// Split a byte string into lines, **keeping each line's terminator**.
///
/// Lines are separated by the line feed, and the terminator is retained rather than stripped
/// for one reason that matters more than tidiness: a difference consisting only of a missing or
/// extra trailing newline is the single most likely real-world mismatch in this suite, given
/// that every corpus program ends its output with one and every golden record carries it. A
/// splitter that discarded terminators would render both sides identically and hide it.
///
/// A trailing run of bytes with no terminator is its own final line, and empty input yields no
/// lines at all — which is what distinguishes "printed nothing" from "printed one empty line".
fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            lines.push(&bytes[start..=index]);
            start = index + 1;
        }
    }
    if start < bytes.len() {
        lines.push(&bytes[start..]);
    }
    lines
}

/// Where one byte offset sits in a byte string, in the terms a reader needs.
struct LineSpan {
    /// Line containing the offset, counted from one.
    line: usize,
    /// Byte within that line, counted from one.
    column: usize,
    /// Index of the line's first byte.
    start: usize,
    /// Index one past the line's last byte, **including** its terminator when it has one.
    end: usize,
}

/// Locate one byte offset within a byte string.
///
/// The offset is clamped to the length, so an offset one past the end — which is exactly what a
/// prefix relation produces — resolves to the position where the next byte would be written
/// rather than being rejected. That keeps the function total: there is no input for which it
/// has no answer, and therefore no path on which it could panic.
fn locate_line(bytes: &[u8], offset: usize) -> LineSpan {
    let clamped = offset.min(bytes.len());
    let preceding = &bytes[..clamped];
    let line = preceding.iter().filter(|byte| **byte == b'\n').count() + 1;
    let start = preceding
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |at| at + 1);
    let end = match bytes[start..].iter().position(|byte| *byte == b'\n') {
        // One past the terminator, so the rendered line shows the newline it ends with.
        Some(at) => start + at + 1,
        None => bytes.len(),
    };
    LineSpan {
        line,
        column: clamped - start + 1,
        start,
        end,
    }
}

/// Render the line of `bytes` containing `offset`, announcing when the offset lies past the end.
///
/// The "past the end" case is not cosmetic: it is how a truncated run is told apart from a run
/// that printed different bytes, and it is what makes a missing trailing newline legible.
fn render_side_line(bytes: &[u8], offset: usize) -> String {
    let span = locate_line(bytes, offset);
    let text = render_line(&bytes[span.start..span.end]);
    if offset < bytes.len() {
        return text;
    }
    if text.is_empty() {
        String::from(PAST_END_OF_OUTPUT)
    } else {
        format!("{text} {PAST_END_OF_OUTPUT}")
    }
}

/// Render one byte as its hexadecimal value and its escaped character form.
fn render_byte(byte: Option<u8>) -> String {
    match byte {
        Some(value) => format!("0x{value:02x} '{}'", escape_bytes(&[value])),
        None => String::from(PAST_END_OF_OUTPUT),
    }
}

/// Render a bounded window of lines around `line`, each numbered and the focus line marked.
///
/// Indented to sit under a labelled detail row, and bounded by [`CONTEXT_LINES`] on each side,
/// so the window is the same size for a two-line output and a two-thousand-line one.
fn render_context(bytes: &[u8], line: usize) -> String {
    let lines = split_lines(bytes);
    if lines.is_empty() {
        return String::from("        (this side printed nothing at all)");
    }
    // `line` can name the position just past a final newline, which is one past the last real
    // line; clamping keeps the window inside the output that exists.
    let focus = line.saturating_sub(1).min(lines.len() - 1);
    let first = focus.saturating_sub(CONTEXT_LINES);
    let last = (focus + CONTEXT_LINES).min(lines.len() - 1);
    lines
        .iter()
        .enumerate()
        .skip(first)
        .take(last + 1 - first)
        .map(|(index, text)| {
            let marker = if index == focus { '>' } else { ' ' };
            let number = index + 1;
            format!("        {marker} {number:>5} | {}", render_line(text))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Which side of a comparison is a strict prefix of the other.
///
/// Only ever set when the two outputs agree on every byte they share and then one of them
/// stops, which is a materially different failure from disagreeing mid-line: a truncation
/// usually means the run died part-way through printing, and an extension usually means it
/// printed more than it was asked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PrefixRelation {
    /// The actual output is a strict prefix of the expected output: it stopped early.
    ActualTruncated,
    /// The expected output is a strict prefix of the actual output: it kept going.
    ActualExtended,
}

impl PrefixRelation {
    /// The token used in a report row.
    pub fn label(self) -> &'static str {
        match self {
            PrefixRelation::ActualTruncated => "actual_truncated",
            PrefixRelation::ActualExtended => "actual_extended",
        }
    }

    /// What the relation tells a reader about the run, spelled out.
    pub fn explanation(self) -> &'static str {
        match self {
            PrefixRelation::ActualTruncated => {
                "the actual output is a strict prefix of the expected output, so the run stopped \
                 before printing everything it was expected to — most often a crash or an abort \
                 part-way through the output"
            }
            PrefixRelation::ActualExtended => {
                "the expected output is a strict prefix of the actual output, so the run printed \
                 everything expected and then continued past it"
            }
        }
    }
}

impl fmt::Display for PrefixRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Exactly where two stdout byte streams first differ, and what that neighbourhood looks like.
///
/// A divergence that cannot be located cannot be minimized, and minimizing a reproducer is what
/// turns an observation into a finding a maintainer can act on. Each corpus program prints one
/// line per semantic property it claims, so a located line number points at a single construct
/// rather than at "somewhere in the program" — this type is what turns that property into an
/// actionable message.
///
/// # Why the fields are public here
///
/// Elsewhere in this harness a type's fields are private, because those types carry *observed
/// evidence* — the bytes a program wrote, the status the operating system reported — and a
/// writable field would let something between the program and the comparison adjust what the
/// program did. Nothing of that kind is stored here. Every field is a pure, recomputable
/// function of the two slices [`locate_stdout_divergence`] was handed, so a caller that altered
/// one would only be describing a divergence wrongly to itself, and could not affect any
/// comparison, verdict or artifact. Public fields let `findings.rs` reach for the offset and the
/// line directly, which is exactly what it needs to highlight them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdoutDivergence {
    /// Offset of the first differing byte, counted from zero.
    ///
    /// When one output is a strict prefix of the other, this is the length of the shorter one:
    /// the position at which it stopped agreeing by ending.
    pub offset: usize,
    /// Line containing [`StdoutDivergence::offset`], counted from one.
    pub line: usize,
    /// Byte within that line, counted from one.
    pub column: usize,
    /// Total length of the expected output in bytes.
    pub expected_len: usize,
    /// Total length of the actual output in bytes.
    pub actual_len: usize,
    /// Set when one output is a strict prefix of the other, and `None` when they disagree on a
    /// byte they both have.
    pub prefix: Option<PrefixRelation>,
    /// The expected byte at the offset, or `None` when the expected output ends before it.
    pub expected_byte: Option<u8>,
    /// The actual byte at the offset, or `None` when the actual output ends before it.
    pub actual_byte: Option<u8>,
    /// The whole expected line containing the offset, escaped, including its terminator.
    pub expected_line: String,
    /// The whole actual line containing the offset, escaped, including its terminator.
    pub actual_line: String,
    /// A bounded, numbered window of expected lines around the divergence.
    pub expected_context: String,
    /// A bounded, numbered window of actual lines around the divergence.
    pub actual_context: String,
}

impl StdoutDivergence {
    /// A one-line statement of where the two streams first differ.
    ///
    /// Contains nothing but printable ASCII, because every quoted byte passes through
    /// [`escape_bytes`], so it is safe to place in a tab-separated report row without further
    /// treatment and cannot render as more than one line.
    pub fn summary(&self) -> String {
        let mut summary = format!(
            "first difference at byte offset {} (line {}, column {}): expected {}, actual {}; \
             expected {} bytes, actual {} bytes",
            self.offset,
            self.line,
            self.column,
            render_byte(self.expected_byte),
            render_byte(self.actual_byte),
            self.expected_len,
            self.actual_len
        );
        if let Some(prefix) = self.prefix {
            summary.push_str(&format!("; {prefix}"));
        }
        summary
    }
}

impl fmt::Display for StdoutDivergence {
    /// The multi-line block a failure message and a finding artifact carry.
    ///
    /// Deliberately more than one line: this is the form a human reads, and the line renderings
    /// and context windows are what make a divergence diagnosable without re-running it. The
    /// one-line form for a report row is [`StdoutDivergence::summary`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}",
            row(
                "first divergent byte offset",
                &format!("{} (0-based)", self.offset)
            )
        )?;
        writeln!(
            f,
            "{}",
            row(
                "location",
                &format!("line {}, column {} (both 1-based)", self.line, self.column)
            )
        )?;
        writeln!(
            f,
            "{}",
            row("expected byte", &render_byte(self.expected_byte))
        )?;
        writeln!(f, "{}", row("actual byte", &render_byte(self.actual_byte)))?;
        writeln!(
            f,
            "{}",
            row("expected length", &format!("{} bytes", self.expected_len))
        )?;
        writeln!(
            f,
            "{}",
            row("actual length", &format!("{} bytes", self.actual_len))
        )?;
        writeln!(
            f,
            "{}",
            row(
                "prefix relation",
                match self.prefix {
                    Some(prefix) => prefix.explanation(),
                    None =>
                        "neither output is a prefix of the other: they disagree on a byte both of \
                         them have",
                }
            )
        )?;
        writeln!(
            f,
            "{}",
            row(&format!("expected line {}", self.line), &self.expected_line)
        )?;
        writeln!(
            f,
            "{}",
            row(&format!("actual line {}", self.line), &self.actual_line)
        )?;
        writeln!(f, "{}", row("expected context", ""))?;
        writeln!(f, "{}", self.expected_context)?;
        writeln!(f, "{}", row("actual context", ""))?;
        write!(f, "{}", self.actual_context)
    }
}

/// Compare two stdout byte streams and locate the first byte at which they differ.
///
/// Returns `None` when the two streams are **byte-identical**, which is the only condition
/// under which this suite considers stdout to agree. Nothing is trimmed, normalized, folded or
/// validated on the way in: a stream that differs from another by a single trailing newline
/// differs, and this function says so.
///
/// `expected` is the authority — the reference compiler's output, the baseline target's output,
/// or the golden record — and `actual` is what is being judged against it. The distinction
/// affects only how the result is worded and which side [`PrefixRelation`] describes; the
/// equality test itself is symmetric.
pub fn locate_stdout_divergence(expected: &[u8], actual: &[u8]) -> Option<StdoutDivergence> {
    let shared = expected.len().min(actual.len());
    // The first position at which the shared prefix disagrees, or the end of that prefix when it
    // agrees throughout. `position` returns the first match, which is precisely "first
    // divergent byte"; `unwrap_or` supplies the total answer for the agreeing case rather than
    // asserting one.
    let offset = expected
        .iter()
        .zip(actual.iter())
        .position(|(left, right)| left != right)
        .unwrap_or(shared);

    let prefix = if offset < shared {
        // They disagree on a byte both sides have, so neither is a prefix of the other.
        None
    } else if actual.len() < expected.len() {
        Some(PrefixRelation::ActualTruncated)
    } else if expected.len() < actual.len() {
        Some(PrefixRelation::ActualExtended)
    } else {
        // Equal lengths and no disagreement anywhere: the streams are identical.
        return None;
    };

    // Only the longer side is guaranteed to have a byte at the offset when one output is a
    // strict prefix of the other, and the two sides agree on every byte before the offset, so
    // the longer side always yields the correct line and column.
    let locator: &[u8] = if expected.len() >= actual.len() {
        expected
    } else {
        actual
    };
    let span = locate_line(locator, offset);

    Some(StdoutDivergence {
        offset,
        line: span.line,
        column: span.column,
        expected_len: expected.len(),
        actual_len: actual.len(),
        prefix,
        expected_byte: expected.get(offset).copied(),
        actual_byte: actual.get(offset).copied(),
        expected_line: render_side_line(expected, offset),
        actual_line: render_side_line(actual, offset),
        expected_context: render_context(expected, span.line),
        actual_context: render_context(actual, span.line),
    })
}

/// Common names for the signals a compiled corpus program can realistically die on.
///
/// Display only, and always alongside the number rather than instead of it. The numbering is
/// the generic Linux one, which all four supported targets share, and it is the *host* kernel
/// that reports a wait status even when the program itself ran under an emulator. A signal
/// outside this table renders as its number alone, which is complete if less immediate.
const SIGNAL_NAMES: &[(i32, &str)] = &[
    (1, "SIGHUP"),
    (2, "SIGINT"),
    (3, "SIGQUIT"),
    (4, "SIGILL"),
    (5, "SIGTRAP"),
    (6, "SIGABRT"),
    (7, "SIGBUS"),
    (8, "SIGFPE"),
    (9, "SIGKILL"),
    (11, "SIGSEGV"),
    (13, "SIGPIPE"),
    (14, "SIGALRM"),
    (15, "SIGTERM"),
    (24, "SIGXCPU"),
    (25, "SIGXFSZ"),
    (31, "SIGSYS"),
];

/// Render a termination, naming the signal when there is one.
///
/// A crash is far more legible as `terminated by signal 11 (SIGSEGV)` than as a bare number:
/// a segmentation fault, an abort and a division fault call for entirely different
/// investigations, and the name is what tells them apart at a glance.
fn render_termination(termination: Termination) -> String {
    match termination.signal() {
        Some(signal) => match SIGNAL_NAMES
            .iter()
            .find(|(number, _)| *number == signal)
            .map(|(_, name)| *name)
        {
            Some(name) => format!("{termination} ({name})"),
            None => termination.to_string(),
        },
        None => termination.to_string(),
    }
}

/// Render one side of a status comparison: who it is, how it ended, and its raw wait status.
///
/// A side with no raw wait status is a recorded expectation rather than an execution, which is
/// stated rather than left as a blank, so a reader is never left wondering whether a status was
/// missing or merely unprinted.
fn render_side_status(label: &str, termination: Termination, raw: Option<i32>) -> String {
    let safe = sanitize_text_for_report(label);
    let rendered = render_termination(termination);
    match raw {
        Some(status) => format!("{safe} {rendered} (raw wait status {status})"),
        None => format!("{safe} {rendered} (a recorded expectation, which carries no wait status)"),
    }
}

/// How two exit statuses differed, and which of the divergence classes that difference is.
///
/// The fields are public for the same reason [`StdoutDivergence`]'s are: every one of them is a
/// pure function of the arguments [`locate_status_divergence`] was handed, so none of them is
/// evidence that could be tampered with — the evidence itself stays behind
/// [`RunOutcome`]'s accessors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusDivergence {
    /// The most diagnostic class of this difference, by the precedence this module documents.
    pub class: DivergenceClass,
    /// How the authoritative side ended.
    pub expected: Termination,
    /// How the side under judgement ended.
    pub actual: Termination,
    /// A one-line statement naming both sides, how each ended, and each raw wait status.
    pub summary: String,
    /// Every fact that contributed, one per entry, each already safe for a single line.
    ///
    /// More than one entry is normal: a signal death is also an absence of an exit code, and a
    /// reader needs both stated rather than having to infer the second from the first.
    pub facts: Vec<String>,
}

impl fmt::Display for StatusDivergence {
    /// The multi-line block a failure message and a finding artifact carry.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", row("exit status class", &self.class.to_string()))?;
        writeln!(f, "{}", row("exit status summary", &self.summary))?;
        write!(f, "{}", row("contributing facts", ""))?;
        for fact in &self.facts {
            write!(f, "\n        - {fact}")?;
        }
        Ok(())
    }
}

/// Compare two exit statuses as **raw wait statuses**, keeping an exit, a crash and a hang
/// distinct.
///
/// Returns `None` only when both sides ran to an ordinary exit, returned the same code, and —
/// where both sides have one — reported the same raw wait status.
///
/// A [`Termination::Signalled`] or [`Termination::TimedOut`] on either side is a divergence
/// **even when both sides agree on it**. That is deliberate and follows
/// [`Termination::divergence_class`]: a crash and a timeout are divergent however they are
/// compared. Two identical crashes are two crashes, and calling them an agreement would let a
/// program that never produced its output pass — which is exactly the softening this suite may
/// not do.
///
/// `expected_raw` and `actual_raw` are `None` for a side that is a recorded expectation rather
/// than an execution, which is the case for the golden-record oracle's authoritative side.
/// Where both are present they are compared, so two terminations that look structurally alike
/// but were reported differently by the operating system are surfaced rather than dropped.
pub fn locate_status_divergence(
    expected_label: &str,
    expected: Termination,
    expected_raw: Option<i32>,
    actual_label: &str,
    actual: Termination,
    actual_raw: Option<i32>,
) -> Option<StatusDivergence> {
    let mut facts: Vec<String> = Vec::new();
    let mut class: Option<DivergenceClass> = None;

    // Precedence is applied by testing the classes in order and keeping the first that fires;
    // every fact is collected regardless, so nothing the reader needs is discarded by the choice.
    if let Some(signal) = expected.signal() {
        class = Some(DivergenceClass::RunCrash);
        facts.push(format!(
            "{} was terminated by signal {signal}, so it never returned an exit code",
            sanitize_text_for_report(expected_label)
        ));
    }
    if let Some(signal) = actual.signal() {
        class = Some(DivergenceClass::RunCrash);
        facts.push(format!(
            "{} was terminated by signal {signal}, so it never returned an exit code",
            sanitize_text_for_report(actual_label)
        ));
    }
    if expected.timed_out() {
        class = class.or(Some(DivergenceClass::Timeout));
        facts.push(format!(
            "{} exceeded its execution budget and was terminated",
            sanitize_text_for_report(expected_label)
        ));
    }
    if actual.timed_out() {
        class = class.or(Some(DivergenceClass::Timeout));
        facts.push(format!(
            "{} exceeded its execution budget and was terminated",
            sanitize_text_for_report(actual_label)
        ));
    }
    if let (Some(wanted), Some(got)) = (expected.exit_code(), actual.exit_code()) {
        if wanted != got {
            class = class.or(Some(DivergenceClass::ExitCodeMismatch));
            facts.push(format!(
                "exit code {wanted} was expected, {got} was observed"
            ));
        }
    }
    if let (Some(wanted), Some(got)) = (expected_raw, actual_raw) {
        if wanted != got {
            class = class.or(Some(DivergenceClass::ExitCodeMismatch));
            facts.push(format!(
                "raw wait status {wanted} was expected, {got} was observed; the comparison is on \
                 the raw status precisely so a difference here cannot be hidden by two \
                 terminations that summarize alike"
            ));
        }
    }

    let class = class?;
    let summary = format!(
        "exit status differs: {}; {}",
        render_side_status(expected_label, expected, expected_raw),
        render_side_status(actual_label, actual, actual_raw)
    );
    Some(StatusDivergence {
        class,
        expected,
        actual,
        summary,
        facts,
    })
}

/// The result of putting one oracle's question to one cell.
///
/// # The three states, and the invariants that keep them apart
///
/// | State | `equal` | `class` | `excluded` |
/// |---|---|---|---|
/// | The two observations agree | `true` | `None` | `None` |
/// | They diverge | `false` | `Some(class)` | `None` |
/// | The comparison was not attempted | `false` | `None` | `Some(reason)` |
///
/// Two consequences are worth stating outright, because they are what stop a report from lying:
///
/// - `class.is_some()` holds **exactly** when a divergence was observed. A consumer that
///   branches on the class therefore cannot mistake an exclusion for a divergence, or the
///   reverse.
/// - An excluded comparison reports `equal == false`. Nothing was compared, so equality was
///   never established, and claiming it would be precisely the silent pass the requirements
///   forbid. [`Comparison::excluded`] carries the recorded reason so a summary can say *why* the
///   comparison was not made, which is what keeps a narrowing visible instead of invisible.
///
/// # Why these fields are public
///
/// A [`Comparison`] is a *judgement recomputable at will* from the observations it was derived
/// from, not the observations themselves. The evidence — the bytes a program wrote and the
/// status the operating system reported — stays behind [`RunOutcome`]'s accessors, where a
/// writable field really would let something adjust what a program did after the fact. Nothing
/// of that sort is reachable from here, so exposing all six judgement fields costs no guarantee
/// and lets the driver, the classifier and the reporter each read what they need without a layer
/// of ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// True only when the two observations agree exactly: identical stdout bytes, and both sides
    /// exited ordinarily with the same code and the same raw wait status.
    pub equal: bool,
    /// The most diagnostic class of the divergence, and `None` when there was none.
    pub class: Option<DivergenceClass>,
    /// A single line naming the oracle, the cell, the class and where the difference begins.
    ///
    /// Guaranteed to be exactly one line: every byte quoted from a program's output is escaped
    /// into printable ASCII, and every piece of free text is passed through the harness's
    /// report-safe rendering, so nothing here can forge a column or split a row.
    pub summary: String,
    /// The multi-line expansion a failure message and a finding artifact carry: the located
    /// offset, line and column, both renderings, both context windows, every status fact, and
    /// the command line of each side.
    ///
    /// Deliberately permitted to span lines, because this is the form a human reads. Feeding it
    /// to the harness's outcome type escapes the line feeds into `\x0a`, which keeps a verdict
    /// row to one line without losing anything a reader needs.
    pub detail: String,
    /// Which oracle asked the question, so an outcome is always attributed correctly.
    pub oracle: Oracle,
    /// The recorded reason this comparison was not attempted, and `None` when it was.
    pub excluded: Option<String>,
}

impl Comparison {
    /// Whether the comparison was actually performed.
    ///
    /// False only for a comparison a program's own record excluded. An unattempted comparison is
    /// neither a pass nor a failure, and this is the predicate that says so without a consumer
    /// having to reason about the field table above.
    pub fn attempted(&self) -> bool {
        self.excluded.is_none()
    }

    /// Whether a divergence was observed, which is exactly `class.is_some()`.
    pub fn is_divergence(&self) -> bool {
        self.class.is_some()
    }

    /// Append an advisory to both renderings.
    ///
    /// Advisories are facts a reader must be told that are not themselves divergences — a
    /// degenerate comparison, or an exit code outside the corpus's contract. They are appended
    /// rather than substituted, so nothing already recorded is displaced, and they are escaped
    /// so the single-line guarantee on the summary survives.
    fn push_note(&mut self, note: &str) {
        let safe = sanitize_text_for_report(note);
        self.summary.push_str(&format!("; note: {safe}"));
        self.detail.push('\n');
        self.detail.push_str(&row("note", &safe));
    }
}

impl fmt::Display for Comparison {
    /// Renders [`Comparison::summary`], the one-line form, so a diagnostic and a report row read
    /// the same.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary)
    }
}

/// The fixed header every detail block opens with, naming the oracle, the cell and the contract.
///
/// Stating what was compared on every block is not padding: the most common misreading of a
/// differential result is to assume a diagnostic difference counted, and the block that carries
/// the verdict is the right place to say that it did not.
fn detail_header(oracle: Oracle, key: &CellKey) -> String {
    let compared = row(
        "compared",
        "stdout bytes and exit status only (standard error is captured for artifacts and never \
         compared)",
    );
    format!("{oracle} for {key}\n{compared}")
}

/// Build the agreement result for one oracle and cell.
///
/// `confirmation` states what agreed, in one clause, so a passing row still says something a
/// reader can check rather than merely asserting success.
fn agreed(oracle: Oracle, key: &CellKey, confirmation: &str, context: &[String]) -> Comparison {
    let mut detail = detail_header(oracle, key);
    for entry in context {
        detail.push('\n');
        detail.push_str(entry);
    }
    detail.push('\n');
    detail.push_str(&row("outcome", confirmation));
    Comparison {
        equal: true,
        class: None,
        summary: format!("{oracle} {key}: agrees — {confirmation}"),
        detail,
        oracle,
        excluded: None,
    }
}

/// Build the divergence result for one oracle and cell from whichever differences were found.
///
/// At least one of `status` and `stdout` must be present; when both are, **both are reported in
/// the detail** and the class is the more diagnostic one. That choice needs no arbitration here:
/// [`StatusDivergence::class`] is always [`DivergenceClass::RunCrash`],
/// [`DivergenceClass::Timeout`] or [`DivergenceClass::ExitCodeMismatch`], every one of which
/// outranks [`DivergenceClass::StdoutMismatch`] in this module's documented precedence, so
/// preferring the status class whenever there is one *is* the precedence.
///
/// When neither is present the result is a divergence of class
/// [`DivergenceClass::StdoutMismatch`] with a detail that says the caller asked for a divergence
/// without supplying one. That case is unreachable from this module — every call site tests both
/// options first — and it is rendered rather than ignored because a comparator that quietly
/// returned "no difference" for an unexplained call would be the one bug in this file capable of
/// turning a real divergence into a pass.
fn diverged(
    oracle: Oracle,
    key: &CellKey,
    status: Option<&StatusDivergence>,
    stdout: Option<&StdoutDivergence>,
    context: &[String],
) -> Comparison {
    let class = match status {
        Some(status) => status.class,
        None => DivergenceClass::StdoutMismatch,
    };

    let mut clauses: Vec<String> = Vec::new();
    if let Some(status) = status {
        clauses.push(status.summary.clone());
    }
    if let Some(stdout) = stdout {
        clauses.push(format!("stdout differs: {}", stdout.summary()));
    }
    if clauses.is_empty() {
        clauses.push(String::from(
            "a divergence was recorded without either a status difference or a stdout difference, \
             which is an internal inconsistency in the suite rather than an observation about a \
             compiler",
        ));
    }

    let mut detail = detail_header(oracle, key);
    detail.push('\n');
    detail.push_str(&row("divergence class", &class.to_string()));
    for entry in context {
        detail.push('\n');
        detail.push_str(entry);
    }
    if let Some(status) = status {
        detail.push('\n');
        detail.push_str(&status.to_string());
    }
    if let Some(stdout) = stdout {
        detail.push('\n');
        detail.push_str(&row("stdout divergence", ""));
        detail.push('\n');
        detail.push_str(&stdout.to_string());
    }

    Comparison {
        equal: false,
        class: Some(class),
        summary: format!("{oracle} {key}: {class} — {}", clauses.join("; and ")),
        detail,
        oracle,
        excluded: None,
    }
}

/// Append an advisory to `comparison` for a side whose exit code lies outside the corpus's
/// contract.
///
/// The contract is 0 to [`MAX_CONTRACT_EXIT_CODE`], because the operating system truncates a
/// larger returned value into a byte — `return 300` was measured to produce wait status 44. An
/// out-of-contract code is therefore evidence that a program returned something the platform could
/// not deliver intact, which changes how a reader should interpret the comparison. It is reported
/// and never repaired: this module does not clamp, mask or reinterpret a status.
fn note_exit_code_contract(comparison: &mut Comparison, label: &str, outcome: &RunOutcome) {
    let termination = outcome.termination();
    if let Some(code) = termination.exit_code() {
        if !termination.within_exit_code_contract() {
            comparison.push_note(&format!(
                "{label} returned exit code {code}, which lies outside the corpus's 0 to \
                 {MAX_CONTRACT_EXIT_CODE} contract; the operating system truncates a returned \
                 value into a byte, so this status did not survive intact and is reported exactly \
                 as observed rather than reinterpreted"
            ));
        }
    }
}

/// Compare two executions of the same program on stdout bytes and exit status.
///
/// The shared engine behind the reference-compiler and cross-backend oracles, which differ only
/// in which execution is the authority and how the two sides are named.
fn compare_executions(
    oracle: Oracle,
    key: &CellKey,
    expected_label: &str,
    expected: &RunOutcome,
    actual_label: &str,
    actual: &RunOutcome,
) -> Comparison {
    let status = locate_status_divergence(
        expected_label,
        expected.termination(),
        Some(expected.raw_wait_status()),
        actual_label,
        actual.termination(),
        Some(actual.raw_wait_status()),
    );
    let stdout = locate_stdout_divergence(expected.stdout(), actual.stdout());

    let mut comparison = if status.is_none() && stdout.is_none() {
        let context = vec![
            row(
                "expected side",
                &format!(
                    "{}, {}, {} stdout bytes",
                    sanitize_text_for_report(expected_label),
                    render_termination(expected.termination()),
                    expected.stdout().len()
                ),
            ),
            row(
                "actual side",
                &format!(
                    "{}, {}, {} stdout bytes",
                    sanitize_text_for_report(actual_label),
                    render_termination(actual.termination()),
                    actual.stdout().len()
                ),
            ),
        ];
        agreed(
            oracle,
            key,
            &format!(
                "stdout is byte-identical ({} bytes) and both sides {} with raw wait status {}",
                actual.stdout().len(),
                render_termination(actual.termination()),
                actual.raw_wait_status()
            ),
            &context,
        )
    } else {
        // A divergence gets the full execution record of both sides, because the command line it
        // carries is what lets a reader reproduce the cell without the harness.
        let context = vec![
            row("expected side", &sanitize_text_for_report(expected_label)),
            row("expected execution", &expected.describe()),
            row("actual side", &sanitize_text_for_report(actual_label)),
            row("actual execution", &actual.describe()),
        ];
        diverged(oracle, key, status.as_ref(), stdout.as_ref(), &context)
    };

    note_exit_code_contract(&mut comparison, expected_label, expected);
    note_exit_code_contract(&mut comparison, actual_label, actual);
    comparison
}

/// Oracle (a): compare bcc against an external reference C compiler.
///
/// Both artifacts were built from the same source for the same target at the same optimization
/// level, and both were executed the same way, so the only variable left is which compiler
/// produced the code. Any divergence is a compiler defect in one of the two.
///
/// The reference compiler is the authority, so it is the expected side and bcc is the side under
/// judgement: a [`PrefixRelation::ActualTruncated`] means **bcc's** binary stopped printing early.
///
/// This oracle detects what cross-backend comparison structurally cannot — a wrong answer bcc
/// produces consistently on all four of its backends, where every backend agrees with every
/// other and all of them are wrong together.
pub fn oracle_a(bcc: &RunOutcome, reference: &RunOutcome, key: &CellKey) -> Comparison {
    compare_executions(
        Oracle::ReferenceCompiler,
        key,
        "the reference compiler",
        reference,
        "bcc",
        bcc,
    )
}

/// Oracle (b): compare one target's backend against the [`Target::BASELINE`] backend.
///
/// Both artifacts came from bcc, from the same source, at the same optimization level, so the
/// only variable is the code generator. A divergence that is not attributable to a documented
/// implementation-defined difference is a defect in one backend — which is where an ABI,
/// register-allocation or instruction-selection fault surfaces, and why the register-exhausting
/// and by-value-aggregate programs are the highest-yield members of the corpus.
///
/// The baseline is the authority and the candidate is the side under judgement. x86-64 is the
/// baseline because it is the intended host, so a baseline cell runs with no emulator and
/// therefore introduces no emulation-related variable into the comparison.
///
/// The driver is responsible for invoking this only for a non-baseline target. Invoked for the
/// baseline it still compares faithfully — the baseline against itself — and says so in an
/// advisory, because a degenerate comparison that reported nothing unusual would be a quiet way
/// to lose a third of this oracle's coverage.
pub fn oracle_b(candidate: &RunOutcome, baseline: &RunOutcome, key: &CellKey) -> Comparison {
    let baseline_label = format!("the {} baseline", Target::BASELINE.triple());
    let candidate_label = format!("the {} candidate", key.target().triple());
    let mut comparison = compare_executions(
        Oracle::CrossBackend,
        key,
        &baseline_label,
        baseline,
        &candidate_label,
        candidate,
    );
    if key.target() == Target::BASELINE {
        comparison.push_note(
            "this cell's target IS the cross-backend baseline, so the comparison is the baseline \
             against itself and can only ever agree; oracle (b) is meaningful for the three \
             non-baseline targets, and a run that reports this note is not exercising it",
        );
    }
    comparison
}

/// Oracle (c): compare a cell against the golden record in the program's own expectation record.
///
/// The authority here is a committed file rather than a second compiler, which is exactly why
/// this oracle exists. If bcc and the reference compiler both change behaviour in the same
/// direction at the same time — a shared misreading of the standard, or a toolchain upgrade that
/// moves both — oracle (a) reports agreement and sees nothing. The recorded bytes do not move,
/// so this oracle does. It also catches plain regression over time, and it costs nothing: the
/// record must exist anyway for a cell to be reproducible from its source file and its commands
/// alone.
///
/// Both halves of the record are used: `expected_stdout` byte for byte — including its trailing
/// newline, which the record format guarantees by appending one to every heredoc body line — and
/// `expect_exit`, compared as an ordinary exit. The record is not an execution, so it carries no
/// raw wait status and none is invented for it.
///
/// When the record does not govern the cell it was handed, the mismatch is reported as an
/// advisory. That is a suite defect rather than a compiler defect — it would mean a program was
/// being judged against another program's golden output — and it has to be visible, because the
/// comparison it produces is meaningless either way.
pub fn oracle_c(actual: &RunOutcome, manifest: &Manifest, key: &CellKey) -> Comparison {
    let oracle = Oracle::GoldenRecord;
    let expected_label = "the recorded golden record";
    let actual_label = "this cell's execution";
    let expected_termination = Termination::Exited(manifest.expect_exit());

    let status = locate_status_divergence(
        expected_label,
        expected_termination,
        None,
        actual_label,
        actual.termination(),
        Some(actual.raw_wait_status()),
    );
    let stdout = locate_stdout_divergence(manifest.expected_stdout_bytes(), actual.stdout());

    let record_row = row(
        "golden record",
        &format!(
            "{} ({} expected stdout bytes, expected exit code {})",
            manifest.path().display(),
            manifest.expected_stdout_bytes().len(),
            manifest.expect_exit()
        ),
    );

    let mut comparison = if status.is_none() && stdout.is_none() {
        let context = vec![
            record_row,
            row(
                "actual side",
                &format!(
                    "{}, {} stdout bytes",
                    render_termination(actual.termination()),
                    actual.stdout().len()
                ),
            ),
        ];
        agreed(
            oracle,
            key,
            &format!(
                "stdout matches the recorded golden output byte for byte ({} bytes) and the exit \
                 code is the recorded {}",
                actual.stdout().len(),
                manifest.expect_exit()
            ),
            &context,
        )
    } else {
        let context = vec![record_row, row("actual execution", &actual.describe())];
        diverged(oracle, key, status.as_ref(), stdout.as_ref(), &context)
    };

    if manifest.area() != key.area() || manifest.program() != key.program() {
        comparison.push_note(&format!(
            "the expectation record supplied governs {}/{} while this cell is {}/{}, so the golden \
             output being compared belongs to a different program; this is a defect in the caller \
             rather than an observation about a compiler",
            sanitize_text_for_report(manifest.area()),
            sanitize_text_for_report(manifest.program()),
            sanitize_text_for_report(key.area()),
            sanitize_text_for_report(key.program())
        ));
    }
    note_exit_code_contract(&mut comparison, actual_label, actual);
    comparison
}

/// Record that one oracle was not attempted for a cell, because the program's own expectation
/// record narrows its coverage to the other two.
///
/// This is how a genuinely incomparable property stays under test instead of being dropped. The
/// worked example is `long double`, whose representation was measured to differ across the four
/// targets — 16, 12, 16 and 16 bytes, x87 80-bit against IEEE binary128 — so cross-backend value
/// equality is meaningless for it. The program still compiles, still runs and is still compared
/// against its same-target reference compiler and its own golden record; only oracle (b)'s value
/// equality is excluded, and the measured reason is recorded in the program's record. Narrowing
/// to one oracle is permitted; dropping a feature because it is difficult is not.
///
/// The result is never a pass. [`Comparison::equal`] is false because nothing was compared, and
/// [`Comparison::class`] is `None` because nothing diverged, so no consumer can read an exclusion
/// as either. [`Comparison::excluded`] carries the record's own recorded reason verbatim, so a
/// summary can print *why* — which is what makes the set of comparisons deliberately not made as
/// visible as the set that was.
///
/// Two inconsistencies are reported rather than smoothed over:
///
/// - The record enables the oracle after all, meaning the caller excluded a comparison the corpus
///   asks for. That silently removes a comparison from the matrix, so it is named.
/// - The record narrows coverage without recording a reason. The record parser is meant to make
///   that impossible, so reaching it means the reason was lost between the parser and here — and
///   an unexplained exclusion is precisely the silent exclusion the requirements forbid.
pub fn excluded_by_manifest(oracle: Oracle, manifest: &Manifest, key: &CellKey) -> Comparison {
    let recorded = manifest.impl_defined_notes();
    let reason = match recorded {
        Some(text) => String::from(text),
        None => String::from(
            "this program's expectation record disables the oracle but records no reason for the \
             narrowing; a narrowing without a recorded reason is a defect in the test material, \
             because an exclusion nobody can justify is indistinguishable from a feature quietly \
             dropped from testing",
        ),
    };

    let mut detail = detail_header(oracle, key);
    detail.push_str(&format!(
        "\n{}",
        row(
            "comparison",
            "not attempted: excluded by this program's own expectation record"
        )
    ));
    detail.push_str(&format!(
        "\n{}",
        row("expectation record", &manifest.path().display().to_string())
    ));
    detail.push_str(&format!(
        "\n{}",
        row(
            "oracles still compared",
            &manifest
                .enabled_oracles()
                .iter()
                .map(|enabled| String::from(enabled.label()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    ));
    detail.push_str(&format!("\n{}", row("recorded reason", "")));
    detail.push('\n');
    detail.push_str(&render_reason_block(&reason));

    let mut comparison = Comparison {
        equal: false,
        class: None,
        summary: format!(
            "{oracle} {key}: not attempted — excluded by the program's own expectation record; \
             recorded reason: {}",
            render_reason_inline(&reason)
        ),
        detail,
        oracle,
        excluded: Some(reason),
    };

    if manifest.oracle_enabled(oracle) {
        comparison.push_note(
            "the expectation record ENABLES this oracle, so recording an exclusion for it removes \
             a comparison the corpus asks for; this is a defect in the caller rather than a \
             narrowing the corpus declared",
        );
    }
    if recorded.is_none() {
        comparison.push_note(
            "no recorded reason accompanied this narrowing, which the record parser is meant to \
             make impossible",
        );
    }
    comparison
}

/// A bounded, dependency-free line diff of two stdout streams, for a finding's `diff.txt`.
///
/// Deliberately **not** a minimal-edit diff. A minimal-edit algorithm would align lines across
/// an insertion, which reads well for source control and badly here: every corpus program prints
/// a fixed sequence of lines, one per semantic property it claims, so line *N* of one run and
/// line *N* of the other are claims about the same property and belong on the same row. An
/// alignment that slid them apart would obscure exactly the correspondence a reader needs. The
/// comparison is therefore positional and honest about being so.
///
/// The header names both lengths and line counts and highlights the first divergence by byte
/// offset, line and column; the divergent line is marked with `*` in the left margin. Rows use
/// `-` for the expected side, `+` for the actual side, and a blank marker for a line the two
/// share. Line terminators are shown, so a difference consisting only of a missing trailing
/// newline is visible rather than invisible.
///
/// Output is bounded by [`MAX_DIFF_LINES`] rendered rows, and each row by
/// [`MAX_RENDERED_LINE_BYTES`] source bytes. Both bounds announce themselves when they bite, so a
/// pathological output cannot quietly turn a finding artifact into a gigabyte of report text and
/// cannot quietly appear complete when it is not. Every bound is a constant, so the same inputs
/// always produce the same text — a diff that is committed as a deliverable must not change
/// between runs.
pub fn unified_diff(expected: &[u8], actual: &[u8]) -> String {
    let expected_lines = split_lines(expected);
    let actual_lines = split_lines(actual);
    let mut rendered = format!(
        "--- expected : {} bytes, {} lines\n+++ actual   : {} bytes, {} lines\n",
        expected.len(),
        expected_lines.len(),
        actual.len(),
        actual_lines.len()
    );

    let divergence = match locate_stdout_divergence(expected, actual) {
        Some(divergence) => divergence,
        None => {
            rendered.push_str(
                "first divergence : none; the two outputs are byte-identical, so there is nothing \
                 to diff\n",
            );
            return rendered;
        }
    };
    rendered.push_str(&format!(
        "first divergence : byte offset {} (line {}, column {}), marked * below\n",
        divergence.offset, divergence.line, divergence.column
    ));

    let positions = expected_lines.len().max(actual_lines.len());
    let mut emitted = 0usize;
    let mut shown = 0usize;
    for index in 0..positions {
        if emitted >= MAX_DIFF_LINES {
            rendered.push_str(&format!(
                "... output truncated after {emitted} rendered lines: {shown} of {positions} \
                 compared line positions are shown\n"
            ));
            return rendered;
        }
        let left = expected_lines.get(index).copied();
        let right = actual_lines.get(index).copied();
        let number = index + 1;
        let marker = if number == divergence.line { '*' } else { ' ' };
        match (left, right) {
            (Some(same), Some(also)) if same == also => {
                rendered.push_str(&format!("{marker} {number:>5}   {}\n", render_line(same)));
                emitted += 1;
            }
            (left, right) => {
                if let Some(only_expected) = left {
                    rendered.push_str(&format!(
                        "{marker} {number:>5} - {}\n",
                        render_line(only_expected)
                    ));
                    emitted += 1;
                }
                if let Some(only_actual) = right {
                    rendered.push_str(&format!(
                        "{marker} {number:>5} + {}\n",
                        render_line(only_actual)
                    ));
                    emitted += 1;
                }
            }
        }
        shown = index + 1;
    }
    rendered
}
