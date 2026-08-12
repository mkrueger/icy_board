.. role:: PPL(code)
   :language: PPL

PCBoard Compatibility
=====================

IcyBoard aims to run PPEs written for PCBoard 15.x unchanged. This page lists
the predefined statements and functions that do **not** behave like the
original, and says why. Everything not named here is implemented and is meant
to match PCBoard; if you find one that does not, please report it.

Of the 507 predefined statements and functions PCBoard offers, 456 are
implemented, 13 more work with one corner of their behaviour missing, 6 answer
with a substitute value on purpose, and 32 are not implemented at all.

.. list-table::
   :header-rows: 1
   :widths: 26 16 10 14 18

   * - Kind
     - Implemented
     - Partial
     - Substituted
     - Not implemented
   * - Statements (223)
     - 202
     - 7
     - 2
     - 12
   * - Functions (284)
     - 254
     - 6
     - 4
     - 20

Checking your own PPE
---------------------

You do not have to read this list to find out whether a particular PPE is
affected. The decompiler will tell you::

    ppld --compat-check MYDOOR.PPE

It scans the compiled code and reports every call that is not fully supported,
grouped by severity, along with the line it appears on. A PPE that reports
nothing uses only opcodes that behave like the original.

What the categories mean
------------------------

**Not implemented**
  The call does nothing and returns a neutral value (`0`, an empty string). A
  PPE that depends on it will not work correctly. These are the ones to care
  about.

**Substituted**
  The call cannot mean anything here, but returning nothing would be worse than
  returning something. It answers with a plausible fixed value and carries on.
  Most PPEs are unaffected.

**Partially implemented**
  The call works for what a PPE normally does with it, but a corner of the
  original's behaviour is missing.

Not implemented
---------------

DOS interrupts and direct memory access
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*Statements:* :PPL:`DOINTR`, :PPL:`VARADDR`, :PPL:`VARSEG`, :PPL:`VAROFF`,
:PPL:`POKE`, :PPL:`POKEB`, :PPL:`POKEW`, :PPL:`POKEDW`

*Functions:* :PPL:`PEEKB`, :PPL:`PEEKW`, and the eighteen register functions
:PPL:`REGAH`, :PPL:`REGAL`, :PPL:`REGAX`, :PPL:`REGBH`, :PPL:`REGBL`,
:PPL:`REGBX`, :PPL:`REGCH`, :PPL:`REGCL`, :PPL:`REGCX`, :PPL:`REGDH`,
:PPL:`REGDL`, :PPL:`REGDX`, :PPL:`REGSI`, :PPL:`REGDI`, :PPL:`REGDS`,
:PPL:`REGES`, :PPL:`REGF`, :PPL:`REGCF`

These call DOS and BIOS interrupts and read and write arbitrary addresses in
the real mode address space. IcyBoard is a native program on a modern operating
system: there is no interrupt vector table to call into, no segment and offset
pair that means anything, and no address a PPE could usefully poke. Emulating
an 8086 to service them would not help either, because what these PPEs reach
for is the real hardware and the real DOS underneath — the video adapter, the
disk, the serial port — none of which is there.

This is not a gap that will be filled. If a PPE needs one of these, it needs a
rewrite of that part.

.. note::
   :PPL:`MKADDR` still works, since it only packs a segment and an offset into
   one number and does not touch memory.

FidoNet mailer configuration
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*Statements:* :PPL:`FDOWRAKA`, :PPL:`FDOADDAKA`, :PPL:`FDOWRORG`,
:PPL:`FDOADDORG`

PCBoard 15.2 added these so a PPE could edit the mailer setup that ran
alongside it: the AKA list and the origin lines. They were never finished —
the original source carries the bodies commented out, and the argument count
its compiler accepted does not match what the dead code read. There is nothing
to be faithful to, so they log a warning and return.

The reading half of the family does work:

* :PPL:`FDORDAKA(n)` returns the *n*-th address this board answers to, counted
  from one, as ``zone:net/node`` with the point appended when there is one, and
  an empty string when there is no such record.
* :PPL:`FDORDAREA(n)` returns the tag of the *n*-th message area that takes
  part in the network.
* :PPL:`FDORDORG(1)` returns the origin line. PCBoard kept a list of them and
  IcyBoard has one, so every other record number answers empty.
