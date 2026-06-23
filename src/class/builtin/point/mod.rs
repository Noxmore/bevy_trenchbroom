use super::*;

#[cfg(feature = "bsp")]
mod bsp;
#[cfg(feature = "bsp")]
pub use bsp::*;

#[derive(Default)]
pub struct PointClassesPlugin;
impl Plugin for PointClassesPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "bsp")]
		app.register_type::<InfoPlayerStart>();
	}
}
