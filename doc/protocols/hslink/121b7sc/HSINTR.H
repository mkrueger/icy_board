
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */

/*
 * hsintr.h - HS/Link low level com port declarations and interrupt service
 *
 */

#define xTXQ_SIZE  5120  /* size of transmitter interrupt queue */
#define xRXQ_SIZE  6400  /* size of receiver interrupt queue */

#define TXQ_SIZE  8192  /* size of transmitter interrupt queue */
#define RXQ_SIZE  8192  /* size of receiver interrupt queue */

#define QLOW_WATER (RXQ_SIZE-512)
                        /* rx qcount to issue XOFF signal */
#define QHIGH_WATER (QLOW_WATER-256)
                        /* rx qcount to resume with XON signal */

#define RXQ_OVERFLOW_BIT 0x80
                        /* Bit set in ComLL.RxErrorBits when buffer overflows */

typedef struct {
        int qnext_in;
        int qnext_out;
        int qcount;
} queue_rec;

#define INIT_QUE(Q)                     \
   {                                    \
      Q.qnext_in = 0;                   \
      Q.qnext_out = 0;                  \
      Q.qcount = 0;                     \
   }

typedef struct {
        long ComSpeed;          /* com port speed */
        uchar ComPort;          /* standard com port number */
        uchar ComIrq;           /* com port IRQ number */

        unsigned ComBase;       /* com port base address */
        unsigned IirBase;       /* ComLL.ComBase+IIR */
        unsigned LsrBase;       /* ComLL.ComBase+LSR */
        unsigned MsrBase;       /* ComLL.ComBase+MSR */

        unsigned PortVect;      /* com port interrupt vector number */
        unsigned PicMask;       /* interrupt controller PIC init mask */
        unsigned IctlBase;      /* interrupt controller base address */
        uchar Is16550;          /* using a buffered uart? */

        queue_rec RxQue;        /* receive data queue */
        uchar RxQue_qdata[RXQ_SIZE+2];

        queue_rec TxQue;        /* transmit data queue */
        uchar TxQue_qdata[TXQ_SIZE+2];
        uchar TxPriority;       /* is there a priority character to send? */

        uchar TXoffActive;      /* has XOFF suspended transmit? */
        uchar RXoffActive;      /* has receive issued XOFF? */
        uchar XmitActive;       /* is THRE interrupt expected? */
        uchar XmitHeld;         /* is transmit held due to flow control? */

        uchar RxErrorBits;      /* last receiver error bits */
        long ErrorLocation;     /* fg CS:IP at time of error */

#ifdef DEBUG
        long IsrCount;          /* count of comm interrupts */
        long StartPolls;        /* count of transmit start polls */
        long StartCount;        /* count of actual transmit starts */
        long StallCount;        /* count of TX stalls */
        long MulTxCount;        /* count of multiple TX preques */
        long MulRxCount;        /* count of multiple RX deques */
#endif

} ComLowLevelData;

extern volatile ComLowLevelData ComLL;   /* Com port low level data values */

#define TXQUE ComLL.TxQue
#define TXQUE_qdata ComLL.TxQue_qdata
#define RXQUE ComLL.RxQue
#define RXQUE_qdata ComLL.RxQue_qdata

/**
#define LL_iodelay __emit__((char)0xEB),__emit__((char)0x00),__emit__((char)0xEB),__emit__((char)0x00),__emit__((char)0xEB),__emit__((char)0x00)
**/
#define LL_iodelay __emit__((char)0xE9),__emit__((char)0x00),__emit__((char)0x00)

void /*interrupt*/ LL_InterruptHandler(void);
        /* new interrupt vector handler */

void pascal LL_startTransmit(void);
        /* kickstart transmit interrupt when needed */

void pascal LL_SendXoff(void);
void pascal LL_SendXon(void);

extern uchar old_FCR;                   /* original register values */
extern uchar old_LCR;
extern uchar old_MCR;
extern uchar old_IER;
extern uchar old_PIC;
extern uchar old_PIC1;
extern uchar new_PIC;

#ifndef __TURBOC__
        #define interrupt __cdecl __interrupt
#endif

extern void (interrupt far *old_vect) (void);    /* old vector handler */

void pascal lowerRTS(void);
void pascal raiseRTS(void);

void pascal LL_lowerRTS(void);
void pascal LL_raiseRTS(void);

#ifdef DEBUG
        #define TRACE_MAX       2000
        typedef struct {
                uchar event;
                uchar data;
        } trace_rec;

        extern trace_rec Trace[TRACE_MAX+1];
        extern int trace_next;
        extern void pascal LL_addtrace(void);
        #define TRACE(EVENT,DATA) {                     \
                asm PUSH DX;                            \
                _DL = EVENT;                            \
                _DH = DATA;                             \
                LL_addtrace();                          \
                asm POP DX;                             \
        }
#else
        #define TRACE(EVENT,DATA) {}
#endif

