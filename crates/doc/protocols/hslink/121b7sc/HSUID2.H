
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */

/* size of CRC block */
#define TEXT_CRC_SIZE           0x2000

/* exe header size */
#define EXE_HEADER_SIZE         0x52

/* first position in header to modify */
#define EXE_FIRST               0x1e

/* position within header of proper_CRC value */
#define EXE_PCRC                0x3d

/* prime number used to encode user numbers */
#define PRIME_KEY 31327

/* position of branding package relative to EOF */
// #define BRAND_OFFSET (- ((long)sizeof(serial_brand_packet)) )

/* password packet within serial brand */
typedef struct {
        uchar epw[10];          /* encoded password */
        long pid;               /* user number * PRIME_KEY */
} password_packet;

/* the complete serial brand packet - encrypted in EXE file */
typedef struct {
        password_packet pwp;    /* encoded password/user number */
        user_number uid;        /* encoded user number */
        char username[45];
        char company[45];
        CRC_type crc;           /* pwp block check */
} serial_brand_packet;

extern serial_brand_packet lvp;

/* key string used to xor bytes in packet */
extern uchar jumble_key[]
#ifdef extern
        = {207,159,5,162,78,125,242,36,77,233,99,94,82,110,228,53,217,69,34,67}
#endif
        ;

/* (un)scramble the sequence of bytes in the serial brand packet */
#define CYPHER_LVP {                                            \
        int i;                                                  \
        uchar *s;                                               \
        s = (uchar *)&lvp;                                      \
        for (i=0; i<sizeof(lvp); ++i)                           \
                s[i] ^= jumble_key[i%20];                       \
}

int pascal brand_detected(char *exe, uchar *par);
void pascal brand_new_user(char *exe, uchar *uid, uchar *pw);


