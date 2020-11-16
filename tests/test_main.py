import asyncio
import time

import pytest

import capara.profiler

SLEEP_TIME = 0.1


def sleep(duration: float):
    time.sleep(duration)


async def async_sleep(duration: float):
    await asyncio.sleep(duration)


async def real_sleep(duration: float):
    sleep(duration)


@pytest.mark.flaky
def test_sanity():
    capara.profiler.start()
    sleep(SLEEP_TIME)
    result = capara.profiler.stop()
    assert len(result.entries) == 2
    entry = result.entries[0]
    assert entry.file_name == __file__
    assert entry.func_name == "sleep"
    assert entry.duration // 100000000 == SLEEP_TIME * 10
    assert entry.call_index == 0


@pytest.mark.flaky
def test_sanity_context():
    profiler = capara.profiler.Profiler()
    with profiler:
        sleep(SLEEP_TIME)
    result = profiler.results
    assert len(result.entries) == 3
    entry = result.entries[0]
    assert entry.file_name == __file__
    assert entry.func_name == "sleep"
    assert entry.duration // 100000000 == SLEEP_TIME * 10
    assert entry.call_index == 0


@pytest.mark.flaky
def test_sanity_async():
    loop = asyncio.get_event_loop()
    capara.profiler.start()
    loop.run_until_complete(async_sleep(SLEEP_TIME))
    result = capara.profiler.stop()
    for entry in result.entries:
        if entry.func_name == "async_sleep":
            break

    assert entry.file_name == __file__
    assert entry.func_name == "async_sleep"
    assert entry.duration // 100000000 == SLEEP_TIME * 10
    assert entry.call_index == 20


async def async_task_self_profiling():
    """
    Task that runs a profiler, then returns the results.
    """
    print(capara.profiler._profiler_context.get())
    profiler = capara.profiler.Profiler()
    with profiler:
        await async_sleep(SLEEP_TIME)
    return profiler.results


async def run_multi_tasks():
    tasks = {async_task_self_profiling(), async_task_self_profiling(), async_task_self_profiling()}
    done, pending = await asyncio.wait(tasks)
    return [task.result() for task in done]


@pytest.mark.flaky
def test_concurrent_tasks():
    loop = asyncio.get_event_loop()
    data = loop.run_until_complete(run_multi_tasks())
    for profiler in data:
        for entry in profiler.entries:
            if entry.func_name == "async_sleep":
                break

        assert entry.file_name == __file__
        assert entry.func_name == "async_sleep"
        assert entry.duration // 100000000 == SLEEP_TIME * 10
        assert entry.call_index == 0


def test_double_start_error():
    capara.profiler.start()
    with pytest.raises(RuntimeError):
        capara.profiler.start()
    capara.profiler.stop()


def test_async_double_start_error():
    async def second_task():
        profiler = capara.profiler.Profiler()
        with profiler:
            await async_sleep(SLEEP_TIME)

    async def first_task():
        profiler = capara.profiler.Profiler()
        with profiler, pytest.raises(RuntimeError):
            await second_task()

    loop = asyncio.get_event_loop()
    loop.run_until_complete(first_task())


def test_stop_without_start():
    with pytest.raises(RuntimeError):
        capara.profiler.stop()


# def test_blocking_asyncio_concurrency():
