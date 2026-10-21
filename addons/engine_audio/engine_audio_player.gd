class_name EngineAudioPlayer
extends Node3D
## Add this node to your car, assign a bank, then set RPM and throttle each frame.
## The host game owns gears/physics and decides when to call play_pop().

@export var bank: EngineAudioBank:
	set(value):
		bank = value
		if is_node_ready(): reload_bank()
@export var rpm := 900.0
@export_range(0.0, 1.0) var throttle := 0.0
@export var limiting := false
@export var enabled := true
@export_range(-40.0, 6.0) var volume_db := 0.0
@export_range(0.05, 1.0) var limiter_period := 0.180
@export_range(0.0, 2.0) var pops_volume := 0.5
@export var output_bus: StringName = &"Master"
@export_range(0.1, 100.0) var unit_size := 5.0

var _voices: Array[AudioStreamPlayer3D] = []
var _pops: Array[AudioStreamPlayer3D] = []
var _mix_bus: StringName
var _engine_bus: StringName
var _filter: AudioEffectLowPassFilter
var _native: AudioEffect
var _phase := 0.0
var _depth := 0.0
var _smoothed_rpm := 900.0
var _next_pop := 0
var _rng := RandomNumberGenerator.new()

func _ready() -> void:
	_rng.randomize()
	_mix_bus = StringName("EngineAudio_%d" % get_instance_id())
	_engine_bus = StringName("%s_Tone" % _mix_bus)
	var mix := _add_bus(_mix_bus, output_bus)
	var guard := AudioEffectLimiter.new()
	guard.threshold_db = -2.0
	guard.ceiling_db = -2.0
	guard.soft_clip_db = 0.0
	AudioServer.add_bus_effect(mix, guard)
	var engine := _add_bus(_engine_bus, _mix_bus)
	_filter = AudioEffectLowPassFilter.new()
	_filter.resonance = 0.4
	AudioServer.add_bus_effect(engine, _filter)
	if ClassDB.class_exists("EngineAudioLimiterEffect"):
		_native = ClassDB.instantiate("EngineAudioLimiterEffect")
		AudioServer.add_bus_effect(engine, _native)
	reload_bank()

func uses_native_limiter() -> bool:
	return _native != null

func reload_bank() -> void:
	# Stop immediately; queue_free alone leaves the old voices alive for a frame.
	for voice in _voices + _pops:
		voice.stop()
		voice.queue_free()
	_voices.clear()
	_pops.clear()
	reset()
	if bank == null: return
	var error := bank.validation_error()
	if not error.is_empty():
		push_warning("EngineAudio: " + error)
		return
	_smoothed_rpm = bank.idle_rpm
	for source in bank.loops:
		# Do not mutate shared imported resources or rely on import loop flags.
		var loop := source.duplicate() as AudioStream
		if loop is AudioStreamWAV:
			loop.loop_mode = AudioStreamWAV.LOOP_FORWARD
			loop.loop_begin = 0
			loop.loop_end = roundi(source.get_length() * loop.mix_rate)
		elif loop is AudioStreamOggVorbis:
			loop.loop = true
			loop.loop_offset = 0.0
		var voice := _voice(loop, _engine_bus)
		_voices.append(voice)
		voice.play()
	for i in 6:
		_pops.append(_voice(null, _mix_bus))

func reset() -> void:
	# Call after replay seeks or respawns; also used automatically on bank changes.
	_phase = 0.0
	_depth = 0.0
	if bank != null: _smoothed_rpm = bank.idle_rpm
	for voice in _pops: voice.stop()
	if _native != null: _native.call("configure", false, limiter_period, true)

func play_pop(strength: float = 1.0) -> void:
	if not enabled or not can_process() or bank == null or bank.pops.is_empty() or _pops.is_empty(): return
	if not is_finite(strength) or strength <= 0 or pops_volume <= 0: return
	var voice := _pops[_next_pop % _pops.size()]
	_next_pop += 1
	var sample := bank.pops[_rng.randi_range(0, bank.pops.size() - 1)].duplicate() as AudioStream
	if sample is AudioStreamWAV: sample.loop_mode = AudioStreamWAV.LOOP_DISABLED
	elif sample is AudioStreamOggVorbis: sample.loop = false
	voice.stream = sample
	voice.unit_size = unit_size
	voice.volume_db = -8.0 + linear_to_db(clampf(strength, 0.0, 1.0) * pops_volume)
	voice.play()

