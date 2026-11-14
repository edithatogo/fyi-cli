"""Offline Linux PTY acceptance for the real Rust CLI; standard library only."""

import errno
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path


def require(condition, message):
    if not condition:
        raise AssertionError(message)


class Session:
    def __init__(self, binary, directory, database="local.db"):
        self.master, self.slave = pty.openpty()
        self.before = termios.tcgetattr(self.slave)
        self.transcript = bytearray()
        self.resize(120, 24)

        environment = os.environ.copy()
        environment.update(TERM="xterm-256color", RUST_LOG="trace", RUST_BACKTRACE="1")
        self.process = subprocess.Popen(  # noqa: S603 - fixed Python launcher and Cargo-provided binary
            [sys.executable, __file__, "--child", binary, database],
            cwd=directory,
            stdin=self.slave,
            stdout=self.slave,
            stderr=self.slave,
            env=environment,
            start_new_session=True,
            close_fds=True,
        )

    def resize(self, width, height):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))

    def read(self, timeout=0.1):
        if select.select([self.master], [], [], timeout)[0]:
            try:
                self.transcript.extend(os.read(self.master, 65536))
            except OSError as error:
                if error.errno != errno.EIO:
                    raise

    def until(self, text, offset=0):
        deadline = time.monotonic() + 10
        while text not in self.transcript[offset:]:
            self.read()
            if time.monotonic() > deadline or self.process.poll() is not None:
                message = f"terminal did not display {text!r}: {bytes(self.transcript)!r}"
                raise AssertionError(message)

    def send(self, keys):
        offset = len(self.transcript)
        os.write(self.master, keys)
        return offset

    def finish(self, code=0):
        deadline = time.monotonic() + 5
        while self.process.poll() is None:
            self.read()
            if time.monotonic() > deadline:
                message = "terminal process did not exit promptly"
                raise AssertionError(message)
        self.read()
        require(self.process.returncode == code, bytes(self.transcript))
        require(termios.tcgetattr(self.slave) == self.before, "terminal flags were not restored")
        content = bytes(self.transcript)
        leave = content.rfind(b"\x1b[?1049l")
        clear = content.rfind(b"\x1b[2J")
        show = content.rfind(b"\x1b[?25h")
        require(0 <= clear < leave, "alternate screen was not cleared before leaving")
        require(show >= 0, "cursor was not restored")
        for forbidden in [b"PRIVATE-BODY", b"PRIVATE-TAGS", b"PRIVATE-MISSING", b"PRIVATE-URL"]:
            require(forbidden not in content, "sensitive fixture field reached terminal output")

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        os.close(self.master)
        os.close(self.slave)


def main(binary):
    with tempfile.TemporaryDirectory(prefix="fyi-tui-pty-") as directory:

        def command(*args):
            result = subprocess.run(  # noqa: S603 - Cargo-provided binary, fixed synthetic arguments, no shell
                [binary, *args],
                cwd=directory,
                capture_output=True,
                timeout=15,
                check=False,
            )
            require(result.returncode == 0, result.stderr)

        command("init-db", "--db", "local.db")
        Path(directory, "authorities.csv").write_text(
            "slug,name,url\nmaple,MapleCouncil,https://catalog.example.invalid/PRIVATE-URL\n",
            encoding="utf-8",
        )
        command("import-authorities", "authorities.csv", "--db", "local.db")
        command(
            "register-request",
            "maple",
            "MapleRecords",
            "PRIVATE-BODY",
            "--tags",
            "PRIVATE-TAGS",
            "--db",
            "local.db",
        )
        # Setup commands may be profiled in an all-features build. The TUI itself
        # must neither create nor rewrite profiling output, even after a clean exit.
        before = {path.name: path.read_bytes() for path in Path(directory).iterdir()}
        session = Session(binary, directory)
        try:
            session.until(b"Requests, counts and timelines are hidden.")
            require(b"MapleRecords" not in session.transcript, "private title was exposed")
            require(b"MapleCouncil" not in session.transcript, "private authority was exposed")
            offset = session.send(b"\tp")
            session.until(b"MapleRecords", offset)
            session.until(b"MapleCouncil", offset)
            offset = session.send(b"d")
            session.until(b"Details", offset)
            offset = session.send(b"\x1b")
            # Wait for the escape key's disambiguation before entering Timeline.
            session.until(b"Requests", offset)
            offset = session.send(b"\r")
            session.until(b"Timeline unavailable.", offset)
            offset = session.send(b"p")
            session.until(b"Requests, counts and timelines are hidden.", offset)
            offset = len(session.transcript)
            session.resize(30, 8)
            session.until(b"Resize terminal", offset)
            offset = len(session.transcript)
            session.resize(120, 24)
            session.until(b"Requests, counts and timelines are hidden.", offset)
            session.send(b"q")
            session.finish()
        finally:
            session.close()

        for exit_action in ["ctrl-c", "sigterm"]:
            session = Session(binary, directory)
            try:
                session.until(b"Requests, counts and timelines are hidden.")
                if exit_action == "ctrl-c":
                    session.send(b"\x03")
                else:
                    session.process.send_signal(signal.SIGTERM)
                session.finish()
            finally:
                session.close()

        session = Session(binary, directory, "PRIVATE-MISSING.db")
        try:
            session.until(b"Local data is unavailable.")
            session.send(b"q")
            session.finish(code=1)
            content = bytes(session.transcript)
            require(
                content.find(b"TUI failed:") > content.rfind(b"\x1b[?1049l"),
                "error preceded cleanup",
            )
        finally:
            session.close()
        after = {path.name: path.read_bytes() for path in Path(directory).iterdir()}
        require(after == before, "TUI launch changed local files")
    sys.stdout.write("PTY acceptance: all runtime contracts passed\n")


if __name__ == "__main__":
    if sys.argv[1] == "--child":
        # This new single-threaded Python process owns its session. Establish the
        # controlling terminal here, without subprocess preexec_fn callbacks.
        fcntl.ioctl(sys.stdin.fileno(), termios.TIOCSCTTY, 0)
        os.execv(sys.argv[2], [sys.argv[2], "tui", "--db", sys.argv[3], "-vv"])  # noqa: S606 - Cargo test binary, no shell
    else:
        main(sys.argv[1])
