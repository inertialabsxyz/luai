local r = tool.call("search_products", {query = "keyboard"})
local electronics = {}
for i = 1, #r.results do
    local p = r.results[i]
    if p.category == "electronics" then
        table.insert(electronics, {name = p.name, price = p.price})
    end
end
return json.encode(electronics)
