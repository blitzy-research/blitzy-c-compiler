//! The verdict decision procedure: one observation about one cell becomes one
//! [`Outcome`], always.
//!
//! This module is where requirement seven — *all tests other than recorded expected
//! divergences and findings must pass* — stops being a sentence and becomes a closed,
//! exhaustive decision procedure. It is pure policy over data: it performs no process
//! execution, opens no file, writes nothing, reads no environment variable and holds no
//! state. Running a program belongs to `execute.rs`, comparing two observations belongs to
//! `compare.rs`, laying artifacts down belongs to `findings.rs` and `report.rs`. What
//! belongs here, and only here, is the judgement.
//!
//! # The closed verdict space
//!
//! Six divergence classes and six verdicts, and the mapping between them is **total**: every
//! situation the harness can be in reaches a verdict. There is deliberately **no "skip
//! because unsupported" verdict**, and no function in this file can return without producing
//! an [`Outcome`] — the return type is [`Outcome`] rather than `Option<Outcome>` precisely so
//! that dropping a cell is not expressible. An oracle whose tooling is genuinely absent is
//! reported as [`Verdict::Unavailable`]: loudly, in the summary, and never as a pass.
//!
//! | Situation | Verdict | Fails the run? |
//! |---|---|---|
//! | An oracle's tooling is genuinely absent | `UNAVAILABLE` | only under `BCC_CONFORMANCE_STRICT` |
//! | A build failure this machine is answerable for — a target's C runtime is absent | `UNAVAILABLE` at environment scope | only under `BCC_CONFORMANCE_STRICT` |
//! | The comparison agrees and no marker governs the cell | `PASS` | no |
//! | The comparison agrees and a marker covers this cell and oracle | `XPASS` | **yes**, unless `BCC_CONFORMANCE_ALLOW_XPASS` |
//! | A divergence, and a marker covers this oracle, target, level **and** class | `XFAIL` | no |
//! | A build produced no artifact, and a marker covers this oracle, target, level **and** class | `XFAIL` on that oracle's arm | no |
//! | A build produced no artifact, and a marker covers the other three dimensions but not this oracle | `FINDING` on that arm | no — a finding is a deliverable |
//! | A divergence with no covering marker | `FINDING` | no — a finding is a deliverable |
//! | Not attempted because the record narrows coverage, and a marker covers the scope | `XFAIL` citing the marker | no |
//! | Not attempted because the record narrows coverage, with no covering marker | `XFAIL` citing the record's own recorded reason | no |
//! | Not attempted while the record *enables* that oracle | `FAIL` | **yes** |
//! | Anything unexplained | `FAIL` | **yes** |
//!
//! The six classes, and what each one means for this decision:
//!
//! | Class | Meaning, and how it can be excused |
//! |---|---|
//! | `compile_failure` | One compiler rejected a program the other accepted. Excusable by a marker citing a limitation the repository **explicitly documents** — never by an inventory that merely omits the construct, which is why GCC case ranges carry no marker and a refusal there is a finding. |
//! | `link_failure` | The program translated but did not link. **Attributable to this machine** when a target's C runtime is absent, in which case it is `UNAVAILABLE` at environment scope and never a finding against the compiler. |
//! | `run_crash` | The program died on a signal instead of exiting. Compared as a raw wait status, so it is never conflated with a numerically equal ordinary exit. |
//! | `exit_code_mismatch` | Two completed runs disagreed on status. |
//! | `stdout_mismatch` | Two completed runs disagreed on bytes — the ordinary shape of a wrong answer. |
//! | `timeout` | Execution outlived its per-cell budget: a first-class divergence, because a program that finishes promptly under one compiler and hangs under another is a defect worth surfacing. |
//!
//! # The decision order
//!
//! [`judge`] applies these in exactly this order, and it is one function so that the whole
//! policy can be audited in a single reading:
//!
//! 1. **An oracle's tooling is genuinely absent** — [`Verdict::Unavailable`], with a
//!    diagnosis that names the tool and the environment variable that would supply it.
//! 2. **The comparison agrees and no marker governs the cell** — [`Verdict::Pass`].
//! 3. **The comparison agrees and a marker covers this cell and oracle** —
//!    [`Verdict::XPass`]. The marker claims a divergence that is no longer there.
//! 4. **A divergence a marker covers on oracle, target, level and class** —
//!    [`Verdict::XFail`], recording the marker identifier and its documented basis, because
//!    the deliverable summary must list every expected divergence with its basis.
//! 5. **A divergence with no covering marker** — [`Verdict::Finding`].
//! 6. **Anything unexplained** — [`Verdict::Fail`].
//!
//! # Why unexpected success fails the run
//!
//! > If a marker is present but the divergence has disappeared, the verdict is XPASS and the
//! > run fails by default, following the standard xunit convention that unexpected success is
//! > a failure. The justification is asymmetric cost: a stale marker is stale documented
//! > knowledge that will mislead the next reader, while retiring it is a trivial test-only
//! > edit.
//!
//! `BCC_CONFORMANCE_ALLOW_XPASS` downgrades it from a failure to a warning for a
//! marker-retirement window, and an unexpected success is listed separately and prominently
//! in the summary either way — see [`unexpected_successes`] — so it can never pass unnoticed.
//! Every `XPASS` detail names the marker and says where to retire it: the program's own
//! `.expected` record **and** the register at [`EXPECTED_DIVERGENCE_REGISTER`], because a
//! marker retired in one place and left in the other is still stale documentation.
//!
//! # Scope matching is strict
//!
//! A marker excuses a divergence only when its scope covers this oracle, this target and this
//! optimization level **and** its class equals the class observed — see [`covers`]. A marker for
//! `compile_failure` on oracle (a) must not absorb a `stdout_mismatch` on oracle (b): that would
//! launder a genuine second defect into an expected divergence. When a marker exists but does not
//! cover the observation, the verdict falls through to [`Verdict::Finding`] and the detail says
//! exactly which dimension failed to match. All **four** dimensions are matched for **every**
//! divergence this module classifies — a comparison that differed and a build that produced nothing
//! alike; the next section gives the reason the oracle dimension is meaningful for a refusal too.
//!
//! A marker changes how a divergence is **classified**, never whether the feature is
//! **exercised**. Nothing here can short-circuit execution because a marker exists: the
//! applicable phases are attempted in order — compile, link, run, compare — and this module
//! is handed no means of preventing any of them.
//!
//! Classification happens at the **first terminal outcome or the completed comparison**, not
//! after every phase has run. A compile failure, a link failure, a crash and a timeout are
//! terminal outcomes reached before any comparison exists, and each is classified where it
//! occurred: [`build_failure`] is the entry point for those, while [`classify`] is the entry
//! point for a comparison that completed, and both reduce to the one policy in [`judge`].
//! That is why a `compile_failure` marker is meaningful at all — the cell it excuses never
//! reaches a comparison, so a rule that required one would make the class unreachable.
//!
//! # A refusal is matched on every dimension, oracle included
//!
//! Scope matching is **strict on all four dimensions — oracle, target, optimization level and
//! class — for every divergence this module classifies**, whether it came from a comparison that
//! differed or from a build that produced nothing. There is no shape-dependent exemption, because
//! the register states the rule without one, and code that honoured it in one shape while relaxing
//! it in another would leave the two describing different contracts.
//!
//! The oracle dimension is meaningful for a refusal because a refusal reaches this module **once
//! per oracle arm, settled against that arm's own authority** — the same-target reference capture
//! for oracle (a), the baseline capture for oracle (b), the record's own `expected_stdout` for
//! oracle (c). An arm whose authority this environment cannot supply never arrives here: it is
//! reported as an unavailable oracle, loudly and in the summary. An arm the program's own record
//! disables never arrives here either: it is a recorded exclusion. So the arms that do arrive are
//! exactly the arms that held an authority and lost the comparison to a refusal, and naming an
//! oracle in a scope narrows a real set rather than a notional one.
//!
//! What this asks of an author is one word. A marker meant to excuse a build refusal scopes
//! **`all oracles`**, because a refusal denies every arm its subject; the scope grammar has that
//! token, so the intent is written in the register where a reader finds it, instead of being
//! inferable only from this file. A marker that names a single oracle and then meets a refusal is
//! reported as non-covering, the oracle dimension is named among the mismatches, and the detail
//! spells out the `all oracles` remedy — so the corpus is corrected by editing a scope, never by
//! this module quietly deciding a basis covers arms it never mentioned.
//!
//! The relaxed alternative is the dangerous direction, which is why it is not taken: an `oracle_a`
//! marker recording that the reference compiler accepts a construct bcc rejects would, under it,
//! also excuse the cross-backend and golden-record arms — two authorities that basis says nothing
//! whatever about. Staleness detection is unaffected either way: it runs on agreement, through any
//! oracle the marker's scope names, so a refusal that disappears still produces an unexpected
//! success and still fails the run until the marker is retired.
//!
//! # A recorded exclusion is an expected divergence, not a missing oracle
//!
//! A program's own record may narrow which oracles judge it. The worked example is
//! `long double`, whose representation was measured to differ across the four targets — 16,
//! 12, 16 and 16 bytes, x87 80-bit against IEEE binary128 — so cross-backend *value* equality
//! is meaningless for it while the same-target reference comparison and the golden record stay
//! entirely meaningful. Such a cell reaches [`Verdict::XFail`]: a documented expected
//! divergence, citing a covering marker when one names it and otherwise the record's own
//! recorded reason.
//!
//! It is deliberately **not** [`Verdict::Unavailable`], and the distinction is the whole point
//! of that verdict. An unavailable oracle is a statement about the *machine* — a tool nobody
//! installed — which is why `BCC_CONFORMANCE_STRICT` escalates it in continuous integration,
//! where the toolchain is installed on purpose. A narrowing is a statement about the *corpus*:
//! a deliberate authoring decision that the record format refuses to accept without a recorded
//! reason, so an undocumented narrowing cannot reach this module from a parsed record at all.
//! Filing the second as the first would fail a run over a comparison the corpus never asked
//! anyone to make.
//!
//! It is equally not a [`Verdict::Pass`]. Nothing was compared, so no equality is claimed; the
//! cell is counted and listed with its recorded reason printed, which keeps the set of
//! comparisons deliberately not made as visible as the set that was. The one shape that is
//! never excused is a record that *enables* an oracle while the comparison claims exclusion —
//! that removes a comparison the corpus asks for, and it is [`Verdict::Fail`].
//!
//! # Findings are deliverables, not defects to patch
//!
//! A [`Verdict::Finding`] does not fail the run, and the asymmetry against [`Verdict::Fail`]
//! is deliberate. A finding is an *explained* result: an undocumented divergence, delivered
//! as a self-contained artifact directory — a verbatim reproducer, its expectation record, a
//! manifest recording the minimization status, exact reproduction commands, the captured
//! outputs per compiler and per backend, an environment fingerprint and the computed diff —
//! plus an entry in the register at [`FINDINGS_REGISTER`]. The reproducer is a byte-for-byte
//! copy of the corpus program; automated reduction is never performed during a run, and
//! reduction is a supervised activity performed on the copy before a finding is promoted to
//! the curated set. **No compiler source change is ever made in response to one.** A
//! [`Verdict::Fail`] is an *unexplained* result — harness-level breakage, an internal
//! inconsistency, a corpus defect — and it fails the run because nobody can act on a result
//! whose meaning is unknown. An internal error is never allowed to masquerade as a pass.
//!
//! # Environment scope is not compiler scope
//!
//! A static link that fails because a target's C runtime was never installed has exactly the
//! shape of a link failure caused by a code-generation defect. Reporting the first as the
//! second would manufacture false findings on a modestly provisioned machine, so an
//! observation the caller attributes to the environment — [`Attribution::Environment`],
//! taken from the build layer's own scope judgement — becomes [`Verdict::Unavailable`] with
//! an explicit environment-scope note rather than a finding. A missing emulator removes a
//! target from oracle (b) and from oracle (a)'s cross arm while the other targets continue; a
//! missing native reference compiler removes oracle (a) entirely while oracles (b) and (c)
//! continue. An absent **compiler under test** is not this module's business at all: it is a
//! hard failure `env.rs` raises before any cell runs, because a suite that cannot compile
//! anything must say so immediately rather than classify 1,296 cells as unavailable.
//!
//! # Invariants callers may rely on
//!
//! - Only `std` is used. The project permits no third-party crate, and this module needs
//!   none: the whole of it is enumerations, borrows and formatted text.
//! - The compiler-unchecked keyword does not appear anywhere in this file.
//! - Every function is pure. No clock, no environment read, no randomness, no counter, no
//!   global mutable state: identical inputs always produce the same verdict and the same
//!   message, byte for byte, so a report diffed between two runs shows only real change and
//!   the fourteen feature-area tests may classify concurrently without a lock. The verdict
//!   policy does read configuration, but from the validated snapshot [`RunConfig`] holds
//!   rather than from the environment, so two concurrent areas cannot apply different
//!   policies.
//! - [`class_significance`] matches [`DivergenceClass`] exhaustively with **no wildcard arm**,
//!   and [`verdict_fails_run`] matches [`Verdict`] the same way. A seventh class or a seventh
//!   verdict is therefore a compile error until it is handled here, which is the mechanical
//!   guarantee that the verdict space stays closed.
//! - This module declares no test function and no `main.rs` sits beside it, so Cargo treats
//!   the directory as a plain module directory rather than a test target, the package manifest
//!   needs no change, and the suite's own test count does not move — which is one of the
//!   mechanical checks that no existing test was skipped or weakened.
//!
//! Edition 2021, minimum supported Rust 1.70.

