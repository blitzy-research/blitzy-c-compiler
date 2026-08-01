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
//! `rustc --test --emit=metadata` on this file still do, because neither needs a package. That
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
//! The repository's sixteen existing integration suites share one helper module,
//! `tests/common/mod.rs`, and this suite is required to integrate with that convention rather than
//! duplicate it. The obligation is conditional on the file existing, because `mod common;` is a
//! compile-time assertion that it does: declared against an absent file it does not fall back, it
//! fails the build, and a suite that cannot compile reuses nothing.
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
//! before any oracle is asked, and it removes the authority every one of them needs. When a marker
//! documents that refusal, **every** arm it blocked is an expected divergence traceable to that one
//! documented root cause — not one expected divergence and a cascade of undocumented findings for
//! the same defect. The marker's own scope is never widened to achieve that, and every outcome
//! detail says which oracle the marker names and which oracle it is excusing.
//!
//! Only the standard library is used, every operation is safe, no lint is suppressed, and not one
//! of the eighteen tests is marked ignored — the repository's ignored-test count is itself the most
//! direct mechanical check that no pre-existing test was skipped or weakened. Edition 2021, minimum
//! supported Rust 1.70.

mod conformance_harness;

use std::fs;
use std::path::{Path, PathBuf};

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
    corpus_root, findings_root, manifest_dir, report_root, shown_path, target_dir_rejection,
    work_root, AreaSpec, Cell, CellKey, DivergenceClass, HarnessError, OptLevel, Oracle, Outcome,
    Target, Verdict, AREAS, AREA_COUNT, BCC_CELL_COUNT, MIN_PROGRAMS_PER_MANDATED_AREA,
    OPT_LEVEL_COUNT, ORACLE_A_COMPARISON_COUNT, ORACLE_B_COMPARISON_COUNT,
    ORACLE_C_ASSERTION_COUNT, PROGRAM_COUNT, TARGET_COUNT, TOTAL_ASSERTION_COUNT,
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
#[test]
fn infra_flag_capability_probe() {
    let caps = oracle_capabilities();
    begin_report_session();
    let probe = match flagprobe::run(&caps) {
        Ok(probe) => probe,
        Err(error) => panic!(
            "{}",
            infrastructure_failure(
                "infra_flag_capability_probe",
                "the flag-capability probe could not be performed at all, so requirement 3 is \
                 unverified and every differential comparison in this suite would rest on an \
                 unchecked assumption",
                &error,
            )
        ),
    };

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
#[test]
fn infra_ub_audit_gate() {
    let caps = oracle_capabilities();
    begin_report_session();
    let audit = match ubaudit::run(&caps) {
        Ok(audit) => audit,
        Err(error) => panic!(
            "{}",
            infrastructure_failure(
                "infra_ub_audit_gate",
                "the undefined-behaviour audit could not enumerate or gate the corpus, so no \
                 program in the matrix has been shown free of undefined behaviour and a \
                 divergence could not be attributed to either compiler",
                &error,
            )
        ),
    };

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
/// into stale documentation: every marker committed in a program's expectation record appears in
/// the register, every identifier the register lists corresponds to a real marker, and every cited
/// basis names a file that actually exists in this repository. A marker changes how a divergence is
/// *classified*; it never changes whether the feature is *exercised*.
#[test]
fn infra_expected_divergence_register() {
    // Deliberately does not call `oracle_capabilities()`: this reads committed files only, so it
    // stays meaningful on a machine with no reference compiler and no emulator at all. It still
    // begins the report session, which needs no capabilities: this test writes no area report, and
    // a run consisting only of tests that write none is exactly the case in which a previous run's
    // summary would otherwise survive and be mistaken for this run's verdict.
    begin_report_session();
    let markers = match manifest::all_markers() {
        Ok(markers) => markers,
        Err(error) => panic!(
            "{}",
            infrastructure_failure(
                "infra_expected_divergence_register",
                "the corpus expected-divergence markers could not be enumerated, so requirement \
                 5's prohibition on silently excluding a feature cannot be enforced in either \
                 direction",
                &error,
            )
        ),
    };

    let register_path = manifest_dir().join(classify::EXPECTED_DIVERGENCE_REGISTER);
    let register = match fs::read_to_string(&register_path) {
        Ok(register) => register,
        Err(error) => panic!(
            "the expected-divergence register {} could not be read: {error}.\n\nIt is a committed \
             deliverable, not a generated artifact: requirement 5 requires every expected \
             divergence to be auditable in one place, and this test is what keeps that place \
             truthful. {} marker(s) are committed in the corpus and would go unregistered.\n\
             Markers found: {}",
            shown_path(&register_path),
            markers.len(),
            marker_identifier_list(&markers),
        ),
    };

    let registered = registered_identifiers(&register);
    let mut violations: Vec<String> = Vec::new();

    for marker in &markers {
        if !registered.iter().any(|entry| entry == marker.id()) {
            violations.push(format!(
                "marker {} is committed in {} ({}) but does not appear in {}; an unregistered \
                 marker is exactly the silent exclusion requirement 5 forbids. Resolve it by \
                 registering the marker or by retiring it from the record beside that program",
                marker.id(),
                marker.program_label(),
                shown_path(marker.program_path()),
                classify::EXPECTED_DIVERGENCE_REGISTER,
            ));
        }

        let basis = marker.basis_absolute_path();
        if !basis.is_file() {
            violations.push(format!(
                "marker {} cites the basis {:?}, which resolves to {} and is not a file; a marker \
                 without a real documented basis reclassifies a divergence on no authority at all",
                marker.id(),
                marker.basis(),
                shown_path(&basis),
            ));
        }
    }

    for identifier in &registered {
        if !markers.iter().any(|marker| marker.id() == identifier) {
            violations.push(format!(
                "{} lists {} but no expectation record in the corpus carries that marker; the \
                 register must describe divergences that are actually exercised, not ones that \
                 were retired without retiring the entry",
                classify::EXPECTED_DIVERGENCE_REGISTER,
                identifier,
            ));
        }
    }

    println!(
        "expected-divergence register — {} marker(s) in the corpus, {} identifier(s) in {}",
        markers.len(),
        registered.len(),
        classify::EXPECTED_DIVERGENCE_REGISTER,
    );
    for marker in &markers {
        println!(
            "  {} [{}] {} — scope: {} — basis: {}",
            marker.id(),
            marker.class().label(),
            marker.program_label(),
            marker.scope().raw(),
            marker.basis(),
        );
    }

    assert!(
        violations.is_empty(),
        "the expected-divergence markers and {} are not consistent — {} violation(s):\n{}",
        classify::EXPECTED_DIVERGENCE_REGISTER,
        violations.len(),
        violations
            .iter()
            .map(|violation| format!("  - {violation}"))
            .collect::<Vec<String>>()
            .join("\n"),
    );
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
    println!("{}", caps.render_fingerprint());

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
                let outcomes = self.subject_refused(&build, &reference);
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
            outcomes.push(self.oracle_b(&subject, baseline.as_ref()));
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
            } => classify::build_failure(
                self.record,
                self.key,
                oracle,
                *class,
                *attribution,
                summary,
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
    fn oracle_b(&self, subject: &Authority, baseline: Option<&Baseline>) -> Outcome {
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
                self.settle(
                    &comparison,
                    Side::ran(CaptureRole::UnderTest, self.key.target(), subject),
                    Some(Side::ran(
                        CaptureRole::Baseline,
                        Target::BASELINE,
                        authority,
                    )),
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
        let outcome = classify::classify(comparison, self.record, self.key, comparison.oracle);
        self.deliver(outcome, comparison, subject, counterpart)
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
            class,
            build_attribution(build),
            &summary,
        );
        let comparison =
            compare::build_refusal(oracle, self.key, class, side, &summary, reference_arm);
        self.deliver(outcome, &comparison, subject, counterpart)
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
    ) -> Outcome {
        if outcome.verdict() != Verdict::Finding {
            return outcome;
        }
        match self.assemble_finding(comparison, subject, counterpart) {
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
    ) -> Result<Finding, HarnessError> {
        let opt = self.key.opt();
        let mut finding = Finding::new(self.key.clone(), comparison, self.record)?
            .with_capture(subject.capture(opt)?);
        if let Some(counterpart) = counterpart {
            finding = finding.with_capture(counterpart.capture(opt)?);
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
    /// settled against the same evidence. When the refusal is documented, `classify.rs` records each
    /// arm as an expected divergence traceable to that one documented root cause; when it is not,
    /// each arm is a finding, and every one of them is delivered with the refusal's own artifacts
    /// rather than as a bare verdict.
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
    /// What the refusal does **not** do is depend on that arm. That the reference compiler accepts
    /// the program is not re-established per cell, because requirement 1's audit gate establishes it
    /// for the whole corpus before any cell runs — which is precisely why a refusal here is
    /// attributed to bcc alone. So on a machine with no cross driver for this target the refusal is
    /// still delivered as a complete artifact directory, with the absence named rather than left
    /// blank, and a finding is never degraded by an absent oracle.
    fn subject_refused(&self, build: &CompileOutcome, reference: &ReferenceArm) -> Vec<Outcome> {
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
        // Rendered once: every oracle blocked by this one refusal is settled against the same
        // evidence, and the phrase is part of that evidence.
        let reference_arm = describe_reference_arm(reference);
        self.oracles
            .iter()
            .map(|oracle| {
                self.settle_refusal(
                    *oracle,
                    build,
                    RefusedSide::UnderTest,
                    Side::refused(CaptureRole::UnderTest, self.key.target(), build),
                    // The arm becomes the finding's counterpart capture only when it actually ran.
                    // Every other ending is carried as the phrase below instead, because there are
                    // no captured bytes to attach and an empty capture would claim there were.
                    match reference {
                        ReferenceArm::Ran(authority) => Some(Side::ran(
                            CaptureRole::ReferenceCompiler,
                            self.key.target(),
                            authority,
                        )),
                        _ => None,
                    },
                    Some(&reference_arm),
                )
            })
            .collect()
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
        let CellPlan { workspace, .. } = self;
        let investigate = outcomes.iter().any(|outcome| {
            matches!(
                outcome.verdict(),
                Verdict::Fail | Verdict::XPass | Verdict::Finding
            )
        });
        if investigate || workspace.keeps_on_success() {
            println!("  workspace retained: {}", workspace.retain().describe());
        } else if let Some(note) = workspace.discard_advisory() {
            println!("  note: {note}");
        }
    }
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

    let programs = select_programs(spec, &caps);
    let mut outcomes: Vec<Outcome> = Vec::new();
    for program in &programs {
        outcomes.extend(run_program(&caps, program));
    }

    let digest = area_digest(spec, &programs, &outcomes, caps.config());
    println!("{digest}");
    publish(spec, &outcomes, &caps);
    conclude(spec, &programs, &outcomes, &digest, caps.config());
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

/// Discard the previous run's report artifacts, so this run's summary can only describe this run.
///
/// Every one of the eighteen tests calls this, including the four infrastructure tests that write
/// no area report at all — precisely because they write none. Without it, a run restricted to the
/// infrastructure tests, or to one area by name, would leave a previous full summary standing in the
/// report root where the next reader would take it for the current verdict. The harness performs the
/// work exactly once per process and blocks concurrent callers until it is done, so calling it from
/// every test costs one directory scan for the whole run.
fn begin_report_session() {
    if let Err(error) = report::begin_session() {
        panic!(
            "the previous run's report artifacts could not be cleared.\n\n{error}\n\nThe run \
             summary is assembled from the per-area report files on disk, so an artifact this run \
             did not write would be aggregated into this run's summary and reported as its result. \
             This fails rather than proceeding, because a summary that silently blends two runs is \
             worse than no summary at all.",
        );
    }
}

/// Create the build-directory roots every cell writes beneath, and claim this run's report tree.
///
/// Called by every area and idempotent in both halves, so no ordering between the fourteen area
/// tests is needed. The report half is done here, before the first cell of the first area is
/// compiled, rather than left to the moment an area publishes: clearing the previous run's
/// artifacts and claiming the directory up front means a conflict with a concurrent run is
/// reported in seconds instead of after a full matrix, and it means the directory holds *this*
/// run's reports for the whole of this run.
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
            publish(spec, &[], caps);
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
            publish(spec, &[], caps);
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
fn publish(spec: &'static AreaSpec, outcomes: &[Outcome], caps: &Capabilities) {
    if let Err(error) = report::write_area(spec.directory(), outcomes, caps) {
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
        Ok(true) => println!(
            " run summary written:\n   {}\n   {}",
            shown_path(&report::summary_markdown_path()),
            shown_path(&report::summary_tsv_path()),
        ),
        // Never left as a bare "pending": the harness states which areas it is still waiting for,
        // and names any area report it REFUSED because that report belongs to a different run —
        // the one case where a missing summary needs acting on rather than merely noting. Pending
        // means only that an area this invocation will still run has not filed its report yet; a
        // narrowed run does not sit pending forever, because the summary is written over the areas
        // the invocation selected and labelled partial when a filter or a reduced matrix narrowed
        // them.
        Ok(false) => println!(
            "  run summary not written — {}",
            report::finalization_pending(caps)
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
    text.push_str(&format!("  programs declared: {PROGRAM_COUNT}\n"));
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

/// Every `XD-` identifier the register mentions, deduplicated and ordered.
///
/// Deliberately tolerant of the surrounding markup: identifiers are whitespace-free by
/// construction, so splitting on all that cannot appear inside one finds them whether it spells
/// them in a table cell, a heading, a list item or a code span.
fn registered_identifiers(register: &str) -> Vec<String> {
    let mut identifiers: Vec<String> = Vec::new();
    for token in register.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
    }) {
        let candidate = token.trim_matches('-');
        if !candidate.starts_with("XD-") || candidate.len() <= "XD-".len() {
            continue;
        }
        let identifier = candidate.to_string();
        if !identifiers.contains(&identifier) {
            identifiers.push(identifier);
        }
    }
    identifiers.sort();
    identifiers
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
