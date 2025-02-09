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
 * hscomm.c - HS/Link com port library
 *
 * This module provides low-level, interrupt driven, full duplex
 * COM port interfaces for the IBM PC and compatible computers,
 * including the use of the buffered 16550 uart chip.  Interrupt
 * service is provided by HSINTR.C
 *
 * This module also implements full xon/xoff handshaking (in both
 * directions) as well as CTS and RTS handshake.
 *
 */

#include <dos.h>
#include <stdio.h>
#include <stdlib.h>
#include <io.h>
#include <conio.h>
#include <bios.h>
#include <fcntl.h>

#ifdef __TURBOC__
#include <mem.h>
#endif

#include "\hdk\hspriv.h"
#include <hdk.h>
#include "hsl.h"
#include "hsidle.h"
#include "hstext.h"

#define extern

#include "uart.h"
#include "hsintr.h"

#undef extern

/* -------------------------------------------------------------- */

/* prototypes for private local procedures */

static void pascal setSpeed(long speed);

/* private data */

static unsigned ptxq_count;     /* TXQUE.qcount during last kbd poll */

#ifndef __BORLANDC__
#define _dos_getvect getvect
#define _dos_setvect setvect
#endif


/* ------------------------------------------------------------ */

/* input from i/o port with delay */
static uchar pascal dinport(int port)
{
        register int val;
        LL_iodelay;
        _DX = port;
        __emit__((char)0xEC);
        LL_iodelay;
        val = _AL;
#ifdef xDEBUG
        disp_error("dinport: port=%03x data=%02x\r\n",port,val);
#endif
        return val;
}

/* output to i/o port with delay */
static void pascal doutport(int port,int val)
{
#ifdef xDEBUG
        disp_error("doutport: port=%03x data=%02x\r\n",port,val);
#endif
        LL_iodelay;
        _DX = port;
        _AL = val;
        __emit__((char)0xEE);
        LL_iodelay;
}


/* ------------------------------------------------------------ */

/* cancel any pending XOFF condition */

void pascal ComCancelXoff(void)
{
        LL_SendXon();
}


/* ------------------------------------------------------------ */

 /* wait for all pending transmit data to be sent */

void pascal ComFlush(int cancel)
{
        if (cancel)
        {
                if (WS.Option.RtsHandshake)
                        raiseRTS();
                if (WS.Option.XonHandshake)
                        LL_SendXon();
                ComLL.TXoffActive = 0;
        }

        while (ComWritePending() && !ComCarrierLost())
        {
                ComIdle(110+cancel);
        }
}


/* ------------------------------------------------------------ */

/* lower RTS to inhibit modem sending more data to us */

void pascal lowerRTS(void)
{
        LL_lowerRTS();
}

/* raise RTS to modem to continue sending */

void pascal raiseRTS(void)
{
        LL_raiseRTS();
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

                if (WS.Option.XonHandshake)
                {
                        LL_SendXoff();
                        LL_startTransmit();
                }

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

                        if ( TIMER_UP(fail_timeout) ||
                             (RXQUE.qcount >= RXQ_SIZE) )
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

                if (WS.Option.XonHandshake)
                        LL_SendXon();
        }
}

/* ------------------------------------------------------------ */

int pascal ComGetCts(void)
{
        if (!WS.Option.CtsHandshake)
                return 1;

        return (dinport(ComLL.ComBase+MSR) & MSR_CTS) != 0;
}

/* ------------------------------------------------------------ */

void pascal ComSetHandshake()
{
        if (WS.cancel_link)
        {
                ComLL.TXoffActive = 0;
        }
}

/* ------------------------------------------------------------ */

static void com_reinit(void)
{
        WS.cancel_link = 0;
        ComLL.TXoffActive = 0;
        ComLL.RXoffActive = 0;
        ComLL.TxPriority = 0;
        ComLL.XmitActive = 0;
        ComLL.ErrorLocation = 0;
        ComLL.RxErrorBits = 0;
        INIT_QUE(TXQUE);
        INIT_QUE(RXQUE);
}

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
        register int b;
        void interrupt (far *new_vect)(void);

        com_reinit();

        /* find port base address, irq number, interrupt number and bit mask */
        if ((WS.Option.ComPort > 0) && (WS.Option.ComPort <= MAX_COM))
        {
                ComLL.ComBase = COM_BASE_TABLE[WS.Option.ComPort-1];
                ComLL.ComIrq  = COM_IRQ_TABLE[WS.Option.ComPort-1];
        }

        /* check for non-standard ComBase and ComIrq settings */
        if (WS.Option.ComBase) ComLL.ComBase = WS.Option.ComBase;
        if (WS.Option.ComIrq)  ComLL.ComIrq = WS.Option.ComIrq;

