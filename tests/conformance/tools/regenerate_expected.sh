#!/bin/sh
# =============================================================================
# tests/conformance/tools/regenerate_expected.sh
#
# MAINTENANCE ONLY -- this script is NEVER invoked by `cargo test`.
# Run it deliberately, by hand, from anywhere:
#
#     sh tests/conformance/tools/regenerate_expected.sh [OPTIONS]
#
# See `tests/conformance/README.md`, sections "Read-only, and the golden-record
# rule", "Grammar" and "Reproducing a cell by hand", for the contract this
# script implements.
# =============================================================================
#
# WHAT THIS SCRIPT IS FOR
# -----------------------
# It regenerates the `expected_stdout` block of a differential-conformance
# expectation record from the REFERENCE compiler, and nothing else in the
# record.
#
# WHY IT IS SEPARATE FROM THE TEST RUN
# ------------------------------------
# `expected_stdout` is oracle (c), the golden-record oracle. It exists to catch
# the one failure mode that pure differential testing structurally cannot
# detect: BOTH compilers changing behaviour in the same direction at the same
# time, which oracle (a) still reports as agreement. If a test run could rewrite
# its own expectations, oracle (c) would be silently disabled -- a wrong answer
# would quietly become the new expectation. Keeping regeneration a deliberate,
# manual, git-recorded human act is what preserves its value, so:
#
#   * nothing in `tests/conformance.rs` or `tests/conformance_harness/` refers
#     to this script, and nothing there may ever shell out to it;
#   * `tests/conformance_harness/manifest.rs` deliberately provides NO writer
#     for the record format. This script is the only writer that exists.
#
# WHY THE REFERENCE COMPILER AND NEVER THE COMPILER UNDER TEST
# -----------------------------------------------------------
# Seeding a golden record from `bcc` would make oracle (c) certify `bcc` against
# itself and would launder a miscompilation into the expectation. This script
# therefore never invokes `bcc` and never even reads `BCC_BIN`. A record whose
# marker says the compiler under test cannot produce these bytes still keeps
# them: that gap is precisely the divergence the marker describes.
#
# WHAT IT WRITES, AND WHAT IT NEVER TOUCHES
# -----------------------------------------
# Writes:  the `expected_stdout` block of the selected `*.expected` records; one
#          short-lived, unpredictably named `mktemp -d` staging DIRECTORY beside
#          each record being rewritten, holding that record's staging copy; and a
#          private `mktemp -d` working area, with a fresh per-cell subdirectory,
#          for compiling and running cells.
# Never:   `findings/`, `support/`, `tools/`, `FINDINGS.md`,
#          `EXPECTED_DIVERGENCES.md`, `README.md`, anything under `target/`,
#          and no key of any record other than `expected_stdout`. Every other
#          byte -- comment banners, command templates, `ub_notes`,
#          `impl_defined_notes` with load-bearing indentation, and every
#          `expected_divergence.*` key -- is copied through verbatim.
#
# SAFETY PROPERTIES
# -----------------
#   * Every declared cell (targets x optimization levels) is compiled, run and
#     captured, and ALL captures must be byte-identical before anything is
#     written. A single golden record is asserted by oracle (c) against every
#     cell of the matrix, so writing one without proving agreement would bake a
#     target-specific answer into a cross-target expectation.
#   * Every reference driver is ATTESTED before it compiles anything: the program
#     at the end of its wrapper chain is established and shown not to be the
#     compiler under test, it must answer `-dumpmachine` with the target it was
#     chosen for, and its default language mode must be C11 or C17 with the GNU
#     extensions enabled. A candidate that fails is refused, not replaced, so no
#     golden byte is ever written by a driver whose authority is unproven.
#   * Every record is validated against the same closed key set, field kinds,
#     required fields, toggles, flag rules, template placeholders and size
#     ceilings that `manifest.rs` enforces -- BEFORE any cell is compiled, so a
#     record the harness would refuse is never rewritten and no work is spent on
#     one that could not be read back.
#   * A rewrite is staged inside an unpredictable `mktemp -d` directory created
#     beside the record and installed with a single `mv` within that one
#     directory, so a record is either fully updated or byte-identical to what it
#     was -- never half-written, and never redirectable through a planted path.
#   * A record whose final byte is not a NEWLINE is refused rather than silently
#     terminated: adding that byte would change a byte outside `expected_stdout`.
#   * Every compile and every run is BOUNDED, by `timeout` where it is present
#     and usable and by this script's own self-tested watchdog otherwise, and
#     runs in a fresh per-cell directory with the same sanitized environment the
#     harness installs, so an ambient locale, TZ, HOME or toolchain variable
#     cannot reach a cell and change the bytes that become a golden record.
#   * Running the script twice in a row produces no diff.
#   * No network access, no package installation, no path outside the corpus
#     directory and the private working area. Every selected record must lie at
#     exactly `<area>/<program>.expected` inside the physical corpus root, with
#     no symbolic link in any component.
#
# PORTABILITY, STATED CONCRETELY
# ------------------------------
# The shell is POSIX only and the awk is POSIX only: no bashisms (no arrays, no
# `[[`, no `local`, no `function`, no `+=`, no `${var:0:n}`, no `$'...'`, no
# process substitution, no `echo -e/-n`, no PIPESTATUS, no `set -o pipefail`)
# and no GNU-awk extensions (no `gensub`, no `IGNORECASE`, no `asort`, no
# `length(array)`, no `\<`, no `systime`).
#
# "Portable POSIX shell" is nonetheless a claim about SYNTAX, not about the
# userland the script actually ran against, and the two are not the same thing:
# a script can be perfectly POSIX and still need a utility POSIX never defined.
# So rather than assert an arbitrary POSIX userland, here is the baseline this
# script is tested on, which a reader can check:
#
#     Linux x86_64
#     /bin/sh -> dash 0.5.12          awk -> mawk 1.3.4 20250131
#     uutils coreutils 0.2.2          mktemp timeout tr tail sort mkdir cat
#                                     basename dirname env id ls
#     GNU coreutils 9.5               cp mv rm
#     GNU diffutils 3.10              cmp
#     GNU grep 3.11                   grep
#
# That the userland is MIXED is deliberate and worth stating: the same script is
# exercised against both the GNU and the uutils spelling of several of these
# utilities, so no single implementation's extensions can quietly become load
# bearing.
#
# Every utility invoked, with every option actually used:
#
#     awk -v                  grep -q / -q -x -F --      tr -d / tr -c SET1 SET2
#     cat                     cmp -s -- / cmp --         tail -c 1
#     cp -p --                mv --                      rm -f -- / rm -rf --
#     mkdir -p -- / mkdir --  sort -- / sort -u          printf
#     mktemp -d / -d --       sleep N                    command -v / -v --
#     cd -P / pwd -P          test A -ef B                wait
#     kill -0 / -TERM / -KILL
#     du -sk / du -k --       rmdir --                   setsid
#     env -i NAME=VALUE ...   id -u                      ls -ldn --
#     ulimit -f BLOCKS
#
# `--` precedes every file operand. POSIX requires end-of-options handling from
# utilities that follow the getopt convention, so this is portable -- and it is
# what makes a record whose name begins with `-` harmless rather than an option.
#
# `du`, `rmdir`, `mkdir`, `tr -c`, `sort -u`, `env -i`, `id -u`, `ls -ldn` and the
# `ulimit -f` shell built-in are all POSIX, so the exception count below is still
# exactly three. Four of them carry a duty worth naming, because each is the whole
# mechanism behind a guarantee stated elsewhere in this file:
#
#   * `env -i` REPLACES a child's environment rather than filtering it, which is
#     what makes "a variable reaches a child only by being named here" true.
#   * `id -u` and `ls -ldn` are how a directory's trustworthiness is decided
#     before it is allowed onto a child's PATH: numeric owner and mode, read
#     without `stat`, which is not POSIX and is not in this tool set.
#   * `ulimit -f` is the hard per-file ceiling on every child. It is a limit the
#     kernel enforces on the process rather than a size this script checks
#     afterwards, so a single runaway write cannot fill the working area between
#     two of the supervisor's ticks.
#
# `sort -u` deserves one line of justification because a pipeline could sort and
# then uniquify instead: `-u` is POSIX, and the alternative would add `uniq` to
# this inventory for no gain.
# `mkdir` carries a second duty beyond creating directories: it is the ATOMIC
# primitive the corpus regeneration lock is built from, because one operation
# either creates the directory or fails, with no window between a test and a
# create for a second sweep to slip through.
#
# Every utility NAME in that list is POSIX WITH EXACTLY THREE EXCEPTIONS, and
# none is left to chance. The Linux interfaces and `test -ef` primary are
# accounted for separately below:
#
#   * `mktemp` is not a POSIX utility at all, and it IS a hard requirement. The
#     private working area and each record's staging directory are both created
#     with `mktemp -d`, because an unpredictable directory created atomically is
#     what keeps a rewrite out of reach of a path planted beside the record.
#     There is deliberately no fallback: a predictable name generated in shell
#     would be the very weakness `mktemp` exists to remove. Its absence is
#     reported as an environment error (exit 3), never worked around.
#   * `timeout` is not a POSIX utility either, and it is NOT required. When it is
#     missing -- or present but proven at startup not to bound anything -- every
#     compile and every run is bounded by this script's own `sleep`/`kill`
#     watchdog instead. What is never optional is the BOUND: nothing this script
#     starts is executed unbounded, and a host that can support neither
#     mechanism is refused rather than run without one.
#   * `setsid` is not a POSIX utility either, and it is NOT required. With it, a
#     cell is started as its own session leader, so terminating the bound or a
#     working-area breach can signal the whole process TREE -- which matters
#     because a compiler driver forks, and signalling only the direct child can
#     leave the real compiler running. Without it the exact child is signalled
#     instead, and that guarantee is genuinely weaker rather than equivalent, so
#     it is stated here rather than glossed over. Its absence never fails a run.
#
# Two Linux interfaces are now deliberately load-bearing security mechanisms,
# not accidental portability leaks. `/proc/<pid>/stat` field 22 authenticates a
# process before every signal, and `/proc/$$/fd/<n>/.` keeps the corpus, work,
# record and staging directories bound to open descriptors across ancestor
# renames. The `test A -ef B` primary verifies that a visible cleanup/install
# name still designates the held object. The script already targets Linux in the
# tested baseline above; on a host without these interfaces it refuses before
# compiling rather than silently falling back to numeric PIDs and repeated names.
#
# Deliberately NOT dependencies, and never invoked at all: `find` and `sed`
# (records are enumerated with shell globbing and inspected with awk, so no
# findutils traversal behaviour is relied on and no `-print0`/`-regex` question
# arises), `binutils` (readelf, objdump, nm), `creduce`, `python`, `perl`, `jq`,
# and the compiler under test. `clang` is only the last of three documented
# defaults in the native-driver search ORDER: the first default that EXISTS
# becomes the single attested candidate, so `clang` is reached only on a host
# with neither `gcc` nor `cc`, and is never required.
# =============================================================================

set -eu

# Byte-exact comparison and deterministic enumeration both depend on the C
# locale: it is what makes `sort` order stable, `grep` byte-oriented and
# `awk`'s `length()` count bytes rather than characters.
LC_ALL=C
export LC_ALL

CF_PROG='regenerate_expected.sh'

# --- Exit codes ---------------------------------------------------------------
# Distinct codes so a caller can tell an unusable environment from a defective
# record from a defective capture without parsing the message text.
CF_EXIT_USAGE=2       # bad command line
CF_EXIT_ENVIRONMENT=3 # a required tool is absent or unusable
CF_EXIT_RECORD=4      # the record or its program is defective
CF_EXIT_CAPTURE=5     # a captured stream is unusable, or cells disagree
CF_EXIT_PENDING=6     # --check: at least one record would change

# --- Limits mirrored from tests/conformance_harness/manifest.rs ---------------
# Writing a record the parser would refuse is a defect, so the same ceilings are
# enforced here on the way in. Keep these in step with `manifest.rs`.
CF_LINE_BYTES_MAX=8192      # RECORD_LINE_BYTES_MAX
CF_HEREDOC_LINES_MAX=4096   # HEREDOC_LINES_MAX
CF_FIELD_BYTES_MAX=65536    # FIELD_BYTES_MAX
CF_RECORD_BYTES_MAX=262144  # RECORD_BYTES_MAX
CF_EXPECT_EXIT_MAX=125      # expect_exit is constrained to 0..125

# --- Resource bounds this script imposes on itself ----------------------------
# None of these is configurable away. A sweep compiles and runs every declared
# cell of every selected record, so each of the three resources it consumes --
# time, records, and space in the private working area -- is given a finite
# ceiling. A bound that can be raised without limit is not a bound.

# Upper ceiling on the per-cell budget, whatever BCC_CONFORMANCE_TIMEOUT_SECS
# asks for. The harness's own default is 30 seconds and a corpus cell takes
# milliseconds; ten minutes is far beyond any honest value and still finite,
# which is the property that matters -- a budget with no ceiling is a way of
# asking for no bound at all while appearing to configure one.
CF_TIMEOUT_SECS_MAX=600

# Ceiling on how many records one invocation will process. The corpus holds 108;
# five hundred and twelve allows for growth by a factor of four and refuses a
# corpus that has become something else -- a directory of generated files, or a
# path standing where the corpus should be.
CF_RECORDS_MAX=512

# Live ceilings on the private working area, measured while a cell is running.
# A cell writes a handful of small files; a runaway one that fills the disk would
# otherwise take the host down with it, and the per-cell time bound does not help
# because a process can write a great deal in a second.
CF_WORK_KB_MAX=65536
CF_WORK_ENTRIES_MAX=256

# How deep the entry walk descends before the depth itself is the breach. A cell
# workspace holds a handful of files one level down; eight levels is far beyond any
# honest shape and finite, which is what a bound has to be. Exceeding it is
# reported as a breach rather than silently truncating the count -- a tree this
# script cannot finish measuring is one it must not certify as within the ceiling.
CF_WALK_DEPTH_MAX=8

# The hard per-file ceiling installed on every child with `ulimit -f`, in 512-byte
# blocks: 131072 blocks is 64 MiB, the same figure as the whole-area ceiling above.
#
# It exists because the live supervisor polls once a second and a process can write
# far more than the area ceiling between two polls -- a redirection to a file is as
# fast as the disk. `ulimit` is a kernel limit rather than an observation, so it
# stops the write itself instead of noticing it afterwards, and it is the only bound
# here that a fast writer cannot outrun. The two work together: this caps any single
# file, and the supervisor catches many files that are each under the cap.
CF_FILE_BLOCKS_MAX=131072

# The status at and above which a POSIX shell can no longer attribute an exit.
#
# `wait` reports death by signal N as 128+N, and a program that RETURNS 128+N as the
# same number, so any status at or above this base is ambiguous rather than a signal.
# tests/conformance_harness/findings.rs reaches the same conclusion and renders such
# a status as `ambiguous <n>`; the wording here matches it deliberately, because two
# artifacts describing the same status differently is how a reader learns to distrust
# both.
CF_SIGNAL_STATUS_BASE=128

# --- The warning gate, mirrored from tests/conformance_harness/mod.rs -----------
# A record's ub_audit_flags value is not merely "non-empty": the parser accepts
# exactly two reductions of the default gate and nothing else, so a value it would
# refuse must be refused here too -- before a compile is spent and long before a
# rewrite. Keep these four in step with UB_AUDIT_GATE_DEFAULT,
# UB_GATE_WITHOUT_PEDANTIC, UB_GATE_WITHOUT_CONVERSION and UB_AUDIT_GATE_MANDATORY.
CF_UB_GATE_DEFAULT='-Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror'
CF_UB_GATE_WITHOUT_PEDANTIC='-Wall -Wextra -Wconversion -Wsign-conversion -Wshadow -Werror'
CF_UB_GATE_WITHOUT_CONVERSION='-Wall -Wextra -pedantic -Wshadow -Werror'
CF_UB_GATE_MANDATORY='-Werror'
# The one area whose gate may drop -pedantic, where the subject under test is by
# definition non-standard and that diagnostic exists precisely to reject it.
CF_UB_GATE_EXTENSION_AREA='08_gcc_extensions'

# --- The marker vocabulary, mirrored from tests/conformance_harness/mod.rs ------
# The six observable divergence shapes plus the one class that names no shape at
# all. Keep in step with DivergenceClass::label and COMPARISON_EXCLUDED_LABEL.
CF_MARKER_CLASSES='compile_failure link_failure run_crash exit_code_mismatch
stdout_mismatch timeout comparison_excluded'
CF_MARKER_CLASS_EXCLUDED='comparison_excluded'

# The oracle, target and optimization-level spellings a marker scope may name.
# Mirrors Oracle::parse, Target::parse and OptLevel::parse.
CF_SCOPE_ORACLE_SPELLINGS='a b c oracle_a oracle_b oracle_c'
CF_SCOPE_TARGET_SPELLINGS='x86_64 i686 aarch64 riscv64
x86_64-linux-gnu i686-linux-gnu aarch64-linux-gnu riscv64-linux-gnu'
CF_SCOPE_OPT_SPELLINGS='-O0 -O1 -O2 O0 O1 O2'

# --- Directories excluded from record enumeration ----------------------------
# The three corpus companions that are siblings of the area directories and are
# never walked for records. `findings/F-NNNN-*/reproducer.expected` IS a valid
# record, but it is a curated, human-promoted deliverable: regenerating it would
# rewrite a captured artifact. `support/` holds the flag probe's only fixture,
# and `tools/` holds this script.
#
# This predicate is the single place those three names are written. Both
# consumers call it rather than repeating them: cf_list_corpus_records, which
# walks the corpus root, and cf_validate_area_name, which vets an --area or
# --program the maintainer typed. A record that arrives by any route is separately
# held to the `NN_<name>` area grammar, which none of these three names can match,
# so there is no third list to drift out of step with this one.
cf_is_excluded_dir() {
	case $1 in
	findings | support | tools) return 0 ;;
	esac
	return 1
}

# --- Mutable state -----------------------------------------------------------
CF_WORK=''         # handle-relative private working area, opened on fd 7
CF_WORK_NAME=''    # physical name mktemp returned, used only for verified cleanup
CF_WORK_OPEN=0
# Whatever staging state is in flight, removed on any exit. Under --check that is a
# candidate FILE inside the private working area; when writing it is the staging
# DIRECTORY beside the record. One variable and one releaser for both, so no exit
# path has to know which kind it is looking at.
CF_STAGING=''
CF_STAGING_ENTRY=''
CF_STAGING_HANDLE=''
CF_STAGING_OPEN=0
# The current record directory is opened on fd 8. Every record/source read and
# every staging/install operation goes through this handle, so a renamed ancestor
# cannot redirect a later path resolution.
CF_RECORD_DIR_NAME=''
CF_RECORD_DIR_HANDLE=''
CF_RECORD_DIR_OPEN=0
# The corpus root is opened on fd 6 once its physical location is established.
# Lock acquisition/reclamation/release use this handle so mutable ancestors
# cannot redirect the coordination namespace.
CF_CORPUS_HANDLE=''
CF_CORPUS_OPEN=0
CF_PROCESSED=0
# The corpus regeneration lock, and whether this run owns it. Released on any
# exit path by cf_cleanup, so an interrupted run does not leave a lock behind
# that the next one has to reclaim.
CF_LOCK_DIR=''
CF_LOCK_DIR_NAME=''
CF_LOCK_HELD=0
# This run's ownership token, written into the lock's owner file when the lock is
# taken. Cleanup removes the lock only while the token on disk is still this one:
# a lock another run has reclaimed is no longer ours to remove, and removing it
# would hand the corpus to two writers at once -- the exact outcome the lock
# exists to prevent.
CF_LOCK_TOKEN=''
# Why the live working-area ceiling was breached, set by cf_quota_breach and
# empty whenever it was not.
CF_QUOTA_REASON=''
CF_CHANGED=0

# The background wrapper that owns one bounded child and its quota supervisor.
# Signal cleanup reaches this direct child, whose own traps terminate and reap the
# cell group before the parent releases staging, the corpus lock or the work root.
CF_ACTIVE_WRAPPER=''
CF_ACTIVE_WRAPPER_START=''
CF_ACTIVE_READY=1
CF_PENDING_SIGNAL=''
CF_SIGNAL_REASON=''
CF_SIGNAL_BREACH=0

# The search path this script and every child it starts use, computed from the
# inherited PATH by cf_sanitize_path and never read from the environment again.
CF_TRUSTED_PATH=''

# Resolved-tool cache. An empty value means "not resolved yet"; resolution is
# lazy so a record that restricts its target list never requires a toolchain it
# does not use.
CF_CC_X86_64=''
CF_CC_I686=''
CF_CC_AARCH64=''
CF_CC_RISCV64=''
CF_RUN_I686=''
CF_RUN_AARCH64=''
CF_RUN_RISCV64=''

# Set by cf_resolve_cell_tools for the target it was asked about.
CF_CELL_CC=''
CF_CELL_RUNNER=''
# Set by cf_choose.
CF_CHOSEN=''

# A literal carriage return, for the capture validation below. POSIX command
# substitution strips trailing NEWLINES only, so a lone CR survives intact.
CF_CR=$(printf '\r')

# =============================================================================
# Diagnostics. Every message goes to stderr so that stdout stays clean.
# =============================================================================

cf_note() {
	printf '%s: %s\n' "$CF_PROG" "$*" >&2
}

cf_error() {
	printf '%s: error: %s\n' "$CF_PROG" "$*" >&2
}

cf_detail() {
	printf '    %s\n' "$*" >&2
}

# Echo at most $2 lines of file $1 to stderr, indented, for a diagnostic. The file
# arrives by redirection rather than as an operand, so a name beginning with a
# hyphen can never be read as an option.
#
# Two details that a plainer version gets wrong, and both cost the reader the one
# thing they came for:
#
#   * AN UNTERMINATED FINAL LINE IS STILL A LINE. `read` returns non-zero at
#     end-of-file, but when it stopped because the input ran out mid-line it has
#     already assigned what it read. A loop that trusts the status alone therefore
#     DROPS that text -- and a compiler or an emulator whose last line is
#     unterminated is exactly the case where that text is the whole diagnosis. So
#     the loop distinguishes "nothing left" from "something left, unterminated".
#   * NO PIPELINE. Reading through `sed` would put the producer's exit status
#     behind the last command in the pipeline, hiding a read failure on the very
#     file being quoted. The line budget is counted here instead, and the file is
#     read directly.
cf_detail_file() {
	cf_detail_file_max=$2
	cf_detail_file_shown=0
	cf_detail_file_line=''
	while [ "$cf_detail_file_shown" -lt "$cf_detail_file_max" ]; do
		if IFS= read -r cf_detail_file_line; then
			cf_detail_file_more=1
		elif [ -n "$cf_detail_file_line" ]; then
			# End of file reached mid-line: the partial line is real output.
			cf_detail_file_more=0
		else
			break
		fi
		cf_detail "| $cf_detail_file_line"
		cf_detail_file_shown=$((cf_detail_file_shown + 1))
		if [ "$cf_detail_file_more" -eq 0 ]; then
			break
		fi
	done < "$1"
}

# Print Linux's process start-time identity token (`/proc/<pid>/stat` field 22).
# Field 2 is parenthesized and may itself contain spaces or closing parentheses,
# so remove through the LAST `) ` and select field 20 of the remaining tail.
cf_process_start_token() {
	cf_pst_pid=$1
	case $cf_pst_pid in
	'' | *[!0-9]*) return 1 ;;
	esac
	[ -r "/proc/$cf_pst_pid/stat" ] || return 1
	cf_pst_stat=$(cat -- "/proc/$cf_pst_pid/stat" 2> /dev/null) || return 1
	cf_pst_tail=${cf_pst_stat##*) }
	[ "$cf_pst_tail" != "$cf_pst_stat" ] || return 1
	cf_pst_token=$(printf '%s\n' "$cf_pst_tail" |
		awk 'NF >= 20 && $20 ~ /^[0-9]+$/ { print $20; ok=1 } END { if (!ok) exit 1 }') ||
		return 1
	[ -n "$cf_pst_token" ] || return 1
	printf '%s\n' "$cf_pst_token"
}

# Signal one direct process only while its Linux start token still matches.
# `env kill` deliberately selects the external utility: dash's builtin rejects
# the `--` required by the procps-ng utility before a negative group operand.
CF_SIGNAL_REASON=''
cf_signal_pid() {
	# $1 = signal, $2 = pid, $3 = start token
	CF_SIGNAL_REASON=''
	CF_SIGNAL_BREACH=0
	cf_spid_now=$(cf_process_start_token "$2" 2> /dev/null) || cf_spid_now=''
	if [ -z "$3" ]; then
		CF_SIGNAL_REASON="process $2 had no start-time identity captured at launch"
		CF_SIGNAL_BREACH=1
		return 1
	fi
	if [ -z "$cf_spid_now" ]; then
		CF_SIGNAL_REASON="process $2 no longer has a readable start-time identity"
		return 1
	fi
	if [ "$cf_spid_now" != "$3" ]; then
		CF_SIGNAL_REASON="process $2 changed start-time identity from $3 to $cf_spid_now"
		CF_SIGNAL_BREACH=1
		return 1
	fi
	command env kill -"$1" -- "$2" 2> /dev/null
}

# Stop and reap the direct wrapper that owns the current cell and its supervisor.
# The wrapper's own TERM trap performs the inner group sweep; waiting for the
# wrapper is therefore the parent-side proof that no child still owns work files.
cf_stop_active_wrapper() {
	if [ -z "$CF_ACTIVE_WRAPPER" ]; then
		return 0
	fi
	if cf_signal_pid TERM "$CF_ACTIVE_WRAPPER" "$CF_ACTIVE_WRAPPER_START"; then
		:
	elif [ "$CF_SIGNAL_BREACH" -eq 1 ]; then
		[ -n "$CF_SIGNAL_REASON" ] && cf_error "refusing to signal active bound wrapper: $CF_SIGNAL_REASON"
	fi
	wait "$CF_ACTIVE_WRAPPER" > /dev/null 2>&1 || true
	CF_ACTIVE_WRAPPER=''
	CF_ACTIVE_WRAPPER_START=''
	CF_ACTIVE_READY=1
}

# Signals landing between `$!` and capture of the wrapper start token are
# remembered, then delivered once that identity is complete. Further lifecycle
# signals are ignored during cleanup so the wait/removal sequence cannot re-enter.
cf_on_signal() {
	cf_os_signal=$1
	if [ "$CF_ACTIVE_READY" -ne 1 ]; then
		[ -n "$CF_PENDING_SIGNAL" ] || CF_PENDING_SIGNAL=$cf_os_signal
		return 0
	fi
	trap '' HUP INT TERM
	cf_cleanup
	trap - "$cf_os_signal"
	kill -"$cf_os_signal" $$ || exit 1
}

# =============================================================================
# Cleanup.
#
# Cleanup sits on EXIT alone. HUP/INT/TERM first reach the direct bound wrapper,
# whose own trap identity-checks, terminates and waits for the cell group and
# quota supervisor. Only after the parent has waited for that wrapper does it
# release staging, the corpus lock and the work root. Deleting those paths first
# would leave a compiler or emulator writing through names this run no longer
# owns.
#
# The parent then restores the signal's default disposition and re-raises it, so
# the caller sees a signal termination rather than a script that resumed after
# cleanup. Signals arriving between `$!` and start-token capture are deferred,
# and further lifecycle signals are masked while the ordered cleanup runs.
# =============================================================================

# Remove whatever staging state is in flight and forget it. `rm -rf` because the
# path is a directory when writing and a file under --check, and safe to call when
# there is none.
cf_release_staging() {
	if [ "$CF_STAGING_OPEN" -eq 1 ]; then
		if [ -n "$CF_STAGING_ENTRY" ] && [ -n "$CF_STAGING_HANDLE" ] &&
			command test "$CF_STAGING_ENTRY" -ef "$CF_STAGING_HANDLE"; then
			rm -rf -- "$CF_STAGING_ENTRY"
		elif [ -n "$CF_STAGING_ENTRY" ] || [ -n "$CF_STAGING_HANDLE" ]; then
			cf_error 'refusing to remove staging: its directory entry no longer matches the pinned handle'
		fi
		exec 9<&-
		CF_STAGING_OPEN=0
	elif [ -n "$CF_STAGING" ]; then
		rm -rf -- "$CF_STAGING"
	fi
	CF_STAGING=''
	CF_STAGING_ENTRY=''
	CF_STAGING_HANDLE=''
}

# Close the durable handle for the record directory after any staging child has
# been retired. Safe to call before the first record and after every return path.
cf_release_record_dir() {
	if [ "$CF_RECORD_DIR_OPEN" -eq 1 ]; then
		exec 8<&-
		CF_RECORD_DIR_OPEN=0
	fi
	CF_RECORD_DIR_NAME=''
	CF_RECORD_DIR_HANDLE=''
}

# Give the corpus regeneration lock back, if this run took it AND it is still this
# run's. Defined beside the staging releaser because both are exit-path duties and
# both must be safe to call when there is nothing to release.
#
# Ownership is VERIFIED rather than assumed, and the difference is not theoretical.
# A run whose own process was killed and whose working area was removed looks stale
# to the next one, which reclaims the lock legitimately; if this handler then ran
# -- on a signal, say -- an unconditional removal would delete the lock the OTHER
# run is holding, and the corpus would have two writers with neither knowing. The
# token settles it: this handler removes the lock only while the token on disk is
# the one this run wrote.
cf_release_lock() {
	if [ "$CF_LOCK_HELD" -eq 1 ] && [ -n "$CF_LOCK_DIR" ] && [ -n "$CF_LOCK_TOKEN" ]; then
		if [ "$(cf_lock_owner_field token)" = "$CF_LOCK_TOKEN" ]; then
			rm -f -- "$CF_LOCK_DIR/owner"
			rmdir -- "$CF_LOCK_DIR" 2> /dev/null || true
		fi
		CF_LOCK_HELD=0
	fi
}

cf_cleanup() {
	cf_stop_active_wrapper
	cf_release_staging
	cf_release_record_dir
	cf_release_lock
	if [ "$CF_WORK_OPEN" -eq 1 ]; then
		if [ -n "$CF_WORK_NAME" ] && [ -n "$CF_WORK" ] &&
			command test "$CF_WORK_NAME" -ef "$CF_WORK"; then
			rm -rf -- "$CF_WORK_NAME"
		elif [ -n "$CF_WORK_NAME" ] || [ -n "$CF_WORK" ]; then
			cf_error 'refusing to remove the private working area: its name no longer matches the pinned handle'
		fi
		exec 7<&-
		CF_WORK_OPEN=0
	fi
	CF_WORK=''
	CF_WORK_NAME=''
	if [ "$CF_CORPUS_OPEN" -eq 1 ]; then
		exec 6<&-
		CF_CORPUS_OPEN=0
	fi
	CF_CORPUS_HANDLE=''
}

trap cf_cleanup EXIT
trap 'cf_on_signal HUP' HUP
trap 'cf_on_signal INT' INT
trap 'cf_on_signal TERM' TERM

# =============================================================================
# Usage.
# =============================================================================

