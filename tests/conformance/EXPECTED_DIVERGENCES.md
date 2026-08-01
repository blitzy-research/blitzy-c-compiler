# Expected Divergences — Register

The auditable, single-place index of every **expected divergence** in the differential
conformance suite.

An *expected divergence* is a divergence between `bcc` and one of the suite's oracles that a
limitation **this repository already documents** authorises. Requirement 5 of the suite's brief
requires such a divergence to be recorded with a marker that references the documented
limitation, and — just as importantly — forbids quietly dropping the feature from the corpus
instead. This file is the place where that record is reviewable in one reading.

## 1. What this register is, and what it is not

Markers do not live here. Each marker lives **inside the program's own `<program>.expected`
record**, beside the source file it belongs to, so that the divergence and the program that
provokes it can never drift apart: move the program and the marker moves with it; delete the
program and the marker goes with it. This file **mirrors** every marker so that the complete set
— which programs are excused, on which oracle, for what documented reason — can be read,
reviewed and audited in one place instead of being reconstructed by grepping a hundred records.

**A marker never changes what a program does.** A marker changes how a divergence is
*classified*; it never changes whether the feature is *exercised*. The applicable phases are
attempted in order — compile, link, run, compare — and classification happens at the **first
terminal outcome or the completed comparison**: a compile failure, a link failure, a crash and a
timeout are terminal outcomes reached before any comparison exists, and each is classified where it
occurred. Nothing in the harness can short-circuit a phase because a marker exists; the classifier
is handed no means of preventing any of them. That is also what makes a `compile_failure` marker
meaningful at all — the cell it excuses never reaches a comparison, so a rule that demanded one
would make the class unreachable. **Silently excluding a language feature from testing is
prohibited**, and the prohibition is enforced mechanically rather than merely intended.

### 1.1 The bidirectional machine check

`infra_expected_divergence_register`, in [`../conformance.rs`](../conformance.rs), asserts
consistency in **both** directions on every run, plus the existence of every cited document:

| Direction | Assertion | Why it exists |
|---|---|---|
| **Forward — mention** | Every `expected_divergence.id` committed in any `.expected` record appears in this register. | An unregistered marker is exactly the silent exclusion requirement 5 forbids. |
| **Forward — description** | Every such marker has exactly **one structured entry** here (§1.3), and every one of the entry's six fields — Identifier, Class, Scope, Program, Basis, Observed — **agrees with the record**. | Being mentioned is not being described. An identifier can sit in a heading while the register says nothing checkable, and the register could then drift away from the record it mirrors with no run noticing. |
| **Reverse** | Every identifier appearing in this register corresponds to a real, **active** marker in a real `.expected` record. | The register must describe divergences that are actually exercised, not ones retired without retiring the entry. |
| **Basis — containment** | Every cited path resolves to a document **inside this repository**, containment being decided on the fully resolved path. | A basis outside the repository is not something this repository documents, and an intermediate symbolic link must not be able to move the answer. |
| **Basis — a real document** | Every cited document is **read**, through the suite's bounded reader: a symbolic link, a device node or a FIFO at the final component is refused, the opened handle is proved to be the entry that was inspected, and an oversized file is refused. | Existence was never the property that mattered. The earlier check followed a link and asserted only that *something* was there, so a basis could point through a link at anything readable and still pass. |
| **Basis — a resolvable locator** | Every locator the citation contains resolves **inside** that document, and at least one locator is present (§2.4). | A citation a reader cannot follow is not a basis. This is what stops "the section that documents this limitation" from counting as an authority. |

Two consequences follow, and both are load-bearing:

1. **An edit here must be paired with an edit to a record, and an edit to a record must be paired
   with an edit here.** Doing one and not the other fails the run. That is deliberate: a marker
   retired in one place and left in the other is still stale documentation, and stale
   documentation about a compiler defect is worse than none.
2. **This document contains no identifier that is not an active marker.** There is no
   illustrative identifier, no placeholder, no retired identifier and none reserved for future
   use — not in a table, not in prose, not in a fenced block, not in a comment. The reverse check
   reads this whole file as a flat token stream, so any identifier it cannot resolve to a live
   marker fails the run. For the same reason this file carries **no example marker block**; the
   record format is documented once, in [`README.md`](README.md), which covers the marker block
   along with the rest of the `.expected` syntax.

Identifiers are stable, uppercase and hyphen-separated, and end in a three-digit sequence number.
They are never reused after retirement: an identifier names one investigation, and reusing it
would silently merge two.

The harness resolves this register by a fixed path — `tests/conformance/EXPECTED_DIVERGENCES.md` —
held as a constant in the harness rather than discovered, and it is read as a **committed
deliverable, never generated**. Renaming or moving the file fails the audit, and so does deleting
it while any marker remains in the corpus. Run the audit on its own with:

```bash
cargo test --test conformance infra_expected_divergence_register -- --nocapture
```

**Prerequisite for every `cargo` command in this file.** They all require the **package-complete
branch** — the one that carries `Cargo.toml`, `src/**` and `include/**`. On a documentation-only
checkout Cargo exits `101` with `could not find Cargo.toml`, so the audit cannot be invoked at all
even though every file it would read is committed and present. Once the package is there, the audit
reads committed files only — no compiler, no emulator, no reference toolchain — so it stays
meaningful on a machine that can run nothing else in the suite.

### 1.2 Where each verdict comes from

The register only makes sense against the closed verdict space the suite uses. In full, from
[`classify.rs`](../conformance_harness/classify.rs):

| Verdict | Meaning | Fails the run? |
|---|---|---|
| `PASS` | The comparison agreed and no marker governs the cell. | No |
| `XFAIL` | Either of two forms: a divergence occurred and a marker covers this oracle, target, optimization level **and** class; or the comparison was **not attempted because the program's own record narrows coverage**, carrying its reasoned exclusion in `impl_defined_notes` (§3.3). | No |
| `XPASS` | A marker covers the cell but the comparison **agreed** — the marker is stale. | **Yes**, by default |
| `FINDING` | A divergence occurred that no marker covers. It is delivered as a self-contained artifact, never patched. | No — a finding is a deliverable |
| `FAIL` | Anything unexplained: harness breakage, an internal inconsistency, a corpus defect. | **Yes** |
| `UNAVAILABLE` | An oracle's tooling is genuinely absent from this machine. Reported loudly, never as a pass. | Only under `BCC_CONFORMANCE_STRICT` |

There is deliberately **no "skip because unsupported" verdict**. This register is what makes `XFAIL`
reachable — so that a divergence can be *explained* — without letting `PASS` absorb a divergence
nobody explained, and without a feature quietly disappearing from the corpus to avoid the question.

### 1.3 The shape of a structured entry

An entry is a **heading whose text contains the marker's identifier**, followed anywhere before the
next heading by a **two-column table** stating six fields. The heading is what binds the entry to the
marker; the table is what the audit compares. Everything else in the section is free prose, at
whatever length the divergence deserves — an entry is expected to *explain* itself, and the skeleton
exists so that the explanation cannot quietly disagree with the record beside the program.

```text
### <n> <identifier> — <short title>

| Field | Value |
|---|---|
| Identifier | <the marker's expected_divergence.id, verbatim> |
| Class | <one of the six class identifiers> |
| Scope | <the marker's expected_divergence.scope, verbatim> |
| Program | <area>/<program> |
| Basis | <the marker's expected_divergence.basis, verbatim> |
| Observed | <the marker's expected_divergence.observed> |
```

Five mechanical details, each of which exists because the alternative is a rule an author cannot
satisfy or an audit that cannot decide:

