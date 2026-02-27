#!/usr/bin/env python3
"""Speed benchmark: equivalent workloads to examples/bench_speed.rs"""

import time
import sys


def run_bench(name, fn, iters):
    fn()  # warmup
    start = time.perf_counter()
    for _ in range(iters):
        fn()
    elapsed = time.perf_counter() - start
    ms_total = elapsed * 1000
    ms_per = ms_total / iters
    print(f"{name:<14} {iters:>4} iters  {ms_total:>8.1f}ms total  {ms_per:>8.3f}ms/iter")


def loop_100k():
    s = 0
    for i in range(1, 100001):
        s += i
    return s


def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)


print(f"=== Python {sys.version.split()[0]} ===")
run_bench("loop-100k", loop_100k, 100)
run_bench("fib(28)", lambda: fib(28), 10)