cf_usage() {
	cat <<'USAGE'
Usage: sh tests/conformance/tools/regenerate_expected.sh [OPTIONS]

Regenerate the expected_stdout golden record of differential-conformance
expectation records from the REFERENCE compiler. Maintenance only: this script
is never invoked by `cargo test`, and it never invokes the compiler under test.

Options:
  --area AREA        Restrict to one area directory, e.g. 01_integer_conversions
  --program SPEC     Restrict to one program, e.g. 04_bitfields/003_compound_assignment
                     A trailing .expected or .c is accepted and ignored.
  --check            Do not write; exit non-zero if any record would change
  -h, --help         Show this help

--area and --program are mutually exclusive, each may be given only once, and
neither may be empty: an empty narrowing option is refused rather than read as
"no restriction", because a maintainer who typed one and got a whole-corpus
rewrite would have been told the opposite of what happened. With neither option,
every record in the corpus is regenerated.

A selected record must be a regular file at exactly <area>/<program>.expected
inside this corpus, with a sibling <program>.c, no symbolic link in any path
component, and lowercase NN_<name> / NNN_<name> spellings. Anything else is
refused and named, never silently skipped.

Environment (all optional; the same names the harness uses):
  BCC_REF_CC                  native reference C compiler   [gcc, cc, clang]
  BCC_REF_CC_I686             i686 reference driver         [i686-linux-gnu-gcc]
  BCC_REF_CC_AARCH64          aarch64 reference driver      [aarch64-linux-gnu-gcc]
  BCC_REF_CC_RISCV64          riscv64 reference driver      [riscv64-linux-gnu-gcc]
  BCC_QEMU_I386               i686 runner       [qemu-i386, qemu-i386-static]
  BCC_QEMU_AARCH64            aarch64 runner    [qemu-aarch64, qemu-aarch64-static]
  BCC_QEMU_RISCV64            riscv64 runner    [qemu-riscv64, qemu-riscv64-static]
  BCC_CONFORMANCE_TIMEOUT_SECS  per-compile and per-run budget [30]
  TMPDIR                       private-work parent [trusted /tmp, then /var/tmp]

TMPDIR's complete physical ancestry must be owned by this account or root and
may be group/other writable only where the sticky bit protects this run's
private entry. An untrusted inherited value is refused rather than followed.

Every reference driver is ATTESTED before it compiles anything, and a driver that
fails is refused rather than used: the program at the end of its wrapper chain
must be established, it must answer -dumpmachine with the target it was chosen
for, and its DEFAULT language mode must be C11 or C17 with the GNU extensions
enabled. No -std flag is ever passed -- the compiler under test has none to
match -- so that default mode IS the language every recorded byte is judged in.
On a host whose unversioned `gcc` is a later major version defaulting to a later
revision, name the gnu17 driver through BCC_REF_CC: the probe order will find the
later one, and it will be refused rather than silently replaced.

BCC_BIN is deliberately NOT read: a golden record seeded from the compiler under
test would certify that compiler against itself. Independence is proved the other
way round, by requiring answers only a reference C driver can give.

Every record is validated in full BEFORE any of its cells are compiled, against the
same rules the harness parser applies -- the closed key set of 25 keys, field kinds,
duplicates, required keys, the four size ceilings, control bytes, the final newline,
marker completeness, the oracle switches, the shared-flag set and the three command
templates, and equally: the declared warning gate as one of the two SANCTIONED
reductions (with -pedantic droppable only in the extension area), a disabled oracle
only ever accompanied by a marker that scopes it and classes it comparison_excluded,
that class in turn scoping only oracles the record disables, and the marker's own
identifier, class, scope and basis grammar with the cited basis resolved to a file
that exists. Validation parity is the point: a record this script would rewrite but
the harness would refuse is a record whose regeneration makes the very test it was
for unrunnable. Only the expected_stdout block is ever written; every other byte is
copied through verbatim, and a record whose last byte is not a NEWLINE is refused
rather than terminated for you.

The PROGRAM is audited too, before it is compiled: a source that calls an
environment-reading, file-opening, network or process interface, or one whose output
could not be a function of the program alone (a clock, a random value, a process
identifier, a locale), is refused and named. Comments and literals are removed first
and only an identifier that is actually CALLED counts, so a program may print the
word "socket" and a variable may be named `time`.

Every compile and every run is bounded and hermetic. Each child is started through
`env -i`, so its environment is REPLACED rather than filtered and a variable reaches
it only by being named: LC_ALL/LANG/LANGUAGE=C, TZ=UTC, TERM=dumb, forced sanitizer
options, and HOME/TMPDIR/TMP/TEMP pointed inside the cell's own fresh private
directory. The PATH children receive is computed rather than inherited -- every
entry must be absolute, exist, and be writable by neither group nor other, with the
sticky bit granting no exemption -- and a host where no entry survives that rule is
refused rather than run. `timeout` is used where it is present and proven at startup
to bound anything, otherwise this script's own self-tested sleep/kill watchdog is,
and nothing is ever run unbounded.

Bounds this script imposes on itself, none of them configurable away:
  * Every compile and every run is bounded, and BCC_CONFORMANCE_TIMEOUT_SECS is
    accepted only in 1..600 seconds. A budget with no ceiling configures no bound
    while appearing to configure one.
  * The private working area is measured while a cell is running -- in BOTH bounding
    modes, by one supervisor, so choosing `timeout` never costs the space bound --
    and the cell is terminated if it passes 65536 KiB or 256 entries at any depth.
    The time bound is no defence against space: a process can write a great deal in
    one second. Each child additionally carries a kernel-enforced `ulimit -f`
    per-file ceiling, so one runaway write cannot fill the area between two ticks.
    Both bounding modes and the file ceiling are self-tested at startup, and what
    each self-test actually established is printed.
  * At most 512 records are processed in one invocation.
  * One writing run at a time, through an atomic lock directory in the corpus. A
    live lock refuses the second sweep; a lock still being initialised is waited
    for and retried rather than mistaken for an abandoned one; a stale lock is
    quarantined by a single rename -- which only one contender can win -- and then
    competed for afresh, loudly. A lock path that is a symbolic link is refused, and
    the release verifies this run's own token before removing anything, so a sweep
    can never release a lock belonging to another. --check takes no lock at all,
    because it writes nothing.
  * A record is written only through the directory its containment check resolved:
    staging and the installing rename are bare names inside that one resolved
    directory, and after the rename the published file is re-read and required to be
    a regular file, still inside the corpus area, and byte-identical to what was
    staged.
  * Every reference driver and runner the selection needs is attested BEFORE the
    first record is processed, so a missing or unusable tool cannot leave the
    corpus half rewritten; and each one's fingerprint is re-checked before every
    cell, so a package upgraded mid-run stops the sweep instead of splitting one
    record's golden bytes across two toolchains.

Exit codes: 0 success, 2 usage, 3 environment (a missing tool, a live lock, or a
            toolchain that changed mid-run), 4 record defect, 5 capture defect (an
            unusable stream, a timeout, a working-area breach, or cells that
            disagree), 6 --check found a record that would change.
Requires `mktemp`; `timeout` and `setsid` are optional. See the PORTABILITY banner
at the top of this script for the exact tested utility baseline.
USAGE
}

cf_usage_error() {
	cf_error "$*"
	cf_detail "run: sh tests/conformance/tools/regenerate_expected.sh --help"
	exit "$CF_EXIT_USAGE"
}

# =============================================================================
# Command line.
# =============================================================================

CF_ARG_AREA=''
CF_ARG_PROGRAM=''
CF_CHECK=0

# Whether the option was GIVEN, tracked separately from the value it carried.
#
# The two are genuinely different facts, and conflating them is how a narrowing
# option comes to widen an operation: `--area=` and `--area ''` both carry an
# empty value, and a check that reads emptiness as absence would treat either as
# "no restriction was asked for" and go on to process every record in the corpus.
# A maintainer who typed a narrowing option and got a whole-corpus rewrite has
# been told the opposite of what happened. So presence is recorded here, the value
# is required to be non-empty below, and both are settled before any path is
# resolved, any temporary directory is created and any record is enumerated.
CF_ARG_AREA_GIVEN=0
CF_ARG_PROGRAM_GIVEN=0

cf_require_value() {
	# $1 = option name, $2 = remaining argument count after the option
	if [ "$2" -lt 2 ]; then
		cf_usage_error "$1 requires a value"
	fi
}

# Refuse a second occurrence of a narrowing option. Last-one-wins would let a
# repeated option silently discard the earlier value -- including replacing a
# valid narrowing with an empty one -- and there is no reading of two different
# restrictions that is obviously the intended one.
cf_require_once() {
	# $1 = option name, $2 = whether it has already been given
	if [ "$2" -ne 0 ]; then
		cf_usage_error "$1 was given more than once; it takes a single value"
	fi
}

while [ "$#" -gt 0 ]; do
	case $1 in
	--area)
		cf_require_once --area "$CF_ARG_AREA_GIVEN"
		cf_require_value --area "$#"
		CF_ARG_AREA_GIVEN=1
		shift
		CF_ARG_AREA=$1
		;;
	--area=*)
		cf_require_once --area "$CF_ARG_AREA_GIVEN"
		CF_ARG_AREA_GIVEN=1
		CF_ARG_AREA=${1#--area=}
		;;
	--program)
		cf_require_once --program "$CF_ARG_PROGRAM_GIVEN"
		cf_require_value --program "$#"
		CF_ARG_PROGRAM_GIVEN=1
		shift
		CF_ARG_PROGRAM=$1
		;;
	--program=*)
		cf_require_once --program "$CF_ARG_PROGRAM_GIVEN"
		CF_ARG_PROGRAM_GIVEN=1
		CF_ARG_PROGRAM=${1#--program=}
		;;
	--check)
		CF_CHECK=1
		;;
	-h | --help)
		cf_usage
		exit 0
		;;
	--)
		shift
		if [ "$#" -gt 0 ]; then
			cf_usage_error "unexpected operand \"$1\"; this script takes options only"
		fi
		break
		;;
	-*)
		cf_usage_error "unknown option \"$1\""
		;;
	*)
		cf_usage_error "unexpected operand \"$1\"; this script takes options only"
		;;
	esac
	shift
done

# Presence, not emptiness: `--area=` is an area that was ASKED FOR and left blank,
# which is a mistake to report rather than a restriction to drop.
if [ "$CF_ARG_AREA_GIVEN" -ne 0 ] && [ "$CF_ARG_PROGRAM_GIVEN" -ne 0 ]; then
	cf_usage_error '--area and --program are mutually exclusive; --program already names its area'
fi
if [ "$CF_ARG_AREA_GIVEN" -ne 0 ] && [ -z "$CF_ARG_AREA" ]; then
	cf_usage_error '--area was given an empty value; it must name one area directory, e.g. 01_integer_conversions'
fi
if [ "$CF_ARG_PROGRAM_GIVEN" -ne 0 ] && [ -z "$CF_ARG_PROGRAM" ]; then
	cf_usage_error '--program was given an empty value; it must name <area>/<program>, e.g. 04_bitfields/003_compound_assignment'
fi

# =============================================================================
# The corpus path grammar.
#
# The corpus spells every area `NN_<name>` and every program `NNN_<name>`, with
# <name> in lowercase letters, digits and underscores. That grammar is enforced
# rather than assumed, in one place, and it does two jobs:
#
#   * it keeps a SELECTOR from reaching outside the corpus, since no accepted name
#     can contain "..", a slash, a glob character or a leading hyphen;
#   * it keeps DISCOVERY honest, since a directory or a record that does not match
#     is reported loudly instead of being walked into or quietly passed over. A
#     record decides what gets compiled and what counts as correct, so a record
#     the corpus layout does not account for must never be regenerated on the
#     strength of merely existing.
#
# Rejecting uppercase is deliberate and not fussiness: on a case-insensitive
# filesystem `01_Foo` and `01_foo` name the same directory while comparing
# unequal, so accepting both spellings would let the identity checks -- program
# against file stem, area against containing directory -- pass or fail depending
# on which spelling a maintainer happened to type.
# =============================================================================

# A <name> tail: at least one character, lowercase alphanumeric and underscore
# only, beginning with a letter so a name can never be all digits or start with a
# separator.
cf_validate_name_tail() {
	# $1 = tail, $2 = the whole name, $3 = what it is (for the message)
	case $1 in
	'')
		cf_usage_error "the $3 \"$2\" has an empty name after its number"
		;;
	[a-z]*) ;;
	*)
		cf_usage_error "the $3 \"$2\" does not begin its name with a lowercase letter"
		;;
	esac
	case $1 in
	*[!a-z0-9_]*)
		cf_usage_error "the $3 \"$2\" contains a character outside [a-z0-9_]; the corpus spells every name in lowercase"
		;;
	esac
}

cf_validate_area_name() {
	# $1 = value, $2 = what it is (for the message)
	if [ -z "$1" ]; then
		cf_usage_error "the $2 is empty"
	fi
	if cf_is_excluded_dir "$1"; then
		cf_usage_error "the $2 \"$1\" is a corpus companion directory, not a feature area; it holds no regenerable record"
	fi
	case $1 in
	[0-9][0-9]_*)
		cf_validate_name_tail "${1#??_}" "$1" "$2"
		;;
	*)
		cf_usage_error "the $2 \"$1\" is not a feature-area directory; areas are named NN_<name> with two digits, e.g. 01_integer_conversions"
		;;
	esac
}

cf_validate_program_name() {
	# $1 = value, $2 = what it is (for the message)
	if [ -z "$1" ]; then
		cf_usage_error "the $2 is empty"
	fi
	case $1 in
	[0-9][0-9][0-9]_*)
		cf_validate_name_tail "${1#???_}" "$1" "$2"
		;;
	*)
		cf_usage_error "the $2 \"$1\" is not a program name; programs are named NNN_<name> with three digits, e.g. 003_compound_assignment"
		;;
	esac
}

CF_SELECT_AREA=''
CF_SELECT_PROGRAM=''

if [ "$CF_ARG_AREA_GIVEN" -ne 0 ]; then
	cf_validate_area_name "$CF_ARG_AREA" 'area'
	CF_SELECT_AREA=$CF_ARG_AREA
fi

if [ "$CF_ARG_PROGRAM_GIVEN" -ne 0 ]; then
	# A trailing .expected or .c is accepted so a tab-completed filename works.
	case $CF_ARG_PROGRAM in
	*.expected) CF_ARG_PROGRAM=${CF_ARG_PROGRAM%.expected} ;;
	*.c) CF_ARG_PROGRAM=${CF_ARG_PROGRAM%.c} ;;
	esac
	case $CF_ARG_PROGRAM in
	*/*/*)
		cf_usage_error "--program takes <area>/<program>, so exactly one slash; got \"$CF_ARG_PROGRAM\""
		;;
	*/*) ;;
	*)
		cf_usage_error "--program takes <area>/<program>, e.g. 04_bitfields/003_compound_assignment; got \"$CF_ARG_PROGRAM\" with no slash"
		;;
	esac
	CF_SELECT_AREA=${CF_ARG_PROGRAM%%/*}
	CF_SELECT_PROGRAM=${CF_ARG_PROGRAM#*/}
	cf_validate_area_name "$CF_SELECT_AREA" 'area in --program'
	cf_validate_program_name "$CF_SELECT_PROGRAM" 'program in --program'
fi

# =============================================================================
# Locate the corpus relative to this script, never relative to the caller.
# =============================================================================

# `cd -P` and `pwd -P` rather than the logical forms, so both roots are PHYSICAL
# paths with every symbolic link already resolved. Every containment comparison
# below is made against these two values, and a comparison against a logical path
# would compare the route taken rather than the place arrived at -- which is
# exactly the difference a symbolic link exists to hide.
CF_TOOLS_DIR=$(CDPATH='' cd -P -- "$(dirname -- "$0")" && pwd -P) || {
	cf_error "cannot resolve the directory holding this script"
	exit "$CF_EXIT_ENVIRONMENT"
}
CF_CORPUS_DIR=$(CDPATH='' cd -P -- "$CF_TOOLS_DIR/.." && pwd -P) || {
	cf_error "cannot resolve the corpus directory above \"$CF_TOOLS_DIR\""
	exit "$CF_EXIT_ENVIRONMENT"
}

if [ "$(basename -- "$CF_TOOLS_DIR")" != 'tools' ]; then
	cf_error "this script must live in tests/conformance/tools/, but it was found in \"$CF_TOOLS_DIR\""
	exit "$CF_EXIT_ENVIRONMENT"
fi
# The package root, two levels above the corpus: a marker's basis cites a path
# relative to it, and the citation is required to name a file that exists.
CF_PACKAGE_ROOT=$(CDPATH='' cd -P -- "$CF_CORPUS_DIR/../.." && pwd -P) || {
	cf_error "cannot resolve the package root above \"$CF_CORPUS_DIR\""
	exit "$CF_EXIT_ENVIRONMENT"
}

if [ ! -f "$CF_CORPUS_DIR/README.md" ]; then
	cf_error "\"$CF_CORPUS_DIR\" does not look like the conformance corpus: README.md is missing"
	cf_detail 'the corpus contract is tests/conformance/README.md; the layout is expected to be unchanged'
	exit "$CF_EXIT_ENVIRONMENT"
fi
if ! exec 6< "$CF_CORPUS_DIR"; then
	cf_error 'cannot open the conformance corpus root as a durable directory handle'
	exit "$CF_EXIT_ENVIRONMENT"
fi
CF_CORPUS_HANDLE="/proc/$$/fd/6/."
if ! command test "$CF_CORPUS_DIR" -ef "$CF_CORPUS_HANDLE"; then
	cf_error 'the conformance corpus root could not be verified through Linux /proc fd 6'
	cf_detail 'this script requires /proc/$$/fd directory handles; it will not fall back to'
	cf_detail 're-resolving mutable path names for lock, staging, install or cleanup'
	exec 6<&-
	CF_CORPUS_HANDLE=''
	exit "$CF_EXIT_ENVIRONMENT"
fi
CF_CORPUS_OPEN=1

# =============================================================================
# Timeout budget.
# =============================================================================

CF_TIMEOUT_SECS=${BCC_CONFORMANCE_TIMEOUT_SECS:-30}
case $CF_TIMEOUT_SECS in
'' | *[!0-9]*)
	cf_error "BCC_CONFORMANCE_TIMEOUT_SECS is \"$CF_TIMEOUT_SECS\"; it must be a whole number of seconds"
	exit "$CF_EXIT_ENVIRONMENT"
	;;
esac
if [ "$CF_TIMEOUT_SECS" -lt 1 ]; then
	cf_error "BCC_CONFORMANCE_TIMEOUT_SECS is \"$CF_TIMEOUT_SECS\"; it must be at least 1 second"
	exit "$CF_EXIT_ENVIRONMENT"
fi
# The budget is bounded ABOVE as well, into a finite range. An arbitrarily large
# value configures no bound while appearing to configure one, and the whole point
# of the bound is that a single hung cell cannot stall a sweep of 1,296 of them.
if [ "$CF_TIMEOUT_SECS" -gt "$CF_TIMEOUT_SECS_MAX" ]; then
	cf_error "BCC_CONFORMANCE_TIMEOUT_SECS is \"$CF_TIMEOUT_SECS\"; it must be at most $CF_TIMEOUT_SECS_MAX seconds"
	cf_detail 'a corpus cell takes milliseconds; a budget this large is the same as no bound at all'
	exit "$CF_EXIT_ENVIRONMENT"
fi

# =============================================================================
# The search path.
#
# Every tool this script runs -- the reference drivers, the emulators, and the
# handful of utilities it leans on -- is found by NAME through PATH, so PATH
# decides which program each name means. An entry that an account other than this
# one can write to therefore decides it too: dropping a file called `awk`, `du`,
# `cmp` or `gcc-13` into such a directory is enough to be selected, and a golden
# record written afterwards would carry bytes from a toolchain nobody chose.
#
# So the inherited PATH is not used as given. It is filtered by exactly the rule
# tests/conformance_harness/env.rs applies to the same question, and the filtered
# value is what this script and every child it starts use:
#
#   * an EMPTY entry is refused -- a shell reads it as the current directory, which
#     would make the selected tool depend on where the script was invoked from;
#   * a RELATIVE entry is refused, for the same reason;
#   * an entry that does not resolve is skipped in silence: it contributes no
#     candidate and its absence is ordinary rather than suspicious;
#   * an entry writable by ANY account, or by every member of its owning GROUP, or
#     owned by an account that is neither the superuser nor this one, is refused --
#     a group is not a single trusted principal, and an owner may change its own
#     permissions at any time. The sticky bit is NOT an exemption: it restrains
#     deleting an entry, not creating one, and creating one is the whole of the
#     attack.
#
# Each accepted entry is recorded as its PHYSICAL path, so a symbolic link that
# resolves into a trusted directory is honoured as the place it arrives at rather
# than as the route taken -- and swapping the link afterwards cannot redirect a
# later lookup.
#
# WHAT THIS CANNOT DO, STATED PLAINLY: the filter is computed with `ls` and `id`,
# found through the PATH it is about to judge, so the bootstrap trusts the inherited
# path for exactly those two lookups. Closing that would need a facility a POSIX
# shell does not have. Everything after the filter -- every driver, every emulator,
# every utility, and every child -- resolves through the filtered value.
#
# A host with no trusted entry at all is refused rather than run: on such a machine
# there is no honest oracle, and a tool that cannot be found states that far better
# than one that silently runs whatever was planted.
# =============================================================================

# Resolve a tool name to an ABSOLUTE path to a regular executable file, or fail.
#
# Absolute, because the cells run with their own working directory: a name
# `command -v` answered with a relative path would be resolved against the
# caller's directory and would not mean the same thing once a cell has changed
# directory. Regular, because a directory or a device is not a tool however
# willing the executable bit looks.
CF_TOOL_PATH=''
cf_tool_path() {
	# $1 = the name or path to resolve
	CF_TOOL_PATH=''
	case $1 in
	-*)
		# `command -v` would read a leading hyphen as one of its own options, so
		# such a value is refused by name rather than probed.
		return 1
		;;
	esac
	cf_tool_path_found=$(command -v "$1" 2> /dev/null) || return 1
	[ -n "$cf_tool_path_found" ] || return 1
	case $cf_tool_path_found in
	/*) ;;
	*)
		cf_tool_path_dir=$(dirname -- "$cf_tool_path_found") || return 1
		cf_tool_path_base=$(basename -- "$cf_tool_path_found") || return 1
		cf_tool_path_dir=$(CDPATH='' cd -P -- "$cf_tool_path_dir" 2> /dev/null && pwd -P) ||
			return 1
		cf_tool_path_found="$cf_tool_path_dir/$cf_tool_path_base"
		;;
	esac
	[ -f "$cf_tool_path_found" ] || return 1
	[ -x "$cf_tool_path_found" ] || return 1
	CF_TOOL_PATH=$cf_tool_path_found
}

# This account's numeric identity, for the ownership half of the rule above.
CF_EUID=$(id -u 2> /dev/null) || CF_EUID=''
case $CF_EUID in
'' | *[!0-9]*) CF_EUID='' ;;
esac

# Why directory $1 is writable by an account this script does not trust, printed on
# stdout; exit status 1 when it is trustworthy. Optional $2 = runtime permits a
# sticky group/other-writable level: mktemp's unpredictable child cannot be
# planted in advance, and the sticky bit prevents another account replacing that
# child after this script creates it. PATH directories deliberately receive no
# such exemption because an attacker needs only to create a NEW tool name there.
#
# The mode string is read from `ls -ld`, whose first field is the ten characters
# POSIX defines: type, then user, group and other permissions in that order. So
# group-write is character 6 and other-write is character 9, and the patterns below
# say exactly that. `-n` prints the owner NUMERICALLY, which is what makes the
# comparison an identity comparison rather than a name lookup.
cf_untrusted_dir() {
	cf_ud_real=$(CDPATH='' cd -P -- "$1" 2> /dev/null && pwd -P) || {
		printf '%s\n' 'it does not resolve to a directory'
		return 0
	}
	# `ls -ld` rather than `find -perm`: this script does not depend on findutils at
	# all (see the PORTABILITY banner), the operand is a single directory path this
	# script resolved itself rather than a name from a listing, and only the first
	# field and the numeric owner are read -- so the filename-handling hazard the
	# check warns about cannot arise here.
	# shellcheck disable=SC2012
	cf_ud_line=$(ls -ldn -- "$cf_ud_real" 2> /dev/null | awk 'NR == 1 { print $1 " " $3; exit }')
	if [ -z "$cf_ud_line" ]; then
		printf '%s\n' 'its permissions could not be read'
		return 0
	fi
	cf_ud_mode=${cf_ud_line%% *}
	cf_ud_owner=${cf_ud_line##* }
	cf_ud_sticky=0
	case $cf_ud_mode in
	*[tT]) cf_ud_sticky=1 ;;
	esac
	case $cf_ud_mode in
	?????w????*)
		if [ "${2:-}" = 'runtime' ] && [ "$cf_ud_sticky" -eq 1 ]; then
			:
		else
			printf '%s\n' 'it is writable by every member of its owning group, which is not a single trusted principal'
			return 0
		fi
		;;
	esac
	case $cf_ud_mode in
	????????w*)
		if [ "${2:-}" = 'runtime' ] && [ "$cf_ud_sticky" -eq 1 ]; then
			:
		else
			printf '%s\n' 'it is writable by any account on this machine (the sticky bit is not an exemption: it restrains deleting an entry, not creating one)'
			return 0
		fi
		;;
	esac
	case $cf_ud_owner in
	'' | *[!0-9]*)
		printf '%s\n' 'its owner could not be read numerically'
		return 0
		;;
	esac
	if [ "$cf_ud_owner" != '0' ] && [ "$cf_ud_owner" != "${CF_EUID:-0}" ]; then
		printf '%s\n' "it is owned by account $cf_ud_owner, which is neither the superuser nor this one, and an owner may change its own permissions at any time"
		return 0
	fi
	return 1
}

# Canonicalize a runtime parent only after every PHYSICAL ancestor has passed the
# ownership/mode rule. The walk uses `cd -P` and `.` so an ancestor name is never
# resolved a second time while it is being judged. Prints the canonical parent.
cf_trusted_runtime_parent() (
	# $1 = candidate, $2 = diagnostic label
	cf_trp_candidate=$1
	cf_trp_label=$2
	CDPATH='' cd -P -- "$cf_trp_candidate" 2> /dev/null || {
		cf_error "$cf_trp_label \"$cf_trp_candidate\" does not resolve to a directory"
		exit 1
	}
	cf_trp_origin=$(pwd -P) || exit 1
	while :; do
		cf_trp_here=$(pwd -P) || exit 1
		if cf_trp_why=$(cf_untrusted_dir . runtime); then
			cf_error "$cf_trp_label \"$cf_trp_candidate\" is not a trusted runtime parent"
			cf_detail "$cf_trp_here: $cf_trp_why"
			exit 1
		fi
		[ "$cf_trp_here" = / ] && break
		CDPATH='' cd -P -- .. || exit 1
	done
	printf '%s\n' "$cf_trp_origin"
)

# Compute CF_TRUSTED_PATH from the inherited PATH, and report what was refused.
cf_sanitize_path() {
	cf_sp_inherited=${PATH:-}
	CF_TRUSTED_PATH=''
	cf_sp_refusals=0
	if [ -z "$cf_sp_inherited" ]; then
		cf_error 'PATH is not set, so no tool can be found by name'
		cf_detail 'every tool this script needs is resolved through PATH; there is nothing to resolve'
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	# An empty element is reported before the split, because the split cannot see the
	# difference between "a:" and "a" -- and an empty element is the one that means
	# "the current directory".
	case $cf_sp_inherited in
	:* | *::* | *:)
		cf_sp_refusals=$((cf_sp_refusals + 1))
		cf_detail 'refused an empty PATH element (a leading, trailing or doubled separator): a shell reads it as the current directory'
		;;
	esac
	cf_sp_rest=$cf_sp_inherited
	while [ -n "$cf_sp_rest" ]; do
		case $cf_sp_rest in
		*:*)
			cf_sp_entry=${cf_sp_rest%%:*}
			cf_sp_rest=${cf_sp_rest#*:}
			;;
		*)
			cf_sp_entry=$cf_sp_rest
			cf_sp_rest=''
			;;
		esac
		[ -n "$cf_sp_entry" ] || continue
		case $cf_sp_entry in
		/*) ;;
		*)
			cf_sp_refusals=$((cf_sp_refusals + 1))
			cf_detail "refused the relative PATH entry \"$cf_sp_entry\": it resolves against a working directory this script does not fix"
			continue
			;;
		esac
		cf_sp_real=$(CDPATH='' cd -P -- "$cf_sp_entry" 2> /dev/null && pwd -P) || continue
		if cf_sp_why=$(cf_untrusted_dir "$cf_sp_entry"); then
			cf_sp_refusals=$((cf_sp_refusals + 1))
			cf_detail "refused the PATH entry \"$cf_sp_entry\": $cf_sp_why"
			continue
		fi
		# Deduplicated on the physical path, so two spellings of one directory do not
		# each contribute a lookup.
		case ":$CF_TRUSTED_PATH:" in
		*":$cf_sp_real:"*) continue ;;
		esac
		if [ -z "$CF_TRUSTED_PATH" ]; then
			CF_TRUSTED_PATH=$cf_sp_real
		else
			CF_TRUSTED_PATH="$CF_TRUSTED_PATH:$cf_sp_real"
		fi
	done
	if [ -z "$CF_TRUSTED_PATH" ]; then
		cf_error 'no directory on PATH is trustworthy, so no tool can be resolved'
		cf_detail "inherited PATH: $cf_sp_inherited"
		cf_detail 'each entry was empty, relative, unresolvable, or writable by an account other than'
		cf_detail 'this one -- on such a host a planted file could be selected as the reference'
		cf_detail 'compiler, so nothing is compiled rather than something being trusted blindly'
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	if [ "$cf_sp_refusals" -ne 0 ]; then
		cf_note "$cf_sp_refusals PATH entry/entries refused (listed above); tools are resolved through the remainder"
	fi
}

cf_sanitize_path
PATH=$CF_TRUSTED_PATH
export PATH
if ! cf_process_start_token $$ > /dev/null; then
	cf_error 'Linux /proc process start-time identities are unavailable'
	cf_detail 'numeric PIDs are not identities after a shell may have reaped them, so this'
	cf_detail 'script refuses to supervise or signal a child without /proc/<pid>/stat field 22'
	exit "$CF_EXIT_ENVIRONMENT"
fi

# =============================================================================
# Private working area. An inherited TMPDIR is accepted only after its complete
# physical ancestry passes the runtime-parent owner/mode rule. With no TMPDIR,
# /tmp is used when trusted and /var/tmp is the fallback; this host's /tmp is
# intentionally refused because it is world-writable without sticky protection.
#
# The directory is opened on fd 7 immediately after mktemp and every later work
# path goes through `/proc/$$/fd/7/.`. A mutable ancestor can rename the visible
# entry, but it cannot redirect a write through that already-open handle. The
# visible name is retained only for identity-checked cleanup.
#
# Deliberately NOT under target/: .gitignore names exactly three paths there, and
# a fourth would be untracked-but-unignored and would dirty `git status`.
# =============================================================================

umask 077
if [ -n "${TMPDIR:-}" ]; then
	CF_WORK_PARENT=$(cf_trusted_runtime_parent "$TMPDIR" TMPDIR) || {
		cf_detail 'unset TMPDIR or point it at a directory owned by this account/root whose'
		cf_detail 'physical ancestry is not group/other writable without sticky protection'
		exit "$CF_EXIT_ENVIRONMENT"
	}
elif CF_WORK_PARENT=$(cf_trusted_runtime_parent /tmp /tmp 2> /dev/null); then
	:
elif CF_WORK_PARENT=$(cf_trusted_runtime_parent /var/tmp /var/tmp 2> /dev/null); then
	cf_note '/tmp is not a trusted runtime parent; using /var/tmp'
else
	cf_error 'neither /tmp nor /var/tmp is a trusted runtime parent'
	exit "$CF_EXIT_ENVIRONMENT"
fi
CF_WORK_NAME=$(mktemp -d -- "$CF_WORK_PARENT/bcc-conformance-regenerate.XXXXXX") || {
	cf_error 'cannot create a private working directory with mktemp -d'
	exit "$CF_EXIT_ENVIRONMENT"
}
if ! exec 7< "$CF_WORK_NAME"; then
	cf_error 'cannot open the private working directory as a durable handle'
	exit "$CF_EXIT_ENVIRONMENT"
fi
CF_WORK="/proc/$$/fd/7/."
if ! command test "$CF_WORK_NAME" -ef "$CF_WORK"; then
	cf_error 'the private working directory changed identity before its handle was verified'
	exec 7<&-
	CF_WORK=''
	exit "$CF_EXIT_ENVIRONMENT"
fi
CF_WORK_OPEN=1
if cf_work_why=$(cf_untrusted_dir "$CF_WORK"); then
	cf_error "the private working directory is not private: $cf_work_why"
	exit "$CF_EXIT_ENVIRONMENT"
fi
mkdir -p -- "$CF_WORK/cell"

# =============================================================================
# Bounded execution.
#
# EVERY compile, every run and every attestation probe is bounded. Nothing this
# script starts is ever executed without a bound: a compiler or a generated
# program that hangs would otherwise block forever, with no status, no
# diagnostic and no cleanup.
#
# WHY THE OUTCOME IS REPORTED OUT OF BAND
# ---------------------------------------
# A cell may legitimately exit with ANY status in 0..125, and `timeout` reports a
# bound that fired as 124. Measured on the implementation here: `timeout 5 sh -c
# 'exit 124'` reports 124 and a genuine bound reports 124, so the status alone
# cannot tell the two apart, and a record declaring `expect_exit = 124` would be
# rejected as a hang. `timeout -k` is no better -- it reports 125 -- so the
# collision cannot be dodged by choosing a different sentinel: every value in
# 0..125 is a status a program may legitimately choose.
#
# So the child's TRUE status is written to a private status file by a wrapper
# whose own LAST command is that write, which is why the wrapper exits 0 whatever
# the child did, and a bound that fired is recorded by a separate marker instead.
# The child cannot forge either file, because it never learns their names.
#
# TWO MECHANISMS, ONE SUPERVISOR, ONE DECODING
# --------------------------------------------
# With `timeout` present it enforces the TIME bound. Without it, this script
# enforces time itself, from the same supervisor that enforces space.
#
# The SPACE bound is enforced identically in both modes, and that symmetry is the
# point rather than a tidiness: a bound that exists in one mode and not the other
# is a bound whose presence depends on which packages happen to be installed. So
# the cell is backgrounded in both modes and one supervisor watches it, measuring
# the working area once a second while it runs and terminating it on a breach. In
# `timeout` mode the supervisor leaves time alone, because `timeout` already owns
# it; in the fallback mode the same loop owns both.
#
# One BACKGROUND WRAPPER owns that cell and supervisor together. The parent keeps
# the wrapper PID plus its Linux start token in trap-visible state. On HUP/INT/TERM
# the parent signals and waits for the wrapper; the wrapper's own trap
# identity-checks, terminates and reaps the cell group and supervisor first. Thus
# work/staging/lock cleanup cannot overtake a still-running compiler.
#
# The supervisor is not the only space bound, because a poll cannot outrun a
# redirection: a cell can write far more than the ceiling between two polls. So
# every child also carries a HARD per-file limit installed with `ulimit -f`, which
# the kernel enforces on the write itself. The two together bound one enormous file
# and many merely large ones.
#
# What neither mechanism can promise without `setsid` is a process GROUP: creating
# one needs a facility POSIX shells do not offer, so a grandchild that outlives its
# signalled parent is possible. That is stated rather than glossed over, and it is
# still bounded execution: the process this script started is terminated, and the
# outcome is reported.
#
# Whichever mechanism ran, the decoding is the same and lives in one place:
# a status file means the child completed and holds its true status; no status
# file means it did not, and the markers say whether that was the time bound, the
# space bound, or something else.
# =============================================================================

CF_BOUND_STATUS_FILE="$CF_WORK/bound.status"
CF_BOUND_FIRED_FILE="$CF_WORK/bound.fired"
# The status the bounding mechanism itself reported, written from inside the
# subshell that ran it so the caller can read it: a subshell's variables do not
# survive, and a file does.
CF_BOUND_OUTER_FILE="$CF_WORK/bound.outer"
# Set by the live working-area supervisor when it terminates a cell for space.
# Out of band for the same reason the bound marker is: a status cannot carry it.
CF_QUOTA_FIRED_FILE="$CF_WORK/bound.quota"
CF_QUOTA_REASON_FILE="$CF_WORK/bound.quota.why"
# Set whenever a numeric PID resolves to a different Linux start token at the
# moment a supervisor or cleanup path would signal it. This is an infrastructure
# refusal, never a timeout or quota outcome.
CF_IDENTITY_FIRED_FILE="$CF_WORK/bound.identity"
CF_IDENTITY_REASON_FILE="$CF_WORK/bound.identity.why"

# Set by cf_bound_run. Exactly one of the first two is meaningful per call.
CF_BOUND_STATUS=''  # the child's true exit status, when it completed
CF_BOUND_TIMEDOUT=0 # 1 when the bound fired and the child was terminated
CF_BOUND_ABORTED='' # non-empty when the bound itself failed to run the child

# The interpreter that hosts the status-writing wrapper. `/bin/sh` rather than a
# PATH lookup because this script's own interpreter is `/bin/sh`: a host that
# lacks it could not have started the script at all, so naming it absolutely adds
# no dependency and removes one lookup that PATH could redirect.
CF_BOUND_SHELL='/bin/sh'

# `env` is a HARD requirement, and the reason is the one below about the child's
# environment: `env -i` is the only POSIX way to hand a child a KNOWN environment
# rather than a filtered version of whatever this script inherited. A host without it
# cannot give the guarantee, so it is refused rather than run with a weaker one.
if cf_tool_path env; then
	CF_ENV_BIN=$CF_TOOL_PATH
else
	cf_error 'the env utility is not on PATH, and it is required'
	cf_detail 'every compile and every run is started through "env -i" so that the child receives'
	cf_detail 'exactly the variables this script names and nothing else; without it an ambient'
	cf_detail 'setting -- or a credential -- would reach a compiler and could reach a golden record'
	exit "$CF_EXIT_ENVIRONMENT"
fi
# Proven, not assumed. A utility named `env` that does not clear the environment
# would leave every child inheriting one while this script reported otherwise.
# The single quotes are the point: this text is a program for the INNER shell, so the
# expansion has to survive this one and happen there -- the same reason the status
# wrapper below quotes the way it does.
# shellcheck disable=SC2016
CF_ENV_PROBE=$(CF_ISOLATION_PROBE=leaked "$CF_ENV_BIN" -i "$CF_BOUND_SHELL" -c \
	'printf %s "${CF_ISOLATION_PROBE-cleared}"' 2> /dev/null || true)
if [ "$CF_ENV_PROBE" != 'cleared' ]; then
	cf_error "\"$CF_ENV_BIN\" -i did not clear the environment it was given"
	cf_detail "a variable exported into it came back as: ${CF_ENV_PROBE:-<no output>}"
	cf_detail 'the child environment is the only thing standing between an ambient credential and a'
	cf_detail 'golden record, so an env that does not clear is refused rather than trusted'
	exit "$CF_EXIT_ENVIRONMENT"
fi

if cf_tool_path timeout; then
	CF_TIMEOUT_BIN=$CF_TOOL_PATH
	CF_BOUND_MODE='timeout'
else
	CF_TIMEOUT_BIN=''
	CF_BOUND_MODE='watchdog'
fi

# The wrapper. Its last command is the status write, so it exits 0 whatever the
# child did -- which is precisely what makes the child's status readable from the
# file rather than confusable with the bound's own report.
#
# Every expansion is escaped rather than single-quoted: this text is a program for
# the INNER shell, so each `$` has to survive the assignment here and be expanded
# there. The first argument is the status file and the rest are the command.
CF_BOUND_WRAPPER="cf_bound_status_file=\$1
shift
\"\$@\"
printf '%s\n' \"\$?\" > \"\$cf_bound_status_file\""

# =============================================================================
# The environment every child is given.
#
# A golden record is a byte-exact expectation, so anything in the environment that
# can change what a compiler emits or what a program prints can change the bytes
# this script writes -- and anything in it that a compiler or a program can PRINT
# can be copied verbatim into a golden record. An ambient credential is both.
#
# So the environment is not filtered, it is REPLACED. Every child is started through
# `env -i`, which clears it, and is then handed exactly the variables named below and
# nothing else. That is the same posture -- and the same list --
# tests/conformance_harness/mod.rs installs on every child it spawns through
# isolate_child_environment; keep them in step.
#
# Four groups, and the reasoning differs for each:
#
#   * THE SEARCH PATH, set to the sanitized value computed under "The search path"
#     rather than to the inherited one. A driver looks up its own subprograms, and an
#     untrusted directory on the child's path decides which ones it finds.
#   * FIXED so that output is reproducible. The C locale makes number and message
#     formatting invariant; a locale that prints a decimal comma would silently
#     change a floating-point line. TZ removes any dependence on the host's time
#     zone, and TERM=dumb stops a tool deciding to emit colour escapes into a
#     stream that is about to be compared byte for byte.
#   * POINTED AT THE CELL so that a well-behaved tool writing a cache, a history
#     file or a temporary file puts it inside the cell's own workspace rather than
#     into the invoking user's home directory or a shared temporary directory. All
#     four spellings are set, because different tools consult different ones and a
#     tool reading the one left unset would fall back to the shared location.
#   * SANITIZER OPTIONS forced to their strictest values, so that no inherited
#     setting can weaken a diagnostic.
#
# Nothing is UNSET any more, and that is the improvement rather than an omission: a
# list of variables to unset has to anticipate every name that could matter --
# LD_PRELOAD, GCC_EXEC_PREFIX, COMPILER_PATH, CPATH, SOURCE_DATE_EPOCH and the rest
# -- and the one nobody thought of is the one that reaches the compiler. Clearing
# the environment inverts the burden: a variable reaches a child only by being named
# here.
#
# WHAT THIS IS, PRECISELY: a known environment, not confinement. A cleared
# environment cannot carry a credential into a compiler or a program, and cannot
# redirect a toolchain -- but a variable is only one route, and nothing here stops a
# child writing to an absolute path of its own choosing. What the hermeticity of a
# cell actually rests on is stated where it is true: every child is spawned with the
# cell's own workspace as its working directory and with a hard file-size limit,
# every path this script hands a tool lies inside that workspace, the corpus
# authoring rules forbid a program from opening a socket or naming a path at all,
# and cf_audit_source_surface refuses a program whose source names an
# environment-reading, file-opening, process-spawning or network API at all.
# =============================================================================

# The sanitizer settings, each the strictest available: a diagnostic terminates the
# process rather than being recovered from, and it is printed rather than suppressed.
# Spelled once, here, because they are handed to children in two places.
CF_ASAN_OPTIONS='abort_on_error=1:halt_on_error=1:detect_leaks=1:print_summary=1:exitcode=1'
CF_UBSAN_OPTIONS='halt_on_error=1:print_stacktrace=1:silence_unsigned_overflow=0'
CF_LSAN_OPTIONS='exitcode=1'
CF_MSAN_OPTIONS='halt_on_error=1:exitcode=1'
CF_TSAN_OPTIONS='halt_on_error=1:exitcode=1'

# =============================================================================
# What a test program is allowed to reach for.
# =============================================================================
#
# A cleared environment and a per-cell workspace bound what a program INHERITS and
# where a well-behaved one writes. Neither bounds what a program's own source code
# asks the C library to do, and this script's whole purpose is to take the bytes such
# a program prints and install them as the definition of correct output.
#
# Two separate guarantees rest on that, and both are the corpus authoring rules
# rather than anything the operating system enforces:
#
#   * HERMETICITY. A generated program must not touch the network or a path outside
#     its workspace. A program that opened a socket or a file would satisfy every
#     check in this script and violate the constraint anyway, because nothing here
#     traps a syscall.
#   * DETERMINISM. Byte-exact comparison across four backends and three optimization
#     levels is only meaningful if the bytes are a function of the program alone. A
#     clock reading, a random value, a process identifier or a locale-dependent
#     conversion makes the golden record a record of one moment on one machine.
#
# So the surface is audited in the source, before anything is compiled: a program
# that NAMES one of these interfaces is refused rather than run. The scan is textual
# and it is deliberately conservative in what it looks at -- comments and literals
# are removed first, so prose and format strings can discuss anything, and only an
# identifier that is actually CALLED counts, so a local variable named `time` is not
# a finding. What it is not is a sandbox: it is the mechanical half of an authoring
# rule whose other half is prose, and a determined author can still defeat it through
# a function pointer or a macro. It closes the accidental case, which is the case that
# actually occurs.

# Interfaces a test program may not call, by the reason it may not. One group per
# variable so the refusal can say which guarantee the call would break.
CF_SURFACE_ENVIRONMENT='getenv secure_getenv putenv setenv unsetenv clearenv
confstr sysconf pathconf uname sysinfo getlogin getpwuid getpwnam'
CF_SURFACE_FILE='fopen freopen fdopen open openat creat fread fwrite fgets fputs
fputc fgetc getc putc getchar getline getdelim fscanf scanf fprintf vfprintf
read write pread pwrite close lseek dup dup2 stat fstat lstat fstatat access
faccessat opendir fdopendir readdir rewinddir closedir remove rename renameat
unlink unlinkat link symlink readlink mkdir mkdirat rmdir chdir fchdir getcwd
chmod chown truncate ftruncate mmap munmap msync tmpfile tmpnam tempnam mkstemp
mkdtemp popen pclose fsync fdatasync sync fseek fseeko ftell ftello rewind
setbuf setvbuf freadable ioctl fcntl flock select poll'
CF_SURFACE_NETWORK='socket socketpair connect bind listen accept accept4 send
sendto sendmsg sendfile recv recvfrom recvmsg shutdown setsockopt getsockopt
getsockname getpeername getaddrinfo freeaddrinfo getnameinfo gethostbyname
gethostbyaddr inet_addr inet_aton inet_ntoa inet_pton inet_ntop if_nametoindex'
CF_SURFACE_PROCESS='system fork vfork clone execl execlp execle execv execvp
execvpe execve posix_spawn posix_spawnp wait waitpid waitid kill raise signal
sigaction sigprocmask alarm pause abort atexit at_quick_exit quick_exit
_exit setjmp longjmp sigsetjmp siglongjmp pthread_create pthread_join
dlopen dlsym dlclose dladdr syscall'
CF_SURFACE_NONDETERMINISM='time clock times gettimeofday settimeofday
clock_gettime clock_getres difftime mktime ctime asctime localtime gmtime
strftime strptime rand rand_r srand random srandom initstate setstate
arc4random arc4random_uniform getpid getppid getuid geteuid getgid getegid
setlocale localeconv nl_langinfo'

# Objects -- not calls -- whose mere appearance is the same defect. `environ` IS the
# environment, and a `main` that names its parameters is reading the invocation.
CF_SURFACE_OBJECTS='environ __environ _environ argc argv envp'

# Print the C source $1 with comments, string literals and character literals
# replaced by a space, one output line per input line so a line number still means
# something.
#
# Removing literals is what makes the audit usable rather than merely strict: a
# program is free to PRINT the word "socket", and a comment is free to explain why the
# program does not open one. Only code is scanned.
cf_strip_source() {
	awk '
		BEGIN { inblock = 0 }
		{
			line = $0
			out = ""
			i = 1
			n = length(line)
			while (i <= n) {
				c = substr(line, i, 1)
				two = substr(line, i, 2)
				if (inblock) {
					if (two == "*/") { inblock = 0; i += 2 } else { i++ }
					continue
				}
				if (two == "/*") { inblock = 1; out = out " "; i += 2; continue }
				if (two == "//") { break }
				if (c == "\"" || c == "'"'"'") {
					quote = c
					i++
					while (i <= n) {
						d = substr(line, i, 1)
						if (d == "\\") { i += 2; continue }
						i++
						if (d == quote) { break }
					}
					out = out " "
					continue
				}
				out = out c
				i++
			}
			print out
		}
	' < "$1"
}

