#undef FULLHELP
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
 * hsdisp.c - HS/Link, display partition management services
 *
 */

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <io.h>
#include <conio.h>
#include <fcntl.h>
#include <dos.h>
#include <bios.h>
#include <time.h>

#include "\hdk\hspriv.h"
#include <hdk.h>

#include "hsl.h"
#include "hsintr.h"
#include "hstext.h"

/* -------------------------------------------------------------------- */

/* private local data */

#define FILES_WIDTH 46
#define CHAT_LEFT (FILES_WIDTH+5)
#define CHAT_WIDTH (75-CHAT_LEFT)

/* send and receive window titles */
static char recv_window_title[FILES_WIDTH+3] = TX_INTITLE;
static char send_window_title[FILES_WIDTH+3] = TX_OUTTITLE;


/* display window definition */
typedef struct {
        char x1,y1,x2,y2;
        char *title;
        char cux,cuy;
        char active;
} window_definition;


static window_definition version_window
        = {1,3,80,6,version,1,1,0};

static window_definition recv_window
        = {1,7,FILES_WIDTH+3,13,recv_window_title,1,1,0};
static window_definition send_window
        = {1,14,FILES_WIDTH+3,20,send_window_title,1,1,0};

static window_definition option_window
        = {CHAT_LEFT,7,80,20,TX_SETTINGS,1,1,0};
static window_definition chatin_window
        = {CHAT_LEFT,7,80,13,TX_INCHAT,1,1,0};
static window_definition chatout_window
        = {CHAT_LEFT,14,80,20,TX_OUTCHAT,1,1,0};

static window_definition batch_window
        = {1,21,80,24,TX_BATCHSTAT,1,1,0};

window_definition *current_window = NULL;

int screenheight;
int screenattr;

#ifndef __TURBOC__
        int directvideo;
        #define WHITE 15
        #define BLINK 128
#endif

/* -------------------------------------------------------------------- */

/* private local procedures */

static void pascal usage_advanced(void);
static void pascal usage_basic(void);
static void pascal usage_pause(void);
void pascal usage_registration(void);

static void pascal select_window(window_definition *wd);
static void pascal title_window(window_definition *wd);

static void pascal frame_window(window_definition *wd);

#define CENTER_TITLE(X1,X2,Y,TITLE)                     \
        gotoxy((X1)+((X2)-(X1)-strlen(TITLE))/2-1,Y),   \
        cprintf(" %s ",TITLE);

static void pascal prepare_chat(void);
static void pascal close_chat(void);
static void pascal add_chatout(register char c);

#define DISP_POS (WS.Option.DispPos)


/* -------------------------------------------------------------------- */

void pascal prepare_display(void)
{
        int extra;
        int i;
        #ifdef __TURBOC__
        struct text_info text_info;
        #endif

        if (!WS.Option.FullDisplay)
                return;

#ifdef __TURBOC__
        gettextinfo(&text_info);
        screenheight = text_info.screenheight;
        screenattr = text_info.attribute;
#else
        screenheight = 25;
        screenattr = 7;
#endif

        /* enlarge windows if needed */
        extra = ((screenheight-25) / 2) - 1;

        if (extra > 1)
        {
                recv_window.y2 += extra;
                chatin_window.y2 += extra;
                send_window.y1 += extra+1;
                send_window.y2 += extra+extra+1;
                chatout_window.y1 += extra+1;
                chatout_window.y2 += extra+extra+1;
                option_window.y2 += extra+extra+1;
                batch_window.y1 += extra+extra+1;
                batch_window.y2 += extra+extra+1;
        }

        textattr( WHITE );
        for (i=3; i<screenheight; ++i)
        {
                gotoxy(1,i+DISP_POS-2);
                clreol();
        }

        frame_window(&version_window);
        frame_window(&batch_window);
        frame_window(&recv_window);
        frame_window(&send_window);
        frame_window(&option_window);

        textattr( WS.Option.MainAttr );

        CENTER_TITLE(option_window.x1,
                     option_window.x2,
                     option_window.y2+DISP_POS-2,
                     TX_TOABORT);

        if (!local_userid())
                textattr( BLINK+WS.Option.MainAttr );

        CENTER_TITLE(batch_window.x1,
                     batch_window.x2,
                     batch_window.y2+DISP_POS-2,
                     local_userid()?
                                TX_TOCHAT : TX_NOCHAT);

        select_version();
}

