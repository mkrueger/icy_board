# PPL compiler phases

The compiler and language server share one authoritative source analysis via
`SemanticVisitor::analyze_sources`.

## Source analysis

1. Parse source text, retaining tokens, file positions and expression identities.
2. Bind module namespaces, validate visibility and apply legacy declaration-kind
   rules. This is source name binding, not executable desugaring: conditions,
   loops, assignment operators, initializers and expression operands are retained.
   The input AST remains untouched; binding operates on a source-preserving copy.
3. Resolve calls and types and check the source constructs directly. A source
   control-flow graph tracks reachable references without skipping diagnostics
   in unreachable branches, loop bodies or control expressions.
4. Finish routine reachability and source diagnostics, producing `CheckedProgram`.

The checked high-level representation is deliberately **source-shaped**: it uses
the AST node vocabulary plus resolved annotations, rather than duplicating every
syntax node in a second enum. Call and binary identities survive transformations.
Member, receiver, assignment-target and function-result annotations are scoped
by file; equal offsets in different files cannot alias.

The language server retains the original AST for editing and the same semantic
results for diagnostics, references, hover and completion. File-local annotations
are selected before editor queries. Invalid/incomplete input still yields useful
editor information, but a program with source errors cannot enter code lowering.

## Executable lowering

`PPECompiler::compile` lowers only a valid checked program:

- Desugar structured control flow, initializers and compound assignments.
- Fold constants only after their source expressions have been checked.
- Use resolved receiver types to capture compound-assignment targets exactly
  once, preserving record storage paths and object identity.
- Register generated temporary storage and generated call/enum annotations
  explicitly. There is no throwaway type probe and no second source-semantic pass.
- Allocate resolved variable/routine storage. Intern constants from the lowered
  program, not folded-away source operands. Constant-pool labels never enter
  source-name lookup tables.
- Resolve the backend `HirProgram`, lower it to PPE commands and assign offsets.

`create_executable` checks backend HIR consistency (including unresolved
expressions and invalid storage/label IDs), then enforces format/runtime limits
and serializes the executable. These are internal/output checks, not repeated
source type checking. Compilation validity is captured before returning to the
caller; serialization does not acquire the shared diagnostic reporter lock.

## Regression coverage

- Source operators, calls, conditions, loops, SELECT, returns and initializers,
  including errors that folding or dead-code removal previously could hide.
- Exact compiler/editor spans, UTF-16 ranges, CRLF and editing recovery.
- No-op AST traversal retaining tokens and parse identities.
- Module binding, legacy declaration kinds, cross-file annotation isolation,
  callbacks, dynamic arrays, record/object compound assignments and enum bitwise
  operations.
- PPE execution and decompiler round trips, precise typed constants, generated
  temporary layout, constant-pool name collisions and invalid-HIR rejection.

The existing `lower_modules` function remains as a compatibility entry point for
source binding. New compiler/editor analysis uses the shared checked-source API.