* :PPL:`FDOQADD`, :PPL:`FDOQMOD`, :PPL:`FDOQDEL` and :PPL:`FDOQRD` work on the
  outbound queue: what is waiting for a link, numbered from one across all
  links. PCBoard queued a file with a normal or a crash flavour; nothing here
  calls a link out of turn, so the flag is read and ignored. :PPL:`FDOQRD` was
  an empty case in the original and answers the file name of that record here.

Substituted values
------------------

.. list-table::
   :header-rows: 1
   :widths: 18 22 60

   * - Opcode
     - Answers with
     - Why
   * - :PPL:`SOUND`
     - nothing
     - Sounds a tone on the PC speaker of the machine the board runs on, not
       the caller's. On a DOS BBS that was the sysop's own machine sitting in
       the room. There is no speaker to drive here, and a server beeping at an
       empty room helps nobody. Use :PPL:`PRINT CHR(7)` to ring the *caller's*
       terminal bell instead, which is almost always what a PPE actually wants.
   * - :PPL:`SOUNDDELAY`
     - nothing
     - The same, with a duration.
   * - :PPL:`GETDRIVE`
     - `0`
     - The current DOS drive letter. There are no drive letters, so this
       reports the default drive.
   * - :PPL:`SETDRIVE`
     - the drive it was given
     - Changing the current drive has no meaning; the call reports success so a
       PPE that checks does not take an error path.
   * - :PPL:`MODEM`
     - `CONNECT 9600/ARQ/V32`
     - The connect string the modem reported. Callers arrive over the network,
       so there is no modem and no connect string. PPEs read this to show off a
       caller's connection, so a plausible one is returned rather than an empty
       string.
   * - :PPL:`PEEKDW`
     - a different number each call
     - Reads a doubleword of memory, and belongs with the group above. It is
       called out separately because a PPE was found reading a VGA register
       through it and spinning until the value changed; a constant would hang
       it, so this one returns a fresh number instead.

Partially implemented
---------------------

The dBase record and file locks — :PPL:`DLOCK`, :PPL:`DLOCKR`, :PPL:`DLOCKG`,
:PPL:`DLOCKF` and :PPL:`DUNLOCK` — always report that the lock was taken.

On PCBoard these mattered because several DOS nodes shared one table over a
network and each had to keep the others out of a record it was writing.
IcyBoard runs every node in one process, so there is no second writer to lock
out. A PPE that takes a lock, checks it and proceeds behaves correctly; one
that expects a lock to *fail* because another node holds it will wait forever
for a conflict that cannot happen.

Everything else about the dBase support is implemented, including the parts the
original PCBoard documentation gets wrong. See :doc:`dbase`.

The FidoNet outbound queue is the other partial family: :PPL:`FDOQADD`,
:PPL:`FDOQMOD` and :PPL:`FDOQDEL` queue and remove files and :PPL:`FDORDORG` and
:PPL:`FDOQRD` read them back, but the crash flavour is ignored and only a single
origin line is kept, as the FidoNet section above describes.

Differences worth knowing
-------------------------

These are supported, but behave differently enough to mention.

**Drive letters and backslashes in paths**
  Backslashes are translated to the native separator, so :PPL:`"data\\x.pcb"`
  works. A path beginning with a drive letter has the drive stripped and is
  resolved relative to the board's root, with a warning in the log. Write paths
  relative to :PPL:`PPEPATH()` and they will work everywhere.

**File names match regardless of case**
  DOS did not distinguish `GW-USER.PCB` from `gw-user.pcb` and neither do we,
  even on a file system that does: a name that does not match exactly is looked
  up again ignoring case.

**Trailing blanks in a file name are ignored**
  A name built with :PPL:`MID` comes back padded to the width that was asked
  for, and PCBoard opened the file anyway. So does IcyBoard.

**Shelling out to DOS**
  :PPL:`SHELL` runs a program on the host. A PPE calling a DOS utility, a
  `.BAT` file or `COMMAND.COM` will not find them.

**An opcode we have not implemented does not kill the PPE**
  It is logged and execution continues with a neutral result. A PPE that uses
  one will misbehave rather than stop, which is usually the more useful of the
  two — but it does mean the log is where the evidence is.
