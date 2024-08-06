from typing import Any, Awaitable, Coroutine, TypeVar

import usim
from usim._primitives.task import Task

from framework import framework


class Framework(framework.Framework):
    """
    A usim implementation of the Framework for discrete-time simulation
    """

    def __init__(self, scope: usim.Scope) -> None:
        super().__init__()

        # Scope is used to spawn concurrent simulation activities (coroutines).
        # μSim waits until all activities spawned in the scope are done
        # or until the timeout specified in the scope is reached.
        # Because of the way μSim works, the scope must be created using `async with` syntax
        # and be passed to this constructor.
        self._scope = scope
        self._tasks: list[Task] = []

    def queue(self) -> framework.Queue:
        return Queue()

    async def sleep(self, seconds: float) -> None:
        await (usim.time + seconds)

    def now(self) -> float:
        # Round to milliseconds to make analysis not too heavy
        return int(usim.time.now * 1000) / 1000

    def spawn(
        self, coroutine: Coroutine[Any, Any, framework.RT]
    ) -> Awaitable[framework.RT]:
        task = self._scope.do(coroutine)
        self._tasks.append(task)
        return task

    def stop_tasks(self) -> None:
        for task in self._tasks:
            task.cancel()


T = TypeVar("T")


class Queue(framework.Queue[T]):
    """
    A usim implementation of the Queue for discrete-time simulation
    """

    def __init__(self):
        super().__init__()
        self._queue = usim.Queue()

    async def put(self, data: T) -> None:
        await self._queue.put(data)

    async def get(self) -> T:
        return await self._queue

    def empty(self) -> bool:
        return len(self._queue._buffer) == 0
