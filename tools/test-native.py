#!/usr/bin/env python3
"""Run every ignored UI test in a fresh Cargo process, X display and D-Bus session."""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

LOCALE = "infra::i18n::tests::every_shipped_locale_parses_and_loads"
CARGO = ["cargo", "test", "--workspace", "--all-features", "--locked", "--lib"]


def native_tests(listing):
    names = re.findall(r"^([\w:]+): test$", listing, re.MULTILINE)
    if names.count(LOCALE) != 1 or len(names) != len(set(names)):
        raise ValueError("ignored inventory must contain exactly one locale test and no duplicates")
    native = sorted(name for name in names if name.rsplit("::", 1)[-1].startswith("native_"))
    if not native or any(name != LOCALE and name not in native for name in names):
        raise ValueError("unclassified ignored tests: explicitly assign them a validation gate")
    return native


def select(tests, pattern):
    """Keep tests whose path contains a `|`-separated substring. Empty pattern keeps all."""
    if not pattern:
        return tests
    parts = [part for part in pattern.split("|") if part]
    if not parts:
        raise SystemExit("empty native test filter")
    chosen = [name for name in tests if any(part in name for part in parts)]
    if not chosen:
        raise SystemExit(f"no native test matches filter {pattern!r}")
    return chosen


def remove_isolated(path):
    """Mesa can rewrite its shader cache while the directory is being removed."""
    for attempt in range(5):
        try:
            shutil.rmtree(path)
            return
        except FileNotFoundError:
            return
        except OSError:
            if attempt == 4:
                raise
            time.sleep(0.05)


def main(argv=None):
    # Imported callers (the unit tests) pass an explicit list. Reading sys.argv
    # there would parse the unittest runner's own flags.
    if argv is None:
        argv = sys.argv[1:] if __name__ == "__main__" else []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--filter",
        default="",
        help="substring of the test path; separate alternatives with |",
    )
    args = parser.parse_args(argv)
    missing = [tool for tool in ("cargo", "timeout", "xvfb-run", "dbus-run-session") if not shutil.which(tool)]
    if missing:
        raise SystemExit("Missing native test prerequisites: " + ", ".join(missing)
                         + ". Install xvfb, xauth and dbus-x11 (or provide private tools on PATH).")
    listing = subprocess.check_output(CARGO + ["--", "--ignored", "--list", "--format", "terse"], text=True)
    tests = select(native_tests(listing), args.filter)
    label = f" matching {args.filter!r}" if args.filter else ""
    print(f"Discovered {len(tests)} ignored native tests{label}", flush=True)
    for index, test in enumerate(tests):
        # Preserve the real HOME for rustup/Cargo; isolate all application and GTK
        # persistence, displays and session buses. Never reuse GTK across tests.
        tmp = tempfile.mkdtemp(prefix="scenedeck-native-")
        try:
            env = dict(os.environ, GDK_BACKEND="x11", GSK_RENDERER="gl",
                       LIBGL_ALWAYS_SOFTWARE="1", GTK_A11Y="none", GIO_USE_VFS="local",
                       MESA_SHADER_CACHE_DISABLE="1", SCENEDECK_NATIVE_RUNNER="1")
            env.pop("WAYLAND_DISPLAY", None)
            for key, directory in [("XDG_CONFIG_HOME", "config"), ("XDG_DATA_HOME", "data"),
                                   ("XDG_CACHE_HOME", "cache"), ("XDG_STATE_HOME", "state"),
                                   ("XDG_RUNTIME_DIR", "run")]:
                path = Path(tmp, directory)
                path.mkdir(mode=0o700)
                env[key] = str(path)
            # No service directories: GTK fixtures must not autoactivate the
            # user's portals, keyring or other desktop services.
            bus_config = Path(tmp, "session-bus.conf")
            bus_config.write_text(
                '<busconfig><type>session</type>'
                f'<listen>unix:abstract=scenedeck-native-{os.getpid()}-{index}</listen>'
                '<auth>EXTERNAL</auth><policy context="default">'
                '<allow own="*"/><allow send_destination="*"/><allow receive_sender="*"/>'
                '</policy></busconfig>'
            )
            command = ["timeout", "--signal=TERM", "--kill-after=15s", "180s",
                       "xvfb-run", "--auto-servernum", "--server-args=-screen 0 1920x1080x24 -nolisten tcp",
                       "dbus-run-session", "--config-file", str(bus_config), "--", *CARGO, test, "--", "--exact", "--ignored",
                       "--test-threads=1", "--nocapture"]
            print(f"Native test: {test}", flush=True)
            subprocess.run(command, check=True, env=env)
        finally:
            remove_isolated(tmp)


if __name__ == "__main__":
    main()
