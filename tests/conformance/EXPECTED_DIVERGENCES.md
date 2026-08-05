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
| **Forward — description** | Every such marker has exactly **one structured entry** here (§1.3), and every one of the entry's eight fields — Identifier, Class, Scope, Program, Basis, Documented, Evidence, Observed — **agrees with the record**. | Being mentioned is not being described. An identifier can sit in a heading while the register says nothing checkable, and the register could then drift away from the record it mirrors with no run noticing. |
| **Reverse** | Every identifier appearing in this register corresponds to a real, **active** marker in a real `.expected` record. | The register must describe divergences that are actually exercised, not ones retired without retiring the entry. |
| **Basis — containment** | Every cited path resolves to a document **inside this repository**, containment being decided on the fully resolved path. | A basis outside the repository is not something this repository documents, and an intermediate symbolic link must not be able to move the answer. |
| **Basis — a real document** | Every cited document is **read**, through the suite's bounded reader: a symbolic link, a device node or a FIFO at the final component is refused, the opened handle is proved to be the entry that was inspected, and an oversized file is refused. | Existence was never the property that mattered. The earlier check followed a link and asserted only that *something* was there, so a basis could point through a link at anything readable and still pass. |
| **Basis — a resolvable locator** | Every locator the citation contains resolves **inside** that document, and at least one locator is present (§2.4). | A citation a reader cannot follow is not a basis. This is what stops "the section that documents this limitation" from counting as an authority. |
| **Basis — followable, not adjudicated** | The audit establishes that the citation can be **followed**: the document exists inside the repository, is readable, and the section named resolves inside it. It does **not** judge whether that section supports the claim. | Whether an inventory's silence means a feature is unimplemented is a judgement, and one of the two mandated markers rests on exactly that. An automated verdict on it would be a guess wearing the authority of a check; the audit's job is to guarantee a reviewer has somewhere concrete to look. |
| **Documented — resolvable *inside the cited range*, when written** | The key is **optional**. When it is present it must carry a verbatim quotation from the cited document, long enough to identify a passage rather than a word, and the audit finds it **within the region the locator resolved to** — a line and its neighbours, a line range, or a section from its heading to the next — with runs of whitespace collapsed. | Searching the whole file let a marker cite one section and quote another, so the audit certified that the words were the document's own while establishing nothing about the section a reader was sent to. Bounding the search makes the two halves of a citation agree with each other, which is the only reason to ask for both. |
| **Evidence — captured, not predicted, when written** | The key is **optional**. When it is present it must name all five of `command`, `exit`, `output`, `toolchain` and `captured`, each with a value, and none of it may read as a prediction. | A captured observation is the strongest thing a marker can carry, and an author who has one should record it. Requiring it made the two mandated markers inexpressible on a branch with no compiler binary, which is a defect in the format rather than in the markers — so it enriches a marker and no longer gates one. Neither mandated marker writes it today, and §4.1 and §4.2 each say so rather than implying an observation that does not exist. |
| **Observed — not anticipatory** | `expected_divergence.observed` does not describe the divergence as **predicted**: wording such as "is expected to", "will reject", "anticipated", "no verdict has been recorded" is refused. | A marker written before the divergence was seen excuses a cell on the strength of an author's expectation. The correct record for an unobserved divergence is no marker at all — the run then reports it as a `FINDING`, which is exactly what requirement 6 asks for. |
| **Scope — a narrowed oracle is marked** | A record that **disables** an oracle must carry a marker whose scope names that oracle, beside the `impl_defined_notes` reason that is required independently. | A disabled oracle is reported `XFAIL`, so it already claims the authority of an expected divergence. Justified by prose alone it was invisible to this audit, had no identifier a report row could cite, and had no basis resolved against any document — a silent exclusion in the clothes of a documented one. |

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
| `XFAIL` | Any of three forms, and every one of them cites a **marker**: a divergence occurred and a marker covers this oracle, target, optimization level **and** class; or the comparison was **not attempted because the program's own record narrows coverage**, in which case a marker scoping that oracle and the `impl_defined_notes` reason are both required (§3.3); or the arm was **blocked by a marked root refusal** on another arm of the same cell, in which case the outcome names the root marker and the arm carrying it and states that no comparison was attempted (§2.3). | No |
| `XPASS` | A marker covers the cell but the comparison **agreed** — the marker is stale. | **Yes**, by default |
| `FINDING` | A divergence occurred that no marker covers. It is delivered as a self-contained artifact, never patched. | No — a finding is a deliverable |
| `FAIL` | Anything unexplained: harness breakage, an internal inconsistency, a corpus defect. | **Yes** |
| `UNAVAILABLE` | An oracle's tooling is genuinely absent from this machine. Reported loudly, never as a pass. | Only under `BCC_CONFORMANCE_STRICT` |

There is deliberately **no "skip because unsupported" verdict**. This register is what makes `XFAIL`
reachable — so that a divergence can be *explained* — without letting `PASS` absorb a divergence
nobody explained, and without a feature quietly disappearing from the corpus to avoid the question.

### 1.3 The shape of a structured entry

An entry is a **heading whose text contains the marker's identifier**, followed anywhere before the
next heading by a **two-column table** stating eight fields. The heading is what binds the entry to the
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
| Documented | <the marker's expected_divergence.documented> |
| Evidence | <the marker's expected_divergence.evidence> |
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
- **The three heredoc fields — `Documented`, `Evidence` and `Observed` — are compared with runs of
  whitespace collapsed**, and only because a table cell cannot contain a newline while those fields
  routinely do. The other five are compared **exactly** after trimming, which is §2.4's
  character-for-character rule made mechanical rather than merely asked for.

Two entries for one identifier is a failure, not a redundancy: one marker is one investigation, and a
reader faced with two entries cannot tell which is the authority.

## 2. The marker contract

### 2.1 The five required keys, and two whose requirement depends on the tier

A marker is a block inside a program's `.expected` record. **Five keys are required — either all
five are present or none is**: a partial block is a hard parse error, not a warning, because each
part carries weight the others cannot. This is the frozen contract the project specification fixes,
reproduced here rather than reinterpreted.

| Key | Holds | Why the block is worthless without it |
|---|---|---|
| `expected_divergence.id` | The stable identifier, mirrored in this register. | Without it the register cannot be cross-checked in either direction. |
| `expected_divergence.class` | One of the six divergence classes below. | Without it the classifier cannot tell which observation the marker excuses. |
| `expected_divergence.scope` | Which oracles, targets and optimization levels the marker covers. | Without it there is no way to tell which cells are excused and which are not. |
| `expected_divergence.basis` | The repository artifact and the located section that authorises the marker. | Without it the divergence is reclassified on no authority at all. |
| `expected_divergence.observed` | The divergence as seen, in prose, and **not** as anticipated. | Without it a reader cannot tell whether what they are looking at is what was marked; written in the future tense it would excuse a cell on an expectation. |

Two further keys are **tiered**: whether they are required depends on *who authorises the marker*.
For one of the exceptions the project specification fixes in advance they remain enrichment, because the
authority is not the record's to supply. For every other marker they are the only route in, and both are
mandatory. §2.1.1 states the rule; here is what each key holds:

| Optional key | Holds | What it adds |
|---|---|---|
| `expected_divergence.documented` | A **verbatim quotation** of the passage that authorises the marker, resolved **inside the region the basis locator names**. | A locator proves a section exists; a quotation from inside it proves the section says something, and pins which sentence the author meant. |
| `expected_divergence.evidence` | The **captured** observation: `command`, `exit`, `output`, `toolchain`, `captured`. | It records that somebody produced the divergence rather than reasoned about it, and it tells the next reader how to produce it again. |

#### 2.1.1 Two tiers of authority, and no third

A marker converts a real failing comparison into a non-failing verdict, so the only question that
matters about one is **who authorised it**. There are exactly two admissible answers, they are decided
in the harness before anything else about the marker is considered, and a marker that satisfies neither
is refused at parse time — the record does not load, and the program's cells are not run rather than
being quietly excused.

| Tier | What authorises the marker | `documented` | `evidence` |
|---|---|---|---|
| **Frozen** | An immutable allowlist **compiled into the conformance harness**, which fixes the identifier, the divergence class and the basis document for each exception the project specification names in advance. | enrichment | enrichment |
| **Observed** | Captured evidence, and nothing else. | **required** | **required** |

**The frozen tier cannot be reached by editing files, and that is the whole of its value.** The
allowlist lives in `tests/conformance_harness/manifest.rs`, in source that no expectation record and no
edit to this register can reach. A record may *instantiate* an entry and may do nothing else with it: if
it names a frozen identifier and declares a different class, or cites a different document, the record is
refused rather than believed. So the shape that used to work — edit a program's record to add a marker,
edit this register to mirror it, and watch the bidirectional audit certify that the two agree — no longer
authenticates anything. Two documents agreeing with each other was never evidence about a compiler; it
was evidence that one author wrote both.

**The observed tier is the only way a new exception enters, and it costs an observation.** Both keys are
required, and each is checked for something a sentence cannot fake: `documented` must quote the cited
passage verbatim and the quotation must resolve **inside the region the basis locator names**, and
`evidence` must carry all five of `command`, `exit`, `output`, `toolchain` and `captured`. That does not
make a marker true — no automated check can weigh whether a passage supports a claim — but it moves
forging one from *writing a plausible sentence* to *fabricating a reproducible observation*, which the
next run's §3.1 safeguard is positioned to contradict.

**Wording authenticates nothing, and treating it as though it did was the defect this replaces.** The
parser still refuses `observed` phrased as a prediction (§1.1, "Observed — not anticipatory"), and that
check is worth keeping for what it actually is: hygiene that turns a muddled record into a clear error at
the moment it is written. It is not an authority test. Its premise — that an author who has made no
observation will say so in one of a listed set of phrases — holds for an honest author writing carelessly
and fails completely for a careless author writing confidently, because any synonym, any paraphrase and
any flatly declarative sentence walks past it. Absence of blacklisted wording is evidence of nothing.

**Why the two mandated markers are frozen rather than evidenced, which is the case the two tiers exist
for.** Neither can satisfy the observed tier, and not because their authors were lazy. One rests on an
inventory's *silence*, which no sentence can quote. The other documents a type whose cross-backend
comparison the same specification switches **off**, so no arm exists that could ever produce a captured
observation for it. Requiring both keys of every marker would therefore not raise the standard: it would
make the two mandated markers inexpressible, and the exclusions they document would then be reported
`XFAIL` with no identifier, no register entry and no resolved basis behind them — strictly less auditable
than the markers such a rule refused. Making both keys unconditionally optional fails the other way, by
making every marker as weak as the weakest one. Tiering them is what lets a mandated exception be
expressible *and* a new exception be expensive.

**The specification names a third marker in advance, and it is deliberately not in the allowlist.** The
wide-and-Unicode-literal candidate of §4.3 — whose identifier this register does not spell, because §1.1's
reverse check requires every `XD-` token written anywhere here to resolve to a live marker — is fixed
*conditionally*: the specification directs that it be attached "only if a divergence is actually observed
and traceable to that documentation gap". A conditional-on-observation marker is precisely an
observed-tier marker, so freezing it would grant it the one thing its own definition withholds. Its
absence from the allowlist is the specification being followed rather than an entry overlooked, and §4.3
records the candidate as analysed with no marker. The allowlist in the harness spells the identifier in a
comment for the same reason this register does not: source is not scanned by the reverse check, so that is
where the name can be recorded without registering it.

**What a frozen marker still owes, and what is never suspended.** Disclosure rather than deletion: leave
`evidence` unwritten, and say in `observed` which half of the pair is measured and which is not. §3.1 is
not suspended for any tier. An unconfirmed marker on a construct that works produces `XPASS` and **fails
the run**, naming the marker and both places it lives, so the gap is a loud state nobody can walk past
rather than a silent pass. Retirement then takes an observation of its own — see §3.2 — and specifically
an **independent** one: an agreement produced by a stand-in that forwards to the reference toolchain is
one toolchain compared with itself, and settles nothing.

**Every non-failing verdict states its tier.** The detail of each `XFAIL` row names the marker and then
names what admitted it — a frozen exception, with the definition quoted, or captured evidence, with the
five fields it rests on. A reader deciding how much weight to give a comparison that differed and did not
fail the run never has to open another file to find out which kind of authority stopped it.

