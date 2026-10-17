local WINDOW_NAME = "custom_signals_copy_window"
local p = game.player
local sigs = {}
local quals = {}
local poles = {}

local function add(p, typ)
	for name, signal in pairs(p) do
		if typ ~= "virtual" or not signal.special then
			sigs[#sigs + 1] = string.format('{"type":"%s","name":"%s"}', typ, name)
		end
	end
end
pcall(function()
	for name in pairs(prototypes.quality) do
		quals[#quals + 1] = '"' .. name .. '"'
	end
end)

for name, prototype in pairs(prototypes.entity) do
	if prototype.type == "electric-pole" then
		local dists = {}

		for quality_name in pairs(prototypes.quality) do
			dists[#dists + 1] = string.format(
				'{"quality":"%s","supply_area_distance":%s}',
				quality_name,
				prototype.get_supply_area_distance(quality_name)
			)
		end

		poles[#poles + 1] = string.format(
			'{"name":"%s","supply_area_distances":%s,"tile_width":%s,"tile_height":%s}',
			name,
			string.format('[%s]', table.concat(dists, ", ")),
			prototype.tile_width,
			prototype.tile_height
		)
	end
end

add(prototypes.item, "item")
add(prototypes.fluid, "fluid")
add(prototypes.recipe, "recipe")
add(prototypes.entity, "entity")
add(prototypes.space_location, "space-location")
add(prototypes.asteroid_chunk, "asteroid-chunk")
add(prototypes.quality, "quality")
add(prototypes.virtual_signal, "virtual")

script.on_event(
	defines.events.on_gui_click,
	function(e)
		local el = e.element
		if el and el.valid and el.name == WINDOW_NAME .. "_close" then
			el.parent.destroy()
			script.on_event(defines.events.on_gui_click, nil)
		end
	end
)

local function format_json(s, opts)
	opts = opts or {}

	local limit = opts.limit or 100
	local indent = opts.indent or 2

	if type(indent) == "number" then indent = string.rep(" ", indent)
	elseif indent == nil then indent = "\t" end

	local n = #s
	local close, lens, stack = {}, {}, {}
	local quote, esc = false, false

	for i = 1, n do
		local c = s:sub(i, i)
		local top = stack[#stack]

		if quote then
			lens[top] = lens[top] + 1
			if esc then esc = false
			elseif c == "\\" then esc = true
			elseif c == '"' then quote = false end
		elseif c == '"' then
			if top then lens[top] = lens[top] + 1 end
			quote = true
		elseif c == "{" or c == "[" then
			stack[#stack+1] = i
			lens[i] = 1
		elseif c == "}" or c == "]" then
			lens[top] = lens[top] + 1
			close[top] = i
			stack[#stack] = nil
			if stack[#stack] then lens[stack[#stack]] = lens[stack[#stack]] + lens[top] end
		elseif top and not c:match("%s") then
			lens[top] = lens[top] + 1
			if c == "," or c == ":" then lens[top] = lens[top] + 1 end
		end
	end

	local function compact(a, b)
		local out, q, e = {}, false, false

		for i = a, b do
			local c = s:sub(i, i)
			if q then
				out[#out+1] = c
				if e then e = false
				elseif c == "\\" then e = true
				elseif c == '"' then q = false end
			elseif c == '"' then
				q = true
				out[#out+1] = c
			elseif not c:match("%s") then
				out[#out+1] = c
				if c == "," or c == ":" then out[#out+1] = " " end
			end
		end

		return table.concat(out)
	end

	local function string_end(i, b)
		local e = false
		i = i + 1
		while i <= b do
			local c = s:sub(i, i)
			if e then e = false
			elseif c == "\\" then e = true
			elseif c == '"' then
				return i
			end
			i = i + 1
		end
		return b
	end

	local function fmt(a, b, depth)
		local c = s:sub(a, a)

		if (c == "{" or c == "[") and lens[a] <= limit then
			local ending = close[a]
			if c == "{" and ending > a + 1 then
				return "{ " .. compact(a + 1, ending - 1) .. " }"
			end
			return compact(a, ending)
		end

		if c ~= "{" and c ~= "[" then
			return compact(a, b)
		end

		local ending = close[a]
		local result = { c }
		local d = depth + 1
		local i = a + 1

		while i < ending do
			while i < ending and s:sub(i,i):match("%s") do i = i + 1 end
			if i >= ending then break end

			result[#result+1] = "\n" .. string.rep(indent, d)

			if c == "{" then
				local j = string_end(i, ending)
				result[#result+1] = s:sub(i, j) .. ": "

				i = j + 1
				while i < ending and s:sub(i,i):match("%s") do i = i + 1 end
				i = i + 1
				while i < ending and s:sub(i,i):match("%s") do i = i + 1 end
			end

			local v = s:sub(i,i)
			local j

			if v == "{" or v == "[" then j = close[i]
			elseif v == '"' then j = string_end(i, ending)
			else
				j = i
				while j < ending and not s:sub(j,j):match('[,%]%}%s]') do j = j + 1 end
				j = j - 1
			end

			result[#result+1] = fmt(i, j, d)
			i = j + 1

			while i < ending and s:sub(i,i):match("%s") do i = i + 1 end

			if s:sub(i,i) == "," then
				result[#result+1] = ","
				i = i + 1
			end
		end

		if #result > 1 then result[#result+1] = "\n" .. string.rep(indent, depth) end
		result[#result+1] = s:sub(ending, ending)

		return table.concat(result)
	end

	return fmt(1, n, 0)
end


if p.gui.screen.custom_signals_copy_window then p.gui.screen.custom_signals_copy_window.destroy()
else
	local f = p.gui.screen.add{type="frame", name=WINDOW_NAME, caption="Signals List", direction="vertical"}
	f.auto_center = true
	f.add{type="label", caption="Select the signals below and press Ctrl+C to copy it."}

	local text = string.format(
		'{"signals":[%s],"qualities":[%s],"electric_poles":[%s]}',
		table.concat(sigs, ","),
		table.concat(quals, ","),
		table.concat(poles, ",")
	)

	local t = f.add{type="text-box", name="bp_text", text=format_json(text, {indent="\t", limit=100})}
	t.style.width = 600
	t.style.height = 600
	t.read_only = true
	t.selectable = true
	p.opened = f
	t.focus()
	t.select_all()
	f.add{type="button", name=WINDOW_NAME .. "_close", caption="Done"}
	p.opened = f
end
