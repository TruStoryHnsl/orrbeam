"""Platform abstraction layer."""

import sys

from .base import Platform

if sys.platform == "darwin":
    from .macos import MacOSPlatform as _PlatformImpl
else:
    from .linux import LinuxPlatform as _PlatformImpl


def get_platform() -> Platform:
    return _PlatformImpl()
