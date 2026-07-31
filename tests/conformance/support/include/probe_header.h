/*
 * probe_header.h - the differential conformance suite's ONE AND ONLY fixture.
 *
 * PURPOSE
 *   This header exists for exactly one reason: so that
 *   tests/conformance_harness/flagprobe.rs can PROVE that a command-line include
 *   directory (-I) is genuinely searched, rather than assuming that it is. That is
 *   numbered requirement 3 of the conformance task, which admits only flags both
 *   compilers honour with the same meaning, and which asks that flag handling be
 *   verified rather than assumed.
 *
 * THE ASSERTION IS TWO-SIDED, AND THE NEGATIVE HALF IS THE LOAD-BEARING ONE
 *   POSITIVE: <cc> -I tests/conformance/support/include probe.c -o probe
 *             must COMPILE, run, and print a value that came from THIS header.
 *   NEGATIVE: <cc> probe.c -o probe                            (no -I)
 *             must FAIL TO COMPILE, this header being reachable by no other route.
 *   Both halves must hold for BOTH compilers, bcc and the reference compiler. A
 *   one-sided check would still pass if the compiler had found the header by some
 *   other route, and would therefore verify nothing at all.
 *
 * UNIQUENESS IS A PRECONDITION, NOT A TIDINESS PREFERENCE
 *   No copy of this file may exist anywhere else in the repository. A duplicate on any
 *   default include search path would silently make the negative half vacuous while
 *   the probe carried on reporting success. This directory is NOT the repository's
 *   root-level include/, which holds bcc's bundled freestanding headers and is
 *   read-only and out of scope.
 *
 * SELF-CONTAINED BY DESIGN
 *   This header includes NO other header, and in particular not <stdio.h>: bcc ships
 *   no stdio.h at all, bundling exactly nine freestanding headers (stddef.h, stdint.h,
 *   stdarg.h, stdbool.h, limits.h, float.h, stdalign.h, stdnoreturn.h, iso646.h). An
 *   #include <stdio.h> here would fail under bcc while succeeding under the reference
 *   compiler, manufacturing a spurious divergence caused by this fixture rather than
 *   by the compiler under test. It also declares NO libc function, and in particular
 *   not printf, which every probe and corpus program hand-declares for itself as
 *   int printf(const char *, ...);
 *
 * PUBLIC SURFACE - EXACT NAMES AND VALUES. DO NOT RENAME.
 *   BCC_CONFORMANCE_PROBE_HEADER_H   include guard
 *   BCC_PROBE_HEADER_VALUE           4242             print with %d
 *   BCC_PROBE_HEADER_NAME            "probe_header"   print with %s
 *   4242 is a plain decimal literal in int range: no suffix, no cast, no arithmetic,
 *   and never derived from a line, file, date, time or counter macro, from the
 *   environment, or from anything target-dependent. It is therefore byte-identical on
 *   x86-64, i686, AArch64 and RISC-V 64 at every optimization level. Object-like
 *   macros carry the payload rather than typed constants because a macro cannot
 *   provoke an unused-constant diagnostic, which the strict -Werror audit gate would
 *   turn into a fatal error manufactured by this fixture itself.
 *
 * GUARD STYLE
 *   #ifndef/#define/#endif is used deliberately and #pragma once is NOT used. Only
 *   directives documented as implemented appear here: #if/#ifdef/#ifndef/#elif/#else/
 *   #endif are documented as implemented, whereas #pragma is dispatched generically
 *   only and "#pragma once" is documented nowhere. A pragma that were accepted and
 *   ignored would leave this header with no guard at all, and one that warned would be
 *   fatal under -Werror; #ifndef has neither failure mode.
 */

#ifndef BCC_CONFORMANCE_PROBE_HEADER_H
#define BCC_CONFORMANCE_PROBE_HEADER_H

/* Primary payload: printed by the -I probe program with %d. */
#define BCC_PROBE_HEADER_VALUE 4242

/* Secondary payload, printed with %s: a second, orthogonal way to show the include. */
#define BCC_PROBE_HEADER_NAME "probe_header"

#endif /* BCC_CONFORMANCE_PROBE_HEADER_H */
