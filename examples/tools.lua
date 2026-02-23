-- tools.lua: demonstrate tool.call() with the DemoHost built into the binary.
-- Available tools: echo, add, upper, fail

-- ── echo ─────────────────────────────────────────────────────────────────────
local r1 = tool.call("echo", { message = "hello from lua" })
log(r1.message)   -- hello from lua

-- ── add ──────────────────────────────────────────────────────────────────────
local r2 = tool.call("add", { a = 17, b = 25 })
log(tostring(r2.result))   -- 42

-- ── upper ────────────────────────────────────────────────────────────────────
local r3 = tool.call("upper", { text = "lua is great" })
log(r3.result)   -- LUA IS GREAT

-- ── chained calls ────────────────────────────────────────────────────────────
local sum = tool.call("add", { a = 10, b = 32 })
local shout = tool.call("upper", { text = "answer is " .. tostring(sum.result) })
log(shout.result)   -- ANSWER IS 42

-- ── error handling ───────────────────────────────────────────────────────────
local ok, err = pcall(function()
    tool.call("fail", {})
end)
log(tostring(ok))   -- false
log(err)            -- this tool always fails

-- ── args are canonical JSON in the transcript ─────────────────────────────────
-- The transcript (printed to stderr) will show each call's args as sorted JSON:
--   [0] echo  args={"message":"hello from lua"}
--   [1] add   args={"a":17,"b":25}
--   [2] upper args={"text":"lua is great"}
--   [3] add   args={"a":10,"b":32}
--   [4] upper args={"text":"answer is 42"}
--   [5] fail  args={}   (status: err)

return 0
