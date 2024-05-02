.--------------------------------------------------------------------------.
 | DOOR32 Revision 1 Specifications                 Updated: Feb 23rd, 2001 |
 `--------------------------------------------------------------------------'

  What is Door32?
  ---------------

  Door32 is a standard text-based drop file designed to take advantage of
  32-bit operating systems.  Supporting handle inheritance, it will allow
  for doors to work under multiple platforms and over both serial and
  socket (telnet) connections.

  Programming Door32 Doors
  ------------------------
  There are several freeware door libraries which support Door32 although
  most if not all of them are for Pascal at this time.  These libraries are
  able to compile doors for DOS, Windows, OS/2, and Linux.

  I personally suggest the D32 library which is freeware and comes with
  source code.  It can compile doors for DOS, Windows, OS/2 and Linux using
  various Pascal compilers.  There are other great door libraries as well,
  so give them all a try!

  Testing Door32 Support in BBS Software
  --------------------------------------
  Included in the Door32 specification archive is a Win32 executable by the
  name of SAMPLE.EXE.  This is a very simple Door32 compatible door compiled
  with the Pascal library mentioned above.  BBS authors may find this
  program useful for testing their Door32 support.

  Notes about Door32 Programming
  ------------------------------
  At this point, there seems to be two things which need to be kept
  in mind when doing Door32 programming:

     1. Under Linux or any operating system with case sensitive file names,
        the Door32 drop file (DOOR32.SYS) will be created with all lower
        cased lettering. IE:  door32.sys

     2. There appears to be a bug in the Win32 programming API (ie, a bug in
        Windows) which prevents Door32 programs from running from a batch
        file.  This means that the doors have to be executed directly from
        the software.  Instead of running MYDOOR.BAT which eventually
        executes MYDOOR.EXE, you must run MYDOOR.EXE directly.  This problem
        has said to have been fixed in Windows NT, but still exists under
        the Windows 9x platforms.

        What does this mean to BBS authors?  Not much.

        What does this mean to DOOR authors?  Maybe some changes...

        Because Door32 doors must be executed directly, there is a good
        chance that the current directory will NOT be the directory that
        your door executable is stored.  When porting an old DOS door,
        the door might assume that the system is in the same directory
        as the .EXE when loading data files.  Because of this, it is
        recommended that you have your Door switch to the directory where
        the door .EXE is located, or read that directory and use it when
        accessing door related files.  In Pascal, this is a sample of
        getting the directory where the door .EXE is found:

        Function ProgPath : String;
        Var
          Dir  : DirStr;
          Name : NameStr;
          Ext  : ExtStr;
        Begin
          FSplit (ParamStr(0), Dir, Name, Ext);
          ProgPath := Dir + '\';
        End;

        The above example will return the path where your door is located.
        Some door libraries might already switch to the directory of the
        executable, so you may not have to worry about it.

  The DOOR32.SYS Drop file
  ------------------------

  Below is the final Revision 1 version of DOOR32.SYS.  It is a straight
  DOS-style text file in all lower cased letters under operating systems
  with case sensitive file systems:

[cut here]-------------------------------------------------------------------
                                     
0                            Line 1 : Comm type (0=local, 1=serial, 2=telnet)
0                            Line 2 : Comm or socket handle
38400                        Line 3 : Baud rate
Mystic 1.07                  Line 4 : BBSID (software name and version)
1                            Line 5 : User record position (1-based)
James Coyle                  Line 6 : User's real name
g00r00                       Line 7 : User's handle/alias
255                          Line 8 : User's security level
58                           Line 9 : User's time left (in minutes)
1                            Line 10: Emulation *See Below
1                            Line 11: Current node number

[cut here]-------------------------------------------------------------------

 * The following are values we've predefined for the emulation:

 0 = Ascii
 1 = Ansi
 2 = Avatar
 3 = RIP
 4 = Max Graphics

 Avatar, RIP, and Max Graphics all have ANSI fallback support, so most door
 libraries that don't support those emulations can just use ANSI if emulation
 2, 3, or 4 is encountered.

 Conclusion
 ----------

 Thats it for the Revision 1 specifications of DOOR32.SYS.  You can find the
 latest information at the official Door32 web page:

    http://www.mysticbbs.com/door32

 You can e-mail mysticbbs@geocities.com for any questions or suggestions
 relating to Door32 or post a message in the FidoNet DOORGAMES echo
                                     