use std::fmt;

use super::compare::Comparison;
use super::compile::FailureScope;
use super::env::{
    Capabilities, RunConfig, VAR_ALLOW_MISSING_ORACLES, VAR_ALLOW_XPASS, VAR_QEMU_AARCH64,
    VAR_QEMU_I386, VAR_QEMU_RISCV64, VAR_REF_CC, VAR_REF_CC_AARCH64, VAR_REF_CC_I686,
    VAR_REF_CC_RISCV64, VAR_STRICT,
};
use super::manifest::{ExpectedDivergence, Manifest};
use super::{shown_path, CellKey, DivergenceClass, HarnessError, Oracle, Outcome, Target, Verdict};

/// Repository-relative path of the expected-divergence register.
///
/// Named here because this module is what tells a maintainer to retire a stale marker, and an
/// instruction to "retire the marker" that does not say where is not actionable. A marker
/// lives in two places — the program's own `.expected` record and this register — and the
/// infrastructure test that audits them asserts the correspondence in both directions, so a
/// marker retired in one place and left in the other is still stale documentation.
pub const EXPECTED_DIVERGENCE_REGISTER: &str = "tests/conformance/EXPECTED_DIVERGENCES.md";

/// Repository-relative path of the findings register.
///
/// Every finding is indexed here, alongside its committed artifact directory, so the set of
/// undocumented divergences this suite has produced is auditable in one place rather than
/// scattered across build output that a later run overwrites.
pub const FINDINGS_REGISTER: &str = "tests/conformance/FINDINGS.md";

/// Longest run of characters of a recorded free-text reason placed in an outcome detail.
///
/// An [`Outcome`] renders as exactly one line of the verdict table and of the tab-separated
/// summary, and a recorded reason is free text a maintainer wrote into an expectation record —
/// several sentences, deliberately. The full text always remains available in the record
/// itself and in the comparison's own multi-line detail; this bound applies only to the
/// one-line form. It is a constant rather than a function of the input so that the same
/// inputs always produce the same text.
const MAX_INLINE_REASON_CHARS: usize = 240;

/// Announcement appended when a reason is shortened for the one-line form.
///
/// Truncation is always announced, never silent: a reader who cannot tell that text was
/// dropped cannot tell whether the reason they are reading is the whole reason.
const INLINE_REASON_TRUNCATION: &str = " [...truncated; the full reason is in the record]";

/// Who is answerable for an observation the build layer reported.
///
/// This is a deliberate local mirror of the build layer's own scope judgement rather than a reuse
/// of it. The mirror keeps the classifier honest about where the judgement was made: the decision
/// that a link failure is the machine's fault and not the compiler's is taken where the evidence
/// is, by the layer that ran the compiler and read its diagnostics, and this module consumes that
/// decision rather than second-guessing it. It also lets an observation be constructed by a caller
/// that never ran a compiler at all — a harness inconsistency, an absent oracle — without
/// inventing a build failure to carry the scope.
///
/// # Why the build layer's scope type is nevertheless imported
///
/// A mirror is only safe while the two sides cannot disagree, and the one thing that guarantees
/// that is a `match` the compiler checks for exhaustiveness. [`Attribution::of_scope`] is
/// that `match`: adding a further scope to the build layer stops this file compiling until the
/// policy says what the new scope means, which is exactly the failure mode a hand-written
/// conversion invites — a scope silently folded into the nearest existing one, at the cost of
/// either a manufactured finding or a discarded defect. The import buys that check and nothing
/// else; the policy in this file is still written in terms of [`Attribution`] alone.
///
/// Callers holding a build failure convert with [`Attribution::of_scope`], passing that failure's
/// own [`FailureScope`] so no call site has to reconstruct the mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Attribution {
    /// The observation is a property of the compiler and the program, and is therefore a
    /// candidate for a finding or for an expected divergence.
    Compiler,
    /// The observation is a property of this machine — a target's C runtime is absent, or a
    /// driver installation cannot run its own stages. Reported as an environment gap, never
    /// as a defect in a compiler that was not given the inputs it needed.
    Environment,
    /// Who is answerable could not be determined, because the build layer could not capture the
    /// evidence its attribution would have rested on.
    ///
    /// Mirrors the build layer's own third scope. It exists so that this module is never handed a
    /// two-way choice over a three-way judgement: a boolean conversion would have to fold the
    /// unattributable case into one of the other two, and both foldings are wrong in ways that
    /// cost something real — one manufactures a finding against a compiler on the strength of text
    /// nobody read, the other silently discards a possible defect as a fact about the machine.
    ///
    /// Judged as a reported gap: neither a pass, nor an accusation, nor a silent skip. It is
    /// surfaced through the same [`Verdict::Unavailable`] path an absent oracle uses, because the
    /// two situations are the same situation — a comparison the run could not soundly make — and
    /// that path is already the one the summary reports loudly and that strict mode refuses to
    /// tolerate.
    Indeterminate,
}

