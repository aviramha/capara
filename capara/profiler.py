import sys

from contextvars import ContextVar
from typing import List, Tuple

from . import capara

_reference_count = 0
_profiler_context: ContextVar[capara.ProfilerContext] = ContextVar('profiler_context')


def start() -> None:
    """Starts the profiler.

    Notes:
        In case the profiler was already started in the same task, this will override existing data.
    """
    _profiler_context.set(capara.ProfilerContext())
    global _reference_count
    _reference_count += 1
    capara.start(_profiler_context)


def stop() -> List[Tuple[str, str, int]]:
    """Stops the profiler. Completely stops the profiler only if reference count equals to zero.

    Returns:
        List of profiler events, each event is a tuple of (file_name, func_name, duration).
    """
    global _reference_count
    _reference_count -= 1
    if _reference_count == 0:
        sys.setprofile(None)
    return _profiler_context.get().entries