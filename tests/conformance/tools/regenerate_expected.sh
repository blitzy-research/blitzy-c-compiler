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
# Writes:  the `expected_stdout` block of the selected `*.expected` records, and
#          one short-lived staging file beside each record being rewritten;
#          a private `mktemp -d` working area for compiling and running cells.
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
#   * A rewrite is staged to a sibling temporary file and installed with `mv`,
#     so a record is either fully updated or byte-identical to what it was.
#   * Running the script twice in a row produces no diff.
#   * No network access, no package installation, no path outside the corpus
#     directory and the private working area.
#
# PORTABILITY
# -----------
# POSIX shell only (`/bin/sh` is `dash` here), and POSIX `awk` only (`awk` is
# `mawk` here). No bashisms, no GNU-awk extensions, and no tool beyond the
# standard set: awk, sed, grep, printf, cat, cmp, cp, find, sort, tr, tail,
# basename, dirname, mktemp, mv, rm, mkdir and `command` -- plus `timeout` when
# it happens to be present, which it need not be. `binutils` (readelf, objdump,
# nm), `creduce`, `python`, `perl` and `jq` are deliberately NOT dependencies;
# `clang` is merely the last candidate the native-driver probe tries, and is
# never required.
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

# --- Directories excluded from record enumeration ----------------------------
# The three corpus companions that are siblings of the area directories and are
# never walked for records. `findings/F-NNNN-*/reproducer.expected` IS a valid
# record, but it is a curated, human-promoted deliverable: regenerating it would
# rewrite a captured artifact. `support/` holds the flag probe's only fixture,
# and `tools/` holds this script.
#
# This predicate and the prune clause in cf_enumerate_records must name the same
# three directories.
cf_is_excluded_dir() {
	case $1 in
	findings | support | tools) return 0 ;;
	esac
	return 1
}

# --- Mutable state -----------------------------------------------------------
CF_WORK=''         # private working area, created by mktemp -d
CF_STAGING=''      # staging file currently in flight, removed on any exit
CF_PROCESSED=0
CF_CHANGED=0

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

# Echo at most $2 lines of file $1 to stderr, indented, for a diagnostic. The
# file arrives by redirection rather than as an operand, so a name beginning
# with a hyphen can never be read as an option.
cf_detail_file() {
	sed -n "1,$2p" < "$1" | while IFS= read -r cf_detail_file_line; do
		cf_detail "| $cf_detail_file_line"
	done
}

# =============================================================================
# Cleanup.
#
# Cleanup sits on EXIT alone. A handler installed on INT or TERM *alongside*
# EXIT is a trap for the author: the handler runs, returns, and the shell
# RESUMES at the next command -- now with the working area already deleted, so
# every later redirection writes into a path that no longer exists and the
# script keeps going after it was asked to stop. Each signal handler therefore
# cleans up, restores the default disposition, and re-raises the same signal at
# this shell, so the script dies from it and the caller sees the conventional
# 128 + signal status. This mirrors the pattern `tests/conformance/README.md`
# publishes under "Reproducing a cell by hand".
# =============================================================================

cf_cleanup() {
	if [ -n "$CF_STAGING" ] && [ -e "$CF_STAGING" ]; then
		rm -f -- "$CF_STAGING"
	fi
	CF_STAGING=''
	if [ -n "$CF_WORK" ] && [ -d "$CF_WORK" ]; then
		rm -rf -- "$CF_WORK"
	fi
	CF_WORK=''
}

trap cf_cleanup EXIT
trap 'cf_cleanup; trap - HUP;  kill -HUP  $$' HUP
trap 'cf_cleanup; trap - INT;  kill -INT  $$' INT
trap 'cf_cleanup; trap - TERM; kill -TERM $$' TERM

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

--area and --program are mutually exclusive. With neither, every record in the
corpus is regenerated.