/* -------------------------------------------------------------------- */

void pascal prepare_chat(void)
{
        if (WS.chat_active)
                return;

        select_window(NULL);
        frame_window(&chatin_window);
        frame_window(&chatout_window);
        --chatin_window.x2;
        --chatout_window.x2;
        WS.chat_active = 1;

        select_window(&chatout_window);
        textattr( WS.Option.MainAttr );
        cprintf(TX_CHATINS);
}


/* -------------------------------------------------------------------- */

void pascal display_chatin(char *s)
{
        window_definition *prev;

        /* chat only in full screen mode */
        if (!WS.Option.FullDisplay)
                return;

        prev = current_window;
        prepare_chat();

        select_window(&chatin_window);
        while (*s)
        {
                if (*s == 27)
                        close_chat();
                else
                if ((*s != 7) || !WS.Option.DisableCtlG)
                        putch(*s);
                ++s;
        }
        select_window(prev);
}

/* -------------------------------------------------------------------- */

/* append single character to chatout queue and display on local window */

void pascal add_chatout(register char c)
{
        register int i;
        register char *s;
        i = 1;
        s = WS.chatout.text;
        while (*s)
        {
                if (++i >= CHAT_MAX_LENGTH)
                        return;
                ++s;
        }
        *s++ = c;
        *s = 0;
        putch(c);
}

/* -------------------------------------------------------------------- */

/*
 * add chacter to outbound chat que and display locally
 * perform word-wrap, if any
 */

void pascal display_chatout(register char c)
{
        static int chatcol;

        if (!WS.Option.FullDisplay)
                return;
        if (!PRIVATE.remote_ready.final_ready)
                return;

        /* initiate chat only if registered */
        if (!WS.chat_active && !local_userid())
                return;

        /* prepare the chat display, if needed */
        prepare_chat();
        select_window(&chatout_window);

        /* append the character to the chat output queue */
        /* perform word-wrap and other special functions */
        ++chatcol;
        add_chatout(c);
        if (wherex() == 1)
                chatcol = 0;

        switch (c)
        {
        case '\r':
                /* translate cr into cr/lf */
                add_chatout('\n');
                chatcol = 0;
                break;
        case 7:
                /* these characters don't move the cursor */
                --chatcol;
                break;
        case 8:
                /* make backspace destructive */
                add_chatout(' ');
                add_chatout(8);
                --chatcol;
                if (chatcol) --chatcol;
                break;
        case 27:
                /* close down chat when ESC key is pressed */
                chatcol = 0;
                close_chat();
                break;

        case ' ':
                /* word wrap on spaces in last few columns of window */
                if (chatcol > (CHAT_WIDTH-4))
                {
                        add_chatout('\r');
                        add_chatout('\n');
                        chatcol = 0;
                }
                break;
        }
}

/* -------------------------------------------------------------------- */

void pascal close_chat(void)
{
        if (!WS.chat_active)
                return;
        if (!WS.Option.FullDisplay)
                return;

        select_window(NULL);
        frame_window(&option_window);
        ++chatin_window.x2;
        ++chatout_window.x2;
        WS.chat_active = 0;

        display_settings();
}

/* -------------------------------------------------------------------- */

void pascal close_display(void)
{
        if (WS.Option.FullDisplay)
        {
                window(1,1,80,screenheight);
                gotoxy(1,screenheight+DISP_POS-2);
                textattr(screenattr);
                clreol();
        }

        if (!local_userid() && ((bios_clock() & 3) == 3))
        {
                delay(3000);
                usage_registration();
                delay(6000);
                newline();
        }

        directvideo = 0;
}


