
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * fossil.h - HS/Link fossil function definitions
 *
 * This header defines the FOSSIL functions used by the fossil version
 * of hs/link.
 *
 */

#define fSetSpeed       0x00    /* set baud rate, code in AL */
        #define fBaudOr         0x03    /* value to OR with speed code */
                                        /* ... 8 data bits, 1 stop, no parity */
        #define fBaudMask       0xE0    /* mask for baud rate bits in fInfo */
        #define fBaud300        0x40
        #define fBaud600        0x60
        #define fBaud1200       0x80
        #define fBaud2400       0xa0
        #define fBaud4800       0xc0
        #define fBaud9600       0xe0
        #define fBaud19200      0x00    /* old 110 */
        #define fBaud38400      0x20    /* old 150 */

#define bPutCh          0x01    /* int14 put character in AL */
                                /* return: AH=00h ok */
                                /*         AH=80h queue full */

#define fReceiveCh      0x02    /* receive character to AL with wait */
                                /* fossil: wait for character */
                                /* int14 return: AH=00h ok */
                                /*                      AL=char */
                                /*               AH=80h no chars available */

#define fStatus         0x03    /* status to AX */
   #define fsAhTimeout  0x80    /* fossil timeout occured in 0x01 or 0x02 */
   #define bsAhLNE      0x80    /* int14 link not established */
   #define bsAhTHRE     0x20    /* int14 free space in tx buffer */
   #define bsAhFE       0x08    /* int14 framing error */
   #define bsAhPE       0x04    /* int14 parity error */
   #define bsAhDO       0x02    /* int14 data overrun */
   #define bsAhDR       0x01    /* int14 data ready */
   #define fsAlCTS      0x10    /* CTS status */
   #define fsAlDCD      0x80    /* carrier detect status */

#define fInit           0x04    /* initialize/check fossil loaded */
   #define fiFossilOk   0x1954  /* fossil return AX if fossil present */
                                /* int14 entry DH=80h to open port */
                                /* int14 entry AL=FFh/DL=port */
   #define biAlOpen     0x01    /* int14 open channel.  missing this bit
                                        closes the channel and lowers DTR! */
   #define biAlDTR      0x02    /* int14 set dtr */
   #define biAlFlushIB  0x10    /* int14 flush input buffer */
   #define biAlFlushOB  0x20    /* int14 flush output buffer */
                                /* int14 return AH=0 if ok else error */

#define fDeInit         0x05    /* deinit */

#define fSetDTR         0x06    /* raise/lower DTR */
#define fPurgeOut       0x09    /* discard pending output */
#define fPurgeIn        0x0a    /* discard pending input */

#define fSendChN        0x0b    /* send character in AL, no wait */

#define fSetFlow        0x0f    /* sets flow control */
        #define fFlowXON        0xF9    /* XON/XOFF send+receive */
        #define fFlowCTS        0xF2    /* CTS/RTS flow control */

#define fCtlCK          0x10    /* enable/disable ^C/^K checking */

#define fReceiveBlk     0x18    /* block receive */
                                /* ES:DI -> buffer */
                                /* CX -> size*/
                                /* return AX -> bytes read */

#define fSendBlk        0x19    /* block transmit */
                                /* ES:DI -> buffer */
                                /* CX -> size*/
                                /* return AX -> */

#define fGetInfo        0x1b    /* return information about the driver */

        struct fInfo {  /* structure returned by fGetInfo function */
                int strsiz;             /* actual size of this structure */
                char majver;            /* major version number */
                char minver;            /* minor version number */
                char far *ident;        /* driver identifier */
                int ibufr;              /* input buffer size */
                int ifree;              /* input buffer free space */
                int obufr;              /* output buffer size */
                int ofree;              /* output buffer free space */
                unsigned char swidth;   /* screen width */
                unsigned char sheight;  /* screen height */
                unsigned char baud;     /* current baud code */
        };

#define fnInit          0x1c    /* new fossil init function */
#define fnDeInit        0x1d    /* new deinit */

#define fxControl       0x1f    /* extended port status/control */
   #define fxRTS        0x02    /* RTS enable mask */

#define fReceiveChN     0x20    /* receive character to AL, no wait */
                                /* return: AH=00h ok */
                                /*              AL=char */
                                /*         AH=80h no chars available */
                                /* not available in int14 */

#define bNetDev         0xC1    /* int14 NETDEV presence check */
   #define bnAx         0xC1FF  /* return AL=0 installed, FF not installed */

/* call fossil function */
#ifdef __TURBOC__

   #define EMIT(CODE) __emit__((char)CODE)
   #define FossilAPI(FUNC)  {                                              \
           _DX = WS.Option.ComPort;                                        \
           EMIT(0x4a);                     /* dec dx */                    \
           _AH = FUNC;                                                     \
           EMIT(0x55);                     /* push bp */                   \
           EMIT(0x1e);                     /* push ds */                   \
           EMIT(0xCD), EMIT(0x14);         /* int 14h */                   \
           EMIT(0x1f);                     /* pop ds */                    \
           EMIT(0x5d);                     /* pop bp */                    \
   }

#else

   extern union _REGS REGS;
   extern struct _SREGS SREGS;
   #define _AX REGS.x.ax
   #define _AH REGS.h.ah
   #define _AL REGS.h.al
   #define _BX REGS.x.bx
   #define _BL REGS.h.bl
   #define _CX REGS.x.cx
   #define _DI REGS.x.di
   #define _DS SREGS.ds
   #define _DX REGS.x.dx
   #define _ES SREGS.es

   #define FossilAPI(FUNC)  {                   \
           _DX = WS.Option.ComPort-1;           \
           _AH = FUNC;                          \
           _int86x(0x14,&REGS,&REGS,&SREGS);    \
   }
#endif

