use super::*;

flat! {
	#[cfg(feature = "bsp")]
	bsp;
}

#[derive(Default)]
pub struct SolidClassesPlugin;
impl Plugin for SolidClassesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<Worldspawn>()
			.register_type::<FuncGroup>()
			.register_type::<FuncGeneric>()
		;

		#[cfg(feature = "bsp")]
		app.register_type::<BspSolidEntity>().register_type::<BspWorldspawn>().register_type::<FuncDetail>();
	}
}

/// The worldspawn entity contains the main structural geometry in the world, and its properties represent map-wide settings. Exactly one must be in every map.
///
/// This is an empty worldspawn implementation to get you up and running with as few lines of code as possible. You will almost certainly override this class with your own at some point.
#[cfg_attr(feature = "bsp", solid_class(base(BspSolidEntity)))]
#[cfg_attr(not(feature = "bsp"), solid_class)]
#[derive(Debug, Clone)]
pub struct Worldspawn;

/// Groups a set of brushes together in-editor. Merged into `worldspawn` on compile.
#[cfg(feature = "bsp")]
#[solid_class(base(BspSolidEntity))]
#[derive(Debug, Clone)]
pub struct FuncGroup;

/// Groups a set of brushes together in-editor.
#[cfg(not(feature = "bsp"))]
#[solid_class(base(Visibility))]
#[derive(Debug, Clone)]
pub struct FuncGroup;

/// Generic brush entity to separate from world geometry. bevy_trenchbroom's version of Quake's `func_wall`.
#[cfg_attr(feature = "bsp", solid_class(base(Visibility, BspSolidEntity)))]
#[cfg_attr(not(feature = "bsp"), solid_class(base(Visibility)))]
#[derive(Debug, Clone)]
pub struct FuncGeneric;