The exact record syntax — `key = value` lines, `#` comments and `key <<END … END` heredoc blocks —
is documented in [`README.md`](README.md). It is not restated here, so that there is one authority
for the format rather than two that can disagree.

### 2.2 The six legal divergence classes

These six are the **only** observational values of `expected_divergence.class`. The divergence-class
space is closed: adding a seventh is a compile error in the harness until every decision point
handles it. One further value exists and names **no** observation — `comparison_excluded`, §2.2.1 —
so the accepted set of the `class` key is these six plus that one.

| Class | The observation it names |
|---|---|
| `compile_failure` | One compiler rejected a program the other accepted. Raised by the build layer, so **no artifact exists** — a refusal, not a comparison (§2.3). |
| `link_failure` | The program translated but did not link. Also a build-layer refusal with no artifact. When a target's C runtime is simply absent from the machine, this is `UNAVAILABLE` at environment scope instead, never a divergence against the compiler. |
| `run_crash` | The program died on a signal rather than exiting. Raised **after** an artifact was built and launched, so it is a status difference between two completed attempts to run, not a refusal. Compared as a raw wait status, so it is never conflated with a numerically equal ordinary exit. |
| `exit_code_mismatch` | Two completed runs disagreed on exit status. |
| `stdout_mismatch` | Two completed runs disagreed on stdout bytes — the ordinary shape of a wrong answer. |
| `timeout` | An invocation outlived its budget. The **only class reachable from both paths**: a compile invocation that never returns is a refusal with no artifact, while a run that never finishes is a status difference on a built artifact. A first-class divergence either way — a program that finishes promptly under one compiler and hangs under another is a defect worth surfacing. |

Standard error is never compared, by any oracle, so no marker may describe a difference in
diagnostic text. Diagnostic wording legitimately differs between compilers; comparing it would
produce a flood of divergences that say nothing about code correctness.

#### 2.2.1 `comparison_excluded` — the one class that names no observation

| Class | What it names |
|---|---|
| `comparison_excluded` | **No comparison is attempted** on the arm this marker scopes, because the program's own record disables that oracle for a reason recorded in its `impl_defined_notes` (§3.3). It is a coverage restriction with an identifier and a resolved basis, not an observation. |

**Why it has to exist.** Spelling a narrowing marker with an observational class — `stdout_mismatch` is
the natural choice for a *value* comparison that has been switched off — asserts, in the one field a
report row quotes, that two completed runs disagreed on bytes. Nothing ran. Every automated check would
pass anyway, and necessarily so: a class is only ever *matched* against an observation, and the arm in
question produces none, so the false statement would sit in a machine-checked field that no machine
could contradict, and a reader auditing this register would be told an observation had been made.

**What the parser enforces, in both directions.** A marker scoped to *any* oracle its record
disables **must** carry this class; a marker carrying this class **must** scope only oracles its record
disables. A mixed scope is therefore inexpressible, and that is intended — a marker either explains
what a comparison saw or explains why a comparison is not made, and one marker cannot honestly do
both. Splitting such a marker in two costs one register entry and buys an unambiguous account of each
arm.

**What it changes about classification: nothing, by construction.** The harness compares an observed
divergence against `Observed(<class>)`, and this value is not of that shape, so no observed divergence
can ever be excused by it — the type system rather than a convention is what guarantees that. It
therefore cannot reach `XPASS` either (§3.3), because nothing is compared on the arm it scopes. What it
buys is exactly what §3.3 says a narrowing needs: an identifier a report row can cite and a basis the
bidirectional audit resolves.

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

**Two of the six classes are build refusals, and a build refusal is one root event whose every arm
the classifier — not the author — makes visible.** `compile_failure` and `link_failure` are raised by
the build layer: the program was rejected, or it translated and did not link, so **no artifact
exists** and the same root cause denies all three oracles their subject at once — there is no program
for the same-target reference comparison to run, none for the cross-backend comparison to run, and
none to compare against the golden record. `timeout` reaches this path too when it is the **compile
invocation** that outlived its budget, for the same reason: nothing was produced.

**`run_crash`, and a `timeout` on the run, are not build refusals and must not be read as ones.**
Both are raised only *after* an artifact was built and launched: a program that died on a signal, or
one that never finished inside its budget. Each is a **status difference between two completed
attempts to run**, assembled by the comparator alongside any byte difference and entered into
classification as a comparison that happened, on the single arm that made it. No arm loses its
subject, so no dependency root is involved and none of the propagation below applies. The artifact,
the captured stdout up to the point of death, and the raw wait status are all available and all reach
the finding, which is why a signal death is the most diagnostic class the comparator can report
rather than an absence.

The scope of a refusal marker follows from *which* claim the basis supports, not from how many arms
the refusal blocks. The frozen contract scopes its refusal marker `oracle_a` alone — rightly, because
oracle (a) is the only arm on which "the reference compiler accepted this program and the compiler
under test did not" is a statement about the two compilers. Requiring `all oracles` instead, on the
grounds that all three arms lose their subject, would turn a classifier responsibility into an
authoring obligation and have oracles (b) and (c) claim a comparison they never made.

What happens instead is **dependency-aware classification**:

- the arm the marker's scope names settles the refusal as the expected divergence it is — `XFAIL`,
  citing the marker and its documented basis;
- every other applicable arm of the same cell is reported as a **dependent blocked** outcome. It is
  also `XFAIL`, and its detail names the root marker, names the arm that carries it, and states
  plainly that **no comparison was attempted on this arm** — so the outcome is visible and counted
  without claiming an authority it never consulted;
- **no separate finding is filed** for the blocked arms, because there is one root event and it is
  already explained.

Measured on the whole matrix, with the refusal reproduced against a stand-in compiler that rejects
the construct: a program refused on all four targets at all three levels produces **12 `XFAIL`
roots** on the marked arm and **21 dependent `XFAIL` arms** — 12 on oracle (c) and 9 on oracle (b),
which is 9 rather than 12 because oracle (b) does not apply to the baseline target at all, there
being nothing for the baseline to be compared against. **Zero finding directories**, where the
un-propagated form produced 21 of them for one root event.

The propagation is deliberately narrow: it requires a marker that already covers this cell's target,
optimization level **and** observed class on some arm. A marker covering none of them propagates
nothing, and every arm stays a `FINDING` — which is what keeps an *undocumented* refusal a finding on
all three arms.

A marker that genuinely means to speak for all three arms may still be scoped `all oracles`, and then
each arm settles on the marker directly. Both spellings are legitimate; neither is imposed on an
author by the format, and no scope is ever widened by anyone but the person who wrote it.

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
- **A quoted phrase or a section number is preferred to a line number in a document the project still
  edits.** All four forms resolve, so this is a durability rule rather than a validity rule: inserting
  one table row into a cited document shifts every line beneath it, and a line locator then names the
  row *above* the one intended while still resolving — the audit widens a single-line citation by one
  line either side, so the drift passes the machine check and misleads only the reader. A phrase moves
  with the passage it quotes. Both live markers cite that way, and §7 records which form each cited
  document takes and why.
- **One canonical rendering, reproduced verbatim.** The string in the program's record is the
  canonical one. Wherever this register mirrors it — the summary-table row in §4 and the structured
  entry's **Basis** field — it must be reproduced **character for character**, including punctuation
  and the exact list of items it enumerates. The structured entry's rendering **is** machine-verified
  against the record (§1.1, §1.3), so a paraphrase there fails the run rather than surviving as a
  second account of the same authority. A paraphrase in the §4 summary row is still a defect no run
  will catch for you, because that row is prose the audit does not compare.

In practice the only two files on this branch that qualify as a basis are
[`docs/technical-specifications.md`](../../docs/technical-specifications.md) and
[`docs/project-guide.md`](../../docs/project-guide.md); §7 lists the specific sections of each that
are legitimately citable.

#### 2.4.1 What a basis establishes, and what it deliberately leaves to a reviewer

A basis is checked for one property: that it can be **followed**. The document exists inside this
repository, it is readable, and the section named resolves inside it. Whether that section *supports*
the marker is a judgement, and the format leaves it to the person reviewing the marker rather than
pretending to decide it.

**This is not the check that decides whether the marker may exist.** That question is answered first, by
§2.1.1's two tiers, and it is answered in the harness rather than in any file a marker's author can edit.
A marker reaching the basis check has already been admitted either as a frozen exception whose class and
document the harness owns, or on captured evidence that includes a quotation resolved inside the very
section this check locates. So the latitude described below is latitude about *strength of citation*
within an already-authorised marker — never latitude about whether an unauthorised one gets in.

That division is deliberate, and the tempting alternative is wrong in an instructive way. Refusing any
basis whose wording rests on what a document does *not* say — "omits", "absent from", "does not list" and
their kin — reasons that an omission from an inventory is evidence only that nobody wrote something down.
As an argument about **strength** that is correct, and §4.1 states it plainly about the very marker it
applies to. As an **admission rule** it fails twice over: it refuses a marker the project specification
mandates, whose basis can be exactly such an omission; and by refusing it, the exclusion that marker
documents ends up reported `XFAIL` with no identifier, no register entry and no resolved citation behind
it, which is less auditable than the marker that was rejected. A format that cannot express its own
specification's markers is the thing that would need changing. Admission is not endorsement, and the two
must not be confused: §4.1 records in as many words that its basis is the weakest of the three analysed
here and that no `bcc` verdict has been captured against it, so a reviewer weighing that citation is given
the material to weigh rather than a verdict to accept.

So the rules that remain *for the basis* are the mechanical ones, and they are the ones a machine can
actually decide: a repository-relative path, a document that reads, and at least one locator that
resolves. They are not the whole of what a marker must satisfy — §2.1.1 is — and reading them as though
they were would describe a weaker contract than the suite actually enforces. What a
reader does with the section they are sent to is their business, and every marker's entry in §4 states
in prose exactly how strong its own basis is, so nobody has to infer it.

**The `documented` key is how an author does better than a citation — and for an observed-tier marker it
is mandatory, not better.** When it is written it must be a quotation, not a description of one; long enough to identify a passage rather than a word;
and it must occur **inside the region the locator resolved to** — a line and its immediate neighbours,
a line range as written, or a section from its heading to the next heading. Bounding the search is the
point of it: searching the whole file let a marker cite one section and quote a sentence from an
unrelated part of the same document, so the check confirmed the words were the document's own while
establishing nothing about the section a reader was sent to. Comparison collapses runs of whitespace,
so re-wrapping is free while paraphrasing is not.

#### 2.4.2 When `evidence` is written it must be captured, and `observed` is always past tense

`expected_divergence.evidence` is optional. When it is written, it names five fields, each as
`name: value` on its own line inside the heredoc, and **all five must be present**:

| Field | Holds | Why a partial block is refused |
|---|---|---|
| `command` | The exact command line that produced the divergence. | Without it the observation cannot be re-run, so it cannot be contradicted. |
| `exit` | The status that command produced. | The status is half of what every oracle in this suite compares. |
| `output` | What it printed, or the relevant part of it. | A refusal's diagnostic and a wrong answer's stdout are the substance of the divergence. |
| `toolchain` | Which compiler and which reference toolchain produced it. | A divergence attributable to a toolchain change is not a divergence attributable to the compiler; the fingerprint is what tells the two apart later. |
| `captured` | When it was captured. | An observation with no date cannot be aged out, and evidence predating the current toolchain should be re-taken rather than trusted. |

A block naming three of the five is worse than no block: it looks like a captured observation and is
not one. So the key may be omitted, and may not be written partially.

`expected_divergence.observed` is **required**, and must describe the divergence **as seen**.
Anticipatory wording — "is expected to", "will reject", "anticipated", "no verdict has been recorded"
— is refused, because prose in the future tense excuses a cell on nobody's observation. Recording what
the divergence *is* rather than what somebody expects is what lets the next reader tell whether the
failure in front of them is the one that was marked.

**The field has two legitimate shapes, one per marker class.** For a marker classed with one of the six
observational classes, it describes the divergence a comparison produced. For a marker classed
`comparison_excluded` (§2.2.1) there **is** no comparison, so describing one would be the very defect
this key exists to prevent; what it records instead is the measurement of the construct that the
exclusion rests on, and it says in as many words that no comparison is attempted on the scoped arm.
Both shapes state what was observed; they differ in what the observation is *of*.

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

