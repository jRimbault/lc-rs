#!/usr/bin/env python3

import argparse
import multiprocessing
import os
import statistics
import sys
from dataclasses import dataclass
from pprint import pprint
from typing import List


def main(args):
    walker = (
        os.path.join(path, file)
        for (path, _, files) in os.walk(args.dir)
        for file in files
    )

    with multiprocessing.Pool() as pool:
        counters = dict(pool.map(analyze_file, walker))

    gmean = statistics.mean(stat.mean for stat in counters.values())
    gmedian = statistics.median(stat.median for stat in counters.values())
    file_name, stats = max(counters.items(), key=lambda t: t[1].max)

    pprint(counters)
    print(f"average line length is {gmean}")
    print(f"median line length is {gmedian}")
    print(f"maximum line length in {file_name} is {stats.max}")


def analyze_file(path):
    with open(path) as file:
        lines_lengths = [
            length for length in (len(line.rstrip()) for line in file) if length
        ]
        return (path, FileStats.from_lines(lines_lengths or [0]))


@dataclass
class FileStats:
    lines: List[int]
    max: int
    mean: float
    median: float

    @staticmethod
    def from_lines(lines_lengths: List[int]):
        return FileStats(
            max=max(lines_lengths),
            mean=statistics.mean(lines_lengths),
            median=statistics.median(lines_lengths),
            lines=lines_lengths,
        )


def parse_args(argv):
    def must_be_dir(path):
        if os.path.isdir(path):
            return path
        raise argparse.ArgumentError()

    parser = argparse.ArgumentParser()
    parser.add_argument("dir", type=must_be_dir)
    return parser.parse_args(argv)


if __name__ == "__main__":
    main(parse_args(sys.argv[1:]))
