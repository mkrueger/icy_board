Icy Board
=========

Hotkey Bars
-----------

Setup, system manager, text/menu editors and call-wait screens show their
existing hint bars in a uniform style: keys are highlighted separately from
their descriptions, and the bar is centered on the frame of the active screen
or dialog. This changes presentation, not the runtime keys.

Key legend: **␛** Esc, **↵** Enter, **⇥** Tab, **⎀** Insert, **⌦** Delete,
**⌫** Backspace, **⇞** PageUp, **⇟** PageDown, **↖** Home, **↘** End,
**␠** Space. Arrows remain arrows and function keys remain F1, F2, etc.
Modifiers stay explicit, for example Ctrl+s and Shift+⇥; a slash separates
alternative keys for the same action.

Use a monospace terminal font with these Unicode symbols. Not every font
supports all glyphs; missing symbols may appear as boxes.


Call Waiting Screen
-------------------

.. image:: ../../assets/call_waiting_screen.png
   :alt: Call waiting screen
   :width: 400px

The call waiting screen is what you see when you start Icy Board. It's like the PCBoard call 
waiting screen but modernized a bit. All important Icy Board configuration utilities are accessible from here.

The first button row is **User / Sysop / Exit**. The second is
**Log Viewer / System Status / Event Monitor**. The old Busy/Not Busy choices
are gone: local logins do not stop network services.

Options Explained
-----------------

User
~~~~

Open a local login prompt for any user in the users file. Network services and
the event scheduler keep running; normal admission and maintenance gates apply.

Sysop
~~~~~
Log in locally as sysop, directly at the command prompt. Network services keep
running; this is not a Busy mode or a bypass of maintenance admission gates.

Exit
~~~~
Exit the IcyBoard application and stop its services. This does not open a shell.

Log Viewer
~~~~~~~~~~
Read the application log and the configured caller log without modifying either.
The application path is the actual configuration path with its extension replaced
by ``.log`` (``config_file.with_extension("log")``), not a hard-coded filename.
The caller source uses ``paths.caller_log``, resolved against the board root.
Native logs are UTF-8; imported caller records that are not valid UTF-8 fall back
to CP437.

While following, the viewer samples about once per second and reopens the file.
Reads and searches cover only the newest 256 KiB, at most 2000 lines and
2048 displayed characters per line, not the whole file. A leading partial line
is discarded. These are display limits, not log retention or rotation settings.
Only regular files are accepted; final symlinks and special files are rejected.
Missing, unconfigured or unreadable sources are reported in the viewer. Terminal
controls are stripped, and web-admin token entries are hidden and not searchable.

The metadata row shows the opened file's size in bytes and modification time in
UTC (or unknown). A read failure clears the old content and marks metadata
unavailable. Replacement/rotation is detected by device/inode changes **on Unix
only**; truncation is detected by a smaller sampled size. These are comparisons
between successful samples, not filesystem notifications: truncate-and-regrow or
changes entirely between samples can be missed. A detected change remains shown
across normal refreshes. This is not comprehensive rotation detection.

* Tab switches between application and caller sources and clears the text filter.
* F toggles following; initially it is on. Pausing freezes the loaded tail and
   its metadata, even if an outstanding read finishes. A newly selected source
   still receives one initial snapshot while paused. Re-enabling follow requests
   a fresh read immediately; if an older read is pending, its result is discarded
   and the fresh read starts as soon as it completes, without another refresh delay.
* E toggles warning/error records for the application source only, using actual
   log-level headers rather than words in the message.
* / edits a case-insensitive substring search. Enter applies it; Esc cancels the
   edit and preserves the committed query. Apply an empty query to clear it.
   Typing alone does not change the displayed matches.
* T switches between matching-lines-only filtering and context mode, which keeps
   surrounding lines visible and highlights matching rows. The application
   warning/error restriction still applies in both modes.
* n/N select the next/previous matching row, wrapping at either end. Match
   navigation pauses following and centers the selection where possible. The
   counter shows the selected match and total matches (0 means none selected).
