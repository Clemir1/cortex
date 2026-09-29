# benches/

Benchmarks de desempenho do núcleo Triad_AEE.

## Regras

- Benchmarks rodam FORA do loop cognitivo (lei da casa: `Genome::evaluate_offline`).
- Nenhum benchmark escreve estado canônico; só lê snapshots.
- Taxas sempre com denominador explícito; ausência nunca vira zero.

## Planejado

- `l1_step`: custo de um passo físico do substrato.
- `workspace_burst`: competição do espaço de trabalho com 128 entradas.
- `bus_throughput`: vazão do barramento de eventos por tick.

Criado pelo scaffold da sessão 6. Implementação fica para as sessões futuras.