Environment (all optional; the same names the harness uses):
  BCC_REF_CC                  native reference C compiler   [gcc, cc, clang]
  BCC_REF_CC_I686             i686 reference driver         [i686-linux-gnu-gcc]
  BCC_REF_CC_AARCH64          aarch64 reference driver      [aarch64-linux-gnu-gcc]
  BCC_REF_CC_RISCV64          riscv64 reference driver      [riscv64-linux-gnu-gcc]
  BCC_QEMU_I386               i686 runner       [qemu-i386, qemu-i386-static]
  BCC_QEMU_AARCH64            aarch64 runner    [qemu-aarch64, qemu-aarch64-static]
  BCC_QEMU_RISCV64            riscv64 runner    [qemu-riscv64, qemu-riscv64-static]
  BCC_CONFORMANCE_TIMEOUT_SECS  per-compile and per-run budget [30]

No -std flag is ever passed, so the reference driver's own DEFAULT language mode
decides what it compiles. The driver of record is the gnu17 one; on a host whose
unversioned `gcc` is a later major version defaulting to a later mode, name the
gnu17 driver through BCC_REF_CC rather than relying on the probe order.

BCC_BIN is deliberately NOT read: a golden record seeded from the compiler under
test would certify that compiler against itself.

Exit codes: 0 success, 2 usage, 3 environment, 4 record defect,
            5 capture defect, 6 --check found a record that would change.
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

cf_require_value() {
	# $1 = option name, $2 = remaining argument count after the option
	if [ "$2" -lt 2 ]; then
		cf_usage_error "$1 requires a value"
	fi
}

while [ "$#" -gt 0 ]; do
	case $1 in
	--area)
		cf_require_value --area "$#"
		shift
		CF_ARG_AREA=$1
		;;
	--area=*)
		CF_ARG_AREA=${1#--area=}
		;;
	--program)
		cf_require_value --program "$#"
		shift
		CF_ARG_PROGRAM=$1
		;;
	--program=*)
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

if [ -n "$CF_ARG_AREA" ] && [ -n "$CF_ARG_PROGRAM" ]; then
	cf_usage_error '--area and --program are mutually exclusive; --program already names its area'
fi

# A name component of an area or a program: the corpus spells both with digits,
# lowercase letters and underscores. Rejecting anything else is what keeps a
# selector from reaching outside the corpus through "..", a slash or a glob.
cf_validate_name_component() {
	# $1 = value, $2 = what it is (for the message)
	if [ -z "$1" ]; then
		cf_usage_error "the $2 is empty"
	fi
	case $1 in
	. | ..)
		cf_usage_error "the $2 \"$1\" is a directory reference, not a name"
		;;
	*[!A-Za-z0-9_-]*)
		cf_usage_error "the $2 \"$1\" contains a character outside [A-Za-z0-9_-]"
		;;
	esac
}

cf_validate_area_name() {
	cf_validate_name_component "$1" "$2"
	if cf_is_excluded_dir "$1"; then
		cf_usage_error "the $2 \"$1\" is a corpus companion directory, not a feature area; it holds no regenerable record"
	fi
	case $1 in
	[0-9][0-9]_*) ;;
	*)
		cf_usage_error "the $2 \"$1\" is not a feature-area directory; areas are named NN_<name>, e.g. 01_integer_conversions"
		;;
	esac
}

CF_SELECT_AREA=''
CF_SELECT_PROGRAM=''

if [ -n "$CF_ARG_AREA" ]; then
	cf_validate_area_name "$CF_ARG_AREA" 'area'
	CF_SELECT_AREA=$CF_ARG_AREA
fi

if [ -n "$CF_ARG_PROGRAM" ]; then
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
	cf_validate_name_component "$CF_SELECT_PROGRAM" 'program in --program'
fi

# =============================================================================
# Locate the corpus relative to this script, never relative to the caller.
# =============================================================================

CF_TOOLS_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd) || {
	cf_error "cannot resolve the directory holding this script"
	exit "$CF_EXIT_ENVIRONMENT"
}
CF_CORPUS_DIR=$(CDPATH='' cd -- "$CF_TOOLS_DIR/.." && pwd) || {
	cf_error "cannot resolve the corpus directory above \"$CF_TOOLS_DIR\""
	exit "$CF_EXIT_ENVIRONMENT"
}

if [ "$(basename -- "$CF_TOOLS_DIR")" != 'tools' ]; then
	cf_error "this script must live in tests/conformance/tools/, but it was found in \"$CF_TOOLS_DIR\""
	exit "$CF_EXIT_ENVIRONMENT"
