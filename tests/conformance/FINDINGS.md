# Findings — Register

The durable, single-place index of every **finding** the differential conformance suite has
recorded.

A *finding* is a divergence between `bcc` and one of the suite's oracles that **no limitation this
repository documents** accounts for. Requirement 6 of the suite's brief says what to do with one:
keep the program — minimized as far as practical — keep the outputs from each compiler and each
backend, keep the **exact reproduction commands**, and **do not attempt to fix the compiler**.
Findings are deliverables, not defects to patch. This file is where the set of them is reviewable in
one reading.

**No finding has been curated.** The register in §2 is intentionally empty: the suite has not yet
recorded an undocumented divergence, and [`findings/`](findings/) accordingly holds nothing but its
`.gitkeep`. An empty register is a factual statement about the current state, not a placeholder —
and it is the same discipline that keeps §7.2 from publishing a coverage percentage nobody in this
repository could verify.

## 1. What a finding is, and what it is not

Three verdicts sit next to one another, and telling them apart *is* the point of this file. All
three begin with an observed difference; they differ in whether the difference is **explained**, and
by what.

| Verdict | The divergence is … | Where it is recorded | Fails the run? |
|---|---|---|---|
| **FINDING** | **not** traceable to any limitation this repository documents | a self-contained artifact directory under [`findings/`](findings/), indexed here | **No** — it is a deliverable |
| **XFAIL** | traceable to a limitation this repository **does** document, cited by a marker | the marker in the program's own `.expected` record, mirrored in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) | No |
| **FAIL** | neither of the above — unexplained: harness breakage, an internal inconsistency, a corpus defect | the area report and the run summary | **Yes** |