**The other window it exists for is a run whose subject is not the compiler under test.** A checkout
carrying this suite ahead of the compiler tree has no `bcc` binary and drives the Cargo-dependent
gates against a documented stand-in that forwards each `--target` to the matching pinned reference
driver (§8.2). On such a run oracle (a) compares that driver with itself, so it agrees by
construction, and a `compile_failure` marker scoped `oracle_a` reports `XPASS` on every cell it
covers. That agreement is a statement about the stand-in and about the harness mechanics, and about
nothing else: a subject which *is* the reference compiler can neither exhibit nor refute a
`bcc`-versus-reference divergence, so it cannot license a retirement. Set the hatch for those runs and
judge the marker on what a real `bcc` does.

### 3.2 Retirement procedure

**The precondition, before either edit: the `XPASS` must have come from the real compiler under test.**
Retirement is a claim that the divergence is gone, and only an **independent** observation supports it.
An agreement produced by a stand-in that forwards to the reference toolchain is one toolchain compared
with itself: it establishes something about the environment and nothing about the compiler, so it
cannot retire a marker in either direction. Where the run that produced the `XPASS` was of that shape,
the correct response is `BCC_CONFORMANCE_ALLOW_XPASS` for the duration of that environment (§3.1) and a
note in the summary — not a retirement. §4.1 is the worked example, and §8.3 records what happened the
times this was got wrong.

Retiring a marker is then exactly two edits, and **doing only one of them fails the run** — the forward
check catches step 2 without step 1, and the reverse check catches step 1 without step 2:

1. **Delete every `expected_divergence.*` key** — the five required ones together with either or both
   of the optional ones the record carries — from the program's `.expected` record. Leave
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

### 3.3 Markers that could never be consulted, dormancy by construction, and narrowed coverage

A marker whose scope named **a cell that is never built** would be dormant by accident: nothing
could ever consult it, so it could never reach `XFAIL`, and — because no comparison is performed —
it could never reach `XPASS` either. It would sit here looking healthy while documenting an
experiment nobody runs, and if the divergence it describes ever appeared in a cell its scope
excluded, the run would fail as unexplained with the explanation sitting unread in the same file.

The record format refuses that state outright rather than tolerating it. A marker is rejected at
parse time when its scope names **only** targets the record does not build, or **only**
optimization levels the record does not sweep. Both are dimensions of the matrix: a cell of that
description does not exist, so nothing about that marker can ever be reported.

**The oracle dimension is different, and the difference is load-bearing.** Naming an oracle the
record has switched off is *not* a dormant scope, because every oracle reaches classification for
every cell of the matrix, in one of two ways: enabled, in which case the comparison is performed
and a marker reclassifies a divergence in it; or disabled, in which case the comparison is a
**recorded exclusion** that is still enumerated, still counted and still reported — and resolved
against the marker itself. A marker scoped to a disabled oracle is therefore consulted on exactly
the cells the record declines to compare:

> **A marker is the mechanism by which a divergence is *explained*.** It explains a divergence a
> comparison observed, or — where the record narrows that comparison away for a recorded reason —
> it explains **why the comparison is not made**, by name and on a documented basis. What a marker
> never does is *skip* anything: the program is compiled and run in full either way.

Such a marker is **dormant by construction**, which is a different thing from dormant by accident
and is the whole reason it is permitted. Because nothing is compared on that arm, it can never
reach `XPASS`, so it cannot blind the suite to a regression in a comparison that is not performed,
and §3.1's unexpected-success policy has nothing to bite on. What it buys is that the exclusion
carries an **identifier a reviewer can look up** and a **basis the audit resolves**, in the one
place this suite audits, instead of being a reason buried in one file. §4.2 is the corpus's one
instance, and its subject — the widest floating type, whose representation was measured to differ
across the four targets — is precisely the case where an exclusion most needs to be auditable.

**A narrowing therefore requires BOTH a marker and a recorded reason, and the format refuses either
one alone.** A record may narrow its own coverage — switch an oracle off, restrict its target list,
or deviate from the default warning gate — and:

- **no narrowing is accepted without a reason in `impl_defined_notes`**, because a narrowing nobody
  justified cannot be reviewed; and
- **no oracle may be switched off without a marker whose scope names it**, because the cells are
  reported `XFAIL`, which already claims the authority of an expected divergence, and requirement 5
  requires an expected divergence to carry a marker referencing the documented limitation; and
- **such a marker must be classed `comparison_excluded`** (§2.2.1), because the arm it scopes is not
  compared, so naming one of the six observational classes there would assert an observation nobody
  made. The rule holds in the other direction too: that class is refused on an arm the record
  compares, where a divergence has a shape and the marker must say which shape it excuses.

The second and third rules were each added after the rule before it proved insufficient in a specific
way. A revision of this
suite had the reason but no marker, and reported those cells `XFAIL` citing the prose. The verdict was
therefore indistinguishable from a marked expected divergence, while §1.1's bidirectional audit could
not see the exclusion at all, no identifier existed for a report row to cite, and no basis had been
resolved against any committed document. That is a silent exclusion wearing the clothes of a documented
one — which is the exact outcome requirement 5 forbids. The two halves now do different work: the
marker supplies the identifier, the class, the scope and the resolved basis; the reason supplies the
prose that explains why the comparison would be meaningless.

The third rule closes what the second leaves open. A required marker makes the narrowing auditable —
but if the `class` field had to be filled from the six observational values, the one that fits a
suppressed *value* comparison would be `stdout_mismatch`, and the marker would then document the arm the
record switches off as though two completed runs had disagreed on bytes. No check could catch that,
because a class is matched only against an observation and that arm makes none. `comparison_excluded` is
the value that states what is actually true, and the parser requires it exactly where an observation is
impossible.

A narrowed cell is still classified and still reported; it is never a silent skip:

| Situation | Verdict | Reported as |
|---|---|---|
| Not attempted because the record narrows coverage, and a marker covers the scope | `XFAIL` | The marker, its documented basis, and the record's recorded reason |
| Not attempted because the record narrows coverage, with **no** covering marker | `FAIL` | A corpus defect: the record is refused at parse time, and were it to reach classification the verdict would be `FAIL` rather than an unearned `XFAIL` |
| Not attempted while the record **enables** that oracle | `FAIL` | An unexplained removal of a comparison the corpus asks for |
| Blocked by a **marked root refusal** on another arm of the same cell | `XFAIL` | The root marker, the arm carrying it, and an explicit statement that no comparison was attempted on this arm (§2.3) |

A narrowing is deliberately **not** `UNAVAILABLE`, and the distinction is the whole point of that
verdict: `UNAVAILABLE` is a statement about the *machine* — a tool nobody installed — which is why
`BCC_CONFORMANCE_STRICT` escalates it in continuous integration, where the toolchain is installed
on purpose. A narrowing is a statement about the *corpus*: a deliberate authoring decision. Nor is
it a `PASS`: nothing was compared, so no equality is claimed. The cell is counted and listed with
its recorded reason printed, which keeps the set of comparisons deliberately **not** made as
visible as the set that was.

**Keeping four things mutually consistent is therefore mandatory**, and no run can check the prose
for you: a program's `oracle_a`/`oracle_b`/`oracle_c` toggles, the reason in its
`impl_defined_notes`, its marker block, and its entry in this register must tell the same story.
Switching an oracle **off** for a program whose marker documents an observed divergence in it turns
that marker's account into fiction: either the marker is retired in the same edit (§3.2) or its
`observed` field and this register's entry are rewritten to describe the exclusion instead, as §4.2
does. Switching an oracle **on** for a program whose marker documents the exclusion of it means
deleting the marker, the register entry and the reason together, in one edit. What is never permitted
is any half alone.

## 4. Divergences analysed: two active markers, and one candidate deliberately left unmarked

**Two markers are active on this branch**, and both are the ones the project specification identifies
in advance — by identifier, class, scope and basis, for a named program each. Every candidate analysed
here has its programme committed, compiled and run on all four targets at all three optimization
levels; what a marker changes is how a divergence would be *classified*, never whether the feature is
*exercised*.

| § | Candidate | Marker | Program | Verdict if it diverges |
|---|---|---|---|---|
| 4.1 | GCC case ranges | `XD-GCCEXT-CASE-RANGES-001` | `08_gcc_extensions/004_case_ranges.c` | `XFAIL` on oracle (a) for a `compile_failure`, with oracles (b) and (c) reported as dependent blocked arms of the same root refusal (§2.3); `FINDING` for a wrong answer, which the class does not cover |
| 4.2 | `long double` across the backends | `XD-TYPE-LONGDOUBLE-001` — class `comparison_excluded` (§2.2.1) | `13_floating_point/004_long_double_target_restricted.c` | `XFAIL` on oracle (b), which the record disables and the marker documents; `FINDING` on oracle (a) or (c), because nothing about representation excuses a same-target disagreement |
| 4.3 | Wide and Unicode literal prefixes | *(none)* | `11_literals_and_strings/003_wide_and_unicode_literals.c` | `FINDING` |

**How strong each basis is, stated per entry rather than assumed.** §2.4.1 is explicit that the audit
guarantees a citation can be *followed*, not that the cited section *supports* the claim. So each
entry below says in prose how much its own basis carries — including §4.3's, which carries none,
because what a candidate *would* have to cite is what a future author needs. §4.1's rests on the
compliance rule that fixes the **required** GCC extension set, so it establishes the boundary of the
documented obligation without establishing that `bcc` refuses the construct; §4.2's rests on an
affirmative design statement plus direct measurement, and documents a comparison the record declines to
make rather than one it performed. A reviewer should weigh them differently, and can, because every
citation named here resolves to a line they can open.

**What the audit establishes about these two entries, and what it leaves to a reviewer, stated here so
the distinction is not lost between sections.** On every run the audit proves that each marker is
authorised by one of the two tiers in §2.1.1 — and for both entries below that tier is **frozen**, so
their identifier, class and basis document are checked against an allowlist compiled into the harness
rather than against anything either of these two files says. It then proves that each marker is
registered in both directions, that each entry's eight fields agree with the record character for
character, that each cited document is contained in this repository and readable, that each locator
resolves inside it, and that each supplied quotation occurs inside the region the locator named. It
proves nothing about whether the cited passage *supports* the exclusion or reclassification it is
attached to; §2.4.1 says so, and it applies to both entries below. Neither entry should be read as an
adjudicated finding of sufficiency, and neither is presented as one.

### 4.1 XD-GCCEXT-CASE-RANGES-001 — GCC case ranges

The construct is committed, compiled and run on all four targets at all three optimization levels with
all three oracles `enabled`, and it carries the marker below. Nothing about the program is skipped: a
marker changes how a divergence would be *classified*, never whether the feature is *exercised*.

