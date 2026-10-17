#!/usr/bin/env python3
"""Runs a Herdr client on a private pseudo-terminal, standing in for the user's terminal.

Herdr forwards pane clipboard writes (OSC 52) to the foreground client, so a run
needs a real attached client to observe them. The client's screen output is drained
to a log so the PTY never blocks.

usage: attach-viewer.py <cols> <rows> <output-log> <command> [args...]
"""
import fcntl
import os
import pty
import struct
import sys
import termios

cols, rows, log_path, *command = sys.argv[1:]
pid, fd = pty.fork()
if pid == 0:
    os.execvp(command[0], command)

fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", int(rows), int(cols), 0, 0))
with open(log_path, "ab") as log:
    while True:
        try:
            data = os.read(fd, 65536)
        except OSError:
            break
        if not data:
            break
        log.write(data)
        log.flush()
os.waitpid(pid, 0)
