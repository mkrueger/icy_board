
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)

var
   line: string;
   par:  string;
   val:  string;

   name:    string;
   addr1:   string;
   addr2:   string;
   addr3:   string;
   cardnum: string;
   cardexp: string;
   amount:  string;

procedure clean(var s: string);
var
   temp: string;
   i:    integer;
begin
   temp := '';
   i := 1;
   for i := 1 to length(s) do
      case s[i] of
         '0'..'9':   temp := temp + s[i];
      end;
   s := temp;
end;

procedure clear;
begin
   name := '';
   addr1 := '';
   addr2 := '';
   addr3 := '';
   cardnum := '';
   cardexp := '';
   amount := '';
end;

procedure process_order;
begin
   clean(cardnum);
   clean(cardexp);
   clean(amount);
   writeln;
   writeln('name  ',name);
   writeln('addr1 ',addr1);
   writeln('addr2 ',addr2);
   writeln('addr3 ',addr3);
   writeln('card  ',cardnum);
   writeln('exp   ',cardexp);
   writeln('amt   ',amount);
   clear;
end;

begin
   clear;
   while not eof do
   begin
      readln(line);
      par := copy(line,1,24);
      val := copy(line,26,255);
      if val <> '' then
      begin
         if par = '                Name(1):' then name := val else
         if par = '     Billing Address(1):' then addr1 := val else
         if par = '                    (2):' then addr2 := val else
         if par = '                    (3):' then addr3 := val else
         if par = '            Card Number:' then cardnum := val else
         if par = '        Expiration Date:' then cardexp := val else
         if par = '           Total Amount:' then amount := copy(val,2,255) else
         if par = '        [Processed by Pr' then process_order;
      end;
   end;
end.

