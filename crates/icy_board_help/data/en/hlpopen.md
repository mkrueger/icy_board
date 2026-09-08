# Help: (OPEN) a DOOR

A door is a game or other program offered by the board. Enter `OPEN`
to display the current conference's door menu and choose a door.
Available doors, access requirements and any charges depend on the
board's configuration.

## Subcommands

- `[number]` Open the door with this menu number, starting at 1.
- `[name]` Open a door by name. Matching ignores letter case and
  accepts a leading part of the name. If several names match, the
  first matching door in the list is chosen.
- `Enter` At an empty door selection prompt, return to the board.

You can put the number or name after `OPEN` to skip the menu prompt.
A protected door asks for its password. An invalid selection or
insufficient access prevents the door from opening.

If files are flagged for download, the board asks whether to continue
before opening a door. Answer `N`, or accept the default, to cancel
the door request; answer `Y` to continue. Download first if you want
to finish your transfers before entering the door.

Once inside a door, follow that program's own instructions. Its keys
and exit command may differ from the main board commands.

## Examples

Open the first door listed in this conference:

```text
OPEN 1
```

If the menu lists a door named CHESS, open it by name:

```text
OPEN CHESS
```
