//! Parser for the `.expected` expectation-record format, corpus discovery, and
//! expected-divergence-marker enumeration.
//!
//! Every corpus program is paired with a sibling `<program>.expected` record holding its cell
//! matrix, the literal command templates that build and run it, the expected exit status, the
//! written undefined-behaviour-freedom argument, and the golden stdout. A maintainer can
//! therefore reproduce any cell from the `.c` file plus its record with no harness at all, and
//! the recorded stdout is what supplies oracle (c) — the one comparison that still fails when
//! both compilers change behaviour in the same direction, where oracle (a) reports agreement.
//!
//! # Grammar
//!
//! - **Comment** — a line whose first non-whitespace character is `#`. A `#` inside a value is
//!   data.
//! - **Scalar** — `key = value`, split on the first `=`, both sides trimmed.
//! - **Heredoc** — `key <<END`, then body lines captured verbatim, then a line that is exactly
//!   `END`. Inside a heredoc nothing is trimmed, no `#` is a comment and no `=` is a separator,
//!   which is what lets a golden stdout reproduce program output byte for byte including
//!   leading spaces and lines that begin with `#`.
//! - A line opens a heredoc when `<<` appears before any `=`, so a scalar value may contain
//!   `<<` and an opener needs no `=`.
//! - Keys are case-sensitive and drawn from `[a-z0-9_.]`; the dot serves the
//!   `expected_divergence.*` family.
//!
//! Lines are split with [`str::lines`], so a record terminated with `"\r\n"` parses
//! identically to one terminated with `"\n"`. A stray carriage return per line would otherwise
//! fail every golden-record comparison while saying nothing about the compiler.
//!
//! ## Trailing-newline convention
//!
//! **A heredoc value is the concatenation of its body lines with a newline appended to each**, so
//! a body of `["a", "b"]` parses to `"a\nb\n"` and an empty body parses to `""`.
//!
//! This is the format's highest-risk detail, because oracle (c) compares a recorded stdout
//! against a captured stream byte for byte. The rule runs in this direction so that a captured
//! stream — which always ends in a newline, since every corpus program's last `printf` does — is
//! byte-identical to the parsed value, and so that the maintenance regeneration script can write
//! a stream verbatim between the opener and `END` with no fix-ups. A stream that does not end in
//! a newline cannot be represented at all; that is a stated limitation rather than a silent
//! truncation, and a program producing one is outside the corpus by construction.
//!
//! # The templates are a claim about the cell, so they are validated structurally
//!
//! A record does not merely mention its command lines; it asserts that those lines rebuild and
//! rerun any cell of its matrix. Validation therefore checks structure and not just the
//! presence of a placeholder somewhere in the line, because each of the following, if
//! unchecked, lets a record describe a cell other than the one the matrix says it runs:
//!
//! - the compiler-under-test line must select the cell's target with `--target <triple>`, or a
//!   record claiming four targets would record one line that builds only the default;
//! - both build lines must carry `<opt>`, or a record claiming a three-level sweep would record
//!   one line that builds only the default level, and a literal level written in its place is
//!   rejected for the same reason;
//! - both build lines must write `-o <out>`, with the placeholder immediately after the flag,
//!   and must carry `-static`, because the harness executes the artifact at the path it asked
//!   for and the three emulated targets need static linkage to run at all;
//! - the reference line must name a per-target driver, because on that side the driver binary
//!   *is* the target selection; the bare `$REF_CC` names no target and is rejected;
//! - the run line must be exactly `<runner> <out>`, or a cross-target cell would be recorded as
//!   executing a foreign binary directly instead of through its emulator;
//! - every flag either line passes must appear in `shared_flags`, and every flag `shared_flags`
//!   declares must appear in both lines. The first direction stops an unverified flag being
//!   smuggled past the shared-flag discipline by writing it into a template; the second stops a
//!   record documenting an invocation that never happens.
//!
//! # Failure posture
//!
//! Every malformed line and every violated obligation is a hard error naming the path, the
//! one-based line number and what was expected. Nothing is dropped silently: a quietly ignored
//! `expected_stdout` would turn oracle (c) into a no-op, and a quietly ignored obligation would
//! let coverage narrow without the recorded reason that makes the narrowing reviewable.
//!
//! The same posture governs the *shape* of a value, not only its presence. An empty element in
//! a comma-separated list is rejected rather than dropped, because dropping it shrinks the
//! matrix or widens a marker scope without saying so. Likewise nothing that could carry a
//! program is passed over during discovery: a dot-prefixed entry, a nested directory, a symbolic
//! link and an unrecognised extension are each a hard error, because each is a way for a
//! committed program to be absent from the matrix while the run still reports success.
//!
//! # Corpus containment
//!
//! Every path this module reads is checked before a byte is read, through the harness root's
//! shared check: it must be an absolute path to a regular, non-symbolic-link file that resolves
//! beneath the canonical corpus root, and a symbolic link is refused rather than followed. That
//! matters most for records, because a record dictates the command templates a cell executes and
//! the golden output it is judged against, so one read from outside the corpus would decide what
//! gets compiled and what counts as correct while every report still showed the corpus path. The
//! resolved path is what the manifest carries onward. [`parse_str`] is deliberately exempt
//! because it touches no filesystem at all; keeping it pure is what lets every rejection above be
//! exercised without laying a file down.
//!
//! # Read-only with respect to the corpus
//!
//! There is deliberately no writer, fixer or update-in-place helper here. Golden records are
//! regenerated only through the maintenance script under `tests/conformance/tools/`, never during
//! a test run, so a wrong answer cannot quietly become the new expectation.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::{
    canonical_corpus_root, comma_separated, corpus_root, ensure_within, findings_root,
    fnv1a64_bytes, is_forbidden_for_side, is_ub_audit_gate_member, joined_target_names,
    manifest_dir, must_escape_for_report, posix_command_line, read_file_bounded,
    require_contained_corpus_file, require_regular_file, sanitize_text_for_report, shown_path,
    stable_digest, ub_audit_gate_required, AreaSpec, CellKey, CompilerSide, DivergenceClass,
    HarnessError, HarnessResult, OptLevel, Oracle, Target, AREAS, BCC_TARGET_FLAG,
    BCC_TARGET_SELECTORS, DIFFERENTIAL_FLAGS_MINIMAL, DIGEST_HEX_DIGITS, EXTENSION_AREA,
    MAX_INSPECTED_FILE_BYTES, SHARED_FLAGS_VERIFIED, UB_AUDIT_GATE_DEFAULT,
    UB_AUDIT_GATE_MANDATORY, UB_AUDIT_GATE_REMOVABLE, UB_GATE_DEFAULT, UB_GATE_WITHOUT_CONVERSION,
    UB_GATE_WITHOUT_PEDANTIC,
};

const HEREDOC_OPENER: &str = "<<";

/// The one accepted heredoc terminator.
///
/// A body line whose *trimmed* text is this token but which is not exactly this token is
/// reported at that line, rather than left to surface as an unterminated-heredoc error at end of
/// file, because invisible trailing whitespace is hard to see in an editor.
const HEREDOC_TERMINATOR: &str = "END";

const SOURCE_EXTENSION: &str = "c";

const RECORD_EXTENSION: &str = "expected";

/// The corpus directory holding committed, curated finding artifacts.
///
/// A sibling of the feature-area directories rather than a child of one, which is why
/// [`AREA_COMPANION_EXTENSIONS`] can stay as narrow as it is. [`load_replay`] reads beneath it so a
/// reviewed, committed reproducer can be replayed by the harness exactly like a freshly generated
/// one; nothing in the suite writes there.
const CURATED_FINDINGS_DIR_NAME: &str = "findings";

/// Extensions a regular file may carry inside a feature-area directory without being a program.
///
/// Exactly one: the expectation record. A feature-area directory holds programs and the records
/// that govern them, and nothing else at all.
///
/// The list is deliberately this short. Admitting a header, a script or a document here would
/// permit a per-program dependency to sit inside an area, and that would quietly dismantle two
/// guarantees the suite is built on. A program must be reproducible from its own source file and
/// its record alone — that is what makes "source file, build commands, and expected output
/// recorded together" literally true — and every program therefore hand-declares the libc
/// prototypes it needs rather than including a hosted header: `printf` in nearly every program,
/// and `_Noreturn void exit(int);` as well in the `_Noreturn` program. The single sanctioned
/// inclusion is area 07's `<stdarg.h>`, which is freestanding, is shipped by both compilers, and
/// cannot be worked around at all, because a variadic function cannot be written without it. A
/// header beside the programs is an invitation to break that, and a shared header would
/// additionally fail against a compiler under test that bundles no `stdio.h`, producing a
/// divergence caused by the test rather than by the compiler. A script beside the programs is
/// worse still: the area directory is scanned, and something that looks like a per-area build step
/// is the beginning of a corpus that no longer builds the way its records say it does.
///
/// The corpus's genuine companions — the fixture support tree, the tooling tree, the findings tree
/// and the two registers — are siblings of the area directories rather than children, so nothing
/// legitimate is displaced by this rule.
///
/// Any other extension is a hard error, which also catches a `.rs` file dropped into the top level
/// of an area directory: the corpus has to stay invisible to the build system for the suite to
/// need no package-manifest change. [`discover_area`] does not descend into nested directories, so
/// this check reaches the top level of each area and no further.
const AREA_COMPANION_EXTENSIONS: &[&str] = &[RECORD_EXTENSION];

/// The only dot-prefixed entry names an area directory may contain.
///
/// Whitelisted by exact name rather than by pattern, and deliberately short. A blanket skip of
/// every dot-prefixed entry would mean that renaming a program to a dot-prefixed name silently
/// deleted twelve cells from the matrix while the run still reported success, so the general rule
/// is rejection and this is the single, named exception: the placeholder that lets an
/// otherwise-empty directory be tracked by version control.
const AREA_PLACEHOLDER_NAMES: &[&str] = &[".gitkeep"];

/// Highest exit status a record may expect.
///
/// The operating system truncates a larger value — a program returning three hundred was
/// measured to exit with status forty-four — so a record that expected such a value would be
/// asserting something the platform cannot deliver.
const EXIT_STATUS_MAX: i32 = 125;

const TOGGLE_ENABLED: &str = "enabled";

const TOGGLE_DISABLED: &str = "disabled";

/// The only flags a record's `shared_flags` may name.
///
/// This is deliberately narrower than [`SHARED_FLAGS_VERIFIED`], and the difference is the whole
/// point. The verified set answers "which flags mean the same thing to both compilers", which is
/// a question about the *compilers*. This set answers "which of those may a data file choose",
/// which is a question about *authority*, and the two answers are not the same:
///
/// - Every entry here is a **switch**: it carries no value, names no path, and cannot be made to
///   name one. A record can therefore change how a program is optimized and linked, and nothing
///   else.
/// - Every flag excluded from here is excluded because it takes a value. A value is either a
///   path or a macro definition, and a record that could supply one could choose where the
///   artifact is written ([`VALUE_TAKING_SHARED_FLAGS`]), which header directory is searched, or
///   what the preprocessor believes — none of which is a decision the expectation record layer
///   is entitled to make, because the harness already derives the output path from the cell's
///   workspace and the source path from the cell itself.
/// - `-c` is excluded for a different reason: it stops the pipeline before a runnable artifact
///   exists, and all three oracles compare the behaviour of a program that ran. A record naming
///   it would describe a cell no oracle could judge.
const RECORD_SHARED_FLAGS_PERMITTED: &[&str] = &["-O0", "-O1", "-O2", "-g", "-static", "-fPIC"];

/// The flag every record must name, because every artifact in the corpus is linked statically.
///
/// Static linkage is not a preference: it is the one linkage mode both compilers spell
/// identically, and it is what lets an emulated target execute with no sysroot and no dynamic
/// loader configuration. A record that omitted it would silently describe a dynamically linked
/// artifact whose cross-target cells could not run at all.
const MANDATORY_SHARED_FLAG: &str = FLAG_STATIC;

/// Verified shared flags that take a value, listed so a record naming one can be refused with a
/// diagnostic that explains which half of the problem it is.
///
/// Both spellings are refused. The **bare** spelling is refused because it consumes the
/// following argument vector element, so `-o` at the end of a flag list silently swallows the
/// next flag the harness appends and redirects the build; the **attached** spelling is refused
/// because it carries the value inline, so `-o../../outside` names a path outside the cell's
/// workspace directly.
const VALUE_TAKING_SHARED_FLAGS: &[&str] = &["-o", "-I", "-D", "-U", "-L", "-l"];

/// The output-selection flag, and the first member of [`DIFFERENTIAL_FLAGS_MINIMAL`].
///
/// It is structural rather than stylistic: the harness executes the artifact at the path it
/// asked for, so a template that does not write the artifact there describes a cell that
/// cannot be run.
const FLAG_OUTPUT: &str = "-o";

/// The static-linkage flag, and the second member of [`DIFFERENTIAL_FLAGS_MINIMAL`].
///
/// Static linkage is the one linkage mode both compilers spell identically, and it is what
/// lets an emulated target execute with no sysroot and no dynamic-loader configuration. A
/// record that omitted it would describe a cell whose three non-native targets could not run.
const FLAG_STATIC: &str = "-static";

/// The compiler-under-test's target-selection flag.
///
/// This flag is legitimate in the compiler-under-test template and nowhere else. The
/// cross-backend oracle compares the compiler against itself, so "both compilers honour the
/// flag with the same meaning" is trivially satisfied there; the reference compiler has no
/// target-selection flag at all, which is why its cross arm selects a driver binary instead.
///
/// An alias for the harness root's spelling rather than a second literal: the flag table there is
/// the single authority for what the selector is called, and two literals could drift apart.
const FLAG_TARGET_SELECT: &str = BCC_TARGET_FLAG;

/// Flags that stop a build short of a runnable executable.
///
/// The whole corpus is judged by executing what was built, so a record declaring one of these
/// as a shared flag would describe cells that produce an object file, an assembly listing or
/// preprocessed text and can never be run. `-S` and `-E` are additionally rejected as
/// forbidden in a differential invocation; listing all three here keeps this check independent
/// of the contents of that table rather than relying on it.
const FLAGS_WITHOUT_EXECUTABLE: &[&str] = &["-c", "-S", "-E"];

// The three sanctioned warning gates and the one area permitted to drop `-pedantic` are defined
// in the harness root, so that the audit module, which runs the gate, and this module, which
// validates a record's claim about it, read one authority rather than two that could drift apart.

/// Largest expectation record the parser will read, in bytes.
///
/// A record holds a small set of scalar fields, its written notes, and a golden stdout. Measured
/// across the committed corpus: the largest golden stdout is 2,220 bytes over 126 lines, the
/// largest single notes field a few kilobytes, and the largest whole record roughly ten kilobytes
/// — an order of magnitude below this bound, so the limit costs the corpus nothing while denying
/// an adversarial or corrupt file the ability to exhaust memory. The exact figures for the notes
/// and the whole record are deliberately given as an order rather than a byte count, because
/// prose is edited and a stated byte count would go stale; the golden-stdout figures are exact,
/// because a golden record is immutable except through the regeneration tool. The size is checked
/// against the file's metadata *before* it is opened and enforced again on the reader, because a
/// file can grow between the two.
const RECORD_BYTES_MAX: u64 = 256 * 1024;

/// Longest single line the parser accepts, in bytes.
///
/// Every line of the format is a key, a short scalar, or one line of a golden stdout. A line
/// longer than this is not a record the corpus could contain, and accepting one would let a
/// single line defeat the whole-file bound by arriving as one enormous field.
const RECORD_LINE_BYTES_MAX: usize = 8 * 1024;

/// Largest value a single field may hold, in bytes.
const FIELD_BYTES_MAX: usize = 64 * 1024;

/// Most lines a single heredoc body may hold.
const HEREDOC_LINES_MAX: usize = 4096;

/// Characters a command template's literal text may never contain.
///
/// Every one of these is grammar to a POSIX shell rather than data: the list operators, the
/// redirections, the command and parameter expansions, the quoting characters, the pattern
/// characters, the comment character and the tilde. A template is not a shell script — the
/// harness executes it as an argument vector with no shell involved — so a template containing
/// any of them is either a mistake or an attempt to make a reproduction script do something the
/// harness itself never did. Either way it is refused at parse time, which is strictly better
/// than quoting it at emit time: quoting would faithfully reproduce a command line nobody meant
/// to write.
///
/// The dollar sign is included even though it introduces the compiler placeholders, because a
/// placeholder is recognised as a **whole token** before this check is reached: by the time a
/// token is being examined as literal text, a dollar sign in it can only be a shell expansion or
/// a misspelled placeholder, and both must be refused rather than quoted. The angle brackets are
/// included for exactly the same reason.
const TEMPLATE_FORBIDDEN_CHARACTERS: &[char] = &[
    '|', '&', ';', '<', '>', '(', ')', '{', '}', '[', ']', '*', '?', '!', '`', '"', '\'', '\\',
    '#', '~', '$', '\n', '\r', '\t',
];

/// Target-selection spellings that must never appear in the reference-compiler template.
///
/// This harness resolves oracle (a)'s cross arm by selecting a cross-driver binary per target,
/// so a template that also selected a target with a flag would be selecting it twice and could
/// disagree with the driver it was handed. The `--target=` spelling belongs to a different
/// compiler family and the GCC drivers reject it outright.
///
/// Borrowed from the harness root rather than restated, because the same two spellings are the
/// ones the compiler-under-test side requires. Two tables would be two authorities on one fact,
/// and the moment they disagreed one side of the split would be enforced and the other would not.
const REFERENCE_FORBIDDEN_SPELLINGS: &[&str] = BCC_TARGET_SELECTORS;

const PLACEHOLDER_BCC: &str = "$BCC";

const PLACEHOLDER_REFERENCE_TEMPLATED: &str = "$REF_CC_<TRIPLE>";

/// Bare placeholder for the reference compiler driver.
///
/// [`render_command_argv`] expands it as a synonym of the templated spelling, so a command line
/// lifted out of a findings artifact and edited by hand still renders. A record's `ref_command` is
/// held to a per-target driver instead — the canonical templated spelling, or the concrete one
/// where the record declares that single target — because the target suffix, the reference
/// compiler having no target-selection flag, is the only thing that selects a cross driver. This
/// constant is what lets `validate_templates` ask whether a template names the reference compiler
/// at all before insisting on how it is spelled.
const PLACEHOLDER_REFERENCE_BARE: &str = "$REF_CC";

const PLACEHOLDER_TRIPLE: &str = "<triple>";

const PLACEHOLDER_OPT: &str = "<opt>";

const PLACEHOLDER_SOURCE: &str = "<src>";

const PLACEHOLDER_OUTPUT: &str = "<out>";

/// Placeholder for the execution runner, which is empty on a natively executing target and the
/// target's emulator otherwise.
const PLACEHOLDER_RUNNER: &str = "<runner>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Scalar,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Required,
    /// Carried only when a documented condition applies, and then mandatory. The conditions
    /// are a restricted target list, a disabled oracle, and a warning-gate deviation.
    Conditional,
    /// Carried only as a deliberate per-program deviation or as part of the optional
    /// expected-divergence marker block.
    Optional,
}

/// One recognised key.
///
/// [`KEYS`] is the single authority on what a record may contain. An unrecognised key is rejected
/// against it rather than ignored, because an unknown key is almost always a typo, and a typo
/// that is ignored silently disables the check the key exists to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeySpec {
    name: &'static str,
    kind: FieldKind,
    presence: Presence,
}

