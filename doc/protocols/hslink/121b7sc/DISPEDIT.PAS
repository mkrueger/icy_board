
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)

(*
 * dispedit - display/edit support functions for interactive
 *            configuration type programs.
 *
 *)

{$i prodef.inc}

unit dispedit;

{$v-}

interface
   uses dos, crt, tools;

   const
      title_attr:    word = (RED*16)      + WHITE;
      border_attr:   word = (BLUE*16)     + LIGHTGRAY;
      prompt_attr:   word = (BLACK*16)    + WHITE;
      input_attr:    word = (GREEN*16)    + WHITE;
      data_attr:     word = (BLACK*16)    + YELLOW;

   type
      charset        = string[128];

      edit_functions = (display, edit, clear);

      border_styles  = (blank_border,          single_border,
                        double_border,         mixed_border,
                        taildouble_border,
                        solid_border,          evensolid_border,
                        thinsolid_border,      lohatch_border,
                        medhatch_border,       hihatch_border);

      display_image_type = array[1..2000] of record
         chr:  char;
         attr: byte;
      end;

      display_image_rec = record
         crt:  display_image_type;
         mode: word;
         attr: byte;
         wmin: word;
         wmax: word;
         x,y:  byte;
      end;

   var
      disp_mem:   ^display_image_type;

      lastData:   ^char;      {pointer to last variable edited}
      lastSize:   integer;    {size of last variable edited}

   const
      allchars:   charset = ' !"#$%&''()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~';
      namechars:  charset = '!#$%&''()+-.0123456789:@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_{}~';

      YES         = 'Y';      NO          = 'N';
      BACKSPACE   = #8;       TAB         = #9;
      ENTERKEY    = #13;      ESC         = #27;
      F1          = #201;     F2          = #202;
      F3          = #203;     F4          = #204;
      F5          = #205;     F6          = #206;
      F7          = #207;     F8          = #208;
      F9          = #209;     F10         = #210;
      HOME        = #213;     UP          = #214;
      PGUP        = #215;     LEFT        = #217;
      RIGHT       = #219;     ENDK        = #221;
      DOWN        = #222;     PGDN        = #223;
      INS         = #224;     DEL         = #225;
      CTRL_F1     = #236;     CTRL_F2     = #237;
      CTRL_F3     = #238;     CTRL_F9     = #244;
      CTRL_F10    = #245;     CTRL_PGUP   = #18;
      CTRL_PGDN   = #4;       CTRL_LEFT   = #1;
      CTRL_RIGHT  = #2;       CTRL_HOME   = #5;
      CTRL_END    = #3;       SHIFT_TAB   = #157;
      ALT_D       = #174;     ALT_I       = #165;

      data_changed: boolean = false;

      py: integer = -1;
      px: integer = -1;

      traceopen: boolean = false;

   var
      tracefd: text;

   procedure setattr(w: word);
   procedure disp(s: string);
   procedure displn(s: string);
   procedure dispc(c: char);
   procedure newline;

   function make_string(ch: char; size: byte): string;

   procedure display_border(topx,topy,
                            botx,boty: integer;
                            style:     border_styles);

   procedure beep;

   function get_key: char;

   procedure edit_string ( func:      edit_functions;
                           x,y:       integer;
                           prompt:    string;
                           var data:  string;
                           width:     integer;
                           var term:  char );

   procedure edit_fname ( func:      edit_functions;
                          x,y:       integer;
                          prompt:    string;
                          var data:  string;
                          width:     integer;
                          isdir:     boolean;
                          var term:  char );

   procedure edit_chars  ( func:      edit_functions;
                           x,y:       integer;
                           prompt:    string;
                           var data;
                           width:     integer;
                           var term:  char );

   procedure edit_longint( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: longint;
                           width:    integer;
                           min,max:  longint;
                           var term: char );

   procedure edit_integer( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: integer;
                           width:    integer;
                           min,max:  integer;
                           var term: char );

   procedure edit_byte   ( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: byte;
                           width:    integer;
                           min,max:  byte;
                           var term: char );

   procedure edit_word   ( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: word;
                           width:    integer;
                           min,max:  word;
                           var term: char );

   procedure edit_hex    ( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: word;
                           width:    integer;
                           var term: char );

   procedure edit_real   ( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: real;
                           width:    integer;
                           deci:     integer;
                           var term: char );

   procedure edit_yesno(   func:      edit_functions;
                           x,y:       integer;
                           prompt:    string;
                           var data:  boolean;
                           var term:  char );

   procedure edit_funkey( func:      edit_functions;
                          x,y:       integer;
                          prompt:    string;
                          key:       char;
                          var term:  char );

   procedure select_next_entry( func:    edit_functions;
                                var en:  integer;
                                maxen:   integer;
                                var key: char);

   procedure clear_screen;

   procedure vscroll_bar(current, min, max: word;
                         x,y1,y2: byte);

   procedure hscroll_bar(current, min, max: word;
                         y,x1,x2: byte);

   procedure opentrace(name: string);
   procedure closetrace;

   procedure input(var line:  string;
                   maxlen:    integer);

   procedure save_display(var disp: display_image_rec);
   procedure restore_display(var disp: display_image_rec);
   procedure shadow_display;


