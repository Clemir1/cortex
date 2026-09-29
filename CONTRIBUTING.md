# CONTRIBUTING — Triad_AEE / Cortex (núcleo mínimo L1–L5)

Como contribuir para o núcleo mínimo. As regras abaixo derivam da base
documental normativa em `doc/` (comece por `doc/README.md`).

## 1. Domain-first

Todo trabalho segue **domínio → módulo → implementação**, nesta ordem:

1. **Domínio** — o que o componente faz: estado único, input único, output
   único, quem consome, o que desaparece se ele for removido (as cinco
   perguntas do gate de admissão). Resposta "nenhuma" ⇒ módulo redundante
   (fundir, não criar).
2. **Módulo** — o contrato (tipos/eventos em `crates/contracts/`), antes de
   qualquer implementação.
3. **Implementação** — código em Rust; política ajustável em Lua.

Implementação sem domínio descrito não entra no core.

## 2. ADRs para decisões irreversíveis

Decisões que não podem ser revertidas sem custo alto (ownership de estado,
fronteiras, taxonomia, linguagem) vivem em ADRs: `doc/adr/`. Um ADR é
imutável — mudar uma decisão exige um ADR novo que substitui o anterior,
nunca a edição do existente.

## 3. Testes de arquitetura obrigatórios

`tests/architecture/` é parte da definição de pronto. Guardas mínimos:

- `no_layer_cycles` — dependências unidirecionais
  foundation → contracts → runtime → L1 → L2 → L3 → L4 → L5, sem ciclos;
- `no_direct_actuation_from_lua` — Lua nunca atua diretamente;
- `no_infrastructure_in_brain` — cognição não conhece SQL/persistência;
- `no_stale_as_value` — STALE/NO_DATA/FALLBACK nunca viram valor medido;
- `no_undefined_layers` — todo módulo declara layer/region no descriptor;
- `no_context_key_collision` — TypedContext sem colisão de chaves;
- `no_global_mutable_state` — estado canônico tem owner único.

Toda alteração de dependências ou fronteiras deve manter esses testes
verdes; se um teste de arquitetura precisa mudar, a mudança exige ADR.

## 4. Formatação e lints

    cargo fmt --all
    cargo clippy --workspace -- -D warnings

Ambos são pré-condição para merge. Idioma: documentação em português (BR);
identificadores e termos técnicos em inglês.

## 5. Lua é apenas política

Lua recebe `PolicySnapshot` e produz `PolicyProposal`; a visão é limitada e
nunca expõe estado real de clusters. Lua **nunca**: estado canônico,
persistência, buffers GPU, threads/locks, ownership (ADR-0003). Política
reprovada na validação de range ou pelas leis não é aplicada — e a recusa
é telemetrada.

## 6. Ausência ≠ zero

Todo valor carrega status: `VALUE`, `NO_DATA`, `STALE`, `INVALID`,
`PENDING`, `DISABLED`, `ERROR`, `FALLBACK`.

Proibições: converter `NO_DATA` em zero; tratar `STALE` como fresco;
publicar `FALLBACK` como valor medido. Ausência de dado é um fato com
status, não um número.

## 7. Promoção extensão → core

Componentes de `extensions/` só entram no core com todos os critérios
cumpridos: **necessidade demonstrada + efeito reproduzível + consumidor
real + benefício multi-seed + sem duplicação funcional**. Nada é apagado:
pesquisa permanece em `extensions/` ou `legacy/research/`.

## 8. Evidência em todo relato

Toda afirmação de estado cita a fonte e o nível de evidência E0–E5
(DECLARED → INITIALIZED → EXECUTED → PRODUCTIVE → CONSUMED →
EFFECT_VALIDATED), sem promoção automática. Documento não promove veredito.

## Checklist de PR

- [ ] Domínio descrito antes da implementação (cinco perguntas do gate)
- [ ] Contratos/tipos revisados antes do código
- [ ] Testes de arquitetura verdes (`tests/architecture/`)
- [ ] `cargo fmt --all` e `cargo clippy --workspace` limpos
- [ ] ADR novo para decisão irreversível (nunca editar ADR existente)
- [ ] Nenhum fallback/STALE publicado como valor medido
- [ ] Lua (se aplicável) restrita a política
