local root = R4_ROOT or "."

tpt.setfpscap(2)

loadfile(root .. "/r4plot.lua")({
	x = 94,
	y = 128,
	components = {
		{
			type        = "cpu",
			name        = "cpu0",
			cores       = "iiiiiiii",
			memory_rows = 64,
			left        = -60,
			top         = 36-48,
			machine_id  = 1000,
			memory_base = 0x400000,
			memory      = root .. "/target/riscv32i-unknown-none-elf/release/powder.bin",
			start_pc    = 0x400000,
		},
		{
			type          = "terminal",
			name          = "terminal0",
			left          = 140,
			screen_bottom = 110,
			keyboard_top  = 128,
			chars_nh      = 12,
			chars_nv      = 10,
			colors 		  = {
								[0] = 0x000000, 
								[1] = 0x666666,
								[2] = 0xFFFFFF, 
								[3] = 0x95533b, 
								[4] = 0xcf7633, 
								[5] = 0xfbc273, 
								[6] = 0x5a382f, 
								[8] = 0xFF0000, 
								[7] = 0x00FF00, 
								[9] = 0x0000FF, 
								[10] = 0xFFFFFF, 
								[11] = 0xFFFFFF, 
								[12] = 0xFFFFFF, 
								[13] = 0xFFFFFF, 
								[14] = 0xFFFFFF, 
								[15] = 0xFFFFFF, 
							},
			font	      = {
								[ 0x10 ] = [[
									........
									...##...
									..####..
									..####..
									...##...
									..####..
									.######.
									........
								]],
								[ 0x11 ] = [[
									...#....
									..##.#..
									..##.#..
									..####..
									...##...
									..####..
									.######.
									........
								]],
									[ 0x12 ] = [[
									...##...
									..####..
									.#####..
									.##.###.
									...####.
									..####..
									.######.
									........
								]],
									[ 0x13 ] = [[
										........
										.#.##.#.
										.######.
										..####..
										..####..
										..####..
										.######.
										........
									]],
								[ 0x14 ] = [[
									.#.##.#.
									.######.
									..####..
									...##...
									..####..
									...##...
									..####..
									.######.
								]],
								[ 0x15 ] = [[
									...##...
									..####..
									...##...
									.######.
									#..##..#
									.#.##.#.
									..####..
									.######.
								]],
								[ 0x16 ] = [[
									........
									........
									...##...
									..####..
									..####..
									...##...
									........
									........
								]],
			},
			grvt_cover    = true,
			single_pixel  = false,
			base_address  = 0xE0010000,
			bus = {
				cpu       = "cpu0",
				bus_index = 3,
			},
		},
		{
			type          = "random_source",
			name          = "random_source0",
			left          = 170,
			base_address  = 0xE0060000,
			bus = {
				cpu       = "cpu0",
				bus_index = 3,
			},
		}, 
		{
			type          = "frame_clock",
			name          = "frame_clock0",
			left          = 210,
			base_address  = 0xE0070000,
			bus = {
				cpu       = "cpu0",
				bus_index = 3,
			},
		},
	},
})

local signs = sim.signs
local no_pointer = signs.JUSTMODE_NONE
signs.new("Type letters/numbers to select rows/columns", 306, 20, no_pointer)
signs.new("Backspace deselect piece/remove selection",   306, 35, no_pointer)
signs.new("Pawn always promotes to queen",   306, 50, no_pointer)
signs.new("En passant + Castling supported",   306, 65, no_pointer)
