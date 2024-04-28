
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)


const
   maxw = 75;

var
   c: char;
   pc: char;
   ppc: char;
   b: integer;
   w: integer;

   ib,ob:   array[1..10240] of char;

begin
   settextbuf(input,ib);
   settextbuf(output,ob);
   c := '?';
   pc := '?';
   w := 0;

   while not eof do
   begin
      ppc := pc;
      pc := c;
      read(c);
      c := upcase(c);

      case c of
         '}',')':
               begin
                  b := ord(ppc)-ord('0');
                  if ppc > '9' then dec(b,7);

                  b := (b*16) + ord(pc)-ord('0');
                  if pc > '9' then dec(b,7);

                  inc(w);
                  case b of

                     $1b:
                        begin
                           writeln('<end>');
                           w := 0;
                        end;

                     0..32:
                           write('.');

                     128..128+32:
                           write('*');

                     else
                           write(chr(b and 127));
                  end;
               end;

         '{':  if pc <> '}' then
               begin
                  writeln;
                  writeln;
                  writeln('TX: ');
                  w := 0;
               end;

         '(':  if pc <> ')' then
               begin
                  writeln;
                  writeln;
                  writeln('RX: ');
                  w := 0;
               end;
      end;

      case c of
         '{','(':
               if w > maxw then
               begin
                  writeln;
                  w := 0;
               end
               else

               if w > 0 then
               begin
                  write(' ');
                  inc(w);
               end;

         ')','}':
               ;

         else
            begin
               write(c);
               inc(w);
            end;
      end;
   end;

   writeln;

end.

