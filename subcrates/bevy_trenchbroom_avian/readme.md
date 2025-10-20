Official integration between `bevy_trenchbroom` and the Avian physics engine.

The version of this crate will the same as the version of `bevy_trenchbroom` it supports.

# Usage
Simply add a `TrenchBroomPhysicsPlugin` with the provided `AvianPhysicsBackend` to your app.
```rust
use bevy::prelude::*;
use bevy_trenchbroom_avian::AvianPhysicsBackend;
use bevy_trenchbroom::prelude::*;

App::new()
	// ...
	.add_plugins(TrenchBroomPhysicsPlugin::new(AvianPhysicsBackend))
	// ...
;
```

# Version support table
| Bevy | Avian | bevy_trenchbroom_avian |
|------|-------|------------------------|
| 0.17 | 0.4   | 0.10                   |