fi
if [ ! -f "$CF_CORPUS_DIR/README.md" ]; then
	cf_error "\"$CF_CORPUS_DIR\" does not look like the conformance corpus: README.md is missing"
	cf_detail 'the corpus contract is tests/conformance/README.md; the layout is expected to be unchanged'
	exit "$CF_EXIT_ENVIRONMENT"
fi

# =============================================================================
# Timeout budget. A bound that fires reports 124, which is why the plain
# spelling is used and never `timeout -k`: -k reports 125, colliding with the
# 0..125 range a cell may legitimately produce as its own exit status.
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

if command -v timeout > /dev/null 2>&1; then
	CF_HAVE_TIMEOUT=1
else
	CF_HAVE_TIMEOUT=0
	cf_note "no timeout utility found; compiles and runs are unbounded (a hang will not self-terminate)"
fi

# Run "$@" under the per-cell bound when one is available.
cf_bound() {
	if [ "$CF_HAVE_TIMEOUT" -eq 1 ]; then
		timeout "$CF_TIMEOUT_SECS" "$@"
	else
		"$@"
	fi
}

# =============================================================================
# Private working area. `mktemp -d` honours TMPDIR and creates a directory that
# did not previously exist, with mode 0700, under an unpredictable name -- so
# nothing can already be sitting at the paths written below. Deliberately NOT
# under target/: .gitignore names exactly three paths there, and a fourth would
# be untracked-but-unignored and would dirty `git status`.
# =============================================================================

umask 077
CF_WORK=$(mktemp -d) || {
	cf_error 'cannot create a private working directory with mktemp -d'
	exit "$CF_EXIT_ENVIRONMENT"
}
mkdir -p -- "$CF_WORK/cell"

# =============================================================================
# Record enumeration.
#
# `-type f` already excludes symbolic links, because find does not follow them
# and tests the link itself. The prune clause must name the same three
# directories as cf_is_excluded_dir above. Output is sorted under LC_ALL=C so
# the processing order is identical on every run and on every machine.
# =============================================================================

cf_enumerate_records() {
	# $1 = directory to walk
	find "$1" \
		\( -path "$CF_CORPUS_DIR/findings" \
		-o -path "$CF_CORPUS_DIR/support" \
		-o -path "$CF_CORPUS_DIR/tools" \) -prune \
		-o -type f -name '*.expected' -print | sort
}

CF_RECORD_LIST="$CF_WORK/records.list"

if [ -n "$CF_SELECT_PROGRAM" ]; then
	CF_ONE_RECORD="$CF_CORPUS_DIR/$CF_SELECT_AREA/$CF_SELECT_PROGRAM.expected"
	if [ ! -d "$CF_CORPUS_DIR/$CF_SELECT_AREA" ]; then
		cf_error "no such area directory: $CF_SELECT_AREA"
		cf_detail "looked for: $CF_CORPUS_DIR/$CF_SELECT_AREA"
		exit "$CF_EXIT_USAGE"
	fi
	if [ ! -f "$CF_ONE_RECORD" ] || [ -L "$CF_ONE_RECORD" ]; then
		cf_error "no such expectation record: $CF_SELECT_AREA/$CF_SELECT_PROGRAM.expected"
		cf_detail "looked for: $CF_ONE_RECORD"
		exit "$CF_EXIT_USAGE"
	fi
	printf '%s\n' "$CF_ONE_RECORD" > "$CF_RECORD_LIST"
elif [ -n "$CF_SELECT_AREA" ]; then
	if [ ! -d "$CF_CORPUS_DIR/$CF_SELECT_AREA" ]; then
		cf_error "no such area directory: $CF_SELECT_AREA"
		cf_detail "looked for: $CF_CORPUS_DIR/$CF_SELECT_AREA"
		exit "$CF_EXIT_USAGE"
	fi
	cf_enumerate_records "$CF_CORPUS_DIR/$CF_SELECT_AREA" > "$CF_RECORD_LIST"
	if [ ! -s "$CF_RECORD_LIST" ]; then
		cf_error "area $CF_SELECT_AREA holds no *.expected record"
		exit "$CF_EXIT_USAGE"
	fi
