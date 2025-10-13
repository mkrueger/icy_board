Icy Board
=========


Call Waiting Screen
-------------------

.. image:: _images/login_screen.png
   :alt: Login screen
   :width: 400px

The call waiting screen is what you see when you start Icy Board. It's like the PCBoard call 
waiting screen but modernized a bit. All important Icy Board configuration utilities are accessible from here.

Options Explained
-----------------

User - Busy/Not Busy
~~~~~~~~~~~~~~~~~~~~

Log in as a regular user. This gets you to login prompt. 
You can log in as any user that exists in the users file.

Sysop - Busy/Not Busy
~~~~~~~~~~~~~~~~~~~~~~
Log in as sysop. This gets you to the command prompt directly.

Shell - Busy/Not Busy
~~~~~~~~~~~~~~~~~~~~~
Just quits the Call Waiting Screen. Historical reasons...

Call Log - On/Off
~~~~~~~~~~~~~~~~~
Toggle call logging. If on, all calls are logged to icboard.log.

Page Bell - On/Off
~~~~~~~~~~~~~~~~~~~
Toggle the page bell. If on, the terminal bell will ring when a user pages sysop

Alarm - On/Off
~~~~~~~~~~~~~~~~~~~~~~~
Toggle the alarm bell. If on, the terminal bell will ring when a user logs in.

ICBSysMgr
~~~~~~~~~
Start the system manager utility. This is a TUI utility to manage users and groups.

ICBText
~~~~~~~
Start the ICBText editor. This is a TUI utility to edit the system messages and prompts.

ICBSetup
~~~~~~~~
Start the setup utility. This is a TUI utility to create and configure an Icy Board
installation.

ICBMoni
~~~~~~~
Start the monitor utility. This is a TUI utility to monitor system activity.
Nodes & logged on users and which ports Icy Board is listening on.

Tools
-----

Icy Board includes a comprehensive suite of tools for BBS management and development:

**Core Executables**

* ``icboard`` - The main BBS server daemon
* ``icbsetup`` - Terminal-based configuration and setup utility
* ``pplc`` - PPL compiler (source → PPE)
* ``ppld`` - PPL decompiler (PPE → source)
* ``mkicbtxt`` - Create/Edit ICBTXT files containing all strings used.
* ``icbsysmgr`` - System manager utility (user/group editor)
* ``ppl-language-server`` - Language server for PPL (for IDE integration)

Directory Layout
~~~~~~~~~~~~~~~~

I tried to simplify the PCBoard system a bit but it has limits.

A typical Icy Board installation follows this structure:

.. code-block:: text

   FOO/                    # Your BBS root (created by icbsetup)
   ├── icboard.toml        # Main configuration file
   ├── icboard.log         # Runtime log file
   ├── users.toml          # User database
   ├── art/                # Graphics and art files
   │   └── help/           # Help Files
   ├── main/               # Main board files
   ├── conferences/        # Conference data
   └── tmp/                # Generated Files for backwards compatibility

main/ files 
~~~~~~~~~~~

The ``main/`` directory contains core system configuration and data files:

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
| ``tcan_user.txt``      | Forbidden usernames (one per line)                            |
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

It's recommended to use .pcb, .ans, .rip, .asc extensions instead of the old …G, …R sheme. 
This makes it easier to draw files with an ansi 
drawing tool as well. And file name lengths ar no longer an issue.
Files can either be CP437 or UTF-8 - IcyBoard will do 
all conversions automatically. Note that UTF-8 requires the UTF-8 BOM.
This is by design it's the only way to make a 
fast and correct decision about the file encoding.

Note: UTF-8 is recommended for everything.

icbsetup
~~~~~~~~

`icbsetup` is the interactive TUI (text user interface) utility 
used to create, configure and maintain an Icy Board installation.  

It's more than the classic PCBoard PCBSETUP untility.

* Create new BBS installations
* Import legacy PCBoard systems
* Help converting PPE plugins to modern systems
