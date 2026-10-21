extends SceneTree
## Render the real demo for documentation, then exit. Requires a display renderer.
func _initialize() -> void: run.call_deferred()

func run() -> void:
	var scene = load("res://demo/demo.tscn").instantiate()
	root.add_child(scene)
	await process_frame
	scene.rpm_slider.value = 4200
	scene.throttle_slider.value = 0.7
	await process_frame
	await RenderingServer.frame_post_draw
	DirAccess.make_dir_recursive_absolute("res://docs")
	var error := root.get_texture().get_image().save_png("res://docs/demo.png")
	scene.engine.enabled = false
	scene.free()
	await create_timer(0.15).timeout
	print("Demo screenshot: ", error)
	quit(error)