#if 0
        /* switch to the redirected IRQ2 on machines with a cascade interrupt */
        if (ComLL.ComIrq == 2)
        {
                register uchar far *machine_type = MACHINE_TYPE_FLAG;
                if (*machine_type == MACHINE_TYPE_AT)
                        ComLL.ComIrq = 9;
        }
#endif

        /* determine interrupt controller, port vector and PIC mask */
        if (ComLL.ComIrq < 8)
        {
                ComLL.IctlBase = ICTL1_BASE;
                ComLL.PortVect = ComLL.ComIrq + 0x08;
                ComLL.PicMask = 1 << ComLL.ComIrq;
        }
        else {
                ComLL.IctlBase = ICTL2_BASE;
                ComLL.PortVect = ComLL.ComIrq + 0x68;
                ComLL.PicMask = 1 << (ComLL.ComIrq-8);
        }

        if ((ComLL.ComBase == 0) || (ComLL.ComIrq == 0))
        {
                disp_error(TX_BADCHAN, WS.Option.ComPort);
		exit(1);
	}

        ComLL.IirBase  = ComLL.ComBase+IIR;
        ComLL.LsrBase  = ComLL.ComBase+LSR;
        ComLL.MsrBase  = ComLL.ComBase+MSR;

        /* turn off doorway redirection */
        /* asm mov ax,6700h */
        /* asm int 16h */
        __emit__((char)0xb8);
        __emit__((char)0x00);
        __emit__((char)0x67);
        __emit__((char)0xcd);
        __emit__((char)0x16);

        disable();

        /* save the old interrupt handler's vector and install new handler */
        old_vect = _dos_getvect(ComLL.PortVect);
        new_vect = LL_InterruptHandler;
        _dos_setvect(ComLL.PortVect, new_vect);
        disable();

        /* save original UART state so it can be restored before exit */
        old_LCR = dinport(ComLL.ComBase+LCR);
        old_MCR = dinport(ComLL.ComBase+MCR);
        old_IER = dinport(ComLL.ComBase+IER);
        old_FCR = dinport(ComLL.ComBase+FCR);

        /* save original PIC state so it can be restored before exit */
        old_PIC = dinport(ComLL.IctlBase+IPICR);
        if (ComLL.ComIrq >= 8)
                old_PIC1 = dinport(ICTL1_BASE+IPICR);

        /* enable the interrupt via the interrupt controller PIC register */
        new_PIC = dinport(ComLL.IctlBase+IPICR) & ~ComLL.PicMask;
        doutport(ComLL.IctlBase+IPICR, new_PIC);

        /* if this is a high irq, also enable the cascade interrupt */
        if (ComLL.ComIrq >= 8)
        {
                b = dinport(ICTL1_BASE+IPICR) & ~0x04;
                doutport(ICTL1_BASE+IPICR, b);
        }

        /* detect 16550 and enable buffering if needed */
        if (WS.Option.Disable16550)
                ComLL.Is16550 = 0;
        else {
                register uchar tlev;
                if (WS.Option.FifoThresh > 13)
                        tlev = FCR_TLEV14;
                else if (WS.Option.FifoThresh > 7)
                        tlev = FCR_TLEV8;
                else if (WS.Option.FifoThresh > 3)
                        tlev = FCR_TLEV4;
                else
                        tlev = FCR_TLEV1;

                tlev |= FCR_EN_FIFO|FCR_CLEAR|FCR_MODE1;
                doutport(ComLL.ComBase+FCR,tlev);

                ComLL.Is16550 = dinport(ComLL.ComBase+FCR);
                ComLL.Is16550 &= FCR_16550;
        }

        /* allow -FT option to remain in effect for levels of 17 and
           higher.  This may benefit the FORVAL internal modem and
           perhaps windows or os/2 with virtual com drivers. */
        if ((ComLL.Is16550 == 0) && (WS.Option.FifoThresh < 17))
                WS.Option.FifoThresh = 0;

        /* clear divisor latch if needed */
        b = dinport(ComLL.ComBase+LCR) & ~LCR_ABDL;
        doutport(ComLL.ComBase+LCR, b);

        /* initialize the 8250 for interrupts */
        doutport(ComLL.ComBase+IER, IER_DAV|IER_THRE);

        /* program 8250 "out2" bit to enable the IRQ pin */
        b = dinport(ComLL.ComBase+MCR) | MCR_OUT2;
        doutport(ComLL.ComBase+MCR, b);

        /* discard any junk sitting on the uart registers */
        for (b=0; b<2; b++)
        {
                dinport(ComLL.ComBase+MSR);
                dinport(ComLL.ComBase+LSR);
                dinport(ComLL.ComBase+IIR);
                dinport(ComLL.ComBase+RBR);
                com_reinit();
                enable();
                disable();
        }

        enable();

        RtsActive = 0;
        if (WS.Option.RtsHandshake)             // 11-17-93
                raiseRTS();

        if (WS.Option.ComSpeed)
                setSpeed(WS.Option.ComSpeed);

        log_error("COM: Speed=%ld Base=%03x IRQ=%d PIC=%02x/%02x FCR=%02x/%02x fifo=%d\r\n",
                ComGetSpeed(),
                ComLL.ComBase,
                ComLL.ComIrq,
                old_PIC,new_PIC,
                old_FCR,ComLL.Is16550,
                WS.Option.FifoThresh);

