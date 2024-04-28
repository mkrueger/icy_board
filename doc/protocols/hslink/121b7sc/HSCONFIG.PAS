
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)


(*
 * hsconfig - HS/Link configuration file manager
 *
 *)

{$B-}    {Boolean complete evaluation}
{$S-}    {Stack checking}
{$R-}    {Range checking}

{$M 16000,12000,500000}  {Stack(min=12), minheap(min=40), maxheap}

program hsConfig;

uses
{   ERRTRAP,  }
   Dos,
   Crt,
   DosMem,
   Tools,
   DispEdit,
   Helpme;

{$I LANGUAGE.INC}

const
{$i hsctext.inc}

const
   copyright:  string[80] = TX_COPYRIGHT;

var
   cfg_fname:  string[65];

{$i options.i}
{$i hsconfig.hli}

(* -------------------------------------------------- *)
procedure reset_colors;
begin
   title_attr := O_CT;
   border_attr := O_CB;
   prompt_attr := O_CW;
   data_attr := O_CW;
   input_attr := O_CM;
   directvideo := O_NV and O_NF;
end;

(* -------------------------------------------------- *)
procedure title_screen(y1,y2: integer; title1,title2: string);
begin
   reset_colors;
   setattr(LightGray);
   clear_screen;

   setattr( O_CB );
   display_border(2,y1,79,y2,mixed_border);

   window(3,y1+1,78,y2-1);
   setattr( O_CW );
   clrscr;
   window(1,1,80,25);

   setattr( O_CT );
   gotoxy(34-length(title1) div 2,y1);
   write(' HS/Link: ',title1,' ');

   gotoxy(39-(length(title2) div 2),y1+2);
   write(' ',title2,' ');

   setattr( O_CM );
   gotoxy(2,25);
   disp(TX_PGDN);
   gotoxy(4+length(TX_PGDN),25);
   disp(TX_PGUP);
   gotoxy(77-length(TX_EXIT)-length(TX_HELP),25);
   disp(TX_HELP);
   gotoxy(79-length(TX_EXIT),25);
   disp(TX_EXIT);
end;


(* -------------------------------------------------- *)
procedure basic_settings(var ex: char);
const
   maxent = 3;   {entries on screen}
var
   entry:   integer;
   pentry:  integer;

   procedure proc_entry(func:  edit_functions);
   begin
      case entry of
         1: edit_integer(func,   5, 5, TX_P, O_P,1,1,8,ex);
         2: edit_fname  (func,   5, 7, TX_U, O_U,28,true,ex);
         3: edit_yesno  (func,   5, 9, TX_HS, O_HS,ex);
      end;

      if ex = F1 then
         case entry of
            1: help(8,7,'-P');
            2: help(8,9,'-U');
            3: help(8,11,'-HS');
         end;

      select_next_entry(func,entry,maxent,ex);
   end;
   
begin
   pentry := 1;
   title_screen(1,11,TX_PG1,TX_SUB1);

   for entry := 1 to maxent do
      proc_entry(display);

   helpscreen(13,13,TX_WELCOME);
   window(1,1,80,25);

   ex := DOWN;
   entry := pentry;

   repeat
      proc_entry(edit);
   until (ex <> UP) and (ex <> DOWN) and (ex <> F1);
end;

(* -------------------------------------------------- *)
procedure com_settings(var ex: char);
const
   maxent = 9;   {entries on screen}
var
   entry:   integer;
   pentry:  integer;

   procedure proc_entry(func:  edit_functions);
   begin
      case entry of
         1: edit_hex    (func,   5, 5,  TX_PB,O_PB,4,ex);
         2: edit_integer(func,   5, 7,  TX_PI,O_PI,2,0,15,ex);
         3: edit_longint(func,   5, 9,  TX_B,O_B,6,0,115200,ex);
         4: edit_longint(func,   5, 11, TX_E,O_E,6,0,115200,ex);
         5: edit_yesno  (func,   5, 13, TX_HC,O_HC,ex);
         6: edit_yesno  (func,   5, 15, TX_HR,O_HR,ex);
         7: edit_yesno  (func,   5, 17, TX_HX,O_HX,ex);
         8: edit_yesno  (func,   5, 19, TX_C,O_C,ex);
         9: edit_yesno  (func,   5, 21, TX_NT,O_NT,ex);
      end;

      if ex = F1 then
         case entry of
            1: help(4, 2+5, '-PB');
            2: help(4, 2+7, '-PI');
            3: help(4, 2+9, '-B');
            4: help(4, 2+11, '-E');
            5: help(4, 3, '-HC');
            6: help(4, 5, '-HR');
            7: help(4, 7, '-HX');
            8: help(4, 9, '-C');
            9: help(4, 11, '-NT');
         end;

      select_next_entry(func,entry,maxent,ex);
   end;
   