impl Attribution {
    /// Mirror the build layer's own scope, exhaustively.
    ///
    /// The only conversion offered, and deliberately so: it is a total `match` over
    /// [`FailureScope`], never a boolean. A boolean cannot carry this judgement, because "is the
    /// machine answerable" is `false` both for a compiler defect and for a failure whose
    /// answerable party is *unknown*, and a caller reading the two as one promotes an
    /// unattributable refusal to [`Attribution::Compiler`] — a manufactured finding against a
    /// compiler on the strength of diagnostics nobody read, which is the most expensive mistake
    /// this module can make: it manufactures a false deliverable on a machine that is merely
    /// modestly provisioned.
    ///
    /// The `match` buys a compiler-checked guarantee instead: a scope added to the build layer
    /// stops this file compiling until the policy states what the new scope means.
    ///
    /// # The harness's own scope is not an observation about anything
    ///
    /// [`FailureScope::Harness`] means a compilation was launched and its result could not be
    /// established, so nothing was observed about either the compiler or the machine. A caller
    /// holding a build failure must therefore ask the build layer's own harness predicate
    /// (`CompileOutcome::is_harness_failure`) **first** and route such a failure to
    /// [`internal_error`], which fails the run rather than reporting a gap. This conversion still
    /// has to answer for the case, because an exhaustive `match` is the whole point of it, and it
    /// answers [`Attribution::Indeterminate`] — the only honest reading left, neither a pass nor an
    /// accusation — so a caller that forgot the routing loses the harness diagnostic's severity but
    /// never manufactures a finding against a compiler.
    pub fn of_scope(scope: FailureScope) -> Attribution {
        match scope {
            FailureScope::Compiler => Attribution::Compiler,
            FailureScope::Environment => Attribution::Environment,
            FailureScope::Indeterminate => Attribution::Indeterminate,
            FailureScope::Harness => Attribution::Indeterminate,
        }
    }

    /// The token used in reports and details.
    pub fn label(self) -> &'static str {
        match self {
            Attribution::Compiler => "compiler scope",
            Attribution::Environment => "environment scope",
            Attribution::Indeterminate => "indeterminate scope",
        }
    }
}

impl fmt::Display for Attribution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Whether a divergence was observed by one oracle's comparison, or by a refusal that removed
/// the authority one oracle arm needed.
///
/// # What this distinction does, and what it deliberately does not do
///
/// It decides **which account the outcome detail gives**, and nothing else. Marker matching is
/// uniformly [`covers`] — strict on all four dimensions, oracle included — for both shapes.
///
/// # Why the oracle dimension is matched for a refusal too
///
/// A single build refusal reaches the classifier once per oracle arm, not once per cell, and each
/// arm is settled against its own authority: the same-target reference capture for oracle (a), the
/// baseline capture for oracle (b), the record's own `expected_stdout` for oracle (c). An arm whose
/// authority is absent — a reference driver this environment does not have, an emulator it does not
/// have, an oracle the record itself disables — never reaches this classification at all; it is
/// reported as unavailable or as a recorded exclusion by the driver. So the arms that do reach here
/// are exactly the arms that had an authority and lost the comparison to a refusal, and naming an
/// oracle in a marker's scope narrows a real set.
///
/// The register states the matching rule in one sentence — a marker excuses a divergence only when
/// its scope covers this oracle **and** this target **and** this optimization level **and** its
/// class equals the class observed — and states it without carving out a shape. Honouring it for a
/// comparison while relaxing it for a refusal would make the code and the register describe
/// different contracts, and the relaxed direction is the dangerous one: an `oracle_a` marker
/// documenting that the reference compiler accepts a construct bcc rejects would silently also
/// excuse the cross-backend and golden-record arms, which that basis says nothing about.
///
/// # What an author must therefore write
///
/// A marker intended to excuse a build refusal must scope **`all oracles`**, because a refusal
/// denies every arm its subject. The scope grammar has that token, so the requirement costs one
/// word rather than a code exemption, and it makes the intent legible in the register instead of
/// inferable only from this file. A marker that names one oracle and then meets a refusal is
/// reported as non-covering, with the oracle dimension named among the mismatches and the
/// `all oracles` remedy spelled out in the detail — see [`marker_non_coverage`].
///
/// Staleness detection is unaffected: if the refusal disappears and the comparisons agree,
/// [`scope_marker`] finds the marker on any oracle its scope names and the run fails with an
/// unexpected success until the marker is retired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DivergenceShape {
    /// One oracle compared two observations and they differed.
    Compared,
    /// A build produced no artifact, so this oracle arm lost the subject of its comparison.
    Refused,
}

/// Everything the harness can observe about one cell under one oracle.
///
/// # Why the input is a closed enumeration
///
/// The six-step decision order has to be applied to *every* situation, and three of those
/// situations carry no comparison at all: an oracle whose tooling is absent never produced
/// one, a program that failed to build never reached execution, and harness-level breakage
/// never reached either. If each of those were handled by its own entry point with its own
/// private ordering, the policy would live in four places and could drift in three of them —
/// and the most likely drift is precisely the dangerous one, where a situation nobody
/// enumerated quietly produces no outcome.
///
/// Enumerating the inputs instead means [`judge`] is a single total function over them: every
/// shape the harness can be in appears in one `match`, the ordering is auditable in one
/// reading, and a shape added later cannot compile until the policy says what it means.
///
/// The variants borrow rather than own, because every one of them is built at a call site that
/// already holds the evidence and is discarded as soon as the verdict exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation<'a> {
    /// An oracle could not be attempted because tooling it needs is genuinely absent from
    /// this environment.
    ///
    /// `diagnosis` must name the tool and the environment variable that would supply it —
    /// [`unavailable_oracle`] builds one from the capability record. This is never a skip and
    /// never a pass: it becomes [`Verdict::Unavailable`], is listed in the summary, and fails
    /// the run under the strict setting intended for continuous integration.
    ToolingAbsent {
        /// Why the arm cannot be attempted, naming the tool and its override variable.
        diagnosis: &'a str,
    },
    /// A comparison was performed, or was deliberately not attempted because the program's
    /// own expectation record narrows its oracle coverage.
    ///
    /// Both cases arrive here because both are judgements about the same three-state
    /// [`Comparison`]: it agreed, it diverged, or it was excluded with a recorded reason.
    Compared(&'a Comparison),
    /// A build did not produce a usable artifact, so no execution and no comparison happened.
    ///
    /// `attribution` decides everything: [`Attribution::Environment`] makes this an
    /// environment gap rather than an observation about a compiler, while
    /// [`Attribution::Compiler`] puts it through the same marker logic a diverging comparison
    /// goes through — which is what lets a documented unimplemented extension be classified as
    /// an expected divergence rather than as a finding.
    Build {
        /// The shape of the failure, from the closed set.
        class: DivergenceClass,
        /// Whether the compiler or this machine is answerable.
        attribution: Attribution,
        /// One line naming what happened, from the build layer that read the diagnostics.
        summary: &'a str,
    },
    /// Something happened that is neither a classified divergence nor a missing oracle.
    ///
    /// A program with no sibling expectation record, a corpus entry that could not be read, a
    /// workspace that could not be created, an argument vector that failed the forbidden-flag
    /// guard, or an internal inconsistency in the harness itself. It becomes
    /// [`Verdict::Fail`], because a result whose meaning is unknown cannot be acted on — and
    /// because an internal error that presented as a pass would be the one defect in this
    /// suite capable of hiding every other.
    Unexplained {
        /// What the harness was attempting, phrased as a gerund.
        context: &'a str,
        /// What went wrong, in enough detail to act on without re-running.
        cause: &'a str,
    },
}