/* -------------------------------------------------------------------- */

void pascal select_recv(void)
{
        if (WS.Option.FullDisplay)
        {
                select_window(&recv_window);
                clreol();
        }
        else {
                if (WS.send_expected)
                        cprintf("\r%39s\r","");
                else {
                        putch('\r');
                        clreol();
                }
        }
}


/* -------------------------------------------------------------------- */

void pascal select_send(void)
{
        if (WS.Option.FullDisplay)
        {
                select_window(&send_window);
                if (WS.Option.Debug>2)
                        newline();
                clreol();
        }
        else {
                if (WS.Option.Debug>2)
                        newline();
                if (WS.receive_expected)
                        gotoxy(40,wherey());
                else
                        putch('\r');
                clreol();
        }
}


/* -------------------------------------------------------------------- */

void pascal select_version(void)
{
        select_window(&version_window);
}


/* -------------------------------------------------------------------- */

void pascal select_option(void)
{
        select_window(&option_window);
}


/* -------------------------------------------------------------------- */

/* display newline to console */

void pascal newline(void)
{
        putch('\r');
        putch('\n');
}


/* -------------------------------------------------------------------- */

void pascal title_window(window_definition *wd)
{
        int x,y;

        if (!WS.Option.FullDisplay)
        {
                cprintf("%s\r\n",wd->title);
                return;
        }

        x = wherex();
        y = wherey();
        window(1,1,80,screenheight);

        textattr( WS.Option.TitleAttr );
        CENTER_TITLE(wd->x1,wd->x2,wd->y1+DISP_POS-2,wd->title);

        select_window(current_window);
        gotoxy(x,y);
}


/* -------------------------------------------------------------------- */

void pascal frame_window(window_definition *wd)
{
        #define TOPLEFT    213
        #define TOPRIGHT   184
        #define BOTLEFT    192
        #define BOTRIGHT   217
        #define LEFTVER    179
        #define RIGHTVER   179
        #define TOPHOR     205
        #define BOTHOR     196
        int i;

        wd->active = 1;
        window(wd->x1+1,wd->y1+DISP_POS-1,wd->x2-1,wd->y2+DISP_POS-3);
        textattr( WS.Option.WindowAttr );
        clrscr();

        window(1,1,80,screenheight);

        textattr( WS.Option.BorderAttr );
        gotoxy(wd->x1,wd->y1+DISP_POS-2);
        putch(TOPLEFT);
        for (i=wd->x1+1; i<=wd->x2-1; i++)
                putch(TOPHOR);
        putch(TOPRIGHT);

        textattr( WS.Option.TitleAttr );
        CENTER_TITLE(wd->x1,wd->x2,wd->y1+DISP_POS-2,wd->title);

        textattr( WS.Option.BorderAttr );
        gotoxy(wd->x1,wd->y2+DISP_POS-2);
        putch(BOTLEFT);
        for (i=wd->x1+1; i<=wd->x2-1; i++)
                putch(BOTHOR);
        putch(BOTRIGHT);

        for (i=wd->y1+1; i<=wd->y2-1; i++)
        {
                gotoxy(wd->x1,i+DISP_POS-2);
                putch(LEFTVER);
                gotoxy(wd->x2,i+DISP_POS-2);
                putch(RIGHTVER);
        }

        wd->cux = 1;
        wd->cuy = 2; /* wd->y2-wd->y1-1; */

        title_window(wd);
}


/* -------------------------------------------------------------------- */

