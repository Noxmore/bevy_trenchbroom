use super::*;

/// Errors that can occur when getting the [default TrenchBroom game config path](https://trenchbroom.github.io/manual/latest/#game_configuration_files).
/// Such errors typically occur when TrenchBroom is not installed or installed in a non-standard location.
#[derive(thiserror::Error, Debug)]
pub enum DefaultTrenchBroomUserdataDirError {
	#[error("Unsupported target OS: {0}")]
	UnsupportedOs(String),
	#[error("Home directory not found")]
	HomeDirNotFound,
	#[error("TrenchBroom user data not found at {}. Have you installed TrenchBroom?", .0.display())]
	UserDataNotFound(PathBuf),
}

/// Errors that can occur when trying to use [`TrenchBroomConfig::write_game_config_to_default_directory`]
#[derive(thiserror::Error, Debug)]
pub enum DefaultTrenchBroomGameConfigError {
	#[error("{0}")]
	UserdataDirError(DefaultTrenchBroomUserdataDirError),
	#[error("Failed to create game config directory: {0}")]
	CreateDirError(io::Error),
	#[error("Failed to write config to {}: {error}", path.display())]
	WriteError { error: io::Error, path: PathBuf },
}

/// Errors that can occur when trying to use [`TrenchBroomConfig::add_game_to_preferences_in_default_directory`]
#[derive(thiserror::Error, Debug)]
pub enum DefaultTrenchBroomPreferencesError {
	#[error(
		"Please set a name for your TrenchBroom config. \
		If you have, make sure you call `write_preferences` after the app is built. (e.g. In a startup system)"
	)]
	UninitializedError,
	#[error("{0}")]
	UserdataDirError(DefaultTrenchBroomUserdataDirError),
	#[error("Failed to read preferences from {}: {error}", path.display())]
	ReadError { error: io::Error, path: PathBuf },
	#[error("Failed to deserialize preferences to JSON from {}: {error}", path.display())]
	DeserializeError { error: serde_json::Error, path: PathBuf },
	#[error("Failed read from preferences at {} as a JSON object", path.display())]
	JsonObjectError { path: PathBuf },
	#[error("Failed to serialize preferences back to JSON: {error}")]
	SerializeError { error: serde_json::Error },
	#[error("Failed to find path to current directory: {error}")]
	CurrentDirError { error: io::Error },
	#[error("Failed to convert path {path} to string. Is the path of the current directory not valid UTF-8?", path = path.display())]
	PathToStringError { path: PathBuf },
	#[error("Failed to write preferences to {}: {error}", path.display())]
	WriteError { error: io::Error, path: PathBuf },
}

impl TrenchBroomConfig {
	/// Writes the configuration into the [default TrenchBroom game config path](https://trenchbroom.github.io/manual/latest/#game_configuration_files).
	///
	/// If you want to customize the path, use [`write_game_config`](Self::write_game_config) instead.
	pub fn write_game_config_to_default_directory(&self) -> Result<(), DefaultTrenchBroomGameConfigError> {
		let path = self.get_default_trenchbroom_game_config_path()?;
		if !path.exists() {
			let err = fs::create_dir_all(&path);
			if let Err(err) = err {
				return Err(DefaultTrenchBroomGameConfigError::CreateDirError(err));
			}
		}

		if let Err(err) = self.write_game_config(&path) {
			return Err(DefaultTrenchBroomGameConfigError::WriteError { error: err, path });
		}

		Ok(())
	}

	/// Adds the game to the preferences file by using the default TrenchBroom user data directory.
	///
	/// If you want to customize the path, use [`add_game_to_preferences`](Self::add_game_to_preferences) instead.
	pub fn add_game_to_preferences_in_default_directory(&self) -> Result<(), DefaultTrenchBroomPreferencesError> {
		let path = self
			.get_default_preferences_path()
			.map_err(DefaultTrenchBroomPreferencesError::UserdataDirError)?;

		self.add_game_to_preferences(&path)?;
		Ok(())
	}

