import sys
from contextvars import ContextVar
from dataclasses import dataclass
from typing import List, Optional

from . import capara


@dataclass
class ReferenceCount:
    count: int = 0


@dataclass
class ProfilerEntry:
    func_name: str
    file_name: str
    duration: Optional[int] = None

    @classmethod
    def from_tuple(cls, file_name: str, func_name: str, duration: Optional[int]) -> "ProfilerEntry":
        return cls(file_name=file_name, func_name=func_name, duration=duration)


@dataclass
class ProfilerResult:
    entries: List[ProfilerEntry]


_reference_count = ReferenceCount()
_profiler_context: ContextVar[Optional[capara.ProfilerContext]] = ContextVar("profiler_context", default=None)


def start() -> None:
    """Starts the profiler.

    Notes:
        Raises RuntimeError if a context already exists in task.
    """
    if is_active():
        raise RuntimeError("Profiler already exists")
    _profiler_context.set(capara.ProfilerContext())

    if _reference_count.count == 0:
        capara.start(_profiler_context)
    _reference_count.count += 1


def stop() -> ProfilerResult:
    """Stops the profiler. Completely stops the profiler only if reference count equals to zero.

    Returns:
        ProfilerResult
    """
    _reference_count.count -= 1
    if _reference_count.count == 0:
        sys.setprofile(None)
    context = _profiler_context.get()
    if context is None:
        raise RuntimeError("No context was found, stop called without start?")
    entries = context.entries
    # Remove stop function entry to avoid garbage
    entries.remove((__file__, "stop", None))
    _profiler_context.set(None)
    return ProfilerResult(entries=[ProfilerEntry.from_tuple(*entry) for entry in entries])


def is_active() -> bool:
    """Checks if profiler is active for current context."""
    return _profiler_context.get() is not None


class Profiler:
    def __init__(self):
        self.results: Optional[ProfilerResult] = None

    def __enter__(self):
        start()

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.results = stop()
        self.results.entries.remove(ProfilerEntry(file_name=__file__, func_name="__exit__", duration=None))
