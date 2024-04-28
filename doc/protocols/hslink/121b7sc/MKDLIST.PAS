
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)

(*
 * mkdlist - find all available files in a subdirectory tree
 *
 *)

{$m 10000,0,500000}

uses DOS;

const
   version = 'MKDLIST 1.0 of 02-18-92   (C) 1992 Samuel H. Smith';

procedure getfiles(dir: dirstr);
var
   DirInfo:       SearchRec;

begin
   if dir[length(dir)] = '\' then
      dec(dir[0]);

   FindFirst(dir+'\*.*',Anyfile,DirInfo);
   while (DosError = 0) do
   begin
      if ((DirInfo.Attr and Directory) = 0) then
         writeln(DirInfo.name);
      FindNext(DirInfo);
   end;

   FindFirst(dir+'\*.*',Anyfile,DirInfo);
   while (DosError = 0) do
   begin
      if ((DirInfo.Attr and Directory) <> 0) and (DirInfo.name[1] <> '.') then
         getfiles(dir+'\'+DirInfo.Name);
      FindNext(DirInfo);
   end;
end;

var
   i:       integer;
   outbuf:  array[1..32000] of char;

begin
   settextbuf(output,outbuf);

   if paramcount = 0 then
   begin
      writeln(version);
      writeln;
      writeln('Usage:  MKDLIST dir ... dir [>out]');
      writeln;
      writeln('Example:');
      writeln(' ECHO C:\UPLOAD >C:PROTO\DUPLIST.DAT');
      writeln(' MKDLIST C:\DL1 D:\DL2 K:\ >>C:PROTO\DUPLIST.DAT');
      writeln(' MKDLIST L:\COMM L:\GIF L:\EDIT L:\GRAPH >>C:PROTO\DUPLIST.DAT');
      writeln(' MKDLIST K:\ >>C:PROTO\DUPLIST.DAT');
      writeln;
      halt(1);
   end;

   for i := 1 to paramcount do
      getfiles(paramstr(i));
end.