/********************************
        log_error("COMM initialization: PIC=%02x/%02x IE=%02x/%02x LS=%02x MC=%02x MS=%02x\r\n",
                old_PIC,
                dinport(ComLL.IctlBase+IPICR),
                old_IER,
                dinport(ComLL.ComBase+IER),
                dinport(ComLL.ComBase+LSR),
                dinport(ComLL.ComBase+MCR),
                dinport(ComLL.ComBase+MSR) );
********************************/
}


/* ------------------------------------------------------------ */

void pascal setSpeed( long speed )
{
        unsigned divisor;
        register uchar b;

        ComFlush(1);

        divisor = (unsigned)(115200L / speed);
        disable();

        /* enable address divisor latch */
        b = dinport(ComLL.ComBase+LCR) | LCR_ABDL;
        doutport(ComLL.ComBase+LCR, b);

        /* set the divisor */
        doutport(ComLL.ComBase+THR, divisor & 255);
        doutport(ComLL.ComBase+THR+1, divisor >> 8);

        /* set 8 bits, 1 stop, no parity, no break, disable divisor latch */
        doutport(ComLL.ComBase+LCR, LCR_8BITS|LCR_1STOP|LCR_NPARITY|LCR_NOBREAK);

        /* discard one garbage character */
        /* dinport(ComLL.ComBase); */
        enable();
}


/* ------------------------------------------------------------ */

long pascal ComGetSpeed(void)
{
        unsigned divisor;
        register uchar b;

        if (ComLL.ComBase == 0)
                return 0;

        disable();

        /* enable address divisor latch */
        b = dinport(ComLL.ComBase+LCR) | LCR_ABDL;
        doutport(ComLL.ComBase+LCR,b);

        /* get the divisor */
        divisor = dinport(ComLL.ComBase+THR);
        divisor = (dinport(ComLL.ComBase+THR+1) << 8) + divisor;

        /* disable the divisor latch */
        b = dinport(ComLL.ComBase+LCR) & ~LCR_ABDL;
        doutport(ComLL.ComBase+LCR,b);

        enable();

        if (divisor)
                return 115200L / divisor;
        else
                return 0;
}


/* ------------------------------------------------------------ */

 /*
  * remove interrupt handlers for the com port must be called before exit to
  * system 
  */

