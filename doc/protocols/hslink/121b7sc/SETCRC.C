
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * setcrc.c - determine HSLINK.EXE file crc and plug value into code at
 *            correct location.
 *
 * Access to this module must be controlled since it is able to
 * circumvent exe file tampering checks.
 *
 */

#include <stdlib.h>
#include <stdio.h>
#include <io.h>
#include <conio.h>
#include <ctype.h>
#include <dos.h>
#include <dir.h>
#include <mem.h>
#include <string.h>
#include <fcntl.h>
#include <sys\\stat.h>

#include "\hdk\hspriv.h"
#include <hdk.h>

#define extern
#include "hsuid2.h"
#undef extern

void pascal ComIoStart(int where) {}
void pascal ComIoEnd(int where) {}

CRC_type get_file_crc(char *fname)
{
        int fd;
        CRC_type text_CRC;
        long p;
                
        fd = _open(fname,O_RDWR | O_DENYNONE);
        if (fd > 0)
        {
                uchar *buf;
                int j;

                /* determine exe file crc */
                buf=calloc(1,TEXT_CRC_SIZE);
                if (buf == NULL)
                {
                        printf("malloc failure!\n");
                        return 0;
                }

                text_CRC = 0;
                p = lseek(fd,0,SEEK_END) - sizeof(lvp) - EXE_HEADER_SIZE;
                lseek(fd,0,SEEK_SET);
                _read(fd,buf,EXE_HEADER_SIZE);
                while (p)
                {
                        unsigned i;
                        i = TEXT_CRC_SIZE;
                        if (i > p)
                                i = p;
                        p -= i;
                        i = _read(fd,buf,i);
                        text_CRC += calculate_CRC(buf,i);
                }
                _close(fd);
                free(buf);
        }

        return text_CRC;
}


void initialize_lvp(char *fname)
{
        int fd;
        static char copyright[] =
                "\r\nCopyright 1991-1993 Samuel H. Smith"
                "\r\nLicensed Material - Property of Samuel H. Smith - All rights reserved"
                "\r\nP.O. BOX 4808, PANORAMA CITY CA, 91412"
                "\r\n(818) 891-4228"
                "\r\n";

                
        fd = _open(fname,O_RDWR | O_DENYNONE);
        if (fd > 0)
        {
                setmem(&lvp,sizeof(lvp),0);

                /* build the '0' serial number lvp.pwp */
                strcpy((char*)lvp.pwp.epw,"1Z12NEO");
                lvp.pwp.pid = ((long)(~atoi("0"))) * (long)PRIME_KEY;

                strcpy(lvp.username,"[UNREGISTERED]");
                strcpy(lvp.company,"Courtesy of The Tool Shop (818)891-1344");

                /* calculate the lvp crc */
                lvp.crc = ~CRCOF(lvp.pwp);

                /* scramble the lvp.pwp */
                CYPHER_LVP;

/*****
DBPF("\r\n lvp.crc=%08lx, LVP after scramble:",lvp.crc);
DUMP_BLOCK((char *)&lvp,sizeof(lvp));
DBPF(" \r\n");
******/
                /* append the lvp to the exe file */
                lseek(fd,0,SEEK_END);
                (void)_write(fd,copyright,strlen(copyright));
                (void)_write(fd,&lvp,sizeof(lvp));

                _close(fd);
        }
}


void insert_crc(char *fname, CRC_type pcrc)
{
        int fd;
        unsigned char header[EXE_HEADER_SIZE];
        CRC_type *hcrc;
        int i;

        fd = _open(fname,O_RDWR | O_DENYNONE);
        if (fd > 0)
        {
                _read(fd,header,sizeof(header));
                for (i=EXE_FIRST; i<EXE_HEADER_SIZE; i++)
                        header[i] = random(255);

                hcrc = &header[EXE_PCRC];
                *hcrc = pcrc;

                lseek(fd,0,SEEK_SET);
                _write(fd,header,sizeof(header));
                _close(fd);
        }
}


int main(int argc, char **argv)
{
        CRC_type crc,pcrc;

        if (argc != 2)
        {
                fprintf(stderr,"Usage: setcrc FILE.EXE\n");
                exit(1);
        }

        initialize_lvp(argv[1]);
        crc = get_file_crc(argv[1]);
        pcrc = CRCOF(crc);
        insert_crc(argv[1],pcrc);
        printf("   crc=%08lx  pcrc=%08lx\n",crc,pcrc);

        return 0;
}
