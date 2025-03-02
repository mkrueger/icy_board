# Icy Board

![Login screen](assets/login_screen.png?raw=true "Login screen")

IcyBoard aims to re-create PCBoard - one of the most famous DOS BBS system of it's time.
I've started this project to learn rust - beginning with porting a decompiler from Adrian Studer
 (https://github.com/astuder/ppld). Which was famous in the early days of 

It works quite well but the PPEs require a BBS to be running to be useful. First approach was to make a general runtime, however PPEs are too PCboard specific.
So all it's needed is a new PCBoard.
There are data structures for almost all PCBoard data structures so making a BBS is the next logical step. Don't expect anything to be runnable soon.


## Goals

* Have a modernized version of PCBoard that runs on modern systems - esp. linux/Raspberry Pi
* Be as compatible as possible (PPE/Handling)
* Provide the whole PCBoard eco system - including config tools
* Maintain the spirit/look & feel of PCBoard (at least for a while)
* Make it as easy as possible to run existing PCBoard installations
* Extend PCBoard, without breaking existing PPEs

### Non Goals

* GUI config tools - it's ment for running on a SSH session :)
  * However if one will invest the time for making a UI it's welcome but the goal is to have modern TUIs.
* Making a shiny out of the box BBS that you just have to install & run. PCBoard was hard - IcyBoard will be hard too
  This is intended - all modern BBSes look the same. You can choose from a wide range of plugins and configuration options.
  Use it!

# Topics
* [GettingStarted](docs/gettingstarted.md)
* [PCBoard Feature Status](docs/feature_parity.md)
* [Differences](docs/differences.md)
  * [New @ Macros](docs/new_macros.md)
* [PPL](docs/ppl.md)
  * [PPLC](docs/pplc.md)
  * [New PPL functions](docs/new_ppl.md)