* Up/Down scroll lines, PageUp/PageDown scroll pages, and Home/End move to the
   first/last page. These vertical navigation keys turn following off; F resumes it.
* Left/Right pan horizontally. Esc outside search editing returns to the owner.

Event Monitor's L key opens the selected execution's output in this same viewer,
as a **fixed UTF-8 file** (invalid UTF-8 uses replacement characters, not CP437).
Tab source switching and E severity filtering are disabled there; follow, search,
context and navigation remain available. Esc returns to Event Monitor. Its path
must pass the event monitor's canonical, regular-file checks directly beneath
the board-root ``event_logs`` directory; missing/invalid paths are reported, not
replaced with output from another execution.

System Status
~~~~~~~~~~~~~
This read-only view shows server uptime (days and hh:mm:ss), new-login admission,
active/total nodes, the board root, disk space, maintenance state, runtime/request
errors and any active Online event. Disk values are **available/total GiB and
percent available** for the board-root filesystem. A display-only low-space
warning appears when available space is strictly **less than 1 GiB OR less than
10%** of total. Exactly 1 GiB or 10% alone does not trigger that condition; the
other condition can still trigger it. Unknown totals/percentages are labelled
unavailable. This warning does not change admission, upload or event policy.

Configuration, runtime, nodes and disk have independent freshness rows: last
successful sample in UTC, age in seconds, and fresh/stale status with busy,
read-failed or pending reasons where applicable. Normal configuration/node/disk
refresh is about once per second; runtime is checked each UI loop. Samples become
stale at two seconds old (or immediately on a busy/read-failed result); a pending
disk read also becomes stale after two seconds. Contention retains the last
successful value without advancing its timestamp. A disk read failure instead
clears the disk value but retains its last-success timestamp. Sources with no
successful sample are explicitly marked; one source's refresh never makes
another source look fresh.

Telnet, SSH, secure WebSocket and web-admin entries distinguish configured
addresses/enabled flags from actual local listener addresses and running state.
Actual listeners are published only after successful binding; the supervisor
marks them stopped when their service task completes. An enabled configuration
alone does not mean a listener is running. Bound listeners with BBS logins gated
are distinguished from stopped services; web admin is not a BBS login endpoint.
Listener rows show time since the state transition and, when the full row fits,
its UTC timestamp. A running local listener does **not** prove public reachability
through NAT, a firewall or a proxy.

Up/Down, PageUp/PageDown and Home/End scroll; Esc returns to call-wait. Both
System Status and Log Viewer return automatically to the owning call-wait screen
when offline maintenance or a restart requires its service-management handshake.
Neither viewer acknowledges that handshake or reopens admission itself.

Event Monitor
~~~~~~~~~~~~~
Show the timed events, their last result and their history, and run one now.
The scheduler keeps running while this screen is open; see :doc:`events`.
This button replaces the runtime F6 shortcut. F6 remains available for history
in the ICBSetup event editor, not as a call-wait shortcut.

Up/Down and Home/End select events. The **Candidate** column is a future schedule
slot, not a guaranteed start or the scheduler's retained backlog. Details show
the local candidate time/countdown and applicable disabled, invalid, weekday or
daily-window restrictions. There are no calendar date-range settings.

History is cached, newest first: PageUp selects an older execution, PageDown a
newer one. Left/Right scroll wrapped detail rows; L opens output for that exact
execution, including pending Online output when recorded. R/F5 requests a refresh
of the loaded-board list and journal, also refreshed about every five seconds;
it does not reload the event configuration file. Details show duration/elapsed
time and exit code where known. Times measure journal attempted starts, not
verified process runtime; interrupted/wait-error durations are unknown.

Enter opens a default-No run confirmation; choose Yes explicitly to queue a
manual run. There is **no interrupt or kill button**: Esc closes a view or dialog,
not a running command. Warnings do not impose a command timeout.

Call Log - On/Off
~~~~~~~~~~~~~~~~~
Toggle caller logging to the configured ``paths.caller_log`` file. This is
separate from the application log and from the read-only Log Viewer.