| Field | Value |
|---|---|
| Identifier | XD-GCCEXT-CASE-RANGES-001 |
| Class | compile_failure |
| Scope | oracle_a; all targets; all opt levels |
| Program | 08_gcc_extensions/004_case_ranges |
| Basis | docs/technical-specifications.md, line 761, the C11 + GCC Extensions Compliance Rule states which GCC extensions are explicitly required and case ranges are not among them |
| Documented | GCC extensions explicitly required: `__attribute__`, `__builtin_*` intrinsics, inline assembly (`asm`/`__asm__` with operand constraints), statement expressions, `typeof`/`__typeof__`, computed goto, `__extension__` |
| Evidence | (not written) |
| Observed | THIS IS A FROZEN SPECIFICATION EXCEPTION, NOT AN OBSERVED DIVERGENCE, and that distinction is now enforced by the harness rather than left to this prose. The marker's identifier, its class and the document its basis cites are fixed in an immutable allowlist compiled into the conformance harness; this record may only instantiate that definition and is refused outright if it restates any part of it, so the authority for carrying this marker does not live in this file and cannot be edited here. Every marker the allowlist does NOT name is admitted on captured evidence alone -- a quotation resolved inside the very section its basis cites, plus a structured, re-runnable observation of command, exit status, output, toolchain and capture date -- and the verdict detail of each arm a marker excuses states which of those two tiers admitted it. Nothing about how this field is worded is what makes the marker acceptable, which is the point: the previous contract could be satisfied by careful phrasing, and this one cannot. WHAT THIS MARKER COVERS, AND WHAT STANDS BEHIND EACH HALF OF IT. It covers exactly one divergence: a refusal by the compiler under test to translate the case-range label form, on oracle (a) alone. A refusal produces no artifact, so oracle (a) has nothing to compare, while the reference compiler accepts the same source and prints the eighteen-line golden record this file carries for it, beginning cr_bucket_neg=10 and ending cr_runtime_hex=12. The reference half is measured on this branch: gcc 13.4.0 accepts this program and prints those bytes on all four targets at all three optimization levels. The compiler-under-test half carries no independent capture, and expected_divergence.evidence is left unwritten rather than filled in from something weaker -- this checkout holds the corpus and no bcc, and the only compiler under test it can offer forwards every invocation to the pinned reference driver for the requested target, so that compiler's agreement is a measurement of the environment, one toolchain compared with itself, and it speaks to neither bcc nor case ranges. A run of that shape has been performed here and is recorded for exactly what it is: non-independent environment evidence, which settles nothing in either direction. So no such refusal has been captured on this branch, and this field describes the divergence the marker is scoped to explain rather than one a capture attests. The marker is present because the frozen project specification fixes it for this program by identifier, class, scope and basis, and its basis is the affirmative compliance rule cited above, which fixes the required GCC extension set and does not place case ranges inside it. The register's section 3.1 XPASS safeguard is what keeps carrying it honest: where the compiler under test accepts the construct, the arm this marker scopes agrees, the verdict is XPASS and the run FAILS, naming the marker and both places it lives. Only a real, independent bcc observation retires it. |

| Aspect | Value |
|---|---|
| **Program** | `tests/conformance/08_gcc_extensions/004_case_ranges.c` — committed |
| **Record** | `tests/conformance/08_gcc_extensions/004_case_ranges.expected` — committed, carrying this analysis in its `impl_defined_notes` and the marker block above |
| **Oracles** | (a), (b) and (c) all `enabled`; all four targets and all three optimization levels declared — twelve `bcc` cells, on the same footing as every other program |
| **Verdict if `bcc` rejects the construct** | `XFAIL` on oracle (a) citing this marker; oracles (b) and (c) reported as dependent blocked arms of the same root event |
| **Verdict if `bcc` accepts it and agrees** | `XPASS` on oracle (a) — the run **fails**, and if the subject was a real `bcc` the marker is retired by §3.2. Oracles (b) and (c) `PASS`. A **surrogate** subject's agreement is not a retirement ground; see below |
| **Verdict if `bcc` accepts it and computes a wrong answer** | `FINDING` — a `stdout_mismatch`, which this marker's class does not cover |

**What the marker covers, and what it deliberately does not.** Its class is `compile_failure` and its
scope is `oracle_a` alone, so it excuses exactly one thing: the compiler under test refusing to
translate a program the reference compiler accepts, observed on the arm where that is a statement about
the two compilers. Oracles (b) and (c) are denied their subject by the same root refusal and did not
observe a divergence of their own; they are reported as **dependent blocked** arms naming this marker
(§2.3), so the whole consequence of the refusal is visible while no arm claims a comparison it never
made and no redundant finding is filed. A wrong ANSWER from a compiler that accepts the syntax is a
`stdout_mismatch`, which this class does not cover, and is reported as the `FINDING` it is.

**What stands behind each half of the observation, because the two halves are not equally evidenced,
and this is the single most misread thing in this register.** The reference half is measured: `gcc`
13.4.0 accepts the program and prints the recorded golden on all four targets at all three levels. The
compiler-under-test half has **no independent capture**, and `expected_divergence.evidence` is left
unwritten rather than filled in from something weaker. This checkout carries the corpus and no `bcc`
(§8.2, and `README.md` §"The enumerable matrix"), and the only compiler under test it can offer
forwards every invocation to the pinned reference driver for the requested target. A run of that shape
has been performed here, and it agreed on all twelve cells — which is a measurement of the
**environment**, one toolchain compared with itself, and evidence about neither `bcc` nor case ranges.
It is recorded for exactly that and nothing more: **non-independent environment evidence, which settles
the question in neither direction.**

**Why the marker is present anyway, and why that is not the same mistake as minting one on a guess.**
The project specification is frozen and fixes this marker for this program by identifier, class, scope
and basis. Carrying it is a requirement of the specification rather than an inference this register may
withdraw — the same ground on which §2.4.1 admits an omission-based citation, since a format that
cannot express its own specification's markers is the thing that needs changing, though this marker no
longer needs that latitude now that it cites the compliance rule. What the specification
does **not** do is suspend §3.1, and §3.1 is what keeps carrying the marker honest: where the compiler
under test accepts the construct, this marker's arm agrees, the verdict is `XPASS` and **the run
fails**, naming the marker and both places it lives. So the unconfirmed half is not quietly excused; it
is a loud, run-failing state that nobody can walk past, and `BCC_CONFORMANCE_ALLOW_XPASS` is the
transition window the specification itself provides for it. That is the difference between this marker
and a speculative one: a speculative marker hides a question, and this one forces it into every run's
summary until somebody with a real `bcc` answers it.

**Retirement takes evidence, not the absence of it — recorded because this marker has been minted and
withdrawn several times and the thrash is itself the lesson (§8.3).** The observation that retires it
is a real, **independent** `bcc` accepting this program and agreeing with the reference compiler. A
stand-in forwarding to the reference toolchain is not that observation, because a toolchain compared
with itself cannot answer the question the marker asks — and an agreement of that shape is exactly what
the surrogate produced. Until such an observation exists the marker stays, and §3.1 keeps saying so on
every run.

**The construct.** GCC's case-range extension: a switch label of the form `case 0 ... 9:`, together
with its character and negative forms `case '0' ... '9':` and `case -20 ... -11:`, its degenerate
single-value form `case 5 ... 5:`, and a range group that deliberately falls through.

**What the documentation actually says, stated so a reader can check it without trusting this file.**
The cited basis is the **governing compliance rule**, and it is affirmative rather than silent. Four
further inventories enumerate the same extension set, and none of the five names case ranges:

| Where | What it AFFIRMS |
|---|---|
| `docs/technical-specifications.md` line 761 — the **basis this marker cites**, inside the section headed "C11 + GCC Extensions Compliance Rule" at line 758 | A **requirement**: "GCC extensions explicitly required: `__attribute__`, `__builtin_*` intrinsics, inline assembly (`asm`/`__asm__` with operand constraints), statement expressions, `typeof`/`__typeof__`, computed goto, `__extension__`", with line 762 adding that "These extensions are not optional; they are required for compiling real-world codebases like SQLite, Lua, and Redis". It fixes the extension set the implementation must provide, and case ranges are outside it |
| `docs/project-guide.md` §5, the Compliance and Quality Review row `C11 frontend with GCC extensions`, marked **Pass** | A **verification record**: on the evidence of 19 frontend files, 33,834 lines and 789 unit plus 189 integration tests, the constructs parsed are "`__attribute__`, statement expressions, `typeof`, computed goto, inline assembly **all parsed**". It states what was checked and found to work, and the list is closed at five named constructs |
| `docs/project-guide.md` §8, the `Production Readiness Assessment` | The implementation is "**feature-complete** for all AAP-specified capabilities", and "the remaining work is exclusively validation, testing, and packaging — no core implementation gaps exist". Nothing outside the verified surface is pending |
| `docs/technical-specifications.md` lines 13, 107 and 506 | The same seven-item extension set, in the capability summary, the module inventory and the file-creation specification for the extension parser |
| `docs/project-guide.md` §2.1, the `C11 Frontend — Parser` row | The same five-construct parsed list for the parser subsystem |

A recursive, case-insensitive search of the two authoritative implementation documents —
`docs/project-guide.md` and `docs/technical-specifications.md` — for `case range` returns **zero**
matches. Those two are the whole of the basis, and the scope of the claim is deliberately drawn around
them: `case range` does occur elsewhere under `docs/`, in this suite's own methodology page
`docs/testing/differential-conformance.md`, which describes this very marker and is therefore the
suite's own reasoning rather than independent evidence for it. A basis that counted it would be citing
itself.

**Exactly how strong that basis is, stated plainly rather than glossed.** It establishes a boundary,
not a behaviour, and the difference matters. It is said here rather than left to a reader to discover,
because §2.4.1 leaves the weighing of a basis to a reviewer and this is the reviewer's material:

- **What it affirms is the boundary of the documented obligation.** The cited rule states which GCC
  extensions are *required*, says in the next line that those extensions are not optional, and does
  not place case ranges inside the set. An implementation that declines the construct is therefore
  within its documented scope — and that is a statement the cited line makes, not a silence a reader
  has to interpret.
- **What it does not affirm is that `bcc` refuses them.** A construct outside the required set may
  still be implemented, and nothing in the repository says whether this one is. So the marker claims
  only that IF `bcc` refuses case ranges, the refusal falls outside the documented obligation and is
  traceable to it rather than being unexplained — which is the distinction requirement 5 draws, and
  the ambiguity the project specification flagged for a maintainer to resolve. It is a pointer to an
  open question, never an assertion that the question is closed.
- **The basis is a compliance rule rather than an inventory, and that is what gives it its footing.**
  An inventory row supplies only the inventory's **silence** about the construct, which is compatible
  with the feature working and the inventory being incomplete, with the feature not working, and with
  nobody having considered the question — and which cannot be *corrected away*, so a marker resting on
  it could outlive its reason. Citing the requirement replaces an inference from silence with a
  statement the document makes. Silence is what the four corroborating inventories add, and they are
  recorded above as corroboration rather than as authority.
- **The citation is followable**, at `docs/technical-specifications.md` line 761, and the record's
  optional `documented` key quotes that line's own requirement sentence, which §1.1 resolves inside the
  cited range on every run. A reader can turn to the compliance rule and weigh it in one click, which
  is what §2.4.1 promises of a citation and all it promises. Whether it supports this marker is the
  reviewer's call, and this register does not pretend to have made it.
- **No captured observation exists, and the record says so in the observation itself.**
  `expected_divergence.evidence` is not written, because no command on this branch has produced the
  refusal — the checkout carries the corpus and no `bcc`, and the surrogate that stands in for it
  forwards to the reference toolchain. The marker discloses that rather than papering over it, and
  §3.1 is what stops the disclosure from being cost-free: an unconfirmed marker fails every run in
  which the construct works. When a run does produce the refusal, adding the evidence block
  strengthens the marker without changing its identifier, class, scope or basis.

**Why the program exists at all, and why the marker does not weaken it.** Constraint C3 forbids
excluding a language feature because it may be unimplemented or awkward, and the suite's mandated
extension list names case ranges explicitly alongside statement expressions, `typeof` and computed
gotos. The program is therefore written, scheduled and run with all three oracles `enabled`, exactly
like every other program — twelve `bcc` cells, and the marker changes not one of them. What the marker
changes is only how a refusal on oracle (a) is *classified*; a wrong answer is a `stdout_mismatch` the
marker does not cover and is delivered as a **finding** — a **verbatim reproducer** with its recorded
minimization status, the captured output of each compiler and each backend, the exact reproduction
commands and an environment fingerprint. Nothing is excluded, and the only thing excused is the one
refusal the basis speaks for, on the one arm where it is a statement about two compilers.

**Note on the warning gate.** The record deviates from the default undefined-behaviour audit gate by
dropping `-pedantic`, with the reason recorded in its own `impl_defined_notes` — the one field the
suite reads a gate reason from: the subject under test is a GNU extension, which is by definition not
standard C, and `-pedantic` exists precisely to reject such constructs, so with it in force the
program could not be compiled at all and the feature would go untested, which C3 forbids. Every other
member of the gate is retained, `-Werror` included, so no other class of defect in this program is
downgraded. A gate deviation is not a divergence: the gate is a property of the **test material**, is
driven by the reference compiler only, and renders no verdict about `bcc`.

**What would make this basis stronger still, recorded so the next reviewer need not rederive it.** One
step remains available, and it is a different kind of statement from the one cited: a sentence naming
the construct **directly** — an entry in the out-of-scope table of `docs/technical-specifications.md`
§0.6.2, or a line saying case ranges are not implemented. That would assert the limitation itself
rather than the boundary of the obligation, and it is the only reading that would let the marker claim
`bcc` refuses the construct. No such sentence exists in the repository today; a recursive, case-insensitive search of
`docs/project-guide.md` and `docs/technical-specifications.md` for `case range` returns zero
matches, and the only mentions anywhere under `docs/` are in this suite's own methodology page.

