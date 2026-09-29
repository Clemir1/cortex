# Fronteira Rust ↔ Lua

> Normativo. Deriva de f2 §3/§35–36 (contrato, estrutura `lua/`), f1 §29/§36
> (políticas de Lua, soft laws), f4 §28 (governança por contratos), f6 §0/§3
> (gates da auditoria: zero edição de core por script). Implementação:
> `crates/bindings/lua/` + raiz `lua/`.

## 1. Princípio

Rust mantém o organismo (estado canônico, invariantes, actuação). Lua ajusta
o organismo (políticas, heurísticas, thresholds, experimentos) **sem nunca
ser dona dele**. A fronteira é o lugar onde a flexibilidade experimental
encontra a segurança estrutural — portanto deve ser **extremamente rígida**
(f2 §36).

## 2. O ciclo canônico (único caminho de influência)

    1. Rust produz PolicySnapshot        (visão limitada, versionada)
    2. Lua analisa                        (função pura: snapshot → proposta)
    3. Lua produz PolicyProposal          (tipada, sem efeitos)
    4. Rust valida range                 (limites numéricos/enum)
    5. LawEnforcer valida invariantes    (leis hard)
    6. Governor arbitra                  (conflitos, ttl, prioridade)
    7. Rust aplica                        (único escritor)
    8. Telemetry observa efeito            (carimbo causal)
    9. Learning avalia                     (keep/revert)

Exemplo conceitual (f2 §36): `EnergySnapshot` → Lua propõe
`ReduceMorphogenesis(0.40)` → Rust valida o range → LawEnforcer valida
invariantes → Governor arbitra → Rust aplica → Telemetry observa o efeito.
`cluster.energy = 0.4` direto é **impossível por construção** — não há API de
escrita exposta ao script.

## 3. Propriedade da fronteira

| Rust (dono) | Lua (hóspede) |
|:--|:--|
| estado canônico, clusters, tecidos, grafos | políticas de energia/atenção/learning/crise |
| persistência, causal, buffers GPU | heurísticas, thresholds, gates |
| concorrência, locks, ownership | estratégias de exploração/exploração |
| invariantes e leis hard | soft policies, modos configuráveis |
| validação e aplicação | experimentos e ablações declarativas |

**Lua nunca possui** (f2 §3): memória canônica; clusters; tissue graph;
persistência; estado causal; buffers GPU; ownership de recursos; threads.
`lua/cluster_state.lua` contendo estado real é anti-padrão nominal do legado.
Lua recebe **visão limitada** — snapshots projetados, nunca o organismo.

## 4. Contrato técnico

- `PolicySnapshot`: dados mínimos, versionados, custo de serialização
  previsível; produzido pelo dono do estado (ex.: ResourceGovernor para
  energia; DevelopmentGovernor para morfogênese).
- `PolicyProposal`: estrutura fechada `{target_module, target_parameter,
  value, reason, confidence, ttl}` — campos fora do schema são rejeitados.
- Proposta sem `reason` é rejeitada; efeito sem `observed_effect` dentro do
  ttl aciona reversão automática (contrato de governança,
  `architecture/contratos_camadas.md` §3).
- Lei de custo (síntese D8): nenhuma política entra em produção sem medir
  custo do controlador contra a economia gerada (f6 F-6).
- Soft laws (f1 §36): esquecimento, decay, exploração, períodos de crise
  ficam em `lua/policies/`; hard laws ficam em Rust (`crates/governance/laws`).

## 5. Estrutura da raiz `lua/`

    lua/
    ├── policies/        # energy, attention, learning, crisis, recovery,
    │                    # forgetting (f1 §29)
    ├── governance/      # federation, ecology (f1 §34/§38)
    ├── development/     # corticalization, regeneration
    └── experiments/     # ablations, seeds, counterfactuals (f2 §35)

`init.lua` com `require`/`package.searchers` modular; um script por política;
sem estado compartilhado entre scripts.

## 6. Determinismo e prova

- Lua roda em janelas declaradas do scheduler — nunca no hot path L1/L3
  (f4 §36: GPU e batch em Rust; Lua é controle/experimentação).
- Mudança de política é **mudança comportamental**: exige gêmea A/B com
  `origin_state_hash` igual e protocolo t+1/t+5 (padrão das runs 772/773,
  768); mudança de display/display-only exige A/A bit-idêntica (padrão f5
  "placeholder determinístico quando OFF").
- Veredito pré-registrado: resultado de política não autoriza calibração
  retroativa (f3 §17 — `NOT_VALID_YET` permanece `NOT_VALID_YET`).
- Testes de arquitetura: `no_direct_actuation_from_lua` (f2 §43); teste de
  fronteira: snapshot→proposal→validação→aplicação→observação com fuzz de
  propostas inválidas (range, tipo, alvo inexistente).
