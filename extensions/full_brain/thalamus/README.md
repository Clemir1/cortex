Status: EXTENSÃO — fora do core (ADR-0006)

# thalamus — roteamento e gating

Papel funcional (fonte 2 §5, divisão 03): routing, queues, attention
routing, cross-region routing, priority, signal admission e regulação
de tráfego.

## Absorvido pelo core por enquanto

- `l4_global/integration`: router, attention, competition e
  synchronization como subsistemas do workspace.
- `runtime/event_bus`: transporte de mensagens tipadas entre
  componentes.

## Restaria exclusivo à extensão

- Gating de admissão de sinais por relevância (filtro antes do L4).
- Regulação fina de tráfego entre regiões e priorização dinâmica de
  filas, como módulo dedicado.

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
