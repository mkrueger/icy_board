
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * hsmem.c - HS/Link, memory management services
 *
 */

#include <stdlib.h>
#include <stdio.h>
#include <dos.h>
#include <conio.h>

#ifdef __TURBOC__
#include <alloc.h>
#include <mem.h>
#endif

#include <hdk.h>

#include "hsl.h"

/* define minimal stack here, enforce larger one below */
/************************
unsigned _stklen = 256;
*************************/


/* -------------------------------------------------------------- */

/* determine memory available to hslink engine */
/* reserves untouched core for use by the stack */

unsigned pascal mem_avail()
{
        char unused[STACKSIZE];
        void *buf;
        unsigned size;
        unsigned step;
        #define MALLOC_MAX      40960
        #define MALLOC_STEP     1024
        #define MALLOC_SLOP     (size/8)

        unused[STACKSIZE-1] = 0;

        /* There appears to be no standard library call to return the
           largest available block on the heap!  This logic attempts to
           allocate various size blocks until the largest available
           block size is found.  THERE MUST BE A BETTER WAY! */

        /* step down from first available size */
        size = MALLOC_MAX;
        step = MALLOC_STEP;
        for (;;)
        {

                if (size <= step)
                {
#ifdef DEBUG
        printf("mem_avail(2) not enough memory\n");
#endif
                        return 0;
                }

                buf = malloc(size+MALLOC_SLOP);
#ifdef DEBUG
        printf("mem_avail(1) buf=%04x size=%u\n",buf,size);
#endif
                if (buf)
                {
                        free(buf);
                        return size;
                }
                size = size-step;
        }
}


/* allocate a block of memory and initialize it to zeros */
void* pascal mem_alloc(unsigned size)
{
        void *block;
        char unused[STACKSIZE];

        unused[STACKSIZE-1] = 0;
        block = malloc(size);

#ifdef DEBUG
        printf("mem_alloc: size=%u block=%04x core=%u\n",size,block,coreleft());
#endif

        if (block == 0)
        {
                disp_error("Not enough memory for HSLINK! %u\r\n",size);
                return 0;
        }

        mem_clear(block,size);
        return block;
}

/* release a previously allocated block of memory */
void pascal mem_free(void *block)
{
#ifdef DEBUG
        printf("mem_free: block=%04x initial core=%u",block,coreleft());
#endif
        if (block)
                free(block);
#ifdef DEBUG
        printf(" final core=%u\n",coreleft());
#endif
}

/* zero a block of memory */
void pascal mem_clear(void *block, unsigned size)
{
#ifdef DEBUG
        printf("mem_clear: block=%04x size=%u\n",block,size);
#endif
        setmem(block,size,0);
}


/* copy non-overlapping blocks of memory */
void pascal mem_copy(uchar *dest, uchar *src, unsigned size)
{
#ifdef __TURBOC__
        _DI = FP_OFF(dest);
        _SI = FP_OFF(src);
        _ES = _DS;
        _CX = size;
        asm CLD;
        asm REP MOVSB;
#else
        while (size--)
                *dest++ = *src++;
#endif
}

