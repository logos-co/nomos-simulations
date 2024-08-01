import random
from abc import abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import TypeVar

from framework.framework import Framework, Queue


class TemporalMixType(Enum):
    NONE = "none"
    PURE_COIN_FLIPPING = "pure-coin-flipping"
    PURE_RANDOM_SAMPLING = "pure-random-sampling"
    PERMUTED_COIN_FLIPPING = "permuted-coin-flipping"
    NOISY_COIN_FLIPPING = "noisy-coin-flipping"


@dataclass
class TemporalMixConfig:
    mix_type: TemporalMixType
    # The minimum size of queue to be mixed.
    # If the queue size is less than this value, noise messages are added.
    min_queue_size: int
    # Generate the seeds used to create the RNG for each queue that will be created.
    seed_generator: random.Random

    def __post_init__(self):
        assert self.seed_generator is not None
        assert self.min_queue_size > 0


T = TypeVar("T")


class TemporalMix:
    @staticmethod
    def queue(
        config: TemporalMixConfig, framework: Framework, noise_msg: T
    ) -> Queue[T]:
        match config.mix_type:
            case TemporalMixType.NONE:
                return NonMixQueue(framework, noise_msg)
            case TemporalMixType.PURE_COIN_FLIPPING:
                return PureCoinFlipppingQueue(
                    config.min_queue_size,
                    random.Random(config.seed_generator.random()),
                    noise_msg,
                )
            case TemporalMixType.PURE_RANDOM_SAMPLING:
                return PureRandomSamplingQueue(
                    config.min_queue_size,
                    random.Random(config.seed_generator.random()),
                    noise_msg,
                )
            case TemporalMixType.PERMUTED_COIN_FLIPPING:
                return PermutedCoinFlipppingQueue(
                    config.min_queue_size,
                    random.Random(config.seed_generator.random()),
                    noise_msg,
                )
            case TemporalMixType.NOISY_COIN_FLIPPING:
                return NoisyCoinFlippingQueue(
                    random.Random(config.seed_generator.random()),
                    noise_msg,
                )
            case _:
                raise ValueError(f"Unknown mix type: {config.mix_type}")


class NonMixQueue(Queue[T]):
    """
    Queue without temporal mixing. Only have the noise generation when the queue is empty.
    """

    def __init__(self, framework: Framework, noise_msg: T):
        self.__queue = framework.queue()
        self.__noise_msg = noise_msg

    async def put(self, data: T) -> None:
        await self.__queue.put(data)

    async def get(self) -> T:
        if self.__queue.empty():
            return self.__noise_msg
        else:
            return await self.__queue.get()

    def empty(self) -> bool:
        return self.__queue.empty()


class MixQueue(Queue[T]):
    def __init__(self, rng: random.Random, noise_msg: T):
        super().__init__()
        # Assuming that simulations run in a single thread
        self._queue: list[T] = []
        self._rng = rng
        self._noise_msg = noise_msg

    async def put(self, data: T) -> None:
        self._queue.append(data)

    @abstractmethod
    async def get(self) -> T:
        pass

    def empty(self) -> bool:
        return len(self._queue) == 0


class MinSizeMixQueue(MixQueue[T]):
    def __init__(self, min_pool_size: int, rng: random.Random, noise_msg: T):
        super().__init__(rng, noise_msg)
        self._mix_pool_size = min_pool_size

    @abstractmethod
    async def get(self) -> T:
        while len(self._queue) < self._mix_pool_size:
            self._queue.append(self._noise_msg)

        # Subclass must implement this method
        pass


class PureCoinFlipppingQueue(MinSizeMixQueue[T]):
    async def get(self) -> T:
        await super().get()

        while True:
            for i in range(len(self._queue)):
                # coin-flipping
                if self._rng.randint(0, 1) == 1:
                    # After removing a message from the position `i`, we don't fill up the position.
                    # Instead, the queue is always filled from the back.
                    return self._queue.pop(i)


class PureRandomSamplingQueue(MinSizeMixQueue[T]):
    async def get(self) -> T:
        await super().get()

        i = self._rng.randint(0, len(self._queue) - 1)
        # After removing a message from the position `i`, we don't fill up the position.
        # Instead, the queue is always filled from the back.
        return self._queue.pop(i)


class PermutedCoinFlipppingQueue(MinSizeMixQueue[T]):
    async def get(self) -> T:
        await super().get()

        self._rng.shuffle(self._queue)

        while True:
            for i in range(len(self._queue)):
                # coin-flipping
                if self._rng.randint(0, 1) == 1:
                    # After removing a message from the position `i`, we don't fill up the position.
                    # Instead, the queue is always filled from the back.
                    return self._queue.pop(i)


class NoisyCoinFlippingQueue(MixQueue[T]):
    async def get(self) -> T:
        if len(self._queue) == 0:
            return self._noise_msg

        while True:
            for i in range(len(self._queue)):
                # coin-flipping
                if self._rng.randint(0, 1) == 1:
                    # After removing a message from the position `i`, we don't fill up the position.
                    # Instead, the queue is always filled from the back.
                    return self._queue.pop(i)
                else:
                    if i == 0:
                        return self._noise_msg