- **Field names may be emphasised.** `| **Basis** | … |` and `| Basis | … |` are the same row.
- **A value may be wrapped in one pair of backticks.** The register renders a literal as a code span
  so it reads correctly; the record holds the bare literal. Exactly one surrounding pair is unwrapped
  before comparison, so `` `all oracles; all targets; all opt levels` `` matches the scope the record
  writes plainly.
- **A `|` inside a value is written `\|`**, as any Markdown table requires, and is unescaped before
  comparison.
- **`Program` is `<area>/<program>`** — the directory name and the program's file stem, without the
  `.c` suffix. An entry naming the wrong program would send a reader to a construct the marker never
  governed.
- **`Observed` is compared with runs of whitespace collapsed**, and only because a table cell cannot
  contain a newline while the record's `observed` field routinely does. The other five fields are
  compared **exactly** after trimming, which is §2.4's character-for-character rule made mechanical
  rather than merely asked for.

Two entries for one identifier is a failure, not a redundancy: one marker is one investigation, and a
reader faced with two entries cannot tell which is the authority.

## 2. The marker contract

### 2.1 The five required keys

A marker is a block of five keys inside a program's `.expected` record. **Either all five are
present or none is**: a partial block is a hard parse error, not a warning, because each part
carries weight the others cannot.

| Key | Holds | Why the block is worthless without it |
|---|---|---|
| `expected_divergence.id` | The stable identifier, mirrored in this register. | Without it the register cannot be cross-checked in either direction. |
| `expected_divergence.class` | One of the six divergence classes below. | Without it the classifier cannot tell which observation the marker excuses. |
| `expected_divergence.scope` | Which oracles, targets and optimization levels the marker covers. | Without it there is no way to tell which cells are excused and which are not. |
| `expected_divergence.basis` | The repository artifact and section that authorises the marker. | Without it the divergence is reclassified on no authority at all. |
| `expected_divergence.observed` | The divergence as actually seen, in prose. | Without it a reader cannot tell whether what they are looking at is what was marked. |

The exact record syntax — `key = value` lines, `#` comments and `key <<END … END` heredoc blocks —
is documented in [`README.md`](README.md). It is not restated here, so that there is one authority
for the format rather than two that can disagree.

### 2.2 The six legal divergence classes

These are the **only** accepted values of `expected_divergence.class`. The class space is closed:
adding a seventh is a compile error in the harness until every decision point handles it.

| Class | The observation it names |
|---|---|
| `compile_failure` | One compiler rejected a program the other accepted. |
| `link_failure` | The program translated but did not link. When a target's C runtime is simply absent from the machine, this is `UNAVAILABLE` at environment scope instead, never a divergence against the compiler. |
| `run_crash` | The program died on a signal rather than exiting. Compared as a raw wait status, so it is never conflated with a numerically equal ordinary exit. |
| `exit_code_mismatch` | Two completed runs disagreed on exit status. |
| `stdout_mismatch` | Two completed runs disagreed on stdout bytes — the ordinary shape of a wrong answer. |
| `timeout` | Execution outlived its per-cell budget. A first-class divergence: a program that finishes promptly under one compiler and hangs under another is a defect worth surfacing. |

Standard error is never compared, by any oracle, so no marker may describe a difference in
diagnostic text. Diagnostic wording legitimately differs between compilers; comparing it would
produce a flood of divergences that say nothing about code correctness.

### 2.3 The scope grammar

A scope is a structured, semicolon-separated list of clauses:

```text
oracle_a; all targets; all opt levels
```

- A clause is either a **whole-dimension** token — `all oracles`, `all targets`, `all opt levels`
  — or a **comma-separated list whose every item belongs to one dimension**: an oracle
  (`oracle_a`, `oracle_b`, `oracle_c`), a target (`x86_64`, `i686`, `aarch64`, `riscv64`) or an
  optimization level (`-O0`, `-O1`, `-O2`).
- Each of the three dimensions may be constrained **at most once**, so a scope has exactly one
  reading.
- A dimension the scope does not mention defaults to **all** of its members. `oracle_b` on its own
  therefore reads as "the cross-backend comparison, on every target, at every optimization level".
- At least one clause must be explicit, and an **empty clause is refused** rather than ignored. A
  stray, doubled, leading or trailing `;` would otherwise leave every dimension to fall back to
  "all" — the widest scope there is, reached by writing nothing.

**Matching is strict.** A marker excuses a divergence only when its scope covers this oracle
**and** this target **and** this optimization level **and** its class equals the class observed. A
marker scoped to one oracle does not excuse a divergence observed on another; a marker for
`compile_failure` does not absorb a `stdout_mismatch`. A marker is never widened to swallow a
divergence it does not describe, because that would launder a genuine second defect into an
expected one while the register still documented only the first. When a marker exists but does not
cover the observation, the verdict falls through to `FINDING` and the detail names exactly which
dimension failed to match.

**Strictness has no shape exemption, so a build refusal must be scoped `all oracles`.** The four
build-refusal classes — `compile_failure`, `link_failure`, `run_crash` and `timeout` — reach
classification once per **oracle arm**, not once per cell, because each arm is settled against its
own authority: the same-target reference capture for (a), the baseline capture for (b), the
record's own `expected_stdout` for (c). An arm whose authority this environment cannot supply is
reported `UNAVAILABLE` and never reaches classification; an arm the program's own record disables
is a recorded exclusion and never reaches it either. So the arms that are classified are exactly
the arms that held an authority, and the oracle dimension narrows a real set for a refusal just as
it does for a comparison.

A refusal denies **every** arm its subject, so a marker that documents one is scoped `all oracles`:

```text
expected_divergence.scope = all oracles; all targets; all opt levels
```

Writing `oracle_a` instead documents only the same-target reference comparison, and the
cross-backend and golden-record arms of the same refusal are then reported as `FINDING` — the
detail names the oracle dimension among the mismatches and states the `all oracles` remedy. That is
the correct reading rather than a rough edge: an `oracle_a` basis records what the *reference
compiler* accepts, which is evidence about one authority and says nothing about the other two. The
fix is one word in the scope, in this register and in the program's record together, never a
per-shape exemption in the matcher.

### 2.4 The basis grammar

The value of `expected_divergence.basis` **must begin with a repository-relative file path**, then
a comma, then the section or description within that file that authorises the marker:

```text
docs/project-guide.md, the open risk register entry that this limitation belongs to
```

- The path is relative to the repository root. An absolute path is rejected, and so is any path
  containing a parent-directory component — a basis cites a repository artifact, not something
  outside it.
- **The cited document must exist, lie inside this repository, and be readable as a committed file.**
  This is machine-verified. The audit resolves the path and requires the *fully resolved* result to
  lie beneath the package root, so no symbolic link along the way can move the answer; then it
  **reads** the document through the suite's bounded reader, which refuses a symbolic link, a device
  node or a FIFO at the final component, proves the opened handle is the entry it inspected, and
  refuses a file past the inspection ceiling. Existence alone was never the property that mattered —
  the document is read because the section the marker cites has to be resolved inside it.
- A basis that names a file but cites no section within it is rejected. A whole-document citation
  cannot be checked by a reader, which is the entire point of recording one.
- **The citation must carry at least one locator, and every locator it carries must resolve.** This
  is machine-verified against the document's own bytes. Three forms are recognised, and they are the
  three §7's inventory already uses:

  | Locator | Written as | Resolves when |
  |---|---|---|
  | A line | `line 246` | the document has at least that many lines |
  | A line range | `lines 696-725`, or with an en dash | both ends are real line numbers and the range does not run backwards |
  | A section number | `§0.6.2` | the document carries a Markdown heading for that number |
  | A quoted phrase | `` `Explicitly Out of Scope` `` | the phrase occurs in the document verbatim |

  Requiring **every** locator to resolve is what stops a correct one from carrying a stale one
  alongside it — a line range that has drifted since it was written is exactly the kind of citation a
  reader gives up on. Requiring **at least one** is what stops a citation from being unfalsifiable
  prose: "the section that documents this limitation" names nothing a run or a reader can turn to, so
  it is refused rather than accepted on trust.
