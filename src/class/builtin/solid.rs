use super::*;

pub struct SolidClassesPlugin;
impl Plugin for SolidClassesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<FuncGroup>()
			.register_type::<FuncGeneric>()
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

#[cfg(feature = "bsp")]
#[solid_class(base(BspSolidEntity))]
#[derive(Debug, Clone)]
pub struct FuncGeneric;

#[cfg(not(feature = "bsp"))]
#[solid_class]
#[derive(Debug, Clone)]
pub struct FuncGeneric;
