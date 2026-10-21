extends Node3D
## A bench demo: no car, physics, autoloads or project-specific dependencies.

const Player = preload("res://addons/engine_audio/engine_audio_player.gd")
const BANKS = [preload("res://demo/banks/vulcan.tres"), preload("res://demo/banks/comet.tres"), preload("res://demo/banks/goliath.tres")]
var engine: EngineAudioPlayer
var rpm_slider: HSlider
var throttle_slider: HSlider
var rpm_label: Label
var status: Label
var sweep := false
var elapsed := 0.0
var limiter: CheckButton

func _ready() -> void:
	var camera := Camera3D.new()
	camera.position.z = 3.0
	add_child(camera)
	camera.make_current()
	engine = Player.new()
	engine.bank = BANKS[0]
	add_child(engine)
	var canvas := CanvasLayer.new()
	add_child(canvas)
	var background := ColorRect.new()
	background.color = Color("0c131b")
	background.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	canvas.add_child(background)
	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "right", "top", "bottom"]: margin.add_theme_constant_override("margin_" + side, 42)
	canvas.add_child(margin)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 18)
	margin.add_child(box)
	_text(box, "HARDPACK  /  ENGINE AUDIO", 18, Color("f0b45a"))
	_text(box, "A car sound. A few inputs.", 38)
	_text(box, "Move the revs, open the throttle, and hear the loops blend.", 18, Color("9baebe"))
	var select := OptionButton.new()
	for bank in BANKS: select.add_item(bank.label)
	select.item_selected.connect(_select_bank)
	box.add_child(select)
	rpm_label = _text(box, "900 RPM", 52, Color("f0b45a"))
	rpm_slider = _slider(box, "ENGINE SPEED", 900, 7000, 900)
	rpm_slider.value_changed.connect(func(_value): sweep = false)
	throttle_slider = _slider(box, "THROTTLE / LOAD", 0, 1, 0.3)
	var buttons := HBoxContainer.new()
	buttons.add_theme_constant_override("separation", 16)
	box.add_child(buttons)
	_button(buttons, "Sweep the rev range", func(): sweep = not sweep; elapsed = 0.0)
	_button(buttons, "Exhaust pop", func(): engine.play_pop())
	limiter = CheckButton.new()
	limiter.text = "Rev limiter"
	buttons.add_child(limiter)
	var mute := CheckButton.new()
	mute.text = "Mute"
	mute.toggled.connect(func(value): engine.enabled = not value)
	buttons.add_child(mute)
	status = _text(box, "", 16, Color("9baebe"))
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(spacer)
	_text(box, "IN YOUR GAME", 14, Color("f0b45a"))
	_text(box, "engine.rpm = car_rpm   ·   engine.throttle = accelerator\nengine.limiting = at_redline   ·   engine.play_pop()", 18)
	_text(box, "Generated demo tones • Replace the bank with your own WAV or Ogg loops", 15, Color("9baebe"))

func _process(delta: float) -> void:
	if sweep:
		elapsed += delta
		var position_in_sweep := (1.0 - cos(elapsed * TAU / 10.0)) * 0.5
		rpm_slider.set_value_no_signal(lerpf(engine.bank.idle_rpm, engine.bank.redline_rpm, position_in_sweep))
	engine.rpm = rpm_slider.value
	engine.throttle = throttle_slider.value
	engine.limiting = limiter.button_pressed
	rpm_label.text = "%s RPM" % int(engine.rpm)
	status.text = "Limiter: %s   •   %d RPM loops   •   %.0f%% throttle" % ["audio-clock native effect" if engine.uses_native_limiter() else "portable frame-timed effect", engine.bank.loops.size(), engine.throttle * 100]

func _select_bank(index: int) -> void:
	engine.bank = BANKS[index]
	rpm_slider.min_value = engine.bank.idle_rpm
	rpm_slider.max_value = engine.bank.redline_rpm
	rpm_slider.value = engine.bank.idle_rpm
	sweep = false

func _text(parent: Node, value: String, size: int, color := Color("e6edf3")) -> Label:
	var label := Label.new()
	label.text = value
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", color)
	parent.add_child(label)
	return label

func _slider(parent: Node, title: String, low: float, high: float, value: float) -> HSlider:
	_text(parent, title, 14, Color("9baebe"))
	var slider := HSlider.new()
	slider.min_value = low
	slider.max_value = high
	slider.step = 0.01 if high <= 1 else 1.0
	slider.value = value
	slider.custom_minimum_size.y = 26
	parent.add_child(slider)
	return slider

func _button(parent: Node, title: String, action: Callable) -> void:
	var button := Button.new()
	button.text = title
	button.custom_minimum_size.y = 42
	button.pressed.connect(action)
	parent.add_child(button)
