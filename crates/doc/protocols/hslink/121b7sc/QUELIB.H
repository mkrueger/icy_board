
/*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 */


/*
 * quelib.inc - Circular Queue library (inline TurboC macros)
 *
 */

#define QLOW_WATER(QUEUE_SIZE) (QUEUE_SIZE-512)
                        /* qcount to issue XOFF signal */

#define QHIGH_WATER(QUEUE_SIZE) (QLOW_WATER(QUEUE_SIZE)-256)
                        /* qcount to resume with XON signal */

typedef struct {
        int qnext_in;
        int qnext_out;
        int qcount;
} queue_rec;


/*
 * INIT_QUE(queue)
 * init a queue for operation
 */
#define INIT_QUE(Q)                     \
   {                                    \
      Q.qnext_in = 0;                   \
      Q.qnext_out = 0;                  \
      Q.qcount = 0;                     \
   }

/*
 * EMPTY_QUE(queue): boolean
 * Check to see if a queue is empty
 */
#define EMPTY_QUE(Q)  (Q.qcount == 0)

/*
 * QUE_FREE(queue): integer
 * Returns the number of free locations in a queue
 */
#define QUE_FREE(Q,QUEUE_SIZE)  (QUEUE_SIZE - Q.qcount)

/*
 * QUE_USED(queue): integer
 * Returns the number locations used in a queue
 */
#define QUE_USED(Q)  (Q.qcount)

/*
 * ENQUE(queue, char)
 * Add a character to the tail of a queue
 */
#define ENQUE(Q,V,QUEUE_SIZE)           \
   {                                    \
      Q_qdata[Q.qnext_in] = V;          \
      if (Q.qnext_in < QUEUE_SIZE)      \
         ++Q.qnext_in;                  \
      else                              \
         Q.qnext_in = 0;                \
      ++Q.qcount;                       \
   }


/*
 * DEQUE(queue,dest)
 *    (EMPTY_QUE check is required before calling DEQUE)
 * Remove a character from a queue
 *
 */
#define DEQUE(Q,V)                      \
   {                                    \
      V = Q_qdata[Q.qnext_out];         \
      POPQUE(Q);                        \
   }

#define QDEQUE(Q,REG,QUEUE_SIZE)        \
   {                                    \
        _CX = --Q.qcount;               \
        _BX = Q.qnext_out;              \
        REG = Q_qdata[_BX];             \
        ++_BX;                          \
        if (_BX > QUEUE_SIZE)           \
                _BX = 0;                \
        Q.qnext_out = _BX;              \
   }


/* --------------------------------------------------------------- */

/*
 * QUE_TOP(queue)
 * Check next character to be deque'd without performing a deque
 * Returns 0 if queue is empty.
 */
#define __QUE_TOP(Q) (EMPTY_QUE(Q)? 0 : Q_qdata[Q.qnext_out])


/*
 * QUE_BOTTOM(queue)
 * Check last character to enqued
 */
#define __QUE_BOTTOM(Q,QUEUE_SIZE)              \
        (Q.qnext_in?    Q_qdata[Q.qnext_in-1]:  \
                        Q_qdata[QUEUE_SIZE-1])


/*
 * PRIORITY_ENQUE(queue, char)
 * Add a character to the head of a queue
 */
#define __PRIORITY_ENQUE(Q,V,QUEUE_SIZE)        \
   {                                            \
      ++Q.qcount;                               \
      if (Q.qnext_out)                          \
         --Q.qnext_out;                         \
      else                                      \
         Q.qnext_out = QUEUE_SIZE-1;            \
      Q_qdata[Q.qnext_out] = V;                 \
   }

/*
 * POPQUE(queue)
 *    (EMPTY_QUE check is required before calling POPQUE)
 * Discard a character from a queue
 *
 */
#define __POPQUE(Q,QUEUE_SIZE)                  \
   {                                            \
      if (Q.qnext_out < QUEUE_SIZE)             \
         ++Q.qnext_out;                         \
      else                                      \
         Q.qnext_out = 0;                       \
      --Q.qcount;                               \
   }


