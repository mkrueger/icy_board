#undef DEBUG
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * fossil.c - HS/Link com port library (fossil version)
 *
 * This module provides low-level, COM port interfaces using the FOSSIL API.
 *
 */

#include <dos.h>
#include <stdio.h>
#include <stdlib.h>
#include <io.h>
#include <conio.h>
#include <bios.h>
#include <fcntl.h>

#include "\hdk\hspriv.h"
#include <hdk.h>
#include "hsl.h"
#include "hsidle.h"
#include "hstext.h"

#define extern

#include "fossil.h"
struct fInfo fInfo;
uchar fcDeInit;  /* current deinit function to use */

#undef extern

/* -------------------------------------------------------------- */

/* prototypes for private local procedures */

static void pascal setSpeed(long speed);

/* private data */

/* ------------------------------------------------------------ */

 /* get fInfo information */

void pascal getFinfo(void)
{
        _CX = sizeof(fInfo);
#ifdef __TURBOC__
        _ES = _DS;
        _DI = &fInfo;
#else
        _ES = FP_SEG(fInfo);
        _DI = FP_OFF(fInfo);
#endif
        FossilAPI(fGetInfo);
}

/* ------------------------------------------------------------ */

/* cancel any pending XOFF condition */

void pascal ComCancelXoff(void)
{
}


/* ------------------------------------------------------------ */

 /* wait for all pending transmit data to be sent */

void pascal ComFlush(int cancel)
{
        while (ComWritePending() && !ComCarrierLost())
        {
                ComIdle(110+cancel);
        }
}


/* ------------------------------------------------------------ */

/* lower RTS to inhibit modem sending more data to us */

void pascal lowerRTS(void)
{
        if (!RtsActive)
                return;
        RtsActive = 0;

        /* this is a function specific to X00 but should be safe */
        _AL = 0;
        FossilAPI(fxControl);

        _AL = 1;
        _BL = _BL & ~fxRTS;             /* clear RTS bit */
        FossilAPI(fxControl);
}

/* raise RTS to modem to continue sending */

void pascal raiseRTS(void)
{
        if (RtsActive)
                return;
        RtsActive = 1;

        /* this is a function specific to X00 but should be safe */
        _AL = 0;
        FossilAPI(fxControl);

        _AL = 1;
        _BL = _BL | fxRTS;              /* enable RTS bit */
        FossilAPI(fxControl);
}


/* ------------------------------------------------------------ */

 /* enter slow-handshake region */

void pascal ComIoStart(int where)
{
        clock_t hold_timeout;
        clock_t fail_timeout;
        unsigned pcount;

        /* properly handle nested IO regions */
        ++WS.IoLevel;
        if (WS.IoLevel > 1)
                return;

        if (WS.Option.SlowHandshake)
        {
                if (WS.Option.RtsHandshake)
                        lowerRTS();

                pcount = ComReadPending();
                hold_timeout = SET_TIMER(SLOW_TIMEOUT);
                fail_timeout = SET_TIMER(SLOW_FAILURE);

                while (!TIMER_UP(hold_timeout))
                {
                        _AX = ComReadPending();
                        if (pcount != _AX)
                        {
                                pcount = _AX;
                                hold_timeout = SET_TIMER(SLOW_TIMEOUT);
                        }
                        ComIdle(where+2000);

                        if (TIMER_UP(fail_timeout))
                        {
                                display_warning(TX_SLOWOFF);
                                ComIoEnd(where+1000);
                                WS.Option.SlowHandshake = 0;
                                break;
                        }
                }
        }
}


/* ------------------------------------------------------------ */

/* exit slow-handshake region */

void pascal ComIoEnd(int where)
{
        if (WS.IoLevel)
                --WS.IoLevel;
        if (WS.IoLevel > 0)
                return;

        if (WS.Option.SlowHandshake)
        {
                if (WS.Option.RtsHandshake)
                        raiseRTS();
        }
}

/* ------------------------------------------------------------ */

int pascal ComGetCts(void)
{
        if (!WS.Option.CtsHandshake)
                return 1;

        /* this may be specific to X00 */
        FossilAPI(fStatus);
        return (_AL & fsAlCTS) != 0;
}

/* ------------------------------------------------------------ */

void pascal ComSetHandshake()
{
        static uchar pAL = -1;

        _AL = 0;
        if (WS.Option.CtsHandshake)
                _AL |= fFlowCTS;

        /* enable xon/xoff hanshake ONLY after final ready handshake */
        if (WS.Option.XonHandshake && PRIVATE.remote_ready.final_ready)
                _AL |= fFlowXON;

        if (_AL != pAL)
        {
                pAL = _AL;
                FossilAPI(fSetFlow);    /* set specified flow ctl modes */

                _AL = 0;
                FossilAPI(fCtlCK);      /* disable ^C/^K checking */
        }
}