impl KeySpec {
    const fn new(name: &'static str, kind: FieldKind, presence: Presence) -> KeySpec {
        KeySpec {
            name,
            kind,
            presence,
        }
    }
}

const KEY_MARKER_ID: &str = "expected_divergence.id";

const KEY_MARKER_CLASS: &str = "expected_divergence.class";

const KEY_MARKER_SCOPE: &str = "expected_divergence.scope";

const KEY_MARKER_BASIS: &str = "expected_divergence.basis";

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

fn lookup_key(name: &str) -> Option<&'static KeySpec> {
    KEYS.iter().find(|spec| spec.name == name)
}

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
    value: String,
    line: usize,
}

/// A heredoc that has been opened and is accumulating body lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHeredoc {
    spec: &'static KeySpec,
    line: usize,
    /// Body lines captured so far, each of which contributes its own newline to the value.
    body: Vec<String>,
}

fn key_error(origin: &Path, line: usize, key: &str, cause: impl Into<String>) -> HarnessError {
    HarnessError::new(
        format!(
            "parsing the expectation record {} at line {line}, key `{key}`",
            shown_path(origin)
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
            shown_path(origin)
        ),
        cause,
    )
}

/// Build an error about the record as a whole, used when the fault is an absent key and there
/// is therefore no line to point at.
fn record_error(origin: &Path, cause: impl Into<String>) -> HarnessError {
    HarnessError::new(
        format!("parsing the expectation record {}", shown_path(origin)),
        cause,
    )
}

/// Split a comma-separated value into trimmed items, rejecting an empty item.
///
/// An empty item is a defect in the record, not a harmless typo, so it is reported rather than
/// dropped. Dropping it silently changes the cell matrix without saying so: `targets =
/// x86_64,, aarch64` would parse as two targets and `opt_levels = -O0,,` as one level, in both
/// cases producing exactly the quiet coverage reduction the requirements forbid. A rejection
/// names the record, the line and the key, so the author sees which list is malformed rather
/// than discovering later that a cell never ran.
///
/// A value that is entirely empty yields an empty vector rather than an error, because "the list
/// is empty" is a different fault with a more specific message that each caller is better placed
/// to phrase.
fn comma_items<'a>(
    origin: &Path,
    line: usize,
    key: &str,
    value: &'a str,
) -> HarnessResult<Vec<&'a str>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut items: Vec<&'a str> = Vec::new();
    for (index, raw) in trimmed.split(',').enumerate() {
        let item = raw.trim();
        if item.is_empty() {
            return Err(key_error(
                origin,
                line,
                key,
                format!(
                    "element {} of the comma-separated list {trimmed:?} is empty; an empty \
                     element is silently dropped by a permissive parser and so would shrink the \
                     matrix without recording that it had shrunk. Remove the stray comma rather \
                     than relying on it being ignored",
                    index + 1
                ),
            ));
        }
        items.push(item);
    }
    Ok(items)
}

/// Split an **argument vector** on whitespace, the way a shell would.
///
/// This is the record format's second and last separator, and the split a key gets is decided by
/// what the value *is* rather than by preference. `shared_flags`, `ub_audit_flags` and the three
/// command templates all denote argument vectors — sequences handed to a process, where whitespace
/// is the separator every shell and every `argv` already uses — so they are split here. `targets`,
/// `opt_levels` and a marker scope denote lists of names, which is prose, so they are comma lists
/// split by [`comma_items`]. A record therefore writes `shared_flags = -static` and
/// `targets = x86_64, i686`, and the two are not interchangeable.
///
/// Consistent with that, an author who reaches for the wrong separator is told so rather than
/// quietly obeyed, and neither mistake can shrink the matrix or widen an argument vector in
/// silence:
///
/// * A comma inside an argument vector becomes **part of the token**, because whitespace is the
///   only separator here. `shared_flags = -static,` yields the single item `"-static,"`, which is
///   not a member of [`RECORD_SHARED_FLAGS_PERMITTED`], so [`parse_shared_flags`] rejects the
///   record and names the offending spelling with its stray comma visible in the quoted form.
/// * Whitespace inside a comma list likewise becomes part of the element, because
///   [`comma_items`] trims each element but never splits one. `targets = x86_64 i686` yields the
///   single element `"x86_64 i686"`, which names no target, so [`parse_targets`] rejects it.
///
/// Unlike [`comma_items`] this needs no fallible form. `split_whitespace` yields no empty item and
/// collapses any run of separators, so the empty-element fault that function must reject cannot
/// arise, and leading or trailing whitespace needs no prior trim. An entirely blank value yields an
/// empty vector, which each caller reports in its own terms — for `shared_flags` that `-static` is
/// missing, for `ub_audit_flags` that an empty deviation is not a deviation.
fn whitespace_items(value: &str) -> Vec<&str> {
    value.split_whitespace().collect()
}

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
    require_field_within_size(origin, &field, spec.name)?;
    require_report_safe_value(origin, &field, spec)?;
    fields.push((spec, field));
    Ok(())
}

/// Reject a field whose value exceeds the per-field byte bound.
///
/// The whole-file and per-line bounds already cap the total, so this bound exists for the one
/// shape they do not cover: a heredoc of many acceptable lines accumulating into one enormous
/// value. Checking all three is what makes the parser's memory use a function of the format
/// rather than of the file it is handed.
fn require_field_within_size(origin: &Path, field: &RawField, key: &str) -> HarnessResult<()> {
    if field.value.len() <= FIELD_BYTES_MAX {
        return Ok(());
    }
    Err(key_error(
        origin,
        field.line,
        key,
        format!(
            "the value is {} bytes, above the {FIELD_BYTES_MAX}-byte limit for a single field; \
             the largest golden stdout in the corpus is 2,220 bytes and the largest notes field \
             a few kilobytes, so a value this large is a corrupt or adversarial record rather \
             than one the corpus could contain",
            field.value.len()
        ),
    ))
}

/// Reject a field value carrying a character that cannot appear literally in a report.
///
/// Every field of every record is written verbatim into a Markdown report row, a tab-separated
/// summary column, a findings manifest, or a diagnostic. A control character in any of them is
/// not data: a tab forges a column and can therefore relabel a verdict, a carriage return erases
/// the line it ends, an escape introducer begins a terminal sequence that can hide a FINDING or
/// repaint it as a PASS, and a NUL truncates the value for any consumer that treats it as a
/// C string.
///
/// The rule is therefore rejection rather than escaping, and it is enforced at parse time so no
/// consumer can forget it. [`must_escape_for_report`] is the shared predicate the harness root
/// owns, so this check and the report-rendering escape can never disagree about which characters
/// must not appear literally.
///
/// The **single** exception is the line feed inside a heredoc value, where it is the format's own
/// line joiner and therefore structural rather than smuggled. A scalar value can never contain
/// one, because the parser reads scalars a line at a time.
///
/// This strictness costs the corpus nothing, and that was measured rather than assumed: every one
/// of the corpus programs emits only printable ASCII and the line feed, so no legitimate golden
/// record needs a character this check refuses.
fn require_report_safe_value(
    origin: &Path,
    field: &RawField,
    spec: &'static KeySpec,
) -> HarnessResult<()> {
    let newline_is_structural = spec.kind == FieldKind::Heredoc;
    for (offset, character) in field.value.char_indices() {
        if !must_escape_for_report(character) {
            continue;
        }
        if newline_is_structural && character == '\n' {
            continue;
        }
        let shown = sanitize_text_for_report(&field.value);
        return Err(key_error(
            origin,
            field.line,
            spec.name,
            format!(
                "the value carries {} at byte offset {offset}, which cannot appear literally in a \
                 report. Every field is written verbatim into a report row, a tab-separated \
                 summary column and a findings manifest, where such a character forges a column, \
                 erases a line, begins a terminal escape sequence, or reorders how the line \
                 renders — none of which says anything about the compiler. The value reads \
                 {shown:?} once made safe{}",
                describe_character(character),
                if newline_is_structural {
                    "; inside a heredoc the line feed is the format's own joiner and is the one \
                     character permitted here"
                } else {
                    ""
                }
            ),
        ));
    }
    Ok(())
}

