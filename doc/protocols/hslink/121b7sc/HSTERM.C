
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * HS/Term - Simple Terminal Emulator based on HS/Link
 *           communication drivers.
 *
 */

#define WHOAMI          "HS/Term"
#define VERSION         "Version 1.1à (01/30/92)"
#define COPYRIGHT       "Copyright (C) 1992 Samuel H. Smith"


#include <stdio.h>
#include <stdlib.h>
#include <io.h>
#include <string.h>
#include <conio.h>
#include <ctype.h>
#include <dos.h>

#include <hdk.h>

#include "quelib.h"
#include "hscomm.h"


/* -------------------------------------------------------------- */
/* display program pusage instructions and terminate execution */

void pusage(void)
{
        cprintf("Usage:  hsterm [options]\r\n"
                "\r\nOptions:\r\n"
                "\r\n   -Bbaud      Open COM port at 300..115200 (default=current port speed)"
                "\r\n   -HS         Handshake Slow (lower RTS during disk I/O)"
                "\r\n   -HC         Disable CTS handshake"
                "\r\n   -Pport      Use COM port 1..8 (default=1)"
                "\r\n");
	exit(1);
}


/* -------------------------------------------------------------- */
/* initialize system, process command line options */
void process_command_options(int argc,
                             char *argv[])
{
	int i;

	/* initialize defaults */
	debug = 0;
        require_carrier = 1;
        slow_handshake = 0;
        CTS_handshake = 1;
        comport = 1;
        comspeed = 0;
        full_display = 0;
        WS.send_expected = 0;
        WS.receive_expected = 0;

	/* process each command option */
        for (i = 1; i < argc; i++)
        {
		if (argv[i][0] == '-')
                        switch (tolower(argv[i][1]))
                        {
                        case 'd':
                                debug++;
				break;

                        case 'p':
                                comport = atoi(argv[i]+2);
				break;

                        case 'b':
                                comspeed = atol(argv[i]+2);
				break;

                        case 'h':
                                switch (tolower(argv[i][2]))
                                {
                                case 's':
                                        slow_handshake=1;
                                        break;
                                case 'c':
                                        CTS_handshake=0;
                                        break;
                                default:
                                        cprintf("Unknown handshake option: %s\r\n",argv[i]);
                                }
                                break;
			}
        }
}


/* -------------------------------------------------------------- */
/* process user break */

int control_c()
{
        cprintf("Control-Break!\r\n");
	uninit_com();
        return 0;       /* ABORT PROGRAM */
}


/* -------------------------------------------------------------- */

void call_hslink(int argc,
                 char *argv[])
{
        char cmdline[128];
        int i;

        strcpy(cmdline,"hslink ");
        for (i = 1; i < argc; i++)
        {
                strcat(cmdline," ");
                strcat(cmdline,argv[i]);
        }

        system(cmdline);
}

/* -------------------------------------------------------------- */

int main(int argc,
         char *argv[])
{
        int pc,c;
        int carrier = 999;
        int cts = 999;
        int skip=0;

        cprintf("\r\n%s, %s  %s\r\n", WHOAMI, VERSION, COPYRIGHT);

        process_command_options(argc, argv);

        comspeed = init_com(comport,comspeed);
        ctrlbrk(control_c);

        cprintf("\r\nRunning on COM%d: %ld bps\r\n", comport,comspeed);
        pc = 0;
        c = 0;
        for (;;)
        {
                while (receive_ready())
                {
                        pc = c;
                        c = receive_char();

                        if ((c == '\r') ||
                            (c == '\n') ||
                            (c == '\b') ||
                            (c == CAN_CHR) ||
                            (c == 7))
                                putch(c);
                        else
                        if (c < ' ')
                                cprintf("^%c",c+'@');
                        else
                                putch(c);
 /***************/
                        if (c == '\\')
                               transmit_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [1]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [2]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [3]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [4]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [5]\r\n");
/*******************/

                        /* recognize startup sequence from an HS/Link session */
                        if ((c == 'R') && (pc == 2))
                                call_hslink(argc,argv);
		}

                if ((skip++ > 1000) && kbhit())
                {
                        skip=0;
			c = getch();
			if (c == 26)
				break;

                        if (c == 1)
                        {
                                /* dumptrace(); */
			}
                        else
                       if (c == 2)
                       {
                                transmit_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [1]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [2]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [3]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [4]\r\n"
                                             "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 [5]\r\n");
                        }
                        else
                                transmit_char(c);
                }

                if (carrier_detect() != carrier)
                {
                        carrier = carrier_detect();
                        if (carrier)
                                cprintf("<carrier detected>\r\n");
                        else
                                cprintf("<carrier lost>\r\n");
		}

 /*****
               if (CTS_ACTIVE != cts)
               {
                        cts = CTS_ACTIVE;
                        if (!cts)
                                cprintf("<CTS flow control>\r\n");
                        else
                                cprintf("<CTS released>\r\n");
		}
******/

        }

        cprintf("<exit>\r\n");
	uninit_com();

        /* dumptrace(); */
	return 0;
}