# Print every identifier the C source $1 CALLS, one per line.
#
# Requiring a following `(` is what keeps this precise: a local variable named `time`
# is not a call to `time()`, and refusing it would be a false positive an author could
# not act on. The whole stripped text is joined into one stream first, so a name and
# its `(` separated by a line break still pair up.
cf_source_calls() {
	cf_strip_source "$1" | awk '
		{ out = out " " $0 }
		END {
			while (match(out, /[A-Za-z_][A-Za-z_0-9]*[ \t]*\(/)) {
				token = substr(out, RSTART, RLENGTH)
				sub(/[ \t]*\($/, "", token)
				print token
				out = substr(out, RSTART + RLENGTH)
			}
		}
	'
}

# Refuse a program whose source reaches outside itself. $1 = source path, $2 = label.
# Returns non-zero after reporting every offending name, so an author sees all of
# them in one run rather than one per invocation.
cf_audit_source_surface() {
	# $1 = source path, $2 = record label
	cf_ass_bad=0
	cf_source_calls "$1" | sort -u > "$CF_WORK/calls.list" || {
		cf_error "$2: cannot read the program source to audit what it calls"
		return 1
	}
	while IFS= read -r cf_ass_call; do
		cf_ass_why=''
		if cf_list_has "$CF_SURFACE_ENVIRONMENT" "$cf_ass_call"; then
			cf_ass_why='reads the environment or the host configuration'
		elif cf_list_has "$CF_SURFACE_FILE" "$cf_ass_call"; then
			cf_ass_why='opens, reads or writes a file'
		elif cf_list_has "$CF_SURFACE_NETWORK" "$cf_ass_call"; then
			cf_ass_why='reaches the network'
		elif cf_list_has "$CF_SURFACE_PROCESS" "$cf_ass_call"; then
			cf_ass_why='starts, signals or unwinds out of a process'
		elif cf_list_has "$CF_SURFACE_NONDETERMINISM" "$cf_ass_call"; then
			cf_ass_why='produces a value that is not a function of the program alone'
		fi
		if [ -n "$cf_ass_why" ]; then
			cf_error "$2: the program calls $cf_ass_call(), which $cf_ass_why"
			cf_detail "first named on line $(cf_first_line "$1" "$cf_ass_call")"
			cf_ass_bad=$((cf_ass_bad + 1))
		fi
	done < "$CF_WORK/calls.list"
	for cf_ass_name in $CF_SURFACE_OBJECTS; do
		if cf_source_names_object "$1" "$cf_ass_name"; then
			cf_error "$2: the program names \"$cf_ass_name\", which is the invocation environment rather than a value of its own"
			cf_detail "first named on line $(cf_first_line "$1" "$cf_ass_name")"
			cf_ass_bad=$((cf_ass_bad + 1))
		fi
	done
	if [ "$cf_ass_bad" -gt 0 ]; then
		cf_detail 'every input a test program needs is a literal in its own source: that is what'
		cf_detail 'makes a cell reproducible from the source and its record alone, keeps the run'
		cf_detail 'hermetic, and makes byte-exact comparison across four backends meaningful'
		return 1
	fi
	return 0
}

# True when the C source $1 uses $2 as a whole identifier outside comments and
# literals. Separate from the call scan because these are objects, not calls.
cf_source_names_object() {
	cf_sno_hits=$(cf_strip_source "$1" | awk -v want="$2" '
		{
			gsub(/[^A-Za-z_0-9]/, " ")
			for (i = 1; i <= NF; i++) { if ($i == want) { found++ } }
		}
		END { print found + 0 }
	')
	[ "$cf_sno_hits" -gt 0 ]
}

# The first line of file $1 on which $2 appears as a whole identifier, or `?` when
# the name survives only across a line boundary. Advisory: it points a reader at the
# right place and is never what decides the refusal.
cf_first_line() {
	cf_strip_source "$1" | awk -v want="$2" '
		{
			gsub(/[^A-Za-z_0-9]/, " ")
			for (i = 1; i <= NF; i++) {
				if ($i == want) { print NR; exit }
			}
		}
	' | awk 'NR == 1 { print; found = 1 } END { if (!found) { print "?" } }'
}

# A tool's identity, as a single line: resolved path, size in KiB blocks, and the
# first line of its version banner.
#
# Built from utilities this script already depends on -- `command -v`, `du -k` and
# awk -- rather than from `stat`, which is not POSIX and is not in this script's
# tool set. That is why the size is in blocks rather than bytes: it is a change
# detector, not a measurement, and the version banner is the field that actually
# moves when a package is upgraded underneath a running sweep.
cf_tool_fingerprint() {
	cf_fp_path=$(command -v -- "$1" 2> /dev/null) || cf_fp_path=$1
	cf_fp_size=$(du -k -- "$cf_fp_path" 2> /dev/null | awk 'NR == 1 { print $1 + 0; exit }')
	case $cf_fp_size in
	'' | *[!0-9]*) cf_fp_size='size-unreadable' ;;
	esac
	cf_fp_banner=$("$1" --version 2> /dev/null | awk 'NR == 1 { print; exit }')
	printf '%s|%s|%s\n' "$cf_fp_path" "$cf_fp_size" "$cf_fp_banner"
}

# Require that a tool is still the tool it was when it was attested.
#
# Attestation happens once per tool, but a sweep compiles up to 1,296 cells and a
# package can be upgraded while it runs. One record's golden bytes must come from
# ONE toolchain, so the fingerprint is re-compared before every cell and a change
# stops the run rather than splitting a record across two toolchains -- which is
# the one failure mode a differential suite cannot detect about itself afterwards,
# because each half is internally consistent.
#
# Keyed by a file in the private working area rather than by a per-target variable,
# so one predicate serves every driver and every runner.
cf_require_stable_tool() {
	# $1 = tool as named, $2 = human label
	[ -n "$1" ] || return 0
	cf_fp_key=$(printf '%s' "$1" | tr -c 'A-Za-z0-9' '_')
	cf_fp_file="$CF_WORK/fingerprint.$cf_fp_key"
	cf_fp_now=$(cf_tool_fingerprint "$1")
	if [ -f "$cf_fp_file" ]; then
		cf_fp_was=$(cat -- "$cf_fp_file")
		if [ "$cf_fp_now" != "$cf_fp_was" ]; then
			cf_error "the $2 changed while this run was in progress"
			cf_detail "attested as: $cf_fp_was"
			cf_detail "now:         $cf_fp_now"
			cf_detail 'one record'"'"'s golden bytes must come from one toolchain, so the run stops'
			cf_detail 'here rather than splitting a record across two. Nothing further is written.'
			exit "$CF_EXIT_ENVIRONMENT"
		fi
		return 0
	fi
	printf '%s\n' "$cf_fp_now" > "$cf_fp_file"
}

# Is `setsid` available? Probed once. With it, a cell is started as its own
# session leader, so its pid is also its process-group id and a whole process
# TREE can be terminated -- which matters because a compiler driver forks: killing
# only the direct child can leave the real compiler running. Without it the exact
# child is terminated and that weaker guarantee is stated rather than implied.
if command -v setsid > /dev/null 2>&1; then
	CF_SETSID='setsid'
else
	CF_SETSID=''
fi

# cf_signal_cell <signal> <pid> <start-token>: signal the cell, or its whole
# group where it was started as a session leader, only while the leader still
# has the identity captured at launch. The external kill utility and `--` are
# required for a negative group operand on the supported procps-ng host; dash's
# builtin accepts a different form and is deliberately bypassed.
cf_signal_cell() {
	CF_SIGNAL_REASON=''
	CF_SIGNAL_BREACH=0
	cf_sc_now=$(cf_process_start_token "$2" 2> /dev/null) || cf_sc_now=''
	if [ -z "$3" ]; then
		CF_SIGNAL_REASON="cell process $2 had no start-time identity captured at launch"
		CF_SIGNAL_BREACH=1
		return 1
	fi
	if [ -z "$cf_sc_now" ]; then
		CF_SIGNAL_REASON="cell process $2 no longer has a readable start-time identity"
		return 1
	fi
	if [ "$cf_sc_now" != "$3" ]; then
		CF_SIGNAL_REASON="cell process $2 changed start-time identity from $3 to $cf_sc_now"
		CF_SIGNAL_BREACH=1
		return 1
	fi
	if [ -n "$CF_SETSID" ]; then
		command env kill -"$1" -- "-$2" 2> /dev/null
	else
		command env kill -"$1" -- "$2" 2> /dev/null
	fi
}

# True only while the visible mktemp name still designates the directory held on
# fd 7. All work I/O uses the handle regardless; this check makes measurement
# loss and cleanup substitution explicit rather than silently acting on a name.
cf_work_identity_current() {
	[ "$CF_WORK_OPEN" -eq 1 ] &&
		[ -n "$CF_WORK_NAME" ] &&
		[ -n "$CF_WORK" ] &&
		command test "$CF_WORK_NAME" -ef "$CF_WORK"
}

# Walk the tree beneath $1, counting entries at EVERY depth, following no link, and
# stopping as soon as the ceiling is exceeded. Depth $2 is the recursion level.
#
# Three properties, each of which the predecessor lacked:
#
#   * ALL DEPTHS. The predecessor descended exactly one level, so a cell that wrote a
#     million files three levels down was measured as holding one directory. A
#     ceiling that a subdirectory defeats is not a ceiling.
#   * NO FOLLOWING. A symbolic link to a directory is counted as one entry and is
#     never descended into. Following one would measure a tree the cell does not own
#     -- and a link to `/` would make the measurement unbounded.
#   * BOUNDED ANYWAY. Counting stops at the first entry past the ceiling, so a
#     directory holding a million files costs the same as one holding a few hundred,
#     and the depth is capped as well. Exceeding the depth cap sets
#     CF_WALK_TOO_DEEP, which the caller treats as a BREACH rather than as a stopping
#     point: a tree this walk cannot finish is one it must not certify as small.
#
# Written with shell globbing rather than `find`, which this script does not depend
# on at all, so the traversal rules above are its own decisions rather than a
# question about which findutils is installed. The three patterns together match
# ordinary, dot- and dot-dot-prefixed names without ever matching `.` or `..`.
CF_WALK_COUNT=0
CF_WALK_TOO_DEEP=0
CF_WALK_UNMEASURED=''
cf_walk_entries() {
	if [ ! -d "$1" ] || [ ! -r "$1" ] || [ ! -x "$1" ]; then
		CF_WALK_UNMEASURED="directory $1 could not be traversed completely"
		return 0
	fi
	for cf_walk_entry in "$1"/* "$1"/.[!.]* "$1"/..?*; do
		if [ ! -e "$cf_walk_entry" ] && [ ! -L "$cf_walk_entry" ]; then
			continue
		fi
		CF_WALK_COUNT=$((CF_WALK_COUNT + 1))
		if [ "$CF_WALK_COUNT" -gt "$CF_WORK_ENTRIES_MAX" ]; then
			return 0
		fi
		if [ -d "$cf_walk_entry" ] && [ ! -L "$cf_walk_entry" ]; then
			if [ "$2" -ge "$CF_WALK_DEPTH_MAX" ]; then
				CF_WALK_TOO_DEEP=1
				return 0
			fi
			cf_walk_entries "$cf_walk_entry" $(($2 + 1))
			if [ "$CF_WALK_COUNT" -gt "$CF_WORK_ENTRIES_MAX" ] ||
				[ "$CF_WALK_TOO_DEEP" -eq 1 ] ||
				[ -n "$CF_WALK_UNMEASURED" ]; then
				return 0
			fi
		fi
	done
	if [ ! -d "$1" ] || [ ! -r "$1" ] || [ ! -x "$1" ]; then
		CF_WALK_UNMEASURED="directory $1 changed or became unreadable during traversal"
	fi
}

# Count the entries beneath $1 at every depth. Leaves the count in CF_WALK_COUNT and
# sets CF_WALK_TOO_DEEP when the tree is deeper than the walk will go.
#
# Both results are returned through variables, and this must NOT be called with
# `$(...)`: a command substitution runs a subshell, so the count would come back and
# the depth flag would not -- and a tree too deep to measure is precisely the case
# where the count is the answer that must not be trusted.
cf_count_entries() {
	CF_WALK_COUNT=0
	CF_WALK_TOO_DEEP=0
	CF_WALK_UNMEASURED=''
	cf_walk_entries "$1" 0
}

# Is the private working area past any live ceiling? Sets CF_QUOTA_REASON to a
# ready-to-print sentence when it is, and clears it when it is not. Every lost
# measurement is itself a breach: a failed `du`, a non-numeric result, an
# unreadable traversal or a changed root identity can hide an arbitrarily large
# tree and therefore cannot certify the cell as within quota.
cf_quota_breach() {
	CF_QUOTA_REASON=''
	if ! cf_work_identity_current; then
		CF_QUOTA_REASON='the private working area name no longer designates the pinned work handle'
		return 0
	fi
	if ! cf_quota_line=$(du -sk "$CF_WORK" 2> /dev/null); then
		CF_QUOTA_REASON='the working-area byte measurement failed, so its live size is unknown'
		return 0
	fi
	if ! cf_quota_kb=$(printf '%s\n' "$cf_quota_line" |
		awk 'NR == 1 && $1 ~ /^[0-9]+$/ { value=$1; next } { bad=1 }
		     END { if (!bad && value != "") print value; else exit 1 }'); then
		CF_QUOTA_REASON='the working-area byte measurement was not one numeric du result'
		return 0
	fi
	if [ "$cf_quota_kb" -gt "$CF_WORK_KB_MAX" ]; then
		CF_QUOTA_REASON="the working area held ${cf_quota_kb} KiB, past the ${CF_WORK_KB_MAX} KiB live ceiling"
		return 0
	fi
	cf_count_entries "$CF_WORK"
	if [ -n "$CF_WALK_UNMEASURED" ]; then
		CF_QUOTA_REASON="the working-area entry measurement was incomplete: $CF_WALK_UNMEASURED"
		return 0
	fi
	if [ "$CF_WALK_TOO_DEEP" -eq 1 ]; then
		CF_QUOTA_REASON="the working area nested more than ${CF_WALK_DEPTH_MAX} levels deep, which is past the depth this measurement covers"
		return 0
	fi
	if [ "$CF_WALK_COUNT" -gt "$CF_WORK_ENTRIES_MAX" ]; then
		CF_QUOTA_REASON="the working area held ${CF_WALK_COUNT} entries, past the ${CF_WORK_ENTRIES_MAX}-entry live ceiling"
		return 0
	fi
	if ! cf_work_identity_current; then
		CF_QUOTA_REASON='the private working area changed identity during live quota measurement'
	fi
	return 0
}

# cf_supervise_cell <pid> <start-token> <seconds>: watch one running cell until it is gone.
#
# ONE supervisor for both bounding mechanisms, which is the whole point: the space
# ceilings are enforced identically whether or not `timeout` is installed, so the
# guarantee does not depend on the package set. The division of labour is the only
# difference between the modes -- in `timeout` mode the utility owns the time bound
# and this loop owns space alone; in the fallback mode this loop owns both.
#
# The working area is measured WHILE the cell runs, because the time bound is no
# defence against space: a process can write a great deal in one second. A breach
# terminates the cell the same way the time bound does, and records WHY out of band,
# because an exit status cannot carry it.
#
# Runs in a subshell of its own, so the counters it keeps cannot leak into the caller.
cf_supervise_cell() {
	cf_sup_pid=$1
	cf_sup_start=$2
	cf_sup_secs=$3
	cf_sup_tick=0
	while :; do
		sleep 1
		cf_sup_now=$(cf_process_start_token "$cf_sup_pid" 2> /dev/null) || return 0
		if [ "$cf_sup_now" != "$cf_sup_start" ]; then
			printf 'cell process %s changed start-time identity from %s to %s\n' \
				"$cf_sup_pid" "$cf_sup_start" "$cf_sup_now" > "$CF_IDENTITY_REASON_FILE"
			: > "$CF_IDENTITY_FIRED_FILE"
			return 0
		fi
		if cf_quota_breach && [ -n "$CF_QUOTA_REASON" ]; then
			printf '%s\n' "$CF_QUOTA_REASON" > "$CF_QUOTA_REASON_FILE"
			: > "$CF_QUOTA_FIRED_FILE"
			if ! cf_signal_cell TERM "$cf_sup_pid" "$cf_sup_start"; then
				if [ "$CF_SIGNAL_BREACH" -eq 1 ]; then
					printf '%s\n' "$CF_SIGNAL_REASON" > "$CF_IDENTITY_REASON_FILE"
					: > "$CF_IDENTITY_FIRED_FILE"
				fi
				return 0
			fi
			sleep 1
			if ! cf_signal_cell KILL "$cf_sup_pid" "$cf_sup_start" &&
				[ "$CF_SIGNAL_BREACH" -eq 1 ]; then
				printf '%s\n' "$CF_SIGNAL_REASON" > "$CF_IDENTITY_REASON_FILE"
				: > "$CF_IDENTITY_FIRED_FILE"
			fi
			return 0
		fi
		cf_sup_tick=$((cf_sup_tick + 1))
		if [ "$CF_BOUND_MODE" != 'timeout' ] && [ "$cf_sup_tick" -ge "$cf_sup_secs" ]; then
			# The marker is written only when the signal was actually delivered. A
			# child that finished a moment before the last tick can no longer be
			# signalled, so it is reported by its own status rather than as a bound it
			# never reached.
			if cf_signal_cell TERM "$cf_sup_pid" "$cf_sup_start"; then
				: > "$CF_BOUND_FIRED_FILE"
				sleep 1
				if ! cf_signal_cell KILL "$cf_sup_pid" "$cf_sup_start" &&
					[ "$CF_SIGNAL_BREACH" -eq 1 ]; then
					printf '%s\n' "$CF_SIGNAL_REASON" > "$CF_IDENTITY_REASON_FILE"
					: > "$CF_IDENTITY_FIRED_FILE"
				fi
			elif [ "$CF_SIGNAL_BREACH" -eq 1 ]; then
				printf '%s\n' "$CF_SIGNAL_REASON" > "$CF_IDENTITY_REASON_FILE"
				: > "$CF_IDENTITY_FIRED_FILE"
			fi
			return 0
		fi
	done
}

# Record one identity refusal for the parent decoder. The first reason wins so a
# secondary cleanup refusal cannot overwrite the event that made the run unsafe.
cf_record_identity_refusal() {
	[ -n "$1" ] || return 0
	if [ ! -f "$CF_IDENTITY_FIRED_FILE" ]; then
		printf '%s\n' "$1" > "$CF_IDENTITY_REASON_FILE"
		: > "$CF_IDENTITY_FIRED_FILE"
	fi
}

# Owned by the background wrapper around one bound. It is the only function that
# may retire the cell and supervisor variables: every signal path calls it, and
# the normal path clears each variable only after its corresponding wait.
cf_bound_owned_cleanup() {
	if [ -n "${cf_bound_child:-}" ]; then
		cf_boc_now=$(cf_process_start_token "$cf_bound_child" 2> /dev/null) || cf_boc_now=''
		if [ -n "$cf_boc_now" ]; then
			if [ "$cf_boc_now" = "${cf_bound_child_start:-}" ]; then
				if cf_signal_cell TERM "$cf_bound_child" "$cf_bound_child_start"; then
					sleep 1
					if ! cf_signal_cell KILL "$cf_bound_child" "$cf_bound_child_start"; then
						[ "$CF_SIGNAL_BREACH" -eq 0 ] ||
							cf_record_identity_refusal "$CF_SIGNAL_REASON"
					fi
				else
					[ "$CF_SIGNAL_BREACH" -eq 0 ] ||
						cf_record_identity_refusal "$CF_SIGNAL_REASON"
				fi
			else
				cf_record_identity_refusal "cell process $cf_bound_child changed start-time identity from ${cf_bound_child_start:-<unavailable>} to $cf_boc_now"
			fi
		fi
		wait "$cf_bound_child" > /dev/null 2>&1 || true
		cf_bound_child=''
		cf_bound_child_start=''
	fi
	if [ -n "${cf_bound_supervisor:-}" ]; then
		cf_boc_now=$(cf_process_start_token "$cf_bound_supervisor" 2> /dev/null) || cf_boc_now=''
		if [ -n "$cf_boc_now" ]; then
			if [ "$cf_boc_now" = "${cf_bound_supervisor_start:-}" ]; then
				if ! cf_signal_pid TERM "$cf_bound_supervisor" "$cf_bound_supervisor_start"; then
					[ "$CF_SIGNAL_BREACH" -eq 0 ] ||
						cf_record_identity_refusal "$CF_SIGNAL_REASON"
				fi
			else
				cf_record_identity_refusal "quota supervisor $cf_bound_supervisor changed start-time identity from ${cf_bound_supervisor_start:-<unavailable>} to $cf_boc_now"
			fi
		fi
		wait "$cf_bound_supervisor" > /dev/null 2>&1 || true
		cf_bound_supervisor=''
		cf_bound_supervisor_start=''
	fi
}

cf_bound_wrapper_on_signal() {
	trap '' HUP INT TERM
	cf_bound_owned_cleanup
	exit "$2"
}

# cf_bound_run <seconds> <workdir> <stdout-file> <stderr-file> <command> [args...]
#
# The child runs with <workdir> as its working directory, with the environment
# above, and with stdin from /dev/null so it can never consume the record or cell
# list an enclosing read loop is iterating over. Results come back in the three
# CF_BOUND_* variables.
cf_bound_run() {
	cf_bound_secs=$1
	cf_bound_workdir=$2
	cf_bound_out=$3
	cf_bound_err=$4
	shift 4

	CF_BOUND_STATUS=''
	CF_BOUND_TIMEDOUT=0
	CF_BOUND_ABORTED=''
	rm -f -- "$CF_BOUND_STATUS_FILE" "$CF_BOUND_FIRED_FILE" \
		"$CF_QUOTA_FIRED_FILE" "$CF_QUOTA_REASON_FILE" "$CF_BOUND_OUTER_FILE" \
		"$CF_IDENTITY_FIRED_FILE" "$CF_IDENTITY_REASON_FILE"
	CF_QUOTA_REASON=''

	if [ ! -d "$cf_bound_workdir" ]; then
		CF_BOUND_ABORTED="the working directory \"$cf_bound_workdir\" does not exist"
		return 0
	fi

	CF_ACTIVE_READY=0
	CF_ACTIVE_WRAPPER=''
	CF_ACTIVE_WRAPPER_START=''
	(
		cf_bound_child=''
		cf_bound_child_start=''
		cf_bound_supervisor=''
		cf_bound_supervisor_start=''
		trap cf_bound_owned_cleanup EXIT
		trap 'cf_bound_wrapper_on_signal HUP 129' HUP
		trap 'cf_bound_wrapper_on_signal INT 130' INT
		trap 'cf_bound_wrapper_on_signal TERM 143' TERM
		CDPATH='' cd -P -- "$cf_bound_workdir" || exit 126
		# The hard per-file backstop, installed on this subshell so every child
		# inherits it. Attempted rather than required: a host whose hard limit is
		# already lower keeps its own, and cf_verify_file_backstop proves at startup
		# that the limit an ordinary host installs actually bites.
		ulimit -f "$CF_FILE_BLOCKS_MAX" 2> /dev/null || true
		# ONE argument vector, built once and used by both mechanisms, so a child
		# receives the same cleared environment whichever one starts it. `set --`
		# prepends: the command and its arguments are already the positional
		# parameters here, and they stay in place after them.
		set -- "$CF_ENV_BIN" -i \
			"PATH=$CF_TRUSTED_PATH" \
			LANG=C LC_ALL=C LANGUAGE=C TZ=UTC TERM=dumb \
			"ASAN_OPTIONS=$CF_ASAN_OPTIONS" \
			"UBSAN_OPTIONS=$CF_UBSAN_OPTIONS" \
			"LSAN_OPTIONS=$CF_LSAN_OPTIONS" \
			"MSAN_OPTIONS=$CF_MSAN_OPTIONS" \
			"TSAN_OPTIONS=$CF_TSAN_OPTIONS" \
			"HOME=$cf_bound_workdir" "TMPDIR=$cf_bound_workdir" \
			"TMP=$cf_bound_workdir" "TEMP=$cf_bound_workdir" \
			"$CF_BOUND_SHELL" -c "$CF_BOUND_WRAPPER" \
			cf_bound "$CF_BOUND_STATUS_FILE" "$@"
		# Backgrounded in BOTH modes, because the supervisor needs a pid to watch and
		# the space ceilings must hold whether or not `timeout` is installed. Where
		# `setsid` exists the cell leads its own session in both modes too, so a
		# termination can reach the whole process tree rather than the direct child --
		# which matters for `timeout` exactly as much as for the fallback, since a
		# compiler driver forks.
		if [ "$CF_BOUND_MODE" = 'timeout' ]; then
			${CF_SETSID:+$CF_SETSID} "$CF_TIMEOUT_BIN" "$cf_bound_secs" "$@" \
				< /dev/null > "$cf_bound_out" 2> "$cf_bound_err" &
		else
			${CF_SETSID:+$CF_SETSID} "$@" \
				< /dev/null > "$cf_bound_out" 2> "$cf_bound_err" &
		fi
		cf_bound_child=$!
		cf_bound_child_start=$(cf_process_start_token "$cf_bound_child" 2> /dev/null) ||
			cf_bound_child_start=''
		# The supervisor's own stderr is discarded: it says nothing a caller needs,
		# and a host where `sleep` is missing would otherwise print one line per tick
		# ahead of the single precise refusal cf_verify_bound already produces for
		# exactly that case.
		cf_supervise_cell "$cf_bound_child" "$cf_bound_child_start" "$cf_bound_secs" 2> /dev/null &
		cf_bound_supervisor=$!
		cf_bound_supervisor_start=$(cf_process_start_token "$cf_bound_supervisor" 2> /dev/null) ||
			cf_bound_supervisor_start=''
		if wait "$cf_bound_child" > /dev/null 2>&1; then
			cf_bound_outer=0
		else
			cf_bound_outer=$?
		fi
		cf_bound_child=''
		cf_bound_child_start=''
		printf '%s\n' "$cf_bound_outer" > "$CF_BOUND_OUTER_FILE"
		# `timeout` reports a bound that fired as 124, and that is the only thing this
		# branch does with the outer status: it is turned into the same marker the
		# fallback writes, so the decoding below is identical for both mechanisms.
		if [ "$CF_BOUND_MODE" = 'timeout' ] && [ "$cf_bound_outer" -eq 124 ]; then
			: > "$CF_BOUND_FIRED_FILE"
		fi
		cf_bound_owned_cleanup
		exit 0
	) &
	CF_ACTIVE_WRAPPER=$!
	CF_ACTIVE_WRAPPER_START=$(cf_process_start_token "$CF_ACTIVE_WRAPPER" 2> /dev/null) ||
		CF_ACTIVE_WRAPPER_START=''
	CF_ACTIVE_READY=1
	if [ -n "$CF_PENDING_SIGNAL" ]; then
		cf_bound_pending=$CF_PENDING_SIGNAL
		CF_PENDING_SIGNAL=''
		cf_on_signal "$cf_bound_pending"
	fi
	wait "$CF_ACTIVE_WRAPPER" > /dev/null 2>&1 || true
	CF_ACTIVE_WRAPPER=''
	CF_ACTIVE_WRAPPER_START=''
	if [ -f "$CF_BOUND_OUTER_FILE" ]; then
		cf_bound_outer=$(cat -- "$CF_BOUND_OUTER_FILE")
	else
		cf_bound_outer='unreported'
	fi

	# One decoding, whichever mechanism ran. The status file wins over the marker,
	# so a child that completed is never reported as a bound it did not reach.
	# Process-identity refusal is the exception: it is an infrastructure breach,
	# not a child outcome, and therefore dominates even a status written earlier.
	if [ -f "$CF_IDENTITY_FIRED_FILE" ]; then
		if [ -f "$CF_IDENTITY_REASON_FILE" ]; then
			cf_bound_identity_reason=$(cat -- "$CF_IDENTITY_REASON_FILE")
		else
			cf_bound_identity_reason='a process identity could not be verified before signalling'
		fi
		CF_BOUND_ABORTED="the bounded run refused a recycled or unverified process identity: $cf_bound_identity_reason"
		return 0
	fi
	if [ -f "$CF_BOUND_STATUS_FILE" ]; then
		CF_BOUND_STATUS=$(cat -- "$CF_BOUND_STATUS_FILE")
		case $CF_BOUND_STATUS in
		'' | *[!0-9]*)
			CF_BOUND_ABORTED="the bounded run recorded \"$CF_BOUND_STATUS\", which is not an exit status"
			CF_BOUND_STATUS=''
			;;
		esac
		return 0
	fi
	if [ -f "$CF_QUOTA_FIRED_FILE" ]; then
		if [ -f "$CF_QUOTA_REASON_FILE" ]; then
			CF_QUOTA_REASON=$(cat -- "$CF_QUOTA_REASON_FILE")
		else
			CF_QUOTA_REASON='the working area passed a live ceiling'
		fi
		CF_BOUND_ABORTED="the cell was terminated for space: $CF_QUOTA_REASON"
		return 0
	fi
	if [ -f "$CF_BOUND_FIRED_FILE" ]; then
		CF_BOUND_TIMEDOUT=1
		return 0
	fi
	CF_BOUND_ABORTED="the bounded run was terminated without completing (the bound reported $cf_bound_outer)"
}

# Prove the bound both reports a true status and terminates a hang, BEFORE any
# cell is spent on it. A `timeout` that is present but does not bound, or a
# fallback whose `sleep` or `kill` is missing, is caught here rather than
# discovered when a compiler hangs -- which is what makes "every compile and
# every run is bounded" a verified claim instead of an intention.
cf_verify_bound_mode() {
	cf_verify_dir="$CF_WORK/boundcheck"
	rm -rf -- "$cf_verify_dir"
	mkdir -p -- "$cf_verify_dir"

	# A command that completes must yield its OWN status, including the value a
	# bound is conventionally reported as. This is the check that would have
	# caught the collision this mechanism exists to remove.
	cf_bound_run "$CF_TIMEOUT_SECS" "$cf_verify_dir" \
		"$cf_verify_dir/out" "$cf_verify_dir/err" \
		"$CF_BOUND_SHELL" -c 'exit 124'
	if [ -n "$CF_BOUND_ABORTED" ] || [ "$CF_BOUND_TIMEDOUT" -ne 0 ] ||
		[ "$CF_BOUND_STATUS" != '124' ]; then
		cf_error "the $CF_BOUND_MODE execution bound does not report a completed command's own status"
		cf_detail "a command exiting 124 was reported as: status \"$CF_BOUND_STATUS\", timed out $CF_BOUND_TIMEDOUT${CF_BOUND_ABORTED:+, aborted: $CF_BOUND_ABORTED}"
		cf_detail 'without a trustworthy status there is no way to tell a program that chose an'
		cf_detail 'exit code from one the bound terminated, so nothing is compiled or run'
		exit "$CF_EXIT_ENVIRONMENT"
	fi

	# A command that outlives its budget must be terminated and reported as a
	# bound. A one-second budget against a five-second sleep: the budget is short
	# so a working bound costs about a second, and the sleep is only as long as it
	# needs to be to outlast it several times over, so a bound that does NOT work
	# is detected in five seconds rather than in the full budget.
	cf_bound_run 1 "$cf_verify_dir" \
		"$cf_verify_dir/out" "$cf_verify_dir/err" \
		"$CF_BOUND_SHELL" -c 'sleep 5'
	if [ "$CF_BOUND_TIMEDOUT" -ne 1 ]; then
		cf_error "the $CF_BOUND_MODE execution bound did not terminate a command that outlived it"
		cf_detail "a 1-second bound on a 5-second sleep was reported as: status \"$CF_BOUND_STATUS\", timed out $CF_BOUND_TIMEDOUT${CF_BOUND_ABORTED:+, aborted: $CF_BOUND_ABORTED}"
		if [ "$CF_BOUND_MODE" = 'watchdog' ]; then
			cf_detail 'no timeout utility was found, so this script bounds runs itself with sleep and'
			cf_detail 'kill; one of those is missing or unusable here'
			cf_detail 'install the timeout utility, which is the preferred mechanism:'
			cf_detail '    apt-get install -y coreutils'
		else
			cf_detail 'the timeout utility on PATH accepted the invocation but did not bound it'
		fi
		cf_detail 'compiling and running unbounded is refused rather than attempted: a hang would'
		cf_detail 'block forever with no status, no diagnostic and no cleanup'
		exit "$CF_EXIT_ENVIRONMENT"
	fi

	# A cell that fills the working area must be terminated for SPACE, in this mode,
	# whichever mechanism owns time. The ceilings are tightened for the duration of
	# the check so that it costs one tick instead of 64 MiB of writing, and restored
	# immediately afterwards -- the mechanism under test is the supervisor, not the
	# size of the number it compares against.
	cf_verify_kb_max=$CF_WORK_KB_MAX
	cf_verify_entries_max=$CF_WORK_ENTRIES_MAX
	CF_WORK_KB_MAX=1
	CF_WORK_ENTRIES_MAX=2
	cf_bound_run "$CF_TIMEOUT_SECS" "$cf_verify_dir" \
		"$cf_verify_dir/out" "$cf_verify_dir/err" \
		"$CF_BOUND_SHELL" -c \
		'awk "BEGIN { line = sprintf(\"%99999s\", \"\"); for (i = 0; i < 40; i++) { print line } }" > flood; sleep 20'
	CF_WORK_KB_MAX=$cf_verify_kb_max
	CF_WORK_ENTRIES_MAX=$cf_verify_entries_max
	case $CF_BOUND_ABORTED in
	*'terminated for space'*) ;;
	*)
		cf_error "the $CF_BOUND_MODE execution bound did not terminate a cell that passed the live working-area ceiling"
		cf_detail "a cell writing far past a 1 KiB ceiling was reported as: status \"$CF_BOUND_STATUS\", timed out $CF_BOUND_TIMEDOUT${CF_BOUND_ABORTED:+, aborted: $CF_BOUND_ABORTED}"
		cf_detail 'the time bound is no defence against space, so a run whose space bound does not'
		cf_detail 'work is refused rather than started: one runaway cell would fill the disk'
		exit "$CF_EXIT_ENVIRONMENT"
		;;
	esac

	rm -rf -- "$cf_verify_dir"
}

# Prove the HARD per-file limit actually stops a write, rather than assuming that
# `ulimit -f` took. It is the only bound a fast writer cannot outrun -- the
# supervisor polls once a second and a redirection is faster than that -- so a host
# where it silently did nothing is a host where the space guarantee is one poll wide.
#
# Deliberately not fatal. On a host whose shell or kernel declines the limit the
# supervisor still holds, so the run continues with the weaker guarantee STATED
# rather than refused: reporting it is what keeps the claim honest, and refusing
# would trade a real capability for a bound that is already covered.
cf_verify_file_backstop() {
	cf_backstop_dir="$CF_WORK/backstopcheck"
	rm -rf -- "$cf_backstop_dir"
	mkdir -p -- "$cf_backstop_dir"
	cf_backstop_blocks=$CF_FILE_BLOCKS_MAX
	CF_FILE_BLOCKS_MAX=1
	cf_bound_run "$CF_TIMEOUT_SECS" "$cf_backstop_dir" \
		"$cf_backstop_dir/out" "$cf_backstop_dir/err" \
		"$CF_BOUND_SHELL" -c \
		'awk "BEGIN { line = sprintf(\"%9999s\", \"\"); for (i = 0; i < 200; i++) { print line } }" > oversize'
	CF_FILE_BLOCKS_MAX=$cf_backstop_blocks
	cf_backstop_size=$(du -k -- "$cf_backstop_dir/oversize" 2> /dev/null |
		awk 'NR == 1 { print $1 + 0; exit }')
	case $cf_backstop_size in
	'' | *[!0-9]*) cf_backstop_size=0 ;;
	esac
	rm -rf -- "$cf_backstop_dir"
	if [ "$cf_backstop_size" -le 4 ]; then
		cf_note "hard per-file ceiling: ulimit -f ${CF_FILE_BLOCKS_MAX} blocks (verified)"
		return 0
	fi
	cf_note "hard per-file ceiling: ulimit -f is NOT enforced on this host (a 512-byte limit still let ${cf_backstop_size} KiB be written)"
	cf_detail 'the live working-area supervisor is therefore the only space bound, and it measures'
	cf_detail 'once a second -- a cell can write more than the ceiling between two measurements'
}

# Prove BOTH mechanisms, not merely whichever one this host will use.
#
# A bound that is only ever exercised in the mode the current machine happens to
# select is a bound whose other half is unproven, and the other half is exactly what
# a host without `timeout` runs. So each mode is verified in turn wherever it can be:
# the fallback always can, since it needs only `sleep` and `kill`.
cf_verify_bound() {
	cf_verify_selected=$CF_BOUND_MODE
	if [ -n "$CF_TIMEOUT_BIN" ]; then
		CF_BOUND_MODE='timeout'
		cf_verify_bound_mode
		cf_note "execution bound: timeout ${CF_TIMEOUT_SECS}s and the live space ceilings (verified)"
	fi
	CF_BOUND_MODE='watchdog'
	cf_verify_bound_mode
	cf_note "execution bound: this script's own sleep/kill watchdog and the live space ceilings (verified)"
	CF_BOUND_MODE=$cf_verify_selected
	cf_verify_file_backstop
	if [ "$CF_BOUND_MODE" = 'watchdog' ]; then
		cf_note "no timeout utility found; this run bounds time with the verified watchdog (${CF_TIMEOUT_SECS}s)"
	else
		cf_note "this run bounds time with timeout ${CF_TIMEOUT_SECS}s"
	fi
}

cf_verify_bound

# =============================================================================
# Record enumeration.
#
# WHY THIS WALKS THE CORPUS ITSELF INSTEAD OF CALLING `find`
# ---------------------------------------------------------
# A record dictates what gets compiled and what counts as correct, so the set of
# records is not a convenience -- it is the scope of every write this script makes.
# Two properties are needed, and a recursive `find … | sort` supplies neither:
#
#   * THE SET MUST BE COMPLETE OR THE RUN MUST FAIL. In a pipeline the exit status
#     is the LAST command's, so `find` failing part way through a traversal --
#     unreadable directory, I/O error, missing utility -- yields a PARTIAL list
#     with a successful status, and the run then reports success having silently
#     skipped records. Nothing downstream can notice, because a shorter list looks
#     exactly like a smaller corpus.
#   * THE SHAPE MUST BE EXACTLY `<area>/<program>.expected`. A recursive walk
#     accepts a record at any depth below any directory it was not told to prune,
#     so a record nested inside an area, or sitting at the corpus root, would be
#     regenerated on the strength of existing -- and would be compiled and
#     rewritten while every message still showed a corpus-relative path.
#
# So the corpus is walked here, one level at a time, with shell pathname expansion:
# every entry at the top level is CLASSIFIED and anything unaccounted for is a hard
# error, and only the direct children of an accepted area directory are considered.
# Depth is then a property of the code rather than a flag, there is no pipeline to
# hide a failure, and `find` stops being a dependency at all.
#
# Pathname expansion is sorted by the collating sequence in effect, which LC_ALL=C
# fixes to byte order; the list is nonetheless sorted explicitly afterwards, as a
# separate command whose status is checked, so the processing order is identical on
# every run and on every machine without relying on that guarantee.
# =============================================================================

# The three Markdown companions that live beside the area directories. Named so an
# unexpected file at the corpus root is reported rather than mistaken for one.
cf_is_corpus_document() {
	case $1 in
	README.md | EXPECTED_DIVERGENCES.md | FINDINGS.md) return 0 ;;
	esac
	return 1
}

# Refuse anything at the corpus root that the layout does not account for.
cf_reject_corpus_entry() {
	# $1 = entry name, $2 = why
	cf_error "the corpus directory holds \"$1\", which the layout does not account for: $2"
	cf_detail "corpus directory: $CF_CORPUS_DIR"
	cf_detail 'the corpus holds exactly: the NN_<name> feature-area directories, the companions'
	cf_detail 'findings/, support/ and tools/, and README.md, EXPECTED_DIVERGENCES.md and'
	cf_detail 'FINDINGS.md. Anything else is refused rather than walked into, because a record'
	cf_detail 'decides what is compiled and what counts as correct'
	cf_detail 'if this is a leftover from an interrupted run, remove it and try again'
	exit "$CF_EXIT_RECORD"
}

# Refuse anything inside an area directory that the layout does not account for.
cf_reject_area_entry() {
	# $1 = area, $2 = entry name, $3 = why
	cf_error "area $1 holds \"$2\", which the layout does not account for: $3"
	cf_detail 'a feature-area directory holds exactly NNN_<name>.c and NNN_<name>.expected pairs'
	cf_detail 'a nested directory, a symbolic link, an unrecognised extension or a name outside'
	cf_detail 'the grammar is refused rather than skipped: a blanket skip would mean that'
	cf_detail 'mis-naming a file silently removed its cells from the run while it still reported'
	cf_detail 'success'
	exit "$CF_EXIT_RECORD"
}

# True when $1 matches the area grammar, without exiting. cf_validate_area_name
# reports a USAGE error, which is right for a selector a maintainer typed and wrong
# for an entry found on disk.
cf_looks_like_area() {
	case $1 in
	[0-9][0-9]_[a-z]*) ;;
	*) return 1 ;;
	esac
	case ${1#??_} in
	*[!a-z0-9_]*) return 1 ;;
	esac
	return 0
}

# True when $1 matches the program grammar, without exiting.
cf_looks_like_program() {
	case $1 in
	[0-9][0-9][0-9]_[a-z]*) ;;
	*) return 1 ;;
	esac
	case ${1#???_} in
	*[!a-z0-9_]*) return 1 ;;
	esac
	return 0
}

# The physical directory $1 resolves to, or nothing when it cannot be entered.
cf_physical_dir() {
	(CDPATH='' cd -P -- "$1" 2> /dev/null && pwd -P) || return 1
}

# Require that $1 is a real area directory of THIS corpus: a directory, not a
# symbolic link, and physically located at $CF_CORPUS_DIR/$1 rather than merely
# reachable through that name. The second half is the one that matters: a symbolic
# link named like an area would let every later check pass while the record read,
# the program compiled and the file rewritten all lived somewhere else entirely.
cf_require_area_dir() {
	# $1 = area name, $2 = 'usage' when a selector named it, 'record' when found
	cf_require_area_path="$CF_CORPUS_DIR/$1"
	if [ -L "$cf_require_area_path" ]; then
		cf_error "area $1 is a symbolic link, which is refused"
		cf_detail "path: $cf_require_area_path"
		cf_detail 'a link could point anywhere, so compiling and rewriting through one would act'
		cf_detail 'outside the corpus while every message still showed a corpus path'
		if [ "$2" = 'usage' ]; then exit "$CF_EXIT_USAGE"; fi
		exit "$CF_EXIT_RECORD"
	fi
	if [ ! -d "$cf_require_area_path" ]; then
		cf_error "no such area directory: $1"
		cf_detail "looked for: $cf_require_area_path"
		if [ "$2" = 'usage' ]; then exit "$CF_EXIT_USAGE"; fi
		exit "$CF_EXIT_RECORD"
	fi
	cf_require_area_real=$(cf_physical_dir "$cf_require_area_path") || cf_require_area_real=''
	if [ "$cf_require_area_real" != "$cf_require_area_path" ]; then
		cf_error "area $1 does not physically live inside this corpus"
		cf_detail "named:   $cf_require_area_path"
		cf_detail "resolves to: ${cf_require_area_real:-<unresolvable>}"
		cf_detail 'every record read, program compiled and file rewritten must lie beneath the'
		cf_detail 'corpus directory itself, so a path that resolves elsewhere is refused'
		if [ "$2" = 'usage' ]; then exit "$CF_EXIT_USAGE"; fi
		exit "$CF_EXIT_RECORD"
	fi
}

# The one dot-prefixed name a directory of this corpus may hold: the marker that
# keeps an otherwise empty committed directory tracked.
CF_KEPT_DOT_ENTRY='.gitkeep'

# The two dot-prefixed names this script itself creates at the corpus root: the
# regeneration lock, and the short-lived directory a stale lock is quarantined into
# before it is discarded. Both are named here rather than beside the lock protocol
# because the ENUMERATION has to know them too -- it walks the corpus root before the
# lock is taken, and it rejects an entry the layout does not account for.
#
# That ordering was a real defect rather than a hypothetical one: a lock left behind
# by a killed run made the next run fail during enumeration, before cf_acquire_lock
# could reclaim it, so the documented "a stale one is reclaimed loudly" was
# unreachable and the corpus needed a manual `rm` to become usable again.
CF_LOCK_ENTRY='.regen-lock'
CF_LOCK_STALE_PREFIX='.regen-lock-stale.'

# True when $1 is one of this script's own transient root entries.
cf_is_regen_state_entry() {
	case $1 in
	"$CF_LOCK_ENTRY" | "$CF_LOCK_STALE_PREFIX"*) return 0 ;;
	esac
	return 1
}

# List the records of ONE accepted area directory, one per line, on stdout.
# Every entry is classified; nothing is passed over silently.
#
# BOTH glob patterns are walked, and the second is not decoration. Pathname
# expansion does not match a leading dot, so `*` alone would pass over a
# dot-prefixed entry in silence -- which would mean that renaming `007_x.c` to
# `.007_x.c` quietly removed its cells from the run while the run still reported
# success, and that a staging file left behind by an interrupted run went unnoticed.
# `tests/conformance/README.md` records the same rule for the harness's own corpus
# discovery: a dot-prefixed entry other than .gitkeep is a hard error, not a skip.
cf_list_area_records() {
	# $1 = area name
	cf_list_area=$1
	for cf_list_entry in "$CF_CORPUS_DIR/$cf_list_area"/* "$CF_CORPUS_DIR/$cf_list_area"/.*; do
		# An unmatched pattern expands to itself, which means the directory holds
		# nothing of that shape. An empty area is a defect in the corpus, and the
		# caller reports it from the empty list rather than being told here.
		if [ ! -e "$cf_list_entry" ] && [ ! -L "$cf_list_entry" ]; then
			continue
		fi
		cf_list_name=${cf_list_entry##*/}
		case $cf_list_name in
		. | ..) continue ;;
		"$CF_KEPT_DOT_ENTRY") continue ;;
		.*)
			cf_reject_area_entry "$cf_list_area" "$cf_list_name" \
				"it is a dot-prefixed entry, and only $CF_KEPT_DOT_ENTRY is allowed to be one"
			;;
		esac
		if [ -L "$cf_list_entry" ]; then
			cf_reject_area_entry "$cf_list_area" "$cf_list_name" 'it is a symbolic link'
		fi
		if [ -d "$cf_list_entry" ]; then
			cf_reject_area_entry "$cf_list_area" "$cf_list_name" \
				'it is a nested directory, and records live only as direct children of an area'
		fi
		if [ ! -f "$cf_list_entry" ]; then
			cf_reject_area_entry "$cf_list_area" "$cf_list_name" 'it is not a regular file'
		fi
		case $cf_list_name in
		*.expected)
			if ! cf_looks_like_program "${cf_list_name%.expected}"; then
				cf_reject_area_entry "$cf_list_area" "$cf_list_name" \
					'its name is not NNN_<name>.expected in lowercase'
			fi
			printf '%s\n' "$cf_list_entry"
			;;
		*.c)
			if ! cf_looks_like_program "${cf_list_name%.c}"; then
				cf_reject_area_entry "$cf_list_area" "$cf_list_name" \
					'its name is not NNN_<name>.c in lowercase'
			fi
			;;
		*)
			cf_reject_area_entry "$cf_list_area" "$cf_list_name" \
				'a feature area holds only .c and .expected files'
			;;
		esac
	done
}