/// Name one offending character precisely enough for a maintainer to find it in an editor.
fn describe_character(character: char) -> String {
    let code_point = u32::from(character);
    let name = match character {
        '\0' => Some("NUL"),
        '\t' => Some("a tab"),
        '\n' => Some("a line feed"),
        '\r' => Some("a carriage return"),
        '\u{1b}' => Some("an escape introducer"),
        '\u{7f}' => Some("the delete character"),
        _ => None,
    };
    match name {
        Some(name) => format!("{name} (U+{code_point:04X})"),
        None => format!("the control or formatting character U+{code_point:04X}"),
    }
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
        if raw_line.len() > RECORD_LINE_BYTES_MAX {
            return Err(line_error(
                origin,
                number,
                format!(
                    "the line is {} bytes, above the {RECORD_LINE_BYTES_MAX}-byte limit; every \
                     line of the format is a key, a short scalar, or one line of a golden stdout, \
                     so a line this long would let a single line defeat the whole-file bound by \
                     arriving as one enormous field",
                    raw_line.len()
                ),
            ));
        }

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
            if open.body.len() >= HEREDOC_LINES_MAX {
                return Err(key_error(
                    origin,
                    number,
                    open.spec.name,
                    format!(
                        "the heredoc body has reached {HEREDOC_LINES_MAX} lines without a closing \
                         `{HEREDOC_TERMINATOR}`; the longest heredoc body in the corpus is 126 \
                         lines, so a body more than an order of magnitude longer is an \
                         unterminated heredoc swallowing the rest of the file rather than a value"
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

/// How a cell's artifact is executed.
///
/// This is an explicit two-way choice rather than an optional runner path, and the distinction is
/// the whole point. "No runner" and "a runner that could not be found" are opposite situations
/// with opposite correct responses: the first means run the artifact directly, the second means
/// the cell cannot be judged at all and its oracles must be reported unavailable. Collapsing them
/// into one absent value leaves the difference to be re-derived by every consumer, and a consumer
/// that gets it wrong executes a foreign binary on the host and reads the resulting failure as a
/// compiler defect.
///
/// Making the choice a type means a missing runner is not expressible here. A caller that has no
/// runner for a non-native target cannot build the [`Execution`] value that would let it proceed,
/// so the situation is forced back to the discovery layer, where the target is marked unexecutable
/// and reported — which is the only place that answer belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Execution {
    /// The artifact runs directly on the host, so `<runner>` contributes no argument at all.
    ///
    /// Accepted only for a target the host executes natively. [`CommandSubstitutions::new`]
    /// refuses it for any other target rather than producing a command line that would run a
    /// foreign binary on the host.
    Native,
    /// The artifact runs under the emulator at this vetted path, substituted for `<runner>`.
    Emulated(PathBuf),
}

impl Execution {
    /// The emulator path, or `None` when the artifact runs natively.
    pub fn runner(&self) -> Option<&Path> {
        match self {
            Execution::Native => None,
            Execution::Emulated(runner) => Some(runner.as_path()),
        }
    }
}

/// Everything one cell needs in order to turn a command template into a literal command line.
///
/// # Why the fields are private
///
/// This is not a plain carrier. Every value it holds has already been vetted by the layer that
/// produced it — a compiler path was checked for ownership, world-writability and location trust,
/// a source path was proved to resolve inside the corpus, an output path was derived from the
/// cell's own workspace — and the whole worth of that vetting is that the *same* value is the one
/// that gets executed. A writable field would let a caller substitute a different path between
/// the check and the use, and nothing downstream could tell: the argument vector, the reproduction
/// command and the finding artifact would all name whatever they were handed.
///
/// Construction additionally establishes two invariants no field could carry on its own. Every
/// path is representable as text without loss, so the string that reaches the command line is
/// byte-for-byte the path that was vetted; and the execution mode agrees with the target, so a
/// foreign artifact can never be handed to the host to run.
///
/// # The reference compiler is optional, deliberately
///
/// Two of the three command templates never mention it. A cell that only needs its compiler-under-
/// test line rendered, or only its run line, must not be obliged to produce a reference driver it
/// has no use for — and on an environment where oracle (a) is unavailable there is no reference
/// driver to produce, while oracles (b) and (c) still have every reason to run. So the reference
/// driver is added by [`CommandSubstitutions::with_reference_compiler`] only where a reference
/// command is actually going to be rendered, and asking to render one without it is an explanatory
/// failure that names the omission rather than a half-expanded line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSubstitutions {
    /// Vetted path to the compiler under test, held as text validated at construction.
    bcc: String,
    /// The reference compiler driver for this cell's target, substituted for the `$REF_CC`
    /// spellings: the native driver for the native arm, the matching GCC cross driver for a cross
    /// arm. `None` until [`CommandSubstitutions::with_reference_compiler`] supplies one.
    reference_compiler: Option<String>,
    target: Target,
    opt: OptLevel,
    source: String,
    output: String,
    /// The execution runner, substituted for `<runner>`. `None` means the artifact runs natively,
    /// where the placeholder contributes no argument so the rendered line begins with the artifact
    /// itself.
    ///
    /// `None` is unambiguous here, and that is the point of holding the runner as an
    /// [`Execution`]-derived value rather than as a bare option a caller could leave empty for any
    /// reason: it can only arise from [`Execution::Native`], which the constructor accepts solely
    /// for a target the host runs natively. A missing emulator cannot reach this field at all.
    runner: Option<String>,
}

impl CommandSubstitutions {
    /// Build the substitutions for one cell, without a reference compiler.
    ///
    /// # Errors
    ///
    /// Rejects a path that cannot be represented as text without loss, and rejects
    /// [`Execution::Native`] for a target the host does not execute natively. Both are caller
    /// defects rather than conditions an environment can legitimately produce, so each becomes an
    /// explanatory failure naming the offending value.
    pub fn new(
        bcc: &Path,
        target: Target,
        opt: OptLevel,
        source: &Path,
        output: &Path,
        execution: &Execution,
    ) -> HarnessResult<CommandSubstitutions> {
        let context = format!(
            "assembling the command substitutions for the {} cell at {}",
            target.triple(),
            opt.flag()
        );
        if matches!(execution, Execution::Native) && !target.is_native() {
            return Err(HarnessError::new(
                context,
                format!(
                    "this cell targets {} but declares native execution, and the host executes {}; \
                     a non-native artifact needs its emulator, and rendering a run command without \
                     one would hand a foreign binary to the host, whose refusal to run it would \
                     then be read as a defect in the compiler that produced it. A target with no \
                     runner is reported as unexecutable by the discovery layer instead",
                    target.triple(),
                    std::env::consts::ARCH
                ),
            ));
        }
        let runner = execution
            .runner()
            .map(|path| require_representable_path(&context, "execution runner", path))
            .transpose()?;
        Ok(CommandSubstitutions {
            bcc: require_representable_path(&context, "compiler under test", bcc)?,
            reference_compiler: None,
            target,
            opt,
            source: require_representable_path(&context, "program source", source)?,
            output: require_representable_path(&context, "build artifact", output)?,
            runner,
        })
    }

    /// Add the reference compiler driver for this cell's target.
    ///
    /// # Errors
    ///
    /// Rejects a path that cannot be represented as text without loss.
    pub fn with_reference_compiler(mut self, reference: &Path) -> HarnessResult<Self> {
        let context = format!(
            "adding the reference compiler driver to the {} cell at {}",
            self.target.triple(),
            self.opt.flag()
        );
        self.reference_compiler = Some(require_representable_path(
            &context,
            "reference compiler",
            reference,
        )?);
        Ok(self)
    }

    /// Vetted path to the compiler under test.
    pub fn bcc(&self) -> &Path {
        Path::new(&self.bcc)
    }

    /// Vetted path to the reference compiler driver for this cell's target, when one was supplied.
    pub fn reference_compiler(&self) -> Option<&Path> {
        self.reference_compiler.as_deref().map(Path::new)
    }

    /// The target this cell is built and executed for.
    pub fn target(&self) -> Target {
        self.target
    }

    /// The optimization level this cell is built at.
    pub fn opt(&self) -> OptLevel {
        self.opt
    }

    /// Resolved path to the program source.
    pub fn source(&self) -> &Path {
        Path::new(&self.source)
    }

    /// Path to the build artifact inside this cell's workspace.
    pub fn output(&self) -> &Path {
        Path::new(&self.output)
    }
}

/// Render one vetted path as the exact text that will become a command-line argument, refusing a
/// path that cannot be represented without loss.
///
/// This is the single place the harness converts a path into argument text, and it converts
/// **losslessly or not at all**. The distinction matters because a path is not always valid
/// Unicode: on this platform it is an arbitrary sequence of non-zero bytes. The lossy conversion
/// every convenient rendering method performs replaces each unrepresentable byte with U+FFFD, and
/// the result is a *different path* that happens to print plausibly.
///
/// Silently accepting that would make three separate claims false at once. The path that was
/// vetted for ownership, world-writability and location trust would not be the path that was
/// executed, so the vetting would guard nothing. A finding's reproduction command would name a
/// file that does not exist, so "exact reproduction commands" would be untrue in precisely the
/// case a maintainer most needs them. And the failure would surface as a puzzling
/// file-not-found from a command line that looks correct on screen, which is the least
/// diagnosable shape a defect can take.
///
/// So an unrepresentable path is refused here, at construction, with a message that says which
/// role the path was filling and shows its lossy rendering purely so a reader can recognise it.
/// Every subsequent rendering is then a plain copy of already-validated text and cannot fail,
/// which is what lets the substitution functions promise a lossless result rather than hope for
/// one.
fn require_representable_path(context: &str, role: &str, path: &Path) -> HarnessResult<String> {
    match path.to_str() {
        Some(text) => Ok(String::from(text)),
        None => Err(HarnessError::new(
            String::from(context),
            format!(
                "the {role} path {} is not valid Unicode, so it cannot be rendered as command-line \
                 text without alteration; it is refused rather than converted approximately, \
                 because an approximate rendering names a different file — the vetting that \
                 approved this path would guard nothing, the reproduction command would not \
                 reproduce the cell, and the failure would arrive as a file-not-found from a \
                 command line that reads correctly. Move the checkout, or the tool, to a path that \
                 is valid Unicode",
                shown_path(path)
            ),
        )),
    }
}

/// The concrete reference-driver spelling for one target, for example `$REF_CC_AARCH64`.
///
/// Derived from the target rather than tabulated, so the accepted spellings and the target table
/// cannot drift apart, and so the spellings keep mirroring the environment-variable names that
/// select the drivers.
fn concrete_reference_placeholder(target: Target) -> String {
    format!(
        "{PLACEHOLDER_REFERENCE_BARE}_{}",
        target.short_name().to_ascii_uppercase()
    )
}

/// True when `token` is the concrete reference-driver spelling of **some** target.
///
/// Separate from [`substitute_token`], which expands only the spelling matching the cell's own
/// target, because the two questions have different answers and both are needed. Template
/// validation asks the broad question — is this token placeholder-shaped, and therefore exempt from
/// the character rules that apply to literal text — and must answer yes for all four spellings, so
/// that a target-restricted record may name its one driver directly. Rendering asks the narrow
/// question, and must answer yes for exactly one.
fn is_concrete_reference_placeholder(token: &str) -> bool {
    Target::ALL
        .iter()
        .any(|target| token == concrete_reference_placeholder(*target))
}

/// Expand a command template into a literal command line.
///
/// Accepted placeholders:
///
/// - `$BCC` — the compiler under test.
/// - `$REF_CC_<TRIPLE>` — the reference driver for this cell's target. The triple selects a
///   *driver binary* rather than a target flag, matching how this harness resolves oracle (a)'s
///   cross arm.
/// - `$REF_CC_X86_64`, `$REF_CC_I686`, `$REF_CC_AARCH64`, `$REF_CC_RISCV64` — the concrete
///   spellings, accepted because a target-restricted record may legitimately name its one
///   driver directly, and because they mirror the environment-variable names that select the
///   drivers. Only the spelling that matches this cell's target is expanded. A record naming a
///   different architecture's driver is describing a comparison other than the one being run, so
///   its token is deliberately left unexpanded for [`residual_placeholder`] to catch rather than
///   quietly rewritten to this cell's driver, which would compare the wrong pair of binaries
///   while every artifact still read as if it had compared the right one.
/// - `$REF_CC` — the bare synonym, matched only where it is a whole token, so it cannot consume
///   the prefix of a concrete spelling.
/// - `<triple>`, `<opt>`, `<src>`, `<out>`, `<runner>` — the cell's triple, optimization-level
///   flag, source path, artifact path and execution runner.
///
/// Rendering is deliberately more permissive than record validation. Anything that expands to a
/// correct command line is safe to render, whereas a record's `ref_command` must *begin* with a
/// per-target driver — the canonical `$REF_CC_<TRIPLE>`, or the concrete spelling where the
/// record declares that one target — because on that side the driver binary is the target
/// selection, and a bare name would silently mean the native one. [`validate_templates`]
/// enforces that, along with the ten other placeholders a record cannot do without.
///
/// # Why the result is an argument vector rather than a string
///
/// A command template is expanded into an **argument vector**, one element per template token,
/// and never into a single string that is later split. The distinction is the whole of this
/// function's safety argument.
///
/// A path can contain a space, and on a build machine it very often does. Substituting such a
/// path into a flat string yields a line that *looks* right and, when a maintainer pastes it into
/// a shell, silently becomes two arguments — so the reproduction command reproduces something
/// other than the cell. Worse, a path containing a semicolon, a backquote or a `$(` would not
/// merely be mis-split but *executed*, which is the difference between a reproduction script and
/// a command-injection primitive. Neither problem can be fixed at emit time by a consumer who has
/// already lost the token boundaries.
///
/// Keeping the boundaries means:
///
/// - the harness executes each element as one argument with no shell involved at all, so shell
///   grammar in a path has nothing to act on;
/// - a reproduction line is produced by quoting each element with [`posix_command_line`], so the
///   line a maintainer pastes decomposes into exactly the elements the harness used;
/// - a placeholder contributes exactly one element even when its value contains whitespace, and
///   `<runner>` on a natively executing target contributes exactly zero rather than leaving an
///   empty word behind.
///
/// A token that is not a placeholder is literal text, and it is validated at parse time by
/// [`validate_templates`] against [`TEMPLATE_FORBIDDEN_CHARACTERS`], so no shell metacharacter
/// can reach this function from a record in the first place.
///
/// # Why every substituted value is exact
///
/// Each path is copied out of the already-validated text the substitutions carry, which
/// [`require_representable_path`] proved at construction to be a lossless rendering of the vetted
/// path. The value that reaches the argument vector is therefore byte-for-byte the value that was
/// checked — there is no approximate conversion anywhere on this path, and so no way for the
/// executed file to differ from the identified one.
///
/// # Returns
///
/// - `Ok(Some(value))` — the token is a placeholder this cell can expand.
/// - `Ok(None)` — the token is not a placeholder for this cell. That covers ordinary literal text
///   and, deliberately, a concrete reference spelling belonging to another architecture.
///
/// # Errors
///
/// Returns an error only for a reference-driver placeholder in substitutions that carry no
/// reference driver. That is a caller defect — a reference command was asked for without the
/// compiler it names — and naming the omission is far more useful than emitting a half-expanded
/// line and letting the residual check describe the symptom.
fn substitute_token(token: &str, subs: &CommandSubstitutions) -> HarnessResult<Option<String>> {
    if token == PLACEHOLDER_BCC {
        return Ok(Some(subs.bcc.clone()));
    }
    // The canonical spelling, the bare synonym, and the one concrete spelling that names THIS
    // cell's target all resolve to this cell's driver. A concrete spelling naming a different
    // architecture deliberately falls through to `Ok(None)`: see the note below.
    if token == PLACEHOLDER_REFERENCE_TEMPLATED
        || token == PLACEHOLDER_REFERENCE_BARE
        || token == concrete_reference_placeholder(subs.target)
    {
        return match &subs.reference_compiler {
            Some(reference) => Ok(Some(reference.clone())),
            None => Err(HarnessError::new(
                format!(
                    "expanding {token} for the {} cell at {}",
                    subs.target.triple(),
                    subs.opt.flag()
                ),
                String::from(
                    "these substitutions carry no reference compiler driver, so this placeholder \
                     has nothing to expand to. A reference command can only be rendered for a \
                     cell whose reference driver resolved; where oracle (a) is unavailable the \
                     cell is reported unavailable for that oracle and its other oracles are \
                     rendered and run as usual. Add the driver with \
                     `CommandSubstitutions::with_reference_compiler` before rendering a reference \
                     command",
                ),
            )),
        };
    }
    // A concrete spelling for another architecture is NOT expanded. Rewriting it to this cell's
    // driver would compile with a compiler the record did not name, while every artifact still
    // read as though the named one had been used — a wrong comparison that leaves no trace. It is
    // left as literal text so the residual-placeholder check refuses the template outright.
    if token == PLACEHOLDER_TRIPLE {
        return Ok(Some(String::from(subs.target.triple())));
    }
    if token == PLACEHOLDER_OPT {
        return Ok(Some(String::from(subs.opt.flag())));
    }
    if token == PLACEHOLDER_SOURCE {
        return Ok(Some(subs.source.clone()));
    }
    if token == PLACEHOLDER_OUTPUT {
        return Ok(Some(subs.output.clone()));
    }
    Ok(None)
}

/// True when `token` is one of the accepted placeholder spellings.
///
/// Used by template validation so that a placeholder token is exempted from the literal-text
/// character rules, which is what lets `<out>` and `$REF_CC_<TRIPLE>` contain angle brackets and
/// a dollar sign while a literal token may not.
fn is_placeholder_token(token: &str) -> bool {
    if token == PLACEHOLDER_RUNNER || is_concrete_reference_placeholder(token) {
        return true;
    }
    // A fully populated probe, so the answer is about the token's spelling alone and never about
    // which values a particular cell happens to carry. The concrete spellings are answered above
    // rather than here, because this probe declares one target and `substitute_token` expands only
    // the spelling matching it — the very narrowing that makes rendering correct would make this
    // question wrong.
    let probe = CommandSubstitutions {
        bcc: String::from("bcc"),
        reference_compiler: Some(String::from("cc")),
        target: Target::X86_64,
        opt: OptLevel::O0,
        source: String::from("s"),
        output: String::from("o"),
        runner: None,
    };
    matches!(substitute_token(token, &probe), Ok(Some(_)))
}

/// Expand a command template into an argument vector, one element per token.
///
/// See [`substitute_token`] for why the result is a vector rather than a string. The `<runner>`
/// placeholder contributes one element on an emulated target and **no** element on a natively
/// executing one, so the vector begins with the artifact itself rather than with an empty word.
pub fn render_command_argv(
    template: &str,
    subs: &CommandSubstitutions,
) -> HarnessResult<Vec<String>> {
    let mut argv: Vec<String> = Vec::new();
    for token in template.split_whitespace() {
        if token == PLACEHOLDER_RUNNER {
            if let Some(runner) = &subs.runner {
                argv.push(runner.clone());
            }
            continue;
        }
        match substitute_token(token, subs)? {
            Some(value) => argv.push(value),
            None => {
                // The residual check applies to this token — a LITERAL one, straight from the
                // template — and never to a substituted value.
                //
                // The distinction is not a refinement, it is the difference between the check
                // working and the check being wrong in both directions. A substituted value is a
                // path, and a path may legitimately contain a dollar sign or a pair of angle
                // brackets: a checkout under a directory literally named `$HOME` or `<build>` is
                // unusual but entirely valid, and scanning the substituted value would reject
                // every cell on such a machine for a defect that does not exist. Meanwhile the
                // thing genuinely worth catching — a template token that is placeholder-shaped but
                // is not a placeholder this harness knows — is exactly what reaches here, because
                // an unrecognised token is passed through as literal text.
                if let Some(residual) = residual_placeholder(token) {
                    return Err(unexpanded_placeholder_error(
                        template, token, &residual, subs,
                    ));
                }
                argv.push(String::from(token));
            }
        }
    }
    if argv.is_empty() {
        return Err(HarnessError::new(
            format!("rendering the command template {template:?}"),
            String::from(
                "the template expands to no argument at all, so there is no command to run; a \
                 template names a program and its arguments, and an empty expansion means every \
                 token was a placeholder that contributed nothing",
            ),
        ));
    }
    Ok(argv)
}

/// Explain a template token that is placeholder-shaped but expanded to nothing.
///
/// The wrong-architecture case gets its own sentence, because it is the one shape of this failure
/// that is a genuine mistake in a record rather than a typo, and because saying so turns a puzzling
/// rejection into an obvious one.
fn unexpanded_placeholder_error(
    template: &str,
    token: &str,
    residual: &str,
    subs: &CommandSubstitutions,
) -> HarnessError {
    let context = format!("rendering the command template {template:?}");
    if is_concrete_reference_placeholder(token) {
        return HarnessError::new(
            context,
            format!(
                "the token {token} names the reference driver of another architecture, but this \
                 cell targets {}, whose driver is spelled {}. It is refused rather than expanded \
                 to this cell's driver: expanding it would compile with a compiler the record did \
                 not name while every artifact still read as though the named one had been used, \
                 so a comparison between the wrong pair of binaries would be recorded as though it \
                 were the right one. A record may name a concrete driver only for the single target \
                 it declares; otherwise write {PLACEHOLDER_REFERENCE_TEMPLATED}, which resolves to \
                 whichever driver the cell needs",
                subs.target.triple(),
                concrete_reference_placeholder(subs.target)
            ),
        );
    }
    HarnessError::new(
        context,
        format!(
            "the token {token:?} is placeholder-shaped — it still contains {residual} — but is not \
             a placeholder this harness recognises, so it would be passed to the compiler as \
             literal text. A half-expanded command line cannot reproduce a cell, so it is rejected \
             rather than recorded. A placeholder is substituted only as a whole token, which is \
             what keeps one placeholder equal to one argument. The accepted placeholders are \
             {PLACEHOLDER_BCC}, {PLACEHOLDER_REFERENCE_TEMPLATED}, {PLACEHOLDER_REFERENCE_BARE}, \
             the concrete {PLACEHOLDER_REFERENCE_BARE}_<ARCH> spelling of this cell's own target, \
             {PLACEHOLDER_TRIPLE}, {PLACEHOLDER_OPT}, {PLACEHOLDER_SOURCE}, {PLACEHOLDER_OUTPUT} \
             and {PLACEHOLDER_RUNNER}"
        ),
    )
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

/// Expand a command template into a single shell line, each element quoted by
/// [`posix_command_line`].
///
/// This is the entry point for every line that is shown to a human or written into a findings
/// artifact, and it is the reason requirement four's "exact reproduction commands" is literally
/// true rather than approximately true: the line is produced from the same argument vector the
/// harness executed, with each element quoted so that pasting it into a shell reconstructs that
/// vector element for element. A path containing a space, a semicolon or a `$(` therefore
/// reproduces as data instead of splitting into two arguments or being executed.
///
/// An ordinary command line is unchanged by the quoting, because a word of alphanumerics, path
/// separators and flag punctuation needs none — so the common case stays readable and the
/// dangerous case stays safe.
pub fn render_command_checked(
    template: &str,
    subs: &CommandSubstitutions,
) -> HarnessResult<String> {
    let argv = render_command_argv(template, subs)?;
    Ok(posix_command_line(&argv))
}

/// Which oracles, targets and optimization levels an expected-divergence marker covers.
///
/// The scope is parsed into structure rather than kept as prose so that the classifier can ask
/// "does this marker cover this cell?" and receive an answer instead of having to guess. Each
/// dimension is normalized to the canonical table order, and a dimension the scope does not
/// mention defaults to every member of that dimension — so `oracle_b` on its own means
/// "cross-backend comparison, on every target, at every optimization level".
/// The fields are private because a scope is the sole authority on whether a divergence is excused,
/// and the three dimension lists have to stay faithful to the `raw` text beside them. A caller able
/// to widen one list would extend a marker's reach beyond what the record says and beyond what the
/// register documents, turning real failures into expected divergences while every report still
/// displayed the original, narrower scope — the one change to this type that could hide a defect
/// rather than reveal one. Parsing is the only way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerScope {
    raw: String,
    oracles: Vec<Oracle>,
    targets: Vec<Target>,
    opt_levels: Vec<OptLevel>,
}

impl MarkerScope {
    /// The scope exactly as the record wrote it, retained for reports and artifacts.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The oracles this scope covers, in canonical table order.
    pub fn oracles(&self) -> &[Oracle] {
        &self.oracles
    }

    /// The targets this scope covers, in canonical table order.
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// The optimization levels this scope covers, in canonical table order.
    pub fn opt_levels(&self) -> &[OptLevel] {
        &self.opt_levels
    }
}

impl fmt::Display for MarkerScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.raw)
    }
}

/// A divergence that a limitation the repository already documents explains.
///
/// A marker changes how a divergence is **classified**, never whether the feature is
/// **exercised**: the applicable phases are attempted in order — compile, link, run, compare
/// — and classification happens at the first terminal outcome or the completed comparison, so
/// a marker never short-circuits a phase. That is what keeps a difficult feature under test
/// instead of quietly dropped, including when the divergence being excused is the compile
/// failing. The basis is carried as three views of one string — the original text, the
/// repository-relative path it cites, and the citation that follows that path — so the
/// infrastructure test that audits the register can prove the cited document is contained in this
/// repository, read it, and resolve the section the citation names inside it.
/// The fields are private for the same reason the scope's are: this type is the mechanism by which
/// a failure is reclassified as expected, so every part of it has to keep pointing at the record
/// and the document it came from. The identifier is what the register is cross-checked against in
/// both directions, `basis_path` and `basis_citation` are what that cross-check resolves against the
/// cited document's own bytes, and
/// `program_path` is what ties the marker to the one program it may excuse. A writable field would
/// let a marker be retargeted at another program, or made to cite a document it was never granted,
/// after every one of those checks had already passed. Parsing a record is the only way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedDivergence {
    id: String,
    class: DivergenceClass,
    scope: MarkerScope,
    basis: String,
    basis_path: PathBuf,
    basis_citation: String,
    observed: String,
    program_path: PathBuf,
}

impl ExpectedDivergence {
    /// Marker identifier, by convention `XD-<AREA>-<TOPIC>-<NNN>`. Unique across the corpus. No
    /// concrete identifier is named here on purpose: the register is the only place a marker
    /// exists, and quoting a real one in a doc comment would outlive its retirement. There is
    /// presently no active marker in the corpus.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The shape of divergence this marker excuses.
    pub fn class(&self) -> DivergenceClass {
        self.class
    }

    /// Which oracles, targets and optimization levels the marker covers.
    pub fn scope(&self) -> &MarkerScope {
        &self.scope
    }

    /// The documented basis, verbatim: a repository-relative path, a comma, then the section or
    /// description that authorises the marker.
    pub fn basis(&self) -> &str {
        &self.basis
    }

    /// The repository-relative path the basis cites, split out so the register cross-check can
    /// assert the cited document exists.
    pub fn basis_path(&self) -> &Path {
        &self.basis_path
    }

    /// The citation that follows the path: the section or description within the cited document
    /// that authorises the marker.
    ///
    /// Split out so the register cross-check can resolve the **locators** inside it against the
    /// document's own bytes — a line or line range that must lie within the file, a `§` section
    /// number that must appear as one of its headings, a backtick-quoted phrase that must occur in
    /// it verbatim. A citation whose locators resolve to nothing is a citation a reader cannot
    /// check, which is the one thing a documented basis may not be.
    pub fn basis_citation(&self) -> &str {
        &self.basis_citation
    }

    /// The divergence as observed, so a reader can recognise it without reproducing the run.
    pub fn observed(&self) -> &str {
        &self.observed
    }

