# experiments — testes de políticas Lua

## Passo a passo
1. Carregue `lua/init.lua` (cria a global `triad`) e o arquivo da política, ex.: `lua/policies/energy.lua` — no host Rust ou num REPL Lua puro.
2. Chame a função com um context de exemplo: `local proposal = triad.policies.energy({ mean_energy = 0.3 })`.
3. Se voltar tabela, confira os 6 campos: `target_module`, `target_parameter`, `value`, `reason` (string OBRIGATÓRIA), `confidence` (0..1), `ttl` (inteiro > 0).
4. Retorno `nil` é válido (política escolheu não propor); `crisis` devolve estado + proposta.

## Context de exemplo
```lua
local context = {
  mean_energy = 0.3,       -- energy / crisis
  active_fraction = 0.35,  -- attention
  prediction_error = 0.2,  -- learning
  crisis_state = "normal", -- crisis
  stable_ticks = 0,        -- crisis
  damage_queue_len = 2,    -- recovery
}
```

## Leis da casa
- Lua é política: nunca escreve estado, nunca age diretamente.
- Sem `os`, `io`, `debug`, `require`, tempo ou aleatoriedade.
- Toda policy é determinística dado o mesmo context.