A second, weaker widening is also available. Two project-guide statements read together — §5’s
Compliance and Quality Review row, which records a passing verdict over an enumerated set of parsed
constructs, and §8’s Production Readiness Assessment, which states the implementation is
feature-complete with remaining work *"exclusively validation, testing, and packaging"* and *"no core
implementation gaps exist"* — support the inference that a construct absent from the verified surface
is documented as absent from the *compiler* rather than merely unmentioned by the *documentation*. A
maintainer who accepts that inference may widen the basis to cite those rows beside the compliance
rule. Both widenings are recorded as available rather than as in force, because a marker's basis should
be the narrowest claim that does the job, and widening it is a decision for whoever has the evidence.

**The ambiguity a real `bcc` will settle, deliberately not resolved here.** It is not yet known whether
the silence is an **implementation gap** (the frontend does not accept case ranges, and the
documentation correctly reflects that) or a **documentation gap** (the frontend accepts them and the
inventories are merely incomplete). No run on this branch has narrowed it, and the reason is worth
stating once more because it is the thing most easily misread: the only compiler under test available
here is the **surrogate** of §8.2, which forwards to the reference driver, so oracle (a) has been
comparing the reference compiler with itself and its agreement carries no information about `bcc`
either way. The two readings lead to opposite actions:

- **What has been observed, and it is less than it looks.** The compiler under test accepted the
  program and oracle (a) agreed with the reference compiler byte for byte on all twelve cells. But the
  compiler under test was a **surrogate** forwarding every invocation to the reference toolchain
  (§8.2), so what that agreement measures is one toolchain compared with itself. It is evidence about
  the **environment**, not about `bcc`, and it therefore settles neither reading. It is recorded here
  because a reader who found the same twelve agreements without this paragraph would draw a stronger
  conclusion from them than they support.
- **What settles the implementation-gap reading.** The first run in which a real `bcc` refuses this
  program reports `XFAIL` on oracle (a) against this marker, with oracles (b) and (c) reported as
  dependent blocked arms of the same root event. **No compiler change is made in response** — findings
  and expected divergences are both reported, never patched. That captured refusal is also the missing
  half of this marker's evidence: adding `expected_divergence.evidence` then strengthens it without
  changing its identifier, class, scope or basis, and a maintainer may additionally *state* the
  limitation in prose so the basis cites an assertion rather than a silence.
- **What settles the documentation-gap reading.** A real `bcc` that accepts the program and agrees
  reports `XPASS` on oracle (a) — and **the run fails**, naming this marker and the two places it
  lives, which is §3.1 doing exactly what it exists for. That is the authorized moment to retire the
  marker by §3.2, and the right follow-up is to extend the documented inventory so the next reader is
  not left drawing inferences from a silence.

Either way a run with a real `bcc` tells a maintainer which reading is right without anyone having to
guess in advance, and this subsection is where the answer will be recorded. Carrying the marker is what
keeps that true rather than optional: with the marker in place, **both** answers fail or flag a run
until somebody writes the answer down, whereas an absent marker lets the accepting case pass silently
and leaves the question open indefinitely.

### 4.2 XD-TYPE-LONGDOUBLE-001 — `long double` across the backends

The program this subsection governs — `tests/conformance/13_floating_point/004_long_double_target_restricted.c`
— is **committed**, together with its record, and both are exercised on every run. The marker documents
the one comparison the record declines to make, and the measurement below is the reason recorded for it.

**What this entry asserts, and what it does not, stated before the tables rather than after them.** The
marker, its `oracle_b` scope, its class, its basis and the measured reason in the record's own
`impl_defined_notes` are all **recorded, registered and machine-checked** — the audit resolves the
identifier in both directions, compares all eight fields against the record character for character,
reads the cited document, and resolves the locator and the quotation inside it. That is the whole of
what any automated check establishes here. Whether the cited passage **semantically supports switching
an oracle off for this type** remains a reviewer's judgement (§2.4.1) rather than something a run can
settle, and this entry does not present itself as an adjudicated finding of sufficiency: it records an
exclusion *with a followable basis and a re-measured reason*, and a reader who needs the stronger claim
should read the program and its record and form their own.

**The review of that program and record has completed, and this section is published on the far side of
it** — §8.2's substantiation column and the driver's empty `PENDING_RECORDS` declaration say the same
thing, and all three are meant to be read together. Three facts the review settled bear directly on this
entry, and each is stated here as it now stands:

- **the type's object representation is not this exclusion's ground.** Character-type access to it is
  permitted (C11 6.2.6.1p4 and 6.5p7); what makes a byte image an unusable oracle is that the padding
  bytes are unspecified (6.2.6.1p6) and that both the padding's extent and the significant bytes'
  encoding are target-dependent. The program inspects no representation either way;
- **the exclusion rests on the significand widths, 64 bits against 113, and on the rounding that
  follows from them — not on exponent range.** The normal exponent range and the finite maximum measure
  identical on all four targets, at -16381 to 16384 and 1.189731e+4932, and the record states that
  non-difference explicitly. The measured reason below is that measurement;
- **the class is `comparison_excluded` (§2.2.1), which is the only honest class on an arm the record
  switches off.** The parser requires that class exactly where no comparison is attempted, and refuses
  it wherever the arm is compared.

The exclusion itself is unaffected by any of that: one oracle, the same nine cells, the same basis.

| Field | Value |
|---|---|
| Identifier | XD-TYPE-LONGDOUBLE-001 |
| Class | comparison_excluded |
| Scope | oracle_b; all targets; all opt levels |
| Program | 13_floating_point/004_long_double_target_restricted |
| Basis | docs/technical-specifications.md, line 511, the type representation is specified with target-parametric sizes covering the floating types including long double |
| Documented | Type representation with target-parametric sizes |
| Evidence | (not written) |
| Observed | NO CROSS-BACKEND VALUE COMPARISON IS ATTEMPTED FOR THIS PROGRAM: the record above disables oracle (b), and this marker's class is comparison_excluded precisely so that this field describes no comparison result. What was observed is a measurement of the TYPE, made with the four pinned reference drivers: sizeof(long double) is 16 on x86_64, 12 on i686, 16 on aarch64 and 16 on riscv64, with the two x86 targets carrying the x87 80-bit extended format inside that storage and the other two carrying IEEE binary128 -- three storage-and-format pairings over two distinct formats, whose significands are 64 bits against 113 and which therefore round at different places. Measured beside it: the twelve cells this program does run print byte-identical stdout, because every value printed was chosen to be exact in all three representations, so the exclusion is about what a cross-backend value comparison over this type would MEAN rather than about any difference this program exhibits. A value difference between two backends over this type is an implementation-defined difference of the kind the brief's carve-out names, not a defect in either. |

| Aspect | Value |
|---|---|
| **Program** | `tests/conformance/13_floating_point/004_long_double_target_restricted.c` |
| **Record** | `tests/conformance/13_floating_point/004_long_double_target_restricted.expected` — carries `oracle_b = disabled`, the measured reason in `impl_defined_notes`, and the marker block above |
| **Oracle (a)** | **enabled** — the same-target reference comparison, on all four targets, at all three optimization levels |
| **Oracle (c)** | **enabled** — the golden record, on every cell |
| **Oracle (b)** | **disabled in the record**, with the measured reason in `impl_defined_notes` and this marker scoping it; the nine not-attempted cells are reported `XFAIL` citing the marker and its basis (§3.3) |
| **Verdict if oracle (a) or (c) diverges** | `FINDING` — nothing about representation excuses a same-target disagreement |
| **Verdict on oracle (b)** | `XFAIL`, always. Nothing is compared there, so the marker can never reach `XPASS` — dormancy by construction (§3.3) |

**The difference.** `long double` does not have one representation across the four supported targets.
Measured directly in this environment:

| Target | `sizeof(long double)` | Representation |
|---|---|---|
| x86-64 | 16 bytes | x87 80-bit extended, padded |
| i686 | 12 bytes | x87 80-bit extended, padded |
| AArch64 | 16 bytes | IEEE binary128 |
| RISC-V 64 | 16 bytes | IEEE binary128 |

Three storage-and-format pairings over two distinct formats — two distinct sizes and two distinct
encodings — with a 64-bit significand on the two x86 targets against a 113-bit significand on the
other two. Cross-backend **value** equality is therefore not merely hard to
achieve for this type — it is meaningless. Two backends printing different digits for the same
`long double` computation are both correct for their own target, so a cross-backend difference here
is *not* evidence of a defect in either. That is precisely the implementation-defined carve-out the
suite's brief reserves: a cross-backend divergence attributable to a documented implementation-defined
difference in type width or representation is not a compiler defect. The consequence was measured on
a value chosen to expose it: `1.0L` divided by `3.0L`, printed at 25 fractional digits, gives
`0.3333333333333333333423684` on the two x87 targets and `0.3333333333333333333333333` on the two
binary128 targets.

**The documented basis, which is why this is a marker and not a bare reason.** `docs/technical-specifications.md`
line 511 specifies the type representation as "target-parametric sizes" covering "floating types
(float, double, long double)" — the size of `long double` is, by the repository's own design, a
function of the target rather than a constant. The target table at lines 457–462 records the same
principle for the properties it enumerates: `i686-linux-gnu` is ELF32 with a 4-byte pointer and
4-byte `long`, while the other three targets are ELF64 with 8-byte pointers and 8-byte `long`. §7
lists both as citable for exactly this statement, and §1.1 resolves both locators inside that
document on every run, so the citation is one a reader can follow rather than one taken on trust.

**Handling — a recorded exclusion, and the worked example of constraint C3.** The type is **not
dropped**. The program is written and run on **all four targets at all three optimization levels**,
and:

- **Oracle (a) is then fully in force.** Every target's output is compared byte-for-byte against the
  *same target's* reference compiler, which is the comparison that can actually detect a defect here
  — both sides then use the same representation, so any difference is a real disagreement, and a
  divergence is a `FINDING`.
- **Oracle (c) is then fully in force.** Every cell is asserted against the golden record, so a
  regression that moved `bcc` and the reference compiler together would still be caught.
- **Oracle (b) is then switched off in the record**, with the measured reason above written into the
  program's own `impl_defined_notes`. The record format refuses a narrowing without a recorded
  reason, so the exclusion cannot be silent; the not-attempted cells are still counted and still
  listed, reported `XFAIL` against this marker and the basis it cites (§3.3).

This is exactly what C3's *"state so explicitly and explain why"* asks for, rather than dropping the
type: the exclusion is **narrow** (one oracle, one program), **explicit** (a disabled toggle that the
format will not accept unexplained), **explained** (a measurement plus a documented basis) and
**named** (this identifier, resolvable in one place).

**Why the marker and the disabled toggle belong together, recorded because the alternative was tried.**
A revision of this suite carried the toggle and the reason but **no marker**, on the reasoning that a
marker asserts an observed divergence while nothing is observed here. The reasoning describes a real
distinction and drew the wrong conclusion from it. The cells were still reported `XFAIL` — so the
verdict claimed the authority of an expected divergence — while §1.1's bidirectional audit could not see
the exclusion, no identifier existed for a report row to cite, and no basis had been resolved against
any document. Prose in one file is not an audit trail.

What the marker asserts here is stated precisely so it is not read as more than it is. Not that the
backends disagree today: they do not, and all twelve cells print identical bytes because every printed
value was chosen to be exact in all three representations. What it asserts is that a cross-backend
**value** comparison over this type could not be read as evidence about `bcc` even if they did
disagree — a statement about what the comparison would MEAN, which is why it stays correct if a later
maintainer adds a value that does diverge. Its `observed` field describes the exclusion and the
measurement behind it, in the past tense, which is exactly what it is. And because nothing is compared
on the arm it scopes, it can never reach `XPASS`: it is **dormant by construction** (§3.3), which is
what makes it safe as well as auditable.

`expected_divergence.evidence` is not written, and the register renders that row as *(not written)*.
There is no command that could produce a captured observation of a comparison the record declines to
make, and inventing one would be worse than omitting it.

