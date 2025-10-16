Display File Conventions
=========================

IcyBoard supports PCBoard-compatible display file conventions, allowing you to create dynamic, personalized displays for your BBS users.

Display Filename Conventions
-----------------------------

With virtually any filename that IcyBoard displays to the caller, you can send different versions of the file based on:

* The security level of the user
* The current language
* The graphics mode (if any) being used

Base Filename
~~~~~~~~~~~~~

The **base filename** is what you enter for the filename in configuration. For example:

.. code-block:: text

   Name/Loc of Conference Join Menu: /bbs/gen/CNFN

In this example, ``CNFN`` is the base filename.

By creating new files with special characters added to the filename, you can make special versions of display files for restricted groups of users.

Language Specific
~~~~~~~~~~~~~~~~~

To make a display file for a particular language, make the filename extension equal to the language extension defined in your language configuration.

.. code-block:: text

                     BASE.LAN
                      |    |
     base filename----+    +-------language extension

.. note::
   The default language does not have a file extension. Therefore, you would use the base filename with no file extension specified.

Security Specific
~~~~~~~~~~~~~~~~~

To make a file only viewable by a particular security level, add the security level to the base filename. For example, to create a ``BRDM`` file only displayed to users with security level 40, create a file called ``BRDM40``.

.. code-block:: text

                BASE###
                 |   |
 base filename---+   +----- Security level

Graphic Specific
~~~~~~~~~~~~~~~~

IcyBoard supports two different graphics-specific files – ANSI and RIPscrip.

* Add a ``G`` to the base filename for ANSI specific files
* Add an ``R`` to the base filename for RIPscrip files

For example, to create an ANSI version of ``BRDM``, create a file called ``BRDMG``.

.. code-block:: text

                  BASEG
                   |  |
  base filename----+  +---- specifies file is for
                            graphics mode only

For RIPscrip mode, create ``BRDMR``.

.. code-block:: text

                 BASER
                  |  |
 base filename----+  +---- specifies file is for
                           RIPscrip mode only

.. note::
   If the user is in RIPscrip mode but a RIPscrip version doesn't exist, IcyBoard will attempt to find an ANSI specific version (because RIPscrip also supports ANSI). If no ANSI file is found, the base filename is used.

Combining
~~~~~~~~~

You can combine multiple methods. For example, you can make graphics and non-graphic versions of security-specific files:

.. code-block:: text

                 BASE###G
                  |   | |
 base filename----+   | +---- specifies this file
                   security   is for graphics only

You can even make this file language-specific:

.. code-block:: text

                  security
                      |     +--- language
                      |     |
                 BASE###G.LAN
                  |     |
 base filename ---+     |
                    graphics specific

Order Of Language, Security, and Graphic Specific Files
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

IcyBoard will use the most specific file that it can find. The search order is:

1. **Language** - Check if the user selected a non-default language
2. **Security** - Check for security-specific versions
3. **Graphics** - Check for graphics mode (ANSI or RIPscrip)

Example: If you have these ``NEWS`` files:

* ``NEWS``
* ``NEWS10``
* ``NEWS20``
* ``NEWS20G``
* ``NEWS.SPA``

**Scenario 1:** User with security level 20, default language, graphics mode enabled
   → Displays: ``NEWS20G``

**Scenario 2:** User selects Spanish (SPA) language, security level 20, graphics enabled
   → Displays: ``NEWS.SPA`` (language-specific file takes precedence, no security/graphics versions with SPA extension exist)

**Scenario 3:** User with security level 25, default language, graphics enabled
   → Displays: ``NEWS`` (no matching security level, no NEWSG file)

All Possible Specific Files
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

For a base file ``BRDM``, you can create:

