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
#[solid_class(base(BspSolidEntity))]
#[derive(Debug, Clone)]
pub struct FuncDetail;

#[cfg(feature = "bsp")]
#[solid_class(base(BspSolidEntity))]
#[derive(Debug, Clone)]
pub struct FuncGroup;

#[cfg(not(feature = "bsp"))]
#[solid_class]
#[derive(Debug, Clone)]
pub struct FuncGroup;