# List every record of the whole corpus, classifying every top-level entry.
cf_list_corpus_records() {
	for cf_list_top in "$CF_CORPUS_DIR"/* "$CF_CORPUS_DIR"/.*; do
		if [ ! -e "$cf_list_top" ] && [ ! -L "$cf_list_top" ]; then
			continue
		fi
		cf_list_topname=${cf_list_top##*/}
		case $cf_list_topname in
		. | ..) continue ;;
		"$CF_KEPT_DOT_ENTRY") continue ;;
		esac
		# This script's own lock and quarantine entries are skipped rather than
		# refused, and the reason is that enumeration happens BEFORE the lock is
		# acquired: refusing them here would make a lock left by a killed run
		# unreclaimable, which is the opposite of what the reclaim exists for. Both
		# are ignored by .gitignore, so neither can be committed either.
		if cf_is_regen_state_entry "$cf_list_topname"; then
			continue
		fi
		case $cf_list_topname in
		.*)
			cf_reject_corpus_entry "$cf_list_topname" \
				"it is a dot-prefixed entry, and only $CF_KEPT_DOT_ENTRY is allowed to be one"
			;;
		esac
		if cf_is_excluded_dir "$cf_list_topname"; then
			# findings/, support/ and tools/ are never walked for records. A
			# curated finding's reproducer.expected IS a valid record, but it is a
			# human-promoted artifact and regenerating it would rewrite captured
			# evidence.
			continue
		fi
		if cf_is_corpus_document "$cf_list_topname"; then
			continue
		fi
		if [ -L "$cf_list_top" ]; then
			cf_reject_corpus_entry "$cf_list_topname" 'it is a symbolic link'
		fi
		if [ ! -d "$cf_list_top" ]; then
			cf_reject_corpus_entry "$cf_list_topname" \
				'only the three Markdown companions are files at the corpus root'
		fi
		if ! cf_looks_like_area "$cf_list_topname"; then
			cf_reject_corpus_entry "$cf_list_topname" \
				'it is a directory whose name is not NN_<name> in lowercase'
		fi
		cf_require_area_dir "$cf_list_topname" 'record'
		cf_list_area_records "$cf_list_topname"
	done
}

# Sort $1 in place, as a command of its own whose status is checked. Deliberately
# not a pipeline: a pipeline's status is its last command's, which is precisely how
# an enumeration failure would pass for a smaller corpus.
cf_sort_in_place() {
	if ! LC_ALL=C sort -- "$1" > "$1.sorted"; then
		cf_error "cannot sort the record list at \"$1\""
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	if ! mv -- "$1.sorted" "$1"; then
		cf_error "cannot install the sorted record list at \"$1\""
		exit "$CF_EXIT_ENVIRONMENT"
	fi
}

CF_RECORD_LIST="$CF_WORK/records.list"

if [ -n "$CF_SELECT_PROGRAM" ]; then
	cf_require_area_dir "$CF_SELECT_AREA" 'usage'
	CF_ONE_RECORD="$CF_CORPUS_DIR/$CF_SELECT_AREA/$CF_SELECT_PROGRAM.expected"
	if [ ! -f "$CF_ONE_RECORD" ] || [ -L "$CF_ONE_RECORD" ]; then
		cf_error "no such expectation record: $CF_SELECT_AREA/$CF_SELECT_PROGRAM.expected"
		cf_detail "looked for: $CF_ONE_RECORD"
		cf_detail 'a symbolic link is refused rather than followed'
		exit "$CF_EXIT_USAGE"
	fi
	printf '%s\n' "$CF_ONE_RECORD" > "$CF_RECORD_LIST"
elif [ -n "$CF_SELECT_AREA" ]; then
	cf_require_area_dir "$CF_SELECT_AREA" 'usage'
	cf_list_area_records "$CF_SELECT_AREA" > "$CF_RECORD_LIST"
	cf_sort_in_place "$CF_RECORD_LIST"
	if [ ! -s "$CF_RECORD_LIST" ]; then
		cf_error "area $CF_SELECT_AREA holds no *.expected record"
		exit "$CF_EXIT_USAGE"
	fi
else
	cf_list_corpus_records > "$CF_RECORD_LIST"
	cf_sort_in_place "$CF_RECORD_LIST"
	if [ ! -s "$CF_RECORD_LIST" ]; then
		cf_error "no *.expected record found beneath $CF_CORPUS_DIR"
		exit "$CF_EXIT_RECORD"
	fi
fi

# --- The selection is bounded ------------------------------------------------
# Counted after selection rather than during enumeration, so the ceiling applies
# however the records were chosen. A corpus that has become something else is
# refused here rather than compiled cell by cell for as long as it takes.
CF_RECORD_COUNT=$(awk 'END { print NR + 0 }' < "$CF_RECORD_LIST")
if [ "$CF_RECORD_COUNT" -gt "$CF_RECORDS_MAX" ]; then
	cf_error "the selection holds $CF_RECORD_COUNT records, past the $CF_RECORDS_MAX-record ceiling"
	cf_detail "corpus: $CF_CORPUS_DIR"
	cf_detail 'the committed corpus holds 108; a selection this large means this is not that corpus'
	exit "$CF_EXIT_RECORD"
fi