begin
   pentry := 1;
   title_screen(1,23,TX_PG2,TX_SUB2);

   for entry := 1 to maxent do
      proc_entry(display);

   ex := DOWN;
   entry := pentry;

   repeat
      proc_entry(edit);
   until (ex <> UP) and (ex <> DOWN) and (ex <> F1);
end;


(* -------------------------------------------------- *)
procedure option_settings_1(var ex: char);
const
   maxent = 9;  {entries on screen}
var
   entry:   integer;
   pentry:  integer;

   procedure proc_entry(func:  edit_functions);
   begin
      case entry of
         1: edit_yesno  (func,   5, 5,  TX_R,O_R,ex);
         2: edit_yesno  (func,   5, 7,  TX_K,O_K,ex);
         3: edit_yesno  (func,   5, 9,  TX_O,O_O,ex);
         4: edit_yesno  (func,   5, 11, TX_NB,O_NB,ex);
         5: edit_yesno  (func,   5, 13, TX_NC,O_NC,ex);
         6: edit_integer(func,   5, 15, TX_S,O_S,4,64,4096,ex);
         7: edit_integer(func,   5, 17, TX_W,O_W,4,0,1000,ex);
         8: edit_yesno  (func,   5, 19, TX_A,O_A,ex);
         9: edit_yesno  (func,   5, 21, TX_BANG,O_BANG,ex);
      end;

      if ex = F1 then
         case entry of
            1: help(4, 2+5, '-R');
            2: help(4, 2+7, '-K');
            3: help(4, 2+9, '-O');
            4: help(4, 2+11, '-NB');
            5: help(4, 2+13, '-NC');
            6: help(4, 2+15, '-S');
            7: help(4, 2+17, '-W');
            8: help(4, 2+19, '-A');
            9: help(4, 2+21, '-!');
        end;

      select_next_entry(func,entry,maxent,ex);
   end;
   
begin
   pentry := 1;
   title_screen(1,23,TX_PG3,TX_SUB3);

   for entry := 1 to maxent do
      proc_entry(display);

   ex := DOWN;
   entry := pentry;

   repeat
      proc_entry(edit);
   until (ex <> UP) and (ex <> DOWN) and (ex <> F1);
end;


(* -------------------------------------------------- *)
procedure option_settings_2(var ex: char);
const
   maxent = 9;  {entries on screen}
var
   entry:   integer;
   pentry:  integer;

   procedure proc_entry(func:  edit_functions);
   begin
      case entry of
         1: edit_integer(func,   5, 3+entry*2,  TX_I,O_I,1,0,4,ex);
         2: edit_yesno  (func,   5, 3+entry*2,  TX_NM,O_NM,ex);
         3: edit_yesno  (func,   5, 3+entry*2,  TX_N5,O_N5,ex);
         4: edit_integer(func,   5, 3+entry*2,  TX_FT,O_FT,3,0,128,ex);
         5: edit_fname  (func,   5, 3+entry*2,  TX_LF,O_LF,28,false,ex);
         6: edit_fname  (func,   5, 3+entry*2,  TX_NO,O_NO,28,false,ex);
         7: edit_yesno  (func,   5, 3+entry*2,  TX_FC,O_FC,ex);
         8: edit_yesno  (func,   5, 3+entry*2,  TX_NA,O_NA,ex);
         9: edit_integer(func,   5, 3+entry*2,  TX_NE,O_NE,2,0,99,ex);
      end;

      if ex = F1 then
         case entry of
            1: help(4, 2+5, '-I');
            2: help(4, 2+1, '-NM');
            3: help(4, 2+9, '-N5');
            4: help(4, 2+4, '-FT');
            5: help(4, 2+13,'-LF');
            6: help(4, 2+0, '-NO');
            7: help(4, 2+6, '-FC');
            8: help(4, 2+4, '-NA');
            9: help(4, 2+6, '-NE');
        end;

      select_next_entry(func,entry,maxent,ex);
   end;
   