	fn get_default_trenchbroom_userdata_path(&self) -> Result<PathBuf, DefaultTrenchBroomUserdataDirError> {
		let trenchbroom_userdata = if cfg!(target_os = "linux") {
			#[allow(deprecated)] // No longer deprecated starting from 1.86
			env::home_dir().map(|path| path.join(".TrenchBroom"))
		} else if cfg!(target_os = "windows") {
			env::var("APPDATA").ok().map(|path| PathBuf::from(path).join("TrenchBroom"))
		} else if cfg!(target_os = "macos") {
			#[allow(deprecated)] // No longer deprecated starting from 1.86
			env::home_dir().map(|path| path.join("Library").join("Application Support").join("TrenchBroom"))
		} else {
			return Err(DefaultTrenchBroomUserdataDirError::UnsupportedOs(env::consts::OS.to_string()));
		};

		let Some(trenchbroom_userdata) = trenchbroom_userdata else {
			return Err(DefaultTrenchBroomUserdataDirError::HomeDirNotFound);
		};

		if !trenchbroom_userdata.exists() {
			return Err(DefaultTrenchBroomUserdataDirError::UserDataNotFound(trenchbroom_userdata));
		}

		Ok(trenchbroom_userdata)
	}

	/// Gets $TRENCHBROOM_DIR/Preferences.json
	fn get_default_preferences_path(&self) -> Result<PathBuf, DefaultTrenchBroomUserdataDirError> {
		let trenchbroom_userdata = self.get_default_trenchbroom_userdata_path()?;
		let preferences_path = trenchbroom_userdata.join("Preferences.json");

		Ok(preferences_path)
	}

	/// Gets $TRENCHBROOM_DIR/games/$NAME
	fn get_default_trenchbroom_game_config_path(&self) -> Result<PathBuf, DefaultTrenchBroomGameConfigError> {
		let trenchbroom_userdata = self
			.get_default_trenchbroom_userdata_path()
			.map_err(DefaultTrenchBroomGameConfigError::UserdataDirError)?;
		let trenchbroom_game_config = trenchbroom_userdata.join("games").join(&self.name);
		Ok(trenchbroom_game_config)
	}

	/// Adds the game to the preferences file by using the current directory as the game path.
	/// It is your choice when to do this in your application, and where the preferences file is located.
	///
	/// If you have a standard TrenchBroom installation, you can use [`add_game_to_preferences_in_default_directory`](Self::add_game_to_preferences_in_default_directory) instead to use the default location.
	pub fn add_game_to_preferences(&self, path: impl AsRef<Path>) -> Result<(), DefaultTrenchBroomPreferencesError> {
		if self.name.is_empty() {
			return Err(DefaultTrenchBroomPreferencesError::UninitializedError);
		}

		let path = path.as_ref();
		// read the preferences file as json
		let preferences = if path.exists() {
			fs::read_to_string(path).map_err(|err| DefaultTrenchBroomPreferencesError::ReadError {
				error: err,
				path: path.to_path_buf(),
			})?
		} else {
			"{}".to_string()
		};

		let mut preferences: serde_json::Value =
			serde_json::from_str(&preferences).map_err(|err| DefaultTrenchBroomPreferencesError::DeserializeError {
				error: err,
				path: path.to_path_buf(),
			})?;
		let preferences = preferences
			.as_object_mut()
			.ok_or(DefaultTrenchBroomPreferencesError::JsonObjectError { path: path.to_path_buf() })?;

		// add the game config to the preferences
		let key = format!("Games/{}/Path", self.name);
		let game_dir = env::current_dir().map_err(|err| DefaultTrenchBroomPreferencesError::CurrentDirError { error: err })?;
		let game_dir = game_dir
			.to_str()
			.ok_or_else(|| DefaultTrenchBroomPreferencesError::PathToStringError { path: game_dir.clone() })?
			.to_string();

		preferences.insert(key, serde_json::Value::String(game_dir));
		let preferences =
			serde_json::to_string_pretty(&preferences).map_err(|err| DefaultTrenchBroomPreferencesError::SerializeError { error: err })?;

		// write the preferences file back to the same path
		fs::write(path, preferences).map_err(|err| DefaultTrenchBroomPreferencesError::WriteError {
			error: err,
			path: path.to_path_buf(),
		})?;

		info!("Successfully wrote TrenchBroom preferences to {}", path.display());

		Ok(())
	}

