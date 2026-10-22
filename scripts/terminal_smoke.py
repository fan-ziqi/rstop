"""Bounded startup/keyboard-exit check using the platform's real pseudoterminal."""
import os
import json
import subprocess
import sys
import time


def check(command, env):
    checks = {}
    for name, keys in {
        "q": (b"q",),
        "ctrl_c": (b"\x03",),
        "double_escape": (b"\x1b", b"\x1b"),
        "buffered_double_escape": (b"\x1b\x1b",),
        "filter_double_escape": (b"2f", b"q", b"\x1b", b"\x1b"),
        "filter_buffered_double_escape": (b"2f", b"q", b"\x1b\x1b"),
        "f10_alias": (b"\x1b[21~",),
    }.items():
        try:
            checks[name] = check_exit(command, env, keys)
        except Exception as error:
            raise RuntimeError(f"Terminal exit check {name} with keys {keys!r}: {error}") from error
    return {"startup": True, "keyboard_exit": True, "exit_keys": checks,
            "output_bytes": sum(result["output_bytes"] for result in checks.values())}


def check_exit(command, env, keys):
    output = bytearray()
    start, send, finished, close = windows(command, env, output) if os.name == "nt" else posix(command, env, output)
    closed = False
    try:
        start()
        deadline = time.monotonic() + 20
        quit_sent = False
        keys_sent = 0
        last_sent = None
        visible_since = None
        while time.monotonic() < deadline:
            if visible_since is None and b"Overview" in output:
                visible_since = time.monotonic()
            now = time.monotonic()
            if (not quit_sent and visible_since is not None and now - visible_since >= 1
                    and (last_sent is None or now - last_sent >= 0.1)):
                send(keys[keys_sent])
                keys_sent += 1
                last_sent = now
                quit_sent = keys_sent == len(keys)
            code = finished()
            if code is not None:
                closed = True
                close()  # Drain the final ConPTY frame, including startup errors, before judging it.
                assert quit_sent and code == 0, f"Terminal startup/exit failed ({code}): {output[-4000:]!r}"
                return {"startup": True, "keyboard_exit": True, "output_bytes": len(output)}
            time.sleep(0.02)
        raise RuntimeError(f"Terminal startup/exit timed out: {output[-4000:]!r}")
    finally:
        if not closed:
            close()


def posix(command, env, output):
    import fcntl
    import pty
    import struct
    import termios
    import threading

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 44, 140, 0, 0))
    original = termios.tcgetattr(slave)
    process = None
    reader = None

    def drain():
        try:
            while data := os.read(master, 65536):
                if len(output) < 2**20:
                    output.extend(data)
        except OSError:
            pass

    def start():
        nonlocal process, reader
        process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env)
        reader = threading.Thread(target=drain, daemon=True)
        reader.start()

    def close():
        if process and process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        restored = termios.tcgetattr(slave) == original
        os.close(slave)
        if reader:
            reader.join(timeout=2)
        os.close(master)
        if process and process.returncode == 0:
            assert restored, "Terminal attributes were not restored"

    return start, lambda data: os.write(master, data), lambda: process.poll(), close


