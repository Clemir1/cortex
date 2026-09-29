-- recovery.lua — orçamento de regeneração pós-crise.
-- Propõe conforme a fila de danos; sem fila, não propõe.
-- Determinística: sem estado, sem tempo, sem aleatoriedade.

local BASE = 2 -- orçamento base por passo.
local CAP = 8  -- teto do orçamento por passo.

triad.register_policy("recovery", function(context)
  context = context or {}

  -- Fila de danos pendentes; default 0.
  local queue = context.damage_queue_len or 0

  -- Sem fila: nada a propor.
  if queue <= 0 then
    return nil
  end

  -- Orçamento = base + fila, limitado ao teto.
  local value = BASE + queue
  if value > CAP then
    value = CAP
  end

  return {
    target_module = "development",
    target_parameter = "regeneration.max_per_step",
    value = value,
    reason = "fila de danos pendente — orçamento de regeneração",
    confidence = 0.65,
    ttl = 8,
  }
end)