void pascal select_window(window_definition *wd)
{
        if (!WS.Option.FullDisplay || WS.Option.TermMode || !(wd->active))
                return;

        if (current_window)
        {
                current_window->cux = wherex();
                current_window->cuy = wherey();
        }

        current_window = wd;
        if (wd)
        {
                window(wd->x1+2,wd->y1+DISP_POS-1,
                       wd->x2-1,wd->y2+DISP_POS-3);
                textattr( WS.Option.WindowAttr );
                gotoxy(wd->cux,wd->cuy);
        }
}


/* -------------------------------------------------------------- */

void pascal usage_pause()
{
        clock_t timeout;
        #define PAUSE_TIMEOUT 30000

        cprintf(TX_HITENTER);

        timeout = SET_TIMER(PAUSE_TIMEOUT);
        while (!TIMER_UP(timeout))
        {
                if (bioskey(1))
                {
                        int c = bioskey(0);
                        switch (c & 0xff)
                        {
                        case 3:
                                exit(99);

                        case '\r':
                        case '\n':
                                return;
                        }
                }
        }
}

/* -------------------------------------------------------------- */

void pascal usage_basic(void)
{
        cprintf(TX_BASICUSAGE);
        usage_pause();
}

/* -------------------------------------------------------------- */

void pascal usage_advanced(void)
{
#ifdef FULLHELP
        cprintf("\r\nAdvanced Options:"
                "\r\n"
                "\r\n  -@fname  Uses fname as an alternate configuration file."
                "\r\n  -!       Force remote to use local -S -W -HX -R settings."
                "\r\n  -A       Disable transmission of ACK codes after each block."
                "\r\n  -Bbaud   Open COM port at 300..115200 (default=current port speed)."
                "\r\n  -C       Disable carrier detect checking."
                "\r\n  -CB      Define border color in full screen mode."
                "\r\n  -CG      Define progress graph color in full screen mode."
                "\r\n  -CM      Define main screen color in full screen mode."
                "\r\n  -CT      Define window title color in full screen mode."
                "\r\n  -CW      Define window contents color in full screen mode."
                "\r\n  -Ebaud   Effective modem-to-modem baud rate (default=current -B setting)."
                "\r\n  -FC      Force CTS handshake even if CTS is initially inactive."
                "\r\n  -FTn     Set 16550 fifo threshold to n (1,4,8,14; default=4)."
                "\r\n  -HC      Disable CTS handshake."
                "\r\n  -HR      Disable RTS handshake in -HS mode."
                "\r\n  -HS      Handshake Slow (send XOFF and/or lower RTS during disk I/O)."
                "\r\n  -HX      Disable XON/XOFF handshake."
                "\r\n  -Imeth   Idle time method (0:none, 1:KB bios, 2:DV, 3:DDOS, 4:WIN/OS2)."
                "\r\n  -K       Keep partial files from aborted transfers."
                "\r\n  -LFfname Write DSZLOG to specified filename."
                "\r\n");
        usage_pause();

        cprintf("\r\nAdditional Advanced Options:"
                "\r\n"
                "\r\n  -O       Allow receive files to overwrite existing files."
                "\r\n  -N!      Do not force remote to use local settings (opposite of -!)."
                "\r\n  -N5      Disable 16550 buffering logic."
                "\r\n  -NB      Disable file buffering."
                "\r\n  -NC      Disable Dynamic-Code-Substitution(tm) logic."
                "\r\n  -NF      Disable Full screen status display."
                "\r\n  -NG      Disable ^G (BELL CODE) in chat mode."
                "\r\n  -NK      Do not keep aborted files (opposite of -K)."
                "\r\n  -NM      Enable Minimal-Blocks(tm) logic."
                "\r\n  -NOdlst  Check dirs contained in 'dlst' for upload duplication."
                "\r\n  -NT      Stamp current file time instead of original file time."
                "\r\n  -NU      Block uploads (incoming files) while sending files."
                "\r\n  -NV      Disable direct Video for DesqView/DoubleDOS/etc."
                "\r\n  -Pport   Use standard COM port 1..8 (default=1)."
                "\r\n  -PBbase  Set non-standard COM port base address (decimal or $hex)."
                "\r\n  -PIirq   Set non-standard COM port IRQ level (1-15)."
                "\r\n  -R       Resume aborted transfer (requires -K -O)."
                "\r\n  -Ssize   Sets transmit block size 2..4096 (default=1024)."
                "\r\n  -T       Activate a 'mini terminal' mode prior to starting the transfer."
                "\r\n  -Udir    Destination directory for received files."
                "\r\n  -Wwino   Number of blocks allowed without ACK 0-1000."
                "\r\n");
        usage_pause();
#endif

        cprintf(TX_EXAMPLES);
        usage_pause();
}

