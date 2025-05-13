use axum::http::StatusCode;
use std::collections::HashMap;

use csm_rs::bss::{
  http_client::get as boot_parameters_get, types::BootParameters,
};

use crate::backend_api::*;

pub async fn get_kernel_parameters_from_mesa(
  config: ReqCfg,
  xnames: &[String],
) -> Result<HashMap<String, String>, (StatusCode, String)> {
  let boot_param_vec: Vec<BootParameters> = boot_parameters_get(
    config.auth_token.as_str(),
    config.site_cfg.shasta_base_url.as_str(),
    &config.site_cfg.shasta_root_cert,
    xnames,
  )
  .await
  .unwrap();

  let mut rmap: HashMap<String, String> = HashMap::new();
  for bp in boot_param_vec {
    let xname = bp.hosts.first().unwrap().to_string();
    let params = bp.params;
    rmap.insert(xname, params);
  }

  Ok(rmap)
}