/// Decide the verdict for one observation about one cell under one oracle.
///
/// This is the whole policy, in the order documented at the top of this module, in one
/// function so that it can be audited in a single reading. Every path returns an [`Outcome`];
/// none returns nothing, and none can drop a cell.
///
/// `manifest` is the program's own expectation record, and it is optional for exactly one
/// reason: two of the mandated unexplained cases are *a program whose record could not be
/// loaded at all*. Where a record is absent, no marker can be consulted, so no divergence can
/// be excused and no unexpected success can be detected; being handed an observation that
/// needs a marker without a record is itself an inconsistency, and it is reported as
/// [`Verdict::Fail`] rather than being quietly treated as "no marker", which would silently
/// convert every expected divergence in that program into a finding.
pub fn judge(
    observation: &Observation<'_>,
    manifest: Option<&Manifest>,
    key: &CellKey,
    oracle: Oracle,
) -> Outcome {
    // Steps 1, 2, 3 and 6 conclude here; the divergence steps 4 and 5 are shared by a
    // diverging comparison and a build failure the caller attributed to the compiler, so both
    // reduce to a class, one line of evidence and the shape the divergence took, then fall
    // through to the marker decision below. Reducing rather than duplicating is what keeps one
    // policy in one place; the shape is what lets that one policy match a marker correctly for
    // both, since a refusal and a comparison are answerable to different dimensions.
    let (class, evidence, shape) = match observation {
        // Step 1: an oracle's tooling is genuinely absent.
        Observation::ToolingAbsent { diagnosis } => {
            return unavailable_tooling_outcome(key, oracle, diagnosis)
        }

        // Step 1, environment scope: the machine, not the compiler, is answerable.
        Observation::Build {
            class,
            attribution: Attribution::Environment,
            summary,
        } => return unavailable_environment_outcome(key, oracle, *class, summary),

        // Step 1, indeterminate scope: nobody could be shown answerable, because the evidence the
        // attribution rests on was not captured whole. Reported on the same unavailable path as an
        // absent oracle, for the same reason — the run could not soundly make this comparison —
        // and never as a pass or a finding.
        Observation::Build {
            class,
            attribution: Attribution::Indeterminate,
            summary,
        } => return unavailable_indeterminate_outcome(key, oracle, *class, summary),

        // Step 6: nothing about this is a divergence or a missing oracle.
        Observation::Unexplained { context, cause } => {
            return fail_outcome(key, oracle, context, cause)
        }

        // A build failure the caller attributed to the compiler and the program.
        Observation::Build {
            class,
            attribution: Attribution::Compiler,
            summary,
        } => (*class, *summary, DivergenceShape::Refused),

        Observation::Compared(comparison) => {
            // A comparison attributed to a different oracle than the one being judged would
            // file this cell's verdict under the wrong authority, so it is an inconsistency
            // rather than a comparison — step 6.
            if comparison.oracle != oracle {
                return fail_outcome(
                    key,
                    oracle,
                    &format!("classifying a comparison for {key}"),
                    &format!(
                        "the comparison was produced by {} but is being judged as {oracle}; an \
                         outcome filed under the wrong oracle would attribute a divergence to an \
                         authority that never made the comparison",
                        comparison.oracle
                    ),
                );
            }
            // The record is what carries the markers, so judging any comparison without it
            // would silently turn every expected divergence in this program into a finding.
            let Some(manifest) = manifest else {
                return fail_outcome(
                    key,
                    oracle,
                    &format!("classifying a comparison for {key}"),
                    "no expectation record accompanied the comparison, so no expected-divergence \
                     marker could be consulted; without the record a divergence cannot be \
                     excused and an unexpected success cannot be detected, and treating that as \
                     \"no marker\" would silently reclassify every expected divergence in this \
                     program as a finding",
                );
            };
            if let Some(inconsistency) = comparison_inconsistency(comparison, manifest, key, oracle)
            {
                return fail_outcome(
                    key,
                    oracle,
                    &format!("classifying a comparison for {key}"),
                    &inconsistency,
                );
            }

            if let Some(reason) = &comparison.excluded {
                // Not attempted, because the program's own record narrows its oracle coverage.
                // Nothing was measured, so there is no observed class for a marker to agree
                // with: the marker is matched on scope alone, and matching it on a class the
                // comparison never produced would reject every legitimate narrowing.
                return match scope_marker(manifest, key, oracle) {
                    Some(marker) => xfail_exclusion_outcome(key, oracle, manifest, marker, reason),
                    None => xfail_recorded_exclusion_outcome(key, oracle, manifest, reason),
                };
            }

            if comparison.equal {
                // Steps 2 and 3: the two observations agree. Whether that is a pass or an
                // unexpected success depends only on whether a marker claims a divergence
                // here, so the class is irrelevant and the scope alone decides.
                return match scope_marker(manifest, key, oracle) {
                    None => pass_outcome(key, oracle, &comparison.summary),
                    Some(marker) => {
                        xpass_outcome(key, oracle, manifest, marker, &comparison.summary)
                    }
                };
            }

            match comparison.class {
                Some(class) => (
                    class,
                    comparison.summary.as_str(),
                    DivergenceShape::Compared,
                ),
                // A comparison that is neither equal, nor excluded, nor classified breaks the
                // comparator's own documented invariant. Reporting it as an agreement would
                // turn a real divergence into a pass, so it is step 6.
                None => {
                    return fail_outcome(
                        key,
                        oracle,
                        &format!("classifying a comparison for {key}"),
                        &format!(
                            "the comparison reports neither equality, nor an exclusion, nor a \
                             divergence class, so what it observed cannot be established; its \
                             own summary reads: {}",
                            comparison.summary
                        ),
                    )
                }
            }
        }
    };

    // Steps 4 and 5: a divergence attributed to the compiler and the program.
    let Some(manifest) = manifest else {
        return fail_outcome(
            key,
            oracle,
            &format!("classifying a {class} divergence for {key}"),
            &format!(
                "no expectation record accompanied the observation, so no expected-divergence \
                 marker could be consulted; the divergence itself was: {evidence}"
            ),
        );
    };
    // Marker matching does not depend on the shape of the divergence: every observation that
    // reaches this point was made under one oracle arm, against that arm's own authority, so all
    // four dimensions — oracle, target, optimization level and class — narrow a real set and all
    // four are matched strictly. `shape` decides only which account the detail gives. See
    // [`DivergenceShape`] for why a refusal is no exception, and what an author writes instead.
    let marker = covering_marker(manifest, key, oracle, class);
    match marker {
        // Step 4: a marker documents this divergence.
        Some(marker) => xfail_divergence_outcome(key, oracle, class, marker, evidence, shape),
        // Step 5: no marker covers it, so it is an undocumented divergence — a finding.
        None => finding_outcome(
            key,
            oracle,
            class,
            evidence,
            marker_non_coverage(manifest, key, oracle, class, shape),
        ),
    }
}

/// Decide the verdict for one comparison — the entry point the driver uses for every cell.
///
/// A thin, deliberate wrapper: it exists so that the ordinary case reads as one call, while
/// the policy it applies stays in [`judge`] alongside every other case. The record is
/// mandatory here because a comparison always has one — the cell was resolved from it.
pub fn classify(
    comparison: &Comparison,
    manifest: &Manifest,
    key: &CellKey,
    oracle: Oracle,
) -> Outcome {
    judge(
        &Observation::Compared(comparison),
        Some(manifest),
        key,
        oracle,
    )
}

/// Record that one oracle arm cannot be attempted here, naming the tool and the environment
/// variable that would supply it.
///
/// This is the entry point for the first step of the decision order, and it is given the whole
/// capability record rather than a prepared message so that the diagnosis is derived from the
/// same probe results the pre-flight report prints. A maintainer therefore reads the same
/// explanation in the summary and in the verdict row.
///
/// # Two situations this refuses to dress up as an unavailable oracle
///
/// Both are caller defects, and both are reported as [`Verdict::Fail`] rather than
/// [`Verdict::Unavailable`], because an unavailable arm is a statement about the *environment*
/// and neither of these is:
///
/// - **An arm that structurally does not exist.** Oracle (b) compares every other target
///   against the baseline, so the baseline has no cross-backend comparison to make. Reporting
///   that as unavailable would make the strict setting fail a run over a comparison nobody
///   asked for. Ask [`oracle_applies`] first.
/// - **An arm the capability record says is available.** Recording it as unavailable would
///   remove a comparison the matrix asks for while the run still reported green, which is
///   exactly the silent gap this suite exists to prevent.
pub fn unavailable_oracle(caps: &Capabilities, key: &CellKey, oracle: Oracle) -> Outcome {
    let target = key.target();
    if !oracle_applies(oracle, target) {
        return unexplained(
            key,
            oracle,
            &format!("recording an unavailable {oracle} arm for {key}"),
            &format!(
                "{baseline} is the cross-backend baseline, so there is no cross-backend \
                 comparison to make for it: every other target is compared against it. An arm \
                 that structurally does not exist is not an unavailable oracle, and reporting it \
                 as one would let the strict setting fail a run over a comparison the matrix \
                 never contained. Call classify::oracle_applies before classifying an arm.",
                baseline = Target::BASELINE
            ),
        );
    }
    if caps.oracle_available(oracle, target) {
        return unexplained(
            key,
            oracle,
            &format!("recording an unavailable {oracle} arm for {key}"),
            "the capability record reports this arm as available, so recording it as unavailable \
             would remove a comparison the matrix asks for while the run still reported green; \
             either the arm was attempted and should be classified from its comparison, or its \
             tooling became unavailable after discovery, which is a condition to report rather \
             than to assume",
        );
    }
    let diagnosis = oracle_diagnosis(caps, target, oracle);
    judge(
        &Observation::ToolingAbsent {
            diagnosis: &diagnosis,
        },
        None,
        key,
        oracle,
    )
}

