#[path = "../commands/olm.rs"]
#[allow(dead_code)]
mod olm;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let output_path = args
        .windows(2)
        .find(|pair| pair[0] == "--out" || pair[0] == "-o")
        .map(|pair| pair[1].clone());

    let report = olm::run_python_domain_demonstration().unwrap_or_else(|error| {
        eprintln!("Falha ao executar demonstração: {}", error);
        std::process::exit(1);
    });
    let json = serde_json::to_string_pretty(&report).unwrap_or_else(|error| {
        eprintln!("Falha ao serializar demonstração: {}", error);
        std::process::exit(1);
    });

    if let Some(path) = output_path {
        std::fs::write(&path, &json).unwrap_or_else(|error| {
            eprintln!("Falha ao guardar '{}': {}", path, error);
            std::process::exit(1);
        });
        println!("Demonstração guardada em {}", path);
    } else {
        println!("{}", json);
    }
}
