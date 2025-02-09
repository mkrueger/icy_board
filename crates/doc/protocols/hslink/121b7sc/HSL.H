
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * hsl.h - HS/Link, HSLINK.EXE application specific declarations
 *
 */

#define STACKSIZE 5000                  /* space for dedicated stack */

#define SPAREMEM 1024


/* -------------------------------------------------------------------- */
/* ---------- public working storage ---------------------------------- */
/* -------------------------------------------------------------------- */

extern char version[75];                /* program version */

extern char RtsActive;                  /* is RTS up or down? */


/* -------------------------------------------------------------------- */
/* ---------- application specific prototypes ------------------------- */
/* -------------------------------------------------------------------- */

/* from hsdisp.c */
void pascal identify_user(void);
void pascal echo_command_line(int argc, char **argv);
void pascal display_comport(int uart);

int pascal terminal_mode(void);
        /* simple glass-tty terminal mode */
        /* returns 0 to perform file transfer, non-0 to exit program */

void pascal set_version(void);
        /* set application version and exefile crc */

