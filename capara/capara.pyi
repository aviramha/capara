from contextvars import ContextVar
from typing import List, Optional, Tuple

class ProfilerContext:
    # file_name, func_name, duration, index
    entries: List[Tuple[str, str, Optional[int], int]]

def start(context: ContextVar[Optional[ProfilerContext]]) -> None: ...
