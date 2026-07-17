import sys
def fib(n):
    if n <= 1: return n
    return fib(n-1) + fib(n-2)
r = fib(32)
x = 0
for i in range(5000000): x += i
s = ""
for i in range(100000): s += "x"
lst = []
for i in range(500000): lst.append(i)
