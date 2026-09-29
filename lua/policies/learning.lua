-- learning.lua — ajusta a TAXA de aprendizagem (eta).
-- LEI MÁXIMA: a aprendizagem NUNCA é suspensa nem parada.
-- Este arquivo propõe apenas eta dentro de 0.005..0.02.
-- Determinística: sem estado, sem tempo, sem aleatoriedade.

-- Faixas da taxa (eta) e do erro preditivo.
local ETA_MIN = 0.005
local ETA_MAX = 0.02
local ERR_LOW = 0.05
local ERR_HIGH = 0.2

triad.register_policy("learning", function(context)
  context = context or {}

  -- Default 0.1 quando o sinal está ausente.
  local err = context.prediction_error or 0.1

  -- Erro alto -> eta maior; erro baixo -> eta menor (linear, puro).
  local t = (err - ERR_LOW) / (ERR_HIGH - ERR_LOW)
  if t < 0 then t = 0 end
  if t > 1 then t = 1 end
  local eta = ETA_MIN + (ETA_MAX - ETA_MIN) * t

  return {
    target_module = "learning",
    target_parameter = "eta",
    value = eta,
    reason = "ajuste de taxa de aprendizagem — nunca suspensão",
    confidence = 0.7,
    ttl = 20,
  }
end)
