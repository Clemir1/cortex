-- crisis.lua — máquina de estados PURA de crise energética.
-- LEI DA CASA: crise muda POLÍTICA, nunca desliga sistemas.
-- O estado ENTRA por argumento (context.crisis_state) e SAI por retorno.
-- Função pura: sem globals mutáveis, sem tempo, sem aleatoriedade.

-- Limiares com HYSTERESE: entrar e sair têm valores distintos.
local EMERGENCY_IN = 0.2   -- entra em emergência abaixo daqui.
local EMERGENCY_OUT = 0.35 -- só sai da emergência acima daqui.
local ALERT_IN = 0.4       -- entra em alerta abaixo daqui.
local ALERT_OUT = 0.5      -- só sai do alerta acima daqui.
local STABLE_GOAL = 3      -- ticks estáveis para voltar ao normal.

-- Transições declarativas: (energia, ticks) -> estado novo ou nil.
-- Retorno nil mantém o estado atual (anti-oscilação).
local TRANSITIONS = {
  normal = {
    to_emergency = function(e, _)
      if e < EMERGENCY_IN then return "emergency" end
    end,
    to_alert = function(e, _)
      if e < ALERT_IN then return "alert" end
    end,
  },
  alert = {
    to_emergency = function(e, _)
      if e < EMERGENCY_IN then return "emergency" end
    end,
    to_normal = function(e, _)
      if e > ALERT_OUT then return "normal" end
    end,
  },
  emergency = {
    to_recovering = function(e, _)
      if e > EMERGENCY_OUT then return "recovering" end
    end,
  },
  recovering = {
    to_emergency = function(e, _)
      -- Recaída durante a recuperação.
      if e < EMERGENCY_IN then return "emergency" end
    end,
    to_normal = function(_, ticks)
      -- Fecha a recuperação após 3 sinais estáveis.
      if ticks >= STABLE_GOAL then return "normal" end
    end,
  },
}

-- Ordem de avaliação fixa (pairs não garante ordem).
local PRIORITY = {
  "to_emergency",
  "to_alert",
  "to_recovering",
  "to_normal",
}

-- Proposta por estado; tabela NOVA a cada chamada (pureza).
local function proposal_for(state)
  if state == "emergency" then
    return {
      target_module = "l4.global",
      target_parameter = "workspace.mode",
      value = "degraded",
      reason = "crise energética — modo degradado (política, não desligamento)",
      confidence = 0.9,
      ttl = 15,
    }
  end
  if state == "recovering" then
    return {
      target_module = "l4.global",
      target_parameter = "workspace.mode",
      value = "restoring",
      reason = "recuperação pós-crise — modo em restauração (política, não desligamento)",
      confidence = 0.9,
      ttl = 15,
    }
  end
  -- normal e alert: sem proposta de modo de trabalho.
  return nil
end

triad.register_policy("crisis", function(context)
  context = context or {}

  -- Sinais de entrada; defaults quando ausentes.
  local energy = context.mean_energy or 0.5
  local ticks = context.stable_ticks or 0
  local state = context.crisis_state or "normal"

  -- Estado desconhecido cai em "normal" (determinístico).
  local trans = TRANSITIONS[state]
  if trans == nil then
    state = "normal"
    trans = TRANSITIONS[state]
  end

  -- Primeira transição que dispara vence; nenhuma mantém o estado.
  local next_state = state
  for _, name in ipairs(PRIORITY) do
    local step = trans[name]
    if step ~= nil then
      local target = step(energy, ticks)
      if target ~= nil then
        next_state = target
        break
      end
    end
  end

  -- Retorna DUAS coisas: o novo estado e a proposta (ou nil).
  return next_state, proposal_for(next_state)
end)
