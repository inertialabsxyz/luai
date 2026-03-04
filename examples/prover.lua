local function is_random(n)
    return tool.call("random", {}).result == 42
end
return is_random()
