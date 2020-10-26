from capara import profiler
from contextvars import ContextVar

profiler.start()
profiler.stop()

# from contextvars import ContextVar
# import asyncio

# c = ContextVar('lol')

# async def main():
#     c.set([1])
#     f = asyncio.get_event_loop().create_task(lol())
#     await f
#     print(c.get())

# async def lol():
#     print(c.get().append(2))
#     print(c.get())

# if __name__ == "__main__":
#     loop = asyncio.get_event_loop()
#     loop.run_until_complete(main())