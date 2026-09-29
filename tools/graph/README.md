# tools/graph

Visualização de grafos a partir de checkpoints e telemetria em var/runs.
Cobre clusters, tecidos, conceitos e o grafo causal.
Entrada: snapshots de checkpoint e logs de telemetria por run.
Saída: DOT/imagens para inspeção visual no desenvolvimento.
Somente leitura: a ferramenta nunca escreve no estado do runtime.
