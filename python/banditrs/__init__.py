"""BanditRS — the Python security linter ``bandit``, rewritten in pure Rust.

This package is deliberately thin. All the analysis lives in the native
executables that the wheel installs next to your interpreter:

===========================  ============================================
``bandit``                   drop-in replacement for the PyCQA command
``banditrs``                 identical alias, for environments that also
                             install the PyCQA ``bandit`` package
``bandit-baseline``          scan against a git baseline
``bandit-config-generator``  emit a configuration profile
===========================  ============================================

The module exists so that the scanner can also be located and driven from
Python::

    import banditrs

    banditrs.find_bandit_bin()                  # -> PosixPath('.../bin/bandit')
    banditrs.run(["-r", "src/", "-f", "json"])  # -> CompletedProcess

Or from the command line, equivalently to calling ``bandit`` directly::

    python -m banditrs -r src/
"""

from __future__ import annotations

import os
import subprocess
import sys
import sysconfig
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Sequence

__all__ = ["COMMANDS", "__version__", "find_bandit_bin", "main", "run"]

#: The executables shipped in the wheel.
COMMANDS = ("bandit", "banditrs", "bandit-baseline", "bandit-config-generator")


def _version() -> str:
    from importlib.metadata import PackageNotFoundError, version

    try:
        return version("banditrs")
    except PackageNotFoundError:  # running from a source checkout
        return "0+unknown"


__version__: str = _version()


def _user_scripts_scheme() -> str:
    """Name of the sysconfig scheme used by ``pip install --user``."""
    if sys.version_info >= (3, 10):
        return sysconfig.get_preferred_scheme("user")
    if os.name == "nt":
        return "nt_user"
    if sys.platform == "darwin" and getattr(sys, "_framework", None):
        return "osx_framework_user"
    return "posix_user"


def find_bandit_bin(name: str = "bandit") -> Path:
    """Return the path to one of the executables shipped in the wheel.

    `name` must be one of :data:`COMMANDS`. Raises :class:`FileNotFoundError`
    if the executable cannot be found, which in practice means the package was
    imported from a source checkout rather than from an installed wheel.
    """
    if name not in COMMANDS:
        msg = f"unknown BanditRS command {name!r}; expected one of {', '.join(COMMANDS)}"
        raise ValueError(msg)

    exe = name + (sysconfig.get_config_var("EXE") or "")

    # The wheel puts the binaries in the "scripts" directory of the environment
    # it is installed into; try that first, then the --user location, then the
    # interpreter's own directory (which covers some relocated layouts).
    candidates = [
        Path(sysconfig.get_path("scripts")) / exe,
        Path(sysconfig.get_path("scripts", scheme=_user_scripts_scheme())) / exe,
        Path(sys.executable).parent / exe,
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate

    searched = "\n  ".join(str(c) for c in candidates)
    msg = (
        f"could not find the {name!r} executable shipped with banditrs.\n"
        f"Looked in:\n  {searched}\n"
        "Is the package installed as a wheel (pip install banditrs), rather "
        "than imported from a source checkout?"
    )
    raise FileNotFoundError(msg)


def run(
    args: Sequence[str] = (),
    *,
    command: str = "bandit",
    **kwargs: Any,
) -> subprocess.CompletedProcess:
    """Run a BanditRS command as a subprocess and return the result.

    `args` are passed straight to the executable, and `kwargs` straight to
    :func:`subprocess.run`, so the usual knobs apply::

        banditrs.run(["-r", "src/", "-f", "json"], capture_output=True, text=True)

    Note that ``bandit`` exits with a non-zero status when it finds issues, so
    ``check=True`` is rarely what you want; inspect ``.returncode`` instead.
    """
    return subprocess.run([str(find_bandit_bin(command)), *args], **kwargs)


def main() -> None:
    """Entry point for ``python -m banditrs``."""
    bandit = str(find_bandit_bin())
    argv = sys.argv[1:]
    if sys.platform == "win32":
        # os.exec* on Windows returns control to the shell immediately, which
        # breaks callers that wait on the exit code; spawn and forward instead.
        sys.exit(subprocess.run([bandit, *argv]).returncode)
    os.execv(bandit, [bandit, *argv])
