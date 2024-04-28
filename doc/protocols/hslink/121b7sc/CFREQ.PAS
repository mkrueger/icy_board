
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)


uses mdosio;

const
   cct : array [1..6] of integer = (2, 3, 17, 19, 24, 30);
   
var
   cf,cf2:             array [0..255] of integer;
   c,pc:               char;
   i:                  integer;
   fd:                 dos_handle;
   n:                  integer;
   buf:                array [1..10240] of char;

begin
   for i := 0 to 255 do
      cf[i]:= 0;
   for i := 0 to 255 do
      cf2[i]:= 0;
   
   fd := dos_open (paramstr (1), open_read);
   
   repeat
      n := dos_read (fd, buf, sizeof (buf));
      
      for i := 1 to n do
      begin
         c := buf[i];
         inc(cf [ord(c)]);
         if c = pc then
            inc(cf2 [ord(c)]);
         pc := c;
      end;
   until n < sizeof (buf);
   
   dos_close(fd);
   
   writeln('single:');
   for i := 1 to 6 do
      writeln(cf [cct [i]]: 5, cct [i]: 4);
   writeln;
   for i := 1 to 6 do
      writeln(cf [cct [i] xor $80]: 5, cct [i] xor $80 : 4);

   writeln;
   writeln('double:');
   for i := 1 to 6 do
      writeln(cf2 [cct [i]]: 5, cct [i]: 4);
   writeln;
   for i := 1 to 6 do
      writeln(cf2 [cct [i] xor $80]: 5, cct [i] xor $80 : 4);

end.

