import sys
from contextvars import ContextVar
from typing import List, Optional, Tuple

from . import capara

_reference_count = 0
_profiler_context: ContextVar[Optional[capara.ProfilerContext]] = ContextVar("profiler_context")


def start() -> None:
    """Starts the profiler.

    Notes:
        In case the profiler was already started in the same task, this will override existing data.
    """
    _profiler_context.set(capara.ProfilerContext())
    global _reference_count
    if _reference_count == 0:
        capara.start(_profiler_context)
    _reference_count += 1


def stop() -> List[Tuple[str, str, Optional[int]]]:
    """Stops the profiler. Completely stops the profiler only if reference count equals to zero.

    Returns:
        List of profiler events, each event is a tuple of (file_name, func_name, duration).
    """
    global _reference_count
    _reference_count -= 1
    if _reference_count == 0:
        sys.setprofile(None)
    context = _profiler_context.get()
    if context is None:
        raise RuntimeError("No context was found, stop called without start?")
    entries = context.entries
    # Remove stop function entry to avoid garbage
    entries.remove((__file__, "stop", None))
    _profiler_context.set(None)
    return entries


class Profiler:
    def __init__(self):
        self.results: Optional[List[Tuple[str, str, Optional[int]]]] = None

    def __enter__(self):
        start()

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.results = stop()
        self.results.remove((__file__, "__exit__", None))
