use std::{pin::Pin, sync::Arc};

use bevy::{
	asset::LoadContext,
	ecs::{schedule::ScheduleLabel, system::ScheduleSystem},
	prelude::*,
};
use bevy_reflect::{FromType, TypeData};
use nil::prelude::*;

use crate::class::QuakeClassSpawnView;

pub struct SceneSystemPlugin;
impl Plugin for SceneSystemPlugin {
	fn build(&self, app: &mut App) {
		app
			.init_resource::<SceneSchedules>()
			// .register_type_data::<AssetServer, ReflectSceneTemp>()
		;
	}
}

#[derive(Deref, DerefMut)]
pub struct SceneLoadContext<'i>(LoadContext<'i>);
impl SystemInput for SceneLoadContext<'_> {
	type Param<'i> = SceneLoadContext<'i>;
	type Inner<'i> = LoadContext<'i>;

	fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
		SceneLoadContext(this)
	}
}

pub struct LoadContextRes {
	ptr: LoadContext<'static>,
}
impl LoadContextRes {
	pub unsafe fn new(load_context: LoadContext) -> Self {
		Self {
			ptr: unsafe { std::mem::transmute(load_context) },
		}
	}
}

/* #[derive(Resource)]
pub struct QuakeClassSpawnViewRes {
	ptr: *mut QuakeClassSpawnView,
}
 */
// pub trait SceneScheduleLabel

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostSpawn;

// #[derive(Component)]
pub struct ReflectSceneTemp;
impl TypeData for ReflectSceneTemp {
	fn clone_type_data(&self) -> Box<dyn TypeData> {
		Box::new(Self)
	}
}
impl<T> FromType<T> for ReflectSceneTemp {
	fn from_type() -> Self {
		Self
	}
}

#[derive(Resource, Default, Clone)]
pub struct SceneSchedules {
	pub schedules: Arc<RwLock<Schedules>>,
}

pub trait SceneSystemAppExt {
	fn add_scene_systems<M>(&mut self, schedule: impl ScheduleLabel, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self;
}
impl SceneSystemAppExt for App {
	fn add_scene_systems<M>(&mut self, schedule: impl ScheduleLabel, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self {
		let scene_schedules = self.world().resource::<SceneSchedules>();

		let mut schedules = scene_schedules.schedules.write();
		schedules.add_systems(schedule, systems);
		drop(schedules);

		self
	}
}
