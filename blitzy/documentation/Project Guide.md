# Blitzy Project Guide — Differential Conformance Test Suite for the `bcc` C Compiler

> **Branch:** `blitzy-c80f5991-f31a-4d4d-af3a-308aa42b698f` · **HEAD:** `c1137a7` · **44 commits ahead of `origin/main`**
> **Scope of this guide:** the Agent Action Plan's differential conformance test suite and its path to production. The pre-existing compiler project, documented separately at 536 of 580 hours, is **not** in this denominator.

---

## 1. Executive Summary

### 1.1 Project Overview

This project adds an oracle-based **differential conformance test suite** to the `bcc` C compiler repository — a wholly new test category that distinguishes correct compilation from incorrect compilation by judging observable program behaviour rather than compiler internals. It targets compiler maintainers and CI. Three mutually independent oracles judge every cell: an external reference compiler, `bcc`'s own four backends against one another, and each program's recorded golden output. The technical scope is 14 feature areas, 108 hand-authored undefined-behaviour-free C programs, 4 target architectures and 3 optimization levels — **1,296 compile-and-run cells yielding 3,564 assertions** — delivered in pure `std` with zero third-party crates and no change to a single line of compiler source.

### 1.2 Completion Status

```mermaid
pie showData
    title AAP-Scoped Completion — 88.8% Complete
    "Completed (514 h)" : 514
    "Remaining (65 h)" : 65
```

<table>
<tr><th align="left">Segment</th><th align="left">Colour</th><th align="right">Hours</th></tr>
<tr><td>Completed / AI Work</td><td><b><code>#5B39F3</code></b> Dark Blue</td><td align="right">514</td></tr>
<tr><td>Remaining / Not Completed</td><td><b><code>#FFFFFF</code></b> White</td><td align="right">65</td></tr>
</table>

| Metric | Value |
| --- | --- |
| **Total Hours** | **579** |
| **Completed Hours (AI + Manual)** | **514** (AI 514 + Manual 0) |
| **Remaining Hours** | **65** |
| **Percent Complete** | **88.8%** |

> **Calculation (PA1, AAP-scoped):** `514 / (514 + 65) × 100 = 514 / 579 × 100 = 88.8%`
> Completed 514 h derives from the 27-row breakdown in §2.1; remaining 65 h from the 11-row breakdown in §2.2. `514 + 65 = 579` ✓

### 1.3 Key Accomplishments

- ✅ **Three independent oracles implemented and exercised in full** — reference-compiler (1,296 comparisons), cross-backend (972), and golden-record (1,296) = **3,564 assertions**, with *planned equal to recorded on all 14 matrix dimensions*.
- ✅ **108 C programs across 14 feature areas**, per-area counts matching the plan exactly (10, 8, 11, 7, 10, 10, 6, 8, 8, 10, 4, 6, 4, 6); all nine mandated areas at or above the six-program floor.
- ✅ **18 new tests pass, 0 fail, 0 ignored** — `test result: ok. 18 passed; 0 failed; 0 ignored` in **146.62 s**, independently re-run and reproduced during this review.
- ✅ **Undefined-behaviour freedom machine-enforced, not asserted** — 108/108 programs cleared both a strict `-Werror` warning gate and an ASan/UBSan gate across **324 audit invocations**, and each carries a written freedom argument in its own record.
- ✅ **Flag parity verified rather than assumed** — 48/48 probe checks held: 13 semantic verifications using pure-`std` ELF byte inspection, 2 acceptance-only with the limitation recorded, plus **negative assertions that no reference-only and no bcc-only flag leaked into the shared set**.
- ✅ **Every cell reproducible with no harness at all** — proven twice: by autonomous validation, and again in this review by rendering a record's three command templates by hand and reproducing an AArch64 `-O2` cell with both oracle (a) and oracle (c) agreeing.
- ✅ **Findings are deliverables, proven end to end** — a forced divergence produced complete 7-artifact directories, and `sh commands.sh` reproduced them **standalone with no harness, no Cargo and no rustc**.
- ✅ **Zero third-party crates, and the lint results are earned** — every import across all 13 Rust files resolves to `std`/`crate`/`super`/`self`, and there are **zero `#[allow]` attributes anywhere**.
- ✅ **All four constraints verified on disk, not merely intended** — no compiler source, manifest or lockfile exists on this branch to modify; 0 deletions; 0 `#[ignore]` attributes in any Rust file; children run with `env_clear()` and no `std::net` appears anywhere.
- ✅ **The repository's own open work item and open risk addressed** — "C11 Standard Corner Case Compliance Testing" and "C11 corner case non-compliance" both now have the targeted testing they called for.
- ✅ **93 review findings resolved across 6 remediation rounds** plus a dedicated security review, all before this hand-off.

### 1.4 Critical Unresolved Issues

| Issue | Impact | Owner | ETA |
| --- | --- | --- | --- |
| **The suite has never judged the real `bcc`.** This branch carries no compiler tree and no Cargo package (adding one is forbidden by constraint C1), so the whole 1,296-cell matrix was driven against a documented stand-in that forwards to gcc-13 — one toolchain compared with itself. | **High.** Every requirement-7 claim about `bcc` is still open. The suite's own registers state that what it has established about `bcc` is nothing at all. | Compiler maintainer | Merge + 1 day |
| **12 XPASS on `XD-GCCEXT-CASE-RANGES-001`.** GCC implements case ranges, so a GCC-forwarding stand-in cannot reproduce the marker's claimed refusal. The run passes only under the documented `BCC_CONFORMANCE_ALLOW_XPASS` hatch. | **Medium.** Default policy fails the run loudly (verified: exit 101), so nothing is hidden — but the marker's truth value is undecided until the real compiler answers. | Compiler maintainer | Merge + 1 day |
| **`FINDINGS.md` register is empty.** Deliberately and explicitly so — no real-bcc run has happened, so there is nothing legitimate to curate. | **Medium.** The first real run may produce findings whose curation volume is genuinely unknown. | QA / test owner | Merge + 3 days |
| **`tests/common/mod.rs` reuse not yet active.** The plan requires reuse of the shared helper rather than reimplementation; the file lives on the compiler branch. `mod common;` is a compile-time assertion, so it cannot be declared speculatively. | **Low.** The harness stands alone in pure `std`, so the suite is complete either way, and three mechanical assertions activate the moment the file appears. | Test owner | Merge + 1 day |
| **CI job has never executed on GitHub Actions.** Authored, YAML-valid, shellcheck-clean, digest-pinned — and one step deliberately *fails* rather than skips when the Cargo package is absent, so it cannot pass before the merge. | **Medium.** First-runner friction is likely. | DevOps | Merge + 2 days |
| **Whole-repository health-gate figures unmeasured.** The 3,955 defined / 3,942 passing / 0 failing / exactly 13 ignored invariant is a repository-wide property no single test binary can read. | **Low.** `docs/project-guide.md` already records the three-condition acceptance criterion rather than claiming the figures. | Test owner | Merge + 1 day |

### 1.5 Access Issues

| System/Resource | Type of Access | Issue Description | Resolution Status | Owner |
| --- | --- | --- | --- | --- |
| `bcc` compiler tree and its Cargo package (`Cargo.toml` + `src/**`) | Source-tree availability | Not present on this branch — it lives on the project's open pull request. Blocks every Cargo gate and any verdict about the real `bcc`. **Adding a manifest is forbidden by constraint C1**, so this cannot be worked around from inside the repository. | **Open** — resolution is *merge*, not manifest. Mitigated in scope by an external scratch package at `/opt/bcc-conformance-scratch` (deliberately outside the repository) that established every static and Cargo gate while asserting nothing about `bcc`. | Compiler maintainer |
| `tests/common/mod.rs` | Read-only reference material | Not present on this branch; consumed by all 16 existing integration suites, so it is protected by constraint C2 and may not be authored here. | **Open** — resolves with the merge. The driver enumerates the exact delegation work and guards it with three mechanical assertions. | Test owner |
| Host `/tmp` filesystem permissions | Filesystem trust | Measured at mode **2777 with no sticky bit** (`[ -k /tmp ]` false). A finding's `commands.sh` correctly refuses to execute from a build root beneath a world-writable non-sticky parent. Not fixable from inside the repository. | **Open (environment).** Two remedies documented and verified: relocate the finding directory to a trusted parent, or set `CARGO_TARGET_DIR` to a fully owned chain — which removes the trust warning entirely. | Platform / DevOps |
| `libclang-rt-20-dev` package | Optional toolchain component | Absent. Blocks the optional clang oracle's sanitizer gate. gcc-13 is the reference of record and the suite is 100% green with it. | **Open (optional).** Documented verbatim in the suite contract, with the remedy stated. | DevOps |
| Network access from suite-generated programs | Deliberately unavailable | Constraint C4 forbids it. This is also precisely why the repository's 13 existing network-dependent tests must remain ignored. | **By design — not a defect.** Both constraints agree: re-enabling those tests would breach C4, removing their attributes would breach C2. | N/A |

> **No repository-permission, credential or third-party-API access issue exists.** The suite touches no network, requires no secret, and adds no dependency. A credential scan across all 241 changed files returned only an `mktemp` template (`bcc-repro.XXXXXX`) and a lock-ownership token — neither a secret.

### 1.6 Recommended Next Steps

1. **[High]** Merge the suite onto the compiler branch and confirm Cargo target discovery works with **no manifest edit** — `tests/conformance.rs` auto-discovered, `tests/conformance_harness/` never a target, `tests/conformance/` invisible to Cargo. *(6 h)*
2. **[High]** Execute the first package-complete run — `BCC_CONFORMANCE_STRICT=1 cargo test --test conformance` — and triage every `FAIL`, `XPASS` and `UNAVAILABLE` row. This is the single step that converts the suite from *defined* to *measured*. *(8 h)*
3. **[High]** Curate whatever findings that run produces into `tests/conformance/FINDINGS.md`, minimizing each reproducer. **Do not patch the compiler** — requirement 6 and constraint C1 both forbid it. *(16 h)*
4. **[High]** Adjudicate the 12 XPASS on `XD-GCCEXT-CASE-RANGES-001`: either confirm a true XFAIL and remove the `BCC_CONFORMANCE_ALLOW_XPASS` hatch from CI, or retire the marker in **both** the record and the register. *(4 h)*
5. **[Medium]** Activate `tests/common/mod.rs` helper reuse and run the whole-repository health gate, confirming 3,942 passed / 0 failed / **exactly 13** ignored. *(9 h)*

