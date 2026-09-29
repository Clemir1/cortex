# tools/profiling

Profiling de performance do núcleo Rust e da janela Lua.
Foco no hot path: L1 (ingestão/percepção) e L3 (orquestração).
Mede custos de governança conforme a lei de custo (D8).
Relatórios ficam em var/runs; o runtime não é alterado.
Otimizações são decididas com números, não por achismo.
