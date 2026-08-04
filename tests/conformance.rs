//! Differential conformance suite — the driver.
//!
//! This file is the whole Cargo-visible surface of the suite. It is a top-level `.rs` file
//! under `tests/`, so Cargo discovers it automatically as the integration-test target
//! `conformance` and no manifest change is required to register it: `cargo test --test
//! conformance`. `tests/conformance_harness/` holds no `main.rs`, so Cargo never treats it as
//! a target and it is reached below as an ordinary module, exactly the idiom this repository
//! already uses for a shared test-helper directory. `tests/conformance/` holds only data —
//! `.c` programs, their `.expected` records, the registers and the fixture header — and no
//! `.rs` file at all, so it is invisible to Cargo and cannot collide with this file's name.
//!
//! **Auto-discovery requires a manifest to be discovered from, and this suite deliberately does
//! not supply one.** "No manifest change is required" is a statement about not *editing*
//! `Cargo.toml`, which C1 makes read-only — not a claim that the suite works without a package.
//! Every `cargo` invocation above presupposes the repository's own `Cargo.toml`, its `bcc` binary
//! target and its `src/**` tree. On a checkout that carries this suite ahead of the compiler tree
//! the `cargo` gates therefore do not run at all, while `rustfmt --check` and a direct
//! `rustc --test --emit=metadata` on this file still do, because neither needs a package. The
//! direct `rustc` form needs one thing Cargo would otherwise have supplied: `CARGO_MANIFEST_DIR`,
//! which `conformance_harness::manifest_dir` reads through the compile-time `env!` macro to
//! resolve every corpus path from the package root. Run it as
//! `CARGO_MANIFEST_DIR="$(pwd)" rustc --edition 2021 --test --emit=metadata --out-dir
//! target/conformance-typecheck tests/conformance.rs` from the repository root; without the
//! variable it stops at that macro rather than at anything wrong with the suite. That
//! split, the way the remaining gates are established without adding a manifest anywhere, and the
//! fact that the resolution is to merge onto the branch that already carries the package rather
//! than to create one here, are recorded in full under "The Cargo integration precondition" in
//! `tests/conformance/README.md`. The run-time resolution of the compiler under test described
//! below is what keeps that arrangement workable.
//!
//! # What this suite decides, and how
//!
//! Every cell of the matrix — one program, one target, one optimization level — is judged on
//! **stdout bytes and exit status alone**, by three independent oracles:
//!
//! - **(a) reference compiler.** bcc against an external reference C compiler at the same
//!   target and level. Catches a wrong answer bcc produces consistently on all four of its
//!   backends, which cross-backend comparison structurally cannot see.
//! - **(b) cross backend.** Every non-baseline target against the x86-64 baseline at the same
//!   level. x86-64 is the baseline because it is the host, so the authority runs with no
//!   emulator and introduces no emulation-related variable. Catches a wrong answer confined to
//!   one code generator — where an ABI, register-allocation or instruction-selection defect
//!   surfaces.
//! - **(c) golden record.** Every cell against the `expected_stdout` committed in the
//!   program's own `.expected` record. Catches the one failure mode differential testing
//!   cannot: both compilers moving the same way at the same time, which oracle (a) reports as
//!   agreement.
//!
//! Standard error is captured into finding artifacts but **never compared**: diagnostic
//! wording legitimately differs between compilers, and comparing it would bury real
//! divergences under noise that says nothing about code correctness.
//!
//! # Eighteen tests, and why they are shaped this way
//!
//! Fourteen area tests, one per corpus directory and named after it, plus four infrastructure
//! tests, every one prefixed `infra_`. The names are a contract with the repository's own
//! documentation, which spells `cargo test --test conformance area_04_bitfields` and
//! `cargo test --test conformance infra_` verbatim.
//!
//! Each area test runs its **entire** matrix, accumulates **every** verdict, and asserts only
//! at the end. That is deliberate rather than lax: the built-in harness stops a test at its
//! first failed assertion, and this suite's deliverable is a summary enumerating every
//! outcome — every expected divergence with its documented basis and every finding with its
//! reproducer. Asserting inside the loop would truncate exactly the artifact the requirements
//! ask for. The failure message therefore reproduces the complete per-program verdict table,
//! so nothing is lost in the runner output either.
//!
//! # The two preconditions are gates, not peers
//!
//! Two of the four infrastructure tests establish preconditions rather than compare anything: that
//! every flag a differential invocation passes means the same thing to both compilers (requirement
//! 3), and that the corpus is free of undefined behaviour (requirement 1). Requirement 1 states the
//! consequence in its own terms — a program containing undefined behaviour permits both compilers to
//! do anything — so while either is unmet, a `PASS` is not evidence of agreement and a divergence is
//! not evidence of a defect.
//!
//! They therefore cannot be eighteen-way peers whose failure leaves fourteen area verdicts standing.
//! The built-in harness runs all eighteen tests concurrently in one process with no ordering between
//! them, so "run the gates first" is not something a caller can arrange and not something a test can
//! assert. What [`preflight`] does instead is perform each gate **once per process**, record it in the
//! report module before any artifact is written, and hand it to every area — which names it in its
//! report, counts it in the run's verdict, and **asserts** on the gates that govern it. A gate that
//! could not be applied at all is not silently a pass either: it is reported as unproven and
//! escalated to a failure under the strict setting, the same rule the suite applies to an oracle
//! whose tooling is absent. The two infrastructure tests keep their own, fuller assertions, because
//! the whole gate report — every command line and the compiler's own words — is what an author fixes
//! a program from.
//!
//! The verdict space is closed and has no silent-skip member. `PASS`, `XFAIL`, `FINDING` and
//! `UNAVAILABLE` are permitted in a passing run and every one of them is still reported;
//! `FAIL` and `XPASS` fail it. An unexpected success fails by default because a stale marker
//! is stale documented knowledge that will mislead the next reader, while retiring one is a
//! trivial test-only edit. An oracle whose tooling is genuinely absent is `UNAVAILABLE`,
//! loudly and in the summary, and becomes a failure under the strict setting intended for
//! continuous integration. An absent compiler under test is neither: it is an immediate hard
//! failure, because without it there is nothing to test and no result could be trusted.
//!
//! # What this file does not do
//!
//! It orchestrates and asserts. Discovery, workspace allocation, compilation, execution,
//! comparison, classification, finding artifacts, reporting, flag probing and the
//! undefined-behaviour audit all live in `conformance_harness`; no process is spawned and no
//! byte compared here. It does not reach for Cargo's compile-time binary-path macro either:
//! resolving the compiler under test is `env.rs`'s single responsibility, which it discharges with
//! `option_env!` plus the `BCC_BIN` override so that a package without a `bcc` binary target
//! produces an explanatory run-time failure instead of an inscrutable compile error.
//!
//! # Integration with `tests/common/mod.rs`, and what happens in each case
//!
//! The repository documents sixteen existing integration suites sharing one helper module,
//! `tests/common/mod.rs` (`docs/project-guide.md`, test inventory), and this suite is required to
//! integrate with that convention rather than duplicate it. Neither those suites nor that helper is
//! present in this checkout: the branch carries the project documentation and this suite, while the
//! compiler crate, its manifest and its existing tests live on the project's open pull request. The
//! obligation is therefore conditional on the file existing, because `mod common;` is a compile-time
//! assertion that it does: declared against an absent file it does not fall back, it fails the build,
//! and a suite that cannot compile reuses nothing.
//!
//! **If `tests/common/mod.rs` is present**, this file declares `mod common;` and delegates to it —
//! specifically its compile helper, its run helper, its assertion macros and its temporary-directory
//! management — instead of the harness's own equivalents. That is the intended shared-code
//! dependency, and it is the only one.
//!
//! **If it is absent**, as on a branch carrying the suite ahead of the shared helper, the
//! declaration is omitted and `conformance_harness` stands alone. It is written to be functionally
//! self-contained in pure `std` for exactly this reason, so the suite is complete either way.
//!
//! Which case holds is not left to a reader's inspection of this comment. It is **checked
//! mechanically** on every run: [`infra_oracle_capability_report`] reports the helper module's state
//! in its pre-flight output, and fails loudly if the file is present in the tree while this driver
//! does not declare it. Prose describing an integration cannot notice when it stops being true; that
//! assertion can.
//!
//! What the suite adds rather than delegates is the material no existing helper has: the three
//! oracles, the expectation-record format, the six-verdict classification, the expected-divergence
//! markers, the finding artifacts and the run summary. Those are extensions of the convention, not
//! departures from it — the suite is still an auto-discovered `tests/<name>.rs` integration target
//! whose shared code lives in a nested non-target directory module, invoked as
//! `cargo test --test conformance`, using only the standard library and the built-in harness.
//!
//! No compiler source change is ever made in response to a finding. Findings are
//! deliverables.
//!
//! # Every finding leaves this file with its artifacts, or it is not a finding
//!
//! A `FINDING` does not fail the run, and that is only sound while the deliverable behind it
//! exists. So there is exactly one route from an observation to an outcome here —
//! [`CellPlan::deliver`] — and it is total over both shapes an observation can take: a comparison
//! that differed, and a compiler that produced no artifact at all. Both are settled the same way,
//! both carry their evidence into the same artifact directory, and when those artifacts cannot be
//! written the verdict becomes `FAIL`, which does fail the run. A divergence nobody can reproduce
//! is an unexplained result, not a delivered one.
//!
//! The second half of that discipline lives in `classify.rs`: a refusal happens once per cell,
//! before any oracle is asked, and it removes the authority every one of them needs. It nevertheless
//! reaches the classifier once per oracle arm, and each arm is matched against a marker's scope
//! **strictly, on all four dimensions — class, oracle, target and optimization level**. A marker is
//! therefore never widened on its own behalf, and every outcome detail names both the oracle the
//! marker covers and the oracle being judged, so the two can never be confused.
//!
//! What follows from a refusal being **one root event** is handled by dependency-aware
//! classification rather than by widening anything. `classify::refusal_root` names the arm whose
//! marker documents a refusal; that arm settles it as an expected divergence, and every other arm of
//! the same cell is reported as a **dependent blocked** expected divergence which cites the root
//! marker, names the arm carrying it, and states that no comparison was attempted on it. No finding
//! directory is written for the blocked arms, because one event has one explanation — but the
//! propagation requires a marker that already covers this cell's target, level and observed class on
//! some arm, so an *undocumented* refusal is still a finding on every applicable arm, delivered with
//! the refusal's own artifacts.
//!
//! **Which markers are live is deliberately not stated here.** An earlier form of this comment named
//! the corpus's marker inventory outright and drew a conclusion from it — that no refusal-class marker
//! was live, so the path above was a mechanism with no instance. That is the same defect as a stale
//! measurement: a claim about the tree, maintained where it cannot observe the tree, in a corpus that
//! is still being completed program by program. The inventory is therefore reported rather than
//! asserted. [`infra_expected_divergence_register`] enumerates every marker it finds, prints each one
//! with its class, scope, program and basis, and states how many expectation records it was able to
//! read out of how many the plan calls for — so a reader learns what is live from the run that just
//! examined the corpus, and this comment cannot be wrong about it.
//!
//! Neither marker is presented here as an adjudicated result, and the distinction matters because a
//! marker is the one mechanism that turns a failure into a pass. What the run establishes is that each
//! is registered in both directions, that every field agrees with its record character for character,
//! and that every cited document, locator and quotation resolves. Whether a cited passage *supports*
//! what its marker excuses is a reviewer's judgement made against
//! `tests/conformance/EXPECTED_DIVERGENCES.md` §2.4.1 — and for `XD-GCCEXT-CASE-RANGES-001` the
//! divergence itself has no independent capture, because the only compiler under test this checkout
//! can offer forwards to the reference toolchain. That gap is disclosed in the marker rather than
//! papered over, and it is not free: while the construct works, the marker's arm agrees, the verdict
//! is `XPASS` and the run **fails** until somebody with a real `bcc` settles it.
//!
//! Only the standard library is used, every operation is safe, no lint is suppressed, and not one
//! of the eighteen tests is marked ignored — the repository's ignored-test count is itself the most
//! direct mechanical check that no pre-existing test was skipped or weakened. Edition 2021, minimum
//! supported Rust 1.70.

mod conformance_harness;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use conformance_harness::classify::{self, Attribution};
use conformance_harness::compare::{self, Comparison, RefusedSide};
use conformance_harness::compile::{self, CompileOutcome, CompileRequest, Compiler};
use conformance_harness::env::{self as discovery, Capabilities, RunConfig};
use conformance_harness::execute::{self, CaptureNames, RunAttempt, RunOutcome};
use conformance_harness::findings::{self, Capture, CaptureRole, Finding};
use conformance_harness::flagprobe;
use conformance_harness::manifest::{self, CommandSubstitutions, Manifest};
use conformance_harness::report;
use conformance_harness::sandbox::{self, Workspace};
use conformance_harness::ubaudit;
use conformance_harness::{
    corpus_root, findings_root, manifest_dir, redact_secrets, report_root, shown_path,
    target_dir_rejection, work_root, AreaSpec, Cell, CellKey, DivergenceClass, HarnessError,
    MarkerClass, OptLevel, Oracle, Outcome, Provenance, Target, Verdict, AREAS, AREA_COUNT,
    BCC_CELL_COUNT, MIN_PROGRAMS_PER_MANDATED_AREA, OPT_LEVEL_COUNT, ORACLE_A_COMPARISON_COUNT,
    ORACLE_B_COMPARISON_COUNT, ORACLE_C_ASSERTION_COUNT, PROGRAM_COUNT, TARGET_COUNT,
    TOTAL_ASSERTION_COUNT,
};

// =================================================================================================
// The fourteen feature-area tests
//
// One test per corpus directory, named `area_<NN>_<directory>` so that a single area can be
// selected by name, which the repository's documentation relies on. Each is a delegation: the
// matrix, the accumulation and the assertion are shared, so adding an area is two lines here and
// one row in the harness's area table.
// =================================================================================================

/// Requirement 2, integer promotions and the usual arithmetic conversions across signedness
/// and width — including the runtime variant of every conversion, so the constant folder's
/// answer cannot stand in for the backend's.
#[test]
fn area_01_integer_conversions() {
    run_area("01_integer_conversions");
}

/// Requirement 2, constant expressions and folding, paired with runtime twins so a divergence
/// is attributable either to the folder or to the code generator rather than to "one of the
/// two".
#[test]
fn area_02_constant_expressions() {
    run_area("02_constant_expressions");
}

/// Requirement 2, aggregate and array initializers: nested, partial with implicit zero fill,
/// and designated in array, struct and mixed forms.
#[test]
fn area_03_initializers() {
    run_area("03_initializers");
}

/// Requirement 2, bitfields including assignment and compound assignment, straddling fields
/// and zero-width separators.
#[test]
fn area_04_bitfields() {
    run_area("04_bitfields");
}

/// Requirement 2, pointer arithmetic and function pointers — one-past-end pointers formed but
/// never dereferenced, and dispatch through a table of function pointers.
#[test]
fn area_05_pointers() {
    run_area("05_pointers");
}

/// Requirement 2, control flow: switch edge cases across dense and sparse label sets, and
/// short-circuit evaluation whose suppressed side effect is observed through `volatile`
/// storage.
#[test]
fn area_06_control_flow() {
    run_area("06_control_flow");
}

/// Requirement 2, variadic functions: default argument promotions, `va_copy`, forwarding, and
/// argument counts that exhaust every target's argument registers.
#[test]
fn area_07_variadics() {
    run_area("07_variadics");
}

/// Requirement 2, the GCC extensions this repository documents as supported — plus case
/// ranges, which no documented inventory lists and which is therefore written and executed
/// anyway rather than dropped for difficulty.
#[test]
fn area_08_gcc_extensions() {
    run_area("08_gcc_extensions");
}

/// Requirement 2's final item, behaviour at multiple optimization levels — asserting that
/// program *output* is identical at `-O0`, `-O1` and `-O2`, which is a property of the
/// optimizer's correctness rather than of its passes having fired.
#[test]
fn area_09_optimization_levels() {
    run_area("09_optimization_levels");
}

/// Declarations and types: complex declarators, qualifiers, linkage, and by-value aggregate
/// copy and return — the repository's own open risk about declarator and conversion edge
/// cases.
#[test]
fn area_10_declarations_and_types() {
    run_area("10_declarations_and_types");
}

/// Literals and strings: escape sequences, concatenation and indexing, and the wide and
/// Unicode forms whose support no documented feature list enumerates.
#[test]
fn area_11_literals_and_strings() {
    run_area("11_literals_and_strings");
}

/// The preprocessor, end to end: expansion, stringize and paste, nested conditionals,
/// variadic macros, and the dedicated program that probes the complete nine-header required
/// bundled freestanding set. It is one of the corpus's two sanctioned header exceptions — the
/// other being area 07, whose variadic programs may include `<stdarg.h>` and nothing else —
/// and the bonus `stdatomic.h` is deliberately excluded from both.
/// Area 07 includes `<stdarg.h>` too, but as the only means of writing a variadic function
/// rather than as the thing under test.
#[test]
fn area_12_preprocessor() {
    run_area("12_preprocessor");
}

/// Floating point on finite values only, plus `long double`, whose representation differs by
/// target and which is therefore compared against its same-target reference while cross-backend
/// value equality alone is excluded, with the reason recorded in the program's own record.
#[test]
fn area_13_floating_point() {
    run_area("13_floating_point");
}

/// ABI and calling convention: register-exhausting parameter lists, aggregates on both sides of
/// every by-register threshold, by-value aggregate return, and callee-saved preservation across
/// nested calls. The densest concentration of cross-backend divergence surface in the corpus.
#[test]
fn area_14_abi_calling_convention() {
    run_area("14_abi_calling_convention");
}

// =================================================================================================
// The four infrastructure tests
//
// Every one is prefixed `infra_` so that `cargo test --test conformance infra_` selects exactly
// these four and nothing else. Each turns one requirement that could have been a prose claim into
// an executable assertion.
// =================================================================================================

/// Requirement 3 — verify flag handling rather than assuming it.
///
/// Every differential comparison in this suite rests on both compilers giving the same meaning to
/// the flags it passes, so the probe checks an **observable consequence** of each flag rather than
/// mere acceptance, and asserts **negatively** that no reference-compiler-only and no bcc-only flag
/// has leaked into the shared set. Acceptance alone would have been a false positive: both
/// compilers accept control-flow protection, and their default scopes differ.
///
/// The probe itself is performed by [`preflight`] rather than here, and this test reads the memoized
/// result. Two reasons: it is then performed once per process instead of once here and once for the
/// areas, and — the reason that matters — the same result is recorded in every area report and
/// asserted by every area, so a failure here can no longer sit beside fourteen areas each reporting a
/// green matrix whose comparisons are not evidence. What this test adds is the **full** account: every
/// row, both command lines and the observable that did not hold, which is what a reader acts on.
#[test]
fn infra_flag_capability_probe() {
    let caps = oracle_capabilities();
    prepare_roots();
    begin_report_session();
    let probe = preflight(&caps).flag_probe("infra_flag_capability_probe");

    println!("{}", probe.render());
    if !probe.satisfied() {
        println!(
            "note: {} flag check(s) could not be performed in this environment; they are listed \
             above and are not silently treated as verified.",
            probe.unavailable().len()
        );
    }

    assert!(!probe.fails_run(), "{}", probe.failure_summary());
}

/// Requirement 1 — undefined-behaviour freedom, proven rather than asserted.
///
/// Drives every corpus program through the reference compiler's strict warning gate and a
/// sanitizer run, honouring the per-program deviations that carry a recorded reason. Both gates are
/// reference-compiler-only, so requirement 3's shared-flag discipline is untouched, and neither
/// renders a verdict about bcc: a gate failure is a defect in the **test program**, because a
/// program containing undefined behaviour makes any divergence it provokes unattributable.
///
/// As with the flag probe, the audit is performed by [`preflight`] and this test reads the memoized
/// result, so the same audit that every area is judged against is the one reported here — and every
/// area whose programs failed a gate fails with it, rather than this test failing alone while fourteen
/// areas report clean matrices. Two reference-compiler invocations per corpus program is also
/// precisely the cost that must not be paid twice.
#[test]
fn infra_ub_audit_gate() {
    let caps = oracle_capabilities();
    prepare_roots();
    begin_report_session();
    let audit = preflight(&caps).ub_audit("infra_ub_audit_gate");

    println!("{}", audit.render());
    if let Some(reason) = audit.unavailable_reason() {
        println!("note: no gate could be applied — {reason}");
    }
    if audit.is_partial() {
        println!(
            "note: a filter meant this audit covered {} of the {} program(s) discovered, so it is \
             partial coverage and cannot be read as a clean audit of the whole corpus.",
            audit.program_count(),
            audit.discovered_count(),
        );
    } else if audit.is_clean() {
        println!(
            "note: all {} program(s) satisfied both gates in {} invocation(s).",
            audit.program_count(),
            audit.invocations_performed(),
        );
    }

    assert!(
        !audit.fails_run(),
        "the undefined-behaviour freedom gate did not hold for {} of {} program(s), and {} gate(s) \
         could not be applied.\n\nRequirement 1 makes undefined-behaviour freedom the precondition \
         that makes both differential oracles sound: while these gates fail, a divergence between \
         two compilers is not evidence of a defect in either, because both are permitted to do \
         anything. Correct the TEST PROGRAM in the corpus — never the compiler — or record a \
         reasoned gate deviation in the program's own expectation record.\n\n{}",
        audit.failures().len(),
        audit.program_count(),
        audit.tally(ubaudit::GateStatus::Unavailable),
        audit.render(),
    );
}

/// Requirement 5 — an expected divergence is marked, never silently excluded.
///
/// Asserts consistency in **both** directions, which is what stops the marker set from decaying
/// into stale documentation:
///
/// - **forward, mention** — every marker committed in a program's expectation record appears in the
///   register;
/// - **forward, description** — every such marker has exactly one *structured entry* there, and all
///   six of its fields agree with the record. Being mentioned is not being described: an identifier
///   can sit in a heading while the register says nothing checkable, and the register could then
///   drift away from the record it mirrors with no run noticing;
/// - **reverse** — every identifier the register lists corresponds to a real, active marker;
/// - **basis** — every cited document is contained in this repository on its *resolved* path, is
///   readable as a committed regular file through the suite's bounded reader, and actually contains
///   the section the citation names. Existence alone was never the property that mattered.
///
/// A marker changes how a divergence is *classified*; it never changes whether the feature is
/// *exercised*.
///
/// # Why this test asserts what a preflight gate has already blocked on
///
/// The audit itself lives in [`marker_integrity`] and is performed **once per process**, before any
/// area compiles its first cell, so that a marker cannot excuse a divergence that this audit has not
/// yet approved of. This test keeps its own, fuller assertion because the whole audit — every
/// violation in full, the marker inventory, the curated-finding reconciliation and the
/// classification tally — is what a maintainer fixes a register or a record from, and a gate line in
/// fourteen area reports is not.
#[test]
fn infra_expected_divergence_register() {
    // Deliberately does not call `oracle_capabilities()`: this reads committed files only, so it
    // stays meaningful on a machine with no reference compiler and no emulator at all. It still
    // begins the report session, which needs no capabilities: this test writes no area report, and
    // a run consisting only of tests that write none is exactly the case in which a previous run's
    // summary would otherwise survive and be mistaken for this run's verdict.
    begin_report_session();
    let audit = marker_integrity();
    print!("{}", audit.narrative());

    assert!(
        audit.violations().is_empty(),
        "the expected-divergence markers and {} are not consistent — {} violation(s):\n{}",
        classify::EXPECTED_DIVERGENCE_REGISTER,
        audit.violations().len(),
        audit
            .violations()
            .iter()
            .map(|violation| format!("  - {}", violation.message()))
            .collect::<Vec<String>>()
            .join("\n"),
    );
}

/// One defect found by [`marker_integrity`], and the area it bears on.
///
/// The area matters because it decides how widely the gate this audit feeds blocks. A defect in one
/// program's marker says nothing about another area's programs, exactly as a failed
/// undefined-behaviour gate does not; a defect in the register itself, or in the curated finding set,
/// belongs to no single area and therefore governs every one of them.
#[derive(Debug, Clone)]
struct MarkerViolation {
    /// The feature area this defect is attributable to, or `None` when it belongs to the whole run.
    area: Option<String>,
    /// What was found, in the sentence the test and the gate both report.
    message: String,
}

impl MarkerViolation {
    /// A defect attributable to one feature area.
    fn in_area(area: impl Into<String>, message: impl Into<String>) -> MarkerViolation {
        MarkerViolation {
            area: Some(area.into()),
            message: message.into(),
        }
    }

    /// A defect that belongs to no single area and therefore governs every one of them.
    fn run_wide(message: impl Into<String>) -> MarkerViolation {
        MarkerViolation {
            area: None,
            message: message.into(),
        }
    }

    /// The feature area this defect is attributable to, or `None` for a run-wide one.
    fn area(&self) -> Option<&str> {
        self.area.as_deref()
    }

    /// What was found.
    fn message(&self) -> &str {
        &self.message
    }
}

/// The whole bidirectional marker audit, performed once per process.
#[derive(Debug)]
struct MarkerIntegrity {
    /// Every defect found, with the area each is attributable to.
    violations: Vec<MarkerViolation>,
    /// The full account the infrastructure test prints: inventory, reconciliation and tallies.
    narrative: String,
    /// How many markers the corpus committed, for the gate's one-line detail.
    marker_count: usize,
}

impl MarkerIntegrity {
    /// Every defect found, in the order the audit found them.
    fn violations(&self) -> &[MarkerViolation] {
        &self.violations
    }

    /// The full account, already formatted with its trailing newline.
    fn narrative(&self) -> &str {
        &self.narrative
    }

    /// How many markers the corpus committed.
    fn marker_count(&self) -> usize {
        self.marker_count
    }

    /// Every defect that bears on the given area: its own, plus every run-wide one.
    ///
    /// Written as a `match` rather than with an `Option` combinator so that the minimum supported
    /// Rust version this repository documents (1.70) is respected: `Option::is_none_or` is the
    /// natural spelling and is stable only from 1.82.
    fn violations_for(&self, area: &str) -> Vec<&MarkerViolation> {
        self.violations
            .iter()
            .filter(|violation| match violation.area() {
                Some(owner) => owner == area,
                None => true,
            })
            .collect()
    }

    /// The distinct areas this audit found a defect in, in the order it found them.
    fn defective_areas(&self) -> Vec<String> {
        let mut areas: Vec<String> = Vec::new();
        for violation in &self.violations {
            if let Some(area) = violation.area() {
                if !areas.iter().any(|seen| seen == area) {
                    areas.push(area.to_string());
                }
            }
        }
        areas
    }

    /// Whether any defect belongs to no single area, and so governs every one of them.
    fn has_run_wide(&self) -> bool {
        self.violations
            .iter()
            .any(|violation| violation.area().is_none())
    }
}

/// Perform the bidirectional marker/register/classification audit once, and memoize it.
///
/// # Why this is a memoized function rather than the body of one test
///
/// Requirement 5's audit decides whether a marker may be trusted to *excuse* a divergence. Until it
/// has passed, a marker classifying a real divergence as `XFAIL` is an unaudited excuse — and the
/// built-in harness runs all eighteen tests concurrently in one process with no ordering between
/// them, so an area could reach that classification, publish it, and be finished long before an
/// independent infrastructure test got round to failing. Performing the audit here, once, and
/// handing it to [`preflight`] as a **blocking gate** removes the race by construction: the audit is
/// complete before any area compiles its first cell, and an area whose markers it faults is withheld
/// rather than classified.
///
/// Memoized in a `OnceLock` for the same reason [`preflight`] is: the audit parses the register,
/// re-loads every record that carries a marker, walks the curated finding set and exercises the
/// classifier over every target, level, oracle and divergence class, and two consumers need the
/// answer. It takes no [`Capabilities`]: it reads committed files only, which is what keeps it
/// meaningful on a machine with no reference compiler and no emulator at all.
///
/// # Fail-closed, and total
///
/// Every failure mode is a **violation** rather than a panic — an unreadable register included. That
/// is deliberate: this function is called from inside `preflight`'s initialization, and a panic there
/// would leave the preflight unrecorded while the reports still had to be written. A violation
/// instead blocks every area, is named in every report, and fails the infrastructure test with the
/// same words. The one exception is [`corpus_inventory`], which is already a documented fatal for the
/// whole suite because a corpus that cannot be enumerated leaves nothing to report about.
fn marker_integrity() -> &'static MarkerIntegrity {
    static INTEGRITY: OnceLock<MarkerIntegrity> = OnceLock::new();
    INTEGRITY.get_or_init(audit_markers)
}

