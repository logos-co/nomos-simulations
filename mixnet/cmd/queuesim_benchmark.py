import argparse

from queuesim import benchmark

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Queuesim Benchmark",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--num-workers", type=int, required=True, help="num workers")
    args = parser.parse_args()

    benchmark.benchmark(args.num_workers)
