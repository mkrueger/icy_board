
(*
 * COPYRIGHT 1992 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 * THIS DOCUMENT CONTAINS CONFIDENTIAL INFORMATION AND TRADE SECRETS
 * PROPRIETARY TO SAMUEL H. SMITH DBA THE TOOL SHOP.
 *
 *)


{ undef OVERLAY}        {DEFINE to enable overlay code}
{$UNDEF  DEBUGGING}      {define to enable test code}

{$I+}    {I/O checking}
{$N-}    {Numeric coprocessor}
{$V-}    {Relaxes string typing}
{$B-}    {Boolean complete evaluation}
{$D+}    {Global debug information}
{$L-}    {Local debug information}

{$ifdef DEBUGGING}
   {$S+}    {Stack checking}
   {$R+}    {Range checking}
{$else}
   {$S-}    {Stack checking}
   {$R-}    {Range checking}
{$endif}