**As observed on a full run.** The nine cells this exclusion covers — three non-baseline targets ×
three optimization levels, oracle (b) only — are reported `XFAIL` citing this marker and its basis, and
the run summary additionally carries an `exclusion` record quoting the record's own reason verbatim. Oracle (a) and
oracle (c) contribute their full complement of comparisons for this program on all four targets, so the
type is measured, not merely mentioned.

**If a maintainer ever re-enables oracle (b) for this program** — for example to compare a
representation-independent property instead of a value — then this marker no longer describes the
cells that are now attempted, and it must be retired or rescoped **in the same edit**, here and in
the record together (§3.2, §8.5). A cross-backend value divergence would otherwise be reported
against a marker whose `observed` field describes an exclusion that no longer exists.

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
- Recursive, case-insensitive searches of the two authoritative implementation documents —
  `docs/project-guide.md` and `docs/technical-specifications.md` — for `unicode`, `char16`, `char32`
  and `wchar` return **zero** matches. As with `XD-GCCEXT-CASE-RANGES-001` above, the claim is scoped to
  those two on purpose: these terms do appear elsewhere under `docs/`, in this suite's own methodology
  page `docs/testing/differential-conformance.md`, which discusses this very question and so cannot be
  evidence for its own conclusion.

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

**A marker on a comparison that is still made implies a test that actually diverges.** Where a
measured implementation-defined difference is *designed around* so that no divergence occurs, and
the oracle that would observe it **stays enabled**, a marker would be simply wrong: it would claim a
divergence the corpus has already eliminated, the comparison would agree, and the verdict would be
`XPASS`. The correct record for a designed-around difference is an `impl_defined_notes` entry in each
affected program — which the record format requires anyway — and an entry in this section so the
decision is visible in one place.

The enabled-oracle condition is what separates this section from §4.2, and the line between them is
worth stating precisely because both concern a measured per-target difference. The three properties
below are designed away while **every oracle keeps judging every cell**, so there is nothing left for
a marker to document and an `XPASS` is exactly what one would produce. §4.2's subject is designed
away in what it *prints*, but the comparison itself — cross-backend **value** equality for the widest
floating type — is one the corpus declines to make on principle, because a difference there would not
be a defect; that declining is recorded, is classified, and is what its marker names. The test is
therefore: if the oracle stays enabled, this section; if the record narrows it away for a recorded
reason, a marker may name the exclusion (§3.3).

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
this repository fixes them for no supported target: recursive, case-insensitive searches of the two
authoritative implementation documents — `docs/project-guide.md` and
`docs/technical-specifications.md` — for `wchar`, `char16`, `char32` and `unicode` return **zero**
matches in either file. Scoped to those two exactly as §4.3 scopes its own searches, and for the same
reason: two of those four terms do occur elsewhere under `docs/`, in this suite's own methodology page
`docs/testing/differential-conformance.md`, which discusses this very question and so cannot be
evidence for its own conclusion. No per-target table is given
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

*Measured, per target, with the suite's toolchain of record:*

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
and rejected on measurement: the alternate reference compiler maps every universal-character-name in
the program to its correct code point **while defining no such macro at all**. Gating on the advertisement rather than
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
a divergence there is **unexpected and must be investigated** rather than assumed to be a permitted
implementation-defined difference. Recording them here is what stops a future maintainer from
"fixing" a real defect by adding a marker for a difference that was never expected in the first
place.

**"Unexpected" is not the same as "a defect in `bcc`", and the two provenances below decide which.**
For a row the standard fixes, a disagreement is a conformance defect and therefore a finding
outright. For a row that is genuinely implementation-defined — bitfield layout is the clearest case
— the measured agreement is agreement between four independently specified ABIs, not a property
inherited from the language, so a cross-backend disagreement is investigated in order rather than
concluded: compare the diverging target against **its own** reference compiler first, then read that
target's psABI for the rule it actually specifies, and only a `bcc` that disagrees with one of those
two authorities is a finding against `bcc`. The paragraphs after the table set out that division in
full, and it is the operative reading of this subsection.

| Property | Measured result on all four targets |
|---|---|
| Bitfield layout, size, alignment and exact byte image | Identical, including a straddling 3-bit / 5-bit / 9-bit sequence and the byte image it produces |
| Right shift of a negative signed value | Arithmetic (sign-propagating) everywhere |
| Integer division and remainder signs | Division truncates toward zero; the remainder takes the sign of the dividend — **fixed by C11 6.5.5p6**, so the survey confirms conformance rather than establishing agreement |
| Byte order | Little-endian everywhere, as the target table records |
| Escape sequences and hexadecimal / octal formatting | Byte-identical output |
| Character-literal values | Identical |
| Variadic argument passing, integer **and** `double` | Byte-identical at all three optimization levels |
| `sizeof(enum E) == sizeof(int)` for an enumeration whose values fit in `int` | True on all four, `sizeof` four bytes, at all three optimization levels — twelve configurations measured, all agreeing |

**Two provenances, deliberately distinguished.** One row in the table above is not an
implementation-defined property at all, and treating it as one would misdescribe what a future
disagreement means. **Integer division and remainder** are fixed by the standard: C11 6.5.5p6 makes
`/` yield the algebraic quotient with any fractional part discarded — truncation toward zero — and
requires `(a / b) * b + a % b` to equal `a` wherever the quotient is representable, from which the
remainder's taking the sign of the dividend follows. Agreement across the four targets was therefore
never in question, and the measurement establishes nothing about the language; it confirms only that
no driver in this environment departs from what C11 already requires. Every other row is genuinely
implementation-defined or unspecified — right shift of a negative signed value is
implementation-defined under C11 6.5.7p5, byte order, bitfield layout and the argument-passing
details are not fixed by the standard at all — and there the measurement is the only thing that
establishes agreement, which is exactly why it was taken.

The distinction is operational, not pedantic, because it changes the correct response to a future
disagreement. On the standard-fixed row a disagreement is a **conformance defect** and therefore a
finding: it is never a candidate for a marker, and §5.4's narrow-the-target-list remedy must not be
applied to it, because narrowing would conceal a defect rather than record a permitted difference. On
an implementation-defined row a disagreement may be a conforming difference, and there §5.4's rule
governs — narrow that program's target list and mint a marker citing the relevant subsection.

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
| Shared-library / dynamic-loader validation gap | `docs/project-guide.md` §2.2 (remaining work: `Shared Library (-shared) End-to-End Validation`) and §6 (open risk: `Shared library dynamic loader incompatibility`, "Open — Requires runtime validation") | **Outside this suite's oracles.** Every test binary is built with `-static`, which is also the one linkage mode both compilers spell identically and the reason emulator execution needs no sysroot. No cell ever produces or loads a shared object, so the limitation cannot manifest here and a marker would describe a comparison the suite never makes. |
| DWARF debugger-validation gap | `docs/project-guide.md` §2.2 (remaining work: `DWARF v4 Debugger Compatibility Testing (GDB/LLDB)`) and §6 (open risk: `DWARF v4 debugger parsing failures`, "Open — Requires manual testing") | **Outside this suite's oracles.** No debugger is ever invoked and `-g` appears in no differential invocation; the suite compares stdout bytes and exit status only. Debug information is therefore never observed, so nothing here could diverge on it. |
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

This document is edited as the project progresses — inserting one table row shifts every line below
it — so its rows are cited **by section and row title** rather than by line number. Both forms
resolve (§2.4), and the phrase form survives an edit to the document it cites, which is the property
a basis needs.

| Where | Section | What it authorises a marker to say |
|---|---|---|
| §2.1 `C11 Frontend — Parser`, and §5 `C11 frontend with GCC extensions` | The parser subsystem row and the requirement conformance matrix, both enumerating the GCC extensions that are parsed | The same extension-inventory basis as above, from the guide's side |
| §2.2 `Shared Library (-shared) End-to-End Validation`, `DWARF v4 Debugger Compatibility Testing (GDB/LLDB)`, `C11 Standard Corner Case Compliance Testing` | Remaining work: shared library end-to-end validation; DWARF v4 debugger compatibility testing; and **the C11 corner-case item this whole suite exists to close** | That a validation activity is documented as outstanding |
| §6 `Shared library dynamic loader incompatibility`, `DWARF v4 debugger parsing failures`, `C11 corner case non-compliance` | The open risk register: the first "Open — Requires runtime validation"; the second "Open — Requires manual testing"; and the third — *"edge cases in complex declarators and type conversions may remain"*, status *"Open — Requires targeted testing"* | That a class of non-compliance is documented as an open, unmitigated risk |

The last row deserves emphasis, because it is the reason this suite exists at all: the repository
already records C11 corner-case non-compliance as an open risk whose mitigation is *targeted
testing*, and names the corresponding work item in §2.2. This register is where the outcome of
that targeted testing becomes auditable.

## 8. Provenance, constraints and maintenance

### 8.1 Provenance

Every fact in this register comes from one of three places, and each is checkable:

- **The two documents in §7**, cited by file and line, and machine-verified to exist.
- **Direct measurement in the suite's own environment** — the byte sizes, signedness, layout and
  formatting results in §4.2 and §5. Each is reproducible with no harness required, and the commands
  come from the same place for every program: **all 108 sources have a record**, so a reader renders
  the `bcc_command`, `ref_command` and `run_command` templates that record carries and needs nothing
  else — see `README.md` §"Reproducing a cell by hand" for the substitution rules. Where a program's
  own source banner cites a specific measurement, as the `long double` program's does for the digits
  in §4.2, the banner carries that measurement's exact command beside the figure, so the figure and
  the way to re-derive it never drift apart.
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
| **C2 — no existing test weakened** | No existing test may be deleted, skipped, weakened or relaxed; no `#[ignore]` attribute may be added or removed. | No marker in this register changes an existing test, and the suite declares no ignored test and no harness test function of its own, so it can move neither the repository's test count nor its ignored count — which is the part that is mechanical here. The count itself, **exactly 13 ignored**, is a whole-repository property no integration test can read, so it is measured by the health gate once this suite and the compiler are on one branch — by **aggregating** the `test result:` line every test binary prints, never by grepping for one of them, because no single line carries the repository-wide figure. The suite contract records the exact command. |
| **C3 — never exclude a feature because it is difficult** | If a feature cannot be tested, say so explicitly and explain why, rather than dropping it. | **Every entry in §4 exists because of C3.** Case ranges (§4.1) are absent from every documented inventory and are tested anyway, with all three oracles enabled and all twelve cells scheduled; they carry `XD-GCCEXT-CASE-RANGES-001`, whose class covers a refusal on oracle (a) alone, so a wrong answer there is a FINDING and whether the construct is *exercised* is not what a marker decides either way. The wide and Unicode literal prefixes (§4.3) are likewise unenumerated and likewise fully tested, with **no marker**, so any divergence there is a FINDING. And `long double` (§4.2) has three different representations across four targets and is written and run rather than dropped, with cross-backend value equality excluded for the reason its own record states **and** for the marker that record is required to carry beside it — which is why its nine oracle (b) arms report `XFAIL` citing `XD-TYPE-LONGDOUBLE-001` while oracle (a) and oracle (c) judge all twelve cells in full. That narrowing is *recorded and machine-checked* rather than adjudicated here: §2.4.1 applies, so whether the cited passage supports the exclusion is a reviewer's judgement made against §4.2, not something this row asserts. Where a comparison genuinely cannot be made, the exclusion is narrowed to a **single oracle**, the program keeps running under the remaining oracles, and the reason is recorded in the program's own record — never here alone. |
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
the findings register, [`FINDINGS.md`](FINDINGS.md) — the committed companion to this file. Its
`findings/` directory holds no curated finding yet, because no divergence has been observed on this
branch. Neither produces a patch.

**Honest measurement.** No coverage figure is published in this file, and none can be: coverage
instrumentation requires a development dependency, which the rule quoted above forbids absolutely,
so any figure here would be unverifiable by anyone in this repository. Where the suite's breadth
must be referenced, it is referenced as an enumerable matrix, and it is published as **three
genuinely different dimensions** that must never be collapsed into one another:

- **Structural** — what a reader can count in `tests/conformance/` with a shell. The corpus is
  structurally complete: fourteen areas, 108 sources, 108 records, no unpaired source and no orphan
  record.
