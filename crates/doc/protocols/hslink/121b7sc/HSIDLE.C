
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * hsidle.c - HS/Link low level idle time handlers
 *
 * This module provides lowest level handlers for idle periods during
 * file transfer.
 *
 */

#pragma inline

#include <dos.h>
#include <stdio.h>
#include <stdlib.h>
#include <io.h>
#include <conio.h>
#include <bios.h>

#include <hdk.h>
#include "hsidle.h"

/* ------------------------------------------------------------ */

/* give up idle time */
/* return 1 if immediate keyboard check is needed */

int pascal idle(void)
{
        switch (WS.Option.IdleMethod)
        {

        case 0: /* -i0 - default - do not give up idle time */
                break;

        case 1: /* -i1 - poll keyboard during idle time */
                if (bioskey(1))
                        return 1;
                break;

        case 2: /* -i2 - give up timeslice under desqview */
                idleDV();
                break;

        case 3: /* -i3 - give up timeslice under doubledos */
                idleDDOS();
                break;

        case 4: /* -i4 - give up timeslice under windows/os2/vcpi/dos5 */
                idleWINDOWS();
                break;
        }

        return 0;
}


/* ------------------------------------------------------------ */

/* give up time under desqview */

void pascal idleDV(void)
{
        _AX = 0x101a;           /* OSTACK */
        asm INT 15H;

        _AX = 0x1000;           /* DV_PAUSE function call */
        asm INT 15H;

        _AX = 0x1025;           /* USTACK */
        asm INT 15H;
}


/* ------------------------------------------------------------ */

/* give up time under doubledos */

void pascal idleDDOS(void)
{
        asm PUSH BP;

        _AX = 0xee01;           /* doubledos give back 1 timeslice */
        asm INT 21H;

        asm POP BP;
}


/* ------------------------------------------------------------ */

/* give up time under windows/dos/vcpi/os2 */

void pascal idleWINDOWS(void)
{
        asm PUSH BP;

        _AX = 0x1680;           /* release current virtual machine timeslice */
        asm INT 2FH;

        asm POP BP;
}