Page Bell - On/Off
~~~~~~~~~~~~~~~~~~~
Toggle the page bell. If on, the terminal bell will ring when a user pages sysop.
An active page also replaces the ready indicator with a red banner containing
the node number and caller name.

External Page Notification
~~~~~~~~~~~~~~~~~~~~~~~~~~
``Page Notification Command`` under Configuration Options can run one trusted
shell command whenever a page starts. Leave it empty to disable this feature.
The board passes caller data as environment variables instead of substituting
it into the command: ``ICB_PAGE_NODE`` contains the node number and
``ICB_PAGE_USER`` contains the caller name. For example, on a Linux desktop::

   notify-send "Icy Board" "Sysop page from $ICB_PAGE_USER on node $ICB_PAGE_NODE"

The command runs asynchronously from the board root. It can therefore also
invoke an administrator-owned script which sends a mail, webhook, ntfy, or
other push notification. Failures are written to the board log and do not
interrupt the caller's page.

Alarm - On/Off
~~~~~~~~~~~~~~~~~~~~~~~
Toggle the alarm bell. If on, the terminal bell will ring when a user logs in.

ICBSM
~~~~~
Start the system manager utility. This is a TUI utility to manage users and groups.

.. image:: ../../assets/icbsm.png
   :alt: The ICBSM user file maintenance menu
   :width: 400px

Its menu carries the entries of the utility it replaces: besides the record editor
it sorts and packs the user file and runs the bulk edits over a selection of users -
security levels, expiration dates, conference registration and phone formats. The
screens ask the same questions in the same order, and PGDN starts the run as it did.
Security levels can also be handed out from a table of upload, download or ratio
steps; the tables are built from the same menu and kept in ``security_tables.toml``
next to the user file.
The entries the original spent on printer reports, index files and the user info
file have no equivalent here. Every operation that rewrites the file copies it
first, and ``Undo`` in the main menu puts that copy back.

The same operations run without a screen for cron jobs and timed events::

    icbsm --pack --inactive-days 365 --keep-security 100 --dry-run
    icbsm --standardize-phones
    icbsm --undo

``--dry-run`` lists the users that would be affected and writes nothing.
ICBSM takes the board lock while it runs, so it refuses to start when another
tool is already writing to the same board.

ICBText
~~~~~~~
Start the ICBText editor. This is a TUI utility to edit the system messages and prompts.

.. image:: ../../assets/mkicbtxt.png
   :alt: The ICBText editor
   :width: 400px

ICBSetup
~~~~~~~~
Start the setup utility. This is a TUI utility to create and configure an Icy Board
installation. Its main menu is shown in :doc:`installation`.

ICBMoni
~~~~~~~
Start the monitor utility. This is a TUI utility to monitor system activity.
It shows nodes and logged-on users. Use System Status for actual listener state.

Show Statistics
~~~~~~~~~~~~~~~
The statistics monitor retains the all-time and today counters and their reset
action. Del asks for confirmation before resetting all statistics, including the
caller number; Y confirms. Statistics reset is not part of System Status or
Log Viewer.

What a Caller Sees
------------------

Past the login prompt the board looks the way PCBoard looked, because that is
the point. The main menu is drawn from ``art/brdm``, so it is the first thing a
board usually makes its own.

.. image:: ../../assets/main_menu.png
   :alt: The main menu
   :width: 400px

The message reader shows the header PCBoard showed and takes the commands it
took.

.. image:: ../../assets/message_reader.png
   :alt: Reading a message
   :width: 400px

File listings read the ``FILE_ID.DIZ`` out of the archive, so a description
arrives with whatever colours and cursor moves its author gave it. The listing
keeps it inside its column either way; ``strip_colors_in_descriptions`` reduces
it to plain text.

.. image:: ../../assets/file_list.png
   :alt: A file listing with FILE_ID.DIZ descriptions
   :width: 400px

Tools
-----

Icy Board includes a comprehensive suite of tools for BBS management and development:

**Core Executables**