.. code-block:: text

   BRDM              # Base file
   BRDMG             # ANSI graphics
   BRDMR             # RIPscrip
   BRDM###           # Security specific (### = security level)
   BRDM###G          # Security + ANSI
   BRDM###R          # Security + RIPscrip
   BRDM.LAN          # Language specific (.LAN = language extension)
   BRDMG.LAN         # Language + ANSI
   BRDMR.LAN         # Language + RIPscrip
   BRDM###.LAN       # Language + security
   BRDM###G.LAN      # Language + security + ANSI
   BRDM###R.LAN      # Language + security + RIPscrip

.. note::
   ``###`` represents any security level (0-255). ``.LAN`` represents any language extension you've defined.

PCBoard @ Macros
----------------

IcyBoard supports PCBoard-compatible @ macros to display dynamic information specific to the caller. All macros must begin and end with the ``@`` character with the text between being in uppercase.




Formatting @ Macros
~~~~~~~~~~~~~~~~~~~

With the exception of Action Related macros, you can format any macro with:

* **Maximum field length** - Total characters to display the macro
* **Justification** - Left (default), right (R), or center (C)

To specify a maximum field length, enter a colon followed by the field length before the last ``@``:

.. code-block:: text

   @USER:30@        # 30 character field, left justified
   @USER:30C@       # 30 character field, center justified
   @USER:30R@       # 30 character field, right justified

@X Color Codes
--------------

IcyBoard supports PCBoard's ``@X`` color codes to colorize display files and prompts. These codes are translated to ANSI for capable terminals and automatically stripped for non-ANSI users.

