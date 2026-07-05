"""Python benchmark — equivalent to bench_lion.lion"""
import re
import time
import datetime
import hashlib
import base64
import subprocess
from collections import Counter


def bench(name, fn):
    start = time.time()
    fn()
    elapsed = (time.time() - start) * 1000.0
    print(f"{name}: {elapsed:.2f} ms")


print("=== Python Benchmarks ===")
print(f"Python {__import__('sys').version.split()[0]}\n")

# 1. Regex
print("--- re (regex) ---")
log_lines = []
for i in range(10000):
    log_lines.append(f"2024-01-15 10:30:45 ERROR [module] something went wrong #{i}")
text = "\n".join(log_lines)

bench("re.findall 10k lines", lambda: (
    len(re.findall(r"ERROR\s+\[.*?\]", text)) == 10000 or print("ERROR")
))

bench("re.sub 10k lines", lambda: (
    len(re.sub(r"\d+", "X", text)) > 0 or print("ERROR")
))

bench("re.split 10k lines", lambda: (
    len(re.split(r"\n", text)) == 10000 or print("ERROR")
))

# 2. Counter
print("\n--- collections.Counter ---")
words = []
tokens = ["the", "and", "of", "to", "a", "in", "for", "is", "on", "that"]
for i in range(50000):
    words.append(tokens[i % 10])

bench("Counter 50k words", lambda: (
    Counter(words)["the"] == 5000 or print("ERROR")
))

# 3. Unique
print("\n--- itertools.unique (manual) ---")
mixed = [i % 1000 for i in range(20000)]

bench("unique 20k items (1k distinct)", lambda: (
    len(set(mixed)) == 1000 or print("ERROR")
))

# 4. Sorted
print("\n--- sorted ---")
unsorted = [9999 - i for i in range(10000)]

bench("sorted 10k items (reverse)", lambda: (
    sorted(unsorted)[0] == 0 or print("ERROR")
))

# 5. Datetime
print("\n--- datetime ---")

bench("datetime.now (10k calls)", lambda: (
    all(datetime.datetime.now().year >= 2020 for _ in range(10000))
))

bench("datetime.format (10k calls)", lambda: (
    all(len(datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")) == 19 for _ in range(10000))
))

# 6. Hashlib
print("\n--- hashlib ---")
hash_data = [f"hello world {i}" for i in range(1000)]

bench("sha256 1k strings", lambda: (
    all(len(hashlib.sha256(s.encode()).hexdigest()) == 64 for s in hash_data)
))

bench("base64 1k strings", lambda: (
    all(base64.b64decode(base64.b64encode(s.encode())).decode() == s for s in hash_data)
))

# 7. Subprocess
print("\n--- subprocess ---")

bench("subprocess.run shell (100 calls)", lambda: (
    all(subprocess.run(f"echo {i}", shell=True, capture_output=True).returncode == 0 for i in range(100))
))

print("\n=== Benchmarks complete ===")