else
	cf_enumerate_records "$CF_CORPUS_DIR" > "$CF_RECORD_LIST"
	if [ ! -s "$CF_RECORD_LIST" ]; then
		cf_error "no *.expected record found beneath $CF_CORPUS_DIR"
		exit "$CF_EXIT_RECORD"
	fi
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

# Print the value of scalar key $2 from record $1. Exit status 3 when the key is
# absent, so the caller can distinguish absent from empty.
cf_read_scalar() {
	awk -v want="$2" '
	BEGIN { inside = 0; found = 0 }
	{
		if (inside) {
			if ($0 == "END") { inside = 0 }
			next
		}
		trimmed = $0
		sub(/^[ \t]+/, "", trimmed)
		sub(/[ \t]+$/, "", trimmed)
		if (trimmed == "" || substr(trimmed, 1, 1) == "#") { next }
		opener = index($0, "<<")
		equals = index($0, "=")
		if (opener > 0 && (equals == 0 || opener < equals)) { inside = 1; next }
		if (equals == 0) { next }
		key = substr($0, 1, equals - 1)
		value = substr($0, equals + 1)
		sub(/^[ \t]+/, "", key)
		sub(/[ \t]+$/, "", key)
		sub(/^[ \t]+/, "", value)
		sub(/[ \t]+$/, "", value)
		if (key == want) { print value; found = 1; exit }
	}
	END { if (! found) { exit 3 } }
	' < "$1"
}