    /// True when this marker's scope covers the given cell and oracle.
    ///
    /// This answers the **scope** question only, and deliberately does not re-check which
    /// program the cell belongs to: a marker is reachable only through the record of the very
    /// program it governs, so the program identity is already established by the time a caller
    /// holds one. Keeping the check to the scope is what makes the answer unambiguous.
    pub fn covers(&self, key: &CellKey, oracle: Oracle) -> bool {
        self.scope.oracles.contains(&oracle)
            && self.scope.targets.contains(&key.target())
            && self.scope.opt_levels.contains(&key.opt())
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
            _ => shown_path(&self.program_path),
        }
    }
    /// The program that provokes the divergence: the `.c` file, not its record.
    pub fn program_path(&self) -> &Path {
        &self.program_path
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

/// One program's fully validated expectation record.
///
/// The fields are private because they carry invariants the parser established and the rest of
/// the suite relies on: the program and area agree with the record's own location, the expected
/// exit status is one the platform can deliver, no flag forbidden in a differential invocation
/// reached the shared set, and every narrowing of coverage carries a recorded reason. Public
/// fields would make an unvalidated manifest constructible.
///
/// # The validated `shared_flags` list is checked and not retained
///
/// The record's declared list is required, parsed, checked against the permitted set, and
/// cross-checked against both build templates — every flag either template passes must be declared,
/// and every declared flag must appear in both — all before this value is constructed. The two
/// templates are then kept verbatim, so the flags a cell actually passes are recoverable from
/// `bcc_command` and `ref_command`, and a finding's reproduction commands show them literally.
/// Keeping a second copy of the list beside them would be a field claiming to publish a fact that no
/// path ever reads, and a reader who found the two disagreeing would have no way to tell which one
/// the invocation used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    path: PathBuf,
    program: String,
    area: String,
    description: String,
    targets: Vec<Target>,
    opt_levels: Vec<OptLevel>,
    /// The two build templates. The flag set both pass is not stored beside them: the record's
    /// `shared_flags` declaration is enforced against these templates at parse time by
    /// [`validate_build_template`], so the templates below are the authority on what each
    /// compiler is given, and a second copy of the declaration could only ever disagree with
    /// them.
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
    /// The `.expected` record this manifest came from, as the path it was loaded by.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// The sibling `.c` program the record governs, derived from [`Manifest::path`] by swapping
    /// the extension.
    ///
    /// Derived rather than stored, because the pairing is the format's own rule: a record and its
    /// program differ only in extension, which is what lets either be found from the other with
    /// no index and no configuration.
    pub fn source_path(&self) -> PathBuf {
        self.path.with_extension(SOURCE_EXTENSION)
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn area(&self) -> &str {
        &self.area
    }

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

    pub fn bcc_command(&self) -> &str {
        &self.bcc_command
    }

    pub fn ref_command(&self) -> &str {
        &self.ref_command
    }

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

    /// The per-program warning-gate flags when the record states them, and `None` when it leaves
    /// the default gate implicit.
    ///
    /// The value is always a subset of the harness root's canonical gate, in gate order, so two
    /// records expressing the same deviation compare equal regardless of the order their authors
    /// typed. The gate tables are defined once at the harness root — [`UB_AUDIT_GATE_DEFAULT`] and
    /// its two sanctioned reductions — rather than here or in the audit module: the audit module
    /// runs whichever gate applies and this module validates the record's claim against the same
    /// table, so one authority has two consumers rather than two that could drift apart.
    pub fn ub_audit_flags(&self) -> Option<&[String]> {
        self.ub_audit_flags.as_deref()
    }

    /// True when this program's gate actually differs from the default gate.
    ///
    /// A record that restates the default gate explicitly is not deviating from it, so this is a
    /// comparison against the table rather than a test for the key's presence: reporting an
    /// explicit restatement as a deviation would put a narrowing in the summary where none
    /// exists.
    pub fn has_ub_audit_deviation(&self) -> bool {
        match &self.ub_audit_flags {
            Some(gate) => !is_same_flag_set(gate, UB_GATE_DEFAULT),
            None => false,
        }
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

    /// The golden record — the stdout every cell of this program is expected to produce — as the
    /// bytes oracle (c) compares against.
    ///
    /// Deliberately the only accessor for it. There is no textual counterpart, because the
    /// comparison this record exists for is byte-exact: handing out a `&str` would invite a
    /// consumer to compare the golden record as text, and a textual comparison silently agrees
    /// about differences that matter — a trailing carriage return, an invalid sequence normalized
    /// on the way in, a byte that renders as the same glyph as another. The suite's contract is
    /// bytes, so this is the form the suite offers.
    pub fn expected_stdout_bytes(&self) -> &[u8] {
        self.expected_stdout.as_bytes()
    }

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
    ///
    /// # Errors
    ///
    /// Each identity is built through [`CellKey::new`], which enforces the canonical-stem rules
    /// on the area and program names. Both names reached this record through the parser, which
    /// already required the program to equal the file stem and the area to be a known feature
    /// area, so a rejection here means those two checks and the stem rules have drifted apart —
    /// a defect in the harness rather than in the corpus. It is surfaced rather than asserted
    /// away, because the alternative spellings are a panic, which would replace a diagnosable
    /// failure with a crash, and a silent fallback, which would file a cell's artifacts under
    /// the wrong name.
    pub fn cells(&self) -> HarnessResult<Vec<CellKey>> {
        let mut cells = Vec::with_capacity(self.cell_count());
        for target in &self.targets {
            for opt in &self.opt_levels {
                cells.push(CellKey::new(
                    self.area.clone(),
                    self.program.clone(),
                    *target,
                    *opt,
                )?);
            }
        }
        Ok(cells)
    }

    /// Render this program's compiler-under-test invocation as one quoted shell line.
    ///
    /// The reproduction form of [`Manifest::render_bcc_argv`]: the line a report shows and a
    /// findings script carries, produced by [`render_command_checked`] from the same argument
    /// vector the harness executed, so a maintainer reads the command that actually ran rather
    /// than a re-derivation of it.
    ///
    /// # Errors
    ///
    /// Every rejection is [`render_command_argv`]'s. A template token that is placeholder-shaped
    /// but is not a placeholder this harness recognises, a concrete reference-driver spelling
    /// naming an architecture other than this cell's, and a template that expands to no argument
    /// at all are all refused: a half-expanded line cannot reproduce a cell, so it is rejected
    /// rather than recorded.
    pub fn render_bcc_command(&self, subs: &CommandSubstitutions) -> HarnessResult<String> {
        render_command_checked(&self.bcc_command, subs)
    }

    /// Render this program's reference-compiler invocation as one quoted shell line.
    ///
    /// The reproduction form of [`Manifest::render_ref_argv`], with the same provenance.
    ///
    /// # Errors
    ///
    /// [`render_command_argv`]'s rejections, plus the one specific to this arm: a
    /// reference-driver placeholder has nothing to expand to when `subs` carries no reference
    /// driver. That is a caller defect rather than a corpus defect — where oracle (a) is
    /// unavailable the cell is reported unavailable for that oracle, and no reference command is
    /// rendered for it at all.
    pub fn render_ref_command(&self, subs: &CommandSubstitutions) -> HarnessResult<String> {
        render_command_checked(&self.ref_command, subs)
    }

    /// The recorded run line, rendered for a reader.
    ///
    /// Unlike the two compile lines, this is not derived from a vector the harness executes — see
    /// [`Manifest::render_bcc_argv`] for why the executed runner comes from the attested capability
    /// record rather than from this template. The two agree in every ordinary configuration, and
    /// the recorded environment fingerprint beside a finding is what lets a reader confirm it.
    ///
    /// The `<runner>` placeholder contributes the emulator on an emulated target and nothing at all
    /// on a natively executing one, so the line begins with the artifact itself rather than with an
    /// empty word.
    ///
    /// # Errors
    ///
    /// [`render_command_argv`]'s rejections: an unrecognised placeholder-shaped token, a concrete
    /// reference-driver spelling belonging to another architecture, or an expansion that yields no
    /// argument.
    pub fn render_run_command(&self, subs: &CommandSubstitutions) -> HarnessResult<String> {
        render_command_checked(&self.run_command, subs)
    }

    /// Build this program's compiler-under-test invocation as an argument vector.
    ///
    /// This is the form the harness **executes**: one element per template token, spawned with no
    /// shell, so no character in a path can be interpreted as grammar. The string renderers above
    /// exist for reports and reproduction scripts and are derived from this same vector, which is
    /// what keeps the line a maintainer reads identical to the command that actually ran.
    ///
    /// The *run* line is deliberately the exception. [`Manifest::render_run_command`] renders the
    /// recorded template for a reader and [`Manifest::render_run_argv`] renders it as a vector the
    /// driver compares against the one it launched, but neither is what *chooses* the runner: the
    /// argument vector that actually executes an artifact is assembled by `execute.rs` from the
    /// runner in the capability record — the one this process discovered and attested itself. A
    /// vector built from the template would take the runner from a corpus file instead, which is
    /// the one input a record must not be able to choose: the attestation exists precisely so that
    /// what runs a cell is not named by the material under test.
    ///
    /// # Errors
    ///
    /// See [`render_command_argv`]. A placeholder-shaped token the harness does not recognise, a
    /// concrete reference-driver spelling for another architecture, and an expansion yielding no
    /// argument are each refused rather than passed to a compiler as literal text.
    pub fn render_bcc_argv(&self, subs: &CommandSubstitutions) -> HarnessResult<Vec<String>> {
        render_command_argv(&self.bcc_command, subs)
    }

    /// Build this program's reference-compiler invocation as an argument vector.
    ///
    /// # Errors
    ///
    /// See [`render_command_argv`], and additionally: a reference-driver placeholder cannot be
    /// expanded when `subs` carries no reference driver, so this is rendered only for a cell whose
    /// reference driver resolved.
    pub fn render_ref_argv(&self, subs: &CommandSubstitutions) -> HarnessResult<Vec<String>> {
        render_command_argv(&self.ref_command, subs)
    }

    /// Build this program's execution invocation as an argument vector.
    ///
    /// # Errors
    ///
    /// See [`render_command_argv`]. The `<runner>` placeholder contributes no element on a
    /// natively executing target, which is why an empty expansion — the one case that would leave
    /// no program to run — is refused there rather than silently accepted.
    pub fn render_run_argv(&self, subs: &CommandSubstitutions) -> HarnessResult<Vec<String>> {
        render_command_argv(&self.run_command, subs)
    }

    pub fn run_command(&self) -> &str {
        &self.run_command
    }

    /// The shared flags this record passes, recovered from its own build template.
    ///
    /// Derived rather than stored, which is what the type's own contract requires: the validated
    /// list is checked against the permitted set and against both templates before a `Manifest`
    /// exists and is then deliberately not retained, because a second copy beside the templates
    /// would be a field claiming to publish a fact no invocation reads. Recovering it here keeps
    /// one source of truth — the template the cell actually renders — so a report can state the
    /// flags without a reader having to wonder which of two lists the build used.
    ///
    /// Returned in template order, and only switches this layer is entitled to choose
    /// ([`RECORD_SHARED_FLAGS_PERMITTED`]) are recognised, so a placeholder, a path or the output
    /// selection the harness owns can never be reported as a record's choice.
    pub fn shared_flags(&self) -> Vec<String> {
        self.bcc_command
            .split_whitespace()
            .filter(|token| RECORD_SHARED_FLAGS_PERMITTED.contains(token))
            .map(String::from)
            .collect()
    }

    pub fn has_marker(&self) -> bool {
        self.marker.is_some()
    }

    /// The golden record as text, for quoting into a finding's manifest.
    ///
    /// Never for comparing: the comparison reads [`Manifest::expected_stdout_bytes`], and the
    /// reasoning there is why this one exists only to be *printed*. The record is required to be
    /// valid UTF-8 by the parser, so the text and the bytes are two views of the same value rather
    /// than a conversion that could lose anything.
    pub fn expected_stdout(&self) -> &str {
        &self.expected_stdout
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

fn parse_targets(origin: &Path, raw: &RawField) -> HarnessResult<Vec<Target>> {
    let mut declared: Vec<Target> = Vec::new();
    for item in comma_items(origin, raw.line, "targets", &raw.value)? {
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

/// Parse and canonicalize the optimization-level sweep, which every record must declare in full.
///
/// Unlike the target list, the sweep carries no restriction clause anywhere in the requirements:
/// the build matrix fixes it at three levels per program, and the two-variant authoring rule
/// exists precisely so that sweeping them discriminates between the constant folder's answer and
/// the backend's. A record that declared fewer would still parse, still run and still report PASS
/// while never exercising the level where a miscompilation lives — a silent coverage reduction,
/// which is the one outcome the requirements rule out unconditionally. Reducing the sweep for fast
/// local iteration is a run-time decision made by the environment and stamped on the report as
/// reduced coverage; it is deliberately not something a committed record may do on its own
/// authority.
fn parse_opt_levels(origin: &Path, raw: &RawField) -> HarnessResult<Vec<OptLevel>> {
    let mut declared: Vec<OptLevel> = Vec::new();
    for item in comma_items(origin, raw.line, "opt_levels", &raw.value)? {
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
            format!(
                "the optimization-level sweep is empty; sweeping the levels is what turns output \
                 equality into a semantic-preservation test, so every program declares all {}: {}",
                OptLevel::ALL.len(),
                comma_separated(&OptLevel::ALL.map(OptLevel::flag))
            ),
        ));
    }
    let levels = canonical(&OptLevel::ALL, &declared);
    if levels.len() != OptLevel::ALL.len() {
        let missing: Vec<&str> = OptLevel::ALL
            .iter()
            .copied()
            .filter(|level| !levels.contains(level))
            .map(OptLevel::flag)
            .collect();
        return Err(key_error(
            origin,
            raw.line,
            "opt_levels",
            format!(
                "the sweep omits {}; every program in the corpus is compared at all {} levels, \
                 because behaviour at multiple optimization levels is itself a mandated property \
                 and the coverage target pins the sweep at three levels per program with no \
                 restriction clause. A shorter sweep is a silent narrowing of coverage rather \
                 than a recordable exclusion: a level that is never compiled cannot diverge, so \
                 a miscompilation confined to it would simply never be seen. Reducing the matrix \
                 for a fast local iteration is a run-time choice that is reported as reduced \
                 coverage, never a property of a record. Declare {}",
                comma_separated(&missing),
                OptLevel::ALL.len(),
                comma_separated(&OptLevel::ALL.map(OptLevel::flag))
            ),
        ));
    }
    Ok(levels)
}

/// The value-taking flag a token names, in either the bare or the attached spelling.
///
/// Returns the flag itself for a bare `-o`, and the flag for an attached `-o/tmp/x` or `-DNAME=1`.
/// A record may name neither spelling — see [`VALUE_TAKING_SHARED_FLAGS`] — so this function
/// exists to *recognise* one in order to refuse it with a diagnostic that explains which of the
/// two problems it is, rather than to accept it.
fn value_taking_flag_named(token: &str) -> Option<&'static str> {
    VALUE_TAKING_SHARED_FLAGS
        .iter()
        .copied()
        .find(|prefix| token == *prefix || token.starts_with(prefix))
}

/// Flags that are verified as shared yet must not appear in a record's shared set, because the
/// command templates already supply them per cell.
///
/// Each entry states the field that owns it:
///
/// - `-o` — owned by the `<out>` placeholder of every template. Listing it here as well would
///   pass it twice, and the second occurrence would silently decide the output path.
/// - `-c` — stops the pipeline before a runnable artifact exists, which makes all three oracles
///   inapplicable, since every one of them compares the behaviour of a program that ran.
const TEMPLATE_OWNED_SHARED_FLAGS: &[&str] = &["-o", "-c"];

/// True when a token is an optimization-level selector in any spelling.
///
/// Used to keep a fixed level out of the shared set: the level is swept per cell and supplied
/// through the `<opt>` placeholder, so a record that pinned one would either fight the sweep or
/// silently win it, and in both cases the three cells of a program would no longer differ in
/// the one dimension they exist to differ in.
fn is_opt_level_selector(item: &str) -> bool {
    match item.strip_prefix("-O") {
        Some(remainder) => remainder.is_empty() || remainder.chars().all(|c| c.is_ascii_digit()),
        None => matches!(item, "-Os" | "-Ofast" | "-Og" | "-Oz"),
    }
}

/// Parse the flags passed identically to both compilers, enforcing the shared-flag discipline at
/// the data layer.
///
/// This is where the requirement "only pass command-line flags that both compilers honour with
/// the same meaning" stops being a convention and becomes a check. Enforcing it here rather than
/// only at the invocation site is deliberate: a maintainer cannot smuggle a
/// reference-compiler-only flag into a differential invocation by editing a record, because the
/// record itself will refuse to load.
///
/// Admissibility is judged from the **reference side**, and that decides the shape of everything
/// below. This key holds the flags passed identically to both compilers, so a flag the reference
/// compiler cannot honour can never be part of a set both compilers honour with the same meaning.
/// That is why the target selectors are rejected here even though the compiler under test not only
/// accepts one of them but *requires* it for every non-native cell: they are inadmissible as
/// *shared* arguments, not inadmissible as such. The compiler-under-test side is validated
/// separately, against the `bcc_command` template, where the selection is required rather than
/// forbidden.
///
/// # The rules, and the authority each protects
///
/// 1. **`-static` is mandatory.** Every artifact in the corpus is linked statically, and a record
///    that omitted it would describe a dynamically linked artifact whose emulated cells could not
///    execute without a sysroot the suite deliberately does not configure. Requiring the presence
///    of the flag — rather than merely permitting it — is what makes the linkage mode a
///    property of the corpus instead of a per-record accident.
/// 2. **No flag may take a value.** A value is a path or a macro definition, and both are outside
///    a data file's authority: the harness derives the artifact path from the cell's workspace and
///    the source path from the cell itself, so a record able to supply either could redirect the
///    build out of its workspace. Both spellings are refused, because they fail differently and
///    both fail: a **bare** `-o` consumes whatever argument the harness appends after it, and an
///    **attached** `-o../../outside` names an escaping path outright. This is the rule that closes
///    the argument-vector-consumption and path-escape problems together.
/// 3. **The permitted set is a set of switches.** After rules one and two, what remains is
///    [`RECORD_SHARED_FLAGS_PERMITTED`]: the three optimization levels, debug information, static
///    linkage and position independence. A record can therefore change how a program is optimized
///    and linked, and nothing else.
/// 4. **A flag the templates already own, or one that pins the sweep, is still rejected.** `-o`
///    and `-c` belong to the templates, and an optimization level — although a switch, and
///    therefore permitted by rule three on its own — is supplied per cell through the `<opt>`
///    placeholder, so naming one here would collapse the three cells of a program into one.
/// 5. **The list is a set.** A duplicate is a partly edited record rather than an intention.
fn parse_shared_flags(origin: &Path, raw: &RawField) -> HarnessResult<Vec<String>> {
    let items = whitespace_items(&raw.value);
    if items.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "shared_flags",
            format!(
                "the shared-flag list is empty; every artifact in the corpus is built statically, \
                 which is the one linkage mode both compilers spell identically and what lets an \
                 emulated target run with no sysroot configuration, so the list must name at \
                 least {MANDATORY_SHARED_FLAG}"
            ),
        ));
    }
    let mut flags: Vec<String> = Vec::with_capacity(items.len());
    for item in items {
        if is_forbidden_for_side(item, CompilerSide::Reference) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} must never appear in the SHARED argument set: it is either \
                     reference-compiler-only, compiler-under-test-only, or accepted by both with a \
                     different default scope. Diagnostic and sanitizer flags belong to the \
                     undefined-behaviour audit gate, which drives the reference compiler alone and \
                     never the compiler under test. Target selection belongs to the \
                     compiler-under-test side alone, where `{BCC_TARGET_FLAG} <triple>` is \
                     REQUIRED for every non-native cell of every oracle because that compiler has \
                     no cross drivers; the reference compiler has no target-selection flag at all \
                     and selects a target by using the matching cross-driver binary, which is why \
                     the selection cannot be a shared argument"
                ),
            ));
        }
        if TEMPLATE_OWNED_SHARED_FLAGS.contains(&item) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} is a verified shared flag but must not be listed here, because the \
                     command templates already supply it per cell. `-o` is owned by the `<out>` \
                     placeholder, and listing it twice would let the second occurrence silently \
                     decide the output path; `-c` stops the pipeline before a runnable artifact \
                     exists, which makes all three oracles inapplicable because every one of them \
                     compares the behaviour of a program that ran"
                ),
            ));
        }
        if is_opt_level_selector(item) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} pins an optimization level, which the shared set must not do: the \
                     level is swept per cell and supplied through the `<opt>` placeholder of each \
                     template. A pinned level would either fight the sweep or silently win it, and \
                     either way the three cells of this program would stop differing in the one \
                     dimension they exist to differ in — which is what turns output equality into \
                     a semantic-preservation test"
                ),
            ));
        }
        if let Some(flag) = value_taking_flag_named(item) {
            let bare = item == flag;
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} names the value-taking flag {flag:?}, which a record may not carry \
                     in either spelling. {} The harness derives the artifact path from the cell's \
                     workspace and the source path from the cell itself, so choosing a path, an \
                     include directory or a macro definition is not a decision this record layer \
                     is entitled to make. A record may name only switches: {}",
                    if bare {
                        "The bare spelling consumes the next element of the argument vector, so it \
                         would silently swallow whatever the harness appends after it and redirect \
                         the build."
                    } else {
                        "The attached spelling carries its value inline, so it can name a path \
                         outside the cell's workspace directly."
                    },
                    comma_separated(RECORD_SHARED_FLAGS_PERMITTED)
                ),
            ));
        }
        if !RECORD_SHARED_FLAGS_PERMITTED.contains(&item) {
            let verified_elsewhere = SHARED_FLAGS_VERIFIED.contains(&item);
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} is not a flag a record may name. A record may name only the \
                     value-free switches {}. {}",
                    comma_separated(RECORD_SHARED_FLAGS_PERMITTED),
                    if verified_elsewhere {
                        format!(
                            "It is a verified shared flag — the full verified set is {} — but \
                             verification answers which flags mean the same thing to both \
                             compilers, not which of them a data file may choose. This one is \
                             withheld from records because it stops the pipeline before a runnable \
                             artifact exists, and all three oracles compare the behaviour of a \
                             program that ran",
                            comma_separated(SHARED_FLAGS_VERIFIED)
                        )
                    } else {
                        format!(
                            "It is not in the verified shared set either: a flag may be passed to \
                             both compilers only once an observable consequence of it has been \
                             verified for both, and the verified set is {}",
                            comma_separated(SHARED_FLAGS_VERIFIED)
                        )
                    }
                ),
            ));
        }
        if flags.iter().any(|known| known == item) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} is named twice; a repeated switch says nothing the single \
                     occurrence does not, and a duplicate is a partly edited record rather than \
                     an intention"
                ),
            ));
        }
        if FLAGS_WITHOUT_EXECUTABLE.contains(&item) {
            return Err(key_error(
                origin,
                raw.line,
                "shared_flags",
                format!(
                    "{item:?} stops the build short of a runnable executable; every cell in the \
                     corpus is judged by executing what was built and comparing its stdout bytes \
                     and exit status, so no program passes merely by compiling. A record carrying \
                     this flag would describe cells that can never be run"
                ),
            ));
        }
        flags.push(String::from(item));
    }
    if !flags.iter().any(|flag| flag == MANDATORY_SHARED_FLAG) {
        return Err(key_error(
            origin,
            raw.line,
            "shared_flags",
            format!(
                "the list does not name {MANDATORY_SHARED_FLAG}; every artifact in the corpus is \
                 linked statically, because that is the one linkage mode both compilers spell \
                 identically and the only one an emulated target can execute with no sysroot and \
                 no dynamic loader configuration. A record omitting it would describe cells that \
                 cannot run on three of the four targets"
            ),
        ));
    }
    Ok(flags)
}

