-- system.lua: showcase of the Phase 7 standard library

-- ── type & tostring ──────────────────────────────────────────────────────────
log(type(42))        -- integer
log(type("hi"))      -- string
log(type(true))      -- boolean
log(type(nil))       -- nil
log(type({}))        -- table
log(tostring(123))   -- "123"
log(tostring(false)) -- "false"

-- ── string library ───────────────────────────────────────────────────────────
local s = "Hello, World!"
log(string.len(s))                          -- 13
log(string.upper(s))                        -- HELLO, WORLD!
log(string.lower(s))                        -- hello, world!
log(string.sub(s, 1, 5))                    -- Hello
log(string.sub(s, -6))                      -- orld!  (last 6 chars including comma)
log(string.rep("ab", 3))                    -- ababab
log(string.rep("ab", 3, "-"))               -- ab-ab-ab
log(string.format("%d + %d = %d", 3, 4, 7)) -- 3 + 4 = 7
log(string.format("hex: %x", 255))          -- hex: ff
log(string.format("str: %s", "lua"))        -- str: lua
local pos = string.find(s, "World")
log(tostring(pos))                          -- 8

-- ── math library ─────────────────────────────────────────────────────────────
log(tostring(math.abs(-7)))         -- 7
log(tostring(math.abs(5)))          -- 5
log(tostring(math.min(3, 1, 4, 1))) -- 1
log(tostring(math.max(3, 1, 4, 1))) -- 4
log(tostring(math.maxinteger))      -- 9223372036854775807
log(tostring(math.mininteger))      -- -9223372036854775808
local scaled = math.scale_div(100, 3, 10)
log(tostring(scaled))               -- 333

-- ── table library ────────────────────────────────────────────────────────────
local t = { 10, 20, 30 }
table.insert(t, 40)
table.insert(t, 2, 15)     -- insert 15 at position 2
log(table.concat(t, ", ")) -- 10, 15, 20, 30, 40
table.remove(t, 2)         -- remove position 2 (15)
log(table.concat(t, ", ")) -- 10, 20, 30, 40

local nums = { 5, 3, 8, 1, 9, 2 }
table.sort(nums)
log(table.concat(nums, " ")) -- 1 2 3 5 8 9

-- sort descending with custom comparator
table.sort(nums, function(a, b) return a > b end)
log(table.concat(nums, " ")) -- 9 8 5 3 2 1

local dst = { 0, 0, 0, 0, 0 }
table.move(nums, 1, 3, 1, dst) -- copy first 3 into dst
log(table.concat(dst, " "))    -- 9 8 5 0 0

-- ── select & unpack ──────────────────────────────────────────────────────────
log(tostring(select("#", "a", "b", "c"))) -- 3
log(tostring(select(2, "a", "b", "c")))   -- b (first of remaining)

local arr = { 7, 8, 9 }
log(tostring(unpack(arr))) -- 7 (first value)

-- ── json library ─────────────────────────────────────────────────────────────
local data = { name = "lua", version = 5, active = true }
-- encode as object (sorted keys)
local encoded = json.encode(data)
log(encoded) -- {"active":true,"name":"lua","version":5}

local arr2 = { 1, 2, 3 }
log(json.encode(arr2)) -- [1,2,3]
log(json.encode(nil))  -- null
log(json.encode(42))   -- 42
log(json.encode("hi")) -- "hi"

local decoded = json.decode('{"x":10,"y":20}')
log(tostring(decoded.x)) -- 10
log(tostring(decoded.y)) -- 20

local decoded_arr = json.decode("[4,5,6]")
log(tostring(decoded_arr[1])) -- 4
log(tostring(decoded_arr[3])) -- 6

-- roundtrip
local original = { score = 100, tag = "win" }
local rt = json.decode(json.encode(original))
log(tostring(rt.score)) -- 100
log(rt.tag)             -- win

-- ── pcall / error ────────────────────────────────────────────────────────────
local ok, err = pcall(function()
    error("something went wrong")
end)
log(tostring(ok)) -- false
log(err)          -- something went wrong

local ok2, val = pcall(function()
    return 42
end)
log(tostring(ok2)) -- true
log(tostring(val)) -- 42

-- ── nested closures with stdlib ───────────────────────────────────────────────
local function map(tbl, fn)
    local result = {}
    for i = 1, #tbl do
        table.insert(result, fn(tbl[i]))
    end
    return result
end

local words = { "hello", "world", "lua" }
local upper_words = map(words, string.upper)
log(table.concat(upper_words, ", ")) -- HELLO, WORLD, LUA

local squares = map({ 1, 2, 3, 4, 5 }, function(x) return x * x end)
log(table.concat(squares, " ")) -- 1 4 9 16 25

-- ── final result ──────────────────────────────────────────────────────────────
return 0
