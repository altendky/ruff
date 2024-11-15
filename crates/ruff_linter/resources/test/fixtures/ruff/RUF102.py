import asyncio

import anyio


async def pass_no_await():
    try:
        await asyncio.sleep(1)
    except Exception as e:
        print(f"{e}")
        raise


async def pass_shielded():
    try:
        await asyncio.sleep(1)
    except Exception as e:
        with anyio.CancelScope(shield=True):
            await asyncio.sleep(1)
            print(f"{e}")
            raise


async def pass_just_value_error():
    try:
        await asyncio.sleep(1)
    except ValueError as e:
        await asyncio.sleep(1)
        print(f"{e}")
        raise


async def fail_exception():
    try:
        await asyncio.sleep(1)
    except Exception as e:
        await asyncio.sleep(1)
        print(f"{e}")
        raise


async def fail_cancelled_error():
    try:
        await asyncio.sleep(1)
    except asyncio.CancelledError as e:
        await asyncio.sleep(1)
        print(f"{e}")
        raise


async def fail_nested_exception():
    try:
        await asyncio.sleep(1)
    except ((ZeroDivisionError, Exception), ValueError) as e:
        await asyncio.sleep(1)
        print(f"{e}")
        raise


async def fail_finally():
    try:
        await asyncio.sleep(1)
    finally:
        await asyncio.sleep(1)
        print(f"{e}")
        raise