/// Decide the verdict for a build that produced no usable artifact.
///
/// `class`, `attribution` and `summary` are the three halves of the build layer's own judgement
/// — a rejected program, a program that translated but did not link, or an invocation that
/// outlived its budget; whether the compiler or this machine is answerable; and one line saying
/// what happened. Convert that layer's environment predicate with
/// [`Attribution::of_scope`] rather than deciding attribution here: the decision belongs
/// where the compiler's diagnostics were read. A build failure the build layer scoped to itself
/// belongs to neither attribution and must reach [`internal_error`] instead, as
/// [`Attribution::of_scope`] records.
///
/// An [`Attribution::Environment`] build failure becomes [`Verdict::Unavailable`] at
/// environment scope. An [`Attribution::Compiler`] one goes through the same marker logic a
/// diverging comparison goes through, which is what lets a compile failure on a limitation the
/// repository **explicitly documents** be an expected divergence instead of a finding — and,
/// where no such documentation exists, keeps it a finding, because a marker may not be minted
/// on the strength of an omission from an inventory.
pub fn build_failure(
    manifest: &Manifest,
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    attribution: Attribution,
    summary: &str,
) -> Outcome {
    judge(
        &Observation::Build {
            class,
            attribution,
            summary,
        },
        Some(manifest),
        key,
        oracle,
    )
}

/// Record something that is neither a classified divergence nor an absent oracle.
///
/// The catch-all sixth step, for harness-level breakage: a program with no sibling expectation
/// record, a corpus entry that could not be read, a workspace that could not be created, an
/// argument vector that failed the forbidden-flag guard, an internal inconsistency. It takes no
/// expectation record, because the most common reason to reach it is that there was none to
/// take.
///
/// `context` says what the harness was attempting, phrased as a gerund; `cause` says what went
/// wrong, in enough detail to act on without re-running.
pub fn unexplained(key: &CellKey, oracle: Oracle, context: &str, cause: &str) -> Outcome {
    judge(
        &Observation::Unexplained { context, cause },
        None,
        key,
        oracle,
    )
}

/// Record a harness failure that already carries its own context and cause.
///
/// The adapter for the question-mark operator: a fallible step inside a cell — resolving a
/// path, rendering a command template, creating a workspace — yields a [`HarnessError`], and
/// the cell must still reach a verdict rather than aborting the area that contains it. The
/// error's two halves map straight onto [`unexplained`], so the message a reader sees is the
/// one the failing step wrote.
pub fn internal_error(key: &CellKey, oracle: Oracle, error: &HarnessError) -> Outcome {
    unexplained(key, oracle, error.context(), error.cause())
}

/// Whether an oracle arm exists at all for a target.
///
/// False for exactly one combination: cross-backend comparison on [`Target::BASELINE`], which
/// is the authority every other target is compared against and therefore has nothing to be
/// compared with. A driver asks this before classifying an arm, so that a comparison the matrix
/// never contained is neither counted as a pass — the baseline against itself can only ever
/// agree, so such a pass would be indistinguishable from real coverage — nor reported as an
/// unavailable oracle, which the strict setting would escalate to a failure.
///
/// This is not an exemption from the no-silent-skip rule. The cell is still judged, in full, by
/// the oracles that do apply to it: its same-target reference comparison and its golden record.
pub fn oracle_applies(oracle: Oracle, target: Target) -> bool {
    match oracle {
        Oracle::CrossBackend => target != Target::BASELINE,
        Oracle::ReferenceCompiler | Oracle::GoldenRecord => true,
    }
}

/// Whether a marker excuses a divergence of this class, for this cell, under this oracle.
///
/// All four dimensions must agree: the marker's scope must cover the oracle, the target and the
/// optimization level, **and** its class must equal the class observed. Scope matching is
/// strict on purpose. A marker for `compile_failure` on oracle (a) must not absorb a
/// `stdout_mismatch` on oracle (b): the two are different defects, one of them documented and
/// one of them not, and widening a marker to cover both would launder a genuine second defect
/// into an expected divergence while the register still described only the first.
///
/// The scope half is [`ExpectedDivergence::covers`], which deliberately does not re-check which
/// program the cell belongs to — a marker is reachable only through the record of the program it
/// governs, so program identity is already established by the time a caller holds one. This
/// module verifies that assumption rather than relying on it: a record whose own program does
/// not match the cell being judged is reported as an inconsistency, because consulting another
/// program's marker is precisely how a divergence could be excused by documentation that was
/// never about it.
pub fn covers(
    marker: &ExpectedDivergence,
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
) -> bool {
    marker.class() == class && marker.covers(key, oracle)
}

/// The marker that excuses a divergence of this class for this cell and oracle, if any.
///
/// `None` means the divergence is undocumented, which is what makes it a finding.
pub fn covering_marker<'a>(
    manifest: &'a Manifest,
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
) -> Option<&'a ExpectedDivergence> {
    manifest
        .marker()
        .filter(|marker| covers(marker, key, oracle, class))
}

/// What one divergence class means, in one sentence, for the outcome detail.
///
/// The `match` is exhaustive with **no wildcard arm**, and that is load-bearing rather than
/// stylistic: a seventh divergence class cannot compile until this function says what it means,
/// which is the mechanical guarantee that the class set stays closed and the classification
/// table stays total. A wildcard arm here would let a new class be added and silently inherit
/// somebody else's explanation.
pub fn class_significance(class: DivergenceClass) -> &'static str {
    match class {
        DivergenceClass::CompileFailure => {
            "one compiler rejected a program the other accepted, so the two disagree about what \
             the language permits; the diagnostics are captured for the artifacts and are never \
             compared, because diagnostic wording legitimately differs between compilers"
        }
        DivergenceClass::LinkFailure => {
            "the program translated but did not link, which is a compiler defect only when the \
             linker was given every input it needed — the same shape arises when a target's C \
             runtime is absent, and that case belongs to the environment rather than to the \
             compiler"
        }
        DivergenceClass::RunCrash => {
            "the program died on a signal instead of exiting, which is why exit status is \
             compared as a raw wait status: a crash must never be conflated with a numerically \
             equal ordinary exit"
        }
        DivergenceClass::ExitCodeMismatch => {
            "both runs completed and disagreed on their exit status, so the two binaries \
             reached different conclusions about the same program"
        }
        DivergenceClass::StdoutMismatch => {
            "both runs completed and their stdout bytes differ, which is the ordinary shape of a \
             wrong answer and the reason every program prints one line per semantic property it \
             claims"
        }
        DivergenceClass::Timeout => {
            "execution outlived its per-cell budget, which is a divergence rather than an \
             infrastructure error: a program that finishes promptly under one compiler and hangs \
             under another is exactly the defect this suite exists to surface"
        }
    }
}

/// Whether a verdict fails the run under this configuration.
///
/// This is the one place the two configurable policies are applied, so they can be changed in
/// one place and cannot drift between the driver and the reporter:
///
/// - [`Verdict::XPass`] fails **by default**, and `BCC_CONFORMANCE_ALLOW_XPASS` downgrades it
///   to a warning for a marker-retirement window.
/// - [`Verdict::Unavailable`] fails only under `BCC_CONFORMANCE_STRICT`, the intended
///   continuous-integration setting. Strict mode is dominant: an acknowledgement through
///   `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` annotates the report and lowers nothing, because a
///   job that demanded strictness while excusing the arms it could not attempt would report a
///   green run over a matrix it never executed.
/// - [`Verdict::Finding`] does **not** fail the run: a finding is a deliverable, reported
///   loudly and counted, not a defect to patch.
///
/// With no configuration set this agrees exactly with [`Verdict::fails_run`], which states the
/// default policy without consulting the environment. The `match` is exhaustive with no
/// wildcard arm, so a seventh verdict cannot compile until its run-failing status is decided
/// here.
pub fn verdict_fails_run(verdict: Verdict, config: &RunConfig) -> bool {
    match verdict {
        Verdict::Pass | Verdict::XFail | Verdict::Finding => false,
        Verdict::Fail => true,
        Verdict::XPass => config.xpass_fails_run(),
        Verdict::Unavailable => config.unavailable_fails_run(),
    }
}

/// Whether one outcome fails the run under this configuration.
pub fn fails_run(outcome: &Outcome, config: &RunConfig) -> bool {
    verdict_fails_run(outcome.verdict(), config)
}

/// Every outcome carrying one verdict, in the order they were accumulated.
///
/// Order is preserved rather than sorted, because an area accumulates its cells in the
/// deterministic order its record declares them, and a report a maintainer diffs between two
/// runs should show only real change.
pub fn select_by_verdict(outcomes: &[Outcome], verdict: Verdict) -> Vec<&Outcome> {
    outcomes
        .iter()
        .filter(|outcome| outcome.verdict() == verdict)
        .collect()
}

/// Every unexpected success, for the separate and prominent listing the summary must carry.
///
/// Named rather than left to a filter at each call site because the requirement is not merely
/// that these outcomes exist somewhere in the table: an unexpected success means a marker is
/// stale, and a stale marker misleads every future reader until somebody retires it. It is
/// listed separately whether or not `BCC_CONFORMANCE_ALLOW_XPASS` is downgrading it to a
/// warning, so the escape hatch can never make one pass unnoticed.
pub fn unexpected_successes(outcomes: &[Outcome]) -> Vec<&Outcome> {
    select_by_verdict(outcomes, Verdict::XPass)
}