- **Substantiated at this checkpoint** — the narrower, milestone figure, which now stands at **all
  108 records**. The last one outstanding,
  `13_floating_point/004_long_double_target_restricted.expected`, has completed its review. What
  substantiation means for it, and for every other record, is that its written
  undefined-behaviour argument, its implementation-defined notes and any marker it carries have been
  read and found to describe what the program actually does — for that record, the
  excess-intermediate-precision, conversion, literal, printing, characteristic-macro and magnitude
  obligations in the argument; the exclusion resting on the significand widths, 64 bits against 113,
  rather than on exponent range, which measures identically on all four targets; and a
  `comparison_excluded` marker whose `observed` field matches §4.2's `Observed` row character for
  character, as §1.1's forward check requires. The driver's `PENDING_RECORDS` declaration is
  therefore **empty**, and the record-substantiation preflight gate reports that every record the
  corpus holds is entitled to be read as evidence rather than withholding a feature area. The gate
  and this column are kept, because they are the mechanism rather than the instance: any record whose
  review has not completed is declared here, withheld before its first compile, and its area fails
  with an explicit non-evidence report rather than publishing verdicts drawn from unreviewed
  material — requirement 6 refusing to let the suite assert something it has no standing to assert.
- **Admitted as evidence about `bcc`** — **zero**, because there is no `bcc` binary on this branch.

| Matrix | Areas | Sources | Records | **Runnable programs** | `bcc` compile-and-run cells | Verdict outcome rows | Comparisons actually performed |
|---|---:|---:|---:|---:|---:|---:|---:|
| **Final planned target** | 14 | 108 | 108 | 108 | 1,296 (108 × 4 targets × 3 levels) | 3,564 | 3,555 |
| **Structural, on this branch** | **14** | **108** | **108** | **108** | **1,296** (108 × 4 × 3) | **3,564** | **3,555** |
| **Substantiated at this checkpoint** | 14 | — | **108** (0 pending) | **108** | **1,296** (108 × 4 × 3) | **3,564** | **3,555** |
| **Admitted as evidence** | 0 | 0 | 0 | **0** | **0** | **0** | **0** |

**A verdict outcome row is not a performed comparison, and this register in particular must not
conflate them, because the difference is created by a marker it holds.** All 3,564 rows reach a
verdict and appear in the summary. **3,555** of them are comparisons actually made. The other **nine**
are §4.2's: oracle (b) on `13_floating_point/004_long_double_target_restricted`, three non-baseline
targets × three optimization levels, which that record **disables** and `XD-TYPE-LONGDOUBLE-001`
documents. So oracle (b) is **963 performed comparisons plus 9 not-attempted `XFAIL` rows = 972 rows**
in total, and §3.3 is explicit that a not-attempted row is neither a pass nor a silent skip: nothing
was compared, so no equality is claimed. Those nine rows are the whole of the gap, which is why the
substantiated row now carries **3,564** verdict rows against **3,555** performed comparisons rather
than one figure for both: the record that narrows the oracle is itself substantiated, so its nine
not-attempted rows are counted in the milestone column too — and it remains the one narrowing record
in the corpus, which is also how it can be identified mechanically:

```text
grep -l 'oracle_b *= *disabled' tests/conformance/*/*.expected     # the one narrowing record -> 1
```

**A runnable program is a source paired with its record, and it is the runnable count — never the
source count — that every cell figure multiplies.** The rows are published separately because they can
disagree: a `.c` file with no sibling record cannot execute at all, since the record is where the
command templates and the `expect_exit` value live and oracle (c) would have nowhere to read a golden
from, so such a file contributes **zero** cells while still raising a count of `*.c` files. Counting
sources alone would therefore over-state the matrix by twelve cells for every unpaired program. On this
branch the structural counts agree at 108 — no unpaired source and no orphan record — and that
agreement is itself the thing a reader should check rather than assume:

```text
find tests/conformance -name '*.c'        | wc -l                 # sources  -> 108
find tests/conformance -name '*.expected' | wc -l                 # records  -> 108
for f in $(find tests/conformance -name '*.c'); do \
  [ -f "${f%.c}.expected" ] || echo "$f"; done | wc -l            # unpaired ->   0
```

**The structural row is now the planned row; the substantiated row has caught up with it; and the
admitted row is still zero.** Structurally the corpus is complete: fourteen areas, one hundred and
eight sources, one hundred and eight records, no unpaired source and no orphan record — so every cell
figure in that row is arithmetic over a **complete** pairing rather than a nominal projection over a
partial one. The substantiated row is the same arithmetic over the **108** records this checkpoint has
reviewed, so the two now agree; the column is kept separate rather than merged away, because it exists
so that "complete" is never read as "reviewed" and it is the column the next unreviewed record moves. The admitted row counts
something different and stricter again — the cells whose result the suite accepts as **evidence about
`bcc`** — and it is zero for one reason only:

> **There is no `bcc` binary on this branch.** The compiler implementation lives on the project's
> open pull request; this branch holds test material and documentation. A suite whose subject is
> absent can run its machinery and can prove its own arithmetic, and it cannot produce one byte of
> evidence about a compiler.

The two fail-closed mechanisms that used to hold the admitted figure at zero are worth stating in
their current state, because both are now **satisfied** and neither is what keeps the row at zero:

