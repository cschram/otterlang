import times

proc calculate_pi(iterations: int): float =
    var pi = 0.0
    var sign = 1.0
    
    for k in 0..<iterations:
        pi += sign / (2.0 * float(k) + 1.0)
        sign = -sign
    
    pi * 4.0

when isMainModule:
    let iterations = 100_000_000
    
    let start = cpuTime()
    let pi = calculate_pi(iterations)
    let duration = cpuTime() - start
    
    echo "π ≈ ", pi
    echo "Iterations: ", iterations
    echo "Time: ", duration, " seconds"

