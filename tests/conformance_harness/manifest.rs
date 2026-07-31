//! Parser for the `.expected` expectation-record format, corpus discovery, and
//! expected-divergence-marker enumeration.
//!
//! Every C program in the corpus is paired with a sibling `<program>.expected` record that
//! holds the program's cell matrix, the literal command templates that build and run it, the
//! expected exit status, the written undefined-behaviour-freedom argument, and the golden
//! stdout. That single file is what makes the requirement "source file, build commands, and
//! expected output recorded together" literally true: a maintainer can reproduce any cell
//! from the `.c` file plus its record with no harness, no Cargo and no Rust toolchain at all.
//! It is also what supplies oracle (c), the golden-record regression that catches the one
//! failure mode pure differential testing structurally cannot detect — both compilers
//! changing behaviour in the same direction at the same time, where the reference-compiler
//! oracle still reports agreement.
//!
//! # Why this module is hand-written
//!
//! The project's rules for feature addition state that the `[dependencies]` section of the
//! package manifest must remain completely empty at all times, that every piece of
//! functionality must be implemented using only the Rust standard library, that no
//! build-dependency or development-dependency entries for external crates are permitted, and
//! that the constraint is absolute and admits no exceptions. A serialization crate was
//! considered for exactly this parser and rejected under that rule, so this module is its
//! mandated substitute: a deliberately minimal, line-oriented parser over `std` alone. It is
//! not, and must not become, a general-purpose configuration library.
//!
//! # Grammar
//!
//! The format is line-oriented so that it is trivial to parse by hand and trivial for a human
//! to read and edit. Lines are split with [`str::lines`], which accepts both `"\n"` and
//! `"\r\n"` as terminators, so a record authored on a system that writes carriage returns
//! parses identically to one that does not — a property that matters because a single stray
//! byte per line would fail every golden-record comparison while saying nothing whatsoever
//! about the compiler.
//!
//! - **Comment** — a line whose first non-whitespace character is `#`. Ignored. A `#` inside a
//!   value is data, never a comment.
//! - **Blank line** — ignored outside a heredoc. Inside a heredoc it is a body line.
//! - **Scalar** — `key = value`. Split on the first `=`; whitespace is trimmed from both the
//!   key and the value.
//! - **Heredoc** — `key <<END` opens a block. Every subsequent line is captured verbatim
//!   until a line that is exactly `END`. Inside a heredoc nothing is trimmed, no `#` is a
//!   comment and no `=` is a separator, which is what lets a golden stdout reproduce program
//!   output byte for byte including leading spaces and lines that begin with `#` or contain
//!   `=`.
//! - A line is read as a heredoc opener when `<<` appears before any `=`, so a scalar value
//!   may legitimately contain `<<` and a heredoc opener needs no `=`.
//! - Keys are case-sensitive, lower-snake, and drawn from the alphabet `[a-z0-9_.]`; the dot
//!   exists for the `expected_divergence.*` family.
//!
//! ## Trailing-newline convention
//!
//! **A heredoc value is the concatenation of its body lines with a newline appended to each.**
//! A body of `["a", "b"]` therefore parses to `"a\nb\n"`, and an empty body parses to `""`.
//!
//! This convention is stated here in full because it is the single highest-risk detail in the
//! format. Oracle (c) compares a recorded stdout against captured stdout byte for byte across
//! every cell of the matrix; a one-byte disagreement in this rule would fail the entire suite
//! while telling a reader nothing about the compiler. Three consequences follow, and they are
//! the reason the rule is written this way rather than the other way:
//!
//! - Every corpus program's final `printf` ends with a newline, so a captured stdout stream
//!   ends with a newline, so the value the parser produces is byte-identical to the stream.
//! - The maintenance regeneration script may write a captured stream verbatim between the
//!   opener and the closing `END` line, and the record round-trips under this rule with no
//!   fix-ups.
//! - A stdout stream that does **not** end in a newline cannot be represented by a
//!   line-oriented format at all. That is a stated limitation rather than a silent
//!   truncation: the corpus authoring rules require one printed line per semantic property
//!   claimed, so such a program is outside the corpus by construction.
//!
//! # Key table
//!
//! | Key | Form | Presence | Obligation |
//! | --- | --- | --- | --- |
//! | `program` | scalar | required | Must equal the record's file stem. |
//! | `area` | scalar | required | Must equal the containing directory and name a real feature area. |
//! | `description` | scalar | required | One line, used verbatim in report rows. |
//! | `targets` | scalar, comma list | required | Defines the cell matrix. A restricted list requires `impl_defined_notes`. |
//! | `opt_levels` | scalar, comma list | required | `-O0`, `-O1`, `-O2` only; anything higher is documented as out of scope. |
//! | `shared_flags` | scalar, space list | required | Subset of the verified shared set, and nothing forbidden in a differential invocation. |
//! | `bcc_command` | scalar | required | Template over `$BCC`, `<triple>`, `<opt>`, `<src>`, `<out>`. |
//! | `ref_command` | scalar | required | Template over `$REF_CC_<TRIPLE>`, `<opt>`, `<src>`, `<out>`. No target flag exists. |
//! | `run_command` | scalar | required | Template over `<runner>`, `<out>`. |
//! | `expect_exit` | scalar, integer | required | Within `0..=125`. |
//! | `oracle_a`, `oracle_b`, `oracle_c` | scalar toggle | required | `enabled` or `disabled`. A disabled oracle requires `impl_defined_notes`. |
//! | `ub_audit_flags` | scalar, space list | optional | A deviation from the default warning gate; requires `impl_defined_notes`. |
//! | `ub_notes` | heredoc | required | The written undefined-behaviour-freedom argument. Non-empty. |
//! | `impl_defined_notes` | heredoc | conditional | Required by a restricted target list, a disabled oracle, or a warning-gate deviation. |
//! | `expected_stdout` | heredoc | required | The golden record. Non-empty. |
//! | `expected_divergence.id` | scalar | optional | Marker identifier, for example `XD-GCCEXT-CASE-RANGES-001`. |
//! | `expected_divergence.class` | scalar | with marker | One of the six divergence classes. |
//! | `expected_divergence.scope` | scalar | with marker | Which oracles, targets and optimization levels the marker covers. |
//! | `expected_divergence.basis` | scalar | with marker | A repository-relative file path, a comma, then the section it cites. |
//! | `expected_divergence.observed` | heredoc | with marker | The divergence as observed. Non-empty. |
//!
//! Presence of any `expected_divergence.*` key requires all five: a partial marker is a hard
//! error, because a marker missing its basis is an assertion with no documented authority
//! behind it.
//!
//! # Failure posture
//!
//! An unterminated heredoc, a duplicate key, an unrecognised key, a key used in the wrong
//! surface form, a malformed line, or any violated obligation above is a hard error naming
//! the file path, the one-based line number, the key, and what was expected. Nothing is ever
//! ignored silently: a quietly dropped `expected_stdout` would turn oracle (c) into a no-op,
//! and a quietly dropped obligation would let a narrowing of coverage pass without the
//! recorded reason that makes it reviewable.
//!
//! # This module is read-only with respect to the corpus
//!
//! There is deliberately no writer, no fixer and no update-in-place helper anywhere in this
//! module. Golden records are regenerated only through the maintenance script under
//! `tests/conformance/tools/`, never automatically during a test run, so a wrong answer can
//! never quietly become the new expectation. The module also spawns no process and opens no
//! socket: it reads a program's source path and its expectation record and nothing else, and
//! it is free of global mutable state so that concurrently executing feature areas may parse
//! records at the same time with no lock.
//!
//! # Compatibility
//!
//! Edition 2021, minimum supported Rust 1.70. No standard-library API newer than 1.70 is used.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::{
    comma_separated, corpus_root, is_forbidden_in_differential, joined_target_names, manifest_dir,
    AreaSpec, CellKey, DivergenceClass, HarnessError, HarnessResult, OptLevel, Oracle, Target,
    AREAS, SHARED_FLAGS_VERIFIED,
};

// ---------------------------------------------------------------------------
// Format constants
// ---------------------------------------------------------------------------

/// Token that opens a heredoc block, written immediately before the terminator.
const HEREDOC_OPENER: &str = "<<";

/// The one accepted heredoc terminator. A body line whose trimmed text is this token but which
/// is not exactly this token is reported at that line rather than left to surface as a
/// baffling unterminated-heredoc error at end of file, because invisible trailing whitespace
/// is otherwise one of the hardest editing mistakes to see.
const HEREDOC_TERMINATOR: &str = "END";

/// Extension of a corpus program.
const SOURCE_EXTENSION: &str = "c";

/// Extension of an expectation record.
const RECORD_EXTENSION: &str = "expected";

/// Extensions that may appear inside a feature-area directory without being a program: the
/// expectation records themselves, area notes, the fixture header, and shell tooling.
///
/// Any other regular file in an area directory is a hard error. That is not pedantry: it is
/// what mechanically enforces the rule that the corpus tree contains no `.rs` file anywhere,
/// which is in turn what keeps the corpus invisible to the build system and the suite free of
/// any package-manifest change.
const AREA_COMPANION_EXTENSIONS: &[&str] = &[RECORD_EXTENSION, "md", "h", "sh"];

/// Highest exit status a record may expect.
///
/// The operating system truncates a larger value — a program returning three hundred was
/// measured to exit with status forty-four — so a record that expected such a value would be
/// asserting something the platform cannot deliver.
const EXIT_STATUS_MAX: i32 = 125;

/// Value of an enabled per-oracle toggle.
const TOGGLE_ENABLED: &str = "enabled";

/// Value of a disabled per-oracle toggle.
const TOGGLE_DISABLED: &str = "disabled";

/// Shared flags that legitimately carry an attached value, so that `-DNAME=1` is recognised as
/// the verified `-D` rather than rejected as an unknown spelling.
const VALUE_TAKING_SHARED_FLAGS: &[&str] = &["-o", "-I", "-D", "-U", "-L", "-l"];

/// Target-selection spellings that must never appear in the reference-compiler template.
///
/// The reference compiler has no target-selection flag at all: the `--target=` spelling
/// belongs to a different compiler family and was measured to be rejected outright, which is
/// why the cross arm of the reference-compiler oracle selects a cross-driver binary instead.
const REFERENCE_FORBIDDEN_SPELLINGS: &[&str] = &["--target", "--sysroot"];

