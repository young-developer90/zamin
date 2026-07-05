@echo off
echo ============================================
echo   Lion vs Python Benchmarks
echo ============================================
echo.

echo.
echo --- Running Lion benchmark ---
echo.
cargo run --bin lion -- run benchmarks/bench_lion.lion 2>&1

echo.
echo.
echo --- Running Python benchmark ---
echo.
python benchmarks/bench_python.py
