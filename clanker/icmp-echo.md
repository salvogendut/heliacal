# External IPv4 ping

Heliacal now enables the sibling Helios `host-icmp` feature alongside CHD.
The existing F9 Network setting controls the NAT; no new network menu or guest
hardware setting is needed. The worker logs the relay's one-time host setup/send
warning so a denied Linux ping socket no longer fails silently.

The relay is implemented in Helios, outside emulated device time. It uses
bounded nonblocking unprivileged Linux ICMP sockets and preserves the guest's
echo identifier, sequence and payload. No raw sockets, host policy changes or
ping subprocesses are used. Non-Linux hosts retain gateway echo, UDP and DNS;
external echo reports unavailable. TCP, IPv6 and traceroute remain unsupported.

Validation: 78 frontend tests, formatting, Clippy, release rebuild and SDL
software/dummy smoke pass. The final release was exercised through real serial
PTYs on disposable SS-20/NetBSD and SS-5/Solaris disks. NetBSD gateway, numeric
Google and Google hostname pings each return 3/3 after IPv4 duplicate-address
detection finishes. Solaris gateway, numeric Google and hostname ping each return 3/3 as well.
Its empty `/etc/nsswitch.conf` needed `hosts: files dns`; the guest uses
`nameserver 192.168.0.1`. The final release passed both guest runs, followed by
clean halt, Quit=0 and PTY cleanup.

Evidence and exact guest transcripts: `../helios/local/icmp-2026-11-16/`.
Core implementation/validation: `../helios/clanker/icmp-echo.md`.
Temporary packet instrumentation was removed before the final rebuild. The
user's running frontend, saved machine catalog and original disks were not
modified. Test clones and processes are separate and disposable.