/// Placeholder for the compiler under test.
const PLACEHOLDER_BCC: &str = "$BCC";

/// Generic placeholder for the reference compiler driver of the cell's target.
const PLACEHOLDER_REFERENCE_TEMPLATED: &str = "$REF_CC_<TRIPLE>";

/// Bare placeholder for the reference compiler driver, accepted as a synonym of the templated
/// spelling so a record may write either.
const PLACEHOLDER_REFERENCE_BARE: &str = "$REF_CC";

/// Placeholder for the cell's target triple.
const PLACEHOLDER_TRIPLE: &str = "<triple>";

/// Placeholder for the cell's optimization-level flag.
const PLACEHOLDER_OPT: &str = "<opt>";

/// Placeholder for the program source path.
const PLACEHOLDER_SOURCE: &str = "<src>";

/// Placeholder for the built artifact path.
const PLACEHOLDER_OUTPUT: &str = "<out>";

/// Placeholder for the execution runner, which is empty on a natively executing target and the
/// target's emulator otherwise.
const PLACEHOLDER_RUNNER: &str = "<runner>";

// ---------------------------------------------------------------------------
// Key registry
// ---------------------------------------------------------------------------

/// Which surface form a key's value takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    /// `key = value`, on one line.
    Scalar,
    /// `key <<END`, then the value's lines, then a closing `END` line.
    Heredoc,
}

impl FieldKind {
    /// The exact surface form this kind requires, phrased for a diagnostic so a maintainer
    /// editing a record by hand is told what to write rather than merely that something is
    /// wrong.
    fn required_form(self, key: &str) -> String {
        match self {
            FieldKind::Scalar => format!("`{key} = <value>` on a single line"),
            FieldKind::Heredoc => format!(
                "`{key} {HEREDOC_OPENER}{HEREDOC_TERMINATOR}`, then the value's lines, then a \
                 closing line containing exactly `{HEREDOC_TERMINATOR}`"
            ),
        }
    }
}

/// Whether a key must appear in every record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    /// Every record must carry it; absence is a hard error.
    Required,
    /// Carried only when a documented condition applies, and then mandatory. The conditions
    /// are a restricted target list, a disabled oracle, and a warning-gate deviation.
    Conditional,
    /// Carried only as a deliberate per-program deviation or as part of the optional
    /// expected-divergence marker block.
    Optional,
}

/// One recognised key: its spelling, the surface form it requires, and whether every record
/// must carry it.
///
/// The table is the single authority on what a record may contain. An unrecognised key is
/// rejected against it rather than ignored, because an unknown key is almost always a typo,
/// and a typo that is ignored silently disables the very check the key exists to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeySpec {
    /// The key's exact spelling.
    name: &'static str,
    /// The surface form the key's value must take.
    kind: FieldKind,
    /// Whether every record must carry the key.
    presence: Presence,
}

impl KeySpec {
    /// Private table constructor, used only to build [`KEYS`].
    const fn new(name: &'static str, kind: FieldKind, presence: Presence) -> KeySpec {
        KeySpec {
            name,
            kind,
            presence,
        }
    }
}

/// Marker key holding the identifier.
const KEY_MARKER_ID: &str = "expected_divergence.id";

/// Marker key holding the divergence class.
const KEY_MARKER_CLASS: &str = "expected_divergence.class";

/// Marker key holding the scope the marker covers.
const KEY_MARKER_SCOPE: &str = "expected_divergence.scope";

/// Marker key holding the documented basis.
const KEY_MARKER_BASIS: &str = "expected_divergence.basis";

/// Marker key holding the divergence as observed.
const KEY_MARKER_OBSERVED: &str = "expected_divergence.observed";

/// The five marker keys. Either all of them are present or none of them is.
const MARKER_KEYS: &[&str] = &[
    KEY_MARKER_ID,
    KEY_MARKER_CLASS,
    KEY_MARKER_SCOPE,
    KEY_MARKER_BASIS,
    KEY_MARKER_OBSERVED,
];

/// Every recognised key, in the order the reference record writes them.
const KEYS: [KeySpec; 22] = [
    KeySpec::new("program", FieldKind::Scalar, Presence::Required),
    KeySpec::new("area", FieldKind::Scalar, Presence::Required),
    KeySpec::new("description", FieldKind::Scalar, Presence::Required),
    KeySpec::new("targets", FieldKind::Scalar, Presence::Required),
    KeySpec::new("opt_levels", FieldKind::Scalar, Presence::Required),
    KeySpec::new("shared_flags", FieldKind::Scalar, Presence::Required),
    KeySpec::new("bcc_command", FieldKind::Scalar, Presence::Required),
    KeySpec::new("ref_command", FieldKind::Scalar, Presence::Required),
    KeySpec::new("run_command", FieldKind::Scalar, Presence::Required),
    KeySpec::new("expect_exit", FieldKind::Scalar, Presence::Required),
    KeySpec::new("oracle_a", FieldKind::Scalar, Presence::Required),
    KeySpec::new("oracle_b", FieldKind::Scalar, Presence::Required),
    KeySpec::new("oracle_c", FieldKind::Scalar, Presence::Required),
    KeySpec::new("ub_audit_flags", FieldKind::Scalar, Presence::Optional),
    KeySpec::new("ub_notes", FieldKind::Heredoc, Presence::Required),
    KeySpec::new(
        "impl_defined_notes",
        FieldKind::Heredoc,
        Presence::Conditional,
    ),
    KeySpec::new("expected_stdout", FieldKind::Heredoc, Presence::Required),
    KeySpec::new(KEY_MARKER_ID, FieldKind::Scalar, Presence::Optional),
    KeySpec::new(KEY_MARKER_CLASS, FieldKind::Scalar, Presence::Optional),
    KeySpec::new(KEY_MARKER_SCOPE, FieldKind::Scalar, Presence::Optional),
    KeySpec::new(KEY_MARKER_BASIS, FieldKind::Scalar, Presence::Optional),
    KeySpec::new(KEY_MARKER_OBSERVED, FieldKind::Heredoc, Presence::Optional),
];

/// Look a key up by exact spelling.
fn lookup_key(name: &str) -> Option<&'static KeySpec> {
    KEYS.iter().find(|spec| spec.name == name)
}

/// Every recognised key spelling, as one comma-separated line for a diagnostic.
fn known_key_names() -> String {
    let names: Vec<&str> = KEYS.iter().map(|spec| spec.name).collect();
    comma_separated(&names)
}

/// One parsed field: its value and the one-based line at which the key appeared.
///
/// For a heredoc the line is that of the opener, because that is the line a maintainer must go
/// to in order to fix the value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawField {
    /// The value: trimmed for a scalar, verbatim for a heredoc.
    value: String,
    /// One-based line number of the key.
    line: usize,
}

/// A heredoc that has been opened and is accumulating body lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHeredoc {
    /// The key the block belongs to.
    spec: &'static KeySpec,
    /// One-based line number of the opener.
    line: usize,
    /// Body lines captured so far, each of which contributes its own newline to the value.
    body: Vec<String>,
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// Build an error that names the record, the one-based line and the key.
fn key_error(origin: &Path, line: usize, key: &str, cause: impl Into<String>) -> HarnessError {
    HarnessError::new(
        format!(
            "parsing the expectation record {} at line {line}, key `{key}`",
            origin.display()
        ),
        cause,
    )
}

/// Build an error that names the record and the one-based line, for a line whose key could not
/// be established.
fn line_error(origin: &Path, line: usize, cause: impl Into<String>) -> HarnessError {
    HarnessError::new(
        format!(
            "parsing the expectation record {} at line {line}",
            origin.display()
        ),
        cause,
    )
}

/// Build an error about the record as a whole, used when the fault is an absent key and there
/// is therefore no line to point at.
fn record_error(origin: &Path, cause: impl Into<String>) -> HarnessError {
    HarnessError::new(
        format!("parsing the expectation record {}", origin.display()),
        cause,
    )
}

// ---------------------------------------------------------------------------
// Value tokenizers
// ---------------------------------------------------------------------------

/// Split a comma-separated value, trimming each item and dropping empty ones.
///
/// Empty items are dropped rather than rejected so that a trailing comma is harmless; the
/// caller still rejects a list that ends up empty, which is the case that actually matters.
fn comma_items(value: &str) -> Vec<&str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect()
}

/// Split a whitespace-separated value into tokens.
fn whitespace_items(value: &str) -> Vec<&str> {
    value.split_whitespace().collect()
}

// ---------------------------------------------------------------------------
// Line parser
// ---------------------------------------------------------------------------

/// True when `key` is drawn from the key alphabet: lower-case ASCII letters, digits,
/// underscore, and the dot that the marker family uses.
fn is_well_formed_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '.'
        })
}

/// Resolve a key token to its registry entry, rejecting a malformed spelling and an
/// unrecognised name with distinct, actionable diagnostics.
fn resolve_key(origin: &Path, line: usize, key: &str) -> HarnessResult<&'static KeySpec> {
    if key.is_empty() {
        return Err(line_error(
            origin,
            line,
            format!(
                "the line has no key before its separator; every line must be a comment, blank, \
                 `key = value`, or `key {HEREDOC_OPENER}{HEREDOC_TERMINATOR}`"
            ),
        ));
    }
    if !is_well_formed_key(key) {
        return Err(line_error(
            origin,
            line,
            format!(
                "{key:?} is not a well-formed key; keys are case-sensitive lower-snake ASCII \
                 drawn from `a-z`, `0-9`, `_` and `.`"
            ),
        ));
    }
    lookup_key(key).ok_or_else(|| {
        line_error(
            origin,
            line,
            format!(
                "{key:?} is not a recognised key; an unrecognised key is rejected rather than \
                 ignored because a typo that is ignored silently disables the check the key \
                 exists to perform. The recognised keys are: {}",
                known_key_names()
            ),
        )
    })
}

