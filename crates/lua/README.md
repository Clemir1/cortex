# ADR — Policy Lua de Federação (SEÇÃO 20.6c)

- **Status**: Proposto (ratificação pelo dono; promoção a ADR-0008+ quando abrir)
- **Série**: SEÇÃO 20 — débito de padrão c) do checklist
- **Fronteira**: doc/rust_lua_boundary.md (Lua NUNCA escreve estado; retorna `Proposal`; Rust valida e aplica)

## Contexto

O CNP federativo (18.4, crates/governance/src/federation.rs) é Rust-canônico:
`need → proposal → agreement → transfer → effect` tipados, com contrato causal
(applied_value/observed_effect/recibo/outcome t+1/t+5) e active/sham A/A.
O desenho canônico (CAMADA.txt) prevê POLICY Lua propositora na federação,
usando a fronteira já viva do organismo: `PolicyHost` (host.rs) boot com
sandbox, `LuaProposal::validate(policy_hash)` (proposal.rs) com rejeição
tipada (`PolicyReject`) e `ValidatedProposal` — Rust valida TODO parâmetro
contra o registry (ParamSpec::lookup) antes de qualquer aplicação.

## Decisão

1. A POLICY de federação é um script `federation.lua` que, dado um NEED
   tipado (recurso escasso + urgência + contexto canônico), RETORNA uma
   `LuaProposal` (rate, bônus de urgência clampado, justificativa) —
   nunca escreve estado, nunca atua (fronteira inviolável).
2. RUST ARBITRA: a proposal Lua entra pelo validador existente
   (`validate` + `ParamSpec::lookup`); o CNP aplica a política SÓ depois
   da validação; rejeição é evento tipado com razão (ausência ≠ zero).
3. CONTROLE: a policy Lua é FEATURE-OFF por padrão (a federação rodando
   Rust-canônica é o estado verde atual); ligar policy Lua = decisão do
   dono com A/A ativo/sham preservado (sham não muda trajetória).
4. A/A: policy determinística ⇒ mesma seed, mesma proposal; o
   comparador ignora _ms/run_id (herança da 17.6).

## Consequências

- (+) A arbitragem federativa vira política auditável sem recompilar;
  sandbox já existe; validação Rust já existe.
- (−) Wire runtime (CNP ⇄ PolicyHost.call) é **débito explícito**: a
  integração entra quando o dono ratificar este ADR — mínimo viável
  já entregue: validador + tipos + esta decisão registrada.
- (−) Lua propositora só na federação: NUNCA no caminho de transfer
  (transfer/effect permanecem Rust puro — Lei 5 fecha o ciclo).

## Wire futuro (débito registrado no checklist)

`FederationEngine` recebe `Option<&PolicyHost>`: `None` = canônico
(comportamento bit-idêntico ao verde atual); `Some` = policy consultada
após o `need`, antes do `proposal`, com `PolicyReject` ⇒ CnpReject tipado
reutilizado. Testes: proposal válida/inválida/clampada + A/A gêmeos com
policy ligada e desligada.
