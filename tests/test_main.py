import asyncio
import time

import capara.profiler

SLEEP_TIME = 0.1

def sleep():
    time.sleep(SLEEP_TIME)

async def async_sleep():
    time.sleep(SLEEP_TIME)

def test_sanity():
    capara.profiler.start()
    sleep()
    data = capara.profiler.stop()
    assert len(data) == 1
    entry = data[0]
    assert entry[0] == __file__
    assert entry[1] == 'sleep'
    assert entry[2] // 100000000 == SLEEP_TIME * 10

def test_sanity_async():
    loop = asyncio.get_event_loop()
    capara.profiler.start()
    loop.run_until_complete(async_sleep())
    data = capara.profiler.stop()
    for entry in data:
        if entry[1] == 'async_sleep':
            break

    assert entry[0] == __file__
    assert entry[1] == 'async_sleep'
    assert entry[2] // 100000000 == SLEEP_TIME * 10
