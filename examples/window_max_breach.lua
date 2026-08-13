-- Verifiable task: did the recent price window breach a threshold?
--
-- Given a JSON array of price observations and a threshold, this program:
--   1. parses the observations,
--   2. finds the maximum over the last WINDOW samples,
--   3. applies the rule "breach = window_max >= threshold",
--   4. returns a boolean plus the deciding value (the max that triggered it).
--
-- This is the kind of bounded, high-value decision a smart contract can act on
-- directly: the proof attests that *this* rule ran over *these* inputs and
-- produced *this* verdict. The inputs are inlined here for a self-contained,
-- deterministic example; in production they would arrive as an attested oracle
-- response (e.g. a signed price feed) bound into the proof.

-- ── Inputs ──────────────────────────────────────────────────────────────────

-- A "JSON-ish" array of observations. Each entry is {t = unix_ts, p = price}.
-- Prices are integers (the VM is integer-only); use minor units (e.g. cents).
local OBSERVATIONS = json.decode([[
  [
    {"t": 1718750000, "p": 9980},
    {"t": 1718750060, "p": 10010},
    {"t": 1718750120, "p": 10005},
    {"t": 1718750180, "p": 10240},
    {"t": 1718750240, "p": 10120},
    {"t": 1718750300, "p": 10075}
  ]
]])

local THRESHOLD = 10200   -- breach if the windowed max reaches this
local WINDOW = 4          -- number of most-recent samples to consider

-- ── Step 1: find the max price over the last WINDOW observations ─────────────

local n = #OBSERVATIONS
local start = n - WINDOW + 1
if start < 1 then start = 1 end   -- window larger than the data: use all of it

local window_max = nil
local window_max_t = nil

for i = start, n do
    local obs = OBSERVATIONS[i]
    local price = obs.p
    if window_max == nil or price > window_max then
        window_max = price
        window_max_t = obs.t
    end
end

-- ── Step 2: apply the decision rule ─────────────────────────────────────────

local breached = window_max >= THRESHOLD

log("samples=" .. tostring(n)
    .. " window=" .. tostring(WINDOW)
    .. " window_max=" .. tostring(window_max)
    .. " threshold=" .. tostring(THRESHOLD)
    .. " breached=" .. tostring(breached))

-- ── Step 3: return the verdict plus the deciding value ──────────────────────

return {
    breached = breached,             -- the boolean a verifier acts on
    deciding_value = window_max,     -- the max sample that drove the decision
    deciding_t = window_max_t,       -- when that sample was observed
    threshold = THRESHOLD,
    window = WINDOW,
    samples = n
}