void pascal ComClose(void)
{
        register int b;

        if (ComLL.ComBase == 0)
                return;

        /* wait for the pending data to flush from the queue */
        ComFlush(2);
        disable();

        /* disable 16550 buffering if it was not originally enabled */
        if (ComLL.Is16550 && !(old_FCR & FCR_FIFO))
                doutport(ComLL.ComBase+FCR, FCR_DISABLE);

        /* restore other registers to previous states */
        doutport(ComLL.ComBase+LCR, old_LCR);
        doutport(ComLL.ComBase+MCR, old_MCR);
        doutport(ComLL.ComBase+IER, old_IER);

        /* discard any junk sitting on the uart registers */
        dinport(ComLL.ComBase+MSR);
        dinport(ComLL.ComBase+LSR);
        dinport(ComLL.ComBase+IIR);
        dinport(ComLL.ComBase+RBR);

        /* restore original interrupt controller state for this
           interrupt only; other interrupts may have changed in multi-
           tasking systems such as DV */
        new_PIC = (dinport(ComLL.IctlBase+IPICR) & ~ComLL.PicMask) |
            (old_PIC & ComLL.PicMask);
        doutport(ComLL.IctlBase+IPICR, new_PIC);

        /* if this is a high irq, also restore cascade interrupt PIC */
        if (ComLL.ComIrq >= 8)
        {
                b = (dinport(ICTL1_BASE+IPICR) & ~0x04) |
                    (old_PIC1 & 0x04);
                doutport(ICTL1_BASE+IPICR, b);
        }

        /* attach the old handler to the interrupt vector */
        _dos_setvect(ComLL.PortVect, old_vect);

        enable();

        /* turn on doorway redirection */
        /* asm mov ax,6701h */
        /* asm int 16h */
        __emit__((char)0xb8);
        __emit__((char)0x01);
        __emit__((char)0x67);
        __emit__((char)0xcd);
        __emit__((char)0x16);

        ComLL.ComBase = 0;

#ifdef DEBUG
        if (WS.Option.Debug)
        {
                int i;
                printf("\nISR Count: %ld Starts: %ld/%ld \n"
                        "Stalls: %ld Holds: %u\n"
                        "MultiTX: %ld MultiRX: %ld\n",
                        ComLL.IsrCount,ComLL.StartCount,ComLL.StartPolls,
                        ComLL.StallCount,WS.Comstat.TransmitHolds,
                        ComLL.MulTxCount,ComLL.MulRxCount);

                for (i=0; i<trace_next; i++)
                {
                        switch (Trace[i].event)
                        {
                        default:
                                printf("%04d:%c-%02x\n",i,
                                        Trace[i].event,Trace[i].data);
                        }
                }
        }
#endif
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

        if ((dinport(ComLL.ComBase+MSR) & MSR_RLSD) == 0)
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
        return RXQUE.qcount;
}


/* ------------------------------------------------------------ */

void pascal ComReportErrors(void)
{
        if (ComLL.RxErrorBits != 0)
        {
                PRIVATE.extnak.errlsr = ComLL.RxErrorBits;
                PRIVATE.extnak.errcsip = ComLL.ErrorLocation;

                sprintf(WS.Comstat.ErrorMessage,"COM:%s%s%s %08lX",
                    (ComLL.RxErrorBits & LSR_OERR)? TX_OVERRUN:"",
                    (ComLL.RxErrorBits & LSR_FERR)? TX_FRAMING:"",
                    (ComLL.RxErrorBits & RXQ_OVERFLOW_BIT)? TX_OVERFLOW:"",
                    ComLL.ErrorLocation);

                report_rx_error(WS.Comstat.ErrorMessage);

                /* don't double-count com errors since they most always
                   end up generating a second error report */
                if (WS.receive_errors > 1)
                        --WS.receive_errors;

                if (ComLL.RxErrorBits & LSR_OERR)
                        ++WS.Comstat.OverrunErrors;

                if (ComLL.RxErrorBits & (LSR_FERR|LSR_BREAK))
                        ++WS.Comstat.FramingErrors;

                if ((WS.Option.SlowHandshake == 0) && (WS.Comstat.OverrunErrors >= MAX_OVERRUN))
                {
                        display_warning(TX_SLOWON);
                        WS.Option.SlowHandshake = 1;
                }

                if (WS.Comstat.FramingErrors > MAX_FRAMING)
                {
                        display_warning(TX_BADB);
                        set_cancel_link(CANCEL_FRAMING_ERRORS);
                }

                ComLL.RxErrorBits = 0;
        }
}


/* ------------------------------------------------------------ */

/* report status of queues and flow control */