/* -------------------------------------------------------------- */

void pascal usage_registration(void)
{
        if (local_userid())
                cprintf(TX_THANKYOU);
        else {
                cprintf(TX_UNREG1);
#ifdef LANG_GERMAN
                usage_pause();
                cprintf(TX_UNREG2);
#endif
        }
}

/* -------------------------------------------------------------- */

void pascal usage_license(void)
{
        newline();
        usage_registration();
        cprintf(TX_CONTACT);
}

/* -------------------------------------------------------------- */

/* display a message with variable args */
void disp_message(char *fmt, ...)
{
        char buffer[200];
        va_list argptr;
        va_start(argptr, fmt);
        vsprintf(buffer, fmt, argptr);
        va_end(argptr);
        cprintf(buffer);
}

/* display and log error message */
void disp_error(char *fmt, ...)
{
        char buffer[200];
        va_list argptr;
        __emit__((char)0x9c);   /* pushf */
        va_start(argptr, fmt);
        vsprintf(buffer, fmt, argptr);
        va_end(argptr);
        cprintf(buffer);
        log_error(buffer);
        __emit__((char)0x9d);   /* popf */
}

/* log error message */
void log_error(char *fmt, ...)
{
        char *logfile;
        int fd;
        char buffer[200];
        time_t timer;
        struct tm *tblock;
        va_list argptr;

        logfile = getenv("HSERR");
        if ((logfile == 0) || (*logfile == 0))
                return;

        ComIoStart(51);

        if (access(logfile,0))
                fd = _creat(logfile,0);
        else
                fd = _open(logfile,O_RDWR);

        if (fd > 0)
        {
                lseek(fd,0,SEEK_END);

                if (*fmt != '\r')
                {
                        /* log the date and time of day  */
                        timer = time(NULL);
                        tblock = localtime(&timer);
                        sprintf(buffer,"%02d-%02d-%02d %02d:%02d:%02d ",
                                tblock->tm_mon+1,
                                tblock->tm_mday,
                                tblock->tm_year,
                                tblock->tm_hour,
                                tblock->tm_min,
                                tblock->tm_sec);
                        _write(fd,buffer,strlen(buffer));
                }

                va_start(argptr, fmt);
                vsprintf(buffer, fmt, argptr);

                _write(fd,buffer,strlen(buffer));
                close(fd);
        }

        ComIoEnd(52);
}


/* -------------------------------------------------------------- */

/* display program usage instructions and terminate execution */

void pascal usage(char *why, char *par)
{
        char message[80];
        directvideo = 0;

        cprintf("\r\n%s\r\n",version);
        identify_user();

        sprintf(message,why,par);
        log_error(TX_USAGEERR,message);

        cprintf(TX_ERROR,message);

        usage_basic();
        usage_advanced();
        usage_license();
}


/* -------------------------------------------------------------------- */

/* display file progress bar graph */