def windows(command, env, output):
    # Windows ConPTY: no external packages or helper compilation required.
    # https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session
    import ctypes as c
    from ctypes import wintypes as w
    from pathlib import Path
    import tempfile
    import threading

    class Coord(c.Structure):
        _fields_ = [("X", c.c_short), ("Y", c.c_short)]

    class Startup(c.Structure):
        _fields_ = [("cb", w.DWORD), ("reserved", w.LPWSTR), ("desktop", w.LPWSTR), ("title", w.LPWSTR)] + [
            (name, w.DWORD) for name in ("x", "y", "width", "height", "xchars", "ychars", "fill", "flags")
        ] + [("show", w.WORD), ("reserved_size", w.WORD), ("reserved_data", c.c_void_p)] + [
            (name, w.HANDLE) for name in ("stdin", "stdout", "stderr")]

    class StartupEx(c.Structure):
        _fields_ = [("startup", Startup), ("attributes", c.c_void_p)]

    class Process(c.Structure):
        _fields_ = [("process", w.HANDLE), ("thread", w.HANDLE), ("pid", w.DWORD), ("tid", w.DWORD)]

    kernel = c.WinDLL("kernel32", use_last_error=True)

    def api(name, result, *args):
        function = getattr(kernel, name)
        function.restype, function.argtypes = result, args
        return function

    handle_ptr = c.POINTER(w.HANDLE)
    size_ptr = c.POINTER(c.c_size_t)
    dword_ptr = c.POINTER(w.DWORD)
    pipe = api("CreatePipe", w.BOOL, handle_ptr, handle_ptr, c.c_void_p, w.DWORD)
    create_console = api("CreatePseudoConsole", c.c_long, Coord, w.HANDLE, w.HANDLE, w.DWORD, handle_ptr)
    init = api("InitializeProcThreadAttributeList", w.BOOL, c.c_void_p, w.DWORD, w.DWORD, size_ptr)
    update = api("UpdateProcThreadAttribute", w.BOOL, c.c_void_p, w.DWORD, c.c_size_t, c.c_void_p, c.c_size_t, c.c_void_p, c.c_void_p)
    create = api("CreateProcessW", w.BOOL, w.LPCWSTR, w.LPWSTR, c.c_void_p, c.c_void_p, w.BOOL, w.DWORD, c.c_void_p, w.LPCWSTR, c.c_void_p, c.c_void_p)
    read = api("ReadFile", w.BOOL, w.HANDLE, c.c_void_p, w.DWORD, dword_ptr, c.c_void_p)
    write = api("WriteFile", w.BOOL, w.HANDLE, c.c_void_p, w.DWORD, dword_ptr, c.c_void_p)
    wait = api("WaitForSingleObject", w.DWORD, w.HANDLE, w.DWORD)
    exit_code = api("GetExitCodeProcess", w.BOOL, w.HANDLE, dword_ptr)
    terminate = api("TerminateProcess", w.BOOL, w.HANDLE, w.UINT)
    close_handle = api("CloseHandle", w.BOOL, w.HANDLE)
    close_console = api("ClosePseudoConsole", None, w.HANDLE)
    delete = api("DeleteProcThreadAttributeList", None, c.c_void_p)

    def require(ok):
        if not ok:
            raise c.WinError(c.get_last_error())

    input_read, input_write, output_read, output_write, console = [w.HANDLE() for _ in range(5)]
    require(pipe(c.byref(input_read), c.byref(input_write), None, 0))
    require(pipe(c.byref(output_read), c.byref(output_write), None, 0))
    result = create_console(Coord(140, 44), input_read, output_write, 0, c.byref(console))
    assert result == 0, f"CreatePseudoConsole failed: {result:#x}"
    size = c.c_size_t()
    init(None, 1, 0, c.byref(size))
    attributes = c.create_string_buffer(size.value)
    require(init(attributes, 1, 0, c.byref(size)))
    require(update(attributes, 0, 0x20016, console, c.sizeof(console), None, None))
    startup = StartupEx()
    startup.startup.cb = c.sizeof(startup)
    # Null standard handles prevent CI's redirected pipes from being copied
    # into the child instead of the pseudoconsole (microsoft/terminal#15814).
    startup.startup.flags = 0x100  # STARTF_USESTDHANDLES
    startup.attributes = c.cast(attributes, c.c_void_p)
    process = Process()
    reader = None
    diagnostics = tempfile.TemporaryDirectory(prefix="rstop-conpty-")
    stderr = Path(diagnostics.name) / "stderr.txt"

    def send(data):
        count = w.DWORD()
        require(write(input_write, c.create_string_buffer(data), len(data), c.byref(count), None))

    def drain():
        buffer = c.create_string_buffer(65536)
        count = w.DWORD()
        while read(output_read, buffer, len(buffer), c.byref(count), None) and count.value:
            if len(output) < 2**20:
                output.extend(buffer.raw[:count.value])

    def start():
        nonlocal reader
        block = c.create_unicode_buffer("\0".join(f"{key}={value}" for key, value in sorted(env.items())) + "\0\0")
        # Start the program through the user's normal console shell. Keep stderr
        # outside the screen buffer so a terminal reset cannot erase a startup error.
        shell = subprocess.list2cmdline([os.environ.get("COMSPEC", "cmd.exe")])
        invocation = subprocess.list2cmdline(command)
        redirect = subprocess.list2cmdline([str(stderr)])
        line = c.create_unicode_buffer(f'{shell} /d /s /c "{invocation} 2>{redirect}"')
        require(create(None, line, None, None, False, 0x80000 | 0x400, block, None, c.byref(startup), c.byref(process)))
        close_handle(input_read)
        close_handle(output_write)
        reader = threading.Thread(target=drain, daemon=True)
        reader.start()

    def finished():
        if wait(process.process, 0) != 0:
            return None
        code = w.DWORD()
        require(exit_code(process.process, c.byref(code)))
        return code.value

    def close():
        if process.process and wait(process.process, 0) != 0:
            terminate(process.process, 1)
            wait(process.process, 5000)
        close_console(console)
        if reader:
            reader.join(timeout=2)
        delete(attributes)
        for handle in (input_write, output_read, process.thread, process.process):
            if handle:
                close_handle(handle)
        if stderr.exists():
            output.extend(stderr.read_bytes())
        diagnostics.cleanup()

    return start, send, finished, close


if __name__ == "__main__":
    command = [os.path.abspath(sys.argv[1]), *sys.argv[2:]]
    print(json.dumps(check(command, os.environ.copy()), indent=2))