/// True when a token names an optimization level literally rather than through the per-cell
/// placeholder.
///
/// The capital letter is what separates the family from `-o`, which selects the output path.
fn is_literal_opt_level(token: &str) -> bool {
    token.starts_with("-O")
}

/// Parse a per-program deviation from the default warning gate, as a **closed subset** of that
/// gate.
///
/// These flags are deliberately **not** checked against the forbidden-in-a-differential-
/// invocation set. The audit gate drives the reference compiler and never the compiler under
/// test, so it legitimately uses diagnostic flags that a differential invocation must never
/// carry; conflating the two sets would make the two genuine deviations in the corpus —
/// dropping strict-conformance diagnostics where an extension is the subject, and dropping
/// conversion diagnostics where a narrowing conversion is the subject — impossible to express.
///
/// It does not follow that the key may hold anything. The gate is what establishes the
/// precondition under which every oracle in this suite is sound: a divergence between two
/// compilers is evidence about a compiler only when the program that provoked it is free of
/// undefined and unspecified behaviour.
///
/// # A deviation is a removal from a fixed gate, never a compiler invocation
///
/// Accepting any token that begins with a hyphen would not be a constraint at all: these flags are
/// passed to a real compiler, so such a rule would let a record hand the reference compiler an
/// option that loads a shared object into the compiler process (`-fplugin=`), substitutes the
/// assembler or the compiler proper (`-B`), replaces the driver's built-in specification
/// (`-specs=`), changes what is compiled or where the output lands (`-I`, `-include`, `-D`, `-o`, a
/// bare filename), or simply turn the audit off while leaving it apparently configured (`-w`,
/// `-fsyntax-only`, or any `-Wno-` spelling). An audit that can be disabled by the record it is
/// auditing is not an audit, and since the audit is what establishes that a program is free of
/// undefined behaviour, disabling it silently removes the precondition under which any divergence
/// this suite reports means anything at all.
///
/// Modelling a deviation as a set of removals closes all of that at once — not by listing the
/// dangerous spellings, which would be a race against the compiler's option table, but by making
/// it impossible to name any spelling that is not one of seven known diagnostic switches:
///
/// - **Subset**: every named flag must be an entry of [`UB_AUDIT_GATE_DEFAULT`], so no option
///   outside those seven can be named.
/// - **No duplicates**: a repeated flag is a partly edited record, not an intention.
/// - **Retains [`UB_AUDIT_GATE_MANDATORY`]**: a gate that merely warns is not a gate, because the
///   audit's entire purpose is to make a diagnostic stop the run.
/// - **Strict**: the value must differ from the full gate. A record naming the entire gate is
///   declaring a deviation that deviates in nothing, which means it should simply omit the key —
///   and leaving that spelling acceptable would let a record appear to justify a narrowing it
///   never made, while also demanding an `impl_defined_notes` reason for nothing.
/// - **Only an authorized removal**: every member the value omits must be one of
///   [`UB_AUDIT_GATE_REMOVABLE`].
/// - **Only a sanctioned gate**: what remains must be exactly [`UB_GATE_WITHOUT_CONVERSION`],
///   where a narrowing conversion is the behaviour under test, or [`UB_GATE_WITHOUT_PEDANTIC`] in
///   [`EXTENSION_AREA`] alone, where the subject is non-standard by definition and that
///   diagnostic exists precisely to reject it. The set of gates the suite runs is closed, so a
///   record may neither combine two removals nor invent a third reduction.
///
/// Flags may be written in any order; the accepted value is returned in gate order, so two records
/// expressing the same deviation compare equal and read alike.
fn parse_ub_audit_flags(origin: &Path, raw: &RawField, area: &str) -> HarnessResult<Vec<String>> {
    let items = whitespace_items(&raw.value);
    if items.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "ub_audit_flags",
            format!(
                "the key is present but names no flag; a deviation from the default warning gate \
                 must state the gate it wants, and an empty deviation would silently disable the \
                 gate altogether. The gate that must be retained in full unless a documented \
                 deviation applies is: {}",
                comma_separated(UB_AUDIT_GATE_DEFAULT)
            ),
        ));
    }

    let mut declared: Vec<String> = Vec::with_capacity(items.len());
    for item in &items {
        if !is_ub_audit_gate_member(item) {
            return Err(key_error(
                origin,
                raw.line,
                "ub_audit_flags",
                format!(
                    "{item:?} is not an entry of the default warning gate, so it may not appear \
                     here. A deviation is expressed as a REMOVAL from the fixed gate {}, never as \
                     a compiler invocation of its own: these flags are passed to a real compiler, \
                     and accepting arbitrary options would let a record load a plugin into the \
                     compiler process, substitute a subprogram, replace the driver \
                     specification, add a search path, name an output, or suppress the very \
                     diagnostics the gate exists to raise. The audit is what establishes that a \
                     program is free of undefined behaviour, so a record able to weaken it could \
                     remove the precondition that makes every divergence this suite reports \
                     meaningful",
                    comma_separated(UB_AUDIT_GATE_DEFAULT)
                ),
            ));
        }
        if declared.iter().any(|known| known == item) {
            return Err(key_error(
                origin,
                raw.line,
                "ub_audit_flags",
                format!(
                    "{item:?} is named twice; a repeated diagnostic switch says nothing the single \
                     occurrence does not, and a duplicate is a partly edited record rather than an \
                     intention"
                ),
            ));
        }
        declared.push(String::from(*item));
    }

    if !declared.iter().any(|flag| flag == UB_AUDIT_GATE_MANDATORY) {
        return Err(key_error(
            origin,
            raw.line,
            "ub_audit_flags",
            format!(
                "the deviation drops {UB_AUDIT_GATE_MANDATORY}, which it may never drop; a gate \
                 that merely warns is not a gate, because the audit's whole purpose is to make a \
                 diagnostic stop the run. Remove the diagnostics the program genuinely cannot \
                 satisfy and keep this one"
            ),
        ));
    }

    if declared.len() >= UB_AUDIT_GATE_DEFAULT.len() {
        return Err(key_error(
            origin,
            raw.line,
            "ub_audit_flags",
            format!(
                "the deviation names the entire default gate {}, so it deviates in nothing; a \
                 program that passes the full gate simply omits this key, and recording a \
                 no-op deviation would demand a recorded reason for a narrowing that was never \
                 made",
                comma_separated(UB_AUDIT_GATE_DEFAULT)
            ),
        ));
    }

    // Every member the deviation omits must be one of the authorized removals. Expressed as
    // "which omissions are not permitted" rather than "which flags are banned", so the rule
    // stays closed: the set it draws from is fixed by the gate itself.
    let unauthorized: Vec<&'static str> = ub_audit_gate_required()
        .into_iter()
        .filter(|member| !declared.iter().any(|flag| flag == member))
        .collect();
    if !unauthorized.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            "ub_audit_flags",
            format!(
                "the deviation drops {}, which no program may drop. Only {} are authorized \
                 removals — strict-conformance diagnostics, for the supported-extension area, \
                 where an extension is non-standard by definition and that diagnostic exists \
                 precisely to reject one; and the conversion diagnostics, for the deliberate \
                 narrowing programs, where a narrowing conversion is the behaviour under test \
                 rather than a mistake. Every other member is non-negotiable",
                comma_separated(&unauthorized),
                comma_separated(UB_AUDIT_GATE_REMOVABLE)
            ),
        ));
    }

    // Canonicalize to gate order so two records expressing the same deviation compare equal and
    // read alike, regardless of the order their authors happened to type.
    let canonical_gate: Vec<String> = UB_AUDIT_GATE_DEFAULT
        .iter()
        .filter(|flag| declared.iter().any(|declared| declared == *flag))
        .map(|flag| String::from(*flag))
        .collect();

    // An authorized removal is not by itself a sanctioned gate: the set of gates the suite runs is
    // closed at two reductions, so a record may not combine both removals or invent a third.
    if is_same_flag_set(&canonical_gate, UB_GATE_WITHOUT_CONVERSION) {
        return Ok(canonical_gate);
    }
    if is_same_flag_set(&canonical_gate, UB_GATE_WITHOUT_PEDANTIC) {
        if area != EXTENSION_AREA {
            return Err(key_error(
                origin,
                raw.line,
                "ub_audit_flags",
                format!(
                    "this gate drops `-pedantic`, which is sanctioned only in the \
                     `{EXTENSION_AREA}` area, where the subject under test is by definition \
                     non-standard and `-pedantic` exists precisely to reject it. This record is \
                     in {area:?}, where a standard-conformance diagnostic is a genuine defect in \
                     the test program rather than a property of the feature under test"
                ),
            ));
        }
        return Ok(canonical_gate);
    }
    Err(key_error(
        origin,
        raw.line,
        "ub_audit_flags",
        format!(
            "{:?} is neither sanctioned reduction of the default warning gate. The \
             `{EXTENSION_AREA}` area may drop `-pedantic`, giving `{}`; a program whose subject is \
             a deliberate narrowing conversion may drop the conversion diagnostics, giving `{}`; \
             and a program that passes the full gate `{}` omits this key altogether. No other gate \
             is accepted, because this gate is what establishes the undefined-behaviour freedom \
             that makes a divergence evidence about the compiler, and a record that could choose \
             its own gate could exempt itself from the check it depends on",
            raw.value.trim(),
            UB_GATE_WITHOUT_PEDANTIC.join(" "),
            UB_GATE_WITHOUT_CONVERSION.join(" "),
            UB_GATE_DEFAULT.join(" ")
        ),
    ))
}

/// Require that every flag a declared gate drops is named, by its exact spelling, in
/// `impl_defined_notes`.
///
/// `impl_defined_notes` is the **single canonical field** for the reason behind a warning-gate
/// deviation, and this is what makes that a contract rather than a convention. Three things settle
/// the choice of field. It is the field a record already has to carry whenever it narrows anything
/// at all, so a gate deviation — which is a narrowing — has no second place to go. `ub_notes` is
/// reserved for a different obligation, the written undefined-behaviour-freedom argument, and a
/// field that answers two questions answers neither reliably. And accepting either field would mean
/// a reviewer looking for the reason has two places to look and no guarantee about which holds it.
///
/// The general narrowing check proves only that *some* reason is recorded. This proves the reason
/// accounts for the flag actually removed, which is the part a reviewer needs: a record that
/// explains a restricted target list while silently dropping a diagnostic has recorded nothing
/// about the removal that matters.
///
/// Matching is on the flag's exact spelling including its leading hyphen, so prose that happens to
/// use the word in another sense — "conversion", "pedantic" — cannot pass for an explanation. The
/// spellings cannot shadow one another either: no member of the default gate is a substring of
/// another member.
///
/// # Errors
///
/// Fails naming the record, every dropped flag, and the flags the notes do not account for. A
/// deviation without a recorded reason is a defect in the **test program**, not in the compiler: the
/// gate's whole value is its strictness, and an unexplained relaxation quietly re-admits the
/// undefined behaviour this suite depends on excluding in order to attribute a divergence at all.
fn require_gate_deviation_is_explained(
    origin: &Path,
    gate: &[String],
    impl_defined_notes: Option<&str>,
) -> HarnessResult<()> {
    let dropped: Vec<&str> = UB_GATE_DEFAULT
        .iter()
        .copied()
        .filter(|member| !gate.iter().any(|flag| flag == member))
        .collect();
    if dropped.is_empty() {
        return Ok(());
    }

    let notes = impl_defined_notes.unwrap_or_default();
    let unexplained: Vec<&str> = dropped
        .iter()
        .copied()
        .filter(|flag| !notes.contains(*flag))
        .collect();
    if unexplained.is_empty() {
        return Ok(());
    }

    Err(record_error(
        origin,
        format!(
            "the warning gate declared here drops {} from the default gate ({}), but \
             `impl_defined_notes` does not name {} anywhere, so the removal has no recorded reason. \
             A DEVIATION WITHOUT A RECORDED REASON IS ITSELF A DEFECT IN THE TEST. \
             `impl_defined_notes` is the one field this suite reads that reason from — `ub_notes` \
             carries the undefined-behaviour-freedom argument and is not searched for it — so name \
             each dropped flag there by its exact spelling and state why this program cannot be \
             compiled with it. Add or extend {}",
            comma_separated(&dropped),
            UB_GATE_DEFAULT.join(" "),
            comma_separated(&unexplained),
            required_form_of("impl_defined_notes")
        ),
    ))
}

/// True when a declared flag list holds exactly the sanctioned flags, in any order.
///
/// Comparison is by set membership and length rather than by sequence, so a record may write
/// the gate in whatever order reads best while still being unable to add or drop a flag: a
/// repeated flag shortens the set it covers and is caught by the membership test.
fn is_same_flag_set(declared: &[String], sanctioned: &[&str]) -> bool {
    declared.len() == sanctioned.len()
        && sanctioned
            .iter()
            .all(|flag| declared.iter().any(|item| item == flag))
}

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

