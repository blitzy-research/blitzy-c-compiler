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

Three verdicts sit next to one another, and telling them apart *is* the point of this file. **Two of
the three classify an observed difference; the third does not classify a difference at all.**
`FINDING` and `XFAIL` are both **divergence classifications** — a comparison ran, the two sides
disagreed, and the verdict says whether the disagreement is *explained* and by what. `FAIL` is a
different kind of thing: it is what the suite reports when it cannot honestly classify anything.

| Verdict | What it says | Where it is recorded | Fails the run? |
|---|---|---|---|
| **FINDING** | *A divergence was observed*, and it is **not** traceable to any limitation this repository documents | a self-contained artifact directory under [`findings/`](findings/), indexed here | **No** — it is a deliverable |
| **XFAIL** | *A divergence was observed*, and it **is** traceable to a limitation this repository documents, cited by a marker. Also used for a comparison the record deliberately declines to make, citing its own recorded reason | the marker in the program's own `.expected` record, mirrored in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md); or, for a declined comparison, that record's `impl_defined_notes` | No |
| **FAIL** | **Not a divergence classification.** Either a hard error — harness breakage, an internal inconsistency, a corpus defect, a record that cannot be resolved — or a comparison the corpus *requires* that could not be performed or whose result contradicts itself. Nothing was established either way | the area report and the run summary | **Yes** |

**Do not read a `FAIL` as a difference in behaviour.** It frequently arises with **no comparison
having run at all** — a `.c` file with no sibling record, a corpus that could not be enumerated, an
oracle the record *enables* while the cell reports it as excluded, or a gate whose own machinery
could not be performed. In each of those the suite has observed nothing about the compiler; it has
observed that it could not ask the question. Treating such a result as evidence about `bcc` is
exactly the mistake the three-way split exists to prevent, and curating a finding on one is the
wrong response — repair the material so the question can be asked (§6.3).

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

| ID | Slug / short title | Program (area / reproducer) | Divergence class | Oracle(s) affected | Target(s) | Opt level(s) | Artifact directory | Status | Superseded by |
|---|---|---|---|---|---|---|---|---|---|

Two notes on reading the table once it has rows.

**One row per generated finding identity — and a generated identity is exactly one cell and one
divergence class.** A run names each transient finding directory from those two things together,
injectively (§2.3), and curation carries that name forward as the `finding_id` and `identity_digest`
lines of the curated `MANIFEST.txt`. So one curated directory — and therefore one `F-NNNN-<slug>` row
— stands for exactly **one** generated identity.

**The oracle is deliberately not part of that identity, and the arithmetic follows from that.** A
divergence observed by two oracles at the same cell is **one** generated identity, one directory and
therefore **one row**: the row's `Oracle(s) affected` column lists both, and the directory's manifest
carries an `observed_by` line naming them together with one `OBSERVATION —` section apiece, so
nothing either oracle saw is lost by their sharing a directory (§2.3 gives the measurement that
settled this). What *does* multiply rows is a difference in the identity itself: two divergence
classes at one cell, or one class at two optimization levels, or one class at two targets, are
distinct identities, distinct evidence and distinct rows. The reverse also holds: every directory
under [`findings/`](findings/) must have exactly one row and every row exactly one directory, so the
index and the evidence enumerate the same set in both directions — and
`infra_expected_divergence_register` audits exactly that, in both directions, on every run.

**A row may still not stand for several generated identities.** The merge that exists is *across
oracles within one identity*, defined in §2.3 and implemented in
[`findings.rs`](../conformance_harness/findings.rs); merging two identities into one row is a
different thing and remains prohibited. `Oracle(s) affected`, `Target(s)` and `Opt level(s)` are
plural in the header because a *curated, minimized* reproducer may legitimately have been reduced
from a wider observation and the row should say what was seen — but the row's identity, and the
evidence it points at, remain the single generated identity recorded in its manifest. If a maintainer
ever wants one row to *replace* several identities, that requires a **merge schema defined here
first**: a stated rule for which fields may hold a list, a stated rule for which of the several
generated identities the curated manifest records, a stated rule for how the evidence of the others is
still reachable, and a check that audits all three. Until such a schema exists, merging identities is
prohibited — an unaudited merge silently loses the pointer to every set of evidence it did not name,
and a lost pointer is a lost deliverable.

And the columns are deliberately narrow, factual fields: the *explanation* lives in the directory's
`MANIFEST.txt`, never here, so that this file stays an index a reader can scan.

### 2.1 The `Status` vocabulary

A closed set of three values. Anything else is not a status but a comment, and belongs in the
finding's `MANIFEST.txt`.

| Status | Meaning | `Superseded by` |
|---|---|---|
| `open` | Recorded and reproducible; not yet triaged by a maintainer. This is what every finding starts as | must be `—` |
| `acknowledged` | A maintainer has confirmed the finding — read the evidence, reproduced or accepted it, and taken ownership of what happens next | must be `—` |
| `superseded` | A later finding subsumes this one; the row stays, because deleting it would erase the history a reader needs | **required**: the `F-NNNN-<slug>` identifier of the finding that replaced it |

**`superseded` is only meaningful with its replacement named, so the `Superseded by` column is
mandatory for it and must be `—` for the other two.** A row saying only that something newer
subsumed it sends a reader looking for a successor the register never identifies — which is the same
broken deliverable as a row pointing at a directory that does not exist, one step removed. Three
rules make the pointer trustworthy:

- The named identifier must be a row **in this same table**, so the successor is reachable by
  reading rather than by searching version control.
- The named identifier must **not** itself be `superseded` by the row that names it. A cycle of two
  rows each deferring to the other identifies no live finding at all.
- A superseded row keeps its own `Artifact directory` and its own evidence. Superseding is a
  statement about which investigation to read *first*, never a licence to delete the earlier
  directory — §5.2's completeness check applies to it exactly as to a live one.