void pascal ComReportStatus(int where)
{
        char temp[80];

        sprintf(temp,
                TX_COMSTATUS
        #ifdef DEBUG
                "s=%03ld "
        #endif
                ,
                ComGetCts()?                    '.':'C',
                RtsActive?                      '.':'R',
                ComLL.TXoffActive?              'T':'.',
                ComLL.RXoffActive?              'X':'.',
                ComLL.XmitActive?               ComLL.XmitActive+'A'-1:'.',
                (ComLL.RxErrorBits & LSR_OERR)? 'O':'.',
                (ComLL.RxErrorBits & LSR_FERR)? 'F':'.',
                ComWritePending(),
                ComReadPending(),
                PRIVATE.rxbuf.nextin,
                where,
                WS.cancel_link
        #ifdef DEBUG
                , ComLL.StallCount
        #endif
                );

        cprintf(temp);
        clreol();

        /* log the comstatus following flow control failure- otherwise
           don't log this (possibly) frequent display */
        if (where == 1101)
                log_error("%s\r\n",temp);
}


/* ------------------------------------------------------------ */

/* wait for and return 1 character from the active com port */

int pascal ComReadChar(void)
{
        uchar c;

        while (RXQUE.qcount == 0)
        {
                ComIdle(130);
                if (ComCarrierLost())
                        return END_PACKET_CHR;
        }

        ComReadStr(&c,1);
        return c;
}

/* ------------------------------------------------------------ */

/* reads multiple characters into a buffer, returning number of
   characters actually read, up to bufsiz */

unsigned pascal ComReadStr(uchar *dest, unsigned bufsiz)
{
        _CX = RXQUE.qcount;
        if (_CX == 0)
                return 0;

        if (_CX < bufsiz)
                bufsiz = _CX;

#ifdef DEBUG
        if (WS.Option.Debug) printf("read: qc=%d siz=%d",RXQUE.qcount,bufsiz);
#endif

        /* block move the characters, if possible */
        if (bufsiz < (RXQ_SIZE-RXQUE.qnext_out))
        {
                mem_copy(dest,&(RXQUE_qdata[RXQUE.qnext_out]),bufsiz);

                disable();
                RXQUE.qnext_out += bufsiz;
                if (RXQUE.qnext_out >= RXQ_SIZE)
                        RXQUE.qnext_out = 0;
                RXQUE.qcount -= bufsiz;
                enable();
#ifdef DEBUG
                dest += bufsiz;
                if (WS.Option.Debug) printf(", block move");
#endif
        }
        else
        {
#ifdef DEBUG
                if (WS.Option.Debug) printf(", singles");
#endif
                /* deque individual characters */
                _CX = bufsiz;
                _BX = RXQUE.qnext_out;
                while (_CX--)
                {
                        *dest++ = RXQUE_qdata[_BX++];
                        if (_BX >= RXQ_SIZE) _BX = 0;
                }

                disable();
                RXQUE.qnext_out = _BX;
                RXQUE.qcount -= bufsiz;
                enable();
        }

#ifdef DEBUG
        if (WS.Option.Debug)
        {       int i;
                printf("(");
                dest -= bufsiz;
                for (i=0; i<bufsiz; i++)
                        printf("%02x%c",dest[i],i==bufsiz-1?')':' ');
                printf("\n");
        }
#endif

        /* report any errors */
        ComReportErrors();

        /* release flow control if needed */
        if (RXQUE.qcount <= QLOW_WATER)
        {
                if (WS.IoLevel == 0)
                {
                        if (ComLL.RXoffActive != 0)
                                LL_SendXon();
                        if ((RtsActive == 0) && WS.Option.RtsHandshake)
                                raiseRTS();
                }
        }

        return bufsiz;
}

/* ------------------------------------------------------------ */

 /*
  * que a character to be transmitted over the com port
  */

void pascal ComWriteChar(uchar c)
{
        ComWriteStr(&c,1);
}


/* ------------------------------------------------------------ */

 /*
  * transmits a string to the specified com port
  */

