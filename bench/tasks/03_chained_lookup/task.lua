local user = tool.call("get_user", {user_id = "u42"})
local orders = tool.call("get_orders", {user_id = user.id})
local total = 0
for i = 1, #orders.orders do
    total = total + orders.orders[i].amount
end
return json.encode({name = user.name, order_total = total})