/* ------------------------------------------------------------ */

 /*
  * initialize communication handlers for operation with the specified com
  * port number and speed.  returns the actual speed of the com port.
  * must be called before any other services here.
  *     chan=1 for com1,
  *     chan=2 for com2
  * if base or irq initialized to non-0 values, they will override the normal
  * value for the specified com channel.
  *
  * before calling this procedure, you must initialize the following
  * variables:
  *     WS.Option.ComPort
  *     WS.Option.ComSpeed (0=default)
  *     WS.Option.ComBase  (0=standard)
  *     WS.Option.ComIrq   (0=standard)
  *     WS.Option.CtsHandshake
  *     WS.Option.RtsHandshake
  *     WS.Option.SlowHandshake
  *     WS.Option.XonHandshake
  */

void pascal ComOpen(void)
{
        WS.cancel_link = 0;

        /* first try new init/deinit */
        fcDeInit = fnDeInit;
        _BX = 0;
        FossilAPI(fnInit);

        /* if new init fails, switch to old one */
        if (_AX != fiFossilOk)
        {
                fcDeInit = fDeInit;
                _BX = 0;
                FossilAPI(fInit);

                /* if old init fails we don't have a FOSSIL loaded */
                if (_AX != fiFossilOk)
                {
                        disp_error(TX_NOFOSSIL, WS.Option.ComPort);
                        exit(1);
                }
        }

        ComSetHandshake();

        if (WS.Option.ComSpeed)
                setSpeed(WS.Option.ComSpeed);

        FossilAPI(fPurgeOut);
        FossilAPI(fPurgeIn);
}


/* ------------------------------------------------------------ */

void pascal setSpeed( long speed )
{
        ComFlush(0);

        switch (speed)
        {
                case 300:       _AL = fBaud300|fBaudOr;         break;
                case 600:       _AL = fBaud600|fBaudOr;         break;
                case 1200:      _AL = fBaud1200|fBaudOr;        break;
                case 2400:      _AL = fBaud2400|fBaudOr;        break;
                case 4800:      _AL = fBaud4800|fBaudOr;        break;
                case 9600:      _AL = fBaud9600|fBaudOr;        break;
                case 19200:     _AL = fBaud19200|fBaudOr;       break;
                case 38400:     _AL = fBaud38400|fBaudOr;       break;

                default:
                        disp_error(TX_BADSPEED,speed);
                        return;
        }

        FossilAPI(fSetSpeed);
}


/* ------------------------------------------------------------ */

