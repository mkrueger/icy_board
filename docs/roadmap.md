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
- Support IcyAnim directly so boards do not need to invoke `icy_play`.

## Explicitly out of scope

DOS, serial and modem support, FOSSIL drivers, PPE DOS/assembler calls and
printer output are compatibility constraints the project does not intend to
recreate. See [differences and improvements](differences.md) for the boundary.