/// The audit itself. See [`marker_integrity`] for why it is performed exactly once.
fn audit_markers() -> MarkerIntegrity {
    // The markers come from the corpus measurement rather than from a second pass of their own, and
    // that is what makes the two halves of this audit describe one corpus. The earlier enumeration
    // aborted on the first record it could not read, so on a corpus still being completed this test
    // reported an infrastructure failure and nothing else — no inventory, no reconciliation, and no
    // list of which records were outstanding. Now every record that parses contributes its marker,
    // every record that does not is a named violation below, and the run states which of the two it
    // is for every program. Fail-closed is preserved: an unreadable record still fails this test, but
    // it fails it having said what it found and what it could not.
    let markers: Vec<manifest::ExpectedDivergence> = corpus_inventory().markers().to_vec();
    let mut narrative = String::new();
    let mut violations: Vec<MarkerViolation> = Vec::new();

    let register_path = manifest_dir().join(classify::EXPECTED_DIVERGENCE_REGISTER);
    let register = match fs::read_to_string(&register_path) {
        Ok(register) => register,
        // A violation rather than a panic, because this runs inside the preflight: see the
        // "Fail-closed, and total" note on `marker_integrity`. It is run-wide, so it blocks every
        // area — an unreadable register leaves no marker in the corpus auditable.
        Err(error) => {
            violations.push(MarkerViolation::run_wide(format!(
                "the expected-divergence register {} could not be read: {error}. It is a committed \
                 deliverable, not a generated artifact: requirement 5 requires every expected \
                 divergence to be auditable in one place, and this audit is what keeps that place \
                 truthful. {} marker(s) are committed in the corpus and would go unregistered. \
                 Markers found: {}",
                shown_path(&register_path),
                markers.len(),
                marker_identifier_list(&markers),
            )));
            return MarkerIntegrity {
                violations,
                narrative,
                marker_count: markers.len(),
            };
        }
    };

    let registered = registered_identifiers(&register);
    let entries = parse_register_entries(&register);

    for marker in &markers {
        // Every defect below is attributed to the area whose record carries the marker, because a
        // marker can only ever excuse a divergence in the program it sits beside: faulting all
        // fourteen areas for one bad marker would report thirteen defects that do not exist, exactly
        // as the undefined-behaviour audit's own narrowing avoids.
        let area = marker_area(marker);
        if !registered.iter().any(|entry| entry == marker.id()) {
            violations.push(MarkerViolation::in_area(
                &area,
                format!(
                    "marker {} is committed in {} ({}) but does not appear in {}; an unregistered \
                     marker is exactly the silent exclusion requirement 5 forbids. Resolve it by \
                     registering the marker or by retiring it from the record beside that program",
                    marker.id(),
                    marker.program_label(),
                    shown_path(marker.program_path()),
                    classify::EXPECTED_DIVERGENCE_REGISTER,
                ),
            ));
        }

        // Being mentioned is not being described. An identifier can appear in a table row, a
        // heading or a sentence while the register says nothing checkable about the divergence, and
        // a register that merely mentions a marker is the stale documentation this audit exists to
        // prevent — so a structured entry, with every field agreeing, is required as well.
        let matching: Vec<&RegisterEntry> = entries
            .iter()
            .filter(|entry| entry.identifier == marker.id())
            .collect();
        match matching.as_slice() {
            [] => violations.push(MarkerViolation::in_area(
                &area,
                format!(
                    "marker {} has no structured entry in {}; an entry is a heading naming the \
                     marker followed by a table stating {}. Mentioning an identifier is not \
                     documenting a divergence: without the fields there is nothing for this audit \
                     to compare, and the register can drift away from the record it is supposed to \
                     mirror without any run noticing",
                    marker.id(),
                    classify::EXPECTED_DIVERGENCE_REGISTER,
                    comma_list(&REGISTER_ENTRY_FIELDS),
                ),
            )),
            [entry] => {
                for mismatch in register_entry_mismatches(marker, entry) {
                    violations.push(MarkerViolation::in_area(
                        &area,
                        format!("marker {} {mismatch}", marker.id()),
                    ));
                }
            }
            many => violations.push(MarkerViolation::in_area(
                &area,
                format!(
                    "marker {} has {} structured entries in {}, at lines {}; one marker is one \
                     investigation and must have one entry, or a reader cannot tell which of them \
                     is the authority and this audit cannot tell which to compare against",
                    marker.id(),
                    many.len(),
                    classify::EXPECTED_DIVERGENCE_REGISTER,
                    comma_list(
                        &many
                            .iter()
                            .map(|entry| entry.heading_line.to_string())
                            .collect::<Vec<String>>()
                    ),
                ),
            )),
        }

        for defect in basis_violations(marker) {
            violations.push(MarkerViolation::in_area(&area, defect));
        }
    }

    for identifier in &registered {
        if !markers.iter().any(|marker| marker.id() == identifier) {
            // Run-wide: the register lists a marker no record carries, so there is no area to
            // attribute it to, and the register is a deliverable of the whole run.
            violations.push(MarkerViolation::run_wide(format!(
                "{} lists {} but no expectation record in the corpus carries that marker; the \
                 register must describe divergences that are actually exercised, not ones that \
                 were retired without retiring the entry",
                classify::EXPECTED_DIVERGENCE_REGISTER,
                identifier,
            )));
        }
    }

    // The second committed register, audited by the same test and for the same reason. A finding is a
    // deliverable, so `FINDINGS.md` and the curated directories beneath `tests/conformance/findings/`
    // make the same kind of promise the marker register does — and decay the same way. Three failures
    // are possible and none of them is visible without a check: a curated directory whose evidence was
    // not refreshed after its reproducer was reduced, so the manifest provably describes a different
    // program from the one it ships; a curated directory that lost an artifact in a rebase, so a
    // reader following a register entry finds no reproduction; and a curated artifact still naming the
    // absolute location of the machine that produced it, which is disclosure rather than evidence once
    // it is committed. `findings::curated_finding_defects` is the mandatory scan the curation
    // procedure in that register names, and running it here is what makes it mandatory rather than
    // advisory.
    // Fails closed: an unreadable or partially readable curated set is reported as a defect rather
    // than read as an empty one, so this audit can never announce that it validated a set it could not
    // enumerate. An absent directory is the one tolerated answer and yields an empty set honestly.
    // Run-wide throughout: the curated finding set and its register are deliverables of the whole
    // run rather than of any one area, so a defect in either governs every area.
    let curated = match curated_finding_directories() {
        Ok(curated) => curated,
        Err(defects) => {
            violations.extend(defects.into_iter().map(MarkerViolation::run_wide));
            Vec::new()
        }
    };
    for directory in &curated {
        for defect in findings::curated_finding_defects(directory) {
            violations.push(MarkerViolation::run_wide(format!(
                "the curated finding {} is not committable as it stands: {defect}",
                shown_path(directory),
            )));
        }
    }
    // And the register's own table, read and held against those directories in both directions. A
    // validated directory nothing indexes is a deliverable nobody can find; an indexed row with no
    // directory is a pointer into nothing. Until this existed, `FINDINGS.md` was never parsed at all —
    // the curated *directories* were checked and the file that indexes them was taken on trust, which
    // is precisely the asymmetry the marker register's bidirectional check exists to avoid.
    let register_rows = match findings_register_rows() {
        Ok(rows) => rows,
        Err(defect) => {
            violations.push(MarkerViolation::run_wide(defect));
            Vec::new()
        }
    };
    violations.extend(
        findings_register_violations(&register_rows, &curated)
            .into_iter()
            .map(MarkerViolation::run_wide),
    );
    narrative.push_str(&format!(
        "curated findings — {} directory(ies) under {}, {} row(s) in it, each validated for \
         completeness, identity, post-reduction consistency, disclosure and register agreement\n",
        curated.len(),
        classify::FINDINGS_REGISTER,
        register_rows.len(),
    ));
    for row in &register_rows {
        narrative.push_str(&format!(
            "  {} [{}] status {} — {}\n",
            row.id, row.class, row.status, row.directory
        ));
    }

    // The inventory, and then how much of the corpus it was drawn from. Both halves are needed and
    // the second used to be missing: "1 marker(s) in the corpus" is a count of what was found, and a
    // reader takes it for a description of the whole corpus unless the line says which records were
    // actually read. On a corpus still being completed those are different numbers, and the
    // difference is exactly one program's markers.
    let inventory = corpus_inventory();
    violations.extend(
        inventory
            .marker_defects()
            .iter()
            .cloned()
            .map(MarkerViolation::run_wide),
    );
    narrative.push_str(&format!(
        "expected-divergence register — {} marker(s) in the corpus, {} identifier(s) in {} — {}\n",
        markers.len(),
        registered.len(),
        classify::EXPECTED_DIVERGENCE_REGISTER,
        inventory.coverage_sentence(),
    ));
    // Named, not merely counted, and pushed into the violation list rather than printed and forgotten:
    // a record the audit could not read is a program whose marker — if it has one — is unregistered
    // as far as this test can tell, which is the silent exclusion requirement 5 forbids. Requirement 5
    // is enforced in both directions or not at all, so an audit that cannot see every record must say
    // so and fail, never pass on the strength of the records it happened to read.
    for entry in inventory.pending() {
        violations.push(MarkerViolation::run_wide(format!(
            "the expectation record for {entry}. Until it parses, this audit cannot tell whether \
             that program carries an expected-divergence marker, so requirement 5's bidirectional \
             check does not hold over the whole corpus — a marker in an unreadable record would be \
             neither registered nor noticed"
        )));
    }
    if !inventory.is_complete() {
        narrative.push_str(&format!(
            "corpus completeness — planned {}, records readable {}, pending {}. The marker \
             inventory above describes the readable records only.\n",
            inventory.planned(),
            inventory.records(),
            inventory.pending().len(),
        ));
    }
    for marker in &markers {
        narrative.push_str(&format!(
            "  {} [{}] {} — scope: {} — basis: {}\n",
            marker.id(),
            marker.class().label(),
            marker.program_label(),
            marker.scope().raw(),
            marker.basis(),
        ));
    }

    // And what each marker actually *does* to a verdict, which the two checks above cannot see. A
    // marker consistent with the register can still be applied wrongly: the register audit compares
    // documents, while this compares the two predicates that decide whether a divergence is excused
    // — the classifier's authority and the finding writer's precondition — over every class, so a
    // marker can never absorb a divergence of a class it does not document.
    let (classification_defects, classification_tally) = marker_classification_audit(&markers);
    violations.extend(classification_defects);
    narrative.push_str(&classification_tally);

    MarkerIntegrity {
        violations,
        narrative,
        marker_count: markers.len(),
    }
}

/// The feature area a marker belongs to, derived from the program it sits beside.
///
/// [`manifest::ExpectedDivergence`] carries the program's path rather than its area, and the driver
/// already derives the pair the same way for every outcome it files, so this reuses that one rule
/// instead of introducing a second.
fn marker_area(marker: &manifest::ExpectedDivergence) -> String {
    let (area, _) = corpus_identity(marker.program_path());
    area
}

/// The pre-flight check: which oracles were discovered, and which arms will therefore run.
///
/// Intended to be run first — `cargo test --test conformance infra_oracle_capability_report --
/// --nocapture` — so that a misconfigured environment is diagnosed in one second rather than after
/// the whole matrix has executed against a silently reduced oracle set. A missing oracle is
/// reported, never converted into a pass; under the strict setting intended for continuous
/// integration it is a failure, because there the toolchain is installed deliberately and a gap
/// means a broken workflow rather than a modest machine.
#[test]
fn infra_oracle_capability_report() {
    let caps = oracle_capabilities();
    begin_report_session();
    let config = caps.config();

    println!("{}", caps.render_report());
    println!("{}", matrix_statement(config));
    // What the corpus actually holds, printed immediately beneath what the matrix declares, because
    // the two are different claims and the pre-flight is where conflating them does the most damage:
    // a maintainer reads this block to learn what the run is about to do, and a declared program
    // count read as a description of the tree is how a summary comes to describe a matrix a sixth
    // larger than the one that existed. `corpus_inventory` also re-derives the record-volume figures
    // the parser's bounds are documented against, so a figure that has gone stale is visible here
    // rather than discovered by a reviewer.
    println!("{}", corpus_inventory().render());
    // The capability report redacts itself; the fingerprint does not, because its text is also an
    // input to the run identity digest and redacting it there would tie a provenance value to which
    // credential-bearing variables happened to be set. It is redacted at every sink instead, here and
    // in the two report artifacts that carry it.
    println!("{}", redact_secrets(&caps.render_fingerprint()));

    // A volume figure the tree has outgrown fails the pre-flight rather than being noted in passing.
    // The figures are documentation of what the corpus holds, not enforcement — the enforced bounds
    // are separate constants and are untouched by this — so the correct response is a one-line edit
    // to the declared figure, which the diagnostic names. Reporting it without failing would put the
    // suite back where it was: a measurement maintained in a place that cannot observe the tree.
    let stale = corpus_inventory().volume_violations();
    assert!(
        stale.is_empty(),
        "the record-volume figures declared in conformance_harness/manifest.rs no longer describe \
         the corpus — {} figure(s) have been outgrown:\n{}\n\nEach is documentation of the observed \
         maximum, not a limit the parser enforces, so raise the named constant to the measured \
         value. The pre-flight above prints the measurement it compared against.",
        stale.len(),
        stale
            .iter()
            .map(|violation| format!("  - {violation}"))
            .collect::<Vec<String>>()
            .join("\n"),
    );

    let unavailable = caps.unavailable_oracle_arms();
    if unavailable.is_empty() {
        println!(
            "all three oracle arms are available: (a) reference compiler, (b) cross backend, \
             (c) golden record."
        );
    } else {
        println!(
            "{} oracle arm(s) are UNAVAILABLE in this environment:",
            unavailable.len()
        );
        for arm in &unavailable {
            println!("  - {arm}");
        }
        println!(
            "Cells depending on them are recorded UNAVAILABLE and reported in the summary. They \
             are never recorded as passes, and there is no skip verdict."
        );
    }

    let executable: Vec<Target> = Target::ALL
        .iter()
        .copied()
        .filter(|target| caps.can_execute(*target))
        .collect();

    assert!(
        !executable.is_empty() || config.missing_oracles_acknowledged(),
        "not one of the {} supported targets can be executed in this environment, so every oracle \
         would be UNAVAILABLE and the suite could decide nothing at all. This is a broken \
         environment rather than a modest one: install the emulators, or acknowledge the reduced \
         oracle set explicitly with {}=1 if a decision-free run is genuinely intended.\n\n{}",
        TARGET_COUNT,
        discovery::VAR_ALLOW_MISSING_ORACLES,
        caps.render_report(),
    );

    assert!(
        unavailable.is_empty() || !config.unavailable_fails_run(),
        "the strict setting is in force and {} oracle arm(s) are unavailable:\n{}\n\nUnder {} an \
         unavailable oracle is a failure, because in continuous integration the toolchain is \
         installed deliberately and a missing arm means a broken workflow rather than a modest \
         machine. Install the missing tooling, or clear the strict setting for a local run.\n\n{}",
        unavailable.len(),
        unavailable
            .iter()
            .map(|arm| format!("  - {arm}"))
            .collect::<Vec<String>>()
            .join("\n"),
        discovery::VAR_STRICT,
        caps.render_report(),
    );

    assert_shared_helper_integration();
    assert_reproduction_script_is_protected();
}

/// Report that a finding's reproduction script cannot be subverted by an inherited shell function,
/// and fail if that is no longer true.
///
/// `commands.sh` is the artifact a maintainer is explicitly invited to **execute**, and its own
/// preamble tells them that an exported shell function named after one of its helpers cannot stand in
/// for the real utility. The harness enforces that whenever it assembles such a script — but a
/// healthy run produces no finding, so on exactly the runs where the suite is green the check would
/// never fire and a regression in the renderers could survive indefinitely. Asserting it here puts it
/// on every run.
///
/// Folded into the pre-flight test rather than given a `#[test]` of its own for the same reason as
/// [`assert_shared_helper_integration`]: the suite's test count is one of the mechanical guarantees
/// that no pre-existing test was skipped or weakened, and a nineteenth test would falsify the count
/// that makes the guarantee checkable.
fn assert_reproduction_script_is_protected() {
    match findings::verify_reproduction_scaffolding("pre-flight: reproduction script protection") {
        Ok(lines) => println!(
            "reproduction script protection: HELD — every helper the {lines} scaffolding line(s) of \
             a finding's `commands.sh` invoke is reached through `command` and covered by the \
             script's own `unset -f`, so an inherited exported shell function cannot stand in for \
             one. Re-checked over the whole script whenever a finding is written.",
        ),
        Err(error) => panic!(
            "a finding's reproduction script would no longer hold the protection its own preamble \
             states, so `commands.sh` could execute an inherited shell function in place of a \
             helper: {error}"
        ),
    }
}

/// Path of the repository's shared integration-test helper module, relative to the package root.
const SHARED_HELPER_RELATIVE: [&str; 2] = ["tests", "common"];

/// Report how this suite integrates with the repository's shared helper module, and fail if the
/// stated integration is no longer true.
///
/// The module doc explains that `mod common;` is declared when `tests/common/mod.rs` exists and
/// omitted when it does not, because the declaration is a compile-time assertion rather than a
/// fallback. A comment saying so is worth very little on its own: the day the shared helper lands on
/// this branch, that paragraph becomes false and nothing about the build would change to say so —
/// the suite would keep passing while quietly duplicating helpers it was required to reuse.
///
/// This closes that gap. It reads the tree rather than trusting the comment, prints which case
/// holds so every pre-flight run puts it on the record, and fails when the file is present while
/// this driver does not declare it.
///
/// The check is folded into the pre-flight test rather than given a `#[test]` of its own on purpose.
/// The suite's test count is one of the mechanical guarantees that no pre-existing test was skipped
/// or weakened, so adding a nineteenth test to verify a comment would falsify the very count that
/// makes the guarantee checkable.
fn assert_shared_helper_integration() {
    let root = SHARED_HELPER_RELATIVE
        .iter()
        .fold(manifest_dir(), |path, part| path.join(part));
    let module = root.join("mod.rs");
    if !module.is_file() {
        println!(
            "shared helper module: absent at {} — `mod common;` is correctly not declared, and \
             `conformance_harness` stands alone in pure std, which is what makes the suite complete \
             on a branch that carries it ahead of the shared helper.",
            module.display()
        );
        return;
    }

    // Read this driver's own source rather than inspect the compiled module tree: whether the
    // declaration is present is a property of the text, and there is no run-time reflection that
    // could answer it. Comment lines are discarded first so that a `mod common;` mentioned in prose
    // — as it is in this file's own documentation — cannot satisfy the check.
    let driver = manifest_dir().join("tests").join("conformance.rs");
    let source = fs::read_to_string(&driver).unwrap_or_else(|error| {
        panic!(
            "the driver source {} could not be read, so this suite cannot confirm whether it \
             integrates with the repository's shared test helper: {error}",
            driver.display()
        )
    });
    let declares_common = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .any(|line| line == "mod common;" || line == "pub mod common;");

    assert!(
        declares_common,
        "the repository's shared integration-test helper module exists at {} but this suite does \
         not declare it.\n\nEvery one of the repository's existing integration suites reaches that \
         module with `mod common;` and reuses its compile helper, run helper, assertion macros and \
         temporary-directory management, and this suite is required to integrate with that \
         convention rather than duplicate it. While the file was absent, omitting the declaration \
         was correct — `mod common;` is a compile-time assertion that the file exists, so declaring \
         it against nothing fails the build. That condition no longer holds.\n\nTo resolve: add \
         `mod common;` beside `mod conformance_harness;` in {}, then replace the harness's own \
         compile, run, assertion and temporary-directory helpers with the shared ones at every call \
         site, and update this file's integration section to describe what is delegated. This \
         assertion exists so that the change is demanded the moment it becomes possible, rather \
         than left to whoever next reads a comment.",
        module.display(),
        driver.display(),
    );

    println!(
        "shared helper module: present at {} and declared by this driver — its compile, run, \
         assertion and temporary-directory helpers are the ones in use.",
        module.display()
    );
}

// =================================================================================================
// Shared orchestration
//
// Private plumbing. It sequences the harness and formats verdicts; it spawns no process, reads no
// corpus file and compares no bytes itself, so every decision this suite makes has exactly one
// implementation and lives in the module that owns it.
// =================================================================================================

/// One code generator's observed behaviour for a cell: how it built, and what its artifact did.
///
/// Owned rather than borrowed because the x86-64 baseline outlives its own workspace: oracle (b)
/// compares the other three targets against it long after that cell has been retired.
struct Authority {
    build: CompileOutcome,
    run: RunOutcome,
}

/// The x86-64 authority for one `(program, optimization level)` pair, or why there is none.
///
/// Distinguishing the four states matters because they demand different verdicts. A baseline that
/// was never executable is an environment gap; a baseline the compiler refused is a compiler
/// observation that must go through the same expected-divergence marker logic every other
/// compiler-attributed divergence goes through, so that a documented limitation covering the whole
/// program does not become a cascade of unexplained failures on the other three targets; and a
/// baseline this suite simply failed to observe is a failure of the suite, which the three dependent
/// targets must report as such rather than inheriting an absence they would not fail for.
enum Baseline {
    /// The baseline cell ran, and oracle (b) can compare against it.
    ///
    /// Boxed because an observation is two captured process results and the other variants are a
    /// line of text apiece, so holding one inline would make every baseline slot the size of the
    /// largest thing it could ever hold.
    Observed(Box<Authority>),
    /// The compiler under test produced no baseline artifact.
    ///
    /// The whole build is retained rather than a class and a summary line, because oracle (b) on
    /// the other three targets is settled against this evidence: when that refusal is not
    /// documented, each of those arms becomes a finding whose artifact directory has to carry the
    /// baseline's own argument vector, diagnostics and exit status. A finding that says "the
    /// baseline was refused" and cannot show the refusal is not a deliverable anybody can
    /// reproduce. Boxed for the same reason [`Baseline::Observed`] is.
    Refused(Box<CompileOutcome>),
    /// A baseline artifact exists but this machine cannot execute it.
    Unrunnable(String),
    /// The baseline cell could not be observed at all, because this process failed.
    ///
    /// Distinct from every other variant because it is the only one that is nobody's answer: the
    /// compiler under test said nothing, the machine said nothing, and this suite lost the result.
    /// Recorded so the three dependent targets report it as a failure rather than inheriting an
    /// absence they would not fail for.
    Internal(String),
}

/// One side of a comparison, in the form a finding's evidence needs it.
///
/// Both observations are optional because the difference a finding records is often an **absence**:
/// a program the compiler refuses has a build and no run, and that asymmetry is the divergence
/// itself. Carrying the two independently — rather than only the [`Authority`] pair, which requires
/// both — is what lets a refusal be delivered as a complete artifact directory instead of as a
/// verdict with nothing behind it.
///
/// Borrowed rather than owned: every construction site already holds the evidence, and the capture
/// the writer needs is cloned from it once, at the moment a finding is actually being assembled, so
/// a passing cell clones nothing.
struct Side<'a> {
    role: CaptureRole,
    target: Target,
    build: Option<&'a CompileOutcome>,
    run: Option<&'a RunOutcome>,
}

impl<'a> Side<'a> {
    /// A side that built an artifact and executed it.
    fn ran(role: CaptureRole, target: Target, authority: &'a Authority) -> Side<'a> {
        Side {
            role,
            target,
            build: Some(&authority.build),
            run: Some(&authority.run),
        }
    }

    /// A side whose build produced no artifact, so there was nothing to execute.
    fn refused(role: CaptureRole, target: Target, build: &'a CompileOutcome) -> Side<'a> {
        Side {
            role,
            target,
            build: Some(build),
            run: None,
        }
    }

    /// Turn this side into the finding capture it describes, at the cell's optimization level.
    ///
    /// The level is supplied rather than stored because every capture in one cell's finding is at
    /// the same level: oracle (b) compares a target against the baseline *at the same level*, so a
    /// side carrying its own would be a second place for the same fact to be wrong.
    fn capture(&self, opt: OptLevel) -> Result<Capture, HarnessError> {
        Capture::observed(
            self.role,
            self.target,
            opt,
            self.build.cloned(),
            self.run.cloned(),
        )
    }
}

/// The reference-compiler arm of one cell, which has six materially different endings.
///
/// Six rather than five because "this arm was not observed" is two different facts with opposite
/// verdicts: a runner or driver this machine does not have is a reported gap the run does not fail
/// for, while this suite's own machinery failing is a failure. Collapsing them would make the
/// second look like the first.
enum ReferenceArm {
    /// The program's own record disables oracle (a), with a reason recorded there.
    ExcludedByRecord,
    /// No reference driver exists for this target in this environment.
    DriverAbsent,
    /// A reference artifact exists but this machine cannot execute it.
    Unrunnable(String),
    /// The reference compiler produced no artifact.
    Refused {
        /// The shape of the refusal.
        class: DivergenceClass,
        /// One line naming what happened.
        summary: String,
        /// Who the build layer found answerable, carried verbatim rather than as a predicate.
        ///
        /// Was a boolean, and could not stay one: the build layer reports three scopes, and the
        /// third — an attribution withdrawn because the diagnostics it rested on were not captured
        /// whole — collapses into "not the environment" under any boolean encoding, which is
        /// exactly the reading that turns an unattributable refusal into an accusation.
        attribution: Attribution,
        /// The build layer's own structured account of the invocation that refused.
        ///
        /// Carried here rather than re-derived because the [`CompileOutcome`] does not outlive the
        /// match that produced this variant, and the command, the termination and the capture
        /// reference are exactly what a maintainer needs in order to reproduce the refusal. Reducing
        /// this arm to `class` and `summary` alone is what previously made a reference-arm refusal
        /// unactionable in the report.
        ///
        /// Boxed because this variant would otherwise be several times the size of every other, and
        /// every `ReferenceArm` value in the run would carry that size whether it held a refusal or not.
        provenance: Box<Provenance>,
    },
    /// An observation to compare bcc against. Boxed for the same reason as [`Baseline::Observed`].
    Ran(Box<Authority>),
    /// This arm could not be observed at all, because this process failed.
    ///
    /// Kept apart from [`ReferenceArm::Unrunnable`] for the reason that governs the whole verdict
    /// space: an absent runner is a reported gap the run does not fail for, while a fault in this
    /// suite's own machinery is a failure. Filing the second as the first would let a harness that
    /// cannot run its own reference arm report a clean run over an oracle it never asked.
    Internal(String),
}

/// How observing one compiler's cell ended.
enum Observed {
    /// The artifact was built and executed.
    Ran(Box<Authority>),
    /// The compiler ran and produced no usable artifact, so there was nothing to execute.
    Refused(Box<CompileOutcome>),
    /// The artifact exists but this machine has no runner for it.
    Unrunnable(String),
}

/// Everything one cell needs, assembled once.
///
/// Exists so that each oracle helper takes two or three arguments instead of seven. Threading the
/// capability record, the expectation record, the identity, the workspace and both command
/// substitutions through every one of them separately would be the same information with more
/// places to get it wrong.
struct CellPlan<'a> {
    caps: &'a Capabilities,
    record: &'a Manifest,
    key: &'a CellKey,
    /// The resolved cell, kept rather than dropped after the workspace is allocated.
    ///
    /// Its two path fields are the *validated* spellings — proved to be regular files, not links,
    /// resolving strictly beneath the corpus root — so every command line and every report row
    /// this plan renders names the same file the constructor checked. Reaching back to the
    /// expectation record's own path instead would render an unvalidated spelling.
    ///
    /// It is also why there is no second copy of the record's path beside it: the reproduction
    /// header this plan writes must name the record the constructor validated, and one path held
    /// in two places is one path that can be updated in one of them.
    cell: Cell,
    oracles: Vec<Oracle>,
    workspace: Workspace,
    bcc: CommandSubstitutions,
    reference: Option<CommandSubstitutions>,
}

impl<'a> CellPlan<'a> {
    /// Resolve the cell, allocate its workspace, and render its exact command lines into it.
    ///
    /// The commands are written for every cell rather than only for a failing one, because a
    /// retained workspace has to be reproducible by hand and a cell that has already been cleaned
    /// up cannot be asked what it ran.
    fn assemble(
        caps: &'a Capabilities,
        record: &'a Manifest,
        key: &'a CellKey,
        execution: &manifest::Execution,
    ) -> Result<CellPlan<'a>, HarnessError> {
        let cell = Cell::new(
            key.clone(),
            record.source_path(),
            record.path().to_path_buf(),
        )?;
        let workspace = sandbox::for_resolved_cell(&cell, caps.config())?;

        // Allocation copied the program and its record into the workspace, and it is the copy that
        // every command line below names. Two things follow. A retained workspace is reproducible on
        // its own, because the recipe it records rebuilds the bytes it holds rather than whatever the
        // corpus contains whenever someone next reads the directory. And the copy is taken once, so
        // all three oracles and every stream in this cell are derived from one set of bytes even if
        // the corpus were edited mid-run.
        let source = workspace.path(sandbox::PROGRAM_SOURCE_NAME)?;

        let compiler = caps.bcc().path().ok_or_else(|| {
            HarnessError::new(
                format!("assembling the cell {key}"),
                "the capability record carries no path for the compiler under test, although \
                 discovery only returns a record once it has resolved one; nothing can be built \
                 without it and building with a guessed path would test something other than the \
                 compiler this run is about",
            )
        })?;

        let bcc_artifact = workspace.path(sandbox::BCC_ARTIFACT_NAME)?;
        let bcc = CommandSubstitutions::new(
            compiler,
            key.target(),
            key.opt(),
            &source,
            &bcc_artifact,
            execution,
        )?;

