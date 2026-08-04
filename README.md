# blitzy-c-compiler
One prompt attempt at building a Rust C-Compiler 

## Differential conformance suite

An oracle-based differential conformance suite validates the compiler's observable program
behaviour against two independent authorities and against its own recorded golden output:

- **Reference-compiler oracle** — each program is compiled with `bcc` and with a reference C
  compiler, both binaries are executed, and stdout bytes plus exit status are compared.
- **Cross-backend oracle** — the same program is compiled for x86-64, i686, AArch64 and
  RISC-V 64 and executed natively or under QEMU user-mode emulation, with every target
  compared against the x86-64 baseline at the same optimization level.
- **Golden-record oracle** — every cell is checked against the `expected_stdout` bytes and the
  `expect_exit` status recorded in the program's own co-located expectation record.

Run it with:

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
delivered as a minimized reproducer), FAIL, or UNAVAILABLE (an oracle's tooling is genuinely
absent). A run summary is written to `target/conformance-report/summary.md`.

- Suite contract, verdict taxonomy, environment variables and how to reproduce any cell by
  hand: [`tests/conformance/README.md`](tests/conformance/README.md)
- Methodology, build matrix and deliverable summary format:
  [`docs/testing/differential-conformance.md`](docs/testing/differential-conformance.md)
- Expected divergences:
  [`tests/conformance/EXPECTED_DIVERGENCES.md`](tests/conformance/EXPECTED_DIVERGENCES.md)
  · Findings: [`tests/conformance/FINDINGS.md`](tests/conformance/FINDINGS.md)