void pascal batch_bargraph(
                        long current,
                        long total,
                        unsigned togo,
                        int graph_width,
                        char *suffix)
{
        int i,frac;
        char *s,graph[80],remain[40];

        /* disable bragraph in line mode */
        if (!WS.Option.FullDisplay)
                return;

        cprintf("\r\n");

        if ((total > 0) && (togo > 0))
        {
                sprintf(remain," %s%s",sectomin(togo),suffix);

                /* leave room for estimated time remaining */
                graph_width -= 1+strlen(remain);

                /* calculate seconds remaining in transfer */
                /* create image of the bargraph */
                frac = (current*graph_width) / total;
                        /* conversion loss warning ok */

                for (s=graph,i=0; i<graph_width; i++)
                        if (i < frac)
                                *s++ = 'Û';
                        else
                                *s++ = '°';
                *s = 0;

                textattr( WS.Option.GraphAttr );
                cprintf(graph);
                textattr( WS.Option.WindowAttr );
                cprintf(remain);

        }

        /* restore original cursor */
        clreol();
        gotoxy(1,wherey()-1);
}

/* -------------------------------------------------------------- */

/* report batch status and combined thruput */

void pascal report_combined()
{
        window_definition *prev;
        unsigned cps;
        long send_tot,recv_tot;
        long send_cur,recv_cur;
        unsigned send_togo,recv_togo;

        if ((WS.tx_start == 0) && (WS.rx_start == 0))
                return;

        if (WS.begin_time == 0)
        {
                if (WS.tx_start)
                {
                        WS.begin_time = WS.tx_start;
                        if (WS.rx_start && (WS.rx_start < WS.tx_start))
                                WS.begin_time = WS.rx_start;
                }
                else
                        WS.begin_time = WS.rx_start;
        }

        if (!WS.Option.FullDisplay)
                return;

        /* find total bytes in send and receive batches */
        send_tot = WS.send_bytes-WS.send_skip_total;
        recv_tot = WS.recv_bytes-WS.recv_skip_total;

        /* find current position in send and receive batches */
        send_cur = WS.send_total+WS.send_current-WS.send_skip_total;
        recv_cur = WS.recv_total+WS.recv_current-WS.recv_skip_total;

        /* find combined cps for the two batches */
        cps = calculate_cps(WS.begin_time,send_cur+recv_cur);

        prev = current_window;
        select_window(&batch_window);
        gotoxy(1,1);
        cprintf(TX_COMBINED,
                        send_cur+recv_cur,
                        send_tot+recv_tot,
                        sectomin(TIMER_SECS(WS.begin_time)),
                        cps);
        clreol();

        /* base combined time remaining on the batch with the longest
           transfer time remaining */
        send_togo = (WS.tx_cps)? (send_tot-send_cur) / WS.tx_cps : 0;
        recv_togo = (WS.rx_cps)? (recv_tot-recv_cur) / WS.rx_cps : 0;

        if (recv_togo > send_togo)
                batch_bargraph(recv_cur, recv_tot, recv_togo, 77,TX_REMAINING);
        else
                batch_bargraph(send_cur, send_tot, send_togo, 77,TX_REMAINING);

        if (prev != NULL)
                select_window(prev);
}

/* -------------------------------------------------------------------- */

/* display file progress bar graph and update combined batch status */

void pascal file_bargraph(long current, long total, unsigned cps)
{
        if (cps == 0)
                cps = WS.Option.EffSpeed / 11;

        batch_bargraph( current,
                        total,
                        (total-current) / cps,
                        FILES_WIDTH, "");

        report_combined();
}

/* -------------------------------------------------------------------- */

/* display file send/receive banner */

void pascal display_file_banner(char *mode,
                         char *fname,
                         unsigned blocks,
                         long bytes)
{
        char buf[80];
        char log[80];
        int i;

        sprintf(buf,TX_BANNER1,blocks,bytes);
        sprintf(log,TX_BANNER2,mode,fname,buf);

        if (WS.Option.FullDisplay)
        {
                i = strlen(buf)+strlen(fname);
                if (i > FILES_WIDTH)
                        cprintf("%s\r\n  %s\r\n",fname,buf);
                else {
                        cprintf("%s%s",fname,buf);
                        if (i < FILES_WIDTH)
                                newline();
                }
        }
        else
                cprintf("\r\n%s",log);

        log_error(log);
}

