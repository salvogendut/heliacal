# Mouse input

The user requested mouse support for the Solaris X desktop. Helios now owns a
Sun Mouse Systems transmitter on SCC channel B (`0x71000000`). It sends the
five-byte long packet `0x80 | (~buttons), dx, dy, 0, 0` at 1200 baud; button
bits are left/middle/right 4/2/1 (pressed = 1 in the inverted serial byte).
Relative motion and button changes are accumulated and split into signed
8-bit packets when the host produces large deltas. SDL Y is inverted before
forwarding because the Sun protocol's positive Y is up.

Heliacal captures the pointer with the keyboard (click the guest display;
Ctrl+Enter releases both). While captured it enables SDL relative mouse mode,
forwards `xrel/yrel`, and sends left/middle/right down/up events. Focus loss and
release enqueue button-up events so a guest does not retain a stuck button.
Guest X only sees the packets after the SCC receive path and its driver initialize.

Validation: core unit tests cover packet encoding, large-motion splitting and
delivery through SCC channel B. A local Solaris 2.6 X startup reached the
desktop-login announcement and produced a nonblank framebuffer; interactive
mouse movement/click verification in the X login remains a manual check.
