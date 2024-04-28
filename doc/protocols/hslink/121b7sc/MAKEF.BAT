rem Make FOSSIL version of hslink
if exist make.f goto again
urm FHSLINK.EXE HSLINK.EXE HSUID.OBJ SETCRC.EXE LINK.OPT COMP.OPT
rem >make.f
:again
rem c:\bc\bin\make -DFOSSIL %1 %2
R:\bp\bin\make -DFOSSIL %1 %2
if errorlevel 1 pause