/// Require that the enabled oracles can actually reach a verdict about this program.
///
/// The three toggles are independent, and that independence is load-bearing: it is what keeps a
/// construct whose value legitimately differs between architectures under test, by letting a
/// record disable cross-backend value equality alone while the program remains fully compared
/// against its same-target reference and its golden record. Narrowing an exclusion to one oracle
/// is permitted; what is not permitted is narrowing it to nothing.
///
/// Independence without a floor was the defect. Nothing required any oracle to be enabled, so a
/// record could disable all three and the program would still compile, still link, still run —
/// and be compared against nothing whatsoever, while the summary counted its cells and
/// reported them as passing. That is precisely the "passes merely by compiling" outcome the
/// requirements forbid, and it is worse than an omitted program, because an omitted program is
/// visibly absent whereas this one is visibly present and silently meaningless.
///
/// Two conditions therefore hold of every record:
///
/// - **Oracle (c) is mandatory.** The golden record applies to every cell of every program, and it
///   is the only oracle that catches both compilers changing behaviour in the same direction at
///   once — the one failure mode differential comparison structurally cannot see, because oracle
///   (a) reports it as agreement.
/// - **At least one differential oracle is mandatory.** A golden record alone would assert only
///   that the output has not changed since a maintainer recorded it, which cannot distinguish a
///   correct answer from a wrong answer that was wrong when it was recorded. An independent
///   authority — the reference compiler, or bcc's other three backends — is what turns
///   "unchanged" into "correct".
fn require_judgeable_oracles(
    fields: &[(&'static KeySpec, RawField)],
    origin: &Path,
    enabled: &[Oracle],
) -> HarnessResult<()> {
    let golden_key = format!("oracle_{}", Oracle::GoldenRecord.letter());
    if !enabled.contains(&Oracle::GoldenRecord) {
        let line = field(fields, &golden_key).map(|raw| raw.line);
        let cause = format!(
            "the golden-record oracle ({}) is disabled, and it may never be: it applies to every \
             cell of every program, and it is the only oracle that detects both compilers changing \
             behaviour in the same direction at the same time — the one failure mode \
             differential comparison structurally cannot see, because a reference comparison \
             reports it as agreement. Narrow an exclusion to the oracle that genuinely cannot \
             judge this program, and record the reason in `impl_defined_notes`",
            Oracle::GoldenRecord.label()
        );
        return Err(match line {
            Some(line) => key_error(origin, line, &golden_key, cause),
            None => record_error(origin, cause),
        });
    }

    let differential: Vec<Oracle> = [Oracle::ReferenceCompiler, Oracle::CrossBackend]
        .into_iter()
        .filter(|oracle| enabled.contains(oracle))
        .collect();
    if differential.is_empty() {
        let labels: Vec<&str> = [Oracle::ReferenceCompiler, Oracle::CrossBackend]
            .iter()
            .map(|oracle| oracle.label())
            .collect();
        return Err(record_error(
            origin,
            format!(
                "both differential oracles are disabled, leaving only the golden record. A golden \
                 record alone asserts that the output has not changed since a maintainer wrote it \
                 down, which cannot tell a correct answer from one that was already wrong when it \
                 was recorded — so the program would run without ever being compared against an \
                 independent authority. At least one of {} must remain enabled",
                comma_separated(&labels)
            ),
        ));
    }
    Ok(())
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
    let mut explicit_clauses = 0usize;

    for (position, clause) in text.split(';').map(str::trim).enumerate() {
        // An empty clause is rejected at its position rather than filtered away. Filtering it was
        // the defect: a scope of `;;;` produced no clause at all, every dimension then fell back
        // to "all", and the marker silently covered every oracle, every target and every
        // optimization level — the widest possible scope, reached by writing nothing. A marker's
        // scope decides which divergences are excused, so widening it by accident is how a real
        // defect on a target the marker was never meant to cover gets classified as expected.
        if clause.is_empty() {
            return Err(key_error(
                origin,
                raw.line,
                KEY_MARKER_SCOPE,
                format!(
                    "clause {} of the scope is empty, which usually means a doubled, leading or \
                     trailing `;`. An empty clause is refused rather than ignored, because \
                     ignoring it lets a scope that constrains nothing fall back to every oracle, \
                     every target and every optimization level — the widest scope there is, \
                     reached by writing nothing. Remove the stray `;`, write each clause \
                     explicitly, or use `all oracles`, `all targets` and `all opt levels` to say \
                     so deliberately",
                    position + 1
                ),
            ));
        }
        explicit_clauses += 1;
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

        let items = comma_items(origin, raw.line, KEY_MARKER_SCOPE, clause)?;
        if items.is_empty() {
            return Err(key_error(
                origin,
                raw.line,
                KEY_MARKER_SCOPE,
                format!(
                    "the clause {clause:?} lists no member; a clause that constrains a dimension \
                     to nothing would make the marker match no cell at all, so the divergence it \
                     describes could never be recognised and the marker would sit in the register \
                     documenting a comparison that never happens"
                ),
            ));
        }
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

    if explicit_clauses == 0 {
        return Err(key_error(
            origin,
            raw.line,
            KEY_MARKER_SCOPE,
            "the scope constrains no dimension; a marker must state at least one clause, because \
             a scope that constrains nothing would cover every oracle, every target and every \
             optimization level, and a marker that excuses every cell of a program can no longer \
             distinguish the divergence it documents from an unrelated defect",
        ));
    }

    // A dimension the scope does not mention defaults to every member of that dimension, which is
    // deliberate and safe now that at least one clause is guaranteed to be explicit: `oracle_b` on
    // its own means "cross-backend comparison, on every target, at every optimization level",
    // which is what a maintainer writing one clause means. What is no longer reachable is the
    // case where NO clause was explicit and every dimension widened at once.
    Ok(MarkerScope {
        raw: String::from(text),
        oracles: oracles.unwrap_or_else(|| Oracle::ALL.to_vec()),
        targets: targets.unwrap_or_else(|| Target::ALL.to_vec()),
        opt_levels: opt_levels.unwrap_or_else(|| OptLevel::ALL.to_vec()),
    })
}

/// Reject a marker whose scope cannot match a single cell of the record that carries it.
///
/// A scope is written independently of the matrix, so the two can disagree, and a disagreement is
/// invisible at run time in the worst possible way: the marker is never consulted, so nothing
/// reports it as unused. It sits in the register as documented knowledge about a comparison this
/// program never makes, and if the divergence it describes ever appears in a cell the scope
/// excludes, the run fails as an unexplained divergence with the explanation sitting unread two
/// lines above.
///
/// Because a scope defaults each unmentioned dimension to all of its members, a disagreement can
/// only arise where the scope constrains a dimension explicitly — a target the record does not
/// build, an optimization level it does not sweep, or an oracle it has switched off. Each is
/// checked and each is named individually, because "the scope does not intersect" is not
/// actionable whereas "the scope names aarch64 but the record builds x86_64 only" is.
///
/// The unexpected-success policy is the reason this must be an error rather than a warning: a
/// marker whose divergence has disappeared already fails the run, so a marker that could never be
/// consulted at all must not be allowed to look healthy.
fn validate_marker_scope_intersects(
    origin: &Path,
    divergence: &ExpectedDivergence,
    enabled_oracles: &[Oracle],
    targets: &[Target],
    opt_levels: &[OptLevel],
) -> HarnessResult<()> {
    let oracles: Vec<&str> = divergence
        .scope
        .oracles
        .iter()
        .filter(|oracle| enabled_oracles.contains(oracle))
        .map(|oracle| oracle.label())
        .collect();
    if oracles.is_empty() {
        let scoped: Vec<&str> = divergence
            .scope
            .oracles
            .iter()
            .map(|oracle| oracle.label())
            .collect();
        let enabled: Vec<&str> = enabled_oracles
            .iter()
            .map(|oracle| oracle.label())
            .collect();
        return Err(record_error(
            origin,
            format!(
                "the marker {} is scoped to {} but this record enables only {}, so the marker \
                 could never be consulted: it would document an expected divergence in a \
                 comparison this program never makes, and were the divergence to appear in an \
                 oracle the scope excludes the run would fail as unexplained with the explanation \
                 sitting unread in the same file. Either widen the scope or enable the oracle it \
                 describes",
                divergence.id,
                comma_separated(&scoped),
                comma_separated(&enabled)
            ),
        ));
    }

    let matched_targets: Vec<&str> = divergence
        .scope
        .targets
        .iter()
        .filter(|target| targets.contains(target))
        .map(|target| target.short_name())
        .collect();
    if matched_targets.is_empty() {
        let scoped: Vec<&str> = divergence
            .scope
            .targets
            .iter()
            .map(|target| target.short_name())
            .collect();
        let declared: Vec<&str> = targets.iter().map(|target| target.short_name()).collect();
        return Err(record_error(
            origin,
            format!(
                "the marker {} is scoped to the targets {} but this record builds only {}, so the \
                 marker could never be consulted. A target-restricted record and a marker scoped \
                 to a different target describe two different experiments",
                divergence.id,
                comma_separated(&scoped),
                comma_separated(&declared)
            ),
        ));
    }

    let matched_levels: Vec<&str> = divergence
        .scope
        .opt_levels
        .iter()
        .filter(|level| opt_levels.contains(level))
        .map(|level| level.flag())
        .collect();
    if matched_levels.is_empty() {
        let scoped: Vec<&str> = divergence
            .scope
            .opt_levels
            .iter()
            .map(|level| level.flag())
            .collect();
        let declared: Vec<&str> = opt_levels.iter().map(|level| level.flag()).collect();
        return Err(record_error(
            origin,
            format!(
                "the marker {} is scoped to the optimization levels {} but this record sweeps only \
                 {}, so the marker could never be consulted",
                divergence.id,
                comma_separated(&scoped),
                comma_separated(&declared)
            ),
        ));
    }

    Ok(())
}

/// Parse a marker basis into its verbatim text, the repository-relative path it cites, and the
/// citation that follows the path.
///
/// The path is separated from the citation by the first comma. It must be relative and must not
/// climb out of the repository, because the register audit resolves it against the package root
/// and asserts the document exists — an absolute or climbing path would let a marker cite
/// something outside the repository, which is no documented basis at all.
///
/// The citation half is returned separately as well as inside the verbatim text, because the
/// register audit resolves the **locators** it contains against the cited document: a line or line
/// range must lie within the document, a `§` section number must appear as one of its headings, and
/// a backtick-quoted phrase must occur in it verbatim. Splitting the halves here is what lets that
/// check read the citation without re-deriving where the path ended.
fn parse_basis(origin: &Path, raw: &RawField) -> HarnessResult<(String, PathBuf, String)> {
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
    Ok((
        String::from(text),
        path.to_path_buf(),
        String::from(citation),
    ))
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
    let (basis, basis_path, basis_citation) = parse_basis(origin, basis_field)?;

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
        basis_citation,
        observed: observed_field.value.clone(),
        program_path: program_path.to_path_buf(),
    }))
}

fn known_area_names() -> String {
    let names: Vec<&str> = AREAS.iter().map(|area| area.directory()).collect();
    comma_separated(&names)
}

/// Where the expected `program` and `area` of a record come from.
///
/// A record always declares both, and both are always checked. This decides what they are checked
/// *against*, which differs between the two places a record is legitimately read from:
///
/// - Inside the corpus, the record's own path is the authority: a record is `<program>.expected`
///   inside `<area>/`, so the declared values must equal the file stem and the containing directory
///   name. Nothing outside the record is needed or trusted.
/// - Inside a finding directory, the path carries no identity — the copy is always
///   `reproducer.expected` inside a directory named after the finding — so the reader states the
///   identity it expects and the record is validated against that.
///
/// Modelling this as a two-variant choice rather than an "skip the check" flag is deliberate: there
/// is no configuration in which the identity goes unchecked, so no caller can obtain one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordIdentity<'a> {
    /// Derive the expected identity from the record's own path: stem is the program, parent
    /// directory is the area. The rule for every record in the corpus.
    FromPath,
    /// Check the declared identity against values the caller supplies, for a record read from
    /// outside the corpus whose path cannot carry one.
    Declared {
        /// The feature area the reader expects the record to declare.
        area: &'a str,
        /// The program the reader expects the record to declare.
        program: &'a str,
    },
}

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

/// Reject a flag that takes a value but is not immediately followed by the placeholder that
/// supplies it.
///
/// Adjacency is the property that matters: `-o <out>` writes the artifact where the harness will
/// look for it, whereas `-o <src>` or a bare `-o` describes a cell that cannot be run even
/// though every required placeholder is present somewhere in the line.
fn require_followed_by(
    origin: &Path,
    raw: &RawField,
    key: &str,
    flag: &str,
    expected: &str,
    tokens: &[&str],
) -> HarnessResult<()> {
    let position = tokens.iter().position(|token| *token == flag);
    match position {
        Some(index) if tokens.get(index + 1) == Some(&expected) => Ok(()),
        Some(_) => Err(key_error(
            origin,
            raw.line,
            key,
            format!(
                "the template {:?} writes {flag} but does not follow it immediately with \
                 {expected}; the value belongs to the flag, so a line that separates them does \
                 not describe the cell the record claims",
                raw.value
            ),
        )),
        None => Err(key_error(
            origin,
            raw.line,
            key,
            format!(
                "the template {:?} omits {flag}; write `{flag} {expected}` so the command line \
                 names the cell it builds",
                raw.value
            ),
        )),
    }
}

/// Validate one build template's structure: the minimal differential flags, the adjacency of
/// the output flag and its placeholder, the absence of a pinned optimization level, and exact
/// agreement between the flags the line passes and the flags the record declares.
///
/// The bidirectional flag check is the point of this function. Requiring every flag in the line
/// to be declared stops a record from smuggling an unverified flag past the shared-flag
/// discipline by writing it into a template instead of into `shared_flags`; requiring every
/// declared flag to appear in the line stops the reverse, a record that declares a flag it never
/// passes and so documents an invocation that never happens. `extra_allowed_flags` carries the
/// flags a particular side may pass without declaring them — only the compiler-under-test's
/// target selection, which is legitimate precisely because the cross-backend oracle compares the
/// compiler against itself.
fn validate_build_template(
    origin: &Path,
    raw: &RawField,
    key: &str,
    shared_flags: &[String],
    extra_allowed_flags: &[&str],
) -> HarnessResult<()> {
    let tokens = whitespace_items(&raw.value);
    for flag in DIFFERENTIAL_FLAGS_MINIMAL {
        if !tokens.contains(flag) {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the template {:?} omits {flag}; every cell is built with the minimal \
                     differential set {}, which is what makes the artifact both findable and \
                     executable on all four targets",
                    raw.value,
                    comma_separated(DIFFERENTIAL_FLAGS_MINIMAL)
                ),
            ));
        }
    }
    require_followed_by(origin, raw, key, FLAG_OUTPUT, PLACEHOLDER_OUTPUT, &tokens)?;
    for &token in tokens.iter().skip(1) {
        if token.starts_with('$') {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the template {:?} names a compiler driver in {token:?} after its first \
                     token; a build line invokes exactly one driver, and a second one would \
                     silently decide which compiler produced the artifact",
                    raw.value
                ),
            ));
        }
        if is_literal_opt_level(token) {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the template {:?} pins the optimization level with {token:?}; the level \
                     comes from the {PLACEHOLDER_OPT} placeholder so that one record describes \
                     every level of its sweep, and a pinned level would compile all three cells \
                     identically while the report claimed a sweep",
                    raw.value
                ),
            ));
        }
        if !token.starts_with('-') {
            continue;
        }
        if extra_allowed_flags.contains(&token)
            || DIFFERENTIAL_FLAGS_MINIMAL.contains(&token)
            || shared_flags.iter().any(|flag| flag == token)
        {
            continue;
        }
        return Err(key_error(
            origin,
            raw.line,
            key,
            format!(
                "the template passes {token:?}, which the record does not declare in \
                 `shared_flags`. Every flag reaching a differential invocation is verified to be \
                 honoured by both compilers with the same meaning, and that verification is keyed \
                 to the declared list: a flag written straight into a template would bypass it. \
                 The declared list is: {}",
                comma_separated(
                    &shared_flags
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<&str>>()
                )
            ),
        ));
    }
    for flag in shared_flags {
        if !tokens.iter().any(|token| token == flag) {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the record declares the shared flag {flag:?} but the template {:?} never \
                     passes it; a declared flag that no invocation carries records an invocation \
                     that never happens, which is exactly the kind of claim the reproduction \
                     commands exist to make checkable",
                    raw.value
                ),
            ));
        }
    }
    Ok(())
}

/// True when a token selects the reference-compiler driver for the cell's own target.
///
/// The templated spelling and the four concrete per-target spellings each resolve to a driver
/// *binary*, which is how the cross arm works at all: the reference compiler has no
/// target-selection flag. The bare spelling is deliberately not accepted here — see
/// [`validate_templates`].
fn is_reference_driver_placeholder(token: &str) -> bool {
    if token == PLACEHOLDER_REFERENCE_TEMPLATED {
        return true;
    }
    Target::ALL.iter().any(|target| {
        token
            == format!(
                "{PLACEHOLDER_REFERENCE_BARE}_{}",
                target.short_name().to_ascii_uppercase()
            )
            .as_str()
    })
}

/// Require that a command template is an argument vector and not a shell script.
///
/// A template is split on whitespace and every token must be either one of the accepted
/// placeholder spellings or literal text free of [`TEMPLATE_FORBIDDEN_CHARACTERS`]. That is the
/// parse-time half of the reproduction-command guarantee, and it pairs with the emit-time half in
/// [`render_command_checked`] as follows:
///
/// - **Parse time refuses shell grammar in the template.** The harness spawns an argument vector
///   with no shell, so a template containing `;`, `|`, a backquote or a `$(` describes something
///   the harness will never do. Quoting it at emit time would faithfully reproduce a command line
///   nobody meant to write; refusing it names the record and the offending token instead.
/// - **Emit time quotes the substituted values.** A path is data the record does not control — it
///   comes from the checkout and the workspace — so it cannot be refused, and it is made safe by
///   quoting rather than by rejection.
///
/// Splitting the responsibility this way is what makes the whole path safe with no case left over:
/// everything a record author writes is checked, and everything the environment supplies is quoted.
///
/// A placeholder token is exempt from the character rules, which is what lets `<out>` and
/// `$REF_CC_<TRIPLE>` carry angle brackets and a dollar sign while a literal token may not. The
/// exemption is safe precisely because it applies to a **whole token**: a placeholder is recognised
/// only when the entire token matches, so `-o<out>` is literal text, is rejected for its angle
/// brackets, and can never smuggle a placeholder into a fused argument.
fn require_shell_free_template(origin: &Path, raw: &RawField, key: &str) -> HarnessResult<()> {
    let template = raw.value.trim();
    if template.is_empty() {
        return Err(key_error(
            origin,
            raw.line,
            key,
            "the template is empty; a cell is reproduced by running this command, and an empty \
             template describes no command at all",
        ));
    }
    for token in template.split_whitespace() {
        if is_placeholder_token(token) {
            continue;
        }
        if let Some(offending) = token
            .chars()
            .find(|character| TEMPLATE_FORBIDDEN_CHARACTERS.contains(character))
        {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the token {token:?} carries {}, which a template may not contain. A template \
                     is an argument vector, not a shell script: the harness spawns each token as \
                     one argument with no shell involved, so this character is either a mistake or \
                     an attempt to make a reproduction script do something the harness itself \
                     never did. Note that a placeholder is recognised only as a WHOLE token, so \
                     a fused spelling such as `-o<out>` is literal text and is refused here — \
                     write `-o <out>` as two tokens instead",
                    describe_template_character(offending)
                ),
            ));
        }
        // Defence in depth. `push_field` already refuses every control and formatting character
        // in every field value, so one cannot reach this point through the parser; the check is
        // repeated here so that a future caller reaching this function by another route cannot
        // place a character in an argument vector that a report would then have to escape.
        if let Some(offending) = token
            .chars()
            .find(|character| must_escape_for_report(*character))
        {
            return Err(key_error(
                origin,
                raw.line,
                key,
                format!(
                    "the token {token:?} carries {}, which cannot appear in an argument the \
                     harness executes or in a reproduction line a maintainer reads",
                    describe_character(offending)
                ),
            ));
        }
    }
    Ok(())
}

/// Name an offending template character in a way a maintainer can act on.
fn describe_template_character(character: char) -> String {
    let role = match character {
        '|' | '&' | ';' => "a shell list operator",
        '<' | '>' => "a shell redirection or an angle bracket belonging to a placeholder",
        '(' | ')' | '`' | '$' => "a shell command or parameter expansion",
        '{' | '}' | '[' | ']' | '*' | '?' | '!' => "a shell pattern or expansion character",
        '"' | '\'' | '\\' => "a shell quoting character",
        '#' => "a shell comment introducer",
        '~' => "a shell tilde expansion",
        '\n' | '\r' | '\t' => "whitespace that is not a plain space",
        _ => "a character reserved by the shell",
    };
    format!("{character:?}, {role}")
}

