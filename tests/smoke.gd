extends SceneTree
## Integration checks without the game. Run after an editor import.
const Player = preload("res://addons/engine_audio/engine_audio_player.gd")
const Bank = preload("res://addons/engine_audio/engine_audio_bank.gd")
var checks := 0
var failures := 0

func _initialize() -> void:
	run.call_deferred()

func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		printerr("FAIL: " + message)

func run() -> void:
	var initial_buses := AudioServer.bus_count
	var bank: EngineAudioBank = load("res://demo/banks/vulcan.tres")
	check(bank.validation_error().is_empty(), "Demo bank is valid")
	var bad := Bank.new()
	check(not bad.validation_error().is_empty(), "Empty bank is rejected")
	bad = bank.duplicate()
	bad.rpms = PackedFloat32Array([900, 900])
	bad.loops = [bank.loops[0], bank.loops[1]]
	check(not bad.validation_error().is_empty(), "Duplicate RPM is rejected")
	for blend in [0.0, 0.85, 1.2]:
		var copy: EngineAudioBank = bank.duplicate()
		copy.blend = blend
		for rpm in range(910, 8401, 10):
			var weights := copy.weights(rpm)
			var power := 0.0
			for i in weights.size():
				power += weights[i] * weights[i]
				check(is_finite(weights[i]) and weights[i] >= 0, "Blend weights are finite and nonnegative")
			check(absf(power - 1.0) < 0.00001, "Blend power is normalized")
		for anchor in bank.rpms:
			var left := copy.weights(anchor - 0.1)
			var right := copy.weights(anchor + 0.1)
			for i in left.size(): check(absf(left[i] - right[i]) < 0.04, "Continuous blend across RPM anchors")
	var one := Bank.new()
	one.rpms = PackedFloat32Array([900])
	one.loops = [bank.loops[0]]
	check(one.validation_error().is_empty(), "Single-loop banks work")
	check(one.weights(5000)[0] == 1.0, "Single loop gets full weight")
	var a := Player.new()
	a.bank = bank
	root.add_child(a)
	var b := Player.new()
	b.bank = bank
	root.add_child(b)
	await process_frame
	check(a._voices.size() == bank.loops.size(), "All loops loaded")
	check(a._engine_bus != b._engine_bus, "Independent buses per car")
	check(AudioServer.bus_count == initial_buses + 4, "Two buses per car")
	a.rpm = 3200
	a.throttle = 1.0
	a._process(0.016)
	check(absf(a._voices[3].pitch_scale - 3200.0 / bank.rpms[3]) < 0.0001, "Playback pitch follows actual RPM")
	check(a._filter.cutoff_hz > b._filter.cutoff_hz, "Throttle filters are independent")
	a.limiting = true
	for i in 30: a._process(0.016)
	check(a._voices[0].playing, "Limiter does not restart or stop loops")
	a.play_pop()
	check(a._pops[0].playing, "Pop plays")
	a.enabled = false
	a._process(0.016)
	check(AudioServer.is_bus_mute(AudioServer.get_bus_index(a._mix_bus)), "Disabled player is silent")
	check(not a._pops[0].playing, "Disable discards ringing pops")
	a.enabled = true
	a.bank = load("res://demo/banks/comet.tres")
	check(a._phase == 0.0, "Bank switch clears limiter phase")
	await process_frame
	check(a._voices.size() == a.bank.loops.size(), "Bank switch replaces voices")
	check(a._voices[0].stream != a.bank.loops[0], "Loop resources are not mutated")
	a.rpm = NAN
	a.throttle = NAN
	a._process(0.016)
	check(is_finite(a._voices[0].pitch_scale), "Invalid controls do not poison audio")
	check(absf(Player._cut(0.150, 0.180) - 0.08) < 0.0001, "Fallback cut has correct dwell")
	a.free()
	b.free()
	await process_frame
	check(AudioServer.bus_count == initial_buses, "Freeing cars removes their buses")
	# Allow the audio thread to retire stopped playback resources before shutdown.
	await create_timer(0.15).timeout
	print("Engine Audio: %d checks, %d failures" % [checks, failures])
	quit(0 if failures == 0 else 1)
