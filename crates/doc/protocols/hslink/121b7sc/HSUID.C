
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * hsuid.c - determine and verify user id numbers for HS/LINK protocol.
 *
 */

#include <stdlib.h>
#include <stdio.h>
#include <io.h>
#include <conio.h>
#include <ctype.h>
#include <dos.h>
#include <string.h>
#include <fcntl.h>
#include <sys\\types.h>
#include <sys\\stat.h>

#include "\hdk\hspriv.h"
#include <hdk.h>

#define extern
#include "hsuid2.h"
#undef extern
#include "hstext.h"

/* -------------------------------------------------------------- */

/* private declarations */

static void pascal determine_local_userid(char *exe);


/* -------------------------------------------------------------- */

/* return local userid - complicated to deter hacking */

user_number pascal local_userid(void)
{
        if (proper_CRC == CRCOF(text_CRC))
                return ~lvp.uid;
        else
                return 0;
}


/* -------------------------------------------------------------- */

int pascal brand_detected(char *exe, uchar *par)
{
        char *s;
        int match;
        char key[80];
        static char key1[] = {~'(',~'B',~'R',~'A',~'N',~'D',~')',0};
        static char key2[] = {~'$',~'B',~'R',~'A',~'N',~'D',~'$',0};
        static char key3[] = {~'R',~'E',~'G',~'I',~'S',~'T',~'E',~'R',0};

        strcpy(key,(char*)par);
        for (s=key; *s; ++s)
                *s = ~toupper(*s);

        match =
                (strcmp(key,key1) == 0) || (strcmp(key,key2) == 0) ||
                (strcmp(key,key3) == 0);

        if (!match)
                determine_local_userid(exe);

        return match;
}


/* -------------------------------------------------------------- */

void pascal brand_new_user(char *exe, uchar *uid, uchar *pw)
{
        int fd;

        /* encode serial_brand_packet at the end of the EXE file */
        /* build the lvp.pwp */
        mem_clear(&lvp,sizeof(lvp));
        strcpy((char*)lvp.pwp.epw,(char*)pw);
        lvp.pwp.pid = ((long)(~atoi((char*)uid))) * (long)PRIME_KEY;

        /* calculate the lvp crc */
        lvp.crc = ~CRCOF(lvp.pwp);

#ifdef DEBUG
DBPF("\nbefore scramble:");
DUMP_BLOCK((uchar *)&lvp.pwp,sizeof(lvp.pwp));
#endif

        /* scramble the lvp.pwp */
        CYPHER_LVP;

#ifdef DEBUG
DBPF("\r\ncrc=%08lx, new LVP after scramble:",lvp.crc);
DUMP_BLOCK((uchar *)&lvp,sizeof(lvp));
#endif

        fd = _open(exe,O_RDWR);
        if (fd > 0)
        {
                (void)lseek(fd,-(long)sizeof(lvp),SEEK_END);
                (void)_write(fd,&lvp,sizeof(lvp));
                _close(fd);
        }

        mem_clear(&lvp,sizeof(lvp));

        determine_local_userid(exe);
}

/* -------------------------------------------------------------- */

/* calculate exe file crc and load the encoded registration packet */

