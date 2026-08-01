# Differential Conformance Suite

This suite distinguishes correct compilation from incorrect compilation by validating `bcc`'s
**observable program behaviour** against **mutually independent oracles**, over a broad,
deliberately **non-adversarial** sample of the C language. It does not search for weak spots and
it does not generate programs: the corpus is hand-authored, organised by language feature area,
and covers the language systematically rather than where problems are expected. Every program is
compiled, executed and compared — an artifact that merely built is never counted as a pass.

Run it with:

```bash
cargo test --test conformance
```

## Black-box framing

**No test in this suite imports a compiler module or inspects an intermediate representation.**
The unit under test is the `bcc` binary as a whole, exercised through its command-line interface
and judged solely by the observable behaviour of the programs it produces. Every white-box
concern — token classification, declarator parsing, SSA construction, register allocation — is
already covered by the repository's existing suites and is deliberately **not** duplicated here.

The gap this suite fills is narrower and more specific than "no execution testing". The existing
multi-architecture and hello-world suites already compile and execute `bcc`'s output on all four
targets, but they assert against **hard-coded literal expectations** for a handful of programs,
and a hard-coded expectation cannot detect a class of wrong answers it was not written to
anticipate. The existing optimization suite asserts that constant folding, dead-code elimination
and common-subexpression elimination **occur** — a property of the optimizer's implementation,
not of its correctness. Neither shape can detect a wrong-but-consistent answer. An independent
oracle can.

## The structural fact that governs this whole directory

`tests/conformance/` contains only `.c`, `.expected`, `.md`, `.h`, `.sh`, `.txt`, `.stdout`,
`.exit` and `.stderr` files — **no `.rs` file at any depth**.

That is not an incidental tidiness rule. Because there is no `.rs` file here, Cargo ignores this
directory entirely, there is no module-name collision with the sibling `tests/conformance.rs`, and
consequently **`Cargo.toml` requires no modification at all**. This is how the no-compiler-source-
change constraint (C1, below) is honoured without qualification. The rule is also enforced
mechanically rather than trusted: corpus discovery rejects any file in a feature-area directory
whose extension is neither `.c` nor `.expected`, so a `.rs` file dropped into an area is a hard
error rather than a silent change in what Cargo builds.

## User-specified rules

**No user-specified rules were provided for this project.** The project's rules document was read
in full and reports that no rules exist; that document remains the authoritative source for the
full text of any rule that may be added later, and it should be consulted rather than this section
if that ever changes.

The absence of rules is **not** permission to lower the bar. The binding constraint set for
everything in this directory is therefore:

