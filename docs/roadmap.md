# Roadmap

Icy Board is in beta. Completed work belongs in the release history and the
[feature status](feature_parity.md), not in a future-work list. This page only
tracks the larger remaining directions; it is not a release-date promise.

## Beta hardening

- Test imports from more real PCBoard installations and reduce manual path and
  PPE cleanup.
- Close the remaining command differences in the
  [command audit](../compat/COMMAND_AUDIT.md), especially the message-reader
  export, edit, forward, view and capture actions.
- Activate or remove the remaining inert setup options and security levels in
  the [options audit](../compat/OPTIONS_AUDIT.md).
- Complete the English and German operator documentation and help files.
- Continue compatibility testing against the PCBoard source and DOSBox oracle.

## After the first beta

- Implement the PPL web statements and functions.
- Improve FTN operation where real networks need it: per-user netmail, an
  ICBSetup editor for AKAs and links, and AreaFix if required.
- Add a self-service password-reset flow without weakening password storage.
- Provide a web administration or caller frontend; IcyTerm can run as
  WebAssembly, but the board still needs a suitable API.

### PPL compiler: semantic analysis before lowering

- Move source-level semantic analysis before structural AST rewrites: resolve
  names and types and validate calls on the original source AST, including code
  that is later optimized away.
- Target pipeline: source AST → semantically checked HIR → lowering → code
  generation. Lowering should consume resolved symbols and types rather than
  repeat source analysis; replace the separate compound-receiver type probe
  with these shared semantic results.
- Preserve source provenance through lowering for later diagnostics. Keep
  internal consistency checks after lowering rather than a second full
  source-level semantic pass.
- Migrate compiler and language-server analysis together, with regression tests
  for diagnostic messages and ranges, module and legacy-language behavior, and
  generated program behavior. Treat this as a separate architecture change,
  not just a reordering of the current compiler passes.

## Explicitly out of scope

DOS, serial and modem support, FOSSIL drivers, PPE DOS/assembler calls and
printer output are compatibility constraints the project does not intend to
recreate. See [differences and improvements](differences.md) for the boundary.