/// Record a parsed field, rejecting a second occurrence of the same key.
///
/// A duplicate is a hard error rather than a last-one-wins overwrite, because the two values
/// are by definition in disagreement and no rule for choosing between them could be anything
/// but arbitrary.
fn push_field(
    fields: &mut Vec<(&'static KeySpec, RawField)>,
    origin: &Path,
    spec: &'static KeySpec,
    field: RawField,
) -> HarnessResult<()> {
    if let Some((_, existing)) = fields.iter().find(|(known, _)| known.name == spec.name) {
        return Err(key_error(
            origin,
            field.line,
            spec.name,
            format!(
                "duplicate key; it was already set at line {}. Two values for one key disagree \
                 by definition, so neither is taken",
                existing.line
            ),
        ));
    }
    fields.push((spec, field));
    Ok(())
}

/// Borrow a parsed field by key spelling.
fn field<'fields>(
    fields: &'fields [(&'static KeySpec, RawField)],
    name: &str,
) -> Option<&'fields RawField> {
    fields
        .iter()
        .find(|(spec, _)| spec.name == name)
        .map(|(_, value)| value)
}

/// Join a heredoc body into its value, appending a newline to each body line.
///
/// This is the trailing-newline convention documented at module level, implemented in exactly
/// one place so that it cannot drift.
fn join_heredoc_body(body: &[String]) -> String {
    let mut value = String::new();
    for line in body {
        value.push_str(line);
        value.push('\n');
    }
    value
}

/// Parse a record's text into its fields, in a single pass with a two-state machine.
///
/// The states are "reading key lines" and "inside a heredoc". Nothing else is needed, and
/// nothing else is offered: the format is deliberately minimal, and a richer parser would
/// invite a richer format that no longer round-trips byte for byte.
fn parse_fields(text: &str, origin: &Path) -> HarnessResult<Vec<(&'static KeySpec, RawField)>> {
    let mut fields: Vec<(&'static KeySpec, RawField)> = Vec::new();
    let mut pending: Option<PendingHeredoc> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let number = index + 1;

        if let Some(mut open) = pending.take() {
            if raw_line == HEREDOC_TERMINATOR {
                let value = join_heredoc_body(&open.body);
                push_field(
                    &mut fields,
                    origin,
                    open.spec,
                    RawField {
                        value,
                        line: open.line,
                    },
                )?;
                continue;
            }
            if raw_line.trim() == HEREDOC_TERMINATOR {
                return Err(key_error(
                    origin,
                    number,
                    open.spec.name,
                    format!(
                        "the line {raw_line:?} looks like the heredoc terminator but carries \
                         surrounding whitespace; the terminator must be a line containing \
                         exactly `{HEREDOC_TERMINATOR}`, because a value's lines are captured \
                         verbatim and cannot be trimmed"
                    ),
                ));
            }
            open.body.push(String::from(raw_line));
            pending = Some(open);
            continue;
        }

        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let opener_at = raw_line.find(HEREDOC_OPENER);
        let equals_at = raw_line.find('=');
        let heredoc_at = match (opener_at, equals_at) {
            (Some(opener), Some(equals)) if opener < equals => Some(opener),
            (Some(opener), None) => Some(opener),
            _ => None,
        };

        if let Some(position) = heredoc_at {
            let key = raw_line[..position].trim();
            let spec = resolve_key(origin, number, key)?;
            let terminator = raw_line[position + HEREDOC_OPENER.len()..].trim();
            if terminator != HEREDOC_TERMINATOR {
                return Err(key_error(
                    origin,
                    number,
                    spec.name,
                    format!(
                        "the heredoc opener names the terminator {terminator:?}; the one accepted \
                         terminator is `{HEREDOC_TERMINATOR}`, so the opener must read \
                         `{key} {HEREDOC_OPENER}{HEREDOC_TERMINATOR}`"
                    ),
                ));
            }
            if spec.kind != FieldKind::Heredoc {
                return Err(key_error(
                    origin,
                    number,
                    spec.name,
                    format!(
                        "this key takes the scalar form, not a heredoc; write {}",
                        spec.kind.required_form(spec.name)
                    ),
                ));
            }
            pending = Some(PendingHeredoc {
                spec,
                line: number,
                body: Vec::new(),
            });
            continue;
        }

        if let Some(position) = equals_at {
            let key = raw_line[..position].trim();
            let spec = resolve_key(origin, number, key)?;
            if spec.kind != FieldKind::Scalar {
                return Err(key_error(
                    origin,
                    number,
                    spec.name,
                    format!(
                        "this key takes the heredoc form, not a scalar; write {}",
                        spec.kind.required_form(spec.name)
                    ),
                ));
            }
            let value = raw_line[position + 1..].trim();
            push_field(
                &mut fields,
                origin,
                spec,
                RawField {
                    value: String::from(value),
                    line: number,
                },
            )?;
            continue;
        }

        return Err(line_error(
            origin,
            number,
            format!(
                "the line {trimmed:?} is neither a comment, a blank line, a `key = value` scalar, \
                 nor a `key {HEREDOC_OPENER}{HEREDOC_TERMINATOR}` heredoc opener"
            ),
        ));
    }

    if let Some(open) = pending {
        return Err(key_error(
            origin,
            open.line,
            open.spec.name,
            format!(
                "the heredoc opened here is never closed; add a line containing exactly \
                 `{HEREDOC_TERMINATOR}` after the value's {} captured line(s)",
                open.body.len()
            ),
        ));
    }

    Ok(fields)
}

// ---------------------------------------------------------------------------
// Command templates
// ---------------------------------------------------------------------------

/// Everything one cell needs in order to turn a command template into a literal command line.
///
/// The fields are public because this is a plain carrier with no invariant to protect, which
/// matches how the harness root models its own cell and outcome records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSubstitutions {
    /// The compiler under test, substituted for `$BCC`.
    pub bcc: PathBuf,
    /// The reference compiler driver for this cell's target, substituted for every accepted
    /// `$REF_CC` spelling. For the native arm this is the native driver; for a cross arm it is
    /// the matching cross driver, because the reference compiler has no target-selection flag.
    pub reference_compiler: PathBuf,
    /// The cell's target, substituted for `<triple>` as its canonical triple.
    pub target: Target,
    /// The cell's optimization level, substituted for `<opt>` as its command-line flag.
    pub opt: OptLevel,
    /// The program source, substituted for `<src>`.
    pub source: PathBuf,
    /// The artifact to build, substituted for `<out>`.
    pub output: PathBuf,
    /// The execution runner, substituted for `<runner>`. `None` on a natively executing target,
    /// where the placeholder and one following space are removed so the rendered line begins
    /// with the artifact itself.
    pub runner: Option<PathBuf>,
}

/// Expand a command template into a literal command line.
///
/// Accepted placeholders, and the reason each spelling is accepted:
///
/// - `$BCC` — the compiler under test.
/// - `$REF_CC_<TRIPLE>` — the reference driver for this cell's target. This is the spelling the
///   reference record uses, and it is deliberately not a target *flag*: the reference compiler
///   has none, so the triple selects a **driver binary**.
/// - `$REF_CC_X86_64`, `$REF_CC_I686`, `$REF_CC_AARCH64`, `$REF_CC_RISCV64` — the concrete
///   spellings, accepted because a target-restricted record may legitimately name its one
///   driver directly, and because they mirror the environment-variable names that select the
///   drivers.
/// - `$REF_CC` — the bare synonym.
/// - `<triple>`, `<opt>`, `<src>`, `<out>`, `<runner>` — the cell's triple, optimization-level
///   flag, source path, artifact path and execution runner.
///
/// The result is what makes a cell reproducible by hand with no harness at all, so substitution
/// is faithful and unquoted: the rendered line is the literal line a maintainer runs, and the
/// corpus and workspace paths are free of whitespace by construction. Use
/// [`render_command_checked`] wherever the rendered line is going to be shown to a human or
/// written into a findings artifact, since a half-expanded command line is worse than none.
pub fn render_command(template: &str, subs: &CommandSubstitutions) -> String {
    let bcc = subs.bcc.display().to_string();
    let reference = subs.reference_compiler.display().to_string();
    let source = subs.source.display().to_string();
    let output = subs.output.display().to_string();

    let mut rendered = String::from(template);
    rendered = rendered.replace(PLACEHOLDER_REFERENCE_TEMPLATED, &reference);
    for target in Target::ALL {
        let concrete = format!(
            "{PLACEHOLDER_REFERENCE_BARE}_{}",
            target.short_name().to_ascii_uppercase()
        );
        rendered = rendered.replace(&concrete, &reference);
    }
    rendered = rendered.replace(PLACEHOLDER_REFERENCE_BARE, &reference);
    rendered = rendered.replace(PLACEHOLDER_BCC, &bcc);
    rendered = rendered.replace(PLACEHOLDER_TRIPLE, subs.target.triple());
    rendered = rendered.replace(PLACEHOLDER_OPT, subs.opt.flag());
    rendered = rendered.replace(PLACEHOLDER_SOURCE, &source);
    rendered = rendered.replace(PLACEHOLDER_OUTPUT, &output);
    match &subs.runner {
        Some(runner) => {
            rendered = rendered.replace(PLACEHOLDER_RUNNER, &runner.display().to_string());
        }
        None => {
            let with_space = format!("{PLACEHOLDER_RUNNER} ");
            rendered = rendered.replace(&with_space, "");
            rendered = rendered.replace(PLACEHOLDER_RUNNER, "");
        }
    }
    String::from(rendered.trim())
}

/// The first placeholder-shaped token still present in a rendered command line, if any.
///
/// Two shapes are recognised: an angle-bracketed token, and a dollar sign followed by an
/// identifier. Both are reported verbatim so a diagnostic can name exactly what failed to
/// expand. Neither shape can occur legitimately in a rendered line, because the templates
/// contain no shell redirection and no shell variable other than the placeholders themselves.
pub fn residual_placeholder(rendered: &str) -> Option<String> {
    if let Some(start) = rendered.find('<') {
        if let Some(offset) = rendered[start..].find('>') {
            return Some(String::from(&rendered[start..=start + offset]));
        }
    }
    for (index, character) in rendered.char_indices() {
        if character != '$' {
            continue;
        }
        let identifier: String = rendered[index + character.len_utf8()..]
            .chars()
            .take_while(|candidate| candidate.is_ascii_alphanumeric() || *candidate == '_')
            .collect();
        if !identifier.is_empty() {
            return Some(format!("${identifier}"));
        }
    }
    None
}

