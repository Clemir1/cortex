//! Registro de políticas Lua (nome → código-fonte). Nada executa aqui.

/// Host de políticas: mapa nome → fonte Lua; registro e consulta apenas.
pub struct PolicyHost {
    policies: std::collections::HashMap<String, String>,
}

impl PolicyHost {
    /// Cria um host vazio, sem políticas registradas.
    pub fn new() -> Self {
        Self {
            policies: std::collections::HashMap::new(),
        }
    }

    /// Registra uma política; recusa nome vazio ou duplicado.
    pub fn register(&mut self, name: &str, source: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("nome vazio".to_string());
        }
        if self.policies.contains_key(name) {
            return Err(format!("política já registrada: {}", name));
        }
        self.policies.insert(name.to_string(), source.to_string());
        Ok(())
    }

    /// Devolve a fonte Lua da política, se registrada.
    pub fn source(&self, name: &str) -> Option<&str> {
        self.policies.get(name).map(|s| s.as_str())
    }

    /// Devolve os nomes registrados, em ordem alfabética.
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.policies.keys().cloned().collect();
        names.sort();
        names
    }
}
