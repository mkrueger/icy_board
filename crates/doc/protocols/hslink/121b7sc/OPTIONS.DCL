
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */

/* generate pascal declarations for config variables and defaults */

#define IN_OPTIONS
#include <hsopt.h>
#define NULLSTR ""

const
   O_BANG: boolean = DEFAULT_BANG; /* priority */        D_BANG = DEFAULT_BANG;
   O_A:    boolean = DEFAULT_A;    /* disable_ack */     D_A = DEFAULT_A;
   O_B:    longint = DEFAULT_B;    /* comspeed */        D_B = DEFAULT_B;
   O_C:    boolean = DEFAULT_C;    /* require_carrier */ D_C = DEFAULT_C;
   O_CB:   word    = DEFAULT_CB;   /* border_attr */     D_CB = DEFAULT_CB;
   O_CG:   word    = DEFAULT_CG;   /* graph_attr */      D_CG = DEFAULT_CG;
   O_CM:   word    = DEFAULT_CM;   /* main_attr */       D_CM = DEFAULT_CM;
   O_CT:   word    = DEFAULT_CT;   /* title_attr */      D_CT = DEFAULT_CT;
   O_CP:   word    = DEFAULT_CP;   /* disp_pos */        D_CP = DEFAULT_CP;
   O_CW:   word    = DEFAULT_CW;   /* window_attr */     D_CW = DEFAULT_CW;
   O_D:    boolean = DEFAULT_D;    /* debug */           D_D = DEFAULT_D;
   O_E:    longint = DEFAULT_E;    /* effspeed */        D_E = DEFAULT_E;
   O_FC:   boolean = DEFAULT_FC;   /* force_CTS */       D_FC = DEFAULT_FC;
   O_FT:   integer = DEFAULT_FT;   /* FifoThresh */      D_FT: integer = DEFAULT_FT;
   O_HC:   boolean = DEFAULT_HC;   /* CTS_handshake */   D_HC = DEFAULT_HC;
   O_HR:   boolean = DEFAULT_HR;   /* RTS_handshake */   D_HR = DEFAULT_HR;
   O_HS:   boolean = DEFAULT_HS;   /* slow_handshake */  D_HS = DEFAULT_HS;
   O_HX:   boolean = DEFAULT_HX;   /* XON_handshake */   D_HX = DEFAULT_HX;
   O_I:    integer = DEFAULT_I;    /* idle_method */     D_I = DEFAULT_I;
   O_K:    boolean = DEFAULT_K;    /* keep_aborted */    D_K = DEFAULT_K;
   O_LF:   string  = DEFAULT_LF;   /* dszlog_file */     D_LF = DEFAULT_LF;
   O_N5:   boolean = DEFAULT_N5;   /* disable_16550 */   D_N5 = DEFAULT_N5;
   O_NA:   boolean = DEFAULT_NA;   /* alternateDle */    D_NA = DEFAULT_NA;
   O_NB:   boolean = DEFAULT_NB;   /* file_buffering */  D_NB = DEFAULT_NB;
   O_NC:   boolean = DEFAULT_NC;   /* code_subst  */     D_NC = DEFAULT_NC;
   O_NE:   integer = DEFAULT_NE;   /* max_error */       D_NE = DEFAULT_NE;
   O_NF:   boolean = DEFAULT_NF;   /* full_display */    D_NF = DEFAULT_NF;
   O_NM:   boolean = DEFAULT_NM;   /* minimal_blocks */  D_NM = DEFAULT_NM;
   O_NO:   string  = DEFAULT_NO;   /* dupe_checklist */  D_NO = DEFAULT_NO;
   O_NT:   boolean = DEFAULT_NT;   /* keep_time */       D_NT = DEFAULT_NT;
   O_NU:   boolean = DEFAULT_NU;   /* block_upload */    D_NU = DEFAULT_NU;
   O_NV:   boolean = DEFAULT_NV;   /* directvideo */     D_NV = DEFAULT_NV;
   O_O:    boolean = DEFAULT_O;    /* allow_overwrite */ D_O = DEFAULT_O;
   O_P:    integer = DEFAULT_P;    /* comport */         D_P = DEFAULT_P;
   O_PB:   word    = DEFAULT_PB;   /* port_base */       D_PB = DEFAULT_PB;
   O_PI:   integer = DEFAULT_PI;   /* port_irq */        D_PI = DEFAULT_PI;
   O_R:    boolean = DEFAULT_R;    /* resume_verify */   D_R = DEFAULT_R;
   O_S:    integer = DEFAULT_S;    /* block_size */      D_S = DEFAULT_S;
   O_U:    string  = DEFAULT_U;    /* receive_dir */     D_U = DEFAULT_U;
   O_W:    integer = DEFAULT_W;    /* maxwind */         D_W = DEFAULT_W;

