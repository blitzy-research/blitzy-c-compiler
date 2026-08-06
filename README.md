# blitzy-c-compiler
One prompt attempt at building a Rust C-Compiler 

## Differential conformance suite

An oracle-based differential conformance suite validates the compiler's observable program
behaviour against three complementary comparison oracles — one external authority, one internal
cross-check, and its own recorded golden output:

- **Reference-compiler oracle** — each program is compiled with `bcc` and with a reference C
  compiler for the same target at the same optimization level, both binaries are executed, and
  stdout bytes plus exit status are compared. This is the one oracle whose authority is independent
  of `bcc`.
- **Cross-backend oracle** — the same program is compiled for x86-64, i686, AArch64 and
  RISC-V 64 and executed natively or under QEMU user-mode emulation, with each of the **three
  non-baseline targets** compared against the x86-64 baseline at the same optimization level. The
  baseline is the authority, so it is not compared with itself.
- **Golden-record oracle** — every cell is checked against the `expected_stdout` bytes and the
  `expect_exit` status recorded in the program's own co-located expectation record.

**Prerequisite:** every command below needs the compiler's own Cargo package in the checkout — a
root `Cargo.toml` declaring the `bcc` binary target and the `src/**` tree it builds from. Cargo
discovers `tests/conformance.rs` automatically, so the suite needs no manifest *change*; it still
needs a manifest to be discovered *from*, and it must never add one. On a checkout that carries the
suite ahead of the compiler tree the Cargo gates therefore do not run and there is no `bcc` to test —
a property of the checkout rather than a defect in the suite. That shape can still verify everything
which does not depend on packaging:

```bash
mkdir -p target/conformance-typecheck
rustfmt --edition 2021 --check tests/conformance.rs tests/conformance_harness/*.rs
CARGO_MANIFEST_DIR="$(pwd)" rustc --edition 2021 --test --emit=metadata \
  --out-dir target/conformance-typecheck tests/conformance.rs
CARGO_MANIFEST_DIR="$(pwd)" rustdoc --edition 2021 --crate-type lib \
  --crate-name conformance --document-private-items \
  -o target/conformance-typecheck/doc tests/conformance.rs
```

Run from the repository root. The first line is what makes the sequence work on a checkout that has
never been built: `target/` does not exist there, and creating it explicitly keeps the commands
independent of whether a given toolchain in the supported range creates an absent `--out-dir` for you.
The last line is the one no other gate can perform: `broken_intra_doc_links` is a rustdoc lint, and
`cargo doc` does not document integration-test targets, so a doc comment naming an item that does not
exist is invisible to every other check
([the doc-link gate](tests/conformance/README.md#the-doc-link-gate)).

Full details, and what each checkout shape can and cannot establish:
[the Cargo integration precondition](tests/conformance/README.md#the-cargo-integration-precondition).

With the package present, run the suite with:

```bash
cargo test --test conformance
```

Useful variants:

```bash
cargo test --test conformance -- --nocapture                    # stream the full verdict table
cargo test --test conformance area_04_bitfields -- --nocapture  # one feature area
cargo test --test conformance infra_ -- --nocapture             # the infrastructure tests only
BCC_CONFORMANCE_QUICK=1 cargo test --test conformance           # reduced matrix, always reported as reduced
```

Every outcome is one of PASS, XFAIL (an expected divergence with a documented basis), XPASS
(a marker whose divergence has disappeared — a failure), FINDING (an undocumented divergence,
delivered with a **verbatim** reproducer, its recorded minimization status, the captured outputs of
each compiler and each backend, an environment fingerprint and exact reproduction commands — a run
files a verbatim copy of the corpus program, because reduction is a supervised curation step
performed before a finding is promoted to the curated set rather than something a run performs),
FAIL, or UNAVAILABLE (an oracle's tooling is genuinely absent). A run summary is written to
`conformance-report/summary.md` beneath the build directory (`target/` unless `CARGO_TARGET_DIR` redirects it),
and every finding also gets a review copy of all seven of its artifacts at
`conformance-report/findings/<finding-id>/` — rendered to the same grade as the rest of the report — so a
finding survives in whatever archive carries the report, given that a FINDING does not fail the run.

- Suite contract, verdict taxonomy, environment variables and how to reproduce any cell by
  hand: [`tests/conformance/README.md`](tests/conformance/README.md)
- Methodology, build matrix and deliverable summary format:
  [`docs/testing/differential-conformance.md`](docs/testing/differential-conformance.md)
- Expected divergences:
  [`tests/conformance/EXPECTED_DIVERGENCES.md`](tests/conformance/EXPECTED_DIVERGENCES.md)
  · Findings: [`tests/conformance/FINDINGS.md`](tests/conformance/FINDINGS.md)