/// Expand a command template and reject a result that still contains a placeholder.
///
/// This is the entry point the harness uses. An unexpanded placeholder means the template names
/// something the substitutions do not supply, which would put a command line into a reproduction
/// script that cannot reproduce anything — so it is a hard error naming both the template and
/// the offending token rather than a line that merely looks plausible.
pub fn render_command_checked(
    template: &str,
    subs: &CommandSubstitutions,
) -> HarnessResult<String> {
    let rendered = render_command(template, subs);
    match residual_placeholder(&rendered) {
        None => Ok(rendered),
        Some(residual) => Err(HarnessError::new(
            format!("rendering the command template {template:?}"),
            format!(
                "the rendered line {rendered:?} still contains the placeholder {residual}; a \
                 half-expanded command line cannot reproduce a cell, so it is rejected rather \
                 than recorded. The accepted placeholders are \
                 {PLACEHOLDER_BCC}, {PLACEHOLDER_REFERENCE_TEMPLATED}, \
                 {PLACEHOLDER_REFERENCE_BARE}, {PLACEHOLDER_TRIPLE}, {PLACEHOLDER_OPT}, \
                 {PLACEHOLDER_SOURCE}, {PLACEHOLDER_OUTPUT} and {PLACEHOLDER_RUNNER}"
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// Expected-divergence marker
// ---------------------------------------------------------------------------

/// Which oracles, targets and optimization levels an expected-divergence marker covers.
///
/// The scope is parsed into structure rather than kept as prose so that the classifier can ask
/// "does this marker cover this cell?" and receive an answer instead of having to guess. Each
/// dimension is normalized to the canonical table order, and a dimension the scope does not
/// mention defaults to every member of that dimension — so `oracle_b` on its own means
/// "cross-backend comparison, on every target, at every optimization level".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerScope {
    /// The scope exactly as the record wrote it, retained for reports and artifacts.
    pub raw: String,
    /// Oracles the marker covers.
    pub oracles: Vec<Oracle>,
    /// Targets the marker covers.
    pub targets: Vec<Target>,
    /// Optimization levels the marker covers.
    pub opt_levels: Vec<OptLevel>,
}

impl fmt::Display for MarkerScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.raw)
    }
}

/// A divergence that a limitation the repository already documents explains.
///
/// A marker changes how a divergence is **classified**, never whether the feature is
/// **exercised**: a marked program still compiles and still runs, which is what keeps a
/// difficult feature under test instead of quietly dropped. The basis is carried as both the
/// original text and the repository-relative path it cites, so the infrastructure test that
/// audits the register can assert the cited document actually exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedDivergence {
    /// Marker identifier, by convention `XD-<AREA>-<TOPIC>-<NNN>`, for example
    /// `XD-GCCEXT-CASE-RANGES-001`. Unique across the corpus.
    pub id: String,
    /// The shape the divergence takes.
    pub class: DivergenceClass,
    /// Which oracles, targets and optimization levels the marker covers.
    pub scope: MarkerScope,
    /// The documented basis, verbatim: a repository-relative path, a comma, then the section or
    /// description that authorises the marker.
    pub basis: String,
    /// The repository-relative path portion of the basis.
    pub basis_path: PathBuf,
    /// The divergence as observed, so a reader can recognise it without reproducing the run.
    pub observed: String,
    /// The program that provokes the divergence: the `.c` file, not its record.
    pub program_path: PathBuf,
}

impl ExpectedDivergence {
    /// True when this marker's scope covers the given cell and oracle.
    ///
    /// This answers the **scope** question only, and deliberately does not re-check which
    /// program the cell belongs to: a marker is reachable only through the record of the very
    /// program it governs, so the program identity is already established by the time a caller
    /// holds one. Keeping the check to the scope is what makes the answer unambiguous.
    pub fn covers(&self, key: &CellKey, oracle: Oracle) -> bool {
        self.scope.oracles.contains(&oracle)
            && self.scope.targets.contains(&key.target)
            && self.scope.opt_levels.contains(&key.opt)
    }

    /// The basis path resolved against the package root, for a caller that needs to test the
    /// cited document's existence without reassembling the path itself.
    pub fn basis_absolute_path(&self) -> PathBuf {
        manifest_dir().join(&self.basis_path)
    }

    /// `area/program` when both can be read from the program path, and the full path otherwise.
    ///
    /// Used in register diagnostics, where naming the owning program matters more than naming
    /// the file it lives in.
    pub fn program_label(&self) -> String {
        let program = self.program_path.file_stem().and_then(|stem| stem.to_str());
        let area = self
            .program_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str());
        match (area, program) {
            (Some(area), Some(program)) => format!("{area}/{program}"),
            _ => self.program_path.display().to_string(),
        }
    }
}

impl fmt::Display for ExpectedDivergence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} [class {}, scope {}, basis {}, program {}]",
            self.id,
            self.class,
            self.scope,
            self.basis,
            self.program_label()
        )
    }
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// One program's fully validated expectation record.
///
/// The fields are private and reached through accessors, which is a deliberate departure from
/// how the harness root models its plain carriers. A manifest is not a plain carrier: it holds
/// invariants that the parser established and that the rest of the suite relies on — that the
/// program and area agree with the record's own location, that the expected exit status is one
/// the platform can deliver, that no flag forbidden in a differential invocation reached the
/// shared set, and that every narrowing of coverage carries a recorded reason. Exposing the
/// fields would make an unvalidated manifest constructible, and an unvalidated manifest is
/// exactly the thing whose absence lets a divergence be read as evidence about the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    path: PathBuf,
    program: String,
    area: String,
    description: String,
    targets: Vec<Target>,
    opt_levels: Vec<OptLevel>,
    shared_flags: Vec<String>,
    bcc_command: String,
    ref_command: String,
    run_command: String,
    expect_exit: i32,
    enabled_oracles: Vec<Oracle>,
    ub_audit_flags: Option<Vec<String>>,
    ub_notes: String,
    impl_defined_notes: Option<String>,
    expected_stdout: String,
    marker: Option<ExpectedDivergence>,
}

impl Manifest {
    /// Absolute path to the `.expected` record this manifest was parsed from.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Absolute path to the sibling `.c` program the record governs.
    ///
    /// Derived rather than stored, because the pairing is the format's own rule: a record and
    /// its program differ only in extension, which is what lets either one be found from the
    /// other with no index and no configuration.
    pub fn source_path(&self) -> PathBuf {
        self.path.with_extension(SOURCE_EXTENSION)
    }

    /// The program stem, for example `004_narrowing_conversions`.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The feature-area directory name, for example `01_integer_conversions`.
    pub fn area(&self) -> &str {
        &self.area
    }

    /// The one-line description used verbatim in report rows.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The targets this program is compared on, in canonical table order.
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// The optimization levels this program is compared at, in ascending order.
    pub fn opt_levels(&self) -> &[OptLevel] {
        &self.opt_levels
    }

    /// The flags passed identically to both compilers in a differential invocation.
    pub fn shared_flags(&self) -> &[String] {
        &self.shared_flags
    }

    /// The template that builds this program with the compiler under test.
    pub fn bcc_command(&self) -> &str {
        &self.bcc_command
    }

    /// The template that builds this program with the reference compiler.
    pub fn ref_command(&self) -> &str {
        &self.ref_command
    }

    /// The template that executes a built artifact.
    pub fn run_command(&self) -> &str {
        &self.run_command
    }

    /// The exit status every cell of this program is expected to produce.
    pub fn expect_exit(&self) -> i32 {
        self.expect_exit
    }

    /// True when the named oracle is enabled for this program.
    ///
    /// The three toggles are independent, and that independence is what keeps a construct whose
    /// value legitimately differs between architectures under test: disabling cross-backend
    /// value equality alone leaves the program fully compared against its same-target reference
    /// and fully compared against its golden record. Narrowing an exclusion to one oracle is
    /// permitted; dropping the feature is not.
    pub fn oracle_enabled(&self, oracle: Oracle) -> bool {
        self.enabled_oracles.contains(&oracle)
    }

    /// The enabled oracles, in requirement order.
    pub fn enabled_oracles(&self) -> &[Oracle] {
        &self.enabled_oracles
    }

    /// The disabled oracles, in requirement order.
    ///
    /// Never silent: `report.rs` lists these so that the set of comparisons deliberately not
    /// made is as visible in the summary as the set that was.
    pub fn disabled_oracles(&self) -> Vec<Oracle> {
        Oracle::ALL
            .iter()
            .copied()
            .filter(|oracle| !self.enabled_oracles.contains(oracle))
            .collect()
    }

    /// The per-program warning-gate flags when this program deviates from the default gate, and
    /// `None` when it uses the default.
    ///
    /// The default gate itself is not defined here: the audit module owns it, and duplicating it
    /// would create two authorities that could drift apart.
    pub fn ub_audit_flags(&self) -> Option<&[String]> {
        self.ub_audit_flags.as_deref()
    }

    /// True when this program deviates from the default warning gate.
    pub fn has_ub_audit_deviation(&self) -> bool {
        self.ub_audit_flags.is_some()
    }

    /// The written undefined-behaviour-freedom argument: the human half of the requirement whose
    /// machine half is the audit gate.
    pub fn ub_notes(&self) -> &str {
        &self.ub_notes
    }

    /// The recorded reason for every narrowing this program applies, when it applies any.
    pub fn impl_defined_notes(&self) -> Option<&str> {
        self.impl_defined_notes.as_deref()
    }

    /// The golden record: the stdout every cell of this program is expected to produce.
    pub fn expected_stdout(&self) -> &str {
        &self.expected_stdout
    }

    /// The golden record as bytes, which is the form the comparison actually uses, since the
    /// comparison is byte-exact rather than textual.
    pub fn expected_stdout_bytes(&self) -> &[u8] {
        self.expected_stdout.as_bytes()
    }

    /// True when this program carries an expected-divergence marker.
    pub fn has_marker(&self) -> bool {
        self.marker.is_some()
    }

    /// This program's expected-divergence marker, when it carries one.
    pub fn marker(&self) -> Option<&ExpectedDivergence> {
        self.marker.as_ref()
    }

