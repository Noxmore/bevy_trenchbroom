use super::*;

flat! {
	#[cfg(feature = "bsp")]
	bsp;
}

#[derive(Default)]
pub struct PointClassesPlugin;
impl Plugin for PointClassesPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "bsp")]
		app.register_type::<InfoPlayerStart>();
	}
}
