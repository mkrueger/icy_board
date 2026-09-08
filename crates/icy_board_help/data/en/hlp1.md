# Help: (1) View Caller Log

View or search the configured caller log. This sysop command requires the
board's command 1 access permission. Unlike PCBoard's per-node caller files,
IcyBoard uses one shared log with node tags.

## Subcommands

- `V` Display the log in file order.
- `P` Display the log on screen; no printer output is available.
- `S` Ask for text and show matching lines, ignoring ASCII letter case.
- `D` Ask for confirmation before deleting the shared caller log.
- Press Enter at the selection prompt to return without an action.

## Description

Search is a literal substring search, not a wildcard or Boolean expression.
Only matching lines are shown, not the whole caller session around a match.
Use command `13` to restrict the listing to one node.

The legacy `P` selection is accepted, but the listing enables line counting
again, so it is not a guaranteed nonstop mode.

Deletion defaults to No. Answering Yes removes the log for all nodes and
writes a new entry recording who deleted it. There is no automatic backup
for this action; save a separate copy first if the history is needed.

## Examples

View the shared log:

```text
1 V
```

Start a search, then enter the desired text at the search prompt:

```text
1 S
```