    /// True when this program is compared on fewer than every target.
    ///
    /// A restriction is always accompanied by a recorded reason, which the parser enforces, so a
    /// true answer here always has an explanation available from
    /// [`Manifest::impl_defined_notes`].
    pub fn is_target_restricted(&self) -> bool {
        self.targets.len() < Target::ALL.len()
    }

    /// True when this program is compared at fewer than every optimization level.
    pub fn is_opt_level_restricted(&self) -> bool {
        self.opt_levels.len() < OptLevel::ALL.len()
    }

    /// Number of cells this program's declared matrix contains.
    pub fn cell_count(&self) -> usize {
        self.targets.len() * self.opt_levels.len()
    }

    /// This program's declared cell matrix: the product of its targets and its optimization
    /// levels, target-major.
    ///
    /// The order is a pure function of the declared sets rather than of the record's formatting,
    /// because both lists are normalized to canonical order at parse time. Two runs therefore
    /// visit the same cells in the same order, which is what makes a retained workspace and a
    /// report row reproducible.
    pub fn cells(&self) -> Vec<CellKey> {
        let mut cells = Vec::with_capacity(self.cell_count());
        for target in &self.targets {
            for opt in &self.opt_levels {
                cells.push(CellKey {
                    area: self.area.clone(),
                    program: self.program.clone(),
                    target: *target,
                    opt: *opt,
                });
            }
        }
        cells
    }

    /// Render this program's compiler-under-test command line for one cell.
    pub fn render_bcc_command(&self, subs: &CommandSubstitutions) -> HarnessResult<String> {
        render_command_checked(&self.bcc_command, subs)
    }

    /// Render this program's reference-compiler command line for one cell.
    pub fn render_ref_command(&self, subs: &CommandSubstitutions) -> HarnessResult<String> {
        render_command_checked(&self.ref_command, subs)
    }

    /// Render this program's execution command line for one cell.
    pub fn render_run_command(&self, subs: &CommandSubstitutions) -> HarnessResult<String> {
        render_command_checked(&self.run_command, subs)
    }
}

impl fmt::Display for Manifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{} ({} targets x {} opt levels = {} cells, oracles",
            self.area,
            self.program,
            self.targets.len(),
            self.opt_levels.len(),
            self.cell_count()
        )?;
        for oracle in &self.enabled_oracles {
            write!(formatter, " {}", oracle.letter())?;
        }
        if self.enabled_oracles.is_empty() {
            formatter.write_str(" none")?;
        }
        match &self.marker {
            Some(marker) => write!(formatter, ", marker {})", marker.id),
            None => formatter.write_str(")"),
        }
    }
}

// ---------------------------------------------------------------------------
// Field validation
// ---------------------------------------------------------------------------

/// The surface form a key requires, for a diagnostic about the key's absence.
fn required_form_of(name: &str) -> String {
    match lookup_key(name) {
        Some(spec) => spec.kind.required_form(spec.name),
        None => format!("a value for `{name}`"),
    }
}

/// Borrow a field that must be present, failing with a diagnostic that names the absent key and
/// the form it takes.
fn required_field<'fields>(
    fields: &'fields [(&'static KeySpec, RawField)],
    origin: &Path,
    name: &str,
) -> HarnessResult<&'fields RawField> {
    field(fields, name).ok_or_else(|| {
        record_error(
            origin,
            format!(
                "required key `{name}` is absent from the record, so there is no line to point \
                 at; expected {}",
                required_form_of(name)
            ),
        )
    })
}

/// Reject an empty value, explaining what the value is for.
fn require_non_empty(origin: &Path, raw: &RawField, key: &str, purpose: &str) -> HarnessResult<()> {
    if raw.value.trim().is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            key,
            format!("the value is empty; {purpose}"),
        ));
    }
    Ok(())
}

/// Reject a template that omits a placeholder it cannot do without.
fn require_placeholders(
    origin: &Path,
    raw: &RawField,
    key: &str,
    needles: &[&str],
) -> HarnessResult<()> {
    for needle in needles {
        if !raw.value.contains(needle) {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the template {:?} omits the required placeholder {needle}; a template that \
                     cannot name every part of its cell cannot reproduce that cell by hand, which \
                     is the whole purpose of recording it",
                    raw.value
                ),
            ));
        }
    }
    Ok(())
}

/// Parse and canonicalize the target list that defines this program's cell matrix.
fn parse_targets(origin: &Path, raw: &RawField) -> HarnessResult<Vec<Target>> {
    let mut declared: Vec<Target> = Vec::new();
    for item in comma_items(&raw.value) {
        let target = Target::parse(item).ok_or_else(|| {
            key_error(
                origin,
                raw.line,
                "targets",
                format!(
                    "{item:?} is not one of the four supported targets; expected a short name or \
                     a triple from: {}",
                    joined_target_names()
                ),
            )
        })?;
        if declared.contains(&target) {
            return Err(key_error(
                origin,
                raw.line,
                "targets",
                format!(
                    "{item:?} appears more than once; the cell matrix is a set, so a target may \
                     be declared at most once"
                ),
            ));
        }
        declared.push(target);
    }
    if declared.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "targets",
            "the target list is empty; a program compared on no target is compared by no oracle, \
             which is a corpus defect rather than a narrowing of coverage",
        ));
    }
    Ok(canonical(&Target::ALL, &declared))
}

/// True when a token names an optimization level the project documents as out of scope.
fn is_out_of_scope_opt_level(item: &str) -> bool {
    let bare = item.strip_prefix('-').unwrap_or(item);
    match bare.strip_prefix('O') {
        Some(remainder) => !matches!(remainder, "0" | "1" | "2"),
        None => false,
    }
}

/// Parse and canonicalize the optimization-level sweep.
fn parse_opt_levels(origin: &Path, raw: &RawField) -> HarnessResult<Vec<OptLevel>> {
    let mut declared: Vec<OptLevel> = Vec::new();
    for item in comma_items(&raw.value) {
        let level = OptLevel::parse(item).ok_or_else(|| {
            let cause = if is_out_of_scope_opt_level(item) {
                format!(
                    "{item:?} is documented as out of scope: the project's own exclusion table \
                     records that only `-O0`, `-O1` and `-O2` are in scope, so no record may \
                     sweep a level the compiler under test does not support"
                )
            } else {
                format!(
                    "{item:?} is not an optimization level; expected one of: {}",
                    comma_separated(&OptLevel::ALL.map(OptLevel::flag))
                )
            };
            key_error(origin, raw.line, "opt_levels", cause)
        })?;
        if declared.contains(&level) {
            return Err(key_error(
                origin,
                raw.line,
                "opt_levels",
                format!(
                    "{item:?} appears more than once; the sweep is a set, so a level may be \
                     declared at most once"
                ),
            ));
        }
        declared.push(level);
    }
    if declared.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "opt_levels",
            "the optimization-level sweep is empty; sweeping the levels is what turns output \
             equality into a semantic-preservation test, so a program must declare at least one",
        ));
    }
    Ok(canonical(&OptLevel::ALL, &declared))
}

/// True when a flag is either exactly a verified shared flag or one of the verified flags that
/// legitimately carries an attached value.
fn is_verified_shared_flag(flag: &str) -> bool {
    if SHARED_FLAGS_VERIFIED.contains(&flag) {
        return true;
    }
    VALUE_TAKING_SHARED_FLAGS
        .iter()
        .any(|prefix| match flag.strip_prefix(prefix) {
            Some(remainder) => !remainder.is_empty(),
            None => false,
        })
}

/// Parse the flags passed identically to both compilers, enforcing the shared-flag discipline at
/// the data layer.
///
/// This is where the requirement "only pass command-line flags that both compilers honour with
/// the same meaning" stops being a convention and becomes a check. Enforcing it here rather than
/// only at the invocation site is deliberate: a maintainer cannot smuggle a
/// reference-compiler-only flag into a differential invocation by editing a record, because the
/// record itself will refuse to load.
fn parse_shared_flags(origin: &Path, raw: &RawField) -> HarnessResult<Vec<String>> {
    let items = whitespace_items(&raw.value);
    if items.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "shared_flags",
            "the shared-flag list is empty; every artifact in the corpus is built statically, \
             which is the one linkage mode both compilers spell identically and what lets an \
             emulated target run with no sysroot configuration",
        ));
    }
    let mut flags = Vec::with_capacity(items.len());
    for item in items {
        if is_forbidden_in_differential(item) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} must never appear in a differential invocation: it is either \
                     reference-compiler-only, compiler-under-test-only, or accepted by both with \
                     a different default scope. Diagnostic and sanitizer flags belong to the \
                     undefined-behaviour audit gate, which drives the reference compiler alone, \
                     and target selection belongs to the cross-backend oracle, where both sides \
                     are the same compiler"
                ),
            ));
        }
        if !is_verified_shared_flag(item) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} is not a verified shared flag; a flag may be passed to both \
                     compilers only once an observable consequence of it has been verified for \
                     both. The verified set is: {}",
                    comma_separated(SHARED_FLAGS_VERIFIED)
                ),
            ));
        }
        flags.push(String::from(item));
    }
    Ok(flags)
}

/// Parse a per-program deviation from the default warning gate.
///
/// These flags are deliberately **not** checked against the forbidden-in-a-differential-
/// invocation set. The audit gate drives the reference compiler and never the compiler under
/// test, so it legitimately uses diagnostic flags that a differential invocation must never
/// carry; conflating the two sets would make the two genuine deviations in the corpus —
/// dropping strict-conformance diagnostics where an extension is the subject, and dropping
/// conversion diagnostics where a narrowing conversion is the subject — impossible to express.
fn parse_ub_audit_flags(origin: &Path, raw: &RawField) -> HarnessResult<Vec<String>> {
    let items = whitespace_items(&raw.value);
    if items.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "ub_audit_flags",
            "the key is present but names no flag; a deviation from the default warning gate must \
             state the gate it wants, and an empty deviation would silently disable the gate \
             altogether",
        ));
    }
    for item in &items {
        if !item.starts_with('-') {
            return Err(key_error(
                origin,
                raw.line,
                "ub_audit_flags",
                format!("{item:?} is not a flag; every item of the warning gate begins with `-`"),
            ));
        }
    }
    Ok(items.into_iter().map(String::from).collect())
}

