# Getting Started

## A board in five minutes

```sh
icbsetup create mybbs     # writes a complete board into mybbs/
cd mybbs
icboard                   # call waiting screen, telnet on port 1337
```

Pick *Sysop* on the call waiting screen with the arrow keys and press Enter to
log in locally, start the board with `icboard --localon` to go straight in, or
reach it from another terminal with `telnet localhost 1337`. Your terminal
needs at least 80x25.

That is a running board. What follows is where everything lives and how to
bring an old installation over. Before you move a real board, read
[known limitations](known_limitations.md) - it says what is missing and what
works differently than PCBoard did.

## Getting the programs

Grab a release for your operating system:
https://github.com/mkrueger/icy_board/releases/latest

Or build from source, which needs a [rust toolchain](https://www.rust-lang.org/tools/install):

`cargo build --release`

If something is missing on your system cargo build will tell you. If you know hat a development environment is it should be straightforward.

Update: On my pi I needed to install openssl-dev:
`sudo apt-get install openssl-dev`


I develop this software on linux - next time I set up I'll add a more detailed description.

## The programs

| Program | What it is for |
| :--- | :--- |
| `icboard` | The board itself. Started in the directory that holds `icboard.toml`. |
| `icbsetup` | Creates a board, imports a PCBoard one, and edits every setting. Start here. |
| `icbsm` | User and group editor, packs the user file and runs the bulk edits. |
| `mkicbtxt` | Edits the system messages, which is how most of the board is reworded. |
| `mkicbmnu` | Edits menus. |
| `icbfile` | Brings a file base into shape - see [icbfile](icbfile.md). |
| `icbmailer` | FTN mail, scan, poll and toss. |
| `pplc`, `ppld` | PPL compiler and decompiler - see [PPL](ppl.md). |
| `icyboard-ppl` | Editor support for PPL: diagnostics, completion, formatting. |

I recommend putting the bin/ directory in the path but you can just `cd bin` for now.

## The first half hour

1. `icbsetup create mybbs`, then `cd mybbs`.
2. `icbsetup` - board name, sysop name and password, and the number of nodes.
   Options the board does not read yet are greyed out and say so.
3. `mkicbtxt` if you want to reword prompts, `icbsm` for users.
4. `icboard` and log in locally, then walk the menu once: `J` join a
   conference, `E` enter a message, `R` read it back, `F` the file
   directories, `G` goodbye.
5. Read `icboard.log` afterwards. It is the first place to look when something
   goes wrong.

NOTE: Ensure that your terminal screen is big enough - 80x25 at least.

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

Importing old installatins is generally difficult mostly because of complex setup situations, PPEs and so on. However `icbsetup import PCBDAT.OLD <OUT_PATH>` will try to import old installations. Instead of the file the directory of the PCBoard installation can be given - `icbsetup import ~/CSB <OUT_PATH>` looks the PCBOARD.DAT up itself.

`--dry-run` imports into a temporary directory and only reports which paths could not be resolved. Paths that pointed to another drive can be given with `--map`, the option may be repeated:

`icbsetup import ~/CSB out --dry-run --map 'D:\FILES=/mnt/files'`

Beside the importlog.txt the import writes an import_report.txt with the counts and the list of unresolved paths.

the importlog.txt contains all operations done and it should usually be enough to turn on an existing pcboard installation in icyboard.

However it'll be required to update all PPEs one by one. Moving them to another directory, making file names relative etc.

I'm interested in bugs & existing installations to improve the import process. But it should be a good starting point to update an existing PCBoard to Icy Board.
