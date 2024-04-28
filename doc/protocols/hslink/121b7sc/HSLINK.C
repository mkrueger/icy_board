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
 * hslink.c - Main program for HS/LINK protocol.
 *             
 */

#include <stdlib.h>
#include <stdio.h>
#include <conio.h>
#include <io.h>
#include <dos.h>

#ifdef __TURBOC__
#include <dir.h>
#include <alloc.h>
#endif

#include "\hdk\hspriv.h"
#include <hdk.h>

#include "hsuid2.h"
#include "hsl.h"
#include "\hdk\hdktext.h"
#include "hstext.h"

/* -------------------------------------------------------------- */
/* private data */

#ifdef FOSSIL
        static char hslink[] = "FHSLINK.EXE";
#else
        static char hslink[] = "HSLINK.EXE";
#endif

static pathname_node *curnode;

/* -------------------------------------------------------------- */
/* private procedure prototypes */

static int control_c(void);

#ifdef DEBUG
        #define TRACE(MSG) disp_error(MSG)
#else
        #define TRACE(MSG) {}
#endif

/* -------------------------------------------------------------- */

/*
 * protocol main program - send specified files, receiving in background.
 *                         continue receiving after last transmit until
 *                         receiver is satisfied.
 *
 */

void main(int argc,
          char *argv[])
{
        char *exe_path;

TRACE("[main 1]\r\n");

        set_version();
        log_error("\r\n%s\r\n",version);

        log_error(TX_HEAPAVAIL,coreleft());

TRACE("[main 2]\r\n");

#ifdef STATIC_WORKSPACE
        /* check that there is sufficient memory for operation */
        if (coreleft() < SPAREMEM)
        {
                disp_error(TX_OUTOFRAM,
                        SPAREMEM-coreleft());
                delay(3000);
                exit(1);
        }
#else
        /* allocate a workspace, if needed */
        if (coreleft() < (SPAREMEM+sizeof(workspace_rec))
        {
                disp_error(TX_OUTOFRAM,
                        SPAREMEM+sizeof(workspace_rec)-coreleft());
                delay(3000);
                exit(1);
        }
        current_hsws = mem_alloc(sizeof(workspace_rec));
#endif
TRACE("[main 3]\r\n");

        /* search PATH if argv[0] does not contain this information
           (compatibility with DOS 2.xx) */
        exe_path = argv[0];
        if (*exe_path == 0)
            exe_path = searchpath(hslink);
        if (exe_path == 0)
            exe_path = hslink;

TRACE("[main 4]\r\n");
        /* record EXE file pointer and check for (BRAND) variants */
        /* also loads any existing branding information and
           checks exe for tampering */
        if (brand_detected(exe_path,(uchar*)argv[1]))
        {
                if (argc == 4)
                {
                        cprintf("\r\n%s\r\n",version);
                        brand_new_user(exe_path,(uchar*)argv[2],(uchar*)argv[3]);

                        if (local_userid())
                        {
                                identify_user();
                                usage_license();
                        }
                        else {
                                cprintf(TX_BADREG);
                                delay(1000);
                        }
TRACE("[main 5-exit]\r\n");
                        exit(0);
                }
        }

TRACE("[main 6]\r\n");
        /* initialize the hdk */
        if (top_init())
                exit(1);

        /* initialize system, process command line options */

TRACE("[main 7]\r\n");
        set_defaults();

        if (argc == 1)
        {
                usage(TX_NOCMDLINE,"");
                exit(CANCEL_BAD_OPTION);
        }

TRACE("[main 8]\r\n");
        if (process_options(argc,argv))
                exit(CANCEL_BAD_OPTION);

TRACE("[main 9]\r\n");
        ComOpen();
TRACE("[main 10]\r\n");

        WS.Option.ComSpeed = ComGetSpeed();
        if (WS.Option.ComSpeed == 0)
        {
                disp_error(TX_BADCOM);
                exit(CANCEL_BAD_COMSPEED);
        }

        if (WS.Option.EffSpeed == 0)
                WS.Option.EffSpeed = WS.Option.ComSpeed;

TRACE("[main 11]\r\n");
        ctrlbrk(control_c);

        /* allocate remaining memory to buffers */
        WS.buffer_sizes = mem_avail();

TRACE("[main 12]\r\n");
        /* we're now ready for SlowHandshake and DirectVideo to work */
        WS.IoLevel = 0;
        directvideo = WS.Option.DirectVideo;

        if (!WS.Option.FullDisplay)
                cprintf("\r\n%s\r\n",version);

TRACE("[main 13]\r\n");

        /* jump into terminal mode, if needed */
        if (WS.Option.TermMode)
        {
                if (terminal_mode())
                {
                        cprintf(TX_EXIT);
                        ComClose();
                        exit(0);
                }
                WS.Option.TermMode = 0;
        }


#ifdef DEBUG
        printf("bufsiz=%u core=%u\n",WS.buffer_sizes,coreleft());
#endif

        /* display opening screen */

        prepare_display();
        identify_user();
        echo_command_line(argc,argv);
        process_filespecs(argc,argv);

        /* verify hardware handshake status */

        if (!ComGetCts() && !WS.Option.ForceCts)
        {
                display_warning(TX_NOCTS);
                WS.Option.CtsHandshake = 0;
        }

        /* start the "settings" display with COM port and speed.  The rest
           is filled in following a ready handshake with the remote */

        display_comport(1);

        /* identify this copy to remote */
        {
                char ident[120];
                sprintf(ident,local_userid()?
                        TX_IDREG:TX_IDUNREG,
                        sender_name,
                        local_userid());
                ComWriteStr(ident,strlen(ident));
        }

        /* wait for ready handshake with remote */

        while (wait_for_ready())
        {
                ComIdle(300+PRIVATE.ready_context);
                service_receive();
        }

        /*
	 * transmit each outgoing file (received files are processed in the
	 * background, during ACK waits) 
	 */

        if (WS.send_expected)
                display_outgoing_files();

#ifdef DEBUG
        printf("buffer_sizes=%d mem_avail=%u\n",WS.buffer_sizes,mem_avail());
#endif
        curnode = WS.first_send;
        while (curnode)
        {

                while (transmit_file(curnode->name))
                {
                        ComIdle(310+PRIVATE.transmit_context);
                        service_receive();
                }

                curnode = curnode->next;      /* select next file in batch */
        }

        /* wait for remaining receive activity to terminate */

        PSEND(TX_TXFILES,
                WS.files_sent,
                WS.files_sent==1? TX_TXDONESINGLE:TX_TXDONEPLURAL);

        while (finish_receive())
        {
                ComIdle(320+PRIVATE.finish_context);
                service_receive();
        }

        /* close down link */

        while (terminate_link())
        {
                ComIdle(330+PRIVATE.terminate_context);
        }

        /* process exit codes */

        if (ComCarrierLost())
                set_cancel_link(CANCEL_CARRIER_LOST);

        if ((WS.files_received+WS.files_sent) ==0)
                set_cancel_link(CANCEL_NO_FILES);

        ComClose();
        close_display();

        disp_error(TX_FINISHED,
                WS.files_sent,
                WS.files_received,
                (int)WS.cancel_link);

        if (WS.cancel_link)
                delay(3000);

        exit(WS.cancel_link);
}


/* -------------------------------------------------------------- */

/* process user break */

int control_c(void)
{
	return 1;       /* continue program */
}


