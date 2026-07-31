/*
 * probe_header.h - the fixture for the -I flag probe.
 *
 * The probe compiles a program with this directory on the include search path and
 * again without it: the first must compile and print a value that came from here,
 * the second must fail.  The negative half is the load-bearing one, so no copy of
 * this file may exist anywhere else in the repository - a duplicate on a default
 * search path would make that half vacuous while the probe still reported success.
 *
 * The header therefore includes nothing and declares no libc function; in
 * particular it does not include <stdio.h>, which bcc does not ship - its bundled
 * set is the nine required freestanding headers plus a bonus stdatomic.h, ten files
 * in all (docs/project-guide.md line 212), and no standard I/O header is among them
 * - and which would fail under bcc while succeeding under the reference compiler.
 * This directory is not that bundled set: it is the fixture directory the probe
 * puts on the include search path, and it holds this file alone.  The payloads
 * are object-like macros rather than typed constants because a macro cannot provoke
 * an unused-constant diagnostic, and the guard is spelled with #ifndef rather than
 * #pragma once because a pragma that was accepted and ignored would leave the
 * header unguarded - either would be fatal under the -Werror audit gate.
 */

#ifndef BCC_CONFORMANCE_PROBE_HEADER_H
#define BCC_CONFORMANCE_PROBE_HEADER_H

#define BCC_PROBE_HEADER_VALUE 4242

#define BCC_PROBE_HEADER_NAME "probe_header"

#endif /* BCC_CONFORMANCE_PROBE_HEADER_H */