Record the supersession in §7.4 by date and description as well, so the reason one finding replaced
another survives outside a single table cell.

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
with the cell's own identity and the divergence class — so that the same divergence always names the
same directory, two different divergences can never collide and overwrite one another's evidence,
and no counter is consulted. That last property is a correctness requirement rather than a
preference: the fourteen feature-area tests run concurrently in one process, so a shared counter
would be a data race *and* would make a name depend on thread scheduling. The exact spelling of the
generated name is documented once, in [`README.md`](README.md#the-finding-identifier) and
[`findings.rs`](../conformance_harness/findings.rs), and is deliberately not restated here so the
two cannot disagree.

**One divergence is filed once, however many oracles saw it.** The oracle is deliberately *not*
part of that name. A cell whose build was refused is refused for oracle (a), for oracle (b) and for
oracle (c) alike — one root cause seen through three windows — and naming per oracle gave each
window its own directory holding another copy of the same reproducer, record, commands, fingerprint
and diagnostics (measured: 33 directories for 12 divergences). The set of oracles that observed it
is recorded *inside* the manifest instead, on its `observed_by` line, and the single directory then
holds the **union** of their authority captures: more evidence, in one place, at a third of the
size.

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
FAIL rather than downgrading it to a silent pass. Seven is also the **complete** set: nothing else
belongs in a curated directory, which is what makes a stray file visible in review (§5.3).

**Every one of the seven is committed, so every one is read for disclosure before it lands.** A
transient finding directory is git-ignored and describes its machine freely; a curated one is public
and permanent. §5.3 is the mandatory review that stands between the two, and §5.2 is the check that
the artifacts describe the program actually in the directory.

**All seven are re-checked against disk immediately before a report advertises them, and a shortfall
fails the run.** Completeness is established twice, at two different instants, because they are two
different claims. The findings writer proves it at the moment of publication — that is the claim that
the directory *was* written completely. The reporting path proves it again immediately before the bytes
that name the directory are written, for both the per-area report and the run summary — that is the
claim the artifact itself makes to a reader who opens it later. Every one of the seven names is checked
without following a symbolic link, since a finding's directory name is derived deterministically from
the divergence and is therefore predictable to anything that might plant one, and `outputs/` is
additionally required to hold at least one capture: an empty captures directory satisfies every
existence test and delivers nothing. A shortfall is reported in the artifact's own diagnostics **and**
fails the area or the summary that would have advertised it, so a row reading `FINDING` can never be an
empty promise. It is a defect in the **suite**, never an observation about the compiler, so the
correction is never a compiler source change.

| Artifact | Content | Why it exists |
|---|---|---|
| `reproducer.c` | The program, **minimized as far as practical** while still provoking the divergence, with the outcome recorded in `MANIFEST.txt`'s `minimization` state (§5.1) | The divergence has to be *in* something. A reproducer no larger than the difference needs is what makes the observation readable in one sitting. "As far as practical" is a judgement about one program, so it is recorded rather than assumed: the audit refuses a directory still carrying the state a run wrote |
| `reproducer.expected` | Its expectation record, in the standard `.expected` format, keeping the **corpus** program's identity rather than the copy's path (§5.2) | Keeps the finding **runnable by the harness**, so it can be re-checked over time instead of becoming a static curiosity that nobody can tell is still true |
| `MANIFEST.txt` | The finding identifier, the affected area and program, the divergence class, **which oracles and which cells diverged**, and a one-paragraph description of the observed difference | The one file to read first. Everything else in the directory is evidence; this is the account of what the evidence shows |
| `commands.sh` | **Exact, copy-pasteable compile and run lines for every cell involved**, runnable as `sh commands.sh` from inside the finding directory, with **relative paths only** (§5.3) | This is the artifact that satisfies requirement 6's *"the exact reproduction commands"* — and it does so **without the harness, without Cargo and without a Rust toolchain** |
| `outputs/<side>-<target>-<opt>.{stdout,exit,stderr}` and, whenever there was a build, `outputs/<side>-<target>-<opt>.compile.{stdout,stderr,exit}` | The captured output from each side involved — the **program's** three streams in the first group and the **compiler's** own three in the second, all **produced from the reproducer this directory contains** (§5.2). `<side>` is `bcc` for the compiler under test or the oracle's letter `a`, `b` or `c` for the authority it was judged against (§3.1) | Evidence, byte for byte. A description of a difference is not the difference; the captures are what a second reader checks the description against. The runtime and compile groups are kept apart so that opening a `.stderr` never requires working out first whether that side's build succeeded |
| `environment.txt` | The fingerprint: the `bcc` version, the reference compiler version, each cross-driver version, each emulator version, and the kernel identification — the last written **without the host's node name** (§5.3) | Lets a divergence be attributed to **toolchain drift rather than to the compiler** — see §3.3 |

| `diff.txt` | The computed difference, with the **first divergent line and byte offset** highlighted | Turns "these two outputs differ" into "they part company here", which is where investigation actually starts |

`reproducer.c` is still a corpus program and every corpus authoring rule still binds it. In
particular it must hand-declare `int printf(const char *, ...);` and **include no header** beyond the
two sanctioned exceptions, print a fixed, deterministic, multi-line sequence, print no addresses or
pointer values, and be free of undefined and unspecified behaviour. Those rules are stated in full
under [Corpus authoring rules](README.md#corpus-authoring-rules).

An exception travels with the corpus program the reproducer was minimized from, and a reproducer may
never claim one its corpus program did not hold:

- **`<stdarg.h>`, and nothing else**, when the corpus program is in `07_variadics` — a variadic
  function cannot be written at all without `va_list` and its macros.
- **The nine required bundled freestanding headers** — `stddef.h`, `stdint.h`, `stdarg.h`,
  `stdbool.h`, `limits.h`, `float.h`, `stdalign.h`, `stdnoreturn.h` and `iso646.h` — when the corpus
  program is
  [`12_preprocessor/003_bundled_header_inclusion.c`](12_preprocessor/003_bundled_header_inclusion.c),
  the suite's dedicated probe for the bundled header set and the only program that exercises
  `include/` at all. A divergence in that probe is very likely to be *about* a header, so a reproducer
  forbidden from including one could not exhibit it. Minimization may narrow the set to the headers
  the divergence actually needs; it may not widen it, and it may not reach for a header the probe
  itself does not include.

Both exceptions, their reasons and their shared obligations are stated under
[The two sanctioned header exceptions](README.md#the-two-sanctioned-header-exceptions), and a
reproducer taking either must state the exception and its reason in its own `ub_notes` exactly as the
corpus program does. Everything else stays forbidden, `stdio.h` first among them and without
exception: `bcc` ships none, so an `#include <stdio.h>` would fail against `bcc` while succeeding
against the reference compiler — a spurious divergence caused by the test rather than by the
compiler, which is the worst possible thing to find in a finding. The bonus `stdatomic.h` is excluded
from both exceptions for the reason given there.

### 3.1 Reading an `outputs/` name

Every entry is `<side>-<target>-<opt>` plus an extension, spelled the same way in the table above:

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
incompatibility for cross-arch testing"*, `docs/project-guide.md` §6, recorded as *Partially
mitigated* with *"emulator version pinning is still recommended for stability"*. A finding that turns out to have been
an emulator upgrade is a finding correctly *closed*, and it can only be closed that way if somebody
wrote down which emulator it was.

`environment.txt` is also one of exactly two artifact classes that legitimately differ between two
runs of one unchanged divergence, and both name them:

| Differs | Why it is kept anyway |
|---|---|
| `environment.txt` | The tool versions and this run's own token are precisely what lets a later reader tell a toolchain change from a compiler change |
| The `duration_ms` field of the `.exit` and `.compile.exit` records under `outputs/` | A duration is part of a capture, and for a timeout it *is* the evidence. Timing telemetry is confined to these two records and kept out of every other artifact |

Everything else is a pure function of the divergence: for one unchanged divergence, two runs produce
the same identifier, the same file set, and byte-identical `reproducer.c`, `reproducer.expected`,
`MANIFEST.txt`, `commands.sh`, `diff.txt` and `.stdout`/`.stderr` captures. That is why the rest of
the directory can be diffed across runs and any difference read as real — and it is the same
statement [`README.md`](README.md) makes about the two-run diff, kept in step here deliberately.

**The curated copy identifies the kernel without identifying the machine.** The generated
fingerprint takes its kernel line from a full `uname -a` banner, whose second field is the host's
node name; a curated `environment.txt` must not carry it. Every fact this artifact exists for — the
kernel release and version, the machine architecture, and each tool's version — survives the
substitution, so dropping the node name costs the fingerprint nothing and is not a redaction of
evidence. The exact form to use, and the rest of the disclosure review that governs a committed
finding, are in §5.3.

## 4. The transient-versus-curated split

**Preserve this split.** It is the whole reason an in-progress run cannot pollute a committed
deliverable.

| Set | Location | Written by | Committed? |
|---|---|---|---|
| **Curated** | `tests/conformance/findings/F-NNNN-<slug>/`, indexed by this file | a **human**, after review | Yes — this is the durable deliverable |
| **Transient** | `target/conformance-findings/`, one directory per divergence of the current run — per cell and class, not per oracle (§2) | the harness, on every run | No — git-ignored, beneath the build directory |

Two further transient trees belong to the same run and are named here because a maintainer
investigating a finding will want them:

| Path | Content |
|---|---|
| `target/conformance-work/` | Per-cell workspaces. **Retained** for any cell that produced a FAIL, an XPASS or a FINDING — and for every cell when `BCC_CONFORMANCE_KEEP_WORK` is set — so a divergence leaves behind exactly the artifacts needed to investigate it; removed otherwise |
| `target/conformance-report/` | `areas/<area>.md` and `areas/<area>.tsv` per feature area, plus `summary.md` and `summary.tsv` for the run as a whole, and `findings/<finding-id>/` — the **review copy** of every finding the run recorded |

**The review copy is a third set, and it is neither of the two above.** A transient finding directory
holds the exact bytes and the runnable commands; the review copy holds the same seven artifact classes
rendered to the grade every other line of the report is held to — redacted, sanitized, bounded, with a
capture that is not text described rather than transcribed — and a `BUNDLE.txt` that states which of the
two a reader is holding. It exists because a FINDING **does not fail the run**: the cell keeps its
workspace and publishes no evidence document, and the transient root is not what an archive of a run
carries, so a passing run could name a finding whose evidence went with the machine. The copy changes
nothing about curation: it is not committed, it carries the manifest's `disclosure_review =
not-performed` unchanged, and promotion into the curated set is still the human act §5 describes. What
it changes is who can read a finding — anyone holding the report, rather than only whoever was standing
at the runner.

**The harness never writes into `tests/conformance/**`.** Not the curated findings directory, not
this register, not the expected-divergence register, not a corpus program. The corpus is opened for
reading only, every write a run performs is beneath the build directory and re-checked against that
root immediately before the bytes are published, and the report and generated-finding roots are
emptied whole at the start of every run so that one run's summary can never aggregate another's
results.

That guarantee rests on the build root being somewhere a run may legitimately empty, which is not
something the environment can be trusted to arrange: `CARGO_TARGET_DIR` is an ordinary variable, and
pointed at `tests/` it would make "beneath the build directory" and "inside the committed corpus"
the same place — at which point emptying a root whole would delete committed test material rather
than last run's output. **The build root is therefore validated before anything is created or
removed under it**: its existing prefix is canonicalized, so no symlink or `..` component can
redirect it after the check, and the result is rejected outright if it *is* the package root, if it
contains the package root, or if it is or contains the committed test tree or the curated finding
set. A build root *inside* the package is fine and is the default — `<package>/target` — because
`target` is git-ignored; what is refused is a build root that overlaps something committed. A
rejected configuration fails the run with the offending path named, rather than being silently
substituted, because a maintainer who set the variable deliberately needs to know it was refused.

**A run's finding artifacts are bounded, and the bound refuses rather than prunes.** The matrix has
1,296 cells and a compiler that diverges everywhere files a directory for every divergence, so three
ceilings apply: one artifact, one finding directory, and everything one run files — plus a ceiling
on the number of directories, set above the number of cells so a wholly-diverging run still files
all of them. The policy is deliberately the opposite of the one the per-cell workspaces use. A
retained workspace is *optional* evidence, so exceeding its ceiling prunes and says so; a finding's
artifacts **are** the deliverable, and a directory missing its captures is not a smaller finding but
one that cannot be acted on. Exceeding a findings ceiling therefore **fails that cell loudly**,
naming the ceiling and the run's totals, and writes nothing — a refused finding leaves no partial
directory behind. Every run states the accounting in `target/conformance-report/summary.md`, so a
run that came anywhere near a ceiling says so before a maintainer has to work it out from the
filesystem.

**Curating a finding is therefore a deliberate human act**, and that is the payoff: copy the
relevant artifacts out of `target/conformance-findings/` into `tests/conformance/findings/`,
minimize the reproducer, allocate the identifier, add the row here. Nothing a run does can perform
any of those steps, so **no in-progress run — and no failed, filtered or reduced run — can ever
place anything in the curated set.**

### 4.1 This file versus the run summary

They are different artifacts answering different questions, and conflating them loses one of them.

| Artifact | Scope | Lifetime |
|---|---|---|
| `target/conformance-report/summary.md` | **This run.** Its section 4 lists every finding the run recorded, with a pointer to the reproducer, to the review copy beside the summary, and to the reproduction commands, alongside the feature areas covered, the outcome tally and every expected divergence with its documented basis | Overwritten by the next run |
| This file | **Every curated finding, across runs** | Committed; durable |

A run's summary is the deliverable the requirements ask each run to emit. This register is what
survives the run — the place a reader goes to ask *"what has this suite ever found?"* rather than
*"what did it find just now?"*

## 5. Curating a finding

Ten steps. Every one is required, and skipping any of them produces a register row that cannot be
trusted.

1. **Run the suite and read the verdicts.** `cargo test --test conformance` and then
   `target/conformance-report/summary.md`; its findings section names every FINDING the run recorded
   and points, for each, at the transient artifact directory *and* at the review copy beside the
   summary. A FINDING does not fail the run, so it will not announce itself by failing — read the
   summary. Curate from the **transient directory**, never from the review copy: the copy is rendered
   for reading and makes no promise of being byte-exact, and a curated finding's captures have to be
   the bytes the programs produced.
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
   this is an expected divergence and not a finding**: add the marker's five required keys, plus
   either optional one you can honestly fill, to the program's own `.expected` record and register it
   in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) instead, then stop. Note the asymmetry
   carefully, because it is the step most easily got wrong in the other direction too: an
   **omission** from a documented inventory is *not* a documented limitation. It records that
   nothing mentions the construct, not that the implementation rejects it — so an omission leaves
   the observation exactly where it was, a finding.

   **There is exactly one exception to that asymmetry, it is narrow, and it is written down rather
   than inferred.** [`08_gcc_extensions/004_case_ranges.c`](08_gcc_extensions/004_case_ranges.c)
   carries `XD-GCCEXT-CASE-RANGES-001`, a `compile_failure` scoped `oracle_a`, whose basis is the
   compliance rule that states which GCC extensions are explicitly required — case ranges are not among
   them, so a refusal falls outside the documented obligation. It is there because the suite's frozen
   brief mandates that marker by identifier, class, scope and basis, and what the basis establishes is
   the boundary of that obligation rather than a statement that the construct is rejected. So a case-range **build refusal** triages as an expected
   divergence rather than a finding, while a case-range **wrong answer** is a `stdout_mismatch` the
   marker's class does not cover and is still a finding. For every other construct the asymmetry holds
   with no exception at all.

   Two properties of that marker are worth carrying into triage, because they are the ones most easily
   got wrong from memory. **Anticipation is refused at parse time:**
   `expected_divergence.observed` may not describe the divergence as predicted, which is a rule about
   *wording* and is what stops a marker being minted in the future tense. **A captured observation is
   optional and this marker has none:** `expected_divergence.evidence` is deliberately unwritten,
   because no `bcc` binary exists on this branch, and
   [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §2.1 and §4.1 both record why an absent capture
   does not disqualify a marker the specification mandates. The marker was retired once on twelve
   `XPASS` outcomes and **reinstated**, because that run was against a surrogate compiler under test
   which forwards to the reference driver — so the arm that agreed was the reference compiler compared
   with itself, which is not an observation about `bcc`. The full analysis, including what a stronger
   basis would look like and what result would retire the marker for good, is in
   [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4.1 and §8.3.

**The regeneration rule, stated once and applying to every curated finding.** *Minimization changes
the program, so every artifact derived from the program must be produced again from the minimized
copy — never copied forward from the transient directory.* A transient capture was produced from the
**corpus** program; once step 4 has edited a byte, that capture is evidence about a different
program than the one in the directory, and a directory whose `outputs/` describe a program it does
not contain is worse than one with no captures at all, because it reads as evidence and is not.

Concretely, after any minimization:

- **Regenerate `commands.sh`** so its compile and run lines name `reproducer.c`, and confirm it runs
  as `sh commands.sh` in a clean shell (§5.2).
- **Regenerate every `outputs/` capture by re-running those exact commands.** That means **both**
  producers on each oracle-(a) cell — the compiler under test *and* the reference compiler — and
  both their program streams and their `.compile.*` streams. Capturing one side and copying the
  other forward is the specific mistake this paragraph exists to prevent: the two would then
  describe different programs, and the diff between them would be an artifact of curation rather
  than an observation about a compiler. The same applies cell by cell to an oracle-(b) finding,
  where the baseline capture and the diverging target's capture must both come from the minimized
  program.
- **Recompute `diff.txt`** from the regenerated captures, so its first divergent line and byte
  offset refer to bytes that are actually in the directory.
- **Regenerate `reproducer.expected`'s `expected_stdout`** from the minimized program's real output,
  and re-check every other key against §5.2. A golden left over from the original program makes the
  finding unloadable or, worse, loadable and wrong.
- **Regenerate `environment.txt`** on the machine where you re-ran the commands, so the fingerprint
  describes the run that produced the captures now in the directory.
- **Re-confirm the minimized program still passes both undefined-behaviour gates (§6.3).** Reduction
  very readily introduces undefined behaviour — removing a bound, an initializer or a `volatile` is
  exactly what a reducer does — and a reduced program with undefined behaviour is not a smaller
  finding, it is not a finding at all.

If a minimized reproducer no longer provokes the divergence, **keep the verbatim copy** and record
the minimization status as verbatim in the manifest. A larger reproducer that reproduces beats a
smaller one that does not.

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

Three checks, all cheap, each catching a class of broken deliverable that is otherwise found by the
next reader. The third runs itself.

**The suite still passes.** `cargo test --test conformance infra_expected_divergence_register`
audits every directory in the curated set on every run and fails on any of these, each naming the
directory, the artifact and what to do about it — a proofreader rather than a gate to argue with:

- an artifact from §3 is **missing**, or is a symlink, which is never followed;
- the directory name and the manifest's identity **disagree** — either it is named in the curated
  `F-NNNN-<slug>` form without a `curated_id` line declaring that name, or it is named as the run
  derived it and the `finding_id` line says otherwise — which is how a copied-and-renamed directory is
  caught;
- the **register and the directory set** disagree in either direction: a row with no directory, a
  directory with no row, a row whose `Artifact directory` is not its own identifier, a status outside
  §2.1's vocabulary, a `Superseded by` pointer to an identifier that does not exist or that forms a
  cycle, or a row whose fields contradict the manifest beside it;
- the reproducer no longer matches the `reproducer_digest` recorded for it, or **any** artifact no
  longer matches the size and digest recorded for it in the manifest's `artifact =` and `capture =`
  inventory lines — which is how a **minimization, or an elision, that was not followed by refreshing
  the rest of the directory** is caught. That failure quotes both the recorded pair and the observed
  pair, so it is also the calculator for the refresh (step 6, step 7);
- a text artifact **discloses** — this machine's package or build root, a value this run treats as a
  secret, or any of the machine-independent private shapes in step 7. Every artifact is scanned,
  including recursively through `outputs/`;
- the manifest does not state **which machine produced the evidence** (`source_machine`), or does not
  record that the mandatory disclosure review of step 7 was **performed** (`disclosure_review`). A
  generated directory carries `not-performed` in that field, truthfully, so promoting one cannot skip
  the review by omission;
- the manifest does not record the **minimization outcome** (`minimization`). This is the second of the
  two curation judgements, audited in the same shape and for the same reason: a run writes
  `not-performed` because its `reproducer.c` is a verbatim copy of the corpus program, and the curated
  audit **rejects that value**, so a promotion cannot leave the judgement unmade. Replace it with
  `reduced: <method and what was removed>` or with `verbatim-by-judgement: <reason>`. Both are
  legitimate outcomes of §5.1 — the second is the ordinary one, because each corpus program already
  exercises one semantic concern — and both must carry a reason, so a bare prefix is refused. Where a
  reduction was performed, the artifact-inventory check above independently catches a directory whose
  captures were copied forward from the unreduced run.

**The finding still reproduces from `commands.sh` alone**, in a clean shell, with no harness, no
Cargo and no Rust toolchain on the path. If it does not, the curated directory is not a reproducer —
it is a story about one.

**Every artifact describes the program that is in the directory.** This is the check that catches a
missed regeneration (step 6), and it is worth running mechanically rather than by eye, because a
stale capture looks exactly like a fresh one:

- `commands.sh` names `reproducer.c` and nothing else, and every path it mentions is relative to the
  finding directory;
- re-running `commands.sh` reproduces each `outputs/*.stdout` and each `outputs/*.exit`
  **byte for byte** — for every side of every cell, not a sampled one;
- `diff.txt`'s quoted bytes and its first-divergence offset are present in the captures beside it;
- `reproducer.expected`'s `expected_stdout` equals the minimized program's real output on the
  baseline cell;
- `MANIFEST.txt`'s description describes the minimized program, and its minimization status says
  which of verbatim or minimized the directory actually holds.

A mismatch in any of these means an artifact was carried forward from the transient directory after
the program changed. Regenerate it — do not adjust the description to fit.

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

### 5.3 Disclosure review — mandatory before committing

**A curated finding is committed, public and durable; a transient one is neither.** Everything under
`target/conformance-findings/` is git-ignored and describes the machine that produced it as freely
as it likes. The moment a directory is copied into `tests/conformance/findings/` that stops being
acceptable, and there is no later step at which it can be undone: a commit is history. This review
is therefore **step 7 of §5 and is required**, not advisory, and it is required even when the
finding came from a machine the curator believes to be uninteresting.

**What the automated scan can and cannot do, and why this step exists anyway.** The audit in §5.2
scans every artifact in two halves, and the halves differ in what they *know*:

- **Byte-exact, about this machine only.** This checkout's package root, the build root, and the value
  of every environment variable this run treats as a secret are known exactly, so an occurrence cannot
  be missed and cannot be a false positive. This half can only ever apply on the machine that produced
  the artifact: on any other machine those roots are not in the text and those secrets were never in
  the environment, so it passes trivially and says nothing.
- **Machine-independent, about any host.** The *shapes* private locations and credentials take
  everywhere: an absolute path inside `/home/`, `/Users/`, `/root/`, `/tmp` or `/var/tmp`, a
  per-account temporary or runtime directory, a mounted volume, a `file://` URL, a drive-qualified
  path — and, for credentials, a PEM private-key block, a URL carrying a password in its authority, an
  HTTP authorization header, and the standardized cloud and source-forge token prefixes. This half is
  what still bites when the producing machine was somebody else's, and it reports the **shape and the
  line numbers** rather than the offending bytes, because the report is itself a file and quoting a
  home directory into it would republish the disclosure.

Two things follow, and they are why this step is a person's and not a validator's. A **host name**, an
**account name**, a **ticket number** and a **colleague's name** have no shape at all — no scan can
recognise one — and whether a given one may be published is a judgement about this project, not a
property of the text. So the judgement is recorded rather than re-derived: the manifest carries

```text
source_machine    = <digest of the producing machine's roots>
disclosure_review = clean            # or: redacted: commands.sh, outputs/a-aarch64-O2.compile.stderr
minimization      = verbatim-by-judgement: one construct, one line per property; nothing left to remove
```

The first is written by the run and says which machine-and-checkout the evidence came from, so a later
reader on a different machine can tell that only the portable half of the scan was in force. The second
is written by the run as `not-performed` — truthfully, because nothing under the build directory is
committed — and the curated audit **rejects that value**, so a promotion cannot happen without a human
replacing it with what they found. The third is the same mechanism applied to §5.1's judgement: a run
writes `not-performed`, the audit rejects it, and a curator states either `reduced: <method>` or
`verbatim-by-judgement: <reason>`. Recording both judgements in the same shape is deliberate — they are
the two things about a curated finding that no validator can determine, and gating one while assuming
the other is how an outward-facing document comes to describe a deliverable the artifact does not
match. A declared redaction is checked further: it must name at least one
artifact that exists in the directory, and it must not name a compared stream, because redacting one of
those destroys the finding rather than protecting the machine.

**Read every file in the directory, not a sample.** The seven artifacts differ in how likely they
are to carry something, and two carry it almost every time:

| Artifact | What to look for |
|---|---|
| `environment.txt` | Account names and absolute home or build paths inside a tool's **version banner**, which is whatever that tool chose to print. The kernel line is *not* a carrier: the fingerprint asks for the system, release, version, machine and operating system by name and never for the node name, so no host identity is collected in the first place — confirm that when regenerating (step 6) rather than eliding it |
| `commands.sh` | **Absolute paths** into a home directory, a workspace root, a CI checkout path, or a temporary directory whose name encodes a run or job identifier; also any tool invoked by an absolute path that reveals where it was installed |
| `outputs/*.compile.stderr` | Diagnostics quote **include paths and source paths verbatim**, and a rejected build usually quotes several |
| `outputs/*.stderr`, `outputs/*.stdout` | A corpus program prints only literals, so anything path-like here is a signal that the reproducer broke a corpus rule and should be re-read against §3 |
| `MANIFEST.txt` | Prose a curator wrote by hand — the one artifact where a ticket number, an internal host name or a colleague's name can arrive without any tool putting it there |
| `reproducer.c`, `reproducer.expected` | Comments and notes added during minimization |
| `diff.txt` | Quotes bytes from the captures, so it inherits whatever they carry |

**Three things must never be committed, in any artifact:** a credential of any kind — a token, key,
password or session identifier; personal data — an account name, a real name, an email address, or
anything else identifying a person; and an infrastructure identifier that is not needed to reproduce
the finding — a node or host name, a container or pod name, an internal address, a job or run
identifier, or an absolute path that encodes any of them.

**Redact by replacing, never by deleting, and never in a way that changes what the evidence says.**
The evidence is the deliverable, so a redaction that removes a byte a comparison depended on has
destroyed the finding to protect the machine:

- Replace a removed value with a **stable, obviously-substituted placeholder** — `<redacted-host>`,
  `<redacted-path>` — so a reader can tell that something was removed rather than that nothing was
  there. A silently deleted line reads as evidence of absence.
- **Never redact inside `outputs/*.stdout` or `outputs/*.exit`.** Those are the compared bytes: the
  divergence *is* the difference between them, and editing either makes the finding unfalsifiable.
  If a program's own stdout carries something that cannot be published, the reproducer violates a
  corpus authoring rule and the correct fix is to rewrite the program and re-capture, not to edit
  the capture.
- When a path must go from `commands.sh`, replace it with a **relative path that still runs** — the
  script has to work as `sh commands.sh` from inside the finding directory, and §5.2 re-checks
  exactly that. A redaction that breaks the script fails the check rather than passing quietly.
- Record the redaction in `MANIFEST.txt`'s `disclosure_review` line, naming every artifact it
  touched — `redacted: commands.sh, outputs/a-aarch64-O2.compile.stderr`. A reader comparing two
  curated findings needs to know that a placeholder is a curation act and not a tool's output, and the
  §5.2 audit reads this field back: a review that removed something and did not say so is reported.
- **Refresh the artifact inventory for every file you edited.** `MANIFEST.txt` records each artifact's
  size and digest, and eliding a value changes both — so the audit will report the file as stale, which
  is exactly what it should do for a file that no longer matches its record. Its failure message quotes
  the observed size and digest beside the recorded pair, so transcribing the observed one into the
  `artifact =` or `capture =` line closes the loop in a single pass; re-running the audit then confirms
  it. This is bookkeeping, not evidence: the `reproducer_digest` line still ties the whole directory to
  the program, and `disclosure_review` still states that the change was a redaction.

**The kernel line is nodename-free already, and that is a substitution made at the source.**
Identifying the host with the full `uname -a` banner would publish the node name in its second field,
so every copied line would carry it and every curator would have to remember to replace it. The
harness therefore asks for the identification by selector — system, kernel release, kernel version,
machine, operating system — and never for the node name, the processor or the hardware platform:

```sh
uname -srvmo          # what the fingerprint records; -srvm when -o is unsupported
```

Every fact the fingerprint exists for is kept — the kernel release and version, the machine
architecture — and nothing that identifies the machine is collected, which is why this is a
substitution rather than a redaction. Doing it at the source rather than at curation time is the point:
a host name is the one private identifier no validator on another machine can recognise, so the only
reliable place to deal with it is before it is written. When regenerating `environment.txt` during
step 6, **confirm** the line carries no node name rather than editing one out; if it does carry one,
the fingerprint was produced by an older harness and the whole file should be regenerated.

**Before committing, re-read the diff rather than the directory.** `git diff --cached` over the
finding directory is the last artifact anyone sees, and it is the one place a stray file — an editor
backup, a `.orig` from a failed patch, a scratch capture from a run you no longer remember — becomes
visible. §3's seven artifacts are the complete set; anything else in the directory is not part of
the deliverable and should not be committed.

## 6. Where the corpus concentrates divergence surface

This section is **triage orientation, not prediction.** It does not claim these constructs are
defective; it says the corpus deliberately concentrates its cross-backend comparison surface here,
so a maintainer reading a new finding can tell immediately whether it sits somewhere the suite was
built to probe hard or somewhere unexpected. An unexpected location is itself information.

The four backends implement four different application binary interfaces, and that is where
observable behaviour has the most room to differ:

| Program | Why it is high-yield |
|---|---|
| [`04_bitfields/005_straddling_and_zero_width.c`](04_bitfields/005_straddling_and_zero_width.c) | Fields straddling a storage-unit boundary, and zero-width separators. Bitfield layout is implementation-defined, yet the four target toolchains were **measured** to agree on size, alignment and byte image — so a cross-backend divergence here is unusually informative and must not be waved away as a layout choice. §6.1 gives the triage order, which starts with the same-target reference comparison rather than with a conclusion |
| [`07_variadics/006_many_args_stack_spill.c`](07_variadics/006_many_args_stack_spill.c) | Enough arguments to exhaust every argument register on every target and force stack spill |
| [`08_gcc_extensions/003_computed_goto.c`](08_gcc_extensions/003_computed_goto.c) | An indirect jump through an array of label addresses — the construct most likely to interact with indirect-branch hardening |
| [`10_declarations_and_types/005_struct_copy_and_return.c`](10_declarations_and_types/005_struct_copy_and_return.c) | By-value aggregate copy, parameter passing and return, where calling conventions differ most visibly between architectures |
| [`14_abi_calling_convention/`](14_abi_calling_convention/) — the whole area | The densest concentration of divergence surface in the corpus: register-exhausting parameter counts ([001](14_abi_calling_convention/001_many_integer_parameters.c), [002](14_abi_calling_convention/002_many_float_parameters.c), [003](14_abi_calling_convention/003_mixed_parameter_classes.c)), aggregates on both sides of every by-register versus by-memory threshold ([004](14_abi_calling_convention/004_small_and_large_struct_passing.c)), by-value aggregate return ([005](14_abi_calling_convention/005_struct_return_by_value.c)), and deep call chains forcing callee-saved spill and restore ([006](14_abi_calling_convention/006_nested_calls_callee_saved.c)). All six programs are committed with their expectation records — six programs, 198 comparisons |
| [`14_abi_calling_convention/006_nested_calls_callee_saved.c`](14_abi_calling_convention/006_nested_calls_callee_saved.c) | Nine nested frames, each keeping ten values live across its nested call and printing them only afterwards — the shape that detects a clobbered callee-saved register and localises it to the exact frame |
| [`14_abi_calling_convention/005_struct_return_by_value.c`](14_abi_calling_convention/005_struct_return_by_value.c) | Aggregate return by value across seven shapes and three consumption paths, in both variants — 42 shape-path-variant combinations, where the four ABIs' return mechanisms differ most |
| [`14_abi_calling_convention/004_small_and_large_struct_passing.c`](14_abi_calling_convention/004_small_and_large_struct_passing.c) | Aggregates on both sides of every target's by-register versus by-memory threshold, every member of every shape read back |
| [`14_abi_calling_convention/003_mixed_parameter_classes.c`](14_abi_calling_convention/003_mixed_parameter_classes.c) | Twenty-six interleaved integer, floating, pointer and aggregate parameters, forcing each ABI to advance both allocators independently and in step |

Three more worth naming, for different reasons:

- [`01_integer_conversions/004_narrowing_conversions.c`](01_integer_conversions/004_narrowing_conversions.c)
  — narrowing at the destination range edge, in both a folded and a `volatile`-runtime variant. It
  is the one program outside `08_gcc_extensions` that declares a warning-gate deviation — it drops the
  two conversion diagnostics, because a narrowing conversion is the behaviour under test — so a
  divergence here should be read together with its `impl_defined_notes`.
- [`13_floating_point/004_long_double_target_restricted.c`](13_floating_point/004_long_double_target_restricted.c)
  — the one program in the corpus that switches an oracle off. Oracle (b) is disabled with the measured
  reason recorded in its `impl_defined_notes`, because `long double` has no single representation across
  the four backends, so its nine cross-backend cells are reported `XFAIL` with that reason rather than
  compared. Oracles (a) and (c) are fully in force, so a divergence there is a finding like any other.
- [`04_bitfields/003_compound_assignment.c`](04_bitfields/003_compound_assignment.c) — compound
  assignment applied to bitfields, a construct requirement 2 names explicitly and one that combines
  a read-modify-write with a non-byte-aligned field.
- [`10_declarations_and_types/001_complex_declarators.c`](10_declarations_and_types/001_complex_declarators.c)
  — arrays of pointers to functions returning pointers and the surrounding family. This one targets
  the repository's own open risk by name (§6.2).

### 6.1 The bitfield measurement, and why it matters here

Bitfield layout **is** implementation-defined, and nothing in this section claims otherwise. C11
6.7.2.1p11 leaves the allocation order of bitfields within a storage unit implementation-defined,
and leaves it implementation-defined whether a field that does not fit in the remainder of a unit
straddles into the next one or moves to a fresh one. Endianness fixes the byte order of a storage
unit but not which end of it fields are allocated from, so little-endianness alone derives nothing
about a byte image, and neither does a common `int` width.

What makes the area unusually informative is a **measurement**, not a derivation. Bitfield **size,
alignment and the exact byte image** of a straddling three-, five- and nine-bit sequence were
measured **identical across all four target toolchains**, as were the read-back values and the
minimum value of a narrow signed field. That is agreement between four independently specified ABIs,
observed rather than inherited from the language — which is why the corpus applies **no target
restriction** here and leaves all three oracles enabled, and why the expectations in
[`04_bitfields/005_straddling_and_zero_width.expected`](04_bitfields/005_straddling_and_zero_width.expected)
are stated as measured target-toolchain expectations rather than as language invariants.

**The consequence for triage is computed by the harness, not left to a reader.** A cross-backend
bitfield divergence is worth investigating and must not be waved away as a layout choice — but on its
own it does not say whose fault it is, because the property the backends are being held to is an ABI
convention. Step 1 of the procedure below used to be an instruction to a maintainer; it is now
performed by the run, and its answer is recorded in the finding's own manifest:

1. **The same-target reference comparison is carried, not requested.** Every cross-backend finding
   ships the same-target reference capture beside the two captures it compared, and its manifest
   states `attribution_b` as one of `abi-observation` — `bcc` agrees with the toolchain implementing
   this target's own ABI, so the cross-target difference is a statement about two ABIs —
   `defect-candidate` — `bcc` disagrees with that toolchain too, so two independent oracles point the
   same way — or `undetermined`, when no reference driver for the target was available and the
   question is therefore not guessed at.
2. **Read the diverging target's psABI** for the bitfield allocation order and straddling rule it
   actually specifies. The ABI, not this register and not the expectation record, is authoritative
   for the target — and it is what an `abi-observation` or an `undetermined` attribution sends you to.
3. **The verdict is a FINDING in every case.** An attribution is not an excuse and never softens a
   verdict: requirement 6 makes an undocumented divergence a deliverable, so the directory, the
   reproducer and the exact commands exist whichever way the attribution came out. What it changes is
   which code a maintainer opens first.

This replaces a genuine inconsistency rather than restating a preference. The expectation record for
[`04_bitfields/005_straddling_and_zero_width`](04_bitfields/005_straddling_and_zero_width.expected)
asserted that a cross-backend divergence in its area *is* a defect rather than an
implementation-defined difference, while the procedure here made same-target agreement an ABI
observation instead. Both were reasoning correctly about the same distinction; neither decided it, so
the two could be read against each other. The decision now lives in one place — in
`findings.rs::Finding::cross_arm_attribution` — and both documents point at it.

Should a toolchain in the set ever change its bitfield layout, the measured basis stops holding and
the record's expectations must be re-derived for that target rather than read as a regression. That
possibility is why the basis is stated as measurement in the first place, and it is why area
`04_bitfields` earns two entries above.

### 6.2 Repository open items a finding may substantiate

Two entries in the repository's own records describe the territory this suite was built to survey. A
finding landing in either one is evidence for a question the project has already written down:

| Open item | Where |
|---|---|
| Remaining-work item *"C11 Standard Corner Case Compliance Testing"*, 5 hours, Medium priority | `docs/project-guide.md` §2.2 |
| Open risk *"C11 corner case non-compliance"* — *"edge cases in complex declarators and type conversions may remain"*, status *Open — Requires targeted testing* | `docs/project-guide.md` §6 |

Worth knowing while triaging, and easy to get wrong from memory: **exactly two expected-divergence
markers are active anywhere in the corpus** — the two the frozen specification fixes in advance — and
the register's own audit re-establishes both on every run, resolving two markers against two registered
identifiers in both directions.

| Marker | Program | Class / scope | What it excuses, and what it does not |
| --- | --- | --- | --- |
| `XD-GCCEXT-CASE-RANGES-001` | `08_gcc_extensions/004_case_ranges` | `compile_failure` / `oracle_a` | A **refusal** to translate the construct, on oracle (a) alone, cited to the compliance rule that fixes the GCC extensions the implementation is explicitly required to support — a set that does not include case ranges. A wrong answer from a compiler that accepts it is a `stdout_mismatch` the class does not cover — a **FINDING** — and so is any divergence on oracle (b) or (c) that is not a dependent blocked arm of the same refusal |
| `XD-TYPE-LONGDOUBLE-001` | `13_floating_point/004_long_double_target_restricted` | `comparison_excluded` / `oracle_b` | The one comparison its record declines to make. Its class names **no observation** — it is the value reserved for an arm nothing is compared on — so it can never excuse a divergence a comparison found. Its nine cross-backend comparisons report `XFAIL` as not attempted; oracles (a) and (c) judge all twelve cells and a divergence on either is a **FINDING** |

Two qualifications belong with that table rather than in a reader's memory, because both change what a
triage decision may rest on. The case-range marker's divergence has **no independent capture** — this
checkout carries no `bcc` and the compiler under test forwards to the reference toolchain, so an
agreement from it measures the environment rather than the compiler; the marker discloses that in its own
`observed` field, `evidence` is unwritten, and `XPASS` fails the run while the construct works. And the
long-double `oracle_b` narrowing is **recorded and machine-checked, not adjudicated**. Its governing
program and record have **completed their review**, which re-measured the exclusion's reason onto the
significand widths, 64 bits against 113, after the claim that the exponent ranges differ was withdrawn,
and the driver's pending declaration is empty — so for triage purposes the nine `XFAIL` arms are
settled: a divergence on oracle (a) or (c) over that program is a **FINDING**, exactly as the table says.
What the review did not and could not close is whether the cited basis *semantically supports* switching
that oracle off — a reviewer's judgement that [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §2.4.1
leaves to the reader for every marker, permanently. If you disagree with the citation, §3.2 there is the
retirement procedure; what you must not do is refile one of those nine arms as a finding, because a
not-attempted comparison observed nothing.

One construct a reader might expect to be excused is not: the wide and Unicode literal prefixes carry no
marker. So apart from a case-range refusal on oracle (a) and that one program's nine cross-backend arms,
a divergence anywhere in the corpus is a **FINDING**, not an XFAIL. The analysis behind each of those
decisions is in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4.

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
corpus **globally**, across all fourteen declared feature areas and every program in them, precisely
so that a program can never be dropped from it quietly — and it is the per-program **`.expected`
record**, not the area directory, that decides whether a program can be gated: a source without its
same-stem record leaves the warning gate undecidable, which the audit records as a **defect** rather
than passing it. All fourteen areas are present and all 108 programs are paired, so the enumeration
completes and the gate runs to its full 216 results. Were a source to land without its same-stem
record, the enumeration would fail before the first program was gated and the audit would be
recorded `UNPERFORMED` for the whole corpus, not merely for that one program.

**Do not read a performed gate as a statement about substantiation.** The two answer different
questions: this gate is satisfied by all 108 programs, and it would still be satisfied by a program
whose record's prose had not been re-checked, because a record's prose not yet having been re-checked is
a different thing from its program failing a gate. **All 108 records** have now been authored or
re-substantiated in the checkpoint sequence that produced this state; the last one outstanding was
[`13_floating_point/004_long_double_target_restricted.expected`](13_floating_point/004_long_double_target_restricted.expected),
and what remained in it was a **wording** correction — the loose *"three different formats"* claim in
its `observed` field, now *"three storage-and-format pairings over two distinct formats"* — which had to
be made in the same edit as the matching `Observed` row in
[`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4.2, because the register audit compares the two
character-for-character and correcting either one alone would fail the audit. The count framing this
distinction feeds is published in the enumerable matrix in §7.2 below.

That distinction is what to check, and it is not a historical footnote: while the audit is
unperformed, **no area completes**, and a divergence observed under those conditions is arithmetic
that ran rather than evidence that holds. The correct response is to land the missing record so the
gate can run, **not** to curate a finding on an unaudited program. The audit prints its expected and
performed figures side by side and reconciles them, so a degraded run stays visible rather than
looking complete — read that reconciliation in the run summary before curating anything.

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

Four columns, because these are four different numbers and collapsing any of them into another is the
easiest way to overstate the suite. **Structural** is what a reader can count here with a shell;
**substantiated** is the narrower milestone figure, and it now stands at all 108 records having been
authored or re-substantiated in the checkpoint sequence that produced this state; **admitted as evidence
about `bcc`** is zero, because there is no `bcc` binary on this branch.

| Quantity | Final planned target | Present on this branch (structural) | Substantiated at this checkpoint | Admitted as evidence about `bcc` |
|---|---:|---:|---:|---:|
| Feature areas | 14 | **14** | 14 | 0 |
| Sources, each paired with its `.expected` record | 108 | **108** | **108** (0 records pending) | 0 |
| Optimization levels per program | 3 | **3** | 3 | — |
| Targets per program | 4 | **4** | 4 | — |
| **`bcc` compile-and-run cells** | 1,296 | **1,296** (108 × 4 × 3) | **1,296** (108 × 4 × 3) | **0** |
| **Verdict outcome rows across the three oracles** | 3,564 | **3,564** | **3,564** | **0** |
| **Comparisons actually performed** | 3,555 | **3,555** | **3,555** | **0** |

**A verdict outcome row is not a comparison that was performed**, and the two are listed separately
because nine rows separate them. Every one of the 3,564 rows reaches a verdict and appears in the run
summary; **3,555** of them are comparisons actually made. The other **nine** are oracle (b) on
[`13_floating_point/004_long_double_target_restricted`](13_floating_point/004_long_double_target_restricted.c)
— three non-baseline targets × three optimization levels — which that record **disables** and which
`XD-TYPE-LONGDOUBLE-001` documents in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4.2. So
oracle (b) is **963 performed comparisons plus 9 not-attempted `XFAIL` rows = 972 rows**. A
not-attempted row is neither a pass nor a silent skip: nothing was compared, so no equality is claimed
— which is exactly the distinction §1.2 draws for a `FINDING` row, applied to the other end of the
verdict space. Those nine rows sit inside the substantiated column too, because the record that narrows
the oracle is itself substantiated — which is why that column reads **3,564** rows against **3,555**
performed rather than one figure for both.

**Structurally, the design target and the committed file set agree**: all fourteen area directories are
present, all 108 sources are paired with a record, and the undefined-behaviour audit — which enumerates
the corpus globally across all fourteen — performs to its full 216 gate results, so every area can
complete. Read that as the structural column and nothing more. It is **not** a claim that all 108
records have been reviewed — that is the substantiated column's question, and the two figures are
published side by side above precisely so that "structurally complete" is never read as "milestone
complete", whichever way they happen to agree today. Nor is either
column a claim about the compiler — one source landing without its record would make the audit
unperformable again and take the structural and substantiated figures down together in the same commit,
which is why [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §8.2 keeps the commands to re-count
every one of these figures rather than only the figures themselves. **Nothing in this table is evidence
about `bcc`.** The suite has been run on this branch only against a *surrogate* compiler
under test, which establishes that the machinery, the records and the golden records agree with one
another and nothing more; judging `bcc` requires the real binary and the merge described in
[`README.md`](README.md#the-cargo-integration-precondition).

### 7.3 Cross-links

| Document | What it holds |
|---|---|
| [`README.md`](README.md) | The suite contract: the three oracles, the verdict taxonomy, the `.expected` record format, the environment variables, the artifact locations, and the procedure for reproducing any cell by hand |
| [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) | The companion register: **documented** divergences, the marker mechanism — five required keys and two optional ones, the followable-citation rule and the not-anticipatory rule the parser enforces — the two markers the frozen brief names, and the bidirectional audit that keeps markers and records from drifting apart |
| [`../conformance.rs`](../conformance.rs) | The suite driver: 14 feature-area tests and 4 infrastructure tests |
| [`../conformance_harness/`](../conformance_harness/) | The harness modules — oracle discovery, workspace isolation, record parsing, compilation, execution, comparison, classification, **finding-artifact writing** and reporting |
| [`../../docs/testing/differential-conformance.md`](../../docs/testing/differential-conformance.md) | The documentation-site page, committed and published in the MkDocs navigation: methodology, oracle definitions, the build matrix, the verdict taxonomy and the deliverable summary format |

Those links are relative — the first two are siblings of this file, the next two sit one level up and
the last two levels up — so they resolve when this document is read in place. The same five, spelled
from the repository root for anyone reading an excerpt of this table out of context: this register is
`tests/conformance/FINDINGS.md`, the path the harness holds as a constant and names in every finding
verdict; its two companions are `tests/conformance/README.md` and
`tests/conformance/EXPECTED_DIVERGENCES.md`; the driver is `tests/conformance.rs` and the harness
modules are under `tests/conformance_harness/`; and the documentation page is
`docs/testing/differential-conformance.md`.

### 7.4 Register changelog

This is the changelog of **the register**, and it admits exactly two kinds of row, by date and
description:

- **A curated-finding lifecycle event** — a curated finding added to §2's table, amended, superseded
  (§2.1) or withdrawn (§2.1). §2.1 requires both a supersession and a withdrawal to be recorded here,
  so this kind cannot be dropped without breaking that requirement.
- **A change to a statement this register makes** — its establishment, or its correction — about the
  corpus, the active marker set, the triage rule or the artifact contract. Those statements are what a
  reader *acts on*: §5 step 3 decides whether an observation is a finding at all, §6.2 decides which
  marker already covers it, and §3 decides what a curated directory must hold. So when one of them is
  established or stops being true, the fact of it and its date are part of the register's evidence
  rather than an aside about it.

Everything else stays out. A change to this document's **wording** — a clearer sentence, a fixed
link, a reordered paragraph — belongs in version control and not here, because a changelog of prose
is not evidence about a compiler; what earns a row is a change to what this register *asserts*. And
no entry names an identifier that does not correspond to a directory under
[`findings/`](findings/), for the same reason §2 has no illustrative row: an identifier in a register
is a promise that the evidence exists.

The two kinds are deliberately not merged into one count. The **audit** reads §2's table and nothing
else — a finding described in a row here has been *recounted*, not *indexed* — so a reader asking what
this suite has ever found reads §2, and a reader asking how this register came to say what it says
reads the rows below.

**A row states what was true on its own date and is never rewritten afterwards**, which is what makes
a dated log worth keeping. So a marker's class, scope or status as quoted in a row below is that
row's date's answer and not necessarily today's — [§6.2](#62-repository-open-items-a-finding-may-substantiate)
carries the current marker set, and the marker blocks in the records plus
[`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4 are the authority the register's bidirectional
audit re-establishes on every run. Where a later row supersedes an earlier one, the earlier row is
left standing and the later one says so.

**The curated findings set is empty, so this log carries no curated-finding row.** The suite has
recorded no undocumented divergence, [`findings/`](findings/) holds only its placeholder, and §2's
table has no row. Every row below is therefore of the second kind — the establishment or the
correction of a statement this register makes — and each one says in its own words that the curated set
was, and remained, empty on its date. That is the distinction the paragraphs above draw: the log is not
empty, and the register still is.

| Date | Change |
|---|---|
| 2026-08-04 | Triage guidance reconciled with the corpus's marker set after a security review of the marker authorities, still **empty**: no register row was added or removed and no curated directory exists. §6.2 had recorded **one** active marker and described case ranges as unmarked; the case-range marker has been **reinstated**, so §6.2 now records **two** — `XD-GCCEXT-CASE-RANGES-001` (`compile_failure`, `oracle_a`) beside `XD-TYPE-LONGDOUBLE-001` (`stdout_mismatch`, `oracle_b`) — as a table, with the triage consequence stated exactly: a case-range **refusal on oracle (a)** is that marker's `XFAIL`, and every other divergence on that program is still a **FINDING**. The retirement it replaces had rested on a compiler under test that forwards to the reference toolchain, so its twelve agreements compared one toolchain with itself and could not establish that a real `bcc` supports the construct; the frozen specification fixes that marker for that program, and reference-versus-itself agreement is not the evidence that retires it. Two qualifications are now stated rather than left to memory: the case-range divergence has **no independent capture** (disclosed in the marker's own `observed` field, with `evidence` unwritten and `XPASS` failing the run while the construct works), and the long-double `oracle_b` narrowing is **recorded and machine-checked rather than adjudicated**, pending a review of the governing program and record. *(That last clause is superseded: the governing program and record have since completed their review, which re-measured the exclusion's reason and corrected it. §6.2 above carries the current state; the clause is left standing here because a dated row records what was true when it was written.)* §5 step 3 keeps the omission rule exactly as the contract enforces it — an omission is admitted by the parser, anticipation is refused at parse time, and triage still treats an inventory's silence as leaving *your* observation a finding — and now separates that from the case the rule does not decide: a marker the specification fixes is not an author's to mint or withdraw. |
| 2026-08-04 | Triage guidance reconciled with the corpus's marker set, still **empty**: no register row was added or removed and no curated directory exists. §6.2 had claimed **zero** active markers and described `long double` as carrying none, both of which had stopped being true; it now records **exactly one** active marker — `XD-TYPE-LONGDOUBLE-001` on `13_floating_point/004_long_double_target_restricted`, class `stdout_mismatch`, scope `oracle_b` — whose nine cross-backend arms report `XFAIL` while oracles (a) and (c) judge all twelve cells, so a divergence on either of those is still a **FINDING**. The case-range marker described in the 2026-08-02 entry below was reinstated after that entry and was **retired again** here, on the run rather than on an argument — **a retirement the entry above reversed**, because the run that produced it had no independent compiler under test: the compiler under test accepted the program, the marked arm agreed on all twelve cells, and the resulting twelve `XPASS` outcomes failed the run until the marker was withdrawn from the record and the register together. A case-range divergence of **any** class is therefore a FINDING. And §5 step 3 now states the omission rule as the contract actually enforces it: an **omission** is admitted by the record parser, which deliberately does not adjudicate a citation's strength, while **anticipation** is refused at parse time — so triage still treats an inventory's silence as leaving an observation a finding, and it is the missing observation rather than the weak citation that disqualified the case-range marker |
| 2026-08-03 | Triage guidance and the curation rule reconciled with the corpus as it now stands, still **empty**: no register row was added or removed. The corpus is complete — fourteen areas, 108 programs, 108 expectation records, no unpaired source and no orphan record — so §6, §6.3 and §7.2 no longer describe sixteen records as outstanding and the runnable count is 108, not 92. And **no expected-divergence marker is active anywhere**: the case-range marker described in the entry below was retired, because its basis was an omission and its observation was an anticipation, and the marker contract now refuses both shapes at parse time. §5 step 3 therefore states the omission rule with **no exception**, and §6.2 records zero active markers, so a case-range divergence of any class is triaged as a **FINDING**. `long double` is unchanged: a recorded oracle-(b) exclusion in its own record, never a marker. |
| 2026-08-02 | Contract corrections, still **empty**: the artifact table in §3 now spells the captured-output dimension `<side>` — matching §3.1 and the writer — and names the `.compile.*` compiler-capture group alongside the runtime one; the curated-reproducer rule in §3 now defers to the canonical README rule and restates **both** sanctioned header exceptions rather than only the variadic one; and the branch-state statements in §6 and §7.2 are aligned with the corpus as committed — all fourteen areas and all 108 programs present, sixteen records still to land, runnable count 92, and no expected-divergence marker active anywhere |
| 2026-08-02 | Register created, **empty**. The curated finding set holds no directory, and the suite has recorded no undocumented divergence. The artifact contract (§3), the transient-versus-curated split (§4) and the curation procedure (§5) are established so that the first finding has a defined home and a defined shape before it is needed, rather than after |
| 2026-08-02 | Triage guidance reconciled with the corpus. **No register row was added or removed — the curated set is still empty.** §6.2 now records that **one** marker is active — the case-range marker, class `compile_failure` — rather than none, so a case-range **build refusal** is no longer triaged as a finding, while a case-range wrong answer still is. (That marker was retired and then reinstated; see the two 2026-08-04 entries.) §5 step 3 now names that one mandated exception to the rule that an omission from a documented inventory is not a documented limitation, so the rule and the register can no longer read as contradicting each other; the rule itself is unchanged for every other observation. §3 now carries **both** sanctioned header exceptions a reproducer may inherit — `<stdarg.h>` for a variadic program, and the nine required bundled headers for the bundled-header probe — with `stdio.h` still forbidden without exception. And the present-state statements in §6, §6.3 and §7.2 now read *sixteen expectation records pending across three areas* rather than *three area directories absent*, which is what the file set holds |
