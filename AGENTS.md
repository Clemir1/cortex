# AGENTS.md — Instruções para agentes e sessões no projeto Triad_AEE

## O que é este projeto
Triad_AEE: organismo cognitivo artificial em Rust + Lua.
Núcleo em camadas L1–L5 (Substrato, Tecido, Cognição local, Cognição global,
Metacognição) + transversais (runtime, learning, cybernetics, governance,
development) + plataforma. Rust é o estado e o mecanismo canônico.
Lua é APENAS política (nunca escreve estado, nunca atua diretamente).

## Regras de comunicação
- Responda SEMPRE em português (pt-br), frases curtas.
- O usuário não lê inglês.

## Coordenação entre sessões (6 e 7)
- Sessão 6 = orquestradora. Sessão 7 = colaboradora.
- Arquivo de coordenação: `CHECKLIST_IMPLEMENTACAO.txt`.
- Protocolo:
  1. Leia o checklist.
  2. Escolha tarefas com 2ª caixa VAZIA.
  3. Escreva o número da sua sessão (6 ou 7) na 2ª caixa ANTES de começar.
  4. Ao concluir, marque [x] na 1ª caixa.
- Nunca desmarque caixas da outra sessão. Conflito? Sessão 6 decide.
- Preferência da sessão 7: tarefas das SEÇÕES 8 (verificação/consistência),
  que não disputam os crates em escrita pela sessão 6.

## Fontes de verdade (LEITURAS PERMITIDAS — apenas estas, são curtas)
1. `_scaffold_contrato.md` (raiz) — assinaturas REAIS das crates + regras de
   scaffold. LEIA ESTE PRIMEIRO antes de escrever qualquer código.
2. `doc/contratos_camadas.md` (~120 linhas)
3. `doc/overview.md` (~142 linhas)
4. `doc/rust_lua_boundary.md` (~88 linhas)
- NUNCA leia: `Renomear.txt.txt`, `NUCLEO_MINIMO.txt.txt`,
  `estrutura_diretorios.md` dentro de um agente que vai escrever código.
  São muito grandes e estouram o contexto (o orquestrador já os processou).
- `doc/` é intocável: nunca edite, nunca apague.

## Leis da casa (invioláveis)
1. Evidência E0–E5: sem promoção automática; taxas SEMPRE com denominador.
2. Ausência ≠ zero: `Qualified<T>`; NO_DATA/STALE/INVALID/FALLBACK nunca
   viram VALUE.
3. FALLBACK exige `provider_id` + `reason`.
4. Aprendizagem nunca é suspensa; crise muda POLÍTICA, nunca desativa sistemas.
5. Decisão→resultado→aprendizagem só fecha com `verified_future_effect`
   (`LearningEnvelope::is_closed()`).
6. Intervenções de governança revertem sem `observed_effect` dentro do ttl.
7. `Genome::evaluate_offline` roda FORA do loop principal.
8. Regiões cerebrais ficam em `extensions/full_brain/` (ADR-0006).

## Erros conhecidos (não perca tempo com eles)
- Comandos cargo da workspace falham até `crates/bindings/lua/Cargo.toml`
  existir. Comportamento conhecido. Não é bloqueio.
- A write tool REJEITA `content` que é JSON válido puro. Contorno: use pwsh
  com here-strings `@'...'@` + `Set-Content -Encoding utf8`.
- Agentes que LEEM arquivos grandes antes de escrever estouram o contexto.
  Regra: NÃO leia antes de escrever. Escreva direto com o spec que já tem.
- Cargo.toml de crates: `version = "0.1.0"`, `edition = "2021"` LITERAIS,
  sem workspace inheritance.
