
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */

/*
 * hsver.c - Sets application version and EXE file crc flag
 *             
 */

#define APPLICATION_VERSION "HS/Link 1.21·B7"

#include <stdio.h>
#include <string.h>
#include <io.h>

#include "\hdk\hspriv.h"

#define extern
#include "hstext.h"
#include "hsl.h"
#undef extern

#include "hspcrc.h"

void pascal set_version(void)
{
        /* replace HDK's version with our own */
        strcpy(sender_name,APPLICATION_VERSION);

#ifdef FOSSIL
        sprintf(version,"FOSSIL %s (%s) - %s",
#else
        sprintf(version,"%s (%s) - %s",
#endif
                sender_name,
                __DATE__,
                hdk_copyright);
}