- **One canonical rendering, reproduced verbatim.** The string in the program's record is the
  canonical one. Wherever this register mirrors it — the summary-table row in §4 and the structured
  entry's **Basis** field — it must be reproduced **character for character**, including punctuation
  and the exact list of items it enumerates. The structured entry's rendering **is** machine-verified
  against the record (§1.1, §1.3), so a paraphrase there fails the run rather than surviving as a
  second account of the same authority. A paraphrase in the §4 summary row is still a defect no run
  will catch for you, because that row is prose the audit does not compare.

In practice the only two files that qualify as a basis today are
[`docs/technical-specifications.md`](../../docs/technical-specifications.md) and
[`docs/project-guide.md`](../../docs/project-guide.md); §7 lists the specific sections of each that
are legitimately citable.

## 3. Unexpected success, retirement, and markers that could never be consulted

### 3.1 Unexpected success is a failure

**If a marker is present but the divergence has disappeared, the verdict is `XPASS` and the run
fails by default**, following the standard xunit convention that unexpected success is a failure.

The justification is **asymmetric cost**. A stale marker is stale documented knowledge that will
mislead the next reader: it asserts that a construct is broken when it works, and it silently
blinds the suite to that construct forever after, because a real regression there would be excused
as expected. Retiring a marker, by contrast, is a **trivial, test-only edit** — two deletions, no
compiler change, no risk. When one side of a trade is "misleading documentation about a compiler
defect, indefinitely" and the other is "delete ten lines", the choice is not close.

`BCC_CONFORMANCE_ALLOW_XPASS` downgrades `XPASS` from a failure to a warning for a
marker-retirement window — for instance while a fix is in flight and the register entry is being
retired in a separate change. It hides nothing: an unexpected success is **always** listed
separately and prominently in the run summary, whether or not the escape hatch is set, and every
`XPASS` detail names the marker and says where to retire it.

### 3.2 Retirement procedure

Retiring a marker is exactly two edits, and **doing only one of them fails the run** — the forward
check catches step 2 without step 1, and the reverse check catches step 1 without step 2:

1. **Delete all five `expected_divergence.*` keys** from the program's `.expected` record. Leave
   the program, its matrix, its oracle toggles and its golden record untouched: the feature stays
   under test, which is the whole point. If the record narrows coverage for a reason that survives
   the marker, keep that reason in `impl_defined_notes`.
2. **Delete the corresponding entry from this register** — both its summary-table row and its
   detailed subsection. Note the retirement in §8.3 by date and description; **do not carry the
   retired identifier forward**, because the reverse check would then look for a marker that no
   longer exists.

Then confirm, on the package-complete branch (§1.1):

```bash
cargo test --test conformance infra_expected_divergence_register -- --nocapture
```

### 3.3 Markers that could never be consulted, and narrowed coverage

A marker whose scope named a comparison its own program never makes would be **dormant**: nothing
would ever consult it, so it could never reach `XFAIL`, and — because no comparison is performed —
it could never reach `XPASS` either. It would sit here looking healthy while documenting an
experiment nobody runs, and if the divergence it describes ever appeared in a cell its scope
excluded, the run would fail as unexplained with the explanation sitting unread in the same file.

The record format refuses that state outright rather than tolerating it. A marker is rejected at
parse time when its scope names **only** oracles the record has switched off, **only** targets the
record does not build, or **only** optimization levels the record does not sweep. The error says
so directly: widen the scope, or enable the oracle it describes. The consequence for authoring is
simple and must be respected:

> **The oracle a marker describes has to remain enabled in that program's record.** A marker is
> the mechanism by which a divergence is *explained*, not the mechanism by which a comparison is
> *skipped*.

A record may nevertheless narrow its own coverage — switch an oracle off, restrict its target list,
or deviate from the default warning gate — and the format refuses to accept any such narrowing
**without a reason recorded in `impl_defined_notes`**. A narrowed cell is still classified and
still reported; it is never a silent skip:

| Situation | Verdict | Reported as |
|---|---|---|
| Not attempted because the record narrows coverage, and a marker covers the scope | `XFAIL` | The marker and its documented basis |
| Not attempted because the record narrows coverage, with no covering marker | `XFAIL` | The record's own recorded reason, printed in full |
| Not attempted while the record **enables** that oracle | `FAIL` | An unexplained removal of a comparison the corpus asks for |

A narrowing is deliberately **not** `UNAVAILABLE`, and the distinction is the whole point of that
verdict: `UNAVAILABLE` is a statement about the *machine* — a tool nobody installed — which is why
`BCC_CONFORMANCE_STRICT` escalates it in continuous integration, where the toolchain is installed
on purpose. A narrowing is a statement about the *corpus*: a deliberate authoring decision. Nor is
it a `PASS`: nothing was compared, so no equality is claimed. The cell is counted and listed with
its recorded reason printed, which keeps the set of comparisons deliberately **not** made as
visible as the set that was.

**Keeping three things mutually consistent is therefore mandatory**, and no run can check the
prose for you: a program's `oracle_a`/`oracle_b`/`oracle_c` toggles, the reason in its
`impl_defined_notes`, and its entry in this register must tell the same story. If a maintainer ever
switches an oracle off for a program that carries a marker scoped to that oracle, the marker must
be retired in the same edit (§3.2) — otherwise the record no longer parses and the whole suite
stops, which is the harness declining to let a never-consultable marker look healthy.

## 4. Divergences analysed, and why none carries a marker today

**No marker is active.** No `.expected` record in the corpus carries an `expected_divergence.*`
block, so this register lists **no identifier at all** — the reverse direction of the audit in §1.1
resolves every identifier in this document to a live marker in a committed record, and an identifier
written ahead of an observed divergence would fail the run for the whole suite.

That is emphatically not an absence of analysis, and it is not the silent exclusion requirement 5
forbids. Three candidates are analysed below; each names the documentation that bears on it, states
why its program is written and run rather than dropped, and records the exact conditions under which
a marker could legitimately be minted later.

| § | Candidate | Program | Status today | Verdict if it diverges today |
|---|---|---|---|---|
| 4.1 | GCC case ranges | `08_gcc_extensions/004_case_ranges.c` — committed | Analysed; **no marker** | `FINDING` |
| 4.2 | `long double` across the backends | `13_floating_point/004_long_double_target_restricted.c` — **not yet committed** | Analysed; oracle (b) to be excluded by the record's own recorded reason, **not** by a marker | `XFAIL` on oracle (b), citing that recorded reason; `FINDING` on oracle (a) or (c) |
| 4.3 | Wide and Unicode literal prefixes | `11_literals_and_strings/003_wide_and_unicode_literals.c` — committed | Analysed; **no marker** | `FINDING` |

**Why none of the three is marked — stated once here, because it is the same rule three times.** A
marker reclassifies a divergence on the authority of a limitation this repository **explicitly
documents** (§2.4, §7). An **omission** from an inventory is not that: it records that no document
mentions a construct, not that the implementation rejects it. A marker minted on an omission claims
a divergence nobody has observed — and if the construct in fact works, the cell agrees, the verdict
is `XPASS`, and **the run fails on a mistake in the test material rather than on a defect in the
compiler**. Worse, until someone notices, that marker blinds the suite to a genuine regression in
exactly the construct it was meant to document. So the rule is:

> **Until a repository artifact explicitly documents the limitation *and* a real divergence has been
> observed and reproduced, the divergence is a `FINDING`.**

