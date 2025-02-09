
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
} Option = {2};

long getspeed()
{
        int code;

        _CX = sizeof(fInfo);
        _ES = _DS;
        _DI = &fInfo;
        FossilAPI(fGetInfo);

        code = fInfo.baud & fBaudMask;
        switch (code)
        {
                case fBaud300:          return 300;
                case fBaud600:          return 600;
                case fBaud1200:         return 1200;
                case fBaud2400:         return 2400;
                case fBaud4800:         return 4800;
                case fBaud9600:         return 9600;
                case fBaud19200:        return 19200;
                case fBaud38400:        return 38400;
                default:                return 0;
        }
}

void open_com(void)
{
        _BX = 0;
        FossilAPI(fInit);
        if (_AX != 0x1954)
        {
                cprintf("FOSSIL driver not present! COM channel: %d\r\n", WS.Option.ComPort);
                exit(1);
        }

        cprintf("<fossil open, speed=%ld>\r\n",getspeed());
}

void close_com(void)
{
        FossilAPI(fDeInit);
        cprintf("<fossil closed>\r\n");
}

void download(void)
{
        close_com();
        system("exe\\fhslink -p2 -u\\tmp >out2");
        open_com();
}

void upload(void)
{
        close_com();
        system("exe\\fhslink -p2 -u\\tmp \\ul\\*.* >out2");
        open_com();
}

main()
{
        int c;
        int quit;

        directvideo = 1;

        cprintf("\r\nFCOMM - IttyBitty FOSSIL Comm Program; 1992 Samuel H. Smith"
                "\r\nKeys: ALT-X:Exit  PGUP:Upload  PGDN:Download"
                "\r\n"
                "\r\n");

        open_com();
        _AL = fBaud2400|fBaudOr;
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
                        putch(c);
                }
        }

        close_com();
        return 0;
}
