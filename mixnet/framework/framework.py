from __future__ import annotations

import abc
from typing import Any, Awaitable, Coroutine, Generic, TypeVar

RT = TypeVar("RT")


class Framework(abc.ABC):
    """
    An abstract class that provides essential asynchronous functions.
    This class can be implemented using any asynchronous framework (e.g., asyncio, usim).
    """

    @abc.abstractmethod
    def queue(self) -> Queue:
        pass

    @abc.abstractmethod
    async def sleep(self, seconds: float) -> None:
        pass

    @abc.abstractmethod
    def now(self) -> float:
        pass

    @abc.abstractmethod
    def spawn(self, coroutine: Coroutine[Any, Any, RT]) -> Awaitable[RT]:
        pass


T = TypeVar("T")


class Queue(abc.ABC, Generic[T]):
    """
    An abstract class that provides asynchronous queue operations.
    """

    @abc.abstractmethod
    async def put(self, data: T) -> None:
        pass

    @abc.abstractmethod
    async def get(self) -> T:
        pass

    @abc.abstractmethod
    def empty(self) -> bool:
        pass