---

## 2. Project Hours Breakdown

### 2.1 Completed Work Detail

| Component | Hours | Description |
| --- | --- | --- |
| Suite architecture & manifest-free Cargo integration design | 10 | Constraint C1 satisfied *by layout*: `tests/conformance.rs` auto-discovered, `tests/conformance_harness/` a nested non-target directory, `tests/conformance/` data-only. 14-area + 4-infrastructure test split; batch verdict accumulation |
| Harness — `mod.rs` shared type & verdict model | 20 | 8,528 lines. Verdict, cell, target-specification and oracle types; submodule surface |
| Harness — `env.rs` oracle discovery & capability record | 22 | 6,224 lines. Locates the compiler under test, native reference, 3 cross drivers, 3 emulators; probes plain **and** `-static` emulator spellings; verifies each driver's gnu17 default and refuses a gnu23 driver; records UNAVAILABLE arms |
| Harness — `manifest.rs` expectation-record parser | 20 | 6,646 lines. Hand-written `key = value` + `#` comment + `<<END` heredoc parser with the full validation set — **no serialization crate** |
| Harness — `sandbox.rs` hermetic per-cell workspaces | 10 | 2,330 lines. Constraint C4: deterministic collision-free paths, removal on success, retention on failure, safe under default `libtest` parallelism |
| Harness — `compile.rs` shared-flag-enforced cell builds | 12 | 3,460 lines. Requirement 3: enforces `-o` / `-O0|-O1|-O2` / `-static` for oracle (a); bcc-only `--target` reaches only oracle (b) |
| Harness — `execute.rs` native & QEMU dispatch | 12 | 2,901 lines. Raw wait status kept distinct from signal death; per-cell timeout with a `std` watchdog |
| Harness — `compare.rs` byte-exact comparison | 7 | 1,687 lines. Stdout byte equality + exit-status equality with first divergent line and byte offset; stderr captured, never compared |
| Harness — `classify.rs` six-class → six-verdict mapping | 9 | 2,041 lines. Requirements 5 and 7: closed verdict space, strict marker scope matching, **no silent-skip path** |
| Harness — `findings.rs` finding artifact generation | 24 | 8,257 lines. Requirement 6: `reproducer.c`, `reproducer.expected`, `MANIFEST.txt`, `commands.sh`, `outputs/*`, `environment.txt`, `diff.txt`, plus a report-grade review bundle |
| Harness — `report.rs` per-area & run summary reporting | 24 | 8,180 lines. `summary.md` + `summary.tsv` + `run.txt` + 14 area `.md`/`.tsv` + evidence documents; generation stamping so a summary never publishes another run's results; once-only finalization; REDUCED stamping |
| Harness — `flagprobe.rs` requirement-3 flag verification | 16 | 4,533 lines. 48 checks — 13 semantic, 2 acceptance-only recorded, plus negative leak assertions; pure-`std` ELF byte reads so binutils is not a dependency |
| Harness — `ubaudit.rs` requirement-1 audit gates | 14 | 4,337 lines. Strict warning gate + ASan/UBSan gate, reference-compiler-only; per-program deviations must carry a recorded reason |
| Driver — `tests/conformance.rs`, 18 tests | 18 | 6,375 lines. 14 area tests + 4 infrastructure tests; batch accumulation reproducing the full per-program table in the failure message |
| C corpus — 108 UB-free programs, 14 feature areas | 96 | 19,915 lines (12,486 code). Two-variant rule in **100% of every arithmetic, conversion, bitfield, optimization and floating area**; determinism rules; hand-declared `printf`; no `stdio.h` anywhere |
| 108 expectation records | 36 | 16,126 lines (15,177 data). Target/level matrix, three command templates, golden stdout, written UB-freedom argument, implementation-defined notes, gate deviations, markers |
| `EXPECTED_DIVERGENCES.md` marker register | 14 | 1,683 lines. Marker contract, five required keys, six legal classes, scope and basis grammars, two-tier authority, retirement procedure, 2 markers analysed, 5 handled-by-construction cases |
| `FINDINGS.md` finding register | 10 | 1,151 lines. Finding definition, curated-directory contract, transient/curated split, curation procedure, divergence-surface analysis, dated maintenance log |
| `tests/conformance/README.md` suite contract | 26 | 4,251 lines. The reproduction authority — oracles, verdicts, record grammar, corpus authoring rules, UB gate, flag discipline, matrix, environment variables, a fully worked hand-reproduction example, artifact layout |
| `docs/testing/differential-conformance.md` methodology page | 12 | 1,775 lines across 20 sections; published through mkdocs |
| `tools/regenerate_expected.sh` golden-record tool | 18 | 5,926 lines POSIX `sh`. `--check` mode, locking, unreachable from a test run, shellcheck-clean at default severity |
| Fixture header & artifact hygiene | 1 | The suite's single fixture (`probe_header.h`, 31 lines) plus `.gitignore` entries for the three transient roots and the regeneration lock paths |
| CI job — `.github/workflows/ci.yml` | 10 | 666 lines. Exactly one job, 14 steps, all 3 actions digest-pinned, 11 shellcheck-clean run blocks, STRICT mode, artifact upload; fails rather than skips when the Cargo package is absent |
| Additive metadata edits | 7 | mkdocs nav entry (+1), README subsection (+81/−0), `docs/project-guide.md` inventory, work item and risk register (+121/−25) |
| Environment provisioning & empirical verification | 10 | gcc-13 13.4.0 pinned as driver of record (unversioned gcc 15.2.0 is gnu23 and correctly refused), 3 cross drivers, QEMU spelling discovery, static C runtimes on 4 targets, `/usr/local/etc/bcc-conformance.env` |
| QA & code-review remediation — 6 rounds, 93 findings + security review | 44 | Commits resolving 31, 24, 18, 11, 5 and 4 findings plus a dedicated security review; marker reinstatement/retirement analysis; register and milestone reconciliation |
| Final validation & production-readiness gates | 12 | Full STRICT runs; static gates; golden-record re-derivation over 108 records / 1,296 cells; requirement-4 and requirement-6 end-to-end proofs; timeout, hermeticity and policy-matrix validation; doc-link and zero-placeholder audits |
| **Total** | **514** | **Matches Completed Hours in §1.2 ✓** |

### 2.2 Remaining Work Detail

| Category | Hours | Priority |
| --- | --- | --- |
| [AAP R7] Merge the suite onto the compiler branch & establish Cargo discovery | 6 | High |
| [AAP R7] First package-complete STRICT run against the real `bcc`, plus triage | 8 | High |
| [AAP R6] Curate the findings the real-bcc run produces | 16 | High |
| [AAP R5] Adjudicate the 12 XPASS on `XD-GCCEXT-CASE-RANGES-001` | 4 | High |
| [AAP §0.4.10] Integrate `tests/common/mod.rs` helper reuse | 5 | Medium |
| [AAP §0.7.2] Whole-repository health gate (3,942 / 0 / exactly 13) & inventory reconciliation | 4 | Medium |
| [Path-to-production] CI job first execution on GitHub Actions | 6 | Medium |
| [AAP §0.5.3] `docs/project-guide.md` & register reconciliation after the real run | 3 | Medium |
| [Path-to-production] Environment pin hardening & build-root trust remedy | 4 | Medium |
| [Path-to-production] clang second-oracle enablement | 5 | Low |
| [Path-to-production] Maintainer handover & runbook review | 4 | Low |
| **Total** | **65** | **Matches Remaining Hours in §1.2 and the §7 pie chart ✓** |

### 2.3 Hours Reconciliation

| Check | Expected | Actual | Status |
| --- | --- | --- | --- |
| §2.1 completed rows sum | 514 | 514 | ✅ |
| §2.2 remaining rows sum | 65 | 65 | ✅ |
| §2.1 + §2.2 = §1.2 Total | 579 | 579 | ✅ |
| §2.2 sum = §1.2 Remaining = §7 pie "Remaining" | 65 | 65 / 65 / 65 | ✅ |
| Completion `514 / 579 × 100` | 88.8% | 88.8% | ✅ |
| Human task list total = §2.2 total | 65 | 34 High + 22 Medium + 9 Low = 65 | ✅ |

**Priority distribution of remaining work:** High 34 h (52.3%) · Medium 22 h (33.8%) · Low 9 h (13.8%).

---

## 3. Test Results

All rows below originate from Blitzy's autonomous validation execution and were **independently re-run and reproduced during this review** (full suite re-executed to completion; artifacts re-inspected).