        // The reference arm is prepared only when this program enables oracle (a) and a driver for
        // this target exists, so an absent cross driver costs the run one reported arm rather than
        // the whole cell.
        let reference = match caps.ref_cc_for(key.target()) {
            Some(driver) if record.oracle_enabled(Oracle::ReferenceCompiler) => {
                let artifact = workspace.path(sandbox::REFERENCE_ARTIFACT_NAME)?;
                Some(
                    CommandSubstitutions::new(
                        compiler,
                        key.target(),
                        key.opt(),
                        &source,
                        &artifact,
                        execution,
                    )?
                    .with_reference_compiler(driver)?,
                )
            }
            _ => None,
        };

        let plan = CellPlan {
            caps,
            record,
            key,
            cell,
            oracles: applicable_oracles(key.target()),
            workspace,
            bcc,
            reference,
        };
        plan.record_commands()?;
        Ok(plan)
    }

    /// Write the cell's exact compile and run lines into its workspace.
    fn record_commands(&self) -> Result<(), HarnessError> {
        let mut text = format!(
            "# {}\n# expectation record: {}\n# expected exit status: {}\n\
             # stdout bytes and exit status are compared; standard error never is, because\n\
             # diagnostic wording legitimately differs between compilers.\n\n\
             # compiler under test\n",
            self.key,
            shown_path(self.cell.expectation()),
            self.record.expect_exit(),
        );
        text.push_str(&self.record.render_bcc_command(&self.bcc)?);
        text.push('\n');
        text.push_str(&self.record.render_run_command(&self.bcc)?);
        text.push('\n');
        match &self.reference {
            Some(reference) => {
                text.push_str("\n# reference compiler\n");
                text.push_str(&self.record.render_ref_command(reference)?);
                text.push('\n');
                text.push_str(&self.record.render_run_command(reference)?);
                text.push('\n');
            }
            None => text.push_str(
                "\n# reference compiler: not invoked for this cell — either this program's record \
                 disables oracle (a),\n# or no reference driver for this target exists in this \
                 environment.\n",
            ),
        }
        self.workspace
            .write_text(sandbox::COMMANDS_NAME, &text)
            .map(|_| ())
    }

    /// Build, execute, judge, and then retire the workspace.
    fn decide(self, baseline: &mut Option<Baseline>) -> Vec<Outcome> {
        let outcomes = self.judge(baseline);
        self.retire(&outcomes);
        outcomes
    }

    /// Ask every oracle that applies to this cell.
    fn judge(&self, baseline: &mut Option<Baseline>) -> Vec<Outcome> {
        let subject = match observe(
            Compiler::Bcc,
            self.record,
            &self.bcc,
            self.key,
            &self.workspace,
            self.caps,
        ) {
            Ok(Observed::Ran(authority)) => authority,
            // A build failure is judged as a compiler-attributed divergence rather than as
            // agreement with a reference compiler that also refuses the program: the
            // undefined-behaviour audit gate establishes that the reference compiler accepts every
            // corpus program, so a refusal here is bcc's alone — and routing it through the marker
            // logic is what lets a documented unimplemented extension be an expected divergence.
            Ok(Observed::Refused(build)) => {
                // The reference arm is observed even though there is nothing to compare it against
                // numerically, because it is what makes the refusal evidence. A program the
                // reference compiler also rejects is a defect in the test material; a program it
                // accepts and bcc rejects is the divergence this suite exists to find. Recording
                // both sides is the difference between a finding a maintainer can act on and a
                // verdict that only asserts something went wrong.
                let reference = self.observe_reference();
                // The baseline observation is passed in, because oracle (b)'s precondition is the
                // baseline's own state and the refusal path has to resolve it exactly as the path
                // where bcc built does. It is read before the assignment below, which is correct
                // ordering rather than an accident: on the baseline target itself there is no
                // separate baseline to compare against, and `refusal_under_oracle_b` handles that
                // case by target rather than by whatever this cell is about to store.
                let outcomes = self.subject_refused(&build, &reference, baseline.as_ref());
                if self.key.target() == Target::BASELINE {
                    // A build whose result could not be established is this suite's fault, so
                    // the three dependent targets must inherit a failure rather than a refusal
                    // they would attribute to the compiler.
                    *baseline = Some(if build.is_harness_failure() {
                        Baseline::Internal(refusal_shape(&build).1)
                    } else {
                        Baseline::Refused(build.clone())
                    });
                }
                return outcomes;
            }
            Ok(Observed::Unrunnable(diagnosis)) => {
                if self.key.target() == Target::BASELINE {
                    *baseline = Some(Baseline::Unrunnable(diagnosis.clone()));
                }
                return self.tooling_absent(&diagnosis);
            }
            Err(error) => {
                if self.key.target() == Target::BASELINE {
                    *baseline = Some(Baseline::Internal(error.to_string()));
                }
                return self.internal(&error);
            }
        };

        let reference = self.observe_reference();

        let mut outcomes = Vec::with_capacity(self.oracles.len());
        outcomes.push(self.oracle_a(&subject, &reference));
        if self.oracles.contains(&Oracle::CrossBackend) {
            // The reference arm travels into oracle (b) as well, not to be compared there but to be
            // CARRIED: a cross-backend divergence over a property the ABIs fix is a different
            // observation from one the compiler under test got wrong, and the same-target reference
            // capture is what tells the two apart. See `CellPlan::corroborant`.
            outcomes.push(self.oracle_b(&subject, baseline.as_ref(), &reference));
        }
        outcomes.push(self.oracle_c(&subject));

        // Moved rather than cloned: this cell is decided, and the observation's only remaining
        // reader is oracle (b) on the other three targets.
        if self.key.target() == Target::BASELINE {
            *baseline = Some(Baseline::Observed(subject));
        }
        outcomes
    }

    /// Attempt the reference arm, resolving it to one of its six endings.
    fn observe_reference(&self) -> ReferenceArm {
        if !self.record.oracle_enabled(Oracle::ReferenceCompiler) {
            return ReferenceArm::ExcludedByRecord;
        }
        let Some(substitutions) = &self.reference else {
            return ReferenceArm::DriverAbsent;
        };
        match observe(
            Compiler::Reference,
            self.record,
            substitutions,
            self.key,
            &self.workspace,
            self.caps,
        ) {
            Ok(Observed::Ran(authority)) => ReferenceArm::Ran(authority), // already boxed
            Ok(Observed::Refused(build)) => {
                let (class, summary) = refusal_shape(&build);
                // The harness question is asked before the attribution one, because a build
                // whose result could not be established is neither the machine's answer nor the
                // compiler's, and no attribution can express that.
                if build.is_harness_failure() {
                    ReferenceArm::Internal(summary)
                } else {
                    ReferenceArm::Refused {
                        class,
                        summary,
                        attribution: build_attribution(&build),
                        provenance: Box::new(build.provenance()),
                    }
                }
            }
            Ok(Observed::Unrunnable(diagnosis)) => ReferenceArm::Unrunnable(diagnosis),
            // The reference arm's own machinery failing is this suite's defect, not a gap in the
            // machine, so it is reported as a failure. The other two oracles still decide this
            // cell: losing one arm must not cost the two that were observed.
            Err(error) => ReferenceArm::Internal(error.to_string()),
        }
    }

    /// Oracle (a): bcc against the reference compiler, same target, same optimization level.
    fn oracle_a(&self, subject: &Authority, reference: &ReferenceArm) -> Outcome {
        let oracle = Oracle::ReferenceCompiler;
        match reference {
            ReferenceArm::ExcludedByRecord => self.excluded(oracle),
            ReferenceArm::DriverAbsent => classify::unavailable_oracle(self.caps, self.key, oracle),
            ReferenceArm::Unrunnable(diagnosis) => classify::judge(
                &classify::Observation::ToolingAbsent {
                    diagnosis: diagnosis.as_str(),
                },
                Some(self.record),
                self.key,
                oracle,
            ),
            ReferenceArm::Internal(cause) => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the reference compiler", self.key),
                &format!(
                    "this arm was never observed, because this suite's own machinery failed while \
                     attempting it: {cause}. That is neither a statement about the compiler under \
                     test nor a gap in this machine, so it is reported as a failure of the harness \
                     rather than as an unavailable oracle — an absence the run would not fail for \
                     is exactly how a cell that was launched and lost comes to look like a pass"
                ),
            ),
            // The machine is answerable, or nobody could be shown to be. Both are reported through
            // the classifier's own scope handling, which renders each as an unavailable arm — this
            // oracle could not be attempted, and saying so is neither a pass nor an accusation.
            //
            // Neither scope can reach the marker logic, so neither can produce a finding, which is
            // why this build failure is the one that does not route through `deliver`: an assembly
            // path here would assert the opposite of the classifier's contract, and there is no
            // observation of the compiler under test to assemble from in any case.
            ReferenceArm::Refused {
                class,
                summary,
                attribution: attribution @ (Attribution::Environment | Attribution::Indeterminate),
                provenance,
            } => classify::build_failure(
                self.record,
                self.key,
                oracle,
                classify::BuildRefusal {
                    class: *class,
                    attribution: *attribution,
                    summary,
                    // No dependency root: this refusal is the REFERENCE compiler's, and a marker in
                    // this record documents a limitation of the compiler under test. Borrowing it
                    // here would excuse a defect in the test material with documentation about
                    // something else entirely. Both attributions reaching this arm are unavailable
                    // scopes in any case, which never reach the marker logic at all.
                    root: None,
                    // The provenance IS carried, unlike the root: it is a record of what this arm
                    // ran, not an authority borrowed from elsewhere, so it says nothing about the
                    // compiler under test and everything about the invocation a maintainer has to
                    // repeat.
                    provenance: Some(provenance.clone()),
                },
            ),
            ReferenceArm::Refused {
                summary,
                attribution: Attribution::Compiler,
                ..
            } => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the reference compiler", self.key),
                &format!(
                    "the reference compiler produced no artifact: {summary}. It is the authority \
                     for this oracle, so its refusal is a defect in the test program or in the \
                     record's reference command rather than a divergence attributable to the \
                     compiler under test — requirement 1's audit gate is where a program the \
                     reference compiler rejects must be corrected"
                ),
            ),
            ReferenceArm::Ran(authority) => {
                let comparison = compare::oracle_a(&subject.run, &authority.run, self.key);
                self.settle(
                    &comparison,
                    Side::ran(CaptureRole::UnderTest, self.key.target(), subject),
                    Some(Side::ran(
                        CaptureRole::ReferenceCompiler,
                        self.key.target(),
                        authority,
                    )),
                )
            }
        }
    }

    /// Oracle (b): this target against the x86-64 baseline at the same optimization level.
    fn oracle_b(
        &self,
        subject: &Authority,
        baseline: Option<&Baseline>,
        reference: &ReferenceArm,
    ) -> Outcome {
        let oracle = Oracle::CrossBackend;
        if !self.record.oracle_enabled(oracle) {
            return self.excluded(oracle);
        }
        if !self.caps.oracle_available(oracle, self.key.target()) {
            return classify::unavailable_oracle(self.caps, self.key, oracle);
        }
        match baseline {
            Some(Baseline::Observed(authority)) => {
                let comparison = compare::oracle_b(&subject.run, &authority.run, self.key);
                self.settle_corroborated(
                    &comparison,
                    Side::ran(CaptureRole::UnderTest, self.key.target(), subject),
                    Some(Side::ran(
                        CaptureRole::Baseline,
                        Target::BASELINE,
                        authority,
                    )),
                    self.corroborant(reference),
                )
            }
            // The baseline was refused by the same compiler under test, so this arm is judged as
            // that same compiler-attributed divergence. Anything else would turn one documented
            // limitation into three unexplained failures on the other targets.
            //
            // It is settled rather than merely classified, and the baseline's own build travels
            // with it as evidence: when the refusal is undocumented this arm is a finding, and a
            // finding is a deliverable that has to show the refusal it is about.
            Some(Baseline::Refused(build)) => self.settle_refusal(
                oracle,
                build,
                RefusedSide::Baseline,
                Side::ran(CaptureRole::UnderTest, self.key.target(), subject),
                Some(Side::refused(
                    CaptureRole::Baseline,
                    Target::BASELINE,
                    build,
                )),
                // No reference-arm phrase: this arm is oracle (b), and the refusal it reports
                // happened in the baseline cell rather than in this one, so naming how *this*
                // cell's reference arm ended would describe an observation the refusal is not
                // about.
                None,
            ),
            Some(Baseline::Unrunnable(diagnosis)) => classify::judge(
                &classify::Observation::ToolingAbsent {
                    diagnosis: diagnosis.as_str(),
                },
                Some(self.record),
                self.key,
                oracle,
            ),
            Some(Baseline::Internal(cause)) => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the cross-backend baseline", self.key),
                &format!(
                    "the {} baseline was never observed, because this suite's own machinery failed \
                     while attempting it: {cause}. The authority this arm compares against is \
                     therefore missing for a reason that is nobody's answer but this harness's, so \
                     it is reported as a failure rather than as an unavailable oracle",
                    Target::BASELINE.triple(),
                ),
            ),
            None => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the cross-backend baseline", self.key),
                &format!(
                    "this program's expectation record enables the cross-backend oracle but omits \
                     {} from its target list, so the authority this arm compares against is never \
                     observed. Either list the baseline target or record the cross-backend \
                     exclusion with its reason; comparing against a baseline that was never run \
                     would mean inventing an authority, and dropping the arm silently is the \
                     exclusion requirement 5 forbids",
                    Target::BASELINE.triple(),
                ),
            ),
        }
    }

    /// Oracle (c): this cell against the stdout committed in its own expectation record.
    fn oracle_c(&self, subject: &Authority) -> Outcome {
        let oracle = Oracle::GoldenRecord;
        if !self.record.oracle_enabled(oracle) {
            return self.excluded(oracle);
        }
        let comparison = compare::oracle_c(&subject.run, self.record, self.key);
        self.settle(
            &comparison,
            Side::ran(CaptureRole::UnderTest, self.key.target(), subject),
            None,
        )
    }

    /// Classify one comparison and deliver whatever it turns out to be.
    fn settle(
        &self,
        comparison: &Comparison,
        subject: Side<'_>,
        counterpart: Option<Side<'_>>,
    ) -> Outcome {
        self.settle_corroborated(comparison, subject, counterpart, None)
    }

    /// Classify one comparison, delivering a third capture that corroborates its attribution.
    ///
    /// The corroborant is never compared. It is carried so that a finding can state whether the
    /// divergence is a candidate defect in the compiler under test or an observation about two
    /// application binary interfaces — a distinction `findings::Finding::cross_arm_attribution`
    /// computes from the captures rather than leaving to a reader's judgement. See
    /// [`CellPlan::corroborant`] for which capture that is and why only oracle (b) needs one.
    fn settle_corroborated(
        &self,
        comparison: &Comparison,
        subject: Side<'_>,
        counterpart: Option<Side<'_>>,
        corroborant: Option<Side<'_>>,
    ) -> Outcome {
        let outcome = classify::classify(comparison, self.record, self.key, comparison.oracle);
        self.deliver(outcome, comparison, subject, counterpart, corroborant)
    }

    /// The same-target reference capture, when the reference arm ran, for an oracle (b) finding.
    ///
    /// # Why a cross-backend finding carries a third capture
    ///
    /// A cross-backend divergence is a difference between two of this compiler's own backends, and on
    /// its own it does not say whose fault it is. Two readings are possible and they lead a maintainer
    /// to different code:
    ///
    /// - the compiler under test got this target wrong, which is a candidate defect; or
    /// - the property under comparison is one each target's application binary interface fixes for
    ///   itself, in which case the two backends differing is a fact about two ABIs.
    ///
    /// The same-target reference compiler settles it: if the compiler under test agrees with the
    /// toolchain that implements this target's ABI, and only the cross-target comparison differs, the
    /// observation is about the ABIs. That is the AAP's own carve-out — a cross-backend divergence
    /// attributable to a documented implementation-defined difference, ABI included, is not a compiler
    /// defect — and it is also step 1 of the triage procedure the findings register documents.
    ///
    /// So the capture that settles it travels with the finding. The verdict is unaffected: it remains a
    /// `FINDING`, because requirement 6 makes an undocumented divergence a deliverable and nothing here
    /// excuses one. What changes is that the artifact states the attribution and carries the evidence
    /// for it, instead of asking a reader to reproduce a comparison the run had already made.
    ///
    /// `None` when the reference arm did not run — no driver, an exclusion, a refusal, a lost arm — in
    /// which case the attribution is recorded as undetermined rather than guessed.
    fn corroborant<'r>(&self, reference: &'r ReferenceArm) -> Option<Side<'r>> {
        match reference {
            ReferenceArm::Ran(authority) => Some(Side::ran(
                CaptureRole::ReferenceCompiler,
                self.key.target(),
                authority,
            )),
            _ => None,
        }
    }

    /// Classify a build that produced no artifact, for one oracle, and deliver the result.
    ///
    /// The verdict comes from the build layer's own report rather than from a comparison, because
    /// there were never two observations to compare: `classify::build_failure` puts an
    /// environment-attributed refusal at environment scope and a compiler-attributed one through
    /// the same marker logic every other compiler-attributed divergence goes through. The rendered
    /// comparison exists so that an *undocumented* refusal still has evidence to be delivered with
    /// — see [`CellPlan::deliver`]. Both roads end in the same place, and that is the point: a
    /// refusal no marker covers is a finding, and a finding is an artifact directory, so a refusal
    /// is never classified and then dropped.
    ///
    /// # Why the dependency root is derived here, and only here
    ///
    /// A refusal produces no artifact, so one root event denies every oracle arm of the cell the
    /// subject of its comparison at once. `classify::refusal_root` names the arm whose marker
    /// documents that event, and `classify::build_failure` then settles that arm on the marker
    /// directly while every other arm becomes a *dependent blocked* expected divergence citing it —
    /// one event, one explanation, no finding directory per arm.
    ///
    /// Deriving it here rather than at each of the six call sites is deliberate, and it is safe for
    /// one reason: both [`RefusedSide`] variants are refusals by **the compiler under test** — this
    /// cell's own build, or the cross-backend baseline cell's — which is exactly what a marker in
    /// this record is written about. The reference arm's own refusal never reaches this function; it
    /// goes to `classify::build_failure` with no root, because a marker documenting a limitation of
    /// the compiler under test must not excuse a defect in the test material.
    ///
    /// The root is `None` unless a marker already covers this cell's target, optimization level and
    /// the observed class on some arm, so an undocumented refusal still becomes a finding on every
    /// applicable arm.
    fn settle_refusal(
        &self,
        oracle: Oracle,
        build: &CompileOutcome,
        side: RefusedSide,
        subject: Side<'_>,
        counterpart: Option<Side<'_>>,
        reference_arm: Option<&str>,
    ) -> Outcome {
        let (class, summary) = refusal_shape(build);
        let outcome = classify::build_failure(
            self.record,
            self.key,
            oracle,
            classify::BuildRefusal {
                class,
                attribution: build_attribution(build),
                summary: &summary,
                root: classify::refusal_root(self.record, self.key, class),
                provenance: Some(Box::new(build.provenance())),
            },
        );
        let comparison =
            compare::build_refusal(oracle, self.key, class, side, &summary, reference_arm);
        // No corroborant: a refusal produced no artifact, so there is no cross-backend VALUE to
        // attribute to an ABI in the first place — the attribution question this carries evidence for
        // does not arise. The reference arm still travels with the refusal, as a capture when it ran
        // and as one phrase naming how it ended when it did not.
        self.deliver(outcome, &comparison, subject, counterpart, None)
    }

    /// The single path from a verdict to the artifacts that verdict obliges the run to produce.
    ///
    /// # Why every finding goes through exactly one place
    ///
    /// A finding does not fail the run, because a finding is a **deliverable**: an undocumented
    /// divergence handed over as a reproducer, its record, the exact reproduction commands, the
    /// captured outputs, an environment fingerprint and the computed difference. That asymmetry is
    /// only sound while the deliverable actually exists. A `FINDING` verdict recorded with nothing
    /// behind it would be the one shape in this suite that looks like a result, cannot be acted on,
    /// and still lets the run report success.
    ///
    /// So this function is the *only* route from a comparison or a refusal to an outcome in this
    /// driver, and it is total over both: a wrong answer and a compiler's refusal to build are
    /// delivered identically. When the artifacts cannot be written the verdict becomes
    /// [`Verdict::Fail`] — either from `findings::record`, which reports the write error with the
    /// path and cause, or from the assembly step, whose own refusals name the precondition that was
    /// violated. Both fail the run, which is the correct answer: a divergence nobody can reproduce
    /// is an unexplained result, not a delivered one.
    fn deliver(
        &self,
        outcome: Outcome,
        comparison: &Comparison,
        subject: Side<'_>,
        counterpart: Option<Side<'_>>,
        corroborant: Option<Side<'_>>,
    ) -> Outcome {
        if outcome.verdict() != Verdict::Finding {
            return outcome;
        }
        match self.assemble_finding(comparison, subject, counterpart, corroborant) {
            Ok(finding) => findings::record(&finding, self.caps),
            Err(error) => classify::internal_error(self.key, outcome.oracle(), &error),
        }
    }

    /// Build the finding: the reproducer, both sides of the comparison, and the golden record.
    ///
    /// A side that built nothing contributes a capture holding its build alone, which is exactly
    /// what the artifact model asks for: the compiler's argument vector, its diagnostics and its
    /// exit status are the evidence of a refusal, and the absence of a run is the difference the
    /// finding records.
    fn assemble_finding(
        &self,
        comparison: &Comparison,
        subject: Side<'_>,
        counterpart: Option<Side<'_>>,
        corroborant: Option<Side<'_>>,
    ) -> Result<Finding, HarnessError> {
        let opt = self.key.opt();
        let mut finding = Finding::new(self.key.clone(), comparison, self.record)?
            .with_capture(subject.capture(opt)?);
        if let Some(counterpart) = counterpart {
            finding = finding.with_capture(counterpart.capture(opt)?);
        }
        // Carried, never compared — see [`CellPlan::corroborant`]. It cannot be mistaken for the
        // authority of this comparison, because `Finding::authority` selects by the observing oracle's
        // own role and this capture belongs to another oracle's.
        if let Some(corroborant) = corroborant {
            finding = finding.with_capture(corroborant.capture(opt)?);
        }
        if comparison.oracle == Oracle::GoldenRecord {
            finding = finding.with_capture(Capture::golden(
                self.key.target(),
                self.key.opt(),
                self.record,
            ));
        }
        Ok(finding)
    }

    /// An arm this program's own record narrows away, judged so the exclusion is reported.
    fn excluded(&self, oracle: Oracle) -> Outcome {
        let comparison = compare::excluded_by_manifest(oracle, self.record, self.key);
        classify::classify(&comparison, self.record, self.key, oracle)
    }

    /// The compiler under test produced no artifact: every applicable oracle says so.
    ///
    /// One refusal, one class, one cell — and every oracle blocked by the same absence, so each is
    /// settled against the same evidence. Each arm is then judged separately: `classify.rs` records
    /// it as an expected divergence only when a marker's scope names **that** oracle as well as the
    /// target, the level and the class, so a marker scoped to one oracle excuses that arm alone and
    /// the arms it does not name remain findings. Whichever way an arm lands, it is delivered with
    /// the refusal's own artifacts rather than as a bare verdict.
    ///
    /// # What the reference arm contributes, and what it does not
    ///
    /// The evidence a refusal needs is the refusal: the argument vector bcc was given, the
    /// diagnostics it wrote, and the status it exited with. The reference arm is observed anyway and
    /// travels with the refusal — as a capture when it ran, and as one phrase naming how it ended
    /// when it did not — because those two endings point a reader at opposite conclusions. A program
    /// the reference compiler also rejects indicts the test material or the record's reference
    /// command; a program it accepts and bcc rejects is the divergence this suite exists to find.
    ///
    /// What the refusal does **not** do is depend on that arm's *content*. That the reference
    /// compiler accepts the program is not re-established per cell, because requirement 1's audit
    /// gate establishes it for the whole corpus before any cell runs — which is precisely why a
    /// refusal here is attributed to bcc alone.
    ///
    /// # Why each oracle's own precondition is still resolved first
    ///
    /// "Every oracle blocked by the same absence" is true only of oracles that were *asked*. An
    /// oracle whose own precondition fails was never asked, and settling it against the refusal
    /// would state an outcome for a comparison that could not have happened:
    ///
    /// - An oracle the program's **own record disables** must report as excluded. Recording it as an
    ///   expected divergence or a finding would contradict the record and, worse, would put an arm
    ///   the corpus deliberately narrowed away into the register or the finding set — the opposite of
    ///   the reasoned, recorded exclusion requirement 5 asks for.
    /// - Oracle (a) with **no reference driver at all** must report as unavailable. There is no
    ///   authority on this machine, so "bcc disagrees with the reference compiler" is a claim about a
    ///   comparison nobody made. `UNAVAILABLE` is loud and never a pass, which is the honest answer.
    /// - Oracle (b) with **no runner for this target, or no baseline observation**, is the same
    ///   situation one oracle over.
    /// - An arm this suite's **own machinery lost** must fail the run, not become a divergence.
    ///
    /// This is exactly the precondition ladder [`CellPlan::oracle_a`], [`CellPlan::oracle_b`] and
    /// [`CellPlan::oracle_c`] apply on the path where bcc *did* build, and it has to be the same
    /// ladder: an oracle's availability is a property of the machine and the record, not of whether
    /// the compiler under test happened to accept the program.
    ///
    /// Oracle (c) has no precondition beyond the record, because its authority is the `expected_stdout`
    /// committed in the record itself — which is present by construction and needs no tool. So a
    /// refusal is **always** settled through at least one oracle, and therefore always delivered as a
    /// complete artifact directory rather than dropped: a machine with no reference driver and no
    /// emulator still reports the refusal, with the absent arms named rather than left blank.
    fn subject_refused(
        &self,
        build: &CompileOutcome,
        reference: &ReferenceArm,
        baseline: Option<&Baseline>,
    ) -> Vec<Outcome> {
        // Asked before anything else, because a refusal whose result this process could not
        // establish is the harness's own defect rather than an observation about any compiler: it
        // has to fail the run, where a refusal attributable to the compiler or to this machine is
        // settled through the marker logic below. Reporting it as an unavailable oracle instead is
        // exactly how a cell that was launched and lost comes to look like one that passed.
        if build.is_harness_failure() {
            let (_, summary) = refusal_shape(build);
            return self
                .oracles
                .iter()
                .map(|oracle| {
                    classify::unexplained(
                        self.key,
                        *oracle,
                        &format!("building {} with the compiler under test", self.key),
                        &format!(
                            "the build was launched and its result could not be established: \
                             {summary}. Nothing was observed about the compiler and nothing about \
                             this machine, so this is reported as a failure of the harness rather \
                             than as an unavailable oracle"
                        ),
                    )
                })
                .collect();
        }
        // Rendered once: every oracle that *is* settled against this one refusal is settled against
        // the same evidence, and the phrase is part of that evidence.
        let reference_arm = describe_reference_arm(reference);
        self.oracles
            .iter()
            .map(|oracle| match oracle {
                Oracle::ReferenceCompiler => {
                    self.refusal_under_oracle_a(build, reference, &reference_arm)
                }
                Oracle::CrossBackend => {
                    self.refusal_under_oracle_b(build, baseline, &reference_arm)
                }
                Oracle::GoldenRecord => {
                    self.refusal_under_oracle_c(build, reference, &reference_arm)
                }
            })
            .collect()
    }

    /// The refusal as oracle (a) sees it, after that oracle's own precondition is resolved.
    ///
    /// The ladder is [`CellPlan::oracle_a`]'s, applied to the same six reference-arm endings, and only
    /// the last of them reaches the marker logic. The others each name a reason this oracle was never
    /// asked, which is a different statement from "bcc diverged from the reference compiler" and must
    /// not be rendered as one.
    fn refusal_under_oracle_a(
        &self,
        build: &CompileOutcome,
        reference: &ReferenceArm,
        reference_arm: &str,
    ) -> Outcome {
        let oracle = Oracle::ReferenceCompiler;
        match reference {
            ReferenceArm::ExcludedByRecord => self.excluded(oracle),
            ReferenceArm::DriverAbsent => classify::unavailable_oracle(self.caps, self.key, oracle),
            ReferenceArm::Unrunnable(diagnosis) => classify::judge(
                &classify::Observation::ToolingAbsent {
                    diagnosis: diagnosis.as_str(),
                },
                Some(self.record),
                self.key,
                oracle,
            ),
            ReferenceArm::Internal(cause) => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the reference compiler", self.key),
                &format!(
                    "the compiler under test refused the program, and this arm was never observed \
                     because this suite's own machinery failed while attempting it: {cause}. The \
                     refusal is reported through the other oracles; this arm is a failure of the \
                     harness rather than an unavailable oracle, because an absence the run would not \
                     fail for is how a cell that was launched and lost comes to look like a pass"
                ),
            ),
            // The reference compiler refused the program too. That indicts the test material or the
            // record's reference command, so it is reported at its own scope exactly as it is on the
            // path where bcc built — not folded into bcc's refusal, which would attribute one
            // program's defect to the compiler under test.
            ReferenceArm::Refused {
                class,
                summary,
                attribution: attribution @ (Attribution::Environment | Attribution::Indeterminate),
                provenance,
            } => classify::build_failure(
                self.record,
                self.key,
                oracle,
                classify::BuildRefusal {
                    class: *class,
                    attribution: *attribution,
                    summary,
                    // No dependency root: this refusal is the REFERENCE compiler's, and a marker in
                    // this record documents a limitation of the compiler under test. Borrowing it
                    // here would excuse a defect in the test material with documentation about
                    // something else entirely. Both attributions reaching this arm are unavailable
                    // scopes in any case, which never reach the marker logic at all.
                    root: None,
                    // The provenance IS carried, unlike the root: it is a record of what this arm
                    // ran, not an authority borrowed from elsewhere, so it says nothing about the
                    // compiler under test and everything about the invocation a maintainer has to
                    // repeat.
                    provenance: Some(provenance.clone()),
                },
            ),
            ReferenceArm::Refused {
                summary,
                attribution: Attribution::Compiler,
                ..
            } => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the reference compiler", self.key),
                &format!(
                    "both compilers refused the program. The reference compiler is the authority for \
                     this oracle, so its refusal ({summary}) is a defect in the test program or in \
                     the record's reference command rather than a divergence attributable to the \
                     compiler under test — requirement 1's audit gate is where a program the \
                     reference compiler rejects must be corrected. The compiler under test's own \
                     refusal is reported through the other oracles"
                ),
            ),
            // The one ending with an authority to compare against: the reference compiler accepted
            // the program and bcc did not, which is the divergence this suite exists to find.
            ReferenceArm::Ran(authority) => self.settle_refusal(
                oracle,
                build,
                RefusedSide::UnderTest,
                Side::refused(CaptureRole::UnderTest, self.key.target(), build),
                Some(Side::ran(
                    CaptureRole::ReferenceCompiler,
                    self.key.target(),
                    authority,
                )),
                Some(reference_arm),
            ),
        }
    }

    /// The refusal as oracle (b) sees it, after that oracle's own precondition is resolved.
    ///
    /// The ladder is [`CellPlan::oracle_b`]'s. The refusal itself is the divergence in the last two
    /// cases, and the baseline's own state decides the rest: an absent runner is an absent oracle, a
    /// baseline this suite lost is a harness failure, and a record that enables the oracle without
    /// listing the baseline target is a corpus defect rather than a compiler one.
    fn refusal_under_oracle_b(
        &self,
        build: &CompileOutcome,
        baseline: Option<&Baseline>,
        reference_arm: &str,
    ) -> Outcome {
        let oracle = Oracle::CrossBackend;
        if !self.record.oracle_enabled(oracle) {
            return self.excluded(oracle);
        }
        if !self.caps.oracle_available(oracle, self.key.target()) {
            return classify::unavailable_oracle(self.caps, self.key, oracle);
        }
        // This cell *is* the baseline, so there is no separate baseline observation to require: the
        // refusal being judged is the baseline's own. `applicable_oracles` normally keeps oracle (b)
        // off the baseline target, and this arm is written for the case where it does not rather than
        // relying on that — a settled refusal is a correct answer here either way.
        if self.key.target() == Target::BASELINE {
            return self.settle_refusal(
                oracle,
                build,
                RefusedSide::UnderTest,
                Side::refused(CaptureRole::UnderTest, self.key.target(), build),
                None,
                Some(reference_arm),
            );
        }
        match baseline {
            // The baseline ran and this target's compiler refused the program: a genuine
            // cross-backend divergence, settled against the refusal's own evidence.
            Some(Baseline::Observed(authority)) => self.settle_refusal(
                oracle,
                build,
                RefusedSide::UnderTest,
                Side::refused(CaptureRole::UnderTest, self.key.target(), build),
                Some(Side::ran(
                    CaptureRole::Baseline,
                    Target::BASELINE,
                    authority,
                )),
                Some(reference_arm),
            ),
            // Both this target and the baseline were refused by the same compiler. One refusal, one
            // class: settled once against this cell's own build, with no counterpart capture, because
            // two refusals of the same program are not a cross-backend difference.
            Some(Baseline::Refused(_)) => self.settle_refusal(
                oracle,
                build,
                RefusedSide::UnderTest,
                Side::refused(CaptureRole::UnderTest, self.key.target(), build),
                None,
                Some(reference_arm),
            ),
            Some(Baseline::Unrunnable(diagnosis)) => classify::judge(
                &classify::Observation::ToolingAbsent {
                    diagnosis: diagnosis.as_str(),
                },
                Some(self.record),
                self.key,
                oracle,
            ),
            Some(Baseline::Internal(cause)) => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the cross-backend baseline", self.key),
                &format!(
                    "the compiler under test refused the program, and the {} baseline this arm \
                     compares against was never observed because this suite's own machinery failed \
                     while attempting it: {cause}. The refusal is reported through the other oracles; \
                     this arm is a failure rather than an unavailable oracle",
                    Target::BASELINE.triple(),
                ),
            ),
            None => classify::unexplained(
                self.key,
                oracle,
                &format!("comparing {} against the cross-backend baseline", self.key),
                &format!(
                    "this program's expectation record enables the cross-backend oracle but omits \
                     {} from its target list, so the authority this arm compares against is never \
                     observed. Either list the baseline target or record the cross-backend exclusion \
                     with its reason; comparing against a baseline that was never run would mean \
                     inventing an authority, and dropping the arm silently is the exclusion \
                     requirement 5 forbids",
                    Target::BASELINE.triple(),
                ),
            ),
        }
    }

    /// The refusal as oracle (c) sees it, after that oracle's own precondition is resolved.
    ///
    /// The only precondition is the record's own toggle. This oracle's authority is the
    /// `expected_stdout` committed in the record beside the program, so it needs no compiler, no
    /// driver and no emulator: a program that produced no artifact produced none of those bytes, which
    /// is a divergence from the golden record that is always observable.
    ///
    /// That is what makes the "a refusal is never dropped" claim hold on the barest machine. Whatever
    /// the other two arms report, this one settles the refusal and therefore delivers the artifact
    /// directory — unless the record disables it, in which case the record has said in writing that
    /// this program has no golden authority, and the exclusion is reported as such.
    fn refusal_under_oracle_c(
        &self,
        build: &CompileOutcome,
        reference: &ReferenceArm,
        reference_arm: &str,
    ) -> Outcome {
        let oracle = Oracle::GoldenRecord;
        if !self.record.oracle_enabled(oracle) {
            return self.excluded(oracle);
        }
        self.settle_refusal(
            oracle,
            build,
            RefusedSide::UnderTest,
            Side::refused(CaptureRole::UnderTest, self.key.target(), build),
            // The reference arm becomes a capture only when it actually ran. Every other ending is
            // carried as the phrase instead, because there are no captured bytes to attach and an
            // empty capture would claim there were.
            match reference {
                ReferenceArm::Ran(authority) => Some(Side::ran(
                    CaptureRole::ReferenceCompiler,
                    self.key.target(),
                    authority,
                )),
                _ => None,
            },
            Some(reference_arm),
        )
    }

    /// Tooling this cell needs is absent: every applicable oracle is unavailable, never a pass.
    fn tooling_absent(&self, diagnosis: &str) -> Vec<Outcome> {
        self.oracles
            .iter()
            .map(|oracle| {
                classify::judge(
                    &classify::Observation::ToolingAbsent { diagnosis },
                    Some(self.record),
                    self.key,
                    *oracle,
                )
            })
            .collect()
    }

    /// The harness itself failed on this cell: reported per oracle rather than aborting the area.
    fn internal(&self, error: &HarnessError) -> Vec<Outcome> {
        internal_error_outcomes(self.key, &self.oracles, error)
    }

    /// Retain the workspace when this cell has something to investigate, otherwise clean it up.
    ///
    /// Cleanup never masks a verdict: a workspace that cannot be removed contributes a printed
    /// note and nothing more, because a decided cell must not be re-decided by its own tidying.
    ///
    /// # Where the retention budget is enforced, and why the accounting is printed rather than filed
    ///
    /// Retention is the one thing this driver does that grows without a natural limit — the matrix
    /// has 1,296 cells, and the run in which the compiler under test can build nothing at all is
    /// exactly the run in which every one of them keeps its directory. [`Workspace::retain`] applies
    /// all four ceilings at the moment of retention, so the bound holds continuously rather than
    /// eventually, and it returns the accounting it charged: the bytes this workspace cost, the run
    /// total it was charged against, and every entry that had to be given up to fit.
    ///
    /// That accounting is written to the terminal beside the cell it belongs to and deliberately
    /// **not** folded into the cell's verdicts. Two reasons, and the second is the load-bearing one.
    /// The verdicts are already decided by the time a workspace is retired, and a tidying step must
    /// not amend them. And the run total depends on the order in which fourteen concurrent areas
    /// happen to conclude their cells, so a verdict row carrying it would differ between two runs of
    /// identical inputs — which is the one property the reports are required to hold. The run-level
    /// view is instead reported once, from the retention totals and the pruning notes, when the
    /// summary is assembled: the place where a reduced set of retained evidence is a fact about the
    /// run rather than about any single cell.
    fn retire(self, outcomes: &[Outcome]) {
        let CellPlan {
            caps, workspace, ..
        } = self;
        let config = caps.config();
        let investigate = outcomes
            .iter()
            .any(|outcome| workspace_is_evidence(outcome, config));
        let keep_everything = workspace.keeps_on_success();

        if !investigate && !keep_everything {
            // Last chance to archive what this cell recorded: the workspace is about to be removed,
            // and for a reported-but-not-failing outcome its capture files are the only copy there is.
            // Done before the discard rather than after, for the obvious reason, and done per outcome
            // rather than per cell because the verdicts differ by oracle.
            let archived = conformance_harness::execute::archive_persisted_captures(&workspace);
            for outcome in outcomes
                .iter()
                .filter(|outcome| needs_durable_evidence(outcome))
            {
                println!(
                    "  {}",
                    report::publish_cell_evidence(
                        outcome,
                        &archived,
                        &format!(
                            "this cell's workspace was discarded because nothing in it fails the \
                             run under the active policy; the {} verdict on this arm is reported, \
                             so its evidence is published here instead",
                            outcome.verdict().label()
                        ),
                    )
                );
            }
        }

        if investigate || keep_everything {
            println!("  workspace retained: {}", workspace.retain().describe());
        } else if let Some(note) = workspace.discard_advisory() {
            println!("  note: {note}");
        }
    }
}

