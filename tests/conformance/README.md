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
   exclude a language feature because it is difficult), and **C4** (compile and execute only within
   the suite's own working directory, with no network access and no file access outside it) — each
   documented with its consequences under
   [Constraints and engineering standards](#constraints-and-engineering-standards); and
2. the repository's own documented engineering standards, chiefly the Zero External Crate
   Dependency Rule, the zero-warning quality gates, and the existing test conventions.

No rule is invented here, and no rule text is paraphrased, because there is none to paraphrase.

---

## The three oracles

Three independent oracles judge every cell. Two are mandated by the requirements; the third costs
nothing and closes a hole the other two structurally cannot see.

The **Cell volume** column below states the **final planned** figures, for the full 108-program
corpus. The committed corpus is smaller today; both sets of numbers are published side by side under
[the enumerable matrix](#the-enumerable-matrix).

| Oracle | Compares | Detects | Cell volume (final planned) |
| --- | --- | --- | --- |
| **(a) Reference compiler** | `bcc` against a reference C compiler, same target, same optimization level | A wrong answer `bcc` produces consistently across all four of its backends | **324** native, up to **972** cross |
| **(b) Cross-backend** | Each non-baseline target against the **x86-64 baseline**, same optimization level | A wrong answer confined to one backend — ABI, register-allocation or instruction-selection defects | **972** comparisons |
| **(c) Golden record** | Each cell against **both** values recorded in the program's own `.expected` record: the `expected_stdout` bytes and the `expect_exit` status | Both compilers changing behaviour in the same direction at the same time, plus toolchain drift and regression over time | **1,296** assertions |

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

**Why `bcc`'s own `--target` flag is legitimate:** the shared-flag requirement governs the
arguments passed **identically to both compilers**, and `--target` never is one. It is
**`bcc`-only, and it is never passed to the reference compiler** — which has no target-selection
flag to be given: the `--target=` spelling belongs to a different compiler family and was measured
to be rejected, so the reference side selects a target by being a **different driver binary** (see
[Why oracle (a)'s cross arm uses a cross driver](#why-oracle-as-cross-arm-uses-a-cross-driver)).

That split is by compiler side, not by oracle, and the distinction matters because it is easy to
get backwards. `bcc` is a single binary with no cross drivers, so `--target <triple>` is the only
way it can reach a non-native backend **at all**; the harness therefore puts it on `bcc`'s side of
**every** cell it assembles — non-native and native alike, under **oracle (a) exactly as much as
under oracle (b)** — because one unconditional spelling keeps a single recorded command template
correct for every cell of a program. Oracle (a)'s cross arm compares `bcc --target <triple> …`
against `<triple>-gcc …`: each side selects the same target in the only way it can. What is
excluded is the flag ever entering the **shared** argument set, and `is_forbidden_for_side` in
`../conformance_harness/mod.rs` is the single place that exclusion is expressed.

Within oracle (b) there is additionally nothing to argue about, since **both sides are `bcc`**:
selecting a target is precisely what the oracle requires, and it could not exist otherwise.

### Oracle (c) — golden-record regression

Assert each cell against the two values the program's own co-located `.expected` record commits to:
the **recorded stdout bytes** in `expected_stdout` and the **recorded exit status** in `expect_exit`.
Both are checked, and either one disagreeing is a divergence — the exit status is part of the golden
record precisely because a program that prints the right bytes and then exits wrongly has still
behaved wrongly.

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
| **PASS** | **Per oracle:** this comparison agreed and no marker governs the cell | No |
| **XFAIL** | Two forms, both reported as XFAIL: a divergence matching an **active marker**, or a comparison the program's own record **deliberately declines to make**, carrying its reasoned exclusion in `impl_defined_notes` | No |
| **XPASS** | A marker is present but the divergence has **disappeared** | **Yes** (by default) |
| **FINDING** | An **undocumented** divergence; an artifact directory is written | No (reported as a deliverable) |
| **FAIL** | Anything unexplained | **Yes** |
| **UNAVAILABLE** | An oracle's tooling is genuinely absent from the environment | No, but reported loudly |

**Verdicts are per oracle, not per cell.** Each enabled oracle renders its own verdict for a cell,
so one cell can carry up to three. A **cell** is all-PASS only when **every enabled oracle's outcome
for it is PASS**; a cell whose oracle (a) agreed while its oracle (b) diverged is not a passing cell,
and the reports list the two outcomes separately rather than collapsing them. That separation is what
lets a divergence be attributed to the oracle that saw it.

**The two XFAIL forms, distinguished.** Both are reported as XFAIL and neither fails the run, but
they arise from opposite directions and must not be confused:

| Form | What happened | Where the explanation lives |
| --- | --- | --- |
| **Marker-covered divergence** | The comparison *was* made, it diverged, and an **active marker** covers this oracle, target, optimization level **and** class | The marker in the program's own record, mirrored in [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md), citing a limitation the repository documents |
| **Recorded exclusion** | The comparison was **not attempted**, because the program's own record narrows its coverage — an oracle switched off, a restricted target list, or a deviating warning gate | The reasoned exclusion in that record's `impl_defined_notes`, which the record format refuses to accept as absent, printed in full in the report |

A recorded exclusion is deliberately **not** UNAVAILABLE and deliberately **not** PASS. UNAVAILABLE
is a statement about the *machine* — a tool nobody installed. A narrowing is a statement about the
*corpus* — an authoring decision, taken on the record. And nothing was compared, so no equality may
be claimed. The cell is still counted and still listed with its reason, which keeps the set of
comparisons deliberately **not** made as visible as the set that was. The one shape that is never
excused is a record that *enables* an oracle while the comparison claims exclusion: that removes a
comparison the corpus asks for, and it is a FAIL.

**There is deliberately no "skip because unsupported" verdict.** An absent oracle is reported as
UNAVAILABLE — loudly, and in the summary — **never as a silent pass**. That is what keeps a modest
environment from masquerading as a passing run. Under `BCC_CONFORMANCE_STRICT` an UNAVAILABLE
becomes a failure; see [Graceful degradation](#graceful-degradation).

The four verdicts permitted in a passing run are PASS, XFAIL, FINDING and UNAVAILABLE, and each is
still reported in full. A finding does not fail the run because a finding is a deliverable; an
unavailable oracle does not fail the run by default because the environment, not the compiler, is
what is incomplete.

**A FINDING verdict always carries its artifact directory.** Every path that can produce one — an
ordinary output or status divergence, and equally a *refused build* attributed to a compiler — is
routed through a single assembly-and-record funnel, so the verdict and the deliverable cannot come
apart. A build that the compiler under test rejected while the reference compiler accepted it, or
the reverse, therefore produces a complete artifact directory for **every applicable oracle** rather
than a verdict row pointing at nothing: the refusing side's compile stdout, stderr and raw wait
status are persisted into the cell workspace **before either ending is taken**, so the evidence
exists whichever way the cell resolves. A cross-backend baseline that was itself refused is treated
the same way, and its finding carries both halves — the baseline's refusal and the target's own
observation.

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

**This holds for a build refusal too, which is why one is scoped `all oracles`.** A refusal reaches
classification once per oracle arm, each settled against its own authority, so the oracle dimension
narrows a real set there as well; a marker meant to document a refusal names every arm it denies.
See [Every validation the parser enforces](#every-validation-the-parser-enforces) for the exact
spelling and what happens when a single oracle is named instead.

**A marker never changes what a program does.** A marker changes how a divergence is *classified*,
never whether the feature is *exercised*, and nothing in the harness may short-circuit a phase
because a marker exists.

**The phase lifecycle, stated precisely.** The applicable phases are attempted in order — compile,
link, run, compare — and classification happens at the **first terminal outcome or the completed
comparison**, not after every phase has run. A compile failure, a link failure, a crash and a timeout
are *terminal outcomes* reached before any comparison exists, and each is classified where it
occurred. That is exactly why a `compile_failure` marker is meaningful at all: the cell it excuses
never reaches a comparison, so a rule demanding one would make the whole class unreachable.

---

## The `.expected` record format

Every program has a sibling `<program>.expected` record. This single file is what makes each test
reproducible in isolation and what supplies oracle (c). This section is **binding on every record**
— the 73 committed on this branch and every one still to be authored — and matches the hand-written
parser (`manifest.rs`) in
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

Below is `tests/conformance/01_integer_conversions/004_narrowing_conversions.expected`
**reproduced verbatim** — the complete file, byte for byte, header comments and final newline
included. It is the exemplar every new record is derived from, and it is copyable: writing these
bytes to a `.expected` file beside a program named `004_narrowing_conversions.c` yields a record the
parser accepts and the undefined-behaviour audit gate passes. Anything shortened for readability
would stop being canonical, so nothing here is shortened.

Note in particular that it declares a warning-gate deviation, and that the deviation's reason names
**both** dropped flags in a paragraph of its own — the audit gate requires exactly that, and a
paraphrase that omitted either flag spelling would be refused. When editing this exemplar, edit the
record first, verify it with `cargo test --test conformance infra_ub_audit_gate`, and then copy the
file back into this block.

```text
# Expectation record for 01_integer_conversions/004_narrowing_conversions.c
#
# Reproduce one cell by hand with no harness: render the three command templates
# below, substituting the target triple, the optimization level, the source path,
# the output path, and the target's runner (empty on the natively executing target).

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
This program contains no undefined behaviour, which is the property the differential oracle
rests on. It deliberately does contain two implementation-defined conversions, and the
distinction matters: an implementation-defined conversion has a definition the implementation
must document and abide by, so a divergence between two compilers that both document the same
definition is still evidence of a defect. Undefined behaviour would permit anything and would
make a divergence prove nothing, which is why none is present.
Conversion to an unsigned narrow type is fully defined by the standard as reduction modulo one
plus the destination's maximum (C11 6.3.1.3p2), so (unsigned char)(-56) and (unsigned
short)(-200) are not implementation-defined at all. Conversion of an out-of-range value to a
SIGNED narrow type is implementation-defined -- C11 6.3.1.3p3 permits either an
implementation-defined value or an implementation-defined signal -- and this program performs
exactly two such conversions, both converting 200 to signed char: the folded (signed char)200
and the runtime assignment of the volatile source holding 200. Neither is value-preserving and
neither is claimed to be; all four supported targets yield a value by two's-complement
truncation, printing -56, with no signal raised, and the target assumptions that make them
comparable are recorded in impl_defined_notes below.
Otherwise: no signed overflow, no shift out of range, no aliasing violation, no uninitialized
read, no object modified twice between sequence points, no argument with a side effect, and no
dependence on padding bytes or on the addresses of unrelated objects.
END
impl_defined_notes <<END
Warning-gate deviation: -Wconversion and -Wsign-conversion are dropped for this program
because a narrowing conversion is precisely the behaviour under test, so those two
diagnostics fire on the feature itself rather than on a defect. Every other member of the
default gate is retained -- -Wall, -Wextra, -pedantic, -Wshadow and -Werror -- so the
diagnostics those flags enable stay in force even though they are not gate members
themselves: -Wsign-compare is enabled by the retained -Wextra, and the out-of-range
constant-conversion overflow diagnostic by the retained -pedantic (both measured with gcc
13.4.0 on this program's exact shape), while -Werror still makes either one fatal.

Plain char is not used -- every narrow type here is explicitly signed or unsigned -- so the
measured plain-char signedness difference between the targets cannot reach this program. All
widths are fixed-width types or int. No pointer values are printed.
Target assumption for the two implementation-defined conversions of 200 to signed char: each
of the four reference toolchains documents this conversion as reduction of the value modulo
two to the power of the destination width, with NO signal raised, which yields -56. That
assumption was verified rather than assumed: the program was executed on all four targets at
-O0, -O1 and -O2 -- twelve configurations -- and every one printed narrow_i8=-56 and
runtime_i8=-56, byte-identically. Because the definition is the same on all four and the
measured value agrees, cross-backend comparison of this program is sound and oracle (b)
remains enabled for it.
What would legitimately break that assumption, and how it must then be handled: a target whose
implementation defined this conversion differently, or which raised a signal instead of
producing a value, would be conforming, so the resulting difference would be an
implementation-defined divergence rather than a compiler defect. It must in that case be
recorded as an expected divergence with this record's target list narrowed and the reason
stated here -- never left to surface as a finding against a backend that did nothing wrong,
and never resolved by dropping the conversion from the corpus, which would remove the feature
requirement 2 asks this program to cover.
END
expected_stdout    <<END
narrow_u8=200 narrow_i8=-56 narrow_u16=65336
runtime_u8=200 runtime_i8=-56 runtime_u16=65336
END
```

Three details in it are worth reading rather than skimming, because each is the record format
carrying its weight:

- **`ub_audit_flags` is present, which means this record deviates from the default warning gate.**
  It drops `-Wconversion` and `-Wsign-conversion`, and the *reason* is written into
  `impl_defined_notes` — in a paragraph of its own, naming both flags by their exact spelling. That
  field is the **only** one searched for it: a deviation whose dropped flags `impl_defined_notes`
  does not name is a hard error at load time, and naming them in `ub_notes` instead does not satisfy
  the requirement, because `ub_notes` is not searched for a gate reason at all — see
  [Recorded reasons](#every-validation-the-parser-enforces).
- **`ub_notes` distinguishes two kinds of well-definedness.** Narrowing to an **unsigned** type is
  defined for every value — the result is reduced modulo one plus the destination maximum, which is
  why `(unsigned char)(-56)` is `200` and `(unsigned short)(-200)` is `65336` everywhere. Narrowing
  to a **signed** type a value it cannot represent — `(signed char)200` — is
  **implementation-defined**, *not* undefined. The distinction is what makes the comparison
  meaningful: an implementation-defined result is a real answer that two compilers can be held to,
  whereas undefined behaviour would license either of them to do anything and would make a
  divergence prove nothing.
- **`impl_defined_notes` is therefore mandatory here, and carries that one case.** It records that
  the signed narrowing is implementation-defined and that all four supported targets perform
  two's-complement truncation identically, which is what licenses the cross-backend comparison
  instead of excluding it.

### The optional marker block

Append all five keys, or none of them, to the record above. This is the **syntax**; whether a marker
may legitimately be written is a separate and much stricter question, answered immediately below.

The block below is an **illustration of the grammar only**. The identifier, the basis and the
observation are placeholders, deliberately not any real marker: no program in the corpus carries a
marker today, and [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) is the sole register of the
ones that ever do.

```text
expected_divergence.id       = ILLUSTRATION-ONLY-NOT-A-REAL-MARKER
expected_divergence.class    = compile_failure
expected_divergence.scope    = all oracles; all targets; all opt levels
expected_divergence.basis    = docs/project-guide.md, the section that explicitly documents this limitation
expected_divergence.observed <<END
bcc: <the diagnostic as it was actually seen>; reference compiler: <what it actually printed>
END
```

The oracle clause reads `all oracles` because the class is a **build refusal**: no artifact is
produced, so every oracle arm loses the subject of its comparison and a marker naming one arm would
leave the other two reported as findings. A marker for a class that *is* a comparison —
`stdout_mismatch` or `exit_code_mismatch` — names the oracle that made it, since only that arm
observed anything.

**A marker may not be minted on an omission, and two conditions must both hold.** A marker
reclassifies a divergence on the authority of a limitation this repository **explicitly documents**.
An *omission* from a documented inventory is not that: it records that no document mentions a
construct, not that the implementation rejects it. So:

1. a repository artifact must **explicitly document the limitation**, and the basis must cite that
   file and a locator within it — the document is resolved, contained, read and searched for that
   locator on every run; and
2. the divergence must have been **observed and reproduced**, and `expected_divergence.observed`
   must state what was actually seen rather than what someone expects to see.

Until **both** hold, an observed divergence is a **FINDING** — which is exactly what a finding is
for, and a first-class reported outcome rather than a compromise. A marker written speculatively
does active harm: if the construct in fact works the cell agrees, the verdict is **XPASS**, and the
run fails on a mistake in the test material rather than a defect in the compiler; and until someone
notices, the marker blinds the suite to a genuine regression in exactly the construct it was meant
to document. `EXPECTED_DIVERGENCES.md` §8.5 states the same rule as a checklist and §4 works through
the three candidates the suite has analysed without marking any of them.

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
  specification files, plugins, wrappers or output paths. Declaring the flags is only half of a
  valid deviation: it must also carry its recorded reason, on the exact terms set out under
  [Recorded reasons](#every-validation-the-parser-enforces) below.

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
- **A `ub_audit_flags` deviation REQUIRES its reason recorded in `impl_defined_notes`, and the
  requirement is enforced per flag by exact spelling** — a deviation without a recorded reason is
  itself a defect in the test. At load time the parser works out which members of the default gate
  the record drops and refuses the record unless `impl_defined_notes` names **every one of them by
  its exact spelling, leading hyphen included**. Naming some of them is not enough: a record that
  drops both `-Wconversion` and `-Wsign-conversion` but writes only the first will not load, and the
  diagnostic names the record, the full default gate, both dropped flags and precisely the one left
  unexplained. Note the split carefully, because it is easy to get wrong: `impl_defined_notes` is
  the **one** field this reason is read from — for a gate deviation exactly as for a restricted
  target list or a disabled oracle — and `ub_notes` is **not searched for it at all**. `ub_notes` is
  separately required of **every** record and holds the written undefined-behaviour-freedom argument
  and nothing else; a field that answers two questions answers neither reliably, and a reviewer must
  have one place to look rather than two. A record that narrows anything and carries no
  `impl_defined_notes` will not load. `infra_ub_audit_gate` then reports each deviation with its
  reason quoted from `impl_defined_notes`, keeping only the paragraphs that name a dropped flag, so
  the recorded reason is what a reader sees rather than a reprint of the whole field.
- `ub_notes` is required and must be non-empty in every record without exception.
  Put the deviation's explanation in a **paragraph of its own** — paragraphs are separated by a
  blank line — because the audit quotes the reason back by collecting exactly those paragraphs of
  `impl_defined_notes` that name a dropped flag. Burying it in a paragraph that also carries the
  target-restriction argument still loads, but it makes the reported reason the whole argument
  rather than the reason. `ub_notes` holds the written undefined-behaviour-freedom argument — the
  human half of the requirement whose machine half is the audit gate — and it is what a reviewer
  reads first when a divergence appears.

**Marker block**

- **A partial `expected_divergence.*` block is a hard error** — all five keys or none.
- `expected_divergence.class` must be one of the six class identifiers listed above.
- `expected_divergence.scope` is structured. Clauses are separated by `;`. A clause either names a
  whole dimension — `all oracles`, `all targets`, `all opt levels` — or lists members of exactly one
  dimension as a comma-separated list. An empty clause is rejected at its position rather than
  filtered away.
- **Scope matching is strict on all four dimensions — oracle, target, optimization level and
  class — with no exemption for any class.** A marker documenting a build refusal
  (`compile_failure`, `link_failure`, `run_crash`, `timeout`) must therefore scope **`all
  oracles`**, because a refusal denies every oracle arm the subject of its comparison. Each arm is
  settled against its own authority — the same-target reference capture, the baseline capture, the
  record's `expected_stdout` — so a scope naming one oracle excuses that arm alone and the others
  are reported as `FINDING`, with the oracle dimension named among the mismatches and the `all
  oracles` remedy stated in the detail. An arm whose authority this environment cannot supply is
  reported `UNAVAILABLE` instead and is never classified against a marker at all.
- `expected_divergence.basis` **must begin with a repository-relative file path**, then `, `, then
  the section or description that authorises the marker. The path must be relative and must not climb
  out of the repository; a whole-document citation with no section is rejected, because a citation a
  reader cannot check is not a basis. Three properties are then machine-verified by
  `infra_expected_divergence_register`, all three on the document's own bytes:
  - **containment** — the *fully resolved* path must lie inside this repository, so no symbolic link
    along the way can move the answer;
  - **a real, readable document** — it is read through the suite's bounded reader, which refuses a
    symbolic link, a device node or a FIFO at the final component and refuses an oversized file.
    Existence alone is not the property that matters; the document is read so the cited section can be
    resolved inside it;
  - **a locator that resolves** — the citation must carry at least one of `line 246`, `lines 696-725`,
    `§0.6.2` or a backtick-quoted phrase from the document, and every locator it carries must resolve.
- The marker's six fields must also be mirrored by a **structured entry** in
  [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md), and every field is compared against this
  record on every run. That register defines the entry shape and the locator grammar in full; mentioning
  an identifier there is not documenting a divergence.

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

The corpus's genuine companions are **siblings** of the area directories rather than children, so
nothing legitimate is displaced by these rules. Two of them are committed on this branch —
`support/`, and `EXPECTED_DIVERGENCES.md` alongside this contract. Three more are **specified by the
plan but not yet present**: `tools/`, `findings/` and `FINDINGS.md`. They are named here as plain
text rather than linked, precisely because a link to a path that does not exist is a broken link. The
sibling rule already accommodates all five, so landing the missing three needs no parser change.

### Read-only, and the golden-record rule

`manifest.rs` is **read-only with respect to the corpus**: there is deliberately no writer, fixer or
update-in-place helper anywhere in the harness.

**`expected_stdout` is never regenerated automatically during a test run**, so a wrong answer can
never quietly become the new expectation. Because `manifest.rs` has no writer at all, that guarantee
is structural rather than procedural.

The plan's maintenance script `tests/conformance/tools/regenerate_expected.sh` is intended to be the
one sanctioned way to refresh a golden record, deliberately outside the test run. It is **not present
on this branch**, which is why it is named here as plain text and not linked. Until it lands, a
golden record is refreshed by hand, and the refreshed bytes must be justified in the change that
touches them — the reviewer, not a script, is the gate.


---

## Corpus authoring rules

**These are not guidelines.** A program that violates any of them produces divergences that say
nothing about the compiler, which is worse than having no program at all — it costs a maintainer
the time to investigate and then teaches them to distrust the suite.

### Headers: hand-declare, do not include

**Hand-declare the libc prototypes a program needs and include no header**, with the single
sanctioned exception recorded below. That is `int printf(const char *, ...);` in nearly every
program, and `_Noreturn void exit(int);` as well in `10_declarations_and_types/008_noreturn.c`.

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

#### The two sanctioned header exceptions

Two exception classes are sanctioned, and **only** two. They differ in what they may include, and
conflating them is a defect in the test material, so they are stated separately.

**Exception 1 — `07_variadics`: `<stdarg.h>`, and nothing else.**

- **Scope:** every program in area `07_variadics`. Each may `#include <stdarg.h>`; no program in that
  area may include any other header.
- **Reason:** a variadic function cannot be written at all without `va_list`, `va_start`, `va_arg`
  and `va_end`. `va_copy` sits on the same documented row but is required only by a program that
  copies a list or traverses one twice — in this corpus that is
  `07_variadics/004_va_copy_multiple_passes.c` alone. Variadic functions are explicitly mandated,
  and constraint C3 forbids
  dropping a feature because it is difficult. `stdarg.h` **is** in `bcc`'s bundled set
  (`docs/technical-specifications.md` line 208) **and** is a freestanding header the reference
  compiler provides too, so it compiles identically under both oracles and the program remains a
  single-file reproducer.

**Exception 2 — `12_preprocessor/003_bundled_header_inclusion.c`: the nine required bundled headers.**

- **Scope:** that one program, and no other. It is the **dedicated probe for the bundled header set**,
  so it may include the nine **required** freestanding headers — `stddef.h`, `stdint.h`, `stdarg.h`,
  `stdbool.h`, `limits.h`, `float.h`, `stdalign.h`, `stdnoreturn.h` and `iso646.h`.
- **Reason:** it is the only place the suite exercises `include/` at all. Restricting it to
  `<stdarg.h>` would leave eight of the nine shipped headers never included by anything, which is a
  coverage hole rather than a discipline.
- **Note:** these nine are freestanding headers that the reference compiler also provides, which is
  what keeps the program compilable under both sides of oracle (a).

**Obligations and limits that apply to both exceptions.**

- **Every program taking either exception must state the exception and its reason in its `ub_notes`.**
- **The bonus `stdatomic.h` is deliberately excluded from both.** It is not among the nine required
  headers, it is not mandated by any requirement, and atomics can require `-latomic`, which is not in
  the shared flag set.
- **No other header is permitted anywhere in the corpus.** In particular no `stdio.h` (`bcc` ships
  none), no `wchar.h`, no `uchar.h` and no `string.h`. Every other program — all of them — includes
  nothing and hand-declares the single libc prototype it needs.

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

Every program passes through two gates. The driver performs them once per run, `infra_ub_audit_gate`
reports them in full, and **every feature area asserts on the gates covering its own programs** — see
[Two of those four are gates](#two-of-those-four-are-gates-and-every-area-is-judged-against-them).

### The warning gate

```text
-Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror
```

A record that omits `ub_audit_flags` accepts this default, and that is the normal case.

**There are exactly two sanctioned deviations.** Each obliges the record to carry a non-empty
`impl_defined_notes`, and each obliges the reason to **name the dropped flags** — the exact spelling
of every flag it drops, leading hyphen included. `impl_defined_notes` is the **one** field that
reason is read from, and `ub_notes` is **not searched for it at all**: a reason recorded only there
leaves the record unloadable, and the diagnostic names the flag left unexplained rather than the
field. The full obligation, and the reason for keeping the two fields to one question each, is
stated under [Recorded reasons](#every-validation-the-parser-enforces):

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

The gate genuinely bites rather than decorating: it rejected the author's own probe program on a real
diagnostic during design.

**The volume, counted two ways, because the two numbers are different.** At the final planned corpus
of 108 programs the audit produces **216 gate results** — 108 programs × 2 gates — and it performs
them with **324 process invocations**, because a program costs three processes: one warning-gate
compile, one sanitizer build, and one sanitizer **run** (the sanitizer gate only means anything if the
instrumented binary is actually executed). At the 73 programs committed today that is **146 gate
results** from **219 process invocations**. The audit prints both figures and reconciles them, so a
missing invocation is visible rather than inferred.
`AuditReport::gate_result_count` counts the gate applications and the report tabulates them;
`AuditReport::invocations_expected` states the invocation figure and `invocations_performed`
states what actually happened, so the two are equal exactly when every gate was applied — which
is how a degraded run stays visible instead of looking complete.

Alongside the machine half, each program carries a **written** undefined-behaviour-freedom argument
in its `ub_notes`. The gates are the machine half of that guarantee; `ub_notes` is the human half,
and it is what a reviewer reads first when a divergence appears. Its **presence** is enforced, not
merely encouraged: the record parser refuses a record whose `ub_notes` key is absent, and the warning
gate records a defect — which fails the gate — for one that is present but empty. What no gate can
decide is whether the argument is *convincing*, which is why the audit report also states, per
feature area, how many programs carry one.

### What the audit report states

`infra_ub_audit_gate` prints one report covering every audited program, and two of its sections exist
to make a *systematic* omission visible where a per-program failure entry cannot:

- **`per-feature-area coverage`** — one row per feature area: how many programs it holds, how many
  satisfied both gates, how many had a gate that could not be applied, how many deviate from the
  default warning gate, and how many carry a written argument. A program without one is named
  individually beneath its area. Areas appear in corpus order, so two runs produce identical text.
- **`gates that could not be applied`** — every gate that could not run, with its program and the
  diagnosis of why. An absent reference compiler makes this section 2 × the program count, each entry
  explaining itself; it is never reported as a pass.

The list of gate members a deviation may **never** drop is derived from the gate table rather than
written out in prose, so the paragraph describing the policy cannot fall out of step with the policy
the audit actually applies.

---

## Flag discipline

Only flags that both compilers honour with the same meaning may be passed identically to both. That
is verified rather than assumed: the probe asserts an **observable consequence** per flag, not mere
acceptance, and asserts **negatively** that no non-shared flag has leaked into the shared set. The
driver performs it once per run, `infra_flag_capability_probe` reports it in full, and — because flag
parity is a property of the configuration rather than of any one program — **every feature area asserts
on it**; see
[Two of those four are gates](#two-of-those-four-are-gates-and-every-area-is-judged-against-them).

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
- **Never pass `-mretpoline`.** It is a `bcc`-only hardening spelling with no reference-compiler
  counterpart, and it is not a target selector, so no invocation the suite makes — on either side,
  under any oracle — carries it.
- **`--target` and `--sysroot` are `bcc`-only *target selectors*: forbidden in the shared flag set
  and on every reference invocation, and admissible on the compiler-under-test side alone.** That
  split is deliberate rather than a loophole, because the two sides select a target by *different
  mechanisms* and neither mechanism exists on the other side:
  - The **reference** side has no target-selection flag, so a reference invocation carries **no**
    target argument whatsoever and picks its architecture by **driver binary** instead.
  - The **compiler under test** has no cross drivers, so `--target <triple>` is its only route to a
    non-native backend. It is therefore passed on `bcc`'s side of **oracle (a)'s cross arm exactly
    as much as under oracle (b)** — *not* only under oracle (b). It is **required** for every
    non-native cell, and each record passes it on the native cell too so that one recorded command
    template stays correct for all twelve cells of a program.

  Enforcement is mechanical on every invocation rather than a convention to remember: at most one
  target selection may be in force, and when `--target` is present its triple must be the cell's
  own. A violation is a hard failure naming the flag, the side and the whole command line.

  Oracle (b) is not where the selector is *used*; it is where the selector is *varied*.
  `is_forbidden_for_side` in `../conformance_harness/mod.rs` is the single place the whole split is
  expressed.

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
| i686 | `i686-linux-gnu` | 4 | 4 | **ELF32** | little | `qemu-i386` *or* `qemu-i386-static` |
| AArch64 | `aarch64-linux-gnu` | 8 | 8 | ELF64 | little | `qemu-aarch64` *or* `qemu-aarch64-static` |
| RISC-V 64 | `riscv64-linux-gnu` | 8 | 8 | ELF64 | little | `qemu-riscv64` *or* `qemu-riscv64-static` |

Sourced from `docs/technical-specifications.md` lines 457–462.

**CRITICAL:** the i686 runner is **`qemu-i386`**, not `qemu-i686`. The emulator's architecture name is
**i386**, so this is the one runner that is *not* named after its target: do not derive the runner
name from the `i686` in the triple. Source: `docs/project-guide.md` lines 420, 424, 428 and 568.
Getting this wrong makes the entire i686 arm silently unavailable.

**Which spelling exists is a property of the distribution, not of the suite.** The harness probes the
**plain** spelling first and then the `-static` spelling for each architecture, so either packaging
works with no configuration:

- the requirements name the plain spelling, which is what Ubuntu 25.10's `qemu-user` package installs —
  on that release `/usr/bin/qemu-<arch>-static` does not exist at all;
- Ubuntu 24.04 LTS and earlier, and Debian, install `/usr/bin/qemu-<arch>-static` from the real
  `qemu-user-static` package, and may not provide the plain spelling.

Both cases are recorded in full, with the measured evidence, under
[Why `qemu-user` and not `qemu-user-static` on Ubuntu 25.10](#why-qemu-user-and-not-qemu-user-static-on-ubuntu-2510).
Where a command in this file has to pick one spelling in order to be copy-pasteable, it picks the plain
one and says so.

**Optimization matrix:** exactly `{-O0, -O1, -O2}`. No other level is in scope.

### The enumerable matrix

Two columns, deliberately. The **final planned target** is what the suite's design calls for; **the
committed corpus** is what a reader can count in this directory right now, with five of the fourteen
areas not yet landed. Quoting the planned column as though it described the present state would be
exactly the unverifiable claim the paragraph below refuses to make.

| Quantity | Final planned target | Committed today |
| --- | --- | --- |
| Feature areas | 14 | **9** |
| Programs | 108 | **73** |
| Optimization levels per program | 3 | 3 |
| Targets per program | 4, unless the record restricts them with a recorded reason | 4, same rule |
| **`bcc` compile-and-run cells** | **1,296** (108 × 4 × 3) | **876** (73 × 4 × 3) |
| Reference cells, native | **324** (108 × 3) | **219** (73 × 3) |
| Reference cells, cross | up to **972** (108 × 3 × 3) | up to **657** (73 × 3 × 3) |
| Oracle (a) comparisons | **1,296** | **876** |
| Oracle (b) comparisons | **972** | **657** |
| Oracle (c) assertions | **1,296** | **876** |
| **Total differential and golden assertions** | **≈3,564, from 108 programs** | **≈2,409, from 73 programs** |

The five areas still to land are `02_constant_expressions`, `03_initializers`, `12_preprocessor`,
`13_floating_point` and `14_abi_calling_convention`; the per-area table below marks each one.

**Never publish a coverage percentage in this file or in either register.** Coverage instrumentation
requires a development dependency, which this repository forbids absolutely — so no percentage in this
repository is measurable, and publishing one would be fabrication. The matrix above **is** the coverage
evidence: the committed column is countable directly from the file set, and the run summary re-reports
what it discovered on every execution. `report.rs` treats a discovered count that disagrees with the
figures the harness holds as a corpus defect, so the numbers cannot silently drift away from the files.

### The 14 feature areas

`Programs` is the planned count for each area. `State` says whether that area is committed in this
directory today or still to land.

| Area directory | Programs | Mandated | State |
| --- | --- | --- | --- |
| `01_integer_conversions` | 10 | yes | committed |
| `02_constant_expressions` | 8 | yes | **planned** |
| `03_initializers` | 11 | yes | **planned** |
| `04_bitfields` | 7 | yes | committed |
| `05_pointers` | 10 | yes | committed |
| `06_control_flow` | 10 | yes | committed |
| `07_variadics` | 6 | yes | committed |
| `08_gcc_extensions` | 8 | yes | committed |
| `09_optimization_levels` | 8 | yes | committed |
| `10_declarations_and_types` | 10 | supplementary | committed |
| `11_literals_and_strings` | 4 | supplementary | committed |
| `12_preprocessor` | 6 | supplementary | **planned** |
| `13_floating_point` | 4 | supplementary | **planned** |
| `14_abi_calling_convention` | 6 | supplementary | **planned** |
| **Planned total** | **108** | | **73 committed across 9 areas** |

The nine mandated areas are the acceptance floor set by the requirements; each carries no fewer than
six programs. The five supplementary areas were added because they carry the widest cross-backend
divergence surface. Six of the nine mandated areas are committed; the remaining three and two of the
five supplementary areas are still to land, which is what the `State` column and the committed column
of the matrix above both record.

### Measured performance budget

- ≈**72.5 ms** per compile-and-run pair, including emulator startup (36 pairs completed in 2.612 s
  wall time).
- The **final planned** matrix is ≈2,592 compile-and-run pairs, so ≈**188 s serially**, and well under
  a minute spread across the harness's default thread pool given 14 independent area tests. The
  **committed** matrix is ≈1,752 pairs, so ≈**127 s serially**.
- A per-cell timeout bounds any runaway execution, and a timeout is classified as a divergence
  rather than swallowed as an infrastructure error.

---

## Running the suite

### The Cargo integration precondition

**Every command in this section — and every quality gate this suite is measured by — requires the
repository's Cargo package to be present in the checkout.** That means a `Cargo.toml` at the repository
root declaring the `bcc` binary target, the `src/**` tree it builds from, and the existing `tests/`
directory. Auto-discovery is what makes `cargo test --test conformance` work without a manifest *change*,
but auto-discovery still presupposes a manifest to be discovered *from*.

**This suite does not supply that manifest and must never add one.** C1 makes `Cargo.toml` read-only, and
the whole no-manifest-change property described under
[the structural fact that governs this whole directory](#the-structural-fact-that-governs-this-whole-directory)
depends on the file being left exactly as the compiler's own branch has it. Creating one here would
satisfy a gate by violating the constraint the gate exists to protect.

So on a checkout that carries this suite **ahead of** the compiler tree — a documentation-only branch, or
this suite reviewed on its own before it is merged — the Cargo gates do not run. That is a property of
the checkout, not a defect in the suite, and it resolves the moment the two are on one branch. What can
and cannot be established in each case:

| Check | Documentation-only checkout | Checkout with the Cargo package |
| --- | --- | --- |
| `rustfmt --edition 2021 --check` on each `.rs` file directly | ✅ runs — needs no manifest | ✅ runs |
| `rustc --edition 2021 --test --emit=metadata tests/conformance.rs` | ✅ runs — type-checks the whole suite, needs no manifest | ✅ runs |
| `cargo test --test conformance --no-run` | ❌ **blocked** — no manifest to discover the target from | ✅ runs |
| `cargo clippy -- -D warnings` | ❌ **blocked** — clippy drives Cargo | ✅ runs |
| `cargo fmt -- --check` | ❌ **blocked** — `cargo fmt` drives Cargo | ✅ runs |
| `cargo test --test conformance` (execution: 1,296 cells) | ❌ **blocked**, and additionally there is no `bcc` to test | ✅ runs |
| Whole-repository health gate `cargo test` | ❌ **blocked**, and the existing suites are not present either | ✅ runs |

The two direct invocations in the first two rows are not a substitute for the Cargo gates and are not
presented as one. They establish the properties that do not depend on packaging — that every file parses,
type-checks and is correctly formatted — which is exactly the subset a manifest-less checkout can honestly
claim. The rest is established by placing the suite in a package.

**Establishing the Cargo gates without adding a manifest to this repository.** The suite's own files are
copied, byte-for-byte unmodified, into a scratch Cargo package created **outside** the repository, which
supplies only the two things the checkout is missing: a minimal `Cargo.toml` with empty dependency
sections, and a `bcc` binary target. `cargo test --test conformance --no-run`,
`cargo clippy -- -D warnings` and `cargo fmt -- --check` then measure exactly these files, because these
files are what the package contains. The scratch package is never committed and never placed inside the
repository, so it cannot become the manifest C1 forbids, and it is not a fixture: nothing in the suite
refers to it, and the suite is unaware it exists.

**What that arrangement can and cannot establish, stated plainly.** It establishes everything *static*:
that the suite compiles as a Cargo integration target, that it is clippy-clean and rustfmt-clean, and
that the harness drives an entire matrix end to end — discovery, workspaces, both compilers, all four
targets, all three oracles, classification, finding artifacts and the run summary. It establishes
**nothing about the real `bcc`**, because the binary target in a scratch package is a stand-in and not
the compiler. Any verdict produced there is a verdict about the stand-in. Judging `bcc` requires the
real binary, which requires the real package — which is the merge described below, and is the only
place the suite's actual purpose can be served.

Because the compiler under test is resolved at **run time** — `option_env!` on Cargo's binary-path macro
plus the `BCC_BIN` override, rather than the compile-time `env!` form — a package that has no `bcc` binary
target still compiles the suite and fails with an explanatory message naming the missing binary, instead
of an inscrutable compile error. That is what makes the arrangement above possible at all.

**The resolution is merge, not manifest.** Apply this suite onto the branch that already carries the Cargo
package, the `bcc` binary target, the compiler source tree and the existing sixteen integration suites,
then run every gate in the third column there. Nothing in the suite needs to change for that to work: it
is an auto-discovered `tests/<name>.rs` target with a nested non-target helper directory and a data-only
corpus directory, which is precisely the shape that merges without touching the manifest.

**What a blocked gate looks like.** Every blocked command above exits `101` with `could not find
Cargo.toml`. That includes the two infrastructure tests which otherwise need no toolchain at all —
the files they read are committed and present — because the runner itself cannot be started. Nothing
here is broken by that: the gate is simply not invocable until the package is present, and it is
stated so that a reader who tries one of these commands on a documentation-only branch knows
immediately which of the two situations they are in.

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

All fourteen are declared in the driver. Five of them — `area_02_constant_expressions`,
`area_03_initializers`, `area_12_preprocessor`, `area_13_floating_point` and
`area_14_abi_calling_convention` — name area directories that have **not landed on this branch yet**,
so they cannot pass here: corpus discovery treats a missing area directory as a corpus defect rather
than as an empty area, which is deliberate, because silently reporting success for zero programs is
the one failure mode a coverage claim must never have.

Plus four infrastructure tests:

```text
infra_flag_capability_probe        infra_expected_divergence_register
infra_ub_audit_gate                infra_oracle_capability_report
```

### Two of those four are gates, and every area is judged against them

`infra_flag_capability_probe` and `infra_ub_audit_gate` do not compare anything. They establish the
two **preconditions** the differential oracles rest on: that every flag a differential invocation
passes means the same thing to both compilers (requirement 3), and that every program in the corpus is
free of undefined behaviour (requirement 1). Requirement 1 states the consequence in its own terms — a
program containing undefined behaviour permits both compilers to do anything — so while either
precondition is unmet, a PASS is not evidence of agreement and a divergence is not evidence of a
defect.

Two ordinary tests cannot express that. The built-in harness runs all eighteen concurrently in one
process with no ordering between them, so "run the gates first" is not something you can arrange and
not something a test can assert; and a failing gate test would sit beside fourteen area tests each
reporting a green matrix whose comparisons are not evidence. What the driver does instead:

- each gate is performed **exactly once per process**, memoized, so the cost — two reference-compiler
  invocations per corpus program for the audit, and a compile with each compiler per flag for the
  probe — is paid once rather than once for the gate test and again for the areas;
- every area performs the gates **before its first cell compiles**, so the result is recorded before
  any artifact is written and **every** artifact names it;
- every area **asserts** on the gates that govern it, after its report has been written. A failing gate
  therefore fails the areas it bears on, not only the infrastructure test that noticed it.

The audit's gates are narrowed to the areas whose programs they cover, because a program in one area
that fails a gate says nothing about another area's programs, and failing all fourteen for it would
report fourteen defects where there is one. Flag parity is a property of the configuration rather than
of any program, so its gate governs every area.

An area that `BCC_CONFORMANCE_ONLY` excluded does not assert on the gates: it produced no comparison,
so it has nothing to distrust. The gate still fails the run, through its own infrastructure test and
through the summary's verdict.

The two infrastructure tests keep their own, fuller assertions. They render the **whole** gate report —
every command line and the compiler's own words — which is what an author actually fixes a program
from, and what an area's one-line gate description deliberately does not try to be.

### What every report says about the gates

Every per-area report and the run summary open with a **`## Preflight gates`** section listing each
gate, the requirement it establishes, its verdict and what was observed. Three verdicts are used and
they are not interchangeable:

| Verdict | Meaning | Effect |
| --- | --- | --- |
| `HELD` | performed, and its precondition holds | none |
| `FAILED` | performed, and its precondition does not hold | **always** blocks; the report is stamped **partial** and every area it governs fails |
| `UNPERFORMED` | could not be performed, so nothing was established either way | the report is stamped **reduced**; blocks only under `BCC_CONFORMANCE_STRICT` |

`UNPERFORMED` follows the rule this suite already applies to an oracle whose tooling is absent: outside
strict mode the environment rather than the corpus is what is incomplete, so the gap is reported and
does not fail; under `BCC_CONFORMANCE_STRICT` — the intended continuous-integration setting, where the
toolchain is installed deliberately — it does. A gate that could not be performed is never silently a
pass, in either mode.

A run that recorded **no** preflight at all is treated as fail-closed: the section says `NOT RECORDED`,
the report is stamped partial, and the blocking count reads `1` rather than `0`, because a zero would be
indistinguishable in every table and every machine-readable field from a preflight that ran and held.

In the summary's machine-readable half the same facts appear as `meta` rows `preflight_gates_blocking`
and `preflight_held`, and as one `preflight` record per gate carrying its name, its requirement, its
verdict and its detail. The `meta` row `run_fails` accounts for the gates as well as the outcomes:
`outcomes_failing_run` and `preflight_gates_blocking` are published separately so a reader can tell
which of the two it was, but an aggregator reading `run_fails` alone can never see `false` while a
precondition was unmet.

### What a retained flag-probe or audit workspace holds

The two probes are held to the same evidence contract as a corpus cell, and it is literal: **every**
compiler and program invocation a probe makes persists its full standard output, its full standard
error and its raw wait status into the probe's own workspace, and it does so at the moment the process
is reaped — **before** any check has decided whether it passed. A check does not know it has failed
until after the invocation it is judging, so recording only on failure would mean the evidence for a
failure was never captured; and `BCC_CONFORMANCE_KEEP_WORK` asks for a passing run's evidence too.

Probe workspaces live beneath the same work root as the corpus cells, under two reserved names that
no cell may use: `target/conformance-work/_flagprobe/<flag>/` for one flag check, and
`target/conformance-work/_ubaudit/<area>/<program>/<gate>/` for one gate applied to one program.

Flag-probe captures are named `{bcc|ref}.{compile|run}.{NN}.{stdout,stderr,exit}`, where `NN` is a
two-digit ordinal allocated per workspace. The ordinal is what makes several invocations in one
workspace legible: the `-O` levels check builds and runs three configurations with each compiler, and
a fixed name would leave only the last of them on disk. A failed check's report row lists its
`capture:` stems positionally beside its `command:` lines, so a reader can go straight from the
command that failed to the bytes it produced.

Audit-gate captures follow the same contract: the warning gate persists its compile, and the sanitizer
gate persists both its build and the instrumented run it then performs.

### Why each area is one batch test rather than one test per program

The built-in test harness stops a test at its **first** failed assertion, and the final deliverable
requires a summary enumerating **every** outcome — every expected divergence with its documented
basis, and every finding with its reproducer. Stopping at the first divergence would truncate
exactly the artifact the requirements ask for.

Each area test therefore runs its entire matrix, accumulates every verdict, and only then asserts —
first that every preflight gate governing it held, then that no cell produced a FAIL or an XPASS. The
gate comes first because it decides what the verdicts are worth: an area whose precondition is unmet is
not a narrower run but a run whose comparisons cannot be read as evidence, so reporting its outcome
tally as the verdict would publish a green matrix nobody can rely on. The failure message reproduces
the **complete** per-program table either way, so nothing is lost in the runner output.

### Reduced runs are always stamped as reduced

`BCC_CONFORMANCE_QUICK` restricts the matrix to the native target at `-O0` and `-O2` only. It is
**never the default**, and every report it produces is **stamped as reduced coverage**, so a quick
run can never be mistaken for a full one. The same stamping applies when a name filter or
`BCC_CONFORMANCE_ONLY` ran only a subset: the summary is explicitly labelled partial.

### Pre-flight check

Run `infra_oracle_capability_report` first in any new environment. It prints the discovered oracle
inventory and states exactly which arms of which oracles will run, so a misconfigured environment is
diagnosed **before** the matrix executes — 876 `bcc` cells on the committed corpus, 1,296 once all
fourteen areas have landed.

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
| `BCC_CONFORMANCE_STRICT` | unset | Treat an unavailable oracle — and an `UNPERFORMED` preflight gate — as a failure. The intended continuous-integration setting |
| `BCC_CONFORMANCE_ALLOW_XPASS` | unset | Downgrade unexpected success from a failure to a warning during a marker-retirement window |
| `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` | unset | Explicitly acknowledge a reduced-oracle environment; the gap is still reported, and under strict mode it is still a failure |
| `BCC_CONFORMANCE_TIMEOUT_SECS` | `30` | Per-cell execution budget, in whole seconds. Accepted range **1–3600**, narrowing to **1–300** when `BCC_CONFORMANCE_STRICT` is set |
| `BCC_CONFORMANCE_KEEP_WORK` | unset | Retain every cell workspace instead of removing it on success |

Configuration is read **once** into a single validated snapshot before any cell runs, so two
concurrently executing area tests cannot observe different policies and the report cannot describe a
policy other than the one applied. A malformed value is a hard error naming the variable; an invalid
value is never treated as absence, because falling back to a probed default there would test a
different tool than the one that was named.

**The per-cell budget is bounded at both ends.** `BCC_CONFORMANCE_TIMEOUT_SECS` must be a whole
number of seconds; anything else is a hard error naming the variable. Zero is rejected outright,
because a budget of zero would time out every cell before it could run. The upper bound depends on
the mode:

| Mode | Accepted range | Ceiling |
| --- | --- | --- |
| Interactive (default) | 1–3600 seconds | 3,600 |
| `BCC_CONFORMANCE_STRICT` set | 1–300 seconds | 300 |

The stricter continuous-integration ceiling is **a constant of the suite and cannot be raised by any
variable** — there is no override, and setting a larger value under strict mode is an error rather
than a value that is silently clamped. The reason a ceiling exists at all is that a budget large
enough to outlive the run is not a long timeout but the *absence* of one: the runaway-process bound
would be gone while the configuration still claimed to have one. Probes are bounded separately and
independently by a fixed 5-second deadline, which this variable does not affect.

**The four `BCC_REF_CC*` defaults are unversioned names, and that is deliberate**: a default cannot know
which major version a distribution has put behind `gcc`. Set them explicitly whenever the unversioned
driver is not the **gnu17** one — which is the case on the host recorded under
[Toolchain of record](#toolchain-of-record), where `gcc` is 15.2.0 and defaults to gnu23. The reason this
matters rather than being cosmetic is given under
[Why the reference drivers are pinned to `gcc-13`](#why-the-reference-drivers-are-pinned-to-gcc-13):
no `-std` flag is ever passed, so the reference compiler's *default* mode is the only thing that selects
the language it compiles.

**Locating the compiler under test.** By default the suite resolves `bcc` through the Cargo-provided
`CARGO_BIN_EXE_bcc` path, which points at the **freshly built** binary. That is what guarantees the
suite never silently validates a **stale build**. `BCC_BIN` overrides it when the subject is an
externally supplied binary.


---

## Environment setup

### Rust toolchain

```bash
rustup toolchain install 1.93.1
rustup default 1.93.1
```

The documented Rust minimum is **1.70+**, edition 2021; 1.93.1 stable matches the documented build.

### Reference compilers, emulators and cross C runtimes

**Package names differ between distribution releases, and three of the obvious choices are traps.** Pick
the line that matches the host, then read the three notes below. Each note records a choice that installs
successfully and *looks* right — `qemu-user-static`, the unversioned `gcc`, and `libc6-dev-i386` — while
failing to deliver what the suite actually needs. None of the three announces itself: two produce a
silently unavailable oracle arm and the third produces a compiler that quietly compiles a different
language.

**Ubuntu 25.10, and any release where `apt-cache policy qemu-user-static` reports no candidate:**

```bash
apt-get install -y \
  gcc-13 gcc-13-i686-linux-gnu gcc-13-aarch64-linux-gnu gcc-13-riscv64-linux-gnu \
  qemu-user \
  libc6-dev libc6-dev-i386-cross libc6-dev-arm64-cross libc6-dev-riscv64-cross
```

**Ubuntu 24.04 LTS and earlier, and Debian, where `qemu-user-static` is a real package:**

```bash
apt-get install -y \
  gcc-13 gcc-13-i686-linux-gnu gcc-13-aarch64-linux-gnu gcc-13-riscv64-linux-gnu \
  qemu-user-static \
  libc6-dev libc6-dev-i386-cross libc6-dev-arm64-cross libc6-dev-riscv64-cross
```

Name the pinned drivers to the harness, because the unversioned `gcc` is not necessarily the right one
(see below):

```bash
export BCC_REF_CC=gcc-13
export BCC_REF_CC_I686=i686-linux-gnu-gcc-13
export BCC_REF_CC_AARCH64=aarch64-linux-gnu-gcc-13
export BCC_REF_CC_RISCV64=riscv64-linux-gnu-gcc-13
```

#### Why `qemu-user` and not `qemu-user-static` on Ubuntu 25.10

On 25.10 `qemu-user-static` is a **virtual** package: `apt-cache policy` reports no installed version
**and no candidate**, and `apt-cache showpkg` reports an empty version list. It is merely *provided by*
`qemu-user-binfmt`. So `apt-get install -y qemu-user-static` does not fail outright — it silently
resolves to `qemu-user-binfmt`, which is worse than failing, because

- it registers system-wide `binfmt_misc` handlers that this suite never uses and does not want: every
  runner is invoked **explicitly** by name, so implicit interpreter registration only changes the host's
  global behaviour; and
- it still ships **no** `qemu-<arch>-static` binaries. On 25.10 `qemu-user` installs the **plain**
  spellings only — `/usr/bin/qemu-i386`, `/usr/bin/qemu-aarch64`, `/usr/bin/qemu-riscv64` — and
  `/usr/bin/qemu-*-static` does not exist at all.

On Ubuntu 24.04 LTS and earlier, and on Debian, `qemu-user-static` **is** a real package and installs
`/usr/bin/qemu-<arch>-static`. That is the historical source of the `-static` spelling, and it is why
the requirements name the plain spelling while some environments only have the suffixed one.

**Either packaging works with no configuration**, because the harness probes the plain spelling first
and then the `-static` spelling for each architecture; see the `BCC_QEMU_*` defaults in
[Environment variables](#environment-variables). Only the *documentation* has to know which release it
is describing — the harness does not.

#### Why the reference drivers are pinned to `gcc-13`

Measured on 25.10: the unversioned `gcc` is **15.2.0**, whose default mode is **gnu23**
(`__STDC_VERSION__` = `202311L`), while `gcc-13` is **13.4.0**, whose default mode is **gnu17**
(`__STDC_VERSION__` = `201710L`).

That difference cannot be papered over with a flag, because **no `-std` flag is ever passed** — `bcc`
has none, so passing one to the reference compiler alone would break the shared-flag discipline that
[Flag discipline](#flag-discipline) exists to enforce. The reference compiler's *default* mode is
therefore load-bearing, and the gnu17 driver has to be the one that is named. Pin it explicitly rather
than relying on whatever `gcc` happens to resolve to, on every release.

#### Why the i686 runtime is `libc6-dev-i386-cross` and not `libc6-dev-i386`

Both packages exist and both install successfully, but they serve different drivers:

- `libc6-dev-i386-cross` installs `/usr/i686-linux-gnu/lib/{libc.a,crt1.o,crti.o,crtn.o}`, which is
  what the **i686 cross driver** resolves. Verified directly:
  `i686-linux-gnu-gcc-13 -static -print-file-name=libc.a` resolves inside `/usr/i686-linux-gnu/lib/`.
- `libc6-dev-i386` installs `/usr/lib32/…`, which is the **multilib** set that only `gcc -m32` uses —
  and `-m32` is excluded outright under [Prohibitions](#prohibitions) and fails on this host anyway.

Installing only `libc6-dev-i386` therefore leaves the i686 arm with no static C runtime, and every i686
cell fails at the link step. Install the `-cross` package; `libc6-dev-i386` is not needed at all.

### Verify before running the full matrix

Every line below is copy-pasteable and was run in the environment recorded under
[Toolchain of record](#toolchain-of-record):

```bash
gcc-13 --version                    # expect 13.4.0
i686-linux-gnu-gcc-13 --version     # expect 13.4.0
aarch64-linux-gnu-gcc-13 --version  # expect 13.4.0
riscv64-linux-gnu-gcc-13 --version  # expect 13.4.0

# The reference compiler's DEFAULT mode must already be gnu17, because no -std flag is ever passed.
gcc-13 -dM -E -x c /dev/null | grep __STDC_VERSION__   # expect 201710L

# Emulators: the plain spelling on Ubuntu 25.10, the -static spelling on 24.04 LTS and earlier.
# The harness accepts either; run whichever the host installed.
qemu-i386 --version    || qemu-i386-static --version      # expect 10.1.0 on 25.10
qemu-aarch64 --version || qemu-aarch64-static --version   # expect 10.1.0 on 25.10
qemu-riscv64 --version || qemu-riscv64-static --version   # expect 10.1.0 on 25.10

# Static C runtimes: each must resolve inside its own target sysroot, not fail.
gcc-13                   -static -print-file-name=libc.a
i686-linux-gnu-gcc-13    -static -print-file-name=libc.a  # expect /usr/i686-linux-gnu/lib/...
aarch64-linux-gnu-gcc-13 -static -print-file-name=libc.a  # expect /usr/aarch64-linux-gnu/lib/...
riscv64-linux-gnu-gcc-13 -static -print-file-name=libc.a  # expect /usr/riscv64-linux-gnu/lib/...


**If you put a pinned driver on `PATH` under an unversioned name, use a script and not a symbolic
link.** GCC derives its exec prefix from `argv[0]`, so a symlink at `/usr/local/bin/gcc` makes it
search `/usr/local/libexec/gcc/…` and fail with `cannot execute 'cc1'`. A one-line script that
`exec`s the real driver by absolute path works, and naming the drivers through the `BCC_REF_CC*`
overrides avoids the question entirely.
cargo test --test conformance infra_oracle_capability_report -- --nocapture
```

The last line is the authoritative check and the only one that inspects the suite's own view of the
environment; the rest exist so that a failure is diagnosed against a single tool rather than against
the whole oracle inventory at once. It requires the Cargo package — see
[the Cargo integration precondition](#the-cargo-integration-precondition).

### Toolchain of record

**This table describes one specific host, named and dated. It is not a claim about any other.**
Measured, not assumed — every version below was obtained from the package manager and then confirmed
by invoking the tool; nothing is inferred from a package name or from an earlier measurement. When the
suite runs somewhere else, `infra_oracle_capability_report` reports that host's own inventory, and
*that* output — not this table — is what the environment fingerprint in a finding artifact records.

- **Distribution:** Ubuntu 25.10 (`VERSION_ID=25.10`)
- **Kernel:** 6.12.85+
- **Measured:** 2026-08-01

| Role | Tool | Version measured | Package |
| --- | --- | --- | --- |
| Reference compiler, native | `gcc` → `gcc-13` | **13.4.0**, default mode gnu17 | `gcc-13` `13.4.0-4ubuntu1` |
| Reference compiler, i686 | `i686-linux-gnu-gcc-13` | **13.4.0** | `gcc-13-i686-linux-gnu` `13.4.0-4ubuntu1cross1` |
| Reference compiler, AArch64 | `aarch64-linux-gnu-gcc-13` | **13.4.0** | `gcc-13-aarch64-linux-gnu` `13.4.0-4ubuntu1cross1` |
| Reference compiler, RISC-V 64 | `riscv64-linux-gnu-gcc-13` | **13.4.0** | `gcc-13-riscv64-linux-gnu` `13.4.0-4ubuntu1cross1` |
| Cross execution | `qemu-<arch>` — the **plain** spellings are what this release's package installs | **10.1.0** | `qemu-user` `1:10.1.0+ds-5ubuntu2.7` |
| Native C runtime and static libc | `libc6-dev` | **2.42** | `2.42-0ubuntu3.1` |
| i686 static C runtime | `libc6-dev-i386-cross` | **2.42** | `2.42-0ubuntu3cross1` |
| AArch64 C runtime | `libc6-dev-arm64-cross` | **2.42** | `2.42-0ubuntu3cross1` |
| RISC-V 64 C runtime | `libc6-dev-riscv64-cross` | **2.42** | `2.42-0ubuntu3cross1` |
| Per-cell timeout | `timeout` at `/usr/bin/timeout` | **uutils coreutils 0.2.2** | `coreutils-from-uutils` |
| Alternate reference oracle | `clang` | **20.1.8**, present | `clang` `1:20.0-63ubuntu1` |
| Reducer for finding minimization | `creduce` | **2.11.0**, present | `creduce` `2.11.0~20240909-2.1` |
| ELF inspection | `binutils` | 2.45 present, but **not a dependency** — the flag probe reads the identification bytes with `std` | `binutils` |

`gcc-13` is the reference compiler of record, and the version is not incidental: it is chosen for
its **gnu17** default, since no `-std` flag may be passed to either compiler. `clang` is selectable
through `BCC_REF_CC` where a second independent oracle is wanted; `creduce` is used for minimization
only when present, and is never required.

Three entries deserve a second look, because each was reasoned about from a stale assumption before
it was measured:

- **The i686 static C runtime comes from `libc6-dev-i386-cross`, not `libc6-dev-i386`.** Both
  packages exist and both install cleanly, but they serve different drivers: the `-cross` package
  owns `/usr/i686-linux-gnu/lib/{libc.a,crt1.o,crti.o,crtn.o}`, which is what the i686 cross driver
  resolves, while `libc6-dev-i386` owns `/usr/lib32/…`, the multilib set only `gcc -m32` uses — and
  `-m32` is excluded outright. Installing only the latter leaves the i686 arm with no static C
  runtime and every i686 cell failing at the link step.

- **The `timeout` implementation is `uutils`, not GNU coreutils**, and it is the *reason* the module
  documentation of `../conformance_harness/execute.rs` carries a measurement table and a
  standard-library safety net. Two of its three spellings were measured defective, which is why
  neither `--signal=KILL` nor `--kill-after` is ever passed. Do not assume GNU semantics from the
  command name.
- **`clang` and `creduce` are present here**, so neither optional capability is hypothetical on this
  machine. They remain optional in the sense that the suite is complete without them. Note that
  `clang`'s default language mode differs from the pinned reference driver's, so selecting it
  changes what the oracle compares against.
- **There is no binfmt registration**, so every QEMU runner is invoked explicitly, and if no
  `timeout` utility is found at all the harness falls back to a standard-library watchdog thread.

**The authoring fingerprint, kept only as history.** While this suite was designed the machine
reported gcc 13.3.0 (package `4:13.2.0-7ubuntu1`), QEMU 8.2.2 from a separate `qemu-user-static`
package, glibc 2.39, GNU coreutils `timeout`, and `clang`/`creduce` absent. Those figures appear
nowhere above and must not be read as current; they are recorded so that a finding captured under
them can be read in context, which is also why every finding artifact carries its own
`environment.txt`.

Each measurement this document relies on was re-taken against the toolchain in the table above, and
each one held:

| Re-confirmed measurement | Result |
| --- | --- |
| Shared flags accepted by all four reference drivers | `-c`, `-O0`, `-O1`, `-O2`, `-static`, `-g`, `-fPIC`, `-D`, `-U`, `-I` — all accepted |
| Reference compiler rejects `--target=<triple>` | `error: unrecognized command-line option '--target=aarch64-linux-gnu'` |
| `-m32` on this host | Fails — `cannot find -lgcc`, multilib absent |
| Reference default language mode | gnu17: `__STDC_VERSION__` = 201710, `__STRICT_ANSI__` undefined |
| `char` signedness | signed on x86-64 and i686, **unsigned** on AArch64 and RISC-V 64 |
| `sizeof(long)` / `sizeof(void *)` | 4 on i686, 8 on the other three |
| `sizeof(long double)` | 16 / 12 / 16 / 16 |
| Right shift of a negative, division and modulo signs, endianness | Arithmetic shift, truncation toward zero, dividend-signed remainder, little-endian — identical on all four |
| Exit status of `return 300` | **44**, which is why expected exit codes are confined to 0–125 |
| ELF identification the flag probe reads | type field at offset `0x10`: `1` object, `2` static executable, `3` dynamic; `PT_INTERP` present only when dynamic; `.debug_info` present with `-g` and absent without |
| `timeout` behaviour, all five documented spellings | `timeout 1 sleep 5` → 124 at 1.007 s; `timeout -s KILL 1 sleep 5` → 124 at **5.008 s**, so no signal is sent; `timeout -s KILL 1 <spin>` → **never returned**; `timeout -k 1 1 <spin>` → **125**; `timeout 1 <spin>` → 124 |
| 12-cell cross-target sweep, 4 targets × 3 levels | Every capture byte-identical, every exit status 0, and equal to the golden record |

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

The same rule governs a preflight gate that could not be applied, because it is the same shape of gap:
a tool this machine does not have. Outside strict mode it is reported as reduced coverage and the run
proceeds; under strict mode it fails. What it never is, in either mode, is a pass — see
[the gate verdicts](#what-every-report-says-about-the-gates).

---

## Reproducing a cell by hand

This is the operational payoff: **every cell is reproducible from the program source and its
expectation record alone** — with no harness, no Cargo and no Rust toolchain.

### Procedure

1. Open the program's `.expected` record.
2. Read `bcc_command`, `ref_command`, `run_command` and `expect_exit`.
3. Substitute the placeholders:
   - `$BCC` → the `bcc` binary;
   - `$REF_CC_<TRIPLE>` → the reference driver matching the target, and specifically the **gnu17**
     one: on the host recorded above that is the `-13` suffixed spelling and never the unversioned
     `gcc`, because no `-std` flag is passed and the driver's own default mode is what selects the
     language it compiles;
   - `<triple>` → the target triple, e.g. `aarch64-linux-gnu`. It is substituted on the **`bcc` side
     only**: the reference driver selects its target by *being* a different binary and receives no
     target flag;
   - `<opt>` → the optimization level, e.g. `-O2`;
   - `<src>` → the program's `.c` file;
   - `<out>` → an output path **inside a private working directory you created yourself**;
   - `<runner>` → **empty** for x86-64, otherwise the matching QEMU runner (`qemu-<arch>`, or
     `qemu-<arch>-static` where only that spelling is packaged).
4. Run each binary with **stdout redirected to its own file** and capture that binary's exit status
   in the same step — `cmd > "$work/x.stdout"; x_exit=$?`. The status is as much of the comparison as
   the bytes are, and `$?` is overwritten by the very next command, so capturing it later is too late.
5. Assert the statuses **before** looking at the bytes, because a status mismatch already settles the
   cell:
   - **oracle (a)** — the `bcc` status must equal the reference status for the same target and level;
   - **oracle (b)** — each non-baseline target's status must equal the x86-64 baseline status at the
     same level;
   - **oracle (c)** — the `bcc` status must equal the record's `expect_exit`.
6. Only then compare the captured streams byte-for-byte: `bcc` against the reference (oracle a), each
   non-baseline target against the x86-64 baseline (oracle b), and `bcc` against the record's
   `expected_stdout` (oracle c). `cmp` is the right tool, because it reports the offset of the first
   differing byte — the same thing the harness's own comparator reports.

Termination by a signal is not a normal exit of the same number. If you are reproducing a crash,
keep the two apart: a shell `$?` in the `128 + signal` range means the runner reported a signal, and
that is a different outcome from a program that chose to return the same value.

### Fully worked example

Cell: `01_integer_conversions/004_narrowing_conversions.c`, target `aarch64`, level `-O2`.

The record declares:

```text
bcc_command        = $BCC --target <triple> <opt> -static <src> -o <out>
ref_command        = $REF_CC_<TRIPLE> <opt> -static <src> -o <out>
run_command        = <runner> <out>
expect_exit        = 0
```

Every placeholder resolved. The script below is literal and copy-pasteable from the repository root.
It creates its own private working directory, keeps every artifact inside it, quotes every expansion,
uses no `eval`, and removes the directory on exit — including on failure:

```bash
#!/usr/bin/env bash
# Reproduce one cell by hand. Every artifact stays inside "$work", which is removed on exit.

# 0. Create the private scratch directory first, and never write to a predictable name.
#    `mktemp -d` creates a directory that did not previously exist, with mode 0700, under an
#    unpredictable name, so nothing can already be sitting at the paths used below. A fixed path
#    such as /tmp/bcc.out is shared and guessable: on a multi-user machine another user can
#    pre-place a symbolic link there, and the redirection in step 3 would then truncate whatever
#    that link points at, with your privileges. The umask protects the files created inside the
#    directory, and the trap removes the whole thing however the shell exits, interrupt included.
umask 077
work="$(mktemp -d)" || exit 1
trap 'rm -rf -- "$work"' EXIT INT TERM
printf 'scratch directory: %s\n' "$work"

src="tests/conformance/01_integer_conversions/004_narrowing_conversions.c"
rec="tests/conformance/01_integer_conversions/004_narrowing_conversions.expected"

# 1. Build with the compiler under test. --target is a bcc-only selector.
./target/debug/bcc --target aarch64-linux-gnu -O2 -static "$src" -o "$work/bcc.out" || exit 1

# 2. Build with the matching reference cross driver, which receives no target flag. The -13 suffix is
#    the gnu17 driver: no -std flag is passed, so the driver's own default mode selects the language.
aarch64-linux-gnu-gcc-13 -O2 -static "$src" -o "$work/ref.out" || exit 1

# 3. Run both under the target's runner, capturing stdout and exit status separately.
qemu-aarch64 "$work/bcc.out" > "$work/bcc.stdout"; bcc_exit=$?
qemu-aarch64 "$work/ref.out" > "$work/ref.stdout"; ref_exit=$?

# 4. Oracle (a): statuses first, then bytes.
if [ "$bcc_exit" -ne "$ref_exit" ]; then
  echo "oracle a: EXIT MISMATCH bcc=$bcc_exit ref=$ref_exit"
else
  cmp -- "$work/bcc.stdout" "$work/ref.stdout" && echo "oracle a: exit and stdout agree"
fi

# 5. Oracle (c): both recorded values, read out of the record itself rather than retyped.
awk '/^expected_stdout[[:space:]]+<<END$/{f=1;next} f&&/^END$/{exit} f' "$rec" \
  > "$work/golden.stdout"
want_exit="$(awk -F'=[[:space:]]*' '/^expect_exit[[:space:]]/{print $2; exit}' "$rec")"
if [ "$bcc_exit" -ne "$want_exit" ]; then
  echo "oracle c: EXIT MISMATCH got=$bcc_exit want=$want_exit"
else
  cmp -- "$work/golden.stdout" "$work/bcc.stdout" && echo "oracle c: golden record matches"
fi
```

Lifting `expected_stdout` and `expect_exit` out of the record with `awk` rather than retyping them is
deliberate. A retyped golden stream is a second, unverified copy of the expectation, and the whole
point of oracle (c) is to compare against the **committed** one.

For the x86-64 cell of the same program, `<runner>` is empty: steps 3 onward invoke `"$work/bcc.out"`
and `"$work/ref.out"` directly, and the reference driver in step 2 is the native gnu17 one — `gcc-13`
on the host recorded above, **not** the unversioned `gcc`, which is a later major version defaulting
to a later language mode.

Every path above is quoted, so a directory name containing a space or a shell metacharacter is
passed through as one argument rather than being split, and `cmp --` cannot mistake a name beginning
with a hyphen for an option. These are the same precautions the generated `commands.sh` takes,
described at the end of this section; reproducing a cell by hand should not be less safe than
reproducing it with the script.

For the x86-64 cell of the same program, `<runner>` is empty, so step 3 invokes `"$work/bcc.out"` and
`"$work/ref.out"` directly, and `$REF_CC_<TRIPLE>` in step 2 is the native `gcc`. The `--target` flag
in step 1 stays: it is passed on the `bcc` side of every cell, native included.

For oracle (b), build the same program for two targets at the same level into two paths beneath
`"$work"`, run each under its own runner, assert the two statuses match, and only then compare the
two captured streams — with the x86-64 cell as the baseline.

For a **finding**, none of this substitution is necessary: `commands.sh` inside the finding
directory already contains these lines, fully resolved, for every cell involved — bounded, checking
both stdout and status, and printing a single `RESULT:` line. Run it with `sh commands.sh`. It creates
its own private scratch directory with `mktemp -d` under a `077` umask, prints where that is, and
removes it again however the script exits — set `REPRO_KEEP=1` to keep it, or
`WORK=<an existing directory>` to write into one of your own, which the script then never creates and
never removes. Every scratch path it writes is refused rather than reused if something is already at
that name, so a planted symbolic link cannot turn one of its redirections into a write somewhere else
on your machine. See
[what a finding artifact directory holds](#what-a-finding-artifact-directory-holds).


---

## Directory layout and artifacts

```text
tests/conformance/
├── README.md                        this file — the suite contract
├── EXPECTED_DIVERGENCES.md          register of every expected-divergence marker
├── FINDINGS.md                      PLANNED — register of every finding
├── 01_integer_conversions/          10 programs: <NNN_name>.c + <NNN_name>.expected
├── 02_constant_expressions/         PLANNED —  8 programs
├── 03_initializers/                 PLANNED — 11 programs
├── 04_bitfields/                     7 programs
├── 05_pointers/                     10 programs
├── 06_control_flow/                 10 programs
├── 07_variadics/                     6 programs  (may #include <stdarg.h>)
├── 08_gcc_extensions/                8 programs  (warning gate drops -pedantic)
├── 09_optimization_levels/           8 programs
├── 10_declarations_and_types/       10 programs
├── 11_literals_and_strings/          4 programs
├── 12_preprocessor/                 PLANNED —  6 programs
├── 13_floating_point/               PLANNED —  4 programs
├── 14_abi_calling_convention/       PLANNED —  6 programs
├── support/
│   └── include/
│       └── probe_header.h           the suite's ONLY fixture
├── tools/                           PLANNED
│   └── regenerate_expected.sh       maintenance-only; never invoked by cargo test
└── findings/                        committed; holds only .gitkeep so far
    └── F-<digest>-<cell-slug>-<oracle>-<class>/   PLANNED — one curated, committed finding
        ├── reproducer.c
        ├── reproducer.expected
        ├── MANIFEST.txt
        ├── commands.sh
        ├── outputs/                 per cell: the program's stdout, stderr and exit status,
        │                            and the compiler's own stdout, stderr and outcome
        ├── environment.txt
        └── diff.txt
```

`PLANNED` marks an entry the plan specifies that has **not landed on this branch yet**. Everything
unmarked is committed and present. Nothing in this document links to a `PLANNED` path: a link that
resolves to nothing is worse than no link, because it reads as a promise the repository does not
keep.

### The finding identifier

A finding directory is named from the divergence itself and from nothing else, so the same divergence
always names the same directory and two different divergences can never name the same one:

```
F-<16 hex digits>-<cell slug>-<oracle letter>-<divergence class>
   e.g. F-9d3c1a5f7b204e68-04_bitfields+005_straddling_and_zero_width+aarch64+O2-b-stdout-mismatch
```

- the **cell slug** is the harness's own cell identity — area, program, target and optimization level
  — with every byte outside `[A-Za-z0-9_]` escaped as `%XX` and the four parts joined with `+`. It is
  **injective**: two different cells cannot produce the same slug, and nothing is abbreviated or
  truncated on the way in;
- the **oracle letter** is `a`, `b` or `c`;
- the **divergence class** is one of `compile-failure`, `link-failure`, `run-crash`,
  `exit-code-mismatch`, `stdout-mismatch` or `timeout`;
- the **digest** is a stable hash of exactly those same components, so it adds a short fixed-width
  handle to quote in conversation without becoming the thing that distinguishes two findings.

Because the slug, the digest and the oracle letter each contain no hyphen, the hyphens above are
unambiguous separators. **No part of the identifier is abbreviated**, so no two findings can collide
and overwrite one another's evidence.

Each generated finding directory also carries a `.run-owner` stamp. It is harness bookkeeping rather
than evidence — it lets a second, concurrent run detect that another run is still writing this
directory and refuse instead of purging it — and it is not part of the deliverable: no artifact may be
written to that name, and the completeness check does not look for it.

### The single fixture

[`support/include/probe_header.h`](support/include/probe_header.h) is the suite's **only** fixture,
used **solely** by the `-I` flag probe to prove that a command-line include directory is actually
searched: **with `-I` the compile succeeds; without `-I` it must fail.** The negative half is the
load-bearing one, so no copy of this header may exist anywhere else in the repository — a duplicate
on a default search path would make that half vacuous while the probe still reported success.

There is no fixture hierarchy, no factory and no test-data directory. Every other input in the suite
is a literal in a program's own source.

### Maintenance tooling

`tools/regenerate_expected.sh` is the plan's golden-record regeneration script. It is
**maintenance-only** and must **never be invoked by `cargo test`**: that separation is what stops a
wrong answer from quietly becoming the new expectation.

It is **not present on this branch**, so it is named rather than linked. The guarantee it supports
does not depend on it, though — `manifest.rs` has no writer, so no test run can rewrite a record
whether the script exists or not. What is missing today is only the *convenience* of regenerating a
record mechanically; the *protection* is already in place.

### The transient-versus-curated split

**Preserve this split.** It is what keeps an in-progress run from polluting a committed deliverable.

**Committed and tracked — this folder.** Present today:

- 73 programs and their 73 expectation records, across nine area directories;
- `support/`;
- two Markdown files: this contract and [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md).

Planned, and not yet present — named as plain text for the reason given under the layout tree:

- the remaining 35 programs and records, across the five unlanded areas, bringing the corpus to 108;
- `tools/` and `findings/`;
- `FINDINGS.md`.

**Transient and git-ignored — elsewhere entirely, beneath the build directory:**

| Path | Content |
| --- | --- |
| `target/conformance-work/` | Per-cell workspaces. **Retained** whenever the cell produced a `FAIL`, an `XPASS` or a `FINDING` outcome, and retained for **every** cell when `BCC_CONFORMANCE_KEEP_WORK` is set; **removed** otherwise, which means a cell whose outcomes were all `PASS`, `XFAIL` or `UNAVAILABLE`. A retained directory leaves behind exactly the artifacts needed to investigate it, and the run prints its path. A timeout is not a separate rule: it reaches the retained set by way of the `FAIL` or `FINDING` verdict it produces. There is deliberately no destructor that deletes, so a panicking cell cannot erase its own evidence |
| `target/conformance-report/run.txt` | Which run produced the reports in this directory: its run token, its configuration fingerprint and the four retention ceilings |
| `target/conformance-report/areas/<area>.md` | Per-area human-readable report |
| `target/conformance-report/areas/<area>.tsv` | Per-area machine-readable report — each area writes only its own file, so there is no contention under parallel execution. Its **first line** is the generation preamble described below; the column header is line two |
| `target/conformance-report/summary.md` | The deliverable summary |
| `target/conformance-report/summary.tsv` | The same data, machine-readable |
| `target/conformance-findings/F-<digest>-<cell-slug>-<oracle>-<class>/` | Auto-generated finding artifacts from the current run |

**`findings.rs` never writes into `tests/conformance/`.** The curated finding set and both registers
are human-maintained committed deliverables. A run writes only beneath the build directory.

Every write beneath the build directory goes through one publisher that refuses to follow a symbolic
link: the destination is inspected with `symlink_metadata`, the bytes are written to a fresh
temporary entry created with `create_new`, and the entry is then renamed into place. A name already
occupied by a link — even a dangling one — is refused rather than followed, so no planted link can
redirect a write, and no partially written report or finding file is ever observable.

#### What a published report cannot contain

A report is an artifact you publish — attached to an issue, uploaded from CI, committed beside a
finding — and the one thing in it the suite did not write is captured text: a compiler diagnostic, a
tool banner, a program's own output. Every such fragment reaches an artifact through one of three
funnels — one for the machine-readable half, two for the Markdown half — and each applies the same
three transformations in the same order:

1. **Redaction.** The value of every environment variable whose *name* marks it credential-bearing
   (`SECRET`, `TOKEN`, `PASSWORD`, `API_KEY`, `AUTH`, `SESSION`, … — deliberately broad, because a
   false positive costs one redacted field while a false negative commits a credential) is replaced
   by `[redacted]`: in the `NAME=value` shape an environment listing renders, at **every** length, and
   wherever the value appears bare provided it is at least **eight characters** long. That length
   bound is not fussiness. A variable named `TOKENIZERS_PARALLELISM` whose value is `false`, or
   `XDG_SESSION_ID` whose value is `1`, is not a credential — and replacing so short a value as a
   substring would rewrite the booleans and digits that carry the report's own evidence, `run_fails`
   included. Measured in this repository's own container: without the bound, the row `run_fails false`
   renders as `run_fails [redacted]`.
2. **Sanitization.** Every control character, every escape introducer and every directional override
   is escaped, so no field can forge a column, split one row into two or repaint a verdict.
3. **Markdown escaping**, applied only on the way into the Markdown half, so no printable character
   that is syntax there can restructure the document.

The order is load-bearing: redaction recognises a value by the characters the environment holds, and a
value containing a tab or a newline is no longer that value once it has been escaped — redacting
afterwards would search for text that no longer exists.

Two things are exact rather than redacted, and both live inside a finding directory beneath the build
directory: the captured streams and the `diff.txt` computed from them, which are evidence and have to
compare byte for byte, and `commands.sh`, which has to stay runnable — a redacted path is not a path.
Nothing a corpus program prints can carry a credential in any case: the authoring rules forbid reading
the environment, and the environment those programs receive is cleared before they run. Everything the
suite renders as a *report* is redacted.

### The report and generated-finding roots are retired at the start of every run

Report and finding paths are deterministic, so a previous run's `areas/09_optimization_levels.tsv`
sits exactly where this run's will, and so does its `summary.md`. The first thing the run does is
therefore **empty the report root and the generated-findings root, whole** — entry by entry, so a
reader holding a directory open keeps a valid handle, and never following a link.

The reason is that finalization would otherwise aggregate files that never described one run: a
reduced run's rows counted beside a full run's, or a finding that was fixed weeks ago presented as
current. Worse, a run that never reached finalization at all — a filtered run, or one that failed
partway — would leave the *previous* run's `summary.md` standing as though it were this one's. The
whole root is emptied rather than the two known artifacts named, because a list of names has to be
kept in step with the artifacts written into it and this has no list to fall behind.

The identity of the run is written to `run.txt` beside the reports, and everything the reports
themselves render is **deterministic**: every `.md` artifact, every data row and every summary field
is byte-identical across two runs with identical inputs, which is what makes them diffable. What they
carry for provenance is the **configuration fingerprint**, which is deterministic too — so a reduced
run's numbers can never be mistaken for a full run's.

The process token, which necessarily differs between runs, is written in exactly three places and no
others: `run.txt`, a finding's `environment.txt`, and the `token=` field of an area report's
[generation preamble](#the-generation-stamp--why-a-summary-never-reports-another-runs-results) — a
comment line whose only consumer is the machine check it serves. It appears in no rendered table, in
no summary field and in no diagnostic, which is why the determinism above still holds as stated.

One qualification, stated because it is real rather than hidden: the byte counts in
[`## Retained evidence`](#retention-budgets) reflect the order in which concurrently failing cells
reached retention. Every verdict, every row and every ordering elsewhere in the reports is a pure
function of the inputs, and a run in which nothing was pruned reproduces byte for byte; but on a run
where the retention ceilings are **contended** — hundreds of cells failing at once, so that whichever
cell concludes first claims the last of the budget — the retained totals and the set of pruning notes
may differ between two otherwise identical runs. The alternative would be to serialize retention
across the fourteen area threads, which would trade a real property (evidence bounded continuously,
as it is produced) for a cosmetic one.

The per-cell workspace root is deliberately **not** wiped: each workspace is purged as it is
allocated, so a cell always starts empty, while a second filtered run cannot destroy the retained
evidence of a run a maintainer is still reading.

### The generation stamp — why a summary never reports another run's results

A report on disk outlives the run that wrote it, so every machine-readable area report opens with a
line naming the run that produced it, the configuration it ran under, and the process that wrote it:

```text
#generation	run=3f9c1a04d7e5b268	config=quick:0;only:-;strict:0;allow_xpass:0;ack_missing:0;timeout:30;targets:x86_64+i686+aarch64+riscv64;levels:O0+O1+O2;oracles:a1111b0111c1111	token=6b0e4d21f8a37c95
```

Three fields, because they answer three different questions and the check needs all three:

| Field | Identifies | Derived from |
| --- | --- | --- |
| `run` | The **sweep**, as a digest — so a file written under settings this run did not use is recognised as foreign in one comparison | The effective matrix and policy, plus the test-name filters this process was started with, which decide which areas could run at all. Deterministic |
| `config` | Those settings **spelled out**, so a foreign file's mismatch can be explained rather than merely detected — "that report swept one target at two levels, this run sweeps four at three" is actionable where an opaque digest is not. One field per fact that changes what a report means, ending with a bit per oracle and target, so a machine missing one cross driver is distinguishable from a fully equipped one | The same settings. Deterministic |
| `token` | The **process** — so a file left behind by an earlier run of an *identical* configuration over an *identical* corpus is recognised too | This process. The one value anywhere in a report that is not a function of the inputs |

The first two are pure functions of the run's inputs, and that is exactly why they cannot tell a
repeat run from the run it repeats: a report an earlier process wrote under the same settings over the
same corpus is byte-for-byte a report this run could have written, so an identity built only from
inputs has nothing to object to. Emptying the report root at the start of a run is the primary defence
and it is **not** sufficient on its own — a purge that cannot remove an entry reports the failure and
the file survives it, which is precisely the case this check is the last line against. Hence the third
field; and hence the deliberately narrow shape of the exception it makes to the determinism rule. The
token appears in the `token=` field of this comment line and nowhere else in any report: in no
rendered Markdown, in no field of either summary half, and in no diagnostic — a token-only mismatch is
described in words rather than by quoting either token. Every artifact a maintainer diffs therefore
stays byte-identical for identical inputs.

**A preamble missing any of the three fields is treated as stale**, the token included. A file
carrying no token cannot be shown to belong to the process reading it, and treating its absence as
"belongs to whoever is reading" would reopen the hole the token closes — so a report written by a
harness that predates the token gets exactly the treatment one predating the whole preamble already
gets: re-run the area to replace it.

`run` and `config` are additionally printed as a `Generation:` line in each per-area Markdown report
and as the run identifier and configuration fingerprint in the summary's **Provenance** section. The
token is not, for the reason just given.

The summary aggregates **only** the area reports carrying the generation of the process reading them.
An area report from an earlier run is neither counted nor deleted: it is listed by name, with the run
that wrote it, and it holds the summary back until this run replaces it. A report written before the
stamp existed has no preamble and is recognised as foreign on its first line. Initialization has
already retired the previous run's reports by the time any area writes, so a foreign file is the
exception rather than the rule; the stamp closes the three cases initialization cannot — a second
suite process sharing one build directory, a file written before the stamp existed, and a file an
earlier identically configured run left behind because the purge could not remove it. It closes them
by identity carried inside the file, with no further destructive step and no lock, which is what the
fourteen concurrently running area tests require: a directory-wide delete taken after they start
would race with a sibling's write.

Two consequences worth knowing:

- **The reports stay byte-identical between runs.** Every Markdown artifact reproduces byte for byte,
  and so does every data row of every `.tsv`; the only thing that differs between two runs with
  identical inputs is the `token=` field of that comment line — plus the one qualification recorded
  above about contended retention ceilings. Diff two runs' area reports and anything else that differs
  is something the run genuinely found.
- **A run that could not aggregate all fourteen current reports says so.** With a test-name filter
  active it still publishes a summary — stamped partial, with every area that did not contribute
  listed as absent, stale or unusable — and without one it waits, printing `run summary pending`,
  which is the ordinary answer for thirteen of the fourteen area tests.

### The configuration digest covers the corpus's bytes, not its file names

The preamble governs whole *files*. Individual **rows** carry a provenance of their own: every row of
every area report ends with a `run` column and an `identity` column, and a row is aggregated only when
both match the run reading it. The summary states both in its **Provenance** section, as a
**`Run identifier`** and a **`Configuration digest`**.

That digest is not the `config=` fingerprint under another name. The fingerprint spells the *settings*
out; the digest covers four inputs, two of which the fingerprint says nothing about:

| Input | Why a row's meaning depends on it |
| --- | --- |
| The row schema | A row read back under a different column layout would be misread field by field |
| The effective matrix and policy | The same fact the fingerprint spells out, folded in so one comparison covers everything |
| The discovered tool set | The same program compared by a different reference compiler, or run under a different emulator, is a different comparison. This is the same inventory the summary's `## Environment fingerprint` section prints — each compiler, each emulator and the kernel — digested |
| **The corpus that was read** | A verdict is a claim about a specific program and a specific expectation record. If either changed, the verdict describes something that is no longer there |

The corpus input is **the bytes of every program and every expectation record** — not their paths, and
not the declared program counts. That distinction is the entire point of the field: editing a program
changes nothing else in the digest — same configuration, same tools, same declared counts — so a digest
built from names and counts would accept an area report written *before* the edit and add its rows to a
summary describing the corpus *after* it, with nothing in the artifact saying so. Content is the only
input that detects it, and it costs no determinism at all: two runs over an unchanged corpus digest
identically, and two checkouts of one commit at different paths agree, because each file contributes
its name **relative to** the corpus root rather than its location on disk.

A file that could not be read, or a feature area that could not be enumerated, contributes the *fact*
that it contributed no bytes — **per file and per area, never all-or-nothing**. That granularity is
load-bearing on a branch like this one, where five of the fourteen area directories have not landed: a
digest that collapsed to a single "corpus unreadable" value the moment one area was missing would be a
constant here, and would detect nothing whatever.

### What a retained cell workspace holds

A workspace is removed when its cell passes and retained when it does not, or always under
`BCC_CONFORMANCE_KEEP_WORK`. A retained one is self-sufficient — everything needed to investigate
the cell without re-running it, and without this harness:

| Entry | Content |
| --- | --- |
| `program.c` | A copy of the program source, taken when the workspace was allocated |
| `program.expected` | A copy of the program's expectation record, likewise |
| `commands.txt` | The exact commands this cell ran |
| `bcc.compile.stdout` / `bcc.compile.stderr` / `bcc.compile.status` | The compiler under test's build: both streams and the raw wait status |
| `ref.compile.stdout` / `ref.compile.stderr` / `ref.compile.status` | The reference compiler's build, where oracle (a) ran |
| `bcc.out` / `ref.out` | The two artifacts, where the builds succeeded |
| `bcc.stdout` / `bcc.stderr` / `bcc.exit` | The execution of the compiler-under-test artifact |
| `ref.stdout` / `ref.stderr` / `ref.exit` | The execution of the reference artifact |

The source and record are copied in **at allocation**, not at retention, so a retained workspace
names the program it tested even for a cell that never got as far as building. The six
`*.compile.*` entries are written on **every** branch, including a refused build, for the same
reason: a compile whose evidence had not yet been recorded when the verdict was taken could not be
investigated afterwards.

Each `*.exit` and `*.compile.status` entry is a line-oriented `key = value` record in the same
spelling an expectation record uses, so it can be read by eye and by a script. It carries the
termination and its label, the exit code and whether that code lies inside the corpus's 0–125
contract, the signal where there was one, the **raw wait status**, the duration and the budget, which
mechanism enforced the budget, the runner, the working directory, both byte counts, the
capture-integrity record for each stream, the argument vector as a shell line, and — only when they
differ — the launch vector that wrapped it.

### Retention budgets

Retention is bounded, because an unbounded one is not a diagnostic aid: a single runaway cell can
fill a build directory and take the rest of the run's evidence down with it. Four ceilings apply, and
every one of them **reports** what it did rather than discarding quietly:

| Ceiling | Value | Applies to |
| --- | --- | --- |
| Per-entry | 8 MiB | One captured file within a workspace |
| Per-workspace | 32 MiB | One retained cell's total |
| Per-run | 2 GiB | Every retained workspace of the run, together |
| Workspace count | 512 | How many workspaces keep their **contents**; past this a workspace is still created, and still named by the report, but is kept as a bare marker |

Every ceiling is enforced **at the moment of retention**, not by a sweep afterwards, so the bound
holds continuously rather than eventually. When a **byte** ceiling bites, the **largest** entries are
pruned first — which is not an arbitrary order: the large entries are the linked executables, while
the small ones are the captured streams, the recorded statuses and the command lines, which is
everything an investigation actually reads. So the cheap evidence is what survives.

The **workspace count** ceiling is different in kind, and the difference is worth knowing before you
open a retained directory: past 512 workspaces a cell that does not pass still gets a workspace and is
still named by the report, but *every* entry in it is pruned — the command lines included. Nothing in
the directory survives to be read.

Every pruning is **reported**. A pruned entry is removed and the decision becomes a **pruning note**
naming the entry, its size, why it went, and how to reproduce it. That last part is the same in both
cases and does not depend on anything being left behind: the directory the entry stood in names the
cell exactly, so re-running that one cell reproduces it. A workspace whose entries were all pruned is
left as an **empty directory**, so the path the report names still exists rather than vanishing.

The run's totals — how many workspaces were retained, how many bytes they hold against the permitted
ceiling, all four ceilings, and how many prunings were performed — are reported under
[the deliverable summary](#the-deliverable-summary)'s `## Retained evidence` section, and as
`retained_workspaces`, `retained_bytes` and one `retention_pruning` record per note in `summary.tsv`.

This is load-bearing rather than tidy: a workspace that was pruned and a workspace that was never
created are indistinguishable on disk and mean opposite things, so a silent pruning would turn
*bounded* evidence into *apparently absent* evidence.

### FULL versus PARTIAL — one predicate, three places

A report is stamped `Coverage: FULL` only when every dimension of its planned-against-recorded matrix
met its plan **and** nothing went wrong while assembling it. The matrix table, the stamp in the first
heading and the `reduced` and `partial` fields of `summary.tsv` are all derived from the same
dimension list, so a shortfall can never sit beside a claim of full coverage. Anything below makes a
report not full, and every one of them is enumerated by name under **Why this report is reduced or
partial**:

| Condition | Stamp |
|---|---|
| `BCC_CONFORMANCE_QUICK` or `BCC_CONFORMANCE_ONLY` narrowed the matrix | reduced |
| An oracle arm's tooling is absent, or a comparison reported `UNAVAILABLE` | reduced |
| A matrix dimension fell short — areas, programs, targets, levels, cells or any oracle's comparisons | reduced when the configuration asked for a smaller matrix, partial when it did not |
| A test-name filter ran a subset of the suite | partial |
| An area report was absent, stale or unusable | partial |
| A defect in the corpus, in an expectation record, in an area report or in the summary's own assembly raised a diagnostic | partial |

Reduced always implies partial, because a smaller matrix is by definition not one complete run — so
`summary.tsv` reports `partial=true` whenever `reduced=true`.

### The deliverable summary

`target/conformance-report/summary.md` is the artifact the requirements ask for. Its four numbered
sections are the four things the requirements name: **1 — Feature areas covered**, **2 — Total tests
and their outcomes**, **3 — Expected divergences and their documented basis**, and **4 — Findings,
with verbatim reproducer, minimization status and reproduction commands**. That fourth title is
deliberately more exact than the requirement's own wording: a run performs no automated reduction, so
the artifact carries the verbatim copy plus the recorded status rather than claiming a reduced
program. It is also this suite's coverage evidence, for
the reason given under [the enumerable matrix](#the-enumerable-matrix).

Alongside those, and each present on every run:

| Section | Content |
| --- | --- |
| `## Preflight gates — the preconditions the oracles rest on` | Each gate, its requirement, its verdict and what was observed. Rendered on **every** run, including when nothing was recorded, because the section's presence is what tells a reader the preconditions were considered at all. See [Two of those four are gates](#two-of-those-four-are-gates-and-every-area-is-judged-against-them) |
| `## Run verdict` | Whether the run passes, and on what — the outcome tally, the **`Preflight gates that did not hold`** count, and a verdict line that accounts for both |
| `## ⚠️ Unexpected successes (XPASS) — stale expected-divergence markers` | Listed separately and prominently, always |
| `## ⚠️ Unavailable oracles` | Every arm that could not be attempted, never silent |
| `## Recorded, reasoned exclusions — what was deliberately not compared` | So the set of comparisons *not* made is as visible as the set that was |
| `## Environment fingerprint` | Each compiler, each emulator and the kernel, so a divergence can be attributed to toolchain drift |
| `## Run configuration` | The effective policy, including a **`Configuration fingerprint`** row — deterministic, so two runs under the same policy agree and a reduced run's numbers cannot be mistaken for a full run's |
| `## Retained evidence` | `retained_workspaces`, `retained_bytes` against the permitted run ceiling, and every `retention_pruning` note |
| `## Why this report is reduced or partial` | Present whenever a quick matrix, a name filter or `BCC_CONFORMANCE_ONLY` narrowed the run |
| `## Diagnostics` | Anything the reporting path itself needs the reader to know |

`summary.tsv` carries the same data, one labelled record per line, including
`retained_workspaces`, `retained_bytes`, one `retention_pruning` record per note, and one `preflight`
record per gate. Its `run_fails` field accounts for the preflight as well as the outcomes, so an
aggregator that reads only that field can never see `false` while a precondition was unmet.

The per-area reports carry the same `## Preflight gates` section, narrowed to the gates that govern
that area, immediately before `## Area at a glance`.

### What a finding artifact directory holds

A finding is a deliverable, so its directory is designed to be reproducible **without this harness**:

| Artifact | Content |
| --- | --- |
| `reproducer.c` | The program, minimized as far as practical |
| `reproducer.expected` | Its expectation record, so it remains runnable by the harness too |
| `MANIFEST.txt` | `finding_id`, an **`identity_digest`**, the area and program, the oracle and its letter, the divergence class, which cells diverged, and a description of what was observed |
| `commands.sh` | Exact, copy-pasteable compile and run lines for every cell involved. Run it as `sh commands.sh`: it carries a `#!/bin/sh` line but is written without an executable bit, so name the interpreter rather than invoking the path directly |
| `outputs/<side>-<target>-<opt>.{stdout,stderr,exit}` | The **program's** two streams byte for byte, and how it ended. Standard error is captured even though it is never compared, because diagnostic text is often the fastest route to a diagnosis |
| `outputs/<side>-<target>-<opt>.compile.{stdout,stderr,exit}` | The **compiler's** own two streams and build outcome for that same cell, written whenever there was a build |
| `environment.txt` | Each compiler version, each emulator version, the kernel — plus this run's **`run_token`** and **`configuration`**, so an artifact can be attributed to the run that produced it |
| `diff.txt` | The computed difference, with the first divergent line and byte offset |

**Reading an `outputs/` name.** `<side>` is `bcc` for the compiler under test — the subject of every
comparison — and the oracle's own letter (`a`, `b` or `c`) for an authority, so a name states which side
produced it without needing a legend. `<target>` is the short target name (`x86_64`, `i686`, `aarch64`,
`riscv64`) and `<opt>` is the optimization level with its hyphen dropped (`O0`, `O1`, `O2`), giving names
such as `bcc-aarch64-O2.stdout` and `a-aarch64-O2.compile.stderr`.

The rule for which stream lands in which entry has **no exceptions**, and that is deliberate rather than
incidental. `.stdout` and `.stderr` always hold the *program's* streams and are empty when the program
never ran; `.exit` always states how the program ended, or that it did not, and never presents a status
it does not have; `.compile.stdout`, `.compile.stderr` and `.compile.exit` always hold the *compiler's*
own streams and outcome. A scheme that put compiler diagnostics into `.stderr` whenever a program had not
run would force a reader opening `a-aarch64-O2.stderr` to work out first whether that side's build
succeeded, turning every inspection into a case analysis. The compiler's standard output is written even
though a compiler ordinarily leaves it empty, because `commands.sh` redirects a maintainer's re-run into
a file of that same name — an entry absent for one build and present for another would make comparing the
re-run against the recorded evidence a case analysis too.

**The identifier is injective.** It carries the full 64-bit digest of the complete finding identity
rather than a truncation of it, and before anything is written the identity is verified against any
`MANIFEST.txt` already at that path. Two distinct findings whose abbreviated names would have
collided therefore get distinct directories, and a genuine collision is **reported** rather than
silently overwriting another finding's evidence.

**What is byte-identical between two runs, and the two things that are not.** A finding directory is
meant to be *diffed* across runs, so almost all of it is a pure function of the divergence. For one
unchanged divergence, two runs produce the same identifier, the same file set, and byte-identical
`reproducer.c`, `reproducer.expected`, `MANIFEST.txt`, `commands.sh`, `diff.txt` and
`.stdout`/`.stderr` captures. Two things legitimately differ, and each is a fact about the **run**
rather than about the writer:

| Differs | Why it is kept anyway |
| --- | --- |
| `duration_ms` in `.exit` and `.compile.exit` | A duration is part of a capture, and for a timeout it is the evidence. Timing telemetry is confined to these two records and kept out of everything a diff reads for whether the divergence changed |
| `environment.txt` | The tool versions and this run's token are exactly what lets a later reader tell a toolchain change from a compiler change |

Neither is noise; they are simply not the parts of a two-run diff that carry information about whether
the divergence changed. Diff two runs' finding directories and anything else that differs is something
the run genuinely found.

**`diff.txt` is stable to its last line, the comparator's verbatim account included.** That property
is bought by a deliberate split rather than inherited. A build's and an execution's one-line
description each state the *budget* they were bounded against, which is a pure function of the
configuration, and neither states the duration it measured. The measured duration is reachable only
through a separate timing variant of each, and that variant is called only from progress output
printed to a terminal. The same discipline is what keeps `MANIFEST.txt` stable: a capture's own description carries
no wall-clock duration even though both underlying observations can report one. Reaching for a timing
variant in anything written to a file would take this guarantee away silently, which is why the two are
separate named methods rather than one method with an option.

**`commands.sh` reproduces the whole oracle contract, not half of it.** Every build and every
execution in it is bounded by the discovered `timeout` utility through a runtime test, so one script
works with or without that utility present — a timeout is one of the divergence classes, so an
unbounded reproduction could hang instead of reporting the timeout as a result. Each side records its
own expected termination, keeping an ordinary exit, a signal death and a timeout **distinct** rather
than folding a signal into `128 + signal` the way a bare `$?` does. And the comparison is on
**stdout and status together**: comparing stdout alone would call two runs equal that disagreed on
how they ended. The script prints a single `RESULT:` line saying which of the three things happened —
the recorded divergence reproduced, nothing differed, or one side never ran at all.

### Cross-references

| Path | What it holds |
| --- | --- |
| [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) | The expected-divergence register, machine-checked against the markers in both directions |
| `FINDINGS.md` | **PLANNED, not present on this branch** — the findings register, indexing every curated reproducer. Named rather than linked |
| [`../conformance.rs`](../conformance.rs) | The suite driver: 14 area tests and 4 infrastructure tests |
| [`../conformance_harness/`](../conformance_harness/) | The harness modules — oracle discovery, workspace isolation, record parsing, compilation, execution, comparison, classification, findings, reporting, the flag probe and the audit gate |
| `docs/testing/differential-conformance.md` | **PLANNED, not present on this branch** — the documentation-site page: methodology, oracle definitions, the build matrix, the verdict taxonomy and the summary format. Named rather than linked |

---

## Adding a program, adding an area, retiring a marker

### Adding a program

Create exactly **two** files in the appropriate area directory:

```text
tests/conformance/<NN_area>/<NNN_name>.c
tests/conformance/<NN_area>/<NNN_name>.expected
```

**No harness code registers the program**, because discovery is directory-driven — but one harness
edit is still required, at step 5 below. Then:

1. Write the program against every rule in [Corpus authoring rules](#corpus-authoring-rules).
2. Copy the [canonical reference record](#the-canonical-reference-record) and edit **five** things,
   not three. The exemplar is a real record for a real program, so every field that describes that
   program must be rewritten to describe yours:
   1. `program` — must equal the file stem exactly.
   2. `area` — must equal the directory name exactly.
   3. `description` — one line naming what this program puts under test.
   4. **`ub_audit_flags` — delete this line** unless your program genuinely needs one of the two
      sanctioned deviations. The exemplar is a deliberate narrowing program and therefore declares
      one; a program that is clean under the full gate omits the key entirely, and a record that
      declares the full gate verbatim is refused as a deviation in nothing.
   5. **`ub_notes` and `impl_defined_notes` — rewrite both.** The exemplar's notes argue about the
      exemplar's conversions and cite the exemplar's measured per-target values; copied unedited
      they would assert something untrue of your program, which is a defect in the record even
      though it parses. Write the undefined-behaviour-freedom argument for the constructs your
      program actually contains and keep it in `ub_notes`; if you kept a deviation at step 4, write
      its reason into `impl_defined_notes` — not into `ub_notes`, which is never searched for a gate
      reason — as a paragraph of its own naming every dropped flag, per
      [Recorded reasons](#every-validation-the-parser-enforces). If your program narrows nothing —
      no restricted target list, no disabled oracle, no gate deviation — `impl_defined_notes` may
      be dropped, but state any implementation-defined property you relied on if you relied on one.
3. Establish the golden record. Once `tools/regenerate_expected.sh` lands, that script is the only
   sanctioned way to do it; until then, run the cell by hand using the recipe under
   [Reproducing a cell by hand](#reproducing-a-cell-by-hand), read the captured stdout back, and
   paste it into `expected_stdout` — having first confirmed against the reference compiler that the
   bytes are *right*, not merely what `bcc` currently emits. A golden record copied from an
   unverified run is a wrong answer promoted to an expectation.
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

No existing test file is edited, and no `#[ignore]` attribute is added or removed anywhere — this suite
declares none at any depth, and no harness module declares a test function of its own, so it can move
neither the repository's test count nor its ignored count. That much is mechanical **here**, and it is
the whole of what a checkout carrying this suite alone can establish.

The count itself — **exactly 13 ignored** — is a property of the *whole repository*, and no integration
test can read another test target's ignored count, so it is verified by the health gate rather than
asserted by the suite: `cargo test 2>&1 | grep "test result"` must report `13 ignored`. On a checkout
without the compiler's Cargo package that gate cannot run at all, for the reasons set out under
[The Cargo integration precondition](#the-cargo-integration-precondition), so the claim is stated
here as what must hold and be measured after the merge, not as something already measured.

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
  `typeof`, computed goto and inline assembly, and omits case ranges). The program is written
  regardless, with all three oracles enabled on all four targets at all three optimization levels.
  **No marker is attached, and that is deliberate:** an omission from an inventory is not a
  documented limitation — it says nothing mentions the construct, not that the frontend rejects it —
  so a rejection here is a **FINDING**, captured with its reproducer and its exact reproduction
  commands, until a repository artifact explicitly documents the limitation *and* a real divergence
  has been observed and reproduced. Nothing is excluded and nothing is excused; the ambiguity
  between an implementation gap and a documentation gap is surfaced in
  [`EXPECTED_DIVERGENCES.md`](EXPECTED_DIVERGENCES.md) §4.1 so a maintainer can settle it from a real
  run rather than from a guess written in advance.
- **`13_floating_point/004_long_double_target_restricted.c`** (planned with the floating-point area)
  — `long double` was measured to have three different representations across the four targets (16,
  12, 16 and 16 bytes; x87 80-bit against IEEE binary128), so cross-backend *value* equality is
  genuinely meaningless for it. The program is still to be written and still run on all four targets
  at all three levels; it is compared against the same-target reference compiler and against its
  golden record, and **only** cross-backend value equality is switched off — a **recorded exclusion**
  carrying its measured reason in the record's own `impl_defined_notes`, not a marker, because a
  marker scoped to an oracle the record has switched off would be dormant and the record format
  refuses it at parse time.

Two other measured differences are handled **by construction** rather than by marker, because a
marker implies a test that diverges and these do not: plain-`char` signedness is avoided by using
explicit `signed char` and `unsigned char`, and the i686 `long` and pointer width difference is
avoided by width normalization. Both are recorded as implementation-defined notes in the affected
programs.

### C4 — Contained execution

Two separate mechanisms discharge this constraint, and keeping them apart is what makes the claim
checkable.

**Corpus-authoring policy** is what keeps the *programs* contained. Every input is a literal in the
program source: **no corpus program opens a socket, calls `fopen`, or reads `argv` or `getenv`**.
That is a property of the 108 sources, verified by reading them, not something the harness enforces
at run time. It is why the whole folder has **exactly one** fixture file, and why the determinism
rules above are what they are — a program that read the clock, the environment or an external file
would not be reproducible, which would make byte-exact comparison meaningless.

**Path discipline** is what keeps the *harness* contained. Every path the harness constructs is
resolved and re-checked against a root beneath the Cargo build directory before it is written: each
cell gets its own deterministic workspace, uniquely determined by area, program, target and
optimization level, and reports and generated findings get their own roots. A per-cell timeout
bounds runaway execution.

**What neither mechanism is.** This is not an operating-system sandbox, and nothing in this suite
claims to be one:

- No namespace, `chroot`, seccomp filter or network restriction is applied to any child process.
  Setting a working directory is not confinement.
- The environment a child receives **is** cleared and replaced — see *The environment every child
  receives* below — and `TMPDIR`, `TMP`, `TEMP` and `HOME` all point at the cell's own workspace. That
  binds a tool that reads those variables and only such a tool: `gcc -### -static …` shows the driver
  writing `/tmp/cc*.s`, `/tmp/cc*.o` and `/tmp/cc*.res` from a hard-coded path, so **that** driver
  keeps its temporaries outside the workspace every time. A variable cannot bind a program that never
  reads it.
- The reference compilers, the cross drivers, the QEMU runners and the bounding `timeout` utility
  are installed tools that live and execute outside the build tree; only their *outputs* are placed
  inside it.
- A crash dump is the host's `kernel.core_pattern` decision. A test binary that dies on a signal may
  write a core file wherever the host has configured, and the harness neither prevents nor observes
  that.

None of this weakens the constraint as the requirements state it, because the constraint is about
what the *generated programs* do, and that is settled by the authoring policy above. It is recorded
so that no reader mistakes path discipline for isolation.

**What this is, stated precisely, because the difference matters.** This is **workspace isolation for
a trusted, committed corpus — not an operating-system sandbox**: no `chroot`, no mount or PID
namespace, no seccomp filter and no landlock. Two further consequences follow from that, and neither
is a gap in the authoring policy above:

- The child does **not** inherit the runner's environment: it is cleared and a small, fixed,
  suite-chosen set is installed in its place, so a program that called `getenv` would see that set and
  not yours. That closes credential leakage and non-determinism, and it does **not** amount to
  confinement — the authoring rules still forbid reading the environment, and the corpus is still
  reviewed rather than merely fenced, because a fixed environment says nothing about the syscalls a
  program may make.
- The **artifact path of every cell is checked to be inside that cell's workspace** before it is
  executed, and the three write roots the harness uses all live beneath the Cargo build directory.

#### The environment every child receives

Every spawn site in the harness — both compilers, the three emulators, the bounding `timeout`
utility, the audit gate's instrumented artefact, and the compiled programs themselves — goes through
one function that clears the environment and installs exactly this:

| Variable | Value | Why |
| --- | --- | --- |
| `PATH` | the entries of your `PATH` that are absolute **and** not writable by an account the suite does not trust | A compiler driver finds its own stages — `cc1`, `as`, `ld`, `collect2` — through `PATH`. Handing it the raw value would let a planted stage be executed by a driver the suite had vetted, substituting the program one level below where tool resolution looked. The pre-flight report prints the exact value and every entry it skipped. |
| `LANG`, `LC_ALL`, `LANGUAGE` | `C` | Number and message formatting must be invariant, because stdout is compared byte for byte. |
| `TZ` | `UTC` | Removes any dependence on the host's time zone. |
| `TERM` | `dumb` | Stops a tool deciding to emit colour escapes into a compared stream. |
| `ASAN_OPTIONS`, `UBSAN_OPTIONS`, `LSAN_OPTIONS`, `MSAN_OPTIONS`, `TSAN_OPTIONS` | strictest available: abort and print on the first diagnostic | An inherited `ASAN_OPTIONS=detect_leaks=0:halt_on_error=0` would turn a program with undefined behaviour into a clean audit pass, removing the precondition that makes every divergence in this suite meaningful. |
| `HOME`, `TMPDIR`, `TMP`, `TEMP` | the cell's own workspace | A tool that writes a cache, a history file or a scratch file where these point writes it inside the build directory rather than into your home directory. |

Nothing else is set and nothing else is inherited. In particular `LD_PRELOAD`, `LD_LIBRARY_PATH`,
`C_INCLUDE_PATH`, `GCC_EXEC_PREFIX` and every credential-bearing variable your CI exports do not
reach any child.

A finding's `commands.sh` reproduces this same environment through an `isolated()` shell function
built on `env -i`, and it prints the search path as `CHILD_PATH` so you can adjust the one value that
belongs to the machine the run happened on. Reproducing a command line without its environment
reproduces a different invocation.

The guarantee is therefore accurate for what this suite runs — a fixed, reviewed corpus of programs
that read no input — and it must **not** be read as a promise about arbitrary code. **Anything
untrusted must be run under an external sandbox**: a container, a virtual machine, or a seccomp or
landlock profile applied outside this harness.

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

All harness code is written to satisfy `cargo clippy -- -D warnings` and `cargo fmt -- --check` with
**no new suppression, allowance attribute or formatting exception**.

Both gates are `cargo` gates, so — exactly as for the commands under
[Running the suite](#running-the-suite) — they require the package-complete branch. On a
documentation-only checkout each exits 101 with `could not find Cargo.toml`, and the claim above is
therefore an obligation on the code as written rather than a result observed on this branch.
Formatting can still be checked here without the package, because `rustfmt` accepts a file path
directly: `rustfmt --edition 2021 --check tests/conformance.rs tests/conformance_harness/*.rs`.

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

Nothing is virtualized either. The file system is used for real, with the harness confining the
paths it writes to per-cell workspaces; no corpus program opens a socket; and time, randomness,
addresses and locale are never observed, because any of them would make byte-exact comparison
meaningless. What is *not* virtualized is equally deliberate: there is no isolation layer around any
child process, for the reasons set out under
[C4 — Contained execution](#c4--contained-execution).

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
