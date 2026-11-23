# Networking and activity indicators

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

Host networking is available as user-mode NAT: enable it in F9 > Network, then
configure Solaris `le0` with DHCP or a static address on the 192.168.0.0/24
network (gateway 192.168.0.1), for example:

```sh
ifconfig le0 plumb 192.168.0.2 netmask 255.255.255.0 up
route add default 192.168.0.1
```

The OS already detects `le0`; guest network configuration turns the interface
up. The status strip shows
RUN, CPU, NET and attached-media activity lamps; NET pulses as Ethernet frames
cross the NAT. On Linux, external IPv4 ping (including `www.google.com`) uses
unprivileged host ICMP sockets. Gateway ping works without them. If external
ping is unavailable, the app logs the host error; see
[Helios networking](https://github.com/salvogendut/helios/blob/main/docs/networking.md) for Linux ping-group policy,
packet-size limits and diagnostics. TCP NAT, IPv6 and traceroute remain
unimplemented.


## Storage activity

SPARCstation 5, SPARCstation 20 and Axil show a lamp for each configured
target, labeled `SCSI0` through `SCSI6`. The Ultra 5 shows both master (M)
and slave (S) positions on each IDE channel. Its disk uses `IDE0M` and
its ATAPI CD-ROM uses `IDE1M`. Empty positions stay dim. Disk reads flash
green, optical reads flash cyan and disk writes flash amber for 150 ms.
