
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)

var
   fd:      text;
   l1,l2:   string;

begin
   assign(fd,paramstr(1));
   reset(fd);
   readln(fd,l1);
   close(fd);

   assign(fd,paramstr(2));
   reset(fd);
   readln(fd,l2);
   close(fd);

   if l1 = l2 then
      halt(0);

   writeln('Mismatch!');
   writeln('   ',paramstr(1),' [',l1,']');
   writeln('   ',paramstr(2),' [',l2,']');
   halt(1);
end.
