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
			.register_type::<FuncGroup>()
			.register_type::<FuncGeneric>()
		;

		#[cfg(feature = "bsp")]
		app.register_type::<BspSolidEntity>().register_type::<BspWorldspawn>().register_type::<FuncDetail>();
	}
}

/// Groups a set of brushes together in-editor. Merged into `worldspawn` on compile.
#[cfg(feature = "bsp")]
#[solid_class(base(BspSolidEntity))]
#[derive(Debug, Clone)]
pub struct FuncGroup;

/// Groups a set of brushes together in-editor.
#[cfg(not(feature = "bsp"))]
#[solid_class]
#[derive(Debug, Clone)]
pub struct FuncGroup;

/// Generic brush entity to separate from world geometry. bevy_trenchbroom's version of Quake's `func_wall`.
#[cfg_attr(feature = "bsp", solid_class(base(BspSolidEntity)))]
#[cfg_attr(not(feature = "bsp"), solid_class)]
#[derive(Debug, Clone)]
pub struct FuncGeneric;
