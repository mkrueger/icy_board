# Help: (13) View Node Caller Log

View caller-log lines for one node or all nodes. Command 13 permission is
required. PCBoard opened separate node log files; IcyBoard filters its
single configured caller log.

## Subcommands

- `13` Display the node list and ask which node to view.
- `13 2` Show lines containing the node tag `[2]`.
- `13 A` Show the shared log without a node filter.
- `13 2 S` Ask for search text and restrict matching lines to node 2.
- `13 A S` Ask for search text and search without a node filter.
- Press Enter at the node-selection prompt to cancel.

## Description

Node numbers start at one and must be within the configured node range.
The node need not currently have a caller. `A` overrides a node filter
when both are given. Supplying a node or `A` is necessary before a listing
can begin.

Text searching ignores ASCII letter case and matches literal substrings.
It shows matching lines in file order, not complete sessions or Boolean
search results. For multiword text, use `S` and enter the phrase at the
search prompt rather than stacking several unlabelled words.

Node matching checks for the literal bracketed tag anywhere on the line.
Untagged lines will not appear in a node-specific view, and text that
merely contains the same tag can match too. This is not a separate,
authoritative per-node archive.

This command only views or searches. Command `1 D` deletes the shared log,
not just the selected node's entries.

## Examples

Search all nodes, then type a phrase at the search prompt:

```text
13 A S
```