func _process(delta: float) -> void:
	if _voices.is_empty(): return
	var mix := AudioServer.get_bus_index(_mix_bus)
	AudioServer.set_bus_mute(mix, not enabled)
	AudioServer.set_bus_volume_db(mix, clampf(volume_db, -80, 6) if is_finite(volume_db) else -80)
	var destination := output_bus if AudioServer.get_bus_index(output_bus) >= 0 else &"Master"
	# Never allow a routing cycle back into this player's own buses.
	if destination == _mix_bus or destination == _engine_bus: destination = &"Master"
	AudioServer.set_bus_send(mix, destination)
	var period := clampf(limiter_period, 0.05, 1.0) if is_finite(limiter_period) else 0.180
	var active := limiting and enabled
	var cut_gain := 1.0
	if _native != null:
		_native.call("configure", active, period, not enabled)
	else:
		# Portable fallback. For exact cuts during frame stalls install the native addon.
		_depth = move_toward(_depth, 1.0 if active else 0.0, delta / (0.06 if active else 0.10))
		_phase = fmod(_phase + delta, period) if _depth > 0 else 0.0
		cut_gain = lerpf(1.0, _cut(_phase, period), _depth)
	if not enabled:
		reset()
	var real := clampf(rpm, bank.idle_rpm, bank.redline_rpm * 1.2) if is_finite(rpm) else bank.idle_rpm
	_smoothed_rpm = lerpf(_smoothed_rpm, real, 1.0 - exp(-delta / 0.04)) if active else real
	var load_amount := clampf(throttle, 0, 1) if is_finite(throttle) else 0.0
	var normalized := inverse_lerp(bank.idle_rpm, bank.redline_rpm, _smoothed_rpm)
	_filter.cutoff_hz = lerpf(900, 8000, clampf(load_amount * 0.8 + normalized * 0.3, 0, 1))
	var weights := bank.weights(_smoothed_rpm)
	var gain := (0.45 + 0.55 * load_amount) * db_to_linear(bank.gain_db) * cut_gain
	for i in _voices.size():
		var voice := _voices[i]
		voice.unit_size = unit_size
		voice.stream_paused = not enabled
		voice.pitch_scale = clampf(_smoothed_rpm / bank.rpms[i], 0.25, 4.0)
		voice.volume_db = linear_to_db(maxf(weights[i] * gain, 0.00001))
		# Spatial gain caps must follow cuts too, or close listeners hide the bounce.
		voice.max_db = linear_to_db(maxf(cut_gain, 0.00001))

static func _cut(seconds: float, period: float) -> float:
	var pickup := period - 0.001
	var coast := pickup - minf(0.050, period * 0.6)
	var fade := coast - 0.005
	if seconds < fade: return 1.0
	if seconds < coast: return lerpf(1.0, 0.08, smoothstep(fade, coast, seconds))
	if seconds < pickup: return 0.08
	return lerpf(0.08, 1.0, smoothstep(pickup, period, seconds))

func _voice(stream: AudioStream, bus_name: StringName) -> AudioStreamPlayer3D:
	var voice := AudioStreamPlayer3D.new()
	voice.stream = stream
	voice.bus = bus_name
	voice.volume_db = -80
	voice.max_db = 0
	voice.unit_size = unit_size
	add_child(voice)
	return voice

func _add_bus(bus_name: StringName, destination: StringName) -> int:
	var index := AudioServer.bus_count
	AudioServer.add_bus(index)
	AudioServer.set_bus_name(index, bus_name)
	AudioServer.set_bus_send(index, destination if AudioServer.get_bus_index(destination) >= 0 else &"Master")
	return index

func _exit_tree() -> void:
	for voice in _voices + _pops:
		if is_instance_valid(voice): voice.stop()
	if _native != null: _native.call("configure", false, 0.180, true)
	for bus_name in [_engine_bus, _mix_bus]:
		var index := AudioServer.get_bus_index(bus_name)
		if index > 0: AudioServer.remove_bus(index)
