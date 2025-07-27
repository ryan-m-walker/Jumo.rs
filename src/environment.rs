#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Dev,
    Pi,
}

pub fn get_environment() -> Environment {
    let env = std::env::var("ENVIRONMENT").unwrap_or(String::from("dev"));

    match env.to_lowercase().as_str() {
        "pi" => Environment::Pi,
        _ => Environment::Dev,
    }
}