/// Whether this outcome makes its cell's workspace evidence that must be kept.
///
/// Two clauses, and the first is the one that changed. Retention used to test a **fixed** verdict set,
/// `FAIL | XPASS | FINDING`, which disagreed with the policy the run actually asserts on: under
/// `BCC_CONFORMANCE_STRICT` an `UNAVAILABLE` *fails the run*, and its workspace — holding the
/// diagnosis, the compiler's own stderr and the termination record — was deleted anyway. A run that
/// fails on an outcome and then destroys that outcome's evidence is the worst combination available:
/// the failure is reported and cannot be investigated. Asking [`classify::fails_run`] means the two
/// answers cannot drift, because they are now one answer.
///
/// The second clause keeps a `FINDING` regardless. A finding does **not** fail the run — it is a
/// deliverable, and requirement 6 asks for the reproducer, both sides' output and the exact commands —
/// so its evidence is needed *precisely* in the runs that pass. `XPASS` needs no clause of its own: it
/// fails the run by default, and where `BCC_CONFORMANCE_ALLOW_XPASS` downgrades it, the marker it
/// invalidates is still listed loudly and its cell still publishes durable evidence below.
fn workspace_is_evidence(outcome: &Outcome, config: &RunConfig) -> bool {
    classify::fails_run(outcome, config) || outcome.verdict() == Verdict::Finding
}

/// Whether this outcome needs a durable evidence document once its workspace is gone.
///
/// Everything except a plain `PASS`. A pass is fully described by its report row — both command lines,
/// both terminations, both capture locations and the byte counts, all carried in the row's provenance
/// columns — and re-running the cell reproduces it, so archiving 3,500 of them per run would bury the
/// documents that matter. Every other verdict reaching this path is *reported* while its workspace is
/// discarded, which is exactly the class that had no durable evidence at all: an expected divergence, a
/// permissive run's absent oracle, and an unexpected success the escape hatch downgraded.
fn needs_durable_evidence(outcome: &Outcome) -> bool {
    outcome.verdict() != Verdict::Pass
}

/// Who the build layer found answerable for a refusal.
///
/// Mirrors the failure's own scope through the classifier's exhaustive conversion, so a scope added
/// to the build layer cannot reach a verdict here until the mapping says what it means. The
/// fallback for a build that reported no failure at all is the honest one: nothing was concluded
/// about who is answerable, so nothing is claimed.
///
/// Written once and used by both refusal sites — the compiler under test and the reference arm —
/// because a difference between them would mean one arm accusing a compiler where the other
/// reported a gap, from identical evidence.
fn build_attribution(build: &CompileOutcome) -> Attribution {
    match build.failure() {
        Some(failure) => Attribution::of_scope(failure.scope()),
        None => Attribution::Indeterminate,
    }
}

/// The shape of a refusal, and one line naming it.
///
/// A build carries a divergence class only when it failed, and every caller here has already
/// established that. The fallback still handles the absent class rather than unwrapping: an
/// artifact that was not produced is a refusal whatever the build layer managed to say about it,
/// and a panic would replace a reportable observation with no result at all.
///
/// The line comes from [`CompileOutcome::describe`], which carries no measured time, so a report
/// row and a finding manifest built from it are byte-identical across two runs of the same inputs.
/// [`CompileOutcome::describe_with_timing`] exists for progress output and is deliberately not used
/// here.
fn refusal_shape(build: &CompileOutcome) -> (DivergenceClass, String) {
    match build.divergence_class() {
        Some(class) => (class, build.describe()),
        None => (
            DivergenceClass::CompileFailure,
            format!(
                "{} — no artifact was produced and the build reported no divergence class",
                build.describe(),
            ),
        ),
    }
}

/// One phrase naming how the reference arm ended, for a refusal finding's evidence.
///
/// Used when the compiler under test refused a program, for every ending the arm can have. Saying
/// *which* of the six happened is what keeps the artifact honest: "the reference driver for this
/// target is absent" and "the reference compiler rejected this program too" point a reader at
/// completely different conclusions, and a finding that merely said the other side was missing would
/// leave that distinction to guesswork.
///
/// [`ReferenceArm::Internal`] is phrased as this suite's own failure rather than as a gap in the
/// environment, for the reason that governs the whole verdict space: an absent runner is a reported
/// gap, while a fault in this harness is a failure, and a finding that described the second as the
/// first would send a maintainer looking for a package that was never missing.
fn describe_reference_arm(arm: &ReferenceArm) -> String {
    match arm {
        ReferenceArm::ExcludedByRecord => String::from(
            "this program's own expectation record disables the reference-compiler oracle, with its \
             reason recorded there",
        ),
        ReferenceArm::DriverAbsent => String::from(
            "no reference driver for this target exists in this environment, so no other compiler \
             was asked to build the program",
        ),
        ReferenceArm::Unrunnable(diagnosis) => format!(
            "the reference arm could not be completed on this machine: {diagnosis}"
        ),
        ReferenceArm::Refused { summary, .. } => format!(
            "the reference compiler also refused this program ({summary}), which points at the test \
             program or the record's reference command rather than at the compiler under test"
        ),
        ReferenceArm::Ran(authority) => format!(
            "the reference compiler built and ran the program: {}",
            authority.build.describe()
        ),
        ReferenceArm::Internal(diagnosis) => format!(
            "the reference arm was never observed because this suite's own machinery failed: \
             {diagnosis}. Nothing here is an observation about either compiler"
        ),
    }
}

/// Build one compiler's artifact for a cell and execute it.
///
/// The build's own evidence is written into the workspace **before** either ending is taken, and
/// that ordering is the point. A cell is retained precisely when something about it needs
/// investigating, and the most common thing needing investigation is a build that refused — the one
/// path that never reaches execution and so, until now, left a retained workspace holding the
/// command lines but not the compiler's answer to them. Persisting on the way past means the
/// evidence is there whichever ending follows, including the refusal.
fn observe(
    compiler: Compiler,
    record: &Manifest,
    substitutions: &CommandSubstitutions,
    key: &CellKey,
    workspace: &Workspace,
    caps: &Capabilities,
) -> Result<Observed, HarnessError> {
    let request = CompileRequest::new(compiler, record, substitutions, workspace)?;
    let build = compile::build(&request, caps)?;
    build.persist(workspace)?;
    if !build.succeeded() {
        // Printed as it happens, and only for a refusal. A refusal is rare and always worth
        // reading, and how long it took is part of reading it: a compiler that declines a program
        // in thirty milliseconds is a verdict, while one that declines it after nearly the whole
        // budget is a hint that something else is wrong. This is terminal progress output, which is
        // the only place a measured time is allowed — the same line without the timing is what
        // reaches the report and the finding manifest, so those stay byte-stable.
        println!("  build refused: {}", build.describe_with_timing());
        return Ok(Observed::Refused(Box::new(build)));
    }
    let names = match compiler {
        Compiler::Bcc => CaptureNames::bcc(),
        Compiler::Reference => CaptureNames::reference(),
    };
    let attempt = execute::run_and_record(build.artifact(), key, workspace, caps, &names)?;
    match attempt {
        RunAttempt::Ran(run) => {
            require_recorded_run_command(record, substitutions, &run)?;
            // Printed as it happens, and only when the execution is divergent by its own shape: a
            // crash or a hang is a divergence however the streams are later compared, so it is
            // always worth reading, and how long it took is part of reading it — a program that dies
            // in three milliseconds is a different story from one killed at the budget. This is
            // terminal progress output, the only place a measured time is allowed; the same line
            // without the timing is what reaches the report, the summary and a finding's diff, so
            // those stay byte-stable.
            if run.divergence_class().is_some() {
                println!("  run diverged by shape: {}", run.describe_with_timing());
            }
            Ok(Observed::Ran(Box::new(Authority { build, run: *run })))
        }
        RunAttempt::RunnerUnavailable(unavailable) => {
            Ok(Observed::Unrunnable(unavailable.describe()))
        }
    }
}

/// Prove the execution that just happened is exactly the one this program's record describes.
///
/// The run-side counterpart of the build-side cross-check `compile.rs` performs against
/// `bcc_command` and `ref_command`, and it exists for the same reason. Each program ships an
/// expectation record carrying a literal `run_command` template, and a maintainer is promised that
/// the program source plus that record reproduce any cell with no harness at all — the promise
/// requirement 4 makes and the promise a finding's `commands.sh` is built on. Both the per-cell
/// `commands.txt` and every finding's reproduction script render that template verbatim.
///
/// Nothing about a disagreement between the template and what actually ran would be visible in a
/// passing run. The harness would go on executing artifacts its own way and comparing their output
/// correctly, while every recorded run line — the one line a maintainer is told to paste — named a
/// command nobody had executed. A maintenance edit to the launch shape, or a record whose template
/// drifted, would turn the run half of every recipe in the corpus into fiction silently.
///
/// The comparison is against [`RunOutcome::argv`] rather than against a vector computed beforehand,
/// because that is the program's own argument vector as literally launched, with this harness's
/// timeout scaffolding deliberately excluded from it. Comparison is element by element rather than
/// line against line: two different vectors can render to the same line once quoting is applied,
/// and it is the vector that is executed.
///
/// # Errors
///
/// A length difference or the first differing element, naming the index, both spellings, the
/// template and both full command lines. A failure to expand the template is propagated too, which
/// means the record names a placeholder this cell's substitutions cannot satisfy.
fn require_recorded_run_command(
    record: &Manifest,
    substitutions: &CommandSubstitutions,
    run: &RunOutcome,
) -> Result<(), HarnessError> {
    let recorded = record.render_run_argv(substitutions)?;
    let launched = run.argv();
    let context = format!(
        "checking the execution of {} against the run template its expectation record declares",
        record.path().display()
    );
    for (index, (actual, expected)) in launched.iter().zip(recorded.iter()).enumerate() {
        if actual == expected {
            continue;
        }
        return Err(HarnessError::new(
            context,
            format!(
                "the execution differs from the one this program's record describes, at argument \
                 {index}: the harness launched {actual:?} where the record's template expands to \
                 {expected:?}. The record is the reproduction recipe a maintainer is promised, so \
                 the two must be one command and the difference is refused rather than preferred \
                 one way or the other. Template in {}: {:?}. Launched: {}. Recorded: {}",
                record.path().display(),
                record.run_command(),
                conformance_harness::posix_command_line(launched),
                conformance_harness::posix_command_line(&recorded)
            ),
        ));
    }
    if launched.len() == recorded.len() {
        return Ok(());
    }
    Err(HarnessError::new(
        context,
        format!(
            "the execution ran with {} argument(s) and the one this program's record describes has \
             {}, agreeing on every argument they share. The record is the reproduction recipe a \
             maintainer is promised, so an execution carrying arguments the recipe omits — or \
             omitting arguments the recipe carries — would make the recipe untrue in a way no \
             comparison of program output could reveal. Template in {}: {:?}. Launched: {}. \
             Recorded: {}",
            launched.len(),
            recorded.len(),
            record.path().display(),
            record.run_command(),
            conformance_harness::posix_command_line(launched),
            conformance_harness::posix_command_line(&recorded)
        ),
    ))
}

/// Run one feature area's entire matrix, report it, and only then decide the test.
///
/// The order is deliberate. Every cell is executed and every verdict accumulated before anything
/// is asserted, because the deliverable this suite owes is a summary enumerating **all** outcomes:
/// asserting inside the loop would stop at the first divergence and truncate exactly that artifact.
/// The report is written before the assertion for the same reason — a failing area must still leave
/// behind the per-area report and, once every area has run, the run summary.
fn run_area(area: &str) {
    let spec = area_spec(area);
    let caps = oracle_capabilities();
    prepare_roots();
    begin_report_session();
    // Before the first cell of this area compiles, so the preconditions the oracles rest on are
    // established and recorded before any artifact is written. Performed once per process and
    // memoized, so the thirteen areas that arrive after the first pay nothing for it.
    let gates = preflight(&caps);

    let programs = select_programs(spec, &caps);

    // The gate BLOCKS here, before the first cell of this area is compiled.
    //
    // Requirement 1 makes undefined-behaviour freedom the precondition that gives an oracle its
    // meaning: a program containing undefined behaviour permits both compilers to do anything, so a
    // comparison over it is not evidence either way. Running the matrix first and asserting the gate
    // afterwards would therefore be wrong in a way that matters rather than merely untidy: a
    // divergence over a program whose gate had failed would be classified, and filed as a FINDING
    // with a full artifact directory. A finding is a deliverable that says "the compiler did this";
    // producing one from a program not shown to be undefined-behaviour-free delivers a claim the
    // suite has no standing to make.
    //
    // So nothing is compiled, nothing is classified and nothing is published as evidence. What is
    // published is the area's report carrying the failing gate, the withholding stated in its own
    // words, and an empty tally that can never be read as agreement — after which the area fails.
    // The report-before-assert discipline the rest of this file keeps is preserved exactly: the
    // report is written first, so the correction can be checked against what the gate said.
    let blocking = gates.recorded.blocking_area(spec.directory());
    if !blocking.is_empty() {
        let withheld = withheld_notes(spec, &programs, &blocking);
        publish(spec, &[], &caps, &withheld);
        panic!("{}", preflight_gap(spec.directory(), &blocking));
    }

    let mut outcomes: Vec<Outcome> = Vec::new();
    for program in &programs {
        outcomes.extend(run_program(&caps, program));
    }

    let digest = area_digest(spec, &programs, &outcomes, caps.config());
    println!("{digest}");
    publish(spec, &outcomes, &caps, &[]);
    conclude(spec, &programs, &outcomes, &digest, caps.config(), gates);
}

/// What the report of a withheld area must say, in the caller's own words.
///
/// Three facts a reader needs before anything else in that report: that the area was withheld rather
/// than merely empty, which gate withheld it, and how many programs did not run. The program count is
/// stated because an empty tally beside a corpus of ten programs is a different thing from an empty
/// tally beside a corpus of none, and only the caller knows which this is — the report module is handed
/// no outcomes and could not tell them apart.
fn withheld_notes(
    spec: &'static AreaSpec,
    programs: &[PathBuf],
    blocking: &[&report::PreflightGate],
) -> Vec<String> {
    let mut notes = vec![format!(
        concat!(
            "⚠️ NO CELL OF THIS AREA WAS RUN, AND NOTHING HERE IS EVIDENCE ABOUT THE COMPILER",
            " UNDER TEST. {} preflight gate(s) governing `{}` did not hold, so its {} program(s)",
            " were withheld before the first compile. Nothing here was compiled, classified or",
            " filed as a finding, and the empty tally below can therefore never be read as",
            " agreement. A gate withholds for one of two reasons and both are preconditions",
            " rather than results: requirement 1 makes undefined-behaviour freedom the property",
            " an oracle rests on, so a comparison made without it is evidence of nothing; and a",
            " record whose own review has not completed has no authority to judge a compiler, so",
            " requirement 6 gives the suite no standing to file what it would produce.",
        ),
        blocking.len(),
        spec.directory(),
        programs.len(),
    )];
    for gate in blocking {
        notes.push(format!("⚠️ withholding gate — {}", gate.describe()));
    }
    // Named individually, and named here rather than left to the gate's own one-line detail, because
    // a reader of THIS area's report is the one person who needs to know exactly which record
    // withheld it and what has to happen for the area to return.
    let pending: Vec<&PendingRecord> = substantiation()
        .pending()
        .iter()
        .copied()
        .filter(|entry| entry.area == spec.directory())
        .collect();
    if !pending.is_empty() {
        for entry in &pending {
            notes.push(format!(
                "⚠️ withheld record — {}/{} is pending re-substantiation, so no cell of it may \
                 produce evidence. Outstanding: {}. Documented basis: {}.",
                entry.area, entry.program, entry.outstanding, entry.basis,
            ));
        }
        // The collateral is stated rather than left to be inferred. A preflight gate's finest
        // granularity is the feature area, so the substantiated programs sharing this area are
        // withheld alongside the pending record. Withholding a little more than strictly necessary is
        // the safe direction; leaving a reader to work out that it happened is not.
        let others = programs.len().saturating_sub(pending.len());
        if others > 0 {
            notes.push(format!(
                "⚠️ withheld with it — the other {others} program(s) in `{}` have completed their \
                 review, and are withheld only because a preflight gate is area-granular. They \
                 return, unchanged, on the first run after the record(s) above are substantiated; \
                 retiring the declaration is the whole of that work.",
                spec.directory(),
            ));
        }
    }
    notes
}

/// Resolve the oracles once, or refuse to start.
///
/// Discovery is memoized in the harness, so calling this from every test is cheap and every test
/// sees the same environment. A failure here is fatal rather than degraded: with no compiler under
/// test there is nothing this suite could decide, and a green run over zero cells would be
/// indistinguishable from a suite that works.
fn oracle_capabilities() -> Capabilities {
    match discovery::discover() {
        Ok(caps) => caps,
        Err(error) => panic!(
            "the differential conformance suite cannot start.\n\n{error}\n\nThe compiler under \
             test and the run configuration are resolved before any cell executes. Point {} at the \
             binary to test, or build the package that provides it; this is never reported as an \
             unavailable oracle, because an absent compiler under test is not a gap in the \
             environment but the absence of the subject.",
            discovery::VAR_BCC_BIN,
        ),
    }
}

/// Measure the corpus once per process, and refuse to start if it cannot be enumerated at all.
///
/// # What this is, and what it deliberately is not
///
/// It is the answer to "how much of the planned corpus is present, and how large are its records",
/// measured from the tree. It is **not** a filter, and nothing here excuses a program from being
/// judged: a source whose record cannot be read is reported as pending, and every consumer treats
/// pending as the corpus defect it is — the driver files a corpus-defect row for it, which fails the
/// run, and the marker audit refuses to call its inventory complete. Requirement 5 forbids silently
/// excluding a feature from testing, so an incomplete corpus is stated loudly rather than swept.
///
/// Memoized because two tests consume it and measuring parses every record in the corpus. A failure
/// is fatal for the same reason [`oracle_capabilities`] is: discovery only fails when a feature area
/// is absent or holds something that is not a program, and a run that could not enumerate the corpus
/// has no basis on which to report anything about it.
fn corpus_inventory() -> &'static manifest::CorpusInventory {
    static INVENTORY: OnceLock<manifest::CorpusInventory> = OnceLock::new();
    INVENTORY.get_or_init(|| match manifest::measure_corpus() {
        Ok(inventory) => inventory,
        Err(error) => panic!(
            "{}",
            infrastructure_failure(
                "the corpus inventory",
                "the corpus could not be enumerated, so neither the matrix this run will sweep nor \
                 the completeness of the corpus it sweeps can be stated, and every count derived \
                 from either would be a guess",
                &error,
            )
        ),
    })
}

/// The declared specification for an area name, which is also a guard against a typo.
fn area_spec(area: &str) -> &'static AreaSpec {
    match AreaSpec::lookup(area) {
        Some(spec) => spec,
        None => panic!(
            "the feature area {area:?} is not one of the {AREA_COUNT} the harness declares, so its \
             verdicts would be filed under a name no report, register or summary knows. The \
             declared areas are: {}",
            AREAS
                .iter()
                .map(AreaSpec::directory)
                .collect::<Vec<&str>>()
                .join(", "),
        ),
    }
}

