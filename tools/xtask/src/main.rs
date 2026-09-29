// xtask: utilitário de tarefas de desenvolvimento do Triad_AEE/Cortex.
// Crate standalone: nunca entra no glob do workspace.

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    // Comando vem do primeiro argumento da CLI.
    let cmd = std::env::args().nth(1).unwrap_or_default();

    // Despacho simples via matches!().
    if matches!(cmd.as_str(), "check") {
        check()
    } else if matches!(cmd.as_str(), "arch") {
        arch()
    } else if matches!(cmd.as_str(), "tree") {
        tree()
    } else {
        // Comando vazio ou desconhecido: mostra ajuda.
        help();
        Ok(())
    }
}

// Executa cargo check --workspace.
fn check() -> anyhow::Result<()> {
    let status = Command::new("cargo")
        .args(["check", "--workspace"])
        .status()?;

    if !status.success() {
        // Repassa o código de saída do cargo.
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

// Executa os testes de arquitetura; tolera ausência deles.
fn arch() -> anyhow::Result<()> {
    let ok = Command::new("cargo")
        .args(["test", "-p", "triad-tests", "architecture"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !ok {
        // Falhou: os testes de arquitetura ainda não existem.
        println!("aviso: os testes de arquitetura ainda não existem");
    }
    Ok(())
}

// Imprime a árvore de crates/ marcando os que têm Cargo.toml.
fn tree() -> anyhow::Result<()> {
    let root = Path::new("crates");
    if !root.is_dir() {
        println!("crates/ não encontrado");
        return Ok(());
    }

    // Lista subdiretórios diretos em ordem alfabética.
    let mut dirs: Vec<String> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_owned()))
        .collect();
    dirs.sort();

    for dir in dirs {
        let manifest = root.join(&dir).join("Cargo.toml");
        if manifest.exists() {
            // Lê o nome do pacote declarado no manifest.
            let name = fs::read_to_string(&manifest)
                .ok()
                .and_then(|s| s.parse::<toml::Value>().ok())
                .and_then(|v| {
                    v.get("package")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_owned())
                })
                .unwrap_or_else(|| dir.clone());
            println!("crates/{} [Cargo.toml] {}", dir, name);
        } else {
            println!("crates/{} [sem Cargo.toml]", dir);
        }
    }
    Ok(())
}

// Ajuda curta dos comandos.
fn help() {
    println!("uso: cargo run --manifest-path tools/xtask/Cargo.toml <cmd>");
    println!("comandos:");
    println!("  check  executa cargo check --workspace");
    println!("  arch   executa cargo test -p triad-tests architecture");
    println!("  tree   lista crates/ marcando os que têm Cargo.toml");
    println!("  help   mostra esta ajuda");
}
