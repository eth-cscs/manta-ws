use crate::{
  common::{self, config::types::MantaConfiguration},
  manta_backend_dispatcher::StaticBackendDispatcher,
};
use axum::{
  Json,
  extract::Path,
  response::{IntoResponse, Response},
};
use hyper::{HeaderMap, StatusCode};
use manta_backend_dispatcher::{
  interfaces::hsm::{
    component_ethernet_interface::ComponentEthernetInterfaceTrait,
    redfish_endpoint::RedfishEndpointTrait,
  },
  types::hsm::inventory::{ComponentEthernetInterface, RedfishEndpointArray},
};

pub async fn get_all_redfish_endpoints(headers: HeaderMap) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("ERROR - Site '{}' not found in configuration", site_name),
      )
        .into_response();
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt =
    backend.get_all_redfish_endpoints(auth_token).await;

  match boot_parameters_rslt {
    Ok(boot_parameters_vec) => {
      return (StatusCode::OK, Json(boot_parameters_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

pub async fn get_all_ethernet(headers: HeaderMap) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("ERROR - Site '{}' not found in configuration", site_name),
      )
        .into_response();
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let eth_interface_rslt = backend
    .get_all_component_ethernet_interfaces(auth_token)
    .await;

  match eth_interface_rslt {
    Ok(eth_interface_vec) => {
      return (StatusCode::OK, Json(eth_interface_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[axum::debug_handler]
pub async fn get_ethernet(
  headers: HeaderMap,
  Path(eth_interface_id): Path<String>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("ERROR - Site '{}' not found in configuration", site_name),
      )
        .into_response();
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let eth_interface_rslt = backend
    .get_component_ethernet_interface(auth_token, &eth_interface_id)
    .await;

  match eth_interface_rslt {
    Ok(eth_interface_vec) => {
      return (StatusCode::OK, Json(eth_interface_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[axum::debug_handler]
pub async fn post_ethernet(
  headers: HeaderMap,
  Json(eth_interface): Json<ComponentEthernetInterface>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let eth_interface_rslt = backend
    .add_component_ethernet_interface(auth_token, &eth_interface)
    .await;

  match eth_interface_rslt {
    Ok(eth_interface_vec) => {
      return (StatusCode::OK, Json(eth_interface_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[axum::debug_handler]
pub async fn delete_ethernet(
  headers: HeaderMap,
  Path(eth_interface_id): Path<String>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let eth_interface_rslt = backend
    .delete_component_ethernet_interface(auth_token, &eth_interface_id)
    .await;

  match eth_interface_rslt {
    Ok(eth_interface_vec) => {
      return (StatusCode::OK, Json(eth_interface_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

pub async fn get_all_redfish(headers: HeaderMap) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("ERROR - Site '{}' not found in configuration", site_name),
      )
        .into_response();
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt =
    backend.get_all_redfish_endpoints(auth_token).await;

  match boot_parameters_rslt {
    Ok(boot_parameters_vec) => {
      return (StatusCode::OK, Json(boot_parameters_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[axum::debug_handler]
pub async fn get_redfish(
  headers: HeaderMap,
  Path(xname): Path<String>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt = backend
    .get_redfish_endpoints(
      auth_token,
      Some(&xname),
      None,
      None,
      None,
      None,
      None,
      None,
    )
    .await;

  match boot_parameters_rslt {
    Ok(boot_parameters_vec) => {
      return (StatusCode::OK, Json(boot_parameters_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[axum::debug_handler]
pub async fn post_redfish(
  headers: HeaderMap,
  Json(redfish_endpoint): Json<RedfishEndpointArray>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt = backend
    .add_redfish_endpoint(auth_token, &redfish_endpoint)
    .await;

  match boot_parameters_rslt {
    Ok(boot_parameters_vec) => {
      return (StatusCode::OK, Json(boot_parameters_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}

#[axum::debug_handler]
pub async fn delete_redfish(
  headers: HeaderMap,
  Path(xname): Path<String>,
) -> Response {
  // Configuration
  let settings = common::config::get_configuration().await.unwrap();

  let configuration: MantaConfiguration = settings.try_deserialize().unwrap();

  let site_name: String = configuration.site;
  let site_detail_value_opt = configuration.sites.get(&site_name);

  let site = match site_detail_value_opt {
    Some(site_detail_value) => site_detail_value,
    None => {
      eprintln!("ERROR - Site '{}' not found in configuration", site_name);
      std::process::exit(1);
    }
  };

  let backend_tech = &site.backend;
  let shasta_base_url = &site.shasta_base_url;

  let root_ca_cert_file = &site.root_ca_cert_file;

  let shasta_root_cert =
    common::config::get_csm_root_cert_content(&root_ca_cert_file).unwrap();

  // Backend
  let backend = StaticBackendDispatcher::new(
    &backend_tech,
    &shasta_base_url,
    &shasta_root_cert,
  );

  // Get auth token
  let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
  let auth_token = auth_header.split(" ").nth(1).unwrap();

  let boot_parameters_rslt =
    backend.delete_redfish_endpoint(auth_token, &xname).await;

  match boot_parameters_rslt {
    Ok(boot_parameters_vec) => {
      return (StatusCode::OK, Json(boot_parameters_vec)).into_response();
    }
    Err(e) => {
      return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string()))
        .into_response();
    }
  }
}