/// Claim the report directory for this run, discarding the previous run's artifacts.
///
/// Every one of the eighteen tests calls this, including the four infrastructure tests that write
/// no area report at all — precisely because they write none. Without it, a run restricted to the
/// infrastructure tests, or to one area by name, would leave a previous full summary standing in the
/// report root where the next reader would take it for the current verdict. The harness performs the
/// work exactly once per process and blocks concurrent callers until it is done, so calling it from
/// every test costs one directory scan for the whole run.
///
/// It is deliberately the **same** entry point `prepare_roots` uses, and there is no second, lighter
/// one. Clearing the report root is a destructive step, and every destructive step in this suite has
/// to refuse a live foreign owner before it removes anything and verify every level on the way down
/// before it follows one. A separate "just invalidate the artifacts" route would be able to delete a
/// concurrent run's reports, and to do it through a redirected directory, from the four tests least
/// likely to be looked at when a report went missing.
fn begin_report_session() {
    if let Err(error) = report::prepare_namespace() {
        panic!(
            "the run's report directory could not be claimed and cleared.\n\n{error}\n\nThe run \
             summary is assembled from the per-area report files on disk, so an artifact this run \
             did not write would be aggregated into this run's summary and reported as its result. \
             This fails rather than proceeding, because a summary that silently blends two runs is \
             worse than no summary at all.",
        );
    }
}

// =================================================================================================
// The preflight gates
//
// Two of this file's eighteen tests establish preconditions rather than compare anything: the
// flag-capability probe establishes that every flag a differential invocation passes means the same
// thing to both compilers (requirement 3), and the undefined-behaviour audit establishes that the
// corpus is free of undefined behaviour (requirement 1). Requirement 1 states the consequence in its
// own terms — a program containing undefined behaviour permits both compilers to do anything — so
// while either precondition is unmet a PASS is not evidence of agreement and a divergence is not
// evidence of a defect.
//
// Two independent `#[test]` functions cannot express that. The built-in harness runs all eighteen
// tests concurrently in one process with no ordering between them, so "run the gates first" is not
// something a caller can arrange and not something a test can assert; and a failing infrastructure
// test leaves all fourteen area verdicts standing beside it, each reporting a green matrix whose
// comparisons are not evidence.
//
// What is achievable, and what this section implements, is:
//
//   * each gate is performed **exactly once per process**, memoized below, so every test — area or
//     infrastructure — observes the same result at the cost of one execution rather than two;
//   * every area calls this before its first cell compiles, so the result is recorded in the report
//     module before any artifact is written and every artifact therefore names it;
//   * every area **asserts** on the gates that govern it, so an unmet precondition fails the areas it
//     bears on rather than only the infrastructure test that noticed it.
//
// The two infrastructure tests keep their own, fuller assertions: they render the whole gate report,
// which is what an author actually fixes a program from. They now read the memoized result instead of
// performing the gate a second time.
// =================================================================================================

/// The two preflight gates, performed once per process. See [`preflight`].
static PREFLIGHT: OnceLock<PreflightGates> = OnceLock::new();

/// This run's preflight gates, and the record every report renders from them.
struct PreflightGates {
    /// The flag-capability probe, or the reason it could not be performed at all.
    probe: Result<flagprobe::FlagProbeReport, HarnessError>,
    /// The undefined-behaviour audit, or the reason it could not be performed at all.
    audit: Result<ubaudit::AuditReport, HarnessError>,
    /// The same gates in the form the report module records and renders.
    ///
    /// Held here as well as recorded there so that an area's assertion and the artifact that area
    /// wrote are derived from one object. Recomputing the blocking set at the assertion would give
    /// two answers a maintainer could not reconcile if they ever disagreed.
    recorded: report::Preflight,
}

/// Perform both preflight gates once, record them, and return them for every later reader.
///
/// Memoized in a `OnceLock`, which is sufficient here and needs no on-disk state: every one of the
/// eighteen tests is a thread in a single process, so the first caller performs the gates and the
/// rest block until it is done. A second caller never re-performs them, which matters — the audit is
/// two reference-compiler invocations per corpus program, and the probe compiles with both compilers
/// for every flag it checks.
///
/// The gates are performed **before** any area compiles its first cell, and the record is published
/// to the report module inside the same initialization, so no report can be written that does not
/// carry the preconditions it rests on.
fn preflight(caps: &Capabilities) -> &'static PreflightGates {
    PREFLIGHT.get_or_init(|| {
        let mut gates = PreflightGates {
            probe: flagprobe::run(caps),
            audit: ubaudit::run(caps),
            recorded: report::Preflight::default(),
        };
        let recorded = gates.assemble(caps.config());
        gates.recorded = recorded.clone();
        // `get_or_init` runs this closure exactly once for the whole process, so this records the
        // preflight exactly once and the `false` return that a second recording would give cannot
        // arise. It is asserted rather than ignored, because a silent failure to record would leave
        // every report stamped "NOT RECORDED" while the gates had in fact been performed.
        assert!(
            report::record_preflight(recorded),
            "this run's preflight gates were performed but could not be recorded, because a \
             preflight had already been recorded. The gates are performed exactly once per process \
             and recorded from that one place, so this means a second recording path exists — and \
             while it does, the reports may describe a preflight the areas were not judged against."
        );
        gates
    })
}

impl PreflightGates {
    /// Express the four kinds of gate in the form the report module records and renders.
    ///
    /// The flag probe contributes one gate governing every area, because flag parity is a property
    /// of the configuration rather than of any program. The audit contributes gates narrowed to the
    /// areas they bear on: a program in one area whose gate did not hold says nothing about another
    /// area's programs, and failing all fourteen for it would report fourteen defects where there is
    /// one. Marker integrity and record substantiation follow that same narrowing rule, each for its
    /// own reason, stated where each gate is built.
    fn assemble(&self, config: &RunConfig) -> report::Preflight {
        let mut gates = vec![self.flag_gate()];
        gates.extend(self.audit_gates(config));
        gates.extend(marker_gates());
        gates.extend(substantiation_gates());
        report::Preflight::new(gates)
    }