begin
   pentry := 1;
   title_screen(1,23,TX_PG4,TX_SUB4);

   for entry := 1 to maxent do
      proc_entry(display);

   ex := DOWN;
   entry := pentry;

   repeat
      proc_entry(edit);
   until (ex <> UP) and (ex <> DOWN) and (ex <> F1);
end;


(* -------------------------------------------------- *)
procedure display_color_table(x,y: integer);
var
   FG,BG:   integer;
   x2,y2:   integer;
begin
   x2 := x+4*16+3;
   y2 := y+1*8+1;

   setattr( O_CB );
   display_border(x,y,x2,y2,single_border);

   window(x+1,y+1,x2-1,y2-1);
   setattr( 0 (* O_CW *) );
   clrscr;
   window(1,1,80,25);

   gotoxy(x+3,y);
   setattr( O_CT );
   disp(TX_COLORS);

   for BG := 0 to 7 do
      for FG := 0 to 15 do
      begin
         setattr(BG*16+FG);
         gotoxy(x+FG*4+2,y+BG*1+1);
         write(bg*16+fg:3);
      end;
end;


(* -------------------------------------------------- *)
procedure screen_settings(var ex: char);
const
   maxent = 8;  {entries on screen}
var
   entry:   integer;
   pentry:  integer;

   procedure proc_entry(func:  edit_functions);
   begin
      case entry of
         1: edit_yesno  (func,   5, 5,  TX_NF,O_NF,ex);
         2: edit_yesno  (func,   5, 6,  TX_NV,O_NV,ex);
         3: edit_word   (func,   5, 7,  TX_CP,O_CP,2,0,20,ex);
         4: edit_word   (func,   5, 8,  TX_CB,O_CB,3,0,255,ex);
         5: edit_word   (func,   5, 9,  TX_CM,O_CM,3,0,255,ex);
         6: edit_word   (func,   5, 10, TX_CT,O_CT,3,0,255,ex);
         7: edit_word   (func,   5, 11, TX_CW,O_CW,3,0,255,ex);
         8: edit_word   (func,   5, 12, TX_CG,O_CG,3,0,255,ex);
      end;

      if ex = F1 then
         case entry of
            1: help(4, 2+5, '-NF');
            2: help(4, 2+6, '-NV');
            3: help(4, 2+7, '-CP');
            4: help(4, 2+8, '-CB');
            5: help(4, 2+9, '-CM');
            6: help(4, 2+10,'-CT');
            7: help(4, 2+11,'-CW');
            8: help(4, 2+12,'-CG');
         end;

      select_next_entry(func,entry,maxent,ex);
   end;
   
label
   redisplay;

begin
   pentry := 1;

redisplay:
   title_screen(1,14,TX_PG5,TX_SUB5);

   for entry := 1 to maxent do
      proc_entry(display);

   display_color_table(6,15);

   ex := DOWN;
   entry := pentry;
   data_changed := false;

   repeat
      proc_entry(edit);
      if data_changed then
      begin
         pentry := entry;
         goto redisplay;
      end;
   until (ex <> UP) and (ex <> DOWN) and (ex <> F1);
end;


(* -------------------------------------------------- *)
procedure edit_config_file;
const
   maxscr = 5;
var
   scrnum:  integer;
   ex:      char;

begin
   scrnum := 1;
   repeat
      case scrnum of
         1: basic_settings(ex);
         2: com_settings(ex);
         3: option_settings_1(ex);
         4: option_settings_2(ex);
         5: screen_settings(ex);
      end;
      
      case ex of
         PGUP:    dec(scrnum);
         PGDN:    inc(scrnum);
      end;

      if scrnum < 1 then
         scrnum := maxscr
      else
      if scrnum > maxscr then
         scrnum := 1;
      
   until ex = ESC;

   setattr(LightGray);
   clear_screen;
end;


(* -------------------------------------------------- *)
function file_exists(name: string): boolean;
var
   DirInfo:     SearchRec;
begin
   FindFirst(name,$21,DirInfo);
   if (DosError <> 0) then
      file_exists := false
   else
      file_exists := true;
end;


(* -------------------------------------------------- *)
procedure load_config_file;
var
   fd:      text;
   line:    string;
   i,j:     integer;
   new_cfg: string;

   procedure locate_config_file;
   begin
      {first check current directory}
      cfg_fname := 'HSLINK.CFG';
      new_cfg := cfg_fname;
      if file_exists(cfg_fname) then
         exit;

      {next check where hslink.exe is located}
      cfg_fname := FSearch( 'HSLINK.EXE', GetEnv('PATH') );
      if cfg_fname <> '' then
      begin
         cfg_fname := copy(cfg_fname,1,length(cfg_fname)-3) + 'CFG';
         new_cfg := cfg_fname;
         if file_exists(cfg_fname) then
            exit;
      end;

      {finally check PATH directories}
      cfg_fname := FSearch( 'HSLINK.CFG', GetEnv('PATH') );
   end;