Alphabetical List of @ Macros
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. list-table::
   :widths: 25 75
   :header-rows: 1

   * - Macro
     - Description
   * - ``@##@``
     - Address message to specific security level in TO: field (e.g., ``@20@``)
   * - ``@##-##@``
     - Address message to security level range in TO: field (e.g., ``@20-35@``)
   * - ``@AREANAME@``
     - **[IcyBoard]** Name of current message area
   * - ``@AREANUM@``
     - **[IcyBoard]** Number of current message area
   * - ``@AUTOMORE@``
     - Subsequent More? prompts treated as ``@PAUSE@`` (auto-continue after 10 seconds)
   * - ``@BEEP@``
     - Send audible tone (CTRL-G) to remote caller
   * - ``@BICPS@``
     - Internal file transfer statistics display
   * - ``@BOARDNAME@``
     - Display BBS name from system configuration
   * - ``@BPS@``
     - Connect speed as reported by IcyBoard at login
   * - ``@BYTELIMIT@``
     - Daily download byte limit (e.g., ``737,280``)
   * - ``@BYTERATIO@``
     - Current byte ratio downloads:uploads (e.g., ``5:1``)
   * - ``@BYTESLEFT@``
     - Bytes available for download today (or "Unlimited")
   * - ``@CARRIER@``
     - Connect speed as returned by modem (e.g., ``14400``)
   * - ``@CITY@``
     - Caller's city from user record (e.g., ``MURRAY, UT``)
   * - ``@CLREOL@``
     - Clear to end of current line
   * - ``@CLS@``
     - Clear screen
   * - ``@CONFNAME@``
     - Current conference name only
   * - ``@CONFNUM@``
     - Current conference number
   * - ``@CURMSGNUM@``
     - Last message number read
   * - ``@DATAPHONE@``
     - Business/data phone from user record
   * - ``@DAYBYTES@``
     - Bytes downloaded today (negative = upload credit)
   * - ``@DELAY:nn@``
     - Pause for nn tenths of a second (0-255, max 25.5 seconds)
   * - ``@DLBYTES@``
     - Total bytes downloaded by caller
   * - ``@DLFILES@``
     - Total files downloaded by caller
   * - ``@EVENT@``
     - Time of next scheduled event (24-hour format)
   * - ``@EXPDATE@``
     - Expiration date of caller's subscription
   * - ``@EXPDAYS@``
     - Days until subscription expires
   * - ``@FILERATIO@``
     - File ratio downloads:uploads (e.g., ``5:1``)
   * - ``@FIRST@``
     - First name in mixed case (e.g., ``Stanley``)
   * - ``@FIRSTU@``
     - First name in uppercase (e.g., ``STANLEY``)
   * - ``@FREESPACE@``
     - Available upload space in current conference
   * - ``@GFXMODE@``
     - **[IcyBoard]** Current graphics mode: Off, Ansi, Avatar, or Rip
   * - ``@HANGUP@``
     - Disconnect caller (must start at beginning of line, display files only)
   * - ``@HIGHMSGNUM@``
     - Highest message number in current conference
   * - ``@HOMEPHONE@``
     - Home phone from user record
   * - ``@INCONF@``
     - Conference name and number with "Conference" suffix
   * - ``@KBLEFT@``
     - Kilobytes available for download today
   * - ``@KBLIMIT@``
     - Daily kilobyte download limit
   * - ``@LASTCALLERNODE@``
     - Last caller to current node (name and city)
   * - ``@LASTCALLERSYSTEM@``
     - Last caller to entire system (name and city)
   * - ``@LASTDATEON@``
     - Last date caller was on BBS
   * - ``@LASTTIMEON@``
     - Last time caller was on BBS (24-hour format)
   * - ``@LIST@``
     - Address message to list of users (TO: field only)
   * - ``@LMR@``
     - Last Message Read pointer in current conference
   * - ``@LOWMSGNUM@``
     - Lowest message number in current conference
   * - ``@MINLEFT@``
     - Minutes remaining (includes flagged download time)
   * - ``@MORE@``
     - Display More? prompt
   * - ``@MSGLEFT@``
     - Total messages posted by user
   * - ``@MSGREAD@``
     - Total messages read by user
   * - ``@NODE@``
     - Current node number
   * - ``@NUMAREA@``
     - **[IcyBoard]** Number of message areas in current conference
   * - ``@NUMBLT@``
     - Number of bulletins in current conference
   * - ``@NUMCALLS@``
     - Total calls answered by BBS
   * - ``@NUMDIR@``
     - Number of file directories in current conference
   * - ``@NUMTIMESON@``
     - Number of times caller has called BBS
   * - ``@OFFHOURS@``
     - Hours when lower speed callers allowed (24-hour format)
   * - ``@OPTEXT@``
     - Internal use - passes information in system prompts
   * - ``@PAUSE@``
     - More? prompt with 10-second auto-continue
   * - ``@POFF@``
     - Disable automatic More? prompts
   * - ``@PON@``
     - Enable automatic More? prompts
   * - ``@POS:nn@``
     - Move cursor to position nn on current line
   * - ``@PRODESC@``
     - Description of default file transfer protocol
   * - ``@PROLTR@``
     - Letter of default file transfer protocol
   * - ``@QOFF@``
     - Disable CTRL-X/CTRL-K abort capability
   * - ``@QON@``
     - Enable CTRL-X/CTRL-K abort capability
   * - ``@RBYTES@``
     - Internal file transfer statistics
   * - ``@RCPS@``
     - Internal file transfer statistics
   * - ``@RFILES@``
     - Internal file transfer statistics
   * - ``@SBYTES@``
     - Internal file transfer statistics
   * - ``@SCPS@``
     - Internal file transfer statistics
   * - ``@SECURITY@``
     - Current security level of caller
   * - ``@SFILES@``
     - Internal file transfer statistics
   * - ``@SYSDATE@``
     - Current system date
   * - ``@SYSOPIN@``
     - Start time for SysOp availability (24-hour format)
   * - ``@SYSOPNAME@``
     - **[IcyBoard]** SysOp name from configuration (or real name if configured)
   * - ``@SYSOPOUT@``
     - End time for SysOp availability (24-hour format)
   * - ``@SYSTIME@``
     - Current system time (24-hour format)
   * - ``@TIMELEFT@``
     - Minutes remaining (excludes flagged download time)
   * - ``@TIMELIMIT@``
     - Daily/session time limit in minutes
   * - ``@TIMEUSED@``
     - Minutes used during current call
   * - ``@TOTALTIME@``
     - Total minutes used today
   * - ``@UPBYTES@``
     - Total bytes uploaded by caller
   * - ``@UPFILES@``
     - Total files uploaded by caller
   * - ``@USER@``
     - Full username in uppercase (e.g., ``EDWARD B. SMITH``)
   * - ``@WAIT@``
     - Display "Press (Enter) to continue?" prompt
   * - ``@WHO@``
     - Display who's online (all nodes). Resets line counter - use ``@PAUSE@`` before it to prevent scrolling

