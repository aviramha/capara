import sys
from contextvars import ContextVar
from dataclasses import dataclass
from typing import List, NamedTuple, Optional

from . import capara

_reference_count = 0
_profiler_context: ContextVar[Optional[capara.ProfilerContext]] = ContextVar("profiler_context", default=None)


class ProfilerEntry(NamedTuple):
    file_name: str
    func_name: str
    # Duration in nanoseconds.
    duration: Optional[int]
    call_index: int


@dataclass
class ProfilerResult:
    entries: List[ProfilerEntry]


def start() -> None:
    """Starts the profiler.

    Notes:
        Raises RuntimeError if a context already exists in task.
    """
    global _reference_count
    if is_active():
        raise RuntimeError("Profiler already exists")
    _profiler_context.set(capara.ProfilerContext())

    if _reference_count == 0:
        capara.start(_profiler_context)
    _reference_count += 1


def stop() -> ProfilerResult:
    """Stops the profiler. Completely stops the profiler only if reference count equals to zero.

    Returns:
        ProfilerResult, with entries sorted by call_index.
    """
    global _reference_count
    if _reference_count > 0:
        _reference_count -= 1
    if _reference_count == 0:
        sys.setprofile(None)
    context = _profiler_context.get()
    if context is None:
        raise RuntimeError("No context was found, stop called without start?")
    entries = context.entries
    _profiler_context.set(None)
    formatted_entries = [ProfilerEntry(*entry) for entry in entries]
    formatted_entries.sort(key=lambda x: x.call_index)
    return ProfilerResult(entries=formatted_entries)


def is_active() -> bool:
    """Checks if profiler is active for current context."""
    return _profiler_context.get() is not None


class Profiler:
    def __init__(self):
        self.results: Optional[ProfilerResult] = None

    def __enter__(self):
        start()
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.results = stop()