The full six-verdict space, of which these are three, is defined once in
[`README.md`](README.md#the-verdict-taxonomy) and implemented in
[`classify.rs`](../conformance_harness/classify.rs). Two properties of it matter here and are worth
stating outright:

- **A FINDING does not fail the run.** It is reported loudly, counted in the run summary and
  accompanied by complete artifacts, but the run still passes on it, because its meaning is *known
  and delivered* rather than unknown. The two verdicts that fail a run are **FAIL** and **XPASS** —
  the latter being a marker whose divergence has disappeared, which is stale documented knowledge
  and is treated as a failure by default.
- **There is no "skip because unsupported" verdict.** A divergence nobody can explain never becomes
  a silent pass; it becomes a finding with its evidence attached, or a FAIL. An oracle whose tooling
  is absent from the machine is reported UNAVAILABLE — loudly, and in the summary — and is likewise
  never a pass.

### 1.1 Report, never patch

**No compiler source change is made in response to any finding.** Two independent instructions say
so, and they agree without tension:

- **Requirement 6** — *do not attempt to fix the compiler; findings are deliverables, not defects to
  patch.*
- **Constraint C1** — *do not modify the compiler's source code.*

That is why `src/**`, `include/**`, `build.rs`, `Cargo.toml` and `Cargo.lock` are read-only
reference material throughout this work, and why no entry in this register will ever be paired with
a patch. The suite's entire response to a divergence it cannot explain is *evidence*: write it down
completely enough that somebody else can act on it, and stop.

Acting on a finding is a maintainer's decision, taken outside this suite and informed by the
artifacts. That is also why §2's `Status` vocabulary has no `fixed` value: this register cannot
observe a fix, so it does not pretend to record one.

### 1.2 A finding is a triage signal, not a verdict on the compiler

A finding says exactly one thing: *at this cell, judged by this oracle, the observable behaviour
differed, and nothing in the repository's documentation accounts for the difference.* It records
which construct, which oracle, which target and which optimization level, with everything needed to
reproduce the observation on another machine.

It does **not** say the compiler is poor, nor that the divergence is the compiler's fault rather
than the reference toolchain's, nor even that it will still be observable next week — which is
precisely why `environment.txt` (§3.3) fingerprints every tool involved. Read a finding as a lead to
investigate, with the investigation's starting evidence already collected.

## 2. The register

**No findings have been curated.** The table below is deliberately present and deliberately empty:
the suite has not yet recorded an undocumented divergence, so there is no row to write. Nothing is
withheld, and no illustrative row has been invented to make the table look populated — a register
row pointing at a directory that does not exist is a **broken deliverable**, because the maintainer
who follows the pointer finds nothing, and reproducibility is the one property that makes a finding
worth having.

| ID | Slug / short title | Program (area / reproducer) | Divergence class | Oracle(s) affected | Target(s) | Opt level(s) | Artifact directory | Status |
|---|---|---|---|---|---|---|---|---|

Two notes on reading the table once it has rows. **One row per finding**, so a divergence observed
by two oracles at the same cell is two rows if the two were investigated separately and one row
naming both oracles if they were not — the artifact directory decides, because the directory is the
finding. And the columns are deliberately narrow, factual fields: the *explanation* lives in the
directory's `MANIFEST.txt`, never here, so that this file stays an index a reader can scan.

### 2.1 The `Status` vocabulary

A closed set of three values. Anything else is not a status but a comment, and belongs in the
finding's `MANIFEST.txt`.

| Status | Meaning |
|---|---|
| `open` | Recorded and reproducible; not yet triaged by a maintainer. This is what every finding starts as |
| `acknowledged` | A maintainer has confirmed the finding — read the evidence, reproduced or accepted it, and taken ownership of what happens next |
| `superseded` | A later finding subsumes this one; the row stays, naming the finding that replaced it, because deleting it would erase the history a reader needs |

**There is deliberately no `fixed` status.** Fixing is out of scope for this work by requirement 6
and constraint C1, so a fix is something this suite can neither perform nor witness. A maintainer
who resolves a finding does so outside this suite; the durable record of that resolution belongs in
the compiler's own history, not in a column here that no run could ever set honestly.

A **retired** finding is not a fourth status either. A finding whose divergence turns out to have
been a defect in the test program — a program with undefined behaviour, most often — was never a
finding at all: the correct action is to rewrite the program and remove the row, recording the
withdrawal in §7.4 by date and description. See §6.3, which is the check that should have caught it
first.

### 2.2 The six legal divergence classes

The `Divergence class` column takes exactly one of these, spelled exactly as shown. The class space
is closed and is shared with the expected-divergence marker vocabulary, so a finding and a marker
describe an observation in the same words:

| Class | The observation it names |
|---|---|
| `compile_failure` | One compiler rejected a program the other accepted |
| `link_failure` | The program translated but did not link |
| `run_crash` | The program died on a signal instead of exiting. Compared as a raw wait status, so it is never conflated with an ordinary exit carrying the same number |
| `exit_code_mismatch` | Two completed runs disagreed on exit status |
| `stdout_mismatch` | Two completed runs disagreed on stdout bytes — the ordinary shape of a wrong answer |
| `timeout` | Execution outlived its per-cell budget. A first-class divergence, not an infrastructure error: a program that finishes promptly under one compiler and hangs under another is exactly the defect this suite exists to surface |

Two boundaries on that list, both of which decide whether a row belongs here at all:

- **A `link_failure` attributable to *this machine*** — a target's cross C runtime is simply not
  installed — is reported UNAVAILABLE at environment scope and is **never** a finding against the
  compiler. Filing one would blame the compiler for an absent package.
- **No class describes a difference in diagnostic text.** Standard error is never compared, by any
  oracle (§3.2), so a finding can never be *about* a compiler's wording.

### 2.3 The identifier convention

A curated finding is identified as:

```text
F-NNNN-<slug>
```

- **`NNNN`** — a zero-padded four-digit sequence number, allocated in **ascending order** and
  **never reused**. An identifier names one investigation; reusing it would silently merge two,
  and a reader holding an archived directory would have no way to tell which one they had.
- **`<slug>`** — a short, lowercase, hyphen-separated slug naming **the construct that diverged**,
  not the symptom. `struct-return-by-value` is a slug; `wrong-output` is not.

The register row's `Artifact directory` value is exactly `findings/F-NNNN-<slug>/`, with the
allocated identifier substituted — a path relative to this file, so it is directly followable from
the rendered table.

**The generated directory name is a different thing, on purpose.** A run does not allocate register
identifiers: it names each transient directory from the divergence itself — a stable digest together
with the cell's own identity, the oracle letter and the divergence class — so that the same
divergence always names the same directory, two different divergences can never collide and
overwrite one another's evidence, and no counter is consulted. That last property is a correctness
requirement rather than a preference: the fourteen feature-area tests run concurrently in one
process, so a shared counter would be a data race *and* would make a name depend on thread
scheduling. The exact spelling of the generated name is documented once, in
[`README.md`](README.md#the-finding-identifier) and
[`findings.rs`](../conformance_harness/findings.rs), and is deliberately not restated here so the
two cannot disagree.

The two are tied together by the artifact itself: curation keeps the generated
`finding_id` and `identity_digest` lines in `MANIFEST.txt` intact, so a curated directory can always
be traced back to the run that produced it, and the short `F-NNNN-<slug>` handle stays what a human
quotes.

## 3. What a curated finding directory holds

A finding is a deliverable, so its directory is designed around one goal: **a maintainer must be
able to reproduce it without the harness.** No Cargo, no Rust toolchain, no `cargo test` — the
reproducer, `commands.sh` and `environment.txt` are sufficient on their own.

Seven artifacts, and all seven are required. A directory missing one is not a finding; it is a
half-recorded observation, and the suite treats a finding whose artifacts could not be written as a
FAIL rather than downgrading it to a silent pass.

| Artifact | Content | Why it exists |
|---|---|---|
| `reproducer.c` | The program, **minimized as far as practical** while still provoking the divergence | The divergence has to be *in* something. A reproducer no larger than the difference needs is what makes the observation readable in one sitting |
| `reproducer.expected` | Its expectation record, in the standard `.expected` format, keeping the **corpus** program's identity rather than the copy's path (§5.2) | Keeps the finding **runnable by the harness**, so it can be re-checked over time instead of becoming a static curiosity that nobody can tell is still true |
| `MANIFEST.txt` | The finding identifier, the affected area and program, the divergence class, **which oracles and which cells diverged**, and a one-paragraph description of the observed difference | The one file to read first. Everything else in the directory is evidence; this is the account of what the evidence shows |
| `commands.sh` | **Exact, copy-pasteable compile and run lines for every cell involved**, runnable as `sh commands.sh` | This is the artifact that satisfies requirement 6's *"the exact reproduction commands"* — and it does so **without the harness, without Cargo and without a Rust toolchain** |
| `outputs/<oracle>-<target>-<opt>.{stdout,exit,stderr}` | The captured output from each compiler and each backend involved | Evidence, byte for byte. A description of a difference is not the difference; the captures are what a second reader checks the description against |
| `environment.txt` | The fingerprint: the `bcc` version, the reference compiler version, each cross-driver version, each emulator version, and the kernel identification | Lets a divergence be attributed to **toolchain drift rather than to the compiler** — see §3.3 |
| `diff.txt` | The computed difference, with the **first divergent line and byte offset** highlighted | Turns "these two outputs differ" into "they part company here", which is where investigation actually starts |

`reproducer.c` is still a corpus program and every corpus authoring rule still binds it. In
particular it must hand-declare `int printf(const char *, ...);` and **include no header** — the
sanctioned exception being `<stdarg.h>` for a variadic program — print a fixed, deterministic,
multi-line sequence, print no addresses or pointer values, and be free of undefined and unspecified
behaviour. Those rules are stated in full under
[Corpus authoring rules](README.md#corpus-authoring-rules); the reason the header rule exists is
that `bcc` ships no `stdio.h`, so an `#include <stdio.h>` would fail against `bcc` while succeeding
against the reference compiler — a spurious divergence caused by the test rather than by the
compiler, which is the worst possible thing to find in a finding.

### 3.1 Reading an `outputs/` name

Every entry is `<side>-<target>-<opt>` plus an extension, where `<side>` is the leading token
written `<oracle>` in the table above:

- **`<side>`** is `bcc` for the compiler under test — the *subject* of every comparison — or the
  oracle's own letter `a`, `b` or `c` for the *authority* it was judged against. A name therefore
  says which side produced it without needing a legend, and an authority's target is the
  authority's **own** target, so a baseline capture can never be mistaken for the subject it judged.
- **`<target>`** is the short target name: `x86_64`, `i686`, `aarch64` or `riscv64`.
- **`<opt>`** is the optimization level with its hyphen dropped: `O0`, `O1` or `O2`.

Concretely, one authority's three captures at a single cell:

```text
outputs/a-aarch64-O2.stdout     the program's standard output, byte for byte
outputs/a-aarch64-O2.exit       how it ended: the decoded termination and the raw wait status
outputs/a-aarch64-O2.stderr     the program's standard error, byte for byte
```

The **compiler's** own streams for that same cell are a different thing entirely and live in their
own entries — `a-aarch64-O2.compile.stdout`, `.compile.stderr` and `.compile.exit` — present
whenever there was a build. That split has no exceptions, and the absence of exceptions is the
point: a
scheme that put compiler diagnostics into `.stderr` whenever the program had not run would force
anyone opening a `.stderr` file to work out first whether that side's build had succeeded, turning
every inspection into a case analysis.

### 3.2 Standard error is captured and never compared

This looks contradictory written down, so it is worth separating the two halves.

**Never compared.** Diagnostic wording legitimately differs between compilers — different phrasing,
different notes, different column numbers, different amounts of helpfulness. Comparing it would bury
every real finding under a flood of differences that say nothing whatsoever about code correctness,
and it would make the oracles unusable within a day. No oracle compares standard error, and no
divergence class describes a difference in it.

**Captured anyway.** Diagnostic text is very often the *fastest route to a diagnosis*: a rejected
build's message usually names the construct outright. So it is preserved in the artifact, to be read
by a human rather than judged by an oracle. Being unsuitable as an oracle input does not make it
unhelpful as evidence.

### 3.3 Why `environment.txt` is one of the seven

A divergence that appears — or disappears — between two runs has two possible explanations: the
compiler changed, or the machine did. Without a record of the machine, those are indistinguishable,
and the wrong one gets believed.

So the fingerprint records every tool the observation depended on: `bcc`, the native reference
compiler, each cross driver, each emulator, the per-cell timeout budget, each target's C runtime,
the host and kernel identification, and this run's own token and configuration. This directly
mitigates a risk the repository already documents in its own register — *"QEMU version
incompatibility for cross-arch testing"*, `docs/project-guide.md` line 256, recorded as *Partially
mitigated* with *"version pinning recommended for stability"*. A finding that turns out to have been
an emulator upgrade is a finding correctly *closed*, and it can only be closed that way if somebody
wrote down which emulator it was.

`environment.txt` is also one of exactly two artifacts that legitimately differ between two runs of
one unchanged divergence — it is a fact about the run, not about the divergence — which is why
everything else in the directory can be diffed across runs and any difference read as real.

## 4. The transient-versus-curated split

**Preserve this split.** It is the whole reason an in-progress run cannot pollute a committed
deliverable.

| Set | Location | Written by | Committed? |
|---|---|---|---|
| **Curated** | `tests/conformance/findings/F-NNNN-<slug>/`, indexed by this file | a **human**, after review | Yes — this is the durable deliverable |
| **Transient** | `target/conformance-findings/`, one directory per finding of the current run | the harness, on every run | No — git-ignored, beneath the build directory |

Two further transient trees belong to the same run and are named here because a maintainer
investigating a finding will want them:

| Path | Content |
|---|---|
| `target/conformance-work/` | Per-cell workspaces. **Retained** for any cell that produced a FAIL, an XPASS or a FINDING — and for every cell when `BCC_CONFORMANCE_KEEP_WORK` is set — so a divergence leaves behind exactly the artifacts needed to investigate it; removed otherwise |
| `target/conformance-report/` | `areas/<area>.md` and `areas/<area>.tsv` per feature area, plus `summary.md` and `summary.tsv` for the run as a whole |

**The harness never writes into `tests/conformance/**`.** Not the curated findings directory, not
this register, not the expected-divergence register, not a corpus program. The corpus is opened for
reading only, every write a run performs is beneath the build directory and re-checked against that
root immediately before the bytes are published, and the report and generated-finding roots are
emptied whole at the start of every run so that one run's summary can never aggregate another's
results.

**Curating a finding is therefore a deliberate human act**, and that is the payoff: copy the
relevant artifacts out of `target/conformance-findings/` into `tests/conformance/findings/`,
minimize the reproducer, allocate the identifier, add the row here. Nothing a run does can perform
any of those steps, so **no in-progress run — and no failed, filtered or reduced run — can ever
place anything in the curated set.**

### 4.1 This file versus the run summary

They are different artifacts answering different questions, and conflating them loses one of them.

| Artifact | Scope | Lifetime |
|---|---|---|
| `target/conformance-report/summary.md` | **This run.** Its section 4 lists every finding the run recorded, with a pointer to the reproducer and the reproduction commands, alongside the feature areas covered, the outcome tally and every expected divergence with its documented basis | Overwritten by the next run |
| This file | **Every curated finding, across runs** | Committed; durable |

A run's summary is the deliverable the requirements ask each run to emit. This register is what
survives the run — the place a reader goes to ask *"what has this suite ever found?"* rather than
*"what did it find just now?"*

## 5. Curating a finding

Eight steps. Every one is required, and skipping any of them produces a register row that cannot be
trusted.

1. **Run the suite and read the verdicts.** `cargo test --test conformance` and then
   `target/conformance-report/summary.md`; its findings section names every FINDING the run recorded
   and points at the transient artifact directory for each. A FINDING does not fail the run, so it
   will not announce itself by failing — read the summary.
2. **Confirm the divergence is real and reproducible.** Re-run the affected cell, and then reproduce
   it **by hand** from the `.expected` command templates — the procedure is in
   [`README.md`](README.md#reproducing-a-cell-by-hand). Reproducing it outside the harness is what
   rules out a harness artifact, which is a category of mistake that costs a maintainer a day if it
   reaches the register. **Then confirm the program is free of undefined and unspecified behaviour
   and that both undefined-behaviour gates were actually performed (§6.3)** — this is the
   precondition that makes the divergence mean anything at all, and a program that fails it is
   rewritten rather than filed.
3. **Check it is genuinely undocumented.** Search `docs/technical-specifications.md` and
   `docs/project-guide.md` for a limitation that actually covers the observation. **If one exists,
   this is an expected divergence and not a finding**: add the five-key marker to the program's own
   `.expected` record and register it in
   [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) instead, then stop. Note the asymmetry
   carefully, because it is the step most easily got wrong in the other direction too: an
   **omission** from a documented inventory is *not* a documented limitation. It records that
   nothing mentions the construct, not that the implementation rejects it — so an omission leaves
   the observation exactly where it was, a finding.
4. **Minimize the reproducer** as far as practical while it still provokes the divergence, keeping
   every corpus authoring rule intact (§5.1).
5. **Allocate the next `F-NNNN-<slug>` identifier** — the next unused four-digit number, ascending,
   never a reused one — and create `tests/conformance/findings/F-NNNN-<slug>/`.
6. **Populate all seven artifacts** from §3. Copy the captures, the fingerprint and the diff out of
   the transient directory rather than reconstructing them; keep `MANIFEST.txt`'s generated
   `finding_id` and `identity_digest` lines so the curated directory stays traceable to the run that
   produced it; and update its description if minimization changed what the reproducer does.
7. **Add one row to the register** in §2, with `Status` = `open`.
8. **Do not modify the compiler.** Requirement 6 and constraint C1 both forbid it (§1.1). If the
   finding suggests a fix, that suggestion belongs in the manifest's description, where a maintainer
   will read it.

### 5.1 Minimization guidance

**Minimization is manual or scripted, and it happens during curation — never during a run.** A run
writes the reproducer as a **verbatim** copy of the corpus program and records that status
explicitly, for three reasons worth restating because the decision can look like a shortcut:
reduction is not a bounded operation (thousands of rebuilds; hours is ordinary), it is not
deterministic (its result depends on the reducer's version and pass schedule, which would make a
finding directory that is *meant* to be diffed between runs change on its own), and the corpus is
already close to minimal by construction.

That last point is the one that makes minimization tractable in the first place. **Every corpus
program exercises one semantic concern and prints one line per semantic property it claims**, so a
single divergent line already localizes the defect to a single construct. There is usually very
little left to remove.

- The standard C reducer (`creduce`) may be used **if it is present as a system tool**, and is
  **never required**: adding a crate is forbidden absolutely (§7.2), so no reducer can be a
  dependency of this suite. Its absence changes one line of a manifest and nothing else — it never
  fails a run and never suppresses an artifact, because the reproducer, the exact commands, the
  captures and the fingerprint do not depend on it.
- **Reduce a copy, never the corpus program.** The corpus is read-only to the harness, and the
  program a finding came from is exercised by the whole matrix rather than by that finding alone.
- Automatic triaging and automatic reduction are complementary rather than alternatives; and a
  **hand**-minimized reproducer is often clearer to a reader than a machine-minimized one, which
  tends to strip the very names and structure that made the construct recognizable. Clarity is part
  of the deliverable.

### 5.2 Verify after curating

Two checks, both cheap, both catching a class of broken deliverable that is otherwise found by the
next reader.

**The finding still reproduces from `commands.sh` alone**, in a clean shell, with no harness, no
Cargo and no Rust toolchain on the path. If it does not, the curated directory is not a reproducer —
it is a story about one.

**`reproducer.expected` parses**, through the harness's replay loader for a finding artifact. Every
rule of the record format applies unchanged; the ones most easily broken while copying a record into
a finding directory are:

- **`program` and `area` keep the *corpus* identity** — the original program's file stem and its
  feature area — **not** the copy's own path. This is the one place the identity rule reads
  differently from the corpus, and it is worth being exact about: a curated copy's stem is
  `reproducer` and its parent directory is a finding identifier, so the path carries no corpus
  identity at all. The check is therefore **re-based rather than weakened** — the loader validates
  the declared identity against the area and program the finding was derived from. Rewriting either
  key to match the copy's path is precisely what makes the record unloadable.
- `expect_exit` is within **0–125** (the operating system truncates larger values);
- the full three-level optimization sweep is declared, and every entry of `targets` is one of the
  four accepted target names or triples;
- `shared_flags` is `-static`, which is mandatory: every artifact in the suite is linked statically,
  and a dynamically linked one could not execute under an emulator without a sysroot the suite
  deliberately does not configure;
- a restricted `targets` list, a `disabled` oracle, or any deviation from the default warning gate
  each carries a **non-empty `impl_defined_notes`** stating the reason — and a gate deviation must
  name every flag it drops, by exact spelling, in that field and no other;
- `ub_notes` is non-empty: it holds the written undefined-behaviour-freedom argument, which is what
  a reviewer reads first when a divergence appears;
- `oracle_c` is enabled — it may never be disabled — and at least one of `oracle_a` or `oracle_b` is
  enabled too, since a golden record alone cannot tell a correct answer from a wrong one that was
  already wrong when it was recorded.

The full list of hard errors the parser enforces is in
[`README.md`](README.md#every-validation-the-parser-enforces).

## 6. Where the corpus concentrates divergence surface

This section is **triage orientation, not prediction.** It does not claim these constructs are
defective; it says the corpus deliberately concentrates its cross-backend comparison surface here,
so a maintainer reading a new finding can tell immediately whether it sits somewhere the suite was
built to probe hard or somewhere unexpected. An unexpected location is itself information.

The four backends implement four different application binary interfaces, and that is where
observable behaviour has the most room to differ:

| Program | Why it is high-yield |
|---|---|
| [`04_bitfields/005_straddling_and_zero_width.c`](04_bitfields/005_straddling_and_zero_width.c) | Fields straddling a storage-unit boundary, and zero-width separators. See the measurement note below — this is the clearest case in the corpus where a cross-backend divergence is a **genuine finding** rather than an implementation-defined difference |
| [`07_variadics/006_many_args_stack_spill.c`](07_variadics/006_many_args_stack_spill.c) | Enough arguments to exhaust every argument register on every target and force stack spill |
| [`08_gcc_extensions/003_computed_goto.c`](08_gcc_extensions/003_computed_goto.c) | An indirect jump through an array of label addresses — the construct most likely to interact with indirect-branch hardening |
| [`10_declarations_and_types/005_struct_copy_and_return.c`](10_declarations_and_types/005_struct_copy_and_return.c) | By-value aggregate copy, parameter passing and return, where calling conventions differ most visibly between architectures |
| `14_abi_calling_convention/` — the whole area | The densest concentration of divergence surface in the corpus: register-exhausting parameter counts, aggregates on both sides of every by-register versus by-memory threshold, by-value aggregate return, and deep call chains forcing callee-saved spill and restore. **Planned, not present on this branch**, so it is named rather than linked |

Three more worth naming, for different reasons:

- [`01_integer_conversions/004_narrowing_conversions.c`](01_integer_conversions/004_narrowing_conversions.c)
  — narrowing at the destination range edge, in both a folded and a `volatile`-runtime variant. It
  is also the one program in the corpus that declares a warning-gate deviation, so a divergence
  here should be read together with its `impl_defined_notes`.
- [`04_bitfields/003_compound_assignment.c`](04_bitfields/003_compound_assignment.c) — compound
  assignment applied to bitfields, a construct requirement 2 names explicitly and one that combines
  a read-modify-write with a non-byte-aligned field.
- [`10_declarations_and_types/001_complex_declarators.c`](10_declarations_and_types/001_complex_declarators.c)
  — arrays of pointers to functions returning pointers and the surrounding family. This one targets
  the repository's own open risk by name (§6.2).

### 6.1 The bitfield measurement, and why it matters here

Bitfield layout looks like the archetypal implementation-defined difference, and for this corpus it
is not. Bitfield **size, alignment and the exact byte image** of a straddling three-, five- and
nine-bit sequence were **measured identical on all four targets**, as were the read-back values and
the minimum value of a narrow signed field.

Bitfields therefore need **no target restriction at all**, and the consequence for triage is direct:
a cross-backend bitfield divergence is a **genuine finding**, not something to be waved away as a
layout choice. That inversion is exactly why area `04_bitfields` earns two entries above.

### 6.2 Repository open items a finding may substantiate

Two entries in the repository's own records describe the territory this suite was built to survey. A
finding landing in either one is evidence for a question the project has already written down:

| Open item | Where |
|---|---|
| Remaining-work item *"C11 Standard Corner Case Compliance Testing"*, 5 hours, Medium priority | `docs/project-guide.md` line 111 |
| Open risk *"C11 corner case non-compliance"* — *"edge cases in complex declarators and type conversions may remain"*, status *Open — Requires targeted testing* | `docs/project-guide.md` line 248 |

Worth knowing while triaging, and easy to get wrong from memory: **no expected-divergence marker is
active anywhere in the corpus on this branch.** Constructs a reader might expect to be excused are
not — GCC case ranges carry no marker (one was minted on an inventory omission and then withdrawn
without the divergence it described ever having been observed), and neither do the wide and Unicode
literal prefixes. A divergence in any of them is therefore a **FINDING**, not an XFAIL. The analysis
behind each of those decisions is in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4.

### 6.3 The precondition: a divergence is only meaningful if the program is UB-free

**Check this before curating anything.** If a program contains undefined or unspecified behaviour, a
divergence between two compilers proves **nothing about either** — both are permitted to do anything
— and the correct action is to **rewrite the test program**, not to file a finding. A finding filed
against a defective program costs a maintainer the investigation and then teaches them to distrust
the register, which is worse than having filed nothing.

Confirm the program passed both gates, each of which is applied by the **reference compiler only**,
so `bcc` never sees a warning or sanitizer flag and the shared-flag discipline is untouched:

```text
warning gate:    -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror
sanitizer gate:  -fsanitize=undefined,address -fno-sanitize-recover=all
```

The warning gate is fixed, and a deviation may only **remove** a member of it — never add anything —
with exactly two sanctioned reductions: `-pedantic` is dropped for area `08_gcc_extensions`, because
an extension is non-standard by definition and `-pedantic` exists precisely to reject one; and
`-Wconversion -Wsign-conversion` are dropped for
[`01_integer_conversions/004_narrowing_conversions.c`](01_integer_conversions/004_narrowing_conversions.c)
alone, because a narrowing conversion is the behaviour under test rather than a mistake. `-Werror`
can never be dropped: a gate that warned without failing would be a gate in name only.

The sanitizer gate is compiled **dynamically linked** (no `-static`), **native only**, and then
**actually run** — an instrumented binary that is never executed establishes nothing. And the gate
never renders a verdict about `bcc`: a diagnostic from it means the **test program** is defective.
It is a suite-authoring gate that establishes the precondition under which a `bcc` divergence is
meaningful at all, and nothing more.

Alongside the machine half, every program carries a **written** undefined-behaviour-freedom argument
in its record's `ub_notes`. Read it first. It is the human half of the same guarantee, and it is
where an author states why the construct is safe — which is the fastest way to notice that it is
not.

**Confirm the gate was actually *performed*, not merely that it exists.** The audit enumerates the
corpus **globally**, across all fourteen declared feature areas, precisely so that a program can
never be dropped from it quietly — and three of those directories have not landed yet, so on this
branch the enumeration fails before the first program is gated and the audit is recorded
`UNPERFORMED`. While it is unperformed, no area completes, and a divergence observed under those
conditions is arithmetic that ran rather than evidence that holds. The correct response is to land
the missing material so the gate can run, not to curate a finding on an unaudited program. The audit
prints its expected and performed figures side by side and reconciles them, so a degraded run stays
visible rather than looking complete.

## 7. Provenance, constraints and maintenance

### 7.1 User-specified rules

**No user-specified rules exist for this project.** The rules document was consulted and returns a
single line reporting that no user rules were provided; that is a complete read, not a partial one.
**No rule places this file in scope** — it exists because requirement 6 requires every undocumented
divergence to be recorded with its reproducer and its exact reproduction commands, and because the
set of them has to be auditable in one place.

Their absence is **not** permission to lower the bar. The binding constraint set is the four
constraints in §7.2 plus the repository's own documented engineering standards, applied at
enterprise standard throughout. The rules document remains the authoritative source for the full
text of any rule added later.

### 7.2 Binding constraints, and what each means for findings

| Constraint | What it requires | Consequence for a finding |
|---|---|---|
| **C1 — no compiler source change** | `src/**`, `include/**`, `build.rs`, `Cargo.toml` and `Cargo.lock` are read-only reference material | No finding is ever answered with a patch (§1.1). And **this directory contains no `.rs` file at any depth** — that is precisely what keeps Cargo blind to it, so it is never a build target and the package manifest needs no change at all |
| **C2 — no existing test weakened** | No existing test may be deleted, skipped, weakened or relaxed; no `#[ignore]` attribute may be added or removed; the repository's ignored-test count stays **exactly 13** | Curating a finding adds committed data files and a table row — it touches no test and declares none, so it can move neither the repository's test count nor its ignored count. The 13 stay ignored because they are network-dependent and **C4 forbids network access**, so the two constraints agree rather than compete: re-enabling them would breach C4, and removing their attributes would breach C2 |
| **C3 — never exclude a feature because it is difficult** | If a feature cannot be tested, say so explicitly and explain why, rather than dropping it | A difficult construct is tested and its divergence recorded, not quietly left out of the corpus so the question never arises. Where a comparison genuinely cannot be made, the exclusion is narrowed to a **single oracle**, the program keeps running under the remaining oracles, and the reason is recorded in the program's own `.expected` record — never by omission |
| **C4 — contained execution** | A generated program may not reach the network or any path outside the sandbox working directory | This binds a curated `reproducer.c` exactly as it binds a corpus program: it **must not** open a socket, call `fopen`, or read `argv` or `getenv`. Every input is a literal in the source. A reproducer that read its input from outside itself would not be reproducible either, so the constraint and the deliverable want the same thing |

**Zero External Crate Dependency Rule.** Quoted verbatim from `docs/technical-specifications.md`
§0.7: *"The `[dependencies]` section of `Cargo.toml` must remain completely empty at all times"*;
*"No `[build-dependencies]` or `[dev-dependencies]` entries for external crates are permitted"*;
*"This constraint is absolute and admits no exceptions."*

The consequence for findings work is concrete: **no reducer, differ, snapshot or serialization crate
may be introduced to support it.** That is why `MANIFEST.txt` is plain text rather than anything a
serializer would emit, why `commands.sh` is a plain shell script, why the diff in `diff.txt` is
computed by the suite's own comparator, why any reducer is an optional *system* tool rather than a
dependency, and why this register is a committed Markdown document rather than a structured data
file.

**Honest measurement.** **No coverage percentage is published in this file, and none can be.**
Coverage instrumentation requires a development dependency, which the rule quoted above forbids
absolutely, so any percentage stated here would be unverifiable by anyone in this repository —
publishing one would be fabrication, and it is the same discipline that keeps §2's table empty
rather than illustrated. Where the suite's breadth has to be referenced, it is referenced as an
**enumerable matrix**, which anyone can count from the committed file set:

| Quantity | Final planned target |
|---|---:|
| Feature areas | 14 |
| Programs, each paired with its `.expected` record | 108 |
| Optimization levels per program | 3 |
| Targets per program | 4 |
| **`bcc` compile-and-run cells** | **1,296** |
| **Differential and golden assertions across the three oracles** | **≈ 3,564** |

Those are the **design target**, not a claim about what this branch has established: three of the
fourteen area directories are not present yet, and the undefined-behaviour audit gate enumerates the
corpus globally across all fourteen, so it cannot complete while any is missing. The suite's own
reports carry the present and admitted figures alongside the planned ones, and
[`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §8.2 states all three side by side with the
commands to check them. Quoting the planned figure as though it described the present state would be
exactly the unverifiable claim this paragraph refuses to make.

### 7.3 Cross-links

| Document | What it holds |
|---|---|
| [`README.md`](README.md) | The suite contract: the three oracles, the verdict taxonomy, the `.expected` record format, the environment variables, the artifact locations, and the procedure for reproducing any cell by hand |
| [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) | The companion register: **documented** divergences, the five-key marker mechanism, and the bidirectional audit that keeps markers and records from drifting apart |
| [`../conformance.rs`](../conformance.rs) | The suite driver: 14 feature-area tests and 4 infrastructure tests |
| [`../conformance_harness/`](../conformance_harness/) | The harness modules — oracle discovery, workspace isolation, record parsing, compilation, execution, comparison, classification, **finding-artifact writing** and reporting |
| `docs/testing/differential-conformance.md` (**planned**, not yet committed — named as plain text for that reason) | The documentation-site page: methodology, oracle definitions, the build matrix, the verdict taxonomy and the deliverable summary format |

Those links are relative — the first two are siblings of this file, the next two sit one level up —
so they resolve when this document is read in place. The same five, spelled from the repository root
for anyone reading an excerpt of this table out of context: this register is
`tests/conformance/FINDINGS.md`, the path the harness holds as a constant and names in every finding
verdict; its two companions are `tests/conformance/README.md` and
`tests/conformance/EXPECTED_DIVERGENCES.md`; the driver is `tests/conformance.rs` and the harness
modules are under `tests/conformance_harness/`; and the planned documentation page is
`docs/testing/differential-conformance.md`.

### 7.4 Register changelog

Recorded by **date and description**. No entry names an identifier that does not correspond to a
directory under [`findings/`](findings/), for the same reason §2 has no illustrative row: an
identifier in a register is a promise that the evidence exists.

| Date | Change |
|---|---|
| 2026-08-02 | Register created, **empty**. The curated finding set holds no directory, and the suite has recorded no undocumented divergence. The artifact contract (§3), the transient-versus-curated split (§4) and the curation procedure (§5) are established so that the first finding has a defined home and a defined shape before it is needed, rather than after |