.. note::
   Macros marked with **[IcyBoard]** are IcyBoard-specific extensions not found in classic PCBoard.

Format
~~~~~~

The format for ``@X`` codes is:

.. code-block:: text

   @X[background][foreground]

Color Code Table
~~~~~~~~~~~~~~~~

.. list-table::
   :widths: 20 20 20 20 20
   :header-rows: 1

   * - Color
     - Background Normal
     - Background Blinking
     - Foreground Normal
     - Foreground Bright
   * - Black
     - 0
     - 8
     - 0
     - 8
   * - Blue
     - 1
     - 9
     - 1
     - 9
   * - Green
     - 2
     - A
     - 2
     - A
   * - Cyan
     - 3
     - B
     - 3
     - B
   * - Red
     - 4
     - C
     - 4
     - C
   * - Magenta
     - 5
     - D
     - 5
     - D
   * - Yellow
     - 6
     - E
     - 6
     - E
   * - White
     - 7
     - F
     - 7
     - F

Example Color Codes
~~~~~~~~~~~~~~~~~~~

.. code-block:: text

   @X07    # Dull white (gray) text on black background
   @X47    # Dull white (gray) text on red background
   @X0B    # Bright cyan text on black background
   @X8F    # Bright white text on blinking black background
   @XC7    # Dull white (gray) text on blinking red background
   @X1F    # Bright white text on blue background

Using @ Macros and @X Codes
----------------------------

You can use ``@`` macros and ``@X`` codes in three places:

1. **Display files** - Any text file shown to users
2. **System prompts** - Configuration text entries
3. **Messages** - Inside messages left on the BBS

Example Usage
~~~~~~~~~~~~~

To display a personalized, colorized message:

.. code-block:: text

   @X1FWelcome, @USER@!@X07
   
   You have @X0E@MINLEFT@@X07 minutes remaining.

This displays "Welcome, [USERNAME]!" in bright white on blue, then shows the remaining minutes in bright yellow on black.

Advantages
~~~~~~~~~~

* **Single file** - One display file works for both ANSI and non-ANSI users
* **Dynamic content** - Information updates automatically
* **Personalization** - Each user sees content tailored to them
* **Easy maintenance** - Update one file instead of multiple versions

Special Addressing Macros
--------------------------

In the TO: field of messages, you can use special macros:

.. list-table::
   :widths: 30 70
   :header-rows: 1

   * - Macro
     - Description
   * - ``@##@``
     - Address message to specific security level (e.g., ``@20@``)
   * - ``@##-##@``
     - Address message to range of security levels (e.g., ``@20-35@``)
   * - ``@LIST@``
     - Address message to a list of users
   * - ``@USER@``
     - Makes message appear personally addressed to each reader

Best Practices
--------------

1. **Test with different user types** - Verify security, language, and graphics variants display correctly
2. **Provide fallbacks** - Always have a base file in case specific versions don't match
3. **Use consistent colors** - Create a color scheme and stick to it
4. **Balance information** - Don't overload screens with too many macros
5. **Consider non-ANSI users** - Ensure content is readable without colors
6. **Document your files** - Keep notes on which variants exist for each display file

See Also
--------

* :doc:`look_and_feel` - General BBS appearance customization
* :doc:`menus` - Menu system configuration
* :doc:`internationalization` - Multi-language support