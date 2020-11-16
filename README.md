# capara
Context aware Python asyncio rust analyzer

capara is a profiler written in Rust that uses `ContextVars` for storing the profile results.
The main goal is to be able to profile *certain* asyncio tasks, out of concurrently running ones.
It's written in Rust for performance and safety, and also because I like Rust and it's good FFI and soundness experience.

Currently capara captures all calls, not aggregated data, and each function call has a duration of nanoseconds.
Some functions might have None duration in case the profiler stopped before the `stop` function was called.

capara doesn't maintain order of call currently, but should be easily achieveable in next versions.

Currently supports Python 3.8-3.9. Support of Python3.7 is achievable by sending a PR to PyO3 (I am lazy at this moment to solve this)
## Warning
capara heavily relies on Python FFI and is in very early development. Use with caution.

# Why should I use capara?
You should use capara if you want to profile certain tasks in your asyncio code.
Other profilers would show data of all functions that ran, even if in background or not as part of your task.

# Usage
```py
from capara.profiler import Profiler

profiler = Profiler()
with profiler:
    do_something

profiler.results
## will return all functions profiled, in a list of tuple, each tuple containing an entry - (file_name, func_name, duration)
## duration is in nanoseconds.
```

# License
capara was written by Aviram Hassan <aviramyhassan@gmail.com>, copyright 2020, licensed under MIT license.
See `LICENSE`