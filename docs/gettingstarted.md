# Getting Started

First grab a release for your operating system:
https://github.com/mkrueger/icy_board/releases/latest

Or build from source. Building from source is easy as well.
It's installing a [rust development](https://www.rust-lang.org/tools/install) environment and just run

`cargo build --release`

If something is missing on your system cargo build will tell you. If you know hat a development environment is it should be straightforward.

Update: On my pi I needed to install openssl-dev:
`sudo apt-get install openssl-dev`


I develop this software on linux - next time I set up I'll add a more detailed description.

# Startup Icy Board

I recommend putting the bin/ directory in the path but you can just `cd bin` for now.

First create a new BBS: `./icbsetup create FOO`
Then start it: `./icboard FOO`

This will fire up a new call waiting screen where you can log in as sysop. By defaulut telnet is enabled on port 1337.

NOTE: Ensure that your terminal screen is big enough - 80x25 at least.

# Tools

* Most important is ICBSetup - that contains all options for IcyBoard. It's a mess!
* ICBText - there you can edit all text messages. This is the main way of extending IcyBoard through PPEs
* ICBSysMgr - that let's you edit the users.

# Directory Layout

I tried to simplify the PCBoard system a bit but it has limits.

I designed IcyBoard for using relative paths. However absolute ones can be used. Relative path root is always where the main icboard.toml is. Regardless of file position.
This makes it easier to move files around - if needed and cut & paste etc.

Basically the file Layout is:
| File/Dir | Description|
| --- | --- |
|icyboard.toml | Main Config File |
|icyboard.log | Log File |
|art/| All ANSIS go in there | 
|art/help/| Help Files | 
|main/| All other bbs files are here | 
|conferences/| Conference data (files/messages) |
|tmp/| Generated Files for backwards compatiblity |

The log file is very important. If something goes wrong it's likely that the log file tells you why.

## main/ files 

| File | Description|
| --- | --- |
|commands.toml | All Commands |
|conferences.toml | Conference data |
|groups| Unix Like /etc/gorups file | 
|icbtext.toml| Contains all Icy Board System Messages | 
|languages.toml| Language descriptions (Date Formats, yes/no characters & localized icbtext.toml locations) | 
|protocols.toml| List & Description of available transfer protocols |
|security_levels.toml| Security Levels & Limits |
|users.toml| Contains registered all User Records |
|tcan_user.txt| Forbidden user names |
|tcan_passwords.txt| Forbidden user passwords |
|tcan_email.txt| Forbidden emails |
|tcan_uploads.txt| Forbidden upload file names |
|vip_user.txt| Users where the sysop is informed about a login |
|email.*| Email message base |

*NOTE: The location & name of all files can be changed in the main icboard.toml.*

# ART files

It's recommended to use .pcb, .ans, .rip, .asc extensions instead of the old *G, *R sheme. 
This makes it easier to draw files with an ansi drawing tool as well. And file name lengths ar no longer
an issue.
Files can either be CP437 or UTF-8 - IcyBoard will do all conversions automatically. Note that UTF-8 requires the UTF-8 BOM. This is by design it's the only way to make a fast and correct decision about the file encoding.

Note: UTF-8 is recommended for everything.

# Importing old installations

Importing old installatins is generally difficult mostly because of complex setup situations, PPEs and so on. However `icbsetup import PCBDAT.OLD <OUT_PATH>` will try to import old installations. 

the importlog.txt contains all operations done and it should usually be enough to turn on an existing pcboard installation in icyboard.

However it'll be required to update all PPEs one by one. Moving them to another directory, making file names relative etc.

I'm interested in bugs & existing installations to improve the import process. But it should be a good starting point to update an existing PCBoard to Icy Board.