void pascal determine_local_userid(char *exe)
{
        int fd;
        uchar *buf;
        int j;
        CRC_type crc1;
        CRC_type *hcrc;
        unsigned crcpos;

#ifdef __TURBOC__
        fd = _open(exe,O_RDONLY | O_DENYNONE);
#else
        fd = _open(exe,O_RDONLY);
#endif
        if (fd < 1)
                fd = _open(exe,O_RDONLY);
        if (fd < 1)
        {
                disp_error(TX_CANTOPENEXE,exe,_doserrno);
                exit(99);
        }

#if (sizeof(PRIVATE) < TEXT_CRC_SIZE)
        DBPF("TEXT_CRC_SIZE %04x TOO BIG!  MAX=%04x WS=%04x\r\n",
                                TEXT_CRC_SIZE,sizeof(PRIVATE),sizeof(WS));
        exit(1);
#endif

        /* read serial number packet from EXE file */
        crcpos = lseek(fd,0,SEEK_END) - sizeof(lvp);
        lseek(fd,(long)crcpos,SEEK_SET);
        j = _read(fd,&lvp,sizeof(lvp));
#ifdef DEBUG
DBPF("\r\nj=%d lvp from exe file:",j);
DUMP_BLOCK((uchar *)&lvp,sizeof(lvp));
#endif

        /* load proper_CRC for the file */
        buf = (uchar *)&PRIVATE;
        lseek(fd,0,SEEK_SET);
        j = _read(fd,buf,EXE_HEADER_SIZE);
        crcpos -= j;
        hcrc = &buf[EXE_PCRC];
        proper_CRC = *hcrc;

#ifdef DEBUG
DBPF("\r\ncrcpos=%u proper_CRC=%08lx j=%d exe header:",crcpos,proper_CRC,j);
DUMP_BLOCK((uchar *)&buf,EXE_HEADER_SIZE);
#endif

        /* determine exe file crc */
        text_CRC = 0;
        while (crcpos)
        {
                unsigned i;
                i = TEXT_CRC_SIZE;
                if (i > crcpos)
                        i = crcpos;
                crcpos -= i;
                i = _read(fd,buf,i);
                text_CRC += calculate_CRC(buf,i);
#ifdef DEBUG
DBPF("\r\ncrc block i=%d crcpos=%u tcrc=%08lx\r\n",i,crcpos,text_CRC);
#endif
        }

        mem_clear(buf,sizeof(PRIVATE));
        // mem_clear(buf,sizeof(WS));
        strcpy(PRIVATE.exe_path,exe);

#ifdef DEBUG
DBPF("pcrc=%08lx tcrc=%08lx\r\n",proper_CRC,text_CRC);
#endif
        _close(fd);

        /* unscramble the lvp.pwp */
        CYPHER_LVP;

#ifdef DEBUG
DBPF("\r\nlvp after decyphering:");
DUMP_BLOCK((uchar *)&lvp,sizeof(lvp));
#endif
        /* decode the user number */
        lvp.uid = lvp.pwp.pid / PRIME_KEY;      /* conversion loss warning ok */

        /* decode the password */
        buf = (uchar *)lvp.pwp.epw;
        crc1 = 0;
        while (*buf)
        {
                crc1 = crc1*36 + toupper(*buf) - '0';
                if (*buf > '9')
                        crc1 -= 7;
                ++buf;
        }

#ifdef DEBUG
DBPF("\ndecode: password=%s crc1=%08lx uid=%u proper_crc=%08lx\n",
                        lvp.pwp.epw,crc1,lvp.uid,proper_CRC);
#endif

        /* verify the lvp crc */
        if (lvp.crc != ~CRCOF(lvp.pwp))
                ++proper_CRC;

#ifdef DEBUG
DBPF("\ndecode(2): pcrc-1=%08lx\n",proper_CRC);
#endif
        /* verify that the password matches the userid */
        if (crc1 != ~CRCOF(lvp.uid))
                ++proper_CRC;

#ifdef DEBUG
DBPF("crc(lvp)=%08lx/%08lx crc(eid)=%08lx/%08lx\n",
        lvp.crc,~CRCOF(lvp.pwp),
        crc1,   ~CRCOF(lvp.uid));
DBPF("lvp.crc=%08lx, uid=%u id=%u FINAL pcrc=%08lx pwp:",
        lvp.crc,lvp.uid,~lvp.uid,proper_CRC);
DUMP_BLOCK((uchar *)&lvp.pwp,sizeof(lvp.pwp));
#endif
}


/* -------------------------------------------------------------- */

/* determine if a userid has been blocked; return 0 if it is ok */

int pascal blocked_userid(user_number *uid)
{
        switch (*uid)
        {

       /* these are known hacks or leaked/stolen registrations */
        case 623:       // "pirated by 30 sysops and users in 813 area code
        case 625:
        case 1906:      // fraud: neil livingston */
        case 2316:
        case 2317:
        case 2680:      // returned for refund: thomas fetherston
        case 2783:      // best products- not authorized
        case 32767:
        case 23456:
        case 12345:
                return 1;

        default:
//              /* anything over 8000 *MUST* be hacked as of 12/92 */
//              if (*uid > 8000)
//                      return 1;
//              else
                        return 0;
        }
}