That is not a compromise. An undocumented divergence is precisely what requirement 6 defines a
finding to be: a first-class, reported outcome, delivered as a self-contained artifact with exact
reproduction commands, and never patched. §8.5 states the same rule as a checklist, and its closing
sentence forbids the speculative marker in as many words.

### 4.1 GCC case ranges — analysed, no marker

| Field | Value |
|---|---|
| **Status** | analysed; **no marker**, no identifier |
| **Program** | `tests/conformance/08_gcc_extensions/004_case_ranges.c` — committed |
| **Record** | `tests/conformance/08_gcc_extensions/004_case_ranges.expected` — committed, carrying the same analysis in its own `impl_defined_notes` |
| **Oracles** | (a), (b) and (c) all enabled; all four targets and all three optimization levels declared |
| **Verdict if `bcc` rejects the construct** | `FINDING` — an undocumented divergence, delivered as an artifact |
| **Verdict if `bcc` accepts it and agrees** | `PASS` |
| **What would justify a marker later** | A repository artifact that **explicitly documents** case ranges as unsupported, **plus** an observed and reproduced divergence — the checklist in §8.5 |

**The construct.** GCC's case-range extension: a switch label of the form `case 0 ... 9:`, together
with its character and negative forms `case '0' ... '9':` and `case -20 ... -11:`.

**What the documentation actually says, stated so a reader can check it without trusting this file.**
Four separate inventories enumerate the GCC extensions the frontend handles, and all four omit case
ranges:

| Where | What it enumerates |
|---|---|
| `docs/project-guide.md` line 81 (parser subsystem row) | `__attribute__`, statement expressions, `typeof`, computed goto, inline assembly |
| `docs/project-guide.md` line 206 (requirement conformance matrix) | the same five, recorded as "all parsed" |
| `docs/technical-specifications.md` lines 13 and 107 (feature requirement and the parser module that implements it) | `__attribute__`, `__builtin_*` intrinsics, inline assembly with operand constraints, statement expressions, `typeof`/`__typeof__`, computed goto, `__extension__` |
| `docs/technical-specifications.md` lines 506 and 761 (the parser file plan and the C11 + GCC Extensions Compliance Rule) | the same seven, described as "not optional" |

A recursive, case-insensitive search of `docs/` for `case range` returns **zero** matches.

**Why that is not a basis, and why no marker is attached.** Every one of those four inventories is
*silent* about case ranges; not one of them says the frontend rejects them. Silence is compatible
with two opposite realities — an unimplemented extension, or an incomplete inventory — and a marker
asserts the first. Attaching one would therefore be a guess dressed as documentation, and a guess
with a specific cost: if `bcc` accepts case ranges, the cell agrees, the verdict is `XPASS` and the
run fails on the test material. Requirement 5 authorises a marker for a limitation the repository
*documents*, not for one a reader might infer, so **a rejection here is a `FINDING`** — captured with
its reproducer, its per-cell outputs and its exact reproduction commands, and reported rather than
excused. The program's own `impl_defined_notes` records the same decision, so the record and this
register tell one story.

**Why the program exists at all.** Constraint C3 forbids excluding a language feature because it may
be unimplemented or awkward, and the suite's mandated extension list names case ranges explicitly
alongside statement expressions, `typeof` and computed gotos. So the program is written regardless.
Its record **declares** all four targets and all three optimization levels — so the suite
**schedules** twelve `bcc` cells for it, on the same footing as every other program — and it is
written to exercise the construct in five distinct shapes: contiguous positive buckets, a
negative-valued range, a character-class range, a single-value range (`case 5 ... 5:`), and a range
group that deliberately falls through. Each switch is driven once from a constant and once from
`volatile` storage, so that constant folding cannot stand in for the backend's own lowering. No
run of these cells has been recorded yet on this branch, which carries the corpus but not the
package; the verdict rule above is what will apply the first time they execute.

**Note on the warning gate.** The record deviates from the default undefined-behaviour audit gate by
dropping `-pedantic`, with the reason recorded in its own `impl_defined_notes`: the subject under
test is a GNU extension, which is by definition not standard C, and `-pedantic` exists precisely to
reject such constructs — with it in force the program could not be compiled at all and the feature
would go untested, which C3 forbids. Every other member of the gate is retained, `-Werror` included,
so no other class of defect in this program is downgraded. A gate deviation is not a divergence: the
gate is a property of the **test material**, driven by the reference compiler only, and it renders no
verdict about `bcc`.

**The ambiguity a run will settle, deliberately not resolved here.** It is not yet known whether the
omission is an **implementation gap** (the frontend does not accept case ranges, and the
documentation correctly reflects that) or a **documentation gap** (the frontend accepts them and the
inventories are merely incomplete). The two lead to opposite actions, and picking one silently would
be worse than either:

- If it is an **implementation gap**, the first run produces a `FINDING`: a real, undocumented
  divergence, delivered with its reproducer and reproduction commands. No compiler change is made in
  response — findings are reported, never patched. If a maintainer then documents the limitation in
  the repository, §8.5 becomes available and the finding may be reclassified by minting a marker at
  that point, on a basis that exists.
- If it is a **documentation gap**, the program simply agrees, the verdict is `PASS`, and the correct
  action is to extend the documented inventory so the next reader is not misled the same way. Note
  that a speculative marker would have turned this outcome into a **failing run** — which is the
  concrete reason none is attached.

Either way the first run tells a maintainer which it is, without anyone having to guess in advance.

### 4.2 `long double` across the backends — analysed; a recorded exclusion, not a marker

The program this subsection governs — `tests/conformance/13_floating_point/004_long_double_target_restricted.c`
— belongs to the floating-point area and is **not yet committed**. The analysis is recorded now
because the measurement already settles it, and because it is the reason the type must be *written*
rather than dropped when that area lands.

| Field | Value |
|---|---|
| **Status** | analysed; the program is not yet in the corpus, and **no marker is provisioned for it** |
| **Program (planned)** | `tests/conformance/13_floating_point/004_long_double_target_restricted.c` |
| **Record (planned)** | `tests/conformance/13_floating_point/004_long_double_target_restricted.expected` |
| **Oracle (a)** | **enabled** — the same-target reference comparison, on all four targets, at all three optimization levels |
| **Oracle (c)** | **enabled** — the golden record, on every cell |
| **Oracle (b)** | **disabled in the record**, with the measured reason recorded in `impl_defined_notes`; the not-attempted cells are reported `XFAIL` citing that reason (§3.3) |
| **Verdict if oracle (a) or (c) diverges** | `FINDING` — nothing about representation excuses a same-target disagreement |

**The difference.** `long double` does not have one representation across the four supported targets.
Measured directly in this environment:

| Target | `sizeof(long double)` | Representation |
|---|---|---|
| x86-64 | 16 bytes | x87 80-bit extended, padded |
| i686 | 12 bytes | x87 80-bit extended, padded |
| AArch64 | 16 bytes | IEEE binary128 |
| RISC-V 64 | 16 bytes | IEEE binary128 |

Cross-backend **value** equality is therefore not merely hard to achieve for this type — it is
meaningless. Two backends printing different digits for the same `long double` computation are both
correct for their own target, so a cross-backend difference here is *not* evidence of a defect in
either. That is precisely the implementation-defined carve-out the suite's brief reserves: a
cross-backend divergence attributable to a documented implementation-defined difference in type width
or representation is not a compiler defect.

**The documented basis for the difference.** `docs/technical-specifications.md` line 511 specifies the
type representation as "target-parametric sizes" covering "floating types (float, double, long
double)" — the size of `long double` is, by the repository's own design, a function of the target
rather than a constant. The target table at lines 457–462 records the same principle for the
properties it enumerates: `i686-linux-gnu` is ELF32 with a 4-byte pointer and 4-byte `long`, while
the other three targets are ELF64 with 8-byte pointers and 8-byte `long`. The measured byte sizes
above are the corroborating evidence.

