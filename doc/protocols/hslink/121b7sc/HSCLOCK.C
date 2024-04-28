
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */

#include <time.h>

/*********************************************************************
         * TIME MEASUREMENT SERVICES PROVIDED BY APPLICATION *
 *********************************************************************/


clock_t pascal bios_clock(void)
        /* return BIOS ticks-since-midnight value */
{
        return (*((clock_t far*)0x46c));
}

