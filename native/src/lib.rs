use godot::prelude::*;
mod limiter;

struct EngineAudioExtension;

#[gdextension]
unsafe impl ExtensionLibrary for EngineAudioExtension {}
