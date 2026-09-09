# DECLARE compatibility audit

Date: 2026-09-06. This audit separates the original PPLC 3.40 compiler's
observed behavior, conclusions from PCBoard runtime source, and IcyBoard's
regression coverage. It also defines the deliberately stricter language-400
contract.

## Evidence and limits

- **Original compiler:** 23 authored sources in [declare](declare) were compiled
  with PPLC 3.40 under DOSBox-X, without `/DISARR`. There are **13 accepted and
  10 rejected** sources. This is compiler evidence, not a PCBoard runtime run.
- The first 21 logs and the 13 successful PPEs are in
  [target/declare-oracle](../target/declare-oracle). The two additional
  unused-formal probes and a repeated declaration-only control are in
  [target/legacy-parameter-oracle](../target/legacy-parameter-oracle).
  Original binaries and generated PPEs are local artifacts, not
  required golden files to distribute.
- [declare_compatibility.rs](../crates/icy_board_engine/tests/declare_compatibility.rs)
  records **21 cases**, not 23. Its golden signatures describe routine kind,
  parameter count, parameter types/ranks/bounds, procedure `pass_flags`, and
  function result type/rank/bounds. IcyBoard PPEs are serialized and read back
  before comparison. A separate optional check reads original logs and decodes
  original PPEs when present; missing local artifacts are skipped.
- The IcyBoard legacy compiler matrix is **language/runtime 340/340, 340/400,
  and 350/400**. These combinations are compared against the authored PPLC
  **3.40** observations; they are not runs of an original 3.50 compiler and do
  not establish behavior for every earlier PPLC release.
- [legacy_parameter_syntax.rs](../crates/icy_board_engine/tests/legacy_parameter_syntax.rs)
  supplies **8 parser regression tests**, including the two additional oracle
  probes. Its generated rank-3 cases and earlier-language checks extend
  IcyBoard coverage; they are not additional original-compiler observations.
- This is a bounded acceptance/rejection and decoded-signature audit, **not
  universal 1:1 compatibility, identical diagnostic text, or byte-identical
  PPE output**. No original runtime output was captured for these 23 probes.
  The full engine, compiler, decompiler and LSP test suites passed after the
  corrections (engine library: 1,567 passed, 5 ignored).

## Original PPLC 3.40 results

The first 21 rows have matching local logs in
[target/declare-oracle](../target/declare-oracle). The final two logs are linked
separately below. Accepted signatures are recorded in the
[golden case table](../crates/icy_board_engine/tests/declare_compatibility.rs);
an emitted result slot must not be confused with procedure `pass_flags`.

| Authored probe | Original result | Observation |
| :--- | :--- | :--- |
| [count](declare/count.pps) | Reject | Procedure declaration and implementation parameter counts differ. |
| [dim](declare/dim.pps) | Reject | Rank-2 implementation formal fails while parsing the header, before body dimension checking. |
| [dim_bound](declare/dim_bound.pps) | Reject | Rank-1 implementation formal is used without an index in the body; this is not rejection of the declaration's different bound. |
| [dim_bound_index](declare/dim_bound_index.pps) | Accept | Declared bound 2, implemented bound 3: emitted formal is `INTEGER`, rank 1, upper bound 3; scalar actual `1` is accepted. |
| [dim_decl_only](declare/dim_decl_only.pps) | Accept | Rank-2 declaration with scalar implementation emits a scalar `INTEGER` formal. |
| [dim_scalar](declare/dim_scalar.pps) | Reject | Scalar declaration, rank-1 implementation: the unindexed body use lacks an array subscript. |
| [dim_scalar_index](declare/dim_scalar_index.pps) | Accept | Scalar declaration, rank-1 implementation: emitted formal retains rank 1 and upper bound 3; scalar actual `1` is accepted. |
| [function_count](declare/function_count.pps) | Reject | Function declaration and implementation parameter counts differ. |
| [function_type](declare/function_type.pps) | Accept | Declared `INTEGER` parameter becomes implementation `STRING`; result remains `INTEGER`. |
| [kind_func](declare/kind_func.pps) | Reject | Declared `FUNCTION`, implemented `PROCEDURE`. |
| [kind_proc](declare/kind_proc.pps) | Accept | Declared `PROCEDURE`, implemented `FUNCTION`: emitted kind is procedure, with no function result slot. |
| [kind_proc_var](declare/kind_proc_var.pps) | Accept | Same kind normalization, but implementation `VAR` survives: procedure `pass_flags = 1`, no result slot. |
| [param_names](declare/param_names.pps) | Accept | Parameter names need not match. |
| [return](declare/return.pps) | Accept | Declared `INTEGER` result becomes implementation `STRING`. |
| [return_reverse](declare/return_reverse.pps) | Accept | Declared `STRING` result becomes implementation `INTEGER`. |
| [type](declare/type.pps) | Accept | Declared `INTEGER` procedure parameter becomes implementation `STRING`. |
| [var_const_decl](declare/var_const_decl.pps) | Accept | `VAR` only on declaration does not reject a constant actual; emitted `pass_flags = 0`. |
| [var_const_impl](declare/var_const_impl.pps) | Reject | `VAR` on implementation rejects a constant actual. |
| [var_decl](declare/var_decl.pps) | Accept | `VAR` only on declaration is ignored; emitted `pass_flags = 0`. |
| [var_expression](declare/var_expression.pps) | Reject | `VAR` on implementation rejects an expression actual. |
| [var_impl](declare/var_impl.pps) | Accept | `VAR` only on implementation wins; emitted `pass_flags = 1`. |
| [dim_unused](declare/dim_unused.pps) | Reject | Rank-2 procedure implementation formal fails even though the body never uses it. |
| [dim_function_unused](declare/dim_function_unused.pps) | Reject | Rank-2 function implementation formal fails even though the body never uses it. |