/// Every outcome that fails the run under this configuration, in accumulation order.
///
/// The driver asserts on this after accumulating a whole area, rather than asserting cell by
/// cell, because the deliverable summary must enumerate every outcome and stopping at the first
/// divergence would truncate exactly the artifact the requirements ask for.
pub fn run_failures<'a>(outcomes: &'a [Outcome], config: &RunConfig) -> Vec<&'a Outcome> {
    outcomes
        .iter()
        .filter(|outcome| fails_run(outcome, config))
        .collect()
}

// Everything below is internal: the marker lookup that ignores the class, the consistency
// checks that make step six total, the per-verdict detail builders, and the small renderers
// they share. They are private because each one is meaningful only as part of the policy above —
// a caller able to build an outcome directly could record a verdict the decision order never
// reached, which is the one way this module's totality could be circumvented.

/// The marker whose scope covers this cell and oracle, whatever class it claims.
///
/// Two situations need the scope answer alone, and in both of them there is no observed class to
/// compare against: an agreement, where a covering marker means the marker is **stale** — it
/// documents a divergence that is no longer there — and an exclusion, where nothing was measured
/// at all. Requiring class agreement in either would be asking a question the observation cannot
/// answer, and would make every stale marker undetectable, which is the one way a marker set
/// decays into documentation nobody can trust.
///
/// This is also the staleness test in full. There is deliberately no separate predicate returning
/// only whether a marker is stale: both callers need the marker itself — one to name it in the
/// unexpected-success outcome, the other to cite it in the exclusion outcome — and a bool-returning
/// wrapper would let a caller establish that a marker is stale without being able to say which one,
/// which is exactly the report a maintainer cannot act on. Retirement happens in two places, the
/// program's own record and the register the suite ships beside the corpus, and an unexpected
/// success fails the run by default so that it actually happens.
fn scope_marker<'a>(
    manifest: &'a Manifest,
    key: &CellKey,
    oracle: Oracle,
) -> Option<&'a ExpectedDivergence> {
    manifest
        .marker()
        .filter(|marker| marker.covers(key, oracle))
}

/// Why this program's marker does not cover the observed divergence, when it has one that does
/// not.
///
/// `None` when the program has no marker at all — the finding detail already says that no marker
/// explains the divergence, and repeating it would add nothing — and `None` when the marker does
/// cover the observation, which the finding path never reaches.
///
/// Every dimension that fails to match is named, in a fixed order, so that two runs produce
/// byte-identical text and a reader is told the whole reason rather than the first part of it. A
/// maintainer reading this row can see immediately whether the honest fix is a second marker, a
/// widened scope that the register also documents, or a finding.
///
/// The `shape` decides only whether the closing sentence names the `all oracles` remedy. Every
/// dimension is matched for both shapes — see [`DivergenceShape`] — so every dimension that failed
/// is reported for both, and a maintainer is never sent to widen a scope while a second mismatch
/// they were not shown would still have produced a finding.
fn marker_non_coverage(
    manifest: &Manifest,
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    shape: DivergenceShape,
) -> Option<String> {
    let marker = manifest.marker()?;
    if covers(marker, key, oracle, class) {
        return None;
    }
    let mut mismatches: Vec<String> = Vec::new();
    if marker.class() != class {
        mismatches.push(format!(
            "it documents class {} while a {class} was observed",
            marker.class()
        ));
    }
    if !marker.scope().oracles().contains(&oracle) {
        mismatches.push(match shape {
            DivergenceShape::Compared => format!(
                "its scope covers {} while this comparison was made by {oracle}",
                joined(marker.scope().oracles())
            ),
            DivergenceShape::Refused => format!(
                "its scope covers {} while this observation was made by {oracle}",
                joined(marker.scope().oracles())
            ),
        });
    }
    if !marker.scope().targets().contains(&key.target()) {
        mismatches.push(format!(
            "its scope covers targets {} while this cell is {}",
            joined(marker.scope().targets()),
            key.target()
        ));
    }
    if !marker.scope().opt_levels().contains(&key.opt()) {
        mismatches.push(format!(
            "its scope covers optimization levels {} while this cell is at {}",
            joined(marker.scope().opt_levels()),
            key.opt()
        ));
    }
    let dimensions = match shape {
        DivergenceShape::Compared => "",
        DivergenceShape::Refused => {
            " No artifact was produced for this cell, so this oracle arm lost the subject of its \
             comparison. A refusal denies every arm its subject, so a marker meant to document one \
             must scope `all oracles`; a marker naming a single oracle documents that oracle's \
             comparison and nothing else."
        }
    };
    Some(format!(
        "Marker {} is present in this program's record but does not cover this observation: {}.\
         {dimensions} A marker is never widened to absorb a divergence it does not describe, \
         because that would launder a genuine second defect into an expected divergence while {} \
         still documented only the first; if this divergence is also documented, it needs its own \
         marker in both places.",
        marker.id(),
        joined(&mismatches),
        EXPECTED_DIVERGENCE_REGISTER
    ))
}

/// The first way, in a fixed order, in which a comparison contradicts itself or the record that
/// governs it.
///
/// Every one of these means the harness or the corpus is inconsistent rather than that a
/// compiler misbehaved, and every one of them could otherwise be read as a result: a comparison
/// that claims equality while carrying a divergence class, an exclusion that claims to have
/// compared something, a record consulted for a different program, an oracle the record disables
/// being compared anyway, an oracle the record enables being recorded as excluded, or a
/// cross-backend comparison of the baseline against itself. The order is fixed so that a
/// doubly-inconsistent value always reports the same defect first.
fn comparison_inconsistency(
    comparison: &Comparison,
    manifest: &Manifest,
    key: &CellKey,
    oracle: Oracle,
) -> Option<String> {
    if manifest.area() != key.area() || manifest.program() != key.program() {
        return Some(format!(
            "the expectation record supplied governs {}/{} while this cell is {}/{}, so any marker \
             consulted would belong to another program; an expected divergence documented for one \
             program must never excuse a divergence in another",
            manifest.area(),
            manifest.program(),
            key.area(),
            key.program()
        ));
    }
    if comparison.equal && comparison.is_divergence() {
        return Some(String::from(
            "the comparison reports equality and a divergence class at once, so it cannot be \
             judged: reading it as an agreement would turn a divergence into a pass, and reading \
             it as a divergence would report one the comparator says did not happen",
        ));
    }
    if comparison.equal && !comparison.attempted() {
        return Some(String::from(
            "the comparison reports equality while also reporting that it was never attempted; \
             nothing was compared, so equality cannot have been established, and claiming it \
             would be exactly the silent pass an excluded comparison exists to avoid",
        ));
    }
    if !comparison.attempted() && comparison.is_divergence() {
        return Some(String::from(
            "the comparison reports both that it was never attempted and that it observed a \
             divergence, so what actually happened cannot be established",
        ));
    }
    if !comparison.attempted() && manifest.oracle_enabled(oracle) {
        return Some(format!(
            "the comparison was recorded as excluded, but this program's record ENABLES \
             {oracle}, so the exclusion removes a comparison the corpus asks for; a narrowing is \
             legitimate only when the record itself declares it"
        ));
    }
    if comparison.attempted() && !manifest.oracle_enabled(oracle) {
        return Some(format!(
            "this program's record DISABLES {oracle}, yet a comparison for it was performed and \
             submitted for judgement; a disabled oracle must be recorded as an exclusion carrying \
             the record's reason, because a comparison the corpus declared meaningless can only \
             produce a meaningless verdict — a spurious divergence would become a false finding, \
             and an accidental agreement would become a pass the corpus never claimed"
        ));
    }
    if !oracle_applies(oracle, key.target()) {
        return Some(format!(
            "{} is the cross-backend baseline, so a cross-backend comparison for it is the \
             baseline against itself and can only ever agree; counting that as a result would \
             add coverage the matrix does not contain. Call classify::oracle_applies before \
             comparing an arm.",
            Target::BASELINE
        ));
    }
    None
}

/// Build the outcome for an oracle whose tooling is genuinely absent.
fn unavailable_tooling_outcome(key: &CellKey, oracle: Oracle, diagnosis: &str) -> Outcome {
    let detail = format!(
        "not attempted: {oracle} cannot be attempted for {key} because tooling it needs is absent \
         from this environment — {diagnosis}. This is reported as unavailable and never as a pass \
         and never as a skip: there is no skip verdict in this suite. It is listed in the run \
         summary, and under {VAR_STRICT} — the intended continuous-integration setting, where the \
         toolchain is installed deliberately, so a missing oracle indicates a broken workflow \
         rather than a modest machine — it fails the run. {VAR_ALLOW_MISSING_ORACLES} \
         acknowledges a deliberately reduced environment in the summary and suppresses nothing, \
         including that escalation. The oracles this environment can attempt still judge this \
         cell in full."
    );
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::Unavailable,
        None,
        None,
        detail,
    )
}

