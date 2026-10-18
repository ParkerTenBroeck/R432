io.stdout:setvbuf("line")

local function script_directory(path)
	return path:match("^(.*)[/\\]") or "."
end

local root = script_directory(arg[0])
local repository = root .. "/.."

-- The R432 modules normally run inside modulepack.  The emulator only needs
-- demangle() from it, so a normal LuaJIT process can use this tiny adapter.
package.preload.modulepack = function()
	return { demangle = function(value) return value end }
end
package.path = table.concat({
	repository .. "/?.lua",
	repository .. "/?/init.lua",
	repository .. "/spaghetti/?.lua",
	repository .. "/spaghetti/?/init.lua",
	package.path,
}, ";")

local emulator = require("r4.emulator")
local bitx = require("spaghetti.bitx")

local options = {
	binary = nil,
	config = nil,
	moves = 100000,
	seed = 1,
	white = "ai-greed",
	black = "ai-greed",
	max_frames = 100000000,
	quiet = false,
}

local function usage(exit_code)
	io.write([[
Usage: ./r432-bench.sh [options]

Options:
  --moves N              Number of complete moves to measure (default: 20)
  --white AI             ai-greed or ai-rand (default: ai-greed)
  --black AI             ai-greed or ai-rand (default: ai-greed)
  --seed N               Random-source seed (default: 1)
  --binary PATH          Raw Powder image to execute
  --config PATH          config.lua used to obtain the CPU geometry
  --max-frames N         Stop a wedged run after this many frames
  --quiet                Print only the configuration and final summary
  -h, --help             Show this help
]])
	os.exit(exit_code)
end

local numeric_options = {
	["--moves"] = "moves",
	["--seed"] = "seed",
	["--max-frames"] = "max_frames",
}
local string_options = {
	["--white"] = "white",
	["--black"] = "black",
	["--binary"] = "binary",
	["--config"] = "config",
}

local index = 1
while index <= #arg do
	local item = arg[index]
	if item == "-h" or item == "--help" then
		usage(0)
	end
	if item == "--quiet" then
		options.quiet = true
		index = index + 1
	else
	local key = numeric_options[item] or string_options[item]
	if not key or not arg[index + 1] then
		io.stderr:write("invalid or incomplete option: " .. tostring(item) .. "\n")
		usage(2)
	end
	local value = arg[index + 1]
	if numeric_options[item] then
		value = tonumber(value)
		if not value or value < 0 or value ~= math.floor(value) then
			io.stderr:write(item .. " requires a non-negative integer\n")
			os.exit(2)
		end
	end
	options[key] = value
	index = index + 2
	end
end

local controller_key = {
	["ai-greed"] = string.byte("2"),
	["ai-rand"] = string.byte("3"),
}
if not controller_key[options.white] or not controller_key[options.black] then
	io.stderr:write("--white and --black must be ai-greed or ai-rand\n")
	os.exit(2)
end
if options.moves == 0 then
	print("No moves requested.")
	os.exit(0)
end

local function read_all(path)
	local file, message = io.open(path, "rb")
	if not file then
		error(message, 0)
	end
	local data = assert(file:read("*a"))
	file:close()
	return data
end

local config = read_all(options.config)
local core_types = config:match('cores%s*=%s*"([im]+)"')
local memory_rows = tonumber(config:match("memory_rows%s*=%s*(%d+)"))
local memory_base_text = config:match("memory_base%s*=%s*(0[xX][%da-fA-F]+)")
local start_pc_text = config:match("start_pc%s*=%s*(0[xX][%da-fA-F]+)")
if not (core_types and memory_rows and memory_base_text and start_pc_text) then
	error("could not read CPU geometry from " .. options.config, 0)
end
local memory_base = assert(tonumber(memory_base_text))
local start_pc = assert(tonumber(start_pc_text))