1. the requirements' own four constraints — **C1** (do not modify the compiler's source code),
   **C2** (do not delete, skip, weaken or relax any existing test or assertion), **C3** (do not
   exclude a language feature because it is difficult), and **C4** (execute only inside the
   sandbox, with no network access and no file access outside the working directory) — each
   documented with its consequences under
   [Constraints and engineering standards](#constraints-and-engineering-standards); and
2. the repository's own documented engineering standards, chiefly the Zero External Crate
   Dependency Rule, the zero-warning quality gates, and the existing test conventions.

No rule is invented here, and no rule text is paraphrased, because there is none to paraphrase.

---

## The three oracles

Three independent oracles judge every cell. Two are mandated by the requirements; the third costs
nothing and closes a hole the other two structurally cannot see.

| Oracle | Compares | Detects | Cell volume |
| --- | --- | --- | --- |
| **(a) Reference compiler** | `bcc` against a reference C compiler, same target, same optimization level | A wrong answer `bcc` produces consistently across all four of its backends | **324** native, up to **972** cross |
| **(b) Cross-backend** | Each non-baseline target against the **x86-64 baseline**, same optimization level | A wrong answer confined to one backend — ABI, register-allocation or instruction-selection defects | **972** comparisons |
| **(c) Golden record** | Each cell against the `expected_stdout` recorded in the program's own `.expected` record | Both compilers changing behaviour in the same direction at the same time, plus toolchain drift and regression over time | **1,296** assertions |

### Oracle (a) — reference-compiler comparison

Compile each program with `bcc` and with a reference C compiler for the **same** target at the
**same** optimization level; execute both binaries; compare **stdout bytes and exit status**. Any
divergence is a compiler defect.

The native arm uses the native reference driver. Each cross arm uses the **matching cross-driver
binary** rather than a target flag, because the reference compiler has no target-selection flag at
all: the `--target=` spelling belongs to a different compiler family and was measured to be
rejected outright, and `-m32` fails on this host because the multilib start files are absent.

### Oracle (b) — cross-backend comparison

Compile the same program with `bcc` for each of the four targets; execute natively or under QEMU
user-mode emulation; compare each non-baseline target against the x86-64 baseline at the same
optimization level. Any cross-backend divergence not attributable to a documented
implementation-defined difference is a compiler defect.

**Why x86-64 is the baseline:** it is the host architecture, so its execution involves no emulator
and therefore no emulation-related variable. Comparing against a cell that was itself emulated
would put two unknowns on one side of the comparison.

**Why `bcc`'s own `--target` flag is legitimate here:** oracle (b) is `bcc`-versus-`bcc`, so
`--target` trivially satisfies the requirement that both compilers honour a flag with the same
meaning — **both sides are `bcc`**. This point is load-bearing rather than pedantic: without it,
cross-backend testing would be impossible, because selecting a target is precisely what oracle (b)
requires. The flag is used **only** within oracle (b) and never enters a shared argument set.

### Oracle (c) — golden-record regression

Assert each cell's output against the `expected_stdout` recorded in the program's own co-located
`.expected` record.

**The hole this closes is the justification for its existence.** Pure differential testing
structurally **cannot** detect **both** compilers changing behaviour in the same direction at the
same time: oracle (a) would compare two changed outputs and report agreement. Oracle (c) is the
only oracle that fails in that case, and it additionally catches toolchain drift and regression
over time. It is also what makes the requirement "source file, build commands, and expected output
recorded together" literally true, because the golden record lives in the same file as the command
templates.

For that reason the golden-record oracle may **never** be disabled in a record. A record that
tries is a hard error. A golden record alone would only assert that output has not changed since a
maintainer wrote it down, which cannot distinguish a correct answer from a wrong answer that was
already wrong when recorded — so **at least one differential oracle must also be enabled**. An
independent authority is what turns "unchanged" into "correct".

### Comparison discipline

These are hard rules, not preferences.

- **Compare stdout bytes and exit status only.**
- **Never compare standard error.** Diagnostic wording legitimately differs between compilers, and
  comparing it would generate a flood of divergences that say nothing about code correctness.
  Standard error **is** captured into finding artifacts, because diagnostic text is often the
  fastest route to a diagnosis — it is simply never compared.
- **Compare exit status using the raw wait status**, so termination by signal is distinguished from
  a normal exit rather than conflated with it. A program killed by signal 11 and a program that
  returned 11 are not the same event.
- **Expected exit codes are constrained to 0–125**, because the operating system truncates larger
  values. Measured: `return 300` was observed as status **44**. A record declaring an
  `expect_exit` outside that range is a hard error.

---

## The verdict taxonomy

The verdict space is closed and exhaustive. Every situation the harness can be in reaches exactly
one of these six verdicts; dropping a cell is not expressible.

| Verdict | Meaning | Fails the run? |
| --- | --- | --- |
| **PASS** | Every enabled oracle agreed | No |
| **XFAIL** | A divergence occurred that matches an active expected-divergence marker | No |
| **XPASS** | A marker is present but the divergence has **disappeared** | **Yes** (by default) |
| **FINDING** | An **undocumented** divergence; an artifact directory is written | No (reported as a deliverable) |
| **FAIL** | Anything unexplained | **Yes** |
| **UNAVAILABLE** | An oracle's tooling is genuinely absent from the environment | No, but reported loudly |

**There is deliberately no "skip because unsupported" verdict.** An absent oracle is reported as
UNAVAILABLE — loudly, and in the summary — **never as a silent pass**. That is what keeps a modest
environment from masquerading as a passing run. Under `BCC_CONFORMANCE_STRICT` an UNAVAILABLE
becomes a failure; see [Graceful degradation](#graceful-degradation).

The four verdicts permitted in a passing run are PASS, XFAIL, FINDING and UNAVAILABLE, and each is
still reported in full. A finding does not fail the run because a finding is a deliverable; an
unavailable oracle does not fail the run by default because the environment, not the compiler, is
what is incomplete.

### The six divergence classes

These identifiers are the legal values of `expected_divergence.class`. Spell them exactly.

| Class | Meaning |
| --- | --- |
| `compile_failure` | One compiler rejected a program the other accepted |
| `link_failure` | The program translated but did not link |
| `run_crash` | The program died on a signal instead of exiting |
| `exit_code_mismatch` | Two completed runs disagreed on status |
| `stdout_mismatch` | Two completed runs disagreed on bytes — the ordinary shape of a wrong answer |
| `timeout` | Execution outlived its per-cell budget |

A `link_failure` that is **attributable to this machine** — a target's cross C runtime is absent —
is reported as UNAVAILABLE at environment scope and never as a finding against the compiler.

**A timeout is a first-class divergence class, not an infrastructure error.** A program that
terminates promptly under one compiler and hangs under another is precisely the kind of defect
this suite exists to surface, so it is classified and reported like any other divergence.

### The unexpected-success policy

If a marker is present but the divergence it describes has disappeared, the verdict is **XPASS and
the run fails by default**, following the standard xunit convention that unexpected success is a
failure.

The justification is asymmetric cost. A stale marker is stale documented knowledge that will
mislead the next reader; retiring it is a trivial test-only edit. Failing loudly is therefore the
cheaper mistake to make.

`BCC_CONFORMANCE_ALLOW_XPASS` exists for a marker-retirement transition window and downgrades
XPASS from a failure to a warning. Either way XPASS is **always** listed separately and
prominently in the summary, so it can never pass unnoticed, and every XPASS detail names the
marker and says where to retire it: the program's own `.expected` record **and**
[`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md), because a marker retired in one place and
left in the other is still stale documentation.

### Marker scope matching is strict

A marker excuses a divergence only when its scope covers this **oracle**, this **target** and this
**optimization level**, *and* its class equals the class observed. A marker scoped to one oracle
does not excuse a divergence on another; a `compile_failure` marker on oracle (a) must not absorb a
`stdout_mismatch` on oracle (b), because that would launder a genuine second defect into an
expected divergence. When a marker exists but does not cover the observation, the verdict falls
through to FINDING and the detail states exactly which dimension failed to match.

**A marked program still compiles and still runs.** A marker changes how a divergence is
*classified*, never whether the feature is *exercised*. Nothing in the harness can short-circuit
execution because a marker exists: classification is reached only after the cell has been
compiled, run and compared.

A recorded exclusion — a program whose own record narrows which oracles judge it — is likewise an
**XFAIL**, not an UNAVAILABLE. The distinction matters: UNAVAILABLE means the environment could
not attempt the comparison, whereas a recorded exclusion is a deliberate, reasoned decision
committed to the repository.

---

## The `.expected` record format

Every program has a sibling `<program>.expected` record. This single file is what makes each test
reproducible in isolation and what supplies oracle (c). This section is **binding on all 108
records** and matches the hand-written parser (`manifest.rs`) in
[`../conformance_harness/`](../conformance_harness/) exactly. A maintainer authoring a new program
should copy from here.

### Grammar

The format is line-oriented and is parsed with no serialization crate.

- **Comment** — a line whose first non-whitespace character is `#`. This applies **only outside a
  heredoc**. Inside a heredoc, `#` is **data**.
- **Scalar entry** — `key = value`, split on the **first** `=`; both sides trimmed.
- **Heredoc entry** — `key <<END`, then body lines, terminated by a line that is **exactly** `END`.
  Inside a heredoc **nothing is trimmed**, `#` is not a comment and `=` is not a separator, which is
  what lets a golden stdout reproduce program output byte for byte including leading spaces and
  lines that begin with `#`.
- A line opens a heredoc when `<<` appears **before** any `=`, so a scalar value may contain `<<`
  and an opener needs no `=`.
- Keys are **case-sensitive** and drawn from `[a-z0-9_.]`; the dot serves the
  `expected_divergence.*` family.
- Lines are split on any standard line break, so a record saved with CRLF endings parses
  identically to one saved with LF.

**The trailing-newline convention is the format's highest-risk detail.** A heredoc value is the
concatenation of its body lines **with a newline appended to each**, so a body of two lines parses
to a value ending in exactly one `\n`, and an empty body parses to the empty string. The rule runs
in this direction so that a captured stream — which always ends in a newline, since every corpus
program's last `printf` emits one — is byte-identical to the parsed value, and so that the
maintenance regeneration script can write a stream verbatim between the opener and `END` with no
fix-ups. A stream that does not end in a newline cannot be represented at all; that is a stated
limitation rather than a silent truncation, and a program producing one is outside the corpus by
construction.

### Hard parse errors

Each of the following **aborts the run**. Nothing is dropped silently, because a quietly ignored
`expected_stdout` would turn oracle (c) into a no-op and a quietly ignored obligation would let
coverage narrow without the recorded reason that makes the narrowing reviewable.

- An **unterminated heredoc** — end of file reached with a heredoc still open. A body line whose
  *trimmed* text is `END` but which is not exactly `END` is reported at that line rather than left
  to surface at end of file, because invisible trailing whitespace is hard to see in an editor.
- A **duplicate key** — a hard error rather than last-one-wins, because two values for one key mean
  a partly edited record and there is no way to tell which was intended.
- An **unknown key** — rejected rather than ignored, because an unrecognised key is almost always a
  typo, and a typo that is ignored silently disables the check the key exists to perform.
- A **malformed line** — a line that is neither blank, nor a comment, nor `key = value`, nor a
  `key <<END` heredoc opener.
- An **empty element in a comma-separated list** — rejected rather than dropped, because dropping it
  shrinks the matrix or widens a marker scope without saying so.
- A **control character** anywhere in a value.

### The complete legal key set

Nothing outside this set may appear in any record. The order below is the order the reference
record writes them.

| Key | Kind | Presence |
| --- | --- | --- |
| `program` | scalar | required |
| `area` | scalar | required |
| `description` | scalar | required |
| `targets` | scalar | required |
| `opt_levels` | scalar | required |
| `shared_flags` | scalar | required |
| `bcc_command` | scalar | required |
| `ref_command` | scalar | required |
| `run_command` | scalar | required |
| `expect_exit` | scalar | required |
| `oracle_a` | scalar | required |
| `oracle_b` | scalar | required |
| `oracle_c` | scalar | required |
| `ub_audit_flags` | scalar | optional |
| `ub_notes` | heredoc | **required, non-empty** |
| `impl_defined_notes` | heredoc | conditional |
| `expected_stdout` | heredoc | **required** |
| `expected_divergence.id` | scalar | optional — all five marker keys or none |
| `expected_divergence.class` | scalar | optional — all five marker keys or none |
| `expected_divergence.scope` | scalar | optional — all five marker keys or none |
| `expected_divergence.basis` | scalar | optional — all five marker keys or none |
| `expected_divergence.observed` | heredoc | optional — all five marker keys or none |

### The canonical reference record

This is the exemplar every new record is derived from.

```text
program            = 004_narrowing_conversions
area               = 01_integer_conversions
description        = Narrowing integer conversions across signedness, folded and runtime variants
targets            = x86_64, i686, aarch64, riscv64
opt_levels         = -O0, -O1, -O2
shared_flags       = -static
bcc_command        = $BCC --target <triple> <opt> -static <src> -o <out>
ref_command        = $REF_CC_<TRIPLE> <opt> -static <src> -o <out>
run_command        = <runner> <out>
expect_exit        = 0
oracle_a           = enabled
oracle_b           = enabled
oracle_c           = enabled
ub_audit_flags     = -Wall -Wextra -pedantic -Wshadow -Werror
ub_notes           <<END
All conversions are value-preserving or explicitly defined: unsigned narrowing is modular,
signed narrowing operands are within the destination range. No signed overflow, no shift
out of range, no aliasing violation, no uninitialized read.
END
impl_defined_notes <<END
Plain char is not used. All widths are fixed-width types or int. No pointer values printed.
END
expected_stdout    <<END
narrow_u8=200 narrow_i8=-56 narrow_u16=65336
runtime_u8=200 runtime_i8=-56 runtime_u16=65336
END
```

### The optional marker block

Append all five keys, or none of them, to the record above.

```text
expected_divergence.id       = XD-GCCEXT-CASE-RANGES-001
expected_divergence.class    = compile_failure
expected_divergence.scope    = oracle_a; all targets; all opt levels
expected_divergence.basis    = docs/project-guide.md, extension inventory omits case ranges
expected_divergence.observed <<END
bcc: error at case label range; reference compiler: compiles and prints range_hits=5
END
```

### Every validation the parser enforces

Each item below is a **hard error**.

**Identity**

- `program` **must equal the file stem**. This is the cheapest guard there is against a record
  copied from another program and only partly edited.
- `area` **must equal the containing directory name**, and must name a real feature area. The area
  determines which report file the program's verdicts are written to.

**Matrix**

- `targets` — the short names `x86_64`, `i686`, `aarch64`, `riscv64` **and** the full triples
  `x86_64-linux-gnu`, `i686-linux-gnu`, `aarch64-linux-gnu`, `riscv64-linux-gnu` are both accepted.
  Anything else is rejected.
- `opt_levels` — the `-O0` and `O0` spellings are both accepted. The **full three-level sweep must
  be declared**; a duplicate level and an empty list are both rejected. **`-O3` and `-Os` are
  rejected**, because `bcc` documents no support for them and `docs/technical-specifications.md`
  §0.6.2 places them out of scope ("Only `-O0`, `-O1`, `-O2` are in scope"). Reducing the sweep for
  fast local iteration is a run-time decision, stamped on the report as reduced coverage; it is
  deliberately not something a committed record may do on its own authority.
- `expect_exit` must be **0–125**.

**Flags**

- `shared_flags` must be a subset of the verified shared set and contain nothing forbidden.
  **In practice: use `-static` only.** Specifically:
  - `-static` is **mandatory** — every artifact in the corpus is linked statically, and a record
    omitting it would describe a dynamically linked artifact whose emulated cells could not execute
    without a sysroot the suite deliberately does not configure.
  - **No flag may take a value.** A value is a path or a macro definition, and both are outside a
    data file's authority. Both the bare spelling (which would consume whatever argument the
    harness appends next) and the attached spelling (which could name an escaping path outright)
    are refused.
  - What remains is a set of switches: the optimization levels, debug information, static linkage
    and position independence.
  - `-o` and `-c` belong to the templates and are rejected here. An **optimization level is also
    rejected here**, because it is supplied per cell through the `<opt>` placeholder and naming one
    here would collapse a program's three cells into one.
  - Duplicates are rejected.
- `ub_audit_flags`, when present, must be a **removal** from the fixed seven-flag warning gate — see
  [The undefined-behaviour audit gate](#the-undefined-behaviour-audit-gate). It must be non-empty,
  free of duplicates, must retain `-Werror`, must actually differ from the default gate, and what
  remains must be exactly one of the two sanctioned reductions. Because a deviation may only remove
  a member of a fixed set, no compiler option outside the gate can be introduced through it — not a
  suppression such as `-w` or `-Wno-error`, and not an option that changes include search,
  specification files, plugins, wrappers or output paths.

**Oracles**

- `oracle_a` / `oracle_b` / `oracle_c` must each be exactly `enabled` or `disabled`.
- **`oracle_c` may never be disabled.** It applies to every cell of every program and is the only
  oracle that detects both compilers changing behaviour in the same direction.
- **At least one differential oracle — `oracle_a` or `oracle_b` — must be enabled**, because a
  golden record alone cannot distinguish a correct answer from a wrong answer that was already
  wrong when it was recorded.

**Recorded reasons**

- **A restricted `targets` list REQUIRES a non-empty `impl_defined_notes`.**
- **A `disabled` oracle REQUIRES a non-empty `impl_defined_notes`.**
- **A `ub_audit_flags` deviation REQUIRES its reason recorded in `impl_defined_notes`** — a
  deviation without a recorded reason is itself a defect in the test. Note the split carefully,
  because it is easy to get wrong: `impl_defined_notes` is where the parser looks for the reason
  behind *any* narrowing of coverage — a restricted target list, a disabled oracle, or a gate
  deviation — while `ub_notes` is separately required of **every** record and holds the written
  undefined-behaviour-freedom argument. A record that narrows anything and carries no
  `impl_defined_notes` will not load.
- `ub_notes` is required and must be non-empty in every record without exception.

**Marker block**

- **A partial `expected_divergence.*` block is a hard error** — all five keys or none.
- `expected_divergence.class` must be one of the six class identifiers listed above.
- `expected_divergence.scope` is structured. Clauses are separated by `;`. A clause either names a
  whole dimension — `all oracles`, `all targets`, `all opt levels` — or lists members of exactly one
  dimension as a comma-separated list. An empty clause is rejected at its position rather than
  filtered away.
- `expected_divergence.basis` **must begin with a repository-relative file path**, then `, `, then
  the section or description that authorises the marker — **and the cited file must exist on disk.**
  This is machine-verified by `infra_expected_divergence_register`. The path must be relative and
  must not climb out of the repository; a whole-document citation with no section is rejected,
  because a citation a reader cannot check is not a basis.

### The command templates and their placeholders

| Placeholder | Expands to |
| --- | --- |
| `$BCC` | the `bcc` binary under test |
| `$REF_CC_<TRIPLE>` | the reference driver matching the cell's target |
| `<triple>` | the cell's target triple, e.g. `aarch64-linux-gnu` |
| `<opt>` | the cell's optimization level, e.g. `-O2` |
| `<src>` | the program's `.c` file |
| `<out>` | the artifact path |
| `<runner>` | empty on the natively executing target, otherwise that target's emulator |

**`compile.rs` cross-checks the assembled argument vector against the rendered template and treats
a mismatch as a hard error**, so the templates must be written exactly as in the canonical record.
A record does not merely mention its command lines; it asserts that those lines rebuild and rerun
any cell of its matrix, so the templates are validated structurally:

- `bcc_command` must select the cell's target with `--target <triple>`, or a record claiming four
  targets would record one line that builds only the default.
- **Both** build lines must carry `<opt>`. A literal level written in its place is rejected, because
  a record claiming a three-level sweep would then record one line that builds only one level.
- Both build lines must write `-o <out>`, with the placeholder **immediately after** the flag, and
  must carry `-static`.
- `ref_command` must name a **per-target** driver. The bare `$REF_CC` spelling names no target and
  is rejected, because on the reference side the driver binary *is* the target selection.
- `run_command` must be **exactly** `<runner> <out>`, or a cross-target cell would be recorded as
  executing a foreign binary directly instead of through its emulator.
- Every flag either build line passes must appear in `shared_flags`, and every flag `shared_flags`
  declares must appear in both build lines. The first direction stops an unverified flag being
  smuggled past the shared-flag discipline by writing it into a template; the second stops a record
  documenting an invocation that never happens.
- A template is an **argument vector, not a shell script**: the harness executes it with no shell
  involved. Shell metacharacters in literal template text are refused at parse time rather than
  quoted at emit time, because quoting would faithfully reproduce a command line nobody meant to
  write. A `$` or an angle bracket is recognised only as part of a **whole-token** placeholder;
  anywhere else it is either a shell expansion or a misspelled placeholder, and both are refused.

### Corpus discovery

Programs are found by a **sorted scan of `<area>/*.c`**. The scan is strict, and nothing that could
carry a program is passed over silently:

- A `.c` file with **no sibling `.expected` is a hard error** — never a skip. The pairing is what
  makes "source file, build commands, and expected output recorded together" true.
- An `.expected` record whose stem matches **no** program in the area is also a hard error: it means
  a program was deleted, renamed, or the record was misnamed at birth.
- An **empty or missing area directory is a hard error**.
- A feature-area directory may contain **only** `.c` and `.expected` files. An unrecognised
  extension, a dot-prefixed entry (other than `.gitkeep`), a **nested directory** and a **symbolic
  link** are each a hard error. A dot-prefixed entry is rejected rather than skipped because a
  blanket skip would mean that renaming `007_x.c` to `.007_x.c` silently removed twelve cells from
  the matrix while the run still reported success.
- Every path the parser reads must resolve to a regular, non-symbolic-link file **beneath this
  corpus directory**. A record dictates what gets compiled and what counts as correct, so one read
  from outside the corpus would decide both while every report still showed a corpus path.

The corpus's genuine companions — `support/`, `tools/`, `findings/` and the three Markdown files —
are **siblings** of the area directories rather than children, so nothing legitimate is displaced by
these rules.

### Read-only, and the golden-record rule

`manifest.rs` is **read-only with respect to the corpus**: there is deliberately no writer, fixer or
update-in-place helper anywhere in the harness.

**`expected_stdout` is regenerated only through
[`tools/regenerate_expected.sh`](tools/regenerate_expected.sh), never automatically during a test
run**, so a wrong answer can never quietly become the new expectation.


---

## Corpus authoring rules

**These are not guidelines.** A program that violates any of them produces divergences that say
nothing about the compiler, which is worse than having no program at all — it costs a maintainer
the time to investigate and then teaches them to distrust the suite.

### Headers: hand-declare, do not include

**Hand-declare `int printf(const char *, ...);` and include no header.**

The reason is not obvious, so it is worth stating plainly. `bcc` bundles only nine freestanding
headers — `stddef.h`, `stdint.h`, `stdarg.h`, `stdbool.h`, `limits.h`, `float.h`, `stdalign.h`,
`stdnoreturn.h`, `iso646.h` (plus a bonus `stdatomic.h`) — and ships **no `stdio.h`**. See
`docs/technical-specifications.md` line 19 and lines 202–214; a grep for "stdio" across `docs/`
returns **zero** matches. An `#include <stdio.h>` would therefore **fail against `bcc` while
succeeding against the reference compiler** — a spurious divergence caused by the test, not the
compiler.

Independent corroboration: published output-comparison experience identifies a missing `printf`
prototype or header as the **most common** portability problem in this class of suite.

**Include no shared project header either.** Each test must be reproducible from a program plus its
`.expected` record alone; a shared header would add a second file and a correct include path to
every reproducer. Self-containment beats reuse here, deliberately. This is also enforced
mechanically: a feature-area directory may contain nothing but `.c` and `.expected` files, so a
header cannot be placed beside the programs at all.

#### The one sanctioned header exception

- **Scope:** the programs in area `07_variadics`, and
  `12_preprocessor/003_bundled_header_inclusion.c`. Those programs — and **only** those — may
  `#include <stdarg.h>`.
- **Reason:** a variadic function cannot be written at all without `va_list`, `va_start`, `va_arg`,
  `va_end` and `va_copy`. Variadic functions are explicitly mandated, and constraint C3 forbids
  dropping a feature because it is difficult. `stdarg.h` **is** in `bcc`'s bundled set
  (`docs/technical-specifications.md` line 205) **and** is a freestanding header provided by the
  reference compiler, so it compiles identically under both oracles and the program remains a
  single-file reproducer.
- **Obligation:** every program taking this exception must state the exception **and its reason** in
  its `ub_notes`.
- **No other header is permitted anywhere in the corpus.** In particular: no `stdio.h` (`bcc` has
  none), no `wchar.h`, no `uchar.h`, no `string.h`, and **avoid `stdatomic.h` entirely** — it is not
  mandated by any requirement and atomics can require `-latomic`, which is not in the shared flag
  set.

### The two-variant rule

This is the most consequential authoring rule in the suite. **Every arithmetic, conversion and
bitfield program must contain both a compile-time-constant variant and a runtime variant with
`volatile`-qualified operands.**

The instruction-level evidence: `volatile int x = 7; return x * 6;` at `-O2` emits a **genuine
runtime multiply**, whereas the non-`volatile` form folds to a single immediate `mov $0x2a`.

Without the runtime variant, optimization silently substitutes the constant folder's answer for the
backend's, and a **code-generation defect escapes detection entirely** — the suite would report
PASS while never having asked the backend to compute anything. This is the mechanism by which
"behaviour at multiple optimization levels" actually discriminates, rather than merely running the
same folded constant three times.

### Determinism

Byte-exact comparison is only meaningful if output is deterministic.

- **Print a fixed, deterministic, multi-line result sequence — one line per semantic property
  claimed.** Never a single aggregate value: a single divergent line must localize the defect to
  **one construct**, which is what makes minimization tractable when a finding is raised.
- **Never print an address or a pointer value.** Express pointer facts only as **differences,
  comparisons and alignment residues**.
- **Never print a plain-`char` signedness-dependent value.** Signedness was **measured**: signed on
  x86-64 and i686, **unsigned** on AArch64 and RISC-V 64. Use explicit `signed char` or
  `unsigned char`.
- **Normalize type widths.** `sizeof(long)` and `sizeof(void *)` were measured as **4 on i686** and
  **8 on the other three targets**. Use fixed-width types or `long long`, or restrict the target
  list and record the restriction. The same hazard applies to `size_t`, `ptrdiff_t` and `intptr_t`.
- **Print floating-point values at fixed precision with margin.** Validated by measurement: the same
  results printed byte-identically across all twelve target-and-optimization configurations under
  this rule.
- **Keep expected exit codes within 0–125** (`return 300` was measured as status 44).
- **No timestamps, no randomness, no uninitialized reads, no locale-dependent formatting, and fixed
  iteration order.**

### The undefined-behaviour-freedom rulebook

Every program must satisfy all of these:

- no signed overflow;
- shift counts strictly within range;
- no aliasing violations;
- no reads of uninitialized storage;
- one-past-end pointers **may be formed but never dereferenced**;
- no object modified twice between sequence points;
- at most one side-effecting argument per call;
- no dependence on padding bytes or on the relative addresses of unrelated objects.

**Why undefined-behaviour freedom is a precondition rather than a nicety:** if a program contains
undefined behaviour, a divergence between two compilers proves nothing about either, because both
are permitted to do anything. Undefined-behaviour freedom is what makes the oracle **sound**. It is
the precondition under which a divergence can be read as evidence about a compiler at all, which is
why it is machine-enforced rather than asserted — see the next section.

---

## The undefined-behaviour audit gate

Every program passes through two gates, and `infra_ub_audit_gate` runs them.

### The warning gate

```text
-Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror
```

A record that omits `ub_audit_flags` accepts this default, and that is the normal case.

**There are exactly two sanctioned deviations**, each of which must record its reason in
`impl_defined_notes`:

| Deviation | Where | Why |
| --- | --- | --- |
| Drop `-pedantic` | area `08_gcc_extensions` only | An extension is non-standard by definition, and `-pedantic` exists precisely to reject one |
| Drop `-Wconversion -Wsign-conversion` | the deliberate narrowing-conversion programs | A narrowing conversion is the **behaviour under test**, not a mistake, and those diagnostics exist to catch an accidental one |

A deviation is expressed as a **removal from the fixed gate, never as an addition**. `-Werror` can
never be dropped: a gate that warned without failing would be a gate in name only, and every
remaining diagnostic would become advice.

### The sanitizer gate

```text
-fsanitize=undefined,address -fno-sanitize-recover=all
```

Compiled **dynamically linked** (no `-static`), **native only**, and then **actually run**. Every
program must be clean under both UndefinedBehaviorSanitizer and AddressSanitizer.

### The two facts most often misunderstood

- **Both gates are reference-compiler-only.** `bcc` never sees a warning flag or a sanitizer flag,
  so the shared-flag discipline is never touched. Sanitizers are also documented as out of scope for
  `bcc` (`docs/technical-specifications.md` §0.6.2: "Sanitizers (ASan, TSan, UBSan) — Not
  supported"), which is a second, independent reason they belong only to the audit.
- **The sanitizer gate never renders a verdict about `bcc`.** A diagnostic means the **test
  program** is defective and must be rewritten. It is a suite-authoring gate that establishes the
  precondition under which a `bcc` divergence is meaningful at all — nothing more, and nothing
  less.

The gate genuinely bites rather than decorating: it rejected the author's own probe program on a
real diagnostic during design. The volume is 108 programs × 2 gates = **216 audit invocations**.

Alongside the machine half, each program carries a **written** undefined-behaviour-freedom argument
in its `ub_notes`. The gates are the machine half of that guarantee; `ub_notes` is the human half,
and it is what a reviewer reads first when a divergence appears.

---

## Flag discipline

Only flags that both compilers honour with the same meaning may be passed identically to both. That
is verified rather than assumed: `infra_flag_capability_probe` asserts an **observable consequence**
per flag, not mere acceptance, and asserts **negatively** that no non-shared flag has leaked into
the shared set.

### The flags actually used

Deliberately minimal and unimpeachable: **`-o`, one of `-O0`/`-O1`/`-O2`, and `-static`.**

### The verified shared set

Accepted by both compilers with identical meaning:

```text
-o  -c  -O0  -O1  -O2  -I  -D  -U  -L  -l  -g  -static  -fPIC
```

These are verified so that maintenance has a **proven envelope to work within**, not because the
suite currently needs all of them. `-L` and `-l` are checked for acceptance only: verifying them
semantically would need an archive-creation tool, and the corpus links nothing but the C runtime,
which is linked by default. That limitation is stated rather than glossed over.

### Prohibitions

- **Never pass a `-std` flag.** `bcc` has none, and the reference compiler's default mode was
  measured as **gnu17** (`__STDC_VERSION__ = 201710L`, no `__STRICT_ANSI__`), which already enables
  the GNU extensions the corpus exercises.
- **Never pass a reference-compiler-only flag in a differential invocation:** `-O3`, `-Os`, any
  `-std=` form, `-pedantic`, `-Wall`, `-Wextra`, `-Werror`, `-Wconversion`, `-Wsign-conversion`,
  `-Wshadow`, `-m32`, `-S`, `-E`, `-fwrapv`, `-fno-strict-aliasing`, any `-fsanitize=` form,
  `-fno-builtin`, `-ffreestanding`, `-nostdlib`. The warning and sanitizer flags belong exclusively
  to the audit gate; `-S` and `-E` stop before a runnable artifact exists, leaving all three oracles
  nothing to compare; the rest redefine the language or the runtime the corpus is written against,
  so the compared programs would no longer be the same program.
- **Never pass `-fcf-protection`**, even though **both** compilers accept it: their **default scopes
  differ**. This is the case that proves flag verification had to be **semantic, not syntactic** —
  acceptance alone would have been a false positive, and a syntactic check would have admitted it.
- `-mretpoline`, `--target` and `--sysroot` are `bcc`-only spellings and are excluded from
  oracle (a) entirely. Target selection is used **only** within oracle (b), where both sides are
  `bcc`.

### Why every artifact is built with `-static`

Two independent reasons:

1. It is the one linkage mode both compilers spell identically, so it survives the shared-flag
   discipline.
2. A statically linked binary is self-contained, so QEMU user-mode execution needs **no sysroot and
   no dynamic-loader configuration**. Verified for all four targets.

### Why oracle (a)'s cross arm uses a cross driver

The reference side selects a target by choosing a **different driver binary**, not by passing a
flag. The reference compiler has no target-selection flag: the `--target=` spelling belongs to a
different compiler family and was measured to be **rejected**, and `-m32` fails on this host because
the multilib start files are absent. That is why i686 coverage comes from the i686 cross driver
rather than from `-m32`.


---

## Targets, runners and the build matrix

| Target | Triple | Pointer | `long` | ELF class | Endianness | Runner |
| --- | --- | --- | --- | --- | --- | --- |
| x86-64 | `x86_64-linux-gnu` | 8 | 8 | ELF64 | little | native (no emulator) |
| i686 | `i686-linux-gnu` | 4 | 4 | **ELF32** | little | `qemu-i386-static` |
| AArch64 | `aarch64-linux-gnu` | 8 | 8 | ELF64 | little | `qemu-aarch64-static` |
| RISC-V 64 | `riscv64-linux-gnu` | 8 | 8 | ELF64 | little | `qemu-riscv64-static` |

Sourced from `docs/technical-specifications.md` lines 457–462.

**CRITICAL:** the i686 runner is **`qemu-i386-static`**. The emulator's architecture name is
**i386**, so this is the one runner that is *not* named after its target: do not derive the runner
name from the `i686` in the triple. Source: `docs/project-guide.md` lines 420, 424, 428 and 568.
Getting this wrong makes the entire i686 arm silently unavailable.

The harness probes **both** the plain and the `-static` spelling of each emulator, because the
requirements name `qemu-aarch64` and `qemu-riscv64` while this environment ships only the `-static`
variants. Either packaging works with no configuration.

**Optimization matrix:** exactly `{-O0, -O1, -O2}`. No other level is in scope.

### The enumerable matrix

| Quantity | Count |
| --- | --- |
| Feature areas | 14 |
| Programs | 108 |
| Optimization levels per program | 3 |
| Targets per program | 4, unless the record restricts them with a recorded reason |
| **`bcc` compile-and-run cells** | **1,296** (108 × 4 × 3) |
| Reference cells, native | **324** (108 × 3) |
| Reference cells, cross | up to **972** (108 × 3 × 3) |
| Oracle (a) comparisons | **1,296** |
| Oracle (b) comparisons | **972** |
| Oracle (c) assertions | **1,296** |
| **Total differential and golden assertions** | **≈3,564, from 108 programs** |

**Never publish a coverage percentage in this file or in either register.** Coverage instrumentation
requires a development dependency, which this repository forbids absolutely — so no percentage in
this repository is measurable, and publishing one would be fabrication. The matrix above **is** the
coverage evidence: it is countable directly from the committed file set, and the run summary
re-reports it on every execution. `report.rs` treats a discovered count that disagrees with these
figures as a corpus defect, so the numbers cannot drift away from the files.

### The 14 feature areas

| Area directory | Programs | Mandated |
| --- | --- | --- |
| `01_integer_conversions` | 10 | yes |
| `02_constant_expressions` | 8 | yes |
| `03_initializers` | 11 | yes |
| `04_bitfields` | 7 | yes |
| `05_pointers` | 10 | yes |
| `06_control_flow` | 10 | yes |
| `07_variadics` | 6 | yes |
| `08_gcc_extensions` | 8 | yes |
| `09_optimization_levels` | 8 | yes |
| `10_declarations_and_types` | 10 | supplementary |
| `11_literals_and_strings` | 4 | supplementary |
| `12_preprocessor` | 6 | supplementary |
| `13_floating_point` | 4 | supplementary |
| `14_abi_calling_convention` | 6 | supplementary |
| **Total** | **108** | |

The nine mandated areas are the acceptance floor set by the requirements; each carries no fewer than
six programs. The five supplementary areas were added because they carry the widest cross-backend
divergence surface.

### Measured performance budget

- ≈**72.5 ms** per compile-and-run pair, including emulator startup (36 pairs completed in 2.612 s
  wall time).
- The full matrix is ≈2,592 compile-and-run pairs, so ≈**188 s serially**, and well under a minute
  spread across the harness's default thread pool given 14 independent area tests.
- A per-cell timeout bounds any runaway execution, and a timeout is classified as a divergence
  rather than swallowed as an infrastructure error.

---

## Running the suite

| Purpose | Command |
| --- | --- |
| Full suite | `cargo test --test conformance` |
| Full suite, streaming the verdict table | `cargo test --test conformance -- --nocapture` |
| One feature area | `cargo test --test conformance area_04_bitfields -- --nocapture` |
| Only the four infrastructure tests | `cargo test --test conformance infra_ -- --nocapture` |
| One program, full matrix | `BCC_CONFORMANCE_ONLY=04_bitfields/003_compound_assignment cargo test --test conformance area_04_bitfields -- --nocapture` |
| Readable interleaved output | `cargo test --test conformance -- --nocapture --test-threads=1` |
| Retain all cell workspaces | `BCC_CONFORMANCE_KEEP_WORK=1 cargo test --test conformance -- --nocapture` |
| Fast local iteration (reduced matrix) | `BCC_CONFORMANCE_QUICK=1 cargo test --test conformance` |
| Pre-flight environment check | `cargo test --test conformance infra_oracle_capability_report -- --nocapture` |
| Whole-repository health gate | `cargo test` |

### The 18 tests

Fourteen area tests, named exactly after their directories:

```text
area_01_integer_conversions        area_08_gcc_extensions
area_02_constant_expressions       area_09_optimization_levels
area_03_initializers               area_10_declarations_and_types
area_04_bitfields                  area_11_literals_and_strings
area_05_pointers                   area_12_preprocessor
area_06_control_flow               area_13_floating_point
area_07_variadics                  area_14_abi_calling_convention
```

Plus four infrastructure tests:

```text
infra_flag_capability_probe        infra_expected_divergence_register
infra_ub_audit_gate                infra_oracle_capability_report
```

### Why each area is one batch test rather than one test per program

The built-in test harness stops a test at its **first** failed assertion, and the final deliverable
requires a summary enumerating **every** outcome — every expected divergence with its documented
basis, and every finding with its reproducer. Stopping at the first divergence would truncate
exactly the artifact the requirements ask for.

Each area test therefore runs its entire matrix, accumulates every verdict, and only then asserts
that no cell produced a FAIL or an XPASS. The failure message reproduces the **complete**
per-program table, so nothing is lost in the runner output either.

### Reduced runs are always stamped as reduced

`BCC_CONFORMANCE_QUICK` restricts the matrix to the native target at `-O0` and `-O2` only. It is
**never the default**, and every report it produces is **stamped as reduced coverage**, so a quick
run can never be mistaken for a full one. The same stamping applies when a name filter or
`BCC_CONFORMANCE_ONLY` ran only a subset: the summary is explicitly labelled partial.

### Pre-flight check

Run `infra_oracle_capability_report` first in any new environment. It prints the discovered oracle
inventory and states exactly which arms of which oracles will run, so a misconfigured environment is
diagnosed **before** 1,296 cells execute.

---

## Environment variables

Every variable has a safe default. **The suite runs correctly with none of them set.** A boolean
variable is true when it is set, non-empty and not the single character `0`.

| Variable | Default | Purpose |
| --- | --- | --- |
| `BCC_BIN` | the Cargo-provided `CARGO_BIN_EXE_bcc` path | Override the compiler under test, for validating an externally built binary |
| `BCC_REF_CC` | probe `gcc`, then `cc`, then `clang` | Native reference compiler for oracle (a) and for both audit gates |
| `BCC_REF_CC_I686` | `i686-linux-gnu-gcc` | Oracle (a)'s i686 arm |
| `BCC_REF_CC_AARCH64` | `aarch64-linux-gnu-gcc` | Oracle (a)'s AArch64 arm |
| `BCC_REF_CC_RISCV64` | `riscv64-linux-gnu-gcc` | Oracle (a)'s RISC-V 64 arm |
| `BCC_QEMU_I386` | probe `qemu-i386`, then `qemu-i386-static` | i686 execution runner |
| `BCC_QEMU_AARCH64` | probe `qemu-aarch64`, then `qemu-aarch64-static` | AArch64 execution runner |
| `BCC_QEMU_RISCV64` | probe `qemu-riscv64`, then `qemu-riscv64-static` | RISC-V 64 execution runner |
| `BCC_CONFORMANCE_QUICK` | unset | Reduce the matrix to the native target at `-O0` and `-O2` only; always reported as reduced coverage |
| `BCC_CONFORMANCE_ONLY` | unset | Restrict the run to one `<area>/<program>` |
| `BCC_CONFORMANCE_STRICT` | unset | Treat an unavailable oracle as a failure — the intended continuous-integration setting |
| `BCC_CONFORMANCE_ALLOW_XPASS` | unset | Downgrade unexpected success from a failure to a warning during a marker-retirement window |
| `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` | unset | Explicitly acknowledge a reduced-oracle environment; the gap is still reported, and under strict mode it is still a failure |
| `BCC_CONFORMANCE_TIMEOUT_SECS` | `30` | Per-cell execution budget |
| `BCC_CONFORMANCE_KEEP_WORK` | unset | Retain every cell workspace instead of removing it on success |

Configuration is read **once** into a single validated snapshot before any cell runs, so two
concurrently executing area tests cannot observe different policies and the report cannot describe a
policy other than the one applied. A malformed value is a hard error naming the variable; an invalid
value is never treated as absence, because falling back to a probed default there would test a
different tool than the one that was named.

**Locating the compiler under test.** By default the suite resolves `bcc` through the Cargo-provided
`CARGO_BIN_EXE_bcc` path, which points at the **freshly built** binary. That is what guarantees the
suite never silently validates a **stale build**. `BCC_BIN` overrides it when the subject is an
externally supplied binary.


---

## Environment setup

```bash
rustup toolchain install 1.93.1
rustup default 1.93.1

apt-get install -y gcc gcc-i686-linux-gnu gcc-aarch64-linux-gnu gcc-riscv64-linux-gnu \
  qemu-user-static libc6-dev-i386 libc6-dev-arm64-cross libc6-dev-riscv64-cross
```

The documented Rust minimum is **1.70+**, edition 2021; 1.93.1 stable matches the documented build.

Verify before running the full matrix:

```bash
gcc --version                      # expect 13.x; default mode is gnu17
i686-linux-gnu-gcc --version       # expect 13.x
aarch64-linux-gnu-gcc --version    # expect 13.x
riscv64-linux-gnu-gcc --version    # expect 13.x
qemu-aarch64-static --version      # expect 8.2.x or newer
cargo test --test conformance infra_oracle_capability_report -- --nocapture
```

### Toolchain of record

Measured, not assumed — each version was obtained from the package manager's candidate version and
confirmed by invoking the tool:

| Role | Tool | Version |
| --- | --- | --- |
| Reference compiler, native | `gcc` | **13.3.0** (package `4:13.2.0-7ubuntu1`) |
| Reference compiler, i686 | `i686-linux-gnu-gcc` | same package version |
| Reference compiler, AArch64 | `aarch64-linux-gnu-gcc` | same package version |
| Reference compiler, RISC-V 64 | `riscv64-linux-gnu-gcc` | same package version |
| Cross execution | `qemu-user-static` | **8.2.2** |
| Native C runtime and static libc | `libc6-dev` | 2.39 |
| Per-cell timeout | `timeout` (coreutils) | present at `/usr/bin/timeout` |
| Alternate reference oracle | `clang` | **optional**, not installed here |
| Reducer for finding minimization | `creduce` | **optional**, not installed here |

`gcc` is the reference compiler of record. `clang` is selectable through `BCC_REF_CC` where it is
installed, giving a second independent oracle at no code cost.

### binutils is not a dependency

The flag probe reads ELF identification bytes directly with the standard library rather than
shelling out to `readelf`, `objdump` or `nm`:

- the type field sits at file offset `0x10` in **both** ELF classes — `1` = relocatable object,
  `2` = executable, `3` = shared object or position-independent executable;
- an interpreter program header is present **only** when the artifact is dynamically linked, which
  is what makes `-static` verifiable rather than merely accepted;
- `-g` produces a `.debug_info` section, asserted **present** with the flag and **absent** without
  it, so both directions of the claim are checked.

### Graceful degradation

| Missing component | Consequence |
| --- | --- |
| The compiler under test | **Hard failure.** The suite cannot function and says so immediately |
| Native reference compiler | Oracle (a) unavailable **entirely**; oracles (b) and (c) still run |
| One cross reference driver | Only that architecture's oracle (a) arm is unavailable |
| One emulator | That target drops out of oracle (b) **and** oracle (a)'s cross arm; the other three targets continue |
| Cross C runtime for one architecture | That architecture's link step fails and is reported as a **link failure at environment scope**, never as a compiler defect |
| `timeout` utility | Execution falls back to a standard-library watchdog thread; no behavioural change |
| `creduce` | Finding minimization becomes manual; findings remain complete, since the reproducer, commands, outputs and environment fingerprint do not depend on it |

Note the second row carefully, because it is the one non-obvious entry: an absent **native**
reference compiler takes oracle (a) out on **every** target, not merely on the native arm, and a
surviving cross driver does not rescue a cross arm. The native compiler is the driver both audit
gates run, and those gates are what establish that a program contains no undefined or unspecified
behaviour — the precondition that makes an oracle (a) divergence mean anything at all. Reporting
such an arm as available would offer a comparison whose verdict could not be interpreted. Oracles
(b) and (c) are untouched, since (b) compares `bcc` against `bcc` and (c) against the recorded
golden stdout.

**Missing tooling never produces a silent pass.** Every unattemptable arm is enumerated by name in
the summary. Under `BCC_CONFORMANCE_STRICT` every UNAVAILABLE becomes a failure — the intended
continuous-integration setting, because in CI the toolchain is installed deliberately and a missing
oracle indicates a broken workflow rather than a modest environment. Strict mode is **dominant**: no
other variable can lower it, and `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` is an acknowledgement
recorded in the report, never a suppression.

---

## Reproducing a cell by hand

This is the operational payoff: **every cell is reproducible from the program source and its
expectation record alone** — with no harness, no Cargo and no Rust toolchain.

### Procedure

1. Open the program's `.expected` record.
2. Read `bcc_command`, `ref_command` and `run_command`.
3. Substitute the placeholders:
   - `$BCC` → the `bcc` binary;
   - `$REF_CC_<TRIPLE>` → the reference driver matching the target;
   - `<triple>` → the target triple, e.g. `aarch64-linux-gnu`;
   - `<opt>` → the optimization level, e.g. `-O2`;
   - `<src>` → the program's `.c` file;
   - `<out>` → any output path you like;
   - `<runner>` → **empty** for x86-64, otherwise the matching `qemu-<arch>-static`.
4. Run both binaries. Compare stdout bytes and exit status against **each other** (oracle a) and
   against `expected_stdout` / `expect_exit` (oracle c).

### Fully worked example

Cell: `01_integer_conversions/004_narrowing_conversions.c`, target `aarch64`, level `-O2`.

The record declares:

```text
bcc_command        = $BCC --target <triple> <opt> -static <src> -o <out>
ref_command        = $REF_CC_<TRIPLE> <opt> -static <src> -o <out>
run_command        = <runner> <out>
expect_exit        = 0
```

Every placeholder resolved — these three lines are literal and copy-pasteable from the repository
root:

```bash
# 1. Build with the compiler under test.
./target/debug/bcc --target aarch64-linux-gnu -O2 -static \
  tests/conformance/01_integer_conversions/004_narrowing_conversions.c -o /tmp/bcc.out

# 2. Build with the matching reference cross driver.
aarch64-linux-gnu-gcc -O2 -static \
  tests/conformance/01_integer_conversions/004_narrowing_conversions.c -o /tmp/ref.out

# 3. Run both under the target's emulator, which is the runner for aarch64.
qemu-aarch64-static /tmp/bcc.out > /tmp/bcc.stdout; echo "bcc exit=$?"
qemu-aarch64-static /tmp/ref.out > /tmp/ref.stdout; echo "ref exit=$?"

# 4a. Oracle (a): the two must be byte-identical, and both exit statuses must match.
cmp /tmp/bcc.stdout /tmp/ref.stdout && echo "oracle a: agree"

# 4b. Oracle (c): the captured stream must equal the record's expected_stdout verbatim,
#     and the exit status must equal expect_exit, which this record declares as 0.
printf 'narrow_u8=200 narrow_i8=-56 narrow_u16=65336\nruntime_u8=200 runtime_i8=-56 runtime_u16=65336\n' \
  | cmp - /tmp/bcc.stdout && echo "oracle c: golden record matches"
```

For the x86-64 cell of the same program, `<runner>` is empty, so step 3 becomes `/tmp/bcc.out` and
`/tmp/ref.out` invoked directly, and `$REF_CC_<TRIPLE>` in step 2 is the native `gcc`.

For oracle (b), build the same program for two targets at the same level and compare the two
captured streams to each other, using the x86-64 cell as the baseline.

For a **finding**, none of this substitution is necessary: `commands.sh` inside the finding
directory already contains these lines, fully resolved, for every cell involved.


---

## Directory layout and artifacts

```text
tests/conformance/
├── README.md                        this file — the suite contract
├── EXPECTED_DIVERGENCES.md          register of every expected-divergence marker
├── FINDINGS.md                      register of every finding
├── 01_integer_conversions/          10 programs: <NNN_name>.c + <NNN_name>.expected
├── 02_constant_expressions/          8 programs
├── 03_initializers/                 11 programs
├── 04_bitfields/                     7 programs
├── 05_pointers/                     10 programs
├── 06_control_flow/                 10 programs
├── 07_variadics/                     6 programs  (may #include <stdarg.h>)
├── 08_gcc_extensions/                8 programs  (warning gate drops -pedantic)
├── 09_optimization_levels/           8 programs
├── 10_declarations_and_types/       10 programs
├── 11_literals_and_strings/          4 programs
├── 12_preprocessor/                  6 programs
├── 13_floating_point/                4 programs
├── 14_abi_calling_convention/        6 programs
├── support/
│   └── include/
│       └── probe_header.h           the suite's ONLY fixture
├── tools/
│   └── regenerate_expected.sh       maintenance-only; never invoked by cargo test
└── findings/
    └── F-NNNN-<slug>/               one curated, committed finding
        ├── reproducer.c
        ├── reproducer.expected
        ├── MANIFEST.txt
        ├── commands.sh
        ├── outputs/                 captured stdout, exit status and stderr per cell
        ├── environment.txt
        └── diff.txt
```

### The single fixture

[`support/include/probe_header.h`](support/include/probe_header.h) is the suite's **only** fixture,
used **solely** by the `-I` flag probe to prove that a command-line include directory is actually
searched: **with `-I` the compile succeeds; without `-I` it must fail.** The negative half is the
load-bearing one, so no copy of this header may exist anywhere else in the repository — a duplicate
on a default search path would make that half vacuous while the probe still reported success.

There is no fixture hierarchy, no factory and no test-data directory. Every other input in the suite
is a literal in a program's own source.

### Maintenance tooling

[`tools/regenerate_expected.sh`](tools/regenerate_expected.sh) regenerates golden records. It is
**maintenance-only** and is **never invoked by `cargo test`**. That separation is what stops a wrong
answer from quietly becoming the new expectation.

### The transient-versus-curated split

**Preserve this split.** It is what keeps an in-progress run from polluting a committed deliverable.

**Committed and tracked — this folder:**

- the 108 programs and their 108 expectation records;
- `support/`, `tools/` and `findings/`;
- the three Markdown files: this contract, [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) and
  [`FINDINGS.md`](FINDINGS.md).

**Transient and git-ignored — elsewhere entirely, beneath the build directory:**

| Path | Content |
| --- | --- |
| `target/conformance-work/` | Per-cell workspaces. Removed on success, **retained on failure**, so a failing cell leaves behind exactly the artifacts needed to investigate it |
| `target/conformance-report/areas/<area>.md` | Per-area human-readable report |
| `target/conformance-report/areas/<area>.tsv` | Per-area machine-readable report — each area writes only its own file, so there is no contention under parallel execution |
| `target/conformance-report/summary.md` | The deliverable summary |
| `target/conformance-report/summary.tsv` | The same data, machine-readable |
| `target/conformance-findings/F-NNNN-<slug>/` | Auto-generated finding artifacts from the current run |

**`findings.rs` never writes into `tests/conformance/`.** The curated finding set and both registers
are human-maintained committed deliverables. A run writes only beneath the build directory.

### The deliverable summary

`target/conformance-report/summary.md` is the artifact the requirements ask for. It reports the
feature areas covered, the total cells and their outcomes, every expected divergence with its
documented basis, and every finding with a pointer to its reproducer and its reproduction commands.
It is also this suite's coverage evidence, for the reason given under
[the enumerable matrix](#the-enumerable-matrix).

### Cross-references

| Path | What it holds |
| --- | --- |
| [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) | The expected-divergence register, machine-checked against the markers in both directions |
| [`FINDINGS.md`](FINDINGS.md) | The findings register, indexing every curated reproducer |
| [`../conformance.rs`](../conformance.rs) | The suite driver: 14 area tests and 4 infrastructure tests |
| [`../conformance_harness/`](../conformance_harness/) | The harness modules — oracle discovery, sandboxing, record parsing, compilation, execution, comparison, classification, findings, reporting, the flag probe and the audit gate |
| `docs/testing/differential-conformance.md` | The documentation-site page: methodology, oracle definitions, the build matrix, the verdict taxonomy and the summary format |

---

## Adding a program, adding an area, retiring a marker

### Adding a program

Create exactly **two** files in the appropriate area directory:

```text
tests/conformance/<NN_area>/<NNN_name>.c
tests/conformance/<NN_area>/<NNN_name>.expected
```

**No harness change is needed**, because discovery is directory-driven. Then:

1. Write the program against every rule in [Corpus authoring rules](#corpus-authoring-rules).
2. Copy the [canonical reference record](#the-canonical-reference-record) and edit `program`, `area`
   and `description`. Remember that `program` must equal the file stem and `area` must equal the
   directory name.
3. Regenerate the golden record through
   [`tools/regenerate_expected.sh`](tools/regenerate_expected.sh).
4. Re-run the area test, then `infra_ub_audit_gate` to confirm the new program is clean under both
   gates.
5. Update the area's program count in the harness area table and in the
   [feature-area table](#the-14-feature-areas) above, since a discovered count that disagrees with
   the declared count is treated as a corpus defect.

### Adding an area

Create the directory with its two files per program **and** add one area test to
[`../conformance.rs`](../conformance.rs), named `area_<NN>_<directory_name>`. The directory name and
the test name must agree, because the area names the report file its verdicts are written to.

### Retiring a marker

Delete **both** halves:

1. the five `expected_divergence.*` keys from the program's `.expected` record; and
2. the corresponding entry from [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md).

**Removing only one side fails `infra_expected_divergence_register`**, which checks both
directions — every marker identifier must appear in the register, and every register entry must
correspond to a real marker. A marker retired in one place and left in the other is still stale
documentation, which is exactly what the check exists to prevent.

---

## Constraints and engineering standards

These govern every file in this directory.

### C1 — No compiler source change

`src/**`, `include/**`, `build.rs`, `Cargo.toml` and `Cargo.lock` are **read-only reference
material**. `include/*.h` is read only to determine what a test program may legally include.

**No `.rs` file at any depth in this directory** — that is what keeps Cargo blind to the corpus and
`Cargo.toml` unmodified. The suite is arranged so that no manifest change is required at all:
`tests/conformance.rs` is auto-discovered as an integration target, `tests/conformance_harness/`
contains no `main.rs` and is therefore never a target, and this directory contains only data files.

### C2 — No existing test deleted, skipped, weakened or relaxed

No existing test file is edited. No `#[ignore]` attribute is added or removed anywhere, and the
repository's ignored-test count stays **exactly 13**, asserted as an invariant rather than merely
intended.

The reason those 13 stay ignored is worth recording, because it looks like an omission otherwise:
they are network-dependent, and C4 forbids network access. Re-enabling them would violate C4 while
removing their `#[ignore]` attributes would violate C2. Both constraints agree, so the resolution is
unambiguous.

### C3 — Never exclude a language feature because it is difficult

Write and execute the test regardless. Where a comparison genuinely cannot be made, narrow the
exclusion to a **single oracle**, keep the program running under the remaining oracles, and record
the reason inside the program's own `.expected`.

Two programs exist **because** of this constraint, and a difficulty-driven plan would not have
produced either:

- **`08_gcc_extensions/004_case_ranges.c`** — GCC case ranges appear in **no** documented `bcc`
  extension inventory (the documented set enumerates `__attribute__`, statement expressions,
  `typeof`, computed goto and inline assembly, and omits case ranges). The program is written and
  executed anyway. If it diverges, the verdict is XFAIL against marker
  `XD-GCCEXT-CASE-RANGES-001`, and the ambiguity is surfaced so a maintainer can determine whether
  the omission is an implementation gap or a documentation gap.
- **`13_floating_point/004_long_double_target_restricted.c`** — `long double` was measured to have
  three different representations across the four targets (16, 12, 16 and 16 bytes; x87 80-bit
  against IEEE binary128), so cross-backend *value* equality is genuinely meaningless for it. The
  program is still written and still executed; it is compared against the same-target reference
  compiler and against its golden record, and only cross-backend value equality is disabled, with
  the measured reason recorded in its own `.expected`.

Two other measured differences are handled **by construction** rather than by marker, because a
marker implies a test that diverges and these do not: plain-`char` signedness is avoided by using
explicit `signed char` and `unsigned char`, and the i686 `long` and pointer width difference is
avoided by width normalization. Both are recorded as implementation-defined notes in the affected
programs.

### C4 — Hermetic execution

Every input is a literal in the program source. **No program opens a socket, calls `fopen`, or reads
`argv` or `getenv`.** Each cell runs in its own deterministic workspace beneath the build directory,
uniquely determined by area, program, target and optimization level, and writes nothing outside it.
A per-cell timeout bounds runaway execution.

This is why the whole folder has **exactly one** fixture file, and why the determinism rules above
are what they are: a program that read the clock, the environment or an external file would not be
reproducible, which would make byte-exact comparison meaningless.

### The Zero External Crate Dependency Rule

Quoting `docs/technical-specifications.md` §0.7 verbatim:

> - The `[dependencies]` section of `Cargo.toml` must remain completely empty at all times
> - No `[build-dependencies]` or `[dev-dependencies]` entries for external crates are permitted
> - This constraint is absolute and admits no exceptions

The consequences for this suite are direct and visible in its design: the harness is pure `std`; the
`.expected` parser is hand-written rather than a serialization crate; and there is no coverage
tooling, no snapshot crate, no temporary-directory crate, no dynamic-test-case crate, no timed-wait
crate and no program generator. Where a crate would ordinarily be reached for, the substitute is
named rather than improvised — deterministic workspaces instead of randomized temporary directories,
batch verdict accumulation instead of dynamically generated test cases, golden records instead of
snapshots, and the system `timeout` utility with a standard-library watchdog fallback instead of a
timed-wait crate.

### Zero-warning discipline

All harness code satisfies `cargo clippy -- -D warnings` and `cargo fmt -- --check` with **no new
suppression, allowance attribute or formatting exception**.

### Report, never patch

**No compiler source change is made in response to any finding.** Findings are deliverables, not
defects to patch. A finding produces an artifact directory and a register entry, and nothing else.

### No mocking, no dependency injection

The reference compiler, the emulators, the system C runtime and static `libc` are all used **for
real**.

This is architectural rather than incidental. A stub would return whatever the author expected,
which is **precisely the assumption the oracle exists to eliminate**. Substituting the reference
compiler would make oracle (a) a test of the author's expectations; simulating execution would
defeat the entire purpose of oracle (b), which is to observe what the generated machine code
actually does on each architecture. The only substitution anywhere in the design is the opt-in
reduced matrix, which is never the default and is always stamped as reduced coverage.

Nothing is virtualized either. The file system is used for real but constrained to per-cell
workspaces; the network is never touched; and time, randomness, addresses and locale are never
observed, because any of them would make byte-exact comparison meaningless.

### Noted non-goals

Recorded here as **decisions rather than oversights**, so their absence is visible:

- **Error-detection testing — the correct *rejection* of invalid programs — is out of scope.** Both
  mandated oracles require the program to compile and run successfully, so invalid-program rejection
  cannot be measured by either. Independent evaluation of this compiler's lineage suggests it would
  be a high-yield future axis, which is exactly why the omission is written down.
- **No marker exists for the documented shared-library or debugger-validation gaps.** This suite
  builds only static executables and never invokes a debugger, so neither documented limitation can
  manifest in its results. A marker would imply a divergence that cannot occur.
- **No standard-error comparison**, for the reason given under
  [Comparison discipline](#comparison-discipline).
- **No random program generation or fuzzing.** The requirement is broad systematic coverage rather
  than adversarial search, and a generator would be an external tool the zero-dependency rule bars.
  The corpus is hand-authored and fully deterministic, which additionally makes every finding
  immediately human-readable.