1. **There is no compiler under test in this checkout.** The flag-capability probe therefore cannot
   be performed at all — it requires a `bcc` path before it can verify a single flag — and a probe
   that established nothing is recorded `UNPERFORMED` and **blocks unconditionally**, the strict
   setting's licence covering an absent tool rather than a precondition that was never checked. That
   gate names no feature area, and a gate naming none governs **every** area, so this one mechanism
   holds the admitted figure at zero for all fourteen regardless of how complete the corpus is
   ([`README.md`](README.md#the-undefined-behaviour-audit-gate) describes the gate machinery).
2. **The per-program record requirement.** A cell is resolvable only from a program **together with
   its same-stem `.expected` record**, and a program without one is a hard error rather than a
   program that runs with defaults, because the record is what supplies the target list, the
   optimization levels, the command templates, the expected exit status and the golden stdout
   ([`README.md`](README.md#corpus-discovery)). It removes nothing here — the pairing is complete at
   108 — but it remains the reason the source count carries no cell figure of its own, and it is
   what a future unpaired source would be caught by.
3. **Whole-corpus enumeration behind the undefined-behaviour audit.** The audit enumerates the
   corpus **globally**, across all fourteen declared areas. While any program was unpaired the
   enumeration failed for it, the audit was recorded as failing, and that gate **blocked the areas
   it named** — because a corpus that was never audited cannot make any divergence attributable,
   which is requirement 1's whole point. All fourteen areas are now present and every program is
   paired, and the gate is **met**: every one of the 108 programs passes the strict warning gate and
   the sanitizer gate, in 324 reference-compiler invocations.

None of the three is a defect to be worked around; together they are what stops a partial corpus, or
an absent compiler, from reporting a green matrix. The consequence for this file is simply that **the
admitted figure is stated as zero while there is no `bcc` to observe**, and quoting the present or
planned figures as though they described what has been established would be exactly the unverifiable
claim the paragraph above refuses to make.

**Measured, not assumed — and this is why the admitted row is not simply the structural row.** The
suite has been run on this branch against a **surrogate** compiler under test rather than `bcc` — a
stand-in that forwards to the reference toolchain — because this checkout carries no compiler at all.
Under the surrogate the cell machinery runs end to end, every dimension of the structural row reports
its figure, and the tally is **3,564 verdict outcome rows: 3,543 `PASS`, 9 `XFAIL` and 12 `XPASS`**.
The nine `XFAIL` are §4.2's not-attempted oracle (b) rows. The twelve `XPASS` are the arms §4.1's
marker scopes, which a stand-in that accepts case ranges necessarily agrees on, so that run is taken
with `BCC_CONFORMANCE_ALLOW_XPASS` set — §4.1 and §8.3 record why a surrogate `XPASS` establishes
nothing. In this checkout itself, with no compiler at all, mechanism 1 blocks every area. Both figures
are reported for exactly one purpose: to show that the structural row is honest arithmetic and that the
machinery producing it works. Neither is a **result**, and one caveat travels with every figure in
them — the compiler under test was a surrogate, so nothing observed is evidence about `bcc`.

So the rows stay separate, and each keeps a distinction the others cannot: what the corpus
**represents** is fully counted, what this checkpoint has **reviewed** now matches it record for
record — every one of the 108, none pending — and what has been **established about `bcc`** remains
nothing at all. Read the first two rows agreeing as the fact it is rather than as a reason to merge
them: they are different *claims* that happen to carry the same figure at this checkpoint, one about
which files exist and one about which of them have been through review, and the second will fall behind
the first again the moment a program lands with a record whose review has not completed. `README.md`
§"The enumerable matrix" carries the same basis in more detail; the two documents are written to be
checked against each other and against the files.


### 8.3 Retirement log

A retirement is recorded here by **date and description only** — never by identifier, because the
reverse-direction audit in §1.1 reads this whole document and would then look for a marker that no
longer exists. An entry naming the program, the divergence and the date is enough for a reader to
reconstruct the rest from version control, which is where the diff belongs.

**No marker is retired as the corpus stands.** Both markers in §4 are active, and no identifier is
spent, so nothing here forbids reuse — that is the honest reading of the corpus rather than an
omission. What the log below records is not a completed retirement but every withdrawal that was
*attempted and reversed*: each row names a date and a program, and every withdrawal has a later row
above it reinstating the same marker. The history is kept rather than deleted precisely because the
same mistake was made more than once, always by reading a run as an argument, and a reader who cannot
see that pattern is likely to repeat it. Retiring a marker for real is §3.2's two deletions plus a row
here; the two deletions are what the audit checks, and the row is what tells the next reader an
identifier is spent and must not be reused for a different divergence (§2.3).

| Date | What was withdrawn, and why |
|---|---|
| 2026-08-04 | **The GCC case-range marker on `08_gcc_extensions/004_case_ranges.c` was reinstated, superseding the retirement recorded immediately below, and the two sentences of this register that had licensed that retirement were corrected.** A security review of the marker authorities found the retirement unsound on its evidence: the `XPASS` that triggered it came from a compiler under test that **forwards every invocation to the reference toolchain**, so the twelve agreements were one toolchain compared with itself — evidence about the environment, and about neither `bcc` nor case ranges. Reference-versus-itself agreement cannot establish that a real `bcc` supports the construct, so it cannot retire a marker the **frozen** project specification fixes for this program by identifier, class, scope and basis. Three edits put the register back in line with its own contract. The marker block returned to the record and the summary row and structured entry returned to §4.1, with the `observed` field now stating explicitly which half of the pair is measured (the reference half, `gcc` 13.4.0, four targets, three levels) and which carries no independent capture, and with `expected_divergence.evidence` left unwritten rather than filled in from the surrogate. §1.1's "Observed — not anticipatory" row and §2.1 no longer claim that an unobserved divergence must carry **no marker at all** — the rule the parser enforces is about *wording*, and generalising it into an existence rule is what made a specified marker unwritable and produced the four reversals logged here. And §3.2 now states the retirement precondition the omission left implicit: the `XPASS` must come from the **real** compiler under test, with `BCC_CONFORMANCE_ALLOW_XPASS` as the sanctioned response for a surrogate environment (§3.1). **Nothing was weakened to achieve this.** No parser rule, no classifier rule and no scope grammar changed; anticipatory wording is still refused at parse time; §8.5 step 1 still applies in full to every marker an author mints of their own accord; and §3.1 still fails a run on `XPASS` by default, which is precisely what keeps an unconfirmed marker a loud state rather than a silent pass. The program, its matrix, its three enabled oracles, its gate deviation and its golden record are **untouched** — twelve cells still run, exactly as they did before and after every previous reversal. |
| 2026-08-04 | **The GCC case-range marker on `08_gcc_extensions/004_case_ranges.c` was retired, on the strength of a run rather than an argument, and the marker contract was left exactly as it stands.** The reinstatement recorded immediately below put the marker back in the record and in this register while the divergence it described had still never been observed — this branch carries the corpus and no `bcc` binary. The first run with a compiler under test settled it: the program compiled, the arm the marker scoped agreed with the reference compiler byte for byte on all four targets at all three optimization levels, and §3.1 did what it exists to do — twelve `XPASS` outcomes, a failing run, and a message naming the marker and both places to retire it. §3.2's two deletions have been performed: every `expected_divergence.*` key is gone from the record, and this register's summary row and structured entry are gone with it, so no identifier survives here that the reverse check cannot resolve. **The program, its matrix, its three enabled oracles, its gate deviation and its golden record are untouched** — twelve cells still run, and the whole analysis the marker carried is preserved in §4.1 and in the program's own `impl_defined_notes`, together with the exact shape a reinstatement must take. Two things are recorded rather than glossed. The compiler under test was a **surrogate** forwarding to the reference toolchain (§8.2), so the observation establishes that the predicted divergence is absent from the only compiler under test available, not that `bcc` accepts case ranges. And the retirement was said not to rest on the `XPASS` alone, but on a reading of §1.1's "Observed — not anticipatory" rule as an existence rule rather than a wording rule — **that reasoning is superseded by the entry above**, which corrected the two sentences it rested on and reinstated the marker. **Nothing was weakened to achieve this** — no rule in §2 was relaxed, the classifier was not touched, and the classification available to a case-range divergence moved in the strict direction: a refusal is now a `FINDING` where it would have been an `XFAIL`, delivered with a reproducer, both sides' output, an environment fingerprint and exact commands, which is better material for settling §4.1's open ambiguity than an excuse would have been. |
| 2026-08-03 | **Both specified markers were reinstated, and the contract that had made them inexpressible was corrected.** The withdrawal recorded immediately below had added two required keys (`documented`, `evidence`), refused any basis resting on an **omission**, and required a refusal-class marker to scope every oracle its refusal blocks. Between them those three rules made the two markers the frozen specification mandates by identifier, class, scope and basis impossible to write — the case-range marker's basis *was* an inventory omission at the time, before §4.1 moved it to the affirmative compliance rule, and its scope *is* `oracle_a` alone — so the contract was not enforcing the specification but overruling it. The corrections: `documented` and `evidence` became **optional enrichment** rather than acceptance conditions (§2.1); an omission-based basis is **admitted**, with the format enforcing only that the citation can be *followed* to a resolvable locator and that a supplied quotation occurs inside the cited range, leaving the strength of the citation to a reviewer reading §4 (§2.4.1); and the `all oracles` requirement was replaced by **dependency-aware classification** in the classifier, where the marked arm settles a refusal and every other arm of the cell is reported as a dependent blocked expected divergence citing that root (§2.3). One rule moved the other way and became **stricter**: a record that disables an oracle must now carry a marker whose scope names it, because a disabled oracle is already reported `XFAIL` and had been claiming that authority on prose alone — so `XD-TYPE-LONGDOUBLE-001` is now required rather than merely permitted, and a markerless narrowing is a `FAIL` (§3.3). Every program, matrix, oracle toggle and golden record is **untouched** by all of this. |
| 2026-08-03 | The GCC case-range marker on `08_gcc_extensions/004_case_ranges.c` was **withdrawn, and the marker contract was changed so that it could not be reinstated in that shape — a change that was itself reversed the same day by the entry above.** Two independent defects were identified, either sufficient on its own: its basis rested on an **omission** from the documented extension inventory — silence, which authorises nothing and which extending the inventory would remove without disturbing the excuse — and its `observed` field described the divergence as **anticipated**, in the future tense, with no command, status, output or toolchain behind it. The parser was made to refuse both, a refusal-class marker was required to cover every oracle its refusal blocks, and `documented` and `evidence` were made required keys. **None of those four rules is in force any longer** — the entry above reversed three of them and kept the anticipatory-wording refusal, which is the one that survived on its own merits and is still enforced (§1.1). The second defect identified here was real and was fixed in the marker rather than in the contract: the reinstated marker's `observed` field states what was seen, in the present tense, and no longer describes the divergence as predicted. The program, its matrix, its oracle toggles and its golden record were **untouched** throughout. The analysis is preserved in §4.1 and in the program's own `impl_defined_notes`. |
| 2026-08-02 | The GCC case-range marker on `08_gcc_extensions/004_case_ranges.c` was **reinstated**, superseding the withdrawal recorded below. The suite's brief specifies that marker by identifier, class, scope and basis, and requires the record beside that program to carry it, so its presence is a requirement of the brief rather than an inference drawn in this register. Its anticipatory nature is not hidden: §4.1 states in as many words that no `bcc` verdict has yet been recorded on this branch, that the marker is therefore anticipatory, and that an acceptance produces the `XPASS` that fails the run — with §3.2's two-deletion retirement as the remedy. The program, its matrix, its oracle toggles and its golden record are again **untouched**, so the feature remains fully under test either way. |
| 2026-08-01 | The GCC case-range marker on `08_gcc_extensions/004_case_ranges.c` was **withdrawn without the divergence it described ever having been observed**. It had been minted on the strength of an omission from the documented extension inventory, which §4 and §8.5 step 2 both establish is not a documented limitation. Because nothing had been observed or reproduced, the marker risked failing a run on `XPASS` — on the test material rather than the compiler — and would meanwhile have blinded the suite to a real regression in exactly that construct. The program, its matrix, its oracle toggles and its golden record are **untouched**, so the feature remains fully under test; a divergence there is now a `FINDING` until an explicit documented limitation and a real observation both exist. The analysis the marker carried is preserved in §4.1 and in the program's own `impl_defined_notes`. |

### 8.4 Cross-links

| Document | What it holds |
|---|---|
| [`README.md`](README.md) | The suite contract: the three oracles, the verdict taxonomy, the environment variables, the artifact locations, the `.expected` record format (including the marker block), and how to reproduce any cell by hand |
| [`FINDINGS.md`](FINDINGS.md) | The register of **undocumented** divergences. Its curated set is empty on this branch, which is what a suite with no compiler under test can honestly report. Each entry points at a finding directory holding a **verbatim** reproducer — a byte-for-byte copy of the corpus program — its recorded minimization status, and exact reproduction commands. Automated reduction is never performed during a run; reduction is a supervised activity performed on the copy before a finding is promoted to the curated set |
| [`../conformance.rs`](../conformance.rs) | The suite driver, including `infra_expected_divergence_register`, the test that keeps this file and the corpus honest in both directions |
| [`../../docs/testing/differential-conformance.md`](../../docs/testing/differential-conformance.md) | The documentation-site page, committed and published in the MkDocs navigation: methodology, oracle definitions, the build matrix, the verdict taxonomy and the deliverable summary format |

### 8.5 Add-a-marker checklist

The mirror of the retirement procedure in §3.2. Every step is required, and the run will tell you if
you skip one:

1. **Confirm the divergence is real and reproducible.** Same divergence, from the recorded commands,
   on a clean workspace, on more than one run. Capture the output from each compiler and each
   backend involved.
2. **Identify the documented basis and make it followable.** Find the actual file and section that
   documents the limitation — §7 is the inventory of citable sections — and write the citation with a
   **locator the audit can resolve** (a line, a line range, a `§` section number or a
   backtick-quoted phrase, §2.4). Where a passage in that range authorises the marker, put it
   **verbatim** in the optional `expected_divergence.documented`, which the audit then resolves
   **inside the cited range** rather than anywhere in the file. Prefer a statement the document
   actually makes: §4.1's marker began on an inventory's silence and now cites the compliance rule
   that fixes the **required** extension set, which is a claim a reader can weigh rather than a gap
   they must interpret. An **omission** from an inventory is nevertheless admitted as a basis — the
   frozen contract's case-range marker was minted in exactly that form, and a format that cannot
   express its own specification's markers is the thing that needs changing — and the audit does not
   adjudicate its strength; what it enforces is that a reader can follow the citation to the passage
   and judge for themselves, and §4 states plainly, per marker, how strong the basis is. Admission is
   not endorsement: an omission is the weakest basis this format accepts, and a marker resting on one
   owes its register entry an explicit statement to that effect. **If there is
   no citable document at all, there is no expected divergence:** the correct outcome is a finding,
   recorded in the committed findings register [`FINDINGS.md`](FINDINGS.md) with its reproducer.
3. **Add the five required `expected_divergence.*` keys** to the program's `.expected` record, plus
   either or both of the optional keys you can honestly fill — they are independent, and a marker may
   carry both. Keep the scope no wider than the oracle, targets,
   levels and class actually observed (§2.3), and write the observation as it was actually seen. For a
   **build refusal** — `compile_failure`, `link_failure`, or a `timeout` on the compile invocation,
   the three that produce no artifact — you may scope the marker to the single arm its basis speaks
   for; you do **not** have to widen it to `all oracles`, because the classifier settles that arm on
   the marker and reports every other arm of the cell as a dependent blocked expected divergence
   citing it (§2.3). A `run_crash` or a `timeout` on the run is not a refusal: an artifact was built
   and launched, so only the arm that compared the two runs observed anything, and the marker names
   that arm exactly as a `stdout_mismatch` marker does. Scope it `all oracles` only when the basis genuinely speaks for all
   three arms. If instead the marker documents a comparison the record deliberately **does not
   make**, scope it to the oracle the record disables, class it **`comparison_excluded`** (§2.2.1) —
   the parser requires that class there and refuses an observational one — and say in `observed` what
   was measured about the construct rather than about a comparison. Such a marker is **required**, not
   optional, is dormant by construction, is consulted on exactly the not-attempted cells, and must
   move together with the toggle and the recorded reason (§3.3, §4.2).
4. **Add the summary-table row in §4 and a structured entry** in the shape §1.3 defines: a heading
   naming the marker, then a table stating Identifier, Class, Scope, Program, Basis and Observed.
   Reproduce the record's basis string **verbatim** in both places — one canonical rendering,
   character for character (§2.4) — so that the record and this register cannot give two accounts of
   the same authority. `Identifier`, `Class`, `Scope`, `Program` and `Basis` are each compared
   exactly after trimming, so any other difference in them fails the run; `Observed` is the one
   field the comparison relaxes, and only to collapsed whitespace, so it may be reflowed to fit a
   table cell but not reworded (§1.3). The entry's six fields are compared against the record on
   every run, so a mismatch fails immediately and names both readings; the §4 summary row is prose
   and is not compared, so it is the one place a paraphrase can survive.
5. **Re-run the audit** — `cargo test --test conformance infra_expected_divergence_register`, on the
   package-complete branch (§1.1) — and then the owning area, to confirm the divergence now
   classifies as `XFAIL` rather than `FINDING`. On this branch that audit resolves **two** markers
   against **two** registered identifiers in both directions, checks that each cited document and
   locator resolves, and checks that each supplied quotation occurs inside its cited range; passing
   it is what proves the register and the corpus still agree.

Do **not** invent a marker of your own speculatively, before the divergence has been observed. A
marker on a program that agrees is an unexpected success, which fails the run — and until it is
noticed, it blinds the suite to a real regression in exactly the construct it was meant to document.
That is why step 1 asks for the observation first and why `evidence` exists at all.

**That rule governs every marker an author mints of their own accord, and the two the frozen brief
names are not an exception to it so much as a case it does not decide.** The brief fixes those two by
identifier, class, scope and basis, for a named program each, and requires the record beside that
program to carry the marker. Step 1 is therefore not something an author of one of those two can
discharge or waive — the decision was made upstream, and this register enforces it rather than
re-litigating it. What is required instead is **disclosure**: leave `evidence` unwritten, and state in
`observed` which half of the pair is measured and which is not, so no reader mistakes a specified
marker for a captured one. §4.1 is written exactly that way.

**What keeps that honest is §3.1, not this checklist.** An unconfirmed marker on a construct that works
produces `XPASS` and fails the run, naming the marker and both places it lives, so the gap is a loud
state rather than a silent pass — and retirement then needs an **independent** observation, which an
agreement produced by a stand-in forwarding to the reference toolchain is not (§3.2). Treating "named
in the brief" as *insufficient warrant to carry* and then retiring a specified marker on surrogate
agreement is what produced the four reversals in §8.3; treating it as *warrant to carry, with the
`XPASS` safeguard intact* is what this register now does. For every candidate the brief does **not**
name — §4.3, §5 and §6 — steps 1 and 2 above apply in full, and a divergence that is unobserved, or
observed with no citable document behind it, is a `FINDING`, which records it with strictly more
evidence than an excuse does.
