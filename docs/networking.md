# Networking and activity indicators

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

Enable **F9 → Network → Host networking**, save, and start or cold-reset
the machine. Solaris uses `hme0` on the Ultra 5 and `le0` on the
SPARCstation 5, SPARCstation 20 and Axil 320. For example, on an Ultra 5
with a fresh interface, run as root:

```sh
ifconfig hme0 plumb
ifconfig hme0 192.168.0.2 netmask 255.255.255.0 up
ping -s -n 192.168.0.1 56 3
```

Skip `plumb` for an existing interface. For off-subnet traffic,
`route add default 192.168.0.1` adds a default route. The status strip shows
RUN, CPU, NET and attached-media activity lamps; NET pulses as Ethernet frames
cross the NAT. On Linux, external IPv4 ping (including `www.google.com`) uses
unprivileged host ICMP sockets. Gateway ping works without them. If external
ping is unavailable, the app logs the host error; see
[Helios networking](https://github.com/salvogendut/helios/blob/main/docs/networking.md) for Linux ping-group policy,
packet-size limits and diagnostics. All four machine models can initiate IPv4 TCP
connections through this same NAT, including HTTP downloads from a reachable
server. Incoming port forwarding, IPv6 and traceroute remain unsupported.


## Storage activity

SPARCstation 5, SPARCstation 20 and Axil show a lamp for each configured
target, labeled `SCSI0` through `SCSI6`. The Ultra 5 shows both master (M)
and slave (S) positions on each IDE channel. Its disk uses `IDE0M` and
its ATAPI CD-ROM uses `IDE1M`. Empty positions stay dim. Disk reads flash
green, optical reads flash cyan and disk writes flash amber for 150 ms.
