%Y	2001 The full proleptic Gregorian year, zero-padded to 4 digits. chrono supports years from -262144 to 262143.
         Note: years before 1 BCE or after 9999 CE, require an initial sign (+/-).
%C	20	The proleptic Gregorian year divided by 100, zero-padded to 2 digits. 1
%y	01	The proleptic Gregorian year modulo 100, zero-padded to 2 digits. 1
		
%m	07	Month number (01–12), zero-padded to 2 digits.
%b	Jul	Abbreviated month name. Always 3 letters.
%B	July	Full month name. Also accepts corresponding abbreviation in parsing.
%h	Jul	Same as %b.
		
%d	08	Day number (01–31), zero-padded to 2 digits.
%e	 8	Same as %d but space-padded. Same as %_d.
		
%a	Sun	Abbreviated weekday name. Always 3 letters.
%A	Sunday	Full weekday name. Also accepts corresponding abbreviation in parsing.
%w	0	Sunday = 0, Monday = 1, …, Saturday = 6.
%u	7	Monday = 1, Tuesday = 2, …, Sunday = 7. (ISO 8601)
		
%U	28	Week number starting with Sunday (00–53), zero-padded to 2 digits. 2
%W	27	Same as %U, but week 1 starts with the first Monday in that year instead.
		
%G	2001	Same as %Y but uses the year number in ISO 8601 week date. 3
%g	01	Same as %y but uses the year number in ISO 8601 week date. 3
%V	27	Same as %U but uses the week number in ISO 8601 week date (01–53). 3
		
%j	189	Day of the year (001–366), zero-padded to 3 digits.
		
%D	07/08/01	Month-day-year format. Same as %m/%d/%y.
%x	07/08/01	Locale’s date representation (e.g., 12/31/99).
%F	2001-07-08	Year-month-day format (ISO 8601). Same as %Y-%m-%d.
%v	 8-Jul-2001	Day-month-year format. Same as %e-%b-%Y.
		
		TIME SPECIFIERS:
%H	00	Hour number (00–23), zero-padded to 2 digits.
%k	 0	Same as %H but space-padded. Same as %_H.
%I	12	Hour number in 12-hour clocks (01–12), zero-padded to 2 digits.
%l	12	Same as %I but space-padded. Same as %_I.
		
%P	am	am or pm in 12-hour clocks.
%p	AM	AM or PM in 12-hour clocks.
		
%M	34	Minute number (00–59), zero-padded to 2 digits.
%S	60	Second number (00–60), zero-padded to 2 digits. 4
%f	26490000	Number of nanoseconds since last whole second. 5
%.f	.026490	Decimal fraction of a second. Consumes the leading dot. 5
%.3f	.026	Decimal fraction of a second with a fixed length of 3.
%.6f	.026490	Decimal fraction of a second with a fixed length of 6.
%.9f	.026490000	Decimal fraction of a second with a fixed length of 9.
%3f	026	Decimal fraction of a second like %.3f but without the leading dot.
%6f	026490	Decimal fraction of a second like %.6f but without the leading dot.
%9f	026490000	Decimal fraction of a second like %.9f but without the leading dot.
		
%R	00:34	Hour-minute format. Same as %H:%M.
%T	00:34:60	Hour-minute-second format. Same as %H:%M:%S.
%X	00:34:60	Locale’s time representation (e.g., 23:13:48).
%r	12:34:60 AM	Locale’s 12 hour clock time. (e.g., 11:11:04 PM). Falls back to %X if the locale does not have a 12 hour clock format.
		
		TIME ZONE SPECIFIERS:
%Z	ACST	Local time zone name. Skips all non-whitespace characters during parsing. Identical to %:z when formatting. 6
%z	+0930	Offset from the local time to UTC (with UTC being +0000).
%:z	+09:30	Same as %z but with a colon.
%::z	+09:30:00	Offset from the local time to UTC with seconds.
%:::z	+09	Offset from the local time to UTC without minutes.
%#z	+09	Parsing only: Same as %z but allows minutes to be missing or present.
		
		DATE & TIME SPECIFIERS:
%c	Sun Jul  8 00:34:60 2001	Locale’s date and time (e.g., Thu Mar 3 23:05:25 2005).
%+	2001-07-08T00:34:60.026490+09:30	ISO 8601 / RFC 3339 date & time format. 7
		
%s	994518299	UNIX timestamp, the number of seconds since 1970-01-01 00:00 UTC. 8