/// Validate the three command templates against each other, against the record's declared flag
/// set, against the target matrix they will be rendered for, and against the one fact that makes
/// the cross arm of the reference-compiler oracle work at all.
///
/// The templates are the whole of requirement 4's isolated reproducibility: a maintainer must be
/// able to rebuild and rerun any cell from the program source and this record alone. That makes
/// them a claim about the cell, and this function is what turns the claim into something
/// checkable.
///
/// Eleven things are required outright, one for every part of a cell a template must be able to
/// name: `$BCC`, `<triple>`, `<opt>`, `<src>` and `<out>` in the compiler-under-test template; a
/// per-target reference driver, `<opt>`, `<src>` and `<out>` in the reference template; and
/// `<runner>` and `<out>` in the run template. Ten are required by containment; the reference
/// driver is required positionally, as the template's first token, which is stricter than
/// containment because a driver named anywhere else is not the program the recorded line would
/// run.
///
/// None of the eleven is stylistic, and none can be caught later by [`render_command_checked`],
/// which only rejects a placeholder the template *did* write and the substitutions failed to
/// expand. What it cannot notice is a template that never wrote the placeholder at all — and such
/// a template still parses, while its cells still run with the missing part supplied by a default
/// nobody chose, so the record describes a cell other than the one the matrix says it runs. That
/// is a green run which compared nothing, the one failure mode a differential suite cannot
/// afford. Every element below is therefore structural rather than cosmetic:
///
/// - The compiler-under-test line must select the cell's target with `--target <triple>`, or a
///   record claiming four targets would record one command that builds only the default, silently
///   comparing a host binary against a cross-compiled one.
/// - Both build lines must carry the cell's optimization level (`<opt>`), or a record claiming a
///   three-level sweep would record one command that builds only the default level, so a
///   maintainer following the record would build something other than the cell whose divergence
///   was reported.
/// - Both build lines must write `-o <out>` with the placeholder immediately after the flag, and
///   must carry `-static`, because the harness executes the artifact at the path it asked for and
///   the three emulated targets need static linkage to run at all.
/// - The reference line must name a per-target driver, not the bare synonym, because the driver
///   *is* the target selection on that side and a bare name would silently mean the native one; a
///   concrete per-target spelling is admissible only where the record declares that one target.
/// - The run line must be exactly `<runner> <out>`, or a cross-target cell would be recorded as
///   executing a foreign binary directly instead of through its emulator.
/// - Every flag either build line passes must be declared in `shared_flags`, and every declared
///   flag must appear in both lines.
fn validate_templates(
    origin: &Path,
    bcc: &RawField,
    reference: &RawField,
    run: &RawField,
    targets: &[Target],
    shared_flags: &[String],
) -> HarnessResult<()> {
    require_shell_free_template(origin, bcc, "bcc_command")?;
    require_shell_free_template(origin, reference, "ref_command")?;
    require_shell_free_template(origin, run, "run_command")?;

    require_placeholders(
        origin,
        bcc,
        "bcc_command",
        &[
            PLACEHOLDER_BCC,
            PLACEHOLDER_TRIPLE,
            PLACEHOLDER_OPT,
            PLACEHOLDER_SOURCE,
            PLACEHOLDER_OUTPUT,
        ],
    )?;
    require_placeholders(
        origin,
        reference,
        "ref_command",
        &[PLACEHOLDER_OPT, PLACEHOLDER_SOURCE, PLACEHOLDER_OUTPUT],
    )?;
    require_placeholders(
        origin,
        run,
        "run_command",
        &[PLACEHOLDER_RUNNER, PLACEHOLDER_OUTPUT],
    )?;

    let bcc_tokens = whitespace_items(&bcc.value);
    if bcc_tokens.first() != Some(&PLACEHOLDER_BCC) {
        return Err(key_error(
            origin,
            bcc.line,
            "bcc_command",
            format!(
                "the template {:?} does not begin with {PLACEHOLDER_BCC}; the compiler under test \
                 is invoked directly, so it is the first token of the line a maintainer runs",
                bcc.value
            ),
        ));
    }
    require_followed_by(
        origin,
        bcc,
        "bcc_command",
        FLAG_TARGET_SELECT,
        PLACEHOLDER_TRIPLE,
        &bcc_tokens,
    )?;
    validate_build_template(
        origin,
        bcc,
        "bcc_command",
        shared_flags,
        &[FLAG_TARGET_SELECT],
    )?;

    // Asked before the first-token test below, because "this template never names the reference
    // compiler at all" is a different editing mistake from "it names it without the target
    // suffix" and is worth its own diagnostic. Every accepted reference-compiler spelling begins
    // with the bare placeholder, so this one containment test covers the templated form, the four
    // concrete forms and the bare form; the first-token test then holds the record to a per-target
    // spelling, and to naming that driver as the program the recorded line actually runs.
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

    let ref_tokens = whitespace_items(&reference.value);
    let driver = ref_tokens.first().copied().unwrap_or_default();
    if !is_reference_driver_placeholder(driver) {
        return Err(key_error(
            origin,
            reference.line,
            "ref_command",
            format!(
                "the template {:?} does not begin with a per-target reference-compiler driver; \
                 write {PLACEHOLDER_REFERENCE_TEMPLATED}, or the concrete spelling for this \
                 record's one target. The bare {PLACEHOLDER_REFERENCE_BARE} is not accepted: the \
                 reference compiler has no target-selection flag, so on that side the driver \
                 binary *is* the target, and a spelling that names no target would record the \
                 native driver as the command for a cross cell — a line that either fails or, \
                 worse, silently builds for the host while the report says otherwise",
                reference.value
            ),
        ));
    }

    // A concrete `$REF_CC_<ARCH>` spelling names one driver, so it is admissible only where the
    // record is restricted to that one target. Rendering is deliberately strict about this too:
    // a foreign spelling is left unexpanded rather than rewritten to the cell's own driver, so
    // that the fault surfaces as an unexpanded placeholder instead of as a comparison silently
    // made against the wrong binary. Rejecting it here means the fault is caught once, at load
    // time, naming the line — rather than once per cell, at render time.
    for target in Target::ALL {
        let concrete = concrete_reference_placeholder(target);
        if !reference.value.contains(&concrete) {
            continue;
        }
        if targets != [target] {
            return Err(key_error(
                origin,
                reference.line,
                "ref_command",
                format!(
                    "the template names {concrete}, the driver for {target} alone, but this record \
                     declares the targets {}. A concrete driver spelling is admissible only in a \
                     record restricted to that one target; for a multi-target record write \
                     {PLACEHOLDER_REFERENCE_TEMPLATED}, which resolves to the driver for whichever \
                     target the cell is being built for. Naming one architecture's driver while \
                     sweeping several would compile every cell with the same driver and report \
                     agreement about a comparison that was never made",
                    comma_separated(
                        &targets
                            .iter()
                            .map(|declared| declared.short_name())
                            .collect::<Vec<&str>>()
                    )
                ),
            ));
        }
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
    for &token in ref_tokens.iter().skip(1) {
        if token.contains(PLACEHOLDER_TRIPLE) {
            return Err(key_error(
                origin,
                reference.line,
                "ref_command",
                format!(
                    "the template passes the cell's triple in {token:?}; the reference compiler \
                     has no target-selection flag, so a triple appearing anywhere but inside the \
                     driver placeholder would be handed to it as an argument it does not \
                     understand"
                ),
            ));
        }
    }
    validate_build_template(origin, reference, "ref_command", shared_flags, &[])?;

    let run_tokens = whitespace_items(&run.value);
    if run_tokens.as_slice() != [PLACEHOLDER_RUNNER, PLACEHOLDER_OUTPUT] {
        return Err(key_error(
            origin,
            run.line,
            "run_command",
            format!(
                "the template {:?} is not exactly `{PLACEHOLDER_RUNNER} {PLACEHOLDER_OUTPUT}`. \
                 The runner placeholder is empty on the natively executing target and the \
                 target's emulator otherwise, so it is what lets one recorded line run a cell on \
                 any of the four targets; omitting it would record a cross cell as executing a \
                 foreign binary directly. No further token is accepted either: every corpus \
                 program reads its whole input from literals in its own source and is passed no \
                 argument, no redirection and no environment, which is what makes a cell \
                 reproducible from the source and this record alone",
                run.value
            ),
        ));
    }
    Ok(())
}

/// Turn parsed fields into a validated manifest.
///
/// Every rejection here is a corpus defect rather than a condition an environment can legitimately
/// produce, so none of them is tolerated: a divergence is evidence about the compiler only when
/// the program that provoked it is well formed and compared under oracles whose exclusions each
/// carry a recorded reason.
fn assemble(
    fields: Vec<(&'static KeySpec, RawField)>,
    origin: &Path,
    identity: RecordIdentity<'_>,
) -> HarnessResult<Manifest> {
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
    let area_field = required_field(&fields, origin, "area")?;
    match identity {
        RecordIdentity::FromPath => {
            let stem = record_stem(origin)?;
            if program_field.value != stem {
                return Err(key_error(
                    origin,
                    program_field.line,
                    "program",
                    format!(
                        "the record claims to govern {:?} but its own file stem is {stem:?}; the \
                         two must agree, because this is the cheapest guard there is against a \
                         record copied from another program and only partly edited",
                        program_field.value
                    ),
                ));
            }
            let directory = record_directory(origin)?;
            if area_field.value != directory {
                return Err(key_error(
                    origin,
                    area_field.line,
                    "area",
                    format!(
                        "the record claims the area {:?} but lives in {directory:?}; the two must \
                         agree, because the area names the report file the program's verdicts are \
                         written to",
                        area_field.value
                    ),
                ));
            }
        }
        RecordIdentity::Declared { area, program } => {
            if program_field.value != program {
                return Err(key_error(
                    origin,
                    program_field.line,
                    "program",
                    format!(
                        "the record claims to govern {:?} but the caller reading it expects {:?}. \
                         This record was read outside the corpus, where the file stem carries no \
                         identity, so the expected identity was supplied by the caller instead. A \
                         disagreement means the record does not describe the program it was \
                         emitted beside",
                        program_field.value, program
                    ),
                ));
            }
            if area_field.value != area {
                return Err(key_error(
                    origin,
                    area_field.line,
                    "area",
                    format!(
                        "the record claims the area {:?} but the caller reading it expects {area:?}. \
                         This record was read outside the corpus, where the containing directory \
                         carries no identity, so the expected area was supplied by the caller \
                         instead",
                        area_field.value
                    ),
                ));
            }
        }
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
    validate_templates(
        origin,
        bcc_field,
        ref_field,
        run_field,
        &targets,
        &shared_flags,
    )?;

    let expect_exit = parse_expect_exit(origin, required_field(&fields, origin, "expect_exit")?)?;

    let mut enabled_oracles: Vec<Oracle> = Vec::new();
    for oracle in Oracle::ALL {
        let key = format!("oracle_{}", oracle.letter());
        let toggle_field = required_field(&fields, origin, &key)?;
        if parse_toggle(origin, toggle_field, &key)? {
            enabled_oracles.push(oracle);
        }
    }
    require_judgeable_oracles(&fields, origin, &enabled_oracles)?;

    let ub_audit_flags = match field(&fields, "ub_audit_flags") {
        Some(raw) => Some(parse_ub_audit_flags(origin, raw, &area_field.value)?),
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

    // Everything this record compares less than the full matrix, gathered so that a narrowing
    // can never reach the corpus without its reason recorded beside it.
    //
    // The optimization-level dimension is deliberately absent from this list, and its absence is
    // load-bearing rather than an omission: a record that declares fewer than all three levels is
    // rejected outright by `parse_opt_levels`, which is strictly stronger than admitting the
    // restriction and asking for a note. A level that is never compiled cannot diverge, so a
    // miscompilation confined to it would never be seen at all — that is a hole in coverage rather
    // than a documented exclusion, and no recorded reason would make it visible in the results.
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
    if let Some(gate) = &ub_audit_flags {
        // Only a gate that actually differs from the default is a narrowing. A record is free to
        // restate the default gate explicitly, and doing so narrows nothing.
        if !is_same_flag_set(gate, UB_GATE_DEFAULT) {
            narrowings.push(format!(
                "the warning gate deviates from the default gate ({})",
                gate.join(" ")
            ));
        }
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
    if let Some(gate) = &ub_audit_flags {
        require_gate_deviation_is_explained(origin, gate, impl_defined_notes.as_deref())?;
    }

    let source = origin.with_extension(SOURCE_EXTENSION);
    let marker = parse_marker(&fields, origin, &source)?;
    if let Some(divergence) = &marker {
        validate_marker_scope_intersects(
            origin,
            divergence,
            &enabled_oracles,
            &targets,
            &opt_levels,
        )?;
    }

    Ok(Manifest {
        path: origin.to_path_buf(),
        program: program_field.value.clone(),
        area: area_field.value.clone(),
        description: description_field.value.clone(),
        targets,
        opt_levels,
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

/// Parse and validate a record from text, touching no filesystem.
///
/// `origin` is the path the text came from. It is not decoration: the identity checks that tie a
/// record to the program it governs read the file stem and the containing directory from it, so
/// it must be the record's real path even when the text was obtained some other way. Keeping the
/// pure parser separate from the reader is what lets the format's behaviour be inspected — and
/// every rejection exercised — without laying a single file down.
///
/// # Errors
///
/// Two layers reject, and every rejection is a corpus defect rather than a condition an
/// environment can legitimately produce. The grammar layer refuses a malformed key line, an
/// unknown or duplicated key, a missing required key, an unterminated or empty heredoc, an
/// oversized field, and a control or formatting character in a value. The assembly layer then
/// refuses a record whose declared program does not equal `origin`'s file stem or whose declared
/// area does not equal its containing directory, an unknown feature area, a command template that
/// is not the exact shape its key requires, a target or optimization-level restriction with no
/// recorded reason, an expected exit status outside the permitted range, and a malformed
/// expected-divergence marker.
pub fn parse_str(text: &str, origin: &Path) -> HarnessResult<Manifest> {
    let fields = parse_fields(text, origin)?;
    assemble(fields, origin, RecordIdentity::FromPath)
}

/// Parse and validate a record that does **not** live in the corpus, against an identity the caller
/// already knows.
///
/// # Why this exists
///
/// A record's declared `program` and `area` are normally checked against its own file stem and
/// containing directory. That is the cheapest possible guard against a record copied from another
/// program and only partly edited, and it is exactly right for the corpus.
///
/// It is also unusable for a **finding artifact**. A finding directory holds a copy of the program
/// and a copy of its record under fixed names — `reproducer.c` beside `reproducer.expected` — inside
/// a directory named after the finding. The copy's stem is therefore `reproducer` and its parent is
/// a finding identifier, so the path carries no corpus identity at all, and the path-derived check
/// would reject a record that is in every other respect the correct and complete one.
///
/// Dropping the identity check for such a record would be the wrong repair: the requirement that a
/// finding be reproducible depends on the emitted pair being genuinely parseable and genuinely about
/// the program beside it. So the check is not weakened, it is **re-based**: the caller — which
/// derived the finding from a cell and therefore knows the area and program with certainty — states
/// the identity it expects, and the record is validated against that instead of against its path.
/// Every other rule in the format applies unchanged.
///
/// # Errors
///
/// Returns an explanatory failure when the record does not parse, when any other validation rule is
/// violated, or when its declared identity disagrees with the one supplied here.
pub fn parse_standalone_str(
    text: &str,
    origin: &Path,
    area: &str,
    program: &str,
) -> HarnessResult<Manifest> {
    let fields = parse_fields(text, origin)?;
    assemble(fields, origin, RecordIdentity::Declared { area, program })
}

/// Read, parse and validate a record from disk.
///
/// A record that cannot be read is a hard error. It is never a skip: a program whose expectations
/// cannot be loaded is a program no oracle can judge, and quietly passing over it would remove a
/// cell from the matrix without anyone being told.
///
/// The path is checked for containment before a byte is read, by the same
/// [`require_contained_corpus_file`] the harness root uses to resolve a cell, so a record is read
/// only when it is a regular, non-symbolic-link file that genuinely resolves inside the corpus.
/// That matters more here than anywhere else in the module: a record dictates the command
/// templates a cell executes and the golden output it is judged against, so a record read from
/// outside the corpus would decide what gets compiled, with which arguments, and what counts as
/// correct — while every report still named the corpus path. The resolved path is what the
/// manifest carries onward, so the identity checks and every later diagnostic name the file that
/// was actually read.
///
/// # Errors
///
/// Three stages reject. Containment refuses a path that is not an existing regular file, is a
/// symbolic link, does not carry the record extension, or does not resolve strictly beneath the
/// corpus root. Reading refuses a file whose size exceeds [`RECORD_BYTES_MAX`] — checked before
/// the open and again on the reader, so a file that grows in between is caught — one that cannot
/// be opened or read, and one whose bytes are not valid UTF-8, with the first invalid offset
/// named. Parsing then applies every rejection [`parse_str`] documents.
pub fn load(expected_path: &Path) -> HarnessResult<Manifest> {
    let context = format!(
        "reading the expectation record {}",
        shown_path(expected_path)
    );
    let resolved = require_contained_corpus_file(
        &context,
        "expectation record",
        expected_path,
        RECORD_EXTENSION,
    )?;
    let text = read_record_text(&resolved)?;
    parse_str(&text, &resolved)
}

/// Read, parse and validate a **reproducer** record from a finding's artifact directory.
///
/// This is the function that makes a finding's central promise true. Every finding ships
/// `reproducer.c` beside `reproducer.expected` precisely "so the pair remains runnable by the
/// harness" — and a promise that has no code path behind it is worth nothing. [`load`] cannot serve
/// here for two independent reasons, and both are properties of where a finding lives rather than
/// of what it contains: a reproducer resolves inside a findings directory rather than inside the
/// corpus, and its path names the finding rather than the program (see
/// [`RecordIdentity::Declared`], and [`parse_standalone_str`] for why the identity is supplied by
/// the caller instead of being read off the path).
///
/// Containment is enforced against the two directories a reproducer may legitimately occupy, and
/// nowhere else:
///
/// - the generated set beneath [`findings_root`], written by the current run;
/// - the curated set beneath the corpus's `findings` directory, promoted by a human after review
///   and committed as a deliverable.
///
/// Both are checked with the same [`ensure_within`] the corpus loader uses, after the same
/// [`require_regular_file`], so "inside a findings directory" is decided by exactly the machinery
/// that decides "inside the corpus" — a reproducer is read only when it is an absolute path to a
/// regular, non-symbolic-link file that genuinely resolves beneath one of those two roots. That
/// matters for the same reason it matters for a corpus record: a record dictates the commands a
/// cell executes and the output it is judged against, so one read from an arbitrary location would
/// decide what gets compiled and what counts as correct.
///
/// A path beneath neither root is refused with both roots named, because the most likely cause is a
/// caller reaching for the wrong loader, and the fix is to say which one.
pub fn load_replay(record_path: &Path, area: &str, program: &str) -> HarnessResult<Manifest> {
    let context = format!(
        "reading the reproducer expectation record {}",
        record_path.display()
    );

    if !record_path.is_absolute() {
        return Err(HarnessError::new(
            context,
            format!(
                "the reproducer record path {} is not absolute; a finding's artifacts are addressed \
                 from the package manifest directory so that replaying one never depends on the \
                 working directory",
                record_path.display()
            ),
        ));
    }

    let scoped = format!("{context}: the reproducer record path");
    require_regular_file(&scoped, record_path)?;

    let generated = findings_root();
    let curated = curated_findings_root();
    let resolved = match ensure_within(&scoped, &generated, record_path) {
        Ok(resolved) => resolved,
        Err(_) => ensure_within(&scoped, &curated, record_path).map_err(|_| {
            HarnessError::new(
                context.clone(),
                format!(
                    "{} does not resolve beneath either directory a reproducer may occupy: the \
                     generated set at {} or the curated set at {}; a corpus record is loaded with \
                     the corpus loader instead, which additionally requires the record's own path \
                     to restate the program's identity",
                    record_path.display(),
                    generated.display(),
                    curated.display()
                ),
            )
        })?,
    };

    let found = resolved
        .extension()
        .and_then(|value| value.to_str())
        .map(String::from);
    if found.as_deref() != Some(RECORD_EXTENSION) {
        return Err(HarnessError::new(
            context,
            format!(
                "the reproducer record path {} does not have the required extension \
                 {RECORD_EXTENSION:?}; a finding ships a .{SOURCE_EXTENSION} reproducer paired with \
                 a sibling .{RECORD_EXTENSION} record",
                resolved.display()
            ),
        ));
    }

    let text = read_record_text(&resolved)?;
    parse_standalone_str(&text, &resolved, area, program)
}

/// The committed, curated findings directory inside the corpus.
///
/// Named in exactly one place so the replay loader and the prose that documents it cannot drift.
/// This module only ever **reads** beneath it; writing into the curated set is reserved for a human
/// promoting a reviewed finding, and no run may touch it.
fn curated_findings_root() -> PathBuf {
    corpus_root().join(CURATED_FINDINGS_DIR_NAME)
}

/// Read a record's text with every read bounded and the opened file proved to be the vetted one.
///
/// Four guarantees apply, and each covers a shape the others do not:
///
/// 1. The entry is inspected without following a final link and the **opened handle** is proved to be
///    the entry that was inspected, by `super::open_verified_regular_file`. A record path is derived
///    from the program's own path and is therefore predictable, so a name re-pointed between the
///    inspection and the open would otherwise have this function parse a file from anywhere on the
///    machine as though it were the corpus's own record — golden stdout, command templates and marker
///    included.
/// 2. The size reported by that handle is checked **before** any byte is read, so an oversized record
///    is refused without being held in memory.
/// 3. The reader is wrapped in [`std::io::Read::take`] one byte past the same bound, because a file
///    can grow between the metadata call and the read. This is the check that makes the second one
///    honest rather than advisory, and the extra byte is what makes overflow *detectable* instead of
///    silently truncating the record to exactly the limit.
/// 4. The bytes are required to be valid UTF-8, and an invalid record is refused with its byte
///    offset named rather than replaced with substitution characters. A record is written into
///    reports and reproduction commands verbatim, so silently rewriting its bytes would mean the
///    suite reported something the corpus does not contain.
fn read_record_text(path: &Path) -> HarnessResult<String> {
    use std::io::Read;

    let context = format!("reading the expectation record {}", shown_path(path));
    // The open and its verification are one step, so the handle is proved to be the entry that was
    // inspected: a record path is derived from the program's own path and is therefore predictable,
    // and a name re-pointed between an inspection and an open would have this function parse a file
    // from anywhere on the machine as though it were the corpus's own record.
    let (file, metadata) = super::open_verified_regular_file(&context, path).map_err(|error| {
        HarnessError::new(
            context.clone(),
            format!(
                "{}; every program in the corpus is paired with a sibling `.{RECORD_EXTENSION}` \
                 record holding its command templates and its golden output, and a record that \
                 cannot be read is a corpus defect rather than a reason to skip the program",
                error.cause()
            ),
        )
    })?;
    if metadata.len() > RECORD_BYTES_MAX {
        return Err(HarnessError::new(
            context,
            format!(
                "the record is {} bytes, above the {RECORD_BYTES_MAX}-byte limit; the largest \
                 record in the committed corpus is roughly ten kilobytes, so a file this large is \
                 corrupt or adversarial rather than one the corpus could contain, and reading it \
                 would let a data file decide how much memory the suite uses",
                metadata.len()
            ),
        ));
    }

    let mut bytes: Vec<u8> = Vec::new();
    // Bounded again on the reader: the size above was true when it was taken, and the file may
    // have grown since. One extra byte is permitted so that exceeding the limit is detectable
    // rather than silently truncating the record to exactly the limit.
    let mut bounded = file.take(RECORD_BYTES_MAX + 1);
    bounded.read_to_end(&mut bytes).map_err(|error| {
        HarnessError::new(
            context.clone(),
            format!("{} could not be read: {error}", shown_path(path)),
        )
    })?;
    if bytes.len() as u64 > RECORD_BYTES_MAX {
        return Err(HarnessError::new(
            context,
            format!(
                "the record exceeded the {RECORD_BYTES_MAX}-byte limit while being read, so it \
                 grew after its size was checked; the read is bounded independently for exactly \
                 this reason, and the record is refused rather than partly parsed"
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        let offset = error.utf8_error().valid_up_to();
        HarnessError::new(
            context,
            format!(
                "the record is not valid UTF-8: the first invalid byte is at offset {offset}. A \
                 record's text is written into reports and reproduction commands verbatim, so an \
                 unreadable byte is refused rather than replaced with a substitution character, \
                 which would make the suite report something the corpus does not contain"
            ),
        )
    })
}

/// Load the record that governs a program, given the program's own path.
///
/// The record is the program's sibling, differing only in extension. A program with no sibling
/// record is a hard error and never a skip, for the same reason: the pairing is what makes the
/// requirement "source file, build commands, and expected output recorded together" true, and a
/// program without its half of that pairing cannot be reproduced by hand or judged by the
/// golden-record oracle.
///
/// # Errors
///
/// Fails when `c_path` does not carry the program extension, when the program itself does not
/// pass containment, when the derived sibling record is absent or is not a regular file, and for
/// every reason [`load`] documents once the record path has been derived. The program is resolved
/// before the record path is derived from it, so a link or an escape is refused before it can be
/// used to legitimise a record path outside the corpus.
pub fn load_for_source(c_path: &Path) -> HarnessResult<Manifest> {
    let context = format!(
        "resolving the expectation record for {}",
        shown_path(c_path)
    );
    let extension = c_path.extension().and_then(|value| value.to_str());
    if extension != Some(SOURCE_EXTENSION) {
        return Err(HarnessError::new(
            context,
            format!(
                "the path is not a `.{SOURCE_EXTENSION}` program, so it has no sibling record; \
                 every corpus program is a `.{SOURCE_EXTENSION}` file paired with a \
                 `.{RECORD_EXTENSION}` record of the same stem"
            ),
        ));
    }
    // Resolve the program first, so a link or an escape is refused before it can be used to
    // derive — and thereby legitimise — a record path outside the corpus. Deriving the record
    // first and checking it later would let a path from outside the corpus name a sibling inside
    // it, or the reverse, and either way the pair would not be the pair the corpus contains.
    let resolved_source =
        require_contained_corpus_file(&context, "program", c_path, SOURCE_EXTENSION)?;
    let record = resolved_source.with_extension(RECORD_EXTENSION);
    // `is_file` follows a symbolic link, so it would answer "yes" for a committed link pointing at
    // any readable file on the machine — whose bytes would then be parsed and quoted back through
    // parser diagnostics, which is a disclosure channel. `require_regular_file` inspects the link
    // itself and refuses it, and `load` re-establishes both properties on the path it is handed.
    require_regular_file(&context, &record).map_err(|error| {
        HarnessError::new(
            context.clone(),
            format!(
                "{error}. A program without its record is a corpus defect and is reported rather \
                 than skipped, because skipping it would silently remove every one of its cells \
                 from the matrix"
            ),
        )
    })?;
    load(&record)
}

/// Reject a companion file in a feature area that no program claims.
///
/// Every recognised companion is an expectation record, and a record governs exactly one program:
/// its same-stem sibling. A record whose stem matches no program in the area is therefore either a
/// record whose program was deleted or renamed, or a record misnamed at birth — and both are
/// silent failures of exactly the shape this suite exists to prevent. Nothing would compile it,
/// nothing would run it, and nothing would say so; the run would report a complete pass over a
/// matrix that was quietly one program smaller than the corpus appears to be.
///
/// Only this direction needs checking here. The opposite direction — a program with no record — is
/// already a hard failure, because resolving a cell requires the record and reports its absence as
/// a corpus defect rather than skipping the program.
///
/// The stems are sorted and de-duplicated first, so the diagnostic names every unpaired record in
/// a stable order rather than whichever one the filesystem happened to hand over first.
fn require_paired_companions(
    area: &str,
    programs: &[PathBuf],
    companion_stems: &mut Vec<String>,
) -> HarnessResult<()> {
    companion_stems.sort();
    companion_stems.dedup();
    let program_stems: Vec<&str> = programs
        .iter()
        .filter_map(|program| program.file_stem().and_then(|stem| stem.to_str()))
        .collect();
    let orphans: Vec<&str> = companion_stems
        .iter()
        .map(String::as_str)
        .filter(|stem| !program_stems.contains(stem))
        .collect();
    if orphans.is_empty() {
        return Ok(());
    }
    Err(HarnessError::new(
        format!("discovering the programs of feature area {area:?}"),
        format!(
            "the expectation record(s) {} have no same-stem `.{SOURCE_EXTENSION}` program in this \
             area; a record governs exactly one program, its sibling of the same stem, which is \
             what makes a cell reproducible from those two files alone. An unpaired record is \
             either a record whose program was deleted or renamed, or a record misnamed at birth, \
             and neither would ever be compiled, run or mentioned — the run would report a \
             complete pass over a matrix quietly smaller than the corpus looks. Either restore \
             the program or remove the record",
            comma_separated(&orphans)
        ),
    ))
}

/// Every program in one feature area, resolved and sorted so that run order is deterministic.
///
/// The returned paths are resolved, matching [`load`] and the harness root's cell resolution, so
/// that discovery, record loading and cell identity all agree on the path a program is known by.
///
/// The scan is strict about what an area directory may contain, and nothing that could carry a
/// program is passed over silently. Programs are the `.c` files; the extensions in
/// [`AREA_COMPANION_EXTENSIONS`] — which is the expectation record and nothing else — are
/// recognised companions, and each one must pair with a same-stem program by
/// [`require_paired_companions`]; the whitelisted names in [`AREA_PLACEHOLDER_NAMES`] are ignored
/// by name rather than by pattern. Everything else — an unrecognised extension, a dot-prefixed
/// entry, a nested directory, a symbolic link — is a hard error.
///
/// That strictness is load-bearing rather than fussy. It mechanically enforces the rule that the
/// corpus tree contains no `.rs` file anywhere, which is what keeps the corpus invisible to the
/// build system and the suite free of any package-manifest change. And each rejection closes a way
/// for a program to disappear from the matrix without a word: an unrecognised extension catches a
/// program misnamed; a dot-prefixed entry is rejected rather than skipped, because a blanket skip
/// means renaming `007_x.c` to `.007_x.c` removes twelve cells from the matrix while the run still
/// reports success; a nested directory is rejected because an area contains files and nothing else,
/// the corpus's genuine subdirectories being siblings of the areas rather than inside one; and a
/// symbolic link is rejected rather than followed, for the reason given in
/// [`require_contained_corpus_file`].///
/// # Errors
///
/// Fails when `area` is not one of the corpus's feature areas, when the corpus root cannot be
/// resolved, and when the area's own directory is absent or unreadable — an area of the corpus is
/// never optional, so a missing one is a corpus defect rather than a smaller matrix. Within the
/// directory it fails on an entry that cannot be read, a name that is not valid UTF-8, a
/// dot-prefixed entry, a symbolic link, a nested directory, anything that is not a regular file,
/// an extension that is neither a program nor a recognised companion, a program that does not
/// resolve strictly beneath the corpus root, a companion record whose stem matches no program in
/// the area, and an area holding no program at all.
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
    // Resolve the corpus root once, so containment is decided against a real path.
    let root = canonical_corpus_root()?;
    let directory = corpus_root().join(name);
    let entries = fs::read_dir(&directory).map_err(|error| {
        HarnessError::new(
            format!("discovering the programs of feature area {name:?}"),
            format!(
                "{} could not be read: {error}; the corpus is discovered by scanning this \
                 directory, so an area that is absent or unreadable is a corpus defect",
                shown_path(&directory)
            ),
        )
    })?;

    let mut programs: Vec<PathBuf> = Vec::new();
    // Stems of the recognised companions, collected so that the pairing rule can be applied
    // once the whole directory has been seen. It cannot be applied entry by entry, because a
    // directory listing is in no particular order and a record may be visited before its
    // program.
    let mut companion_stems: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!(
                    "an entry of {} could not be read: {error}",
                    shown_path(&directory)
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
                        shown_path(&path)
                    ),
                )
            })?;
        if AREA_PLACEHOLDER_NAMES.contains(&file_name) {
            continue;
        }
        if file_name.starts_with('.') {
            return Err(HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!(
                    "{file_name:?} is a dot-prefixed entry, which an area directory may not \
                     contain except for {}. Skipping such an entry silently is worse than \
                     rejecting it: renaming a program to a dot-prefixed name — an editor backup, \
                     a partial checkout, a stray copy — would remove every one of its cells from \
                     the matrix while the run still reported success. Remove the entry or give it \
                     its proper name",
                    comma_separated(AREA_PLACEHOLDER_NAMES)
                ),
            ));
        }
        // `symlink_metadata` inspects the entry itself rather than what it points at. Following a
        // link here would mean a committed link could introduce a program the corpus does not
        // contain — a file that is then compiled AND EXECUTED — or an expectation record read
        // from anywhere on the machine. A link is therefore refused outright rather than
        // resolved, and the refusal is explicit rather than a silent skip, because a link that
        // someone committed on purpose is a corpus defect that must be seen.
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!("{} could not be inspected: {error}", shown_path(&path)),
            )
        })?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!(
                    "{file_name:?} is a symbolic link; an area directory holds only regular files \
                     that genuinely live in the corpus. A link is refused rather than followed, \
                     because every discovered program is compiled AND EXECUTED and every \
                     discovered record is parsed and quoted into diagnostics, so following one \
                     would let the corpus name a file it does not contain while every report and \
                     every reproduction command still showed the corpus path"
                ),
            ));
        }
        if file_type.is_dir() {
            return Err(HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!(
                    "{file_name:?} is a directory, and a feature area contains files and nothing \
                     else. The corpus's genuine subdirectories — the findings tree, the fixture \
                     support tree and the tooling tree — are siblings of the areas rather than \
                     children, so no legitimate layout needs this. Skipping it instead would let a \
                     whole tree of programs sit in the corpus, committed and never run"
                ),
            ));
        }
        if !file_type.is_file() {
            return Err(HarnessError::new(
                format!("discovering the programs of feature area {name:?}"),
                format!(
                    "{file_name:?} is neither a regular file nor a directory; a device node, \
                     socket or FIFO cannot be a program or a record, and reading one can block \
                     indefinitely"
                ),
            ));
        }
        match path.extension().and_then(|value| value.to_str()) {
            // The resolved path rather than the joined one, so that discovery, record loading and
            // cell resolution all agree on the path a program is known by, and so the containment
            // guarantee is visible in what this function returns rather than only asserted inside
            // it. The decision is delegated to the same primitive every other consumer uses, so
            // that discovery cannot develop its own idea of what "inside the corpus" means.
            Some(SOURCE_EXTENSION) => {
                let context = format!("discovering the programs of feature area {name:?}");
                programs.push(ensure_within(&context, &root, &path)?);
            }
            Some(companion) if AREA_COMPANION_EXTENSIONS.contains(&companion) => {
                match path.file_stem().and_then(|stem| stem.to_str()) {
                    Some(stem) => companion_stems.push(String::from(stem)),
                    None => {
                        return Err(HarnessError::new(
                            format!("discovering the programs of feature area {name:?}"),
                            format!(
                                "{file_name:?} has a stem that is not valid UTF-8; a companion is \
                                 paired with its program by stem, so a stem that cannot be read \
                                 cannot be paired"
                            ),
                        ));
                    }
                }
            }
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

    require_paired_companions(name, &programs, &mut companion_stems)?;

    if programs.is_empty() {
        return Err(HarnessError::new(
            format!("discovering the programs of feature area {name:?}"),
            format!(
                "{} contains no `.{SOURCE_EXTENSION}` program; an empty feature area is a corpus \
                 defect, because no area of the corpus is optional and none may be skipped",
                shown_path(&directory)
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
///
/// # Errors
///
/// Every rejection [`discover_area`] documents, for whichever area fails first. The areas are
/// visited in table order, so the same corpus defect is reported the same way on every run.
pub fn discover_all() -> HarnessResult<Vec<PathBuf>> {
    let mut programs: Vec<PathBuf> = Vec::new();
    for area in AREAS {
        programs.extend(discover_area(area.directory())?);
    }
    Ok(programs)
}

/// A digest of the corpus **as it is on disk**: every program's bytes and every record's bytes.
///
/// Consumed by the reporting module's run identity, which is what decides whether an area report
/// found on disk may contribute its rows to this run's totals. Nothing else about a run changes when
/// a program is edited — the configuration is the same, the tool set is the same, the declared program
/// counts are the same — so a digest derived from anything but the bytes would accept a report written
/// before the edit and add its rows to a summary describing the corpus after it. That is the failure
/// the identity exists to prevent, and content is the only thing that detects it.
///
/// Deterministic and content-addressed, which keeps the reporting module's determinism rule intact:
/// two runs over an unchanged corpus produce the same value on any machine and under any toolchain,
/// because the enumeration order is fixed by [`discover_all`] and [`stable_digest`] is a fixed
/// specification rather than the standard library's release-unstable hasher. The corpus root's path
/// is deliberately **not** an input: two checkouts of the same commit at different paths describe the
/// same corpus, and making the path matter would report a mismatch where there is none.
///
/// Infallible by construction, because it is computed on the path that writes a report and a report
/// must be written even when the corpus is defective. The degradation is **per feature area** rather
/// than all-or-nothing, and that matters on a checkout carrying some areas but not all: an area that
/// cannot be enumerated contributes the fact that it could not, while every area that *can* still
/// contributes its bytes. Aborting the whole digest on the first unreadable area would make it a
/// constant on such a checkout, so an edit to a program in a readable area would move nothing — which
/// is the very insensitivity this function exists to remove. The defect itself is reported loudly
/// elsewhere, by program discovery and by the undefined-behaviour audit, so it is not swallowed here,
/// merely survived.
///
/// A failure contributes the area or file it happened to and nothing more — never the diagnostic
/// text, which embeds the path it was reported for. Two checkouts of one commit at different paths
/// would otherwise disagree about a corpus they hold identically. Collapsing an absent area and an
/// unreadable one to the same component is correct rather than merely convenient: for this digest's
/// question — did that report describe this corpus? — both mean the same thing, that the area
/// contributed no bytes at all.
///
/// Each file is read through [`read_file_bounded`], so a corpus entry that has become a symbolic
/// link, a device or a directory is refused rather than followed, and an entry too large to inspect
/// is refused rather than held in memory. Both refusals land in the same unreadable path as any other.
pub fn corpus_content_digest() -> String {
    let context = "digesting the corpus content for this run's report identity";
    let mut components: Vec<String> = Vec::new();
    for spec in AREAS.iter() {
        let area = spec.directory();
        match discover_area(area) {
            Ok(programs) => {
                // The count is carried beside the per-file components so that an area gaining or
                // losing a pair differs even in the vanishing case where the surviving bytes digest
                // alike.
                components.push(format!("{area}:programs={}", programs.len()));
                for program in &programs {
                    components.push(corpus_file_component(context, program));
                    components.push(corpus_file_component(
                        context,
                        &program.with_extension(RECORD_EXTENSION),
                    ));
                }
            }
            Err(_) => components.push(format!("{area}:unenumerable")),
        }
    }
    let borrowed: Vec<&str> = components.iter().map(String::as_str).collect();
    stable_digest(&borrowed)
}

/// One corpus file's contribution to [`corpus_content_digest`]: its name, its length and its bytes.
///
/// The name is the area-and-file tail rather than the absolute path, for the reason
/// [`corpus_content_digest`] gives: the checkout's location is not part of what the corpus *is*. The
/// length is carried beside the byte digest so that a file whose bytes happen to digest alike still
/// differs when its size does, which costs nothing and removes one whole class of silent collision.
///
/// A file that cannot be read contributes `unreadable`, not silence. Silence would let an unreadable
/// record digest identically to a readable empty one.
fn corpus_file_component(context: &str, path: &Path) -> String {
    let name = corpus_relative_name(path);
    match read_file_bounded(context, path, MAX_INSPECTED_FILE_BYTES) {
        Ok(bytes) => format!(
            "{name}:{}:{:0width$x}",
            bytes.len(),
            fnv1a64_bytes(&bytes),
            width = DIGEST_HEX_DIGITS
        ),
        Err(_) => format!("{name}:unreadable"),
    }
}

/// A corpus path as `<area>/<file>`, falling back to the whole shown path if it lies elsewhere.
///
/// The fallback cannot arise for a path [`discover_all`] produced — every one of those is proved
/// strictly beneath the corpus root — and exists so that this function is total rather than
/// panicking on a shape it was not given.
fn corpus_relative_name(path: &Path) -> String {
    let area = path.parent().and_then(Path::file_name);
    let file = path.file_name();
    match (area, file) {
        (Some(area), Some(file)) => format!(
            "{}/{}",
            area.to_string_lossy().as_ref(),
            file.to_string_lossy().as_ref()
        ),
        _ => shown_path(path),
    }
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
///
/// # Errors
///
/// Fails on a duplicate marker identifier, and on every rejection [`discover_all`] and
/// [`load_for_source`] document — because enumerating the markers means loading every record in
/// the corpus, a defect in any one of them surfaces here rather than being stepped over.
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
