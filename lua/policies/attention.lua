-- attention.lua — regula o threshold de saliência da atenção.
-- Proxy de estresse: context.active_fraction (default 0.5).
-- Determinística: sem estado, sem tempo, sem aleatoriedade.

-- Limiares da fração ativa (banda de tolerância).
local LOW = 0.4
local HIGH = 0.7

triad.register_policy("attention", function(context)
  context = context or {}

  -- Default 0.5 quando o sinal está ausente.
  local fraction = context.active_fraction or 0.5

  -- Meio da banda: nada a propor.
  if fraction >= LOW and fraction <= HIGH then
    return nil
  end

  local value
  local reason
  if fraction < LOW then
    -- Folga de recursos: threshold menor amplia o foco.
    value = 0.25
    reason = "fração ativa baixa — threshold reduzido para ampliar foco"
  else
    -- Sobrecarga: threshold maior contém o estresse.
    value = 0.45
    reason = "fração ativa alta — threshold elevado para conter estresse"
  end

  return {
    target_module = "l3.attention",
    target_parameter = "salience_threshold",
    value = value,
    reason = reason,
    confidence = 0.6,
    ttl = 5,
  }
end)