/// Parse one per-oracle toggle.
fn parse_toggle(origin: &Path, raw: &RawField, key: &str) -> HarnessResult<bool> {
    let value = raw.value.trim();
    if value.eq_ignore_ascii_case(TOGGLE_ENABLED) {
        return Ok(true);
    }
    if value.eq_ignore_ascii_case(TOGGLE_DISABLED) {
        return Ok(false);
    }
    Err(key_error(
        origin,
        raw.line,
        key,
        format!(
            "{value:?} is neither `{TOGGLE_ENABLED}` nor `{TOGGLE_DISABLED}`; a toggle that \
             cannot be read would silently decide whether an oracle runs, so it is rejected"
        ),
    ))
}

/// Parse the expected exit status, bounded by what the platform can actually deliver.
fn parse_expect_exit(origin: &Path, raw: &RawField) -> HarnessResult<i32> {
    let value = raw.value.trim();
    let status: i32 = value.parse().map_err(|_| {
        key_error(
            origin,
            raw.line,
            "expect_exit",
            format!(
                "{value:?} is not a decimal integer; expected an exit status in \
                 0..={EXIT_STATUS_MAX}"
            ),
        )
    })?;
    if !(0..=EXIT_STATUS_MAX).contains(&status) {
        return Err(key_error(
            origin,
            raw.line,
            "expect_exit",
            format!(
                "{status} is outside 0..={EXIT_STATUS_MAX}; the operating system truncates a \
                 larger value — returning three hundred was measured to exit with status \
                 forty-four — so a record may not expect a status the platform cannot deliver"
            ),
        ));
    }
    Ok(status)
}

/// Reorder a chosen subset into the canonical order of the full table.
///
/// Normalizing here is what makes a program's cell order a pure function of the declared sets
/// rather than of the record's formatting, so two runs visit the same cells in the same order.
fn canonical<T: Copy + PartialEq>(all: &[T], chosen: &[T]) -> Vec<T> {
    all.iter()
        .copied()
        .filter(|candidate| chosen.contains(candidate))
        .collect()
}

/// Parse every item of one scope clause with the same dimension's parser, or report that the
/// clause does not belong to that dimension.
fn parse_uniform<T: Copy + PartialEq>(
    items: &[&str],
    parse: fn(&str) -> Option<T>,
) -> Option<Vec<T>> {
    let mut parsed: Vec<T> = Vec::with_capacity(items.len());
    for item in items {
        let value = parse(item)?;
        if !parsed.contains(&value) {
            parsed.push(value);
        }
    }
    Some(parsed)
}

/// Constrain one scope dimension, rejecting a second constraint on the same dimension.
fn set_dimension<T>(
    slot: &mut Option<Vec<T>>,
    value: Vec<T>,
    origin: &Path,
    raw: &RawField,
    dimension: &str,
) -> HarnessResult<()> {
    if slot.is_some() {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_SCOPE,
            format!(
                "the {dimension} dimension is constrained more than once; each of the oracle, \
                 target and optimization-level dimensions may appear at most once in a scope, so \
                 that the scope has exactly one reading"
            ),
        ));
    }
    *slot = Some(value);
    Ok(())
}

/// Parse a marker scope into structure.
///
/// Clauses are separated by `;`. A clause either names a whole dimension — `all oracles`,
/// `all targets`, `all opt levels` — or lists members of exactly one dimension, in which case
/// every item of the clause must belong to that same dimension. A dimension the scope does not
/// mention defaults to all of its members, so `oracle_b` alone reads as "the cross-backend
/// comparison, on every target, at every optimization level".
///
/// Lower-casing is applied only to the whole-dimension test, never to the member lists, because
/// an optimization-level spelling is case-sensitive.
fn parse_scope(origin: &Path, raw: &RawField) -> HarnessResult<MarkerScope> {
    let text = raw.value.trim();
    if text.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_SCOPE,
            "the scope is empty; a marker with no scope could not be matched against any cell, so \
             the divergence it describes could never be recognised",
        ));
    }

    let mut oracles: Option<Vec<Oracle>> = None;
    let mut targets: Option<Vec<Target>> = None;
    let mut opt_levels: Option<Vec<OptLevel>> = None;

    for clause in text
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let lowered = clause.to_ascii_lowercase();
        if let Some(dimension) = lowered.strip_prefix("all ") {
            match dimension.trim() {
                "oracle" | "oracles" => {
                    set_dimension(&mut oracles, Oracle::ALL.to_vec(), origin, raw, "oracle")?;
                }
                "target" | "targets" => {
                    set_dimension(&mut targets, Target::ALL.to_vec(), origin, raw, "target")?;
                }
                "opt level"
                | "opt levels"
                | "opt_level"
                | "opt_levels"
                | "optimization level"
                | "optimization levels" => {
                    set_dimension(
                        &mut opt_levels,
                        OptLevel::ALL.to_vec(),
                        origin,
                        raw,
                        "optimization-level",
                    )?;
                }
                other => {
                    return Err(key_error(
                        origin,
                        raw.line,
                        KEY_MARKER_SCOPE,
                        format!(
                            "`all {other}` names no dimension; the whole-dimension forms are \
                             `all oracles`, `all targets` and `all opt levels`"
                        ),
                    ));
                }
            }
            continue;
        }

        let items = comma_items(clause);
        if let Some(parsed) = parse_uniform(&items, Oracle::parse) {
            set_dimension(
                &mut oracles,
                canonical(&Oracle::ALL, &parsed),
                origin,
                raw,
                "oracle",
            )?;
            continue;
        }
        if let Some(parsed) = parse_uniform(&items, Target::parse) {
            set_dimension(
                &mut targets,
                canonical(&Target::ALL, &parsed),
                origin,
                raw,
                "target",
            )?;
            continue;
        }
        if let Some(parsed) = parse_uniform(&items, OptLevel::parse) {
            set_dimension(
                &mut opt_levels,
                canonical(&OptLevel::ALL, &parsed),
                origin,
                raw,
                "optimization-level",
            )?;
            continue;
        }
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_SCOPE,
            format!(
                "the clause {clause:?} does not constrain a single dimension; a clause is either \
                 `all oracles`, `all targets` or `all opt levels`, or a comma-separated list \
                 whose every item belongs to one dimension — an oracle from {}, a target from \
                 {}, or a level from {}",
                comma_separated(&Oracle::ALL.map(Oracle::label)),
                joined_target_names(),
                comma_separated(&OptLevel::ALL.map(OptLevel::flag)),
            ),
        ));
    }

    Ok(MarkerScope {
        raw: String::from(text),
        oracles: oracles.unwrap_or_else(|| Oracle::ALL.to_vec()),
        targets: targets.unwrap_or_else(|| Target::ALL.to_vec()),
        opt_levels: opt_levels.unwrap_or_else(|| OptLevel::ALL.to_vec()),
    })
}

/// Parse a marker basis into its verbatim text and the repository-relative path it cites.
///
/// The path is separated from the citation by the first comma. It must be relative and must not
/// climb out of the repository, because the register audit resolves it against the package root
/// and asserts the document exists — an absolute or climbing path would let a marker cite
/// something outside the repository, which is no documented basis at all.
fn parse_basis(origin: &Path, raw: &RawField) -> HarnessResult<(String, PathBuf)> {
    let text = raw.value.trim();
    if text.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_BASIS,
            "the basis is empty; a divergence may be classified as expected only when a \
             limitation the repository already documents authorises it",
        ));
    }
    let (path_text, citation) = text.split_once(',').ok_or_else(|| {
        key_error(
            origin,
            raw.line,
            KEY_MARKER_BASIS,
            format!(
                "{text:?} is not a basis; write a repository-relative file path, then a comma, \
                 then the section or description that authorises the marker"
            ),
        )
    })?;
    let path_text = path_text.trim();
    let citation = citation.trim();
    if path_text.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_BASIS,
            "the basis names no file before its comma; the register audit asserts that the cited \
             document exists, so the citation must begin with a path",
        ));
    }
    if citation.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_BASIS,
            "the basis names a file but cites no section within it; a whole-document citation \
             cannot be checked by a reader, which is the point of recording it",
        ));
    }
    let path = Path::new(path_text);
    if path.is_absolute() {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_BASIS,
            format!(
                "{path_text:?} is absolute; a basis cites a repository artifact, so the path is \
                 relative to the package root"
            ),
        ));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_BASIS,
            format!(
                "{path_text:?} climbs out of the repository; a basis cites a repository artifact, \
                 so the path may not contain a parent-directory component"
            ),
        ));
    }
    Ok((String::from(text), path.to_path_buf()))
}

