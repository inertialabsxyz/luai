local items = {"apple", "banana", "cherry"}
local total = 0
for i = 1, #items do
    local r = tool.call("get_price", {item = items[i]})
    total = total + r.price
end
return total