The [unused procedure log](../target/legacy-parameter-oracle/dim_unused.pcboard.log)
and [unused function log](../target/legacy-parameter-oracle/dim_function_unused.pcboard.log)
both report `Closing parenthesis not found (INTEGER VALUE(3)` at the
implementation header. The
[repeated declaration-only control](../target/legacy-parameter-oracle/dim_decl_only.pcboard.log)
compiles successfully and produces a 649-byte PPE.

### Legacy contract: source language below 400

Parameter count must match, but ordinary parameter type, `VAR` mode and
declaration dimensions do not constrain the implementation. Function result
type likewise comes from the implementation. Parameter names are irrelevant.
The emitted formal header preserves the **implementation's** type, rank and
bounds; a rank-1 formal is not flattened to `dim = 0`.

The kind rule is asymmetric: an explicit `DECLARE PROCEDURE` can normalize a
`FUNCTION` implementation to a procedure. The reverse is rejected. **Only the
kind comes from that declaration; parameter types and `VAR` still come from the
implementation.** The
[kind_proc_var](declare/kind_proc_var.pps) golden has `pass_flags = 1`, not zero.
This does not permit direct `VAR` parameters on a genuine function.

Implementation formals are split at raw commas by the original parser: a
second dimension is mistaken for the next formal, so rank-2/3 implementation
syntax is rejected. The unused rank-2 probes isolate this from errors in the
body; rank-3 follows the same comma rule and has IcyBoard parser coverage, not
a separate original probe in this set. Multidimensional `DECLARE` syntax is
accepted and ignored when determining implementation dimensions. Ordinary
global/local multidimensional arrays are unaffected.

Both the compiler and LSP collect implementation signatures package-wide from
the module-qualified, kind-normalized ASTs **before checking calls**. This keeps
implementation types, results and `VAR` requirements available even when the
implementation is in a later file. The cross-file tests keep declarations
first and vary caller/implementation order; they do not prove arbitrary
declaration ordering or original PPLC multi-file behavior. The shared path is
[module lowering](../crates/icy_board_engine/src/compiler/modules.rs),
[signature collection](../crates/icy_board_engine/src/semantic/mod.rs),
[compiler analysis](../crates/icy_board_engine/src/compiler/mod.rs), and
[LSP analysis](../crates/ppl-lsp/src/main.rs).

### Legacy array calls: source-derived runtime behavior

Accepting `Work(1)` for a rank-1 formal and retaining its header are compiler
observations. The following execution semantics are **source-derived**, not
observed by running those PPEs in PCBoard:

- A scalar input is assigned to formal element zero, not to the whole array.
- Call-frame save/restore covers only formal element zero. Other elements
  persist between calls and are shared across recursive invocations of that
  routine's formal storage.
- Procedure `VAR` copyback transfers only element zero to the caller's scalar
  or selected array element, not the formal's bounds or tail.
- Local-array initialization is separate and starts after the parameters;
  it does not reset formal-array tails.

The evidence is `stkinit`, `stkclean`, `initLocals`, `clearProc` and `TOK_PCALL`
in [SCREXEC.CPP](../pcboard/pcb-main/SOURCE/PPL/SCREXEC.CPP), plus function
argument setup in [EVALP.CPP](../pcboard/pcb-main/SOURCE/PPL/EVALP.CPP).
These paths address the first value in formal storage rather than copying the
entire array. The source is referenced locally, not reproduced here.

[legacy_array_parameters.rs](../crates/icy_board_engine/src/vm/tests/legacy_array_parameters.rs)
contains **10 IcyBoard VM tests** covering header round-trips, scalar input,
persistent tails, value/`VAR` recursion, scalar and indexed copyback, strings,
and VM-supplied arguments. Its compiled-source matrix additionally includes
350/350; a constructed-bytecode test checks unmarked parameters on runtime 400,
the static flag, and that bit `0x04` does not enable modern calls on old
runtimes. The original nine were IcyBoard execution checks, not original-runtime goldens.