/// Parse the optional expected-divergence marker block.
///
/// Either all five marker keys are present or none is. A partial marker is a hard error because
/// each part carries weight the others cannot: without an identifier the register cannot be
/// cross-checked, without a class the classifier cannot match the divergence, without a scope it
/// cannot tell which cells are covered, without a basis there is no documented authority, and
/// without the observation a reader cannot tell whether what they are seeing is what was marked.
fn parse_marker(
    fields: &[(&'static KeySpec, RawField)],
    origin: &Path,
    program_path: &Path,
) -> HarnessResult<Option<ExpectedDivergence>> {
    let present: Vec<&str> = MARKER_KEYS
        .iter()
        .copied()
        .filter(|name| field(fields, name).is_some())
        .collect();
    if present.is_empty() {
        return Ok(None);
    }
    if present.len() != MARKER_KEYS.len() {
        let missing: Vec<&str> = MARKER_KEYS
            .iter()
            .copied()
            .filter(|name| field(fields, name).is_none())
            .collect();
        let anchor = present
            .iter()
            .filter_map(|name| field(fields, name))
            .map(|found| found.line)
            .min()
            .unwrap_or(1);
        return Err(key_error(
            origin,
            anchor,
            KEY_MARKER_ID,
            format!(
                "the expected-divergence marker is partial: {} is absent. A marker is written in \
                 full or not at all, because each part carries weight the others cannot — the \
                 identifier is what the register cross-check matches, the class and scope are what \
                 decide which cells the marker covers, the basis is the documented authority, and \
                 the observation is what lets a reader recognise the divergence",
                comma_separated(&missing)
            ),
        ));
    }

    let id_field = required_field(fields, origin, KEY_MARKER_ID)?;
    let id = id_field.value.trim();
    if id.is_empty() {
        return Err(key_error(
            origin,
            id_field.line,
            KEY_MARKER_ID,
            "the marker identifier is empty; the register cross-check matches markers by \
             identifier in both directions, so an unnamed marker could never be audited",
        ));
    }
    if id.chars().any(char::is_whitespace) {
        return Err(key_error(
            origin,
            id_field.line,
            KEY_MARKER_ID,
            format!(
                "{id:?} contains whitespace; an identifier is a single token, by convention \
                 `XD-<AREA>-<TOPIC>-<NNN>`, so that the register can be searched for it exactly"
            ),
        ));
    }

    let class_field = required_field(fields, origin, KEY_MARKER_CLASS)?;
    let class = DivergenceClass::parse(&class_field.value).ok_or_else(|| {
        key_error(
            origin,
            class_field.line,
            KEY_MARKER_CLASS,
            format!(
                "{:?} names no divergence class; the set is closed at six members so that the \
                 classification table stays total, and a newly observed shape is mapped onto one \
                 of them rather than appended as a seventh. The classes are: {}",
                class_field.value.trim(),
                comma_separated(&DivergenceClass::ALL.map(DivergenceClass::label))
            ),
        )
    })?;

    let scope_field = required_field(fields, origin, KEY_MARKER_SCOPE)?;
    let scope = parse_scope(origin, scope_field)?;

    let basis_field = required_field(fields, origin, KEY_MARKER_BASIS)?;
    let (basis, basis_path) = parse_basis(origin, basis_field)?;

    let observed_field = required_field(fields, origin, KEY_MARKER_OBSERVED)?;
    require_non_empty(
        origin,
        observed_field,
        KEY_MARKER_OBSERVED,
        "record the divergence as observed, so that a reader can tell whether what they are \
         seeing is what was marked and so that an unexpected success is recognisable when the \
         divergence disappears",
    )?;

    Ok(Some(ExpectedDivergence {
        id: String::from(id),
        class,
        scope,
        basis,
        basis_path,
        observed: observed_field.value.clone(),
        program_path: program_path.to_path_buf(),
    }))
}

// ---------------------------------------------------------------------------
// Record assembly
// ---------------------------------------------------------------------------

/// Every feature-area directory name, as one comma-separated line for a diagnostic.
fn known_area_names() -> String {
    let names: Vec<&str> = AREAS.iter().map(|area| area.directory).collect();
    comma_separated(&names)
}

/// The record's file stem, which the `program` key must match.
fn record_stem(origin: &Path) -> HarnessResult<&str> {
    origin
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            record_error(
                origin,
                "the record path has no readable file stem; every record is `<program>.expected` \
                 beside `<program>.c`, so a name that cannot be read is a corpus defect",
            )
        })
}

/// The name of the directory the record lives in, which the `area` key must match.
fn record_directory(origin: &Path) -> HarnessResult<&str> {
    origin
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            record_error(
                origin,
                "the record path has no readable parent directory name; every record lives \
                 directly inside its feature-area directory",
            )
        })
}

/// Validate the three command templates against each other and against the one fact that makes
/// the cross arm of the reference-compiler oracle work at all.
fn validate_templates(
    origin: &Path,
    bcc: &RawField,
    reference: &RawField,
    run: &RawField,
) -> HarnessResult<()> {
    require_placeholders(
        origin,
        bcc,
        "bcc_command",
        &[PLACEHOLDER_BCC, PLACEHOLDER_SOURCE, PLACEHOLDER_OUTPUT],
    )?;
    require_placeholders(
        origin,
        reference,
        "ref_command",
        &[PLACEHOLDER_SOURCE, PLACEHOLDER_OUTPUT],
    )?;
    require_placeholders(origin, run, "run_command", &[PLACEHOLDER_OUTPUT])?;

    // Every accepted reference-compiler spelling begins with the bare placeholder, so one
    // containment test covers the templated form, the four concrete forms and the bare form.
    if !reference.value.contains(PLACEHOLDER_REFERENCE_BARE) {
        return Err(key_error(
            origin,
            reference.line,
            "ref_command",
            format!(
                "the template {:?} never names the reference compiler; write \
                 {PLACEHOLDER_REFERENCE_TEMPLATED}, which selects the driver binary for the \
                 cell's target",
                reference.value
            ),
        ));
    }
    for spelling in REFERENCE_FORBIDDEN_SPELLINGS {
        if reference.value.contains(spelling) {
            return Err(key_error(
                origin,
                reference.line,
                "ref_command",
                format!(
                    "the template carries {spelling:?}; the reference compiler has no \
                     target-selection flag — that spelling belongs to a different compiler family \
                     and was measured to be rejected — so the cross arm selects a cross-driver \
                     binary through {PLACEHOLDER_REFERENCE_TEMPLATED} instead"
                ),
            ));
        }
    }
    if reference.value.contains(PLACEHOLDER_BCC) {
        return Err(key_error(
            origin,
            reference.line,
            "ref_command",
            format!(
                "the reference template names {PLACEHOLDER_BCC}; a record that built both sides \
                 with the same compiler would report agreement while comparing nothing"
            ),
        ));
    }
    if bcc.value.contains(PLACEHOLDER_REFERENCE_BARE) {
        return Err(key_error(
            origin,
            bcc.line,
            "bcc_command",
            format!(
                "the compiler-under-test template names {PLACEHOLDER_REFERENCE_BARE}; a record \
                 that built both sides with the same compiler would report agreement while \
                 comparing nothing"
            ),
        ));
    }
    Ok(())
}