begin

   if paramcount = 0 then
      locate_config_file
   else
      cfg_fname := paramstr(1);

   if cfg_fname = '' then
   begin
      gotoxy(1,25);
      clreol;
      cfg_fname := new_cfg;
      write(TX_CANTFIND,cfg_fname);

      if (LastMode = MONO) or (LastMode = BW80) then
      begin
         O_CM := 15;
         O_CT := 15;
         O_CW := 112;
      end;
      reset_colors;

      help(8,5,TX_NEWCONFIG);
      exit;
   end;

   assign(fd,cfg_fname);
   {$i-} reset(fd); {$i+}
   if ioresult <> 0 then
   begin
      gotoxy(1,25);
      clreol;
      write(TX_CANTOPEN,cfg_fname);
      help(8,5,TX_OPENERROR);
      exit;
   end;

   while not eof(fd) do
   begin
      fillchar(line,sizeof(line),0);
      readln(fd,line);
      for i := 1 to length(line) do
         line[i] := upcase(line[i]);

      if copy(line,1,1) = '-' then
      delete(line,1,1);

      case line[1] of
         '!':  O_BANG := NOT D_BANG;
         'A':  O_A := NOT D_A;
         'B':  O_B := ATOL(COPY(LINE,2,255));

         'C':  case line[2] of
                  'B':  O_CB := HATOI(COPY(LINE,3,255));
                  'G':  O_CG := HATOI(COPY(LINE,3,255));
                  'M':  O_CM := HATOI(COPY(LINE,3,255));
                  'P':  O_CP := HATOI(COPY(LINE,3,255));
                  'T':  O_CT := HATOI(COPY(LINE,3,255));
                  'W':  O_CW := HATOI(COPY(LINE,3,255));
                  else  O_C := NOT D_C;
               end;

         'D':  O_D := NOT D_D;
         'E':  O_E := ATOL(COPY(LINE,2,255));

         'F':  case line[2] of
                  'C':  O_FC := NOT D_FC;
                  'T':  O_FT := HATOI(COPY(LINE,3,255));
               end;

         'H':  case line[2] of
                  'C':  O_HC := NOT D_HC;
                  'R':  O_HR := NOT D_HR;
                  'S':  O_HS := NOT D_HS;
                  'X':  O_HX := NOT D_HX;
               end;

         'I':  O_I := HATOI(COPY(LINE,2,255));
         'K':  O_K := NOT D_K;

         'L':  case line[2] of
                  'F':  O_LF := COPY(LINE,3,255);
               end;

         'N':  case line[2] of
                  '5':  begin
                           O_N5 := NOT D_N5;
                           O_FT := 0;
                           D_FT := 0;
                        end;
                  'A':  O_NA := NOT D_NA;
                  'B':  O_NB := NOT D_NB;
                  'E':  O_NE := HATOI(COPY(LINE,3,255));
                  'F':  O_NF := NOT D_NF;
                  'K':  O_K := D_K;
                  'M':  O_NM := NOT D_NM;
                  'O':  O_NO := COPY(LINE,3,255);
                  'T':  O_NT := NOT D_NT;
                  'U':  O_NU := NOT D_NU;
                  'V':  O_NV := NOT D_NV;
               end;

         'O':  O_O := NOT D_O;

         'P':  case line[2] of
                  'B':  O_PB := HATOI(COPY(LINE,3,255));
                  'I':  O_PI := HATOI(COPY(LINE,3,255));
                  else  O_P := HATOI(COPY(LINE,2,255));
               end;

         'R':  O_R := NOT D_R;
         'S':  O_S := HATOI(COPY(LINE,2,255));
         'U':  O_U := COPY(LINE,2,255);
         'W':  O_W := HATOI(COPY(LINE,2,255));
      end;

   end;

   close(fd);
end;


(* -------------------------------------------------- *)
procedure save_config_file;
var
   fd:      text;
   count:   integer;
