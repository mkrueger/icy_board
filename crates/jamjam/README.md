# jamjam

This is a library to load & convert message bases. It started as a library for the JAM message base format. But it made sense to add pcboard message base loading & conversion code as well for my project.

## JAM

JAM is a message base format.

From Wikipedia:

> The JAM Message Base Format was one of the most popular file formats of message bases on DOS-based BBSes in the 1990s. JAM stands for "Joaquim-Andrew-Mats" after the original authors of the API, Joaquim Homrighausen, Andrew Milner, Mats Birch, and Mats Wallin.[1] Joaquim was the author of FrontDoor, a DOS-based FidoNet-compatible mailer. Andrew was the author of RemoteAccess, a popular DOS-based Bulletin Board System. JAM was originally released in 1993 in C, however the most popular implementation was Mark May's "MK Source for Msg Access" written in Pascal which also saw its initial release in 1993.

I need that as part of a pcboard rewrite (yes pcb has an own message base format but JAM is more common).

## JAM copyright

jamjam is licensed under MIT-X11 or Apache 2.0 (your choice). JAM itself is from:

JAM(mbp) - Copyright 1993 Joaquim Homrighausen, Andrew Milner,
                               Mats Birch, Mats Wallin.
                               ALL RIGHTS RESERVED.

Note that jamjam doesn't contain any 3rd party source code but took some source code comments out of the official JAM (JAM.txt) document.

## PCBoard

This message format is only used inside PCBoard - an old bulettin board system. This format is the base of the more well known QWK standard. The QWK format inherits some funny things from that - for example the 'password' field which is only used by PCBoad AFAIK.

It's only the goal to read/understand the PCBoard format for convert it to something else. I don't bother/mind writing update routines for that. However the format is documented and feel free to add it :).

## QWK

Started to add some QWK support. ATM I'm not at the point where I need the export. But need importing large message bases for testing purposes.
Reading QWK basese work. However the *.ndx files seem to be pretty useless. There is a +-1 error in current QKW implementations.

However JamJam does the correct thing when reading (and will do when writing). The program that defines QWK is PCBOARD :). PCBoard doesn't count the first record in messages.dat so 0 means it's the 2nd record in the file.
Which completely makes sense.

## Example

``` cargo run --example read_jam data/jam/ra ```

## More formats are awaiting

Maybe Squish next?
