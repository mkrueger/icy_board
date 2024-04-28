
#define COMBASE 0x2f8
#define LSR 5
#define LSR_THRE 0x20
#define LSR_DAV 0x01
#define FCR 2

main(int argc, char *argv[])
{
        int c;
        int n;
        int j,q;

        c = atoi(argv[1]);
        printf("fcr = %d (0x%02x)\r\n",c,c);

        _DX = COMBASE+FCR;
        _AL = c;
        asm out dx,al;

        n = atoi(argv[2]);
        q = atoi(argv[3]);

        for (;;)
        {
                /* usage: comm fcr count- sends count codes */
                if (argc >= 3)
                {
                        if (n == 0) break;
                        n--;

                        /* wait for thre condition */
                        for (;;)
                        {
                                _DX = COMBASE+LSR;
                                asm in al,dx;

                                /* display receive chars while waiting */
                                if (_AL & LSR_DAV)
                                {
                                        _DX = COMBASE;
                                        asm in al,dx;
                                        c = _AL;
                                        putchar(c);
                                }
                                else
                                if (_AL & LSR_THRE)
                                        break;
                        }

                        /* transmit a character */
                        c = (n & 31) + '@';
                        j=0;
                        do {
                                _AL = c;
                                _DX = COMBASE;
                                asm out dx,al;
                                j++;
                        } while (j < q);
                }

                /* send keyboard input */
                if (bioskey(1))
                {
                        c = (bioskey(0) & 0xFF);
                        if (c == 27)
                                break;

                        /* wait for thre condition */
                        for (;;)
                        {
                                _DX = COMBASE+LSR;
                                asm in al,dx;

                                /* display receive chars while waiting */
                                if (_AL & LSR_DAV)
                                {
                                        _DX = COMBASE;
                                        asm in al,dx;
                                        c = _AL;
                                        putchar(c);
                                }
                                else
                                if (_AL & LSR_THRE)
                                        break;
                        }

                        /* transmit a character */
                        _DX = COMBASE;
                        _AL = c;
                        asm out dx,al;
                }

                /* display input from comm port */
                _DX = COMBASE+LSR;
                asm in al,dx;
                if (_AL & LSR_DAV)
                {
                        _DX = COMBASE;
                        asm in al,dx;
                        c = _AL;
                        putchar(c);
                }
        }
}

