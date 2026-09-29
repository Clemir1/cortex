# Regras de dependência

> Normativo. Deriva de f1 §49/§53 (grafo de dependências), f2 §49 (direção,
> proibições), f2 §43 (testes de arquitetura), f1 §47–48 (admissão/fusão).
> Enforçado por `tests/architecture/` + revisão de ADR para exceções.

## 1. Direção principal (nunca invertida)

    foundation → contracts → runtime → l1_substrate → l2_tissue
    → l3_local → l4_global → l5_meta

    learning, cybernetics, governance, development:
      atravessam por contratos (contracts) — nunca importam implementação
      de camada para alterá-la.
    platform (gpu, persistence, telemetry, storage, math):
      entra por interfaces (traits em contracts) — nenhuma camada cognitiva
      conhece SQL, buffers ou formatos de arquivo.

Cargo workspace com crates relacionadas e `Cargo.lock`/`target` compartilhados
(f2 §2): subdividir grandes projetos em múltiplos crates é a convenção.

## 2. Proibições absolutas (violá-las = quebra de arquitetura)

- `foundation → World Model`; `substrate → Global Workspace` (f2 §49).
- `semantic → database` (f2 §49) — cognição nunca conhece persistência.
- `Lua → GPU buffer`; Lua → estado canônico (f2 §49; ADR-0003).
- `L5 → L4` import direto para alterar objetos: L5 publica `PolicyProposal`
  via contrato (f1 §53); idem L3→L2 usa `AdaptationRequest`.
- Imports circulares entre camadas (feedback volta por **eventos**, f1 §53).
- Infraestrutura dentro de camadas cognitivas e "cérebro" dentro de
  infraestrutura (`no_infrastructure_in_brain`, f2 §43).
- Estado mutável global (`no_global_mutable_state`, f2 §43).
- Colisão de chaves de contexto tipado (`no_context_key_collision`, f2 §43).
- Camada não declarada no mapa (`no_undefined_layers`, f2 §43).
- `STALE` tratado como valor (`no_stale_as_value`, f2 §43; lei-contrato).

## 3. Regra de decisão Rust × Lua por componente (f2 §58)

    Rust se: mantém estado canônico · executa a cada step · pesado ·
             concorrente · usa GPU · mexe em memória · determinístico ·
             protege invariantes.
    Lua se: define política · threshold · estratégia · heurística
             substituível · experimental · mudança rápida.
    Ambos: Rust = mecanismo, Lua = política
           (ex.: scheduler Rust + política de prioridades Lua).

## 4. Descriptor de módulo (taxonomia única — síntese D4)

Todo módulo registra em `contracts`:

    region             # 12 divisões conceituais ou TRANSVERSE (f2 §5)
    layer              # L1–L5 (f1 §2)
    domain             # ex.: energy, tissue, semantic, decision
    entity_kind        # mecanismo | política | serviço | extensão
    execution_pipeline # bio | cog | meta — NÃO infere camada (f5 §4)

Nenhum campo é inferido de outro (f2 §52: pipeline ≠ camada; f5: rótulo
"L1-L2" de bio_pipeline não prova camada L2). Display agrega pelo mapa
declarado com `source=REGISTRY` (f6 R2a) — nunca por default silencioso
(anti-padrão do legado: 26 módulos contados como L3, "L5*" INFERRED).

## 5. Admissão e fusão como testes (f1 §47–48)

- **Cinco perguntas** no PR que cria/promove módulo: estado único? input
  único? output único? quem consome? o que desaparece se remover?
  Resposta "nenhuma" ⇒ rejeitado.
- **Fusão:** dois módulos que leem quase os mesmos dados, controlam o mesmo
  actuator ou otimizam a mesma variável **devem** virar um (é o destino
  documentado de grande parte da governança legada — f1 §48; f6 §1 mostra
  26 reguladores L5-de-fato).
- **Promoção extensão→core:** necessidade demonstrada + efeito reproduzível
  + consumidor real + benefício multi-seed + sem duplicação (f1 §52;
  ADR-0006).

## 6. Suíte `tests/architecture/` (gatilho de CI)

    no_layer_cycles.rs                # dependência unidirecional
    no_direct_actuation_from_lua.rs   # fronteira ADR-0003
    no_infrastructure_in_brain.rs     # separação plataforma × cognição
    no_stale_as_value.rs              # ausência ≠ zero
    no_undefined_layers.rs            # mapa completo
    no_context_key_collision.rs       # TypedContext sem colisão
    no_global_mutable_state.rs        # ownership estrito

Adicionar: `no_duplicate_metric.rs` (MetricRegistry — f4 §39), `admission_
five_questions.rs` (checklist de PR), `fusion_rule.rs` (varredura de pares
duplicados). Estes testes existem para **impedir que a nova arquitetura volte
a ser o sistema atual com nomes diferentes** (f2 §43).
