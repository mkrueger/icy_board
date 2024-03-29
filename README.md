# icy_board

Aim is to make PCBoard clone that's finished on PCboards 100th birthday - for sure it'll take much time.
I've started to port a PPL decompiler 2022 and it's now a compiler, decompiler and runtime.
(https://github.com/mkrueger/PPLEngine)

It works quite well but the PPEs require a BBS to be running to be useful. First approach was to make a general runtime, however PPEs are too PCboard specific.
So all it's needed is a new PCBoard.
There are data structures for almost all PCBoard data structures so making a BBS is the next logical step. Don't expect anything to be runnable.

![Login screen](assets/login_screen.png?raw=true "Login screen")
