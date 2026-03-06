-- prediction_market.lua: Resolve a prediction market question using external data.
--
-- This program demonstrates provable AI agent execution for prediction markets.
-- It fetches real data via HTTP, uses an LLM for judgement, and returns a
-- structured resolution that can be verified via ZK proof.
--
-- The proof attests:
--   1. This exact resolution logic ran (program_hash)
--   2. These specific data sources were consulted (tool_responses_hash)
--   3. This resolution was produced (output_hash)

-- ── Configuration ──────────────────────────────────────────────────────────

local MARKET_QUESTION = "Is the current Bitcoin price above $100,000 USD?"
local MARKET_CRITERIA = "Resolve YES if the spot price of Bitcoin (BTC) in USD is strictly above 100,000 at the time of resolution. Use at least two independent price sources."

-- ── Step 1: Fetch price data from multiple sources ─────────────────────────

log("Fetching BTC price from source 1 (CoinGecko)...")
local ok1, r1 = pcall(function()
    return tool.call("http_get", {
        url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd"
    })
end)

local price1 = nil
local source1_raw = ""
if ok1 and r1.status == 200 then
    source1_raw = r1.body
    local data = json.decode(r1.body)
    if data and data.bitcoin and data.bitcoin.usd then
        price1 = data.bitcoin.usd
        log("CoinGecko BTC price: " .. tostring(price1))
    else
        log("CoinGecko: unexpected response format")
    end
else
    if ok1 then
        log("CoinGecko: HTTP " .. tostring(r1.status))
    else
        log("CoinGecko: request failed - " .. tostring(r1))
    end
end

log("Fetching BTC price from source 2 (CoinCap)...")
local ok2, r2 = pcall(function()
    return tool.call("http_get", {
        url = "https://api.coincap.io/v2/assets/bitcoin"
    })
end)

local price2 = nil
local source2_raw = ""
if ok2 and r2.status == 200 then
    source2_raw = r2.body
    local data = json.decode(r2.body)
    if data and data.data and data.data.priceUsd then
        -- CoinCap returns price as a string like "97234.12"
        -- We only need the integer part for our comparison
        local price_str = data.data.priceUsd
        local dot_pos = string.find(price_str, ".")
        if dot_pos then
            price_str = string.sub(price_str, 1, dot_pos - 1)
        end
        price2 = tonumber(price_str)
        log("CoinCap BTC price: " .. tostring(price2))
    else
        log("CoinCap: unexpected response format")
    end
else
    if ok2 then
        log("CoinCap: HTTP " .. tostring(r2.status))
    else
        log("CoinCap: request failed - " .. tostring(r2))
    end
end

-- ── Step 2: Get timestamp ──────────────────────────────────────────────────

local time_result = tool.call("time_now", {})
local resolution_timestamp = time_result.timestamp
log("Resolution timestamp: " .. tostring(resolution_timestamp))

-- ── Step 3: Build evidence summary ─────────────────────────────────────────

local sources_found = 0
local evidence_lines = {}

if price1 then
    sources_found = sources_found + 1
    table.insert(evidence_lines, "CoinGecko: $" .. tostring(price1))
end
if price2 then
    sources_found = sources_found + 1
    table.insert(evidence_lines, "CoinCap: $" .. tostring(price2))
end

local evidence_summary = table.concat(evidence_lines, "; ")

-- ── Step 4: Determine resolution ───────────────────────────────────────────

local resolution = "UNKNOWN"
local confidence = "none"
local reasoning = ""

if sources_found == 0 then
    resolution = "UNRESOLVABLE"
    confidence = "none"
    reasoning = "Could not fetch price data from any source."
elseif sources_found == 1 then
    -- Only one source available — resolve but with lower confidence
    local price = price1 or price2
    if price > 100000 then
        resolution = "YES"
        confidence = "medium"
        reasoning = "Price is above $100,000 but only one source available."
    else
        resolution = "NO"
        confidence = "medium"
        reasoning = "Price is at or below $100,000 (one source only)."
    end
else
    -- Both sources available — check agreement
    local both_above = (price1 > 100000) and (price2 > 100000)
    local both_below = (price1 <= 100000) and (price2 <= 100000)

    if both_above then
        resolution = "YES"
        confidence = "high"
        reasoning = "Both sources confirm BTC price is above $100,000."
    elseif both_below then
        resolution = "NO"
        confidence = "high"
        reasoning = "Both sources confirm BTC price is at or below $100,000."
    else
        -- Sources disagree — use LLM for tiebreaker analysis
        log("Sources disagree, consulting LLM for analysis...")
        local llm_result = tool.call("llm_query", {
            prompt = "Two price sources disagree on whether Bitcoin is above $100,000. "
                .. "Source 1 says $" .. tostring(price1) .. ", Source 2 says $" .. tostring(price2) .. ". "
                .. "Given this is likely a timing difference and the prices are close to the threshold, "
                .. "should this market resolve YES or NO? Reply with just YES or NO and a one-sentence reason.",
            context = MARKET_CRITERIA
        })
        local llm_answer = llm_result.response

        if string.find(llm_answer, "YES") then
            resolution = "YES"
        else
            resolution = "NO"
        end
        confidence = "low"
        reasoning = "Sources disagree. LLM tiebreaker: " .. llm_answer
    end
end

log("Resolution: " .. resolution .. " (confidence: " .. confidence .. ")")

-- ── Step 5: Return structured result ───────────────────────────────────────

return {
    market_question = MARKET_QUESTION,
    resolution_criteria = MARKET_CRITERIA,
    resolution = resolution,
    confidence = confidence,
    reasoning = reasoning,
    evidence = evidence_summary,
    sources_consulted = sources_found,
    resolution_timestamp = resolution_timestamp
}
