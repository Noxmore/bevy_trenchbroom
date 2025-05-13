use super::*;

pub struct SolidClassesPlugin;
impl Plugin for SolidClassesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<FuncGroup>()
		;

		#[cfg(feature = "bsp")]
		app.register_type::<FuncDetail>();
	}
}

#[cfg(feature = "bsp")]
#[derive(SolidClass, Component, Reflect, Debug, Clone)]
#[reflect(QuakeClass, Component)]
#[geometry(GeometryProvider::new())]
pub struct FuncDetail;

#[derive(SolidClass, Component, Reflect, Debug, Clone)]
#[reflect(QuakeClass, Component)]
#[cfg_attr(feature = "bsp", base())]
#[cfg_attr(feature = "bsp", geometry(GeometryProvider::new()))]
#[cfg_attr(not(feature = "bsp"), geometry(GeometryProvider::new().smooth_by_default_angle()))] // TODO: Default colliders???
pub struct FuncGroup;