**Handling — a recorded exclusion, and the worked example of constraint C3.** The type is **not
dropped**. The program is to be written and run on **all four targets at all three optimization
levels**, and:

- **Oracle (a) is fully in force.** Every target's output is compared byte-for-byte against the
  *same target's* reference compiler, which is the comparison that can actually detect a defect here
  — both sides then use the same representation, so any difference is a real disagreement, and a
  divergence is a `FINDING`.
- **Oracle (c) is fully in force.** Every cell is asserted against the golden record, so a regression
  that moved `bcc` and the reference compiler together would still be caught.
- **Oracle (b) is switched off in the record**, with the measured reason above written into the
  program's own `impl_defined_notes`. The record format refuses a narrowing without a recorded
  reason, so the exclusion cannot be silent; the not-attempted cells are still counted and still
  listed, reported `XFAIL` with that reason printed in full (§3.3). This is the second of the two
  `XFAIL` forms: not a marker-covered divergence, but a comparison the corpus deliberately declines
  to make, for a reason on the record.

This is exactly what C3's *"state so explicitly and explain why"* asks for, rather than dropping the
type: the exclusion is **narrow** (one oracle, one program), **explicit** (a disabled toggle that the
format will not accept unexplained) and **explained** (a measurement plus a documented basis).

**Why no marker — and why provisioning one would be wrong twice over.** First, §3.3: a marker whose
scope named only oracle (b) while the record has oracle (b) switched off would be **dormant** —
nothing could ever consult it, so it could reach neither `XFAIL` nor `XPASS` — and the record format
**refuses that state at parse time**, so such a record would not even load. Second, an identifier
written here before its record exists would fail the reverse direction of the audit in §1.1 and take
the whole suite down with it. The recorded-exclusion route reaches the same reported outcome —
`XFAIL`, counted, with its reason visible — using the mechanism the harness actually implements for
a comparison the corpus declines to make.

**If a maintainer ever re-enables oracle (b) for this program** — for example to compare a
representation-independent property instead of a value — then the recorded reason no longer explains
the cells that are now attempted, and a cross-backend value divergence becomes a `FINDING` unless a
marker is minted at that time on a documented basis, by the checklist in §8.5. What is not permitted
is either half alone: an oracle switched off with no recorded reason, or a marker describing a
comparison the record does not perform.

### 4.3 Wide and Unicode string literals — analysed, no marker

`tests/conformance/11_literals_and_strings/003_wide_and_unicode_literals.c` is a **full participant
in the corpus**: written, with all three oracles enabled and no marker attached. Its record declares
all four targets and all three optimization levels, so the suite schedules twelve `bcc` cells for it
on the same footing as every other program.

**The documentation gap, and what it does and does not establish.** Support for the wide, UTF-8,
16-bit and 32-bit string and character literal prefixes is **not enumerated** in the documented
literal inventory:

- `docs/technical-specifications.md` line 99 describes literal handling as "Numeric literal parsing
  (decimal, hex, octal, binary, float), string/character literal parsing with escape sequences" — no
  literal prefix is named.
- Line 498 repeats the inventory in the file plan, listing the suffixes `u`, `l`, `ll` and `f` plus
  C escape sequences, and again no prefix.
- Line 497's 44-keyword C11 list contains neither of the two C11 character type names.
- Recursive searches of `docs/` for `unicode`, `char16`, `char32` and `wchar` return **zero**
  matches.

What that establishes is that nothing in the repository *mentions* these prefixes. It does **not**
establish that the frontend rejects them — and the difference is the whole reason no marker is
attached. These are standard C11 constructs rather than an extension, so a frontend that implements
C11 plausibly handles them already.

**Why no marker is attached, which is the interesting part.** A speculative marker would be actively
harmful. If `bcc` handles these literals correctly, a marker would claim a divergence that never
existed, the cell would agree, the verdict would be `XPASS`, and **the run would fail** on a mistake
in the test material rather than a defect in the compiler. Worse, the marker would blind the suite to
a genuine future regression in exactly that construct.

**Until then, a divergence here is a `FINDING`** — and that is the correct outcome, not a compromise:
an undocumented divergence is precisely what requirement 6 defines a finding to be. It is delivered
as a **verbatim** reproducer — a byte-for-byte copy of this program — together with its recorded
minimization status, the captured outputs per compiler and per backend, an environment fingerprint
and exact reproduction commands. A run performs no automated reduction; reduction is a supervised
activity performed on that copy before a finding is promoted to the curated set. Nothing here is
excused.

**What it would take to mint a marker later.** Not an observation alone, and not the omission alone —
**both**, and the omission is the harder half, because an omission is not a documented limitation
(§4, §8.5 step 2). Concretely:

1. **A repository artifact must explicitly document the limitation.** Extending the literal inventory
   at `docs/technical-specifications.md` lines 99 and 498 to record that the prefixes are not
   supported would do it. Until such a statement exists there is no citable basis, the basis grammar
   in §2.4 has nothing to check, and the divergence stays a `FINDING`.
2. **Confirm the divergence is real and reproducible** — the same divergence, from the recorded
   commands, on a clean workspace, on more than one run — capturing the outputs from each compiler
   and each backend.
3. **Add all five `expected_divergence.*` keys** to
   `tests/conformance/11_literals_and_strings/003_wide_and_unicode_literals.expected`, **minting the
   identifier at that time**, with `expected_divergence.basis` citing the artifact and section from
   step 1. Keep the scope no wider than the oracle, targets, levels and class actually observed, and
   write the observation as it was actually seen.
4. **Add a matching row to the table in §4 and a detailed subsection here**, reproducing the record's
   basis string **verbatim** (§2.4).
5. **Re-run the register audit** — `cargo test --test conformance infra_expected_divergence_register`,
   on the package-complete branch (§1.1) — to confirm the record and this register agree in both
   directions.

The program's own `impl_defined_notes` already records this gap descriptively, together with the same
reasoning about why no marker is attached, so the two documents agree today and will keep agreeing
after any future activation.

## 5. Handled by construction — not by marker

**A marker implies a test that actually diverges.** Where a measured implementation-defined
difference is *designed around*, so that no divergence occurs at all, a marker would be simply
wrong: it would claim a divergence that the corpus has already eliminated, the comparison would
agree, and the verdict would be `XPASS`. The correct record for a designed-around difference is an
`impl_defined_notes` entry in each affected program — which the record format requires anyway — and
an entry in this section so the decision is visible in one place.

The following three properties could each make a program's output depend on which target it was
built for. All three are **handled by construction, with no marker and no target restriction**, so
every affected program stays fully compared on all four backends. The first two were measured to
differ; the third is implementation-defined and documented nowhere in this repository, which is a
stronger reason to design the dependence away rather than a weaker one.

### 5.1 Plain-`char` signedness

Measured **signed** on x86-64 and i686, **unsigned** on AArch64 and RISC-V 64.

*Handled by:* never using plain `char` for a value whose signedness can affect the output. Programs
use explicit `signed char` and `unsigned char` where the distinction is the subject under test, and
where a byte must be widened for printing it is converted through `unsigned char` first. No program
prints a plain-`char` signedness-dependent value.

*Why not a marker:* restricting the target list would have removed three backends from the
comparison for a property the corpus can simply stop depending on. Removing the dependence keeps all
four targets fully compared and excludes nothing.

### 5.2 `sizeof(long)` and pointer width on i686

Measured **4 bytes on i686** and **8 bytes on the other three targets**, for both `long` and
`void *`. This is not an accident of the environment: `docs/technical-specifications.md` lines
457–462 specify `i686-linux-gnu` as ELF32 with a 4-byte pointer and a 4-byte `long`, against ELF64
with 8-byte pointers and `long` for x86-64, AArch64 and RISC-V 64.