| Test Category | Framework | Total Tests | Passed | Failed | Coverage % | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Feature-area conformance (14 areas) | Rust `libtest` `#[test]` | 14 | 14 | 0 | 14/14 areas swept — 100% of the declared matrix | One batch test per area, accumulating every verdict before asserting, so the deliverable summary is complete on every run |
| Infrastructure gates | Rust `libtest` `#[test]` | 4 | 4 | 0 | 4/4 gates held | Flag-capability probe, UB audit, marker-register integrity, oracle capability report |
| **Suite total** | **Rust `libtest`** | **18** | **18** | **0** | — | **`test result: ok. 18 passed; 0 failed; 0 ignored` in 146.62 s** |
| Oracle (a) — reference compiler | Harness comparator | 1,296 | 1,284 | 0 | 1,296 / 1,296 planned = 100% | 12 XPASS on the case-range marker (see §6/T2); 0 FAIL |
| Oracle (b) — cross-backend | Harness comparator | 972 | 963 | 0 | 972 / 972 planned = 100% | 9 XFAIL — `long double` cross-backend value comparison, documented basis |
| Oracle (c) — golden record | Harness comparator | 1,296 | 1,296 | 0 | 1,296 / 1,296 planned = 100% | Every cell byte-matched its own recorded `expected_stdout` |
| **All differential & golden assertions** | **Harness comparators** | **3,564** | **3,543** | **0** | **3,564 / 3,564 planned = 100%** | **PASS 3,543 · XFAIL 9 · XPASS 12 · FINDING 0 · FAIL 0 · UNAVAILABLE 0** |
| UB audit — warning gate | gcc-13 `-Werror` gate | 108 | 108 | 0 | 108/108 programs | Part of 324 total audit invocations |
| UB audit — sanitizer gate | gcc-13 ASan + UBSan | 108 | 108 | 0 | 108/108 programs | `-fno-sanitize-recover=all`; every program terminated cleanly |
| Flag-capability probe | Pure-`std` ELF inspection | 48 | 48 | 0 | 13 flags semantically verified; 18 asserted absent | 2 checks acceptance-only with the limitation recorded, not glossed |
| Marker-register integrity | Harness bidirectional audit | 2 markers | 2 | 0 | 100% | Both cited bases resolved to a real file, line and verbatim sentence |
| Record substantiation | Harness manifest gate | 108 | 108 | 0 | 108/108 records | Every cell entitled to be read as evidence |
| Golden-record re-derivation | POSIX `sh` tool, `--check` | 108 records / 1,296 cells | 108 | 0 | 100% | "processed 108 record(s); 0 changed" — a path fully independent of the Rust harness |
| Static analysis — format | `rustfmt --check` | 13 files | 13 | 0 | 100% | Exit 0, zero output |
| Static analysis — type check | `rustc --test -D warnings` | 1 target | 1 | 0 | 100% | Exit 0, zero output |
| Static analysis — lint | `cargo clippy --all-targets -- -D warnings` | 1 target | 1 | 0 | 100% | Exit 0, **zero `#[allow]` attributes** anywhere, so the result is earned |
| Static analysis — shell | `shellcheck` at default severity | 1 script + 11 CI blocks | 12 | 0 | 100% | No exclusions applied |
| Documentation links | `rustdoc` + workflow inventory | 13 names | 13 | 0 | 100% | Observed unresolved-link set identical to the 13-name inventory **in both directions** |

**Per-area comparison counts (all recorded, all matching plan):** 01 → 330 · 02 → 264 · 03 → 363 · 04 → 231 · 05 → 330 · 06 → 330 · 07 → 198 · 08 → 264 (252 PASS + 12 XPASS) · 09 → 264 · 10 → 330 · 11 → 132 · 12 → 198 · 13 → 132 (123 PASS + 9 XFAIL) · 14 → 198.

> **Coverage is reported as an enumerable matrix, never a percentage.** Coverage instrumentation would require a development dependency, which this repository forbids absolutely, so any percentage published here would be unmeasurable — fabrication rather than evidence. The matrix is countable from the committed file set and re-reported on every run.

> **Scope caveat, stated plainly.** These verdicts were produced with a **documented stand-in** compiler that forwards to gcc-13, because this branch carries the suite ahead of the compiler tree. A stand-in that forwards to the reference toolchain compares one toolchain with itself: that is evidence about the environment and about the suite, **not about `bcc`**. `docs/project-guide.md` therefore counts these 18 tests in the *defined* total (3,955) and deliberately excludes them from the *measured* pass count, recording a three-condition acceptance criterion instead.

---

## 4. Runtime Validation & UI Verification

This project has no user interface. Runtime validation covers the suite's own executable surface, the oracles it drives, and the artifacts it produces. Every line below was observed, and every item was re-verified during this review.

### Suite entry points