implementation


   (* -------------------------------------------------- *)
   procedure setattr(w: word);
   begin
      textbackground(w shr 4);
      textcolor(w and 15);
   end;


   (* -------------------------------------------------- *)
   procedure disp(s: string);
   begin
      write(s);
      if traceopen then
         write(tracefd,s);
   end;

   procedure newline;
   begin
      disp(^M^J);
   end;

   procedure displn(s: string);
   begin
      disp(s);
      newline;
   end;

   procedure dispc(c: char);
   begin
      disp(c);
   end;


   (* -------------------------------------------------- *)
   function make_string(ch: char; size: byte): string;
   var
      st: string;
   begin
      fillchar(st[1],size,ch);
      st[0] := chr(size);
      make_string := st;
   end;


   (* -------------------------------------------------- *)
   procedure display_border(topx,topy,
                            botx,boty: integer;
                            style:     border_styles);
      (* display a window border.  enter with desired color settingx*)
   var
      left:        string[80];
      right:       string[80];
      top:         string[80];
      bottom:      string[80];
      width:       integer;
      b:           string[8];
      i,j:         integer;

   const
     border_table:  array[blank_border..hihatch_border] of string[8] =
       ('        ',  { blank     }         'ÚÄ¿³³ÀÄÙ',  { single    }
        'ÉÍ»ººÈÍ¼',  { double    }         'ÕÍ¸³³ÔÍ¾',  { mixed     }
        'ÌÍ»ººÈÍ¼',  { taildouble}
        'ÛÛÛÛÛÛÛÛ',  { solid     }         'ÛßÛÛÛÛÜÛ',  { evensolid }
        'ÞßÝÞÝÞÜÝ',  { thinsolid }         '°°°°°°°°',  { lohatch   }
        '±±±±±±±±',  { medhatch  }         '²²²²²²²²'); { hihatch   }

      topleft  = 1;    {border character locations in border strings}
      tophor   = 2;
      topright = 3;
      leftver  = 4;
      rightver = 5;
      botleft  = 6;
      bothor   = 7;
      botright = 8;

      filler = ^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J^@^H^J;

   begin
      b := border_table[style];

      gotoxy(topx,topy);
      write(b[TopLeft]);
      for i := topx+1 to botx-1 do
         write(b[TopHor]);
      write(b[TopRight]);

      gotoxy(topx,boty);
      write(b[BotLeft]);
      for i := topx+1 to botx-1 do
         write(b[BotHor]);
      write(b[BotRight]);

      for i := topy+1 to boty-1 do
      begin
         gotoxy(topx,i);
         write(b[LeftVer]);
         gotoxy(botx,i);
         write(b[RightVer]);
      end;
   end;


   (* -------------------------------------------------- *)
   procedure beep;
   begin
      disp(^G);
   end;


   (* -------------------------------------------------- *)
   function get_key: char;
   var
      c: char;
   begin
      c := readkey;
      if c = #0 then
         c := chr(ord(readkey) + 142);
      get_key := c;
   end;


   (* -------------------------------------------------- *)
   procedure raw_editor( func:       edit_functions;
                         x,y:        integer;
                         prompt:     string;
                         var data:   string;
                         width:      integer;
                         var term:   char;
                         upper:      boolean;
                         legal:      charset );
   var
      col:        integer;
      ch:         char;
      filler:     string;
      fillch:     char;
      insmode:    boolean;

   begin
      if length(data) > width then
         data[0] := chr(width);
      if upper then
         stoupper(data);

      case func of
         display:
            fillch := '_';
         edit:
            fillch := '°';
         clear:
            begin
               fillch := ' ';
               data := '';
            end;
      end;

      filler := make_string( fillch, width - length(data) ) + ' ';

      lowvideo;
      gotoxy( x, y );
      setattr(prompt_attr);
      disp( prompt );

      if func <> edit then
      begin
         setattr(data_attr);
         disp( copy( data, 1, width ) );
         lowvideo;
         disp( filler );
         highvideo;
         exit;
      end;

      setattr(input_attr);
      disp( copy( data, 1, width ) );
      lowvideo;
      disp( filler );

  (* default to overtype mode *)
      setattr(input_attr);
      insmode := false;
      gotoxy(75,1);
      disp('OVR');

   (* record location of last edited data *)
      lastData := @data;
      lastSize := width+1;

   (* general edit string function *)
      inc(x,length(prompt));
      col := 0;
      term := '0';

      repeat
         gotoxy( x + col, y );
         ch := get_key;

         case ch of
            HOME: col := 0;

            ENDK: col := length(data);

            LEFT: if col > 0 then
                     dec(col)
                  else
                     term := UP;

            RIGHT:
                  if col < length(data) then
                     inc(col)
                  else
                     term := DOWN;

            DEL:  if col < length( data ) then
                  begin
                     delete( data, col + 1, 1 );
                     disp( copy( data, col + 1, width )+ fillch );
                     data_changed := true;
                  end;

            INS:  begin
                     insmode := not insmode;
                     gotoxy(75,1);
                     if insmode then
                        disp('INS')
                     else
                        disp('OVR');
                  end;

            BACKSPACE:
                  if col > 0 then
                  begin
                     delete( data, col, 1 );
                     disp( ^h + copy( data, col, width )+ fillch );
                     dec(col);
                     data_changed := true;
                  end
                  else
                     beep;

            F1..F10, ESC,
            ALT_D, ALT_I,
            ENTERKEY, UP, DOWN,
            PGUP, PGDN,
            CTRL_PGUP, CTRL_PGDN,
            CTRL_HOME, CTRL_END:
                  term := ch;

            else  begin
                     if upper then
                        ch := upcase(ch);

                     if pos(ch,legal) > 0 then
                     begin
                        if insmode and
                           (length( data) >= col) and
                           (length( data ) < width) then
                        begin
                           insert( ' ',data, col+1 );
                           disp( copy( data, col+1, width ) );
                           data_changed := true;
                           gotoxy( x + col, y );
                        end;

                        if col < width then
                        begin
                           inc(col);
                           if col > length( data ) then
                              data := data + ch
                           else
                              data[ col ] := ch;

                           disp( ch );
                           data_changed := true;
                        end
                        else
                           beep;
                     end
                     else

                     begin
                      { gotoxy(1,1);
                        write('ch=',ord(ch):3); }
                        beep;
                     end;
                  end;
         end;

      until term <> '0';

      gotoxy( x, y );
      setattr(data_attr);
      disp( data );
      lowvideo;
      disp( make_string( '_', width-length(data) )+' ' );
   end;


   (* -------------------------------------------------- *)
   procedure edit_string( func:      edit_functions;
                          x,y:       integer;
                          prompt:    string;
                          var data:  string;
                          width:     integer;
                          var term:  char );
   begin
      raw_editor( func, x, y, prompt, data, width, term, false, allchars);
   end;


   (* -------------------------------------------------- *)
   procedure edit_fname ( func:      edit_functions;
                          x,y:       integer;
                          prompt:    string;
                          var data:  string;
                          width:     integer;
                          isdir:     boolean;
                          var term:  char );
   begin
      raw_editor( func, x, y, prompt, data, width, term, true, namechars);

      if isdir and (data[length(data)] <> '\') and (length(data) > 1) then
      begin
         inc(data[0]);
         data[length(data)] := '\';
      end;
   end;


   (* -------------------------------------------------- *)
   procedure edit_chars( func:      edit_functions;
                         x,y:       integer;
                         prompt:    string;
                         var data;
                         width:     integer;
                         var term:  char );
   var
      cdata:   array[1..255] of char absolute data;
      sdata:   string;
      i:       integer;

   begin
      for i := 1 to width do
         sdata[i] := cdata[i];
      sdata[0] := chr(width);
      while sdata[length(sdata)] = ' ' do
         dec(sdata[0]);

      raw_editor( func, x, y, prompt, sdata, width, term, false, allchars);

      sdata := ljust(sdata,width);
      for i := 1 to width do
         cdata[i] := sdata[i];
   end;


   (* -------------------------------------------------- *)
   procedure edit_longint( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: longint;
                           width:    integer;
                           min,max:  longint;
                           var term: char );
   var
      temp:       string;
      code:       integer;
      new_data:   longint;
      keys:       string[11];

   begin
      keys := '0123456789';
      if min < 0 then
         keys := keys + '-';

      str(data,temp);      { convert data to string }

      repeat
         raw_editor( func, x, y, prompt, temp, width, term, false, keys);

         if func <> edit then
            exit;

      (* record location of last edited data *)
         lastData := @data;
         lastSize := sizeof(data);

         val( temp, new_data, code );  { convert string to int }

         if (new_data < min) or (new_data > max) then
            code := 1;                 { out of range }

         if code = 0 then
            data := new_data
         else
         begin
            beep;           { code is 0 if data is valid }
            str(data,temp);
            if (term >= F1) and (term <= F10) then
               exit;  { allow invalid data without change on F-keys}
         end;

      until ( code = 0 );
   end;


   (* -------------------------------------------------- *)
   procedure edit_integer( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: integer;
                           width:    integer;
                           min,max:  integer;
                           var term: char );
   var
      int:     longint;
   begin
      int := data;
      edit_longint(func,x,y,prompt,int,width,min,max,term);
      data := int;

      (* record location of last edited data *)
      if func=edit then
      begin
         lastData := @data;
         lastSize := sizeof(data);
      end;
   end;


   (* -------------------------------------------------- *)
   procedure edit_byte   ( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: byte;
                           width:    integer;
                           min,max:  byte;
                           var term: char );
   var
      int:        longint;
   begin
      int := data;
      edit_longint(func,x,y,prompt,int,width,min,max,term);
      data := int;

      (* record location of last edited data *)
      if func=edit then
      begin
         lastData := @data;
         lastSize := sizeof(data);
      end;
   end;


   (* -------------------------------------------------- *)
   procedure edit_word( func:     edit_functions;
                        x,y:      integer;
                        prompt:   string;
                        var data: word;
                        width:    integer;
                        min,max:  word;
                        var term: char );
   var
      int:     longint;
   begin
      int := data;
      edit_longint(func,x,y,prompt,int,width,min,max,term);
      data := int;

      (* record location of last edited data *)
      if func=edit then
      begin
         lastData := @data;
         lastSize := sizeof(data);
      end;
   end;


   (* -------------------------------------------------- *)
   procedure edit_hex ( func:     edit_functions;
                        x,y:      integer;
                        prompt:   string;
                        var data: word;
                        width:    integer;
                        var term: char );
   var
      temp:       string;
      new_data:   word;

   begin
      temp := itoh(data);
      if length(temp) > width then
         temp := copy(temp,length(temp)-width+1,255);

      raw_editor( func, x, y, prompt, temp, width, term, true, '0123456789ABCDEFabcdef');

      if func <> edit then
         exit;

      (* record location of last edited data *)
      lastData := @data;
      lastSize := sizeof(data);

      new_data := htoi(temp);
      data := new_data
   end;


   (* -------------------------------------------------- *)
   procedure edit_real   ( func:     edit_functions;
                           x,y:      integer;
                           prompt:   string;
                           var data: real;
                           width:    integer;
                           deci:     integer;
                           var term: char );
   var
      temp:       string;
      code:       integer;
      new_data:   real;

   begin
      str(data:0:deci,temp);      { convert data from float to string }

      repeat
         raw_editor( func, x, y, prompt, temp, width, term, true, '0123456789.E-');

         if func <> edit then
            exit;

      (* record location of last edited data *)
         lastData := @data;
         lastSize := sizeof(data);

         val( temp, new_data, code );  { convert string to int }

         if code = 0 then
            data := new_data
         else
         begin
            beep;           { code is 0 if data is valid }
            str(data,temp);
            if (term >= F1) and (term <= F10) then
               exit;  { allow invalid data without change on F-keys}
         end;

      until ( code = 0 );
   end;


   (* -------------------------------------------------- *)
   procedure edit_yesno( func:      edit_functions;
                         x,y:       integer;
                         prompt:    string;
                         var data:  boolean;
                         var term:  char );
   var
      yesno:   string;

   begin
      if ord(data)=2 then
         yesno := 'A'
      else
      if data then
         yesno := 'Y'
      else
         yesno := 'N';

      raw_editor( func, x, y, prompt, yesno, 1, term, true, 'YNA');

      if yesno[1] = 'A' then
         byte(data) := 2
      else
         data := yesno[1] = 'Y';

      (* record location of last edited data *)
      if func = edit then
      begin
         lastData := @data;
         lastSize := sizeof(data);
      end;
   end;


   (* -------------------------------------------------- *)
   procedure edit_funkey( func:      edit_functions;
                          x,y:       integer;
                          prompt:    string;
                          key:       char;
                          var term:  char );
   begin
      if func = edit then
      begin
         gotoxy( x, y );
         setattr(title_attr);
         disp( '<'+prompt+'>' );

         term := get_key;
         if term = ENTERKEY then
            term := key;
      end;

      gotoxy( x, y );
      setattr(input_attr);
      disp( ' '+prompt+' ' );
   end;


   (* -------------------------------------------------- *)
   procedure select_next_entry( func:    edit_functions;
                                var en:  integer;
                                maxen:   integer;
                                var key: char);
   begin
      if func = display then
         exit;

      case key of
         TAB, ENTERKEY, DOWN:
            begin
               key := DOWN;
               if en < maxen then
                  inc(en)
               else
                  en := 1;
            end;

         UP:   if en > 1 then
                  dec(en)
               else
                  en := maxen;

         CTRL_HOME:
               begin
                  en := 1;
                  key := DOWN;
               end;

         CTRL_END:
               begin
                  en := maxen;
                  key := UP;
               end;
      end;
   end;

(* -------------------------------------------------- *)
   procedure clear_screen;
   begin
      clrscr;
      py := -1;
      px := -1;
   end;

(* -------------------------------------------------- *)
   procedure vscroll_bar(current, min, max: word;
                         x,y1,y2: byte);
   var
      y: integer;
      i: integer;
   begin
      y := ((longint(current-min) * longint(y2-y1)) div longint(max-min)) + y1;
      if y = py then
         exit;

      py := y;
      for i := y1 to y2 do
      begin
         gotoxy(x,i);
         if i = y then
            disp('Û')
         else
            disp('°');
      end;
   end;

(* -------------------------------------------------- *)
   procedure hscroll_bar(current, min, max: word;
                         y,x1,x2: byte);
   var
      x: integer;
      i: integer;
   begin
      x := ((longint(current-min) * longint(x2-x1)) div longint(max-min)) + x1;
      if x = px then
         exit;

      px := x;
      for i := x1 to x2 do
      begin
         gotoxy(i,y);
         if i = x then
            disp('Û')
         else
            disp('°');
      end;
   end;

   (* ------------------------------------------------------------ *)
   procedure input(var line:  string;
                   maxlen:    integer);
   var
      c:     char;

   begin
      line := '';

      repeat
         c := get_key;

         case c of
            ' '..#126:
               if length(line) < maxlen then
               begin
                  inc(line[0]);
                  line[length(line)] := c;
                  disp(c);
               end
               else
                  beep;

            ^H,#127:
               if length(line) > 0 then
               begin
                  dec(line[0]);
                  disp(^H' '^H);
               end;

            ^M:   ;

            ^C:   begin
                      displn('^C');
                      halt(99);
                  end;
         end;

      until (c = ^M);
   end;


(* -------------------------------------------------- *)
   procedure opentrace(name: string);
   begin
      assign(tracefd,name);
      rewrite(tracefd);
      traceopen := true;
   end;

   procedure closetrace;
   begin
      close(tracefd);
      traceopen := false;
   end;



(* -------------------------------------------------- *)
   procedure save_display(var disp: display_image_rec);
   begin
      disp.crt := disp_mem^;
      disp.mode := lastmode;
      disp.attr := textattr;
      disp.wmin := windmin;
      disp.wmax := windmax;
      disp.x := wherex;
      disp.y := wherey;
   end;

   procedure restore_display(var disp: display_image_rec);
   begin
      disp_mem^ := disp.crt;
      lastmode := disp.mode;
      textattr := disp.attr;
      windmin := disp.wmin;
      windmax := disp.wmax;
      gotoxy(disp.x,disp.y);
   end;


   procedure shadow_display;
   var
      i: integer;
   begin
      for i := 1 to 2000 do
         with disp_mem^[i] do
            attr := attr and 7;
   end;


(* -------------------------------------------------- *)
var
   Vmode: byte absolute $0040:$0049;   {Current video mode}
begin
   if (Vmode = 1{MDA}) or (Vmode = 7{VgaMono}) then
      disp_mem := ptr($B000,0)
   else
      disp_mem := ptr($B800,0);

   assignCrt(output);
   rewrite(output);
   directvideo := pos('/BIO',GetEnv('PCB')) = 0;
end.