long pascal ComGetSpeed(void)
{
        getFinfo();

        switch (fInfo.baud & fBaudMask)
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


/* ------------------------------------------------------------ */

 /*
  * remove interrupt handlers for the com port must be called before exit to
  * system 
  */

void pascal ComClose(void)
{
        FossilAPI(fcDeInit);
}


/* -------------------------------------------------------------- */

/* check carrier loss and return non-0 if carrier lost */

int pascal ComCarrierLost(void)
{
        static uchar lost = 0;

        /* only say 'carrier lost' once! */
        if (lost)
                return 1;

        if (WS.Option.RequireCarrier==0)
                return 0;

        if (ComReadPending())
                return 0;

        FossilAPI(fStatus);
        if ((_AL & fsAlDCD) == 0)
                lost = 1;

        if (lost)
        {
                ERECV(TX_NOCD);
                set_cancel_link(CANCEL_CARRIER_LOST);
                return 1;
        }

        return 0;
}

/* ------------------------------------------------------------ */

 /* see if any receive data is ready on the active com port */
 /* returns actual number of characters waiting */

int pascal ComReadPending(void)
{
        getFinfo();
        return fInfo.ibufr-fInfo.ifree;
}


/* ------------------------------------------------------------ */

void pascal ComReportErrors(void)
{
        FossilAPI(fStatus);
        if (_AH & 2)
        {
                strcpy(WS.Comstat.ErrorMessage,TX_COMOVERRUN);
                report_rx_error(WS.Comstat.ErrorMessage);
                if (WS.receive_errors > 1)
                        --WS.receive_errors;
        }
}


/* ------------------------------------------------------------ */

/* report status of queues and flow control */

void pascal ComReportStatus(int where)
{
        char temp[80];

        sprintf(temp,TX_FOSSILSTATUS,
                ComWritePending(),
                ComReadPending(),
                PRIVATE.rxbuf.nextin,
                where,
                WS.cancel_link);

        cprintf(temp);
        clreol();

        if (where == 1101)
                log_error("%s\r\n",temp);
}


/* ------------------------------------------------------------ */

/* wait for and return 1 character from the active com port */

int pascal ComReadChar(void)
{
        uchar c;

        /* this may be specific to X00 - if x00 is not present or no chars
         * are waiting, we fall through to slower and more compatible code */

        FossilAPI(fReceiveChN);
        c = _AL;
        if (_AH != 0)
        {
                while (!ComReadPending())
                {
                        ComIdle(120);
                        if (ComCarrierLost())
                                return END_PACKET_CHR;
                }

                FossilAPI(fReceiveCh);
                c = _AL;
        }

#ifdef DEBUG
        if (WS.Option.Debug>2)
        {
                printf("(%02x)%04d ",c,trace_next);
                if (c == END_PACKET_CHR)
                        printf("\n\n");
        }
#endif

        return c;
}

/* ------------------------------------------------------------ */

/* reads multiple characters into a buffer, returning number of
   characters actually read, up to bufsiz*/

unsigned pascal ComReadStr(uchar *dest, unsigned bufsiz)
{
        _ES = _DS;
        _DI = FP_OFF(dest);
        _CX = bufsiz;
        FossilAPI(fReceiveBlk);
        return _AX;
}


/* ------------------------------------------------------------ */

 /*
  * que a character to be transmitted over the com port
  */

void pascal ComWriteChar(uchar c)
{

#ifdef DEBUG
        if (WS.Option.Debug>2)
        {
                printf("{%02x}%04d ",c,trace_next);
                if (c == END_PACKET_CHR)
                        printf("\n\n");
        }
#endif

        /* wait here if output buffer is full */
        for (;;)
        {
                _AL = c;
                FossilAPI(fSendChN);
                if (_AX == 1)           /* char sent ok */
                        return;

                ComIdle(140);           /* waiting for que space */
                service_receive();      /* stub? safe? */
                if (ComCarrierLost())   /* abort if needed */
                        return;
        }
}


/* ------------------------------------------------------------ */

 /*
  * transmits a string to the specified com port
  */

void pascal ComWriteStr(uchar *src, unsigned count)
{
        int n;
        for (;;)
        {
                _ES = _DS;
                _DI = FP_OFF(src);
                _CX = count;
                FossilAPI(fSendBlk);
                if (_AX == count)
                        break;

                /* advance in buffer and retry write if fossil did
                   not accept the entire buffer on previous call */
                n = _AX;
                count -= n;
                src += n;
        }
}


/* ------------------------------------------------------------ */

 /* returns the number of characters queued to be transmitted */

int pascal ComWritePending(void)
{
        static int fudge=1;
        int pending;
        getFinfo();

        pending = fInfo.obufr-fInfo.ofree-fudge;
        if (pending == -1)
        {                       /* this hack compensates for a difference */
                fudge = 0;      /* between X00 and Opus!Comm */
                pending = 0;
        }
        return pending;
}

/* ------------------------------------------------------------ */

/* returns the highest possible ComWritePending value before a call to
   ComWrite will block on a transmit queue full condition */

int pascal ComWriteMax(void)
{
        getFinfo();
        return fInfo.obufr-100;
}

/* -------------------------------------------------------------- */

/* discard any pending output */

void pascal discard_TxQue(void)
{
        FossilAPI(fPurgeOut);
        ComWriteChar(END_PACKET_CHR);
        ComWriteChar(END_PACKET_CHR);
}

/* -------------------------------------------------------------- */

/* Comm driver is idle - service keyboard, check for user abort */

void pascal ComIdle(int where)
{
        static clock_t poll_timeout = 0;
        static int pc;
        static int local_can_count;
        int c;

        if (WS.IoLevel == 0)
        {
                if (WS.Option.RtsHandshake)
                        raiseRTS();
        }

        /* dispose of idle time according to current idle method */
        if (idle())
                poll_timeout = 0;

        /* check keyboard only twice per second to reduce DOS overhead */
        /* DOS seems to disable interrupts during 'kbhit()', which causes
           data loss at high baud rates. */

        if (!TIMER_UP(poll_timeout))
                return;

        poll_timeout = SET_TIMER(KEYBOARD_POLL_TIME);

        /* disable status in Option.Debug modes */
        if (WS.Option.Debug == 1)
        {
                select_version();
                if (WS.Option.Debug>2)
                    newline();
                else
                    cprintf("\r");
                ComReportStatus(where);
        }

        if (WS.cancel_link == CANCEL_REMOTE)
                discard_TxQue();

        while (bioskey(1))
        {
                c = bioskey(0);
                switch (c & 0xff)
                {

                case CAN_CHR:
                        if (pc == c)
                        {
                                if (++local_can_count >= CANCEL_COUNT)
                                {
                                        int i;
                                        ERECV(TX_CTRLX);
                                        set_cancel_link(CANCEL_KEYBOARD);
                                        PRIVATE.can_count = local_can_count;
                                        discard_TxQue();
                                        ComCancelXoff();
                                        for (i=0; i<=CANCEL_COUNT+2; i++)
                                                ComWriteChar(CAN_CHR);
                                }
                        }
                        else
                                local_can_count = 1;

                        if (WS.Option.TermMode)
                                ComWriteChar(c);
                        break;

                        /* control-d = debug modes */
                case 4:
                        WS.Option.Debug = (++WS.Option.Debug) & 3;
                        return;

                        /* trap all alt- and function keys */
                case 0:
                        break;

                        /* all other keyboard input initiates chat mode */
               default:
                        if (WS.Option.TermMode)
                                ComWriteChar(c);
                        else
                                display_chatout(c);
                }

                pc = c;
        }
}