* ``icboard`` - The BBS server and local call-waiting screen
* ``icbsetup`` - Terminal-based configuration and setup utility
* ``pplc`` - PPL compiler (source → PPE)
* ``ppld`` - PPL decompiler (PPE → source)
* ``mkicbtxt`` - Edit the system messages and prompts
* ``mkicbmnu`` - Edit menus
* ``icbsm`` - System manager utility (user/group editor)
* ``icbfile`` - File-base maintenance and import
* ``icbmailer`` - FTN mail scanning, polling and tossing
* ``ppl-lsp`` - PPL language server for editors

Directory Layout
~~~~~~~~~~~~~~~~

I tried to simplify the PCBoard system a bit but it has limits.

A typical Icy Board installation follows this structure:

.. code-block:: text

   FOO/                    # Your BBS root (created by icbsetup)
   ├── icboard.toml        # Main configuration file
   ├── icboard.log         # Runtime log file
   ├── art/                # Graphics and art files
   │   └── help/           # Help Files
   ├── main/               # Main board files
   ├── conferences/        # Conference menus, files and message bases
   └── tmp/                # Generated Files for backwards compatibility

main/ files 
~~~~~~~~~~~

The ``main/`` directory contains core system configuration and data files:

See :doc:`configuration/index` for the complete TOML format reference,
including exact keys, required fields, defaults and examples. Custom commands
are explained in :doc:`customizing/adding_commands`.

**Configuration Files**

+------------------------+---------------------------------------------------------------+
| File                   | Description                                                   |
+========================+===============================================================+
| ``commands.toml``      | Command definitions and keyboard shortcuts                    |
| ``conferences.toml``   | Conference structure and access controls                      |
| ``languages.toml``     | Language definitions (date formats, yes/no chars, locale)     |
| ``protocols.toml``     | File transfer protocol configurations                         |
| ``security_levels.toml`` | Security level definitions and user limits                  |
+------------------------+---------------------------------------------------------------+

**User Management**

+------------------------+---------------------------------------------------------------+
| File                   | Description                                                   |
+========================+===============================================================+
| ``users.toml``         | User database with all registered accounts                    |
| ``groups``             | Unix-style groups file for permission management              |
| ``vip_user.txt``       | VIP users list (sysop notified on login)                      |
+------------------------+---------------------------------------------------------------+

**Security & Validation**

+------------------------+---------------------------------------------------------------+
| File                   | Description                                                   |
+========================+===============================================================+
| ``tcan_users.txt``     | Forbidden usernames (one per line)                            |
| ``tcan_passwords.txt`` | Forbidden passwords (weak/common passwords)                   |
| ``tcan_email.txt``     | Blocked email domains or addresses                            |
| ``tcan_uploads.txt``   | Prohibited upload filenames/patterns                          |
+------------------------+---------------------------------------------------------------+

**System Files**

+------------------------+---------------------------------------------------------------+
| File                   | Description                                                   |
+========================+===============================================================+
| ``icbtext.toml``       | System messages and prompts (customizable)                    |
|                        | Localized versions: ``icbtext_de.toml``, etc.                 |
| ``email.*``            | Email message base files (JAM format)                         |
+------------------------+---------------------------------------------------------------+

art/ files
~~~~~~~~~~

It is recommended to use ``.pcb``, ``.ans``, ``.rip`` and ``.asc`` extensions instead of the old ``@X`` naming scheme.
This makes it easier to draw files with an ansi 
drawing tool as well, and file-name lengths are no longer an issue.
Files can either be CP437 or UTF-8 - IcyBoard will do 
all conversions automatically. Note that UTF-8 requires the UTF-8 BOM.
This is by design it's the only way to make a 
fast and correct decision about the file encoding.

Note: UTF-8 is recommended for everything.

icbsetup
~~~~~~~~

``icbsetup`` is the interactive TUI (text user interface) utility
used to create, configure and maintain an Icy Board installation.  

It covers more than the classic setup utility.

* Create new BBS installations
* Import legacy PCBoard systems
* Help converting PPE plugins to modern systems