void pascal ComWriteStr(uchar *src, unsigned count)
{
        /* wait here if output buffer is full */
        if (TXQUE.qcount > (TXQ_SIZE-1-count))
        {
#ifdef DEBUG
                if (WS.Option.Debug)
                        printf("TXQ FULL! %d<%d\n",TXQUE.qcount,count);
#endif
                ComIdle(140);
                service_receive();
                if (ComCarrierLost())
                        return;
        }

#ifdef DEBUG
        if (WS.Option.Debug) printf("write: count=%d",count);
#endif

        /* move data in a block if no wrapping would occur */
        if (count < (TXQ_SIZE-TXQUE.qnext_in))
        {
                mem_copy(&(TXQUE_qdata[TXQUE.qnext_in]),src,count);

                disable();
                TXQUE.qnext_in += count;
                if (TXQUE.qnext_in >= TXQ_SIZE)
                        TXQUE.qnext_in = 0;
                TXQUE.qcount += count;
                enable();
#ifdef DEBUG
                if (WS.Option.Debug) printf(", block move");
#endif
        }
        else
        {
                /* enque individual characters to be transmitted */
                _BX = TXQUE.qnext_in;
                _CX = count;
                while (_CX--)
                {
                        TXQUE_qdata[_BX++] = *src++;
                        if (_BX >= TXQ_SIZE) _BX = 0;
                }

                disable();
                TXQUE.qnext_in = _BX;
                TXQUE.qcount += count;
                enable();
#ifdef DEBUG
                src -= count;
                if (WS.Option.Debug) printf(", singles");
#endif
        }

#ifdef DEBUG
        if (WS.Option.Debug)
        {       int i;
                printf("<");
                for (i=0; i<count; i++)
                        printf("%02x%c",src[i],i==count-1?'>':' ');
                printf("\n");
        }
#endif

        /* force an initial interrupt to get things rolling (in case
           there are no more pending transmit-ready interrupts */

        if (ComLL.XmitActive != 1)
                LL_startTransmit();

        ptxq_count = 0;
}


/* ------------------------------------------------------------ */

 /* returns the number of characters queued to be transmitted */

int pascal ComWritePending(void)
{
        return TXQUE.qcount;
}

/* ------------------------------------------------------------ */

/* returns the highest possible ComWritePending value before a call to
   ComWrite will block on a transmit queue full condition */

int pascal ComWriteMax(void)
{
        return TXQ_SIZE-250;
}

/* -------------------------------------------------------------- */

/* discard any pending output */

void pascal discard_TxQue(void)
{
        if (TXQUE.qcount > 100)
        {
                disable();
                INIT_QUE(TXQUE);
                enable();
                ComCancelXoff();
                ComWriteChar(END_PACKET_CHR);
                ComWriteChar(END_PACKET_CHR);
        }
}

/* -------------------------------------------------------------- */

/* Comm driver is idle - service keyboard, check for user abort */

void pascal ComIdle(int where)
{
        static clock_t poll_timeout = 0;
        static int pc;
        static int local_can_count;
        int c;

        /* restart output after a flow pause */
        if (ComLL.XmitHeld)
        {
                disable();
                if ((ComLL.XmitActive != 1) && (TXQUE.qcount != 0))
                        LL_startTransmit();
                enable();
        }

        if (ComLL.RxErrorBits)
                ComReportErrors();

        /* release flow control if needed */

        if (RXQUE.qcount <= QLOW_WATER)
        {
                if (WS.IoLevel == 0)
                {
                        if (ComLL.RXoffActive != 0)
                                LL_SendXon();
                        if (WS.Option.RtsHandshake)
                                raiseRTS();
                }
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

        /* try to catch a lost THRE interrupt */

        if (!ComLL.XmitHeld && TXQUE.qcount && (TXQUE.qcount == ptxq_count))
           if (dinport(ComLL.ComBase+LSR) & LSR_THRE)
           {

                /* diagnostic for multiple lost THRE errors */
#if 0
                EVERSION("THRE failure! PIC=%02x/%02x IE=%02x/%02x LS=%02x MC=%02x MS=%02x\r\n",
                                old_PIC,
                                dinport(ComLL.IctlBase+IPICR),
                                old_IER,
                                dinport(ComLL.ComBase+IER),
                                dinport(ComLL.ComBase+LSR),
                                dinport(ComLL.ComBase+MCR),
                                dinport(ComLL.ComBase+MSR)
                                );
#endif
                disable();
                if ((ComLL.XmitActive == 1) && TXQUE.qcount)
                        ComLL.XmitActive = 2;
                enable();
           }
        ptxq_count = TXQUE.qcount;


        /* restart output after a flow pause */
        if ((ComLL.XmitActive != 1) && (TXQUE.qcount != 0))
                LL_startTransmit();

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
                                        char cancels[CANCEL_COUNT+2] = {CAN_CHR,CAN_CHR,CAN_CHR,CAN_CHR,CAN_CHR,CAN_CHR};
                                        ERECV(TX_CTRLX);
                                        PRIVATE.can_count = local_can_count;
                                        set_cancel_link(CANCEL_KEYBOARD);
                                        discard_TxQue();
                                        ComCancelXoff();
                                        ComWriteStr(cancels,CANCEL_COUNT+2);
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