# Read scalar key $2 of record $1 into CF_FIELD, or fail loudly. Status 3 from
# the reader means the key is absent; anything else means awk itself failed, and
# the two are reported differently so a broken environment is never mistaken for
# a defective record.
CF_FIELD=''
cf_require_scalar() {
	# $1 = record path, $2 = key, $3 = record label
	if CF_FIELD=$(cf_read_scalar "$1" "$2"); then
		return 0
	else
		cf_require_scalar_rc=$?
	fi
	if [ "$cf_require_scalar_rc" -ne 3 ]; then
		cf_error "$3: awk failed while reading the key \"$2\" (status $cf_require_scalar_rc)"
		exit "$CF_EXIT_RECORD"
	fi
	cf_error "$3: the required key \"$2\" is missing"
	cf_detail 'every record declares program, area, description, targets, opt_levels,'
	cf_detail 'shared_flags, the three command templates, expect_exit, the three oracle'
	cf_detail 'switches, ub_notes and expected_stdout; see tests/conformance/README.md'
	exit "$CF_EXIT_RECORD"
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

# Resolve a tool into CF_CHOSEN, or fail loudly.
#   $1 = value of the override variable (may be empty)
#   $2 = name of the override variable, for the message
#   $3 = human description of the tool
#   $4 = install hint, printed as text and NEVER executed
#   $5.. = candidates, probed in order
cf_choose() {
	cf_choose_value=$1
	cf_choose_var=$2
	cf_choose_desc=$3
	cf_choose_hint=$4
	shift 4
	if [ -n "$cf_choose_value" ]; then
		if command -v "$cf_choose_value" > /dev/null 2>&1; then
			CF_CHOSEN=$cf_choose_value
			return 0
		fi
		cf_error "$cf_choose_var names \"$cf_choose_value\", which is not an executable on PATH"
		cf_detail "needed as: $cf_choose_desc"
		exit "$CF_EXIT_ENVIRONMENT"
	fi
	for cf_choose_candidate in "$@"; do
		if command -v "$cf_choose_candidate" > /dev/null 2>&1; then
			CF_CHOSEN=$cf_choose_candidate
			return 0
		fi
	done
	cf_error "no $cf_choose_desc found"
	cf_detail "probed, in order: $*"
	cf_detail "set $cf_choose_var to name one, or install it yourself -- this script never installs anything:"
	cf_detail "    $cf_choose_hint"
	exit "$CF_EXIT_ENVIRONMENT"
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
			cf_choose "${BCC_REF_CC:-}" 'BCC_REF_CC' \
				'native reference C compiler for x86_64-linux-gnu' \
				'apt-get install -y gcc   (or gcc-13, where the unversioned gcc defaults to a later language mode)' \
				gcc cc clang
			CF_CC_X86_64=$CF_CHOSEN
			cf_note "reference compiler for x86_64-linux-gnu: $CF_CC_X86_64"
		fi
		CF_CELL_CC=$CF_CC_X86_64
		CF_CELL_RUNNER=''
		;;
	i686)
		if [ -z "$CF_CC_I686" ]; then
			cf_choose "${BCC_REF_CC_I686:-}" 'BCC_REF_CC_I686' \
				'i686 reference cross driver' \
				'apt-get install -y gcc-i686-linux-gnu libc6-dev-i386-cross   (libc6-dev-i386 on older packaging)' \
				i686-linux-gnu-gcc
			CF_CC_I686=$CF_CHOSEN
			cf_note "reference compiler for i686-linux-gnu: $CF_CC_I686"
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
			cf_choose "${BCC_REF_CC_AARCH64:-}" 'BCC_REF_CC_AARCH64' \
				'aarch64 reference cross driver' \
				'apt-get install -y gcc-aarch64-linux-gnu libc6-dev-arm64-cross' \
				aarch64-linux-gnu-gcc
			CF_CC_AARCH64=$CF_CHOSEN
			cf_note "reference compiler for aarch64-linux-gnu: $CF_CC_AARCH64"
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
			cf_choose "${BCC_REF_CC_RISCV64:-}" 'BCC_REF_CC_RISCV64' \
				'riscv64 reference cross driver' \
				'apt-get install -y gcc-riscv64-linux-gnu libc6-dev-riscv64-cross' \
				riscv64-linux-gnu-gcc
			CF_CC_RISCV64=$CF_CHOSEN
			cf_note "reference compiler for riscv64-linux-gnu: $CF_CC_RISCV64"
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

	cf_capture_bin="$CF_WORK/cell/program"
	cf_capture_cc_err="$CF_WORK/cell/compile.stderr"
	cf_capture_run_err="$CF_WORK/cell/run.stderr"
	rm -f -- "$cf_capture_bin" "$cf_capture_cc_err" "$cf_capture_run_err"

	# stdin comes from /dev/null so a child can never consume the record or cell
	# list the enclosing read loops are iterating over.
	if cf_bound "$CF_CELL_CC" "$cf_capture_opt" -static "$cf_capture_src" \
		-o "$cf_capture_bin" < /dev/null > "$cf_capture_cc_err" 2>&1; then
		cf_capture_status=0
	else
		cf_capture_status=$?
	fi
	if [ "$cf_capture_status" -ne 0 ]; then
		if [ "$cf_capture_status" -eq 124 ] && [ "$CF_HAVE_TIMEOUT" -eq 1 ]; then
			cf_error "$cf_capture_rec: compiling $cf_capture_label reached the ${CF_TIMEOUT_SECS}s bound"
			cf_detail 'raise BCC_CONFORMANCE_TIMEOUT_SECS if the bound is simply too tight'
		else
			cf_error "$cf_capture_rec: the reference compiler failed on $cf_capture_label (status $cf_capture_status)"
			cf_detail "command: $CF_CELL_CC $cf_capture_opt -static $cf_capture_src -o <out>"
			cf_detail_file "$cf_capture_cc_err" 20
		fi
		exit "$CF_EXIT_CAPTURE"
	fi
	if [ ! -f "$cf_capture_bin" ]; then
		cf_error "$cf_capture_rec: the reference compiler reported success on $cf_capture_label but produced no output file"
		exit "$CF_EXIT_CAPTURE"
	fi

	if [ -z "$CF_CELL_RUNNER" ]; then
		if cf_bound "$cf_capture_bin" \
			< /dev/null > "$cf_capture_out" 2> "$cf_capture_run_err"; then
			cf_capture_status=0
		else
			cf_capture_status=$?
		fi
	else
		if cf_bound "$CF_CELL_RUNNER" "$cf_capture_bin" \
			< /dev/null > "$cf_capture_out" 2> "$cf_capture_run_err"; then
			cf_capture_status=0
		else
			cf_capture_status=$?
		fi
	fi

	if [ "$cf_capture_status" -eq 124 ] && [ "$CF_HAVE_TIMEOUT" -eq 1 ]; then
		cf_error "$cf_capture_rec: running $cf_capture_label reached the ${CF_TIMEOUT_SECS}s bound"
		cf_detail 'status 124 is the bound firing, not the program choosing to exit with 124'
		cf_detail 'raise BCC_CONFORMANCE_TIMEOUT_SECS if the bound is simply too tight'
		exit "$CF_EXIT_CAPTURE"
	fi
	if [ "$cf_capture_status" -ne "$cf_capture_expect" ]; then
		cf_error "$cf_capture_rec: $cf_capture_label exited with $cf_capture_status but the record declares expect_exit = $cf_capture_expect"
		cf_detail 'the status is as much of the expectation as the bytes are, so stdout captured'
		cf_detail 'from a run that ended the wrong way is not a golden record'
		if [ "$cf_capture_status" -gt 128 ]; then
			cf_detail "a status above 128 means the runner reported signal $((cf_capture_status - 128)); that is not a normal exit of the same number"
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
	if [ -n "$CF_STAGING" ] && [ -e "$CF_STAGING" ]; then
		rm -f -- "$CF_STAGING"
	fi
	CF_STAGING=''
	cf_detail 'nothing was written; the record is unchanged'
	exit "$CF_EXIT_RECORD"
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

# Audit the record's heredoc structure without writing anything, so a defective
# record is refused BEFORE a single compile is spent on it. Returns the same
# status codes as cf_rewrite_record.
cf_audit_record_structure() {
	awk '
	BEGIN { state = 0; seen = 0; fatal = 0 }
	state == 1 { if ($0 == "END") { state = 0 } next }
	{
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
			if (terminator != "END") { fatal = 7; exit fatal }
			if (key == "expected_stdout") {
				if (seen) { fatal = 4; exit fatal }
				seen = 1
			}
			state = 1
			next
		}
	}
	END {
		if (fatal) { exit fatal }
		if (state != 0) { exit 5 }
		if (! seen) { exit 6 }
	}
	' < "$1"
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

	if [ ! -f "$cf_rec" ] || [ -L "$cf_rec" ]; then
		cf_error "$cf_rec_label: the record is not a regular file (a symbolic link is refused)"
		exit "$CF_EXIT_RECORD"
	fi
	if [ ! -f "$cf_rec_src" ] || [ -L "$cf_rec_src" ]; then
		cf_error "$cf_rec_label: no sibling program \"$cf_rec_stem.c\" beside the record"
		cf_detail 'the corpus pairs every record 1:1 with the program it describes; a record'
		cf_detail 'without a program describes nothing and can regenerate nothing'
		exit "$CF_EXIT_RECORD"
	fi
	if grep -q -- "$CF_CR" < "$cf_rec"; then
		cf_error "$cf_rec_label: the record contains a CARRIAGE RETURN"
		cf_detail 'this line-oriented rewrite requires LF endings; convert the file first'
		exit "$CF_EXIT_RECORD"
	fi

	# --- Structure, before any compile is spent -------------------------------
	if cf_audit_record_structure "$cf_rec"; then
		cf_rec_rc=0
	else
		cf_rec_rc=$?
	fi
	if [ "$cf_rec_rc" -ne 0 ]; then
		cf_report_structure_error "$cf_rec_rc" "$cf_rec_label"
	fi

	# --- Identity -------------------------------------------------------------
	cf_require_scalar "$cf_rec" 'program' "$cf_rec_label"
	if [ "$CF_FIELD" != "$cf_rec_stem" ]; then
		cf_error "$cf_rec_label: program = \"$CF_FIELD\" but the file stem is \"$cf_rec_stem\""
		cf_detail 'the cheapest guard there is against a record copied from another program'
		cf_detail 'and only partly edited; fix the key or rename the file'
		exit "$CF_EXIT_RECORD"
	fi
	cf_require_scalar "$cf_rec" 'area' "$cf_rec_label"
	if [ "$CF_FIELD" != "$cf_rec_area" ]; then
		cf_error "$cf_rec_label: area = \"$CF_FIELD\" but the containing directory is \"$cf_rec_area\""
		exit "$CF_EXIT_RECORD"
	fi

	# --- Expected exit status --------------------------------------------------
	cf_require_scalar "$cf_rec" 'expect_exit' "$cf_rec_label"
	cf_rec_expect=$CF_FIELD
	case $cf_rec_expect in
	'' | *[!0-9]*)
		cf_error "$cf_rec_label: expect_exit = \"$cf_rec_expect\" is not a whole number"
		exit "$CF_EXIT_RECORD"
		;;
	esac
	if [ "$cf_rec_expect" -gt "$CF_EXPECT_EXIT_MAX" ]; then
		cf_error "$cf_rec_label: expect_exit = $cf_rec_expect is above $CF_EXPECT_EXIT_MAX"
		cf_detail 'the operating system truncates a larger value, so it could never be observed'
		exit "$CF_EXIT_RECORD"
	fi

	# --- Matrix ---------------------------------------------------------------
	# The record's own declaration is honoured exactly and never widened.
	cf_require_scalar "$cf_rec" 'targets' "$cf_rec_label"
	cf_rec_targets_raw=$CF_FIELD
	cf_require_scalar "$cf_rec" 'opt_levels' "$cf_rec_label"
	cf_rec_opts_raw=$CF_FIELD

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
		cf_rec_staging="$CF_WORK/candidate.expected"
		rm -f -- "$cf_rec_staging"
		if ! cp -p -- "$cf_rec" "$cf_rec_staging"; then
			cf_error "$cf_rec_label: cannot stage a candidate in the working area"
			exit "$CF_EXIT_ENVIRONMENT"
		fi
	else
		cf_rec_staging="$cf_rec.regen.$$"
		if [ -e "$cf_rec_staging" ]; then
			cf_error "$cf_rec_label: a staging file is already at \"$cf_rec_staging\""
			cf_detail 'it is refused rather than reused, so a planted path cannot redirect a write;'
			cf_detail 'remove the leftover from an interrupted run and try again'
			exit "$CF_EXIT_RECORD"
		fi
		CF_STAGING=$cf_rec_staging
		# Seed the staging file FROM the record so it inherits the record's exact
		# permissions; the redirection below then replaces only its contents. This
		# is why the private working area's 0700 umask cannot leak into the corpus.
		if ! cp -p -- "$cf_rec" "$cf_rec_staging"; then
			cf_error "$cf_rec_label: cannot create the staging file \"$cf_rec_staging\""
			cf_detail 'the corpus directory must be writable to regenerate a record; use --check'
			cf_detail 'to compare without writing'
			exit "$CF_EXIT_ENVIRONMENT"
		fi
	fi

	if cf_rewrite_record "$cf_rec" "$cf_rec_golden" "$cf_rec_staging" "$cf_rec_bodylines"; then
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
	if cmp -s -- "$cf_rec" "$cf_rec_staging"; then
		rm -f -- "$cf_rec_staging"
		CF_STAGING=''
		cf_note "$cf_rec_label: $cf_rec_cells cell(s) agree; unchanged"
		return 0
	fi

	CF_CHANGED=$((CF_CHANGED + 1))
	if [ "$CF_CHECK" -eq 1 ]; then
		rm -f -- "$cf_rec_staging"
		cf_note "$cf_rec_label: $cf_rec_cells cell(s) agree; WOULD CHANGE (--check wrote nothing)"
		return 0
	fi
	# One rename inside one directory, so the record is either fully updated or
	# byte-identical to what it was -- never half-written.
	if ! mv -- "$cf_rec_staging" "$cf_rec"; then
		cf_error "$cf_rec_label: cannot install the regenerated record"
		cf_rec_abort
	fi
	CF_STAGING=''
	cf_note "$cf_rec_label: $cf_rec_cells cell(s) agree; expected_stdout rewritten ($cf_rec_bodylines line(s))"
}

# =============================================================================
# Main loop.
# =============================================================================

if [ "$CF_CHECK" -eq 1 ]; then
	cf_note 'running in --check mode: no record will be written'
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
