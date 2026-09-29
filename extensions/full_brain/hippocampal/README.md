Status: EXTENSÃO — fora do core (ADR-0006)

# hippocampal — memória histórica

Papel funcional (fonte 2 §5, divisão 07): memória episódica, allocortex,
spatial/context memory, reconsolidation, replay, episódios
autobiográficos e indexação de memória.

## Absorvido pelo core por enquanto

- `l3_local/memory`: memória local (um dos quatro domínios L3).
- `l4_global/memory_integration`: working/episodic/semantic,
  retrieval, reconsolidation, consolidation e provenance.

## Restaria exclusivo à extensão

- Replay offline de episódios (conecta-se a `extensions/dream`).
- Memória autobiográfica e spatial/context memory dedicadas.
- Indexação de memória como subsistema próprio, mantendo a separação
  armazenar / recuperar / usar (fonte 2 §20).

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