/// Build the outcome for a build failure this machine, not the compiler, is answerable for.
fn unavailable_environment_outcome(
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    summary: &str,
) -> Outcome {
    let detail = format!(
        "not attempted at {scope}: a {class} was observed for {key} under {oracle} and the build \
         layer attributed it to this machine rather than to the compiler under test — {summary}. \
         In general, {significance}. It is therefore reported as unavailable at environment scope \
         and deliberately NOT as a finding: a static link that fails because a target's C runtime \
         was never installed has exactly the shape of a link failure caused by a code-generation \
         defect, and reporting the first as the second would manufacture a false finding against \
         a compiler that was never given the inputs it needed. It is listed in the run summary, \
         and under {VAR_STRICT} it fails the run, because in continuous integration the runtime \
         is installed deliberately.",
        scope = Attribution::Environment,
        significance = class_significance(class)
    );
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::Unavailable,
        None,
        None,
        detail,
    )
}

/// Build the outcome for a build failure nobody could be shown answerable for.
///
/// # Why this is reported rather than resolved
///
/// The build layer withdraws an attribution when the diagnostics it would have been read from were
/// truncated or could not be drained. There is genuinely no sound verdict available for such a
/// cell: calling it a pass would assert an agreement nothing established, calling it a finding
/// would accuse a compiler on the strength of text nobody read, and skipping it would hide the
/// whole situation. So it takes the one verdict that means "this comparison could not soundly be
/// made" — the same verdict an absent oracle takes — and it is listed in the summary and refused
/// under strict mode for the same reasons.
///
/// The detail deliberately names the remedy, because unlike an absent oracle this one is usually
/// fixable from the artifacts already on disk: the retained workspace holds the compiler's captured
/// streams and the capture-integrity record beside them, so a maintainer can see how much was lost
/// and re-run the single cell to read the rest.
fn unavailable_indeterminate_outcome(
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    summary: &str,
) -> Outcome {
    let detail = format!(
        "not attempted at {scope}: a {class} was observed for {key} under {oracle}, but the build \
         layer could not establish whether the compiler or this machine is answerable, because the \
         diagnostics its attribution would have rested on were not captured whole — {summary}. In \
         general, {significance}. It is therefore reported as unavailable at indeterminate scope \
         and deliberately NOT as a finding, because a finding is a deliverable a maintainer is \
         expected to act on and this one would rest on text nobody read; nor as a pass, because \
         nothing was shown to agree. The cell's retained workspace holds the compiler's captured \
         streams and the capture-integrity record beside them, so how much was lost is visible and \
         the single cell can be re-run to read the rest. It is listed in the run summary, and under \
         {VAR_STRICT} it fails the run.",
        scope = Attribution::Indeterminate,
        significance = class_significance(class)
    );
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::Unavailable,
        None,
        None,
        detail,
    )
}

/// Build the outcome for two observations that agree, with no marker claiming otherwise.
fn pass_outcome(key: &CellKey, oracle: Oracle, evidence: &str) -> Outcome {
    let detail = format!(
        "{oracle} agrees for {key} and no expected-divergence marker governs this cell: \
         {evidence}"
    );
    Outcome::new(key.clone(), oracle, Verdict::Pass, None, None, detail)
}

/// Build the outcome for an agreement that a marker says should have been a divergence.
fn xpass_outcome(
    key: &CellKey,
    oracle: Oracle,
    manifest: &Manifest,
    marker: &ExpectedDivergence,
    evidence: &str,
) -> Outcome {
    let detail = format!(
        "unexpected success: {reference} claims a divergence within a scope that covers this \
         cell, but {oracle} agrees for {key}, so the marker is stale. Retire it in BOTH places, \
         or the documentation stays wrong in one of them: this program's own record at {record}, \
         and the register at {EXPECTED_DIVERGENCE_REGISTER}. This verdict fails the run by \
         default, following the convention that unexpected success is a failure — the cost is \
         asymmetric, because a stale marker is stale documented knowledge that will mislead the \
         next reader, whereas retiring it is a trivial test-only edit. {VAR_ALLOW_XPASS} \
         downgrades it to a warning for a retirement window and hides nothing: an unexpected \
         success is listed separately and prominently in the summary either way. The agreement \
         was: {evidence}",
        reference = marker_reference(marker),
        record = shown_path(manifest.path())
    );
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::XPass,
        None,
        Some(String::from(marker.id())),
        detail,
    )
}

/// Build the outcome for a divergence a marker documents.
///
/// The `shape` decides which of two accounts the detail gives, because the two situations are
/// materially different and a reader must not have to guess which one they are looking at:
///
/// - [`DivergenceShape::Compared`] — this oracle made a comparison, it differed, and the marker's
///   own scope names this oracle. The ordinary expected divergence.
/// - [`DivergenceShape::Refused`] — the compiler under test produced no artifact for this cell, so
///   this oracle arm lost the subject of its comparison, and the marker documents that refusal.
///   Reaching this arm means the marker's scope names this oracle, because matching is strict on
///   every dimension for both shapes; the detail says which arm lost what, so the classification
///   is auditable from the row rather than only from this file.
fn xfail_divergence_outcome(
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    marker: &ExpectedDivergence,
    evidence: &str,
    shape: DivergenceShape,
) -> Outcome {
    let detail = match shape {
        DivergenceShape::Compared => format!(
            "expected divergence: the {class} observed by {oracle} for {key} is documented by \
             {reference}. In general, {significance}. A marker changes how a divergence is \
             CLASSIFIED, never whether the feature is EXERCISED: this program was compiled and run \
             in full, which is what keeps a difficult feature under test instead of quietly \
             dropped. If the divergence ever disappears, this becomes an unexpected success and \
             fails the run so that the marker is retired. The divergence was: {evidence}",
            reference = marker_reference(marker),
            significance = class_significance(class)
        ),
        DivergenceShape::Refused => format!(
            "expected divergence: no artifact was produced for {key}, so {oracle} lost the subject \
             of its comparison, and that {class} is documented by {reference}, whose scope names \
             {oracle} among {named}. Matching is strict on every dimension for a refusal exactly \
             as it is for a comparison: this arm had an authority to compare against — an arm \
             whose authority this environment cannot supply is reported as unavailable, and one \
             the record itself disables as a recorded exclusion — so naming an oracle narrows a \
             real set, and {register} therefore documents precisely the arms it says it does. In \
             general, {significance}. A marker changes how a divergence is CLASSIFIED, never \
             whether the feature is EXERCISED: this program was compiled in full and the \
             compiler's own diagnostics are recorded. If the refusal ever disappears, the \
             comparison this marker's scope names becomes an unexpected success and fails the run \
             so that the marker is retired. The refusal was: {evidence}",
            reference = marker_reference(marker),
            named = joined(marker.scope().oracles()),
            register = EXPECTED_DIVERGENCE_REGISTER,
            significance = class_significance(class)
        ),
    };
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::XFail,
        Some(class),
        Some(String::from(marker.id())),
        detail,
    )
}

/// Build the outcome for a comparison the record narrows away, whose narrowing a marker
/// documents.
///
/// This is the shape the `long double` programs take: their representation was measured to
/// differ across the four targets, so cross-backend value equality is meaningless for them,
/// and the record excludes that one oracle while the same-target reference comparison and the
/// golden record continue in full. The exclusion is recorded and reasoned, never silent, and it
/// is classified as an expected divergence because the marker gives it a documented basis — the
/// summary must list every expected divergence with exactly that.
fn xfail_exclusion_outcome(
    key: &CellKey,
    oracle: Oracle,
    manifest: &Manifest,
    marker: &ExpectedDivergence,
    reason: &str,
) -> Outcome {
    let detail = format!(
        "recorded exclusion: {oracle} was not attempted for {key} because this program's own \
         expectation record at {record} narrows its oracle coverage, and the narrowing is \
         documented by {reference}. Nothing was compared, so no equality is claimed and this is \
         not a pass; the oracles this program keeps — {kept} — still judge the cell in full, \
         which is how a construct whose value legitimately differs between architectures stays \
         under test instead of being dropped for being difficult. Recorded reason: {recorded}",
        record = shown_path(manifest.path()),
        reference = marker_reference(marker),
        kept = joined(manifest.enabled_oracles()),
        recorded = inline_reason(reason)
    );
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::XFail,
        None,
        Some(String::from(marker.id())),
        detail,
    )
}

