use axum::{
  extract::Query,
  http::{HeaderMap, StatusCode},
  response::{IntoResponse, Response},
};
//use hyper::HeaderMap;
use std::collections::HashMap;

use crate::backend_api::*;
use crate::http_response::*;

pub async fn get_kernel_parameters(
  headers: HeaderMap,
  Query(params): Query<Vec<(String, String)>>,
) -> Response {
  let input_map = match compute_get_entries(params) {
    Ok(good) => good,
    Err(e) => return e,
  };

  let dc = input_map.get("dc").unwrap().first().unwrap();
  let xnames = input_map.get("node").unwrap();

  let cfg = match get_req_cfg(&headers, dc.to_string()) {
    Ok(good) => good,
    Err(e) => return e,
  };

  let kernel_params =
    get_kernel_parameters_from_mesa(cfg, &xnames).await.unwrap();

  if kernel_params.len() != xnames.len() {
    let difference = xnames.len() - kernel_params.len();

    let mut missing_list = "".to_owned();
    for x in xnames {
      if !kernel_params.contains_key(x.as_str()) {
        if missing_list.len() > 0 {
          missing_list.push_str(",");
        }
        missing_list.push_str(x.as_str());
      };
    }

    let mut s = "".to_owned();
    if difference != 1 {
      s.push_str("s")
    }
    return error_respond(
      StatusCode::NOT_FOUND,
      format!("{difference} node{s} missing: {missing_list}"),
    );
  };

  let json = serde_json::to_string_pretty(&kernel_params).unwrap();
  json.into_response()
}

fn compute_get_entries(
  params: Vec<(String, String)>,
) -> Result<HashMap<String, Vec<String>>, Response> {
  let map = adjust_get_entries(&params);

  let result = vet_get_entries(&map);
  match result {
    Ok(_) => return Ok(map),
    Err(e) => {
      return Err(error_respond(StatusCode::BAD_REQUEST, format!("{e}")));
    }
  }
}

fn adjust_get_entries(
  entries: &Vec<(String, String)>,
) -> HashMap<String, Vec<String>> {
  let mut map: HashMap<String, Vec<String>> = HashMap::new();

  for (key, value) in entries.iter() {
    map
      .entry(key.clone())
      .and_modify(|v| v.push(value.clone()))
      .or_insert(vec![value.to_string()]);
  }

  map
}

fn vet_get_entries(map: &HashMap<String, Vec<String>>) -> Result<(), String> {
  let mut dc_found = false;
  let mut node_found = false;
  for (k, v) in map.iter() {
    let words = v;
    match k.as_str() {
      "dc" => {
        dc_found = true;
        if words.len() != 1 {
          return Err("One and only \"dc\" must be specified!".to_string());
        }
      }
      "node" => node_found = true,
      _ => return Err(format!("Unrecognized key \"{k}\"").to_string()),
    }
  }

  if !dc_found {
    return Err("One and only \"dc\" must be specified!".to_string());
  } else if !node_found {
    return Err("At least one \"node\" must be specified!".to_string());
  }

  Ok(())
}