    /// The flag-capability probe as one gate governing the whole run.
    fn flag_gate(&self) -> report::PreflightGate {
        let probe = match &self.probe {
            Ok(probe) => probe,
            // The probe machinery itself failed, so nothing was checked. That is `Unperformed`
            // rather than `Failed` — no flag was shown to mean two things — and it blocks
            // unconditionally, because a probe that cannot run establishes nothing at all and the
            // strict setting's licence covers an absent tool, not a broken harness.
            Err(error) => {
                return report::PreflightGate::new(
                    FLAG_GATE_NAME,
                    REQUIREMENT_FLAGS,
                    report::GateVerdict::Unperformed,
                    format!(
                        "the probe could not be performed at all, so no flag this suite passes has \
                         been shown to mean the same thing to both compilers: {error}"
                    ),
                    Vec::new(),
                    true,
                );
            }
        };
        let failures = probe.failures();
        let unavailable = probe.unavailable();
        let (verdict, detail) = if !failures.is_empty() {
            (
                report::GateVerdict::Failed,
                format!(
                    "{} of {} check(s) did not hold: {}",
                    failures.len(),
                    probe.checks().len(),
                    failures
                        .iter()
                        .map(|check| check.subject().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
        } else if !unavailable.is_empty() {
            (
                report::GateVerdict::Unperformed,
                format!(
                    "{} of {} check(s) could not be performed because a tool is absent, so the \
                     flag(s) they cover are unverified rather than verified: {}",
                    unavailable.len(),
                    probe.checks().len(),
                    unavailable
                        .iter()
                        .map(|check| check.subject().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
        } else {
            (
                report::GateVerdict::Held,
                format!(
                    "all {} check(s) held, {} of them for acceptance only with the limitation \
                     recorded",
                    probe.checks().len(),
                    probe.limitations().len()
                ),
            )
        };
        report::PreflightGate::new(
            FLAG_GATE_NAME,
            REQUIREMENT_FLAGS,
            verdict,
            detail,
            Vec::new(),
            probe.fails_run(),
        )
    }

    /// The undefined-behaviour audit as up to two gates, each narrowed to the areas it bears on.
    ///
    /// A clean audit contributes exactly one `Held` gate. An audit with failures contributes a
    /// `Failed` gate naming the areas whose programs did not satisfy a gate, and an audit with gates
    /// that could not be applied contributes a separate `Unperformed` one — separate because the two
    /// carry different policy: a failure always blocks, while a gate that could not be applied blocks
    /// only under the strict setting, exactly as `AuditReport::fails_run` decides it.
    fn audit_gates(&self, config: &RunConfig) -> Vec<report::PreflightGate> {
        let audit = match &self.audit {
            Ok(audit) => audit,
            // As with the probe: nothing was audited, so no program in the matrix has been shown
            // free of undefined behaviour and every divergence the run produces is unattributable.
            Err(error) => {
                return vec![report::PreflightGate::new(
                    UB_GATE_NAME,
                    REQUIREMENT_UB,
                    report::GateVerdict::Unperformed,
                    format!(
                        "the corpus could not be enumerated or gated at all, so no program has been \
                         shown free of undefined behaviour: {error}"
                    ),
                    Vec::new(),
                    true,
                )];
            }
        };
        let failed_areas = distinct_areas(&audit.failures());
        let unapplied_areas = distinct_areas(&audit.unapplied());
        if failed_areas.is_empty() && unapplied_areas.is_empty() {
            return vec![report::PreflightGate::new(
                UB_GATE_NAME,
                REQUIREMENT_UB,
                report::GateVerdict::Held,
                format!(
                    "all {} audited program(s) satisfied both gates in {} invocation(s), and {} of \
                     them record the written freedom argument",
                    audit.program_count(),
                    audit.invocations_performed(),
                    audit.ub_notes_recorded_count()
                ),
                Vec::new(),
                false,
            )];
        }
        let mut gates: Vec<report::PreflightGate> = Vec::new();
        if !failed_areas.is_empty() {
            gates.push(report::PreflightGate::new(
                format!("{UB_GATE_NAME} — gates that did not hold"),
                REQUIREMENT_UB,
                report::GateVerdict::Failed,
                format!(
                    "{} gate(s) across {} program(s) were not satisfied, in area(s) {}. Correct the \
                     TEST PROGRAM the audit names — never the compiler — or record a reasoned gate \
                     deviation in the program's own expectation record",
                    audit.failures().len(),
                    audit.program_count(),
                    failed_areas.join(", ")
                ),
                failed_areas,
                true,
            ));
        }
        if !unapplied_areas.is_empty() {
            gates.push(report::PreflightGate::new(
                format!("{UB_GATE_NAME} — gates that could not be applied"),
                REQUIREMENT_UB,
                report::GateVerdict::Unperformed,
                format!(
                    "{} gate(s) could not be applied on this machine, in area(s) {}, so the \
                     programs they cover are unproven rather than proven or disproven{}",
                    audit.unapplied().len(),
                    unapplied_areas.join(", "),
                    match audit.unavailable_reason() {
                        Some(reason) => format!(" — {reason}"),
                        None => String::new(),
                    }
                ),
                unapplied_areas,
                config.strict(),
            ));
        }
        gates
    }

    /// The flag-capability probe, or the message a test should fail with instead.
    fn flag_probe(&self, test: &str) -> &flagprobe::FlagProbeReport {
        match &self.probe {
            Ok(probe) => probe,
            Err(error) => panic!(
                "{}",
                infrastructure_failure(
                    test,
                    "the flag-capability probe could not be performed at all, so requirement 3 is \
                     unverified and every differential comparison in this suite would rest on an \
                     unchecked assumption",
                    error,
                )
            ),
        }
    }

    /// The undefined-behaviour audit, or the message a test should fail with instead.
    fn ub_audit(&self, test: &str) -> &ubaudit::AuditReport {
        match &self.audit {
            Ok(audit) => audit,
            Err(error) => panic!(
                "{}",
                infrastructure_failure(
                    test,
                    "the undefined-behaviour audit could not enumerate or gate the corpus, so no \
                     program in the matrix has been shown free of undefined behaviour and a \
                     divergence could not be attributed to either compiler",
                    error,
                )
            ),
        }
    }
}

/// The name the reports give the flag-capability gate.
const FLAG_GATE_NAME: &str = "flag-capability probe";

/// The name the reports give the undefined-behaviour gate.
const UB_GATE_NAME: &str = "undefined-behaviour audit";

/// The name the reports give the expected-divergence marker gate.
const MARKER_GATE_NAME: &str = "expected-divergence marker integrity";

/// The name the reports give the record-substantiation gate.
const SUBSTANTIATION_GATE_NAME: &str = "record substantiation";

/// The requirement the flag-capability gate establishes.
const REQUIREMENT_FLAGS: &str = "requirement 3 — verified flag handling";

/// The requirement the undefined-behaviour gate establishes.
const REQUIREMENT_UB: &str = "requirement 1 — undefined-behaviour freedom";

/// The requirement the marker gate establishes.
const REQUIREMENT_MARKERS: &str =
    "requirement 5 — an expected divergence is marked, never silently excluded";

/// The requirement the substantiation gate establishes.
const REQUIREMENT_SUBSTANTIATION: &str =
    "requirement 4 — the record is the reproduction authority, and requirement 6 — a finding is a \
     deliverable the suite has standing to make";

/// The marker audit as up to two gates, each narrowed to what it actually bears on.
///
/// # Why this is a gate and not only an infrastructure test
///
/// A marker's whole function is to make the difference between a divergence reported as a `FINDING`
/// and one excused as an `XFAIL`. Requirement 5's audit is what establishes that a marker is entitled
/// to do that — that it is registered, described, consistent with its record field for field, cites a
/// basis that exists and says what it is cited for, and excuses only the divergence class it
/// documents. Until the audit has passed, a marker applying itself is an unaudited excuse, and the
/// built-in harness's concurrency means an area could reach that classification, publish it in its
/// report and finish before an independent test failed. So the audit is performed inside
/// [`preflight`] — once, before any area compiles — and an area whose markers it faults is withheld
/// rather than classified. `infra_expected_divergence_register` keeps its own fuller assertion,
/// because the whole audit is what a maintainer fixes a register from.
///
/// # Why two gates rather than one
///
/// Exactly the reason [`PreflightGates::audit_gates`] gives: a marker can only ever excuse a
/// divergence in the program it sits beside, so a defective marker faults **its** area and no other.
/// A defect in the register itself, in the curated finding set or in the corpus enumeration belongs to
/// no single area and therefore governs every one of them. Both gates are `Failed` and both block
/// unconditionally: nothing was merely unavailable here — the audit reads committed files only and
/// always reaches an answer, so a defect is always a defect in the test material.
fn marker_gates() -> Vec<report::PreflightGate> {
    let audit = marker_integrity();
    if audit.violations().is_empty() {
        return vec![report::PreflightGate::new(
            MARKER_GATE_NAME,
            REQUIREMENT_MARKERS,
            report::GateVerdict::Held,
            format!(
                "the bidirectional audit of {} marker(s) against {} held in every direction — \
                 registration, structured description field for field, reverse reconciliation, \
                 basis citation, and the classification contract over every target, level, oracle \
                 and divergence class",
                audit.marker_count(),
                classify::EXPECTED_DIVERGENCE_REGISTER,
            ),
            Vec::new(),
            false,
        )];
    }

    let mut gates: Vec<report::PreflightGate> = Vec::new();
    let defective = audit.defective_areas();
    if !defective.is_empty() {
        let count: usize = defective
            .iter()
            .map(|area| audit.violations_for(area).len())
            .sum();
        gates.push(report::PreflightGate::new(
            format!("{MARKER_GATE_NAME} — marker(s) not entitled to excuse a divergence"),
            REQUIREMENT_MARKERS,
            report::GateVerdict::Failed,
            format!(
                "{count} defect(s) across {} area(s) — {} — so no marker in them may classify a \
                 divergence as an expected one. Correct the marker in the program's own record, or \
                 its entry in {}, until `infra_expected_divergence_register` passes",
                defective.len(),
                defective.join(", "),
                classify::EXPECTED_DIVERGENCE_REGISTER,
            ),
            defective,
            true,
        ));
    }
    if audit.has_run_wide() {
        let run_wide: Vec<&MarkerViolation> = audit
            .violations()
            .iter()
            .filter(|violation| violation.area().is_none())
            .collect();
        gates.push(report::PreflightGate::new(
            format!("{MARKER_GATE_NAME} — register or curated-finding defect"),
            REQUIREMENT_MARKERS,
            report::GateVerdict::Failed,
            format!(
                "{} defect(s) belong to {}, {} or the corpus enumeration rather than to any one \
                 area, so they govern every area: {}",
                run_wide.len(),
                classify::EXPECTED_DIVERGENCE_REGISTER,
                classify::FINDINGS_REGISTER,
                first_sentences(&run_wide),
            ),
            Vec::new(),
            true,
        ));
    }
    gates
}

/// The record-substantiation audit as up to two gates.
///
/// # What this gate exists to stop
///
/// Structural discovery is not admission. The driver finds every source in the corpus, loads every
/// record beside it and sweeps every declared cell — which is right, and is what keeps requirement 5's
/// prohibition on silent exclusion honest. But a record that parses is not the same thing as a record
/// whose contents have been reviewed, and this repository's own documentation keeps the two apart: the
/// enumerable matrix in `tests/conformance/README.md` and the honest-measurement section of
/// `tests/conformance/EXPECTED_DIVERGENCES.md` both publish a **substantiated** column beside the
/// structural one for exactly that reason.
///
/// The two columns carry the same figure at this checkpoint — all 108 records are substantiated, and
/// [`PENDING_RECORDS`] is correspondingly empty — so the gate below currently reports `HELD` rather
/// than withholding anything. That is the instance, not the mechanism, and the distinction is the
/// reason the column and this gate are both kept: the substantiated figure falls behind the structural
/// one again the moment a program lands with a record whose review has not completed, and the gate is
/// what makes that fall visible instead of letting the run publish verdicts over it.
///
/// Without a gate, that distinction lived only in prose. Given a real compiler under test, an
/// unsubstantiated record's `expected_stdout`, `expect_exit`, command templates and marker would all
/// be used to judge that compiler, and its verdicts would enter the area report and the run summary
/// indistinguishable from the substantiated ones — and a divergence over it could be filed as a
/// FINDING, which is a deliverable asserting "the compiler did this". A suite has no standing to make
/// that assertion from material whose own authority is still under review, which is why this fails
/// closed rather than warning.
///
/// # Why it withholds an area rather than a single program
///
/// A preflight gate's finest granularity is the feature area, and deliberately so: it is the unit an
/// area test asserts on and the unit a report is written for. So the gate names the area holding the
/// pending record, `run_area` withholds that area before its first compile, publishes a report saying
/// which record withheld it and that nothing was compiled, classified or filed, and then fails. The
/// substantiated programs sharing that area are withheld with it; [`withheld_notes`] says so in the
/// report rather than leaving a reader to infer it, and they return the moment the record is
/// substantiated. Withholding a little more than strictly necessary is the safe direction: the unsafe
/// direction is publishing one cell of evidence the suite cannot stand behind.
///
/// # Why the pending set is declared here
///
/// Substantiation is a **review state**, and no property of a file expresses it. It is deliberately
/// not inferred from anything on disk: `oracle_b = disabled` happens to single out the same record
/// today, and using it would be a false proxy that silently withheld the next legitimately narrowed
/// record and stopped withholding this one the moment its narrowing changed. So the set is declared,
/// each entry citing the committed documents that record the pending state and naming what is
/// outstanding, and it is checked against the corpus in both directions — a declared record that is
/// not in the corpus is itself a defect, because a stale withholding suppresses evidence for no
/// reason and would not otherwise be visible.
fn substantiation_gates() -> Vec<report::PreflightGate> {
    let audit = substantiation();
    let mut gates: Vec<report::PreflightGate> = Vec::new();

    if !audit.stale().is_empty() {
        // Run-wide: the driver's own admission data is wrong, so no area's admission can be trusted
        // until it is corrected. This is the direction a reader would never otherwise see — a
        // withholding that suppresses nothing, because it names something that is not there.
        gates.push(report::PreflightGate::new(
            format!("{SUBSTANTIATION_GATE_NAME} — stale withholding"),
            REQUIREMENT_SUBSTANTIATION,
            report::GateVerdict::Failed,
            format!(
                "{} declared pending record(s) name nothing in the corpus: {}. A withholding that \
                 names a record which is not there suppresses no evidence and hides that it is \
                 suppressing none, so the declaration is corrected or retired before any area is \
                 judged",
                audit.stale().len(),
                audit.stale().join("; "),
            ),
            Vec::new(),
            true,
        ));
    }

    let pending = audit.pending();
    if pending.is_empty() {
        if audit.stale().is_empty() {
            gates.push(report::PreflightGate::new(
                SUBSTANTIATION_GATE_NAME,
                REQUIREMENT_SUBSTANTIATION,
                report::GateVerdict::Held,
                format!(
                    "every one of the {} record(s) the corpus holds is substantiated, so every \
                     cell this run reaches is entitled to be read as evidence about the compiler \
                     under test",
                    corpus_inventory().records(),
                ),
                Vec::new(),
                false,
            ));
        }
        return gates;
    }

    let areas: Vec<String> = {
        let mut seen: Vec<String> = Vec::new();
        for entry in pending {
            if !seen.iter().any(|area| area == entry.area) {
                seen.push(entry.area.to_string());
            }
        }
        seen
    };
    gates.push(report::PreflightGate::new(
        format!("{SUBSTANTIATION_GATE_NAME} — record(s) pending re-substantiation"),
        REQUIREMENT_SUBSTANTIATION,
        report::GateVerdict::Failed,
        format!(
            "{} record(s) in area(s) {} have not completed their review, so nothing in those areas \
             may be compiled, classified or filed as evidence about the compiler under test: {}",
            pending.len(),
            areas.join(", "),
            pending
                .iter()
                .map(|entry| format!(
                    "{}/{} — outstanding: {} (basis: {})",
                    entry.area, entry.program, entry.outstanding, entry.basis
                ))
                .collect::<Vec<String>>()
                .join("; "),
        ),
        areas,
        true,
    ));
    gates
}

/// The first sentence of each of a few violations, for a gate's one-line detail.
///
/// A gate line is read in fourteen reports and has to stay one line; the whole violation belongs to
/// `infra_expected_divergence_register`, which prints every one of them in full. Three is enough to
/// tell a reader what kind of defect this is and where to look, and the count above it says how many
/// there are in total.
fn first_sentences(violations: &[&MarkerViolation]) -> String {
    let shown: Vec<String> = violations
        .iter()
        .take(3)
        .map(|violation| {
            let message = violation.message();
            match message.find(". ") {
                Some(end) => message[..end].to_string(),
                None => message.to_string(),
            }
        })
        .collect();
    if violations.len() > shown.len() {
        format!(
            "{}; and {} more, all listed by infra_expected_divergence_register",
            shown.join("; "),
            violations.len() - shown.len(),
        )
    } else {
        shown.join("; ")
    }
}

/// One record whose review has not completed, and why it is withheld.
///
/// Declared rather than inferred: see [`substantiation_gates`] for why no property of a file can
/// express a review state, and why using one as a proxy would be worse than declaring it.
struct PendingRecord {
    /// The feature area directory holding it.
    area: &'static str,
    /// The program the record sits beside, without its extension.
    program: &'static str,
    /// The committed documents that record the pending state, cited so a reader can check it.
    basis: &'static str,
    /// What remains to be done before the record is substantiated.
    outstanding: &'static str,
}

/// Every record whose review has not completed at this checkpoint.
///
/// Kept as a declaration in the driver because substantiation is a review state, and audited against
/// the corpus in both directions by [`substantiation`]. **Retiring an entry is the whole of the work
/// once its review completes**: delete the row, and the area it named is admitted again on the next
/// run with no other change anywhere.
///
/// The set is **empty**, and it is kept rather than deleted because it is the mechanism, not the
/// instance: the one record it held,
/// `13_floating_point/004_long_double_target_restricted.expected`, has completed its review, and the
/// defects that review found have been corrected in the record itself rather than merely noted.
/// Four corrections, in the order they were made:
///
/// - its written undefined-behaviour argument now carries the excess-intermediate-precision,
///   conversion, literal, printing, characteristic-macro and magnitude obligations it had left to
///   the program's own comments;
/// - its measured reason for disabling oracle (b) was re-measured, dropping the unsupportable
///   exponent-range claim in favour of the significand widths — 64 bits against 113 — that the
///   exclusion actually rests on, with the loose "three different formats" wording corrected to
///   "three storage-and-format pairings over two distinct formats";
/// - the argument's claim that character-type access to the type's object representation would be
///   **undefined behaviour** was withdrawn as false. Such access is permitted; what makes a byte
///   image unusable here is that the padding bytes are unspecified and that both the padding and
///   the encoding are target-dependent, and the record now says that instead — the program inspects
///   no representation either way;
/// - the marker's class was changed from `stdout_mismatch` to `comparison_excluded`. The old value
///   described the arm the record **switches off** as though two completed runs had disagreed on
///   bytes, which no machine check could contradict precisely because nothing runs on that arm. The
///   parser now requires the narrowing class there and refuses it on an arm that is compared, so
///   the class and the oracle toggle cannot drift apart again.
///
/// Every record the corpus holds is therefore substantiated, and the gate below now reports that
/// rather than withholding an area.
const PENDING_RECORDS: [PendingRecord; 0] = [];

/// What the record-substantiation audit found.
struct Substantiation {
    /// The declared pending records that are present in the corpus, and so genuinely withheld.
    pending: Vec<&'static PendingRecord>,
    /// Declared pending records that name nothing in the corpus, which is a defect of its own.
    stale: Vec<String>,
}

impl Substantiation {
    /// The records genuinely withheld from producing evidence.
    fn pending(&self) -> &[&'static PendingRecord] {
        &self.pending
    }

    /// The declarations that name nothing in the corpus.
    fn stale(&self) -> &[String] {
        &self.stale
    }
}

/// Resolve the declared pending set against the corpus once, and memoize it.
///
/// Both directions are checked, for the reason [`substantiation_gates`] records: a declared record
/// that is present is withheld, and a declared record that is absent is a stale withholding and a
/// defect. The corpus side comes from [`corpus_inventory`], which has already read every area, so this
/// adds no scan of its own.
fn substantiation() -> &'static Substantiation {
    static SUBSTANTIATION: OnceLock<Substantiation> = OnceLock::new();
    SUBSTANTIATION.get_or_init(|| {
        let mut pending: Vec<&'static PendingRecord> = Vec::new();
        let mut stale: Vec<String> = Vec::new();
        for entry in &PENDING_RECORDS {
            let record = corpus_root()
                .join(entry.area)
                .join(format!("{}.expected", entry.program));
            let source = record.with_extension("c");
            if record.is_file() && source.is_file() {
                pending.push(entry);
            } else {
                stale.push(format!(
                    "{}/{} is declared pending re-substantiation but {} is not a readable pair in \
                     the corpus",
                    entry.area,
                    entry.program,
                    shown_path(&record),
                ));
            }
        }
        Substantiation { pending, stale }
    })
}

/// The distinct feature areas named by a list of audit results, in the order the corpus gave them.
fn distinct_areas(results: &[(&ubaudit::ProgramAudit, &ubaudit::GateResult)]) -> Vec<String> {
    let mut areas: Vec<String> = Vec::new();
    for (audit, _) in results {
        if !areas.iter().any(|area| area == audit.area()) {
            areas.push(audit.area().to_string());
        }
    }
    areas
}

/// Why an area fails when a precondition it depends on did not hold.
///
/// Carried by the area assertion rather than only by the infrastructure test, which is the whole
/// point: an area that reported a clean matrix while its precondition was unmet would be publishing
/// a green result over comparisons that are not evidence.
fn preflight_gap(area: &str, blocking: &[&report::PreflightGate]) -> String {
    let mut text = format!(
        "the feature area {area:?} did not run against sound preconditions: {} preflight gate(s) \
         that govern it did not hold.\n\n",
        blocking.len(),
    );
    for gate in blocking {
        text.push_str("   ");
        text.push_str(&gate.describe());
        text.push('\n');
    }
    text.push_str(
        "\nThese gates are not comparisons and they are not verdicts about the compiler under \
         test. They establish the four preconditions that give this area's cells their meaning: \
         that every flag this suite passes means the same thing to both compilers (requirement 3); \
         that every program in the corpus is free of undefined behaviour (requirement 1); that \
         every expected-divergence marker is registered, described and consistent with its record \
         before it is allowed to excuse anything (requirement 5); and that every record judging a \
         compiler has completed its own review (requirements 4 and 6). Requirement 1 states the \
         consequence in its own terms — a program containing undefined behaviour permits both \
         compilers to do anything — and the other three fail the same way round: an unverified \
         flag makes the two invocations incomparable, an unaudited marker can excuse a real \
         divergence, and an unsubstantiated record has no authority to judge anything. So while a \
         gate is unmet this area's PASSes are not evidence of agreement and its divergences are not \
         evidence of a defect. That is why the area fails here rather than reporting a matrix nobody \
         can read.\n\n\
         NO CELL OF THIS AREA WAS RUN. The gate is checked before the first compile, so nothing was \
         built, nothing was classified, and no finding was filed — a finding is a deliverable \
         asserting that the compiler did something, and producing one from a program not shown to be \
         free of undefined behaviour, or from a record whose own review has not completed, would \
         assert something this suite has no standing to assert. The area's report was still written \
         and states the withholding, the gate, the record withheld where one was, and the number of \
         programs held back, so the correction can be checked against what the gate said.\n\n\
         The correction is always in the TEST MATERIAL or the machine, never in the compiler: fix \
         the program the audit names, correct the marker or its register entry, complete the \
         record's review and retire its entry from the driver's pending declaration, or install the \
         tool the probe names — then run again. The full gate reports, with every command line and \
         the compiler's own words, are printed by `cargo test --test conformance infra_ -- \
         --nocapture`.",
    );
    text
}

/// Why an incomplete finding artifact fails the run, and what to do about it.
///
/// One message for both scopes that raise it — a feature area and the run summary — because the
/// defect and the correction are identical in either: a report on disk names a deliverable that is
/// not there. The sentences themselves come from the harness, which is the only place that read the
/// directories, so this frames them rather than restating them.
fn artifact_gap(shortfalls: &[String]) -> String {
    let mut text = format!(
        "{} reported finding(s) do not carry their complete artifacts, so a published report names \
         deliverables that are not on disk.\n\n",
        shortfalls.len(),
    );
    for shortfall in shortfalls {
        text.push_str("   ");
        text.push_str(shortfall);
        text.push_str("\n\n");
    }
    text.push_str(
        "Requirement 6 makes a finding a deliverable rather than a defect to patch: the program \
         minimized as far as practical, the outputs from each compiler and each backend, and the \
         exact reproduction commands are what is actually being delivered. A row that says FINDING \
         while its directory is missing one of those promises evidence it does not hold, and a \
         reader who follows the row finds nothing — which is why this fails the run even though \
         FINDING itself never does.\n\n\
         This is a defect in the SUITE, not an observation about the compiler under test, so the \
         correction is never a compiler source change. Check whether something removed or replaced \
         the generated-findings tree while the run was executing — a concurrent `cargo test` sharing \
         this build directory is the usual cause, and each run claims the tree precisely so that is \
         reported rather than tolerated — and whether the run was interrupted between creating a \
         finding's directory and filling it.\n\n\
         The report and the summary were both still written, and each states this in its own \
         diagnostics, so what was observed survives for comparison. Re-run with \
         `BCC_CONFORMANCE_KEEP_WORK=1` to retain every cell workspace beside the findings.",
    );
    text
}

/// Create the build-directory roots every cell writes beneath, and claim this run's report and
/// generated-findings trees.
///
/// Called by every area, and by the two infrastructure tests that perform a preflight gate — the flag
/// probe and the undefined-behaviour audit each allocate workspaces beneath the work root, so a run
/// restricted to them writes there too. Every one of the three halves is idempotent, so no ordering
/// between the sixteen callers is needed.
///
/// The report and findings halves are done here, before the first cell of the first area is compiled,
/// rather than left to the moment an area publishes or a divergence needs filing: clearing the previous
/// run's artifacts and claiming the directories up front means a conflict with a concurrent run is
/// reported in seconds instead of after a full matrix, and it means those directories hold *this* run's
/// artifacts for the whole of this run.
fn prepare_roots() {
    if let Err(error) = sandbox::ensure_roots() {
        panic!(
            "the suite's build-directory roots could not be prepared.\n\n{error}\n\nEvery path \
             the harness writes — every cell workspace, every report, every generated finding — \
             lies beneath one of these roots, so without them there is nowhere it is permitted to \
             write, and it stops here rather than writing somewhere else.",
        );
    }
    if let Err(error) = report::prepare_namespace() {
        panic!(
            "the run's report directory could not be prepared.\n\n{error}\n\nThe report is the \
             only account of a run that outlives the process, and every area file it aggregates is \
             bound to this run's identity. A directory that cannot be cleared and claimed would \
             leave an earlier run's rows in place to be counted, so the run stops here rather than \
             producing a summary describing a matrix it did not execute.",
        );
    }
    if let Err(error) = findings::prepare_namespace() {
        panic!(
            "the run's generated-findings directory could not be prepared.\n\n{error}\n\nA finding \
             is a deliverable, and its directory is named from the divergence itself, so a previous \
             run's set left standing beside this one's would present a divergence this run never \
             reproduced as a current deliverable. Claiming and retiring the directory up front means \
             a conflict with a concurrent run is reported in seconds rather than at the moment a \
             divergence needs filing.",
        );
    }
}

/// Enumerate the area's programs by directory scan, honouring the single-program filter.
///
/// Discovery is a scan rather than a list in this file, so adding a program to the corpus needs no
/// change here. An area that cannot be enumerated is a corpus gap and fails, but its report is
/// written first so the deliverable still shows the gap.
fn select_programs(spec: &'static AreaSpec, caps: &Capabilities) -> Vec<PathBuf> {
    let discovered = match manifest::discover_area(spec.directory()) {
        Ok(discovered) => discovered,
        Err(error) => {
            publish(
                spec,
                &[],
                caps,
                &[String::from(concat!(
                    "⚠️ NO CELL OF THIS AREA WAS RUN: its corpus could not be enumerated, so",
                    " there was no program to compile. The gap itself is stated in full by the",
                    " failure this report accompanies.",
                ))],
            );
            panic!("{}", corpus_gap(spec, Some(&error)));
        }
    };

    let Some(filter) = caps.config().only() else {
        return discovered;
    };

    let mut selected: Vec<PathBuf> = Vec::new();
    for path in discovered {
        let (_, program) = corpus_identity(&path);
        if filter.matches(spec.directory(), &program) {
            selected.push(path);
        }
    }

    // A filter naming *this* area must select exactly one program, and the harness explains at
    // length why matching nothing fails rather than shrinking the run. A filter naming another area
    // deliberately excludes this one, which is partial coverage rather than a failure.
    if filter.area() == spec.directory() {
        if let Err(error) = filter.require_match(selected.len()) {
            publish(
                spec,
                &[],
                caps,
                &[format!(
                    concat!(
                        "⚠️ NO CELL OF THIS AREA WAS RUN: the {} filter names this area but",
                        " selected no program in it, so the run this report describes was never",
                        " the run that was asked for.",
                    ),
                    discovery::VAR_ONLY
                )],
            );
            panic!("{error}");
        }
    }
    selected
}

/// Run every cell of one program that this run's matrix retains.
fn run_program(caps: &Capabilities, source: &Path) -> Vec<Outcome> {
    let record = match manifest::load_for_source(source) {
        Ok(record) => record,
        Err(error) => return vec![corpus_defect(source, &error)],
    };
    let declared = match record.cells() {
        Ok(declared) => declared,
        Err(error) => return vec![corpus_defect(source, &error)],
    };
    let (targets, opt_levels) = caps.config().effective_matrix();

    let letters: String = record
        .enabled_oracles()
        .iter()
        .map(|oracle| oracle.letter())
        .collect();
    // The narrowings are printed rather than left implicit: a program that restricts its targets,
    // its levels or its oracles is exercising a feature while excluding one comparison, and
    // requirement 5 forbids that being invisible.
    let mut narrowings: Vec<String> = Vec::new();
    if record.is_target_restricted() {
        narrowings.push(format!(
            "targets restricted to {}",
            record
                .targets()
                .iter()
                .map(|target| target.short_name())
                .collect::<Vec<&str>>()
                .join("/"),
        ));
    }
    if record.is_opt_level_restricted() {
        narrowings.push(format!(
            "levels restricted to {}",
            record
                .opt_levels()
                .iter()
                .map(|opt| opt.flag())
                .collect::<Vec<&str>>()
                .join("/"),
        ));
    }
    for oracle in record.disabled_oracles() {
        narrowings.push(format!("{} excluded by record", oracle.label()));
    }
    if let Some(marker) = record.marker() {
        narrowings.push(format!("expected divergence {}", marker.id()));
    }

    println!(
        " {}/{}: {} declared cell(s), oracles [{letters}]{}",
        record.area(),
        record.program(),
        declared.len(),
        if narrowings.is_empty() {
            String::new()
        } else {
            format!(" — {}", narrowings.join("; "))
        },
    );

    let mut outcomes: Vec<Outcome> = Vec::new();
    for opt in &opt_levels {
        // The authority is per optimization level: comparing a target built at one level against a
        // baseline built at another would compare two different programs and call the difference a
        // backend defect.
        let mut baseline: Option<Baseline> = None;
        for target in ordered_targets(&targets) {
            let Some(key) = declared
                .iter()
                .find(|key| key.target() == target && key.opt() == *opt)
            else {
                continue;
            };
            outcomes.extend(run_cell(caps, &record, key, &mut baseline));
        }
    }
    outcomes
}

/// Decide one cell: one program, one target, one optimization level.
fn run_cell(
    caps: &Capabilities,
    record: &Manifest,
    key: &CellKey,
    baseline: &mut Option<Baseline>,
) -> Vec<Outcome> {
    let oracles = applicable_oracles(key.target());

    // Nothing can be observed on a target this machine cannot execute. Reported per oracle as
    // unavailable, never as a pass and never as a skip.
    let Some(execution) = caps.execution_for(key.target()) else {
        if key.target() == Target::BASELINE {
            *baseline = Some(Baseline::Unrunnable(format!(
                "this machine cannot execute {} artifacts, so the cross-backend authority could \
                 not be established; the capability report names the runner and its override \
                 variable",
                key.target().triple(),
            )));
        }
        return oracles
            .iter()
            .map(|oracle| classify::unavailable_oracle(caps, key, *oracle))
            .collect();
    };

    match CellPlan::assemble(caps, record, key, &execution) {
        Ok(plan) => plan.decide(baseline),
        Err(error) => internal_error_outcomes(key, &oracles, &error),
    }
}

/// The oracles that can apply to a target at all.
///
/// Only the cross-backend oracle is ever inapplicable, and only on the baseline target, which
/// cannot be compared against itself.
fn applicable_oracles(target: Target) -> Vec<Oracle> {
    Oracle::ALL
        .iter()
        .copied()
        .filter(|oracle| classify::oracle_applies(*oracle, target))
        .collect()
}

/// The baseline first, so the authority for oracle (b) exists before anything is compared to it.
fn ordered_targets(targets: &[Target]) -> Vec<Target> {
    let mut ordered: Vec<Target> = Vec::with_capacity(targets.len());
    if targets.contains(&Target::BASELINE) {
        ordered.push(Target::BASELINE);
    }
    for target in targets {
        if *target != Target::BASELINE {
            ordered.push(*target);
        }
    }
    ordered
}

/// One failing row per applicable oracle when the harness itself failed on a cell.
///
/// Recorded rather than raised, so one broken cell does not deprive the area of every other
/// program's verdict — and it still fails the run, because the row is a failure.
fn internal_error_outcomes(
    key: &CellKey,
    oracles: &[Oracle],
    error: &HarnessError,
) -> Vec<Outcome> {
    oracles
        .iter()
        .map(|oracle| classify::internal_error(key, *oracle, error))
        .collect()
}

/// A failing row for a program whose own record could not be read.
///
/// A `.c` file with no readable sibling `.expected` is a corpus defect, never a skip: without the
/// record there is no golden stdout, no matrix, no marker and no reproduction command, so the
/// program cannot take part in any oracle.
fn corpus_defect(source: &Path, error: &HarnessError) -> Outcome {
    let (area, program) = corpus_identity(source);
    match CellKey::new(area, program, Target::BASELINE, OptLevel::O0) {
        Ok(key) => classify::internal_error(&key, Oracle::GoldenRecord, error),
        Err(cause) => panic!(
            "the corpus file {} cannot be identified as an area and a program, so its own failure \
             could not even be reported under a name.\n\n{cause}\n\nThe original failure was: \
             {error}",
            shown_path(source),
        ),
    }
}

/// The area and program halves of a corpus path.
///
/// Both are empty when a path is not the canonical `<area>/<program>.c` shape, which
/// [`CellKey::new`] then rejects with an explanatory failure rather than inventing a name.
fn corpus_identity(source: &Path) -> (String, String) {
    let program = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
    let area = source
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    (area, program)
}

/// Write this area's report and, when every area this invocation selected has one, finalize the run
/// summary.
///
/// Finalization is an order-independent check-and-write called by every area: every caller but the
/// last finds the set incomplete and does nothing, and the last one aggregates. That is what lets
/// the summary exist without an extra test to write it, which would have changed the suite's test
/// count.
///
/// The set waited for is the set *this invocation* can produce, not the full fourteen. An unfiltered
/// run therefore aggregates all fourteen and reports full coverage, while a filtered or reduced run
/// still produces a summary — labelled partial, naming what it covered and what it did not. The
/// summary is a named deliverable, so a run that swept a subset must still say what it swept rather
/// than publish nothing at all.
fn publish(spec: &'static AreaSpec, outcomes: &[Outcome], caps: &Capabilities, notes: &[String]) {
    if let Err(error) = report::write_area(spec.directory(), outcomes, caps, notes) {
        panic!(
            "the report for the feature area {:?} could not be written.\n\n{error}\n\nThe per-area \
             report and the run summary are this suite's deliverable — the areas covered, every \
             outcome, every expected divergence with its documented basis and every finding with \
             its reproducer — so failing to write one is a failure of the run rather than a \
             cosmetic loss. The accumulated verdicts are printed above.",
            spec.directory(),
        );
    }

    match report::try_finalize(caps) {
        Ok(true) => {
            println!(
                " run summary written:\n   {}\n   {}",
                shown_path(&report::summary_markdown_path()),
                shown_path(&report::summary_tsv_path()),
            );
            // Asserted in the arm that wrote the summary, and only there, so the caller that fails
            // is the caller that made the promise. The check itself ran inside the harness,
            // immediately before the summary's bytes were written; this is where its answer is
            // turned into the outcome of a test. Placed after the write for the reason the whole
            // file is ordered this way — a summary withheld on account of an incomplete finding
            // would delete the record of which finding was incomplete.
            let shortfalls = report::artifact_shortfalls(report::SUMMARY_SCOPE);
            assert!(shortfalls.is_empty(), "{}", artifact_gap(&shortfalls));
        }
        // Never left as a bare "pending": the harness states which areas it is still waiting for,
        // and names any area report it REFUSED because that report belongs to a different run —
        // the one case where a missing summary needs acting on rather than merely noting. Pending
        // means only that an area this invocation will still run has not filed its report yet; a
        // narrowed run does not sit pending forever, because the summary is written over the areas
        // the invocation selected and labelled partial when a filter or a reduced matrix narrowed
        // them.
        Ok(false) => println!(
            "  run summary not written — {}",
            report::finalization_pending()
        ),
        Err(error) => panic!(
            "the run summary could not be finalized.\n\n{error}\n\nThe summary is the artifact the \
             requirements ask for by name; a run that cannot produce it has not delivered its \
             result, so this fails rather than passing quietly with the per-area reports alone.",
        ),
    }
}

/// The area's full account: what ran, every verdict, and where the artifacts are.
///
/// Printed on every run and reproduced verbatim in the failure message, so the outcome table
/// survives whether or not output is captured.
fn area_digest(
    spec: &'static AreaSpec,
    programs: &[PathBuf],
    outcomes: &[Outcome],
    config: &RunConfig,
) -> String {
    let rule = "=".repeat(96);
    let mut text = format!(
        "{rule}\n area {} — differential conformance\n{rule}\n",
        spec.directory(),
    );

    text.push_str(&format!(
        " programs:  {} enumerated of {} declared for this area\n",
        programs.len(),
        spec.program_count(),
    ));
    if programs.len() < spec.program_count() {
        text.push_str(&format!(
            " NOTE:      this area is short of its declared corpus{}. The shortfall is reported \
             rather than passed over: requirement 2 makes the area part of the coverage floor.\n",
            if spec.mandated() {
                format!(
                    ", and it is one of the areas requirement 2 mandates, with a floor of \
                     {MIN_PROGRAMS_PER_MANDATED_AREA} programs"
                )
            } else {
                String::new()
            },
        ));
    }
    text.push_str(&format!(" outcomes:  {}\n", tally(outcomes)));
    if config.is_reduced_run() {
        text.push_str(&format!(
            " REDUCED:   partial coverage — {}. This report cannot be read as a full run.\n",
            reduction_reason(config),
        ));
    }

    text.push_str(" verdicts:\n");
    if outcomes.is_empty() {
        text.push_str("   (none — no cell of this area was executed)\n");
    }
    for outcome in outcomes {
        text.push_str(&format!("   {outcome}\n"));
    }

    let unexpected = classify::unexpected_successes(outcomes);
    if !unexpected.is_empty() {
        text.push_str(&format!(
            " UNEXPECTED SUCCESS — {} marker(s) no longer describe a divergence:\n",
            unexpected.len(),
        ));
        for outcome in &unexpected {
            text.push_str(&format!("   {outcome}\n"));
        }
        text.push_str(
            "   A marker whose divergence has gone is stale documented knowledge that will \
             mislead the next reader. Retire it in the program's own record and in the register; \
             that is a test-only edit.\n",
        );
    }

    let unavailable = classify::select_by_verdict(outcomes, Verdict::Unavailable);
    if !unavailable.is_empty() {
        text.push_str(&format!(
            " UNAVAILABLE — {} arm(s) could not be attempted in this environment; they are \
             reported, never counted as passes.\n",
            unavailable.len(),
        ));
    }

    let discovered = classify::select_by_verdict(outcomes, Verdict::Finding);
    if !discovered.is_empty() {
        text.push_str(&format!(
            " FINDINGS — {} undocumented divergence(s), written to {}. Findings are deliverables: \
             no compiler source change is made in response to any of them.\n",
            discovered.len(),
            shown_path(&findings_root()),
        ));
    }

    text.push_str(&format!(
        " artifacts: {}\n            {}\n            {}\n{rule}",
        shown_path(&report::area_markdown_path(spec)),
        shown_path(&report::area_tsv_path(spec)),
        shown_path(&work_root()),
    ));
    text
}

/// One line counting every verdict in the closed set, so none can be quietly absent.
fn tally(outcomes: &[Outcome]) -> String {
    let counts: Vec<String> = Verdict::ALL
        .iter()
        .map(|verdict| {
            format!(
                "{} {}",
                verdict.label(),
                classify::select_by_verdict(outcomes, *verdict).len(),
            )
        })
        .collect();
    format!("{} total — {}", outcomes.len(), counts.join(", "))
}

/// Why a run is reduced, in the terms the operator chose.
fn reduction_reason(config: &RunConfig) -> String {
    let (targets, opt_levels) = config.effective_matrix();
    let mut reasons: Vec<String> = Vec::new();
    if config.quick_mode() {
        reasons.push(format!(
            "{} is set, so the matrix is {} target(s) x {} level(s) instead of {TARGET_COUNT} x \
             {OPT_LEVEL_COUNT}",
            discovery::VAR_QUICK,
            targets.len(),
            opt_levels.len(),
        ));
    }
    if let Some(filter) = config.only() {
        reasons.push(format!(
            "{} restricts the run to {filter}",
            discovery::VAR_ONLY,
        ));
    }
    if reasons.is_empty() {
        reasons.push("the effective matrix is narrower than the declared one".to_string());
    }
    reasons.join("; ")
}

/// Decide the area, after every verdict has been accumulated and reported.
fn conclude(
    spec: &'static AreaSpec,
    programs: &[PathBuf],
    outcomes: &[Outcome],
    digest: &str,
    config: &RunConfig,
    gates: &PreflightGates,
) {
    if programs.is_empty() {
        // A filter naming another area excludes this one deliberately; anything else is a gap.
        assert!(config.only().is_some(), "{}", corpus_gap(spec, None));
        println!(
            " area {:?} was deliberately excluded by the {} filter. This run is partial coverage \
             and its reports are stamped as such.",
            spec.directory(),
            discovery::VAR_ONLY,
        );
        return;
    }

    // A backstop, and stated as one. `run_area` withholds a gated area before its first compile and
    // fails there, so in an unmodified run this can never fire — the outcomes below exist precisely
    // because no gate governing this area blocked. It is kept, and kept cheap, because the property it
    // guards is the one requirement 1 rests on: if a future edit were to reorder `run_area` and let a
    // gated area reach its cells, the consequence would be a FINDING artifact filed from a program not
    // shown to be undefined-behaviour-free, and that is a claim the suite has no standing to make.
    // One comparison here is what makes the guarantee structural rather than a property of the order
    // two statements happen to be written in.
    //
    // An area excluded by the program filter returns above without reaching this, and rightly: it
    // produced no comparison, so it has nothing to distrust. A gate that fails there still fails the
    // run, through its own infrastructure test and through the run summary's verdict.
    let blocking = gates.recorded.blocking_area(spec.directory());
    assert!(
        blocking.is_empty(),
        "{}",
        preflight_gap(spec.directory(), &blocking)
    );

    // A finding is a deliverable, and the report this area just published names its artifacts and
    // tells a reader to execute the script inside them. The harness revalidated every one of those
    // directories against disk immediately before writing those bytes — the whole required artifact
    // set and the captures inside it, without following a link — and this is where that answer
    // decides the test. It is separate from the verdict tally below on purpose: FINDING does not fail
    // a run, because a divergence the suite recorded honestly is the deliverable working as intended.
    // An *empty* finding is a different thing entirely — a row that reads as a recorded observation
    // and delivers nothing — and it is a defect in the suite rather than an observation about the
    // compiler, so it fails here where no verdict policy could reach it.
    let shortfalls = report::artifact_shortfalls(spec.directory());
    assert!(shortfalls.is_empty(), "{}", artifact_gap(&shortfalls));

    assert!(
        !outcomes.is_empty() || config.is_reduced_run(),
        "the feature area {:?} enumerated {} program(s) and produced no outcome at all, so it \
         decided nothing while reporting success. Every program declares at least one target and \
         one optimization level, so an empty full run means the declared matrix and this run's \
         effective matrix do not intersect — which would make a green area indistinguishable from \
         a working one.\n\n{digest}",
        spec.directory(),
        programs.len(),
    );

    let unexpected = classify::unexpected_successes(outcomes);
    if !unexpected.is_empty() && !config.xpass_fails_run() {
        println!(
            " WARNING: {} unexpected success(es) were downgraded from failures by {}. That escape \
             hatch is for a marker-retirement window only — retire the stale marker(s) and clear \
             the variable.",
            unexpected.len(),
            discovery::VAR_ALLOW_XPASS,
        );
    }

    let failures = classify::run_failures(outcomes, config);
    assert!(
        failures.is_empty(),
        "the feature area {:?} did not pass: {} of {} outcome(s) fail this run.\n\n{}\n\n{}\n\n{}",
        spec.directory(),
        failures.len(),
        outcomes.len(),
        failures
            .iter()
            .map(|outcome| format!("   {outcome}"))
            .collect::<Vec<String>>()
            .join("\n"),
        digest,
        failing_guidance(config),
    );

    // A finding does not fail the run — it is a deliverable — but a finding whose deliverable is not
    // there does, and that is a different statement. Until now the report noted the loss as a
    // diagnostic and the area still passed, so a run could announce a divergence, name a directory
    // holding nothing, and report success: the one shape of result that is worse than a failure,
    // because it reads as a recorded observation.
    //
    // Checked here, last, so a reader of a broken area sees the real divergences first and this only
    // when nothing else is wrong. Checked against disk while the run that wrote them is still able to
    // say so, and against the writer's own completeness validator rather than a second opinion about
    // what a finding holds.
    let incomplete = incomplete_finding_artifacts(outcomes);
    assert!(
        incomplete.is_empty(),
        "the feature area {:?} recorded {} finding(s) whose artifacts are not a deliverable.\n\n{}\n\
         \nA finding is kept as the program, the outputs from each compiler and backend, and the \
         exact reproduction commands — so a row naming artifacts that are not there is a defect in \
         this suite rather than an observation about the compiler. Nothing beneath the build \
         directory is committed, so the usual cause is an interrupted or concurrently cleaned run: \
         re-run the area. If it recurs, the write path is at fault and the divergence has not been \
         delivered.\n\n{}",
        spec.directory(),
        incomplete.len(),
        incomplete
            .iter()
            .map(|defect| format!("   {defect}"))
            .collect::<Vec<String>>()
            .join("\n"),
        digest,
    );
}

/// Why any finding this area recorded falls short of being a deliverable, in directory order.
///
/// # Why the directories are deduplicated first
///
/// A finding is identified by its cell and its divergence class, not by the oracle that observed it,
/// so one refused build seen by three oracles is three outcomes pointing at **one** directory. Asking
/// each outcome independently would report a single missing artifact three times and make a reader
/// hunt for three directories that are one. The set of directories is therefore collapsed before
/// anything is checked, which also means the disk is read once per finding rather than once per arm.
///
/// The check itself is [`findings::artifact_defect`] — the same validator the write path applies at
/// publication and the reporter applies when it calls a row's artifacts present. One definition of
/// what a complete finding holds, asked by everyone who needs the answer.
fn incomplete_finding_artifacts(outcomes: &[Outcome]) -> Vec<String> {
    let mut directories: BTreeMap<String, (PathBuf, Vec<String>)> = BTreeMap::new();
    for outcome in outcomes {
        if outcome.verdict() != Verdict::Finding {
            continue;
        }
        // A finding with no class cannot name a directory at all. That is an internal inconsistency
        // rather than a lost artifact, and the area report already diagnoses it by name, so it is not
        // re-reported here as a missing deliverable it never had.
        let Some(class) = outcome.class() else {
            continue;
        };
        let id = findings::FindingId::derive(outcome.key(), class);
        let entry = directories
            .entry(String::from(id.as_str()))
            .or_insert_with(|| (id.directory(), Vec::new()));
        entry.1.push(outcome.oracle().to_string());
    }
    directories
        .into_iter()
        .filter_map(|(id, (directory, observers))| {
            findings::artifact_defect(&directory).map(|defect| {
                format!(
                    "finding {id} (observed by {}): {defect}",
                    observers.join(", ")
                )
            })
        })
        .collect()
}

/// What a reader of a failing area should do next, and what they must not do.
fn failing_guidance(config: &RunConfig) -> String {
    let mut text = String::from(
        "how to read this:\n  \
         FAIL        an unexplained divergence, or a harness or corpus defect. The row names the \
         program, target, optimization level and oracle.\n  \
         XPASS       a marker is present but its divergence has gone. Retire the marker in the \
         program's record and in the register.\n  \
         FINDING     an undocumented divergence. Its artifacts hold the reproducer, both sides' \
         output, the environment fingerprint and the exact commands.\n  \
         XFAIL       an expected divergence with a documented basis; it does not fail the run.\n  \
         UNAVAILABLE an oracle whose tooling is absent; reported, never a pass.\n\n\
         Reproduce any cell without this harness by rendering the command templates in the \
         program's own `.expected` record, or by running the `commands.txt` written into the \
         cell's retained workspace.\n\n\
         Findings are deliverables, not defects to patch: no change is made to the compiler's \
         source in response to one.",
    );
    if config.unavailable_fails_run() {
        text.push_str(&format!(
            "\n\n{} is in force, so an unavailable oracle also fails this run: in continuous \
             integration the toolchain is installed deliberately, and a missing arm means a broken \
             workflow rather than a modest machine.",
            discovery::VAR_STRICT,
        ));
    }
    text
}

/// Why an area with no runnable corpus is a failure rather than a skip.
fn corpus_gap(spec: &'static AreaSpec, error: Option<&HarnessError>) -> String {
    let mut text = format!(
        "the feature area {:?} has no runnable corpus.\n\n",
        spec.directory(),
    );
    if let Some(error) = error {
        text.push_str(&format!("{error}\n\n"));
    }
    text.push_str(&format!(
        "This area is expected at {} and to hold {} program(s), each a `.c` file with a sibling \
         `.expected` record naming its targets, its optimization levels, its command templates and \
         its expected stdout.{}\n\nAn area that cannot be enumerated fails rather than being \
         skipped: requirement 2 makes it part of the coverage floor, and a suite that passed over \
         a missing area would report a green run for a feature it never exercised.",
        shown_path(&corpus_root().join(spec.directory())),
        spec.program_count(),
        if spec.mandated() {
            format!(
                " It is one of the areas requirement 2 mandates, with a floor of \
                 {MIN_PROGRAMS_PER_MANDATED_AREA} programs."
            )
        } else {
            String::new()
        },
    ));
    text
}

/// The build matrix this run will actually cover, stated as counts rather than as a percentage.
///
/// No coverage percentage appears anywhere in this suite. Coverage instrumentation would need a
/// development dependency, which this repository forbids absolutely, so any percentage printed here
/// would be a number nobody could verify. The matrix is countable from the committed corpus.
fn matrix_statement(config: &RunConfig) -> String {
    let (targets, opt_levels) = config.effective_matrix();
    let mut text = String::from("build matrix\n");
    text.push_str(&format!(
        "  targets:           {}\n",
        targets
            .iter()
            .map(|target| target.triple())
            .collect::<Vec<&str>>()
            .join(", "),
    ));
    text.push_str(&format!(
        "  optimization:      {}\n",
        opt_levels
            .iter()
            .map(|opt| opt.flag())
            .collect::<Vec<&str>>()
            .join(", "),
    ));
    text.push_str(&format!("  feature areas:     {AREA_COUNT}\n"));
    // "planned", and the corpus inventory printed immediately after this block states how many
    // programs and records are actually present. The two were previously one number in one place,
    // which is how a declared count came to be read as a description of the tree.
    text.push_str(&format!(
        "  programs planned:  {PROGRAM_COUNT} (the plan; the corpus inventory below states what is \
         present)\n"
    ));
    text.push_str(&format!(
        "  per-cell timeout:  {} s\n",
        config.timeout_secs(),
    ));
    if config.is_reduced_run() {
        text.push_str(&format!(
            "  REDUCED RUN:       {}\n",
            reduction_reason(config),
        ));
    } else {
        text.push_str(&format!(
            "  full matrix:       {BCC_CELL_COUNT} compile-and-run cells under test \
             ({PROGRAM_COUNT} programs x {TARGET_COUNT} targets x {OPT_LEVEL_COUNT} levels), \
             {ORACLE_A_COMPARISON_COUNT} oracle (a), {ORACLE_B_COMPARISON_COUNT} oracle (b) and \
             {ORACLE_C_ASSERTION_COUNT} oracle (c) comparisons — {TOTAL_ASSERTION_COUNT} \
             assertions\n",
        ));
    }
    // Rendered through `shown_path` rather than `Path::display`, because all three roots descend
    // from the build directory and `CARGO_TARGET_DIR` can move that directory. A value carrying an
    // escape introducer or a directional override would otherwise repaint this pre-flight block —
    // the one block a maintainer reads to learn where the run is writing.
    text.push_str(&format!(
        "  workspaces:        {}\n  reports:           {}\n  findings:          {}\n",
        shown_path(&work_root()),
        shown_path(&report_root()),
        shown_path(&findings_root()),
    ));
    // A rejected `CARGO_TARGET_DIR` is stated here rather than swallowed. The build roots fall back
    // to the manifest-relative default when the variable cannot be trusted, so the run continues
    // correctly — but it continues writing somewhere other than where the caller asked, and a
    // maintainer who set that variable deliberately has to be told why it was ignored.
    if let Some(rejection) = target_dir_rejection() {
        text.push_str(&format!("  CARGO_TARGET_DIR:  ignored — {rejection}\n"));
    }
    text
}

/// The token a register entry writes for an optional marker key the record does not carry.
///
/// An entry states all eight fields whether or not the marker carries all eight keys, because a table
/// with a row missing is indistinguishable from a table an author forgot to finish. Two of the keys
/// are optional (§2.1 of the register), so their rows need a way to say "the record does not carry
/// this" that a reader and the audit both recognise — this token is it. An empty cell is accepted
/// too, for an author who prefers one.
const REGISTER_ENTRY_UNWRITTEN: &str = "(not written)";

/// The eight fields a register entry must state, in the order the audit reports them.
///
/// Held in one place so that the parser, the comparison and the diagnostics cannot disagree about
/// what an entry consists of. `Program` is included because an entry that named the wrong program
/// would send a reader to a construct the marker never governed, and `Observed` because an entry
/// that omitted it would describe an authority without describing what it excuses.
const REGISTER_ENTRY_FIELDS: [&str; 8] = [
    "Identifier",
    "Class",
    "Scope",
    "Program",
    "Basis",
    "Documented",
    "Evidence",
    "Observed",
];

/// One structured entry parsed out of the expected-divergence register.
///
/// The register is a committed deliverable rather than a generated artifact, so it is prose with a
/// small, checkable skeleton inside it: a heading naming the marker, then a two-column table whose
/// left column names a field. Everything the audit compares comes from that table, and the heading
/// line is retained so a diagnostic can point a maintainer at the entry rather than at the file.
#[derive(Debug)]
struct RegisterEntry {
    identifier: String,
    heading_line: usize,
    fields: Vec<(String, String)>,
}

impl RegisterEntry {
    /// The value the entry states for `field`, if it states it at all.
    fn value(&self, field: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str())
    }
}

/// Every structured entry the register contains, in document order.
///
/// An entry begins at any Markdown heading whose text contains an `XD-` identifier and ends at the
/// next heading of any level, so a section cannot silently absorb the tables of the one after it.
/// Inside that span every two-column table row whose left cell names one of
/// [`REGISTER_ENTRY_FIELDS`] contributes a field; every other row, and all surrounding prose, is
/// ignored, because an entry is allowed to explain itself at whatever length the divergence
/// deserves.
///
/// A `|` inside a value is written `\|`, as a Markdown table requires, and is unescaped here so the
/// comparison sees the value the author meant. A value wrapped in a single pair of backticks is
/// unwrapped for the same reason: the surrounding pair is this document's way of rendering a literal,
/// not part of the literal.
fn parse_register_entries(register: &str) -> Vec<RegisterEntry> {
    let lines: Vec<&str> = register.lines().collect();
    // Fenced blocks are excluded from the structure entirely, in one pre-pass rather than in each
    // scan below. The register documents its own entry shape inside a fence, so a `#` line there is
    // an illustration: counting it as a heading would end the entry it appears in, and counting a
    // row there as a field would compare a marker against a template.
    let fenced = fenced_lines(&lines);
    let is_heading = |index: usize| !fenced[index] && lines[index].trim_start().starts_with('#');

    let mut entries: Vec<RegisterEntry> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !is_heading(index) {
            continue;
        }
        let heading = line.trim_start().trim_start_matches('#');
        let Some(identifier) = first_identifier(heading) else {
            continue;
        };
        entries.push(RegisterEntry {
            identifier,
            heading_line: index + 1,
            fields: Vec::new(),
        });
    }
    // A second pass fills each entry from the lines between its heading and the next one. Doing it
    // this way rather than inside the first pass keeps the span rule in one expression — an entry
    // owns everything up to the next heading — instead of spread across a running state machine.
    for entry in &mut entries {
        let start = entry.heading_line;
        let end = (start..lines.len())
            .find(|index| is_heading(*index))
            .unwrap_or(lines.len());
        let mut fields: Vec<(String, String)> = Vec::new();
        for index in start..end {
            if fenced[index] {
                continue;
            }
            if let Some((name, value)) = parse_register_row(lines[index]) {
                if REGISTER_ENTRY_FIELDS.contains(&name.as_str())
                    && !fields.iter().any(|(existing, _)| existing == &name)
                {
                    fields.push((name, value));
                }
            }
        }
        entry.fields = fields;
    }
    entries
}

/// Which lines lie inside a fenced code block, fence lines included.
///
/// A fence is three or more backticks or tildes at the start of a line, and a fence of one character
/// does not close a fence of the other — which is what lets the register show a backtick fence inside
/// a tilde fence, or the reverse, without the structure of the document changing underneath it.
fn fenced_lines(lines: &[&str]) -> Vec<bool> {
    let mut inside: Vec<bool> = Vec::with_capacity(lines.len());
    let mut opener: Option<char> = None;
    for line in lines {
        let trimmed = line.trim_start();
        let fence = ['`', '~']
            .into_iter()
            .find(|character| trimmed.starts_with(&character.to_string().repeat(3)));
        match (opener, fence) {
            (None, Some(character)) => {
                opener = Some(character);
                inside.push(true);
            }
            (Some(open), Some(character)) if open == character => {
                opener = None;
                inside.push(true);
            }
            (state, _) => inside.push(state.is_some()),
        }
    }
    inside
}

/// One `| Field | value |` row, reduced to its two cells.
///
/// `None` for anything that is not a two-cell row: prose, a fence, a separator row, or a table with
/// a different shape. The left cell is stripped of the emphasis markers a register author may use to
/// make the field name stand out, so `| **Basis** | ... |` and `| Basis | ... |` are the same row.
fn parse_register_row(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 2 {
        return None;
    }
    // Split on unescaped pipes only, so a value containing `\|` stays one cell.
    let mut cells: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in trimmed[1..trimmed.len() - 1].chars() {
        match (escaped, character) {
            (true, '|') => {
                current.push('|');
                escaped = false;
            }
            (true, other) => {
                current.push('\\');
                current.push(other);
                escaped = false;
            }
            (false, '\\') => escaped = true,
            (false, '|') => {
                cells.push(current.clone());
                current.clear();
            }
            (false, other) => current.push(other),
        }
    }
    if escaped {
        current.push('\\');
    }
    cells.push(current);
    if cells.len() != 2 {
        return None;
    }
    let name = cells[0].trim().trim_matches('*').trim().to_string();
    let value = unwrap_code_span(cells[1].trim());
    if name.is_empty() {
        return None;
    }
    Some((name, value))
}

/// `text` without one surrounding pair of backticks, if it carries one.
///
/// The register renders a literal — a scope, a class, a path — as a code span so it reads correctly,
/// while the program's record holds the bare literal. Unwrapping exactly one pair keeps the two
/// comparable without letting a value that legitimately contains a backtick be altered.
fn unwrap_code_span(text: &str) -> String {
    let inner = text
        .strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .filter(|inner| !inner.contains('`'));
    match inner {
        Some(inner) => inner.trim().to_string(),
        None => text.to_string(),
    }
}

/// The first `XD-` identifier in `text`, if it holds one.
///
/// Shares the tokenizer with [`registered_identifiers`] so that a heading the reverse check sees an
/// identifier in is an entry the forward check can also find. Two different notions of "contains an
/// identifier" is exactly how one direction of the audit would start passing while the other failed.
fn first_identifier(text: &str) -> Option<String> {
    identifier_tokens(text).into_iter().next()
}

/// Every `XD-` identifier in `text`, in order, with duplicates retained.
fn identifier_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
    })
    .filter_map(|token| {
        let candidate = token.trim_matches('-');
        match candidate.starts_with("XD-") && candidate.len() > "XD-".len() {
            true => Some(candidate.to_string()),
            false => None,
        }
    })
    .collect()
}