/// Build the outcome for a narrowing documented by the record's own recorded reason.
///
/// Reached when the record disables the oracle and no marker's scope additionally names it. The
/// verdict is still [`Verdict::XFail`], because the narrowing is still documented — just by the
/// record's `impl_defined_notes` rather than by a marker — and three facts make that the honest
/// classification rather than a convenient one:
///
/// - **A reason always exists.** The record parser refuses to accept a record that narrows its
///   oracle coverage without recording why, so an undocumented narrowing cannot reach here from
///   a parsed record at all. The prohibition on silent exclusion is enforced upstream, in the
///   format, which is a stronger place for it than a verdict.
/// - **It is a corpus decision, not a machine deficiency.** The comparator's own worked example
///   is `long double`, whose representation was measured to differ across the four targets, so
///   cross-backend *value* equality is meaningless for it while the same-target reference
///   comparison and the golden record remain entirely meaningful. Reporting that as an
///   unavailable oracle would file a deliberate, reasoned authoring decision as a missing tool,
///   and the strict setting — which exists to catch a continuous-integration workflow that
///   failed to install something — would then fail a run over a comparison the corpus never
///   asked anyone to make.
/// - **It is still not a pass, and still loud.** Nothing was compared, so no equality is
///   claimed; the outcome is counted and listed with its recorded reason printed, which is what
///   keeps the set of comparisons deliberately not made as visible as the set that was.
///
/// The one shape that is *not* excused here is a record that enables the oracle while the
/// comparison claims exclusion. That really would remove a comparison the corpus asks for, and
/// it is reported as [`Verdict::Fail`] by [`comparison_inconsistency`] before this is reached.
fn xfail_recorded_exclusion_outcome(
    key: &CellKey,
    oracle: Oracle,
    manifest: &Manifest,
    reason: &str,
) -> Outcome {
    let detail = format!(
        "recorded exclusion: {oracle} was not attempted for {key} because this program's own \
         expectation record at {record} narrows its oracle coverage. No expected-divergence \
         marker additionally names this oracle, target and optimization level, so the documented \
         basis is the record's own recorded reason, which the record format requires before it \
         will accept any narrowing at all — a narrowing nobody justified cannot be written down. \
         Nothing was compared, so no equality is claimed and this is not a pass; it is counted \
         and listed so the comparison deliberately not made stays as visible as the ones that \
         were. The oracles this program keeps — {kept} — still judge the cell in full, which is \
         how a property whose value legitimately differs between architectures stays under test \
         instead of being dropped for being difficult. Recorded reason: {recorded}",
        record = shown_path(manifest.path()),
        kept = joined(manifest.enabled_oracles()),
        recorded = inline_reason(reason)
    );
    Outcome::new(key.clone(), oracle, Verdict::XFail, None, None, detail)
}

/// Build the outcome for an undocumented divergence: a finding.
fn finding_outcome(
    key: &CellKey,
    oracle: Oracle,
    class: DivergenceClass,
    evidence: &str,
    marker_note: Option<String>,
) -> Outcome {
    let mut detail = format!(
        "undocumented divergence: the {class} observed by {oracle} for {key} is documented by no \
         expected-divergence marker. In general, {significance}. ",
        significance = class_significance(class)
    );
    if let Some(note) = marker_note {
        detail.push_str(&note);
        detail.push(' ');
    }
    detail.push_str(&format!(
        "This is recorded as a finding, which is a DELIVERABLE and not a defect to patch: emit \
         the artifact directory — a verbatim reproducer with its recorded minimization status, \
         its expectation record, a manifest, the exact reproduction commands, the captured \
         outputs per compiler and per backend, an environment fingerprint and the computed diff \
         — and index it in {FINDINGS_REGISTER}. No compiler source change is made in response \
         to a finding. Unlike an unexplained failure \
         this does not fail the run, because its meaning is known and delivered rather than \
         unknown; it is still reported loudly and counted in the summary. The divergence was: \
         {evidence}"
    ));
    Outcome::new(
        key.clone(),
        oracle,
        Verdict::Finding,
        Some(class),
        None,
        detail,
    )
}

/// Build the outcome for anything unexplained.
fn fail_outcome(key: &CellKey, oracle: Oracle, context: &str, cause: &str) -> Outcome {
    let detail = format!(
        "unexplained, while {context}: {cause}. This is neither a classified divergence nor an \
         absent oracle, so it is reported as a failure and it fails the run: a result whose \
         meaning cannot be established must never be presented as a pass, because an internal \
         error that passed would hide every defect behind it. Cell {key}, {oracle}."
    );
    Outcome::new(key.clone(), oracle, Verdict::Fail, None, None, detail)
}

/// Render a marker for an outcome detail: its identifier, its class, its scope and its
/// documented basis.
///
/// The basis is always included, because the deliverable summary must list every expected
/// divergence *with its documented basis*, and a row that names only an identifier would send
/// every reader to a second file to learn what the identifier means.
fn marker_reference(marker: &ExpectedDivergence) -> String {
    format!(
        "marker {id} (class {class}, scope {scope}, documented basis: {basis})",
        id = marker.id(),
        class = marker.class(),
        scope = marker.scope(),
        basis = inline_reason(marker.basis())
    )
}

/// Why one oracle arm cannot be attempted for one target, naming the tool and the environment
/// variable that would supply it.
///
/// The execution obstacle is reported first, because an inability to run the program blocks
/// every oracle and is the more useful fact whenever both apply. The `match` over the oracle is
/// exhaustive with no wildcard arm.
fn oracle_diagnosis(caps: &Capabilities, target: Target, oracle: Oracle) -> String {
    if !caps.can_execute(target) {
        return match runner_variable_for(target) {
            Some(variable) => format!(
                "no emulator is available to execute a {target} binary, so this target drops out \
                 of oracle (b) and out of oracle (a)'s cross arm while the other targets \
                 continue; install the QEMU user-mode runner for this architecture, or set \
                 {variable} to an executable path (both the plain and the statically linked \
                 spellings of the runner name are probed, because packagings differ on which \
                 they ship)"
            ),
            None => format!(
                "a {target} binary cannot be executed on this {host} host: no runner is defined \
                 for this target and it is not the host architecture",
                host = caps.host_arch()
            ),
        };
    }
    match oracle {
        Oracle::ReferenceCompiler if !caps.ref_cc_native().is_available() => format!(
            "the native reference compiler is absent, which takes oracle (a) out for every target \
             including {target}: it is the driver both undefined-behaviour audit gates run, and \
             without those gates a divergence could not be attributed to the compiler under test. \
             {diagnosis}. Set {VAR_REF_CC} to an executable path to select it explicitly. Oracles \
             (b) and (c) are unaffected and still run in full.",
            diagnosis = caps.ref_cc_native().diagnosis()
        ),
        Oracle::ReferenceCompiler => match reference_variable_for(target) {
            Some(variable) => format!(
                "no reference compiler targeting {target} is available, and the reference side \
                 selects a target by choosing a different driver binary rather than by a flag — \
                 the drivers this harness uses reject the target-selection spelling that belongs \
                 to another compiler family, and the word-size flag was measured to fail for want \
                 of multilib start files; install the {triple} cross driver, or set {variable} to \
                 an executable path",
                triple = target.triple()
            ),
            None => format!(
                "no reference driver targets {target} on a {host} host; cross drivers are defined \
                 for the three non-baseline targets only, so the baseline arm relies on the \
                 native driver",
                host = caps.host_arch()
            ),
        },
        Oracle::CrossBackend => format!(
            "the {baseline} baseline cannot be executed, so there is nothing for {target} to be \
             compared against; the baseline is the authority every other target is compared with, \
             which is why it is the host architecture and runs with no emulator",
            baseline = Target::BASELINE
        ),
        Oracle::GoldenRecord => format!(
            "{target} cannot be executed, so there is no observation to compare against the \
             recorded golden output"
        ),
    }
}

/// The environment variable that names the execution runner for a target, when one is defined.
///
/// `None` for the baseline, which the intended host executes directly and for which no runner is
/// defined.
fn runner_variable_for(target: Target) -> Option<&'static str> {
    match target {
        Target::I686 => Some(VAR_QEMU_I386),
        Target::Aarch64 => Some(VAR_QEMU_AARCH64),
        Target::Riscv64 => Some(VAR_QEMU_RISCV64),
        Target::X86_64 => None,
    }
}

/// The environment variable that names the reference cross driver for a target, when one is
/// defined.
///
/// `None` for the baseline, which is served by the native driver named by `BCC_REF_CC`.
fn reference_variable_for(target: Target) -> Option<&'static str> {
    match target {
        Target::I686 => Some(VAR_REF_CC_I686),
        Target::Aarch64 => Some(VAR_REF_CC_AARCH64),
        Target::Riscv64 => Some(VAR_REF_CC_RISCV64),
        Target::X86_64 => None,
    }
}

/// Render free text recorded by a maintainer for the single-line form of an outcome.
///
/// Runs of whitespace, including the line feeds a heredoc field carries, collapse to single
/// spaces, and the result is bounded by [`MAX_INLINE_REASON_CHARS`] characters with the
/// truncation announced. Collapsing is preferable to escaping here because the alternative
/// renders a paragraph as a wall of escape sequences: the escaping the outcome type applies at
/// construction is a correctness guarantee, and this keeps the guarantee from costing legibility
/// on the one field that is deliberately prose.
///
/// The bound counts characters rather than bytes, so it cannot split a multi-byte character, and
/// it is a constant, so the same reason always renders the same way.
fn inline_reason(raw: &str) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_INLINE_REASON_CHARS {
        return collapsed;
    }
    let kept: String = collapsed.chars().take(MAX_INLINE_REASON_CHARS).collect();
    format!("{kept}{INLINE_REASON_TRUNCATION}")
}

/// Join displayable values with commas for a one-line detail, naming the empty case explicitly.
///
/// An empty list renders as `none` rather than as nothing at all, because a sentence that reads
/// "the oracles this program keeps — " tells a reader less than nothing.
fn joined<T: fmt::Display>(values: &[T]) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
