import time

def calculate_pi(iterations):
    pi = 0.0
    sign = 1.0
    
    for k in range(iterations):
        pi += sign / (2.0 * k + 1.0)
        sign = -sign
    
    return pi * 4.0

if __name__ == "__main__":
    iterations = 100000000
    
    start = time.perf_counter()
    pi = calculate_pi(iterations)
    duration = time.perf_counter() - start
    
    print(f"π ≈ {pi:.10f}")
    print(f"Iterations: {iterations}")
    print(f"Time: {duration:.6f} seconds")