/// Every `XD-` identifier the register mentions, deduplicated and ordered.
///
/// Deliberately tolerant of the surrounding markup: identifiers are whitespace-free by
/// construction, so splitting on all that cannot appear inside one finds them whether it spells
/// them in a table cell, a heading, a list item or a code span. This is the reverse direction of the
/// audit, and its tolerance is the point — an identifier written anywhere in the document, including
/// in prose or as a leftover from a retirement, must resolve to a live marker or fail the run.
fn registered_identifiers(register: &str) -> Vec<String> {
    let mut identifiers: Vec<String> = Vec::new();
    for identifier in identifier_tokens(register) {
        if !identifiers.contains(&identifier) {
            identifiers.push(identifier);
        }
    }
    identifiers.sort();
    identifiers
}

/// Every way in which the register's entry for `marker` fails to describe it.
///
/// Empty when the entry states all eight fields and every one agrees with the record. A mismatch is
/// reported per field, with both readings quoted, so a maintainer sees which document is wrong rather
/// than only that the two disagree.
///
/// Comparison is exact after trimming for the five single-line fields, because §2.4 of the register
/// requires one canonical rendering reproduced character for character. `Documented`, `Evidence` and
/// `Observed` are compared with runs of whitespace collapsed, and only because a Markdown table cell
/// cannot contain a newline while those three fields are heredocs that frequently do: the alternative
/// would be a rule no author could satisfy, which is a rule that ends up unenforced.
fn register_entry_mismatches(
    marker: &manifest::ExpectedDivergence,
    entry: &RegisterEntry,
) -> Vec<String> {
    let mut mismatches: Vec<String> = Vec::new();
    let program = marker.program_label();
    let expected: [(&str, &str); 8] = [
        ("Identifier", marker.id()),
        ("Class", marker.class().label()),
        ("Scope", marker.scope().raw()),
        ("Program", &program),
        ("Basis", marker.basis()),
        ("Documented", marker.documented()),
        ("Evidence", marker.evidence()),
        ("Observed", marker.observed()),
    ];
    for (field, recorded) in expected {
        let Some(stated) = entry.value(field) else {
            mismatches.push(format!(
                "its entry at line {} states no {field} field; the eight fields {} are each required, \
                 because an entry that omits one describes an authority the record does not, and \
                 the two accounts then differ in a way no reader can reconcile. The record says \
                 {}",
                entry.heading_line,
                comma_list(&REGISTER_ENTRY_FIELDS),
                quoted_for_diagnostic(recorded),
            ));
            continue;
        };
        let agrees = match field {
            // The two optional keys may be absent from the record. An entry still states their rows,
            // and states them as unwritten — either literally empty or with the register's own token
            // — so a reader can tell "the record does not carry this" apart from "somebody stopped
            // filling in the table".
            _ if recorded.trim().is_empty() => {
                stated.trim().is_empty() || stated.trim() == REGISTER_ENTRY_UNWRITTEN
            }
            "Documented" | "Evidence" | "Observed" => {
                collapse_whitespace(stated) == collapse_whitespace(recorded)
            }
            _ => stated.trim() == recorded.trim(),
        };
        if !agrees {
            mismatches.push(format!(
                "its entry at line {} states {field} as {} while the record says {}; §2.4 of the \
                 register requires one canonical rendering reproduced character for character, \
                 because two slightly different accounts of the same authority leave the next \
                 reader to decide which one is the marker",
                entry.heading_line,
                quoted_for_diagnostic(stated),
                quoted_for_diagnostic(recorded),
            ));
        }
    }
    mismatches
}

/// Every way in which the document a marker cites fails to be a basis a reader can check.
///
/// Four properties are asserted, and each closes a distinct way a basis can be hollow:
///
/// The three together establish that a citation can be FOLLOWED: the document exists inside this
/// repository, it is readable, and the section the marker names can be located inside it. They do not
/// establish that the section supports the claim — that is a reviewer's judgement, and the frozen
/// marker contract deliberately leaves it to one, because one of the two mandated markers rests on an
/// inventory's silence, which no automated check can weigh. `XD-GCCEXT-CASE-RANGES-001` is that
/// marker, and the register states its basis's weakness in prose where a reviewer will read it. What the audit guarantees is that the
/// reviewer has somewhere concrete to look.
///
/// The optional `expected_divergence.documented` key exists for an author who can do better than a
/// citation. When it is written, the quotation must occur INSIDE the region the locator resolved to —
/// not merely somewhere in the same file, which is a check a marker could satisfy while citing one
/// section and quoting another. See [`documented_quotation_violation`].
///
/// - **Containment.** The cited path is resolved and required to lie beneath the package root, so a
///   marker cannot reclassify a divergence on the authority of something outside this repository.
///   Containment is decided on the fully resolved path, so no symbolic link along the way changes
///   the answer.
/// - **A real, readable, bounded document.** The document is read through
///   [`conformance_harness::read_file_bounded`], which refuses a symbolic link, a device node or a
///   FIFO at the final component, proves the opened handle is the entry it inspected, and refuses a
///   file past the inspection ceiling. The previous shape of this check — `Path::is_file` — followed
///   a link and asserted only existence, so a basis could point through a link at anything readable
///   and the check would still pass.
/// - **A locator that resolves.** The citation half must name something findable *inside* the
///   document, and everything it names must be found. A citation that resolves to nothing is a
///   citation a reader cannot check, which is the one thing a documented basis may not be.
fn basis_violations(marker: &manifest::ExpectedDivergence) -> Vec<String> {
    let context = format!("auditing the documented basis of marker {}", marker.id());
    let absolute = marker.basis_absolute_path();
    if let Err(error) = conformance_harness::ensure_within(&context, &manifest_dir(), &absolute) {
        return vec![format!(
            "marker {} cites the basis {:?}, which does not resolve to a document inside this \
             repository: {error}. A marker reclassifies a failure on the authority of something \
             this repository documents, so a basis that leaves the repository is no authority at \
             all",
            marker.id(),
            marker.basis(),
        )];
    }
    let bytes = match conformance_harness::read_file_bounded(
        &context,
        &absolute,
        conformance_harness::MAX_INSPECTED_FILE_BYTES,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            return vec![format!(
                "marker {} cites the basis {:?}, which resolves to {} and could not be read as a \
                 committed document: {error}. Existence alone was never the property that matters — \
                 the audit reads the document so that the section the marker cites can be resolved \
                 inside it",
                marker.id(),
                marker.basis(),
                shown_path(&absolute),
            )]
        }
    };
    let document = String::from_utf8_lossy(&bytes);
    match resolve_locators(marker.basis_citation(), &document) {
        Ok(resolution) => {
            if let Some(violation) =
                documented_quotation_violation(marker, &document, &absolute, &resolution)
            {
                return vec![violation];
            }
            println!(
                "  basis of {} resolved in {}: {}{}",
                marker.id(),
                shown_path(&absolute),
                comma_list(&resolution.descriptions),
                match marker.documented().trim().is_empty() {
                    // Stated rather than left to inference: the optional key was not written, so
                    // nothing about a documenting sentence has been established, and a line claiming
                    // one occurs would be the audit reporting a check it never made.
                    true => String::from(
                        concat!(
                            " — no documenting quotation was supplied, so the audit establishes",
                            " that the citation resolves and nothing about what the cited section",
                            " says",
                        ),
                    ),
                    false => format!(
                        " — and its documenting sentence occurs verbatim {}",
                        resolution.span_label()
                    ),
                }
            );
            Vec::new()
        }
        Err(reason) => vec![format!(
            "marker {} cites {:?} in {}, but {reason}. A basis names a section a reader can turn to: \
             write a line number (`line 246`), a line range (`lines 696-725`), a section number \
             (`§0.6.2`) or a backtick-quoted phrase from the document, so that the citation can be \
             followed rather than taken on trust",
            marker.id(),
            marker.basis_citation(),
            shown_path(&absolute),
        )],
    }
}

/// Why a marker's documenting quotation is not a quotation of the CITED SECTION, or `None`.
///
/// Absent when the optional `expected_divergence.documented` key is not written: the frozen marker
/// contract does not require it, so its absence is not a violation. What is a violation is writing
/// one that cannot be found where the marker says to look.
///
/// # Why the search is bounded to the cited range rather than to the whole document
///
/// This check previously searched the entire file, and that made it possible to satisfy while
/// defeating its own purpose: a marker could cite one section, quote a sentence from a completely
/// unrelated part of the same document, and pass — so the audit certified that the words were the
/// document's own while establishing nothing about the section a reader was sent to. Since the basis
/// must carry a locator anyway, and the locator resolves to concrete positions, the quotation is now
/// required to occur INSIDE the union of those positions. The two halves of a citation then have to
/// agree with each other, which is the whole point of asking for both.
///
/// A citation may resolve to several positions — a line, a range, a section, a quoted phrase — and any
/// one of them satisfying the search is enough: an author who cites two sections is not required to
/// have the sentence in both.
///
/// The quotation is compared with runs of whitespace collapsed, and that is the only latitude given:
/// a Markdown document wraps its lines wherever its own formatting demands, so requiring the
/// quotation to match the file's line breaks would be requiring the author to reproduce an accident.
/// Everything else must match — every word, in order — because the whole value of the check is that a
/// reader can find the sentence and judge whether it says what the marker claims. A quotation
/// spanning several lines in the record is normalised the same way, so an author may wrap it for
/// legibility.
fn documented_quotation_violation(
    marker: &manifest::ExpectedDivergence,
    document: &str,
    absolute: &Path,
    resolution: &LocatorResolution,
) -> Option<String> {
    if marker.documented().trim().is_empty() {
        return None;
    }
    let quotation = collapse_whitespace(marker.documented());
    let lines: Vec<&str> = document.lines().collect();
    if resolution
        .spans
        .iter()
        .any(|span| collapse_whitespace(&span.text(&lines)).contains(&quotation))
    {
        return None;
    }
    let occurs_elsewhere = collapse_whitespace(document).contains(&quotation);
    Some(format!(
        "marker {} quotes {} as the sentence documenting the limitation, but that text does not \
         occur inside the section its basis cites — {} in {}{}. A path, a readable file and a \
         locator that resolves establish only that a document exists and that a line is inside it; \
         they establish nothing about whether the section says what the marker claims, and a \
         quotation taken from somewhere else in the same document establishes nothing about it \
         either. Copy the sentence that states the limitation FROM THE SECTION YOU CITED, exactly, \
         allowing only a change of line wrapping — or widen the citation to the section the sentence \
         is really in. If no sentence in the repository states it, leave the key out: the contract \
         does not require it, and a divergence nothing documents is a FINDING — a deliverable with a \
         reproducer and exact commands — rather than something to be excused here",
        marker.id(),
        quoted_for_diagnostic(marker.documented()),
        comma_list(&resolution.descriptions),
        shown_path(absolute),
        match occurs_elsewhere {
            true => ". The text does occur elsewhere in that document, which is why the search is \
                     bounded: quoting an unrelated section would otherwise pass"
                .to_string(),
            false => String::new(),
        }
    ))
}

/// Resolve every locator a citation contains against the cited document.
///
/// `Ok` carries a [`LocatorResolution`]: a description of each locator that resolved, for the audit's
/// own output — a maintainer reading a passing run sees which sections were actually checked, not
/// merely that something was — and the concrete document region each one names, which is what bounds
/// the documenting-quotation check to the section the marker cited. `Err` carries the first reason the
/// citation cannot be followed.
///
/// Three locator forms are recognised, and they are the three a citation in this repository naturally
/// uses, because §7 of the register already writes bases this way:
///
/// - `line N` and `lines N-M` (any dash, including an en dash) — `N` and `M` must be real line
///   numbers of the document, and a range must not run backwards;
/// - `§X` — the document must carry a Markdown heading whose text begins with `X`;
/// - a backtick-quoted phrase — it must occur in the document verbatim.
///
/// **Every** locator present must resolve, and **at least one** must be present. Requiring all of
/// them is what stops a correct locator from carrying an incorrect one alongside it; requiring one is
/// what stops a citation from being unfalsifiable prose.
fn resolve_locators(citation: &str, document: &str) -> Result<LocatorResolution, String> {
    let lines: Vec<&str> = document.lines().collect();
    let mut resolution = LocatorResolution::default();

    for phrase in backtick_phrases(citation) {
        // Located as well as found: the line the phrase occurs on becomes the span, so a quotation
        // bound to this locator has to sit in the same neighbourhood as the phrase that named it.
        let Some(index) = lines.iter().position(|line| line.contains(phrase.as_str())) else {
            return Err(format!(
                "the quoted phrase `{phrase}` does not occur in that document"
            ));
        };
        resolution.push(
            format!("phrase `{phrase}`"),
            DocumentSpan::around(index + 1, lines.len()),
        );
    }

    for section in section_locators(citation) {
        let heading = lines.iter().position(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('#') && trimmed.trim_start_matches('#').trim_start() == section
                || trimmed.starts_with('#')
                    && trimmed
                        .trim_start_matches('#')
                        .trim_start()
                        .starts_with(&format!("{section} "))
        });
        let Some(heading) = heading else {
            return Err(format!(
                "that document carries no heading for section {section}"
            ));
        };
        // A section's span runs from its heading to the line before the next heading of any level,
        // so quoting inside it means quoting inside the section a reader would actually read.
        let end = (heading + 1..lines.len())
            .find(|index| lines[*index].trim_start().starts_with('#'))
            .unwrap_or(lines.len());
        resolution.push(
            format!("section {section}"),
            DocumentSpan {
                first: heading + 1,
                last: end.max(heading + 1),
            },
        );
    }

    for (first, last) in line_locators(citation) {
        if first == 0 || last < first {
            return Err(format!(
                "the line reference {first}-{last} is not a range that can be read"
            ));
        }
        if last > lines.len() {
            return Err(format!(
                "the line reference {first}-{last} runs past the end of that document, which has \
                 {} line(s)",
                lines.len()
            ));
        }
        let description = match first == last {
            true => format!("line {first}"),
            false => format!("lines {first}-{last}"),
        };
        // A single-line citation is widened by one line either side, and only that far: a Markdown
        // table row or a bullet frequently wraps, so a sentence cited by its opening line may finish
        // on the next one, while a window any wider would start absorbing neighbouring claims.
        let span = match first == last {
            true => DocumentSpan::around(first, lines.len()),
            false => DocumentSpan { first, last },
        };
        resolution.push(description, span);
    }

    match resolution.descriptions.is_empty() {
        true => Err(String::from(
            "that citation carries no locator the audit can resolve inside the document",
        )),
        false => Ok(resolution),
    }
}

/// A citation resolved against the document it cites: what was found, and where.
///
/// The `where` half is what lets the documenting quotation be checked against the SECTION the marker
/// cites rather than against the whole file. Both vectors are parallel and are pushed together, so a
/// description and the span it describes cannot drift apart.
#[derive(Debug, Default)]
struct LocatorResolution {
    /// Each locator as the audit reports it, for the passing run's own output.
    descriptions: Vec<String>,
    /// The document region each locator resolved to, one-based and inclusive.
    spans: Vec<DocumentSpan>,
}

impl LocatorResolution {
    /// Record one resolved locator and the region it names.
    fn push(&mut self, description: String, span: DocumentSpan) {
        self.descriptions.push(description);
        self.spans.push(span);
    }

    /// The resolved regions as one phrase, for a diagnostic or a progress line.
    fn span_label(&self) -> String {
        let spans: Vec<String> = self
            .spans
            .iter()
            .map(|span| match span.first == span.last {
                true => format!("at line {}", span.first),
                false => format!("within lines {}-{}", span.first, span.last),
            })
            .collect();
        match spans.is_empty() {
            true => String::from("in the cited document"),
            false => spans.join(" or "),
        }
    }
}

/// A one-based, inclusive region of a document.
#[derive(Debug, Clone, Copy)]
struct DocumentSpan {
    first: usize,
    last: usize,
}

impl DocumentSpan {
    /// The region a single-line citation resolves to: that line and one line either side of it.
    fn around(line: usize, total: usize) -> DocumentSpan {
        DocumentSpan {
            first: line.saturating_sub(1).max(1),
            last: line.saturating_add(1).min(total.max(1)),
        }
    }

    /// The text of this region, joined by newlines.
    ///
    /// Out-of-range bounds yield the empty string rather than panicking: every span here is built
    /// from a position already validated against the document, and a check that could abort the run
    /// on an arithmetic edge would be a worse failure than the one it is looking for.
    fn text(&self, lines: &[&str]) -> String {
        let first = self.first.saturating_sub(1);
        let last = self.last.min(lines.len());
        match first < last {
            true => lines[first..last].join("\n"),
            false => String::new(),
        }
    }
}

/// Every backtick-quoted phrase in `citation`.
///
/// Only balanced pairs count, and an empty pair is skipped: a stray backtick is a typographical
/// accident rather than a locator, and treating it as one would fail a citation for a reason that has
/// nothing to do with whether it can be followed.
fn backtick_phrases(citation: &str) -> Vec<String> {
    let mut phrases: Vec<String> = Vec::new();
    let mut rest = citation;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let phrase = after[..close].trim();
        if !phrase.is_empty() {
            phrases.push(phrase.to_string());
        }
        rest = &after[close + 1..];
    }
    phrases
}

/// Every `§`-prefixed section number in `citation`.
///
/// A section number is the run of digits and dots that follows the sign, so `§0.6.2` yields `0.6.2`
/// and a trailing sentence-ending dot is not mistaken for part of it.
fn section_locators(citation: &str) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    for fragment in citation.split('§').skip(1) {
        let number: String = fragment
            .chars()
            .take_while(|character| character.is_ascii_digit() || *character == '.')
            .collect();
        let number = number.trim_end_matches('.').to_string();
        if !number.is_empty() && !sections.contains(&number) {
            sections.push(number);
        }
    }
    sections
}

/// Every `line N` or `lines N-M` reference in `citation`, as an inclusive pair.
///
/// A single line yields the pair `(N, N)`, so the caller has one shape to check. Any dash spelling
/// separates a range, because a citation written by hand uses a hyphen and one copied out of a
/// rendered document uses an en dash, and refusing the second would be refusing a correct citation
/// for its typography.
///
/// The word must stand on its own: `multiline 5` is not a line reference, and treating it as one
/// would invent a locator the author never wrote and then hold the citation to it.
fn line_locators(citation: &str) -> Vec<(usize, usize)> {
    const DASHES: [char; 4] = ['-', '\u{2010}', '\u{2013}', '\u{2014}'];
    let lowered = citation.to_ascii_lowercase();
    let mut locators: Vec<(usize, usize)> = Vec::new();
    let mut rest = lowered.as_str();
    let mut consumed = 0usize;
    while let Some(position) = rest.find("line") {
        // "no preceding character, or a preceding character that is not alphanumeric", spelled as a
        // negated `is_some_and`. The reading is unchanged either way: absent a preceding character
        // the word does stand on its own, and a preceding character disqualifies it only when that
        // character is alphanumeric. The spelling is chosen rather than incidental, because the two
        // obvious alternatives each fail a gate. `Option::is_none_or` stabilized in Rust 1.82, above
        // the 1.70 minimum this file and every harness module document, so it makes the suite
        // unbuildable on a supported toolchain and clippy reports it as `incompatible_msrv`.
        // `map_or(true, ..)` builds on 1.70 but clippy reports it as `unnecessary_map_or` wherever
        // the effective minimum admits `is_none_or` -- including when no minimum is declared at all
        // -- so it fails `-D warnings` instead. `is_some_and` has been stable since 1.70 itself and
        // is clean under every combination, so no allow attribute is needed to keep it so.
        let starts_word = !lowered[..consumed + position]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric());
        let after = &rest[position + "line".len()..];
        let after = after.strip_prefix('s').unwrap_or(after);
        consumed = lowered.len() - after.len();
        rest = after;
        if !starts_word {
            continue;
        }
        let after = after.trim_start();
        let first: String = after
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if first.is_empty() {
            continue;
        }
        let Ok(first) = first.parse::<usize>() else {
            continue;
        };
        let tail = after
            .trim_start_matches(|character: char| character.is_ascii_digit())
            .trim_start();
        let mut last = first;
        if let Some(tail) = tail.strip_prefix(DASHES) {
            let second: String = tail
                .trim_start()
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            if let Ok(second) = second.parse::<usize>() {
                last = second;
            }
        }
        let locator = (first, last);
        if !locators.contains(&locator) {
            locators.push(locator);
        }
    }
    locators
}

