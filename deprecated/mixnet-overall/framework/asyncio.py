from __future__ import annotations

import asyncio
import time
from typing import Any, Awaitable, Coroutine, Generic, TypeVar

from framework import framework


class Framework(framework.Framework):
    """
    An asyncio implementation of the Framework
    """

    def __init__(self):
        super().__init__()

    def queue(self) -> framework.Queue:
        return Queue()

    async def sleep(self, seconds: float) -> None:
        await asyncio.sleep(seconds)

    def now(self) -> float:
        return time.time()

    def spawn(
        self, coroutine: Coroutine[Any, Any, framework.RT]
    ) -> Awaitable[framework.RT]:
        return asyncio.create_task(coroutine)


T = TypeVar("T")


class Queue(framework.Queue[T]):
    """
    An asyncio implementation of the Queue
    """

    def __init__(self):
        super().__init__()
        self._queue = asyncio.Queue()

    async def put(self, data: T) -> None:
        await self._queue.put(data)

    async def get(self) -> T:
        return await self._queue.get()

    def empty(self) -> bool:
        return self._queue.empty()