# =============================================================================
# The corpus regeneration lock.
#
# One WRITING run at a time. Two concurrent sweeps would each compile every
# declared cell and would then rename records independently, so the corpus could
# end up holding a mixture from both -- and each record would still be internally
# consistent, which is exactly what makes the mixture hard to notice afterwards.
#
# `mkdir` is the primitive because it is atomic: one operation either creates the
# directory or fails because it already exists, with no window between the test
# and the creation. But atomic creation alone is not a lock protocol, and these are
# the three places where two writers would otherwise both proceed:
#
#   * A LOCK BEING TAKEN IS NOT A STALE LOCK. The winner of `mkdir` writes its owner
#     file a moment later, so a contender that read the directory in between saw no
#     owner -- and treating "no owner" as "abandoned" let it declare a LIVE lock
#     stale and proceed. A missing or partial owner file is therefore read as
#     INITIALISING and retried, bounded, and only a complete owner naming a dead
#     process is stale.
#   * RECLAIMING MUST BE A COMPETITION SOMEBODY LOSES. Overwriting the owner file of
#     a stale lock in place is not exclusive: two contenders can both do it and both
#     conclude they hold it. So a stale lock is QUARANTINED with a single `mv`, which
#     only one contender can win because `mv` needs its source to exist, and the
#     winner then competes for a fresh `mkdir` like anyone else.
#   * RELEASING MUST PROVE OWNERSHIP. A run whose lock has been legitimately
#     reclaimed no longer owns the directory at that path, and removing it would hand
#     the corpus to two writers. Every lock therefore carries a TOKEN, and cleanup
#     removes the lock only while the token on disk is this run's.
#
# A symbolic link at the lock path is refused outright rather than followed: `mkdir`
# on a link to an existing directory fails, so the run would otherwise read whatever
# owner file the link's target happens to hold.
#
# --check takes no lock at all: it writes nothing inside the corpus, so it cannot
# race with anything, and making it wait would be a bound with no purpose.
#
# Both the lock and its quarantine live at the corpus root under names `.gitignore`
# covers, so neither ever dirties `git status` and neither can be committed.
# =============================================================================

CF_LOCK_DIR_NAME="$CF_CORPUS_DIR/$CF_LOCK_ENTRY"
CF_LOCK_DIR="$CF_CORPUS_HANDLE/$CF_LOCK_ENTRY"

# How many times acquisition retries while another run is initialising its lock, at
# one second each. Fifty seconds is far longer than the microseconds between a
# `mkdir` and the write that follows it, and finite -- so a genuinely wedged lock
# still reports rather than spinning for ever.
CF_LOCK_ATTEMPTS_MAX=50

# Read one `name=value` field out of the lock's owner file, or print nothing.
#
# An ABSENT owner file is an ordinary answer here rather than an error, and getting
# that wrong is not hypothetical: a lock being taken has no owner file yet, and a
# lock being reclaimed loses one, so both of the cases the protocol above exists to
# distinguish reach this function with nothing to read. The redirection is performed
# inside a subshell so that its failure cannot terminate a run under `set -e`, and
# the diagnostic the shell would print about it goes nowhere -- the caller decides
# what an empty answer means, which is the whole point of returning one.
cf_lock_owner_field() {
	[ -f "$CF_LOCK_DIR/owner" ] || return 0
	(awk -v want="$1" -F '=' '$1 == want { print $2; exit }' < "$CF_LOCK_DIR/owner") 2> /dev/null ||
		true
}

# Write this run's ownership into a lock this run has just created.
#
# The token is what makes the lock identifiable rather than merely present: it is
# derived from the unpredictable name `mktemp -d` gave the private working area, so no
# other run can produce it, and it is written together with the pid and the working
# area that make staleness decidable.
cf_lock_claim() {
	CF_LOCK_TOKEN="${CF_WORK_NAME##*/}.$$"
	printf 'token=%s\npid=%s\nwork=%s\n' "$CF_LOCK_TOKEN" "$$" "$CF_WORK" > "$CF_LOCK_DIR/owner"
	CF_LOCK_HELD=1
}

# Move a stale lock out of the way, atomically, returning success only to the one
# contender that actually did it.
#
# `mktemp -d` gives an unpredictable destination that cannot be planted in advance,
# and `mv` requires its SOURCE to exist -- which is what makes this exclusive: of two
# contenders reaching this point together, the second finds nothing to move and loses.
# The quarantined lock is then discarded, so nothing accumulates.
cf_lock_quarantine_stale() {
	cf_lock_pen=$(mktemp -d -- "$CF_CORPUS_HANDLE/$CF_LOCK_STALE_PREFIX""XXXXXX" 2> /dev/null) || return 1
	if mv -- "$CF_LOCK_DIR" "$cf_lock_pen/lock" 2> /dev/null; then
		rm -rf -- "$cf_lock_pen"
		return 0
	fi
	rm -rf -- "$cf_lock_pen"
	return 1
}

cf_acquire_lock() {
	if [ -L "$CF_LOCK_DIR" ]; then
		cf_error "the regeneration lock path is a symbolic link: $CF_LOCK_DIR_NAME"
		cf_detail 'a link there would make this run read another directory'"'"'s owner file and could'
		cf_detail 'let two sweeps believe they each hold the lock; remove it and try again'
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	cf_lock_attempt=0
	while [ "$cf_lock_attempt" -lt "$CF_LOCK_ATTEMPTS_MAX" ]; do
		cf_lock_attempt=$((cf_lock_attempt + 1))
		if mkdir -- "$CF_LOCK_DIR" 2> /dev/null; then
			cf_lock_claim
			return 0
		fi
		cf_lock_token=$(cf_lock_owner_field token)
		cf_lock_pid=$(cf_lock_owner_field pid)
		cf_lock_work=$(cf_lock_owner_field work)
		case $cf_lock_pid in
		'' | *[!0-9]*) cf_lock_pid='' ;;
		esac
		if [ -z "$cf_lock_token" ] || [ -z "$cf_lock_pid" ] || [ -z "$cf_lock_work" ]; then
			# INITIALISING, not stale. Wait and look again rather than concluding
			# anything: the run that won `mkdir` writes this file immediately
			# afterwards, and mistaking that gap for an abandoned lock is exactly how
			# two writers both proceeded.
			sleep 1
			continue
		fi
		if kill -0 "$cf_lock_pid" 2> /dev/null && [ -d "$cf_lock_work" ]; then
			cf_error "another regeneration is already running (process $cf_lock_pid)"
			cf_detail "lock: $CF_LOCK_DIR_NAME"
			cf_detail 'two sweeps would each rename records independently, so the corpus could end up'
			cf_detail 'holding a mixture from both runs'
			cf_detail 'wait for it to finish, or use --check, which takes no lock and writes nothing'
			exit "$CF_EXIT_ENVIRONMENT"
		fi
		# STALE: a complete owner file naming a process that is gone, or whose private
		# working area is gone. Quarantine it and go round again -- the fresh `mkdir` on
		# the next iteration is what decides the winner, so no contender ever adopts a
		# lock it did not create.
		if cf_lock_quarantine_stale; then
			cf_note "reclaimed a stale lock at $CF_LOCK_DIR_NAME (owner $cf_lock_pid is gone)"
		fi
	done
	cf_error "cannot take the regeneration lock at $CF_LOCK_DIR_NAME after $CF_LOCK_ATTEMPTS_MAX attempt(s)"
	cf_detail 'the lock exists but names no complete owner, so it is either being taken right now or'
	cf_detail 'was left half-created; nothing was compiled and nothing was written'
	cf_detail 'if no other regeneration is running, remove the directory and try again'
	exit "$CF_EXIT_ENVIRONMENT"
}

if [ "$CF_CHECK" -eq 1 ]; then
	cf_note 'no lock is taken in --check mode, which writes nothing inside the corpus'
else
	cf_acquire_lock
fi

# =============================================================================
# Reading a record.
#
# The reader mirrors tests/conformance_harness/manifest.rs exactly, in its order
# of decisions, so this script can never disagree with the parser about what a
# line is:
#
#   1. inside a heredoc, a line equal to exactly "END" closes it and every other
#      line is body -- nothing is trimmed, "#" is data and "=" is not a
#      separator;
#   2. outside a heredoc, a line whose trimmed form is empty or starts with "#"
#      is blank or comment;
#   3. otherwise the line opens a heredoc when "<<" appears and either there is
#      no "=" or the "<<" comes first -- so a scalar value may contain "<<" and
#      an opener needs no "=";
#   4. otherwise it is a scalar, split on the FIRST "=", both sides trimmed.
#
# Tracking heredoc state in the reader is what stops a line inside a heredoc
# body that merely LOOKS like `targets = ...` from shadowing the real key.
# =============================================================================

# Scan a record ONCE and print one line per logical entry, plus one line per
# defect. Tab-separated, five fields always, so the caller reads every line the
# same way:
#
#   K <TAB> S|H <TAB> <line> <TAB> <key>  <TAB> <scalar value, or heredoc line count>
#   ! <TAB> X   <TAB> <line> <TAB> -      <TAB> <what is wrong with that line>
#
# Exit status 0 when no defect was reported, 1 when at least one was, and anything
# else means awk itself failed -- three outcomes the caller keeps apart, so a
# broken environment is never mistaken for a defective record.
#
# The four parser ceilings are enforced here, on the way IN, because a record this
# script accepted and rewrote must still parse afterwards.
cf_scan_record() {
	awk \
		-v linemax="$CF_LINE_BYTES_MAX" \
		-v hlinesmax="$CF_HEREDOC_LINES_MAX" \
		-v hbytesmax="$CF_FIELD_BYTES_MAX" \
		-v recmax="$CF_RECORD_BYTES_MAX" '
	function fault(line, message) {
		printf "!\tX\t%d\t-\t%s\n", line, message
		bad = 1
	}
	BEGIN { state = 0; total = 0; hkey = ""; hline = 0; hlines = 0; hbytes = 0; bad = 0 }
	{
		len = length($0)
		total += len + 1
		if (len > linemax) {
			fault(NR, sprintf("this line is %d bytes, above the %d-byte limit the record parser accepts", len, linemax))
			exit 1
		}
		if (state == 1) {
			if ($0 == "END") {
				printf "K\tH\t%d\t%s\t%d\n", hline, hkey, hlines
				if (hlines > hlinesmax) {
					fault(hline, sprintf("the heredoc body of %s is %d lines, above the %d-line limit", hkey, hlines, hlinesmax))
				}
				if (hbytes > hbytesmax) {
					fault(hline, sprintf("the heredoc body of %s is %d bytes, above the %d-byte limit", hkey, hbytes, hbytesmax))
				}
				state = 0
				next
			}
			hlines++
			hbytes += len + 1
			next
		}
		trimmed = $0
		sub(/^[ \t]+/, "", trimmed)
		sub(/[ \t]+$/, "", trimmed)
		if (trimmed == "" || substr(trimmed, 1, 1) == "#") { next }
		opener = index($0, "<<")
		equals = index($0, "=")
		if (opener > 0 && (equals == 0 || opener < equals)) {
			key = substr($0, 1, opener - 1)
			sub(/^[ \t]+/, "", key)
			sub(/[ \t]+$/, "", key)
			terminator = substr($0, opener + 2)
			sub(/^[ \t]+/, "", terminator)
			sub(/[ \t]+$/, "", terminator)
			if (key == "") {
				fault(NR, "a heredoc opener has no key before its <<")
			}
			if (terminator != "END") {
				fault(NR, sprintf("the heredoc opener for %s names the terminator \"%s\"; the one accepted terminator is END", key, terminator))
			}
			hkey = key
			hline = NR
			hlines = 0
			hbytes = 0
			state = 1
			next
		}
		if (equals == 0) {
			fault(NR, "this line is malformed: it is neither blank, a comment, a scalar (key = value) nor a heredoc opener (key <<END)")
			next
		}
		key = substr($0, 1, equals - 1)
		value = substr($0, equals + 1)
		sub(/^[ \t]+/, "", key)
		sub(/[ \t]+$/, "", key)
		sub(/^[ \t]+/, "", value)
		sub(/[ \t]+$/, "", value)
		if (key == "") {
			fault(NR, "a scalar line has no key before its =")
			next
		}
		printf "K\tS\t%d\t%s\t%s\n", NR, key, value
	}
	END {
		if (state == 1) {
			fault(hline, sprintf("the record ends inside the UNTERMINATED heredoc opened for %s; every body is closed by a line containing exactly END", hkey))
		}
		if (total > recmax) {
			fault(0, sprintf("the record is %d bytes, above the %d-byte limit", total, recmax))
		}
		if (bad) { exit 1 }
	}
	' < "$1"
}

# =============================================================================
# The closed key set, mirrored from tests/conformance_harness/manifest.rs.
#
# manifest.rs declares exactly 25 keys, each with a KIND (scalar or heredoc) and a
# PRESENCE (required, optional or conditional), and treats an unknown key, a
# duplicate key, a key written in the wrong kind and a missing required key as hard
# parse errors. Every one of those is a record the harness REFUSES -- so a record
# this script rewrote and reported as fine would be a record that then fails the
# very test the rewrite was for. Keeping the two in step is the whole point:
# whatever this script accepts, the parser must accept too.
#
# Keep this table and manifest.rs's KEYS array in step.
# =============================================================================

# Print S or H for key $1; exit status 1 when the key is not one of the 25.
cf_key_kind() {
	case $1 in
	program | area | description | targets | opt_levels | shared_flags | \
		bcc_command | ref_command | run_command | expect_exit | \
		oracle_a | oracle_b | oracle_c | ub_audit_flags | \
		ub_audit_source_suppressions | \
		expected_divergence.id | expected_divergence.class | \
		expected_divergence.scope | expected_divergence.basis)
		printf 'S\n'
		;;
	ub_notes | impl_defined_notes | expected_stdout | \
		expected_divergence.documented | expected_divergence.evidence | \
		expected_divergence.observed)
		printf 'H\n'
		;;
	*)
		return 1
		;;
	esac
}

# Warnings a program may suppress IN ITS SOURCE with diagnostic-control pragmas,
# as registered by ub_audit_source_suppressions.
#
# Mirrors UB_AUDIT_SOURCE_SUPPRESSIONS_SANCTIONED in
# tests/conformance_harness/mod.rs, which sanctions each warning for ONE named
# area and program rather than corpus-wide. Keep these in step: a record this
# script accepts and the harness refuses is exactly the defect writing a record
# through the parser's own rules exists to prevent. One entry per line, as
# `<area> <program> <warning>`.
CF_SANCTIONED_SOURCE_SUPPRESSIONS='03_initializers 004_designated_array -Woverride-init'

# The 15 keys every record must carry.
CF_REQUIRED_KEYS='program area description targets opt_levels shared_flags
bcc_command ref_command run_command expect_exit oracle_a oracle_b oracle_c
ub_notes expected_stdout'

# The five marker keys that stand or fall together, and the two that enrich them.
CF_MARKER_REQUIRED_KEYS='expected_divergence.id expected_divergence.class
expected_divergence.scope expected_divergence.basis expected_divergence.observed'
CF_MARKER_OPTIONAL_KEYS='expected_divergence.documented expected_divergence.evidence'
# The same five, spelled for a message rather than for iteration.
CF_MARKER_REQUIRED_SHOWN='expected_divergence.{id,class,scope,basis,observed}'

# The three marker keys whose VALUE has a grammar, named individually because the
# diagnostics quote them.
CF_MARKER_ID_KEY='expected_divergence.id'
CF_MARKER_CLASS_KEY='expected_divergence.class'
CF_MARKER_SCOPE_KEY='expected_divergence.scope'
CF_MARKER_BASIS_KEY='expected_divergence.basis'
CF_MARKER_OBSERVED_KEY='expected_divergence.observed'

# The tokens a command template is required to lead with. Mirrors PLACEHOLDER_BCC,
# PLACEHOLDER_REFERENCE_TEMPLATED and PLACEHOLDER_REFERENCE_BARE in manifest.rs.
# Written with an escaped dollar inside double quotes: these are LITERAL text that
# must reach the comparison unexpanded.
CF_PLACEHOLDER_BCC="\$BCC"
CF_PLACEHOLDER_REF_TEMPLATED="\$REF_CC_<TRIPLE>"
CF_PLACEHOLDER_REF_BARE="\$REF_CC"

# The flags a record may name in shared_flags, and the two a command template owns.
# Mirrors RECORD_SHARED_FLAGS_PERMITTED, MANDATORY_SHARED_FLAG and
# TEMPLATE_OWNED_SHARED_FLAGS in manifest.rs.
CF_SHARED_FLAGS_PERMITTED='-O0 -O1 -O2 -g -static -fPIC'
CF_SHARED_FLAG_MANDATORY='-static'
CF_TEMPLATE_OWNED_FLAGS='-o -c'

# The two accepted values of an oracle switch.
CF_TOGGLE_ENABLED='enabled'
CF_TOGGLE_DISABLED='disabled'

# A tab, for reading the scan's output field by field.
CF_TAB=$(printf '\t')

# Where the validator keeps its per-record state. Files rather than variables
# named after keys, because building a variable name from a key would mean
# evaluating a string taken from the record -- and this script uses no `eval`.
CF_VAL_DIR=''

# The file name a key's value and presence are recorded under. The keys are the 25
# known strings by the time this is called, so the mapping is total and collision
# free; the dot becomes an underscore only because a dot in a name reads badly.
cf_key_slot() {
	printf '%s' "$1" | tr '.' '_'
}

# True when key $1 appeared in the record.
cf_val_seen() {
	[ -e "$CF_VAL_DIR/seen/$(cf_key_slot "$1")" ]
}

# Print the recorded value of key $1: the scalar value, or a heredoc's line count.
cf_val_get() {
	cat -- "$CF_VAL_DIR/val/$(cf_key_slot "$1")"
}

# Report one record defect. Collected rather than fatal on the spot, so a
# maintainer sees everything wrong with a record in one run instead of fixing one
# fault per invocation.
CF_VAL_FAULTS=0
cf_val_fault() {
	# $1 = record label, $2 = line number (0 = the record as a whole), $3 = message
	if [ "$2" -gt 0 ]; then
		cf_error "$1 line $2: $3"
	else
		cf_error "$1: $3"
	fi
	CF_VAL_FAULTS=$((CF_VAL_FAULTS + 1))
}

# Collapse a command template to single-space-separated tokens, so the adjacency
# checks below can be written as plain patterns and a template that merely spaces
# its tokens differently is not refused for it.
cf_normalize_template() {
	printf '%s' "$1" | awk '{
		out = ""
		for (i = 1; i <= NF; i++) { out = (out == "" ? $i : out " " $i) }
		printf "%s", out
	}'
}

# Validate a command template. $1 = record label, $2 = key, $3 = the value,
# $4 = the token that must lead it, $5 = a second acceptable leading token or
# empty, $6 = a space-separated list of single tokens it must contain, $7 = a
# two-token sequence it must contain in that order, or empty.
cf_validate_template() {
	cf_tmpl_label=$1
	cf_tmpl_key=$2
	cf_tmpl_norm=$(cf_normalize_template "$3")
	cf_tmpl_lead=$4
	cf_tmpl_lead_alt=$5
	cf_tmpl_needs=$6
	cf_tmpl_pair=$7

	if [ -z "$cf_tmpl_norm" ]; then
		cf_val_fault "$cf_tmpl_label" 0 "$cf_tmpl_key is empty; it must be the command line that builds or runs a cell"
		return 0
	fi
	cf_tmpl_first=${cf_tmpl_norm%% *}
	if [ "$cf_tmpl_first" != "$cf_tmpl_lead" ] &&
		{ [ -z "$cf_tmpl_lead_alt" ] || [ "$cf_tmpl_first" != "$cf_tmpl_lead_alt" ]; }; then
		cf_val_fault "$cf_tmpl_label" 0 "$cf_tmpl_key begins with \"$cf_tmpl_first\"; the compiler is required POSITIONALLY as the first token, spelled $cf_tmpl_lead${cf_tmpl_lead_alt:+ or $cf_tmpl_lead_alt}"
	fi
	for cf_tmpl_need in $cf_tmpl_needs; do
		case " $cf_tmpl_norm " in
		*" $cf_tmpl_need "*) ;;
		*)
			cf_val_fault "$cf_tmpl_label" 0 "$cf_tmpl_key omits the required placeholder $cf_tmpl_need; a template missing it could not render the cell it describes"
			;;
		esac
	done
	if [ -n "$cf_tmpl_pair" ]; then
		case " $cf_tmpl_norm " in
		*" $cf_tmpl_pair "*) ;;
		*)
			cf_val_fault "$cf_tmpl_label" 0 "$cf_tmpl_key does not write \"$cf_tmpl_pair\" as adjacent tokens; the output placeholder must follow the flag it belongs to"
			;;
		esac
	fi
}

# Validate a record COMPLETELY, before a single compile is spent on it.
#
#   $1 = record path, $2 = record label, $3 = the file stem, $4 = the area directory
#
# Exits with CF_EXIT_RECORD when anything is wrong. Every value the rest of the
# script needs is left in CF_VAL_DIR, so there is exactly ONE reader of the record
# format in this script and no way for a second one to disagree with it.
cf_validate_record() {
	cf_vr_path=$1
	cf_vr_label=$2
	cf_vr_stem=$3
	cf_vr_area=$4
	CF_VAL_FAULTS=0
	CF_VAL_DIR="$CF_WORK/record"
	rm -rf -- "$CF_VAL_DIR"
	mkdir -p -- "$CF_VAL_DIR/seen" "$CF_VAL_DIR/val"

	# --- Bytes the format cannot carry ---------------------------------------
	# A control byte would survive into a report line and into the rewritten
	# record, where it can move a cursor, hide text or terminate an escape
	# sequence in whatever reads it next. Tab and newline are the two the format
	# uses; every other one is refused. NUL and CR are named separately because
	# each has its own reason and its own remedy.
	if grep -q -- "$CF_CR" < "$cf_vr_path"; then
		cf_val_fault "$cf_vr_label" 0 'the record contains a CARRIAGE RETURN; this line-oriented rewrite requires LF endings, so convert the file first'
	fi
	tr -d '\000-\010\013\014\015\016-\037\177' < "$cf_vr_path" > "$CF_VAL_DIR/printable"
	if ! cmp -s -- "$cf_vr_path" "$CF_VAL_DIR/printable"; then
		cf_val_fault "$cf_vr_label" 0 'the record contains a control byte other than tab and newline; the format is line-oriented text and a control byte would survive into every report that quotes it'
	fi

	# --- The last byte must be a newline -------------------------------------
	# Every line of this record is read and written back a line at a time, and a
	# written line always ends in a newline. So a record whose final line is
	# unterminated would come back one byte LONGER -- a byte changed outside
	# expected_stdout, which is the one thing the rewrite promises never to do.
	# It is refused rather than silently terminated, because appending a byte to a
	# file the maintainer did not ask to have appended to is not this script's
	# decision to make.
	if [ ! -s "$cf_vr_path" ]; then
		cf_val_fault "$cf_vr_label" 0 'the record is empty'
	elif [ -n "$(tail -c 1 < "$cf_vr_path")" ]; then
		cf_val_fault "$cf_vr_label" 0 'the record does not end in a NEWLINE; a rewrite would have to add one, which would change a byte outside expected_stdout -- terminate the final line and try again'
	fi

	# --- One scan, then the closed key table ---------------------------------
	if cf_scan_record "$cf_vr_path" > "$CF_VAL_DIR/entries"; then
		cf_vr_scan=0
	else
		cf_vr_scan=$?
	fi
	if [ "$cf_vr_scan" -gt 1 ]; then
		cf_error "$cf_vr_label: awk failed while scanning the record (status $cf_vr_scan)"
		exit "$CF_EXIT_RECORD"
	fi

	while IFS="$CF_TAB" read -r cf_vr_tag cf_vr_kind cf_vr_line cf_vr_key cf_vr_rest; do
		if [ "$cf_vr_tag" = '!' ]; then
			cf_val_fault "$cf_vr_label" "$cf_vr_line" "$cf_vr_rest"
			continue
		fi
		if ! cf_vr_want=$(cf_key_kind "$cf_vr_key"); then
			cf_val_fault "$cf_vr_label" "$cf_vr_line" "unknown key \"$cf_vr_key\"; the record format has a closed set of keys and an unknown one is almost always a typo the parser would reject"
			continue
		fi
		if [ "$cf_vr_want" != "$cf_vr_kind" ]; then
			if [ "$cf_vr_want" = 'H' ]; then
				cf_val_fault "$cf_vr_label" "$cf_vr_line" "$cf_vr_key is a heredoc field but is written as a scalar; it must read \"$cf_vr_key <<END\" with its body below"
			else
				cf_val_fault "$cf_vr_label" "$cf_vr_line" "$cf_vr_key is a scalar field but is written as a heredoc; it must read \"$cf_vr_key = <value>\""
			fi
			continue
		fi
		cf_vr_slot=$(cf_key_slot "$cf_vr_key")
		if [ -e "$CF_VAL_DIR/seen/$cf_vr_slot" ]; then
			cf_val_fault "$cf_vr_label" "$cf_vr_line" "duplicate key \"$cf_vr_key\"; two values for one key disagree about what the record says, and there is no way to tell which was meant"
			continue
		fi
		: > "$CF_VAL_DIR/seen/$cf_vr_slot"
		printf '%s' "$cf_vr_rest" > "$CF_VAL_DIR/val/$cf_vr_slot"
	done < "$CF_VAL_DIR/entries"

	# --- Presence -------------------------------------------------------------
	for cf_vr_key in $CF_REQUIRED_KEYS; do
		if ! cf_val_seen "$cf_vr_key"; then
			cf_val_fault "$cf_vr_label" 0 "the required key \"$cf_vr_key\" is absent"
		fi
	done

	# A marker is all five keys or none of them: a partial one classifies a real
	# divergence as expected on the strength of an incomplete citation.
	cf_vr_marker=0
	for cf_vr_key in $CF_MARKER_REQUIRED_KEYS $CF_MARKER_OPTIONAL_KEYS; do
		if cf_val_seen "$cf_vr_key"; then
			cf_vr_marker=1
		fi
	done
	if [ "$cf_vr_marker" -eq 1 ]; then
		for cf_vr_key in $CF_MARKER_REQUIRED_KEYS; do
			if ! cf_val_seen "$cf_vr_key"; then
				cf_val_fault "$cf_vr_label" 0 "the expected-divergence marker is partial: \"$cf_vr_key\" is absent, and all five of $CF_MARKER_REQUIRED_SHOWN stand or fall together"
			fi
		done
	fi

	# A gate deviation must carry its reason. manifest.rs searches
	# impl_defined_notes for it -- never ub_notes -- so a deviation without that
	# block is a record the parser refuses.
	if cf_val_seen ub_audit_flags && ! cf_val_seen impl_defined_notes; then
		cf_val_fault "$cf_vr_label" 0 'ub_audit_flags deviates from the default warning gate but there is no impl_defined_notes block recording why; a deviation without a recorded reason is a defect in the test'
	fi

	# --- The marker's own grammar ---------------------------------------------
	# A marker is the one mechanism in the suite that turns a failure into a pass, so
	# a malformed one is worse than a missing test: it would report a compiler defect
	# as an expected divergence. The parser refuses each of these, and a record this
	# script rewrote under a marker the parser refuses could not be read back.
	#
	# Cleared before the scope is read, never after: the oracle cross-checks below
	# consult this, and a value surviving from the previous record would judge this
	# one against a scope it does not carry.
	CF_SCOPE_ORACLES=''
	if cf_val_seen "$CF_MARKER_ID_KEY"; then
		cf_vr_marker_id=$(cf_val_get "$CF_MARKER_ID_KEY")
		if [ -z "$cf_vr_marker_id" ]; then
			cf_val_fault "$cf_vr_label" 0 "$CF_MARKER_ID_KEY is empty; the identifier is what the register cross-check matches, so a marker without one cannot be audited in either direction"
		elif [ "$(printf '%s' "$cf_vr_marker_id" | awk '{ print NF }')" != '1' ]; then
			cf_val_fault "$cf_vr_label" 0 "$CF_MARKER_ID_KEY = \"$cf_vr_marker_id\" is not a single token; the register cross-check matches it exactly, so it may not contain whitespace"
		fi
	fi
	if cf_val_seen "$CF_MARKER_CLASS_KEY"; then
		cf_vr_marker_class=$(cf_val_get "$CF_MARKER_CLASS_KEY")
		if ! cf_list_has "$CF_MARKER_CLASSES" "$cf_vr_marker_class"; then
			cf_val_fault "$cf_vr_label" 0 "$CF_MARKER_CLASS_KEY = \"$cf_vr_marker_class\" is not a divergence class; the classifier's mapping is total over $(printf '%s' "$CF_MARKER_CLASSES" | tr '\n' ' ') and a class outside it could never match a cell"
		fi
	fi
	if cf_val_seen "$CF_MARKER_SCOPE_KEY"; then
		cf_validate_scope "$cf_vr_label" "$(cf_val_get "$CF_MARKER_SCOPE_KEY")"
	fi
	if cf_val_seen "$CF_MARKER_BASIS_KEY"; then
		cf_validate_basis "$cf_vr_label" "$(cf_val_get "$CF_MARKER_BASIS_KEY")"
	fi
	if cf_val_seen "$CF_MARKER_OBSERVED_KEY" &&
		[ "$(cf_val_get "$CF_MARKER_OBSERVED_KEY")" -eq 0 ]; then
		cf_val_fault "$cf_vr_label" 0 "$CF_MARKER_OBSERVED_KEY has an empty body; the observation is how a reader tells whether what they are seeing is what was marked, so a marker without one describes nothing"
	fi

	# --- Values ---------------------------------------------------------------
	if cf_val_seen program && [ "$(cf_val_get program)" != "$cf_vr_stem" ]; then
		cf_val_fault "$cf_vr_label" 0 "program = \"$(cf_val_get program)\" but the file stem is \"$cf_vr_stem\"; the cheapest guard there is against a record copied from another program and only partly edited"
	fi
	if cf_val_seen area && [ "$(cf_val_get area)" != "$cf_vr_area" ]; then
		cf_val_fault "$cf_vr_label" 0 "area = \"$(cf_val_get area)\" but the containing directory is \"$cf_vr_area\""
	fi
	if cf_val_seen description && [ -z "$(cf_val_get description)" ]; then
		cf_val_fault "$cf_vr_label" 0 'description is empty'
	fi
	if cf_val_seen ub_notes && [ "$(cf_val_get ub_notes)" -eq 0 ]; then
		cf_val_fault "$cf_vr_label" 0 'ub_notes has an empty body; it carries the written argument for why the program is free of undefined behaviour, which is what makes a divergence attributable at all'
	fi
	if cf_val_seen ub_audit_flags; then
		if [ -z "$(cf_val_get ub_audit_flags)" ]; then
			cf_val_fault "$cf_vr_label" 0 'ub_audit_flags is present but empty; omit the key to accept the default gate rather than declaring an empty one'
		else
			# The gate is the one requirement-1 mechanism a record can narrow, so the
			# narrowing itself is checked here rather than trusted. Only the two
			# sanctioned reductions are accepted, and -pedantic only in the extension
			# area: a record naming any other gate is one the parser refuses, so
			# rewriting it would produce a file the harness could no longer read.
			cf_validate_gate "$cf_vr_label" "$(cf_val_get ub_audit_flags)" "$cf_vr_area"
		fi
	fi

	# A source-level suppression is registered, not merely mentioned. The parser
	# validates the value whenever the key is present -- the condition that makes it
	# required is a property of the SOURCE, which no record parser can see -- so the
	# same three checks it applies are applied here: the key names at least one
	# warning, every item is a `-W` warning name, and every item is sanctioned for
	# this exact area and program.
	if cf_val_seen ub_audit_source_suppressions; then
		cf_vr_supp=$(cf_val_get ub_audit_source_suppressions)
		if [ -z "$cf_vr_supp" ]; then
			cf_val_fault "$cf_vr_label" 0 'ub_audit_source_suppressions is present but names no warning; a program that suppresses nothing in its source omits the key, because an empty registration would satisfy a presence check while recording no exception at all'
		else
			for cf_vr_warning in $cf_vr_supp; do
				case $cf_vr_warning in
				-W?*) ;;
				*)
					cf_val_fault "$cf_vr_label" 0 "ub_audit_source_suppressions names \"$cf_vr_warning\", which is not a -W warning name"
					continue
					;;
				esac
				cf_vr_sanctioned=0
				while IFS= read -r cf_vr_sanction; do
					[ -n "$cf_vr_sanction" ] || continue
					if [ "$cf_vr_sanction" = "$cf_vr_area $cf_vr_stem $cf_vr_warning" ]; then
						cf_vr_sanctioned=1
						break
					fi
				done <<SANCTIONS
