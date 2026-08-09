Timed events
============

A timed event is a program the board runs at a fixed hour of the day, for instance a
nightly backup, a message base pack or a call to ``icbmailer``. Because such a program
usually rewrites the very files the board keeps open - the user base, the message bases,
the configuration - Icy Board does what PCBoard did: it clears the board first and only
then starts the program.

The event file
--------------

The events live in their own file. Its name is set in ICBSetup under
*General* → *Event setup* → *Name/Location of Event File*; a new board gets
``main/events.toml``. Pressing F2 on that entry opens the file in the editor configured
as *external editor* and creates it when it does not exist yet.

.. code-block:: toml

   [[event]]
   description = "Nightly maintenance"
   enabled = true
   time = "03:00:00"
   days = "YYYYYYY"
   mode = "fixed"
   command = "scripts/nightly.sh"

   [[event]]
   description = "Weekly fidonet poll"
   time = "05:30:00"
   days = "NYNNNNN"
   mode = "slide"
   command = "icbmailer poll"

``description``
   Shown in the log. Purely informational.

``enabled``
   An event that is set to ``false`` never fires. Defaults to ``true``.

``time``
   Wall clock time in ``HH:MM:SS``, in the time zone of the machine the board runs on.

``days``
   Seven characters, ``Y`` or ``N``, starting with Sunday. ``YYYYYYY`` is every day,
   ``NNNNNNY`` is Saturdays only. Defaults to every day.

``mode``
   What happens to the callers that are still online when the clock reaches the event.

   ``fixed``
      They are told the board is going down and the line is dropped. This is the
      default and the closest to what PCBoard did.

   ``slide``
      The event waits for the last caller to hang up. No new caller gets in while it
      waits, so the wait is bounded by the sessions that were already running.

   ``idle``
      The occurrence is skipped when somebody is online, and tried again the next day.

``command``
   Handed to ``sh -c`` (``cmd /C`` on Windows) with the board directory as the working
   directory. An event without a command only clears the board - which is what you want
   when a scheduler outside the BBS does the actual work.

Clearing the board
------------------

Three settings in ICBSetup decide how much warning the callers get. All of them count
backwards from the event time.

*Minutes prior to event to suspend the system*
   From this moment on nobody may log on any more - a caller reaching the board is shown
   ``Access Denied - Upcoming Event Pending ...`` and disconnected. The callers who are
   already online are shown ``Awaiting Event Timer - All activity suspended ...`` and
   dropped.

   The same number also caps the time of a session that starts shortly before the event:
   a caller logging on ten minutes before the suspend period gets ten minutes, not their
   usual limit, and PPL's ``ADJTIME`` may then only take time away, never give it back.
   ``EVENT()`` returns true for such a session.

*Disallow uploads prior to event* and *Minutes prior to event uploads disallowed*
   Together they refuse uploads for a while before the event, so that no transfer is cut
   in half by it. The caller is shown ``Uploads Are Currently Disabled``.

Enabling events at all
----------------------

Nothing of this happens unless *Event enabled* is switched on in ICBSetup. When it is
off the event file is still read but never looked at, which makes it easy to prepare the
events before letting them loose.

Importing from PCBoard
----------------------

``PCBOARD.DAT`` only knows the single daily event, not the ``EVENT.DAT`` list. The
importer therefore writes one daily event with the time from ``PCBOARD.DAT``, in
``slide`` mode when PCBoard's *slide event* flag was set, and leaves its command empty -
PCBoard ran ``EVENT.BAT``, which will not do anything useful here. The suspend period and
the upload settings are carried over unchanged.