begin
   gotoxy(1,25);
   clreol;
   writeln(copyright);
   write(TX_SAVING+cfg_fname);

   assign(fd,cfg_fname);
   {$i-} rewrite(fd); {$i+}
   if ioresult <> 0 then
   begin
      gotoxy(1,25);
      clreol;
      write(TX_CANTCREATE,cfg_fname);
      help(8,5,TX_CREATEERROR);
      exit;
   end;

   count := 0;

   if O_BANG <> D_BANG then begin inc(count); writeln(fd,'-!'); end;
   if O_A <> D_A   then begin inc(count); writeln(fd,'-A'); end;
   if O_B <> D_B   then begin inc(count); writeln(fd,'-B',O_B); end;
   if O_C <> D_C   then begin inc(count); writeln(fd,'-C'); end;
   if O_CB <> D_CB then begin inc(count); writeln(fd,'-CB',O_CB); end;
   if O_CG <> D_CG then begin inc(count); writeln(fd,'-CG',O_CG); end;
   if O_CM <> D_CM then begin inc(count); writeln(fd,'-CM',O_CM); end;
   if O_CP <> D_CP then begin inc(count); writeln(fd,'-CP',O_CP); end;
   if O_CT <> D_CT then begin inc(count); writeln(fd,'-CT',O_CT); end;
   if O_CW <> D_CW then begin inc(count); writeln(fd,'-CW',O_CW); end;
   if O_D <> D_D   then begin inc(count); writeln(fd,'-D'); end;
   if O_E <> D_E   then begin inc(count); writeln(fd,'-E',O_E); end;
   if O_FC <> D_FC then begin inc(count); writeln(fd,'-FC'); end;
   if O_HC <> D_HC then begin inc(count); writeln(fd,'-HC'); end;
   if O_HR <> D_HR then begin inc(count); writeln(fd,'-HR'); end;
   if O_HS <> D_HS then begin inc(count); writeln(fd,'-HS'); end;
   if O_HX <> D_HX then begin inc(count); writeln(fd,'-HX'); end;
   if O_I <> D_I   then begin inc(count); writeln(fd,'-I',O_I); end;
   if O_K <> D_K   then begin inc(count); writeln(fd,'-K'); end;
   if O_LF <> D_LF then begin inc(count); writeln(fd,'-LF',O_LF); end;
   if O_N5 <> D_N5 then begin inc(count); writeln(fd,'-N5'); end;
   if O_FT <> D_FT then begin inc(count); writeln(fd,'-FT',O_FT); end;
   if O_NA <> D_NA then begin inc(count); writeln(fd,'-NA'); end;
   if O_NB <> D_NB then begin inc(count); writeln(fd,'-NB'); end;
   if O_NC <> D_NC then begin inc(count); writeln(fd,'-NC'); end;
   if O_NE <> D_NE then begin inc(count); writeln(fd,'-NE',O_NE); end;
   if O_NF <> D_NF then begin inc(count); writeln(fd,'-NF'); end;
   if O_NM <> D_NM then begin inc(count); writeln(fd,'-NM'); end;
   if O_NO <> D_NO then begin inc(count); writeln(fd,'-NO',O_NO); end;
   if O_NT <> D_NT then begin inc(count); writeln(fd,'-NT'); end;
   if O_NU <> D_NU then begin inc(count); writeln(fd,'-NU'); end;
   if O_NV <> D_NV then begin inc(count); writeln(fd,'-NV'); end;
   if O_O <> D_O   then begin inc(count); writeln(fd,'-O'); end;
   if O_P <> D_P   then begin inc(count); writeln(fd,'-P',O_P); end;
   if O_PB <> D_PB then begin inc(count); writeln(fd,'-PB$',itoh(O_PB)); end;
   if O_PI <> D_PI then begin inc(count); writeln(fd,'-PI',O_PI); end;
   if O_R <> D_R   then begin inc(count); writeln(fd,'-R'); end;
   if O_S <> D_S   then begin inc(count); writeln(fd,'-S',O_S); end;
   if O_U <> D_U   then begin inc(count); writeln(fd,'-U',O_U); end;
   if O_W <> D_W   then begin inc(count); writeln(fd,'-W',O_W); end;
   close(fd);
   writeln;

   if count = 0 then
   begin
      writeln(TX_DEFAULT);
      writeln(TX_NOCONFIG);
      writeln(TX_EMPTY,cfg_fname,TX_DELETED);
      erase(fd);
   end;
end;


(* -------------------------------------------------- *)
begin
   setattr(White);
   reset_colors;
   writeln(copyright);

   load_config_file;
   edit_config_file;
   save_config_file;
end.