$CF_SANCTIONED_SOURCE_SUPPRESSIONS
SANCTIONS
				if [ "$cf_vr_sanctioned" -ne 1 ]; then
					cf_val_fault "$cf_vr_label" 0 "ub_audit_source_suppressions registers \"$cf_vr_warning\", which is not sanctioned for $cf_vr_area/$cf_vr_stem; each warning is sanctioned for one named program only, in UB_AUDIT_SOURCE_SUPPRESSIONS_SANCTIONED"
				fi
			done
		fi
	fi

	cf_vr_disabled=''
	for cf_vr_key in oracle_a oracle_b oracle_c; do
		if ! cf_val_seen "$cf_vr_key"; then
			continue
		fi
		cf_vr_value=$(cf_val_get "$cf_vr_key")
		if [ "$cf_vr_value" != "$CF_TOGGLE_ENABLED" ] &&
			[ "$cf_vr_value" != "$CF_TOGGLE_DISABLED" ]; then
			cf_val_fault "$cf_vr_label" 0 "$cf_vr_key = \"$cf_vr_value\"; an oracle switch is $CF_TOGGLE_ENABLED or $CF_TOGGLE_DISABLED and nothing else"
		elif [ "$cf_vr_value" = "$CF_TOGGLE_DISABLED" ]; then
			cf_vr_disabled="$cf_vr_disabled ${cf_vr_key#oracle_}"
		fi
	done

	# --- A disabled oracle and its marker, checked in both directions ---------
	# Requirement 5 forbids silently excluding a feature from testing, and a
	# disabled oracle IS an exclusion. The parser therefore refuses a narrowing
	# without a marker that scopes it, and refuses a marker that claims
	# comparison_excluded while scoping an oracle that still runs. Both directions
	# are enforced here, because a record that is merely internally consistent can
	# still hide an exclusion, and a record this script rewrote past the parser's
	# refusal would be unreadable by the harness that has to run it.
	#
	# oracle_c may never be disabled and at least one of oracle_a/oracle_b must
	# remain enabled -- both are checked by require_judgeable_oracles in
	# manifest.rs, and both are checked here too, because a record with no judging
	# oracle left would compile and run 12 cells and conclude nothing from any of
	# them.
	case " $cf_vr_disabled " in
	*' c '*)
		cf_val_fault "$cf_vr_label" 0 'oracle_c is disabled; the golden record is the one oracle that detects both compilers changing together, and it costs nothing to run, so it may never be switched off'
		;;
	esac
	if [ -n "$cf_vr_disabled" ]; then
		case " $cf_vr_disabled " in
		*' a '* )
			case " $cf_vr_disabled " in
			*' b '*)
				cf_val_fault "$cf_vr_label" 0 'both oracle_a and oracle_b are disabled; at least one differential oracle must remain, or the record declares 12 cells that no independent authority ever judges'
				;;
			esac
			;;
		esac
		if [ "$cf_vr_marker" -eq 0 ]; then
			cf_val_fault "$cf_vr_label" 0 "the record disables$cf_vr_disabled but carries no expected-divergence marker; a narrowed oracle without a marker is exactly the silent exclusion requirement 5 forbids, so record the reason as $CF_MARKER_REQUIRED_SHOWN"
		else
			for cf_vr_key in $cf_vr_disabled; do
				case " $CF_SCOPE_ORACLES " in
				*" $cf_vr_key "*) ;;
				*)
					cf_val_fault "$cf_vr_label" 0 "oracle_$cf_vr_key is disabled but $CF_MARKER_SCOPE_KEY does not scope it; the marker must name the oracle it excuses, so that the set of comparisons not being made is as visible as the set that is"
					;;
				esac
			done
			if cf_val_seen "$CF_MARKER_CLASS_KEY" &&
				[ "$(cf_val_get "$CF_MARKER_CLASS_KEY")" != "$CF_MARKER_CLASS_EXCLUDED" ]; then
				cf_val_fault "$cf_vr_label" 0 "the record disables$cf_vr_disabled, so $CF_MARKER_CLASS_KEY must be $CF_MARKER_CLASS_EXCLUDED; a disabled oracle makes no comparison at all, so it can never produce the observable divergence any other class describes"
			fi
		fi
	fi
	if [ "$cf_vr_marker" -eq 1 ] && cf_val_seen "$CF_MARKER_CLASS_KEY" &&
		[ "$(cf_val_get "$CF_MARKER_CLASS_KEY")" = "$CF_MARKER_CLASS_EXCLUDED" ]; then
		for cf_vr_key in $CF_SCOPE_ORACLES; do
			case " $cf_vr_disabled " in
			*" $cf_vr_key "*) ;;
			*)
				cf_val_fault "$cf_vr_label" 0 "$CF_MARKER_CLASS_KEY is $CF_MARKER_CLASS_EXCLUDED and $CF_MARKER_SCOPE_KEY names oracle_$cf_vr_key, but oracle_$cf_vr_key is not disabled; excusing an oracle that still runs would turn a real divergence into an expected one"
				;;
			esac
		done
	fi

	if cf_val_seen expect_exit; then
		cf_vr_value=$(cf_val_get expect_exit)
		case $cf_vr_value in
		'' | *[!0-9]*)
			cf_val_fault "$cf_vr_label" 0 "expect_exit = \"$cf_vr_value\" is not a whole number"
			;;
		*)
			if [ "$cf_vr_value" -gt "$CF_EXPECT_EXIT_MAX" ]; then
				cf_val_fault "$cf_vr_label" 0 "expect_exit = $cf_vr_value is above $CF_EXPECT_EXIT_MAX; the operating system truncates a larger value, so it could never be observed"
			fi
			;;
		esac
	fi

	# --- Shared flags ---------------------------------------------------------
	if cf_val_seen shared_flags; then
		cf_vr_value=$(cf_val_get shared_flags)
		if ! cf_split_commas "$cf_vr_value" > "$CF_VAL_DIR/flags"; then
			cf_val_fault "$cf_vr_label" 0 "shared_flags = \"$cf_vr_value\" holds an empty element; a stray comma is a hard error rather than a dropped flag"
			: > "$CF_VAL_DIR/flags"
		fi
		if [ ! -s "$CF_VAL_DIR/flags" ]; then
			cf_val_fault "$cf_vr_label" 0 "shared_flags is empty; every artifact in the corpus is built statically, so the list carries at least $CF_SHARED_FLAG_MANDATORY"
		fi
		cf_vr_static=0
		: > "$CF_VAL_DIR/flags.seen"
		while IFS= read -r cf_vr_flag; do
			if grep -q -x -F -- "$cf_vr_flag" "$CF_VAL_DIR/flags.seen"; then
				cf_val_fault "$cf_vr_label" 0 "shared_flags names \"$cf_vr_flag\" twice; a repeated flag is a partly edited record rather than an intention"
			fi
			printf '%s\n' "$cf_vr_flag" >> "$CF_VAL_DIR/flags.seen"
			if [ "$cf_vr_flag" = "$CF_SHARED_FLAG_MANDATORY" ]; then
				cf_vr_static=1
			fi
			cf_vr_owned=0
			for cf_vr_candidate in $CF_TEMPLATE_OWNED_FLAGS; do
				if [ "$cf_vr_flag" = "$cf_vr_candidate" ]; then
					cf_vr_owned=1
				fi
			done
			if [ "$cf_vr_owned" -eq 1 ]; then
				cf_val_fault "$cf_vr_label" 0 "shared_flags names \"$cf_vr_flag\", which belongs to a command template rather than to the shared argument set: it takes an operand the template supplies"
				continue
			fi
			cf_vr_ok=0
			for cf_vr_candidate in $CF_SHARED_FLAGS_PERMITTED; do
				if [ "$cf_vr_flag" = "$cf_vr_candidate" ]; then
					cf_vr_ok=1
				fi
			done
			if [ "$cf_vr_ok" -eq 0 ]; then
				cf_val_fault "$cf_vr_label" 0 "shared_flags names \"$cf_vr_flag\", which is not one of $CF_SHARED_FLAGS_PERMITTED; only flags both compilers honour with the same meaning may appear in a differential invocation"
			fi
		done < "$CF_VAL_DIR/flags"
		if [ -s "$CF_VAL_DIR/flags" ] && [ "$cf_vr_static" -eq 0 ]; then
			cf_val_fault "$cf_vr_label" 0 "shared_flags omits $CF_SHARED_FLAG_MANDATORY; it is the one linkage mode both compilers spell identically and what lets the emulators run a binary with no sysroot"
		fi
	fi

	# --- Command templates ----------------------------------------------------
	# The three templates are what make a cell reproducible by hand with no
	# harness at all, so a template that could not render its cell is a defect in
	# the record even though it parses.
	if cf_val_seen bcc_command; then
		cf_validate_template "$cf_vr_label" 'bcc_command' "$(cf_val_get bcc_command)" \
			"$CF_PLACEHOLDER_BCC" '' '<triple> <opt> <src> <out>' '-o <out>'
	fi
	if cf_val_seen ref_command; then
		cf_validate_template "$cf_vr_label" 'ref_command' "$(cf_val_get ref_command)" \
			"$CF_PLACEHOLDER_REF_TEMPLATED" "$CF_PLACEHOLDER_REF_BARE" \
			'<opt> <src> <out>' '-o <out>'
	fi
	if cf_val_seen run_command; then
		cf_vr_value=$(cf_normalize_template "$(cf_val_get run_command)")
		if [ "$cf_vr_value" != '<runner> <out>' ]; then
			cf_val_fault "$cf_vr_label" 0 "run_command = \"$cf_vr_value\"; it must be exactly \"<runner> <out>\", because a runner that contributes nothing on the natively executing target has to leave the line valid"
		fi
	fi

	if [ "$CF_VAL_FAULTS" -ne 0 ]; then
		cf_detail "$CF_VAL_FAULTS defect(s) in this record; see tests/conformance/README.md for the format"
		cf_detail 'nothing was compiled and nothing was written: a record the harness would refuse'
		cf_detail 'must never be rewritten and reported as fine'
		exit "$CF_EXIT_RECORD"
	fi
}

# True when whitespace-separated list $1 contains word $2 exactly.
cf_list_has() {
	for cf_lh_item in $1; do
		if [ "$cf_lh_item" = "$2" ]; then
			return 0
		fi
	done
	return 1
}

# Canonicalize a declared warning gate into gate order, so two records expressing the
# same deviation compare equal. Leaves the canonical form in CF_GATE_CANONICAL and
# returns non-zero after faulting on anything the parser would refuse.
#
# The result is returned through a variable rather than printed, and that is not a
# style choice. A fault increments CF_VAL_FAULTS, and a function whose output is read
# with `$(...)` runs in a SUBSHELL, where every such increment is discarded the moment
# it returns -- so a record naming a flag outside the gate was diagnosed and then
# admitted anyway. Reporting a fault and returning a value are two effects, and a
# command substitution can only carry one of them back.
#
# Mirrors parse_ub_audit_flags in tests/conformance_harness/manifest.rs, whose rules
# exist because these flags are passed to a real compiler: a deviation may only ever
# REMOVE a member of the fixed gate, never introduce an option of its own, so no
# plugin, search path, output name or blanket suppression can be named at all.
CF_GATE_CANONICAL=''
cf_canonical_gate() {
	# $1 = label, $2 = the declared value
	CF_GATE_CANONICAL=''
	cf_cg_seen=''
	for cf_cg_flag in $2; do
		if ! cf_list_has "$CF_UB_GATE_DEFAULT" "$cf_cg_flag"; then
			cf_val_fault "$1" 0 "ub_audit_flags names \"$cf_cg_flag\", which is not a member of the default warning gate ($CF_UB_GATE_DEFAULT); a deviation is expressed as a REMOVAL from that fixed gate and never as a compiler invocation of its own, so no option outside it may appear"
			return 1
		fi
		if cf_list_has "$cf_cg_seen" "$cf_cg_flag"; then
			cf_val_fault "$1" 0 "ub_audit_flags names \"$cf_cg_flag\" twice; a repeated diagnostic switch says nothing the single occurrence does not, and a duplicate is a partly edited record rather than an intention"
			return 1
		fi
		cf_cg_seen="$cf_cg_seen $cf_cg_flag"
	done
	cf_cg_canonical=''
	for cf_cg_flag in $CF_UB_GATE_DEFAULT; do
		if cf_list_has "$cf_cg_seen" "$cf_cg_flag"; then
			if [ -z "$cf_cg_canonical" ]; then
				cf_cg_canonical=$cf_cg_flag
			else
				cf_cg_canonical="$cf_cg_canonical $cf_cg_flag"
			fi
		fi
	done
	CF_GATE_CANONICAL=$cf_cg_canonical
	return 0
}

# Validate a declared warning gate as one of the two SANCTIONED reductions, or fault.
#
# "Non-empty" is not the contract. The parser accepts exactly two values -- the
# default gate without the conversion diagnostics, where a narrowing conversion is
# the behaviour under test, and the default gate without -pedantic, in the extension
# area alone, where the subject is non-standard by definition and that diagnostic
# exists precisely to reject it -- and refuses everything else, including the full
# gate spelled out (a deviation that deviates in nothing) and any combination of the
# two removals. A record this script rewrote under a gate the parser refuses is a
# record that then fails the very test the rewrite was for.
cf_validate_gate() {
	# $1 = label, $2 = declared value, $3 = area
	cf_canonical_gate "$1" "$2" || return 0
	cf_vg_canonical=$CF_GATE_CANONICAL
	if ! cf_list_has "$cf_vg_canonical" "$CF_UB_GATE_MANDATORY"; then
		cf_val_fault "$1" 0 "ub_audit_flags drops $CF_UB_GATE_MANDATORY, which it may never drop: without it every remaining diagnostic becomes advice and the gate stops being a gate"
		return 0
	fi
	if [ "$cf_vg_canonical" = "$CF_UB_GATE_DEFAULT" ]; then
		cf_val_fault "$1" 0 "ub_audit_flags names the entire default gate, so it deviates in nothing; a program that passes the full gate omits the key altogether, and recording a no-op deviation would demand a recorded reason for a narrowing that was never made"
		return 0
	fi
	if [ "$cf_vg_canonical" = "$CF_UB_GATE_WITHOUT_CONVERSION" ]; then
		return 0
	fi
	if [ "$cf_vg_canonical" = "$CF_UB_GATE_WITHOUT_PEDANTIC" ]; then
		if [ "$3" != "$CF_UB_GATE_EXTENSION_AREA" ]; then
			cf_val_fault "$1" 0 "ub_audit_flags drops -pedantic, which is sanctioned only in the $CF_UB_GATE_EXTENSION_AREA area, where the subject under test is by definition non-standard; this record is in $3, where a standard-conformance diagnostic is a genuine defect in the test program"
		fi
		return 0
	fi
	cf_val_fault "$1" 0 "ub_audit_flags = \"$2\" is neither sanctioned reduction of the default warning gate; the only accepted values are \"$CF_UB_GATE_WITHOUT_CONVERSION\" (a deliberate narrowing conversion) and \"$CF_UB_GATE_WITHOUT_PEDANTIC\" (the $CF_UB_GATE_EXTENSION_AREA area only), and a program that passes the full gate omits the key"
	return 0
}

# Validate an expected-divergence scope, and set CF_SCOPE_ORACLES to the oracle
# letters it covers.
#
# Mirrors parse_scope in manifest.rs: clauses separated by `;`, none of them empty,
# each either a whole-dimension form or a comma list drawn UNIFORMLY from one
# dimension, and each dimension constrained at most once. An empty clause is refused
# rather than ignored for the reason the parser gives: ignoring it lets a scope that
# constrains nothing fall back to every oracle, every target and every optimization
# level -- the widest scope there is, reached by writing nothing.
cf_validate_scope() {
	# $1 = label, $2 = the declared scope
	CF_SCOPE_ORACLES=''
	cf_vs_oracles_set=0
	cf_vs_targets_set=0
	cf_vs_opts_set=0
	cf_vs_position=0
	cf_vs_rest=$2
	if [ -z "$(printf '%s' "$2" | tr -d ' \t')" ]; then
		cf_val_fault "$1" 0 "$CF_MARKER_SCOPE_KEY is empty; a marker with no scope could not be matched against any cell, so the divergence it describes could never be recognised"
		return 0
	fi
	while :; do
		case $cf_vs_rest in
		*';'*)
			cf_vs_clause=${cf_vs_rest%%;*}
			cf_vs_rest=${cf_vs_rest#*;}
			cf_vs_more=1
			;;
		*)
			cf_vs_clause=$cf_vs_rest
			cf_vs_more=0
			;;
		esac
		cf_vs_position=$((cf_vs_position + 1))
		cf_vs_clause=$(printf '%s' "$cf_vs_clause" | awk '{ $1 = $1; print }')
		if [ -z "$cf_vs_clause" ]; then
			cf_val_fault "$1" 0 "clause $cf_vs_position of $CF_MARKER_SCOPE_KEY is empty, which usually means a doubled, leading or trailing \";\"; an empty clause is refused rather than ignored, because ignoring it lets a scope that constrains nothing fall back to the widest scope there is"
			[ "$cf_vs_more" -eq 1 ] || return 0
			continue
		fi
		cf_vs_lowered=$(printf '%s' "$cf_vs_clause" | tr '[:upper:]' '[:lower:]')
		case $cf_vs_lowered in
		'all oracle' | 'all oracles')
			cf_vs_oracles_set=$((cf_vs_oracles_set + 1))
			CF_SCOPE_ORACLES='a b c'
			;;
		'all target' | 'all targets')
			cf_vs_targets_set=$((cf_vs_targets_set + 1))
			;;
		'all opt level' | 'all opt levels' | 'all opt_level' | 'all opt_levels' | \
			'all optimization level' | 'all optimization levels')
			cf_vs_opts_set=$((cf_vs_opts_set + 1))
			;;
		'all '*)
			cf_val_fault "$1" 0 "$CF_MARKER_SCOPE_KEY clause \"$cf_vs_clause\" names no dimension; the whole-dimension forms are \"all oracles\", \"all targets\" and \"all opt levels\""
			;;
		*)
			cf_validate_scope_clause "$1" "$cf_vs_clause"
			;;
		esac
		[ "$cf_vs_more" -eq 1 ] || break
	done
	for cf_vs_count in "$cf_vs_oracles_set" "$cf_vs_targets_set" "$cf_vs_opts_set"; do
		if [ "$cf_vs_count" -gt 1 ]; then
			cf_val_fault "$1" 0 "$CF_MARKER_SCOPE_KEY constrains one dimension in more than one clause; each of oracles, targets and optimization levels is named at most once, so that the set of cells a marker covers is decidable"
		fi
	done
	# A dimension no clause constrains widens to all of it, which is what a
	# maintainer writing one clause means. parse_scope resolves the oracle
	# dimension the same way, and the resolved set -- never the written one -- is
	# what the class and narrowing cross-checks read, so an omitted oracle clause
	# has to widen HERE or the two would disagree on scopes nobody wrote.
	if [ -z "$CF_SCOPE_ORACLES" ]; then
		CF_SCOPE_ORACLES='a b c'
	fi
	return 0
}

# Validate one comma-list scope clause: every item must parse, and all of them must
# belong to the SAME dimension.
cf_validate_scope_clause() {
	# $1 = label, $2 = the clause
	cf_vsc_kind=''
	cf_vsc_items=0
	cf_vsc_rest=$2
	while :; do
		case $cf_vsc_rest in
		*','*)
			cf_vsc_item=${cf_vsc_rest%%,*}
			cf_vsc_rest=${cf_vsc_rest#*,}
			cf_vsc_more=1
			;;
		*)
			cf_vsc_item=$cf_vsc_rest
			cf_vsc_more=0
			;;
		esac
		cf_vsc_item=$(printf '%s' "$cf_vsc_item" | awk '{ $1 = $1; print }')
		if [ -z "$cf_vsc_item" ]; then
			cf_val_fault "$1" 0 "$CF_MARKER_SCOPE_KEY clause \"$2\" holds an empty element; a stray comma is a hard error rather than a dropped element"
			[ "$cf_vsc_more" -eq 1 ] || return 0
			continue
		fi
		cf_vsc_items=$((cf_vsc_items + 1))
		if cf_list_has "$CF_SCOPE_ORACLE_SPELLINGS" "$cf_vsc_item"; then
			cf_vsc_this='oracle'
			CF_SCOPE_ORACLES="$CF_SCOPE_ORACLES ${cf_vsc_item#oracle_}"
		elif cf_list_has "$CF_SCOPE_TARGET_SPELLINGS" "$cf_vsc_item"; then
			cf_vsc_this='target'
		elif cf_list_has "$CF_SCOPE_OPT_SPELLINGS" "$cf_vsc_item"; then
			cf_vsc_this='optimization level'
		else
			cf_val_fault "$1" 0 "$CF_MARKER_SCOPE_KEY names \"$cf_vsc_item\", which is not an oracle, a target or an optimization level; the accepted spellings are $CF_SCOPE_ORACLE_SPELLINGS for oracles, x86_64/i686/aarch64/riscv64 and their -linux-gnu triples for targets, and -O0/-O1/-O2 for levels"
			[ "$cf_vsc_more" -eq 1 ] || return 0
			continue
		fi
		if [ -z "$cf_vsc_kind" ]; then
			cf_vsc_kind=$cf_vsc_this
			# Count this clause against its dimension, exactly once, the moment the
			# dimension becomes known. cf_validate_scope counts only the whole-dimension
			# forms, so without this a scope could constrain the same dimension twice by
			# writing one clause as a list and the other as "all X" -- and the check that
			# each dimension is named at most once, which is what makes the covered set
			# of cells decidable, would never see it.
			case $cf_vsc_this in
			oracle) cf_vs_oracles_set=$((cf_vs_oracles_set + 1)) ;;
			target) cf_vs_targets_set=$((cf_vs_targets_set + 1)) ;;
			*) cf_vs_opts_set=$((cf_vs_opts_set + 1)) ;;
			esac
		elif [ "$cf_vsc_kind" != "$cf_vsc_this" ]; then
			cf_val_fault "$1" 0 "$CF_MARKER_SCOPE_KEY clause \"$2\" mixes a $cf_vsc_kind with a $cf_vsc_this; one clause constrains one dimension, so write each dimension as its own clause separated by \";\""
			return 0
		fi
		[ "$cf_vsc_more" -eq 1 ] || break
	done
	return 0
}