*Handled by:* width normalization. Programs use fixed-width types or `long long` and
`unsigned long long`, both 64-bit on all four targets, for any value that is printed. Array extents
are printed as element counts rather than byte counts. Pointer facts are expressed only as
**differences, comparisons and alignment residues** — never as a printed address, which would be
non-deterministic in any case. One-past-the-end pointers may be formed but are never dereferenced.

*The same hazard applies to `size_t`, `ptrdiff_t` and `intptr_t`*, whose widths follow the pointer
width, so no program prints a value of those types either; a count that must be printed is converted
to `long long` or `unsigned long long` first.

*Why not a marker:* the difference is documented and expected, and depending on it would produce a
cross-backend divergence caused by the test rather than by the compiler.

### 5.3 The underlying type of the wide and Unicode character types

The signedness and width of `wchar_t`, `char16_t` and `char32_t` are **implementation-defined**, and
this repository fixes them for no supported target: recursive, case-insensitive searches of `docs/`
for `wchar`, `char16`, `char32` and `unicode` return **zero** matches. No per-target table is given
here for exactly that reason. One measured on this machine would describe the reference toolchain
installed today rather than any contract `bcc` documents, and citing it would make the corpus depend
on a property no repository artifact settles — which is a stronger argument for removing the
dependence than a measured difference would have been.

*Handled by:* never naming `wchar_t`, `char16_t` or `char32_t` anywhere in the corpus. Two reasons
compound here, and either alone would be sufficient. First, relying on the signedness of any of
those types would risk a cross-backend divergence caused by the test rather than by the compiler.
Second, the 16-bit and 32-bit character type names cannot be named at all: they are `uchar.h`
typedefs, and neither `uchar.h` nor `wchar.h` is among the nine **required** bundled freestanding
headers (`docs/technical-specifications.md` line 19 and lines 202–214), so nothing in the corpus can
reach them. Two header exceptions are sanctioned, and only two — neither of which helps here: the
programs in area `07_variadics` may include `<stdarg.h>` and nothing else, and the dedicated
bundled-header probe `12_preprocessor/003_bundled_header_inclusion.c` may include the nine required
bundled freestanding headers, with the bonus `stdatomic.h` deliberately excluded from both. Every
other program — `11_literals_and_strings/003_wide_and_unicode_literals.c` included — includes no
header at all and hand-declares the single libc prototype it needs.

Wide and Unicode literals are therefore **indexed in place**, and each element is cast explicitly to
`long long` or `unsigned long long` before printing, with every code point restricted to the range
`0x00`–`0x7FFF` so that no printed value can be affected by the signedness or the width of whichever
element type the implementation chose.

*Why not a marker:* same reasoning as §5.1 — the dependence is removed rather than excused, and all
four backends stay compared.

### 5.4 Mapping a universal-character-name to the extended execution character set

C11 leaves **implementation-defined** how a universal-character-name maps to a member of the
extended execution character set, and that mapping is what every wide (`L`) value in
`11_literals_and_strings/003_wide_and_unicode_literals.c` is. This is the sharpest
mislabelling hazard in the corpus: an implementation that mapped `U+00A9` to anything other than
`0xA9` would still be **conforming**, yet a cross-backend comparison against a target that maps it
to `0xA9` would report a divergence that is not a compiler defect at all.

*Handled by:* **pinning the mapping with `_Static_assert` rather than assuming it.** The program
asserts every wide value it uses — `0x41`, `0x24`, `0xA9`, `0x100`, `0x7FF`, `0x7FFF` and `0x7A` —
so an implementation whose mapping differs is refused **at translation time**, with a message
naming the expected value, instead of silently producing a number that the oracles would then have
to attribute after the fact. Verified as a negative control: altering one asserted value makes
translation fail with `static assertion failed: wide execution character set: U+00A9 must map to
0xA9`. The 16-bit and 32-bit values are pinned the same way; those are *specified* by C11 6.4.5 as
UTF-16 and UTF-32 rather than implementation-defined, and are asserted only so that an incorrect
encoding is also refused at translation time.

*Measured, per target, with the reference toolchain (gcc 13.4.0):*

| Target | `__STDC_ISO_10646__` | `sizeof(wchar_t)` | `wchar_t` signedness | `U+00A9` → |
| --- | --- | --- | --- | --- |
| x86-64 | 201706 | 4 | signed | 169 |
| i686 | 201706 | 4 | signed | 169 |
| AArch64 | 201706 | 4 | **unsigned** | 169 |
| RISC-V 64 | 201706 | 4 | signed | 169 |

All four define `__STDC_ISO_10646__`, which is the standard's own mechanism for an implementation
to *document* that `wchar_t` values are Unicode code points, and all four map every
universal-character-name in the program to its code point.

*`__STDC_ISO_10646__` is deliberately **not** used as a compile-time gate.* Requiring it was tried
and rejected on measurement: clang 20.1.8 maps every universal-character-name in the program to its
correct code point **while defining no such macro at all**. Gating on the advertisement rather than
on the behaviour would refuse a compiler that is doing exactly the right thing, and would refuse the
compiler under test for a property the suite never needed it to announce. The macro is recorded as
corroborating evidence; the asserts are the gate, because they test the property the printed values
actually depend on.

*Why not a marker:* because the dependence is **removed as a hazard rather than excused**. The
mapping agrees on all four targets and is now pinned, so oracle (b) stays **enabled** and all four
backends remain compared — which is the outcome a marker would have given up. Should a compiler ever
fail one of these asserts, that is not a finding either: it is a conforming
implementation-defined difference, and the correct response is to narrow that program's target list
and mint a marker here citing this subsection, never to relax the assert and never to drop the
feature from the corpus.

### 5.5 Measured non-differences — why several areas need no restriction at all

These are equally load-bearing, and for the opposite reason: each was measured **identical on all
four targets**, which is why the areas that exercise them are compared without restriction, and why
a divergence there is a **genuine finding** rather than an expected implementation-defined
difference. Recording them here is what stops a future maintainer from "fixing" a real defect by
adding a marker for a difference that was never expected in the first place.

| Property | Measured result on all four targets |
|---|---|
| Bitfield layout, size, alignment and exact byte image | Identical, including a straddling 3-bit / 5-bit / 9-bit sequence and the byte image it produces |
| Right shift of a negative signed value | Arithmetic (sign-propagating) everywhere |
| Integer division and remainder signs | Division truncates toward zero; the remainder takes the sign of the dividend |
| Byte order | Little-endian everywhere, as the target table records |
| Escape sequences and hexadecimal / octal formatting | Byte-identical output |
| Character-literal values | Identical |
| Variadic argument passing, integer **and** `double` | Byte-identical at all three optimization levels |
| `sizeof(enum E) == sizeof(int)` for an enumeration whose values fit in `int` | True on all four, `sizeof` four bytes, at all three optimization levels — twelve configurations measured, all agreeing |

**End-to-end validation of the oracle contract.** One program compiled for all four targets at
`{-O0, -O1, -O2}` — twelve configurations — produced **byte-identical stdout in every one**. The
oracle contract this register governs is therefore not aspirational: agreement across the full
matrix is the measured normal case, which is exactly what makes a disagreement worth investigating.

## 6. Markers deliberately not created

Three documented limitations were considered as candidates and deliberately **not** turned into
markers. Recording the decision explicitly is how the omission stays a decision rather than
becoming an oversight that nobody can distinguish from carelessness.

