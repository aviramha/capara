from contextvars import ContextVar
from typing import List, Optional, Tuple

class ProfilerContext:
    entries: List[Tuple[str, str, Optional[int]]]

def start(context: ContextVar[Optional[ProfilerContext]]) -> None: ...