	/// Writes the game configuration into a directory, it is your choice when to do this in your application, and where you want to save the config to.
	///
	/// If you have a standard TrenchBroom installation, you can use [`write_game_config_to_default_directory`](Self::write_game_config_to_default_directory) instead to use the default location.
	pub fn write_game_config(&self, directory: impl AsRef<Path>) -> io::Result<()> {
		if self.name.is_empty() {
			return Err(io::Error::other(
				"Please set a name for your TrenchBroom config. \
				If you have, make sure you call `write_game_config` after the app is built. (e.g. In a startup system)",
			));
		}

		let folder = directory.as_ref();

		//////////////////////////////////////////////////////////////////////////////////
		//// GAME CONFIGURATION && ICON
		//////////////////////////////////////////////////////////////////////////////////

		// The game config file is basically json, so we can get 99% of the way there by just creating a json object.
		let mut json = json::object! {
			"version": self.tb_format_version,
			"name": self.name.clone(),
			"fileformats": self.file_formats.iter().map(|format| json::object! { "format": format.config_str() }).collect::<Vec<_>>(),
			"filesystem": {
				"searchpath": self.assets_path.s(),
				"packageformat": { "extension": self.package_format.extension(), "format": self.package_format.format() }
			},
			"materials": {
				"root": self.material_root.s(),
				// .D is required for WADs to work
				"extensions": [".D", self.texture_extension.clone()],
				"palette": self.texture_pallette.s(),
				"attribute": "wad",
				"excludes": self.texture_exclusions.clone(),
			},
			"entities": {
				"definitions": [ format!("{}.fgd", self.name) ],
				"defaultcolor": format!("{} {} {} {}", self.entity_default_color.x, self.entity_default_color.y, self.entity_default_color.z, self.entity_default_color.w),
				"scale": "$$scale$$", // Placeholder
				"setDefaultProperties": self.entity_set_default_properties,
			},
			"tags": {
				"brush": self.brush_tags.iter().map(|tag| tag.to_json("classname")).collect::<Vec<_>>(),
				"brushface": self.face_tags.iter().map(|tag| tag.to_json("material")).collect::<Vec<_>>()
			},
		};

		if let Some(icon) = &self.icon {
			fs::write(folder.join("Icon.png"), icon)?;
			json.insert("icon", "Icon.png").unwrap();
		}

		let insert_defaults = self.default_face_attributes.is_any_set();
		if insert_defaults || !self.surface_flags.is_empty() || !self.content_flags.is_empty() {
			let mut face_attributes = json::object! {
				"surfaceflags": self.surface_flags.as_slice(),
				"contentflags": self.content_flags.as_slice(),
			};

			if insert_defaults {
				face_attributes.insert("defaults", &self.default_face_attributes).unwrap();
			}

			json.insert("faceattribs", face_attributes).unwrap();
		}

		if let Some(bounds) = self.soft_map_bounds {
			json.insert("softMapBounds", bounds.fgd_to_string_unquoted()).unwrap();
		}

		let mut buf = json.pretty(4);

		if let Some(expression) = &self.get_entity_scale_expression() {
			buf = buf.replace("\"$$scale$$\"", expression);
		}

		fs::write(folder.join("GameConfig.cfg"), buf)?;

		//////////////////////////////////////////////////////////////////////////////////
		//// FGD
		//////////////////////////////////////////////////////////////////////////////////

		fs::write(folder.join(format!("{}.fgd", self.name)), self.to_fgd())?;

		info!("Successfully wrote TrenchBroom game config to {}", folder.display());

		Ok(())
	}
}
