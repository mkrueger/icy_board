# Roadmap/TODO

- [ ] Implement all search commands (text/file search)
- [ ] Mail QWK support
- [ ] Solution for file base meta data 
  - I don't want a "DIRS" file but there is metadata that is not included in the .ZIP - let's say uploader.
    Every other thing is easy to extract from the ZIP file.
- [ ] Support more compression algorithms - there is a ZIP Library PR pending which I need to take care of.
  I implemented all outdated ZIP algorithms for the rust zip libary so that the files from the 80' all extract.
  But this needs to be merged.
- [ ] Finsh/sync commands & help files
- [ ] ICBSETUP needs a "cmd editor"
- [ ] Rework the mkicbmnu - due to changes in the icbsetup menu system that got broken
- [ ] Look at the NEWS/INTRO feature of PCBOARD how that really works
- [ ] Logon mail scan
- [ ] Finish internal message reader - I don't like it have a message reader PPE going but needs to be finished in any case…
- [ ] Implement RM command
- [x] Implement SELECT command
- [ ] Implement group chat (CHAT command)
- [ ] Finish SSH/Websocket support - works somewhat but SSH only with icy term so far
- [ ] Mailer Support (BINKP?)
- [ ] Go throught the PCBoard options and ensure they're working - atm some do - some don't

-> 1st BETA

After BETA

- [ ] PPL tree sitter grammar
- [ ] PPL web statements/functions
- [ ] PPL dabase3 statements/functions
- [ ] Self-service password reset using email
- [ ] Web Frontend (IcyTerm can run as Webassembly but needs the data from somewhere)
- [ ] Support for IcyAnim - no need to use icy_play in icy_board anymore 
- [ ]