| Documented limitation | Where it is documented | Why no marker |
|---|---|---|
| Shared-library / dynamic-loader validation gap | `docs/project-guide.md` line 109 (remaining work: shared library end-to-end validation) and line 246 (open risk: shared library dynamic loader incompatibility, "Open — Requires runtime validation") | **Outside this suite's oracles.** Every test binary is built with `-static`, which is also the one linkage mode both compilers spell identically and the reason emulator execution needs no sysroot. No cell ever produces or loads a shared object, so the limitation cannot manifest here and a marker would describe a comparison the suite never makes. |
| DWARF debugger-validation gap | `docs/project-guide.md` line 110 (remaining work: DWARF v4 debugger compatibility testing) and line 247 (open risk: DWARF v4 debugger parsing failures, "Open — Requires manual testing") | **Outside this suite's oracles.** No debugger is ever invoked and `-g` appears in no differential invocation; the suite compares stdout bytes and exit status only. Debug information is therefore never observed, so nothing here could diverge on it. |
| Error detection — the rejection of invalid programs | Not a repository limitation but a **noted non-goal** of this suite | **Not measurable by either mandated oracle.** Both oracles require the program to compile and run successfully before anything can be compared, so an invalid program produces no comparable output. It is a valuable future axis — naming it here makes its absence deliberate rather than an omission — but it is a different kind of test suite, not an expected divergence. |

**Out of scope entirely, and therefore not expected divergences either.** The following are in the
technical specification's own out-of-scope list (`docs/technical-specifications.md` §0.6.2, lines
696–725) and consequently lie outside the suite's matrix rather than inside it as excused
divergences: `-O3` and higher optimization levels (line 716 — only `-O0`, `-O1` and `-O2` are in
scope, which is exactly the optimization matrix this suite sweeps, and `-Os` appears in no
documented level list at all); sanitizers (line 713 — used **only** against
the reference compiler in the undefined-behaviour audit gate, never passed to `bcc`); link-time
optimization (714); profile-guided optimization (715); loop unrolling, vectorization and
auto-parallelization (717); interprocedural optimization (718); PE/COFF (702) and Mach-O (703)
object formats; Windows and macOS hosts and targets (704); C++ or any language other than C (705);
and any architecture beyond the four supported ones (723). A construct that is out of scope is not
compared at all, so it cannot diverge, and no marker may claim otherwise.

## 7. Documented bases available for future markers

A marker may only reclassify a divergence on the authority of something this repository already
documents, and the cited document must be contained, readable and carry the locator the citation
names (§2.4). Today that means one of exactly two files.
The list below is the inventory of citable sections, so that a maintainer minting a marker can find
a real basis instead of inventing one — or discover that there is no documented basis, in which case
the divergence is a **finding**, not an expected divergence.

**`docs/technical-specifications.md`**

| Lines | Section | What it authorises a marker to say |
|---|---|---|
| 13, 55, 98, 107, 506, 761 | The GCC extension inventory, stated six times: the feature requirement, the implementation strategy, the lexer keyword table, the parser extension module, that module's file plan, and the C11 + GCC Extensions Compliance Rule | That a GCC extension is not among those the frontend is documented to support |
| 19, 202–214 | The bundled freestanding header set — nine headers, and no `stdio.h` | That a header a program would need is not shipped, and therefore that a program needing one of its names must hand-declare instead — the only sanctioned inclusion being the freestanding `<stdarg.h>` that area 07 cannot do without |
| 99, 497, 498 | The literal inventory (numeric forms, escape sequences, and the `u`, `l`, `ll` and `f` suffixes) and the C11 keyword inventory (44 keywords) | That a keyword, literal form or literal prefix is not enumerated among those documented |
| 457–462 | The target table: pointer size, `long` size, ELF class, endianness and register file per target | That a difference between targets is a documented implementation-defined property rather than a defect |
| 511 | Type representation with target-parametric sizes, covering `float`, `double` and `long double` | That a type's representation is, by design, a function of the target |
| 696–725 (§0.6.2) | The explicit out-of-scope list | That a construct is outside the implementation's scope entirely — though see §6: an out-of-scope construct is normally not compared at all rather than excused |

**`docs/project-guide.md`**

| Lines | Section | What it authorises a marker to say |
|---|---|---|
| 81, 206 | The parser subsystem row and the requirement conformance matrix, both enumerating the GCC extensions that are parsed | The same extension-inventory basis as above, from the guide's side |
| 109, 110, 111 | Remaining work: shared library end-to-end validation; DWARF v4 debugger compatibility testing; **C11 Standard Corner Case Compliance Testing** — the item this whole suite exists to close | That a validation activity is documented as outstanding |
| 246, 247, 248 | The open risk register: shared library dynamic loader incompatibility ("Open — Requires runtime validation"); DWARF v4 debugger parsing failures ("Open — Requires manual testing"); and **C11 corner case non-compliance** — *"edge cases in complex declarators and type conversions may remain"*, status *"Open — Requires targeted testing"* | That a class of non-compliance is documented as an open, unmitigated risk |

The last row deserves emphasis, because it is the reason this suite exists at all: the repository
already records C11 corner-case non-compliance as an open risk whose mitigation is *targeted
testing*, and names the corresponding work item at line 111. This register is where the outcome of
that targeted testing becomes auditable.

## 8. Provenance, constraints and maintenance

### 8.1 Provenance

Every fact in this register comes from one of three places, and each is checkable:

- **The two documents in §7**, cited by file and line, and machine-verified to exist.
- **Direct measurement in the suite's own environment** — the byte sizes, signedness, layout and
  formatting results in §4.2 and §5 — reproducible from the commands recorded in each program's
  `.expected` record, with no harness required.
- **The programs' own records**, which are authoritative for their markers. Where this file and a
  record could ever disagree, the record is right and this file is the defect; the bidirectional
  audit in §1.1 is what stops the disagreement from lasting.

**User-specified rules:** none exist for this project. The rules document was consulted and
reports that no user rules were provided, so no rule places this file in scope — it exists
because requirement 5 requires every expected divergence to be auditable in one place. Their
absence is **not** permission to lower the bar: the binding constraint set is the four
constraints in §8.2 plus the repository's own documented engineering standards, applied at
enterprise standard throughout. The rules document remains the authoritative source for the full
text of any rule added later.

### 8.2 Binding constraints, and what each means for this register

| Constraint | What it requires | Consequence here |
|---|---|---|
| **C1 — no compiler source change** | `src/**`, `include/**`, `build.rs`, `Cargo.toml` and `Cargo.lock` are read-only reference material. | Nothing in this suite modifies any of them. **This directory contains no `.rs` file at any depth** — that is precisely what keeps Cargo blind to it, so it is never a build target and the package manifest needs no change at all. |
| **C2 — no existing test weakened** | No existing test may be deleted, skipped, weakened or relaxed; no `#[ignore]` attribute may be added or removed. | No marker in this register changes an existing test, and the suite declares no ignored test and no harness test function of its own, so it can move neither the repository's test count nor its ignored count — which is the part that is mechanical here. The count itself, **exactly 13 ignored**, is a whole-repository property no integration test can read, so it is measured by the health gate (`cargo test 2>&1 | grep "test result"`) once this suite and the compiler are on one branch. |
| **C3 — never exclude a feature because it is difficult** | If a feature cannot be tested, say so explicitly and explain why, rather than dropping it. | **Every entry in §4 exists because of C3.** Case ranges (§4.1) are absent from every documented inventory and are tested anyway, with **no** marker minted on that omission; the wide and Unicode literal prefixes (§4.3) are likewise unenumerated and likewise fully tested; and `long double` (§4.2) has three different representations across four targets and is analysed here so that it is written rather than dropped when the floating-point area lands. Where a comparison genuinely cannot be made, the exclusion is narrowed to a **single oracle**, the program keeps running under the remaining oracles, and the reason is recorded in the program's own record — never here alone. |
| **C4 — contained execution** | Generated programs may not reach the network or any path outside the sandbox working directory. | Discharged by two separate mechanisms, and keeping them apart is what makes the claim checkable. **Corpus-authoring policy:** every input is a literal in the program source; no program opens a socket or reads a file, and the whole corpus has exactly **one** fixture file — the header used by the include-path flag probe. **Path discipline:** each cell is launched with its own workspace as its working directory, and the harness confines every path it constructs to roots beneath the build directory. **Environment isolation:** every child is spawned with the environment cleared and a small fixed set installed in its place — a search path restricted to the `PATH` entries that are absolute and not writable by an untrusted account, a fixed C locale, `TZ=UTC`, `TERM=dumb`, the strictest sanitizer options, and the cell's workspace under `HOME`, `TMPDIR`, `TMP` and `TEMP` — so no credential and no behaviour-changing variable reaches a program the suite does not control, and a compiler driver cannot be made to execute a substituted `cc1` or `as` from a directory tool resolution refused. None of the three is an operating-system sandbox: there is no namespace, `chroot`, seccomp filter, landlock profile or network restriction around any child; a tool that hard-codes a temporary path rather than reading `TMPDIR` keeps its own temporaries where it always did; and whether a crash writes a core image outside the workspace is decided by the host's `kernel.core_pattern` and core-size limit rather than by the harness — see `README.md` §"C4 — Contained execution". Untrusted input must be run under an external sandbox. |