# Validate an expected-divergence basis: a repository-relative path, a comma, then
# the section that authorises the marker -- and the path must name a file that
# actually exists, because the register audit resolves the citation inside it.
cf_validate_basis() {
	# $1 = label, $2 = the declared basis
	case $2 in
	*','*) ;;
	*)
		cf_val_fault "$1" 0 "$CF_MARKER_BASIS_KEY = \"$2\" is not a basis; write a repository-relative file path, then a comma, then the section or description that authorises the marker"
		return 0
		;;
	esac
	# Split on the FIRST comma only: a citation legitimately contains commas of its
	# own, and the path never does.
	cf_vb_path=$(printf '%s' "${2%%,*}" | awk '{ $1 = $1; print }')
	cf_vb_citation=$(printf '%s' "${2#*,}" | awk '{ $1 = $1; print }')
	if [ -z "$cf_vb_path" ]; then
		cf_val_fault "$1" 0 "$CF_MARKER_BASIS_KEY names no file before its comma; the register audit asserts that the cited document exists, so the citation must begin with a path"
		return 0
	fi
	if [ -z "$cf_vb_citation" ]; then
		cf_val_fault "$1" 0 "$CF_MARKER_BASIS_KEY names a file but cites no section within it; a whole-document citation cannot be checked by a reader, which is the point of recording it"
		return 0
	fi
	case $cf_vb_path in
	/*)
		cf_val_fault "$1" 0 "$CF_MARKER_BASIS_KEY names the absolute path \"$cf_vb_path\"; a basis cites a repository artifact, so the path is relative to the package root"
		return 0
		;;
	*..*)
		cf_val_fault "$1" 0 "$CF_MARKER_BASIS_KEY names \"$cf_vb_path\", which climbs out of the repository; a basis cites a repository artifact, so the path may not contain a parent-directory component"
		return 0
		;;
	esac
	if [ ! -f "$CF_PACKAGE_ROOT/$cf_vb_path" ]; then
		cf_val_fault "$1" 0 "$CF_MARKER_BASIS_KEY cites \"$cf_vb_path\", which is not a file in this repository; a divergence may be classified as expected only when a limitation the repository already DOCUMENTS authorises it, and a citation nobody can open documents nothing"
	fi
	return 0
}

# Split comma list $1 into one trimmed item per line on stdout. Exit status 1
# when an element is empty, which the format treats as a hard error rather than
# a dropped element.
cf_split_commas() {
	printf '%s' "$1" | awk '
	BEGIN { RS = "," }
	{
		item = $0
		gsub(/^[ \t\n]+/, "", item)
		gsub(/[ \t\n]+$/, "", item)
		if (item == "") { empty = 1; exit }
		print item
	}
	END { if (empty) { exit 1 } }
	'
}

# =============================================================================
# Matrix normalisation. Both spellings the parser accepts are accepted here.
# =============================================================================

CF_NORM_TARGET=''
cf_normalize_target() {
	case $1 in
	x86_64 | x86_64-linux-gnu) CF_NORM_TARGET='x86_64' ;;
	i686 | i686-linux-gnu) CF_NORM_TARGET='i686' ;;
	aarch64 | aarch64-linux-gnu) CF_NORM_TARGET='aarch64' ;;
	riscv64 | riscv64-linux-gnu) CF_NORM_TARGET='riscv64' ;;
	*) return 1 ;;
	esac
	return 0
}

# Print the canonical triple for a normalised short name. The default arm exists
# so an unmapped name can never silently yield an empty triple and a malformed
# cell label; every caller passes a value cf_normalize_target has accepted.
cf_target_triple() {
	case $1 in
	x86_64) printf 'x86_64-linux-gnu\n' ;;
	i686) printf 'i686-linux-gnu\n' ;;
	aarch64) printf 'aarch64-linux-gnu\n' ;;
	riscv64) printf 'riscv64-linux-gnu\n' ;;
	*)
		cf_error "internal: no triple mapping for target \"$1\""
		exit "$CF_EXIT_ENVIRONMENT"
		;;
	esac
}

CF_NORM_OPT=''
cf_normalize_opt() {
	case $1 in
	-O0 | O0) CF_NORM_OPT='-O0' ;;
	-O1 | O1) CF_NORM_OPT='-O1' ;;
	-O2 | O2) CF_NORM_OPT='-O2' ;;
	*) return 1 ;;
	esac
	return 0
}

# True when a token names an optimization level the project documents as out of
# scope, so the diagnostic can say why rather than merely that it is unknown.
cf_is_out_of_scope_opt() {
	case $1 in
	-O* | O*) return 0 ;;
	esac
	return 1
}

# =============================================================================
# Tool discovery.
#
# The variable names are the harness's own, so one mental model covers both.
# BCC_BIN is deliberately absent from this list: the compiler under test is not a
# dependency of this script.
#
# Nothing is ever installed and nothing is ever fetched. A missing tool is
# reported with the package that carries it, as text for a human to act on.
# =============================================================================

# `cf_tool_path`, which resolves a name to an absolute executable, is defined much
# earlier -- under "The search path", beside the sanitization that decides which
# directories it may resolve through. It has to be, because the execution bound
# resolves `env` and `timeout` with it before any of the tool discovery below runs.

# Resolve a tool into CF_CHOSEN, or fail loudly.
#   $1 = value of the override variable (may be empty)
#   $2 = name of the override variable, for the message
#   $3 = human description of the tool
#   $4 = install hint, printed as text and NEVER executed
#   $5.. = candidates, probed in order
#
# The candidate order mirrors tests/conformance_harness/env.rs exactly: an
# override is the ONLY candidate, and when there is none the first documented
# default that EXISTS becomes the candidate. Attestation then either accepts that
# one candidate or refuses it -- a refusal ends the search rather than advancing
# to the next name. The harness records the reason, and it holds here for the same
# reason: silently substituting a different implementation for one the script
# declined to trust would mean the golden bytes came from a compiler nobody chose,
# and it would let this script and the harness disagree about which driver is the
# oracle for an arm.
cf_choose() {
	cf_choose_value=$1
	cf_choose_var=$2
	cf_choose_desc=$3
	cf_choose_hint=$4
	shift 4
	if [ -n "$cf_choose_value" ]; then
		if cf_tool_path "$cf_choose_value"; then
			CF_CHOSEN=$CF_TOOL_PATH
			return 0
		fi
		cf_error "$cf_choose_var names \"$cf_choose_value\", which is not a regular executable file on PATH"
		cf_detail "needed as: $cf_choose_desc"
		cf_detail 'an override is honoured as written and never quietly replaced by a default, so'
		cf_detail 'a name that resolves to nothing is reported rather than worked around'
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	for cf_choose_candidate in "$@"; do
		if cf_tool_path "$cf_choose_candidate"; then
			CF_CHOSEN=$CF_TOOL_PATH
			return 0
		fi
	done
	cf_error "no $cf_choose_desc found"
	cf_detail "probed, in order: $*"
	cf_detail "set $cf_choose_var to name one, or install it yourself -- this script never installs anything:"
	cf_detail "    $cf_choose_hint"
	exit "$CF_EXIT_ENVIRONMENT"
}

# =============================================================================
# Reference-driver attestation.
#
# WHY A DRIVER HAS TO PROVE ITSELF BEFORE IT MAY WRITE A GOLDEN BYTE
# ------------------------------------------------------------------
# `expected_stdout` is oracle (c). Its whole authority is that the bytes came
# from an INDEPENDENT compiler that speaks the language the corpus is written in
# and builds for the target the cell claims. `command -v` establishes none of
# that: it answers yes for a wrapper that hands the work to the compiler under
# test, for a driver built for a different architecture, for a driver whose
# default language mode is a different revision of C, and for a program that is
# not a compiler at all. Each of those would let this script exit 0 having
# rewritten golden bytes from an authority nobody would have accepted knowingly,
# and the record would then certify that answer forever.
#
# So four questions are asked of every reference driver, and each is asked because
# the other three can pass while it fails:
#
#   1. WHICH PROGRAM ACTUALLY COMPILES? A wrapper script hides the implementation
#      behind a name. The chain of `exec` lines is followed to the program at the
#      end of it, and a wrapper whose behaviour is not determined by exactly one
#      unconditional `exec` is REFUSED rather than assumed to run itself.
#   2. IS IT INDEPENDENT OF THE COMPILER UNDER TEST? See the note below.
#   3. WHAT TARGET DOES IT BUILD FOR? Asked with -dumpmachine. A driver from a
#      different platform family is not a weaker oracle but a false one.
#   4. WHAT LANGUAGE DOES IT COMPILE BY DEFAULT? Asked with -dM -E. No -std flag
#      is ever passed -- the compiler under test has none to match -- so the
#      driver's own default mode IS the language every recorded byte is judged in,
#      and it cannot be corrected after the fact.
#
# HOW INDEPENDENCE IS PROVED WITHOUT READING BCC_BIN
# --------------------------------------------------
# This script must not read BCC_BIN: naming the compiler under test at all is the
# first step towards invoking it. Independence is therefore proved POSITIVELY
# rather than by comparison. Questions 3 and 4 are questions only a
# reference-class driver can answer: the compiler under test has neither
# -dumpmachine nor -dM -E in its documented flag set, so a driver -- or a wrapper
# chain ending in one -- that is really the compiler under test cannot answer
# them, and is refused for that reason without ever being named. Question 1 is
# what makes that sound: without following the wrapper chain, the answers would
# describe a launcher rather than the program that does the work.
#
# On top of that positive proof, a resolved implementation whose own file name is
# the compiler under test's is refused by name. That is redundant with the probes
# and cheap, and it turns the most likely honest mistake -- pointing an override
# at the wrong binary -- into a message that says so directly.
#
# These checks mirror tests/conformance_harness/env.rs (read_wrapper_target,
# vet_declared_target, vet_language_mode and machine_mismatch), so a driver this
# script accepts is one the harness would accept too. Keep them in step.
# =============================================================================

# The name of the compiler under test. Used ONLY to refuse a driver that resolves
# to a program of that name; the value of BCC_BIN is never read, and no program of
# this name is ever executed.
CF_COMPILER_UNDER_TEST_NAME='bcc'

# How many wrapper links are followed before a chain is declared unattested. A
# wrapper that wraps a wrapper is ordinary; four levels is far beyond any real
# installation.
CF_WRAPPER_DEPTH_MAX=4

# How much of a candidate is read while looking for an `exec` line. A file longer
# than this cannot be attested at all: the statements never read could include
# another `exec`, and "the part I read looked fine" is not attestation.
CF_WRAPPER_BYTES_MAX=8192

# The revision range a reference driver's DEFAULT mode must fall in: C11 at the
# oldest, C17 at the newest. Mirrors MIN_STDC_VERSION and MAX_STDC_VERSION in
# tests/conformance_harness/env.rs, including the reason. Below the range the
# driver would reject the corpus's C11 constructs outright. Above it, C23 changes
# the meaning of constructs the corpus contains -- a UTF-8 string literal changes
# type, and `bool`, `true` and `false` become keywords -- so a C23 driver is a
# false authority rather than a newer one.
CF_STDC_VERSION_MIN=201112
CF_STDC_VERSION_MAX=201710

# Set by cf_attest_driver for the driver it was asked about.
CF_ATTEST_IMPL=''    # the program at the end of the wrapper chain
CF_ATTEST_CHAIN=''   # the chain as text, for the provenance line
CF_ATTEST_MACHINE='' # the triple the driver reported
CF_ATTEST_MODE=''    # the __STDC_VERSION__ it compiles by default

# Print the absolute program an `exec`-only wrapper script hands its work to.
#
# Exit status: 0 the file is a wrapper and its target was printed; 1 the file is
# not a wrapper script at all, which is the ordinary answer for a compiled
# compiler; 2 the file looks like a script but what it runs could not be
# established, which is refused rather than guessed at.
#
# A script is attested only when it matches ONE unconditional `exec` in full: a
# shebang, any number of blank and comment lines, and exactly one `exec` naming an
# ABSOLUTE program. Any other statement -- a conditional, an assignment, a second
# `exec`, a redirection -- means the file's behaviour is not determined by that
# line, so what it really runs is unknown. A relative program word is refused for
# the same reason: resolving it would mean guessing a working directory, and a
# guess must not decide whether an oracle is independent.
cf_wrapper_target() {
	awk -v limit="$CF_WRAPPER_BYTES_MAX" '
	BEGIN { bytes = 0; execs = 0; program = ""; verdict = 0 }
	{
		# The shebang is tested BEFORE the read bound, and the order is
		# load-bearing rather than incidental: a compiled compiler is a binary whose
		# first newline may lie beyond the bound -- measured at 8952 bytes for one of
		# the four cross drivers here, against 6540 for another -- so accounting for
		# bytes first would report "a script I could not read" for an ELF file that
		# is not a script at all. This is the same order env.rs reads them in.
		if (NR == 1 && substr($0, 1, 2) != "#!") { verdict = 1; exit }
		bytes += length($0) + 1
		if (bytes >= limit) { verdict = 2; exit }
		if (NR == 1) { next }
		line = $0
		sub(/^[ \t]+/, "", line)
		sub(/[ \t]+$/, "", line)
		if (line == "" || substr(line, 1, 1) == "#") { next }
		words = split(line, word, /[ \t]+/)
		if (word[1] != "exec") { verdict = 2; exit }
		if (execs > 0) { verdict = 2; exit }
		execs++
		# The option grammar of the exec builtin is small and documented: -c and -l
		# take nothing, -a NAME takes one word, -- ends the options. Skipping -a
		# without skipping its argument would mistake the replacement argv[0] for
		# the program, which matters here more than anywhere: presenting a
		# different argv[0] is exactly what a compiler launcher is written to do.
		i = 2
		while (i <= words) {
			if (word[i] == "--") { i++; break }
			if (substr(word[i], 1, 1) != "-") { break }
			if (word[i] == "-a") { i++ }
			i++
		}
		if (i > words) { verdict = 2; exit }
		program = word[i]
		if (substr(program, 1, 1) != "/") { verdict = 2; exit }
	}
	END {
		if (verdict) { exit verdict }
		if (NR == 0) { exit 1 }
		if (execs != 1) { exit 2 }
		print program
	}
	' < "$1"
}

# Follow the wrapper chain from $1 to the program that actually does the work,
# into CF_ATTEST_IMPL, with the chain as text in CF_ATTEST_CHAIN.
#
# Exit status 0 on success; 1 when the chain could not be established, in which
# case CF_ATTEST_CHAIN carries the reason for the caller's message.
cf_resolve_implementation() {
	CF_ATTEST_IMPL=$1
	CF_ATTEST_CHAIN=$1
	cf_resolve_depth=0
	while [ "$cf_resolve_depth" -lt "$CF_WRAPPER_DEPTH_MAX" ]; do
		if cf_resolve_next=$(cf_wrapper_target "$CF_ATTEST_IMPL"); then
			cf_resolve_status=0
		else
			cf_resolve_status=$?
		fi
		case $cf_resolve_status in
		0) ;;
		1)
			# Not a wrapper: this file is its own implementation.
			return 0
			;;
		*)
			CF_ATTEST_CHAIN="\"$CF_ATTEST_IMPL\" is a script whose behaviour is not determined by a single unconditional exec of an absolute program, so the compiler it really runs could not be established"
			return 1
			;;
		esac
		if [ ! -f "$cf_resolve_next" ] || [ ! -x "$cf_resolve_next" ]; then
			CF_ATTEST_CHAIN="\"$CF_ATTEST_IMPL\" execs \"$cf_resolve_next\", which is not a regular executable file"
			return 1
		fi
		CF_ATTEST_IMPL=$cf_resolve_next
		CF_ATTEST_CHAIN="$CF_ATTEST_CHAIN -> $cf_resolve_next"
		cf_resolve_depth=$((cf_resolve_depth + 1))
	done
	CF_ATTEST_CHAIN="the wrapper chain from \"$1\" is more than $CF_WRAPPER_DEPTH_MAX links deep, so the compiler at the end of it could not be established"
	return 1
}

# Run an attestation probe: $1 = driver, $2.. = arguments. Stdout lands in
# CF_PROBE_OUT and stderr in CF_PROBE_ERR, both files. Exit status 0 when the
# probe completed successfully; 1 with CF_PROBE_WHY set otherwise.
CF_PROBE_OUT=''
CF_PROBE_ERR=''
CF_PROBE_WHY=''
cf_run_probe() {
	CF_PROBE_OUT="$CF_WORK/probe.stdout"
	CF_PROBE_ERR="$CF_WORK/probe.stderr"
	CF_PROBE_WHY=''
	mkdir -p -- "$CF_WORK/probe"
	cf_bound_run "$CF_TIMEOUT_SECS" "$CF_WORK/probe" \
		"$CF_PROBE_OUT" "$CF_PROBE_ERR" "$@"
	if [ -n "$CF_BOUND_ABORTED" ]; then
		CF_PROBE_WHY="the probe could not be carried out: $CF_BOUND_ABORTED"
		return 1
	fi
	if [ "$CF_BOUND_TIMEDOUT" -eq 1 ]; then
		CF_PROBE_WHY="it did not answer within the ${CF_TIMEOUT_SECS}s bound"
		return 1
	fi
	if [ "$CF_BOUND_STATUS" -ne 0 ]; then
		CF_PROBE_WHY="it rejected the arguments (status $CF_BOUND_STATUS)"
		return 1
	fi
	return 0
}

# True when $1 names $2's instruction set. The 32-bit x86 family is the one target
# with several spellings in circulation for the same instruction set, and a driver
# may legitimately report any of them; no other target has any a GNU driver would
# report. Mirrors I686_ARCHITECTURE_ALIASES in env.rs.
cf_architecture_matches() {
	if [ "$1" = "$2" ]; then
		return 0
	fi
	if [ "$2" = 'i686' ]; then
		case $1 in
		i686 | i586 | i486 | i386 | x86) return 0 ;;
		esac
	fi
	return 1
}

# Print why the reported triple $1 is not $2's, or nothing when it is. Mirrors
# machine_mismatch in env.rs, decision for decision.
cf_machine_mismatch() {
	if [ "$1" = "$(cf_target_triple "$2")" ]; then
		return 0
	fi
	# Components, empties dropped: a driver may or may not state a vendor, so
	# `x86_64-linux-gnu` and `x86_64-pc-linux-gnu` are both ordinary answers.
	cf_mismatch_arch=$(printf '%s\n' "$1" | awk 'BEGIN { FS = "-" } {
		for (i = 1; i <= NF; i++) { if ($i != "") { print $i; exit } }
	}')
	if [ -z "$cf_mismatch_arch" ]; then
		printf 'that names no architecture at all\n'
		return 0
	fi
	if ! cf_architecture_matches "$cf_mismatch_arch" "$2"; then
		printf 'its architecture %s is not %s\n' "$cf_mismatch_arch" "$2"
		return 0
	fi
	# `linux` must appear, and whatever follows it -- when anything does -- must be
	# `gnu`: the comparison is byte-exact against stdout, so the reference arm has
	# to agree with the compiler under test on type widths, calling convention and
	# the formatting of the C library it links. musl, android, uclibc and gnux32
	# each satisfy none of that, and each says so in that component.
	cf_mismatch_env=$(printf '%s\n' "$1" | awk 'BEGIN { FS = "-"; seen = 0 } {
		for (i = 1; i <= NF; i++) {
			if ($i == "") { continue }
			if (seen) { print $i; exit }
			if ($i == "linux") { seen = 1 }
		}
		if (! seen) { print "@no-linux" }
	}')
	case $cf_mismatch_env in
	'@no-linux')
		printf 'it does not name the linux operating system, so it belongs to a different platform family\n'
		;;
	'' | 'gnu') ;;
	*)
		printf 'its environment %s is not gnu, so it targets a different C library or application binary interface\n' "$cf_mismatch_env"
		;;
	esac
}

# Refuse the driver currently being attested, printing every reason given, and
# stop. Top level rather than nested inside cf_attest_driver so there is exactly
# one definition of what a refusal reads like.
#
# CF_ATTEST_EXTRA_FILE, when it names a non-empty file, is echoed as part of the
# refusal. It carries what the candidate itself said, which for "it rejected the
# arguments" is usually the whole diagnosis.
CF_ATTEST_EXTRA_FILE=''
cf_attest_refuse() {
	cf_error "$cf_attest_var: \"$cf_attest_path\" is refused as the $cf_attest_desc"
	while [ "$#" -gt 0 ]; do
		cf_detail "$1"
		shift
	done
	if [ -n "$CF_ATTEST_EXTRA_FILE" ] && [ -s "$CF_ATTEST_EXTRA_FILE" ]; then
		cf_detail 'what it said for itself:'
		cf_detail_file "$CF_ATTEST_EXTRA_FILE" 10
	fi
	cf_detail "point $cf_attest_var at a driver that satisfies this, or unset it to probe the documented defaults"
	cf_detail 'nothing is compiled and no golden byte is written from a driver whose authority is unproven'
	exit "$CF_EXIT_ENVIRONMENT"
}

# Attest the reference driver $1 for target $2, whose override variable is $3 and
# whose human description is $4. Returns with CF_ATTEST_* set, or exits 3.
#
# The three probes ask the LAUNCHER rather than the program at the end of its
# wrapper chain, exactly as vet_reference_driver does in env.rs, because the
# launcher is what every compile actually invokes: a wrapper that adds flags is
# part of the driver's behaviour, and attesting the program behind it would
# describe something else. The chain is resolved for the independence question,
# which is the one that is about which program does the work.
cf_attest_driver() {
	cf_attest_path=$1
	cf_attest_target=$2
	cf_attest_var=$3
	cf_attest_desc=$4
	cf_attest_triple=$(cf_target_triple "$cf_attest_target")
	# Cleared per driver so a refusal can never quote what a PREVIOUS candidate said.
	CF_ATTEST_EXTRA_FILE=''

	# 1. Which program actually compiles.
	if ! cf_resolve_implementation "$cf_attest_path"; then
		cf_attest_refuse "$CF_ATTEST_CHAIN" \
			'a wrapper is accepted only when a shebang, comments and exactly one unconditional' \
			'exec of an absolute program account for the whole file; anything else leaves the' \
			'compiler that does the work unknown, and an unknown compiler cannot be shown to be' \
			'independent of the compiler under test'
	fi

	# 2. Independence from the compiler under test, by name as well as by probe.
	if [ "$(basename -- "$CF_ATTEST_IMPL")" = "$CF_COMPILER_UNDER_TEST_NAME" ]; then
		cf_attest_refuse \
			"it resolves to \"$CF_ATTEST_IMPL\", which is named after the compiler under test" \
			'a golden record seeded from the compiler under test would certify that compiler' \
			'against itself and launder a miscompilation into the expectation'
	fi

	# 3. Which target it builds for.
	if ! cf_run_probe "$cf_attest_path" -dumpmachine; then
		CF_ATTEST_EXTRA_FILE=$CF_PROBE_ERR
		cf_attest_refuse \
			"it did not state which target it builds for: asked with -dumpmachine, $CF_PROBE_WHY" \
			'the only thing that distinguishes the driver for a target from any other program on' \
			'this machine is its own answer to that question, so silence is refused rather than' \
			'trusted -- and a program that cannot answer it is not a reference C driver at all'
	fi
	CF_ATTEST_MACHINE=$(awk '
		{ line = $0
		  sub(/^[ \t]+/, "", line)
		  sub(/[ \t]+$/, "", line)
		  if (line != "") { print line; exit } }
	' < "$CF_PROBE_OUT")
	if [ -z "$CF_ATTEST_MACHINE" ]; then
		cf_attest_refuse \
			'it answered -dumpmachine successfully but printed no target' \
			'a program that exits zero having printed nothing has not answered the question'
	fi
	cf_attest_mismatch=$(cf_machine_mismatch "$CF_ATTEST_MACHINE" "$cf_attest_target")
	if [ -n "$cf_attest_mismatch" ]; then
		cf_attest_refuse \
			"it reports that it targets $CF_ATTEST_MACHINE, but it was selected as the driver for $cf_attest_target ($cf_attest_triple): $cf_attest_mismatch" \
			'a reference compiler from a different platform family than the arm it serves is not a' \
			'weaker oracle but a false one: every byte recorded on that arm would come from a' \
			'program built for something else'
	fi

	# 4. Which language it compiles by default.
	if ! cf_run_probe "$cf_attest_path" -dM -E -x c -; then
		CF_ATTEST_EXTRA_FILE=$CF_PROBE_ERR
		cf_attest_refuse \
			"it did not state which language it compiles by default: asked with -dM -E -x c -, $CF_PROBE_WHY" \
			'this script passes no standard-selection flag -- the compiler under test has none to' \
			'match -- so the driver own default mode IS the language every recorded byte is judged' \
			'in, and an unproven language makes every one of those bytes unattributable'
	fi
	if grep -q '^#define __STRICT_ANSI__' < "$CF_PROBE_OUT"; then
		cf_attest_refuse \
			'it predefines __STRICT_ANSI__, so its default mode rejects the GNU extensions' \
			'the corpus exercises statement expressions, typeof, computed goto and case ranges,' \
			'and a strictly conforming default mode refuses them -- which would be recorded as a' \
			'divergence attributable to the oracle rather than to the compiler'
	fi
	CF_ATTEST_MODE=$(awk '
		$1 == "#define" && $2 == "__STDC_VERSION__" {
			value = $3
			sub(/[uUlL]+$/, "", value)
			print value
			exit
		}
	' < "$CF_PROBE_OUT")
	case $CF_ATTEST_MODE in
	'')
		cf_attest_refuse \
			'it predefined no __STDC_VERSION__, so either it is not a C driver or it did not answer' \
			'a driver that cannot say which revision of C it compiles cannot be an authority on' \
			'what a C program should print'
		;;
	*[!0-9]*)
		cf_attest_refuse \
			"it defined __STDC_VERSION__ as \"$CF_ATTEST_MODE\", which is not a revision number" \
			'the value has to be comparable against a range for the mode to be provable'
		;;
	esac
	if [ "$CF_ATTEST_MODE" -lt "$CF_STDC_VERSION_MIN" ]; then
		cf_attest_refuse \
			"it compiles __STDC_VERSION__ ${CF_ATTEST_MODE}L by default, older than the C11 (${CF_STDC_VERSION_MIN}L) this corpus is written in" \
			'it would reject the corpus rather than judge it, and every rejection would be' \
			'recorded as a divergence attributable to the oracle'
	fi
	if [ "$CF_ATTEST_MODE" -gt "$CF_STDC_VERSION_MAX" ]; then
		cf_attest_refuse \
			"it compiles __STDC_VERSION__ ${CF_ATTEST_MODE}L by default, newer than the C17 (${CF_STDC_VERSION_MAX}L) this corpus is written in" \
			'the difference is observable in this corpus: a UTF-8 string literal changes type, and' \
			'bool, true and false become keywords. No standard-selection flag can correct it,' \
			'because passing one to the reference compiler alone would break the shared-flag' \
			'discipline the comparison rests on' \
			'a version-suffixed driver such as gcc-13 pins the older mode, where the unsuffixed' \
			'name follows whatever the distribution currently defaults to'
	fi
}

# Resolve and attest the reference driver for target $1 into CF_CHOSEN, announcing
# its provenance once so the log records what produced the bytes.
#   $1 = target short name
#   $2 = value of the override variable (may be empty)
#   $3 = name of the override variable
#   $4 = human description
#   $5 = install hint
#   $6.. = candidates, probed in order
cf_choose_driver() {
	cf_choose_driver_target=$1
	shift
	cf_choose "$@"
	cf_attest_driver "$CF_CHOSEN" "$cf_choose_driver_target" "$2" "$3"
	cf_note "reference compiler for $(cf_target_triple "$cf_choose_driver_target"): $CF_ATTEST_CHAIN"
	cf_detail "attested: targets $CF_ATTEST_MACHINE, compiles __STDC_VERSION__ ${CF_ATTEST_MODE}L by default, GNU extensions enabled"
}

# Resolve the reference driver and the runner for target $1 into CF_CELL_CC and
# CF_CELL_RUNNER. Results are cached, and each tool is announced once so the
# provenance of the regenerated bytes is visible in the log.
#
# x86-64 is the natively executing target and has an EMPTY runner. Every other
# target runs under its emulator, including i686: routing it through qemu-i386
# rather than executing a 32-bit binary directly keeps the emulation variable
# uniform across all three non-baseline targets, exactly as the harness does.
#
# The i686 runner is spelled qemu-i386, never qemu-i686.
cf_resolve_cell_tools() {
	case $1 in
	x86_64)
		if [ -z "$CF_CC_X86_64" ]; then
			cf_choose_driver x86_64 "${BCC_REF_CC:-}" 'BCC_REF_CC' \
				'native reference C compiler for x86_64-linux-gnu' \
				'apt-get install -y gcc   (or gcc-13, where the unversioned gcc defaults to a later language mode)' \
				gcc cc clang
			CF_CC_X86_64=$CF_CHOSEN
		fi
		CF_CELL_CC=$CF_CC_X86_64
		CF_CELL_RUNNER=''
		;;
	i686)
		if [ -z "$CF_CC_I686" ]; then
			cf_choose_driver i686 "${BCC_REF_CC_I686:-}" 'BCC_REF_CC_I686' \
				'i686 reference cross driver' \
				'apt-get install -y gcc-i686-linux-gnu libc6-dev-i386-cross   (libc6-dev-i386 on older packaging)' \
				i686-linux-gnu-gcc
			CF_CC_I686=$CF_CHOSEN
		fi
		if [ -z "$CF_RUN_I686" ]; then
			cf_choose "${BCC_QEMU_I386:-}" 'BCC_QEMU_I386' \
				'i686 execution runner' \
				'apt-get install -y qemu-user-static   (qemu-user, where only that packaging exists)' \
				qemu-i386 qemu-i386-static
			CF_RUN_I686=$CF_CHOSEN
			cf_note "runner for i686-linux-gnu: $CF_RUN_I686"
		fi
		CF_CELL_CC=$CF_CC_I686
		CF_CELL_RUNNER=$CF_RUN_I686
		;;
	aarch64)
		if [ -z "$CF_CC_AARCH64" ]; then
			cf_choose_driver aarch64 "${BCC_REF_CC_AARCH64:-}" 'BCC_REF_CC_AARCH64' \
				'aarch64 reference cross driver' \
				'apt-get install -y gcc-aarch64-linux-gnu libc6-dev-arm64-cross' \
				aarch64-linux-gnu-gcc
			CF_CC_AARCH64=$CF_CHOSEN
		fi
		if [ -z "$CF_RUN_AARCH64" ]; then
			cf_choose "${BCC_QEMU_AARCH64:-}" 'BCC_QEMU_AARCH64' \
				'aarch64 execution runner' \
				'apt-get install -y qemu-user-static   (qemu-user, where only that packaging exists)' \
				qemu-aarch64 qemu-aarch64-static
			CF_RUN_AARCH64=$CF_CHOSEN
			cf_note "runner for aarch64-linux-gnu: $CF_RUN_AARCH64"
		fi
		CF_CELL_CC=$CF_CC_AARCH64
		CF_CELL_RUNNER=$CF_RUN_AARCH64
		;;
	riscv64)
		if [ -z "$CF_CC_RISCV64" ]; then
			cf_choose_driver riscv64 "${BCC_REF_CC_RISCV64:-}" 'BCC_REF_CC_RISCV64' \
				'riscv64 reference cross driver' \
				'apt-get install -y gcc-riscv64-linux-gnu libc6-dev-riscv64-cross' \
				riscv64-linux-gnu-gcc
			CF_CC_RISCV64=$CF_CHOSEN
		fi
		if [ -z "$CF_RUN_RISCV64" ]; then
			cf_choose "${BCC_QEMU_RISCV64:-}" 'BCC_QEMU_RISCV64' \
				'riscv64 execution runner' \
				'apt-get install -y qemu-user-static   (qemu-user, where only that packaging exists)' \
				qemu-riscv64 qemu-riscv64-static
			CF_RUN_RISCV64=$CF_CHOSEN
			cf_note "runner for riscv64-linux-gnu: $CF_RUN_RISCV64"
		fi
		CF_CELL_CC=$CF_CC_RISCV64
		CF_CELL_RUNNER=$CF_RUN_RISCV64
		;;
	*)
		cf_error "internal: no toolchain mapping for target \"$1\""
		exit "$CF_EXIT_ENVIRONMENT"
		;;
	esac

	# The two tools this cell is about to use must still be the ones that were
	# attested. Cheap enough to do per cell, and the only place a mid-run package
	# upgrade can be caught before it contaminates a golden record.
	cf_resolve_triple=$(cf_target_triple "$1")
	cf_require_stable_tool "$CF_CELL_CC" "reference compiler for $cf_resolve_triple"
	cf_require_stable_tool "$CF_CELL_RUNNER" "execution runner for $cf_resolve_triple"
}

# =============================================================================
# Capture validation.
#
# A golden record is a byte-exact expectation, so every property that would make
# byte-exact comparison meaningless -- or would make the record unparseable -- is
# a hard error that names the offending cell rather than something quietly
# repaired.
# =============================================================================

cf_validate_capture() {
	# $1 = captured stdout file, $2 = cell label, $3 = record label
	cf_validate_capture_file=$1
	cf_validate_capture_cell=$2
	cf_validate_capture_rec=$3

	if [ ! -s "$cf_validate_capture_file" ]; then
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced EMPTY stdout"
		cf_detail 'every corpus program prints at least one line; an empty stream is a defect in'
		cf_detail 'the program, not an expectation worth recording'
		exit "$CF_EXIT_CAPTURE"
	fi

	# True only when the last byte is NOT a newline: command substitution strips
	# trailing newlines, so a well-terminated stream yields the empty string.
	if [ -n "$(tail -c 1 < "$cf_validate_capture_file")" ]; then
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced stdout with NO TRAILING NEWLINE"
		cf_detail 'a heredoc value is its body lines each with a newline appended, so a stream that'
		cf_detail 'does not end in a newline cannot be represented in the record format at all'
		exit "$CF_EXIT_CAPTURE"
	fi

	tr -d '\000' < "$cf_validate_capture_file" > "$cf_validate_capture_file.nonul"
	if ! cmp -s -- "$cf_validate_capture_file" "$cf_validate_capture_file.nonul"; then
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced stdout containing a NUL BYTE"
		cf_detail 'the record format is line-oriented text; a NUL cannot survive it'
		exit "$CF_EXIT_CAPTURE"
	fi
	rm -f -- "$cf_validate_capture_file.nonul"

	if grep -q -- "$CF_CR" < "$cf_validate_capture_file"; then
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced stdout containing a CARRIAGE RETURN"
		cf_detail 'a CR would silently break byte-exact comparison between cells and against the record'
		exit "$CF_EXIT_CAPTURE"
	fi

	if grep -q '^END$' < "$cf_validate_capture_file"; then
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced stdout containing a line that is exactly END"
		cf_detail 'END is the heredoc terminator, so such a line would close the expected_stdout'
		cf_detail 'block early and corrupt the record; change what the program prints'
		exit "$CF_EXIT_CAPTURE"
	fi

	# One pass for the three parser ceilings, so a regenerated record is
	# guaranteed to still parse. Byte total is exact because the stream has
	# already been proven to end in a newline.
	if awk \
		-v linemax="$CF_LINE_BYTES_MAX" \
		-v linesmax="$CF_HEREDOC_LINES_MAX" \
		-v bytesmax="$CF_FIELD_BYTES_MAX" '
		length($0) > linemax { over = 2; exit }
		{ total += length($0) + 1 }
		END {
			if (over) { exit over }
			if (NR > linesmax) { exit 3 }
			if (total > bytesmax) { exit 4 }
		}
	' < "$cf_validate_capture_file"; then
		cf_validate_capture_limit=0
	else
		cf_validate_capture_limit=$?
	fi
	case $cf_validate_capture_limit in
	0) ;;
	2)
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced a stdout line longer than $CF_LINE_BYTES_MAX bytes"
		cf_detail 'the record parser refuses a line that long, so the record would not parse'
		exit "$CF_EXIT_CAPTURE"
		;;
	3)
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced more than $CF_HEREDOC_LINES_MAX stdout lines"
		cf_detail 'the record parser refuses a heredoc body that long, so the record would not parse'
		exit "$CF_EXIT_CAPTURE"
		;;
	4)
		cf_error "$cf_validate_capture_rec: $cf_validate_capture_cell produced more than $CF_FIELD_BYTES_MAX bytes of stdout"
		cf_detail 'the record parser refuses a field that large, so the record would not parse'
		exit "$CF_EXIT_CAPTURE"
		;;
	*)
		cf_error "$cf_validate_capture_rec: awk failed while measuring the stdout of $cf_validate_capture_cell (status $cf_validate_capture_limit)"
		exit "$CF_EXIT_CAPTURE"
		;;
	esac
}

# =============================================================================
# Building and running one cell.
#
# Flag discipline: the ONLY flags passed are the cell's optimization level, the
# mandatory -static, and -o. No -std (the driver's own default mode decides the
# language), no --target (a bcc-only selector; the reference driver selects its
# target by BEING a different binary), no --sysroot, no -m32, no warning flag,
# no sanitizer flag, and nothing from outside {-O0,-O1,-O2}.
#
# -static is mandatory on all four targets: it is the one linkage mode both
# compilers spell identically, and it is what lets the emulators run the
# binaries with no sysroot and no dynamic-loader configuration.
#
# stdout is redirected to a FILE and never captured into a shell variable:
# command substitution strips ALL trailing newlines, which would silently
# corrupt every golden record.
# =============================================================================

cf_capture_cell() {
	# $1 = source .c path
	# $2 = target short name
	# $3 = optimization flag
	# $4 = expected exit status
	# $5 = destination stdout file
	# $6 = record label
	cf_capture_src=$1
	cf_capture_target=$2
	cf_capture_opt=$3
	cf_capture_expect=$4
	cf_capture_out=$5
	cf_capture_rec=$6
	cf_capture_label="$(cf_target_triple "$cf_capture_target") $cf_capture_opt"

	cf_resolve_cell_tools "$cf_capture_target"

	# A FRESH workspace per cell, inside the private working area. The compiler and
	# the program both run with this as their working directory, so a relative path
	# either of them happens to name lands here and not in whatever directory the
	# maintainer invoked the script from -- and a cell cannot inherit a stray file
	# from the cell before it.
	cf_capture_cell_dir="$CF_WORK/cell"
	rm -rf -- "$cf_capture_cell_dir"
	mkdir -p -- "$cf_capture_cell_dir"
	cf_capture_bin="$cf_capture_cell_dir/program"
	cf_capture_cc_err="$cf_capture_cell_dir/compile.stderr"
	cf_capture_run_err="$cf_capture_cell_dir/run.stderr"
	cf_capture_cc_out="$cf_capture_cell_dir/compile.stdout"

	# The compiler's stdout and stderr are captured separately and BOTH reported,
	# because a driver that explains itself on stdout would otherwise explain
	# itself into a file nobody reads.
	cf_bound_run "$CF_TIMEOUT_SECS" "$cf_capture_cell_dir" \
		"$cf_capture_cc_out" "$cf_capture_cc_err" \
		"$CF_CELL_CC" "$cf_capture_opt" -static "$cf_capture_src" -o "$cf_capture_bin"
	cf_capture_command="$CF_CELL_CC $cf_capture_opt -static $cf_capture_src -o <out>"
	if [ -n "$CF_BOUND_ABORTED" ]; then
		cf_error "$cf_capture_rec: compiling $cf_capture_label could not be carried out: $CF_BOUND_ABORTED"
		cf_detail "command: $cf_capture_command"
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	if [ "$CF_BOUND_TIMEDOUT" -eq 1 ]; then
		cf_error "$cf_capture_rec: compiling $cf_capture_label reached the ${CF_TIMEOUT_SECS}s bound"
		cf_detail "command: $cf_capture_command"
		cf_detail 'raise BCC_CONFORMANCE_TIMEOUT_SECS if the bound is simply too tight'
		exit "$CF_EXIT_CAPTURE"
	fi
	if [ "$CF_BOUND_STATUS" -ne 0 ]; then
		cf_error "$cf_capture_rec: the reference compiler failed on $cf_capture_label (status $CF_BOUND_STATUS)"
		cf_detail "command: $cf_capture_command"
		if [ "$CF_BOUND_STATUS" -ge "$CF_SIGNAL_STATUS_BASE" ]; then
			cf_detail "the status is ambiguous $CF_BOUND_STATUS: a POSIX shell reports death by signal $((CF_BOUND_STATUS - CF_SIGNAL_STATUS_BASE)) and a program that RETURNED $CF_BOUND_STATUS as the same number, so which of the two happened cannot be established here"
		fi
		if [ -s "$cf_capture_cc_out" ]; then
			cf_detail_file "$cf_capture_cc_out" 20
		fi
		if [ -s "$cf_capture_cc_err" ]; then
			cf_detail_file "$cf_capture_cc_err" 20
		fi
		exit "$CF_EXIT_CAPTURE"
	fi

	# The artifact must be a regular file inside the cell's own workspace before it
	# is executed. The output path is this script's own, so this can only fail if
	# something replaced it -- which is exactly when running it would be wrong.
	if [ ! -f "$cf_capture_bin" ] || [ -L "$cf_capture_bin" ]; then
		cf_error "$cf_capture_rec: the reference compiler reported success on $cf_capture_label but left no regular output file"
		cf_detail "expected a regular file at: $cf_capture_bin"
		exit "$CF_EXIT_CAPTURE"
	fi

	if [ -z "$CF_CELL_RUNNER" ]; then
		cf_bound_run "$CF_TIMEOUT_SECS" "$cf_capture_cell_dir" \
			"$cf_capture_out" "$cf_capture_run_err" "$cf_capture_bin"
	else
		cf_bound_run "$CF_TIMEOUT_SECS" "$cf_capture_cell_dir" \
			"$cf_capture_out" "$cf_capture_run_err" "$CF_CELL_RUNNER" "$cf_capture_bin"
	fi
	if [ -n "$CF_BOUND_ABORTED" ]; then
		cf_error "$cf_capture_rec: running $cf_capture_label could not be carried out: $CF_BOUND_ABORTED"
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	if [ "$CF_BOUND_TIMEDOUT" -eq 1 ]; then
		cf_error "$cf_capture_rec: running $cf_capture_label reached the ${CF_TIMEOUT_SECS}s bound"
		cf_detail 'the bound is reported by the run not completing, never by an exit status, so a'
		cf_detail 'program that legitimately exits 124 or 125 is not mistaken for a hang'
		cf_detail 'raise BCC_CONFORMANCE_TIMEOUT_SECS if the bound is simply too tight'
		exit "$CF_EXIT_CAPTURE"
	fi

	# Every status in 0..125 is a status a program may legitimately choose, and the
	# bound above is reported out of band, so the comparison here is a plain one:
	# whatever the program exited with must be what the record declared.
	if [ "$CF_BOUND_STATUS" -ne "$cf_capture_expect" ]; then
		cf_error "$cf_capture_rec: $cf_capture_label exited with $CF_BOUND_STATUS but the record declares expect_exit = $cf_capture_expect"
		cf_detail 'the status is as much of the expectation as the bytes are, so stdout captured'
		cf_detail 'from a run that ended the wrong way is not a golden record'
		if [ "$CF_BOUND_STATUS" -ge "$CF_SIGNAL_STATUS_BASE" ]; then
			cf_detail "the status is ambiguous $CF_BOUND_STATUS: death by signal $((CF_BOUND_STATUS - CF_SIGNAL_STATUS_BASE)) and a normal exit of $CF_BOUND_STATUS are the same number to a POSIX shell, so this run cannot say which occurred -- tests/conformance_harness/findings.rs reports such a status the same way, as ambiguous rather than as a signal"
		fi
		if [ -s "$cf_capture_run_err" ]; then
			cf_detail_file "$cf_capture_run_err" 20
		fi
		exit "$CF_EXIT_CAPTURE"
	fi

	cf_validate_capture "$cf_capture_out" "$cf_capture_label" "$cf_capture_rec"
}

# =============================================================================
# The rewrite engine.
#
# A state machine over the record's lines that replaces ONLY the expected_stdout
# block and copies every other byte through unchanged. It is deliberately NOT a
# file-tail truncation: expected_stdout is the last key in most records but NOT
# in all of them -- the two marker-carrying records place the whole
# expected_divergence.* block after it -- and blind truncation would silently
# destroy those keys.
#
# The one line it synthesises is the opener. `expected_stdout` is 15 characters,
# so four spaces put "<<" at column 20, which is where every base key's operator
# sits (the longest, `impl_defined_notes`, is 18 characters plus one space). The
# marker keys use their own column and are copied verbatim, so both alignment
# schemes survive without the engine knowing either exists.
#
# Two hazards are handled by the same heredoc state that the reader uses:
#   * a line inside ANOTHER heredoc's body that merely looks like an opener --
#     including a literal `expected_stdout    <<END` written as prose -- is
#     copied as data and does not trigger replacement;
#   * a line inside a heredoc body that merely looks like a scalar cannot be
#     mistaken for a key.
#
# Exit status: 0 rewritten, 4 duplicate expected_stdout, 5 unterminated heredoc,
# 6 no expected_stdout block, 7 heredoc opener naming a terminator other than
# END, 8 the body file could not be spliced in full.
# =============================================================================

# Discard the in-flight staging file and stop, leaving the record on disk
# byte-identical to what it was. Safe to call before any staging file exists.
cf_rec_abort() {
	cf_release_staging
	cf_detail 'nothing was written; the record is unchanged'
	exit "$CF_EXIT_RECORD"
}

# =============================================================================
# Writing through directories held open by identity.
# =============================================================================
#
# Every write this script performs inside the corpus goes through one of the three
# functions below. The record parent is opened on fd 8 after containment and
# owner/mode validation; the unpredictable staging child is opened on fd 9.
# Record/source reads, staging writes, the installing rename and cleanup all use
# `/proc/$$/fd/<n>/.` paths derived from those descriptors.
#
# The alternative, which these replace, was to check the resolved path and then
# write through the path as WRITTEN. That is two resolutions of one name at two
# different moments, and they agree only while no component of the name changes in
# between. A feature-area directory replaced by a symbolic link after the check
# would be approved here and written somewhere else, with every message in the run
# still displaying a corpus path -- and the file being written is the one the whole
# suite treats as the definition of correct output.
#
# A name is still retained for diagnostics and final removal, but it is never
# trusted by presence alone: `test -ef` must prove that it still designates the
# open descriptor. If an ancestor or staging entry changes, install/removal is
# refused rather than redirected.

# The name a staging file always takes inside its staging directory. A fixed leaf is
# safe precisely because the DIRECTORY around it is the unpredictable part.
CF_STAGING_LEAF='record'

# Open one record directory on fd 8 and validate the object reached through the
# handle, not merely the path that named it. Ancestor substitutions after this
# point cannot redirect any record/source/staging operation.
CF_PIN_REASON=''
cf_pin_record_dir() {
	# $1 = named physical directory
	CF_PIN_REASON=''
	cf_release_staging
	cf_release_record_dir
	if ! exec 8< "$1"; then
		CF_PIN_REASON='the record directory could not be opened as a durable handle'
		return 1
	fi
	CF_RECORD_DIR_NAME=$1
	CF_RECORD_DIR_HANDLE="/proc/$$/fd/8/."
	CF_RECORD_DIR_OPEN=1
	if ! command test "$CF_RECORD_DIR_NAME" -ef "$CF_RECORD_DIR_HANDLE"; then
		CF_PIN_REASON='the record directory changed identity while its handle was being opened'
		cf_release_record_dir
		return 1
	fi
	if cf_prd_why=$(cf_untrusted_dir "$CF_RECORD_DIR_HANDLE"); then
		CF_PIN_REASON="the record directory is not a trusted staging parent: $cf_prd_why"
		cf_release_record_dir
		return 1
	fi
	return 0
}

# Open the freshly-created unpredictable staging directory on fd 9. All staging
# file reads/writes use the handle; the parent entry is used only after `-ef`
# proves it still names this object.
cf_pin_staging() {
	# $1 = staging entry through the fd-8 record parent
	if ! exec 9< "$1"; then
		CF_PIN_REASON='the staging directory could not be opened as a durable handle'
		return 1
	fi
	CF_STAGING_ENTRY=$1
	CF_STAGING_HANDLE="/proc/$$/fd/9/."
	CF_STAGING_OPEN=1
	if ! command test "$CF_STAGING_ENTRY" -ef "$CF_STAGING_HANDLE"; then
		CF_PIN_REASON='the staging directory changed identity while its handle was being opened'
		cf_release_staging
		return 1
	fi
	return 0
}

# Create a staging directory inside the pinned directory $1 and seed it with $2 as
# CF_STAGING_LEAF. Prints the staging directory's name RELATIVE to $1, so the caller
# can neither be handed nor act on an absolute path built elsewhere.
#
# $2 is the caller's already-read SNAPSHOT of the record, not the corpus entry's
# name. The seed exists only to give the staging file the record's own permissions --
# the rewriter truncates and replaces its content immediately afterwards -- and
# taking it from the snapshot means the corpus record is read exactly once per
# record, so the mode that ends up published cannot come from an entry substituted
# after the record was validated.
cf_stage_beside() {
	# $1 = pinned physical directory, $2 = path of the snapshot to seed from
	cf_sb_seed=$2
	(
		CDPATH='' cd -P -- "$1" || exit 1
		cf_sb_dir=$(mktemp -d -- './.regen.XXXXXX' 2> /dev/null) || exit 1
		cf_sb_dir=${cf_sb_dir#./}
		# The seed is named absolutely and is resolved after the `cd`, which is why it
		# is read into a variable first: `$CF_WORK` is a `/proc/$$/fd` handle path and
		# `$$` is the invoking shell's identifier in this subshell too, so the handle
		# still designates the private working directory here.
		cp -p -- "$cf_sb_seed" "$cf_sb_dir/$CF_STAGING_LEAF" || {
			rm -rf -- "$cf_sb_dir"
			exit 1
		}
		printf '%s\n' "$cf_sb_dir"
	)
}

# Install the staged record from the fd-9 staging handle onto the fd-8 record
# parent. Both descriptors refer to directories already verified by identity, so
# no written ancestor is resolved at install time.
cf_install_staged() {
	# $1 = record-parent handle, $2 = staging handle, $3 = record base
	if ! command test "$CF_STAGING_ENTRY" -ef "$2"; then
		return 1
	fi
	mv -- "$2/$CF_STAGING_LEAF" "$1/$3"
}

# Confirm that the file now at $1/$2 is exactly the bytes retained in $3, still
# physically inside $4, and still a regular file. Sets CF_VERIFY_REASON and returns
# non-zero on the first thing that is not true.
CF_VERIFY_REASON=''
cf_verify_published() {
	# $1 = pinned physical directory, $2 = record base, $3 = retained bytes,
	# $4 = the area directory the record must physically live in
	CF_VERIFY_REASON=''
	cf_vp_dir=$(cf_physical_dir "$1") || cf_vp_dir=''
	if [ "$cf_vp_dir" != "$4" ]; then
		CF_VERIFY_REASON="after the rename the directory resolves to \"${cf_vp_dir:-<unresolvable>}\" rather than \"$4\""
		return 1
	fi
	if [ -L "$1/$2" ]; then
		CF_VERIFY_REASON='the installed record is a symbolic link; a rename cannot produce one, so the name was replaced'
		return 1
	fi
	if [ ! -f "$1/$2" ]; then
		CF_VERIFY_REASON='the installed record is not a regular file'
		return 1
	fi
	if ! cmp -s -- "$3" "$1/$2"; then
		CF_VERIFY_REASON='the installed record differs byte for byte from what was staged'
		return 1
	fi
	return 0
}

# Turn a structure status from the auditor or the rewriter into an actionable
# message, then abort. One mapper serves both so the two can never disagree.
cf_report_structure_error() {
	# $1 = status, $2 = record label
	case $1 in
	4)
		cf_error "$2: the record declares expected_stdout more than once"
		cf_detail 'a duplicate key is a hard parse error, and there is no way to tell which of'
		cf_detail 'the two blocks is the golden record'
		;;
	5)
		cf_error "$2: the record ends inside an UNTERMINATED heredoc"
		cf_detail 'every heredoc body is closed by a line containing exactly END, with no'
		cf_detail 'surrounding whitespace'
		;;
	6)
		cf_error "$2: the record has NO expected_stdout block to regenerate"
		cf_detail 'expected_stdout is required: without it oracle (c) has nothing to assert'
		;;
	7)
		cf_error "$2: a heredoc opener names a terminator other than END"
		cf_detail 'the one accepted terminator is END, so an opener must read: key <<END'
		;;
	8)
		cf_error "$2: the captured stdout could not be spliced into the record in full"
		;;
	*)
		cf_error "$2: awk failed while processing the record (status $1)"
		;;
	esac
	cf_rec_abort
}

cf_rewrite_record() {
	# $1 = record path, $2 = new body file, $3 = staging path, $4 = body line count
	CF_REWRITE_BODY="$2" awk -v bodylines="$4" '
	BEGIN { body = ENVIRON["CF_REWRITE_BODY"]; state = 0; seen = 0; fatal = 0 }
	# state 0 = outside any heredoc
	# state 1 = inside a heredoc that is NOT expected_stdout: copy it through
	# state 2 = inside the OLD expected_stdout body: swallow it
	state == 1 { print; if ($0 == "END") { state = 0 } next }
	state == 2 { if ($0 == "END") { state = 0 } next }
	{
		trimmed = $0
		sub(/^[ \t]+/, "", trimmed)
		sub(/[ \t]+$/, "", trimmed)
		if (trimmed == "" || substr(trimmed, 1, 1) == "#") { print; next }
		opener = index($0, "<<")
		equals = index($0, "=")
		if (opener > 0 && (equals == 0 || opener < equals)) {
			key = substr($0, 1, opener - 1)
			sub(/^[ \t]+/, "", key)
			sub(/[ \t]+$/, "", key)
			terminator = substr($0, opener + 2)
			sub(/^[ \t]+/, "", terminator)
			sub(/[ \t]+$/, "", terminator)
			if (terminator != "END") { fatal = 7; exit fatal }
			if (key == "expected_stdout") {
				if (seen) { fatal = 4; exit fatal }
				seen = 1
				printf "expected_stdout    <<END\n"
				spliced = 0
				while ((getline line < body) > 0) {
					print line
					spliced++
				}
				close(body)
				if (spliced != bodylines) { fatal = 8; exit fatal }
				print "END"
				state = 2
				next
			}
			print
			state = 1
			next
		}
		print
	}
	END {
		if (fatal) { exit fatal }
		if (state != 0) { exit 5 }
		if (! seen) { exit 6 }
	}
	' < "$1" > "$3"
}

# =============================================================================
# Processing one record.
# =============================================================================

cf_process_record() {
	cf_rec=$1
	cf_rec_dir=$(dirname -- "$cf_rec")
	cf_rec_base=$(basename -- "$cf_rec")
	cf_rec_stem=${cf_rec_base%.expected}
	cf_rec_area=$(basename -- "$cf_rec_dir")
	cf_rec_label="$cf_rec_area/$cf_rec_stem"
	cf_rec_src="$cf_rec_dir/$cf_rec_stem.c"

	# --- Containment, before the record is read at all ------------------------
	# Re-established here rather than assumed from enumeration, because a record
	# can also arrive from --program, and because this is the last point before the
	# record is read, its program compiled, and the file rewritten. Three things
	# are required, and the third is the one a symbolic link is designed to defeat:
	# the name must match the corpus grammar, nothing may be a link, and the
	# directory the pair actually lives in must BE the corpus's own area directory
	# rather than merely be reachable through its name.
	if ! cf_looks_like_area "$cf_rec_area"; then
		cf_error "$cf_rec_label: \"$cf_rec_area\" is not a feature-area directory name"
		cf_detail "record path: $cf_rec"
		exit "$CF_EXIT_RECORD"
	fi
	if ! cf_looks_like_program "$cf_rec_stem"; then
		cf_error "$cf_rec_label: \"$cf_rec_stem\" is not a program name"
		cf_detail "record path: $cf_rec"
		exit "$CF_EXIT_RECORD"
	fi
	if ! cf_pin_record_dir "$cf_rec_dir"; then
		cf_error "$cf_rec_label: the record directory could not be pinned safely"
		cf_detail "$CF_PIN_REASON"
		exit "$CF_EXIT_RECORD"
	fi
	cf_rec_real_dir=$(cf_physical_dir "$CF_RECORD_DIR_HANDLE") || cf_rec_real_dir=''
	if [ "$cf_rec_real_dir" != "$CF_CORPUS_DIR/$cf_rec_area" ]; then
		cf_error "$cf_rec_label: the record does not physically live in this corpus"
		cf_detail "named:       $cf_rec"
		cf_detail "its directory resolves to: ${cf_rec_real_dir:-<unresolvable>}"
		cf_detail "expected:    $CF_CORPUS_DIR/$cf_rec_area"
		cf_detail 'a record decides what is compiled and what counts as correct, so one read from'
		cf_detail 'outside the corpus would decide both while every message showed a corpus path'
		exit "$CF_EXIT_RECORD"
	fi
	# From here on the RESOLVED directory is the one every read and every write goes
	# through, and the name the record arrived under is used only in messages.
	#
	# The containment check above resolves a path; acting on the path as WRITTEN in the
	# staging and the installing rename would be a second resolution performed later.
	# Two resolutions of the same name are two different answers whenever a component
	# of that name changes in between -- and an area directory replaced by a symbolic
	# link between the check and the rename would be checked here and written somewhere
	# else. Opening the resolved directory and operating through fd 8 removes the gap
	# rather than narrowing it:
	# `cf_rec_phys_dir` is a durable handle path, and every read and mutation below
	# is relative to that object even if a written ancestor changes later.
	cf_rec_phys_dir=$CF_RECORD_DIR_HANDLE
	cf_rec_phys="$cf_rec_phys_dir/$cf_rec_base"
	cf_rec_src="$cf_rec_phys_dir/$cf_rec_stem.c"

	if [ ! -f "$cf_rec_phys" ] || [ -L "$cf_rec_phys" ]; then
		cf_error "$cf_rec_label: the record is not a regular file (a symbolic link is refused)"
		exit "$CF_EXIT_RECORD"
	fi

	# --- ONE READ OF THE CORPUS RECORD, AND ONE ONLY --------------------------
	# Taken here, immediately after the leaf has been checked and while the
	# directory is pinned, into the private working area -- which is itself an
	# fd-backed handle this run created and validated as private. Everything that
	# reads the OLD record from this point on reads this snapshot: the validator,
	# the staging seed that carries the record's mode, the rewriter that copies
	# every line outside expected_stdout through, and the comparison that decides
	# whether anything changed. The corpus entry itself is read exactly once, by
	# the `cp` below.
	#
	# WHY, PRECISELY. The predecessor validated `cf_rec_phys` and then re-opened
	# the record for the rewrite -- and re-opened it under the name the record
	# ARRIVED under rather than through the pinned directory. That is two reads of
	# one name at two moments, and they return the same bytes only while nothing
	# replaces the entry in between. A substitution in that window produced a
	# candidate whose non-golden lines came from a DIFFERENT file while the
	# installation still targeted the pinned corpus entry, so a record could be
	# rewritten to carry another file's targets, oracle switches, marker or command
	# templates -- and every message in the run would still have displayed the
	# corpus path. The file being written is the one the whole suite treats as the
	# definition of correct output, so a second read of it is not a detail.
	#
	# A snapshot rather than merely a pinned path, because pinning the DIRECTORY
	# closes the ancestor window and not the leaf one: the entry inside the pinned
	# directory can still be replaced between two reads. `cp` opens the source
	# once, so these bytes are one inode's contents and cannot be a splice of two.
	#
	# `-p` because the staging seed derives the published record's mode from this
	# file, so the snapshot has to carry the record's own permissions rather than
	# whatever this run's umask would produce.
	#
	# What deliberately does NOT move to the snapshot: the installing rename and
	# the verification that follows it. Those must act on the corpus, through the
	# pinned directory, because their whole purpose is to change and then re-read
	# what is actually on disk.
	cf_rec_snapshot="$CF_WORK/record.snapshot"
	rm -f -- "$cf_rec_snapshot"
	if ! cp -p -- "$cf_rec_phys" "$cf_rec_snapshot"; then
		cf_error "$cf_rec_label: the record could not be read into a snapshot"
		cf_detail "named:    $cf_rec"
		cf_detail "resolved: $cf_rec_phys"
		cf_detail 'the record is read exactly once, and every later decision is made from that'
		cf_detail 'one snapshot, so a failure here is refused rather than retried by re-reading'
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	if [ ! -f "$cf_rec_snapshot" ] || [ -L "$cf_rec_snapshot" ]; then
		cf_error "$cf_rec_label: the record snapshot is not a regular file"
		cf_detail 'the private working area is this run''s own fd-backed handle, so this cannot'
		cf_detail 'happen without the working area having been tampered with mid-run'
		exit "$CF_EXIT_ENVIRONMENT"
	fi

	if [ ! -f "$cf_rec_src" ] || [ -L "$cf_rec_src" ]; then
		cf_error "$cf_rec_label: no sibling program \"$cf_rec_stem.c\" beside the record"
		cf_detail 'the corpus pairs every record 1:1 with the program it describes; a record'
		cf_detail 'without a program describes nothing and can regenerate nothing'
		cf_detail 'a symbolic link is refused rather than followed'
		exit "$CF_EXIT_RECORD"
	fi
	# --- The whole record, before a single compile is spent on it --------------
	# One validator, run in full, covering every property the parser enforces:
	# the closed key set, field kinds, duplicates, required keys, the four size
	# ceilings, control bytes, the final newline, marker completeness, the oracle
	# switches, the shared-flag set and the three command templates. It stores
	# every value the rest of this function needs, so there is exactly ONE reader
	# of the record format here and no second one to disagree with it.
	#
	# Read from the SNAPSHOT, so that what is validated and what is rewritten are
	# the same bytes rather than two reads that happen to agree. Every diagnostic
	# it prints is keyed on the label, so nothing about the message changes.
	cf_validate_record "$cf_rec_snapshot" "$cf_rec_label" "$cf_rec_stem" "$cf_rec_area"

	# --- What the program itself reaches for, before it is compiled ------------
	# The record has been validated; this validates the other half of the pair. A
	# program that opens a file or reads the clock would compile, run, and have its
	# output installed as the definition of correct behaviour -- so the surface is
	# audited before any compiler is invoked on it, not after.
	if ! cf_audit_source_surface "$cf_rec_src" "$cf_rec_label"; then
		exit "$CF_EXIT_RECORD"
	fi

	# --- Matrix ---------------------------------------------------------------
	# The record's own declaration is honoured exactly and never widened.
	cf_rec_expect=$(cf_val_get expect_exit)
	cf_rec_targets_raw=$(cf_val_get targets)
	cf_rec_opts_raw=$(cf_val_get opt_levels)

	cf_rec_targets_file="$CF_WORK/targets.list"
	cf_rec_opts_file="$CF_WORK/opts.list"

	if ! cf_split_commas "$cf_rec_targets_raw" > "$cf_rec_targets_file"; then
		cf_error "$cf_rec_label: targets = \"$cf_rec_targets_raw\" holds an empty element"
		cf_detail 'an empty element is a hard error rather than a dropped one, so a stray comma'
		cf_detail 'can never silently shrink the matrix'
		exit "$CF_EXIT_RECORD"
	fi
	if ! cf_split_commas "$cf_rec_opts_raw" > "$cf_rec_opts_file"; then
		cf_error "$cf_rec_label: opt_levels = \"$cf_rec_opts_raw\" holds an empty element"
		exit "$CF_EXIT_RECORD"
	fi
	if [ ! -s "$cf_rec_targets_file" ]; then
		cf_error "$cf_rec_label: targets is empty; a program compared on no target is compared by no oracle"
		exit "$CF_EXIT_RECORD"
	fi
	if [ ! -s "$cf_rec_opts_file" ]; then
		cf_error "$cf_rec_label: opt_levels is empty"
		exit "$CF_EXIT_RECORD"
	fi

	# Normalise both lists up front, so an unsupported value is refused before a
	# single compile is spent.
	cf_rec_targets_norm="$CF_WORK/targets.norm"
	: > "$cf_rec_targets_norm"
	while IFS= read -r cf_rec_item; do
		if ! cf_normalize_target "$cf_rec_item"; then
			cf_error "$cf_rec_label: targets names \"$cf_rec_item\", which is not one of the four supported targets"
			cf_detail 'accepted: x86_64, i686, aarch64, riscv64 and their -linux-gnu triples'
			exit "$CF_EXIT_RECORD"
		fi
		printf '%s\n' "$CF_NORM_TARGET" >> "$cf_rec_targets_norm"
	done < "$cf_rec_targets_file"
	if ! awk 'seen[$0]++ { exit 1 }' < "$cf_rec_targets_norm"; then
		cf_error "$cf_rec_label: targets = \"$cf_rec_targets_raw\" names the same target twice"
		cf_detail 'the cell matrix is a set, so a target may be declared at most once'
		exit "$CF_EXIT_RECORD"
	fi

	cf_rec_opts_norm="$CF_WORK/opts.norm"
	: > "$cf_rec_opts_norm"
	while IFS= read -r cf_rec_item; do
		if ! cf_normalize_opt "$cf_rec_item"; then
			if cf_is_out_of_scope_opt "$cf_rec_item"; then
				cf_error "$cf_rec_label: opt_levels names \"$cf_rec_item\", which is documented as out of scope"
				cf_detail 'docs/technical-specifications.md records that only -O0, -O1 and -O2 are in'
				cf_detail 'scope, so no record may sweep a level the compiler under test does not support'
			else
				cf_error "$cf_rec_label: opt_levels names \"$cf_rec_item\", which is not an optimization level"
				cf_detail 'accepted: -O0, -O1, -O2, and the bare O0, O1, O2 spellings'
			fi
			exit "$CF_EXIT_RECORD"
		fi
		printf '%s\n' "$CF_NORM_OPT" >> "$cf_rec_opts_norm"
	done < "$cf_rec_opts_file"
	if ! awk 'seen[$0]++ { exit 1 }' < "$cf_rec_opts_norm"; then
		cf_error "$cf_rec_label: opt_levels = \"$cf_rec_opts_raw\" names the same level twice"
		cf_detail 'the sweep is a set, so a level may be declared at most once'
		exit "$CF_EXIT_RECORD"
	fi

	# --- Capture every declared cell -----------------------------------------
	# Every cell must agree byte-for-byte before anything is written: oracle (c)
	# asserts ONE golden record against EVERY cell of the matrix, so recording a
	# stream without proving agreement would bake a target-specific answer into a
	# cross-target expectation.
	rm -rf -- "$CF_WORK/cells"
	mkdir -p -- "$CF_WORK/cells"
	cf_rec_golden=''
	cf_rec_golden_label=''
	cf_rec_cells=0

	while IFS= read -r cf_rec_target; do
		while IFS= read -r cf_rec_opt; do
			cf_rec_cell_out="$CF_WORK/cells/$cf_rec_target${cf_rec_opt}.stdout"
			cf_capture_cell "$cf_rec_src" "$cf_rec_target" "$cf_rec_opt" \
				"$cf_rec_expect" "$cf_rec_cell_out" "$cf_rec_label"
			cf_rec_cell_label="$(cf_target_triple "$cf_rec_target") $cf_rec_opt"
			cf_rec_cells=$((cf_rec_cells + 1))
			if [ -z "$cf_rec_golden" ]; then
				cf_rec_golden=$cf_rec_cell_out
				cf_rec_golden_label=$cf_rec_cell_label
			elif ! cmp -s -- "$cf_rec_golden" "$cf_rec_cell_out"; then
				cf_error "$cf_rec_label: declared cells DISAGREE, so there is no single golden record to write"
				cf_detail "cell A: $cf_rec_golden_label"
				cf_detail "cell B: $cf_rec_cell_label"
				cf_detail "$(cmp -- "$cf_rec_golden" "$cf_rec_cell_out" 2>&1 || true)"
				cf_detail 'one golden record is asserted against every cell of the matrix, so cells that'
				cf_detail 'disagree mean the PROGRAM is MIS-AUTHORED, not that the backends diverged:'
				cf_detail 'fix what the program prints so it is target-invariant. Never relax the record'
				cf_detail 'and never restrict the target list to make the disagreement go away.'
				cf_detail 'Nothing was written; the record is unchanged.'
				exit "$CF_EXIT_CAPTURE"
			fi
		done < "$cf_rec_opts_norm"
	done < "$cf_rec_targets_norm"

	if [ "$cf_rec_cells" -eq 0 ] || [ -z "$cf_rec_golden" ]; then
		cf_error "$cf_rec_label: the declared matrix produced no cell"
		exit "$CF_EXIT_RECORD"
	fi

	# --- Stage the rewrite ----------------------------------------------------
	cf_rec_bodylines=$(awk 'END { print NR }' < "$cf_rec_golden")

	if [ "$CF_CHECK" -eq 1 ]; then
		# --check writes nothing at all inside the corpus, not even a staging
		# file: the candidate is built in the private working area instead.
		#
		# Seeded from the snapshot rather than from the corpus entry. The seed
		# exists to give the candidate the record's own mode -- the rewriter
		# truncates and replaces its content immediately -- and re-reading the
		# corpus for it would be one more read of a mutable leaf for a property
		# the snapshot already carries.
		cf_rec_staging="$CF_WORK/candidate.expected"
		rm -f -- "$cf_rec_staging"
		CF_STAGING=$cf_rec_staging
		if ! cp -p -- "$cf_rec_snapshot" "$cf_rec_staging"; then
			cf_error "$cf_rec_label: cannot stage a candidate in the working area"
			exit "$CF_EXIT_ENVIRONMENT"
		fi
	else
		# --- Atomic, unpredictable, same-directory staging --------------------
		#
		# `mktemp -d` in the RECORD'S OWN directory, and every word of that matters:
		#
		#   * ATOMIC. One operation creates the directory or fails. The predecessor
		#     tested a predictable path with `-e` and then wrote to it, which is two
		#     operations with a window between them: a concurrent writer of the same
		#     user could replace the path in that window, and `-e` cannot even see a
		#     DANGLING symbolic link, so the test could pass while the write
		#     followed a link somewhere else entirely.
		#   * UNPREDICTABLE. A name derived from the process identifier is guessable
		#     by anyone who can read the process table, so it can be planted in
		#     advance. This name cannot be predicted, so there is nothing to plant.
		#   * A DIRECTORY, created by the same call that names it. Directory
		#     creation never follows a final symbolic link -- it fails outright if
		#     the path exists as anything at all -- so link redirection is not
		#     defeated by a check but excluded by the operation. The staging FILE is
		#     then created inside a directory that was empty a moment ago and is
		#     readable and writable only by this user.
		#   * SAME DIRECTORY as the record, so installing it is one rename within
		#     one directory: the record is either fully updated or byte-identical to
		#     what it was, never half-written.
		#
		#   * THROUGH THE PINNED DIRECTORY. Created relative to the resolved
		#     directory this record's containment check approved, never relative to
		#     the name the record arrived under, so the directory the staging file
		#     lands in is the directory that was verified and not a second
		#     resolution of the same name taken later.
		#
		# The permissions still come from the record itself, by copying the SNAPSHOT
		# in as the staging file's first content, so the private umask cannot leak
		# into the corpus and a regenerated record keeps the mode it had. From the
		# snapshot rather than from the corpus entry because the snapshot already
		# carries that mode, and re-reading the entry for it would be a second read of
		# a mutable leaf -- one that could substitute the mode the published record
		# ends up with.
		if ! cf_rec_staging_rel=$(cf_stage_beside "$cf_rec_phys_dir" "$cf_rec_snapshot"); then
			cf_error "$cf_rec_label: cannot stage the record beside it"
			cf_detail "in: $cf_rec_phys_dir"
			cf_detail 'the corpus directory must be writable to regenerate a record; use --check'
			cf_detail 'to compare without writing'
			exit "$CF_EXIT_ENVIRONMENT"
		fi
		cf_rec_staging_dir="$cf_rec_phys_dir/$cf_rec_staging_rel"
		if ! cf_pin_staging "$cf_rec_staging_dir"; then
			cf_error "$cf_rec_label: cannot pin the staging directory after creating it"
			cf_detail "$CF_PIN_REASON"
			exit "$CF_EXIT_ENVIRONMENT"
		fi
		CF_STAGING=$cf_rec_staging_dir
		cf_rec_staging="$CF_STAGING_HANDLE/$CF_STAGING_LEAF"
	fi

	# The OLD record comes from the snapshot, never from a name that could have been
	# replaced since it was validated. The name the record arrived under is used
	# from here on for messages only.
	if cf_rewrite_record "$cf_rec_snapshot" "$cf_rec_golden" "$cf_rec_staging" "$cf_rec_bodylines"; then
		cf_rec_rc=0
	else
		cf_rec_rc=$?
	fi
	if [ "$cf_rec_rc" -ne 0 ]; then
		cf_report_structure_error "$cf_rec_rc" "$cf_rec_label"
	fi

	cf_rec_newbytes=$(awk '{ total += length($0) + 1 } END { print total + 0 }' < "$cf_rec_staging")
	if [ "$cf_rec_newbytes" -gt "$CF_RECORD_BYTES_MAX" ]; then
		cf_error "$cf_rec_label: the regenerated record would be $cf_rec_newbytes bytes, above the $CF_RECORD_BYTES_MAX-byte limit"
		cf_rec_abort
	fi

	# --- Install, or report ---------------------------------------------------
	CF_PROCESSED=$((CF_PROCESSED + 1))
	# Compared against the snapshot, which is the record this candidate was built
	# from. Comparing against the corpus entry instead would ask a different
	# question -- whether the candidate matches whatever is there NOW -- and would
	# answer "unchanged" for a record that had been replaced since it was read,
	# leaving the substituted bytes in place and reporting agreement.
	if cmp -s -- "$cf_rec_snapshot" "$cf_rec_staging"; then
		cf_release_staging
		cf_note "$cf_rec_label: $cf_rec_cells cell(s) agree; unchanged"
		return 0
	fi

	CF_CHANGED=$((CF_CHANGED + 1))
	if [ "$CF_CHECK" -eq 1 ]; then
		cf_release_staging
		cf_note "$cf_rec_label: $cf_rec_cells cell(s) agree; WOULD CHANGE (--check wrote nothing)"
		return 0
	fi
	# Retained before the rename because the rename consumes the staging file, and
	# what was published has to be compared against something afterwards.
	if ! cp -p -- "$cf_rec_staging" "$CF_WORK/published.expected"; then
		cf_error "$cf_rec_label: cannot retain a copy of the staged record for verification"
		cf_rec_abort
	fi
	# One rename inside one directory, so the record is either fully updated or
	# byte-identical to what it was -- never half-written. Performed relative to the
	# pinned directory, so the rename lands where the containment check looked.
	if ! cf_install_staged "$cf_rec_phys_dir" "$CF_STAGING_HANDLE" "$cf_rec_base"; then
		cf_error "$cf_rec_label: cannot install the regenerated record"
		cf_rec_abort
	fi
	# --- And confirm that what is now on disk is what was written -------------
	# The rename reported success; this asks the file system what actually exists at
	# the record's name now. A write is the one operation in this script whose
	# result nothing downstream re-reads, so an unverified success here is a claim
	# that the next run would inherit as fact -- and "the regenerated record" is the
	# file the whole suite then treats as the definition of correct output.
	if ! cf_verify_published "$cf_rec_phys_dir" "$cf_rec_base" \
		"$CF_WORK/published.expected" "$CF_CORPUS_DIR/$cf_rec_area"; then
		cf_error "$cf_rec_label: the installed record is not what was staged"
		cf_detail "$CF_VERIFY_REASON"
		cf_detail "named:    $cf_rec"
		cf_detail "resolved: $cf_rec_phys"
		cf_detail 'the staged bytes are kept for inspection at:'
		cf_detail "  $CF_WORK/published.expected"
		cf_rec_abort
	fi
	cf_release_staging
	cf_note "$cf_rec_label: $cf_rec_cells cell(s) agree; expected_stdout rewritten ($cf_rec_bodylines line(s))"
}

# =============================================================================
# Main loop.
# =============================================================================

if [ "$CF_CHECK" -eq 1 ]; then
	cf_note 'running in --check mode: no record will be written'
fi

# =============================================================================
# Whole-selection toolchain pre-flight.
#
# Every driver and runner the SELECTION will need is resolved and attested here,
# before the first record is processed and so before the first byte is written.
#
# Resolution is otherwise lazy, and laziness is wrong for a writing sweep: a
# driver that is missing or that fails attestation would then be discovered
# part-way through, with earlier records already rewritten and later ones not --
# a corpus in two states, which is worse than either. Attesting up front makes the
# run all-or-nothing with respect to the toolchain.
#
# The target union is read from the records themselves rather than assumed to be
# all four, so a selection that declares fewer targets does not demand a driver it
# will never use. Order is the declaration order of the first record that names
# each target, which keeps the log stable.
# =============================================================================

CF_PREFLIGHT_TARGETS=$(
	while IFS= read -r cf_pf_record; do
		[ -n "$cf_pf_record" ] || continue
		cat -- "$cf_pf_record"
	done < "$CF_RECORD_LIST" | awk -F '=' '
		/^targets[ \t]*=/ {
			n = split($2, cf_parts, ",")
			for (i = 1; i <= n; i++) {
				cf_t = cf_parts[i]
				gsub(/[ \t]/, "", cf_t)
				if (cf_t != "" && ! seen[cf_t]++) { order[++k] = cf_t }
			}
		}
		END { for (i = 1; i <= k; i++) { print order[i] } }
	'
)

if [ -n "$CF_PREFLIGHT_TARGETS" ]; then
	for cf_pf_target in $CF_PREFLIGHT_TARGETS; do
		cf_resolve_cell_tools "$cf_pf_target"
	done
fi

while IFS= read -r cf_main_record; do
	if [ -n "$cf_main_record" ]; then
		cf_process_record "$cf_main_record"
	fi
done < "$CF_RECORD_LIST"

cf_note "processed $CF_PROCESSED record(s); $CF_CHANGED changed"

if [ "$CF_CHECK" -eq 1 ] && [ "$CF_CHANGED" -gt 0 ]; then
	cf_error "--check: $CF_CHANGED record(s) would change"
	cf_detail 'rerun without --check to regenerate them, then review the diff before committing'
	exit "$CF_EXIT_PENDING"
fi

# Falling off the end is deliberate, and it is the successful path. A POSIX `if`
# whose condition is false and which carries no `else` yields status 0, so the
# script exits 0 here without an explicit `exit` -- which would otherwise leave
# the EXIT trap looking unreachable to a static analyser.