/// `text` with every run of whitespace collapsed to one space and the ends trimmed.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// `text` rendered for a diagnostic: quoted, and with newlines made visible rather than wrapping.
fn quoted_for_diagnostic(text: &str) -> String {
    format!("{:?}", collapse_whitespace(text))
}

/// `items` as one comma-separated list, for a diagnostic that has to name a fixed set.
fn comma_list(items: &[impl AsRef<str>]) -> String {
    items
        .iter()
        .map(|item| item.as_ref().to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

/// Whether the finding writer and the classifier still agree about what a marker *documents*, over
/// every committed marker, every cell of the full matrix and every divergence class.
///
/// # The failure this exists to prevent
///
/// A marker excuses a divergence only when all four dimensions agree: its scope must cover the
/// oracle, the target and the optimization level, **and** its class must equal the class observed.
/// That is [`classify::covering_marker`]. The scope half alone is
/// [`manifest::ExpectedDivergence::covers`], which is deliberately class-blind because two callers
/// have no observed class to compare against — a stale marker on an agreement, and an arm the record
/// excluded.
///
/// Two predicates that answer *almost* the same question are exactly how a suite drifts. Asking the
/// class-blind one where the class matters produces the worst possible outcome for this suite: a
/// marked program that diverges in a class its marker does not claim, on an oracle the marker's scope
/// happens to name, is refused its finding artifact and reported as an unexplained failure — and the
/// refusal's own advice sends a maintainer to reclassify a genuine wrong answer as an expected
/// divergence, which requirement 5 forbids and which would launder a second defect into
/// documentation that never described it. The compiler accepting a construct and computing the wrong
/// value is the most plausible real outcome for a newly implemented feature, so this is not a corner.
///
/// # What is asserted, and why it exercises the writer for real
///
/// For every committed marker, at every cell of `Target::ALL × OptLevel::ALL`, for every oracle and
/// every class:
///
/// - [`classify::covering_marker`] answers `Some` exactly when the scope covers the arm **and** the
///   class matches — never for the other five classes, and never outside the scope;
/// - [`Finding::new`] — the writer's own precondition, not a copy of it — refuses exactly the
///   documented combination and admits every other, so a class the marker does not claim is
///   artifact-eligible even on an arm the marker's scope names;
/// - the admitted finding's marker note states *which* dimension missed, and never claims a scope
///   miss on an arm the scope covers, because that note ships inside a committed artifact and a
///   maintainer acts on it.
///
/// The writer is called rather than re-implemented, which is what makes this a regression test for
/// the defect rather than a restatement of the intended rule. It is safe to call in an audit:
/// [`Finding::new`] validates and reads, and writing artifacts is a separate step this never
/// reaches, so nothing is compiled, executed or written. The comparison handed to it is synthetic and
/// says so in its own summary.
///
/// Folded into the register audit deliberately: it needs no compiler, no emulator and no toolchain —
/// only the committed records — and the suite's test count is fixed by its own health gate, so a new
/// `#[test]` is not available to spend.
fn marker_classification_audit(
    markers: &[manifest::ExpectedDivergence],
) -> (Vec<MarkerViolation>, String) {
    let mut violations: Vec<MarkerViolation> = Vec::new();
    let mut documented_refusals = 0usize;
    let mut undocumented_admissions = 0usize;

    for marker in markers {
        // Attributed to the marker's own area for the reason given where the attribution is defined:
        // a marker can only excuse a divergence in the program it sits beside.
        let area = marker_area(marker);
        // The record is loaded from the program the marker governs, so the manifest handed to both
        // predicates is the very one a real cell would use — including its own program identity,
        // which `Finding::new` checks against the cell key.
        let record = match manifest::load_for_source(marker.program_path()) {
            Ok(record) => record,
            Err(error) => {
                violations.push(MarkerViolation::in_area(
                    &area,
                    format!(
                        "marker {} governs {} but its expectation record could not be loaded: \
                         {error}. Without the record neither the classifier's authority nor the \
                         finding writer's precondition can be exercised, so the agreement between \
                         them is unestablished",
                        marker.id(),
                        marker.program_label(),
                    ),
                ));
                continue;
            }
        };

        for &target in &Target::ALL {
            for &opt in &OptLevel::ALL {
                let key = match CellKey::new(record.area(), record.program(), target, opt) {
                    Ok(key) => key,
                    Err(error) => {
                        violations.push(MarkerViolation::in_area(
                            &area,
                            format!(
                                "marker {}: the cell identity {}/{} @ {target} {} could not be \
                                 built: {error}",
                                marker.id(),
                                record.area(),
                                record.program(),
                                opt.flag(),
                            ),
                        ));
                        continue;
                    }
                };
                for &oracle in &Oracle::ALL {
                    for &class in &DivergenceClass::ALL {
                        let probe = ClassificationProbe {
                            marker,
                            record: &record,
                            key: key.clone(),
                            oracle,
                            class,
                        };
                        let (result, defects) = probe.audit();
                        violations.extend(
                            defects
                                .into_iter()
                                .map(|defect| MarkerViolation::in_area(&area, defect)),
                        );
                        match result {
                            ProbeResult::DocumentedRefused => documented_refusals += 1,
                            ProbeResult::UndocumentedAdmitted => undocumented_admissions += 1,
                            ProbeResult::Inconsistent => {}
                        }
                    }
                }
            }
        }
    }

    // Narrowing markers are counted and named separately, because they are the one kind that can
    // document NO combination at all: their class names no observation, so every combination they
    // are probed against is correctly admitted as a finding. Without this line a reader comparing
    // the two counts would have no way to tell a narrowing marker from a marker whose scope had
    // silently stopped matching anything.
    let narrowing = markers
        .iter()
        .filter(|marker| marker.class().observed().is_none())
        .count();
    let tally = format!(
        "marker classification — {} marker(s) exercised over {} target(s) × {} level(s) × {} \
         oracle(s) × {} class(es): {} documented combination(s) refused by the finding writer as \
         expected divergences, {} undocumented combination(s) admitted as findings. A class the \
         marker does not claim is NOT excused by it, even on an arm its scope names{}\n",
        markers.len(),
        Target::ALL.len(),
        OptLevel::ALL.len(),
        Oracle::ALL.len(),
        DivergenceClass::ALL.len(),
        documented_refusals,
        undocumented_admissions,
        match narrowing {
            0 => String::new(),
            count => format!(
                " — and {count} of the {} document a comparison their record declines to make, so \
                 they claim no observable class and correctly document none of these combinations",
                markers.len()
            ),
        },
    );

    (violations, tally)
}

/// One (marker, cell, oracle, class) combination of [`marker_classification_audit`].
///
/// A value rather than a long argument list, following the `Side`/`CellPlan` idiom already used in
/// this file: the four dimensions plus the record travel together because every check below asks
/// about the same combination, and grouping them keeps each check to one line at its call site.
struct ClassificationProbe<'a> {
    /// The committed marker under audit.
    marker: &'a manifest::ExpectedDivergence,
    /// The record that carries it — the same record a real cell of this program would use.
    record: &'a Manifest,
    /// The cell the divergence is imagined at.
    key: CellKey,
    /// The oracle that would have observed it.
    oracle: Oracle,
    /// The class that would have been observed, which is the dimension the defect ignored.
    class: DivergenceClass,
}

/// What one probe established, so the audit can count the two legitimate answers separately and
/// report anything else as a defect.
enum ProbeResult {
    /// The marker documents this divergence and the writer refused it: an expected divergence.
    DocumentedRefused,
    /// Nothing documents it and the writer admitted it, with its artifacts: a finding.
    UndocumentedAdmitted,
    /// Neither, which the returned violations describe.
    Inconsistent,
}

impl ClassificationProbe<'_> {
    /// Whether the marker's scope reaches this arm — the class-blind half of the question.
    fn scope_covers(&self) -> bool {
        self.marker.covers(&self.key, self.oracle)
    }

    /// Whether the marker **documents** this divergence: the scope half **and** the class.
    ///
    /// A marker classed `comparison_excluded` documents no observation at all, so it can never
    /// answer yes here however wide its scope is — which is the property that keeps a narrowing
    /// marker from excusing a divergence a real comparison found.
    fn documented(&self) -> bool {
        self.scope_covers() && self.marker.class() == MarkerClass::Observed(self.class)
    }

    /// The combination, named once for every diagnostic this probe can produce.
    fn situation(&self) -> String {
        format!(
            "marker {} (class {}, scope {}) against a {} observed by {} at {}",
            self.marker.id(),
            self.marker.class().label(),
            self.marker.scope().raw(),
            self.class,
            self.oracle,
            self.key,
        )
    }

    /// Ask both authorities about this combination and report every disagreement.
    ///
    /// Three checks, made for every combination rather than for a sample: the classifier's
    /// predicate, the finding writer's own precondition — called, not restated — and the note the
    /// writer produced when it admitted the finding.
    fn audit(&self) -> (ProbeResult, Vec<String>) {
        let mut defects: Vec<String> = Vec::new();
        let documented = self.documented();
        let situation = self.situation();

        // Check one: the classifier's authority, which is what decides XFAIL against FINDING.
        let authority = classify::covering_marker(self.record, &self.key, self.oracle, self.class);
        if authority.is_some() != documented {
            defects.push(format!(
                "{situation}: classify::covering_marker answered {} where {} is correct. A marker \
                 documents a divergence only when its scope covers the arm ({} here) AND its class \
                 equals the class observed ({} here). Relaxing this predicate would let one marker \
                 absorb a second, undocumented defect",
                describe_documentation(authority.is_some()),
                describe_documentation(documented),
                self.scope_covers(),
                self.marker.class() == MarkerClass::Observed(self.class),
            ));
        }

        // Check two: the finding writer's own precondition, exercised rather than restated.
        let comparison = synthetic_divergence(self.oracle, self.class);
        let result = match (
            documented,
            Finding::new(self.key.clone(), &comparison, self.record),
        ) {
            (true, Err(_)) => ProbeResult::DocumentedRefused,
            (true, Ok(_)) => {
                defects.push(format!(
                    "{situation}: the finding writer ADMITTED a divergence this marker documents. \
                     A documented divergence is an expected divergence; filing it as a finding \
                     would put an entry in the findings register that the repository has already \
                     explained, and would break the invariant that a marker identifier never \
                     appears on a finding's outcome"
                ));
                ProbeResult::Inconsistent
            }
            (false, Ok(finding)) => {
                defects.extend(marker_note_defects(
                    &finding,
                    self.marker,
                    self.class,
                    self.scope_covers(),
                ));
                ProbeResult::UndocumentedAdmitted
            }
            (false, Err(error)) => {
                defects.push(format!(
                    "{situation}: the finding writer REFUSED to assemble the finding, reporting: \
                     {error}. This divergence is undocumented, so the classifier rules it a \
                     finding and the writer must produce its artifacts; a refusal here surfaces as \
                     an unexplained failure with no reproducer, and its advice would send a \
                     maintainer to reclassify an undocumented divergence as a documented one. The \
                     precondition must ask classify::covering_marker — the class as well as the \
                     scope — and never the class-blind scope half"
                ));
                ProbeResult::Inconsistent
            }
        };
        (result, defects)
    }
}

/// `documented` or `undocumented`, so the two halves of a disagreement read the same way.
fn describe_documentation(documented: bool) -> &'static str {
    if documented {
        "documented"
    } else {
        "undocumented"
    }
}

/// Whether the note an admitted finding carries tells the truth about why the marker does not
/// document the divergence.
///
/// Asserted as properties rather than as an exact sentence: the note must name the marker, must name
/// the class actually observed whenever that differs from the class the marker claims, and must not
/// assert the wrong dimension — no scope miss on an arm the scope covers, and no scope hit on an arm
/// it does not. The note ships inside a committed artifact directory and is the sentence a
/// maintainer acts on, so a note that named the wrong dimension would be the same misdirection in a
/// quieter place.
fn marker_note_defects(
    finding: &Finding,
    marker: &manifest::ExpectedDivergence,
    class: DivergenceClass,
    scope_covers: bool,
) -> Vec<String> {
    let context = format!(
        "the finding admitted for {} under {} carries a marker note that",
        finding.key(),
        finding.oracle(),
    );
    let Some(note) = finding.marker_note() else {
        return vec![format!(
            "{context} is absent, although {} carries marker {}. A reader must be told that a \
             divergence in this program was considered and scoped elsewhere, because that is a \
             different situation from a program nobody has documented at all",
            marker.program_label(),
            marker.id(),
        )];
    };

    let mut defects: Vec<String> = Vec::new();
    if !note.contains(marker.id()) {
        defects.push(format!(
            "{context} does not name marker {}, so a reader cannot look up what was documented: \
             {note}",
            marker.id(),
        ));
    }
    if marker.class() != MarkerClass::Observed(class) && !note.contains(class.label()) {
        defects.push(format!(
            "{context} does not name the {class} actually observed, only the {} the marker claims, \
             so it describes the documentation rather than the divergence: {note}",
            marker.class().label(),
        ));
    }
    if scope_covers
        && (note.contains("does not cover this cell") || note.contains("covers neither"))
    {
        defects.push(format!(
            "{context} claims the marker's scope does not reach this cell, but its scope {} does \
             cover {} at {}; the dimension that missed is the class. A note that sends a reader to \
             widen a scope that was never the problem is worse than no note: {note}",
            marker.scope().raw(),
            finding.oracle(),
            finding.key(),
        ));
    }
    if !scope_covers && note.contains("scope covers this cell") {
        defects.push(format!(
            "{context} claims the marker's scope covers this cell, but its scope {} does not reach \
             {} at {}: {note}",
            marker.scope().raw(),
            finding.oracle(),
            finding.key(),
        ));
    }
    defects
}

/// A divergence of a given class and oracle, constructed in memory for the audit above.
///
/// Says what it is in its own summary, because it travels into [`Finding::new`] and a reader of any
/// diagnostic quoting it must not mistake it for something a compiler produced. Nothing here is
/// compiled, executed, compared or written: the audit asks the writer a question about
/// classification and discards the answer's evidence.
fn synthetic_divergence(oracle: Oracle, class: DivergenceClass) -> Comparison {
    Comparison {
        equal: false,
        class: Some(class),
        summary: format!(
            "expected-divergence register audit: a synthetic {class} attributed to {oracle}, \
             constructed in memory to ask the finding writer whether this divergence is documented; \
             nothing was compiled, executed or compared to produce it"
        ),
        detail: String::from(
            "This comparison is an audit construct, not an observation. It exists so that the \
             finding writer's marker precondition is exercised for every divergence class against \
             every committed marker, which is what keeps that precondition and the classifier's own \
             authority from drifting apart.",
        ),
        oracle,
        excluded: None,
        // No provenance, and that is the accurate value rather than a gap. A provenance records the
        // executions an outcome was reached from, and this construct was reached from none: nothing
        // was compiled, run or compared to produce it. Filling the fields with plausible-looking
        // blanks would make an audit construct indistinguishable from a measurement.
        provenance: None,
    }
}

/// Every curated finding directory committed under the corpus, in a deterministic order.
///
/// An empty result is the ordinary and honest state of a branch that has recorded no finding: the
/// directory holds only its own placeholder, and the register says so. A regular file is skipped
/// rather than reported, because the placeholder that keeps an empty directory in version control is
/// a regular file and is not a finding.
///
/// # Why this fails closed, and what the one tolerated absence is
///
/// The curated set is an audit input, and an audit that reads "nothing to check" from an error has not
/// established that there is nothing to check — it has established nothing at all. Turning **every**
/// listing failure into an empty set, or discarding a per-entry error, would let a permission change, a
/// partially unreadable directory, or a `findings/` replaced by a symbolic link all read as "no
/// findings committed", and the whole curated audit would silently pass over the set it was meant to
/// validate.
///
/// So exactly one condition is tolerated, and it is the one that carries information: [`NotFound`] —
/// the directory is not there, which is the true and complete answer for a branch that has never
/// curated a finding. Every other listing error, every per-entry error, and any entry that is a
/// symbolic link or resolves outside the root is returned as a defect for the caller to report.
///
/// A symbolic link is refused rather than skipped, and the distinction matters: skipping one would
/// leave a curated finding *reachable by a reader following the register* while invisible to the
/// validator, which is the one asymmetry this audit exists to prevent.
///
/// [`NotFound`]: io::ErrorKind::NotFound
fn curated_finding_directories() -> Result<Vec<PathBuf>, Vec<String>> {
    let root = corpus_root().join(conformance_harness::CURATED_FINDINGS_DIR_NAME);
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        // The only tolerated answer: there is no curated set, which is a fact rather than a failure to
        // learn one.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(vec![format!(
                "the curated finding set at {} could not be listed, so this run has established \
                 nothing about the committed findings — not that there are none: {error}. An audit \
                 that read this as an empty set would report every curated finding as validated \
                 while having read none of them",
                shown_path(&root)
            )])
        }
    };

    let mut directories: Vec<PathBuf> = Vec::new();
    let mut defects: Vec<String> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                defects.push(format!(
                    "an entry of the curated finding set at {} could not be read, so the set was \
                     only partially enumerated and a curated finding may have gone unaudited: \
                     {error}",
                    shown_path(&root)
                ));
                continue;
            }
        };
        let path = entry.path();
        let kind = match entry.file_type() {
            Ok(kind) => kind,
            Err(error) => {
                defects.push(format!(
                    "the type of {} in the curated finding set could not be determined, so whether \
                     it is a finding to audit is unknown: {error}",
                    shown_path(&path)
                ));
                continue;
            }
        };
        if kind.is_symlink() {
            defects.push(format!(
                "{} in the curated finding set is a symbolic link. A reader following the register \
                 would arrive at whatever it points at while this audit refuses to follow it, so the \
                 evidence a maintainer sees and the evidence that was validated would be two \
                 different things — commit the directory itself",
                shown_path(&path)
            ));
            continue;
        }
        if !kind.is_dir() {
            // A regular file is the placeholder that keeps the directory in version control. Not a
            // finding, and not a defect.
            continue;
        }
        // Belt and braces against a name that escapes the root — `..` cannot come out of `read_dir`,
        // but the containment is the property every later read depends on, so it is established here
        // where the path is first admitted rather than assumed at each use.
        if !path.starts_with(&root) {
            defects.push(format!(
                "{} was enumerated from the curated finding set but does not lie beneath it, so \
                 auditing it would read a directory the register does not index",
                shown_path(&path)
            ));
            continue;
        }
        directories.push(path);
    }

    if !defects.is_empty() {
        return Err(defects);
    }
    directories.sort();
    Ok(directories)
}

/// One row of the curated findings register.
///
/// The narrow, factual fields the register's own table declares. The explanation lives in each
/// directory's `MANIFEST.txt` and is deliberately not duplicated here, so this type holds only what an
/// audit can check against the filesystem.
struct FindingsRegisterRow {
    /// The `F-NNNN-<slug>` identifier.
    id: String,
    /// The divergence class the row states.
    class: String,
    /// The artifact directory the row points at, as written.
    directory: String,
    /// The row's status, from the register's closed vocabulary.
    status: String,
    /// The identifier this row defers to, when its status is `superseded`.
    superseded_by: Option<String>,
    /// The line the row was read from, for a diagnostic that can be located.
    line: usize,
}

/// The closed status vocabulary the register declares.
const FINDINGS_STATUS_VALUES: &[&str] = &["open", "acknowledged", "superseded"];

/// The register's spelling of an empty cell.
const FINDINGS_EMPTY_CELL: &str = "—";

/// Read the curated findings register's table.
///
/// # Why the whole file is parsed rather than searched
///
/// The register is prose with one table in it, and the table is what the audit is about. Rows are
/// recognised structurally — a pipe-delimited line inside the section that declares the table's
/// header, with the header and separator skipped — rather than by pattern-matching an identifier
/// anywhere in the file, because an identifier quoted in a paragraph is a mention and not a row. The
/// distinction matters: a maintainer who describes a finding in §7.4's changelog has not indexed it,
/// and an audit that could not tell those apart would report the changelog as the index.
///
/// # Errors
///
/// Returns a defect describing the file when it cannot be read or when a row cannot be parsed into the
/// fields the register declares. A register the audit cannot read is a register nobody can rely on, so
/// it is reported rather than treated as empty.
fn findings_register_rows() -> Result<Vec<FindingsRegisterRow>, String> {
    let path = manifest_dir().join(classify::FINDINGS_REGISTER);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            return Err(format!(
            "{} could not be read, so the curated findings it indexes cannot be held against the \
                 directories on disk: {error}",
            classify::FINDINGS_REGISTER
        ))
        }
    };
    let mut rows: Vec<FindingsRegisterRow> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        let cells: Vec<String> = trimmed
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`').trim().to_string())
            .collect();
        // The register's table has ten columns. Any other pipe-delimited line in the file belongs to
        // one of its explanatory tables, and is not a row of the index.
        if cells.len() != 10 {
            continue;
        }
        let identifier = cells[0].clone();
        if !identifier.starts_with("F-") {
            continue;
        }
        rows.push(FindingsRegisterRow {
            id: identifier,
            class: cells[3].clone(),
            directory: cells[7].clone(),
            status: cells[8].clone(),
            superseded_by: (cells[9] != FINDINGS_EMPTY_CELL && !cells[9].is_empty())
                .then(|| cells[9].clone()),
            line: index + 1,
        });
    }
    Ok(rows)
}

/// Hold the curated findings register against the directories on disk, in both directions.
///
/// Every check here closes a way the register and the evidence can disagree while each looks intact on
/// its own:
///
/// - **Identity, both ways.** Every row names a directory beneath `findings/` that exists, and every
///   directory beneath `findings/` has exactly one row. A directory nothing indexes is a deliverable a
///   reader cannot find; a row pointing at nothing is a broken pointer wearing the appearance of an
///   index entry.
/// - **The path is the identifier.** A row's `Artifact directory` must be exactly
///   `findings/<its own identifier>/`, which is what makes the index followable by reading rather than
///   by searching. A row whose path names another finding's evidence is the one shape of error that
///   sends a reader confidently to the wrong place.
/// - **The manifest agrees.** The directory's own `MANIFEST.txt` must declare that identifier on its
///   `curated_id` line and the same divergence class the row states, so the index cannot describe one
///   finding while the evidence describes another.
/// - **Status is from the closed vocabulary**, and `superseded` — and only `superseded` — carries a
///   successor. That successor must be a row in this same table, and must not itself defer back to the
///   row that named it, because a cycle identifies no live finding at all.
/// - **Identifiers are unique.** Two rows sharing one identifier make every other check ambiguous.
fn findings_register_violations(rows: &[FindingsRegisterRow], curated: &[PathBuf]) -> Vec<String> {
    let mut violations: Vec<String> = Vec::new();
    let register = classify::FINDINGS_REGISTER;
    let directory_name = |path: &PathBuf| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    for row in rows {
        if rows.iter().filter(|other| other.id == row.id).count() > 1 {
            violations.push(format!(
                "{register} line {} lists {} more than once; one identifier names one investigation, \
                 so a duplicate row makes every check about it ambiguous",
                row.line, row.id
            ));
        }
        let expected_path = format!(
            "{}/{}/",
            conformance_harness::CURATED_FINDINGS_DIR_NAME,
            row.id
        );
        if row.directory != expected_path {
            violations.push(format!(
                "{register} line {} indexes {} but points at `{}` rather than `{expected_path}`; the \
                 path is what makes the row followable by reading, and a row pointing at another \
                 finding's evidence sends a reader confidently to the wrong place",
                row.line, row.id, row.directory
            ));
        }
        if !FINDINGS_STATUS_VALUES.contains(&row.status.as_str()) {
            violations.push(format!(
                "{register} line {} gives {} the status `{}`, which is not one of the closed set {}; \
                 anything else is a comment and belongs in the finding's own manifest",
                row.line,
                row.id,
                row.status,
                comma_list(
                    &FINDINGS_STATUS_VALUES
                        .iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<String>>()
                )
            ));
        }
        match (&row.superseded_by, row.status.as_str()) {
            (Some(successor), "superseded") => {
                match rows.iter().find(|other| &other.id == successor) {
                    None => violations.push(format!(
                        "{register} line {} says {} is superseded by {successor}, which is not a row \
                         in this table; a successor the register never identifies sends a reader \
                         looking for a finding that is not indexed",
                        row.line, row.id
                    )),
                    Some(other)
                        if other.status == "superseded"
                            && other.superseded_by.as_deref() == Some(row.id.as_str()) =>
                    {
                        violations.push(format!(
                            "{register} lines {} and {} defer to each other: {} is superseded by {} \
                             and {} by {}. A cycle identifies no live finding at all",
                            row.line, other.line, row.id, other.id, other.id, row.id
                        ))
                    }
                    Some(_) => {}
                }
            }
            (None, "superseded") => violations.push(format!(
                "{register} line {} marks {} superseded without naming its successor; the pointer is \
                 what makes the status meaningful, so the column is mandatory for it",
                row.line, row.id
            )),
            (Some(successor), status) => violations.push(format!(
                "{register} line {} gives {} the status `{status}` and still names {successor} as its \
                 successor; only `superseded` carries one, and the column must read `\
                 {FINDINGS_EMPTY_CELL}` otherwise",
                row.line, row.id
            )),
            (None, _) => {}
        }

        match curated.iter().find(|path| directory_name(path) == row.id) {
            None => violations.push(format!(
                "{register} line {} indexes {} but no directory of that name exists beneath \
                 `{}/`; a row pointing at nothing is a broken pointer with the appearance of an index \
                 entry, and the deliverable it promises cannot be read",
                row.line, row.id, conformance_harness::CURATED_FINDINGS_DIR_NAME
            )),
            Some(path) => violations.extend(register_row_agrees_with_manifest(row, path)),
        }
    }

    for path in curated {
        let name = directory_name(path);
        if !rows.iter().any(|row| row.id == name) {
            violations.push(format!(
                "the curated finding {} has no row in {register}; a directory nothing indexes is a \
                 deliverable a reader cannot find, and the register and the evidence must enumerate \
                 the same set in both directions",
                shown_path(path)
            ));
        }
    }
    violations
}

/// Whether one register row agrees with the manifest inside the directory it points at.
fn register_row_agrees_with_manifest(row: &FindingsRegisterRow, path: &Path) -> Vec<String> {
    let mut violations: Vec<String> = Vec::new();
    let register = classify::FINDINGS_REGISTER;
    let manifest_path = path.join(findings::MANIFEST_NAME);
    let text = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(error) => {
            violations.push(format!(
                "the manifest of the curated finding {} could not be read, so {register}'s row for \
                 {} cannot be checked against it: {error}",
                shown_path(path),
                row.id
            ));
            return violations;
        }
    };
    let value = |prefix: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(prefix))
            .map(str::trim)
            .map(String::from)
    };
    match value(findings::MANIFEST_CURATED_ID_PREFIX) {
        Some(declared) if declared == row.id => {}
        Some(declared) => violations.push(format!(
            "{register} indexes {} but the manifest in {} declares `{}` as {}; the index and the \
             evidence must name the same finding",
            row.id,
            shown_path(path),
            findings::MANIFEST_CURATED_ID_PREFIX.trim_end(),
            declared
        )),
        None => violations.push(format!(
            "{register} indexes {} but the manifest in {} declares no `{}` line, so nothing inside \
             the directory claims the identifier the register indexes it under",
            row.id,
            shown_path(path),
            findings::MANIFEST_CURATED_ID_PREFIX.trim_end()
        )),
    }
    match value("divergence_class = ") {
        Some(declared) if declared == row.class => {}
        Some(declared) => violations.push(format!(
            "{register} states the divergence class of {} as `{}` while the manifest in {} records \
             `{declared}`; the index must describe the evidence it points at",
            row.id,
            row.class,
            shown_path(path)
        )),
        None => violations.push(format!(
            "the manifest of the curated finding {} records no divergence class, so {register}'s \
             `{}` cannot be confirmed",
            shown_path(path),
            row.class
        )),
    }
    violations
}

/// The corpus markers as one readable list, for a message that has to name them all.
fn marker_identifier_list(markers: &[manifest::ExpectedDivergence]) -> String {
    if markers.is_empty() {
        return "(none)".to_string();
    }
    markers
        .iter()
        .map(|marker| format!("{} ({})", marker.id(), marker.program_label()))
        .collect::<Vec<String>>()
        .join(", ")
}

/// The message an infrastructure test carries when it could not be performed at all.
///
/// Says explicitly that nothing has been established about the compiler under test, because an
/// infrastructure failure read as a verdict would be worse than no result.
fn infrastructure_failure(test: &str, consequence: &str, error: &HarnessError) -> String {
    format!(
        "{test} could not be performed.\n\n{error}\n\nConsequence: {consequence}.\n\nThis is an \
         infrastructure failure rather than a verdict: nothing has been established about the \
         compiler under test either way, which is why it fails loudly instead of passing.",
    )
}
