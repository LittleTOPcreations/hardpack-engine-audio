extends SceneTree
## Capture the actual output: a valid source alone does not prove audible playback.
const Player = preload("res://addons/engine_audio/engine_audio_player.gd")
var failures := 0

func _initialize() -> void: run.call_deferred()

func check(ok: bool, message: String) -> void:
	if not ok:
		failures += 1
		printerr("FAIL: " + message)

func run() -> void:
	Engine.max_fps = 120
	var world := Node3D.new()
	root.add_child(world)
	var listener := Camera3D.new()
	listener.position.z = 3
	world.add_child(listener)
	listener.make_current()
	var player := Player.new()
	player.bank = load("res://demo/banks/vulcan.tres")
	world.add_child(player)
	player.rpm = 3500
	player.throttle = 0.7
	var capture := AudioEffectCapture.new()
	capture.buffer_length = 2.0
	var slot := AudioServer.get_bus_effect_count(0)
	AudioServer.add_bus_effect(0, capture)
	await create_timer(0.3).timeout
	capture.clear_buffer()
	await create_timer(0.4).timeout
	var samples := capture.get_buffer(capture.get_frames_available())
	var stats := measure(samples)
	check(samples.size() > 1000, "Mixer supplies samples")
	check(stats.x > 0.001, "Engine is audible")
	check(stats.y < 0.9, "Demo engine has peak headroom")
	paused = true
	await create_timer(0.3).timeout
	capture.clear_buffer()
	await create_timer(0.2).timeout
	check(measure(capture.get_buffer(capture.get_frames_available())).y < 0.00001, "Tree pause is silent")
	paused = false
	await create_timer(0.3).timeout
	capture.clear_buffer()
	await create_timer(0.2).timeout
	check(measure(capture.get_buffer(capture.get_frames_available())).x > 0.001, "Tree resume restores audio")
	player.limiting = true
	player.play_pop()
	await create_timer(0.3).timeout
	capture.clear_buffer()
	await create_timer(0.5).timeout
	var limited := measure(capture.get_buffer(capture.get_frames_available()))
	check(limited.x > 0.0001 and limited.y < 0.95, "Limiter and pops produce bounded audio")
	player.enabled = false
	await create_timer(0.3).timeout
	capture.clear_buffer()
	await create_timer(0.2).timeout
	var silent := measure(capture.get_buffer(capture.get_frames_available()))
	check(silent.y < 0.00001, "Disable is silent at the listener")
	player.enabled = true
	player.limiting = false
	player.bank = load("res://demo/banks/goliath.tres")
	await create_timer(0.3).timeout
	capture.clear_buffer()
	await create_timer(0.2).timeout
	var switched := measure(capture.get_buffer(capture.get_frames_available()))
	check(switched.x > 0.001, "New bank is audible after switching")
	AudioServer.remove_bus_effect(0, slot)
	world.free()
	await create_timer(0.15).timeout
	print("Mixer: RMS %.5f, peak %.5f; %d failures" % [stats.x, stats.y, failures])
	quit(0 if failures == 0 else 1)

func measure(samples: PackedVector2Array) -> Vector2:
	var power := 0.0
	var peak := 0.0
	for sample in samples:
		if not is_finite(sample.x) or not is_finite(sample.y):
			check(false, "Samples must be finite")
			return Vector2.ZERO
		power += sample.length_squared() * 0.5
		peak = maxf(peak, maxf(absf(sample.x), absf(sample.y)))
	return Vector2(sqrt(power / maxi(samples.size(), 1)), peak)
