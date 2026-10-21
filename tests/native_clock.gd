## Actual native mixer test: a scene-thread stall must not change cut spacing.
extends SceneTree

var failures := 0

func verify(ok: bool, message: String = "Native class is available") -> void:
	if not ok:
		failures += 1
		printerr(message)

func _initialize() -> void: run.call_deferred()

func run() -> void:
	Engine.max_fps = 120
	var idx := AudioServer.bus_count
	AudioServer.add_bus(idx)
	AudioServer.set_bus_name(idx, "LimiterClockProbe")
	var effect = ClassDB.instantiate("EngineAudioLimiterEffect")
	if effect == null:
		printerr("Native limiter is not installed")
		quit(1)
		return
	AudioServer.add_bus_effect(idx, effect)
	var capture := AudioEffectCapture.new()
	capture.buffer_length = 8.0
	AudioServer.add_bus_effect(idx, capture)
	var rate := int(AudioServer.get_mix_rate())
	var data := PackedByteArray()
	# Longer than the whole test: WAV loop interpolation inserts a boundary
	# transient that would be mistaken for an ignition cut in this DC probe.
	data.resize(rate * 10 * 2)
	for i in rate * 10: data.encode_s16(i * 2, 8192)
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.mix_rate = rate
	stream.data = data
	var player := AudioStreamPlayer.new()
	player.stream = stream
	player.bus = "LimiterClockProbe"
	root.add_child(player)
	effect.configure(true, 0.180, false)
	player.play()
	await create_timer(0.4).timeout
	capture.clear_buffer()
	await create_timer(0.8).timeout
	# Deliberate stall: no scene updates, while audio must keep cutting.
	OS.delay_msec(550)
	await create_timer(0.8).timeout
	var samples := capture.get_buffer(capture.get_frames_available())
	var cuts: Array[int] = []
	var low := 1.0
	var high := 0.0
	for i in range(1, samples.size()):
		low = minf(low, samples[i].x)
		high = maxf(high, samples[i].x)
		if samples[i].x < 0.05 and samples[i - 1].x >= 0.05: cuts.append(i)
	verify(absf(low / high - 0.08) < 0.001, "actual mixer reaches the full ignition-cut depth")
	verify(cuts.size() >= 10, "mixer produces cuts throughout scene stall")
	var expected := roundi(rate * 0.180)
	for i in range(1, cuts.size()):
		verify(cuts[i] - cuts[i - 1] == expected, "every cut has exact sample spacing, even through stall")
	effect.configure(false, 0.180, false)
	# Leave room for the Dummy device's render-ahead blocks as well as the
	# 100 ms audio release; wall timers do not identify a mixer boundary.
	await create_timer(0.30).timeout
	verify(is_equal_approx(effect.current_gain(), 1.0), "release returns unity")
	effect.configure(true, 0.180, false)
	await create_timer(0.16).timeout
	effect.configure(false, 0.180, true)
	await create_timer(0.15).timeout
	verify(is_equal_approx(effect.current_gain(), 1.0), "hard reset clears cut")
	player.stop()
	AudioServer.remove_bus(idx)
	player.queue_free()
	effect = null
	capture = null
	await process_frame
	await create_timer(0.05).timeout
	print("limiter-audio-clock: PASS ", cuts.size(), " exact ", expected, "-sample cycles; 550 ms scene stall; release/reset")
	quit(0 if failures == 0 else 1)