### S3 runtime follow-up (2026-09-09)

Two separate authored probes were compiled by PPLC 3.40 and executed on an
isolated PCBoard 15.4/M installation. They do not turn the 23 DECLARE compiler
observations above into runtime observations.

- [var_binding.pps](var_binding.pps) verifies reverse copy-out (`alias=10`),
  a changed index retaining the original target (`changed_index=9:2:1`),
  one index-function evaluation (`index_calls=1:8`), a VAR index parameter
  (`parameter_index=2:7:0`), and recursive scalar VAR (`recursive_var=1`).
- [var_array_recursion.pps](var_array_recursion.pps) produces
  `13:3;13:3;12:3;12|23:5;23:5;23`, confirming persistent tails and copy-out
  before frame restoration. Its array formal is last; the original compiler
  rejected the initial variant with another formal after the array.
- Captures: `target/s3-legacy-oracle/run-8rr6bh4i` and
  `target/s3-legacy-oracle/run-d_fi7w19`. Both runs verified the runtime banner,
  exited successfully and left live PCB/COMPAT/FOSSIL fingerprints unchanged.

IcyBoard now binds indices once for all runtimes. Classic runtimes copy out
before frame restoration; runtime 400 retains its existing frame-safe
copy-out after restoration, including when fed legacy-language source.
The old recursion test expectation described IcyBoard's prior behavior, not
PCBoard's, and is now split by runtime with these original observations.

## Strict contract: source language 400

An authored `DECLARE` is a checked contract. Declaration and implementation
must agree on:

- routine kind and parameter count;
- parameter types, including nominal enum/record identity;
- `VAR` modes, array ranks, exact bounds, and the dynamic marker (`[]` is not
  `[0]`, nor is `[,]` the same declaration as `[0, 0]`);
- function return type and array rank;
- all of the above recursively within callback signatures.

Parameter names are not part of the signature. `DECLARE` remains optional;
without one, the implementation supplies the signature. Direct function `VAR`
parameters remain invalid syntax; semantic comparison tests also exercise
their modes using constructed ASTs. Callbacks require language/runtime 400.

Strictness follows the **source language**, not the PPE target. Language 350
targeting runtime 400 keeps the legacy rules; language 400 does not become
permissive by targeting an older format. A feature's runtime requirement is a
separate check. See
[strict_declare.rs](../crates/icy_board_engine/tests/strict_declare.rs) and the
[language guide](../docs/new_ppl.md#declare-contracts-and-language-versions).

## Modern array ABI and correction to earlier notes

Language-400 whole-array formals require runtime 400 and carry variable-header
flag **`0x04`, `VARIABLE_FLAG_ARRAY_PARAMETER`**. It is independent of static
`0x01` and dynamic-storage `0x02`; a bounded array formal normally has `0x04`,
and a dynamic array formal `0x06`. Value calls copy the entire array and its
current bounds; `VAR` additionally copies the final array and bounds back.
Exact bounds in a strict declaration/implementation comparison do not require
the actual argument to retain those initial bounds.

Legacy formal arrays remain **unmarked**, even when compiled to runtime 400.
Their nonzero rank and implementation bounds remain in the header, but calls
use the element-zero convention above. Runtime 400 or `dim > 0` alone must
not select whole-array passing. The VM interprets `0x04` only for array formals
on runtime 400 or newer, not as a change to classic PPE formats.

Preserving scalar call behavior does not require discarding formal dimensions.
Strict language-400 matching replaces the former permissive `DECLARE` check.

**Recompile unreleased runtime-400 programs using array parameters.** Earlier
unmarked 400 array parameters cannot be distinguished from classic formals.
There is no compatibility shim that guesses the intended ABI. The separate
earlier static/dynamic flag correction also requires recompiling affected beta
PPEs. See [PPE entry flags](../docs/ppe_format.md#entry-header--11-bytes) and
[modern array tests](../crates/icy_board_engine/src/vm/tests/array_parameters.rs).

## Reproducing the compiler evidence

Use the [compiler oracle wrapper](pplc_oracle.py) with the authored sources
linked above and select a scratch artifact directory with `--output-dir`.
Do not use `--disarr` for this audit. The wrapper performs CP437 conversion and
CRLF preparation; inspect both the compiler verdict and the emitted signature,
not merely its exit status or whether a PPE file exists. See
[README.md](README.md#1-compiler-oracle--run-the-original-pplcexe) for setup.
Runtime conclusions would require a separate live PCBoard run and captured
output before being promoted from source-derived to runtime-oracle evidence.