/*
                        Specific Qmail stuff
*/

#define  call   goto        /* :-))                                  */

#ifndef BYTE_DECL
typedef unsigned char byte ;
#define BYTE_DECL
#endif

/*--------- FILES RECEIVED in *.QWK.-- Warning : DOS is not case-sensitive */

#define MSG_FILE    "messages.dat" /* Message filename prepared by Qmail */
#define CNTRL_FILE  "control.dat"  /* List of conferences by Qmail       */
#define NEWFILES    "newfiles.dat" /* List of new files   by Qmail       */
#define WELCOME     "welcome"      /* 1st Screen of te BBS               */
#define WELCOMEG    "welcomeg"     /* 1st Screen, ansi mode              */
#define NEWS        "news"         /* news file, ascii mode              */
#define NEWSG       "newsg"        /* news file, ansi  mode              */
#define MSG_EXT     ".msg"         /* Extension of reply file            */

/*-------------------------------------------------------------------------*/

typedef
struct MsgHeaderType         /* RECEIVED MESSAGE HEADER STRUCTURE      */
       {
           byte Status        ,   /* ??? */
                NumMsg   [7 ] ,   /* Numero du message,envoi = conf !  */
                MsgDate  [8 ] ,   /* mm-dd-yy                          */
                MsgTime  [5 ] ,   /* HH:MM                             */
                ForWho   [25] ,   /* Destinataire                      */
                Author   [25] ,   /* Nous mˆme...                      */
                Subject  [25] ,   /*                                   */
                PassWord [12] ,   /* Si sender ou group password       */
                RefMsg   [8 ] ,   /* Message r‚f‚renc‚                 */
                SizeMsg  [6 ] ,   /* en ascii, nb blocs de 128 bytes   */
                Delete        ,   /* ???? = E1  (??????)               */
                Conference    ,   /* ???? = 00 ou 0A pour un envoi     */
                Read          ,   /* ???? = 0x00,0x20,0x05 parfois     */
                Nothing[3]    ;   /* ???? = 0x20, ou E, ou E*, etc !   */
       } ;

typedef
struct QmailRepType   /* SEND MESSAGE HEADER STRUCTURE. */
       {
           byte Status;            /* ??? */
           byte ConfNum  [7] ;     /* Numero de la conf‚rence concern‚e */
           byte MsgDate  [13];     /* mm-dd-yyHH:MM                     */
           byte ForWho   [25];     /* Destinataire                      */
           byte Author   [25];     /* Nous mˆme...                      */
           byte Subject  [25];     /*                                   */
           byte PassWord [12];     /* Si sender ou group password       */
           byte RefMsg   [8] ;     /* Message r‚f‚renc‚                 */
           byte SizeMsg  [6] ;     /* en ascii, nb blocs de 128 bytes   */
           byte Delete       ;     /* ????  = E1*/
           byte Unknown2     ;     /* ???? = 00 ou 0A ou 04             */
           byte Unknown3     ;     /* ???? = 00                         */
           byte Unknown4     ;     /* ???? = SPACE                      */
           byte Unknown5     ;     /* ???? = SPACE                      */
           byte Unknown6     ;     /* ???? = 00                         */
       } ;
                        /* Variables used to read Control.dat         */

extern int LastConf ;
extern int Security ;
extern char BoardName[],
     UserName [],
     First    [],
     City     [],
     HomePhone[],
     DataPhone[];


extern char ConfName[MAXCONF][NAMCONF];extern  int ReadControl(char *Path);
extern  int MkIndex(char *SrcDir,char *DestDir);
extern  void WriteIndex(FILE *fx,int count,int Size,long Offset);

/*---------------------------- fin de qmail.h ---------------------------*/