/* -------------------------------------------------------------- */

void pascal display_settings(void)
{
        char temp1[100];
        char temp2[100];

        if (WS.chat_active)
                return;

        sprintf(temp1,TX_NWINDOW,
                WS.Option.MaxWind,
                WS.Option.BlockSize,
                WS.Option.XonHandshake? TX_NXONXOFF:"",
                WS.Option.CtsHandshake? TX_NCTS:"",
                WS.Option.RtsHandshake? TX_NRTS:"",
                WS.Option.SlowHandshake? TX_NSLOW:"",
                WS.Option.AlternateDle? TX_NALT:TX_NOLD);

        sprintf(temp2,TX_NREMVER,
                WS.remote_version,
                WS.remote_userid,
                WS.hacked_remote? '*':' ',
                local_userid());

        select_option();

        if (WS.Option.FullDisplay)
        {
                clrscr();
                gotoxy(1,1);

                display_comport(0);

                cprintf(TX_REMSERIAL);
                if (WS.remote_userid)
                        cprintf("%05u",WS.remote_userid);
                if (!local_userid())
                        textattr( BLINK+WS.Option.WindowAttr );
                if (!WS.remote_userid)
                        cprintf(TX_NONE);
                if (WS.hacked_remote)
                        cprintf("*");
                textattr( WS.Option.WindowAttr );
                            
                cprintf(TX_REMVER,      WS.remote_version);
/*
cprintf("\r\n   Alternate DLE: %s",WS.Option.AlternateDle? "ON":"OFF");
*/
                cprintf(TX_ACKWIN);
                cprintf(WS.Option.DisableAck? TX_NONE:"%u",WS.Option.MaxWind);

                cprintf(TX_BLOCKSIZE,   WS.Option.BlockSize);
                cprintf(TX_XONXOFF,     WS.Option.XonHandshake?   TX_ON:TX_OFF);
                cprintf(TX_CTSHS,       WS.Option.CtsHandshake?   TX_ON:TX_OFF);
                cprintf(TX_RTSHS,       WS.Option.RtsHandshake?   TX_ON:TX_OFF);
                cprintf(TX_SLOWHS,      WS.Option.SlowHandshake?  TX_ON:TX_OFF);
                cprintf(TX_RESUMEOP,    WS.Option.ResumeVerify?   TX_ON:TX_OFF);
                cprintf(TX_KEEP,        WS.Option.KeepAborted?    TX_ON:TX_OFF);
                cprintf(TX_ALLOWOV,     WS.Option.AllowOverwrite? TX_ON:TX_OFF);
        }
        else {
                cprintf(temp1);
                cprintf(temp2);
        }

        log_error(temp1);
        log_error(temp2);
}

/* -------------------------------------------------------------------- */

void pascal report_rx_error( char *what )
{
        char message[80];

        select_window(&recv_window);
        if (WS.receive_errors && WS.Option.FullDisplay)
                gotoxy(1,wherey()-1);

        ++WS.receive_errors;

        sprintf(message,TX_RXERR1,
                WS.receive_errors,what,PRIVATE.last_received);
        log_error(TX_RXERR2,message);

        message[FILES_WIDTH-1] = 0;
        strcat(message,"\r\n");
        PRECV(message);
}

/* -------------------------------------------------------------------- */

void pascal report_tx_error( char *what, block_number block )
{
        char message[80];

        select_window(&send_window);
        if (WS.transmit_errors && WS.Option.FullDisplay)
                gotoxy(1,wherey()-1);

        ++WS.transmit_errors;

        sprintf(message,TX_TXERR1,
                WS.transmit_errors, what, (int) block,PRIVATE.last_sent);
        log_error(TX_TXERR2,message);

        message[FILES_WIDTH-1] = 0;
        strcat(message,"\r\n");
        PSEND(message);
}

