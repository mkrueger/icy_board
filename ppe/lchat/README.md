# LIQUID CHAT V1.2

Found that PPE by surprise while working on my PCBoard replacement. I had no clue that
I wrote that (but I did - could remember it when I saw it) - was a suprise :).

So I decided to revive it for my upcoming BBS. Unfortunately I lost all original data from that time.
Good that PPE is decompileable.

The recovered source has since been rewritten as structured PPL 4.00. The page
schedule, emergency code, ANSI screens, local/remote chat and colour configuration
remain compatible with the decompiled version.

## What is LIQUID CHAT?
LIQUID CHAT is a O command replacement for PCBoard 15.x and IcyBoard

## ANSI ART
Thanks to O-DOG for the Chat_ansi
(ps. would nice if you could contact me)

## How to install LIQUID CHAT?
PCBSETUP
  type b b goto commandlist and install LIQUID CHAT:
    O     0         0      0   <PPEPATH>LCHAT.PPE

  if u want to skip the page reason question try :
  !<PPEPATH>LCHAT.PPE /A
(on F-keys pp with /SHIFT)

## Controls

While paging:

- caller or sysop: `A` aborts the page
- sysop: `C` accepts and starts chat
- sysop: `S` disables the local terminal bell

While chatting:

- `Esc` ends chat
- `Ctrl+R` redraws both panes
- `Ctrl+W` clears the active pane

## Configuration

`lchat.cfg` keeps its original 18-line layout: five sysop colours, five caller
colours, seven daily paging windows starting with Monday, and the emergency code.

## Where can I get the newest LIQUID PPE's (HOW TO REGISTER) !
  No longer needed :). But would nice to give some feedback.

(C) 1995-2024 by Mike Krüger (no longer using SubZero pseudonym)
ANSI ART : O-DOG (I can't remember his real name or anything)