**Zero External Crate Dependency Rule.** Quoted verbatim from `docs/technical-specifications.md`
§0.7: *"The `[dependencies]` section of `Cargo.toml` must remain completely empty at all times"*;
*"No `[build-dependencies]` or `[dev-dependencies]` entries for external crates are permitted"*;
*"This constraint is absolute and admits no exceptions."* This is why the register is a committed
Markdown document parsed by a hand-written check rather than a structured data file handled by a
serialization crate, and why no snapshot-testing, coverage or parameterization crate appears
anywhere in the suite.

**Report, never patch.** Requirement 6 and C1 agree without tension: **no compiler source change is
made in response to any divergence, expected or otherwise.** An expected divergence produces a
marker and an entry here; an undocumented divergence produces a finding artifact and an entry in
the findings register, `tests/conformance/FINDINGS.md` — a **planned** companion to this file, not
yet committed, which is why it is named here as plain text rather than linked. Neither produces a
patch.

**Honest measurement.** No coverage figure is published in this file, and none can be: coverage
instrumentation requires a development dependency, which the rule quoted above forbids absolutely,
so any figure here would be unverifiable by anyone in this repository. Where the suite's breadth
must be referenced, it is referenced as an enumerable matrix — and the **final planned target** is
distinguished from what is **committed today**, because only the second is countable from the file
set as it stands:

| Matrix | Areas | Programs | `bcc` compile-and-run cells | Differential and golden assertions |
|---|---:|---:|---:|---:|
| **Final planned target** | 14 | 108 | 1,296 (108 × 4 targets × 3 levels) | ≈ 3,564 across the three oracles |
| **Committed today** | 11 | 92 | 1,104 (92 × 4 × 3) | ≈ 3,036 across the three oracles |

The committed figure is what a reader can verify right now by counting `tests/conformance/*/*.c`;
the planned figure is what the suite's design calls for once the three remaining areas land. Quoting
the planned figure as though it were the present state would be exactly the unverifiable claim the
paragraph above refuses to make.

### 8.3 Retirement log

Recorded by **date and description only** — never by identifier, because the reverse-direction audit
in §1.1 reads this whole document and would look for a marker that no longer exists. An entry naming
the program, the divergence and the date is enough for a reader to reconstruct the history from
version control.

| Date | What was withdrawn, and why |
|---|---|
| 2026-08-01 | The GCC case-range marker on `08_gcc_extensions/004_case_ranges.c` was **withdrawn without the divergence it described ever having been observed**. It had been minted on the strength of an omission from the documented extension inventory, which §4 and §8.5 step 2 both establish is not a documented limitation. Because nothing had been observed or reproduced, the marker risked failing a run on `XPASS` — on the test material rather than the compiler — and would meanwhile have blinded the suite to a real regression in exactly that construct. The program, its matrix, its oracle toggles and its golden record are **untouched**, so the feature remains fully under test; a divergence there is now a `FINDING` until an explicit documented limitation and a real observation both exist. The analysis the marker carried is preserved in §4.1 and in the program's own `impl_defined_notes`. |

### 8.4 Cross-links

| Document | What it holds |
|---|---|
| [`README.md`](README.md) | The suite contract: the three oracles, the verdict taxonomy, the environment variables, the artifact locations, the `.expected` record format (including the marker block), and how to reproduce any cell by hand |
| `FINDINGS.md` (**planned**, not yet committed — named as plain text for that reason) | The register of **undocumented** divergences. Each entry points at a finding directory holding a **verbatim** reproducer — a byte-for-byte copy of the corpus program — its recorded minimization status, and exact reproduction commands. Automated reduction is never performed during a run; reduction is a supervised activity performed on the copy before a finding is promoted to the curated set |
| [`../conformance.rs`](../conformance.rs) | The suite driver, including `infra_expected_divergence_register`, the test that keeps this file and the corpus honest in both directions |
| `docs/testing/differential-conformance.md` (**planned**, not yet committed — named as plain text for that reason) | The documentation-site page: methodology, oracle definitions, the build matrix, the verdict taxonomy and the deliverable summary format |

### 8.5 Add-a-marker checklist

The mirror of the retirement procedure in §3.2. Every step is required, and the run will tell you if
you skip one:

1. **Confirm the divergence is real and reproducible.** Same divergence, from the recorded commands,
   on a clean workspace, on more than one run. Capture the output from each compiler and each
   backend involved.
2. **Identify and verify the documented basis.** Find the actual file and section that
   **explicitly documents the limitation** — §7 is the inventory of citable sections — and write the
   citation with a **locator the audit can resolve**: a line, a line range, a `§` section number or a
   backtick-quoted phrase (§2.4). An **omission** from an inventory is not a documented limitation: it
   records that nothing mentions the construct, not that the implementation rejects it. **If there is
   no documented basis, there is no expected divergence:** the correct outcome is a finding, recorded
   in the findings register `tests/conformance/FINDINGS.md` (planned) with its reproducer.
3. **Add all five `expected_divergence.*` keys** to the program's `.expected` record. Keep the scope
   no wider than the oracle, targets, levels and class actually observed (§2.3), keep the oracle it
   describes enabled (§3.3), and write the observation as it was actually seen. For one of the four
   **build-refusal** classes the observed oracle set is *every* arm, because no artifact exists for
   any of them to compare — so `all oracles` is the accurate scope there, not a wider one (§2.3).
4. **Add the summary-table row in §4 and a structured entry** in the shape §1.3 defines: a heading
   naming the marker, then a table stating Identifier, Class, Scope, Program, Basis and Observed.
   Reproduce the record's basis string **verbatim** in both places — one canonical rendering,
   character for character (§2.4) — so that the record and this register cannot give two accounts of
   the same authority. The entry's six fields are compared against the record on every run, so a
   mismatch fails immediately and names both readings; the §4 summary row is prose and is not
   compared, so it is the one place a paraphrase can survive.
5. **Re-run the audit** — `cargo test --test conformance infra_expected_divergence_register`, on
   the package-complete branch (§1.1) — and then the owning area, to confirm the divergence now
   classifies as `XFAIL` rather than `FINDING`.

Do **not** attach a marker speculatively, before the divergence has been observed. A marker on a
program that agrees is an unexpected success, which fails the run — and until it is noticed, it
blinds the suite to a real regression in exactly the construct it was meant to document.
