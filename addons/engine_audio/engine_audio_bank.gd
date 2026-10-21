class_name EngineAudioBank
extends Resource
## A set of recordings at known RPMs. Loops and RPMs must be in ascending order.
## Use explicit resources so Godot includes every recording in exported games.

@export var label := "Engine"
@export var idle_rpm := 900.0
@export var redline_rpm := 7000.0
@export var rpms := PackedFloat32Array()
@export var loops: Array[AudioStream] = []
## Zero blends the nearest two loops; 1.2 gives broader, smoother overlap.
@export_range(0.0, 3.0) var blend := 1.2
@export_range(-40.0, 6.0) var gain_db := -8.0
@export var pops: Array[AudioStream] = []

func validation_error() -> String:
	if not is_finite(idle_rpm) or not is_finite(redline_rpm) or idle_rpm <= 0 or redline_rpm <= idle_rpm:
		return "Use positive idle RPM and a higher redline RPM."
	if loops.is_empty() or loops.size() != rpms.size():
		return "Supply one RPM for each loop (at least one loop)."
	var previous := 0.0
	for i in rpms.size():
		if not is_finite(rpms[i]) or rpms[i] <= previous:
			return "Loop RPMs must be positive, unique and in ascending order."
		if not loops[i] is AudioStreamWAV and not loops[i] is AudioStreamOggVorbis:
			return "Engine loops must be imported WAV or Ogg resources."
		if loops[i].get_length() <= 0:
			return "Engine loops must contain audio."
		previous = rpms[i]
	for pop in pops:
		if pop == null or pop.get_length() <= 0:
			return "Pop resources must contain audio."
	if not is_finite(blend) or blend < 0 or not is_finite(gain_db):
		return "Blend and gain must be finite; blend cannot be negative."
	return ""

func weights(rpm: float) -> PackedFloat32Array:
	# Taken from Hardpack Rally's equal-power / Gaussian log-RPM blending.
	var result := PackedFloat32Array()
	result.resize(rpms.size())
	if rpms.is_empty(): return result
	var real := maxf(rpm, 1.0)
	if blend <= 0 or rpms.size() < 3:
		var i := 0
		while i < rpms.size() - 1 and rpms[i + 1] < real: i += 1
		if i < rpms.size() - 1 and real > rpms[i]:
			var t := clampf(log(real / rpms[i]) / log(rpms[i + 1] / rpms[i]), 0, 1)
			result[i] = sqrt(1.0 - t)
			result[i + 1] = sqrt(t)
		else:
			result[i] = 1.0
		return result
	var gaps: Array[float] = []
	for i in rpms.size() - 1: gaps.append(log(rpms[i + 1] / rpms[i]))
	gaps.sort()
	var sigma := maxf(gaps[gaps.size() / 2], 0.02) * blend
	var power := 0.0
	var best := 0
	for i in rpms.size():
		var distance := log(real / rpms[i]) / sigma
		result[i] = exp(-0.5 * distance * distance)
		power += result[i] * result[i]
		if absf(log(real / rpms[i])) < absf(log(real / rpms[best])): best = i
	if power < 0.000001:
		result.fill(0.0)
		result[best] = 1.0
	else:
		for i in result.size(): result[i] /= sqrt(power)
	return result
