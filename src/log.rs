pub fn log(message: String) {
  eprintln!(
    "{} {message}",
    chrono::offset::Utc::now()
      .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
  );
}
