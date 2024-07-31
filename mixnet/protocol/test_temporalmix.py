import random
from unittest import IsolatedAsyncioTestCase

import framework.asyncio as asynciofw
from framework.framework import Queue
from protocol.temporalmix import (
    NoisyCoinFlippingQueue,
    NonMixQueue,
    PermutedCoinFlipppingQueue,
    PureCoinFlipppingQueue,
    PureRandomSamplingQueue,
    TemporalMix,
    TemporalMixConfig,
    TemporalMixType,
)


class TestTemporalMix(IsolatedAsyncioTestCase):
    async def test_queue_builder(self):
        # Check if the queue builder generates the correct queue type
        for mix_type in TemporalMixType:
            await self.__test_queue_builder(mix_type)

    async def __test_queue_builder(self, mix_type: TemporalMixType):
        queue: Queue[int] = TemporalMix.queue(
            TemporalMixConfig(mix_type, 4, random.Random(0)),
            asynciofw.Framework(),
            -1,
        )
        match mix_type:
            case TemporalMixType.NONE:
                self.assertIsInstance(queue, NonMixQueue)
            case TemporalMixType.PURE_COIN_FLIPPING:
                self.assertIsInstance(queue, PureCoinFlipppingQueue)
            case TemporalMixType.PURE_RANDOM_SAMPLING:
                self.assertIsInstance(queue, PureRandomSamplingQueue)
            case TemporalMixType.PERMUTED_COIN_FLIPPING:
                self.assertIsInstance(queue, PermutedCoinFlipppingQueue)
            case TemporalMixType.NOISY_COIN_FLIPPING:
                self.assertIsInstance(queue, NoisyCoinFlippingQueue)
            case _:
                self.fail(f"Unknown mix type: {mix_type}")

    async def test_non_mix_queue(self):
        queue: Queue[int] = TemporalMix.queue(
            TemporalMixConfig(TemporalMixType.NONE, 4, random.Random(0)),
            asynciofw.Framework(),
            -1,
        )

        # Check if queue is FIFO
        await queue.put(0)
        await queue.put(1)
        self.assertEqual(0, await queue.get())
        self.assertEqual(1, await queue.get())

        # Check if noise is generated when queue is empty
        self.assertEqual(-1, await queue.get())

        # FIFO again
        await queue.put(2)
        self.assertEqual(2, await queue.get())
        await queue.put(3)
        self.assertEqual(3, await queue.get())

    async def test_pure_coin_flipping_queue(self):
        await self.__test_mix_queue(TemporalMixType.PURE_COIN_FLIPPING)

    async def test_pure_random_sampling(self):
        await self.__test_mix_queue(TemporalMixType.PURE_RANDOM_SAMPLING)

    async def test_permuted_coin_flipping_queue(self):
        await self.__test_mix_queue(TemporalMixType.PERMUTED_COIN_FLIPPING)

    async def test_noisy_coin_flipping_queue(self):
        await self.__test_mix_queue(TemporalMixType.NOISY_COIN_FLIPPING)

    async def __test_mix_queue(self, mix_type: TemporalMixType):
        queue: Queue[int] = TemporalMix.queue(
            TemporalMixConfig(mix_type, 4, random.Random(0)),
            asynciofw.Framework(),
            -1,
        )

        # Check if noise is generated when queue is empty
        self.assertEqual(-1, await queue.get())

        # Put only 2 elements even though the min queue size is 4
        await queue.put(0)
        await queue.put(1)

        # Wait until 2 elements are returned from the queue
        waiting = {0, 1}
        while len(waiting) > 0:
            e = await queue.get()
            if e in waiting:
                waiting.remove(e)
            else:
                # Check if it's the noise
                self.assertEqual(-1, e)

        # Check if noise is generated when there is no real message inserted
        self.assertEqual(-1, await queue.get())
