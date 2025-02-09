
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/* ------------------------------------------------------------ */

/*
 * Interrupt controller definitions
 *
 */

#define ICTL1_BASE   0x20       /* interrupt controller #1 base address */
#define ICTL2_BASE   0xA0       /* interrupt controller #2 base address */

#define IACKR        0x00       /* offset from base to interrupt ACK register */
   #define IACK_NEOI    0x20    /* nonspecific EOI */

#define IPICR        0x01       /* offset from base to interrupt PIC register */

#define MACHINE_TYPE_FLAG MK_FP(0xFFFF,0x000E)
                                /* ROM flag indicates if second interrupt
                                   controller is available */

#define MACHINE_TYPE_AT 0xFC    /* MACHINE_TYPE_FLAG value for AT class
                                   machines with second interrupt controllers */


/*
 * Uart register definitions
 *
 */

#define RBR          0          /* receive buffer register */
#define THR          0          /* transmit holding register */

#define DLM          1          /* divisor latch MSB */

#define IER          1          /* interrupt enable register */
   #define IER_DAV      0x01    /* data available interrupt */
   #define IER_THRE     0x02    /* THR empty interrupt */
   #define IER_LSRC     0x04    /* line status change interrupt */
   #define IER_MSR      0x08    /* modem status interrupt */


#define IIR          2          /* interrupt identification register */
   #define IIR_PENDING  0x01    /* low when interrupt pending */

   #define IIR_MASK     0x06    /* mask for interrupt identification */
     #define IIR_MSR      0x00  /* modem status change interrupt */
     #define IIR_THRE     0x02  /* transmit holding reg empty interrupt */
     #define IIR_DAV      0x04  /* data available interrupt */
     #define IIR_LSR      0x06  /* line status change interrupt */


#define FCR          2          /* FIFO control register */
   #define FCR_16550    0x80    /* bit set if chip is 16550 or 16550AN */
   #define FCR_FIFO     0x40    /* non-0 if 16550 buffering enabled */

   #define FCR_EN_FIFO  0x01    /* enable transmit and receive fifos */
   #define FCR_CLEAR    0x06    /* clear tx and rx fifos */
   #define FCR_MODE1    0x08    /* change tx/rx pins to mode 1 */
   #define FCR_TLEV1    0x00    /* fifo trigger level = 1 */
   #define FCR_TLEV4    0x40    /* fifo trigger level = 4 */
   #define FCR_TLEV8    0x80    /* fifo trigger level = 8 */
   #define FCR_TLEV14   0xC0    /* fifo trigger level = 14 */

   #define FCR_ENABLE1  (FCR_EN_FIFO|FCR_CLEAR|FCR_MODE1|FCR_TLEV1)
                                /* sample fifo enable byte */

   #define FCR_DISABLE  0x00    /* write to port to disable FIFO */


#define LCR          3          /* line control register */
   #define LCR_5BITS    0x00    /* 5 data bits */
   #define LCR_7BITS    0x02    /* 7 data bits */
   #define LCR_8BITS    0x03    /* 8 data bits */

   #define LCR_1STOP    0x00    /* 1 stop bit */
   #define LCR_2STOP    0x04    /* 2 stop bits */

   #define LCR_NPARITY  0x00    /* no parity */
   #define LCR_EPARITY  0x38    /* even parity */

   #define LCR_NOBREAK  0x00    /* break disabled */
   #define LCR_BREAK    0x40    /* break enabled */

   #define LCR_NORMAL   0x00    /* normal */
   #define LCR_ABDL     0x80    /* address baud divisor latch */


#define MCR          4          /* modem control register */
   #define MCR_DTR      0x01    /* active DTR */
   #define MCR_RTS      0x02    /* active RTS */
   #define MCR_OUT1     0x04    /* enable OUT1 */
   #define MCR_OUT2     0x08    /* enable OUT2 -- COM INTERRUPT ENABLE */
   #define MCR_LOOP     0x10    /* loopback mode */


#define LSR          5          /* line status register */
  #define LSR_DAV      0x01     /* data available */
  #define LSR_OERR     0x02     /* overrun error */
  #define LSR_PERR     0x04     /* parity error */
  #define LSR_FERR     0x08     /* framing error */
  #define LSR_BREAK    0x10     /* break received */
  #define LSR_THRE     0x20     /* THR empty */
  #define LSR_TSRE     0x40     /* transmit shift register empty */


#define MSR          6          /* modem status register */
  #define MSR_DCTS     0x01     /* delta CTS */
  #define MSR_DDSR     0x02     /* delta DSR */
  #define MSR_DRING    0x04     /* delta ring */
  #define MSR_DRLSD    0x08     /* delta receive line signal detect */
  #define MSR_CTS      0x10     /* clear to send */
  #define MSR_DSR      0x20     /* data set ready */
  #define MSR_RING     0x40     /* ring detect */
  #define MSR_RLSD     0x80     /* receive line signal detect */


#ifdef extern

#define MAX_COM 8

/* table of port base and interrupt vector for each com port */

/*       COM1  COM2  COM3  COM4  COM5  COM6  COM7  COM8 */
unsigned COM_BASE_TABLE[] =
        {0x3F8, 0x2F8, 0x3E8, 0x2E8, 0x000, 0x000, 0x2E8, 0x3E8};

char COM_IRQ_TABLE[] =
        {4,     3,     4,     3,     0,     0,     5,     5};


#endif

