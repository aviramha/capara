import sys
from . import capara
from contextvars import ContextVar

_reference_count = 0
_profiler_context: ContextVar[capara.ProfilerContext] = ContextVar('profiler_context')

def lol():
    import time
    #time.sleep(3)


def start() -> None:
    _profiler_context.set(capara.ProfilerContext())
    capara.start(_profiler_context)
    lol()
    stop()
    a = _profiler_context.get()
    print(a.entries)



def stop() -> None:
    sys.setprofile(None)