/// Turn parsed fields into a validated manifest.
///
/// Validation is strict throughout, and every rejection is a corpus defect rather than a
/// condition an environment can legitimately produce. That distinction is what the whole suite
/// rests on: a divergence is evidence about the compiler only when the program that provoked it
/// is known to be well formed, undefined-behaviour-free, and compared under oracles whose
/// exclusions each carry a recorded reason.
fn assemble(fields: Vec<(&'static KeySpec, RawField)>, origin: &Path) -> HarnessResult<Manifest> {
    for spec in KEYS
        .iter()
        .filter(|spec| matches!(spec.presence, Presence::Required))
    {
        if field(&fields, spec.name).is_none() {
            return Err(record_error(
                origin,
                format!(
                    "required key `{}` is absent from the record, so there is no line to point \
                     at; expected {}",
                    spec.name,
                    spec.kind.required_form(spec.name)
                ),
            ));
        }
    }

    let program_field = required_field(&fields, origin, "program")?;
    let stem = record_stem(origin)?;
    if program_field.value != stem {
        return Err(key_error(
            origin,
            program_field.line,
            "program",
            format!(
                "the record claims to govern {:?} but its own file stem is {stem:?}; the two must \
                 agree, because this is the cheapest guard there is against a record copied from \
                 another program and only partly edited",
                program_field.value
            ),
        ));
    }

    let area_field = required_field(&fields, origin, "area")?;
    let directory = record_directory(origin)?;
    if area_field.value != directory {
        return Err(key_error(
            origin,
            area_field.line,
            "area",
            format!(
                "the record claims the area {:?} but lives in {directory:?}; the two must agree, \
                 because the area names the report file the program's verdicts are written to",
                area_field.value
            ),
        ));
    }
    if AreaSpec::lookup(&area_field.value).is_none() {
        return Err(key_error(
            origin,
            area_field.line,
            "area",
            format!(
                "{:?} is not a feature area of the corpus; the areas are: {}",
                area_field.value,
                known_area_names()
            ),
        ));
    }

    let description_field = required_field(&fields, origin, "description")?;
    require_non_empty(
        origin,
        description_field,
        "description",
        "the description is used verbatim in the report row for every cell of this program, so an \
         empty one would leave the summary unreadable",
    )?;

    let targets = parse_targets(origin, required_field(&fields, origin, "targets")?)?;
    let opt_levels = parse_opt_levels(origin, required_field(&fields, origin, "opt_levels")?)?;
    let shared_flags =
        parse_shared_flags(origin, required_field(&fields, origin, "shared_flags")?)?;

    let bcc_field = required_field(&fields, origin, "bcc_command")?;
    let ref_field = required_field(&fields, origin, "ref_command")?;
    let run_field = required_field(&fields, origin, "run_command")?;
    validate_templates(origin, bcc_field, ref_field, run_field)?;

    let expect_exit = parse_expect_exit(origin, required_field(&fields, origin, "expect_exit")?)?;

    let mut enabled_oracles: Vec<Oracle> = Vec::new();
    for oracle in Oracle::ALL {
        let key = format!("oracle_{}", oracle.letter());
        let toggle_field = required_field(&fields, origin, &key)?;
        if parse_toggle(origin, toggle_field, &key)? {
            enabled_oracles.push(oracle);
        }
    }

    let ub_audit_flags = match field(&fields, "ub_audit_flags") {
        Some(raw) => Some(parse_ub_audit_flags(origin, raw)?),
        None => None,
    };

    let ub_notes_field = required_field(&fields, origin, "ub_notes")?;
    require_non_empty(
        origin,
        ub_notes_field,
        "ub_notes",
        "record the written argument for why this program is free of undefined and unspecified \
         behaviour; the audit gate is the machine half of that guarantee and this is the human \
         half, and without it a divergence cannot be read as evidence about the compiler",
    )?;

    let impl_defined_notes = match field(&fields, "impl_defined_notes") {
        Some(raw) => {
            require_non_empty(
                origin,
                raw,
                "impl_defined_notes",
                "the key is present but empty; a narrowing of coverage is recorded with its \
                 reason, and an empty reason is indistinguishable from no reason at all",
            )?;
            Some(raw.value.clone())
        }
        None => None,
    };

    let expected_stdout_field = required_field(&fields, origin, "expected_stdout")?;
    require_non_empty(
        origin,
        expected_stdout_field,
        "expected_stdout",
        "record the golden stdout; no program in the corpus passes merely by compiling, and an \
         empty golden record would turn the golden-record oracle into a no-op for this program",
    )?;

    let mut narrowings: Vec<String> = Vec::new();
    if targets.len() < Target::ALL.len() {
        let names: Vec<&str> = targets.iter().map(|target| target.short_name()).collect();
        narrowings.push(format!(
            "the target list is restricted to {} of {} targets ({})",
            targets.len(),
            Target::ALL.len(),
            comma_separated(&names)
        ));
    }
    let disabled: Vec<Oracle> = Oracle::ALL
        .iter()
        .copied()
        .filter(|oracle| !enabled_oracles.contains(oracle))
        .collect();
    if !disabled.is_empty() {
        let labels: Vec<&str> = disabled.iter().map(|oracle| oracle.label()).collect();
        narrowings.push(format!("{} disabled", comma_separated(&labels)));
    }
    if ub_audit_flags.is_some() {
        narrowings.push(String::from(
            "the warning gate deviates from the default gate",
        ));
    }
    if !narrowings.is_empty() && impl_defined_notes.is_none() {
        return Err(record_error(
            origin,
            format!(
                "this record narrows its coverage — {} — but carries no `impl_defined_notes`, so \
                 there is no line to point at. A feature is never dropped because it is \
                 difficult: an exclusion may be narrowed to a single oracle or a single target, \
                 but only with the reason recorded here, which is what makes the set of \
                 comparisons deliberately not made as visible as the set that was. Add {}",
                narrowings.join("; "),
                required_form_of("impl_defined_notes")
            ),
        ));
    }

    let source = origin.with_extension(SOURCE_EXTENSION);
    let marker = parse_marker(&fields, origin, &source)?;

    Ok(Manifest {
        path: origin.to_path_buf(),
        program: program_field.value.clone(),
        area: area_field.value.clone(),
        description: description_field.value.clone(),
        targets,
        opt_levels,
        shared_flags,
        bcc_command: bcc_field.value.clone(),
        ref_command: ref_field.value.clone(),
        run_command: run_field.value.clone(),
        expect_exit,
        enabled_oracles,
        ub_audit_flags,
        ub_notes: ub_notes_field.value.clone(),
        impl_defined_notes,
        expected_stdout: expected_stdout_field.value.clone(),
        marker,
    })
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse and validate a record from text, touching no filesystem.
///
/// `origin` is the path the text came from. It is not decoration: the identity checks that tie a
/// record to the program it governs read the file stem and the containing directory from it, so
/// it must be the record's real path even when the text was obtained some other way. Keeping the
/// pure parser separate from the reader is what lets the format's behaviour be inspected — and
/// every rejection exercised — without laying a single file down.
pub fn parse_str(text: &str, origin: &Path) -> HarnessResult<Manifest> {
    let fields = parse_fields(text, origin)?;
    assemble(fields, origin)
}

/// Read, parse and validate a record from disk.
///
/// A record that cannot be read is a hard error. It is never a skip: a program whose expectations
/// cannot be loaded is a program no oracle can judge, and quietly passing over it would remove a
/// cell from the matrix without anyone being told.
pub fn load(expected_path: &Path) -> HarnessResult<Manifest> {
    let text = fs::read_to_string(expected_path).map_err(|error| {
        HarnessError::new(
            format!("reading the expectation record {}", expected_path.display()),
            format!(
                "{error}; every program in the corpus is paired with a sibling \
                 `.{RECORD_EXTENSION}` record holding its command templates and its golden \
                 output, and a record that cannot be read is a corpus defect rather than a reason \
                 to skip the program"
            ),
        )
    })?;
    parse_str(&text, expected_path)
}

/// Load the record that governs a program, given the program's own path.
///
/// The record is the program's sibling, differing only in extension. A program with no sibling
/// record is a hard error and never a skip, for the same reason: the pairing is what makes the
/// requirement "source file, build commands, and expected output recorded together" true, and a
/// program without its half of that pairing cannot be reproduced by hand or judged by the
/// golden-record oracle.
pub fn load_for_source(c_path: &Path) -> HarnessResult<Manifest> {
    let extension = c_path.extension().and_then(|value| value.to_str());
    if extension != Some(SOURCE_EXTENSION) {
        return Err(HarnessError::new(
            format!("resolving the expectation record for {}", c_path.display()),
            format!(
                "the path is not a `.{SOURCE_EXTENSION}` program, so it has no sibling record; \
                 every corpus program is a `.{SOURCE_EXTENSION}` file paired with a \
                 `.{RECORD_EXTENSION}` record of the same stem"
            ),
        ));
    }
    let record = c_path.with_extension(RECORD_EXTENSION);
    if !record.is_file() {
        return Err(HarnessError::new(
            format!("resolving the expectation record for {}", c_path.display()),
            format!(
                "the sibling record {} does not exist; a program without its record is a corpus \
                 defect and is reported rather than skipped, because skipping it would silently \
                 remove every one of its cells from the matrix",
                record.display()
            ),
        ));
    }
    load(&record)
}

/// Every program in one feature area, sorted so that run order is deterministic.
///
/// The scan is strict about what an area directory may contain. Programs are the `.c` files;
/// expectation records, area notes, the fixture header and shell tooling are recognised companions
/// and ignored; dot-prefixed entries and nested directories are ignored. **Anything else is a hard
/// error**, and that strictness is load-bearing rather than fussy: it is what mechanically
/// enforces the rule that the corpus tree contains no `.rs` file anywhere, which is in turn what
/// keeps the corpus invisible to the build system and the suite free of any package-manifest
/// change. It also catches the mistake that would otherwise be invisible — a program misnamed with
/// the wrong extension, silently absent from the matrix.
pub fn discover_area(area: &str) -> HarnessResult<Vec<PathBuf>> {
    let name = area.trim();
    if AreaSpec::lookup(name).is_none() {
        return Err(HarnessError::new(
            format!("discovering the programs of feature area {name:?}"),
            format!(
                "{name:?} is not a feature area of the corpus; the areas are: {}",
                known_area_names()
            ),
        ));
    }
    let directory = corpus_root().join(name);
    let entries = fs::read_dir(&directory).map_err(|error| {
        HarnessError::new(
            format!("discovering the programs of feature area {name:?}"),
            format!(
                "{} could not be read: {error}; the corpus is discovered by scanning this \
                 directory, so an area that is absent or unreadable is a corpus defect",
                directory.display()
            ),
        )
    })?;

    let mut programs: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!(
                    "an entry of {} could not be read: {error}",
                    directory.display()
                ),
            )
        })?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                HarnessError::new(
                    format!("discovering the programs of feature area {name:?}"),
                    format!(
                        "{} has a name that is not valid UTF-8; every corpus path is written into \
                         reports and reproduction commands, so it must be readable text",
                        path.display()
                    ),
                )
            })?;
        if file_name.starts_with('.') {
            continue;
        }
        let metadata = fs::metadata(&path).map_err(|error| {
            HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!("{} could not be inspected: {error}", path.display()),
            )
        })?;
        if metadata.is_dir() {
            continue;
        }
        match path.extension().and_then(|value| value.to_str()) {
            Some(SOURCE_EXTENSION) => programs.push(path),
            Some(companion) if AREA_COMPANION_EXTENSIONS.contains(&companion) => {}
            _ => {
                return Err(HarnessError::new(
                    format!("discovering the programs of feature area {name:?}"),
                    format!(
                        "{file_name:?} is neither a `.{SOURCE_EXTENSION}` program nor a \
                         recognised companion ({}); an unrecognised file in an area directory is \
                         reported rather than ignored, because a program misnamed with the wrong \
                         extension would otherwise vanish from the matrix without a word, and \
                         because the corpus tree must contain no build-system target of any kind",
                        comma_separated(AREA_COMPANION_EXTENSIONS)
                    ),
                ));
            }
        }
    }

    if programs.is_empty() {
        return Err(HarnessError::new(
            format!("discovering the programs of feature area {name:?}"),
            format!(
                "{} contains no `.{SOURCE_EXTENSION}` program; an empty feature area is a corpus \
                 defect, because no area of the corpus is optional and none may be skipped",
                directory.display()
            ),
        ));
    }
    // Every path shares one parent, so sorting the paths is a sort by file name.
    programs.sort();
    Ok(programs)
}

/// Every program in the corpus, in feature-area table order and then file-name order.
///
/// Consumed by the undefined-behaviour audit, which drives every program through both gates, and
/// by the register cross-check. A feature area that is absent or empty fails here rather than
/// shrinking the run quietly.
pub fn discover_all() -> HarnessResult<Vec<PathBuf>> {
    let mut programs: Vec<PathBuf> = Vec::new();
    for area in AREAS {
        programs.extend(discover_area(area.directory)?);
    }
    Ok(programs)
}

/// Every expected-divergence marker in the corpus, in area order and then file order.
///
/// This is the corpus side of the register consistency loop. The infrastructure test that audits
/// the register consumes this enumeration to assert, in both directions, that every marker
/// identifier appears in the committed register and that every register entry corresponds to a
/// real marker, and to assert that every cited basis names a document that actually exists —
/// which is what stops the marker set from decaying into stale documentation. The assertions are
/// the driver's; the enumeration and the resolvable basis path are this module's.
///
/// A duplicate identifier is a hard error. Two markers sharing a name would make the register
/// cross-check ambiguous in one direction and satisfiable by the wrong program in the other, so
/// the duplicate is reported with both owning programs named.
pub fn all_markers() -> HarnessResult<Vec<ExpectedDivergence>> {
    let mut markers: Vec<ExpectedDivergence> = Vec::new();
    for program in discover_all()? {
        let manifest = load_for_source(&program)?;
        if let Some(marker) = manifest.marker() {
            if let Some(previous) = markers.iter().find(|known| known.id == marker.id) {
                return Err(HarnessError::new(
                    "enumerating the corpus expected-divergence markers",
                    format!(
                        "the identifier {:?} is used by both {} and {}; identifiers are unique \
                         across the corpus, because the register cross-check matches markers by \
                         identifier in both directions and a duplicate would make one direction \
                         ambiguous and the other satisfiable by the wrong program",
                        marker.id,
                        previous.program_label(),
                        marker.program_label()
                    ),
                ));
            }
            markers.push(marker.clone());
        }
    }
    Ok(markers)
}