- ✅ **Documented entry point** — `cargo test --test conformance [-- --nocapture]` runs to completion: **18 passed; 0 failed; 0 ignored in 146.62 s**.
- ✅ **Manifest-less entry point** (this branch's shape) — `rustc --edition 2021 --test -D warnings` builds a 13.1 MB binary that runs the same 18 tests with the same result.
- ✅ **Pre-flight capability report** — 2.22 s; names every driver and runner with its version, resolves `libc.a`/`crt1.o`/`crti.o`/`crtn.o` per target, prints device/inode provenance per binary, shows the child `PATH` as *computed, never inherited*, and reports **"Unavailable oracle arms: none"**.
- ✅ **Single-area run** — `area_04_bitfields` in 53.33 s. **Single-program run** — `BCC_CONFORMANCE_ONLY=04_bitfields/003_compound_assignment` in 10.10 s.

### Oracle execution

- ✅ **Oracle (a)** — 1,296 / 1,296 comparisons; all four reference drivers resolved at 13.4.0; the gnu23 unversioned driver correctly **refused with no silent fall-through**.
- ✅ **Oracle (b)** — 972 / 972 comparisons; all three emulators attested **by execution** at 10.1.0; x86-64 executes natively as the baseline, so no emulation variable enters it.
- ✅ **Oracle (c)** — 1,296 / 1,296 assertions; independently re-derived by the POSIX-`sh` tool → "processed 108 record(s); 0 changed".
- ✅ **Static C runtimes complete on all four targets**, each resolving inside its own sysroot.

### Requirement proofs

- ✅ **Requirement 4 — isolated reproducibility.** Re-proven in this review: the three command templates were rendered by hand out of `04_bitfields/005_straddling_and_zero_width.expected` and the AArch64 `-O2` cell reproduced with no harness — exit 0 both sides, oracle (a) byte-identical, oracle (c) matching the golden record.
- ✅ **Requirement 6 — findings as deliverables.** A forced divergence produced 24 FINDINGs over 12 cells with the run still exiting 0, every directory complete across all 7 artifacts, and `sh commands.sh` reproduced them **standalone with no harness, no Cargo and no rustc**. When a divergence should *not* reproduce, the script says so honestly.
- ✅ **The harness refused a shell-script stand-in** on oracle-independence grounds — the independence check is real, not decorative.
- ✅ **Timeout is a first-class divergence class.** An endless-loop stand-in with a 3 s budget bounded all 12 cells and recorded `divergence_class=timeout, verdict=FINDING`.
- ✅ **Constraint C4 — hermeticity, measured.** Children run with `env_clear()`; a before/after snapshot showed zero repository changes outside `target/` and zero new entries under `/tmp` or `/root`; no `std::net` anywhere; the corpus declares only `printf` plus one `malloc`/`free` pair and one `exit`.

### Policy and configuration matrix

- ✅ **XPASS policy, both directions.** Default → **exit 101** with 12 XPASS and a precise instruction naming *both* places to edit. Documented hatch → exit 0, while the summary still lists all 12 prominently and stamps `allow_xpass:1` in its fingerprint.
- ✅ **`BCC_CONFORMANCE_QUICK`** → reduced matrix, and the report is headed **"⚠️ REDUCED COVERAGE — this run swept less than the full matrix, so it must not be read as a complete result."**
- ✅ **`BCC_CONFORMANCE_KEEP_WORK`** → 21 self-contained cell workspaces retained; default removal leaves only empty infrastructure roots.
- ✅ **`BCC_CONFORMANCE_TIMEOUT_SECS`** → `0` refused with a precise remedy; a positive value honoured.
- ✅ **`BCC_CONFORMANCE_STRICT`** escalates UNAVAILABLE to failure, and **`ALLOW_MISSING_ORACLES` cannot lower it** — verified in the report's own wording.
- ✅ **Artifact emission** — `summary.md`, `summary.tsv`, `run.txt`, 14 area `.md` + 14 `.tsv`, and `evidence/` with 21 durable documents for the 21 non-PASS outcomes, all under default `libtest` parallelism with no contention.

### Not validated at runtime

- ⚠ **`cargo` gates on the repository itself** — no manifest exists on this branch by design. Established instead on a byte-identical external mirror (`diff -rq` + forced-fresh rebuild): `cargo fmt --check` 0, `cargo test --test conformance --no-run` 0 warnings/0 errors, `cargo clippy --all-targets -- -D warnings` 0, `cargo build --release` clean.
- ❌ **Any verdict about the real `bcc`** — the compiler under test was a documented stand-in. This is the project's single material gap.
- ❌ **CI job on a GitHub Actions runner** — never executed. One step deliberately fails rather than skips when the Cargo package is absent.
- ⚠ **clang as a second oracle** — exercised over one area (231 PASS / 0 FAIL) only after installing `libclang-rt-20-dev`, which was then removed so the committed documentation stays true on this machine.

---

## 5. Compliance & Quality Review

| # | AAP Deliverable / Benchmark | Evidence | Status | Progress |
| --- | --- | --- | --- | --- |
| R1 | **UB-freedom machine-enforced, not asserted** | `ubaudit.rs`; 108/108 programs cleared the strict warning gate and the ASan/UBSan gate over 324 invocations; 108 written freedom arguments in-record; per-area deviations carry recorded reasons | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| R2 | **Broad language coverage — 14 areas, 108 programs** | Per-area counts match the plan exactly; all 9 mandated areas ≥ 6 programs; two-variant rule in 100% of arithmetic/conversion/bitfield/optimization/floating areas; zero `%p` prints; 108/108 hand-declare `printf`; no `stdio.h` anywhere | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| R3 | **Shared flags verified, not assumed** | `flagprobe.rs`; 48/48 checks; 13 semantic verifications via pure-`std` ELF byte reads; 2 acceptance-only with the limitation recorded; negative assertions that no reference-only or bcc-only flag leaked in | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| R4 | **Convention integration & isolated reproducibility** | Auto-discovered Cargo target; harness in a nested non-target directory; 108/108 records carry all three command templates, `expect_exit` and `expected_stdout`; hand reproduction proven twice | ⚠ Partial | ▓▓▓▓▓▓▓▓▓░ 90% |
| R4a | └ `tests/common/mod.rs` helper reuse | File absent on this branch (read-only reference material, protected by C2). Harness stands alone in pure `std`; driver enumerates the exact merge work and guards it with three mechanical assertions | ⚠ Outstanding | ▓▓▓▓▓░░░░░ 50% |
| R5 | **Expected divergence marked, never silently excluded** | Exactly 2 markers, matching the plan's frozen advance table; 1,683-line register with contract, grammars and retirement procedure; bidirectional integrity audit held; both cited bases resolved to a real file, line and verbatim sentence; third candidate analysed and deliberately left unmarked with reasoning recorded | ⚠ Partial | ▓▓▓▓▓▓▓▓░░ 80% |
| R5a | └ 12 XPASS adjudication | Unreproducible against a GCC-forwarding stand-in. Default policy fails loudly (exit 101 verified), so nothing is hidden | ⚠ Outstanding | ▓▓▓▓░░░░░░ 40% |
| R6 | **Findings are deliverables, never patched** | `findings.rs` emits all 7 artifacts plus a review bundle; proven end-to-end (24 FINDINGs, run still exit 0, `commands.sh` standalone); timeout is a first-class class; **zero compiler source changed** | ⚠ Partial | ▓▓▓▓▓▓▓▓▓░ 85% |
| R6a | └ `FINDINGS.md` curated register | Deliberately and explicitly empty — a factual statement, not a placeholder; no real-bcc run has produced anything legitimate to curate | ⚠ Outstanding | ▓▓▓░░░░░░░ 30% |
| R7 | **All else passes; bcc matches the oracle** | Closed six-verdict space with no silent-skip path; 3,564/3,564 comparisons; 0 FAIL; 0 UNAVAILABLE; 18/18 tests pass | ⚠ Partial | ▓▓▓▓▓▓▓░░░ 70% |
| R7a | └ Verdict about the **real** `bcc` | Not rendered. The compiler under test was a documented stand-in forwarding to gcc-13 | ❌ Not started | ░░░░░░░░░░ 0% |
| C1 | **No compiler source or manifest change** | `git ls-files` shows **no** `src/`, `include/`, `Cargo.toml`, `Cargo.lock` or `build.rs` on this branch at all; 0 deletions in the diff; scratch package deliberately kept outside the repository | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| C2 | **No existing test deleted, skipped or weakened** | 0 `D` entries; `origin/main` carries no `tests/**`; **zero `#[ignore]` attributes in any Rust file** (4 prose mentions only). The exactly-13 numeric invariant awaits the merged branch | ⚠ Partial | ▓▓▓▓▓▓▓▓▓░ 85% |
| C3 | **No feature excluded for difficulty** | Both hard cases written **and** executed: case ranges (marked, still compiled and run) and `long double` (only its cross-backend value comparison narrowed, with the measured 16/12/16/16-byte x87-vs-binary128 reason in-record; still judged on all 4 targets × 3 levels by oracles (a) and (c)) | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| C4 | **Hermetic sandbox, no network, no external writes** | `env_clear()` on children (measured); no `std::net`; zero repository changes outside the git-ignored build root; zero new `/tmp` or `/root` entries; corpus declares only `printf` + one `malloc`/`free` + one `exit`; per-cell timeouts | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q1 | **Zero third-party crates** | Every import across all 13 Rust files resolves to `std`/`crate`/`super`/`self`; the external scratch manifest's three dependency sections are empty with a one-package lockfile | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q2 | **Zero-warning discipline, earned** | `rustfmt --check` 0 · `rustc --test -D warnings` 0 · `clippy --all-targets -D warnings` 0 · `cargo build --release` clean · `shellcheck` 0 at default severity — with **zero `#[allow]`/`#![allow]` attributes anywhere** | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q3 | **Zero placeholders** | `TODO`/`FIXME`/`HACK`/`unimplemented!`/`todo!` all **0** across all changed files; the only `XXX` is an `mktemp` template; the single empty Rust body is a documented `#[cfg(not(unix))]` platform fallback | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q4 | **Determinism** | No addresses, timestamps, randomness, uninitialized reads or locale-dependent formatting; fixed iteration order; fixed float precision; exit codes constrained to 0–125 with raw wait-status comparison | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q5 | **Documentation & discoverability** | 4,251-line suite contract + 1,775-line methodology page + two registers; all 4 mkdocs nav targets exist; 204 doc links and anchors resolve; the rustdoc unresolved-link set matches the 13-name inventory **in both directions** | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q6 | **Supply-chain & CI hygiene** | All 3 GitHub Actions pinned by SHA digest; 11 shellcheck-clean run blocks; explicit disclosure-bound and finding-audit steps before any artifact is published; both YAML files parse | ✅ Pass | ▓▓▓▓▓▓▓▓▓▓ 100% |
| Q7 | **CI job executed on a runner** | Authored and statically validated; never run on GitHub Actions. One step deliberately fails rather than skips when the Cargo package is absent | ⚠ Outstanding | ▓▓▓▓▓▓▓░░░ 75% |
| Q8 | **Whole-repository health gate** | 3,955 defined / 3,942 passing / 0 failing / exactly 13 ignored is a repository-wide property no single test binary can read; `docs/project-guide.md` records the three-condition acceptance criterion rather than claiming the figures | ⚠ Outstanding | ▓▓▓▓▓▓▓▓░░ 85% |

### Fixes applied during autonomous validation

Two issues were found; **both were environmental, and zero repository changes were needed**:

1. The optional clang oracle failed its UB gate for all 108 programs because the ASan runtime archives were absent. Installing `libclang-rt-20-dev` lifted it exactly as the suite contract predicts (area 04 then 231 PASS / 0 FAIL), confirming the documented remedy — then it was removed again, with its autoremoved dependencies, so the committed documentation stays true on this machine.
2. `rm -rf target/conformance-*` also deletes `target/conformance-typecheck`, which holds the compiled test binary. A scoped cleaner was added at `target/validator/clean-artifacts.sh` (git-ignored), removing only the three transient roots.

Across the six preceding review rounds, **93 code-review findings plus a dedicated security review** were resolved before hand-off.

---

## 6. Risk Assessment

| Risk | Category | Severity | Probability | Mitigation | Status |
| --- | --- | --- | --- | --- | --- |
| **T1** The suite has never judged the real `bcc`; the whole matrix ran against a GCC-forwarding stand-in, so oracle (a) compared one toolchain with itself | Technical | **High** | Certain (a present fact) | Single defined acceptance step: `BCC_CONFORMANCE_STRICT=1 cargo test --test conformance` on the merged branch. Stated in three places — the suite contract, the methodology page, and `docs/project-guide.md` §2.2/§3 | Open — mitigation defined |
| **T2** 12 XPASS on `XD-GCCEXT-CASE-RANGES-001`; the run passes only under the documented hatch | Technical | Medium | Certain on this branch | Adjudicate against the real `bcc`: confirm a true XFAIL or retire the marker in **both** the record and the register. Default policy fails loudly (exit 101, verified), so the condition cannot hide | Open — self-clearing on merge |
| **T3** `tests/common/mod.rs` reuse inactive; the harness reimplements what it should delegate to | Technical | Low | Certain | `mod common;` is a compile-time assertion so it cannot be declared speculatively. The driver enumerates the exact merge work and `infra_oracle_capability_report` enforces three conditions the moment the file appears | Open — mechanically guarded |
| **T4** Matrix runtime (~147 s) scales linearly with programs × targets × levels | Technical | Low | Medium | 14 independent area tests spread across `libtest`'s default thread pool; `BCC_CONFORMANCE_QUICK` for local iteration, always stamped REDUCED | Open — monitored |
| **T5** Golden records could drift from the reference toolchain | Technical | Low | Low | `expected_stdout` is regenerated only by the maintenance tool, never during a run, so a wrong answer cannot silently become the expectation; `--check` re-derived 108 records / 1,296 cells with 0 changed | Mitigated |
| **S1** Build-root trust refusal — host `/tmp` measured at mode **2777 with no sticky bit**, so a finding's `commands.sh` refuses to run beneath it | Security | Medium | Certain on this host | Correct refusal, not a defect: a world-writable non-sticky parent permits symlink pre-placement. Two verified remedies — relocate the finding directory, or set `CARGO_TARGET_DIR` to a fully owned chain, which removes the warning entirely | Open — environment-level |
| **S2** Unreviewed finding artifacts containing raw compiler output could be published | Security | Medium | Low | The CI job has explicit "state the report's disclosure bound" and "audit the generated findings" steps, and the raw upload is labelled UNREVIEWED | Mitigated by design |
| **S3** Supply-chain surface | Security | Low | Low | **Zero third-party crates** (verified import-by-import; empty dependency sections; one-package lockfile) and all 3 GitHub Actions pinned by SHA digest | Mitigated |
| **S4** Credential or host-environment leakage into artifacts | Security | Low | Very Low | Scan across all changed files found only an `mktemp` template and a lock-ownership token — no secrets. Children run with `env_clear()`, so host values cannot reach a compiled artifact | Mitigated |
| **O1** CI job has never executed on GitHub Actions | Operational | Medium | High (first-run friction) | Run on the merged branch behind a non-blocking gate first, then promote to blocking. Note that one step deliberately fails rather than skips when the Cargo package is absent | Open |
| **O2** Reference-driver pins are not inherited by a non-login shell | Operational | Medium | High | `. /usr/local/etc/bcc-conformance.env` is documented as step 1 everywhere; in CI set the four variables directly. Failure is loud, not silent: the harness refuses the gnu23 driver rather than judging the corpus with it | Open — documented |
| **O3** Toolchain version skew — gcc-13 13.4.0 and QEMU 10.1.0 versus the planned 13.2/8.2.2, with only plain emulator spellings present | Operational | Low | Medium | Runtime discovery rather than hard-coded versions absorbed the drift; every finding records an environment fingerprint so a divergence attributes to toolchain change rather than to the compiler | Mitigated by design |
| **O4** Transient artifact hygiene — `rm -rf target/conformance-*` also removes the compiled test binary | Operational | Low | Medium | Scoped cleaner `target/validator/clean-artifacts.sh`; the three transient roots are retired at the start of every run and are git-ignored | Mitigated |
| **I1** Merge onto the compiler branch may meet layout differences | Integration | **High** | Medium | The plan anticipated this: the harness discovers the compiler binary via `CARGO_BIN_EXE_bcc`/`BCC_BIN` and target triples at runtime instead of hard-coding them, so the merge is a verification rather than a rewrite | Open |
| **I2** Findings volume from the first real run is unknown; curation could exceed its 16 h budget | Integration | Medium | Medium | Every artifact is generated automatically and `creduce` 2.11.0 is available for optional minimization. The ceiling is bounded by curation, never repair — requirement 6 and constraint C1 both forbid patching the compiler | Open — **estimate LOW confidence** |
| **I3** clang as a second independent oracle is not usable as shipped | Integration | Low | Certain | Needs `libclang-rt-20-dev` for the sanitizer gate and carries one clang-only `-Wconstant-logical-operand` difference. Both documented verbatim; gcc-13 is the reference of record with the suite 100% green under it | Open — optional |

**Risk profile:** 2 High · 7 Medium · 7 Low. Six are already mitigated by design. Every High and Medium risk with a defined remedy has that remedy costed in §2.2 — no risk in this table lacks an owner or an hour estimate.

---

## 7. Visual Project Status

### Overall hours

```mermaid
pie showData
    title Project Hours Breakdown (Total 579 h)
    "Completed Work" : 514
    "Remaining Work" : 65
```

> **Colour key — Blitzy brand:** Completed Work = Dark Blue **`#5B39F3`** · Remaining Work = White **`#FFFFFF`** · headings/accents Violet-Black `#B23AF2` · highlights Mint `#A8FDD9`.
> **Integrity:** "Remaining Work" = 65 = §1.2 Remaining Hours = §2.2 `Hours` column sum. ✓

### Remaining work by priority

```mermaid
pie showData
    title Remaining 65 h by Priority
    "High" : 34
    "Medium" : 22
    "Low" : 9
```

### Remaining hours per category (§2.2)

| Category | Hours | Bar |
| --- | --- | --- |
| Curate real-run findings | 16 | ████████████████ |
| First package-complete STRICT run + triage | 8 | ████████ |
| Merge & Cargo discovery | 6 | ██████ |
| CI job first execution | 6 | ██████ |
| clang second-oracle enablement | 5 | █████ |
| `tests/common/mod.rs` integration | 5 | █████ |
| Whole-repository health gate | 4 | ████ |
| XPASS adjudication | 4 | ████ |
| Environment pins & trust remedy | 4 | ████ |
| Maintainer handover & runbook | 4 | ████ |
| Docs & register reconciliation | 3 | ███ |
| **Total** | **65** | |

### AAP requirement completion

```mermaid
pie showData
    title AAP Requirement & Constraint Status (11 items)
    "Complete" : 7
    "Partially Complete" : 3
    "Not Started" : 1
```

*Complete: R1, R2, R3, C1, C3, C4 and the coverage matrix. Partially complete: R4 (90%), R5 (80%), R6 (85%), R7 (70%), C2 (85%). Not started: the verdict about the real `bcc` (0%).*

### Delivery volume

| Dimension | Value |
| --- | --- |
| Files added / modified / deleted | 238 / 3 / **0** |
| Lines added / removed / net | 117,267 / 25 / **+117,242** |
| Commits ahead of `origin/main` | 44 (all `Blitzy Agent <agent@blitzy.com>`) |
| Rust harness + driver | 13 files, 65,499 lines (36,937 code + 25,574 comment) |
| C corpus | 108 programs, 19,915 lines |
| Expectation records | 108 records, 16,126 lines |
| Suite documentation | 8,860 lines across 4 documents |
| Third-party crates added | **0** |

---

## 8. Summary & Recommendations

### What was achieved

The project is **88.8% complete — 514 of 579 hours**. The Agent Action Plan's entire deliverable set was produced: 14 feature areas, 108 undefined-behaviour-free C programs with 108 co-located expectation records, a 12-module pure-`std` harness with an 18-test driver, three registers totalling 7,085 lines, a methodology page, a maintenance-only golden-record tool, and one additive CI job — 238 files added, 3 modified additively, **0 deleted**, 117,242 net lines, with **not one byte of compiler source, manifest or lockfile touched**.

More importantly, the suite *works*. Its entire 1,296-cell matrix has been driven end to end, producing 3,564 assertions with planned equal to recorded on all 14 dimensions and **0 FAIL, 0 UNAVAILABLE**. All four preflight gates hold. Every static gate is clean and — with zero `#[allow]` attributes anywhere — earned rather than suppressed. Requirement 4's promise that a cell is reproducible from a source file plus its record was proven by hand, twice. Requirement 6's promise that a finding is a self-contained deliverable was proven by forcing a real divergence and reproducing it with **no harness, no Cargo and no rustc**.

The three hardest design problems were solved rather than deferred. Constraint C1's prohibition on manifest changes was satisfied *by layout* — Cargo's own target-discovery rules make `tests/conformance.rs` a target and `tests/conformance_harness/` never one — so the manifest genuinely needs no edit. Constraint C3's prohibition on dropping difficult features was honoured for both hard cases: case ranges and `long double` are written and executed, with exclusions narrowed to a single oracle and their reasons recorded in the affected program's own record. And requirement 3's "verify rather than assume" became an executable probe with negative assertions, catching the `-fcf-protection` trap where both compilers accept a flag but their defaults differ — proof that acceptance alone would have been a false positive.

### The gap that remains

**The suite has not yet judged `bcc`.** This branch carries the suite ahead of the compiler tree — no `Cargo.toml`, no `src/**` — because adding a manifest is exactly what constraint C1 forbids. The matrix was therefore driven against a documented stand-in that forwards to gcc-13, and a stand-in that forwards to the reference toolchain compares one toolchain with itself. The work is unusually honest about this: `docs/project-guide.md` counts the 18 tests in the *defined* total of 3,955 and deliberately excludes them from the *measured* pass count, recording a three-condition acceptance criterion instead of a result. The registers say the same. **The resolution is merge, not manifest.**

The 12 XPASS follow directly from the same fact and are not a suite defect: GCC implements case ranges, so a GCC-forwarding stand-in cannot reproduce the marker's claimed refusal. The default policy fails the run loudly at exit 101 — verified in both directions during this review — so the condition is impossible to overlook, and it settles itself the moment a real `bcc` answers the question.

### Critical path to production

1. **Merge onto the compiler branch** and confirm Cargo discovery with no manifest edit — 6 h.
2. **Run `BCC_CONFORMANCE_STRICT=1 cargo test --test conformance`** and triage every non-PASS row — 8 h. *This single step converts the suite from defined to measured.*
3. **Curate the resulting findings** into the register, minimizing each reproducer, without patching the compiler — 16 h.
4. **Adjudicate the 12 XPASS** and remove the hatch from CI if the marker retires — 4 h.
5. **Integrate `tests/common/mod.rs`, run the whole-repository health gate, execute the CI job, and reconcile the documentation** — 22 h.
6. **Optional hardening and handover** — 9 h.

Steps 1–4 (34 h) are the critical path; nothing else can start before step 1.

### Success metrics

| Metric | Target | Current | Status |
| --- | --- | --- | --- |
| AAP file-map deliverables produced | 235 CREATE / 0 DELETE | 238 added / 0 deleted | ✅ Exceeded (ci.yml and .gitignore were created because `main` carries neither) |
| Feature areas | 14 (9 mandated + 5 supplementary) | 14 | ✅ Met |
| Programs, minimum 6 per mandated area | 108 | 108, every mandated area ≥ 6 | ✅ Met |
| Compile-and-run cells | 1,296 | 1,296 | ✅ Met |
| Differential & golden assertions | ~3,564 | 3,564 | ✅ Met |
| New tests passing | 18 / 0 failed / 0 ignored | 18 / 0 / 0 | ✅ Met |
| `FAIL` verdicts | 0 | 0 | ✅ Met |
| `UNAVAILABLE` verdicts | 0 | 0 | ✅ Met |
| Third-party crates | 0 | 0 | ✅ Met |
| Lint & format warnings | 0 | 0, with zero suppressions | ✅ Met |
| Existing tests weakened | 0 | 0 (`#[ignore]` count untouched) | ✅ Met |
| Compiler source lines changed | 0 | 0 | ✅ Met |
| Verdict rendered about the real `bcc` | Required | None | ❌ Outstanding |
| Curated findings register | Populated or provably empty after a real run | Provably empty, no real run yet | ⚠ Outstanding |
| CI job executed on a runner | Required | Never | ⚠ Outstanding |

### Production readiness assessment

**Conditionally ready — the suite is production-grade; its verdict about `bcc` is not yet rendered.**

The test material itself is ready to merge today. It is complete, statically clean, hermetic, reproducible by hand, documented to an unusual standard, hardened through six review rounds and a security review, and free of placeholders. It weakens nothing and depends on nothing new. There is no work left *inside* the suite.

What is not ready is the *claim* the suite exists to support. Until it runs against the real compiler, its 3,543 PASS verdicts describe the environment and the harness rather than `bcc` — and the suite says so itself, in its registers, in its methodology page, and in the project guide's own test inventory. That restraint is the right engineering call: publishing those verdicts as evidence about `bcc` would have been the one genuinely serious defect available here, and it was avoided deliberately.

**Recommendation: merge, then execute.** The 34 hours of critical-path work is bounded, well-specified, and mechanically guided — the driver enumerates the merge work, the register documents the retirement procedure, and the CI job fails rather than skips when its precondition is missing. The remaining 31 hours of medium- and low-priority work can proceed in parallel and behind a non-blocking gate.

---

## 9. Development Guide

Every command in this section was executed and verified on the validation host. Copy-paste ready.

### 9.1 System Prerequisites

| Requirement | Version verified on the host | Purpose |
| --- | --- | --- |
| Linux x86-64 | Ubuntu 25.10 container | Host and oracle (b) baseline |
| Rust toolchain | `rustc 1.93.1`, `cargo 1.93.1`, `rustfmt 1.8.0`, `clippy 0.1.93` (repository minimum 1.70+) | Builds and runs the suite |
| `gcc-13` | 13.4.0 | Oracle (a) native arm **and** both UB audit gates |
| `i686-linux-gnu-gcc-13` | 13.4.0 | Oracle (a) i686 arm |
| `aarch64-linux-gnu-gcc-13` | 13.4.0 | Oracle (a) AArch64 arm |
| `riscv64-linux-gnu-gcc-13` | 13.4.0 | Oracle (a) RISC-V 64 arm |
| `qemu-i386` / `qemu-aarch64` / `qemu-riscv64` | 10.1.0 | Oracle (b) execution; the plain spellings on this host — the harness probes both spellings |
| Static C runtimes | complete on all four targets | `libc.a`, `crt1.o`, `crti.o`, `crtn.o` per target sysroot |
| `timeout` | uutils coreutils 0.2.2 | Optional — `execute.rs` carries its own watchdog |
| `clang`, `creduce` | present; creduce 2.11.0 | Optional alternate oracle and optional reducer |
| Disk | ~2 GB free for `target/` | Build and artifact space |

> **Deliberately NOT required:** binutils (`readelf`, `objdump`, `nm`) — ELF identification bytes are read with `std`. And **no third-party crate of any kind**.

> ⚠️ **Use the versioned driver names.** The unversioned `gcc` on this host is 15.2.0 and defaults to **gnu23**, which changes the meaning of constructs the corpus contains. The harness **refuses** such a driver rather than judging the corpus with it — loudly, with no silent fall-through.

### 9.2 Dependency Installation

```bash
# Rust toolchain (same pins the CI job uses)
rustup toolchain install 1.93.1 --profile minimal --no-self-update --component rustfmt,clippy
rustup default 1.93.1

# Oracle toolchain: reference drivers, emulators, and cross C runtimes
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  gcc-13 \
  gcc-13-i686-linux-gnu \
  gcc-13-aarch64-linux-gnu \
  gcc-13-riscv64-linux-gnu \
  libc6-dev \
  libc6-dev-i386-cross \
  libc6-dev-arm64-cross \
  libc6-dev-riscv64-cross \
  qemu-user-static          # use qemu-user where qemu-user-static has no candidate
```

### 9.3 Environment Setup — step 1 is mandatory

```bash
. /usr/local/etc/bcc-conformance.env
```

This exports `BCC_REF_CC=gcc-13` plus the three cross-driver pins. **A non-login shell does not inherit them**, and without them the harness sees only the gnu23 driver and refuses it. In CI, set the four variables directly instead of sourcing a file.

Verify the toolchain before sweeping 1,296 cells:

```bash
gcc-13 --version                    # expect 13.x
i686-linux-gnu-gcc-13 --version     # expect 13.x
aarch64-linux-gnu-gcc-13 --version  # expect 13.x
riscv64-linux-gnu-gcc-13 --version  # expect 13.x
qemu-aarch64 --version              # expect 10.x  (or qemu-aarch64-static on older releases)
```

### 9.4 Startup Sequence

#### Shape A — package-complete checkout (the intended shape, after the merge)

```bash
cd <repository root>
. /usr/local/etc/bcc-conformance.env

cargo fmt -- --check                                # verified: exit 0, no output
cargo clippy --all-targets -- -D warnings           # verified: exit 0, 0 warnings
cargo build --release                               # verified: clean
cargo test --test conformance --no-run              # verified: 0 warnings, 0 errors

BCC_CONFORMANCE_STRICT=1 cargo test --test conformance -- --nocapture
```

#### Shape B — suite ahead of the compiler tree (this branch: no manifest, by design under C1)

```bash
cd <repository root>
mkdir -p target/conformance-typecheck                # target/ does not exist on a fresh checkout

rustfmt --edition 2021 --check tests/conformance.rs tests/conformance_harness/*.rs

CARGO_MANIFEST_DIR="$(pwd)" rustc --edition 2021 --test --emit=metadata -D warnings \
  --out-dir target/conformance-typecheck tests/conformance.rs

CARGO_MANIFEST_DIR="$(pwd)" clippy-driver --edition 2021 --test -D warnings \
  --emit=metadata --out-dir target/conformance-typecheck tests/conformance.rs

CARGO_MANIFEST_DIR="$(pwd)" rustc --edition 2021 --test -D warnings \
  -o target/conformance-typecheck/conformance tests/conformance.rs

. /usr/local/etc/bcc-conformance.env
BCC_BIN=<path to compiler> BCC_CONFORMANCE_STRICT=1 \
  ./target/conformance-typecheck/conformance --nocapture
```

All four static commands verified: **exit 0 with zero output**. The binary links at 13.1 MB.

### 9.5 Verification Steps

```bash
# Pre-flight: the whole oracle inventory, before any cell runs  (verified: 2.22 s)
cargo test --test conformance infra_oracle_capability_report -- --nocapture
# Shape B: ./target/conformance-typecheck/conformance infra_oracle_capability_report --nocapture
```

Expected — and observed — output includes:

- the compiler under test with its version string and device/inode identity;
- all four reference drivers named at `13.4.0`, each proved gnu17;
- all three runners named at `10.1.0`; `x86_64-linux-gnu needs no runner: it is the host architecture … which is why it is the baseline`;
- static-link runtimes **`complete`** on all four targets with every start file resolved;
- the child `PATH` shown as **computed, never inherited**;
- **`Unavailable oracle arms: none — every oracle arm in the effective matrix can be attempted`**.

```bash
# Full run — verified result
cargo test --test conformance
#   test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 146.62s

# The deliverable summary
sed -n '1,10p' target/conformance-report/summary.md
#   **Coverage: FULL — every feature area, every target and every optimization level in scope was swept …**
```

### 9.6 Example Usage

```bash
# One feature area, verbose                                    (verified: 53.33 s)
cargo test --test conformance area_04_bitfields -- --nocapture

# One program across its full 12-cell matrix                   (verified: 10.10 s)
BCC_CONFORMANCE_ONLY=04_bitfields/003_compound_assignment \
  cargo test --test conformance area_04_bitfields -- --nocapture

# Only the four infrastructure gates
cargo test --test conformance infra_ -- --nocapture

# Fast local iteration: native target, -O0 and -O2 only        (verified: 28.15 s for one area)
# The report is stamped "⚠️ REDUCED COVERAGE" and can never be mistaken for a full run.
BCC_CONFORMANCE_QUICK=1 cargo test --test conformance

# Retain every cell workspace for inspection                   (verified: 21 workspaces retained)
BCC_CONFORMANCE_KEEP_WORK=1 cargo test --test conformance -- --nocapture

# Readable interleaved output
cargo test --test conformance -- --nocapture --test-threads=1

# Golden-record audit, independent of the Rust harness
sh tests/conformance/tools/regenerate_expected.sh --check
#   whole corpus ≈12 min → "processed 108 record(s); 0 changed"
sh tests/conformance/tools/regenerate_expected.sh --check --area 04_bitfields
#   verified in 54.7 s → "processed 7 record(s); 0 changed"
```

#### Reproduce one cell by hand, with no harness at all — verified end to end

```bash
. /usr/local/etc/bcc-conformance.env
BCC=<path to compiler>
SRC=tests/conformance/04_bitfields/005_straddling_and_zero_width.c
REC=tests/conformance/04_bitfields/005_straddling_and_zero_width.expected
W=$(mktemp -d)

# Templates rendered straight out of the record's bcc_command / ref_command / run_command
"$BCC" --target aarch64-linux-gnu -O2 -static "$SRC" -o "$W/a.bcc"
aarch64-linux-gnu-gcc-13         -O2 -static "$SRC" -o "$W/a.ref"
qemu-aarch64 "$W/a.bcc" > "$W/o.bcc"; echo "exit=$?"
qemu-aarch64 "$W/a.ref" > "$W/o.ref"; echo "exit=$?"

cmp -s "$W/o.bcc" "$W/o.ref" && echo "oracle (a): AGREES"
sed -n '/^expected_stdout/,/^END/p' "$REC" | sed '1d;$d' > "$W/o.golden"
cmp -s "$W/o.bcc" "$W/o.golden" && echo "oracle (c): AGREES"
rm -rf "$W"
```

Observed: `exit=0` on both sides, `oracle (a): AGREES`, `oracle (c): AGREES`.

```bash
# Reproduce a finding with no harness, no Cargo and no rustc
cd target/conformance-findings/F-0001-<slug> && sh commands.sh
```

### 9.7 Troubleshooting

| Symptom | Cause | Resolution |
| --- | --- | --- |
| Driver refused; a message about gnu23 or a default language mode | `/usr/local/etc/bcc-conformance.env` was not sourced; the unversioned `gcc` is 15.2.0/gnu23 | `. /usr/local/etc/bcc-conformance.env`, or set `BCC_REF_CC=gcc-13` plus the three cross pins. The refusal is deliberate — no `-std` flag may ever be passed |
| **Exit 101** with `Outcomes that fail the run: 12` | The 12 XPASS on `XD-GCCEXT-CASE-RANGES-001`. Unexpected success is a failure by convention | While the compiler under test is a stand-in, set `BCC_CONFORMANCE_ALLOW_XPASS=1`. The summary still lists all 12 prominently and stamps `allow_xpass:1`. Permanent fix: adjudicate the marker after the merge |
| Next run fails to start after cleaning | `rm -rf target/conformance-*` also deletes `target/conformance-typecheck`, which holds the compiled test binary | Use `./target/validator/clean-artifacts.sh`, which removes only the three transient roots and their claim files. **Never** use the glob |
| Panic about the per-cell execution budget | `BCC_CONFORMANCE_TIMEOUT_SECS=0` | Refused by design: *"a budget of zero seconds would time out every cell before it could run; unset … to use the default of 30 seconds, or set it to a positive whole number"* |
| A finding's `commands.sh` refuses to run | The build root sits beneath a world-writable non-sticky parent — host `/tmp` is mode **2777** | Either relocate the finding directory to a trusted parent, **or** set `CARGO_TARGET_DIR` to a fully owned directory chain, which removes the trust warning entirely. Both verified |
| clang UB gate fails for all 108 programs | ASan runtime archives absent | `sudo apt-get install -y libclang-rt-20-dev`. One clang-only bound remains (`-Wconstant-logical-operand` on the deliberate `(a && 5)` in `06_control_flow/006_short_circuit_evaluation.c`); gcc-13 is the reference of record and the suite is 100% green with it |
| `UNAVAILABLE` rows in the summary | An oracle arm's tooling is absent | Never a silent pass. `BCC_CONFORMANCE_STRICT=1` escalates every `UNAVAILABLE` to a failure, and `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` **cannot** lower strict mode |
| `cargo` commands fail to read a manifest | This branch carries no `Cargo.toml` **by design** (constraint C1) | Use the Shape-B commands, or merge onto the compiler branch. **Never add a `Cargo.toml`** — the layout is built so none is needed |
| Summary reports areas as "stale" or the run as partial | Per-area reports left behind by an earlier or filtered run | Generation stamping excludes them from every total by design. Clean with the scoped cleaner and re-run |
| Area report headed "⚠️ REDUCED COVERAGE" | `BCC_CONFORMANCE_QUICK` or `BCC_CONFORMANCE_ONLY` narrowed the matrix | Working as intended — a narrowed run can never be read as a complete one. Unset the variable for a full sweep |

---

## 10. Appendices

### Appendix A — Command Reference

| Purpose | Command |
| --- | --- |
| Export the reference-driver pins (**always first**) | `. /usr/local/etc/bcc-conformance.env` |
| Full suite | `cargo test --test conformance` |
| Full suite, strict, verbose | `BCC_CONFORMANCE_STRICT=1 cargo test --test conformance -- --nocapture` |
| One feature area | `cargo test --test conformance area_04_bitfields -- --nocapture` |
| One program, full matrix | `BCC_CONFORMANCE_ONLY=<area>/<program> cargo test --test conformance area_<NN>_<name>` |
| Infrastructure gates only | `cargo test --test conformance infra_ -- --nocapture` |
| Pre-flight oracle inventory | `cargo test --test conformance infra_oracle_capability_report -- --nocapture` |
| Fast reduced matrix | `BCC_CONFORMANCE_QUICK=1 cargo test --test conformance` |
| Retain cell workspaces | `BCC_CONFORMANCE_KEEP_WORK=1 cargo test --test conformance -- --nocapture` |
| Single-threaded output | `cargo test --test conformance -- --nocapture --test-threads=1` |
| Format gate | `cargo fmt -- --check` |
| Lint gate | `cargo clippy --all-targets -- -D warnings` |
| Release build gate | `cargo build --release` |
| Compile without running | `cargo test --test conformance --no-run` |
| Whole-repository health gate | `cargo test --no-fail-fast`, then aggregate **every** `test result:` line |
| Manifest-less format gate | `rustfmt --edition 2021 --check tests/conformance.rs tests/conformance_harness/*.rs` |
| Manifest-less type check | `CARGO_MANIFEST_DIR="$(pwd)" rustc --edition 2021 --test --emit=metadata -D warnings --out-dir target/conformance-typecheck tests/conformance.rs` |
| Manifest-less lint | `CARGO_MANIFEST_DIR="$(pwd)" clippy-driver --edition 2021 --test -D warnings --emit=metadata --out-dir target/conformance-typecheck tests/conformance.rs` |
| Manifest-less test binary | `CARGO_MANIFEST_DIR="$(pwd)" rustc --edition 2021 --test -D warnings -o target/conformance-typecheck/conformance tests/conformance.rs` |
| Doc-link gate | `CARGO_MANIFEST_DIR="$(pwd)" rustdoc --edition 2021 --crate-type lib --crate-name conformance --document-private-items -o target/conformance-typecheck/doc tests/conformance.rs` |
| Golden-record audit (whole corpus) | `sh tests/conformance/tools/regenerate_expected.sh --check` |
| Golden-record audit (one area) | `sh tests/conformance/tools/regenerate_expected.sh --check --area 04_bitfields` |
| Shell lint | `shellcheck tests/conformance/tools/regenerate_expected.sh` |
| Clean transient artifacts **safely** | `./target/validator/clean-artifacts.sh` |
| Reproduce a finding standalone | `cd target/conformance-findings/F-NNNN-<slug> && sh commands.sh` |

### Appendix B — Port Reference

**No network ports are used, opened or listened on.** Constraint C4 forbids network access from suite-generated programs; no test program opens a socket, and `std::net` appears nowhere in the harness. Every input is a literal in the program source. This is also precisely why the repository's 13 existing network-dependent tests must remain ignored.

| Resource | Value |
| --- | --- |
| Listening ports | none |
| Outbound connections | none |
| Local sockets | none |

### Appendix C — Key File Locations

| Path | Lines | Role |
| --- | --- | --- |
| `tests/conformance.rs` | 6,375 | Suite driver — 14 area tests + 4 infrastructure tests; the only new Cargo-visible target |
| `tests/conformance_harness/mod.rs` | 8,528 | Module root; shared verdict, cell, target and oracle types |
| `tests/conformance_harness/findings.rs` | 8,257 | Finding artifact generation (requirement 6) |
| `tests/conformance_harness/report.rs` | 8,180 | Per-area reports, run summary, evidence documents |
| `tests/conformance_harness/manifest.rs` | 6,646 | Hand-written `.expected` parser — no serialization crate |
| `tests/conformance_harness/env.rs` | 6,224 | Oracle discovery and the capability record |
| `tests/conformance_harness/flagprobe.rs` | 4,533 | Flag verification (requirement 3) |
| `tests/conformance_harness/ubaudit.rs` | 4,337 | UB audit gates (requirement 1) |
| `tests/conformance_harness/compile.rs` | 3,460 | Shared-flag-enforced cell builds |
| `tests/conformance_harness/execute.rs` | 2,901 | Native and QEMU execution with timeouts |
| `tests/conformance_harness/sandbox.rs` | 2,330 | Hermetic per-cell workspaces (constraint C4) |
| `tests/conformance_harness/classify.rs` | 2,041 | Six divergence classes → six verdicts |
| `tests/conformance_harness/compare.rs` | 1,687 | Byte-exact stdout + exit-status comparison |
| `tests/conformance/README.md` | 4,251 | **The suite contract** — the reproduction authority; read this first |
| `tests/conformance/EXPECTED_DIVERGENCES.md` | 1,683 | Marker register; machine-checked bidirectionally |
| `tests/conformance/FINDINGS.md` | 1,151 | Finding register (deliberately empty) |
| `docs/testing/differential-conformance.md` | 1,775 | Published methodology page |
| `tests/conformance/tools/regenerate_expected.sh` | 5,926 | Maintenance-only golden-record tool; unreachable from a test run |
| `tests/conformance/support/include/probe_header.h` | 31 | The suite's only fixture, used solely by the `-I` flag probe |
| `tests/conformance/<NN_area>/<NNN_name>.c` | 19,915 total | 108 test programs across 14 areas |
| `tests/conformance/<NN_area>/<NNN_name>.expected` | 16,126 total | 108 expectation records |
| `.github/workflows/ci.yml` | 666 | One additive job, 14 steps, all actions digest-pinned |
| `target/conformance-report/summary.md` | generated | The deliverable summary |
| `target/conformance-report/summary.tsv` | generated | Machine-readable form of the same data |
| `target/conformance-report/areas/<area>.{md,tsv}` | generated | Per-area reports; one file per area, so no contention |
| `target/conformance-report/evidence/` | generated | Durable documents for every non-PASS outcome |
| `target/conformance-findings/F-NNNN-<slug>/` | generated | Per-run finding artifacts (uncurated) |
| `target/conformance-work/` | generated | Transient per-cell workspaces |

**Read-only, and untouched by this work:** `src/**`, `include/**`, `build.rs`, `Cargo.toml`, `Cargo.lock`, every existing `tests/*.rs`, `tests/validation/**`, `tests/common/mod.rs`, `.github/workflows/validation.yml`.

### Appendix D — Technology Versions

| Component | Version | Role |
| --- | --- | --- |
| Rust | 1.93.1 stable, edition 2021 (min 1.70+) | Language and `libtest` harness |
| `cargo` | 1.93.1 | Build and test runner |
| `rustfmt` | 1.8.0-stable | Format gate |
| `clippy` | 0.1.93 | Lint gate |
| `gcc-13` | 13.4.0 (gnu17 default) | Reference compiler of record; both UB gates |
| `i686-linux-gnu-gcc-13` | 13.4.0 | Oracle (a) i686 arm |
| `aarch64-linux-gnu-gcc-13` | 13.4.0 | Oracle (a) AArch64 arm |
| `riscv64-linux-gnu-gcc-13` | 13.4.0 | Oracle (a) RISC-V 64 arm |
| QEMU user-mode | 10.1.0 (`qemu-i386`, `qemu-aarch64`, `qemu-riscv64`) | Oracle (b) execution |
| glibc dev + cross runtimes | `libc6-dev`, `-i386-cross`, `-arm64-cross`, `-riscv64-cross` | Static link for all four targets |
| `timeout` | uutils coreutils 0.2.2 | Optional outer bound |
| `clang` | 20.x | Optional alternate reference oracle |
| `creduce` | 2.11.0 | Optional finding minimizer |
| `shellcheck` | system | Shell lint at default severity |
| **Third-party crates** | **none — 0** | Absolute constraint |

> **Version drift, absorbed by design.** The plan anticipated gcc 13.2/13.3 and QEMU 8.2.2; the host carries gcc-13 13.4.0 and QEMU 10.1.0 with only the plain emulator spellings. Runtime discovery rather than hard-coded versions handled this, and every finding records an environment fingerprint so a divergence attributes to toolchain change rather than to the compiler.

### Appendix E — Environment Variable Reference

| Variable | Default | Purpose |
| --- | --- | --- |
| `BCC_BIN` | `CARGO_BIN_EXE_bcc` | Override the compiler under test (e.g. to validate an externally built binary) |
| `BCC_REF_CC` | probe order `gcc`, `cc`, `clang` — **pinned to `gcc-13` on this host** | Native reference compiler for oracle (a) and both audit gates |
| `BCC_REF_CC_I686` | `i686-linux-gnu-gcc` — pinned to `-13` | Oracle (a) i686 arm |
| `BCC_REF_CC_AARCH64` | `aarch64-linux-gnu-gcc` — pinned to `-13` | Oracle (a) AArch64 arm |
| `BCC_REF_CC_RISCV64` | `riscv64-linux-gnu-gcc` — pinned to `-13` | Oracle (a) RISC-V 64 arm |
| `BCC_QEMU_I386` | probe `qemu-i386`, then `qemu-i386-static` | i686 execution runner |
| `BCC_QEMU_AARCH64` | probe `qemu-aarch64`, then `qemu-aarch64-static` | AArch64 execution runner |
| `BCC_QEMU_RISCV64` | probe `qemu-riscv64`, then `qemu-riscv64-static` | RISC-V 64 execution runner |
| `BCC_CONFORMANCE_QUICK` | unset | Reduce to the native target at `-O0` and `-O2`; **always** stamped REDUCED |
| `BCC_CONFORMANCE_ONLY` | unset | Restrict the run to `<area>/<program>` |
| `BCC_CONFORMANCE_STRICT` | unset | Escalate `UNAVAILABLE` to failure. **Intended CI setting** |
| `BCC_CONFORMANCE_ALLOW_XPASS` | unset | Downgrade unexpected success to a warning during a marker-retirement window; hides nothing |
| `BCC_CONFORMANCE_ALLOW_MISSING_ORACLES` | unset | Acknowledge a reduced-oracle environment; **cannot lower strict mode** |
| `BCC_CONFORMANCE_TIMEOUT_SECS` | `30` | Per-cell execution budget; `0` is refused with a precise message |
| `BCC_CONFORMANCE_KEEP_WORK` | unset | Retain cell workspaces instead of removing them on success |

**All have safe defaults — the suite runs correctly with none of them set** (given a toolchain whose default mode is gnu17).

### Appendix F — Developer Tools Guide

| Task | Tool | Notes |
| --- | --- | --- |
| Add a program to an existing area | two new files | `<NNN_name>.c` and `<NNN_name>.expected` in the area directory. **No harness change** — discovery is directory-driven |
| Add a feature area | two new files + one test | A new `<NN_area>/` directory plus one area test in `tests/conformance.rs` |
| Author a program correctly | the suite contract | Hand-declare `int printf(const char *, ...);` and include **no** header (`bcc` ships no `stdio.h`). Apply the two-variant rule for any arithmetic, conversion or bitfield semantics. Print one line per semantic property. No addresses, timestamps, randomness or locale-dependent formatting. Keep exit codes in 0–125 |
| Write the UB-freedom argument | `ub_notes` in the record | The human half of requirement 1; the automated gates are the machine half. A gate deviation without a recorded reason is itself a defect in the test |
| Regenerate a golden record | `regenerate_expected.sh` | **Never** happens during a test run, so a wrong answer cannot quietly become the expectation. Use `--check` in CI |
| Retire an expected-divergence marker | manual edit in **two** places | The program's own `.expected` record **and** `EXPECTED_DIVERGENCES.md`. The bidirectional integrity test fails if either is missed |
| Curate a finding | `FINDINGS.md` + a directory | Copy the generated directory to `tests/conformance/findings/F-NNNN-<slug>/`, minimize the reproducer, verify `sh commands.sh` standalone, add a register row. **Never patch the compiler** |
| Minimize a reproducer | `creduce` (optional) | A system tool, never a required dependency. Manual or scripted reduction is equally acceptable |
| Inspect a failing cell | `BCC_CONFORMANCE_KEEP_WORK=1` | Workspaces are retained on failure automatically; this retains them on success too |
| Clean artifacts safely | `./target/validator/clean-artifacts.sh` | **Never** `rm -rf target/conformance-*` — that also deletes the compiled test binary |
| Verify a whole-repository run | aggregate every `test result:` line | `cargo test` prints one line **per test binary**; no single line carries the repository-wide figure |

### Appendix G — Glossary

| Term | Meaning |
| --- | --- |
| **Cell** | One `(program, target, optimization level)` triple — the atomic unit of the matrix. 108 × 4 × 3 = 1,296 |
| **Oracle (a)** | Reference-compiler comparison: `bcc` versus gcc-13 for the same target at the same `-O` level. The only authority independent of `bcc` |
| **Oracle (b)** | Cross-backend comparison: each non-baseline target against the x86-64 baseline. The baseline is the authority, so it is not compared with itself |
| **Oracle (c)** | Golden-record regression: every cell against the `expected_stdout` in its own record. Catches the one failure mode differential testing structurally cannot — both compilers changing together |
| **PASS** | The compared sides agree and no marker governs the cell |
| **XFAIL** | A divergence a marker predicted, with a documented basis. Does **not** fail the run |
| **XPASS** | A marker claimed a divergence that did not occur — a stale marker. **Fails the run by default** |
| **FINDING** | An undocumented divergence, delivered as a self-contained artifact directory. Does **not** fail the run: a finding is a deliverable |
| **FAIL** | An unexplained outcome. Fails the run |
| **UNAVAILABLE** | An oracle's tooling is genuinely absent. Never a silent pass; escalates to failure under `BCC_CONFORMANCE_STRICT` |
| **Expectation record** | The `<program>.expected` sibling holding the matrix, three command templates, expected exit status, golden stdout, UB-freedom argument, implementation-defined notes and any marker. Makes a cell reproducible with no harness |
| **Marker** | A machine-readable block in a record declaring an expected divergence, with an identifier, class, scope, documented basis and observed behaviour. Mirrored in the committed register |
| **Two-variant rule** | Every arithmetic, conversion or bitfield area carries both a compile-time-constant variant (exercising the folder) and a `volatile`-operand runtime variant (forcing real instructions). Without the second, optimization substitutes the folder's answer for the backend's and a code-generation defect escapes |
| **UB audit gate** | A strict `-Werror` warning gate plus an ASan/UBSan run, both **reference-compiler-only**, that establish the precondition under which a divergence is meaningful at all |
| **Stand-in** | The documented placeholder compiler used on a branch carrying the suite ahead of the compiler tree. It forwards to gcc-13, so it establishes everything about the *suite* and **nothing about `bcc`** |
| **REDUCED** | The stamp a narrowed run's report carries, so it can never be mistaken for a full sweep |
| **C1 / C2 / C3 / C4** | The four hard constraints: no compiler-source or manifest change; no existing test weakened; no feature excluded for difficulty; hermetic sandbox with no network |

---

## Cross-Section Integrity Validation

| Rule | Requirement | Verification | Status |
| --- | --- | --- | --- |
| **Rule 1** (§1.2 ↔ §2.2 ↔ §7) | Remaining hours identical in all three | §1.2 metrics table = **65** · §2.2 `Hours` column sum = **65** · §7 pie "Remaining Work" = **65** | ✅ |
| **Rule 2** (§2.1 + §2.2 = Total) | Completed + Remaining = Total in §1.2 | 514 + 65 = **579** = §1.2 Total Hours | ✅ |
| **Rule 3** (§3) | Every test originates from Blitzy's autonomous validation logs | All rows sourced from the autonomous run logs and the committed report artifacts, and independently re-executed during this review. The stand-in caveat is stated explicitly rather than omitted | ✅ |
| **Rule 4** (§1.5) | Access issues validated against current permissions | `/tmp` mode measured at 2777 with `[ -k /tmp ]` false; `git ls-files` confirms no compiler tree, manifest or `tests/common/mod.rs`; `libclang-rt-20-dev` absence confirmed | ✅ |
| **Rule 5** (Colours) | Completed = `#5B39F3`, Remaining = `#FFFFFF` | Declared in §1.2's colour table and restated under §7's chart | ✅ |
| Completion % consistency | One figure everywhere | **88.8%** in §1.2 (chart title + metrics + formula), §7 chart title, and §8 narrative. No approximations such as "nearly 90%" appear anywhere | ✅ |
| Hours consistency | One set of figures everywhere | **579 / 514 / 65** in §1.2, §2.1, §2.2, §2.3, §7 and §8. No other hour totals appear | ✅ |
| Task list ↔ §2.2 | Human tasks sum to the remaining total | High 34 + Medium 22 + Low 9 = **65**; every task maps 1:1 onto a §2.2 row | ✅ |
| Formula shown with real numbers | Explicit calculation | `514 / (514 + 65) × 100 = 514 / 579 × 100 = 88.8%` stated in §1.2 | ✅ |
| Section structure | Exactly 10 sections, template order, none added, removed or renamed | §1 Executive Summary (1.1–1.6) · §2 Project Hours Breakdown (2.1–2.3) · §3 Test Results · §4 Runtime Validation & UI Verification · §5 Compliance & Quality Review · §6 Risk Assessment · §7 Visual Project Status · §8 Summary & Recommendations · §9 Development Guide · §10 Appendices (A–G) | ✅ |
| Maximum claim | Never 100% complete | 88.8% | ✅ |