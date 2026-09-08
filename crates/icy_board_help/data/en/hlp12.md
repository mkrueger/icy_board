# Help: (12) Log Off Another Node

Request a forced logoff for the caller on another node. Command 12
permission is required. PCBoard marked the node for pending logoff;
IcyBoard sends a shutdown request to that node's session and records the
request in the caller log.

## Subcommands

- `12` Display the node list and ask for a node number.
- `12 2` Send a forced-logoff request to node 2, if it is another active
  node with a session channel.
- Press Enter at the node-number prompt to cancel.

## Safety

Node numbers start at one. The number must be within the configured node
range. This command rejects your own node and nodes without a session
channel. Use `G` to end your own session normally.

There is no Yes/No confirmation after a valid node is selected. Check the
node and warn the caller first: a forced logoff can interrupt message
entry or a transfer. Sending the request is not an acknowledgement that
the caller has already disconnected; check the node list afterward.

This is not a server restart, a DOS exit, or a replacement for PCBoard's
node recycle command.

## Examples

Inspect the node list first:

```text
11
```

Then open the logoff prompt and select a node only if intended:

```text
12
```