/* -------------------------------------------------------------------- */

void pascal display_warning( char *what )
{
   PVERSION(TX_WARNING1,what);
   if (!WS.Option.FullDisplay)
        newline();
   log_error(TX_WARNING2,what);
}

/* -------------------------------------------------------------------- */

void pascal display_incoming_files()
{
        sprintf(recv_window_title,TX_INCOMING,
                WS.receive_expected,
                WS.receive_expected != 1? TX_PLURAL:TX_SINGULAR,
                WS.recv_bytes);
        title_window(&recv_window);
}

/* -------------------------------------------------------------------- */

void pascal display_outgoing_files()
{
        sprintf(send_window_title,TX_OUTGOING,
                WS.send_expected,
                WS.send_expected != 1? TX_PLURAL:TX_SINGULAR,
                WS.send_bytes);
        title_window(&send_window);
}

/* -------------------------------------------------------------- */

/*********************************************************************
          * APPLICATION-SPECIFIC DISPLAYS (NOT PART OF HDK) *
 *********************************************************************/

void pascal identify_user()
{
        unsigned id;

        id = local_userid();
        if (id)
                cprintf(TX_SERNO,id);
        else
                cprintf(TX_NOSERNO);
}


/* -------------------------------------------------------------- */

void pascal echo_command_line(int argc, char **argv)
{
        int i;
        char temp[200];

        strcpy(temp,TX_CMDLINE);
        for (i=1; i<argc; i++)
        {
                strcat(temp," ");
                strcat(temp,argv[i]);
        }

        select_version();
        newline();
        disp_message(temp);
        if (!WS.Option.FullDisplay)
                newline();

        log_error("%s\r\n",temp);
}


/* -------------------------------------------------------------------- */

void pascal display_comport(int uart)
{
        POPTION(WS.Option.FullDisplay?
                TX_FSOPEN : TX_NFSOPEN,
                        WS.Option.ComBase+WS.Option.ComIrq?
                                'x' : '0'+WS.Option.ComPort,
                        WS.Option.ComSpeed);

#ifdef DEBUG
        if (uart)
        {
                disp_error("   COM port base: %03x\r\n",ComLL.ComBase);
                disp_error("   COM port IRQ#: %d\r\n",ComLL.ComIrq);
                disp_error("   COM port type: %s\r\n",ComLL.Is16550? "16550":"8250");
                if (WS.Option.FifoThresh)
                        disp_error("  FIFO threshold: %d\r\n",WS.Option.FifoThresh);
                disp_error("   COM prev. LCR: %02x\r\n",old_LCR);
                disp_error("   COM prev. MCR: %02x\r\n",old_MCR);
                disp_error("   COM prev. IER: %02x\r\n",old_IER);
                disp_error("   COM prev. PIC: %02x\r\n",old_PIC);
                disp_error("   COM prev. FCR: %02x\r\n",old_FCR);
        }
#endif

}


/* -------------------------------------------------------------- */

int pascal terminal_mode(void)
        /* returns 0 to perform file transfer, non-0 to exit program */
{
        int ppc,pc,c;

        cprintf(TX_TERMMODE);
        pc = 0;
        c = 0;

        /* monitor carrier detect */
        while (!ComCarrierLost() && !WS.cancel_link)
        {

                /* service receive characters */

                while (ComReadPending())
                {
                        ppc = pc;
                        pc = c;
                        c = ComReadChar();

                        /* recognize HS/Link startup sequence */
                        if ((ppc == 'S') && (pc == '*') && (c == 2))
                        {
                                cprintf(TX_AUTOSTART);
                                return 0;
                        }

                        putch(c);
                }

                /* service local keyboard during 'idle' process */
                ComIdle(701);
        }

        return 1;       /* exit due to carrier loss or cancel */
}

/* -------------------------------------------------------------- */

#pragma argsused
void pascal filter_rx_block( char *fname, long offset, unsigned size, char *data )
{
}

