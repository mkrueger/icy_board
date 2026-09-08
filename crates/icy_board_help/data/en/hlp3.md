# Help: (3) Pack Message Bases

Pack the JAM message bases of all configured areas in the current
conference. This is not limited to the currently selected message area.
Command 3 permission is required. PCBoard invoked an external pack utility;
IcyBoard performs the work directly.

## Subcommands

- `3` Start the interactive pack questions.
- `N` At the initial confirmation, cancel without packing.
- `Y` At the initial confirmation, continue to the pack options.
- `Y` At the new-index question, rebuild indexes only and skip purge and
  renumber questions.
- `N` At the new-index question, continue with a full pack.

The letters above are for an English session. Follow the displayed Yes/No
choices if the session uses another language.

## Pack Options

A full pack removes deleted messages and messages whose pack-out date has
arrived. Optional criteria also remove older messages and private messages
already read by their recipients. Leaving the date blank does not prevent
the other removal rules from applying.

The date prompt accepts six digits in `MMDDYY` order. Currently this compact
form is interpreted as a year in the 1900s, not the 2000s. Do not use it to
request a modern-year cutoff. Leave it blank, or enter `010180`, to disable
the date criterion.

If renumbering is selected, a positive new low message number starts the
surviving messages at that number. Zero or an empty answer cancels
renumbering, not the pack. Without renumbering, surviving numbers stay the
same. Stored references to old numbers may need review after renumbering.

Index-only mode rebuilds the indexes without purging or renumbering message
content. It is not a backup or a way to recover content already packed out.

## Safety

The initial confirmation defaults to No. There is no final confirmation
after the options. Do not stack affirmative answers unless the complete
sequence is intentional.

Take a separate, consistent backup of every affected message base before
starting. No automatic archival backup is created by this command. Recover
accidentally deleted messages before a full pack; command `4` cannot restore
content that packing has removed.

Arrange a maintenance window and stop other writers first. This command has
no board-wide check that other callers are offline. Areas are processed
separately: an error in one does not undo changes already made in another.

## Examples

Open the questions without pre-authorizing a destructive action:

```text
3
```
