use axum::http::HeaderMap;
use axum::{http::StatusCode, response::Response};
use std::{fmt::Display, fs::File, io::Read, path::PathBuf};

use config::Config;
use directories::ProjectDirs;

use crate::http_response::*;
use crate::log::*;

pub struct SiteCfg {
  pub site: String,
  pub shasta_base_url: String,
  pub shasta_root_cert: Vec<u8>,
}

pub struct ReqCfg {
  pub auth_token: String,
  pub site_cfg: SiteCfg,
}

pub fn get_req_cfg(
  headers: &HeaderMap,
  site: String,
) -> Result<ReqCfg, Response> {
  let site_cfg = match get_site_cfg(site) {
    Ok(good) => good,
    Err(e) => {
      return Err(bad_config(&e));
    }
  };

  let auth_token = match get_auth_token(headers) {
    Ok(good) => good,
    Err(e) => {
      return Err(unauthorized_access(&e));
    }
  };

  let req_cfg = ReqCfg {
    auth_token,
    site_cfg,
  };

  Ok(req_cfg)
}

fn get_auth_token(headers: &HeaderMap) -> Result<String, String> {
  let auth_field = match headers.get("authorization") {
    Some(good) => good,
    None => {
      let e = format!("Unauthorized access");
      return Err(e.to_string());
    }
  };

  let auth_field_str = match auth_field.to_str() {
    Ok(good) => good,
    Err(e) => {
      return Err(e.to_string());
    }
  };

  let auth_token = match auth_field_str.split(" ").nth(1) {
    Some(good) => good.to_string(),
    None => {
      let e = format!("Unauthorized access");
      return Err(e.to_string());
    }
  };

  Ok(auth_token)
}

fn get_site_cfg(site: String) -> Result<SiteCfg, String> {
  let settings = get_configuration()?;
  let sites_table = match settings.get_table("sites") {
    Ok(good) => good,
    Err(e) => {
      return Err(e.to_string());
    }
  };

  let site_value = match sites_table.get(&site) {
    Some(good) => good.clone().into_table(),
    None => {
      let e = format!("site {site} not found.");
      return Err(e.to_string());
    }
  };

  let site_table = match site_value {
    Ok(good) => good,
    Err(e) => {
      return Err(e.to_string());
    }
  };

  let shasta_base_url = match site_table.get("shasta_base_url") {
    Some(good) => good.to_string(),
    None => {
      let e = format!("shasta_base_url for site {site} not found.");
      return Err(e.to_string());
    }
  };

  let shasta_root_cert = get_csm_root_cert_content(&site)?;

  let site_cfg = SiteCfg {
    site,
    shasta_base_url: shasta_base_url.to_string(),
    shasta_root_cert,
  };

  Ok(site_cfg)
}

fn get_csm_root_cert_content(site: &str) -> Result<Vec<u8>, String> {
  let mut config_path = get_configuration_file_path()?;

  config_path.push(site.to_string() + "_root_cert.pem");

  let mut buf = Vec::new();
  let root_cert_file_rslt = File::open(config_path);

  let _ = match root_cert_file_rslt {
    Ok(mut file) => file.read_to_end(&mut buf),
    Err(_) => {
      return Err("Root cert file for CSM not found.".to_string());
    }
  };

  Ok(buf)
}

fn get_configuration_file_path() -> Result<PathBuf, String> {
  // XDG Base Directory Specification
  let project_dirs = ProjectDirs::from(
    "local", /*qualifier*/
    "cscs",  /*organization*/
    "manta", /*application*/
  );

  let dirs = match project_dirs {
    Some(good_dirs) => good_dirs,
    None => {
      let reason = "cannot find a valid home directory";
      return Err(reason.to_string());
    }
  };

  let ret = PathBuf::from(dirs.config_dir());
  Ok(ret)
}

/// Reads configuration parameters related to manta from environment variables or file. If both
/// defiend, then environment variables takes preference
fn get_configuration() -> Result<Config, String> {
  let mut config_path = get_configuration_file_path()?;
  config_path.push("config.toml"); // ~/.config/manta/config is the file

  let built_config = ::config::Config::builder()
    .add_source(::config::File::from(config_path))
    .add_source(
      ::config::Environment::with_prefix("MANTA")
        .try_parsing(true)
        .prefix_separator("_"),
    )
    .build();

  let config = match built_config {
    Ok(good_config) => good_config,
    Err(reason) => {
      return Err(reason.to_string());
    }
  };

  Ok(config)
}

fn bad_config(e: &impl Display) -> Response {
  log(format!("ERROR {e}"));
  let error_message = "Bad server configuration".to_string();
  return error_respond(StatusCode::INTERNAL_SERVER_ERROR, error_message);
}

fn unauthorized_access(e: &impl Display) -> Response {
  log(format!("ERROR {e}"));
  let error_message = "Unauthorized access".to_string();
  return error_respond(StatusCode::UNAUTHORIZED, error_message);
}
