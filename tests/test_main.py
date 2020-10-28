import time

import capara.profiler

def sleepy_function():
    time.sleep(0.5)

def test_sanity():
    capara.profiler.start()
    sleepy_function()
    data = capara.profiler.stop()
    print(data)