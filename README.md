# icy_board

Aim is to make PCBoard clone that's finished on PCboards 100th birthday - for sure it'll take much time.
I've started to port a PPL decompiler 2022 and it's now a compiler, decompiler and runtime.
(https://github.com/mkrueger/PPLEngine)

It works quite well but the PPEs require a BBS to be running to be useful. First approach was to make a general runtime, however PPEs are too PCboard specific.
So all it's needed is a new PCBoard.
There are data structures for almost all PCBoard data structures so making a BBS is the next logical step. Don't expect anything to be runnable soon.

![Login screen](assets/login_screen.png?raw=true "Login screen")

## Goals

* Have a modernized version of PCBoard that runs on modern systems - esp. linux/Raspberry Pi
* Be as compatible as possible (PPE/Handling)
* Provide the whole PCBoard eco system - including config tools
* Maintain the spirit/look & feel of PCBoard (at least for a while)
* Make it as easy as possible to run existing PCBoard installations

### Non Goals

* GUI config tools - it's ment for running on a SSH session :)
  * However if one will invest the time for making a UI it's welcome but the goal is to have modern TUIs.

## Differences to PCBoard

The goal is to be as compatible as possible but enhancing PCBoard for the moderen age implies changes that may break compatibiltiy.

* All config files are in .toml format and can be changed with any text editor
  * Old config files are created automatically for old .PPE compatibility files but some PPEs might break. (Report please)
* Long file name support! yeah! (However old config files have limits)
  * Applies to old PPEs as well they got their limits extended, unless they check for file lengths themself.
* Message base format changed to JAM
  * Planned is to support multiple message base formats
  * Means all PPEs/tools break that may read the old format
* Bullettins are now in text format as well. They're using the trashcan format.

### CP437 is dead

All files should be UTF-8. The importer automatically converts files to UTF-8.

However it's possible to use CP437 files with IcyBoard. Icy Board differenciates UTF-8 from CP437 with the UTF8 BOM.
All files without  FF EF FE at the start are treated as UTF8. So it's no hard requirement. Copy old DOS files and they will show up fine regardless of CP437 or UTF-8 used.

So it's possible to use a modern (non CP437) text editor to alter the BBS files.

* Config files, Menus, (in general: non display text files) are ALWAYS UTF-8.
* UTF8->CodePage 437 chars will be translated by the table found here: <https://en.wikipedia.org/wiki/Code_page_437>
  
### Command changes

* Conferences can now have more than one message area - message area switch is implemented with "I" command
* "I" command switched to "IW"

### PPE/Menus

* `.MNU` files are converted to a new .toml format (in the assumption that no `.PPE` will handle `.MNU` files)
* PCBoard is now case sensitive on unix but that does NOT apply to `.PPE` files. Note: May change for new `.PPE` files but will always be remain for old ones.

### Trashcans/Bullettins

A simple text file read line by line. If a line starts with '#' it's treated as a comment.
Leading & traling whitespaces are ignored
