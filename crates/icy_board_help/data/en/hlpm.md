# Help: (M)ode

This command will allow you to change the graphics mode between no-color,
ANSI, RIP, AVATAR and graphics modes.

## Subcommands

- `CTTY` Non-ANSI mode. No color and just ASCII control codes
  the lowest possible display mode.
- `ANSI` ANSI cursor positioning. Uses ANSI control codes but
  doesn't display colors. It's faster for screen drawing
  on slow connections.
- `GRAPH` ANSI color and cursor positioning. Uses ansi control
  codes and ansi colors. This is the default mode.
- `AVATAR` AVATAR color and cursor positioning. Like GRAPH but using
  avatar control codes and ansi colors.
- `RIP` RIPscrip graphics mode. If RIPscrip versions of display
  files are available they will be used.

## Description

Without subcommand this command toggles between graphics and non graphics
mode. If you execute this command with a subcommand then you may choose
which mode you want. If you see garbage after selecting a graphics mode
enter the 'm' command again to switch to non graphics mode and switch to
another mode.

## About RIPscrip/AVATAR

Teminal clients usually support ANSI. RIPscrip/Avatar is only supported
for some terminals (like Icy Term). Ensure your terminal is capable of
interpreting these codes. Otherwise garbage characters will appear on your
screen.

## Examples

If you want to switch to AVATAR mode you can enter:

```text
M AVT
```
