Status: EXTENSÃO — fora do core (ADR-0006)

# sensory — entrada e representação perceptiva

Papel funcional (fonte 2 §5, divisão 08): sensory adapters, feature
extraction, spatial representation, multimodal grounding e modalidades
futuras (visão, áudio, etc.).

## Absorvido pelo core por enquanto

- `contracts/messages`: contratos tipados de entrada sensorial.
- `l3_local`: primeiras representações extraídas alimentam semantic,
  memory, attention e prediction.

## Restaria exclusivo à extensão

- Adapters por modalidade (visual, auditory, textual, proprioceptive,
  synthetic — fonte 2 §21).
- Grounding multimodal e representação espacial sensorial dedicada.
- O domínio existe arquiteturalmente desde já, mas como extensão: o
  core não carrega modalidades que ainda não possui.

Promoção: cinco itens cumulativos do ADR-0006 + ADR próprio.
