
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


#include <dos.h>
#include <stdio.h>
#include <stdlib.h>
#include <io.h>
#include <conio.h>
#include <bios.h>

#include "fossil.h"
struct fInfo fInfo;

struct {
        int ComPort;
} Option = {1};

void open_com(void)
{
        _BX = 0;
        FossilAPI(fInit);
        if (_AX != 0x1954)
        {
                cprintf("FOSSIL driver not present! COM channel: %d\r\n", WS.Option.ComPort);
                exit(1);
        }

        cprintf("<fossil open>\r\n");
}

void close_com(void)
{
        FossilAPI(fDeInit);
        cprintf("<fossil closed>\r\n");
}

void download(void)
{
        close_com();
        system("exe\\fhslink -u\\tmp ");
        open_com();
}

void upload(void)
{
        close_com();
        system("exe\\fhslink -u\\tmp \\ul\\*.* ");
        open_com();
}

main()
{
        int c;
        int quit;

        directvideo = 1;

        cprintf("\r\nFECHO - FOSSIL ECHO Utility; 1992 Samuel H. Smith"
                "\r\nThis program echos all COM1 input back to COM1.  Useful in testing HS/Link."
                "\r\nKeys: ALT-X:Exit  "
                "\r\n");

        open_com();
        _AL = fBaud9600|fBaudOr;
        FossilAPI(fSetSpeed);

        c = 0;
        quit = 0;
        for (;;)
        {
                if (bioskey(1))
                {
                        c = bioskey(0);
                        switch (c)
                        {
                        case 0x2d00:
                                quit = 1;
                                break;

                        case 0x5100:
                                download();
                                break;

                        case 0x4900:
                                upload();
                                break;

                        default:
                                _AL = c;
                                if (c & 0xff)
                                        FossilAPI(fSendChN)
                                else
                                        cprintf("[%04x]",c);
                        }
                }

                if (quit)
                        break;

                FossilAPI(fStatus);
                if (_AH & 1)
                {
                        FossilAPI(fReceiveCh);
                        c = _AL;
                        cprintf(" %02x ",c);
                        _AL = c;
                        FossilAPI(fSendChN)
                }
        }

        close_com();
        return 0;
}
