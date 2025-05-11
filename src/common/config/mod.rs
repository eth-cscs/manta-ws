pub mod types;

use std::{fs::File, io::Read, path::PathBuf};

use config::Config;
use directories::ProjectDirs;
use manta_backend_dispatcher::error::Error;

pub fn get_default_config_path() -> PathBuf {
  // XDG Base Directory Specification
  let project_dirs = ProjectDirs::from(
    "local", /*qualifier*/
    "cscs",  /*organization*/
    "manta", /*application*/
  );

  PathBuf::from(project_dirs.unwrap().config_dir())
}

pub fn get_default_manta_config_file_path() -> PathBuf {
  // XDG Base Directory Specification
  let project_dirs = ProjectDirs::from(
    "local", /*qualifier*/
    "cscs",  /*organization*/
    "manta", /*application*/
  );

  let mut config_file_path = PathBuf::from(project_dirs.unwrap().config_dir());
  config_file_path.push("config.toml");
  config_file_path
}

pub fn get_csm_root_cert_content(file_path: &str) -> Result<Vec<u8>, Error> {
  let mut buf = Vec::new();
  let root_cert_file_rslt = File::open(file_path);

  let file_rslt = if root_cert_file_rslt.is_err() {
    let mut config_path = get_default_config_path();
    config_path.push(file_path);
    File::open(config_path)
  } else {
    root_cert_file_rslt
  };

  match file_rslt {
    Ok(mut file) => {
      let _ = file.read_to_end(&mut buf);

      Ok(buf)
    }
    Err(_) => Err(Error::Message(
      "CA public root file cound not be found.".to_string(),
    )),
  }
}

pub fn get_default_manta_audit_file_path() -> PathBuf {
  // XDG Base Directory Specification
  let project_dirs = ProjectDirs::from(
    "local", /*qualifier*/
    "cscs",  /*organization*/
    "manta", /*application*/
  );

  let mut log_file_path = PathBuf::from(project_dirs.unwrap().data_dir());
  log_file_path.push("manta.log");

  log_file_path
}

pub fn get_default_mgmt_plane_ca_cert_file_path() -> PathBuf {
  // XDG Base Directory Specification
  let project_dirs = ProjectDirs::from(
    "local", /*qualifier*/
    "cscs",  /*organization*/
    "manta", /*application*/
  );

  let mut ca_cert_file_path = PathBuf::from(project_dirs.unwrap().config_dir());
  ca_cert_file_path.push("alps_root_cert.pem");

  ca_cert_file_path
}

/// Get Manta configuration full path. Configuration may be the default one or specified by user.
/// This function also validates if the config file is TOML format
pub async fn get_config_file_path() -> PathBuf {
  // Get config file path from ENV var
  if let Ok(env_config_file_name) = std::env::var("MANTA_CONFIG") {
    let mut env_config_file = std::path::PathBuf::new();
    env_config_file.push(env_config_file_name);
    env_config_file
  } else {
    // Get default config file path ($XDG_CONFIG/manta/config.toml
    get_default_manta_config_file_path()
  }
}

/// Reads configuration parameters related to manta from environment variables or file. If both
/// defiend, then environment variables takes preference
pub async fn get_configuration() -> Result<Config, Error> {
  // Get config file path
  let config_file_path = get_config_file_path().await;

  // If config file does not exists, then use config file generator to create a default config
  // file
  if !config_file_path.exists() {
    // Configuration file does not exists --> create a new configuration file
    return Err(Error::Message(format!(
      "Configuration file '{}' not found. Creating a new one.",
      config_file_path.to_string_lossy()
    )));
  };

  // Process config file and check format (toml) is correct
  let config_file = config::File::new(
    config_file_path
      .to_str()
      .expect("Configuration file name not defined"),
    config::FileFormat::Toml,
  );

  // Process config file
  config::Config::builder()
    .add_source(config_file)
    .add_source(
      ::config::Environment::with_prefix("MANTA")
        .try_parsing(true)
        .prefix_separator("_"),
    )
    .build()
    .map_err(|e| {
      Error::Message(format!("Error processing configuration file: {}", e))
    })
}
