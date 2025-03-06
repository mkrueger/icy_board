# Differences to PCBoard

The goal is to be as compatible as possible but enhancing PCBoard for the moderen age implies changes that may break compatibiltiy.

* All config files are in .toml format and can be changed with any text editor
  * Old config files are created automatically for old .PPE compatibility files but some PPEs might break. (Report please)
  * TOML formats include: Bullettins, Script Questionnaires and Menus
  * Exceptions so far: Trashcan file 
* Long file name support! yeah! (However old config files have limits)
  * Applies to old PPEs as well they got their limits extended, unless they check for file lengths themself.
* Message base format changed to JAM
  * Planned is to support multiple message base formats
  * Means all PPEs/tools break that may read the old format
* 'DIR' files are now binary - they're no longer a text file. They contain meta data.

## CP437 is dead

All files should be UTF-8. The importer automatically converts files to UTF-8.

However it's possible to use CP437 files with IcyBoard. Icy Board differenciates UTF-8 from CP437 with the UTF8 BOM.
All files without  FF EF FE at the start are treated as UTF8. So it's no hard requirement. Copy old DOS files and they will show up fine regardless of CP437 or UTF-8 used.

So it's possible to use a modern (non CP437) text editor to alter the BBS files.

* Config files, Menus, (in general: non display text files) are ALWAYS UTF-8.
* UTF8->CodePage 437 chars will be translated by the table found here: <https://en.wikipedia.org/wiki/Code_page_437>
  
## Command changes

* Added Read & Write email (@/@W)

## PPE/Menus

* `.MNU` files are converted to a new .toml format (in the assumption that no `.PPE` will handle `.MNU` files)
* PCBoard is now case sensitive on unix but that does NOT apply to `.PPE` files. Note: May change for new `.PPE` files but will always remain for old ones for compatiblity reasons.

## Trashcan

A simple text file read line by line. If a line starts with '#' it's treated as a comment.
Leading & traling whitespaces are ignored


## Enhancements

* Access System is more complex. The old one had user levels. An access now consists of a combination of:
  * security level
  * group (like unix groups)
  * age 
* Added more trashcans: email/passwords
* New 'vip' users vip_users.txt (same as trashcan). But a list of users which the sysop gets a notification for logon (from RemoteAccess)
* Surveys (questionnaires) header can now have a different header length than 5 lines
Header is separated from questions with a line starting by "*****". Note that the pcb importer inserts a "*****" line after the 5th during import.

## Planned Enhancements/Discussion 

* Access system time limit - so it's only open at certain days & times - however only command where that makes sense to me is the sysop page.
But I like the RA "DayTimes" system.

