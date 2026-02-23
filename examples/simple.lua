local function f(n)
    if n <= 1 then return 1 end
    return n * f(n - 1)
end
return f(5)
