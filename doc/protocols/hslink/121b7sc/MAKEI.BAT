rem Make INTERRUPT version of HSLINK
if not exist make.f goto again
urm HSLINK.EXE HSUID.OBJ SETCRC.EXE LINK.OPT COMP.OPT
urm MAKE.F
:again
rem c:\bc\bin\make %1 %2
R:\bp\bin\make %1 %2
if errorlevel 1 pause
smartdrv /c