local image = read_all(options.binary)
local memory_capacity = memory_rows * emulator.row_size * 4
if #image > memory_capacity then
	error(("image is %d bytes but the configured CPU has only %d bytes"):format(
		#image, memory_capacity), 0)
end

local TERMINAL_BASE = 0xE0010000
local TERMINAL_SIZE = 0x2000
local SCROLLPRINT_BASE = TERMINAL_BASE + 0x200
local SCROLLPRINT_END = SCROLLPRINT_BASE + 0x200
local RANDOM_SOURCE = 0xE0060000
local FRAME_CLOCK = 0xE0070000
local PERIPHERAL_EU = 3

local frame = 0
local random_state = options.seed % 0x100000000
if random_state == 0 then
	random_state = 1
end

-- Printer::init() resets once, select_controller() resets again, then
-- read_blocking() polls once for each of the two menus.
local keyboard = {
	0, 0, controller_key[options.white],
	0, 0, controller_key[options.black],
}
local keyboard_index = 1
local terminal_reads = 0
local terminal_writes = 0

local current_side
local think_started
local partial_move
local measurements = {}

local function cell_name(x, y)
	-- The Powder board is drawn with rank 8 at terminal row 1.
	return string.char(string.byte("a") + x - 1) .. tostring(9 - y)
end

local function finish_move(x, y)
	partial_move.to = cell_name(x, y)
	table.insert(measurements, partial_move)
	if not options.quiet then
		print(("%3d  %-5s  %s%s  %10d frames"):format(
			#measurements,
			partial_move.side,
			partial_move.from,
			partial_move.to,
			partial_move.frames))
	end
	partial_move = nil
end

local function observe_terminal_write(address, value)
	terminal_writes = terminal_writes + 1
	if address < SCROLLPRINT_BASE or address >= SCROLLPRINT_END then
		return
	end
	local flags = math.floor((address - SCROLLPRINT_BASE) / 4)
	if bitx.band(flags, 0x40) == 0 then
		return
	end
	local position = bitx.band(bitx.rshift(value, 16), 0x3FF)
	local x = bitx.band(position, 0x1F)
	local y = bitx.band(bitx.rshift(position, 5), 0x1F)
	local byte = bitx.band(value, 0xFF)

	if x == 11 and y == 5 and byte == string.byte("W") then
		current_side = "White"
	elseif x == 11 and y == 4 and byte == string.byte("B") then
		current_side = "Black"
	elseif x == 11 and y == 6 and current_side then
		-- This is the final write made by draw_turn().  The following guest
		-- instructions begin controller selection for that side.
		think_started = frame
	elseif x >= 1 and x <= 8 and y >= 1 and y <= 8 then
		if think_started then
			-- redraw_move() draws the source first.  Its first terminal access is
			-- the end boundary, so terminal rendering time is not included.
			partial_move = {
				side = current_side or "?",
				from = cell_name(x, y),
				frames = frame - think_started,
			}
			think_started = nil
		elseif partial_move then
			finish_move(x, y)
		end
	end
end

local function random_next()
	random_state = bitx.bxor(random_state, bitx.lshift(random_state, 13))
	random_state = bitx.bxor(random_state, bitx.rshift(random_state, 17))
	random_state = bitx.bxor(random_state, bitx.lshift(random_state, 5))
	return random_state
end

local function in_range(value, first, size)
	return value >= first and value < first + size
end

local function bus_access(ix_eu, store, load, _, _, address, value)
	local terminal = in_range(address, TERMINAL_BASE, TERMINAL_SIZE)
	local random = address == RANDOM_SOURCE
	local clock = address == FRAME_CLOCK
	if not (terminal or random or clock) then
		return 0, false, false
	end

	-- The three devices are attached after EU 3 in config.lua.  A request
	-- issued earlier in a frame is held until it reaches that EU.
	if ix_eu ~= PERIPHERAL_EU then
		return 0, false, true
	end

	if terminal then
		if store then
			observe_terminal_write(address, value)
			return 0, true, false
		end
		if load and address == TERMINAL_BASE then
			terminal_reads = terminal_reads + 1
			local key = keyboard[keyboard_index] or 0
			keyboard_index = keyboard_index + 1
			return key, true, false
		end
		return 0, true, false
	elseif random then
		return random_next(), true, false
	else
		return frame % 0x100000000, true, false
	end
end

local function new_cpu()
	local cpu = emulator.make_context({
		mem_row_count = memory_rows,
		core_types = core_types,
		bus_access = bus_access,
	})
	for offset = 0, #image - 1, 4 do
		local b0, b1, b2, b3 = image:byte(offset + 1, offset + 4)
		b1, b2, b3 = b1 or 0, b2 or 0, b3 or 0
		cpu.mem[offset] = b0 + b1 * 0x100 + b2 * 0x10000 + b3 * 0x1000000
	end
	cpu.pc = start_pc
	cpu.started = true
	return cpu
end

local cpu = new_cpu()

print("R432 Powder AI benchmark")
print(("image:  %s (%d bytes, mapped at 0x%08X)"):format(
	options.binary, #image, memory_base))
print(("CPU:    %s (%d EUs, %d instruction slots/frame)"):format(
	core_types, #core_types, #core_types * emulator.sub_eu_count))
print(("match:  %s (White) vs %s (Black), seed %u"):format(
	options.white, options.black, options.seed))
if not options.quiet then
	print("ply  side   move      compute")
end

while #measurements < options.moves and frame < options.max_frames do
	cpu:frame("none")
	frame = frame + 1
	if not cpu.started then
		break
	end
end

if #measurements < options.moves then
	if cpu.started then
		error(("CPU timed out after %d frames with only %d/%d moves measured " ..
			"(pc=0x%08X, terminal reads=%d, writes=%d, side=%s)")
			:format(frame, #measurements, options.moves, cpu.pc, terminal_reads,
				terminal_writes, tostring(current_side)), 0)
	else
		print(("\nGame ended after %d plies (%d frames); requested maximum was %d.")
			:format(#measurements, frame, options.moves))
	end
end

local function print_summary(side)
	local count, total, minimum, maximum = 0, 0
	for _, measurement in ipairs(measurements) do
		if not side or measurement.side == side then
			count = count + 1
			total = total + measurement.frames
			minimum = math.min(minimum or measurement.frames, measurement.frames)
			maximum = math.max(maximum or measurement.frames, measurement.frames)
		end
	end
	if count ~= 0 then
		print(("%-7s %3d moves, avg %.1f, min %d, max %d frames"):format(
			side or "Overall", count, total / count, minimum, maximum))
	end
end

print("")
print_summary("White")
print_summary("Black")
print_summary(nil)
